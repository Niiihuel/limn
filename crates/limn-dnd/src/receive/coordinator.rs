//! The single coordinator owned by the future receive-only protocol worker.

use std::{path::PathBuf, time::Instant};

use super::{
    control::{AttemptGate, Header, OfferHeader, ReceiveControl},
    DropDecision, OfferId, ReceiveEpoch, ReceiveEvent, ReceiveStatus, SurfacePoint,
};
use crate::{ReceiveError, ReceiveLimits};

pub(super) struct Coordinator {
    limits: ReceiveLimits,
    control: ReceiveControl,
    wake: Option<Box<dyn Fn() + Send>>,
    active: Option<Active>,
    pending_sync: Option<SyncRequest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SyncRequest {
    pub(super) epoch: ReceiveEpoch,
    pub(super) deadline: Instant,
}

#[derive(Clone, Copy)]
struct Active {
    id: OfferId,
    epoch: ReceiveEpoch,
    point: SurfacePoint,
    selected_copy: bool,
    phase: Phase,
}

#[derive(Clone, Copy)]
pub(super) enum Phase {
    Hover,
    Reading { deadline: Instant },
    AwaitingDecision { deadline: Instant },
    TerminalQueued,
}

impl Phase {
    fn deadline(self) -> Option<Instant> {
        match self {
            Self::Reading { deadline } | Self::AwaitingDecision { deadline } => Some(deadline),
            Self::Hover | Self::TerminalQueued => None,
        }
    }
}

impl Coordinator {
    #[cfg(test)]
    pub(super) fn with_control(
        limits: ReceiveLimits,
        control: ReceiveControl,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, ReceiveError> {
        limits.validate()?;
        Ok(Self::with_wake(limits, control, Box::new(wake)))
    }

    pub(super) fn with_wake(
        limits: ReceiveLimits,
        control: ReceiveControl,
        wake: Box<dyn Fn() + Send>,
    ) -> Self {
        Self {
            limits,
            control,
            wake: Some(wake),
            active: None,
            pending_sync: None,
        }
    }

    pub(super) fn notify(&self) {
        self.wake.as_ref().expect("worker still owns its wake")();
    }

    pub(super) fn take_wake(&mut self) -> Box<dyn Fn() + Send> {
        self.wake.take().expect("wake transfers to the reaper once")
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
        self.pending_sync
            .map(|sync| sync.deadline)
            .into_iter()
            .chain(self.active.and_then(|active| active.phase.deadline()))
            .min()
    }

    pub(super) fn offer_phase(&self, id: OfferId) -> Option<Phase> {
        self.active
            .filter(|active| active.id == id)
            .map(|active| active.phase)
    }

    pub(super) fn reconcile(&mut self, now: Instant) -> Result<(), ReceiveError> {
        if self.pending_sync.is_some_and(|sync| now >= sync.deadline) {
            self.control
                .header()
                .request_close(Some(ReceiveError::Timeout));
        }
        if self.control.header().is_closing() {
            self.pending_sync = None;
        }
        self.reconcile_controls();
        if let Some(active) = self.active {
            if active
                .phase
                .deadline()
                .is_some_and(|deadline| now >= deadline)
            {
                self.end(active.id, Err(ReceiveError::Timeout));
            }
        }
        Ok(())
    }

    pub(super) fn next_sync(&mut self, now: Instant) -> Result<Option<SyncRequest>, ReceiveError> {
        self.reconcile(now)?;
        let epoch = {
            let header = self.control.header();
            if header.is_closing() {
                return Err(ReceiveError::Closed);
            }
            if header.armed || self.pending_sync.is_some() {
                return Ok(None);
            }
            header.epoch
        };
        let Some(deadline) = now.checked_add(self.limits.startup_timeout) else {
            self.control
                .header()
                .request_close(Some(ReceiveError::InvalidLimits));
            return Err(ReceiveError::InvalidLimits);
        };
        let request = SyncRequest { epoch, deadline };
        self.pending_sync = Some(request);
        Ok(Some(request))
    }

    pub(super) fn sync_done(
        &mut self,
        epoch: ReceiveEpoch,
        now: Instant,
    ) -> Result<(), ReceiveError> {
        self.reconcile(now)?;
        let mut header = self.control.header();
        if header.is_closing() {
            return Err(ReceiveError::Closed);
        }
        if !self.pending_sync.is_some_and(|sync| sync.epoch == epoch) {
            return Err(ReceiveError::InvalidPhase);
        }
        self.pending_sync = None;
        if header.epoch == epoch {
            header.armed = true;
            header.status = ReceiveStatus::Ready;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn sync_failed(
        &mut self,
        epoch: ReceiveEpoch,
        error: ReceiveError,
    ) -> Result<(), ReceiveError> {
        {
            let mut header = self.control.header();
            if header.is_closing() {
                return Err(ReceiveError::Closed);
            }
            if !self.pending_sync.is_some_and(|sync| sync.epoch == epoch) {
                return Err(ReceiveError::InvalidPhase);
            }
            self.pending_sync = None;
            header.request_close(Some(error));
        }
        self.reconcile_controls();
        Ok(())
    }

    pub(super) fn enter(
        &mut self,
        id: OfferId,
        point: SurfacePoint,
        version: u32,
        has_uri_list: bool,
    ) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let queued = {
            let mut header = self.control.header();
            if header.is_closing() {
                return Err(ReceiveError::Closed);
            }
            if !header.armed {
                return Err(ReceiveError::InvalidPhase);
            }
            if self.active.is_some() {
                return Err(ReceiveError::Busy);
            }
            if version < 3 {
                return Err(ReceiveError::Unavailable);
            }
            if !has_uri_list {
                return Err(ReceiveError::InvalidUriList);
            }
            let epoch = header.epoch;
            self.active = Some(Active {
                id,
                epoch,
                point,
                selected_copy: false,
                phase: Phase::Hover,
            });
            header.offer = Some(OfferHeader {
                id,
                epoch,
                gate: AttemptGate::Open,
                ready: false,
                selected_copy: false,
                deadline: None,
            });
            header.decision = None;
            self.control.enqueue(ReceiveEvent::Hovered { id, point })
        };
        self.delivered(queued);
        Ok(())
    }

    pub(super) fn action(&mut self, id: OfferId, selected_copy: bool) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let mut active = self.current(id)?;
        let mut header = self.control.header();
        if !Self::is_live(&header, active) {
            return Err(ReceiveError::StaleOffer);
        }
        active.selected_copy = selected_copy;
        self.active = Some(active);
        if let Some(offer) = header.offer.as_mut() {
            offer.selected_copy = selected_copy;
        }
        Ok(())
    }

    pub(super) fn motion(&mut self, id: OfferId, point: SurfacePoint) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let mut active = self.current(id)?;
        if matches!(active.phase, Phase::Hover) {
            let queued = {
                let header = self.control.header();
                if !Self::is_live(&header, active) {
                    return Err(ReceiveError::StaleOffer);
                }
                active.point = point;
                self.active = Some(active);
                self.control.enqueue(ReceiveEvent::Hovered { id, point })
            };
            self.delivered(queued);
        }
        Ok(())
    }

