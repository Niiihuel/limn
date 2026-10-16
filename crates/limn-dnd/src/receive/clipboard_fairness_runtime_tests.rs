//! Removing an in-flight seat must preserve the next continuously eligible seat.
//! This child uses the existing native fixture; it assigns no tag, phase or cursor.

use super::*;

fn seat_snapshot(case: &LiveOwnerCase, id: ClipboardStamp) -> TextSeatSnapshot {
    case.clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .find(|seat| seat.seat == id)
        .expect("the real surviving seat is still observed")
}

fn assert_same_pending_focus(
    before: TextSeatSnapshot,
    after: TextSeatSnapshot,
    endpoint: &SurfaceLease,
) {
    assert!(after.seat == before.seat);
    assert!(after.keyboard_generation == before.keyboard_generation);
    assert!(after.selection.unwrap().id == before.selection.unwrap().id);
    assert!(after.focus.unwrap().generation == before.focus.unwrap().generation);
    assert!(after.focus.unwrap().registration == Some(endpoint.id()));
    assert!(!after.focus.unwrap().sealed);
}

fn serve_one_prefix_with_host_marker(case: &mut LiveOwnerCase) -> usize {
    // This is the existing host's real wl_display.sync, on its own event queue.
    // The guest's preceding Sync is already flushed. A single server pass replies
    // to that prefix plus this marker; callbacks emitted after reading the reply
    // cannot be answered until the test explicitly serves the server again.
    let mut queue = case.fixture().host.new_event_queue::<HostSync>();
    let mut marker = HostSync::default();
    case.fixture().host.display().sync(&queue.handle(), ());
    case.fixture().host.flush().unwrap();
    case.fixture_mut().serve();
    let deadline = Instant::now() + WAIT;
    loop {
        case.fixture().host_read();
        queue.dispatch_pending(&mut marker).unwrap();
        if marker.0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the real host prefix marker did not arrive"
        );
        thread::yield_now();
    }
    case.observed.completed_pumps.load(Ordering::SeqCst)
}

