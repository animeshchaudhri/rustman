//! LSP (Language Server Protocol) Process Client implementation.
//!
//! This module provides a client for communicating with LSP servers via stdio.
//! It handles document synchronization, hover requests, and completion requests.
//!
//! Enable with the `lsp-process` Cargo feature. Not available on WASM targets.

pub mod config;
pub mod overlay;

/// JSON-RPC method name for server-push progress notifications.
const METHOD_PROGRESS: &str = "$/progress";
/// JSON-RPC method name sent by the server when it creates a work-done token.
const METHOD_WORK_DONE_PROGRESS_CREATE: &str = "window/workDoneProgress/create";
/// Progress `kind` value that signals the end of a work-done sequence.
const PROGRESS_KIND_END: &str = "end";

use self::config::{
    LspCommand, ensure_rust_analyzer_config, lsp_server_config,
    resolve_lsp_command,
};
use crate::canvas_editor::lsp::{
    LspClient, LspDocument, LspPosition, LspTextChange,
};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

// =============================================================================
// Text Model - Internal document representation for tracking text changes
// =============================================================================

/// Internal representation of a text document as a vector of lines.
///
/// Used to track document state and convert between character and byte indices.
struct TextModel {
    /// The document content stored as a vector of lines (without newline characters)
    lines: Vec<String>,
}

impl TextModel {
    /// Creates a new `TextModel` from a string.
    ///
    /// Splits the text into lines for easier manipulation.
    /// An empty string creates a single empty line.
    fn from_text(text: &str) -> Self {
        let lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(String::from).collect()
        };
        Self { lines }
    }

    /// Applies a text change (edit) to the document.
    ///
    /// Handles multi-line insertions and deletions by splicing the lines vector.
    fn apply_change(&mut self, change: &LspTextChange) {
        let start_line = change.range.start.line as usize;
        let end_line = change.range.end.line as usize;

        if start_line >= self.lines.len() || end_line >= self.lines.len() {
            return;
        }

        let start_col = change.range.start.character as usize;
        let end_col = change.range.end.character as usize;

        let start_byte = char_to_byte_index(&self.lines[start_line], start_col);
        let end_byte = char_to_byte_index(&self.lines[end_line], end_col);

        let prefix = self.lines[start_line][..start_byte].to_string();
        let suffix = self.lines[end_line][end_byte..].to_string();

        let inserted: Vec<&str> = change.text.split('\n').collect();
        let mut replacement: Vec<String> = Vec::new();

        if inserted.len() == 1 {
            replacement.push(format!("{}{}{}", prefix, inserted[0], suffix));
        } else {
            replacement.push(format!("{}{}", prefix, inserted[0]));
            for mid in inserted.iter().take(inserted.len() - 1).skip(1) {
                replacement.push((*mid).to_string());
            }
            replacement.push(format!(
                "{}{}",
                inserted[inserted.len() - 1],
                suffix
            ));
        }

        self.lines.splice(start_line..=end_line, replacement);
    }

    /// Converts a UTF-8 character position to a UTF-16 position.
    ///
    /// This is necessary because LSP uses UTF-16 for character positions.
    fn to_utf16_position(&self, position: LspPosition) -> LspPosition {
        let line_index = position.line as usize;
        let char_index = position.character as usize;
        let line = self.lines.get(line_index).map_or("", |l| l.as_str());

        let utf16_col =
            line.chars().take(char_index).map(|c| c.len_utf16() as u32).sum();
        LspPosition { line: position.line, character: utf16_col }
    }
}

/// Converts a character index to a byte index in a string.
///
/// Returns the length of the string if the index is out of bounds.
fn char_to_byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices().nth(char_index).map_or(s.len(), |(idx, _)| idx)
}

// =============================================================================
// Document State - Tracks the state of an open document
// =============================================================================

/// Represents the state of a single open document.
struct DocumentState {
    /// The text content of the document
    text: TextModel,
}

// =============================================================================
// LSP Request Types
// =============================================================================

/// Enumeration of LSP request types that we track for response handling.
enum LspRequestKind {
    /// Hover request — shows type information and documentation
    Hover,
    /// Completion request — provides auto-complete suggestions
    Completion,
    /// Definition request — go to definition
    Definition,
}

