use super::super::{control::ReceiveControl, ReceiveEpoch};
use super::{GuestStartup, SyncPhase};
use crate::{ReceiveError, ReceiveLimits, ReceiveStatus};
use sctk::reexports::client::{backend::WaylandError, Connection};
use std::{
    os::unix::net::UnixStream,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use wayland_backend::client::Backend;
use wayland_server::{
    backend::GlobalId,
    protocol::{wl_compositor, wl_data_device, wl_data_device_manager, wl_seat, wl_surface},
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, New, Resource,
};

const STARTUP: Duration = Duration::from_secs(5);

#[derive(Default)]
struct ServerState {
    manager_versions: Vec<u32>,
    seat_versions: Vec<u32>,
    devices: usize,
    released_devices: usize,
    released_seats: usize,
    device_resources: Vec<wl_data_device::WlDataDevice>,
    surfaces: Vec<wl_surface::WlSurface>,
    requests: Vec<offers::OfferRequest>,
}

impl GlobalDispatch<wl_data_device_manager::WlDataDeviceManager, ()> for ServerState {
    fn bind(
        state: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<wl_data_device_manager::WlDataDeviceManager>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        let manager = init.init(resource, ());
        state.manager_versions.push(manager.version());
    }
}

impl GlobalDispatch<wl_seat::WlSeat, ()> for ServerState {
    fn bind(
        state: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<wl_seat::WlSeat>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        let seat = init.init(resource, ());
        state.seat_versions.push(seat.version());
        seat.capabilities(wl_seat::Capability::Pointer);
    }
}

impl Dispatch<wl_data_device_manager::WlDataDeviceManager, ()> for ServerState {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_data_device_manager::WlDataDeviceManager,
        request: wl_data_device_manager::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_data_device_manager::Request::GetDataDevice { id, .. } => {
                let device = init.init(id, ());
                state.device_resources.push(device);
                state.devices += 1;
            }
            _ => panic!("receive-only startup sent an unexpected manager request"),
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for ServerState {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_seat::WlSeat,
        request: wl_seat::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        assert!(matches!(request, wl_seat::Request::Release));
        state.released_seats += 1;
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for ServerState {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_data_device::WlDataDevice,
        request: wl_data_device::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        assert!(matches!(request, wl_data_device::Request::Release));
        state.released_devices += 1;
    }
}

struct Fixture {
    display: Display<ServerState>,
    server: ServerState,
    host: Connection,
    manager_global: Option<GlobalId>,
    seat_globals: Vec<GlobalId>,
    host_surfaces: Vec<sctk::reexports::client::protocol::wl_surface::WlSurface>,
}

impl Fixture {
    fn new(manager: Option<u32>, seats: &[u32]) -> Self {
        let display = Display::<ServerState>::new().unwrap();
        let mut handle = display.handle();
        handle.create_global::<ServerState, wl_compositor::WlCompositor, _>(1, ());
        let manager_global = manager.map(|version| {
            handle.create_global::<ServerState, wl_data_device_manager::WlDataDeviceManager, _>(
                version,
                (),
            )
        });
        let seat_globals = seats
            .iter()
            .map(|&version| handle.create_global::<ServerState, wl_seat::WlSeat, _>(version, ()))
            .collect();
        let (server, client) = UnixStream::pair().unwrap();
        handle.insert_client(server, Arc::new(())).unwrap();
        let host = Connection::from_socket(client).unwrap();
        Self {
            display,
            server: ServerState::default(),
            host,
            manager_global,
            seat_globals,
            host_surfaces: Vec::new(),
        }
    }

    fn guest(
        &self,
        control: ReceiveControl,
        now: Instant,
        wakes: Arc<AtomicUsize>,
    ) -> GuestStartup {
        self.guest_with_wake(control, now, move || {
            wakes.fetch_add(1, Ordering::SeqCst);
        })
    }

    fn guest_with_wake(
        &self,
        control: ReceiveControl,
        now: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> GuestStartup {
        // The fixture keeps the actual host display alive until after the guest is dropped.
        let backend = unsafe { Backend::from_foreign_display(self.host.backend().display_ptr()) };
        GuestStartup::new(
            Connection::from_backend(backend),
            control,
            ReceiveLimits {
                startup_timeout: STARTUP,
                ..ReceiveLimits::default()
            },
            now,
            wake,
        )
        .expect("guest startup capability is available")
    }

    fn serve(&mut self) {
        self.display.dispatch_clients(&mut self.server).unwrap();
        self.display.flush_clients().unwrap();
    }

    fn host_read(&self) {
        if let Some(guard) = self.host.prepare_read() {
            match guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => panic!("private host reader failed: {error}"),
            }
        } else {
            self.host.backend().dispatch_inner_queue().unwrap();
        }
    }

    fn step(&mut self, guest: &mut GuestStartup, now: Instant) -> Result<(), ReceiveError> {
        self.serve();
        self.host_read();
        guest.pump(now)
    }

    fn until_ready(&mut self, guest: &mut GuestStartup, control: &ReceiveControl, now: Instant) {
        for _ in 0..32 {
            guest.pump(now).unwrap();
            self.step(guest, now).unwrap();
            if control.status() == ReceiveStatus::Ready {
                return;
            }
        }
        panic!("private startup did not reach Ready");
    }

    fn until_error(&mut self, guest: &mut GuestStartup, now: Instant) -> ReceiveError {
        for _ in 0..32 {
            if let Err(error) = guest.pump(now) {
                return error;
            }
            if let Err(error) = self.step(guest, now) {
                return error;
            }
        }
        panic!("private startup did not reject the unavailable protocol");
    }
}

#[path = "offer_tests.rs"]
mod offers;

fn control() -> ReceiveControl {
    ReceiveControl::new(ReceiveEpoch(17)).unwrap()
}

fn wakes() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(0))
}

