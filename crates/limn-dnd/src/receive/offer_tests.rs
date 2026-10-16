use super::super::super::{coordinator::Phase, OfferId};
use super::super::allocate_offer_id;
use super::*;
use crate::{ReceiveEvent, SurfacePoint};
use sctk::reexports::client::{self as client, Proxy};
use std::{
    os::fd::OwnedFd,
    sync::{atomic::AtomicU64, Mutex},
};
use wayland_server::{protocol::wl_data_offer, WEnum};

#[derive(Debug)]
pub(super) enum OfferRequest {
    Accept(usize, Option<String>),
    Actions(
        usize,
        WEnum<wl_data_device_manager::DndAction>,
        WEnum<wl_data_device_manager::DndAction>,
    ),
    Receive(usize, String, OwnedFd),
    ReceiveTaken(usize, String),
    Destroy(usize),
    Finish(usize),
}

impl GlobalDispatch<wl_compositor::WlCompositor, ()> for ServerState {
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

impl Dispatch<wl_compositor::WlCompositor, ()> for ServerState {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_compositor::WlCompositor,
        request: wl_compositor::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => state.surfaces.push(init.init(id, ())),
            _ => panic!("private surface fixture requested an unexpected compositor object"),
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for ServerState {
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

impl Dispatch<wl_data_offer::WlDataOffer, usize> for ServerState {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &wl_data_offer::WlDataOffer,
        request: wl_data_offer::Request,
        key: &usize,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        state.requests.push(match request {
            wl_data_offer::Request::Accept { mime_type, .. } => {
                OfferRequest::Accept(*key, mime_type)
            }
            wl_data_offer::Request::SetActions {
                dnd_actions,
                preferred_action,
            } => OfferRequest::Actions(*key, dnd_actions, preferred_action),
            wl_data_offer::Request::Receive { mime_type, fd } => {
                OfferRequest::Receive(*key, mime_type, fd)
            }
            wl_data_offer::Request::Destroy => OfferRequest::Destroy(*key),
            wl_data_offer::Request::Finish => OfferRequest::Finish(*key),
            _ => panic!("receive-only fixture observed an unexpected offer request"),
        });
    }
}

#[derive(Default)]
struct SurfaceHost {
    surfaces: Vec<client::protocol::wl_surface::WlSurface>,
}

impl client::Dispatch<client::protocol::wl_registry::WlRegistry, ()> for SurfaceHost {
    fn event(
        state: &mut Self,
        registry: &client::protocol::wl_registry::WlRegistry,
        event: client::protocol::wl_registry::Event,
        _: &(),
        _: &Connection,
        handle: &client::QueueHandle<Self>,
    ) {
        if let client::protocol::wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            if interface == "wl_compositor" {
                let compositor = registry
                    .bind::<client::protocol::wl_compositor::WlCompositor, _, _>(
                        name,
                        1,
                        handle,
                        (),
                    );
                state.surfaces.push(compositor.create_surface(handle, ()));
                state.surfaces.push(compositor.create_surface(handle, ()));
            }
        }
    }
}

client::delegate_noop!(SurfaceHost: client::protocol::wl_compositor::WlCompositor);
client::delegate_noop!(SurfaceHost: ignore client::protocol::wl_surface::WlSurface);

impl Fixture {
    fn create_surfaces(&mut self) {
        let mut queue = self.host.new_event_queue::<SurfaceHost>();
        let mut state = SurfaceHost::default();
        self.host.display().get_registry(&queue.handle(), ());
        for _ in 0..32 {
            self.host.flush().unwrap();
            self.serve();
            self.host_read();
            queue.dispatch_pending(&mut state).unwrap();
            if state.surfaces.len() == 2 {
                self.host.flush().unwrap();
                self.serve();
                self.host_surfaces = state.surfaces;
                assert_eq!(self.server.surfaces.len(), 2);
                return;
            }
        }
        panic!("private host did not create its two surfaces");
    }

    fn introduce_offer(&self, device: usize, key: usize) -> wl_data_offer::WlDataOffer {
        let device = &self.server.device_resources[device];
        let offer = device
            .client()
            .unwrap()
            .create_resource::<wl_data_offer::WlDataOffer, _, ServerState>(
                &self.display.handle(),
                3,
                key,
            )
            .unwrap();
        device.data_offer(&offer);
        offer
    }

