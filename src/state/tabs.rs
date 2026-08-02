use iced::widget::text_editor;
use iced_code_editor::CodeEditor;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::domain::{
    collection::SavedRequest,
    request::{ApiKeyLocation, AuthType, BodyType, FormField, HttpMethod, KeyValue},
    response::HttpResponse,
};
use crate::jobs::JobManager;

#[derive(Debug, Default)]
pub struct WsState {
    pub url: String,
    pub connected: bool,
    pub connecting: bool,
    pub draft: String,
    pub messages: Vec<WsMessage>,
    pub outgoing_tx: Option<mpsc::Sender<String>>,
}

#[derive(Debug, Clone)]
pub struct WsMessage {
    pub text: String,
    pub is_outgoing: bool,
}

/// A rich preview for a binary response body, populated asynchronously once
/// the response arrives (see `AppMsg::SpreadsheetPreviewReady` /
/// `AppMsg::PdfPreviewReady`). `None` means either the response isn't binary,
/// the binary type has no richer preview, or the preview hasn't finished
/// rendering/parsing yet.
#[derive(Default)]
pub enum ResponsePreview {
    #[default]
    None,
    Spreadsheet(Result<crate::services::spreadsheet::ParsedSheet, String>),
    Pdf(PdfPreviewState),
}

pub struct PdfPreviewState {
    pub page_count: usize,
    pub current_page: usize,
    /// `None` while the current page is still rendering.
    pub current_image: Option<iced::widget::image::Handle>,
}

/// One `test(name, condition)` call's outcome from a test script run.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
}

pub struct RequestTabState {
    pub id: String,
    pub title: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub params: Vec<KeyValue>,
    pub body_type: BodyType,
    pub body_editor: CodeEditor,
    pub form_fields: Vec<FormField>,
    pub auth_type: AuthType,
    pub bearer_token: String,
    pub basic_user: String,
    pub basic_pass: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_location: ApiKeyLocation,
    pub cookie_string: String,
    pub cookies: Vec<KeyValue>,
    pub jwt_secret: String,
    pub jwt_subject: String,
    pub jwt_algo: String,
    pub pre_request_editor: CodeEditor,
    pub test_editor: CodeEditor,
    pub active_request_tab: crate::message::RequestTab,
    pub active_response_tab: crate::message::ResponseTab,
    pub response: Option<HttpResponse>,
    pub is_loading: bool,
    pub modified: bool,
    pub body_indent_tabs: bool,
    pub response_editor: CodeEditor,
    pub response_viewer_lines: usize,
    pub viewer_processing: bool,
    pub parsed_json: Option<serde_json::Value>,
    pub response_preview: ResponsePreview,
    /// Results of the last test-script run against this tab's response.
    pub test_results: Vec<TestResult>,
    /// Set if the pre-request or test script itself failed to run (syntax
    /// error, unknown function, etc.) — distinct from a test *assertion*
    /// failing, which shows up as a normal failed `TestResult` instead.
    pub script_error: Option<String>,
    /// `print(...)` calls from the last pre-request + test script run,
    /// in order, for debugging a script without needing an assertion.
    pub script_logs: Vec<String>,
    /// `script_logs` joined into one selectable/copyable text_editor's
    /// content — kept in sync wherever `script_logs` is set. A plain `text`
    /// widget can't be selected/copied in this iced version, and there's no
    /// read-only mode, so this is edited-but-ignored: actions apply so the
    /// selection renders correctly, but nothing reads the result back out.
    pub console_editor: text_editor::Content,
    pub ws: WsState,
    pub saved_as: Option<(String, String)>,
    pub jobs: JobManager,
    pub undo: Vec<EditSnapshot>,
    pub redo: Vec<EditSnapshot>,
    pub last_edit: Option<EditKind>,
    pub headers_bulk: Option<text_editor::Content>,
    pub params_bulk: Option<text_editor::Content>,
}

/// Identifies the field an edit targets, so a run of keystrokes in one field
/// coalesces into a single undo entry instead of one per character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Url,
    HeaderKey(usize),
    HeaderValue(usize),
    ParamKey(usize),
    ParamValue(usize),
    HeadersBulk,
    ParamsBulk,
    Bearer,
    BasicUser,
    BasicPass,
    ApiKeyName,
    ApiKeyValue,
    Cookie,
    JwtSecret,
    JwtSubject,
    JwtAlgo,
}