// =============================================================================
// LSP Events - Events sent back to the main application
// =============================================================================

/// Events that can be sent from the LSP client to the application.
///
/// Receive these by polling the `mpsc::Receiver` you pass to
/// [`LspProcessClient::new_with_server`].
pub enum LspEvent {
    /// Hover information received from the LSP server.
    Hover {
        /// Markdown or plain-text hover content.
        text: String,
    },
    /// Completion items received from the LSP server.
    Completion {
        /// List of completion label strings.
        items: Vec<String>,
    },
    /// Definition location received from the LSP server.
    Definition {
        /// Target document URI.
        uri: String,
        /// Target range within that document.
        range: crate::canvas_editor::lsp::LspRange,
    },
    /// Progress notification from the LSP server.
    Progress {
        /// Progress token identifier.
        token: String,
        /// Key of the server that sent this notification.
        server_key: String,
        /// Human-readable title for the progress operation.
        title: String,
        /// Optional status message.
        message: Option<String>,
        /// Optional percentage complete (0–100).
        percentage: Option<u32>,
        /// `true` when this is the final progress notification.
        done: bool,
    },
    /// Log message from the LSP server's stderr.
    Log {
        /// Key of the server that sent this message.
        server_key: String,
        /// The log line.
        message: String,
    },
}

// =============================================================================
// LSP Process Client - Main client implementation
// =============================================================================

/// Client for communicating with an LSP server process.
///
/// Manages the lifecycle of the server process and handles all communication.
/// Implements [`LspClient`] so it can be plugged directly into a [`CodeEditor`].
///
/// # Examples
///
/// ```no_run
/// use std::sync::mpsc;
/// use iced_code_editor::{LspProcessClient, LspEvent};
///
/// let (tx, rx) = mpsc::channel::<LspEvent>();
/// let client = LspProcessClient::new_with_server(
///     "file:///home/user/project",
///     tx,
///     "lua-language-server",
/// );
/// ```
///
/// [`CodeEditor`]: crate::CodeEditor
pub struct LspProcessClient {
    /// The child process running the LSP server
    child: Child,
    /// Channel for sending messages to the writer thread
    writer: mpsc::Sender<Vec<u8>>,
    /// Map of URI to document state for all open documents
    documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Counter for generating unique request IDs
    request_id: AtomicU64,
    /// Map of pending request IDs to their types (for response routing)
    pending_requests: Arc<Mutex<HashMap<u64, LspRequestKind>>>,
    /// Handle to the writer thread (kept alive for the client's lifetime)
    _writer_thread: thread::JoinHandle<()>,
    /// Handle to the reader thread (kept alive for the client's lifetime)
    _reader_thread: thread::JoinHandle<()>,
    /// Handle to the stderr thread (kept alive for the client's lifetime)
    _stderr_thread: thread::JoinHandle<()>,
}

impl LspProcessClient {
    /// Creates a new LSP client connected to the specified server.
    ///
    /// # Arguments
    ///
    /// * `root_uri` — the root URI of the workspace (e.g. `"file:///home/user/project"`)
    /// * `events` — channel to send [`LspEvent`]s back to the application
    /// * `server_key` — key identifying the LSP server (e.g. `"lua-language-server"`)
    ///
    /// # Errors
    ///
    /// Returns an error string when the server key is not recognised, when the
    /// server binary cannot be found, or when the process cannot be spawned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::mpsc;
    /// use iced_code_editor::{LspProcessClient, LspEvent};
    ///
    /// let (tx, _rx) = mpsc::channel::<LspEvent>();
    /// let client = LspProcessClient::new_with_server(
    ///     "file:///tmp/project",
    ///     tx,
    ///     "lua-language-server",
    /// );
    /// assert!(client.is_ok());
    /// ```
    pub fn new_with_server(
        root_uri: &str,
        events: mpsc::Sender<LspEvent>,
        server_key: &str,
    ) -> Result<Self, String> {
        let config = lsp_server_config(server_key)
            .ok_or_else(|| format!("Unsupported LSP server: {}", server_key))?;

        if server_key == "rust-analyzer" {
            ensure_rust_analyzer_config();
        }

        let command = resolve_lsp_command(config)?;
        Self::new_with_command(root_uri, events, &command, server_key)
    }