    fn offer(&self, device: usize, key: usize, mime: &str) -> wl_data_offer::WlDataOffer {
        let offer = self.introduce_offer(device, key);
        offer.offer(mime.to_owned());
        offer.source_actions(
            wl_data_device_manager::DndAction::Copy | wl_data_device_manager::DndAction::Move,
        );
        offer
    }

    fn enter(
        &self,
        device: usize,
        offer: &wl_data_offer::WlDataOffer,
        surface: usize,
        x: f64,
        y: f64,
    ) {
        self.server.device_resources[device].enter(
            10,
            &self.server.surfaces[surface],
            x,
            y,
            Some(offer),
        );
    }

    fn deliver(&mut self, guest: &mut GuestStartup, now: Instant) -> Result<(), ReceiveError> {
        self.display.flush_clients().unwrap();
        self.host_read();
        guest.pump(now)
    }

    fn received(&self, key: usize) -> usize {
        self.server
            .requests
            .iter()
            .filter(|request| match request {
                OfferRequest::Receive(actual, mime, _)
                | OfferRequest::ReceiveTaken(actual, mime)
                    if *actual == key =>
                {
                    assert_eq!(mime, "text/uri-list", "unexpected receive MIME");
                    true
                }
                _ => false,
            })
            .count()
    }

    fn destroyed(&self, key: usize) -> bool {
        self.server
            .requests
            .iter()
            .any(|request| matches!(request, OfferRequest::Destroy(actual) if *actual == key))
    }

