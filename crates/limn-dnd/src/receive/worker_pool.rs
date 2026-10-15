//! Shared worker reservations and a single off-caller-thread reaper.
//!
//! A slot owns the foreign-window owner before spawning. A cleanup signal only
//! makes its handle eligible for join: it never releases the surface or owner.

use std::{
    any::Any,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex, MutexGuard, OnceLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::{control::ReceiveControl, ReceiveEpoch, ReceiveStatus};
use crate::ReceiveError;

const MAX_WORKERS: usize = 4;
const MAX_SURFACES: usize = 4;
const MAX_REGISTRIES: usize = 64;

pub(super) type Wake = Box<dyn Fn() + Send>;
type Owner = Arc<dyn Any + Send + Sync>;
type Payload = Box<dyn Any + Send>;
type Job = Box<dyn FnOnce(ExitSignal) -> WorkerExit + Send>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceKey {
    pub(crate) display: usize,
    pub(crate) surface: usize,
}

pub(crate) struct WorkerPool {
    shared: Arc<Shared>,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    registries: AtomicUsize,
    #[cfg(test)]
    fail_reaper: AtomicBool,
    #[cfg(test)]
    fail_worker: AtomicBool,
}

struct State {
    slots: [Option<Slot>; MAX_WORKERS],
    surfaces: [Option<SurfaceSlot>; MAX_SURFACES],
    next_id: u64,
    reaper_started: bool,
    failed: bool,
    pool_dropped: bool,
}

struct Slot {
    display: usize,
    // None preserves the old per-surface exclusion of standalone and legacy.
    endpoint_timeout: Option<Duration>,
    id: u64,
    kind: SlotKind,
}

enum SlotKind {
    Legacy,
    Owned(OwnedSlot),
}

struct OwnedSlot {
    owner: Option<Owner>,
    control: ReceiveControl,
    job: Option<Job>,
    handle: Option<JoinHandle<Option<WorkerExit>>>,
    handoff: bool,
    cleaned: bool,
    phase: Phase,
    // Unknown cleanup never returns a slot, even after the thread has exited.
    stranded: Option<WorkerExit>,
    panic_payload: Option<Payload>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Reserved,
    Running,
    Rejected,
    Joining,
    Unconfirmed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Token {
    index: usize,
    id: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct OwnerId(Token);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct SurfaceId {
    index: usize,
    id: u64,
}

struct SurfaceSlot {
    id: u64,
    worker: Token,
    key: SurfaceKey,
    kind: SurfaceKind,
}

enum SurfaceKind {
    CoupledToWorker,
    ExplicitEndpoint {
        window: Option<Owner>,
        control: ReceiveControl,
        detached: Arc<AtomicBool>,
        seed: Option<PendingSurface>,
        retiring: bool,
    },
}

pub(super) struct PendingSurface {
    pub(super) id: SurfaceId,
    pub(super) key: SurfaceKey,
    pub(super) control: ReceiveControl,
    pub(super) wake: Wake,
    pub(super) ready_deadline: Instant,
}

pub(super) struct SurfaceLease {
    // Tests correlate this observation with the production slot/seed identity.
    #[cfg(test)]
    id: SurfaceId,
    control: ReceiveControl,
    detached: Arc<AtomicBool>,
}

pub(super) struct Reservation {
    shared: Arc<Shared>,
    token: Option<Token>,
}

/// The legacy caller drops this only after its existing synchronous join.
pub(crate) struct LegacyReservation {
    shared: Arc<Shared>,
    token: Token,
}

#[derive(Clone)]
pub(super) struct ExitSignal {
    shared: Arc<Shared>,
    token: Token,
}

pub(super) struct WorkerExit {
    pub(super) wake: Wake,
    pub(super) cause: Option<ReceiveError>,
    pub(super) payload: Option<Payload>,
}

enum Reap {
    Join(Token, JoinHandle<Option<WorkerExit>>),
    Rejected(Token, Option<Job>),
}

impl WorkerPool {
    pub(crate) fn global() -> &'static Arc<Self> {
        static POOL: OnceLock<Arc<WorkerPool>> = OnceLock::new();
        POOL.get_or_init(Self::new)
    }

    fn new() -> Arc<Self> {
        Arc::new(Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    slots: std::array::from_fn(|_| None),
                    surfaces: std::array::from_fn(|_| None),
                    next_id: 0,
                    reaper_started: false,
                    failed: false,
                    pool_dropped: false,
                }),
                changed: Condvar::new(),
                registries: AtomicUsize::new(0),
                #[cfg(test)]
                fail_reaper: AtomicBool::new(false),
                #[cfg(test)]
                fail_worker: AtomicBool::new(false),
            }),
        })
    }

    pub(super) fn reserve_owned(
        self: &Arc<Self>,
        key: SurfaceKey,
        owner: Owner,
        control: ReceiveControl,
    ) -> Result<Reservation, ReceiveError> {
        let mut state = self.shared.lock();
        if state.failed {
            return Err(ReceiveError::Unavailable);
        }
        let (token, surface) = state.available_coupled(key)?;
        self.shared.start_reaper(&mut state)?;
        state.slots[token.index] = Some(Slot {
            display: key.display,
            endpoint_timeout: None,
            id: token.id,
            kind: SlotKind::Owned(OwnedSlot::new(owner, control)),
        });
        state.surfaces[surface.index] = Some(SurfaceSlot {
            id: surface.id,
            worker: token,
            key,
            kind: SurfaceKind::CoupledToWorker,
        });
        Ok(Reservation {
            shared: self.shared.clone(),
            token: Some(token),
        })
    }

    pub(crate) fn reserve_legacy(
        self: &Arc<Self>,
        key: SurfaceKey,
    ) -> Result<LegacyReservation, ReceiveError> {
        let mut state = self.shared.lock();
        let (token, surface) = state.available_coupled(key)?;
        state.slots[token.index] = Some(Slot {
            display: key.display,
            endpoint_timeout: None,
            id: token.id,
            kind: SlotKind::Legacy,
        });
        state.surfaces[surface.index] = Some(SurfaceSlot {
            id: surface.id,
            worker: token,
            key,
            kind: SurfaceKind::CoupledToWorker,
        });
        Ok(LegacyReservation {
            shared: self.shared.clone(),
            token,
        })
    }

    pub(super) fn reserve_owner(
        self: &Arc<Self>,
        display: usize,
        owner: Owner,
        control: ReceiveControl,
        endpoint_timeout: Duration,
    ) -> Result<(OwnerId, Reservation), ReceiveError> {
        if endpoint_timeout.is_zero()
            || endpoint_timeout > Duration::from_secs(60)
            || Instant::now().checked_add(endpoint_timeout).is_none()
        {
            return Err(ReceiveError::InvalidLimits);
        }
        if display == 0 {
            return Err(ReceiveError::Unavailable);
        }
        let mut state = self.shared.lock();
        if state.failed {
            return Err(ReceiveError::Unavailable);
        }
        if state
            .slots
            .iter()
            .flatten()
            .any(|slot| slot.display == display)
        {
            return Err(ReceiveError::Busy);
        }
        let token = state.available_worker()?;
        self.shared.start_reaper(&mut state)?;
        state.slots[token.index] = Some(Slot {
            display,
            endpoint_timeout: Some(endpoint_timeout),
            id: token.id,
            kind: SlotKind::Owned(OwnedSlot::new(owner, control)),
        });
        Ok((
            OwnerId(token),
            Reservation {
                shared: self.shared.clone(),
                token: Some(token),
            },
        ))
    }

    pub(super) fn reserve_surface(
        self: &Arc<Self>,
        owner: OwnerId,
        key: SurfaceKey,
        window: Owner,
        wake: Wake,
    ) -> Result<SurfaceLease, ReceiveError> {
        // Clone under the pool lock, then acquire the authority before rechecking
        // the pool. No path takes these locks in the opposite order.
        let parent = self.shared.control(owner.0).ok_or(ReceiveError::Closed)?;
        let header = parent.header();
        if header.is_closing() {
            return Err(ReceiveError::Closed);
        }
        let mut state = self.shared.lock();
        if state.failed {
            return Err(ReceiveError::Unavailable);
        }
        let (display, timeout) = state.explicit_owner(owner.0)?;
        if key.display == 0 || key.display != display {
            return Err(ReceiveError::Unavailable);
        }
        let ready_deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(ReceiveError::InvalidLimits)?;
        let id = state.available_surface(key)?;
        let control = ReceiveControl::new_for_owner(ReceiveEpoch(1), parent.clone())?;
        let detached = Arc::new(AtomicBool::new(false));
        state.surfaces[id.index] = Some(SurfaceSlot {
            id: id.id,
            worker: owner.0,
            key,
            kind: SurfaceKind::ExplicitEndpoint {
                window: Some(window),
                control: control.clone(),
                detached: detached.clone(),
                seed: Some(PendingSurface {
                    id,
                    key,
                    control: control.clone(),
                    wake,
                    ready_deadline,
                }),
                retiring: false,
            },
        });
        Ok(SurfaceLease {
            #[cfg(test)]
            id,
            control,
            detached,
        })
    }

    pub(super) fn take_pending_surfaces(
        &self,
        owner: OwnerId,
    ) -> Result<Vec<PendingSurface>, ReceiveError> {
        let mut state = self.shared.lock();
        state.explicit_owner(owner.0)?;
        let mut pending = Vec::new();
        for surface in state.surfaces.iter_mut().flatten() {
            if surface.worker == owner.0 {
                if let SurfaceKind::ExplicitEndpoint {
                    seed,
                    retiring: false,
                    ..
                } = &mut surface.kind
                {
                    pending.extend(seed.take());
                }
            }
        }
        Ok(pending)
    }

    /// The worker has already removed this registration's protocol resources.
    pub(super) fn retire_surface(
        &self,
        owner: OwnerId,
        surface: SurfaceId,
    ) -> Result<(), ReceiveError> {
        self.shared.retire_surface(owner.0, surface)
    }

    pub(super) fn close_owner(&self, owner: OwnerId, cause: Option<ReceiveError>) {
        self.shared.request_close(owner.0, cause);
    }

    /// Spend immediately before get_registry. Neither cleanup nor retry refunds it.
    pub(crate) fn consume_registry(&self) -> Result<(), ReceiveError> {
        self.shared
            .registries
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                (used < MAX_REGISTRIES).then_some(used + 1)
            })
            .map(|_| ())
            .map_err(|_| ReceiveError::ResourceLimit)
    }

    #[cfg(test)]
    pub(crate) fn isolated() -> Arc<Self> {
        Self::new()
    }

    #[cfg(test)]
    fn fail_reaper_start(&self) {
        self.shared.fail_reaper.store(true, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn fail_next_worker_spawn(&self) {
        self.shared.fail_worker.store(true, Ordering::Relaxed);
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.shared.lock().pool_dropped = true;
        self.shared.changed.notify_all();
    }
}

impl State {
    fn next_id(&mut self) -> Result<u64, ReceiveError> {
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ReceiveError::ResourceLimit)?;
        Ok(self.next_id)
    }

    fn available_worker(&mut self) -> Result<Token, ReceiveError> {
        let index = self
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(ReceiveError::Busy)?;
        Ok(Token {
            index,
            id: self.next_id()?,
        })
    }

    fn available_surface(&mut self, key: SurfaceKey) -> Result<SurfaceId, ReceiveError> {
        if self
            .surfaces
            .iter()
            .flatten()
            .any(|surface| surface.key == key)
        {
            return Err(ReceiveError::Busy);
        }
        let index = self
            .surfaces
            .iter()
            .position(Option::is_none)
            .ok_or(ReceiveError::Busy)?;
        Ok(SurfaceId {
            index,
            id: self.next_id()?,
        })
    }

    fn available_coupled(&mut self, key: SurfaceKey) -> Result<(Token, SurfaceId), ReceiveError> {
        if self
            .slots
            .iter()
            .flatten()
            .any(|slot| slot.display == key.display && slot.endpoint_timeout.is_some())
        {
            return Err(ReceiveError::Busy);
        }
        let surface = self.available_surface(key)?;
        let token = self.available_worker()?;
        Ok((token, surface))
    }

    fn explicit_owner(&self, token: Token) -> Result<(usize, Duration), ReceiveError> {
        let slot = self.slots[token.index]
            .as_ref()
            .filter(|slot| slot.id == token.id)
            .ok_or(ReceiveError::Closed)?;
        let timeout = slot.endpoint_timeout.ok_or(ReceiveError::Closed)?;
        Ok((slot.display, timeout))
    }

    fn owned(&mut self, token: Token) -> Option<&mut OwnedSlot> {
        let slot = self.slots[token.index]
            .as_mut()
            .filter(|slot| slot.id == token.id)?;
        match &mut slot.kind {
            SlotKind::Owned(owned) => Some(owned),
            SlotKind::Legacy => None,
        }
    }
}