    /// Creates a new LSP client with a specific command.
    ///
    /// This is the internal implementation that spawns the process.
    ///
    /// # Errors
    ///
    /// Returns an error string if the process cannot be spawned or if stdio
    /// handles cannot be acquired.
    fn new_with_command(
        root_uri: &str,
        events: mpsc::Sender<LspEvent>,
        command: &LspCommand,
        server_key: &str,
    ) -> Result<Self, String> {
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    if command.program == "rust-analyzer" {
                        "LSP server program rust-analyzer not found. Please install rust-analyzer or set RUST_ANALYZER/RUST_ANALYZER_PATH environment variable".to_string()
                    } else {
                        format!("LSP server program {} not found", command.program)
                    }
                } else {
                    e.to_string()
                }
            })?;

        let stdin = child.stdin.take().ok_or("stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("stdout unavailable")?;
        let stderr = child.stderr.take().ok_or("stderr unavailable")?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let pending_reader = pending_requests.clone();
        let events_reader = events.clone();
        let events_log = events;
        let server_key = server_key.to_string();
        let server_key_reader = server_key.clone();
        let server_key_log = server_key;
        let tx_reader = tx.clone();

        let writer_thread = thread::spawn(move || {
            let mut input = stdin;
            for bytes in rx {
                if input.write_all(&bytes).is_err() {
                    break;
                }
                let _ = input.flush();
            }
        });

        let reader_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut content_length: Option<usize> = None;
                let mut line = String::new();

                loop {
                    line.clear();
                    if reader
                        .read_line(&mut line)
                        .ok()
                        .filter(|n| *n > 0)
                        .is_none()
                    {
                        return;
                    }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some(value) = trimmed.strip_prefix("Content-Length:")
                        && let Ok(len) = value.trim().parse::<usize>()
                    {
                        content_length = Some(len);
                    }
                }

                let Some(len) = content_length else { continue };
                let mut buf = vec![0u8; len];
                if reader.read_exact(&mut buf).is_err() {
                    return;
                }

                if let Ok(value) =
                    serde_json::from_slice::<serde_json::Value>(&buf)
                {
                    if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                        if let Some(method) =
                            value.get("method").and_then(|m| m.as_str())
                        {
                            handle_server_request(id, method, &tx_reader);
                        } else {
                            handle_client_response(
                                id,
                                &value,
                                &pending_reader,
                                &events_reader,
                            );
                        }
                    } else if let Some(method) =
                        value.get("method").and_then(|m| m.as_str())
                        && let Some(params) = value.get("params")
                    {
                        handle_server_notification(
                            method,
                            params,
                            &events_reader,
                            &server_key_reader,
                        );
                    }
                }
            }
        });

        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let _ = events_log.send(LspEvent::Log {
                    server_key: server_key_log.clone(),
                    message: line.to_string(),
                });
            }
        });

        let client = Self {
            child,
            writer: tx,
            documents: Arc::new(Mutex::new(HashMap::new())),
            request_id: AtomicU64::new(1),
            pending_requests,
            _writer_thread: writer_thread,
            _reader_thread: reader_thread,
            _stderr_thread: stderr_thread,
        };

        let initialize = json!({
            "jsonrpc": "2.0",
            "id": client.next_id(),
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {
                    "textDocument": {
                        "synchronization": {
                            "dynamicRegistration": false,
                            "willSave": false,
                            "didSave": true
                        }
                    },
                    "window": {
                        "workDoneProgress": true
                    }
                },
                "workspaceFolders": null
            }
        });
        client.send_message(&initialize);

        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        client.send_message(&initialized);

        Ok(client)
    }

    /// Generates the next unique request ID using atomic operations.
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Sends a JSON-RPC message to the LSP server.
    ///
    /// Formats the message with the required `Content-Length` header.
    fn send_message(&self, value: &serde_json::Value) {
        if let Ok(data) = serde_json::to_vec(&value) {
            let mut header =
                format!("Content-Length: {}\r\n\r\n", data.len()).into_bytes();
            header.extend_from_slice(&data);
            let _ = self.writer.send(header);
        }
    }

    /// Applies text changes to a document and converts them to JSON format.
    ///
    /// Also converts positions to UTF-16 as required by LSP.
    fn apply_change_and_convert(
        &self,
        uri: &str,
        changes: &[LspTextChange],
    ) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = docs.get_mut(uri) else { return out };

        for change in changes {
            let start = state.text.to_utf16_position(change.range.start);
            let end = state.text.to_utf16_position(change.range.end);

            out.push(json!({
                "range": {
                    "start": { "line": start.line, "character": start.character },
                    "end": { "line": end.line, "character": end.character }
                },
                "text": change.text
            }));

            state.text.apply_change(change);
        }
        out
    }
}

