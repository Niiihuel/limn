use super::*;
use crate::receive::worker_pool::SurfaceKey;
use crate::ReceiveError;
use sctk::reexports::calloop::{generic::Generic, Interest, LoopSignal, Mode, PostAction};
use sctk::reexports::client::globals::GlobalListContents;
use sctk::reexports::client::protocol::{wl_compositor::WlCompositor, wl_registry::WlRegistry};
use sctk::reexports::client::{delegate_noop, Dispatch as ClientDispatch};
use std::{
    cell::RefCell,
    os::{fd::AsFd, unix::net::UnixStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use wayland_server::{
    protocol::{wl_compositor, wl_surface},
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, New,
};

const WAIT: Duration = Duration::from_secs(3);

#[derive(Clone, Default)]
pub(super) struct Hooks {
    backend: Arc<AtomicUsize>,
    registry: Arc<AtomicUsize>,
    tls: Option<TlsHook>,
}

impl Hooks {
    pub(super) fn backend_entered(&self) {
        self.backend.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn registry_entered(&self) {
        self.registry.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) fn install_tls(&self) {
        if let Some(hook) = self.tls.clone() {
            TLS_EXIT.with(|slot| *slot.borrow_mut() = Some(TlsExit(hook)));
        }
    }
}

#[derive(Clone)]
struct TlsHook {
    gate: Arc<Gate>,
    entered: mpsc::Sender<()>,
    opened: mpsc::Sender<bool>,
}

struct TlsExit(TlsHook);

impl Drop for TlsExit {
    fn drop(&mut self) {
        let _ = self.0.entered.send(());
        let _ = self.0.opened.send(self.0.gate.wait());
    }
}

thread_local! {
    static TLS_EXIT: RefCell<Option<TlsExit>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct Gate {
    opened: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn wait(&self) -> bool {
        let guard = self
            .opened
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, WAIT * 2, |opened| !*opened)
            .unwrap_or_else(|error| error.into_inner());
        *guard
    }

    fn release(&self) {
        *self
            .opened
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = true;
        self.changed.notify_all();
    }
}

struct ReleaseOnDrop(Arc<Gate>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

struct DropThread {
    handle: Option<JoinHandle<()>>,
    gate: Arc<Gate>,
}

impl DropThread {
    fn join(&mut self) {
        self.gate.release();
        self.handle.take().unwrap().join().unwrap();
    }
}

impl Drop for DropThread {
    fn drop(&mut self) {
        self.gate.release();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct Protocol;

impl GlobalDispatch<wl_compositor::WlCompositor, ()> for Protocol {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<wl_compositor::WlCompositor>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        init.init(resource, ());
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for Protocol {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_compositor::WlCompositor,
        request: wl_compositor::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                init.init(id, ());
            }
            _ => panic!("the private legacy host only creates a surface"),
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for Protocol {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_surface::WlSurface,
        request: wl_surface::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        assert!(matches!(request, wl_surface::Request::Destroy));
    }
}

struct ServerState {
    display: Display<Protocol>,
    protocol: Protocol,
}

struct PrivateServer {
    stop: Arc<AtomicBool>,
    signal: LoopSignal,
    thread: Option<JoinHandle<()>>,
}

impl PrivateServer {
    fn new() -> (Self, UnixStream) {
        let (server_socket, host_socket) = UnixStream::pair().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let (ready, signal) = mpsc::channel();
        let thread = thread::spawn(move || {
            let display = Display::<Protocol>::new().unwrap();
            let mut handle = display.handle();
            handle.create_global::<Protocol, wl_compositor::WlCompositor, _>(1, ());
            handle.insert_client(server_socket, Arc::new(())).unwrap();
            let fd = display.as_fd().try_clone_to_owned().unwrap();
            let mut event_loop = EventLoop::<ServerState>::try_new().unwrap();
            event_loop
                .handle()
                .insert_source(
                    Generic::new(fd, Interest::READ, Mode::Level),
                    |_, _, state| {
                        state.display.dispatch_clients(&mut state.protocol)?;
                        state.display.flush_clients()?;
                        Ok(PostAction::Continue)
                    },
                )
                .unwrap();
            ready.send(event_loop.get_signal()).unwrap();
            let mut state = ServerState {
                display,
                protocol: Protocol,
            };
            // A fixture failure closes its socket instead of leaving registry roundtrip stuck.
            let deadline = Instant::now() + WAIT * 3;
            while !stopped.load(Ordering::SeqCst) && Instant::now() < deadline {
                if event_loop
                    .dispatch(Some(Duration::from_millis(50)), &mut state)
                    .is_err()
                {
                    break;
                }
            }
        });
        let signal = signal.recv_timeout(WAIT).expect("private server started");
        (
            Self {
                stop,
                signal,
                thread: Some(thread),
            },
            host_socket,
        )
    }

    fn disconnect(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.signal.wakeup();
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

impl Drop for PrivateServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.signal.wakeup();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Host;
delegate_noop!(Host: WlCompositor);
delegate_noop!(Host: ignore WlSurface);

impl ClientDispatch<WlRegistry, GlobalListContents> for Host {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

struct Fixture {
    surface: WlSurface,
    host: Connection,
    server: PrivateServer,
}

impl Fixture {
    fn new() -> Self {
        let (server, socket) = PrivateServer::new();
        let host = Connection::from_socket(socket).unwrap();
        let (globals, mut queue) = registry_queue_init::<Host>(&host).unwrap();
        let handle = queue.handle();
        let compositor: WlCompositor = globals.bind(&handle, 1..=1, ()).unwrap();
        let surface = compositor.create_surface(&handle, ());
        queue.roundtrip(&mut Host).unwrap();
        Self {
            surface,
            host,
            server,
        }
    }

    fn key(&self) -> SurfaceKey {
        SurfaceKey {
            display: self.host.backend().display_ptr() as usize,
            surface: self.surface.id().as_ptr() as usize,
        }
    }

    fn spawn(&self, pool: Arc<WorkerPool>, hooks: Hooks) -> Option<Worker> {
        // The fixture keeps its connection, surface and private server alive through every join.
        unsafe {
            Worker::spawn_in(
                self.host.backend().display_ptr().cast(),
                self.surface.id().as_ptr().cast(),
                Box::new(|| {}),
                pool,
                hooks,
            )
        }
    }
}

#[test]
fn legacy_worker_rejects_shared_surface_before_entering_the_foreign_backend() {
    let fixture = Fixture::new();
    let pool = WorkerPool::isolated();
    for _ in 0..63 {
        pool.consume_registry().unwrap();
    }
    let held = pool.reserve_legacy(fixture.key()).unwrap();
    let hooks = Hooks::default();
    let worker = fixture.spawn(pool.clone(), hooks.clone());
    let rejected = worker.is_none();
    drop(worker); // An unexpected RED worker still has a live protocol server during its join.
    let remaining_credit = pool.consume_registry();
    drop(held);

    assert_eq!(
        hooks.backend.load(Ordering::SeqCst),
        0,
        "a conflicting receiver must be rejected before touching the foreign backend"
    );
    assert!(rejected);
    assert_eq!(hooks.registry.load(Ordering::SeqCst), 0);
    assert_eq!(
        remaining_credit,
        Ok(()),
        "rejected admission does not spend a registry credit"
    );
}

#[test]
fn legacy_worker_keeps_its_shared_slot_through_drop_join_and_tls() {
    let fixture = Fixture::new();
    let pool = WorkerPool::isolated();
    let gate = Arc::new(Gate::default());
    let (entered, tls_entered) = mpsc::channel();
    let (opened, tls_opened) = mpsc::channel();
    let hooks = Hooks {
        tls: Some(TlsHook {
            gate: gate.clone(),
            entered,
            opened,
        }),
        ..Hooks::default()
    };
    let worker = fixture.spawn(pool.clone(), hooks).unwrap();
    let _release = ReleaseOnDrop(gate.clone());
    tls_entered
        .recv_timeout(WAIT)
        .expect("the real legacy worker reaches TLS teardown");
    let key = fixture.key();
    let fillers: Vec<_> = (1..=3)
        .map(|offset| {
            pool.reserve_legacy(SurfaceKey {
                display: key.display,
                surface: key.surface + offset,
            })
            .unwrap()
        })
        .collect();
    let fifth = pool.reserve_legacy(SurfaceKey {
        display: key.display,
        surface: key.surface + 4,
    });
    let fifth_rejected = matches!(fifth, Err(ReceiveError::Busy));
    drop(fifth);

    let (started, dropping) = mpsc::channel();
    let (finished, dropped) = mpsc::channel();
    let mut drop_thread = DropThread {
        gate: gate.clone(),
        handle: Some(thread::spawn(move || {
            started.send(()).unwrap();
            drop(worker);
            let _ = finished.send(());
        })),
    };
    dropping.recv_timeout(WAIT).unwrap();
    let still_joining = matches!(
        dropped.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
    let conflict = pool.reserve_legacy(key);
    let retained = matches!(conflict, Err(ReceiveError::Busy));
    drop(conflict);
    drop_thread.join();
    let really_opened = tls_opened.recv_timeout(WAIT).unwrap();
    dropped.recv_timeout(WAIT).unwrap();
    let released = pool.reserve_legacy(key).is_ok();
    drop(fillers);

    assert!(
        really_opened,
        "TLS must leave through explicit gate release, not its timeout"
    );
    assert!(still_joining, "legacy Drop waits for the actual TLS end");
    assert!(
        retained,
        "the exact surface stays reserved until join returns"
    );
    assert!(
        fifth_rejected,
        "the actual legacy worker occupies one of the four shared slots"
    );
    assert!(released);
}

#[test]
fn legacy_worker_spends_registry_credit_at_the_real_helper_and_never_refunds_failure() {
    let mut fixture = Fixture::new();
    let pool = WorkerPool::isolated();
    for _ in 0..63 {
        pool.consume_registry().unwrap();
    }
    // Keep the wl_display and wl_surface objects alive, but make the real helper fail on EOF.
    fixture.server.disconnect();
    let first = Hooks::default();
    drop(fixture.spawn(pool.clone(), first.clone()).unwrap());
    let after_failure = pool.consume_registry();
    let second = Hooks::default();
    drop(fixture.spawn(pool, second.clone()).unwrap());

    assert_eq!(
        first.registry.load(Ordering::SeqCst),
        1,
        "this test crosses registry_queue_init rather than only exercising the atomic counter"
    );
    assert_eq!(
        after_failure,
        Err(ReceiveError::ResourceLimit),
        "the failed real GetRegistry attempt consumed credit 64 permanently"
    );
    assert_eq!(
        second.registry.load(Ordering::SeqCst),
        0,
        "an exhausted worker must not enter registry_queue_init again"
    );
}