    fn finished(&self) -> bool {
        self.server
            .requests
            .iter()
            .any(|request| matches!(request, OfferRequest::Finish(_)))
    }
}

#[path = "transfer_tests.rs"]
mod transfers;

#[path = "runtime_tests.rs"]
mod runtime;

fn ready_fixture() -> (Fixture, GuestStartup, ReceiveControl, Instant) {
    let mut fixture = Fixture::new(Some(3), &[5, 5]);
    fixture.create_surfaces();
    let control = control();
    let now = Instant::now();
    // Both host surfaces and their actual display remain owned by the returned fixture.
    let backend = unsafe { Backend::from_foreign_display(fixture.host.backend().display_ptr()) };
    let mut guest = GuestStartup::for_surface(
        Connection::from_backend(backend),
        fixture.host_surfaces[0].id(),
        control.clone(),
        ReceiveLimits::default(),
        now,
        || {},
    )
    .unwrap();
    fixture.until_ready(&mut guest, &control, now);
    (fixture, guest, control, now)
}

fn hovered(control: &ReceiveControl) -> OfferId {
    let events = control.poll();
    match events.as_slice() {
        [ReceiveEvent::Hovered { id, .. }] => *id,
        _ => panic!("expected one exact hover, got {events:?}"),
    }
}

fn copy_enter(
    fixture: &mut Fixture,
    guest: &mut GuestStartup,
    control: &ReceiveControl,
    device: usize,
    key: usize,
    now: Instant,
) -> (wl_data_offer::WlDataOffer, OfferId) {
    let offer = fixture.offer(device, key, "text/uri-list");
    offer.offer("text/plain".to_owned());
    fixture.enter(device, &offer, 0, 10.0, 20.0);
    offer.action(wl_data_device_manager::DndAction::Copy);
    fixture.deliver(guest, now).unwrap();
    (offer, hovered(control))
}

#[test]
fn offer_ids_never_wrap_or_reuse_an_exhausted_private_counter() {
    let counter = AtomicU64::new(u64::MAX - 1);
    assert_eq!(allocate_offer_id(&counter), Ok(OfferId(u64::MAX - 1)));
    assert_eq!(
        allocate_offer_id(&counter),
        Err(ReceiveError::ResourceLimit)
    );
    assert_eq!(
        allocate_offer_id(&counter),
        Err(ReceiveError::ResourceLimit)
    );
    assert_eq!(counter.load(Ordering::SeqCst), u64::MAX);
}

#[test]
fn concurrent_offer_ids_use_one_checked_private_sequence() {
    let counter = Arc::new(AtomicU64::new(1));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let counter = counter.clone();
            std::thread::spawn(move || {
                (0..16)
                    .map(|_| allocate_offer_id(&counter).unwrap())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut ids: Vec<_> = workers
        .into_iter()
        .flat_map(|worker| worker.join().unwrap())
        .map(|id| id.0)
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, (1..=64).collect::<Vec<_>>());
    assert_eq!(counter.load(Ordering::SeqCst), 65);
}

#[test]
fn selection_and_queued_mime_callbacks_are_discarded_without_payload_requests() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let offer = fixture.offer(0, 1, "text/uri-list");
    fixture.server.device_resources[0].selection(Some(&offer));
    fixture.deliver(&mut guest, now).unwrap();
    fixture.serve();
    assert!(control.poll().is_empty());
    assert!(fixture
        .server
        .requests
        .iter()
        .any(|request| matches!(request, OfferRequest::Destroy(1))));
    assert_eq!(fixture.received(1), 0);
    assert!(!fixture.finished());
    assert_eq!(control.status(), ReceiveStatus::Ready);
}

#[test]
fn foreign_surfaces_and_non_file_mimes_reject_only_their_exact_offers() {
    for (surface, mime) in [(1, "text/uri-list"), (0, "text/plain")] {
        let (mut fixture, mut guest, control, now) = ready_fixture();
        let offer = fixture.offer(0, 1, mime);
        fixture.enter(0, &offer, surface, 10.0, 20.0);
        fixture.deliver(&mut guest, now).unwrap();
        fixture.serve();
        assert!(control.poll().is_empty());
        assert!(fixture
            .server
            .requests
            .iter()
            .any(|request| matches!(request, OfferRequest::Accept(1, None))));
        // Destroying a hovered v3 offer cancels the whole drag on wlroots.
        assert!(!fixture.destroyed(1), "a hovered offer survives until Leave");
        fixture.server.device_resources[0].leave();
        fixture.deliver(&mut guest, now).unwrap();
        fixture.serve();
        assert!(fixture.destroyed(1));
        assert_eq!(fixture.received(1), 0);
        assert!(!fixture.finished());
    }
}

#[test]
fn an_offer_refused_during_hover_is_destroyed_only_after_leave_or_drop() {
    use wl_data_device_manager::DndAction;
    for drop_it in [false, true] {
        let (mut fixture, mut guest, control, now) = ready_fixture();
        let (_offer, id) = copy_enter(&mut fixture, &mut guest, &control, 0, 1, now);
        control.cancel(id).unwrap();
        guest.pump(now).unwrap();
        fixture.serve();
        assert!(guest.state.offer_phase(id).is_none());
        let refused = fixture.server.requests.iter().position(|request| {
            matches!(request, OfferRequest::Accept(1, None))
        });
        assert!(refused.is_some(), "the cancelled hover is refused in place");
        assert!(fixture.server.requests[refused.unwrap()..].iter().any(|request| matches!(
            request,
            OfferRequest::Actions(1, WEnum::Value(DndAction::None), WEnum::Value(DndAction::None))
        )));
        assert!(!fixture.destroyed(1), "a hovered offer survives until Leave or Drop");
        let device = fixture.server.device_resources[0].clone();
        device.motion(11, 30.0, 40.0);
        fixture.deliver(&mut guest, now).unwrap();
        fixture.serve();
        assert!(!fixture.destroyed(1));
        if drop_it {
            device.send_event(wl_data_device::Event::Drop {}).unwrap();
        } else {
            device.leave();
        }
        fixture.deliver(&mut guest, now).unwrap();
        fixture.serve();
        assert!(fixture.destroyed(1));
        assert_eq!(fixture.received(1), 0);
        assert!(!fixture.finished());
        assert!(guest.state.offers.is_empty());
        control.poll();
        let (_, next) = copy_enter(&mut fixture, &mut guest, &control, 1, 2, now);
        assert_ne!(id, next);
    }
}

#[test]
fn source_copy_support_does_not_replace_an_exact_selected_copy_action() {
    use wl_data_device_manager::DndAction;
    for action in [
        None,
        Some(WEnum::Value(DndAction::None)),
        Some(WEnum::Value(DndAction::Move)),
        Some(WEnum::Value(DndAction::Ask)),
        Some(WEnum::Value(DndAction::Copy | DndAction::Move)),
        Some(WEnum::Unknown(99)),
    ] {
        let (mut fixture, mut guest, control, now) = ready_fixture();
        let offer = fixture.offer(0, 1, "text/uri-list");
        fixture.enter(0, &offer, 0, 10.0, 20.0);
        if let Some(dnd_action) = action {
            offer
                .send_event(wl_data_offer::Event::Action { dnd_action })
                .unwrap();
        }
        fixture.deliver(&mut guest, now).unwrap();
        let id = hovered(&control);
        fixture.server.device_resources[0]
            .send_event(wl_data_device::Event::Drop {})
            .unwrap();
        fixture.deliver(&mut guest, now).unwrap();
        fixture.serve();
        assert_eq!(fixture.received(1), 0);
        assert!(fixture
            .server
            .requests
            .iter()
            .any(|request| matches!(request, OfferRequest::Destroy(1))));
        assert!(control.poll().iter().all(|event| !matches!(
            event,
            ReceiveEvent::DropStarted { .. } | ReceiveEvent::DropEnded { .. }
        )));
        assert!(guest.state.offer_phase(id).is_none());
        let (_, next) = copy_enter(&mut fixture, &mut guest, &control, 1, 2, now);
        assert_ne!(id, next);
    }
}

#[test]
fn pre_enter_selected_copy_survives_mime_until_explicit_accept() {
    use wl_data_device_manager::DndAction;

    for actions in [
        vec![DndAction::Copy],
        vec![DndAction::Move, DndAction::Copy],
    ] {
        let (mut fixture, mut guest, control, now) = ready_fixture();
        let offer = fixture.introduce_offer(0, 1);
        offer.source_actions(DndAction::Copy | DndAction::Move);
        for action in actions {
            offer.action(action);
        }
        offer.offer("text/uri-list".into());
        offer.offer("text/plain".into());
        fixture.enter(0, &offer, 0, 10.0, 20.0);
        fixture.deliver(&mut guest, now).unwrap();
        let id = hovered(&control);

        fixture.server.device_resources[0]
            .send_event(wl_data_device::Event::Drop {})
            .unwrap();
        fixture.deliver(&mut guest, now).unwrap();
        assert!(
            matches!(control.poll().as_slice(), [ReceiveEvent::DropStarted { id: actual, point, .. }]
                if *actual == id && *point == SurfacePoint { x: 10.0, y: 20.0 }),
            "the exact selected Copy before Enter must start the drop"
        );
        fixture.serve();
        assert_eq!(fixture.received(1), 1);
        assert!(!fixture.finished());

        let writer = fixture.take_writer(1);
        let payload = b"file:///tmp/received-before-enter\r\n";
        assert_eq!(rustix::io::write(&writer, payload).unwrap(), payload.len());
        drop(writer);
        guest.pump(now).unwrap();
        assert!(control.poll().is_empty(), "a prefix cannot publish Ready");
        guest.pump(now).unwrap();
        assert!(
            matches!(control.poll().as_slice(), [ReceiveEvent::DropReady { id: actual, paths }]
            if *actual == id && *paths == vec![std::path::PathBuf::from("/tmp/received-before-enter")])
        );
        fixture.serve();
        assert!(!fixture.finished(), "EOF does not authorize finish");

        control.decide(id, crate::DropDecision::Accept).unwrap();
        guest.pump(now).unwrap();
        assert!(
            matches!(control.poll().as_slice(), [ReceiveEvent::DropEnded { id: actual, result: Ok(()) }]
            if *actual == id)
        );
        fixture.serve();
        assert_eq!(fixture.finish_count(1), 1);
    }
}

#[test]
fn pre_enter_actions_other_than_copy_never_request_or_finish_payload() {
    use wl_data_device_manager::DndAction;

    for actions in [
        vec![],
        vec![WEnum::Value(DndAction::Move)],
        vec![WEnum::Value(DndAction::Ask)],
        vec![WEnum::Value(DndAction::None)],
        vec![WEnum::Unknown(99)],
        vec![WEnum::Value(DndAction::Copy | DndAction::Move)],
        vec![WEnum::Value(DndAction::Copy), WEnum::Value(DndAction::Move)],
    ] {
        let (mut fixture, mut guest, control, now) = ready_fixture();
        let offer = fixture.introduce_offer(0, 1);
        offer.source_actions(DndAction::Copy | DndAction::Move);
        for dnd_action in actions {
            offer
                .send_event(wl_data_offer::Event::Action { dnd_action })
                .unwrap();
        }
        offer.offer("text/uri-list".into());
        fixture.enter(0, &offer, 0, 10.0, 20.0);
        fixture.deliver(&mut guest, now).unwrap();
        let id = hovered(&control);
        fixture.server.device_resources[0]
            .send_event(wl_data_device::Event::Drop {})
            .unwrap();
        fixture.deliver(&mut guest, now).unwrap();
        fixture.serve();
        assert_eq!(fixture.received(1), 0);
        assert!(!fixture.finished());
        assert_eq!(
            fixture
                .server
                .requests
                .iter()
                .filter(|request| matches!(request, OfferRequest::Destroy(1)))
                .count(),
            1
        );
        assert!(control.poll().iter().all(|event| matches!(event,
            ReceiveEvent::Left { id: actual } if *actual == id)));
        assert!(guest.state.offer_phase(id).is_none());
        let (_, next) = copy_enter(&mut fixture, &mut guest, &control, 1, 2, now);
        assert_ne!(
            id, next,
            "a rejected pre-Enter action releases only its offer"
        );
    }
}

#[test]
fn copy_drop_freezes_motion_and_survives_leave_while_receive_is_pending() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_offer, id) = copy_enter(&mut fixture, &mut guest, &control, 0, 1, now);
    let device = fixture.server.device_resources[0].clone();
    device.motion(11, 42.0, 51.0);
    device.send_event(wl_data_device::Event::Drop {}).unwrap();
    device.leave();
    fixture.deliver(&mut guest, now).unwrap();
    let events = control.poll();
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropStarted { id: actual, point, .. }] if *actual == id && *point == SurfacePoint { x: 42.0, y: 51.0 })
    );
    assert!(matches!(
        guest.state.offer_phase(id),
        Some(Phase::Reading { .. })
    ));
    // Started was observed before the private server processes the Receive request.
    assert_eq!(fixture.received(1), 0);
    fixture.serve();
    assert_eq!(fixture.received(1), 1);
    assert!(fixture.server.requests.iter().any(
        |request| matches!(request, OfferRequest::Accept(1, Some(mime)) if mime == "text/uri-list")
    ));
    assert!(fixture.server.requests.iter().any(|request| matches!(request, OfferRequest::Actions(1, WEnum::Value(actions), WEnum::Value(preferred)) if *actions == wl_data_device_manager::DndAction::Copy && *preferred == wl_data_device_manager::DndAction::Copy)));
    for request in &fixture.server.requests {
        if let OfferRequest::Receive(1, mime, writer) = request {
            assert_eq!(mime, "text/uri-list");
            assert!(!rustix::fs::fcntl_getfl(writer)
                .unwrap()
                .contains(rustix::fs::OFlags::NONBLOCK));
        }
    }
    assert!(!fixture.finished());
    control.cancel(id).unwrap();
    guest.pump(now).unwrap();
    let events = control.poll();
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropEnded { id: actual, result: Err(ReceiveError::Cancelled) }] if *actual == id)
    );
}