#[test]
fn a_guest_queue_progresses_only_when_the_private_host_reader_runs() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    let control = control();
    let now = Instant::now();
    let wake_count = wakes();
    let mut guest = fixture.guest(control.clone(), now, wake_count.clone());
    assert_eq!(control.status(), ReceiveStatus::Starting);
    guest.pump(now).unwrap();
    for _ in 0..4 {
        fixture.serve();
        guest.pump(now).unwrap();
        assert_eq!(control.status(), ReceiveStatus::Starting);
    }
    fixture.until_ready(&mut guest, &control, now);
    assert!(wake_count.load(Ordering::SeqCst) > 0);
}

#[test]
fn a_missing_or_old_manager_is_unavailable_without_ready() {
    for manager in [None, Some(2)] {
        let mut fixture = Fixture::new(manager, &[5]);
        let control = control();
        let now = Instant::now();
        let wake_count = wakes();
        let mut guest = fixture.guest(control.clone(), now, wake_count.clone());
        assert_eq!(
            fixture.until_error(&mut guest, now),
            ReceiveError::Unavailable
        );
        assert_eq!(control.status(), ReceiveStatus::Closing);
        assert!(fixture.server.manager_versions.is_empty());
        assert!(wake_count.load(Ordering::SeqCst) > 0);
    }
}

#[test]
fn no_seat_with_release_support_is_unavailable_without_ready() {
    for seats in [&[][..], &[4][..]] {
        let mut fixture = Fixture::new(Some(3), seats);
        let control = control();
        let now = Instant::now();
        let mut guest = fixture.guest(control.clone(), now, wakes());
        assert_eq!(
            fixture.until_error(&mut guest, now),
            ReceiveError::Unavailable
        );
        assert_eq!(control.status(), ReceiveStatus::Closing);
        assert!(fixture.server.seat_versions.is_empty());
    }
}

#[test]
fn newer_globals_bind_exactly_manager_three_and_seat_five() {
    let mut fixture = Fixture::new(Some(4), &[4, 5, 7]);
    let control = control();
    let now = Instant::now();
    let mut guest = fixture.guest(control.clone(), now, wakes());
    fixture.until_ready(&mut guest, &control, now);
    assert_eq!(fixture.server.manager_versions, vec![3]);
    assert_eq!(fixture.server.seat_versions, vec![5, 5]);
    assert_eq!(fixture.server.devices, 2);
}

#[test]
fn a_fifth_supported_seat_exceeds_the_shared_receiver_budget() {
    let mut fixture = Fixture::new(Some(3), &[5, 5, 5, 5, 5]);
    let control = control();
    let now = Instant::now();
    let mut guest = fixture.guest(control.clone(), now, wakes());
    assert_eq!(
        fixture.until_error(&mut guest, now),
        ReceiveError::ResourceLimit
    );
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert!(fixture.server.seat_versions.len() <= 4);
}

#[test]
fn a_server_that_never_responds_expires_the_initial_absolute_deadline() {
    let fixture = Fixture::new(Some(3), &[5]);
    let control = control();
    let now = Instant::now();
    let mut guest = fixture.guest(control.clone(), now, wakes());
    guest.pump(now).unwrap();
    guest
        .pump(now + STARTUP - Duration::from_millis(1))
        .unwrap();
    assert_eq!(control.status(), ReceiveStatus::Starting);
    assert_eq!(guest.pump(now + STARTUP), Err(ReceiveError::Timeout));
    assert_eq!(control.status(), ReceiveStatus::Closing);
}

#[test]
fn a_paused_host_reader_times_out_even_when_the_server_has_replied() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    let control = control();
    let now = Instant::now();
    let wake_count = wakes();
    let mut guest = fixture.guest(control.clone(), now, wake_count.clone());
    guest.pump(now).unwrap();
    fixture.serve();
    guest
        .pump(now + STARTUP - Duration::from_millis(1))
        .unwrap();
    assert_eq!(control.status(), ReceiveStatus::Starting);
    assert_eq!(guest.pump(now + STARTUP), Err(ReceiveError::Timeout));
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert!(wake_count.load(Ordering::SeqCst) > 0);
}