// =============================================================================
// Reader thread helper functions
// =============================================================================

/// Handles an LSP server request that requires a JSON-RPC response.
///
/// Currently handles `window/workDoneProgress/create` by replying with a null
/// result. Unknown methods are silently ignored.
fn handle_server_request(id: u64, method: &str, tx: &mpsc::Sender<Vec<u8>>) {
    if method == METHOD_WORK_DONE_PROGRESS_CREATE {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null
        });
        if let Ok(data) = serde_json::to_vec(&response) {
            let mut header =
                format!("Content-Length: {}\r\n\r\n", data.len()).into_bytes();
            header.extend_from_slice(&data);
            let _ = tx.send(header);
        }
    }
}

/// Dispatches a server response to the appropriate pending request handler.
///
/// Looks up the request kind by `id`, parses the result, and emits a
/// [`LspEvent::Hover`], [`LspEvent::Completion`], or [`LspEvent::Definition`].
fn handle_client_response(
    id: u64,
    value: &serde_json::Value,
    pending: &Arc<Mutex<HashMap<u64, LspRequestKind>>>,
    events: &mpsc::Sender<LspEvent>,
) {
    let kind = {
        let mut map = pending.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&id)
    };

    let Some(kind) = kind else { return };
    let result = value.get("result").unwrap_or(&serde_json::Value::Null);

    match kind {
        LspRequestKind::Hover => {
            let text = parse_hover_text(result).unwrap_or_default();
            let _ = events.send(LspEvent::Hover { text });
        }
        LspRequestKind::Completion => {
            let items = parse_completion_items(result);
            if !items.is_empty() {
                let _ = events.send(LspEvent::Completion { items });
            }
        }
        LspRequestKind::Definition => {
            if let Some((uri, range)) = parse_definition_location(result) {
                let _ = events.send(LspEvent::Definition { uri, range });
            }
        }
    }
}

/// Handles a server-initiated notification (e.g. `$/progress`).
///
/// Parses the progress payload and emits a [`LspEvent::Progress`].
/// Notifications for unknown methods are silently ignored.
fn handle_server_notification(
    method: &str,
    params: &serde_json::Value,
    events: &mpsc::Sender<LspEvent>,
    server_key: &str,
) {
    if method != METHOD_PROGRESS {
        return;
    }

    let Some(token) = params.get("token").and_then(|t| {
        t.as_str()
            .map(String::from)
            .or_else(|| t.as_i64().map(|i| i.to_string()))
    }) else {
        return;
    };

    let Some(val) = params.get("value") else { return };

    let kind = val.get("kind").and_then(|k| k.as_str()).unwrap_or("");
    let title = val
        .get("title")
        .and_then(|t| t.as_str())
        .map(String::from)
        .unwrap_or_default();
    let message = val.get("message").and_then(|m| m.as_str()).map(String::from);
    let percentage =
        val.get("percentage").and_then(|p| p.as_u64()).map(|p| p as u32);
    let done = kind == PROGRESS_KIND_END;

    let _ = events.send(LspEvent::Progress {
        token,
        server_key: server_key.to_string(),
        title,
        message,
        percentage,
        done,
    });
}