#[test]
fn busy_device_and_selection_do_not_replace_another_devices_reading_drop() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_, a) = copy_enter(&mut fixture, &mut guest, &control, 0, 1, now);
    fixture.server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    fixture.deliver(&mut guest, now).unwrap();
    assert!(
        matches!(control.poll().as_slice(), [ReceiveEvent::DropStarted { id, .. }] if *id == a)
    );
    let b = fixture.offer(1, 2, "text/uri-list");
    fixture.enter(1, &b, 0, 500.0, 600.0);
    b.action(wl_data_device_manager::DndAction::Move);
    let selection = fixture.offer(1, 3, "text/uri-list");
    fixture.server.device_resources[1].selection(Some(&selection));
    fixture.server.device_resources[1].leave();
    fixture.deliver(&mut guest, now).unwrap();
    fixture.serve();
    assert!(control.poll().is_empty());
    assert!(matches!(
        guest.state.offer_phase(a),
        Some(Phase::Reading { .. })
    ));
    assert_eq!(fixture.received(1), 1);
    assert_eq!(fixture.received(2), 0);
    assert_eq!(fixture.received(3), 0);
    assert!(fixture
        .server
        .requests
        .iter()
        .any(|request| matches!(request, OfferRequest::Accept(2, None))));
    assert!(fixture
        .server
        .requests
        .iter()
        .any(|request| matches!(request, OfferRequest::Destroy(2))));
    assert!(fixture
        .server
        .requests
        .iter()
        .any(|request| matches!(request, OfferRequest::Destroy(3))));
    assert!(!fixture
        .server
        .requests
        .iter()
        .any(|request| matches!(request, OfferRequest::Destroy(1))));
}

