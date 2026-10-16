use super::*;
use crate::receive::{
    runtime::testing,
    worker_pool::{SurfaceKey, WorkerPool},
};
use crate::{DropDecision, FileDropReceiver};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use std::{
    cell::Cell,
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    ptr::NonNull,
    sync::{mpsc, Weak},
    thread::{self, JoinHandle, ThreadId},
};

// Public native fixtures share the actual process-wide admission budgets. Hold
// one test-only guard through confirmed cleanup so independent test threads do
// not accidentally spend each other's four worker/surface permits.
fn public_pool_guard() -> std::sync::MutexGuard<'static, ()> {
    static PUBLIC_POOL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    PUBLIC_POOL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

const WAIT: Duration = Duration::from_secs(3);
const DATA: &[u8] = b"file:///tmp/receiver-a.txt\r\nfile:///tmp/receiver-b.txt\r\n";

struct UnavailableWindow(Arc<AtomicUsize>);

impl HasWindowHandle for UnavailableWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(HandleError::Unavailable)
    }
}

impl HasDisplayHandle for UnavailableWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(HandleError::Unavailable)
    }
}

struct FixtureWindow {
    connection: Connection,
    surface: client::protocol::wl_surface::WlSurface,
    dropped: Option<mpsc::Sender<(ThreadId, bool)>>,
    observed: testing::Observations,
}

impl HasWindowHandle for FixtureWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let surface =
            NonNull::new(self.surface.id().as_ptr().cast()).ok_or(HandleError::Unavailable)?;
        // This object owns a real surface and the connection that allocated it.
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::Wayland(WaylandWindowHandle::new(surface)))
        })
    }
}

impl HasDisplayHandle for FixtureWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let display = NonNull::new(self.connection.backend().display_ptr().cast())
            .ok_or(HandleError::Unavailable)?;
        // The connection owns this actual private display for the borrow's lifetime.
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                display,
            )))
        })
    }
}

impl Drop for FixtureWindow {
    fn drop(&mut self) {
        if let Some(sent) = &self.dropped {
            let _ = sent.send((
                thread::current().id(),
                self.observed.cleaned.load(Ordering::SeqCst),
            ));
        }
    }
}

// Keep the real private server and host reader alive even if a public assertion fails.
struct PublicCase {
    fixture: Option<Fixture>,
    receiver: Option<FileDropReceiver>,
    owner: Weak<FixtureWindow>,
    final_wake: mpsc::Receiver<()>,
    _public_pool_guard: std::sync::MutexGuard<'static, ()>,
}

impl PublicCase {
    fn attach(
        fixture: Fixture,
        hooks: testing::Hooks,
        dropped: Option<mpsc::Sender<(ThreadId, bool)>>,
    ) -> Self {
        let public_pool_guard = public_pool_guard();
        let window = Arc::new(FixtureWindow {
            connection: fixture.host.clone(),
            surface: fixture.host_surfaces[0].clone(),
            dropped,
            observed: hooks.observed.clone(),
        });
        let owner = Arc::downgrade(&window);
        let callback_owner = owner.clone();
        let (final_wake, final_result) = mpsc::channel();
        let calls = Cell::new(0_u64); // Fn + Send, intentionally not Sync.
        let receiver = testing::with_hooks(hooks, || {
            // Fixture, its event-loop substitute and reader live through actual cleanup.
            unsafe {
                FileDropReceiver::attach_owned(window, ReceiveLimits::default(), move || {
                    calls.set(calls.get().wrapping_add(1));
                    if callback_owner.upgrade().is_none() {
                        let _ = final_wake.send(());
                    }
                })
            }
        })
        .expect("the public owned receiver admits a valid Wayland fixture")
        .expect("the actual fixture handles are Wayland");
        Self {
            fixture: Some(fixture),
            receiver: Some(receiver),
            owner,
            final_wake: final_result,
            _public_pool_guard: public_pool_guard,
        }
    }

    fn receiver(&self) -> &FileDropReceiver {
        self.receiver.as_ref().unwrap()
    }
    fn fixture(&mut self) -> &mut Fixture {
        self.fixture.as_mut().unwrap()
    }

    fn step(&mut self) {
        let fixture = self.fixture();
        fixture.serve();
        fixture.host_read();
    }
}

