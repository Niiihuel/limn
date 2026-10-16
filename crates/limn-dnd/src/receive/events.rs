//! The outgoing event boundary shared by the receive-only coordinator and UI.

use std::collections::VecDeque;

use super::ReceiveEvent;

pub(super) struct ReceiveMailbox {
    reliable: VecDeque<ReceiveEvent>,
    hover: Option<ReceiveEvent>,
}

impl ReceiveMailbox {
    pub(super) fn new() -> Self {
        Self {
            reliable: VecDeque::with_capacity(5),
            hover: None,
        }
    }

    /// Return an undeliverable event intact, so callers retain terminal/payload ownership.
    pub(super) fn push(&mut self, event: ReceiveEvent) -> Result<(), ReceiveEvent> {
        if matches!(
            &event,
            ReceiveEvent::Hovered { .. } | ReceiveEvent::Left { .. }
        ) {
            self.hover = Some(event);
            return Ok(());
        }
        if self.reliable.len() == 5 {
            return Err(event);
        }
        if let ReceiveEvent::DropStarted { id, .. } | ReceiveEvent::DropEnded { id, .. } = &event {
            if let Some(
                ReceiveEvent::Hovered { id: hover, .. } | ReceiveEvent::Left { id: hover },
            ) = &self.hover
            {
                if hover == id {
                    self.hover = None;
                }
            }
        }
        self.reliable.push_back(event);
        Ok(())
    }

    pub(super) fn poll(&mut self) -> Vec<ReceiveEvent> {
        let mut events: Vec<_> = self.reliable.drain(..).collect();
        if let Some(hover) = self.hover.take() {
            events.push(hover);
        }
        events
    }

    pub(super) fn remaining_reliable(&self) -> usize {
        5 - self.reliable.len()
    }

    /// Transfer a withdrawn payload to the caller for destruction outside a shared lock.
    pub(super) fn retire_ready(&mut self, id: super::OfferId) -> Option<ReceiveEvent> {
        let index = self.reliable.iter().position(
            |event| matches!(event, ReceiveEvent::DropReady { id: actual, .. } if *actual == id),
        )?;
        self.reliable.remove(index)
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