#[test]
fn queued_callbacks_for_a_departed_offer_do_not_change_the_new_device_offer() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (old, a) = copy_enter(&mut fixture, &mut guest, &control, 0, 1, now);
    fixture.server.device_resources[0].leave();
    let b = fixture.offer(1, 2, "text/uri-list");
    fixture.enter(1, &b, 0, 30.0, 40.0);
    b.action(wl_data_device_manager::DndAction::Copy);
    old.action(wl_data_device_manager::DndAction::Move);
    fixture.deliver(&mut guest, now).unwrap();
    let b_id = hovered(&control);
    assert_ne!(a, b_id);
    fixture.server.device_resources[1]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    fixture.deliver(&mut guest, now).unwrap();
    assert!(
        matches!(control.poll().as_slice(), [ReceiveEvent::DropStarted { id, point, .. }] if *id == b_id && *point == SurfacePoint { x: 30.0, y: 40.0 })
    );
    fixture.serve();
    assert_eq!(fixture.received(1), 0);
    assert_eq!(fixture.received(2), 1);
}

#[test]
fn the_eight_offer_budget_includes_the_active_drop_across_all_devices() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_, a) = copy_enter(&mut fixture, &mut guest, &control, 0, 1, now);
    fixture.server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    fixture.deliver(&mut guest, now).unwrap();
    control.poll();
    for key in 2..=8 {
        fixture.offer(1, key, "text/uri-list");
    }
    fixture.deliver(&mut guest, now).unwrap();
    assert_eq!(control.status(), ReceiveStatus::Ready);
    assert!(matches!(
        guest.state.offer_phase(a),
        Some(Phase::Reading { .. })
    ));
    fixture.offer(0, 9, "text/uri-list");
    assert_eq!(
        fixture.deliver(&mut guest, now),
        Err(ReceiveError::ResourceLimit)
    );
    assert_eq!(control.status(), ReceiveStatus::Closing);
    let events = control.poll();
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::ResourceLimit) }] if *id == a)
    );
    fixture.serve();
    assert_eq!(fixture.received(1), 1);
    assert!((2..=9).all(|key| fixture.received(key) == 0));
    assert!(!fixture.finished());
}