#[test]
fn discovery_and_arm_use_distinct_callbacks_with_only_one_in_flight() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    let control = control();
    let now = Instant::now();
    let mut guest = fixture.guest(control.clone(), now, wakes());
    guest.pump(now).unwrap();
    let (discovery, phase) = guest.pending_sync().expect("discovery sync exists");
    assert_eq!(phase, SyncPhase::Discovery);
    fixture.step(&mut guest, now).unwrap();
    let arm = guest.pending_sync().expect("arm sync follows discovery");
    assert_ne!(discovery, arm.0);
    assert_eq!(arm.1, SyncPhase::Arm(ReceiveEpoch(17)));
    assert_eq!(control.status(), ReceiveStatus::Starting);
    for _ in 0..4 {
        guest.pump(now).unwrap();
        assert_eq!(guest.pending_sync(), Some(arm.clone()));
    }
    fixture.until_ready(&mut guest, &control, now);
    assert_eq!(guest.pending_sync(), None);
}

#[test]
fn invalidations_and_discovery_do_not_grant_a_new_startup_deadline() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    let control = control();
    let now = Instant::now();
    let mut guest = fixture.guest(control.clone(), now, wakes());
    guest.pump(now).unwrap();
    fixture
        .step(&mut guest, now + STARTUP - Duration::from_millis(2))
        .unwrap();
    let arm_17 = guest.pending_sync().expect("initial Arm is pending");
    assert_eq!(arm_17.1, SyncPhase::Arm(ReceiveEpoch(17)));
    let epoch_18 = control.invalidate_pending().unwrap();
    assert_eq!(epoch_18, ReceiveEpoch(18));
    fixture
        .step(&mut guest, now + STARTUP - Duration::from_millis(1))
        .unwrap();
    let arm_18 = guest.pending_sync().expect("current epoch needs a new Arm");
    assert_ne!(arm_17.0, arm_18.0);
    assert_eq!(arm_18.1, SyncPhase::Arm(epoch_18));
    assert_eq!(control.status(), ReceiveStatus::Starting);
    assert_eq!(guest.pump(now + STARTUP), Err(ReceiveError::Timeout));
    assert_eq!(control.status(), ReceiveStatus::Closing);
}

#[test]
fn removing_the_manager_or_only_seat_before_arm_never_publishes_ready() {
    for remove_manager in [false, true] {
        let mut fixture = Fixture::new(Some(3), &[5]);
        let control = control();
        let now = Instant::now();
        let observed_ready = Arc::new(AtomicBool::new(false));
        let callback_control = control.clone();
        let callback_ready = observed_ready.clone();
        let mut guest = fixture.guest_with_wake(control.clone(), now, move || {
            if callback_control.status() == ReceiveStatus::Ready {
                callback_ready.store(true, Ordering::SeqCst);
            }
        });
        guest.pump(now).unwrap();
        fixture.step(&mut guest, now).unwrap();
        assert!(matches!(guest.pending_sync(), Some((_, SyncPhase::Arm(_)))));
        assert_eq!(control.status(), ReceiveStatus::Starting);
        let removed = if remove_manager {
            fixture.manager_global.clone().unwrap()
        } else {
            fixture.seat_globals[0].clone()
        };
        // Notify removal while still permitting bind requests already in transit.
        fixture
            .display
            .handle()
            .disable_global::<ServerState>(removed);
        assert_eq!(
            fixture.until_error(&mut guest, now),
            ReceiveError::Unavailable
        );
        assert_eq!(control.status(), ReceiveStatus::Closing);
        let _ = fixture.step(&mut guest, now);
        assert_eq!(control.status(), ReceiveStatus::Closing);
        assert!(!observed_ready.load(Ordering::SeqCst));
    }
}

#[test]
fn a_stale_first_pump_timestamp_cannot_start_discovery_after_the_real_deadline() {
    let fixture = Fixture::new(Some(3), &[5]);
    let control = control();
    let now = Instant::now();
    let wake_count = wakes();
    let callback_wakes = wake_count.clone();
    // As in the other startup tests, the private host owns and reads the display.
    let backend = unsafe { Backend::from_foreign_display(fixture.host.backend().display_ptr()) };
    let mut guest = GuestStartup::new(
        Connection::from_backend(backend),
        control.clone(),
        ReceiveLimits {
            startup_timeout: Duration::from_millis(20),
            ..ReceiveLimits::default()
        },
        now,
        move || {
            callback_wakes.fetch_add(1, Ordering::SeqCst);
        },
    )
    .unwrap();
    let deadline = guest.state.startup_deadline;
    while Instant::now() < deadline {
        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }

    assert_eq!(guest.pump(now), Err(ReceiveError::Timeout));
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert!(
        guest.state.registry.is_none(),
        "an expired first pump must not issue GetRegistry"
    );
    assert_eq!(guest.pending_sync(), None);
    assert_eq!(guest.state.discovery_epoch, None);
    assert!(!guest.state.discovery_complete);
    assert!(wake_count.load(Ordering::SeqCst) > 0);
}
