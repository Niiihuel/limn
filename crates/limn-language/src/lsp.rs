//! LSP 3.17 over stdio. Only explicit host calls start processes or submit edits.
//! Responses retain the originating document/revision; outdated work never reaches the UI.
use crate::{Language, Position, Range};
use limn_editor::BufferSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use web_time::{Duration, Instant};

const MAX_FRAME: usize = 16 * 1024 * 1024;
const MAX_INCOMING_FRAME: usize = 4 * 1024 * 1024;
const MAX_QUEUED_BYTES: usize = 16 * 1024 * 1024;
const MAX_DOCUMENTS: usize = 32;
const MAX_REQUESTS: usize = 64;
pub type RequestId = u64;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub root: PathBuf,
    pub initialization_options: Value,
}
impl ServerConfig {
    pub fn new(executable: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            root: root.into(),
            initialization_options: Value::Null,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentVersion {
    pub uri: String,
    pub document_id: u64,
    pub revision: u64,
    pub version: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestKind {
    Completion,
    Hover,
    Definition,
    References,
    Rename,
    Formatting,
    CodeActions,
    Symbols,
}
#[derive(Clone, Debug)]
pub enum Request {
    Completion(Position),
    Hover(Position),
    Definition(Position),
    References {
        position: Position,
        include_declaration: bool,
    },
    Rename {
        position: Position,
        new_name: String,
    },
    Formatting {
        tab_size: u32,
        insert_spaces: bool,
    },
    CodeActions {
        range: Range,
        diagnostics: Vec<Value>,
    },
    Symbols,
}
impl Request {
    pub fn kind(&self) -> RequestKind {
        match self {
            Self::Completion(_) => RequestKind::Completion,
            Self::Hover(_) => RequestKind::Hover,
            Self::Definition(_) => RequestKind::Definition,
            Self::References { .. } => RequestKind::References,
            Self::Rename { .. } => RequestKind::Rename,
            Self::Formatting { .. } => RequestKind::Formatting,
            Self::CodeActions { .. } => RequestKind::CodeActions,
            Self::Symbols => RequestKind::Symbols,
        }
    }
    fn method(&self) -> &'static str {
        match self.kind() {
            RequestKind::Completion => "textDocument/completion",
            RequestKind::Hover => "textDocument/hover",
            RequestKind::Definition => "textDocument/definition",
            RequestKind::References => "textDocument/references",
            RequestKind::Rename => "textDocument/rename",
            RequestKind::Formatting => "textDocument/formatting",
            RequestKind::CodeActions => "textDocument/codeAction",
            RequestKind::Symbols => "textDocument/documentSymbol",
        }
    }
    fn capability(&self) -> &'static str {
        match self.kind() {
            RequestKind::Completion => "completionProvider",
            RequestKind::Hover => "hoverProvider",
            RequestKind::Definition => "definitionProvider",
            RequestKind::References => "referencesProvider",
            RequestKind::Rename => "renameProvider",
            RequestKind::Formatting => "documentFormattingProvider",
            RequestKind::CodeActions => "codeActionProvider",
            RequestKind::Symbols => "documentSymbolProvider",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    #[serde(default)]
    pub severity: Option<u32>,
    #[serde(default)]
    pub code: Option<Value>,
    #[serde(default)]
    pub source: Option<String>,
    pub message: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspError {
    pub code: i64,
    pub message: String,
}
impl std::fmt::Display for LspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for LspError {}
fn error(message: impl Into<String>) -> LspError {
    LspError {
        code: -32603,
        message: message.into(),
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Initialized {
        capabilities: Value,
        server_info: Value,
    },
    Response {
        id: RequestId,
        document: DocumentVersion,
        kind: RequestKind,
        result: Result<Value, LspError>,
    },
    Diagnostics {
        document: DocumentVersion,
        items: Vec<Diagnostic>,
    },
    /// A server without versioned diagnostics cannot prove they describe the current buffer.
    UnversionedDiagnostics {
        uri: String,
        items: Vec<Diagnostic>,
    },
    Stopped {
        reason: String,
    },
}
#[derive(Clone)]
struct Pending {
    document: DocumentVersion,
    kind: RequestKind,
    started: Instant,
}
struct State {
    next_version: i32,
    capabilities: Option<Value>,
    documents: HashMap<String, DocumentVersion>,
    pending: HashMap<RequestId, Pending>,
}
enum Payload {
    Bytes(Vec<u8>),
    Document {
        snapshot: BufferSnapshot,
        document: DocumentVersion,
        language: Option<Language>,
    },
}
struct Outgoing {
    payload: Payload,
    charge: usize,
}
impl Outgoing {
    fn serialize(self) -> Result<Vec<u8>, String> {
        match self.payload {
            Payload::Bytes(bytes) => Ok(bytes),
            Payload::Document {
                snapshot,
                document,
                language,
            } => {
                crate::coordinates::warm_index(&snapshot);
                let text: String = snapshot.chunks().collect();
                let value = if let Some(language) = language {
                    json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":document.uri,"languageId":language.lsp_id(),"version":document.version,"text":text}}})
                } else {
                    json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":document.uri,"version":document.version},"contentChanges":[{"text":text}]}})
                };
                struct Limited(Vec<u8>);
                impl Write for Limited {
                    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                        if self.0.len().saturating_add(bytes.len()) > MAX_FRAME {
                            return Err(std::io::Error::other(
                                "serialized document exceeds LSP frame limit",
                            ));
                        }
                        self.0.extend_from_slice(bytes);
                        Ok(bytes.len())
                    }
                    fn flush(&mut self) -> std::io::Result<()> {
                        Ok(())
                    }
                }
                let mut bytes = Limited(Vec::new());
                serde_json::to_writer(&mut bytes, &value).map_err(|error| error.to_string())?;
                Ok(bytes.0)
            }
        }
    }
}
struct Wire {
    sender: mpsc::SyncSender<Outgoing>,
    bytes: Arc<AtomicUsize>,
}
impl Clone for Wire {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            bytes: self.bytes.clone(),
        }
    }
}
impl Wire {
    fn send(&self, value: Value) -> Result<(), LspError> {
        let bytes = serde_json::to_vec(&value).map_err(|error| self::error(error.to_string()))?;
        if bytes.len() > MAX_FRAME {
            return Err(error("The LSP message exceeds the size limit"));
        }
        let len = bytes.len();
        if self
            .bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(len)
                    .filter(|next| *next <= MAX_QUEUED_BYTES)
            })
            .is_err()
        {
            return Err(error("The LSP queue is full"));
        }
        if self
            .sender
            .try_send(Outgoing {
                payload: Payload::Bytes(bytes),
                charge: len,
            })
            .is_err()
        {
            self.bytes.fetch_sub(len, Ordering::Release);
            return Err(error("The LSP queue is not available"));
        }
        Ok(())
    }
    fn document(
        &self,
        snapshot: &BufferSnapshot,
        document: &DocumentVersion,
        language: Option<Language>,
    ) -> Result<(), LspError> {
        let charge = snapshot.byte_count().saturating_add(2048);
        if self
            .bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(charge)
                    .filter(|next| *next <= MAX_QUEUED_BYTES)
            })
            .is_err()
        {
            return Err(error("The LSP queue is full"));
        }
        let outgoing = Outgoing {
            payload: Payload::Document {
                snapshot: snapshot.clone(),
                document: document.clone(),
                language,
            },
            charge,
        };
        if self.sender.try_send(outgoing).is_err() {
            self.bytes.fetch_sub(charge, Ordering::Release);
            return Err(error("The LSP queue is not available"));
        }
        Ok(())
    }
}
type WakeCallback = Arc<dyn Fn() + Send + Sync>;
#[derive(Clone)]
struct EventSink {
    sender: mpsc::SyncSender<Event>,
    waker: Arc<Mutex<Option<WakeCallback>>>,
}
impl EventSink {
    fn wake(&self) {
        let callback = self.waker.lock().ok().and_then(|callback| callback.clone());
        if let Some(callback) = callback {
            callback();
        }
    }
    fn send(&self, event: Event) -> Result<(), mpsc::SendError<Event>> {
        // Wake before a potentially full queue, and again after publication. Neither a
        // sleeping UI nor a drain racing this send can strand an event in the channel.
        self.wake();
        let result = self.sender.send(event);
        self.wake();
        result
    }
}
struct StartingProcess(Option<Child>);
impl Drop for StartingProcess {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
pub struct LanguageServer {
    child: Child,
    wire: Wire,
    events: Option<mpsc::Receiver<Event>>,
    state: Arc<Mutex<State>>,
    next_id: AtomicU64,
    stopped: Arc<AtomicBool>,
    initialize_deadline: Instant,
    waker: Arc<Mutex<Option<WakeCallback>>>,
}
impl LanguageServer {
    pub fn spawn(config: ServerConfig) -> Result<Self, LspError> {
        let root_uri = file_uri(&config.root)?;
        let child = Command::new(&config.executable)
            .args(&config.args)
            .current_dir(&config.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| {
                error(format!(
                    "No se pudo iniciar {}: {source}",
                    config.executable.display()
                ))
            })?;
        let mut starting = StartingProcess(Some(child));
        let child = starting.0.as_mut().unwrap();
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| error("Falta stdin del servidor"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| error("Falta stdout del servidor"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| error("Falta stderr del servidor"))?;
        let (sender, receiver) = mpsc::sync_channel::<Outgoing>(16);
        let bytes = Arc::new(AtomicUsize::new(0));
        let wire = Wire {
            sender,
            bytes: bytes.clone(),
        };
        let (event_sender, events) = mpsc::sync_channel(8);
        let waker = Arc::new(Mutex::new(None));
        let event_sender = EventSink {
            sender: event_sender,
            waker: waker.clone(),
        };
        let state = Arc::new(Mutex::new(State {
            next_version: 0,
            capabilities: None,
            documents: HashMap::new(),
            pending: HashMap::new(),
        }));
        let stopped = Arc::new(AtomicBool::new(false));
        let writer_stopped = stopped.clone();
        let writer_events = event_sender.clone();
        std::thread::Builder::new()
            .name("lsp-stdin".into())
            .spawn(move || {
                while let Ok(message) = receiver.recv() {
                    let charge = message.charge;
                    let result =
                        message
                            .serialize()
                            .map_err(std::io::Error::other)
                            .and_then(|message| {
                                write!(stdin, "Content-Length: {}\r\n\r\n", message.len())
                                    .and_then(|_| stdin.write_all(&message))
                                    .and_then(|_| stdin.flush())
                            });
                    bytes.fetch_sub(charge, Ordering::Release);
                    if let Err(source) = result {
                        writer_stopped.store(true, Ordering::Release);
                        let _ = writer_events.send(Event::Stopped {
                            reason: format!("Writing to the language server failed: {source}"),
                        });
                        break;
                    }
                }
            })
            .map_err(|source| error(source.to_string()))?;
        let reader_state = state.clone();
        let reader_wire = wire.clone();
        let reader_stopped = stopped.clone();
        std::thread::Builder::new()
            .name("lsp-stdout".into())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                loop {
                    match read_frame(&mut reader) {
                        Ok(Some(message)) => {
                            if let Err(reason) =
                                receive(message, &reader_state, &reader_wire, &event_sender)
                            {
                                reader_stopped.store(true, Ordering::Release);
                                let _ = event_sender.send(Event::Stopped { reason });
                                break;
                            }
                        }
                        Ok(None) => {
                            reader_stopped.store(true, Ordering::Release);
                            let _ = event_sender.send(Event::Stopped {
                                reason: "The language server closed the connection".into(),
                            });
                            break;
                        }
                        Err(source) => {
                            reader_stopped.store(true, Ordering::Release);
                            let _ = event_sender.send(Event::Stopped {
                                reason: format!("Invalid LSP response: {source}"),
                            });
                            break;
                        }
                    }
                }
            })
            .map_err(|source| error(source.to_string()))?;
        // Drain stderr without retaining unbounded logs or allowing a full pipe to deadlock LSP.
        std::thread::Builder::new()
            .name("lsp-stderr".into())
            .spawn(move || {
                let mut reader = stderr;
                let mut chunk = [0u8; 4096];
                while matches!(reader.read(&mut chunk), Ok(1..)) {}
            })
            .map_err(|source| error(source.to_string()))?;
        wire.send(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{
            "processId":std::process::id(),"clientInfo":{"name":"OpenIDE","version":env!("CARGO_PKG_VERSION")},
            "rootUri":root_uri,"workspaceFolders":[{"uri":root_uri,"name":config.root.file_name().and_then(|name|name.to_str()).unwrap_or("workspace")}],
            "initializationOptions":config.initialization_options,
            "capabilities":{"general":{"positionEncodings":["utf-16"]},"workspace":{"applyEdit":false,"workspaceFolders":true,"configuration":false},
                "textDocument":{"synchronization":{"dynamicRegistration":false,"didSave":false},
                    "completion":{"completionItem":{"snippetSupport":false,"insertReplaceSupport":true}},
                    "hover":{"contentFormat":["markdown","plaintext"]},"definition":{"linkSupport":true},
                    "publishDiagnostics":{"versionSupport":true},"codeAction":{"codeActionLiteralSupport":{"codeActionKind":{"valueSet":["","quickfix","refactor","source"]}}}}}
        }}))?;
        Ok(Self {
            child: starting.0.take().unwrap(),
            wire,
            events: Some(events),
            state,
            next_id: AtomicU64::new(1),
            stopped,
            initialize_deadline: Instant::now() + Duration::from_secs(15),
            waker,
        })
    }
    pub fn set_waker(&self, waker: Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut callback) = self.waker.lock() {
            *callback = Some(waker);
        }
    }
    pub fn is_ready(&self) -> bool {
        !self.stopped.load(Ordering::Acquire)
            && self
                .state
                .lock()
                .is_ok_and(|state| state.capabilities.is_some())
    }
    pub fn capabilities(&self) -> Option<Value> {
        self.state.lock().ok()?.capabilities.clone()
    }
    pub fn open(
        &self,
        path: &Path,
        snapshot: &BufferSnapshot,
        language: Language,
    ) -> Result<DocumentVersion, LspError> {
        let uri = file_uri(path)?;
        if snapshot.byte_count() > 8 * 1024 * 1024 {
            return Err(error("Documento demasiado grande para LSP"));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("Estado LSP no disponible"))?;
        self.require_ready(&state)?;
        if state.documents.contains_key(&uri) {
            return Err(error("The document is already open in the language server"));
        }
        if state.documents.len() >= MAX_DOCUMENTS {
            return Err(error("The limit of open LSP documents was reached"));
        }
        let version = state
            .next_version
            .checked_add(1)
            .ok_or_else(|| error("The LSP document version is exhausted"))?;
        let document = DocumentVersion {
            uri: uri.clone(),
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
            version,
        };
        self.wire.document(snapshot, &document, Some(language))?;
        state.next_version = version;
        state.documents.insert(uri, document.clone());
        Ok(document)
    }
    /// Full-document changes are legal for both Full and Incremental sync servers. The
    /// position/range-free event avoids stale edit deltas after undo, multicursor or reload.
    pub fn change(
        &self,
        path: &Path,
        snapshot: &BufferSnapshot,
    ) -> Result<DocumentVersion, LspError> {
        let uri = file_uri(path)?;
        if snapshot.byte_count() > 8 * 1024 * 1024 {
            return Err(error("Documento demasiado grande para LSP"));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("Estado LSP no disponible"))?;
        self.require_ready(&state)?;
        let prior = state
            .documents
            .get(&uri)
            .ok_or_else(|| error("Documento LSP cerrado"))?;
        if prior.document_id != snapshot.document_id() {
            return Err(error(
                "The document's identity changed: close it and open it again",
            ));
        }
        if prior.revision == snapshot.revision() {
            return Ok(prior.clone());
        }
        if prior.revision > snapshot.revision() {
            return Err(error("An earlier revision cannot be synchronized"));
        }
        let version = state
            .next_version
            .checked_add(1)
            .ok_or_else(|| error("The LSP document version is exhausted"))?;
        let document = DocumentVersion {
            uri: uri.clone(),
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
            version,
        };
        self.wire.document(snapshot, &document, None)?;
        cancel_document_requests(&mut state, &self.wire, &uri);
        state.next_version = version;
        state.documents.insert(uri, document.clone());
        Ok(document)
    }
    pub fn close(&self, path: &Path) -> Result<(), LspError> {
        let uri = file_uri(path)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("Estado LSP no disponible"))?;
        if !state.documents.contains_key(&uri) {
            return Ok(());
        }
        self.wire.send(json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}))?;
        cancel_document_requests(&mut state, &self.wire, &uri);
        state.documents.remove(&uri);
        Ok(())
    }
    pub fn request(
        &self,
        path: &Path,
        snapshot: &BufferSnapshot,
        request: Request,
    ) -> Result<RequestId, LspError> {
        let uri = file_uri(path)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("Estado LSP no disponible"))?;
        self.require_ready(&state)?;
        let capability = &state.capabilities.as_ref().unwrap()[request.capability()];
        if capability.is_null() || capability == &Value::Bool(false) {
            return Err(error(format!("El servidor no ofrece {}", request.method())));
        }
        let document = state
            .documents
            .get(&uri)
            .filter(|document| {
                document.document_id == snapshot.document_id()
                    && document.revision == snapshot.revision()
            })
            .cloned()
            .ok_or_else(|| error("Synchronize the current revision before making an LSP request"))?;
        if state.pending.len() >= MAX_REQUESTS {
            return Err(error("Demasiadas solicitudes LSP pendientes"));
        }
        let mut params = json!({"textDocument":{"uri":uri}});
        match &request {
            Request::Completion(position)
            | Request::Hover(position)
            | Request::Definition(position) => {
                validate_position(snapshot, *position)?;
                params["position"] = json!(position);
            }
            Request::References {
                position,
                include_declaration,
            } => {
                validate_position(snapshot, *position)?;
                params["position"] = json!(position);
                params["context"] = json!({"includeDeclaration":include_declaration});
            }
            Request::Rename { position, new_name } => {
                validate_position(snapshot, *position)?;
                if new_name.is_empty() || new_name.len() > 1024 {
                    return Err(error("Invalid symbol name"));
                }
                params["position"] = json!(position);
                params["newName"] = json!(new_name);
            }
            Request::Formatting {
                tab_size,
                insert_spaces,
            } => {
                params["options"] =
                    json!({"tabSize":(*tab_size).clamp(1,32),"insertSpaces":insert_spaces});
            }
            Request::CodeActions { range, diagnostics } => {
                validate_position(snapshot, range.start)?;
                validate_position(snapshot, range.end)?;
                params["range"] = json!(range);
                params["context"] = json!({"diagnostics":diagnostics});
            }
            Request::Symbols => {}
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.wire
            .send(json!({"jsonrpc":"2.0","id":id,"method":request.method(),"params":params}))?;
        state.pending.insert(
            id,
            Pending {
                document,
                kind: request.kind(),
                started: Instant::now(),
            },
        );
        Ok(id)
    }
    pub fn cancel(&self, id: RequestId) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if state.pending.remove(&id).is_none() {
            return false;
        }
        let _ = self
            .wire
            .send(json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":id}}));
        true
    }
    /// Polling performs a second revision check: a queued response may have become stale
    /// between the reader receiving it and the host rendering its next frame.
    pub fn try_recv(&self) -> Option<Event> {
        if !self.stopped.load(Ordering::Acquire) {
            let mut state = self.state.lock().ok()?;
            if state.capabilities.is_none() && Instant::now() >= self.initialize_deadline {
                self.stopped.store(true, Ordering::Release);
                return Some(Event::Stopped {
                    reason: "The server did not complete initialize in time".into(),
                });
            }
            let expired = state
                .pending
                .iter()
                .find(|(_, pending)| pending.started.elapsed() >= Duration::from_secs(30))
                .map(|(id, _)| *id);
            if let Some(id) = expired {
                let pending = state.pending.remove(&id)?;
                let _ = self
                    .wire
                    .send(json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":id}}));
                return Some(Event::Response {
                    id,
                    document: pending.document,
                    kind: pending.kind,
                    result: Err(error("The LSP request timed out")),
                });
            }
        }
        loop {
            let event = self.events.as_ref()?.try_recv().ok()?;
            match &event {
                Event::Response { document, .. } | Event::Diagnostics { document, .. }
                    if !self
                        .state
                        .lock()
                        .ok()?
                        .documents
                        .get(&document.uri)
                        .is_some_and(|current| current == document) =>
                {
                    continue;
                }
                Event::UnversionedDiagnostics { uri, .. }
                    if !self.state.lock().ok()?.documents.contains_key(uri) =>
                {
                    continue;
                }
                _ => {}
            }
            return Some(event);
        }
    }
    fn require_ready(&self, state: &State) -> Result<(), LspError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(error("The language server stopped"));
        }
        if state.capabilities.is_none() {
            return Err(error("The language server is starting"));
        }
        Ok(())
    }
}
impl Drop for LanguageServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self
            .wire
            .send(json!({"jsonrpc":"2.0","id":u64::MAX,"method":"shutdown","params":null}));
        let _ = self
            .wire
            .send(json!({"jsonrpc":"2.0","method":"exit","params":null}));
        // Never block a render thread waiting for a malfunctioning server or full event queue.
        self.events.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub fn file_uri(path: &Path) -> Result<String, LspError> {
    url::Url::from_file_path(path)
        .map(String::from)
        .map_err(|_| error("LSP requiere una ruta absoluta"))
}
pub fn uri_path(uri: &str) -> Result<PathBuf, LspError> {
    let url = url::Url::parse(uri).map_err(|_| error("Invalid LSP URI"))?;
    if url.scheme() != "file" {
        return Err(error("The LSP location is not a local file"));
    }
    url.to_file_path()
        .map_err(|_| error("La URI LSP no identifica una ruta local"))
}
/// Decode a server edit list against the exact response snapshot before handing it to an
/// editor transaction. Invalid UTF-16, backwards ranges and overlapping edits are rejected.
/// The host must additionally match Event::Response.document to its current controller.
pub fn text_edits(
    snapshot: &BufferSnapshot,
    value: &Value,
) -> Result<Vec<limn_editor::TextEdit>, LspError> {
    let items = value
        .as_array()
        .ok_or_else(|| error("The response has no list of changes"))?;
    if items.len() > 4096 {
        return Err(error("Demasiados cambios en una respuesta LSP"));
    }
    let mut edits = Vec::with_capacity(items.len());
    let mut bytes = 0usize;
    for item in items {
        let range: Range = serde_json::from_value(item["range"].clone())
            .map_err(|_| error("Invalid LSP range"))?;
        let start = crate::utf16_to_offset(snapshot, range.start)
            .ok_or_else(|| error("Invalid UTF-16 start"))?;
        let end = crate::utf16_to_offset(snapshot, range.end)
            .ok_or_else(|| error("Invalid UTF-16 end"))?;
        if start > end {
            return Err(error("The LSP range is reversed"));
        }
        let replacement = item["newText"]
            .as_str()
            .ok_or_else(|| error("The replacement text is missing"))?;
        bytes = bytes.saturating_add(replacement.len());
        if bytes > 8 * 1024 * 1024 {
            return Err(error("The LSP changes exceed the size limit"));
        }
        edits.push(limn_editor::TextEdit {
            range: start..end,
            replacement: replacement.into(),
        });
    }
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    if edits.windows(2).any(|pair| {
        pair[0].range.end > pair[1].range.start || pair[0].range.start == pair[1].range.start
    }) {
        return Err(error("Los cambios LSP se superponen"));
    }
    Ok(edits)
}