    pub(super) fn leave(&mut self, id: OfferId) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        if matches!(self.current(id)?.phase, Phase::Hover) {
            self.retire_hover(id);
        }
        Ok(())
    }

    pub(super) fn start(&mut self, id: OfferId, now: Instant) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let mut active = self.current(id)?;
        if !matches!(active.phase, Phase::Hover) {
            return Err(ReceiveError::InvalidPhase);
        }
        if !active.selected_copy {
            self.retire_hover(id);
            return Err(ReceiveError::InvalidPhase);
        }
        let Some(deadline) = now.checked_add(self.limits.transfer_timeout) else {
            self.retire_hover(id);
            return Err(ReceiveError::InvalidLimits);
        };
        let queued = {
            let mut header = self.control.header();
            if !Self::is_live(&header, active) {
                return Err(ReceiveError::StaleOffer);
            }
            // No next offer is admitted before Ended is drained. This reserves
            // Started, Ready and Ended within the five reliable mailbox slots.
            if self.control.remaining_reliable() < 3 {
                drop(header);
                self.retire_hover(id);
                return Err(ReceiveError::Busy);
            }
            active.phase = Phase::Reading { deadline };
            self.active = Some(active);
            if let Some(offer) = header.offer.as_mut() {
                offer.deadline = Some(deadline);
            }
            self.control.enqueue(ReceiveEvent::DropStarted {
                id,
                epoch: active.epoch,
                point: active.point,
            })
        };
        self.delivered(queued);
        Ok(())
    }

    // Sequential callers provide the modeled transition time. The real worker
    // decodes without guards and calls complete_validated with a fresh post-decode time.
    #[cfg(test)]
    pub(super) fn complete(
        &mut self,
        id: OfferId,
        bytes: &[u8],
        now: Instant,
    ) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let active = self.current(id)?;
        if !matches!(active.phase, Phase::Reading { .. }) {
            return Err(ReceiveError::InvalidPhase);
        }
        self.check_deadline(active, now)?;
        match crate::uri_list::decode_received(bytes, &self.limits) {
            Ok(paths) => self.complete_validated(id, paths, now),
            Err(error) => {
                self.reconcile_controls();
                self.current(id)?;
                self.end(id, Err(error));
                Err(error)
            }
        }
    }

    pub(super) fn complete_validated(
        &mut self,
        id: OfferId,
        paths: Vec<PathBuf>,
        post_now: Instant,
    ) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let mut active = self.current(id)?;
        if !matches!(active.phase, Phase::Reading { .. }) {
            return Err(ReceiveError::InvalidPhase);
        }
        let queued = {
            let mut header = self.control.header();
            if !Self::is_live(&header, active) {
                drop(header);
                self.reconcile_controls();
                return Err(ReceiveError::StaleOffer);
            }
            let post_now = post_now.max(Instant::now());
            if active
                .phase
                .deadline()
                .is_some_and(|deadline| post_now >= deadline)
            {
                drop(header);
                self.end(id, Err(ReceiveError::Timeout));
                return Err(ReceiveError::Timeout);
            }
            let Some(deadline) = post_now.checked_add(self.limits.decision_timeout) else {
                drop(header);
                self.end(id, Err(ReceiveError::InvalidLimits));
                return Err(ReceiveError::InvalidLimits);
            };
            active.phase = Phase::AwaitingDecision { deadline };
            self.active = Some(active);
            if let Some(offer) = header.offer.as_mut() {
                offer.ready = true;
                offer.deadline = Some(deadline);
            }
            self.control.enqueue(ReceiveEvent::DropReady { id, paths })
        };
        self.delivered(queued);
        Ok(())
    }

    pub(super) fn decide(
        &mut self,
        id: OfferId,
        decision: DropDecision,
        now: Instant,
        mut finish_copy: impl FnMut(OfferId) -> Result<(), ReceiveError>,
    ) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let active = self.current(id)?;
        self.check_deadline(active, now)?;
        if decision == DropDecision::Reject {
            return self.cancel(id);
        }
        if !matches!(active.phase, Phase::AwaitingDecision { .. }) || !active.selected_copy {
            return Err(ReceiveError::InvalidPhase);
        }
        if let Err(error) = self.control.claim(id, active.epoch, now) {
            self.reconcile_controls();
            return Err(error);
        }
        // Claim is the arbitration point. No guard survives into protocol/user code.
        let result = finish_copy(id);
        self.end(id, result);
        result
    }

    pub(super) fn cancel(&mut self, id: OfferId) -> Result<(), ReceiveError> {
        self.control.cancel(id)?;
        self.reconcile_controls();
        Ok(())
    }

    pub(super) fn reading_deadline(
        &mut self,
        id: OfferId,
        now: Instant,
    ) -> Result<Instant, ReceiveError> {
        self.reconcile_controls();
        let active = self.current(id)?;
        let Phase::Reading { deadline } = active.phase else {
            return Err(ReceiveError::InvalidPhase);
        };
        let live = Self::is_live(&self.control.header(), active);
        if !live {
            self.reconcile_controls();
            return Err(ReceiveError::StaleOffer);
        }
        self.check_deadline(active, now)?;
        Ok(deadline)
    }

    pub(super) fn fail(&mut self, id: OfferId, error: ReceiveError) -> Result<(), ReceiveError> {
        self.reconcile_controls();
        let active = self.current(id)?;
        if matches!(active.phase, Phase::Hover) {
            return Err(ReceiveError::InvalidPhase);
        }
        self.end(id, Err(error));
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn expire(&mut self, now: Instant) -> Result<(), ReceiveError> {
        self.reconcile(now)
    }

    #[cfg(test)]
    pub(super) fn close(&mut self) -> Result<(), ReceiveError> {
        self.control.close()?;
        self.reconcile_controls();
        Ok(())
    }

    // Only the sequential fixtures use this forwarding method. The host polls
    // ReceiveControl directly, without borrowing or locking the worker coordinator.
    #[cfg(test)]
    pub(super) fn poll(&mut self) -> Vec<ReceiveEvent> {
        let events = self.control.poll();
        self.reconcile_controls();
        events
    }

    fn current(&self, id: OfferId) -> Result<Active, ReceiveError> {
        self.active
            .filter(|active| active.id == id && !matches!(active.phase, Phase::TerminalQueued))
            .ok_or(ReceiveError::StaleOffer)
    }

    fn is_live(header: &Header, active: Active) -> bool {
        !header.is_closing()
            && header.armed
            && header.epoch == active.epoch
            && header.offer.is_some_and(|offer| {
                offer.id == active.id
                    && offer.epoch == active.epoch
                    && offer.gate == AttemptGate::Open
            })
    }

    fn reconcile_controls(&mut self) {
        let Some(active) = self.active else {
            return;
        };
        let offer = {
            let mut header = self.control.header();
            if matches!(active.phase, Phase::TerminalQueued)
                && header.offer.is_some_and(|offer| {
                    offer.id == active.id
                        && matches!(offer.gate, AttemptGate::Ended { drained: true })
                })
            {
                header.offer = None;
                header.decision = None;
                self.active = None;
                return;
            }
            header.offer
        };
        if matches!(active.phase, Phase::TerminalQueued) {
            return;
        }
        let cancellation = match offer.filter(|offer| offer.id == active.id) {
            Some(OfferHeader {
                gate: AttemptGate::Cancelled(error),
                ..
            }) => Some(error),
            _ => None,
        };
        if let Some(error) = cancellation {
            if matches!(active.phase, Phase::Hover) {
                self.retire_hover(active.id);
            } else {
                self.end(active.id, Err(error));
            }
        }
    }

    fn check_deadline(&mut self, active: Active, now: Instant) -> Result<(), ReceiveError> {
        if active
            .phase
            .deadline()
            .is_some_and(|deadline| now >= deadline)
        {
            self.end(active.id, Err(ReceiveError::Timeout));
            return Err(ReceiveError::Timeout);
        }
        Ok(())
    }

    fn retire_hover(&mut self, id: OfferId) {
        let queued = {
            let mut header = self.control.header();
            if header.offer.is_some_and(|offer| offer.id == id) {
                header.offer = None;
                header.decision = None;
            }
            self.active = None;
            self.control.enqueue(ReceiveEvent::Left { id })
        };
        self.delivered(queued);
    }

    fn end(&mut self, id: OfferId, mut result: Result<(), ReceiveError>) {
        let (retired, queued) = {
            let mut header = self.control.header();
            let stale_context = self
                .active
                .is_some_and(|active| active.epoch != header.epoch)
                || !header.armed
                || header.is_closing();
            if let Some(offer) = header.offer.as_mut().filter(|offer| offer.id == id) {
                if let AttemptGate::Cancelled(error) = offer.gate {
                    result = Err(error);
                }
                offer.gate = AttemptGate::Ended { drained: false };
                offer.ready = false;
                offer.deadline = None;
            }
            header.decision = None;
            if let Some(active) = self.active.as_mut() {
                active.phase = Phase::TerminalQueued;
            }
            let retired = if result.is_err() || stale_context {
                self.control.retire_ready(id)
            } else {
                None
            };
            let queued = self.control.enqueue(ReceiveEvent::DropEnded { id, result });
            (retired, queued)
        };
        drop(retired);
        self.delivered(queued);
    }

    fn delivered(&self, queued: Result<(), ReceiveEvent>) {
        queued.expect("one admitted drop reserves space for Started, Ready and Ended");
        self.notify();
    }
}

#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod tests;