#[test]
fn an_offer_invalidated_from_started_wake_fails_io_once_without_consuming_b() {
    let mut fixture = Fixture::new(Some(3), &[5, 5]);
    fixture.create_surfaces();
    let control = control();
    let now = Instant::now();
    let observed = Arc::new(Mutex::new(Vec::<ReceiveEvent>::new()));
    let killed = Arc::new(Mutex::new(
        None::<client::protocol::wl_data_offer::WlDataOffer>,
    ));
    let callback_control = control.clone();
    let callback_events = observed.clone();
    let callback_offer = killed.clone();
    // The fixture owns the real host display and both surfaces until guest cleanup.
    let backend = unsafe { Backend::from_foreign_display(fixture.host.backend().display_ptr()) };
    let mut guest = GuestStartup::for_surface(
        Connection::from_backend(backend),
        fixture.host_surfaces[0].id(),
        control.clone(),
        ReceiveLimits::default(),
        now,
        move || {
            let events = callback_control.poll();
            if events
                .iter()
                .any(|event| matches!(event, ReceiveEvent::DropStarted { .. }))
            {
                let offer = callback_offer
                    .lock()
                    .unwrap()
                    .clone()
                    .expect("exact A proxy was captured");
                offer
                    .send_request(client::protocol::wl_data_offer::Request::Destroy {})
                    .unwrap();
            }
            callback_events.lock().unwrap().extend(events);
        },
    )
    .unwrap();
    fixture.until_ready(&mut guest, &control, now);
    let a = fixture.offer(0, 1, "text/uri-list");
    fixture.enter(0, &a, 0, 10.0, 20.0);
    a.action(wl_data_device_manager::DndAction::Copy);
    fixture.deliver(&mut guest, now).unwrap();
    let a_id = observed
        .lock()
        .unwrap()
        .iter()
        .find_map(|event| match event {
            ReceiveEvent::Hovered { id, .. } => Some(*id),
            _ => None,
        })
        .expect("A hover was observed by the callback");
    *killed.lock().unwrap() = Some(
        guest
            .state
            .offer_proxy(a_id)
            .expect("A maps to its exact proxy"),
    );
    observed.lock().unwrap().clear();
    let b = fixture.offer(1, 2, "text/uri-list");
    fixture.deliver(&mut guest, now).unwrap();
    fixture.server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    fixture.deliver(&mut guest, now).unwrap();
    let events = std::mem::take(&mut *observed.lock().unwrap());
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropStarted { id: started, .. }, ReceiveEvent::DropEnded { id: ended, result: Err(ReceiveError::Io) }] if *started == a_id && *ended == a_id)
    );
    assert_eq!(control.status(), ReceiveStatus::Ready);
    fixture.serve();
    assert_eq!(fixture.received(1), 0);
    assert_eq!(fixture.received(2), 0);
    assert!(!fixture.finished());
    fixture.enter(1, &b, 0, 30.0, 40.0);
    b.action(wl_data_device_manager::DndAction::Copy);
    fixture.deliver(&mut guest, now).unwrap();
    let events = std::mem::take(&mut *observed.lock().unwrap());
    assert!(matches!(events.as_slice(), [ReceiveEvent::Hovered { id, .. }] if *id != a_id));
}

