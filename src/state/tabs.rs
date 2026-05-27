use iced::widget::text_editor;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::domain::{
    collection::SavedRequest,
    request::{ApiKeyLocation, AuthType, BodyType, FormField, HttpMethod, KeyValue},
    response::{ConsoleEntry, HttpResponse, TestResult},
};

/// Live WebSocket state for a tab.
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

/// A single request tab — everything needed to render and send one request.
#[derive(Debug)]
pub struct RequestTabState {
    pub id: String,
    pub title: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub params: Vec<KeyValue>,
    pub body_type: BodyType,
    pub body_editor: text_editor::Content,
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
    pub active_request_tab: crate::message::RequestTab,
    pub active_response_tab: crate::message::ResponseTab,
    pub response: Option<HttpResponse>,
    pub is_loading: bool,
    pub modified: bool,
    pub console: Vec<ConsoleEntry>,
    pub test_results: Vec<TestResult>,
    pub search_query: String,
    pub search_visible: bool,
    /// true = use tabs for Format, false = 2-space indent
    pub body_indent_tabs: bool,
    /// true = show raw text, false = show pretty-printed JSON
    pub json_raw_mode: bool,
    /// Read-only content for the response body viewer (selectable text)
    pub response_viewer: text_editor::Content,
    /// Line count of response_viewer — stored so view() never calls .text() to count lines
    pub response_viewer_lines: usize,
    /// True while background JSON parse + Content build is in progress
    pub viewer_processing: bool,
    /// Pre-parsed JSON value — set once in update() when response arrives, never in view()
    pub parsed_json: Option<serde_json::Value>,
    /// WebSocket state
    pub ws: WsState,
    /// collection_id + saved_request_id if this tab is linked to a saved request
    pub saved_as: Option<(String, String)>,
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
            body_editor: text_editor::Content::new(),
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
            active_request_tab: crate::message::RequestTab::Params,
            active_response_tab: crate::message::ResponseTab::Body,
            ws: WsState::default(),
            response: None,
            is_loading: false,
            modified: false,
            console: Vec::new(),
            test_results: Vec::new(),
            search_query: String::new(),
            search_visible: false,
            body_indent_tabs: false,
            json_raw_mode: false,
            response_viewer: text_editor::Content::new(),
            response_viewer_lines: 0,
            viewer_processing: false,
            parsed_json: None,
            saved_as: None,
        }
    }

    pub fn from_saved(req: &SavedRequest) -> Self {
        let mut tab = Self::new();
        tab.title = req.name.clone();
        tab.method = req.method.clone();
        tab.url = req.url.clone();
        tab.headers = req.headers.clone();
        tab.params = req.params.clone();
        tab.body_type = req.body_type.clone();
        tab.body_editor = text_editor::Content::with_text(&req.body);
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
        tab.saved_as = Some((req.collection_id.clone(), req.id.clone()));
        tab
    }
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
    pub jwt_secret: String,
    #[serde(default)]
    pub jwt_subject: String,
    #[serde(default = "default_jwt_algo")]
    pub jwt_algo: String,
    pub pre_request_script: String,
    pub test_script: String,
    pub timeout_ms: u64,
    pub saved_as: Option<(String, String)>,
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
            body: t.body_editor.text(),
            form_fields: t.form_fields.clone(),
            auth_type: t.auth_type.clone(),
            bearer_token: t.bearer_token.clone(),
            basic_user: t.basic_user.clone(),
            basic_pass: t.basic_pass.clone(),
            api_key_name: t.api_key_name.clone(),
            api_key_value: t.api_key_value.clone(),
            api_key_location: t.api_key_location.clone(),
            cookie_string: t.cookie_string.clone(),
            jwt_secret: t.jwt_secret.clone(),
            jwt_subject: t.jwt_subject.clone(),
            jwt_algo: t.jwt_algo.clone(),
            pre_request_script: t.pre_request_editor.text(),
            test_script: t.test_editor.text(),
            timeout_ms: t.timeout_ms,
            saved_as: t.saved_as.clone(),
        }
    }
}