/// A restorable checkpoint of the text-editable request fields. The body and
/// script editors are excluded — they manage their own undo history.
#[derive(Clone)]
pub struct EditSnapshot {
    url: String,
    method: HttpMethod,
    params: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    auth_type: AuthType,
    bearer_token: String,
    basic_user: String,
    basic_pass: String,
    api_key_name: String,
    api_key_value: String,
    api_key_location: ApiKeyLocation,
    cookie_string: String,
    jwt_secret: String,
    jwt_subject: String,
    jwt_algo: String,
}

impl RequestTabState {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "New Request".to_owned(),
            method: HttpMethod::Get,
            url: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            body_type: BodyType::None,
            body_editor: make_code_editor("", "json"),
            form_fields: Vec::new(),
            auth_type: AuthType::None,
            bearer_token: String::new(),
            basic_user: String::new(),
            basic_pass: String::new(),
            api_key_name: String::new(),
            api_key_value: String::new(),
            api_key_location: ApiKeyLocation::Header,
            cookie_string: String::new(),
            cookies: Vec::new(),
            jwt_secret: String::new(),
            jwt_subject: String::new(),
            jwt_algo: "HS256".to_owned(),
            pre_request_editor: make_code_editor("", "txt"),
            test_editor: make_code_editor("", "txt"),
            active_request_tab: crate::message::RequestTab::Params,
            active_response_tab: crate::message::ResponseTab::Body,
            ws: WsState::default(),
            response: None,
            is_loading: false,
            modified: false,
            body_indent_tabs: false,
            response_editor: make_code_editor("", "txt"),
            response_viewer_lines: 0,
            viewer_processing: false,
            parsed_json: None,
            response_preview: ResponsePreview::default(),
            test_results: Vec::new(),
            script_error: None,
            script_logs: Vec::new(),
            console_editor: text_editor::Content::new(),
            saved_as: None,
            jobs: JobManager::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: None,
            headers_bulk: None,
            params_bulk: None,
        }
    }

    /// Capture the text-editable fields for the undo history.
    pub fn edit_snapshot(&self) -> EditSnapshot {
        EditSnapshot {
            url: self.url.clone(),
            method: self.method.clone(),
            params: self.params.clone(),
            headers: self.headers.clone(),
            auth_type: self.auth_type.clone(),
            bearer_token: self.bearer_token.clone(),
            basic_user: self.basic_user.clone(),
            basic_pass: self.basic_pass.clone(),
            api_key_name: self.api_key_name.clone(),
            api_key_value: self.api_key_value.clone(),
            api_key_location: self.api_key_location.clone(),
            cookie_string: self.cookie_string.clone(),
            jwt_secret: self.jwt_secret.clone(),
            jwt_subject: self.jwt_subject.clone(),
            jwt_algo: self.jwt_algo.clone(),
        }
    }

    /// Restore a snapshot produced by [`Self::edit_snapshot`].
    pub fn restore_edit(&mut self, s: EditSnapshot) {
        self.url = s.url;
        self.method = s.method;
        self.params = s.params;
        self.headers = s.headers;
        self.auth_type = s.auth_type;
        self.bearer_token = s.bearer_token;
        self.basic_user = s.basic_user;
        self.basic_pass = s.basic_pass;
        self.api_key_name = s.api_key_name;
        self.api_key_value = s.api_key_value;
        self.api_key_location = s.api_key_location;
        self.cookie_string = s.cookie_string;
        self.jwt_secret = s.jwt_secret;
        self.jwt_subject = s.jwt_subject;
        self.jwt_algo = s.jwt_algo;
    }

    /// Push the current state onto the undo stack (capped) and clear the redo stack.
    pub fn push_undo(&mut self) {
        self.undo.push(self.edit_snapshot());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Replaces `script_logs` wholesale and rebuilds `console_editor` to match.
    pub fn set_script_logs(&mut self, logs: Vec<String>) {
        self.console_editor = text_editor::Content::with_text(&logs.join("\n"));
        self.script_logs = logs;
    }

    /// Appends to `script_logs` (e.g. a test script's logs on top of a
    /// pre-request script's) and rebuilds `console_editor` to match.
    pub fn extend_script_logs(&mut self, logs: Vec<String>) {
        self.script_logs.extend(logs);
        self.console_editor = text_editor::Content::with_text(&self.script_logs.join("\n"));
    }

    pub fn set_viewer_content(&mut self, text: &str, is_json: bool) {
        self.response_viewer_lines = text.lines().count().max(1);
        let syntax = if is_json { "json" } else { "txt" };
        self.response_editor = make_code_editor(text, syntax);
    }

    pub fn reset_body_editor(&mut self, text: &str) {
        let syntax = body_syntax(&self.body_type);
        let indent = if self.body_indent_tabs {
            iced_code_editor::IndentStyle::Tab
        } else {
            iced_code_editor::IndentStyle::Spaces(4)
        };
        let mut ed = make_code_editor(text, syntax);
        ed.set_indent_style(indent);
        self.body_editor = ed;
    }

    /// Replace the whole body through the editor's edit history (select-all +
    /// paste) so the change stays undoable, instead of rebuilding the editor.
    pub fn replace_body_text(&mut self, text: String) -> iced::Task<iced_code_editor::Message> {
        if text.is_empty() {
            self.reset_body_editor(&text);
            return iced::Task::none();
        }
        let _ = self.body_editor.update(&iced_code_editor::Message::SelectAll);
        self.body_editor.update(&iced_code_editor::Message::Paste(text))
    }

    pub fn sync_editor_themes(&mut self) {
        let style = crate::ui::theme::Palette::code_editor_style();
        self.body_editor.set_theme(style);
        self.response_editor.set_theme(style);
    }


    pub fn is_websocket(&self) -> bool {
        let u = self.url.trim_start().to_ascii_lowercase();
        u.starts_with("ws://") || u.starts_with("wss://")
    }

    pub fn from_saved(req: &SavedRequest) -> Self {
        let mut tab = Self::new();
        tab.title = req.name.clone();
        tab.method = req.method.clone();
        let (url, params) = crate::domain::request::reconcile_url_params(&req.url, &req.params);
        tab.url = url;
        tab.headers = req.headers.clone();
        tab.params = params;
        tab.body_type = req.body_type.clone();
        tab.body_editor = make_code_editor(&req.body, body_syntax(&req.body_type));
        tab.form_fields = req.form_data_fields.clone();
        tab.auth_type = req.auth_type.clone();
        tab.bearer_token = req.bearer_token.clone();
        tab.basic_user = req.basic_user.clone();
        tab.basic_pass = req.basic_pass.clone();
        tab.api_key_name = req.api_key_name.clone();
        tab.api_key_value = req.api_key_value.clone();
        tab.api_key_location = req.api_key_location.clone();
        tab.cookie_string = req.cookie_string.clone();
        tab.cookies = req.cookies.clone();
        tab.jwt_secret = req.jwt_secret.clone();
        tab.jwt_subject = req.jwt_subject.clone();
        tab.jwt_algo = req.jwt_algo.clone();
        tab.pre_request_editor = make_code_editor(&req.pre_request_script, "txt");
        tab.test_editor = make_code_editor(&req.test_script, "txt");
        tab.saved_as = Some((req.collection_id.clone(), req.id.clone()));
        tab
    }
}

