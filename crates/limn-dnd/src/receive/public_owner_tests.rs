// Wire only as a child of receiver_api_tests; consumer operations stay public.
use super::*;
use crate::{EndpointStatus, FileDropEndpoint, WaylandDataOwner};

struct FixtureDisplay {
    connection: Connection,
}

// This actual display owner deliberately does not implement HasWindowHandle.
impl HasDisplayHandle for FixtureDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let display = NonNull::new(self.connection.backend().display_ptr().cast())
            .ok_or(HandleError::Unavailable)?;
        // The retained connection owns this real private native display.
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                display,
            )))
        })
    }
}

struct PublicOwnerCase {
    owner: Option<WaylandDataOwner>,
    fixture: Option<Fixture>,
    display: Weak<FixtureDisplay>,
    wakes: Arc<AtomicUsize>,
}

impl PublicOwnerCase {
    fn new() -> Self {
        let mut fixture = Fixture::new(Some(3), &[5]);
        fixture.create_surfaces();
        let display = Arc::new(FixtureDisplay {
            connection: fixture.host.clone(),
        });
        let weak = Arc::downgrade(&display);
        let wakes = Arc::new(AtomicUsize::new(0));
        let callback_wakes = wakes.clone();
        let calls = Cell::new(0_u64); // Fn + Send, intentionally not Sync.
                                      // Fixture, host reader and this exact connection stay alive until the
                                      // public owner confirms cleanup and actual worker join with Closed.
        let owner = unsafe {
            WaylandDataOwner::attach_owned(display, ReceiveLimits::default(), move || {
                calls.set(calls.get().wrapping_add(1));
                callback_wakes.fetch_add(1, Ordering::SeqCst);
            })
        }
        .expect("the actual public Wayland display is admitted")
        .expect("the display is Wayland");
        Self {
            owner: Some(owner),
            fixture: Some(fixture),
            display: weak,
            wakes,
        }
    }

    fn owner(&self) -> &WaylandDataOwner {
        self.owner.as_ref().unwrap()
    }
    fn fixture(&self) -> &Fixture {
        self.fixture.as_ref().unwrap()
    }
    fn fixture_mut(&mut self) -> &mut Fixture {
        self.fixture.as_mut().unwrap()
    }

    fn step(&mut self) {
        self.fixture_mut().serve();
        self.fixture().host_read();
    }