#[test]
fn a_started_callback_that_crosses_the_transfer_deadline_never_requests_payload() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    fixture.create_surfaces();
    let control = control();
    let now = Instant::now();
    let observed = Arc::new(Mutex::new(Vec::<ReceiveEvent>::new()));
    let callback_control = control.clone();
    let callback_events = observed.clone();
    // Both surfaces and their actual display remain owned by the fixture.
    let backend = unsafe { Backend::from_foreign_display(fixture.host.backend().display_ptr()) };
    let mut guest = GuestStartup::for_surface(
        Connection::from_backend(backend),
        fixture.host_surfaces[0].id(),
        control.clone(),
        ReceiveLimits {
            transfer_timeout: Duration::from_millis(5),
            ..ReceiveLimits::default()
        },
        now,
        move || {
            let events = callback_control.poll();
            for event in &events {
                if let ReceiveEvent::DropStarted { id, .. } = event {
                    let deadline = {
                        let header = callback_control.header();
                        let offer = header.offer.expect("Started has a published offer header");
                        assert_eq!(offer.id, *id);
                        offer
                            .deadline
                            .expect("Started has an absolute transfer deadline")
                    };
                    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
                        if remaining.is_zero() {
                            break;
                        }
                        std::thread::sleep(remaining);
                    }
                }
            }
            callback_events.lock().unwrap().extend(events);
        },
    )
    .unwrap();
    fixture.until_ready(&mut guest, &control, now);
    let offer = fixture.offer(0, 1, "text/uri-list");
    fixture.enter(0, &offer, 0, 10.0, 20.0);
    offer.action(wl_data_device_manager::DndAction::Copy);
    fixture.deliver(&mut guest, now).unwrap();
    observed.lock().unwrap().clear();
    fixture.server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    fixture.deliver(&mut guest, now).unwrap();
    let events = std::mem::take(&mut *observed.lock().unwrap());
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropStarted { id: started, .. }, ReceiveEvent::DropEnded { id: ended, result: Err(ReceiveError::Timeout) }] if started == ended)
    );
    assert!(control.poll().is_empty());
    assert_eq!(control.status(), ReceiveStatus::Ready);
    fixture.serve();
    assert!(!fixture.finished());
    assert_eq!(fixture.received(1), 0);
}

#[path = "receiver_api_tests.rs"]
mod receiver_api;

#[cfg(test)]
#[path = "owner_wayland_tests.rs"]
mod owner_wayland;
