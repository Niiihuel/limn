//! Shared admission and the single receive-only mailbox.
//! Header guards never own paths, protocol objects, or user callbacks.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use super::{
    events::ReceiveMailbox, DropDecision, OfferId, ReceiveEpoch, ReceiveEvent, ReceiveStatus,
};
use crate::ReceiveError;

#[derive(Clone)]
pub(super) struct ReceiveControl {
    shared: Arc<Shared>,
}

struct Shared {
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

    pub(super) fn request_close(&mut self, cause: Option<ReceiveError>) {
        if !matches!(self.status, ReceiveStatus::Closed(_)) {
            self.armed = false;
            self.status = ReceiveStatus::Closing;
            self.close_cause = self.close_cause.or(cause);
            self.cancel_open(cause.unwrap_or(ReceiveError::Cancelled));
        }
    }
}

impl ReceiveControl {
    pub(super) fn new(epoch: ReceiveEpoch) -> Result<Self, ReceiveError> {
        Ok(Self {
            shared: Arc::new(Shared {
                header: Mutex::new(Header {
                    epoch,
                    armed: false,
                    status: ReceiveStatus::Starting,
                    close_cause: None,
                    offer: None,
                    decision: None,
                }),
                mailbox: Mutex::new(ReceiveMailbox::new()),
            }),
        })
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
        let Some(next) = header.epoch.0.checked_add(1) else {
            header.request_close(Some(ReceiveError::ResourceLimit));
            return Err(ReceiveError::ResourceLimit);
        };
        header.epoch = ReceiveEpoch(next);
        Ok(header.epoch)
    }

    pub(super) fn decide(&self, id: OfferId, decision: DropDecision) -> Result<(), ReceiveError> {
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
