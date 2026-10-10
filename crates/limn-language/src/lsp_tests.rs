use super::*;
use limn_editor::{EditKind, TextBuffer};

fn wait(server: &LanguageServer, predicate: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(event) = server.try_recv() {
            assert!(!matches!(event, Event::Stopped { .. }), "{event:?}");
            if predicate(&event) {
                return event;
            }
        }
        assert!(
            Instant::now() < deadline,
            "LSP process did not answer before deadline"
        );
        std::thread::sleep(Duration::from_millis(3));
    }
}
fn fixture(root: &Path) -> LanguageServer {
    let mut config = ServerConfig::new("python3", root);
    config.args = vec![format!(
        "{}/tests/fixtures/lsp_server.py",
        env!("CARGO_MANIFEST_DIR")
    )];
    let server = LanguageServer::spawn(config).unwrap();
    wait(&server, |event| matches!(event, Event::Initialized { .. }));
    server
}

#[test]
fn stdio_lifecycle_capabilities_and_all_language_requests_use_real_framed_process() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("niño.rs");
    let server = fixture(root.path());
    let snapshot = TextBuffer::new("a😀 value").unwrap().snapshot();
    let version = server.open(&path, &snapshot, Language::Rust).unwrap();
    assert_eq!(version.version, 1);
    let diagnostic = wait(&server, |event| matches!(event, Event::Diagnostics { .. }));
    let Event::Diagnostics { document, items } = diagnostic else {
        unreachable!()
    };
    assert_eq!(document.document_id, snapshot.document_id());
    assert_eq!(
        crate::utf16_to_offset(&snapshot, items[0].range.end),
        Some(2)
    );
    let position = Position {
        line: 0,
        character: 1,
    };
    for request in [
        Request::Completion(position),
        Request::Hover(position),
        Request::Definition(position),
        Request::References {
            position,
            include_declaration: true,
        },
        Request::Rename {
            position,
            new_name: "new_name".into(),
        },
        Request::Formatting {
            tab_size: 4,
            insert_spaces: true,
        },
        Request::CodeActions {
            range: Range {
                start: position,
                end: Position {
                    line: 0,
                    character: 3,
                },
            },
            diagnostics: vec![],
        },
        Request::Symbols,
    ] {
        let kind = request.kind();
        let id = server.request(&path, &snapshot, request).unwrap();
        let response = wait(
            &server,
            |event| matches!(event,Event::Response{id:received,..} if *received==id),
        );
        let Event::Response {
            document,
            kind: received,
            result,
            ..
        } = response
        else {
            unreachable!()
        };
        assert_eq!(received, kind);
        assert_eq!(document, version);
        assert!(result.is_ok());
    }
    server.close(&path).unwrap();
    assert!(server
        .request(&path, &snapshot, Request::Hover(position))
        .is_err());
}

#[test]
fn superseded_requests_diagnostics_and_reopened_document_identity_cannot_cross_revisions() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("file.rs");
    let server = fixture(root.path());
    let mut buffer = TextBuffer::new("a😀 value").unwrap();
    let old = buffer.snapshot();
    server.open(&path, &old, Language::Rust).unwrap();
    let delayed = server
        .request(&path, &old, Request::Hover(Position::default()))
        .unwrap();
    buffer.insert("new", EditKind::Typing, 0).unwrap();
    let current = buffer.snapshot();
    let version = server.change(&path, &current).unwrap();
    assert_eq!(version.version, 2);
    let diagnostic = wait(
        &server,
        |event| matches!(event,Event::Diagnostics {document,..} if document.revision==current.revision()),
    );
    let Event::Diagnostics { items, .. } = diagnostic else {
        unreachable!()
    };
    assert!(items.is_empty());
    assert!(
        !server.cancel(delayed),
        "change already cancelled the old request"
    );
    assert!(server
        .request(&path, &old, Request::Completion(Position::default()))
        .is_err());
    let cancelled = server
        .request(&path, &current, Request::Hover(Position::default()))
        .unwrap();
    assert!(server.cancel(cancelled));
    server.close(&path).unwrap();
    let replacement = TextBuffer::new("other identity").unwrap().snapshot();
    server.open(&path, &replacement, Language::Rust).unwrap();
    let diagnostic = wait(&server, |event| matches!(event, Event::Diagnostics { .. }));
    let Event::Diagnostics { document, .. } = diagnostic else {
        unreachable!()
    };
    assert_eq!(document.document_id, replacement.document_id());
    assert!(server.change(&path, &current).is_err());
}

