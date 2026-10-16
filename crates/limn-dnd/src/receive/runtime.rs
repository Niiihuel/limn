//! One owned receive worker, driven by controls, pipe readiness and a guest tick.

use std::{
    any::Any,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::Arc,
    time::{Duration, Instant},
};

use sctk::reexports::calloop::{
    generic::Generic,
    ping::{make_ping, Ping, PingSource},
    EventLoop, Interest, Mode, PostAction, RegistrationToken,
};

use super::{
    clipboard_control::{from_receive_error, ClipboardControl, ClipboardWorker},
    control::ReceiveControl,
    read_tag::ReadTag,
    wayland::GuestStartup,
    worker_pool::{ExitSignal, OwnerId, SurfaceKey, Wake, WorkerExit, WorkerPool},
};
use crate::{ClipboardError, ReceiveError, ReceiveLimits, TextClipboardLimits};

const GUEST_TICK: Duration = Duration::from_millis(20);
type PanicPayload = Box<dyn Any + Send>;

/// # Safety
///
/// `key` must identify a live Wayland display and surface. `owner` must keep
/// both alive, and the host must preserve its coordinated reader until the
/// pool confirms cleanup and join. Raw handles alone do not prove ownership.
pub(super) unsafe fn spawn_owned(
    pool: Arc<WorkerPool>,
    key: SurfaceKey,
    owner: Arc<dyn Any + Send + Sync>,
    control: ReceiveControl,
    limits: ReceiveLimits,
    wake: Wake,
) -> Result<Ping, ReceiveError> {
    limits.validate()?;
    if key.display == 0 || key.surface == 0 {
        return Err(ReceiveError::Unavailable);
    }
    let (ping, source) = make_ping().map_err(|_| ReceiveError::Io)?;
    let started = Instant::now();
    let reservation = pool.reserve_owned(key, owner, control.clone())?;
    let job = Job {
        pool,
        target: Target::Surface(key),
        control,
        limits,
        clipboard: None,
        wake,
        source,
        started,
        #[cfg(test)]
        hooks: testing::take(),
    };
    // The slot owns the real job before OS spawn, including any rejected spawn.
    reservation.spawn(move |exit| job.run(exit))?;
    Ok(ping)
}

/// # Safety
///
/// `display` must be the live Wayland display retained by `owner`. Its host
/// must preserve coordinated reading until certified cleanup and actual join.
pub(super) unsafe fn spawn_owner(
    pool: Arc<WorkerPool>,
    display: usize,
    owner: Arc<dyn Any + Send + Sync>,
    control: ReceiveControl,
    limits: ReceiveLimits,
    wake: Wake,
) -> Result<(OwnerId, Ping), ReceiveError> {
    unsafe { spawn_owner_common(pool, display, owner, control, limits, None, wake) }
}

/// # Safety
/// The display owner and coordinated host reader obey `spawn_owner`'s lifetime.
pub(super) unsafe fn spawn_owner_with_text(
    pool: Arc<WorkerPool>,
    display: usize,
    owner: Arc<dyn Any + Send + Sync>,
    control: ReceiveControl,
    receive: ReceiveLimits,
    text: TextClipboardLimits,
    wake: Wake,
) -> Result<(OwnerId, Ping, ClipboardControl), ClipboardError> {
    receive.validate().map_err(from_receive_error)?;
    text.validate()?;
    if display == 0 {
        return Err(ClipboardError::Unavailable);
    }
    let (clipboard, worker) = ClipboardControl::new(control.clone(), text)?;
    let (owner, ping) =
        unsafe { spawn_owner_common(pool, display, owner, control, receive, Some(worker), wake) }
            .map_err(from_receive_error)?;
    Ok((owner, ping, clipboard))
}