/// Sends shutdown/exit notifications and kills the process on drop.
impl Drop for LspProcessClient {
    fn drop(&mut self) {
        let shutdown = json!({
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "shutdown",
            "params": null
        });
        self.send_message(&shutdown);

        let exit = json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": {}
        });
        self.send_message(&exit);

        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
    }
}

// =============================================================================
// LSP Response Parsing Functions
// =============================================================================

/// Parses hover text from an LSP hover response.
fn parse_hover_text(result: &serde_json::Value) -> Option<String> {
    let contents = result.get("contents")?;
    hover_text_from_contents(contents)
}

/// Recursively extracts hover text from various content formats.
///
/// Handles strings, arrays, and objects with a `"value"` field.
fn hover_text_from_contents(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(items) => {
            let parts: Vec<String> =
                items.iter().filter_map(hover_text_from_contents).collect();
            if parts.is_empty() { None } else { Some(parts.join("\n")) }
        }
        serde_json::Value::Object(map) => {
            map.get("value").and_then(|v| v.as_str()).map(String::from)
        }
        _ => None,
    }
}

/// Parses completion items from an LSP completion response.
///
/// Handles both array responses and object responses with an `"items"` field.
fn parse_completion_items(result: &serde_json::Value) -> Vec<String> {
    let mut items = Vec::new();

    if let Some(array) = result.as_array() {
        items.extend(array.iter());
    } else if let Some(array) = result.get("items").and_then(|v| v.as_array()) {
        items.extend(array.iter());
    }

    items
        .iter()
        .filter_map(|item| item.get("label").and_then(|v| v.as_str()))
        .map(String::from)
        .collect()
}

/// Parses definition location from an LSP definition response.
///
/// Handles `Location`, `Location[]`, and `LocationLink[]` responses.
fn parse_definition_location(
    result: &serde_json::Value,
) -> Option<(String, crate::canvas_editor::lsp::LspRange)> {
    fn extract_location(
        loc: &serde_json::Value,
    ) -> Option<(String, crate::canvas_editor::lsp::LspRange)> {
        let uri = loc.get("uri")?.as_str()?.to_string();
        let range_val = loc.get("range")?;

        let start = range_val.get("start")?;
        let end = range_val.get("end")?;

        let start_line = start.get("line")?.as_u64()? as u32;
        let start_char = start.get("character")?.as_u64()? as u32;
        let end_line = end.get("line")?.as_u64()? as u32;
        let end_char = end.get("character")?.as_u64()? as u32;

        Some((
            uri,
            crate::canvas_editor::lsp::LspRange {
                start: crate::canvas_editor::lsp::LspPosition {
                    line: start_line,
                    character: start_char,
                },
                end: crate::canvas_editor::lsp::LspPosition {
                    line: end_line,
                    character: end_char,
                },
            },
        ))
    }

    fn extract_link(
        link: &serde_json::Value,
    ) -> Option<(String, crate::canvas_editor::lsp::LspRange)> {
        let uri = link.get("targetUri")?.as_str()?.to_string();
        let range_val =
            link.get("targetSelectionRange").or(link.get("targetRange"))?;

        let start = range_val.get("start")?;
        let end = range_val.get("end")?;

        let start_line = start.get("line")?.as_u64()? as u32;
        let start_char = start.get("character")?.as_u64()? as u32;
        let end_line = end.get("line")?.as_u64()? as u32;
        let end_char = end.get("character")?.as_u64()? as u32;

        Some((
            uri,
            crate::canvas_editor::lsp::LspRange {
                start: crate::canvas_editor::lsp::LspPosition {
                    line: start_line,
                    character: start_char,
                },
                end: crate::canvas_editor::lsp::LspPosition {
                    line: end_line,
                    character: end_char,
                },
            },
        ))
    }

    if let Some(array) = result.as_array() {
        if let Some(first) = array.first() {
            if first.get("targetUri").is_some() {
                extract_link(first)
            } else {
                extract_location(first)
            }
        } else {
            None
        }
    } else if result.is_object() {
        extract_location(result)
    } else {
        None
    }
}

// =============================================================================
// LspClient Trait Implementation
// =============================================================================