/// Candidate paths only; availability is established by a successful protocol initialize.
/// Environment overrides are explicit, then PATH is searched without installing anything.
pub fn server_candidates(language: Language) -> Vec<PathBuf> {
    let (override_name, names): (&str, &[&str]) = match language {
        Language::Rust => ("OPENIDE_RUST_ANALYZER", &["rust-analyzer"]),
        Language::JavaScript | Language::TypeScript | Language::Tsx => (
            "OPENIDE_TYPESCRIPT_LANGUAGE_SERVER",
            &["typescript-language-server"],
        ),
        Language::Python => (
            "OPENIDE_PYTHON_LANGUAGE_SERVER",
            &["pyright-langserver", "pylsp"],
        ),
        Language::Json => (
            "OPENIDE_JSON_LANGUAGE_SERVER",
            &["vscode-json-language-server"],
        ),
        Language::Css => (
            "OPENIDE_CSS_LANGUAGE_SERVER",
            &["vscode-css-language-server"],
        ),
        Language::Html => (
            "OPENIDE_HTML_LANGUAGE_SERVER",
            &["vscode-html-language-server"],
        ),
        Language::Toml => ("OPENIDE_TOML_LANGUAGE_SERVER", &["taplo"]),
        Language::Plain | Language::Markdown => return Vec::new(),
    };
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os(override_name)
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        candidates.push(path);
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths) {
            for name in names {
                let path = directory.join(name);
                if path.is_file() && !candidates.contains(&path) {
                    candidates.push(path);
                }
            }
        }
    }
    candidates
}
/// Known stdio switches; an explicitly configured executable can provide its own args.
pub fn stdio_args(executable: &Path) -> Vec<String> {
    match executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
    {
        "typescript-language-server"
        | "pyright-langserver"
        | "vscode-json-language-server"
        | "vscode-css-language-server"
        | "vscode-html-language-server" => vec!["--stdio".into()],
        "taplo" => vec!["lsp".into(), "stdio".into()],
        _ => Vec::new(),
    }
}
fn validate_position(snapshot: &BufferSnapshot, position: Position) -> Result<(), LspError> {
    crate::utf16_to_offset(snapshot, position)
        .map(|_| ())
        .ok_or_else(|| error("UTF-16 position outside the document"))
}
fn cancel_document_requests(state: &mut State, wire: &Wire, uri: &str) {
    state.pending.retain(|id, pending| {
        if pending.document.uri != uri {
            return true;
        }
        let _ = wire.send(json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":id}}));
        false
    });
}
fn read_frame(reader: &mut impl BufRead) -> std::io::Result<Option<Value>> {
    let mut length = None;
    let mut headers = 0;
    loop {
        let mut line = String::new();
        let count = (&mut *reader).take(8193).read_line(&mut line)?;
        if count == 0 {
            return if headers == 0 {
                Ok(None)
            } else {
                Err(std::io::Error::other("truncated header"))
            };
        }
        headers += count;
        if headers > 8192 {
            return Err(std::io::Error::other("oversized header"));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                if length.is_some() {
                    return Err(std::io::Error::other("duplicate Content-Length"));
                }
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(std::io::Error::other)?,
                );
            }
        }
    }
    let length = length
        .filter(|length| *length <= MAX_INCOMING_FRAME)
        .ok_or_else(|| std::io::Error::other("invalid Content-Length"))?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    let value = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
    Ok(Some(value))
}
fn receive(
    message: Value,
    state: &Arc<Mutex<State>>,
    wire: &Wire,
    events: &EventSink,
) -> Result<(), String> {
    if message["jsonrpc"] != "2.0" {
        return Err("The server did not send JSON-RPC 2.0".into());
    }
    if let Some(method) = message["method"].as_str() {
        if let Some(id) = message.get("id") {
            // Server-initiated edits and commands never execute silently. Configuration and
            // workspace folders are handled explicitly; unsupported requests receive errors.
            let result = match method {
                "workspace/configuration" => Some(json!(message["params"]["items"]
                    .as_array()
                    .map(|items| vec![Value::Null; items.len()])
                    .unwrap_or_default())),
                "window/workDoneProgress/create" => Some(Value::Null),
                "workspace/applyEdit" => Some(
                    json!({"applied":false,"failureReason":"The host reviews changes before applying them"}),
                ),
                _ => None,
            };
            let reply = match result {
                Some(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
                None => {
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not supported"}})
                }
            };
            wire.send(reply).map_err(|error| error.to_string())?;
            return Ok(());
        }
        if method == "textDocument/publishDiagnostics" {
            let params = &message["params"];
            let Some(uri) = params["uri"].as_str() else {
                return Ok(());
            };
            let document = state
                .lock()
                .map_err(|_| "estado LSP no disponible")?
                .documents
                .get(uri)
                .cloned();
            let Some(document) = document else {
                return Ok(());
            };
            let items = params["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
                .take(4096)
                .filter_map(|item| serde_json::from_value::<Diagnostic>(item.clone()).ok())
                .collect();
            if let Some(version) = params["version"].as_i64() {
                if version == document.version as i64 {
                    events
                        .send(Event::Diagnostics { document, items })
                        .map_err(|error| error.to_string())?;
                }
            } else {
                events
                    .send(Event::UnversionedDiagnostics {
                        uri: uri.into(),
                        items,
                    })
                    .map_err(|error| error.to_string())?;
            }
        }
        return Ok(());
    }
    let Some(id) = message["id"].as_u64() else {
        return Ok(());
    };
    if id == 0 {
        if let Some(failure) = message.get("error") {
            return Err(failure["message"]
                .as_str()
                .unwrap_or("initialize failed")
                .into());
        }
        let result = &message["result"];
        let capabilities = result["capabilities"].clone();
        if !capabilities.is_object() {
            return Err("initialize returned no capabilities".into());
        }
        if capabilities
            .get("positionEncoding")
            .and_then(Value::as_str)
            .is_some_and(|encoding| encoding != "utf-16")
        {
            return Err("The server chose a position encoding that is not supported".into());
        }
        wire.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))
            .map_err(|error| error.to_string())?;
        state
            .lock()
            .map_err(|_| "estado LSP no disponible")?
            .capabilities = Some(capabilities.clone());
        events
            .send(Event::Initialized {
                capabilities,
                server_info: result["serverInfo"].clone(),
            })
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    let pending = state
        .lock()
        .map_err(|_| "estado LSP no disponible")?
        .pending
        .remove(&id);
    if let Some(pending) = pending {
        let result = match message.get("error") {
            Some(failure) => Err(LspError {
                code: failure["code"].as_i64().unwrap_or(-32603),
                message: failure["message"]
                    .as_str()
                    .unwrap_or("Solicitud LSP fallida")
                    .into(),
            }),
            None => Ok(message["result"].clone()),
        };
        events
            .send(Event::Response {
                id,
                document: pending.document,
                kind: pending.kind,
                result,
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Check an explicit binary off the UI thread. A rustup shim with a missing component is
/// unavailable; discovery never installs software or mistakes its path for a working server.
pub fn probe(executable: &Path, args: &[&str]) -> Result<String, LspError> {
    let mut child = Command::new(executable)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|source| error(source.to_string()))?;
    let stdout = child.stdout.take().ok_or_else(|| error("sin stdout"))?;
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut output = String::new();
        let _ = stdout.take(8192).read_to_string(&mut output);
        let _ = sender.send(output);
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                return Ok(receiver
                    .recv_timeout(Duration::from_millis(100))
                    .unwrap_or_default()
                    .trim()
                    .into())
            }
            Ok(Some(_)) => return Err(error("El ejecutable no pudo iniciar el servidor")),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error(
                    "The server did not answer the version check",
                ));
            }
        }
    }
}

#[cfg(test)]
#[path = "lsp_tests.rs"]
mod tests;