#[test]
fn protocol_rejects_oversized_duplicate_or_truncated_frames_without_unbounded_allocations() {
    for bytes in [
        b"Content-Length: 999999999\r\n\r\n".as_slice(),
        b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
        b"Content-Length: 5\r\n\r\n{}",
    ] {
        assert!(read_frame(&mut std::io::Cursor::new(bytes)).is_err());
    }
    let mut input = std::io::Cursor::new(b"Content-Length: 2\r\n\r\n{}");
    assert_eq!(read_frame(&mut input).unwrap(), Some(json!({})));
    let (sender, _receiver) = mpsc::sync_channel(1);
    let wire = Wire {
        sender,
        bytes: Arc::new(AtomicUsize::new(0)),
    };
    wire.send(json!({"first":true})).unwrap();
    let before = wire.bytes.load(Ordering::Acquire);
    assert!(wire.send(json!({"second":true})).is_err());
    assert_eq!(wire.bytes.load(Ordering::Acquire), before);
}

#[test]
fn edits_reject_surrogate_splits_overlaps_and_preserve_original_snapshot_coordinates() {
    let snapshot = TextBuffer::new("a😀c\r\nniño").unwrap().snapshot();
    let edit = json!({"range":{"start":{"line":0,"character":1},"end":{"line":0,"character":3}},"newText":"ok"});
    assert_eq!(
        text_edits(&snapshot, &json!([edit])).unwrap()[0].range,
        1..2
    );
    let split = json!({"range":{"start":{"line":0,"character":2},"end":{"line":0,"character":3}},"newText":"bad"});
    assert!(text_edits(&snapshot, &json!([split])).is_err());
    assert!(text_edits(&snapshot, &json!([edit, edit])).is_err());
}

#[test]
fn event_sink_wakes_a_sleeping_consumer_before_backpressure_and_after_publication() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let sink = EventSink {
        sender,
        waker: Arc::new(Mutex::new(Some(Arc::new(move || {
            observed.fetch_add(1, Ordering::Release);
        })))),
    };
    sink.send(Event::Stopped {
        reason: "first".into(),
    })
    .unwrap();
    let worker = std::thread::spawn(move || {
        sink.send(Event::Stopped {
            reason: "second".into(),
        })
        .unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    while calls.load(Ordering::Acquire) < 3 {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(receiver.recv().unwrap(),Event::Stopped{reason} if reason=="first"));
    worker.join().unwrap();
    assert!(matches!(receiver.recv().unwrap(),Event::Stopped{reason} if reason=="second"));
    assert_eq!(calls.load(Ordering::Acquire), 4);
}

/// Explicit executable required: CI and release validation run this against an installed
/// rust-analyzer, never a fixture or a rustup shim mistaken for an available language server.
#[test]
#[ignore = "requires OPENIDE_RUST_ANALYZER pointing to a working rust-analyzer binary"]
fn real_rust_analyzer_initialize_open_hover_symbols_and_close() {
    let executable = std::env::var_os("OPENIDE_RUST_ANALYZER").expect("set OPENIDE_RUST_ANALYZER");
    assert!(probe(Path::new(&executable), &["--version"])
        .unwrap()
        .contains("rust-analyzer"));
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.rs");
    let source = "fn example() -> i32 { 42 }\nfn main() { let value = example(); }\n";
    std::fs::write(&path, source).unwrap();
    let mut config = ServerConfig::new(PathBuf::from(executable), root.path());
    config.initialization_options = json!({"detachedFiles":[path],"checkOnSave":false,"cargo":{"buildScripts":{"enable":false}},"procMacro":{"enable":false}});
    let server = LanguageServer::spawn(config).unwrap();
    wait(&server, |event| matches!(event, Event::Initialized { .. }));
    let snapshot = TextBuffer::new(source).unwrap().snapshot();
    server.open(&path, &snapshot, Language::Rust).unwrap();
    let id = server.request(&path, &snapshot, Request::Symbols).unwrap();
    let event = wait(
        &server,
        |event| matches!(event,Event::Response{id:received,..} if *received==id),
    );
    let Event::Response { result, .. } = event else {
        unreachable!()
    };
    let symbols = result.unwrap();
    assert!(
        symbols
            .as_array()
            .is_some_and(|symbols| symbols.iter().any(|symbol| symbol["name"] == "example")),
        "{symbols}"
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let id = server
            .request(
                &path,
                &snapshot,
                Request::Hover(Position {
                    line: 0,
                    character: 4,
                }),
            )
            .unwrap();
        let event = wait(
            &server,
            |event| matches!(event,Event::Response{id:received,..} if *received==id),
        );
        let Event::Response { result, .. } = event else {
            unreachable!()
        };
        match result {
            Ok(value) if !value.is_null() => break,
            Ok(_) => {}
            Err(error) if matches!(error.code, -32801 | -32802) => {}
            Err(error) => panic!("real hover failed: {error}"),
        }
        // The initialize handshake precedes detached-file semantic indexing.
        assert!(
            Instant::now() < deadline,
            "rust-analyzer did not produce semantic hover before deadline"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    server.close(&path).unwrap();
}
