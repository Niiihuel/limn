//! Shared admission and the single receive-only mailbox.
//! Header guards never own paths, protocol objects, or user callbacks.

use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Instant;

use super::{
    clipboard_control::{TextHeader, TextRequestCore, TextRequestId},
    events::ReceiveMailbox,
    worker_pool::SurfaceId,
    DropDecision, OfferId, ReceiveEpoch, ReceiveEvent, ReceiveStatus,
};
use crate::{ClipboardError, ReceiveError};

#[derive(Clone)]
pub(super) struct ReceiveControl {
    shared: Arc<Shared>,
}

struct Shared {
    parent: Option<ReceiveControl>,
    registration: Option<SurfaceId>,
    header: Mutex<Header>,
    mailbox: Mutex<ReceiveMailbox>,
}

pub(super) struct Header {
    pub(super) epoch: ReceiveEpoch,
    pub(super) armed: bool,
    pub(super) status: ReceiveStatus,
    pub(super) close_cause: Option<ReceiveError>,
    pub(super) offer: Option<OfferHeader>,
    pub(super) decision: Option<DecisionSlot>,
    pub(super) incoming: IncomingGate,
    pub(super) text: Option<TextHeader>,
    pub(super) text_sealed: Option<(SurfaceId, ReceiveEpoch)>,
    text_request: Option<Weak<TextRequestCore>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum IncomingLease {
    Drop {
        registration: Option<SurfaceId>,
        id: OfferId,
    },
    Text(TextRequestId),
}

pub(super) struct IncomingGate {
    current: Option<IncomingLease>,
}

impl IncomingGate {
    pub(super) fn current(&self) -> Option<IncomingLease> {
        self.current
    }

    pub(super) fn acquire(&mut self, lease: IncomingLease) -> Result<(), ReceiveError> {
        if self.current.is_some() {
            return Err(ReceiveError::Busy);
        }
        self.current = Some(lease);
        Ok(())
    }

    pub(super) fn release(&mut self, exact: IncomingLease) -> bool {
        if self.current != Some(exact) {
            return false;
        }
        self.current = None;
        true
    }
}

#[derive(Clone, Copy)]
pub(super) enum DecisionSlot {
    Queued(OfferId, DropDecision),
    Taken,
}

#[derive(Clone, Copy)]
pub(super) struct OfferHeader {
    pub(super) id: OfferId,
    pub(super) epoch: ReceiveEpoch,
    pub(super) gate: AttemptGate,
    pub(super) ready: bool,
    pub(super) selected_copy: bool,
    pub(super) deadline: Option<Instant>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AttemptGate {
    Open,
    Cancelled(ReceiveError),
    Claimed,
    Ended { drained: bool },
}

impl Header {
    pub(super) fn is_closing(&self) -> bool {
        matches!(
            self.status,
            ReceiveStatus::Closing | ReceiveStatus::Closed(_)
        )
    }

    fn cancel_open(&mut self, error: ReceiveError) {
        self.decision = None;
        if let Some(offer) = self.offer.as_mut() {
            if offer.gate == AttemptGate::Open {
                offer.gate = AttemptGate::Cancelled(error);
                offer.ready = false;
            }
        }
    }

    fn cancel_text(&mut self, error: ClipboardError) {
        self.text_sealed = None;
        if let Some(text) = self.text.as_ref() {
            text.cancel(error);
        }
        // Core owns metadata only. This never reacquires the parent header.
        if let Some(request) = self.text_request.as_ref().and_then(Weak::upgrade) {
            request.cancel(error);
        }
    }

    pub(super) fn set_text_request(&mut self, request: Weak<TextRequestCore>) {
        self.text_request = Some(request);
    }

    pub(super) fn request_close(&mut self, cause: Option<ReceiveError>) {
        if !matches!(self.status, ReceiveStatus::Closed(_)) {
            self.armed = false;
            self.status = ReceiveStatus::Closing;
            self.close_cause = self.close_cause.or(cause);
            self.cancel_open(cause.unwrap_or(ReceiveError::Cancelled));
            self.cancel_text(ClipboardError::Closed);
        }
    }
}

impl ReceiveControl {
    pub(super) fn new(epoch: ReceiveEpoch) -> Result<Self, ReceiveError> {
        Self::with_parent(epoch, None, None)
    }