impl LspClient for LspProcessClient {
    fn did_open(&mut self, document: &LspDocument, text: &str) {
        let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        docs.insert(
            document.uri.clone(),
            DocumentState { text: TextModel::from_text(text) },
        );

        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": document.uri,
                    "languageId": document.language_id,
                    "version": document.version,
                    "text": text
                }
            }
        });
        self.send_message(&msg);
    }

    fn did_change(
        &mut self,
        document: &LspDocument,
        changes: &[LspTextChange],
    ) {
        let content_changes =
            self.apply_change_and_convert(&document.uri, changes);
        if content_changes.is_empty() {
            return;
        }

        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": document.uri,
                    "version": document.version
                },
                "contentChanges": content_changes
            }
        });
        self.send_message(&msg);
    }

    fn did_save(&mut self, document: &LspDocument, text: &str) {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": { "uri": document.uri },
                "text": text
            }
        });
        self.send_message(&msg);
    }

    fn did_close(&mut self, document: &LspDocument) {
        let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        docs.remove(&document.uri);

        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didClose",
            "params": {
                "textDocument": { "uri": document.uri }
            }
        });
        self.send_message(&msg);
    }

    fn request_hover(&mut self, document: &LspDocument, position: LspPosition) {
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = docs.get(&document.uri) else { return };
        let pos = state.text.to_utf16_position(position);

        let id = self.next_id();
        {
            let mut pending =
                self.pending_requests.lock().unwrap_or_else(|e| e.into_inner());
            pending.insert(id, LspRequestKind::Hover);
        }

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": document.uri },
                "position": { "line": pos.line, "character": pos.character }
            }
        });
        self.send_message(&msg);
    }

    fn request_completion(
        &mut self,
        document: &LspDocument,
        position: LspPosition,
    ) {
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = docs.get(&document.uri) else { return };
        let pos = state.text.to_utf16_position(position);

        let id = self.next_id();
        {
            let mut pending =
                self.pending_requests.lock().unwrap_or_else(|e| e.into_inner());
            pending.insert(id, LspRequestKind::Completion);
        }

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": document.uri },
                "position": { "line": pos.line, "character": pos.character },
                "context": { "triggerKind": 1 }
            }
        });
        self.send_message(&msg);
    }

    fn request_definition(
        &mut self,
        document: &LspDocument,
        position: LspPosition,
    ) {
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = docs.get(&document.uri) else { return };
        let pos = state.text.to_utf16_position(position);

        let id = self.next_id();
        {
            let mut pending =
                self.pending_requests.lock().unwrap_or_else(|e| e.into_inner());
            pending.insert(id, LspRequestKind::Definition);
        }

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": document.uri },
                "position": { "line": pos.line, "character": pos.character }
            }
        });
        self.send_message(&msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a `Content-Length`-framed JSON string from raw bytes sent on the channel.
    fn decode_sent(data: Vec<u8>) -> serde_json::Value {
        let header_end = data
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("missing header separator");
        let body = &data[header_end + 4..];
        serde_json::from_slice(body).expect("invalid JSON body")
    }

    // -------------------------------------------------------------------------
    // handle_server_request
    // -------------------------------------------------------------------------

    #[test]
    fn test_handle_server_request_work_done_progress_create() {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        handle_server_request(42, METHOD_WORK_DONE_PROGRESS_CREATE, &tx);

        let bytes = rx.try_recv().expect("expected a response on the channel");
        let value = decode_sent(bytes);
        assert_eq!(value["id"], 42);
        assert_eq!(value["jsonrpc"], "2.0");
        assert!(value["result"].is_null());
    }

    #[test]
    fn test_handle_server_request_unknown_method_ignored() {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        handle_server_request(1, "unknown/method", &tx);
        assert!(
            rx.try_recv().is_err(),
            "unknown methods must not send a reply"
        );
    }

    // -------------------------------------------------------------------------
    // handle_client_response
    // -------------------------------------------------------------------------

    #[test]
    fn test_handle_client_response_hover() {
        let (events_tx, events_rx) = mpsc::channel::<LspEvent>();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        pending.lock().unwrap().insert(1u64, LspRequestKind::Hover);

        let value = serde_json::json!({
            "id": 1,
            "result": { "contents": { "value": "hover info" } }
        });
        handle_client_response(1, &value, &pending, &events_tx);

        match events_rx.try_recv().expect("expected a Hover event") {
            LspEvent::Hover { text } => assert_eq!(text, "hover info"),
            _ => panic!("expected LspEvent::Hover"),
        }
        assert!(pending.lock().unwrap().is_empty());
    }

    #[test]
    fn test_handle_client_response_completion() {
        let (events_tx, events_rx) = mpsc::channel::<LspEvent>();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        pending.lock().unwrap().insert(2u64, LspRequestKind::Completion);

        let value = serde_json::json!({
            "id": 2,
            "result": { "items": [{ "label": "foo" }, { "label": "bar" }] }
        });
        handle_client_response(2, &value, &pending, &events_tx);

        match events_rx.try_recv().expect("expected a Completion event") {
            LspEvent::Completion { items } => {
                assert_eq!(items, vec!["foo", "bar"]);
            }
            _ => panic!("expected LspEvent::Completion"),
        }
    }

    #[test]
    fn test_handle_client_response_definition() {
        let (events_tx, events_rx) = mpsc::channel::<LspEvent>();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        pending.lock().unwrap().insert(3u64, LspRequestKind::Definition);

        let value = serde_json::json!({
            "id": 3,
            "result": {
                "uri": "file:///foo/bar.rs",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 5 }
                }
            }
        });
        handle_client_response(3, &value, &pending, &events_tx);

        match events_rx.try_recv().expect("expected a Definition event") {
            LspEvent::Definition { uri, .. } => {
                assert_eq!(uri, "file:///foo/bar.rs");
            }
            _ => panic!("expected LspEvent::Definition"),
        }
    }

    #[test]
    fn test_handle_client_response_unknown_id_ignored() {
        let (events_tx, events_rx) = mpsc::channel::<LspEvent>();
        let pending = Arc::new(Mutex::new(HashMap::new()));

        let value = serde_json::json!({ "id": 99, "result": null });
        handle_client_response(99, &value, &pending, &events_tx);
        assert!(
            events_rx.try_recv().is_err(),
            "unknown IDs must not emit events"
        );
    }

    // -------------------------------------------------------------------------
    // handle_server_notification
    // -------------------------------------------------------------------------

    #[test]
    fn test_handle_server_notification_progress_done() {
        let (events_tx, events_rx) = mpsc::channel::<LspEvent>();
        let params = serde_json::json!({
            "token": "my-token",
            "value": {
                "kind": "end",
                "title": "Indexing",
                "message": "done"
            }
        });

        handle_server_notification(
            METHOD_PROGRESS,
            &params,
            &events_tx,
            "lua-ls",
        );

        match events_rx.try_recv().expect("expected a Progress event") {
            LspEvent::Progress { token, done, server_key, .. } => {
                assert_eq!(token, "my-token");
                assert!(done);
                assert_eq!(server_key, "lua-ls");
            }
            _ => panic!("expected LspEvent::Progress"),
        }
    }

    #[test]
    fn test_handle_server_notification_progress_not_done() {
        let (events_tx, events_rx) = mpsc::channel::<LspEvent>();
        let params = serde_json::json!({
            "token": "tok",
            "value": { "kind": "report", "title": "Building" }
        });

        handle_server_notification(
            METHOD_PROGRESS,
            &params,
            &events_tx,
            "rust-analyzer",
        );

        match events_rx.try_recv().expect("expected a Progress event") {
            LspEvent::Progress { done, .. } => assert!(!done),
            _ => panic!("expected LspEvent::Progress"),
        }
    }

    #[test]
    fn test_handle_server_notification_unknown_method_ignored() {
        let (events_tx, events_rx) = mpsc::channel::<LspEvent>();
        let params = serde_json::json!({});
        handle_server_notification(
            "$/somethingElse",
            &params,
            &events_tx,
            "server",
        );
        assert!(events_rx.try_recv().is_err());
    }
}