/// Syntax token for a given body type.
pub fn body_syntax(body_type: &BodyType) -> &'static str {
    match body_type {
        BodyType::Json => "json",
        _ => "txt",
    }
}

/// Create a themed CodeEditor for the given content and syntax.
pub fn make_code_editor(content: &str, syntax: &str) -> CodeEditor {
    use crate::ui::theme::{Palette, MONO};
    let mut ed = CodeEditor::new(content, syntax);
    ed.set_theme(Palette::code_editor_style());
    ed.set_font(MONO);
    ed.set_font_size(12.0, true);
    // Roomier than the auto 1.43x — JSON reads better with a bit of air.
    ed.set_line_height(18.5);
    ed.set_indent_style(iced_code_editor::IndentStyle::Spaces(4));
    ed
}

pub struct TabManager {
    pub tabs: Vec<RequestTabState>,
    pub active: usize,
}

impl Default for TabManager {
    fn default() -> Self {
        Self { tabs: vec![RequestTabState::new()], active: 0 }
    }
}

impl TabManager {
    pub fn active_tab(&self) -> &RequestTabState {
        &self.tabs[self.active]
    }

    pub fn active_tab_mut(&mut self) -> &mut RequestTabState {
        &mut self.tabs[self.active]
    }

    pub fn new_tab(&mut self) {
        self.tabs.push(RequestTabState::new());
        self.active = self.tabs.len() - 1;
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() == 1 {
            self.tabs[0] = RequestTabState::new();
            return;
        }
        self.tabs.remove(idx);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }

    pub fn switch_to(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = idx;
            // Belt-and-suspenders redraw: this tab's editors may not have
            // been drawn since a viewport change (window resize, panel
            // split drag) while it wasn't visible, so force a clean render
            // now rather than trusting whatever was last cached.
            let tab = &mut self.tabs[idx];
            tab.body_editor.invalidate_render_cache();
            tab.response_editor.invalidate_render_cache();
            tab.pre_request_editor.invalidate_render_cache();
            tab.test_editor.invalidate_render_cache();
        }
    }


    pub fn reorder(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let active_id = self.tabs[self.active].id.clone();
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        if let Some(new_active) = self.tabs.iter().position(|t| t.id == active_id) {
            self.active = new_active;
        }
    }

    pub fn open_request(&mut self, req: &SavedRequest) {
        let req_id = &req.id;
        if let Some(idx) = self
            .tabs
            .iter()
            .position(|t| t.saved_as.as_ref().map(|(_, id)| id) == Some(req_id))
        {
            self.active = idx;
        } else {
            self.tabs.push(RequestTabState::from_saved(req));
            self.active = self.tabs.len() - 1;
        }
    }
}

// ── Serialisable snapshot for session persistence ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSnapshot {
    pub id: String,
    pub title: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub params: Vec<KeyValue>,
    pub body_type: BodyType,
    pub body: String,
    pub form_fields: Vec<FormField>,
    pub auth_type: AuthType,
    pub bearer_token: String,
    pub basic_user: String,
    pub basic_pass: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_location: ApiKeyLocation,
    pub cookie_string: String,
    #[serde(default)]
    pub cookies: Vec<KeyValue>,
    #[serde(default)]
    pub jwt_secret: String,
    #[serde(default)]
    pub jwt_subject: String,
    #[serde(default = "default_jwt_algo")]
    pub jwt_algo: String,
    pub pre_request_script: String,
    pub test_script: String,
    pub saved_as: Option<(String, String)>,
    #[serde(default)]
    pub active_request_tab: crate::message::RequestTab,
    #[serde(default)]
    pub active_response_tab: crate::message::ResponseTab,
}

fn default_jwt_algo() -> String { "HS256".to_owned() }

impl From<&RequestTabState> for TabSnapshot {
    fn from(t: &RequestTabState) -> Self {
        Self {
            id: t.id.clone(),
            title: t.title.clone(),
            method: t.method.clone(),
            url: t.url.clone(),
            headers: t.headers.clone(),
            params: t.params.clone(),
            body_type: t.body_type.clone(),
            body: t.body_editor.content(),
            form_fields: t.form_fields.clone(),
            auth_type: t.auth_type.clone(),
            bearer_token: t.bearer_token.clone(),
            basic_user: t.basic_user.clone(),
            basic_pass: t.basic_pass.clone(),
            api_key_name: t.api_key_name.clone(),
            api_key_value: t.api_key_value.clone(),
            api_key_location: t.api_key_location.clone(),
            cookie_string: t.cookie_string.clone(),
            cookies: t.cookies.clone(),
            jwt_secret: t.jwt_secret.clone(),
            jwt_subject: t.jwt_subject.clone(),
            jwt_algo: t.jwt_algo.clone(),
            pre_request_script: t.pre_request_editor.content(),
            test_script: t.test_editor.content(),
            saved_as: t.saved_as.clone(),
            active_request_tab: t.active_request_tab.clone(),
            active_response_tab: t.active_response_tab.clone(),
        }
    }
}

#[cfg(test)]
mod session_round_trip_tests {
    use super::*;

    fn restore(raw_body: &str) -> String {
        let mut tab = RequestTabState::new();
        tab.body_editor = make_code_editor(raw_body, "json");
        let snap: TabSnapshot = (&tab).into();
        let encoded = serde_json::to_string(&snap).unwrap();
        let decoded: TabSnapshot = serde_json::from_str(&encoded).unwrap();
        let restored = make_code_editor(&decoded.body, "json");
        restored.content()
    }

    #[test]
    fn pretty_json_survives_session_round_trip() {
        let raw = "{\n  \"name\": \"anime\",\n  \"tags\": [\"a\", \"b\"],\n  \"n\": 42\n}";
        assert_eq!(restore(raw), raw);
    }

    #[test]
    fn minified_json_survives_session_round_trip() {
        let raw = "{\"name\":\"anime\",\"tags\":[\"a\",\"b\"],\"n\":42}";
        assert_eq!(restore(raw), raw);
    }

    #[test]
    fn json_with_escapes_survives_session_round_trip() {
        let raw = "{\n  \"body\": \"line1\\nline2\",\n  \"q\": \"say \\\"hi\\\"\"\n}";
        assert_eq!(restore(raw), raw);
    }

    #[test]
    fn crlf_body_survives_session_round_trip() {
        let raw = "{\r\n  \"a\": 1\r\n}";
        assert_eq!(restore(raw), raw);
    }
}