unsafe fn spawn_owner_common(
    pool: Arc<WorkerPool>,
    display: usize,
    owner: Arc<dyn Any + Send + Sync>,
    control: ReceiveControl,
    limits: ReceiveLimits,
    clipboard: Option<ClipboardWorker>,
    wake: Wake,
) -> Result<(OwnerId, Ping), ReceiveError> {
    limits.validate()?;
    if display == 0 {
        return Err(ReceiveError::Unavailable);
    }
    // A failed Ping precedes admission of every native resource.
    let (ping, source) = make_ping().map_err(|_| ReceiveError::Io)?;
    let started = Instant::now();
    let (owner, reservation) =
        pool.reserve_owner(display, owner, control.clone(), limits.startup_timeout)?;
    let job = Job {
        pool,
        target: Target::Owner { display, owner },
        control,
        limits,
        clipboard,
        wake,
        source,
        started,
        #[cfg(test)]
        hooks: testing::take(),
    };
    reservation.spawn(move |exit| job.run(exit))?;
    Ok((owner, ping))
}

#[derive(Clone, Copy)]
enum Target {
    Surface(SurfaceKey),
    Owner { display: usize, owner: OwnerId },
}

impl Target {
    fn request_close(
        self,
        pool: &WorkerPool,
        control: &ReceiveControl,
        cause: Option<ReceiveError>,
    ) {
        match self {
            Self::Surface(_) => control.header().request_close(cause),
            Self::Owner { owner, .. } => pool.close_owner(owner, cause),
        }
    }
}

struct Job {
    pool: Arc<WorkerPool>,
    target: Target,
    control: ReceiveControl,
    limits: ReceiveLimits,
    clipboard: Option<ClipboardWorker>,
    wake: Wake,
    source: PingSource,
    started: Instant,
    #[cfg(test)]
    hooks: testing::Hooks,
}

// No Calloop Rc, LoopHandle or EventLoop crosses this retention boundary.
// An unconfirmed slot keeps the SDK state and the same wake indefinitely.
struct FailedCleanup {
    _guest: GuestStartup,
    _work_panic: Option<PanicPayload>,
    _cleanup_panic: Option<PanicPayload>,
}