    pub(super) fn new_for_surface(
        epoch: ReceiveEpoch,
        parent: ReceiveControl,
        registration: SurfaceId,
    ) -> Result<Self, ReceiveError> {
        if !parent.is_root() {
            return Err(ReceiveError::Unavailable);
        }
        Self::with_parent(epoch, Some(parent), Some(registration))
    }

    fn with_parent(
        epoch: ReceiveEpoch,
        parent: Option<ReceiveControl>,
        registration: Option<SurfaceId>,
    ) -> Result<Self, ReceiveError> {
        Ok(Self {
            shared: Arc::new(Shared {
                parent,
                registration,
                header: Mutex::new(Header {
                    epoch,
                    armed: false,
                    status: ReceiveStatus::Starting,
                    close_cause: None,
                    offer: None,
                    decision: None,
                    incoming: IncomingGate { current: None },
                    text: None,
                    text_sealed: None,
                    text_request: None,
                }),
                mailbox: Mutex::new(ReceiveMailbox::new()),
            }),
        })
    }

    pub(super) fn is_root(&self) -> bool {
        self.shared.parent.is_none()
    }

    pub(super) fn belongs_to(&self, root: &Self) -> bool {
        self.shared
            .parent
            .as_ref()
            .is_some_and(|parent| Arc::ptr_eq(&parent.shared, &root.shared))
    }

    pub(super) fn registration(&self) -> Option<SurfaceId> {
        self.shared.registration
    }

    pub(super) fn incoming(&self) -> Option<IncomingLease> {
        self.shared
            .parent
            .as_ref()
            .unwrap_or(self)
            .header()
            .incoming
            .current()
    }

    pub(super) fn reserve_incoming_drop(
        &self,
        registration: Option<SurfaceId>,
        id: OfferId,
    ) -> Result<(), ReceiveError> {
        if self.registration() != registration {
            return Err(ReceiveError::StaleOffer);
        }
        let lease = IncomingLease::Drop { registration, id };
        if let Some(parent) = self.shared.parent.as_ref() {
            let mut root = parent.header();
            let endpoint = self.header();
            if root.is_closing() || endpoint.is_closing() {
                return Err(ReceiveError::Closed);
            }
            if root.status != ReceiveStatus::Ready
                || endpoint.status != ReceiveStatus::Ready
                || !endpoint.armed
            {
                return Err(ReceiveError::Unavailable);
            }
            root.incoming.acquire(lease)
        } else {
            let mut root = self.header();
            if root.is_closing() {
                return Err(ReceiveError::Closed);
            }
            if root.status != ReceiveStatus::Ready || !root.armed {
                return Err(ReceiveError::Unavailable);
            }
            root.incoming.acquire(lease)
        }
    }

    pub(super) fn release_incoming_after_cleanup(&self, exact: IncomingLease) -> bool {
        self.shared
            .parent
            .as_ref()
            .unwrap_or(self)
            .header()
            .incoming
            .release(exact)
    }

    #[cfg(test)]
    pub(super) fn lifetime_observer(&self) -> impl Fn() -> bool + Send + 'static {
        let weak = Arc::downgrade(&self.shared);
        move || weak.strong_count() > 0
    }

    pub(super) fn header(&self) -> MutexGuard<'_, Header> {
        self.shared
            .header
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn invalidate_pending(&self) -> Result<ReceiveEpoch, ReceiveError> {
        let mut header = self.header();
        if header.is_closing() {
            return Err(ReceiveError::Closed);
        }
        header.armed = false;
        header.cancel_open(ReceiveError::Cancelled);
        header.cancel_text(ClipboardError::Cancelled);
        let Some(next) = header.epoch.0.checked_add(1) else {
            header.request_close(Some(ReceiveError::ResourceLimit));
            return Err(ReceiveError::ResourceLimit);
        };
        header.epoch = ReceiveEpoch(next);
        Ok(header.epoch)
    }

    pub(super) fn decide(&self, id: OfferId, decision: DropDecision) -> Result<(), ReceiveError> {
        let parent = self.shared.parent.as_ref().map(Self::header);
        if parent.as_ref().is_some_and(|header| header.is_closing()) {
            return Err(ReceiveError::Closed);
        }
        let mut header = self.header();
        if header.is_closing() {
            return Err(ReceiveError::Closed);
        }
        let decision_busy = header.decision.is_some();
        let epoch = header.epoch;
        let armed = header.armed;
        let offer = header.offer.as_mut().ok_or(ReceiveError::StaleOffer)?;
        if offer.id != id || offer.epoch != epoch || !armed || offer.gate != AttemptGate::Open {
            return Err(ReceiveError::StaleOffer);
        }
        if decision_busy {
            return Err(ReceiveError::Busy);
        }
        if offer
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            offer.gate = AttemptGate::Cancelled(ReceiveError::Timeout);
            offer.ready = false;
            return Err(ReceiveError::Timeout);
        }
        if decision == DropDecision::Accept && (!offer.ready || !offer.selected_copy) {
            return Err(ReceiveError::InvalidPhase);
        }
        header.decision = Some(DecisionSlot::Queued(id, decision));
        Ok(())
    }

