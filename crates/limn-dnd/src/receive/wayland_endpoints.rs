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
        // A terminal's SDK offer may be gone while the coordinator still holds
        // its undrained Ended. Only that coordinator's exact acknowledgement
        // releases the lease, unless the registration is fully detached below.
        if let Some(lease) = self.lease {
            let retained = self
                .endpoint_for_registration(lease.registration)
                .is_some_and(|index| {
                    let endpoint = &self.endpoints[index];
                    endpoint.retiring || endpoint.coordinator.offer_phase(lease.id).is_some()
                });
            if !retained {
                self.lease = None;
            }
        }
        Ok(())
    }

    pub(super) fn finish_retirements(&mut self) -> Result<(), ReceiveError> {
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
                    .id
                    .is_some_and(|id| self.endpoints[index].coordinator.offer_phase(id).is_some())
            }) || self
                .pipe
                .as_ref()
                .is_some_and(|(id, _)| self.endpoints[index].coordinator.offer_phase(*id).is_some())
            {
                return Err(ReceiveError::InvalidPhase);
            }
            // Caller removed the runtime watcher before reaching this boundary.
            // A pending callback keeps only its identity tombstone, not authority.
            self.pool.retire_surface(owner, registration)?;
            if self
                .lease
                .is_some_and(|lease| lease.registration == Some(registration))
            {
                self.lease = None;
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