impl Drop for PublicCase {
    fn drop(&mut self) {
        if let Some(receiver) = &self.receiver {
            receiver.close();
        }
        let deadline = Instant::now() + WAIT;
        while self.owner.upgrade().is_some() {
            let progress = catch_unwind(AssertUnwindSafe(|| self.step()));
            if progress.is_err() || Instant::now() >= deadline {
                // A failed test must not tear down an unconfirmed foreign display.
                if let Some(fixture) = self.fixture.take() {
                    std::mem::forget(fixture);
                }
                return;
            }
            thread::yield_now();
        }
    }
}

fn drive(case: &mut PublicCase, mut done: impl FnMut(&mut PublicCase) -> bool) {
    let deadline = Instant::now() + WAIT;
    loop {
        case.step();
        if done(case) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the public receiver made no progress"
        );
        thread::yield_now();
    }
}

struct ReleaseOnDrop {
    release: Option<mpsc::SyncSender<()>>,
    dropper: Option<JoinHandle<ThreadId>>,
}

impl ReleaseOnDrop {
    fn finish(&mut self) -> ThreadId {
        if let Some(release) = self.release.take() {
            let _ = release.try_send(());
        }
        self.dropper.take().unwrap().join().unwrap()
    }
}

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.try_send(());
        }
        if let Some(dropper) = self.dropper.take() {
            let _ = dropper.join();
        }
    }
}

#[test]
fn public_limits_are_validated_before_any_native_handle_query() {
    let queried = Arc::new(AtomicUsize::new(0));
    let window = Arc::new(UnavailableWindow(queried.clone()));
    let wakes = Arc::new(AtomicUsize::new(0));
    let invalid = ReceiveLimits {
        max_files: 0,
        ..ReceiveLimits::default()
    };
    let callback = wakes.clone();
    // These traits return HandleError and never manufacture a native pointer.
    let result = unsafe {
        FileDropReceiver::attach_owned(window.clone(), invalid, move || {
            callback.fetch_add(1, Ordering::SeqCst);
        })
    };
    assert!(matches!(result, Err(ReceiveError::InvalidLimits)));
    assert_eq!(queried.load(Ordering::SeqCst), 0);
    let result = unsafe { FileDropReceiver::attach_owned(window, ReceiveLimits::default(), || {}) };
    assert!(matches!(result, Err(ReceiveError::Unavailable)));
    assert!(queried.load(Ordering::SeqCst) > 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
}

#[test]
fn public_receiver_keeps_epoch_point_and_batch_through_eof_and_explicit_accept() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    fixture.create_surfaces();
    let mut case = PublicCase::attach(fixture, testing::Hooks::default(), None);
    let epoch = case.receiver().invalidate_pending().unwrap();
    drive(&mut case, |case| {
        case.receiver().status() == ReceiveStatus::Ready
    });
    let offer = case.fixture().offer(0, 31, "text/uri-list");
    case.fixture().enter(0, &offer, 0, 41.0, 73.0);
    offer.action(wl_data_device_manager::DndAction::Copy);
    case.fixture().server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    let mut events = Vec::new();
    drive(&mut case, |case| {
        events.extend(case.receiver().poll());
        case.fixture().received(31) == 1
            && events
                .iter()
                .any(|event| matches!(event, ReceiveEvent::DropStarted { .. }))
    });
    let started: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ReceiveEvent::DropStarted { id, epoch, point } => Some((*id, *epoch, *point)),
            _ => None,
        })
        .collect();
    assert_eq!(started.len(), 1, "one exact public Started: {events:?}");
    let (id, actual_epoch, point) = started[0];
    assert_eq!(actual_epoch, epoch);
    assert_eq!(point, SurfacePoint { x: 41.0, y: 73.0 });
    assert!(events.iter().all(|event| matches!(event,
        ReceiveEvent::Hovered { id: actual, .. } | ReceiveEvent::DropStarted { id: actual, .. } if *actual == id)));
    let writer = case.fixture().take_writer(31);
    let mut remaining = DATA;
    while !remaining.is_empty() {
        let count = rustix::io::write(&writer, remaining).unwrap();
        assert!(count > 0);
        remaining = &remaining[count..];
    }
    for _ in 0..3 {
        case.step();
        assert!(case.receiver().poll().is_empty());
    }
    assert_eq!(case.fixture().finish_count(31), 0);
    drop(writer);
    events.clear();
    drive(&mut case, |case| {
        events.extend(case.receiver().poll());
        events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
    });
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropReady { id: actual, paths }]
        if *actual == id && *paths == vec![PathBuf::from("/tmp/receiver-a.txt"), PathBuf::from("/tmp/receiver-b.txt")])
    );
    assert_eq!(case.fixture().finish_count(31), 0);
    case.receiver().decide(id, DropDecision::Accept).unwrap();
    events.clear();
    drive(&mut case, |case| {
        events.extend(case.receiver().poll());
        events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
    });
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropEnded { id: actual, result: Ok(()) }] if *actual == id)
    );
    drive(&mut case, |case| case.fixture().finish_count(31) == 1);
    assert_eq!(
        case.receiver().decide(id, DropDecision::Accept),
        Err(ReceiveError::StaleOffer)
    );
    case.receiver().cancel(id);
    case.receiver().close();
    drive(&mut case, |case| {
        matches!(case.receiver().status(), ReceiveStatus::Closed(_))
    });
    assert!(case.receiver().poll().is_empty());
    assert_eq!(case.fixture().finish_count(31), 1);
    assert_eq!(
        case.receiver().invalidate_pending(),
        Err(ReceiveError::Closed)
    );
}

