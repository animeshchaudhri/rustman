use iced::widget::text_editor;
use iced_code_editor::CodeEditor;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::domain::{
    collection::SavedRequest,
    request::{ApiKeyLocation, AuthType, BodyType, FormField, HttpMethod, KeyValue},
    response::{ConsoleEntry, HttpResponse, TestResult},
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
    pub pre_request_editor: text_editor::Content,
    pub test_editor: text_editor::Content,
    pub timeout_ms: u64,
    pub timeout_text: String,
    pub active_request_tab: crate::message::RequestTab,
    pub active_response_tab: crate::message::ResponseTab,
    pub response: Option<HttpResponse>,
    pub is_loading: bool,
    pub modified: bool,
    pub console: Vec<ConsoleEntry>,
    pub test_results: Vec<TestResult>,
    pub body_indent_tabs: bool,
    pub response_editor: CodeEditor,
    pub response_viewer_lines: usize,
    pub viewer_processing: bool,
    pub parsed_json: Option<serde_json::Value>,
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
            pre_request_editor: text_editor::Content::new(),
            test_editor: text_editor::Content::new(),
            timeout_ms: 30_000,
            timeout_text: "30000".to_owned(),
            active_request_tab: crate::message::RequestTab::Params,
            active_response_tab: crate::message::ResponseTab::Body,
            ws: WsState::default(),
            response: None,
            is_loading: false,
            modified: false,
            console: Vec::new(),
            test_results: Vec::new(),
            body_indent_tabs: false,
            response_editor: make_code_editor("", "txt"),
            response_viewer_lines: 0,
            viewer_processing: false,
            parsed_json: None,
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
        tab.pre_request_editor = text_editor::Content::with_text(&req.pre_request_script);
        tab.test_editor = text_editor::Content::with_text(&req.test_script);
        tab.timeout_ms = req.timeout_ms;
        tab.timeout_text = req.timeout_ms.to_string();
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
    pub timeout_ms: u64,
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
            pre_request_script: t.pre_request_editor.text(),
            test_script: t.test_editor.text(),
            timeout_ms: t.timeout_ms,
            saved_as: t.saved_as.clone(),
            active_request_tab: t.active_request_tab.clone(),
            active_response_tab: t.active_response_tab.clone(),
        }
    }
}
