// Wire as a child of owner_runtime_tests; all fixture and runtime hooks are real.
use super::*;
use crate::receive::control::IncomingLease;

#[test]
fn cancelled_live_endpoint_keeps_incoming_until_its_exact_runtime_watcher_is_gone() {
    let observed = testing::Observations::default();
    let (entered, retirement_entered) = mpsc::sync_channel(1);
    let (release, retirement_release) = mpsc::sync_channel(1);
    let mut case = LiveOwnerCase::new(testing::Hooks {
        observed: observed.clone(),
        before_retire_sync: Some((entered, retirement_release)),
        ..Default::default()
    });
    let mut release = OpenOnDrop(Some(release));
    let lifecycle = case.lifecycle.clone();
    case.drive(|_| lifecycle.status() == ReceiveStatus::Ready);

    let witness = Arc::new(OnceLock::<ReceiveControl>::new());
    let callback_witness = witness.clone();
    let callback_ping = case.ping.clone();
    let (terminal, terminals) = mpsc::channel();
    let (a, _) = case.register(0, move || {
        let Some(control) = callback_witness.get() else {
            return;
        };
        let ended = {
            let header = control.header();
            header
                .offer
                .is_some_and(|offer| matches!(offer.gate, AttemptGate::Ended { drained: false }))
        };
        if ended {
            // The actual terminal ACK reenters control from the worker wake.
            // Started is still consumed by LiveOwnerCase::start, not this wake.
            let _ = terminal.send(poll(control, &callback_ping));
        }
    });
    let a_control = a.control();
    assert!(witness.set(a_control.clone()).is_ok());
    let (b, _) = case.register(1, || {});
    let b_control = b.control();
    case.drive(|_| {
        a_control.status() == ReceiveStatus::Ready && b_control.status() == ReceiveStatus::Ready
    });
    let (a_id, writer_a) = case.start(0, 1, &a_control);
    let a_watcher_fd = watched_fd(&observed, a_id);

    a_control.cancel(a_id).unwrap();
    case.ping.ping();
    retirement_entered
        .recv_timeout(WAIT)
        .expect("the real worker pauses before removing A's obsolete Generic");
    let ended = terminals
        .recv_timeout(WAIT)
        .expect("A's wake drained its actual cancellation terminal");
    assert!(matches!(ended.as_slice(),
        [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) }] if *id == a_id));
    assert!(matches!(
        terminals.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));

    // The ACK has now been reconciled by the actual Coordinator in this pump.
    // Its absence alone cannot certify that calloop dropped the old read FD.
    assert!(a_control.header().offer.is_none());
    assert_eq!(a_control.status(), ReceiveStatus::Ready);
    assert!(!a.is_detached());
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert!(a_watcher_fd.upgrade().is_some());
    let exact = IncomingLease::Drop {
        registration: Some(a.id()),
        id: a_id,
    };
    assert!(case.lifecycle.incoming() == Some(exact));
    assert!(a_control.incoming() == Some(exact));
    assert!(b_control.incoming() == Some(exact));

    release.release();
    wait(
        || {},
        || a_watcher_fd.upgrade().is_none() && case.lifecycle.incoming().is_none(),
    );
    assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
    assert_eq!(a_control.status(), ReceiveStatus::Ready);
    assert!(!a.is_detached());
    let ping = case.ping.clone();
    assert!(poll(&a_control, &ping).is_empty());

    // A remains registered, and its source writer remains open while B uses
    // the same real device through Receive, EOF, decision and exactly one Finish.
    let (b_id, writer_b) = case.start(1, 2, &b_control);
    assert_ne!(a_id, b_id);
    let b_watcher_fd = watched_fd(&observed, b_id);
    assert!(b_watcher_fd.upgrade().is_some());
    write_chunk(&writer_b, DATA);
    drop(writer_b);
    let mut ready = Vec::new();
    case.drive(|_| {
        ready.extend(poll(&b_control, &ping));
        ready
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
    });
    assert!(
        matches!(ready.as_slice(), [ReceiveEvent::DropReady { id, paths }]
        if *id == b_id && *paths == vec![std::path::PathBuf::from("/tmp/received")])
    );
    b_control.decide(b_id, DropDecision::Accept).unwrap();
    case.ping.ping();
    let mut ended = Vec::new();
    case.drive(|_| {
        ended.extend(poll(&b_control, &ping));
        ended
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
    });
    assert!(matches!(ended.as_slice(),
        [ReceiveEvent::DropEnded { id, result: Ok(()) }] if *id == b_id));
    case.drive(|fixture| fixture.finish_count(2) == 1);
    assert_eq!(case.fixture().finish_count(1), 0);
    assert_eq!(case.fixture().finish_count(2), 1);
    assert_eq!(case.fixture().received(1), 1);
    assert_eq!(case.fixture().received(2), 1);
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(a_control.status(), ReceiveStatus::Ready);
    assert!(!a.is_detached());
    assert!(poll(&a_control, &ping).is_empty());
    assert!(poll(&b_control, &ping).is_empty());
    drop(writer_a);

    case.close();
    assert!(a.is_detached() && b.is_detached());
    assert!(matches!(case.lifecycle.status(), ReceiveStatus::Closed(_)));
    host_still_works(case.fixture_mut());
}