#[test]
fn public_drop_never_joins_and_keeps_owner_and_exclusion_until_guest_exit() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    fixture.create_surfaces();
    let (entered, gate_entered) = mpsc::sync_channel(1);
    let (release, gate_release) = mpsc::sync_channel(1);
    let hooks = testing::Hooks {
        before_dispatch: Some((entered, gate_release)),
        ..testing::Hooks::default()
    };
    let observed = hooks.observed.clone();
    let (owner_dropped, owner_result) = mpsc::channel();
    let mut case = PublicCase::attach(fixture, hooks, Some(owner_dropped));
    let mut release = ReleaseOnDrop {
        release: Some(release),
        dropper: None,
    };
    gate_entered
        .recv_timeout(WAIT)
        .expect("the real guest worker reaches its dispatch gate");
    let key = SurfaceKey {
        display: case.fixture().host.backend().display_ptr() as usize,
        surface: case.fixture().host_surfaces[0].id().as_ptr() as usize,
    };
    let caller = thread::current().id();
    let receiver = case.receiver.take().unwrap();
    let (returned, drop_result) = mpsc::channel();
    release.dropper = Some(thread::spawn(move || {
        let dropping = thread::current().id();
        drop(receiver);
        let _ = returned.send(());
        dropping
    }));
    let returned_without_exit = drop_result.recv_timeout(Duration::from_millis(100));
    let owner_retained = case.owner.upgrade().is_some();
    let reserved = WorkerPool::global().reserve_legacy(key);
    let exclusion_retained = matches!(reserved, Err(ReceiveError::Busy));
    drop(reserved);
    let premature_owner_drop = owner_result.try_recv();
    let dropping = release.finish(); // Release and join before checking assertions.
    drive(&mut case, |case| case.final_wake.try_recv().is_ok());
    let (owner_thread, cleaned) = owner_result.recv_timeout(WAIT).unwrap();

    assert_eq!(
        returned_without_exit,
        Ok(()),
        "public Drop does not wait for the blocked guest"
    );
    assert!(owner_retained);
    assert!(matches!(
        premature_owner_drop,
        Err(mpsc::TryRecvError::Empty)
    ));
    assert!(exclusion_retained);
    assert!(
        observed.gate_released.load(Ordering::SeqCst),
        "the gate opened explicitly, not by its timeout"
    );
    assert!(
        cleaned,
        "the actual guest cleaned up before releasing its window owner"
    );
    assert_ne!(owner_thread, caller);
    assert_ne!(owner_thread, dropping);
    assert!(case.owner.upgrade().is_none());
    assert!(WorkerPool::global().reserve_legacy(key).is_ok());
}

#[path = "public_owner_tests.rs"]
mod shared_owner;
