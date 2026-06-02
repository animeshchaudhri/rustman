use std::collections::HashMap;
use std::path::PathBuf;

use iced::Task;

use crate::{
    domain::collection::SavedRequest,
    message::{AppMsg, Message},
    services::{http, storage},
    state::{session::AppSession, sidebar::SidebarState, tabs::TabManager},
};

use super::AppState;

pub(crate) fn init() -> (AppState, Task<Message>) {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rustman");

    let db = storage::open(&data_dir).ok();

    let collections = db
        .as_ref()
        .and_then(|c| storage::get_collections(c).ok())
        .unwrap_or_default();

    let history = db
        .as_ref()
        .and_then(|c| storage::get_history(c).ok())
        .unwrap_or_default();

    let environments = db
        .as_ref()
        .and_then(|c| storage::get_environments(c).ok())
        .unwrap_or_default();

    let mut requests: HashMap<String, Vec<SavedRequest>> = HashMap::new();
    if let Some(db) = db.as_ref() {
        for col in &collections {
            if let Ok(reqs) = storage::get_requests(db, &col.id) {
                requests.insert(col.id.clone(), reqs);
            }
        }
    }

    let session: Option<AppSession> = db
        .as_ref()
        .and_then(|c| storage::load_session(c).ok())
        .flatten();

    let mut tabs = TabManager::default();
    if let Some(sess) = session {
        use crate::state::tabs::{body_syntax, make_code_editor, RequestTabState};
        use iced::widget::text_editor;
        let restored: Vec<RequestTabState> = sess
            .tabs
            .into_iter()
            .map(|snap| {
                let mut t = RequestTabState::new();
                t.id = snap.id;
                t.title = snap.title;
                t.method = snap.method;
                t.url = snap.url;
                t.headers = snap.headers;
                t.params = snap.params;
                t.body_type = snap.body_type.clone();
                t.body_editor = make_code_editor(&snap.body, body_syntax(&snap.body_type));
                t.form_fields = snap.form_fields;
                t.auth_type = snap.auth_type;
                t.bearer_token = snap.bearer_token;
                t.basic_user = snap.basic_user;
                t.basic_pass = snap.basic_pass;
                t.api_key_name = snap.api_key_name;
                t.api_key_value = snap.api_key_value;
                t.api_key_location = snap.api_key_location;
                t.cookie_string = snap.cookie_string;
                t.jwt_secret = snap.jwt_secret;
                t.jwt_subject = snap.jwt_subject;
                t.jwt_algo = snap.jwt_algo;
                t.pre_request_editor = text_editor::Content::with_text(&snap.pre_request_script);
                t.test_editor = text_editor::Content::with_text(&snap.test_script);
                t.timeout_ms = snap.timeout_ms;
                t.saved_as = snap.saved_as;
                t.active_request_tab = snap.active_request_tab;
                t.active_response_tab = snap.active_response_tab;
                t
            })
            .collect();
        if !restored.is_empty() {
            tabs.tabs = restored;
            tabs.active = sess.active_tab.min(tabs.tabs.len().saturating_sub(1));
        }
    }

    let state = AppState {
        tabs,
        sidebar: SidebarState::default(),
        collections,
        requests,
        history,
        environments,
        http_client: http::build_client(),
        db,
        data_dir,
        status_message: None,
        palette_open: false,
        palette_query: String::new(),
        palette_selected: 0,
        profile_avatar: None,
        parsed_cache: crate::services::cache::ParsedBodyCache::default(),
        save_dialog_open: false,
        save_dialog_name: String::new(),
        save_dialog_collection_id: None,
        save_dialog_new_col: false,
        save_dialog_new_col_name: String::new(),
        git_log: Vec::new(),
        curl_modal_open: false,
        curl_modal_command: String::new(),
        github_username: String::from("animeshchaudhri"),
        github_email: String::from("ac04@duck.com"),
        github_website: String::from("animesh.us"),
        accent_idx: 0,
        theme_is_dark: true,
        panel_split: 5,
        ui_scale: 1.0,
    };

    let avatar_task = Task::perform(
        async move {
            match reqwest::get("https://avatars.githubusercontent.com/animeshchaudhri").await {
                Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
                _ => None,
            }
        },
        |bytes| match bytes {
            Some(data) => Message::App(AppMsg::AvatarLoaded(data)),
            None => Message::App(AppMsg::Noop),
        },
    );

    (state, avatar_task)
}