impl OwnedSlot {
    fn new(owner: Owner, control: ReceiveControl) -> Self {
        Self {
            owner: Some(owner),
            control,
            job: None,
            handle: None,
            handoff: false,
            cleaned: false,
            phase: Phase::Reserved,
            stranded: None,
            panic_payload: None,
        }
    }
}

impl SurfaceLease {
    #[cfg(test)]
    pub(super) fn id(&self) -> SurfaceId {
        self.id
    }

    pub(super) fn control(&self) -> ReceiveControl {
        self.control.clone()
    }

    pub(super) fn request_retire(&self) {
        self.control.header().request_close(None);
    }

    pub(super) fn is_detached(&self) -> bool {
        self.detached.load(Ordering::Acquire)
    }
}

impl Drop for SurfaceLease {
    fn drop(&mut self) {
        self.request_retire();
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn start_reaper(self: &Arc<Self>, state: &mut State) -> Result<(), ReceiveError> {
        if !state.reaper_started {
            #[cfg(test)]
            if self.fail_reaper.load(Ordering::Relaxed) {
                return Err(ReceiveError::Unavailable);
            }
            let shared = self.clone();
            let handle = thread::Builder::new()
                .name("limn-dnd-reaper".into())
                .spawn(move || reaper_thread(shared))
                .map_err(|_| ReceiveError::Unavailable)?;
            // This one process service is never synchronously joined by its caller.
            drop(handle);
            state.reaper_started = true;
        }
        Ok(())
    }

    fn control(&self, token: Token) -> Option<ReceiveControl> {
        self.lock().owned(token).map(|owned| owned.control.clone())
    }

    fn request_close(&self, token: Token, cause: Option<ReceiveError>) {
        if let Some(control) = self.control(token) {
            let children = {
                let mut parent = control.header();
                parent.request_close(cause);
                let children: Vec<ReceiveControl> = {
                    let state = self.lock();
                    state
                        .surfaces
                        .iter()
                        .flatten()
                        .filter_map(|surface| {
                            if surface.worker != token {
                                return None;
                            }
                            match &surface.kind {
                                SurfaceKind::ExplicitEndpoint { control, .. } => {
                                    Some(control.clone())
                                }
                                SurfaceKind::CoupledToWorker => None,
                            }
                        })
                        .collect()
                };
                for child in &children {
                    child.header().request_close(cause);
                }
                drop(parent);
                children
            };
            // A concurrent retirement may have removed the slot's control clone.
            // Any last mailbox/payload destructor must run after the parent guard.
            drop(children);
        }
    }

    fn retire_surface(&self, token: Token, id: SurfaceId) -> Result<(), ReceiveError> {
        let control = {
            let state = self.lock();
            let surface = state.surfaces[id.index]
                .as_ref()
                .filter(|surface| surface.id == id.id && surface.worker == token)
                .ok_or(ReceiveError::Closed)?;
            match &surface.kind {
                SurfaceKind::ExplicitEndpoint { control, .. } => control.clone(),
                SurfaceKind::CoupledToWorker => return Err(ReceiveError::InvalidPhase),
            }
        };
        control.header().request_close(None);
        let (window, seed, detached) = {
            let mut state = self.lock();
            let surface = state.surfaces[id.index]
                .as_mut()
                .filter(|surface| surface.id == id.id && surface.worker == token)
                .ok_or(ReceiveError::Closed)?;
            let SurfaceKind::ExplicitEndpoint {
                window,
                seed,
                retiring,
                detached,
                ..
            } = &mut surface.kind
            else {
                return Err(ReceiveError::InvalidPhase);
            };
            if *retiring {
                return Err(ReceiveError::Closed);
            }
            *retiring = true;
            (window.take(), seed.take(), detached.clone())
        };
        // The occupied Retiring slot survives any destructor failure here.
        drop(window);
        // If State never took this seed, the pool still owns its sole wake.
        let wake = seed.map(|seed| seed.wake);
        let retired = {
            let mut state = self.lock();
            let surface = &mut state.surfaces[id.index];
            assert!(surface
                .as_ref()
                .is_some_and(|surface| surface.id == id.id && surface.worker == token));
            surface.take()
        };
        drop(retired);
        detached.store(true, Ordering::Release);
        self.changed.notify_all();
        if let Some(wake) = wake {
            // Invocation and the box's final Drop both happen after Detached.
            if let Err(panic) = catch_unwind(AssertUnwindSafe(wake)) {
                discard_panic(panic);
            }
        }
        Ok(())
    }

    fn reject(&self, token: Token, cause: Option<ReceiveError>) {
        self.request_close(token, cause);
        let mut state = self.lock();
        let failed = state.failed;
        if let Some(owned) = state.owned(token) {
            owned.phase = if failed {
                Phase::Unconfirmed
            } else {
                Phase::Rejected
            };
        }
        drop(state);
        self.changed.notify_all();
    }

    fn take_job(&self, token: Token) -> Option<Job> {
        let mut state = self.lock();
        loop {
            let failed = state.failed;
            let owned = state
                .owned(token)
                .expect("reserved worker slot survives handoff");
            if failed {
                owned.phase = Phase::Unconfirmed;
                return None;
            }
            if owned.handoff {
                return Some(
                    owned
                        .job
                        .take()
                        .expect("one worker consumes its registered job"),
                );
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    fn next_reap(&self) -> Option<Reap> {
        let mut state = self.lock();
        loop {
            for (index, slot) in state.slots.iter_mut().enumerate() {
                let Some(slot) = slot else { continue };
                let token = Token { index, id: slot.id };
                let SlotKind::Owned(owned) = &mut slot.kind else {
                    continue;
                };
                if owned.phase == Phase::Rejected {
                    owned.phase = Phase::Joining;
                    return Some(Reap::Rejected(token, owned.job.take()));
                }
                if owned.phase == Phase::Running && owned.cleaned && owned.handoff {
                    if let Some(handle) = owned.handle.take() {
                        owned.phase = Phase::Joining;
                        return Some(Reap::Join(token, handle));
                    }
                }
            }
            if state.pool_dropped && state.slots.iter().all(Option::is_none) {
                return None;
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    fn release(&self, token: Token) -> ReceiveControl {
        // Unclaimed seeds (including a rejected spawn's wakes) are admitted
        // resources too: only the worker/reaper retires their final ownership.
        let endpoints: [Option<SurfaceId>; MAX_SURFACES] = {
            let state = self.lock();
            std::array::from_fn(|index| {
                let surface = state.surfaces[index].as_ref()?;
                (surface.worker == token
                    && matches!(surface.kind, SurfaceKind::ExplicitEndpoint { .. }))
                .then_some(SurfaceId {
                    index,
                    id: surface.id,
                })
            })
        };
        for id in endpoints.into_iter().flatten() {
            self.retire_surface(token, id)
                .expect("confirmed cleanup retires each endpoint once");
        }
        // Keep the slot occupied while the owner's final destructor runs.
        let (owner, control) = {
            let mut state = self.lock();
            let owned = state.owned(token).expect("joining retains its worker slot");
            (owned.owner.take(), owned.control.clone())
        };
        drop(owner);
        let (retired, coupled) = {
            let mut state = self.lock();
            let slot = &mut state.slots[token.index];
            assert!(slot.as_ref().is_some_and(|slot| slot.id == token.id));
            let retired = slot.take();
            let coupled: [Option<SurfaceSlot>; MAX_SURFACES] = std::array::from_fn(|index| {
                let surface = &mut state.surfaces[index];
                if surface
                    .as_ref()
                    .is_some_and(|surface| surface.worker == token)
                {
                    surface.take()
                } else {
                    None
                }
            });
            (retired, coupled)
        };
        drop(retired);
        drop(coupled);
        self.changed.notify_all();
        control
    }

    fn unconfirmed(&self, token: Token, outcome: Option<WorkerExit>, panic: Option<Payload>) {
        self.request_close(token, Some(ReceiveError::Io));
        if let Some(owned) = self.lock().owned(token) {
            owned.phase = Phase::Unconfirmed;
            owned.stranded = outcome;
            owned.panic_payload = panic;
        }
    }

    fn reaper_failed(&self) {
        let tokens: [Option<Token>; MAX_WORKERS] = {
            let mut state = self.lock();
            state.failed = true;
            std::array::from_fn(|index| {
                let slot = state.slots[index].as_mut()?;
                let SlotKind::Owned(owned) = &mut slot.kind else {
                    return None;
                };
                owned.phase = Phase::Unconfirmed;
                Some(Token { index, id: slot.id })
            })
        };
        self.changed.notify_all();
        for token in tokens.into_iter().flatten() {
            self.request_close(token, Some(ReceiveError::Io));
        }
    }
}

impl Reservation {
    pub(super) fn spawn(
        mut self,
        worker: impl FnOnce(ExitSignal) -> WorkerExit + Send + 'static,
    ) -> Result<(), ReceiveError> {
        let token = self.token.take().expect("reservation is consumed once");
        let failed = {
            let mut state = self.shared.lock();
            let failed = state.failed;
            let owned = state.owned(token).expect("reserved slot");
            // Store first: a failed service cannot retire this captured job in UI.
            owned.job = Some(Box::new(worker));
            if failed {
                owned.phase = Phase::Unconfirmed;
            }
            failed
        };
        if failed {
            self.shared
                .request_close(token, Some(ReceiveError::Unavailable));
            return Err(ReceiveError::Unavailable);
        }
        #[cfg(test)]
        let fail = self.shared.fail_worker.swap(false, Ordering::Relaxed);
        #[cfg(not(test))]
        let fail = false;
        let handle = if fail {
            None
        } else {
            let shared = self.shared.clone();
            // A rejected OS spawn only drops this small Arc/token closure on the
            // caller. The actual job, wake and any payload stay in the slot.
            thread::Builder::new()
                .name("limn-dnd-receive".into())
                .spawn(move || worker_thread(shared, token))
                .ok()
        };
        let Some(handle) = handle else {
            self.shared.reject(token, Some(ReceiveError::Unavailable));
            return Err(ReceiveError::Unavailable);
        };
        let failed = {
            let mut state = self.shared.lock();
            let failed = state.failed;
            let owned = state.owned(token).expect("spawn retains its worker slot");
            owned.handle = Some(handle);
            owned.handoff = true;
            owned.phase = if failed {
                Phase::Unconfirmed
            } else {
                Phase::Running
            };
            failed
        };
        self.shared.changed.notify_all();
        if failed {
            self.shared
                .request_close(token, Some(ReceiveError::Unavailable));
            Err(ReceiveError::Unavailable)
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    fn exit_signal(&self) -> ExitSignal {
        ExitSignal {
            shared: self.shared.clone(),
            token: self.token.expect("unconsumed reservation"),
        }
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            self.shared.reject(token, None);
        }
    }
}

impl Drop for LegacyReservation {
    fn drop(&mut self) {
        let (retired, coupled) = {
            let mut state = self.shared.lock();
            let slot = &mut state.slots[self.token.index];
            let retired = if slot.as_ref().is_some_and(|slot| slot.id == self.token.id) {
                slot.take()
            } else {
                None
            };
            let coupled: [Option<SurfaceSlot>; MAX_SURFACES] = std::array::from_fn(|index| {
                let surface = &mut state.surfaces[index];
                if surface
                    .as_ref()
                    .is_some_and(|surface| surface.worker == self.token)
                {
                    surface.take()
                } else {
                    None
                }
            });
            (retired, coupled)
        };
        drop(retired);
        drop(coupled);
        self.shared.changed.notify_all();
    }
}

impl ExitSignal {
    pub(super) fn guest_cleaned(&self) {
        self.shared.request_close(self.token, None);
        if let Some(owned) = self.shared.lock().owned(self.token) {
            owned.cleaned = true;
        }
        self.shared.changed.notify_all();
    }
}

struct WorkerReturned(ExitSignal);

impl Drop for WorkerReturned {
    fn drop(&mut self) {
        let uncertain = {
            let mut state = self.0.shared.lock();
            state.owned(self.0.token).is_some_and(|owned| {
                let uncertain = thread::panicking() || !owned.cleaned;
                if uncertain && owned.phase != Phase::Joining {
                    owned.phase = Phase::Unconfirmed;
                }
                uncertain
            })
        };
        self.0
            .shared
            .request_close(self.0.token, uncertain.then_some(ReceiveError::Io));
        self.0.shared.changed.notify_all();
    }
}

fn worker_thread(shared: Arc<Shared>, token: Token) -> Option<WorkerExit> {
    let signal = ExitSignal {
        shared: shared.clone(),
        token,
    };
    let _returned = WorkerReturned(signal.clone());
    let job = shared.take_job(token)?;
    Some(job(signal))
}

fn reaper_thread(shared: Arc<Shared>) {
    if let Err(panic) = catch_unwind(AssertUnwindSafe(|| reap(&shared))) {
        shared.reaper_failed();
        // A destructor/SDK failure cannot return unconfirmed resources to UI.
        // Keep this one bounded registry alive; never create a replacement reaper.
        discard_panic(panic);
        std::mem::forget(shared);
    }
}

fn reap(shared: &Shared) {
    while let Some(work) = shared.next_reap() {
        let (token, outcome) = match work {
            Reap::Rejected(token, job) => {
                drop(job);
                (token, None)
            }
            Reap::Join(token, handle) => match handle.join() {
                Ok(Some(outcome)) => (token, Some(outcome)),
                Ok(None) => {
                    shared.unconfirmed(token, None, None);
                    continue;
                }
                Err(panic) => {
                    shared.unconfirmed(token, None, Some(panic));
                    continue;
                }
            },
        };
        if let Some(outcome) = outcome {
            let confirmed = shared
                .lock()
                .owned(token)
                .is_some_and(|owned| owned.cleaned && owned.phase == Phase::Joining);
            if !confirmed {
                shared.unconfirmed(token, Some(outcome), None);
                continue;
            }
            let WorkerExit {
                wake,
                cause,
                payload,
            } = outcome;
            // The owner remains in the occupied slot if payload cleanup panics.
            drop(payload);
            let control = shared.release(token);
            {
                let mut header = control.header();
                header.status = ReceiveStatus::Closed(header.close_cause.or(cause));
            }
            // Move the box: both invocation and its last Drop are inside catch.
            if let Err(panic) = catch_unwind(AssertUnwindSafe(wake)) {
                discard_panic(panic);
            }
        } else {
            let control = shared.release(token);
            let mut header = control.header();
            header.status = ReceiveStatus::Closed(header.close_cause);
        }
    }
}

fn discard_panic(payload: Payload) {
    if let Err(secondary) = catch_unwind(AssertUnwindSafe(move || drop(payload))) {
        // Its destructor already failed once. Do not unwind through the service again.
        std::mem::forget(secondary);
    }
}

#[cfg(test)]
#[path = "worker_pool_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "worker_pool_failure_tests.rs"]
mod failure_tests;

#[cfg(test)]
#[path = "worker_pool_owner_tests.rs"]
mod owner_tests;