    fn until(&mut self, mut done: impl FnMut(&mut Self) -> bool) {
        let deadline = Instant::now() + WAIT;
        loop {
            self.step();
            if done(self) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the actual public owner made no progress"
            );
            thread::yield_now();
        }
    }

    fn register(&self, surface: usize) -> (FileDropEndpoint, Weak<FixtureWindow>) {
        let window = Arc::new(FixtureWindow {
            connection: self.fixture().host.clone(),
            surface: self.fixture().host_surfaces[surface].clone(),
            dropped: None,
            observed: Default::default(),
        });
        let weak = Arc::downgrade(&window);
        let calls = Cell::new(0_u64); // Each endpoint owns an independent Send wake.
                                      // Both actual handles belong to this owner's same live connection.
        let endpoint = unsafe {
            self.owner().register_surface_owned(window, move || {
                calls.set(calls.get().wrapping_add(1));
            })
        }
        .expect("the actual surface registers with its public owner");
        (endpoint, weak)
    }

    fn start(
        &mut self,
        endpoint: &FileDropEndpoint,
        surface: usize,
        key: usize,
        point: SurfacePoint,
    ) -> (OfferId, ReceiveEpoch, OwnedFd) {
        let offer = self.fixture().offer(0, key, "text/uri-list");
        offer.action(wl_data_device_manager::DndAction::Copy);
        self.fixture().enter(0, &offer, surface, point.x, point.y);
        self.fixture().server.device_resources[0]
            .send_event(wl_data_device::Event::Drop {})
            .unwrap();
        let mut events = Vec::new();
        self.until(|case| {
            events.extend(endpoint.poll());
            case.fixture().received(key) == 1
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
        assert_eq!(started.len(), 1);
        let (id, epoch, actual_point) = started[0];
        assert_eq!(actual_point, point);
        assert!(events.iter().all(|event| matches!(event,
            ReceiveEvent::Hovered { id: actual, .. } | ReceiveEvent::DropStarted { id: actual, .. } if *actual == id)));
        (id, epoch, self.fixture_mut().take_writer(key))
    }
}

impl Drop for PublicOwnerCase {
    fn drop(&mut self) {
        self.owner().close();
        let deadline = Instant::now() + WAIT;
        while !matches!(self.owner().status(), ReceiveStatus::Closed(_)) {
            let progress = catch_unwind(AssertUnwindSafe(|| self.step()));
            if progress.is_err() || Instant::now() >= deadline {
                // Never destroy an unconfirmed foreign display during unwind.
                if let Some(fixture) = self.fixture.take() {
                    std::mem::forget(fixture);
                }
                return;
            }
            thread::yield_now();
        }
        drop(self.owner.take());
    }
}

#[derive(Default)]
struct PublicHostSync(bool);

impl client::Dispatch<client::protocol::wl_callback::WlCallback, ()> for PublicHostSync {
    fn event(
        state: &mut Self,
        _: &client::protocol::wl_callback::WlCallback,
        _: client::protocol::wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &client::QueueHandle<Self>,
    ) {
        state.0 = true;
    }
}

#[test]
fn public_display_owner_receives_two_surfaces_and_preserves_host_until_actual_close() {
    let mut case = PublicOwnerCase::new();
    case.until(|case| case.owner().status() == ReceiveStatus::Ready);
    let (a, a_window) = case.register(0);
    let (b, b_window) = case.register(1);
    // No server dispatch or host read occurred since admission, so no earlier
    // Ready callback can race this first invalidation in the private fixture.
    assert_eq!(a.status(), EndpointStatus::Registering);
    let a_epoch = a.invalidate_pending().unwrap();
    case.until(|_| a.status() == EndpointStatus::Ready && b.status() == EndpointStatus::Ready);
    assert_eq!(case.fixture().server.devices, 1);
    let (a_id, started_epoch, writer) = case.start(&a, 0, 71, SurfacePoint { x: 41.0, y: 73.0 });
    assert_eq!(started_epoch, a_epoch);
    assert!(b.poll().is_empty());
    let mut remaining = DATA;
    while !remaining.is_empty() {
        let count = rustix::io::write(&writer, remaining).unwrap();
        assert!(count > 0);
        remaining = &remaining[count..];
    }
    for _ in 0..3 {
        case.step();
        assert!(
            a.poll().is_empty(),
            "an open source writer cannot publish Ready"
        );
    }
    assert_eq!(case.fixture().finish_count(71), 0);
    drop(writer);
    let mut ready = Vec::new();
    case.until(|_| {
        ready.extend(a.poll());
        ready
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
    });
    assert!(
        matches!(ready.as_slice(), [ReceiveEvent::DropReady { id, paths }]
        if *id == a_id && *paths == vec![PathBuf::from("/tmp/receiver-a.txt"), PathBuf::from("/tmp/receiver-b.txt")])
    );
    assert_eq!(case.fixture().finish_count(71), 0);
    a.decide(a_id, DropDecision::Accept).unwrap();
    let mut ended = Vec::new();
    case.until(|_| {
        ended.extend(a.poll());
        ended
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
    });
    assert!(
        matches!(ended.as_slice(), [ReceiveEvent::DropEnded { id, result: Ok(()) }] if *id == a_id)
    );
    case.until(|case| case.fixture().finish_count(71) == 1);
    assert_eq!(
        a.decide(a_id, DropDecision::Accept),
        Err(ReceiveError::StaleOffer)
    );
    a.cancel(a_id);
    assert!(a.poll().is_empty());
    assert!(b.poll().is_empty());

    a.detach();
    case.until(|_| a.status() == EndpointStatus::Detached(None));
    assert!(a_window.upgrade().is_none());
    assert_eq!(case.owner().status(), ReceiveStatus::Ready);
    assert_eq!(b.status(), EndpointStatus::Ready);
    let (fresh_a, fresh_window) = case.register(0);
    // The same paused server/reader guarantees this is still first registration.
    assert_eq!(fresh_a.status(), EndpointStatus::Registering);
    fresh_a.invalidate_pending().unwrap();
    case.until(|_| fresh_a.status() == EndpointStatus::Ready);
    // The old detached control cannot invalidate the new registration's barrier.
    assert_eq!(a.invalidate_pending(), Err(ReceiveError::Closed));
    assert_eq!(fresh_a.status(), EndpointStatus::Ready);
    assert!(fresh_a.poll().is_empty());

    let (b_id, _, writer_b) = case.start(&b, 1, 72, SurfacePoint { x: 31.0, y: 51.0 });
    assert_ne!(a_id, b_id);
    assert!(fresh_a.poll().is_empty());
    b.cancel(b_id);
    let mut cancelled = Vec::new();
    case.until(|_| {
        cancelled.extend(b.poll());
        cancelled
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
    });
    assert!(
        matches!(cancelled.as_slice(), [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) }] if *id == b_id)
    );
    assert!(a.poll().is_empty());
    assert_eq!(case.fixture().finish_count(72), 0);
    drop(writer_b);
    case.owner().close();
    case.until(|case| case.owner().status() == ReceiveStatus::Closed(None));
    assert_eq!(fresh_a.status(), EndpointStatus::Detached(None));
    assert_eq!(b.status(), EndpointStatus::Detached(None));
    assert_eq!(a.status(), EndpointStatus::Detached(None));
    assert!(case.display.upgrade().is_none());
    assert!(fresh_window.upgrade().is_none());
    assert!(b_window.upgrade().is_none());
    assert!(case.wakes.load(Ordering::SeqCst) > 0);
    assert_eq!(case.fixture().finish_count(71), 1);
    assert_eq!(case.fixture().server.devices, 1);

    // This independent host queue still receives an actual protocol sync after
    // Closed, and every borrowed host surface remains alive.
    let mut queue = case.fixture().host.new_event_queue::<PublicHostSync>();
    let mut alive = PublicHostSync::default();
    case.fixture().host.display().sync(&queue.handle(), ());
    case.fixture().host.flush().unwrap();
    case.until(|_| {
        queue.dispatch_pending(&mut alive).unwrap();
        alive.0
    });
    assert_eq!(
        case.fixture().server.released_devices,
        case.fixture().server.devices
    );
    assert_eq!(
        case.fixture().server.released_seats,
        case.fixture().server.seat_versions.len()
    );
    assert!(case
        .fixture()
        .server
        .surfaces
        .iter()
        .all(|surface| surface.is_alive()));
}