    pub(super) fn take_decision(&self) -> Option<(OfferId, DropDecision)> {
        let parent = self.shared.parent.as_ref().map(Self::header);
        if parent.as_ref().is_some_and(|header| header.is_closing()) {
            return None;
        }
        let mut header = self.header();
        let Some(DecisionSlot::Queued(id, decision)) = header.decision else {
            return None;
        };
        if header.is_closing() || !header.armed {
            return None;
        }
        let epoch = header.epoch;
        let offer = header.offer.as_mut()?;
        if offer.id != id || offer.epoch != epoch || offer.gate != AttemptGate::Open {
            return None;
        }
        if offer
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            offer.gate = AttemptGate::Cancelled(ReceiveError::Timeout);
            offer.ready = false;
            return None;
        }
        header.decision = Some(DecisionSlot::Taken);
        Some((id, decision))
    }

    pub(super) fn claim(
        &self,
        id: OfferId,
        epoch: ReceiveEpoch,
        now: Instant,
    ) -> Result<(), ReceiveError> {
        // The owner cut and this claim arbitrate under the same parent header.
        // Neither guard survives into the caller's protocol request.
        let parent = self.shared.parent.as_ref().map(Self::header);
        if parent.as_ref().is_some_and(|header| header.is_closing()) {
            return Err(ReceiveError::StaleOffer);
        }
        let mut header = self.header();
        let now = now.max(Instant::now());
        if header.is_closing() || !header.armed || header.epoch != epoch {
            return Err(ReceiveError::StaleOffer);
        }
        let offer = header.offer.as_mut().ok_or(ReceiveError::StaleOffer)?;
        if offer.id != id || offer.epoch != epoch || offer.gate != AttemptGate::Open {
            return Err(ReceiveError::StaleOffer);
        }
        if offer.deadline.is_some_and(|deadline| now >= deadline) {
            offer.gate = AttemptGate::Cancelled(ReceiveError::Timeout);
            offer.ready = false;
            return Err(ReceiveError::Timeout);
        }
        if !offer.ready || !offer.selected_copy {
            return Err(ReceiveError::InvalidPhase);
        }
        offer.gate = AttemptGate::Claimed;
        offer.ready = false;
        header.decision = None;
        Ok(())
    }

    pub(super) fn cancel(&self, id: OfferId) -> Result<(), ReceiveError> {
        let mut header = self.header();
        if header
            .offer
            .is_some_and(|offer| offer.id == id && offer.gate == AttemptGate::Open)
        {
            header.cancel_open(ReceiveError::Cancelled);
        }
        Ok(())
    }

    pub(super) fn close(&self) -> Result<(), ReceiveError> {
        self.header().request_close(None);
        Ok(())
    }

    pub(super) fn status(&self) -> ReceiveStatus {
        self.header().status
    }

    pub(super) fn poll(&self) -> Vec<ReceiveEvent> {
        let events = {
            let mut mailbox = self
                .shared
                .mailbox
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            mailbox.poll()
        };
        for event in &events {
            if let ReceiveEvent::DropEnded { id, .. } = event {
                self.acknowledge_ended(*id);
            }
        }
        events
    }

    pub(super) fn acknowledge_ended(&self, id: OfferId) {
        let mut header = self.header();
        if let Some(offer) = header.offer.as_mut() {
            if offer.id == id && matches!(offer.gate, AttemptGate::Ended { .. }) {
                offer.gate = AttemptGate::Ended { drained: true };
            }
        }
    }

    pub(super) fn remaining_reliable(&self) -> usize {
        self.shared
            .mailbox
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remaining_reliable()
    }

    pub(super) fn enqueue(&self, event: ReceiveEvent) -> Result<(), ReceiveEvent> {
        self.shared
            .mailbox
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event)
    }

    pub(super) fn retire_ready(&self, id: OfferId) -> Option<ReceiveEvent> {
        self.shared
            .mailbox
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retire_ready(id)
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