#[test]
fn removing_x_with_its_clipboard_callback_in_flight_seals_waiting_a_before_b() {
    let observed = testing::Observations::default();
    let (entered, seal_entered) = mpsc::sync_channel(1);
    let (release, seal_release) = mpsc::sync_channel(1);
    let mut case = LiveOwnerCase::with_text_seats(
        testing::Hooks {
            observed: observed.clone(),
            before_clipboard_seal: Some((entered, seal_release)),
            ..Default::default()
        },
        ReceiveLimits::default(),
        TextClipboardLimits::default(),
        &[5],
    );
    let mut release = OpenOnDrop(Some(release));
    drive_text(&mut case, |case| {
        case.lifecycle.status() == ReceiveStatus::Ready && case.fixture().has_keyboard(0)
    });
    // Native hotplug fixes X,A,B order without assuming an order for an initial
    // registry snapshot. Each new real global is consumed before adding the next.
    for device in 1..3 {
        let global = case
            .fixture()
            .display
            .handle()
            .create_global::<ServerState, wl_seat::WlSeat, _>(5, ());
        case.fixture_mut().seat_globals.push(global);
        drive_text(&mut case, |case| case.fixture().has_keyboard(device));
    }
    assert_eq!(case.fixture().server.devices, 3);
    let (endpoint, _) = case.register(0, || {});
    let control = endpoint.control();
    drive_text(&mut case, |_| {
        let header = control.header();
        header.status == ReceiveStatus::Ready && header.armed
    });
    let epoch = control.header().epoch;

    // Identify the real X/A/B seat snapshots using successive native Selections
    // before any Enter. Registering an unfocused surface emits no clipboard seal.
    let mut seats = Vec::new();
    for (device, key) in [(0, 1001), (1, 1002), (2, 1003)] {
        case.fixture().text_selection(device, key, true, true);
        drive_text(&mut case, |case| {
            case.clipboard()
                .observed_seats()
                .into_iter()
                .flatten()
                .filter(|seat| seat.selection.is_some())
                .count()
                == device + 1
        });
        let seat = case
            .clipboard()
            .observed_seats()
            .into_iter()
            .flatten()
            .find(|seat| seat.selection.is_some() && !seats.contains(&seat.seat))
            .unwrap();
        assert!(seat.focus.is_none());
        seats.push(seat.seat);
    }
    assert_eq!(seats.len(), 3);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    assert!(matches!(
        seal_entered.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    for device in 0..3 {
        case.fixture_mut().keyboard_enter(device, 0);
    }
    case.fixture_mut().display.flush_clients().unwrap();
    case.fixture().host_read();
    wait(
        || case.fixture().host_read(),
        || {
            case.clipboard()
                .observed_seats()
                .into_iter()
                .flatten()
                .filter(|seat| {
                    seat.focus.is_some_and(|focus| {
                        focus.registration == Some(endpoint.id()) && !focus.sealed
                    })
                })
                .count()
                == 3
        },
    );
    let after_enters = observed.completed_pumps.load(Ordering::SeqCst);
    wait(
        || case.fixture().host_read(),
        || observed.completed_pumps.load(Ordering::SeqCst) > after_enters,
    );
    // Initial round-robin order is X,A,B. No reply has been served since Enter;
    // the actual first X Sync is flushed and occupies the sole guest channel.
    let a_before = seat_snapshot(&case, seats[1]);
    let b_before = seat_snapshot(&case, seats[2]);
    for seat in [a_before, b_before] {
        assert!(seat.focus.unwrap().registration == Some(endpoint.id()));
        assert!(!seat.focus.unwrap().sealed);
    }
    assert_eq!(control.header().epoch, epoch);
    assert!(case.lifecycle.incoming().is_none());
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);

    // Real registry removal is queued BEFORE the server answers the already
    // emitted X Sync. Its physical Done must become a logical tombstone.
    let x_global = case.fixture().seat_globals[0].clone();
    case.fixture()
        .display
        .handle()
        .disable_global::<ServerState>(x_global);
    let old_prefix_pump = serve_one_prefix_with_host_marker(&mut case);
    wait(
        || case.fixture().host_read(),
        || observed.completed_pumps.load(Ordering::SeqCst) > old_prefix_pump,
    );
    assert!(case
        .clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .all(|seat| seat.seat != seats[0]));
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    assert!(matches!(
        seal_entered.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    assert_same_pending_focus(a_before, seat_snapshot(&case, seats[1]), &endpoint);
    assert_same_pending_focus(b_before, seat_snapshot(&case, seats[2]), &endpoint);
    assert_eq!(control.header().epoch, epoch);
    assert_eq!(control.status(), ReceiveStatus::Ready);
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert!(case.lifecycle.incoming().is_none());

    // Exactly one more guest callback is answered. Existing gate reports that
    // exact physical callback's original deadline after matching its full tag.
    serve_one_prefix_with_host_marker(&mut case);
    let emitted_deadline = seal_entered
        .recv_timeout(WAIT)
        .expect("the first actual survivor callback reaches its pre-seal gate");
    assert!(
        Instant::now() < emitted_deadline,
        "the survivor callback is handled before its own original deadline"
    );
    assert_same_pending_focus(a_before, seat_snapshot(&case, seats[1]), &endpoint);
    assert_same_pending_focus(b_before, seat_snapshot(&case, seats[2]), &endpoint);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    let survivor_pump = observed.completed_pumps.load(Ordering::SeqCst);
    release.release();
    // Stop at A OR B, never wait specifically for A and hide the wrong order
    // behind a generic timeout. The next survivor's reply is still withheld.
    wait(
        || case.fixture().host_read(),
        || {
            let a = seat_snapshot(&case, seats[1]);
            let b = seat_snapshot(&case, seats[2]);
            observed.completed_pumps.load(Ordering::SeqCst) > survivor_pump
                && observed.clipboard_seals.load(Ordering::SeqCst) == 1
                && (a.focus.unwrap().sealed || b.focus.unwrap().sealed)
        },
    );
    let a_first = seat_snapshot(&case, seats[1]);
    let b_first = seat_snapshot(&case, seats[2]);
    assert_eq!(
        (a_first.focus.unwrap().sealed, b_first.focus.unwrap().sealed),
        (true, false),
        "removing X cannot skip continuously waiting A and seal B first"
    );
    assert!(a_first.focus.unwrap().generation == a_before.focus.unwrap().generation);
    assert!(a_first.selection.unwrap().id == a_before.selection.unwrap().id);
    assert!(a_first.keyboard_generation == a_before.keyboard_generation);
    assert!(a_first.focus.unwrap().registration == Some(endpoint.id()));
    assert_same_pending_focus(b_before, b_first, &endpoint);
    assert_eq!(control.header().epoch, epoch);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 1);
    assert_eq!(case.fixture().server.released_devices, 1);
    assert_eq!(case.fixture().server.released_seats, 1);
    assert_eq!(case.fixture().server.released_keyboards, 1);
    assert_eq!(
        case.fixture()
            .server
            .requests
            .iter()
            .filter(|request| matches!(request, OfferRequest::Destroy(1001)))
            .count(),
        1
    );
    assert!(case
        .fixture()
        .server
        .requests
        .iter()
        .all(|request| !matches!(request, OfferRequest::Destroy(1002 | 1003))));

    drive_text(&mut case, |case| {
        observed.clipboard_seals.load(Ordering::SeqCst) == 2
            && seat_snapshot(case, seats[1]).focus.unwrap().sealed
            && seat_snapshot(case, seats[2]).focus.unwrap().sealed
    });
    assert_eq!(control.header().epoch, epoch);
    assert!(case.lifecycle.incoming().is_none());
    assert!(observed.watched_pipe.lock().unwrap().is_none());
    for key in 1001..=1003 {
        assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);
        assert_no_dnd_requests(&case, key);
    }
    assert_eq!(control.status(), ReceiveStatus::Ready);
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert!(!endpoint.is_detached());
    case.close();
    assert!(endpoint.is_detached());
    host_still_works(case.fixture_mut());
    assert_eq!(case.fixture().server.released_keyboards, 3);
    assert!(case.fixture().server.keyboards.is_empty());
}