impl Job {
    fn run(self, exit: ExitSignal) -> WorkerExit {
        let Self {
            pool,
            target,
            control,
            limits,
            clipboard,
            wake,
            source,
            started,
            #[cfg(test)]
            mut hooks,
        } = self;
        // Nothing in this constructor touches the foreign display or calls wake.
        let mut guest = match target {
            Target::Surface(_) => {
                GuestStartup::dormant(pool.clone(), control.clone(), limits, started, wake)
            }
            Target::Owner { owner, .. } => GuestStartup::dormant_owner(
                pool.clone(),
                owner,
                control.clone(),
                limits,
                started,
                wake,
                clipboard,
            ),
        };
        #[cfg(test)]
        guest.install_clipboard_test_hooks(
            hooks.observed.clone(),
            hooks.before_clipboard_seal.take(),
        );
        let mut source = Some(source);
        let mut runtime = None;
        let work = catch_unwind(AssertUnwindSafe(|| -> Result<(), ReceiveError> {
            // The pool retained the exact owner before it enabled this job.
            match target {
                Target::Surface(key) => unsafe { guest.install_foreign(key)? },
                Target::Owner { display, .. } => unsafe { guest.install_foreign_display(display)? },
            }
            #[cfg(test)]
            if hooks.panic_startup {
                panic!("private startup panic");
            }
            runtime = Some(Runtime::new(
                source
                    .take()
                    .expect("the one ping source is installed once"),
                #[cfg(test)]
                hooks.observed.clone(),
            )?);
            let mut readable = None;
            loop {
                if control.header().is_closing() {
                    return Ok(());
                }
                #[cfg(test)]
                if let Some((entered, release)) = hooks.before_dispatch.take() {
                    entered.send(()).map_err(|_| ReceiveError::Io)?;
                    let released = release.recv_timeout(Duration::from_secs(3)).is_ok();
                    hooks
                        .observed
                        .gate_released
                        .store(released, std::sync::atomic::Ordering::SeqCst);
                    if !released {
                        return Err(ReceiveError::Io);
                    }
                }
                if control.header().is_closing() {
                    return Ok(());
                }
                guest.pump_readable(Instant::now(), readable.take())?;
                #[cfg(test)]
                hooks
                    .observed
                    .completed_pumps
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let runtime = runtime.as_mut().expect("loop was installed before pumping");
                #[cfg(test)]
                if runtime
                    .pipe
                    .as_ref()
                    .is_some_and(|(id, _)| Some(*id) != guest.pipe_fd().map(|(current, _)| current))
                {
                    if let Some((entered, release)) = hooks.before_retire_sync.take() {
                        entered.send(()).map_err(|_| ReceiveError::Io)?;
                        let released = release.recv_timeout(Duration::from_secs(3)).is_ok();
                        hooks
                            .observed
                            .retirement_gate_released
                            .store(released, std::sync::atomic::Ordering::SeqCst);
                        if !released {
                            return Err(ReceiveError::Io);
                        }
                    }
                }
                runtime.sync_pipe(&guest)?;
                guest.finish_retirements()?;
                let wait = guest.next_deadline().map_or(GUEST_TICK, |deadline| {
                    GUEST_TICK.min(deadline.saturating_duration_since(Instant::now()))
                });
                readable = runtime.dispatch(wait)?;
            }
        }));
        let (cause, work_panic) = match work {
            Ok(Ok(())) => (None, None),
            Ok(Err(error)) => {
                let cause = {
                    let header = control.header();
                    if header.is_closing() {
                        header.close_cause
                    } else {
                        Some(error)
                    }
                };
                target.request_close(&pool, &control, cause);
                (control.header().close_cause, None)
            }
            Err(panic) => {
                target.request_close(&pool, &control, Some(ReceiveError::Io));
                (Some(ReceiveError::Io), Some(panic))
            }
        };
        // Parent authority and every endpoint are cut even if cleanup itself
        // fails before it can enter GuestStartup::cleanup.
        target.request_close(&pool, &control, None);
        let cleanup = catch_unwind(AssertUnwindSafe(|| -> Result<(), ReceiveError> {
            // These contain only control and FD sources. Destroy all Calloop Rc
            // state here, before either cleanup or retention of the SDK guest.
            drop(runtime.take());
            drop(source.take());
            #[cfg(test)]
            if hooks.panic_cleanup {
                panic!("private cleanup panic");
            }
            guest.cleanup()
        }));
        match cleanup {
            Ok(Ok(())) => {
                #[cfg(test)]
                hooks
                    .observed
                    .cleaned
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                let wake = guest.take_wake();
                // cleanup removed every SDK/FD owner; only plain metadata remains.
                drop(guest);
                exit.guest_cleaned();
                WorkerExit {
                    wake,
                    cause,
                    payload: work_panic,
                }
            }
            failed => {
                target.request_close(&pool, &control, Some(ReceiveError::Io));
                let cleanup_panic = match failed {
                    Err(panic) => Some(panic),
                    Ok(Err(_)) => None,
                    Ok(Ok(())) => unreachable!("confirmed cleanup took its success branch"),
                };
                #[cfg(test)]
                hooks
                    .observed
                    .cleanup_failed
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                let wake = guest.take_wake();
                // No cleanup signal: the pool cannot join/release this uncertain slot.
                WorkerExit {
                    wake,
                    cause: Some(ReceiveError::Io),
                    payload: Some(Box::new(FailedCleanup {
                        _guest: guest,
                        _work_panic: work_panic,
                        _cleanup_panic: cleanup_panic,
                    })),
                }
            }
        }
    }
}

struct Runtime {
    events: EventLoop<'static, Option<ReadTag>>,
    pipe: Option<(ReadTag, RegistrationToken)>,
    #[cfg(test)]
    observed: testing::Observations,
}

impl Runtime {
    fn new(
        source: PingSource,
        #[cfg(test)] observed: testing::Observations,
    ) -> Result<Self, ReceiveError> {
        let events = EventLoop::try_new().map_err(|_| ReceiveError::Io)?;
        events
            .handle()
            .insert_source(source, |(), _, _| {})
            .map_err(|_| ReceiveError::Io)?;
        Ok(Self {
            events,
            pipe: None,
            #[cfg(test)]
            observed,
        })
    }

