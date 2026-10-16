//! Surface registrations reuse the same coordinator and shared protocol queue.

use std::panic::{catch_unwind, AssertUnwindSafe};

use super::*;
use crate::receive::coordinator::Phase;

pub(super) struct Endpoint {
    pub(super) registration: Option<SurfaceId>,
    pub(super) surface: Option<ObjectId>,
    pub(super) control: ReceiveControl,
    pub(super) coordinator: Coordinator,
    pub(super) ready_deadline: Option<Instant>,
    pub(super) first_ready: bool,
    pub(super) retiring: bool,
}

#[derive(Clone, Copy)]
pub(super) struct DropLease {
    pub(super) registration: Option<SurfaceId>,
    pub(super) id: OfferId,
}

impl State {
    fn drop_lease(&self) -> Option<DropLease> {
        match self.control.incoming()? {
            IncomingLease::Drop { registration, id } => Some(DropLease { registration, id }),
            IncomingLease::Text(_) => None,
        }
    }

    fn release_drop_lease(&mut self, lease: DropLease) {
        if self
            .drop_reader
            .as_ref()
            .is_some_and(|(id, _)| *id != lease.id)
        {
            return;
        }
        if self
            .control
            .release_incoming_after_cleanup(IncomingLease::Drop {
                registration: lease.registration,
                id: lease.id,
            })
        {
            self.drop_reader = None;
        }
    }

    pub(super) fn release_unwatched_drop(&mut self) {
        let Some(lease) = self.drop_lease() else {
            return;
        };
        let Some(endpoint) = self.endpoint_for_registration(lease.registration) else {
            return;
        };
        if self.endpoints[endpoint].retiring
            || self.endpoints[endpoint]
                .coordinator
                .offer_phase(lease.id)
                .is_some()
            || self
                .offers
                .iter()
                .any(|offer| offer.drop_id() == Some(lease.id))
        {
            return;
        }
        // None proves this lease never created a reader. After EOF/cancel only
        // zero strong owners proves that no external runtime watcher remains.
        if self
            .drop_reader
            .as_ref()
            .is_none_or(|(id, reader)| *id == lease.id && reader.strong_count() == 0)
        {
            self.release_drop_lease(lease);
        }
    }

    pub(super) fn offer_phase(&self, id: OfferId) -> Option<Phase> {
        self.endpoints
            .iter()
            .find_map(|endpoint| endpoint.coordinator.offer_phase(id))
    }

    pub(super) fn endpoint_for_offer(&self, id: OfferId) -> Option<usize> {
        self.endpoints
            .iter()
            .position(|endpoint| endpoint.coordinator.offer_phase(id).is_some())
    }

    pub(super) fn endpoint_for_registration(
        &self,
        registration: Option<SurfaceId>,
    ) -> Option<usize> {
        self.endpoints
            .iter()
            .position(|endpoint| endpoint.registration == registration)
    }

    pub(super) fn notify(&self) {
        if let Some(wake) = self.lifecycle_wake.as_ref() {
            wake();
        } else if let Some(endpoint) = self.endpoints.first() {
            endpoint.coordinator.notify();
        }
    }

    pub(super) fn request_close(&self, cause: Option<ReceiveError>) {
        if let Some(owner) = self.owner {
            self.pool.close_owner(owner, cause);
        } else {
            self.control.header().request_close(cause);
        }
    }

    pub(super) fn intake_endpoints(&mut self) -> Result<(), ReceiveError> {
        let Some(owner) = self.owner else {
            return Ok(());
        };
        for seed in self.pool.take_pending_surfaces(owner)? {
            // Registration's unsafe admission retained the real surface owner.
            // Do not reassign the host surface's queue or userdata.
            let surface =
                unsafe { ObjectId::from_ptr(WlSurface::interface(), seed.key.surface as *mut _) };
            let retiring = surface.is_err();
            if retiring {
                seed.control
                    .header()
                    .request_close(Some(ReceiveError::Unavailable));
            }
            assert!(
                self.endpoints.len() < 4,
                "pool bounds all admitted surfaces"
            );
            self.endpoints.push(Endpoint {
                registration: Some(seed.id),
                surface: surface.ok(),
                coordinator: Coordinator::with_wake(
                    self.limits,
                    seed.control.clone(),
                    seed.wake,
                    Some(seed.ready_deadline),
                ),
                control: seed.control,
                ready_deadline: Some(seed.ready_deadline),
                first_ready: false,
                retiring,
            });
        }
        Ok(())
    }

    pub(super) fn reconcile_endpoints(&mut self, now: Instant) -> Result<(), ReceiveError> {
        for endpoint in &mut self.endpoints {
            if !endpoint.first_ready
                && endpoint
                    .ready_deadline
                    .is_some_and(|deadline| now >= deadline)
            {
                endpoint
                    .control
                    .header()
                    .request_close(Some(ReceiveError::Timeout));
            }
            endpoint.coordinator.reconcile(now)?;
            if endpoint.registration.is_some() && endpoint.control.header().is_closing() {
                endpoint.retiring = true;
            }
        }
        Ok(())
    }

    pub(super) fn finish_retirements(&mut self) -> Result<(), ReceiveError> {
        self.finish_clipboard_retirements()?;
        // The caller has removed the runtime watcher. Coordinator acknowledgement
        // alone cannot prove that its Arc<OwnedFd> has been dropped.
        if let Some(lease) = self.drop_lease() {
            let retained = self
                .endpoint_for_registration(lease.registration)
                .is_some_and(|index| {
                    let endpoint = &self.endpoints[index];
                    endpoint.retiring || endpoint.coordinator.offer_phase(lease.id).is_some()
                });
            if !retained {
                self.release_drop_lease(lease);
            }
        }
        let Some(owner) = self.owner else {
            return Ok(());
        };
        let mut index = 0;
        while index < self.endpoints.len() {
            if !self.endpoints[index].retiring {
                index += 1;
                continue;
            }
            let registration = self.endpoints[index]
                .registration
                .expect("explicit endpoint registration");
            if self.offers.iter().any(|offer| {
                offer
                    .drop_id()
                    .is_some_and(|id| self.endpoints[index].coordinator.offer_phase(id).is_some())
            }) || self.pipe.as_ref().is_some_and(|(tag, _)| match tag {
                ReadTag::Drop(id) => self.endpoints[index].coordinator.offer_phase(*id).is_some(),
                ReadTag::Text(_) => false,
            }) {
                return Err(ReceiveError::InvalidPhase);
            }
            // Caller removed the runtime watcher before reaching this boundary.
            // A pending callback keeps only its identity tombstone, not authority.
            self.pool.retire_surface(owner, registration)?;
            if let Some(lease) = self
                .drop_lease()
                .filter(|lease| lease.registration == Some(registration))
            {
                self.release_drop_lease(lease);
            }
            let mut endpoint = self.endpoints.remove(index);
            // Preserve the next surviving registration when vector indices shift.
            if index < self.next_arm {
                self.next_arm -= 1;
            }
            self.next_arm = if self.endpoints.is_empty() {
                0
            } else {
                self.next_arm % self.endpoints.len()
            };
            let wake = endpoint.coordinator.take_wake();
            drop(endpoint);
            // The unique wake transferred from the seed observes Detached. Its
            // invocation and final destructor are both isolated outside guards.
            if let Err(panic) = catch_unwind(AssertUnwindSafe(wake)) {
                if let Err(secondary) = catch_unwind(AssertUnwindSafe(move || drop(panic))) {
                    std::mem::forget(secondary);
                }
            }
        }
        Ok(())
    }
}