    fn sync_pipe(&mut self, guest: &GuestStartup) -> Result<(), ReceiveError> {
        let pipe = guest.pipe_fd();
        if self.pipe.as_ref().map(|(id, _)| *id) == pipe.as_ref().map(|(id, _)| *id) {
            return Ok(());
        }
        if let Some((_, token)) = self.pipe.take() {
            self.events.handle().remove(token);
        }
        if let Some((id, fd)) = pipe {
            #[cfg(test)]
            let observed = self.observed.clone();
            #[cfg(test)]
            let watched = Arc::downgrade(&fd);
            let token = self
                .events
                .handle()
                .insert_source(
                    Generic::new(fd, Interest::READ, Mode::Level),
                    move |_, _, readable| {
                        #[cfg(test)]
                        observed
                            .pipe_callbacks
                            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        *readable = Some(id);
                        Ok::<_, std::io::Error>(PostAction::Continue)
                    },
                )
                .map_err(|_| ReceiveError::Io)?;
            self.pipe = Some((id, token));
            #[cfg(test)]
            {
                *self.observed.watched_pipe.lock().unwrap() = Some((id, watched));
            }
        }
        Ok(())
    }

    fn dispatch(&mut self, wait: Duration) -> Result<Option<ReadTag>, ReceiveError> {
        let mut readable = None;
        self.events
            .dispatch(Some(wait), &mut readable)
            .map_err(|_| ReceiveError::Io)?;
        Ok(readable)
    }
}

#[cfg(test)]
pub(super) mod testing {
    use std::{
        cell::RefCell,
        sync::{
            atomic::{AtomicBool, AtomicUsize},
            mpsc::{Receiver, SyncSender},
            Arc,
        },
    };

    type WatchedPipe = (super::ReadTag, std::sync::Weak<std::os::fd::OwnedFd>);

    #[derive(Clone, Default)]
    pub(in crate::receive) struct Observations {
        pub(in crate::receive) watched_pipe: Arc<std::sync::Mutex<Option<WatchedPipe>>>,
        pub(in crate::receive) clipboard_seals: Arc<AtomicUsize>,
        pub(in crate::receive) guest_dispatches: Arc<AtomicUsize>,
        pub(in crate::receive) clipboard_quarantine_dispatch: Arc<AtomicUsize>,
        pub(in crate::receive) keyboard_enter_dispatch: Arc<AtomicUsize>,
        pub(in crate::receive) retirement_gate_released: Arc<AtomicBool>,
        pub(in crate::receive) pipe_callbacks: Arc<AtomicUsize>,
        pub(in crate::receive) completed_pumps: Arc<AtomicUsize>,
        pub(in crate::receive) cleaned: Arc<AtomicBool>,
        pub(in crate::receive) gate_released: Arc<AtomicBool>,
        pub(in crate::receive) cleanup_failed: Arc<AtomicBool>,
    }

    #[derive(Default)]
    pub(in crate::receive) struct Hooks {
        pub(in crate::receive) observed: Observations,
        pub(in crate::receive) before_dispatch: Option<(SyncSender<()>, Receiver<()>)>,
        pub(in crate::receive) before_retire_sync: Option<(SyncSender<()>, Receiver<()>)>,
        pub(in crate::receive) before_clipboard_seal:
            Option<(SyncSender<std::time::Instant>, Receiver<()>)>,
        pub(in crate::receive) panic_startup: bool,
        pub(in crate::receive) panic_cleanup: bool,
    }

    thread_local! {
        static NEXT: RefCell<Option<Hooks>> = const { RefCell::new(None) };
    }

    struct Restore(Option<Hooks>);

    impl Drop for Restore {
        fn drop(&mut self) {
            NEXT.with(|next| {
                next.replace(self.0.take());
            });
        }
    }

    pub(in crate::receive) fn with_hooks<T>(hooks: Hooks, f: impl FnOnce() -> T) -> T {
        let _restore = Restore(NEXT.with(|next| next.replace(Some(hooks))));
        f()
    }

    pub(in crate::receive) fn take() -> Hooks {
        NEXT.with(|next| next.borrow_mut().take().unwrap_or_default())
    }
}
