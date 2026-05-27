use std::collections::HashMap;
use std::path::PathBuf;

use base64::Engine as _;
use iced::{clipboard, Element, Size, Task};
use rfd;
use serde::Serialize as _;
use rusqlite::Connection;

use futures_util::SinkExt as _;
use iced::keyboard;

use crate::{
    domain::{
        collection::{Collection, SavedRequest},
        environment::AppEnvironment,
        history::HistoryEntry,
    },
    message::{AppMsg, ImportMsg, Message, PaletteMsg, RequestMsg, ResponseMsg, SettingsMsg, SidebarMsg, StorageMsg, WsMsg},
    services::{curl, http, storage, websocket},
    state::{session::AppSession, sidebar::SidebarState, tabs::TabManager},
};

// ── Top-level application state ───────────────────────────────────────────────

pub struct AppState {
    pub tabs: TabManager,
    pub sidebar: SidebarState,
    pub collections: Vec<Collection>,
    pub requests: HashMap<String, Vec<SavedRequest>>,
    pub history: Vec<HistoryEntry>,
    pub environments: Vec<AppEnvironment>,
    pub http_client: reqwest::Client,
    pub db: Option<Connection>,
    pub data_dir: PathBuf,
    pub status_message: Option<String>,
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_selected: usize,
    pub profile_avatar: Option<Vec<u8>>,
    pub save_dialog_open: bool,
    pub save_dialog_name: String,
    pub save_dialog_collection_id: Option<String>,
    pub save_dialog_new_col: bool,
    pub save_dialog_new_col_name: String,
    pub git_log: Vec<crate::services::vcs::CommitInfo>,
    pub curl_modal_open: bool,
    pub curl_modal_command: String,
    pub github_username: String,
    pub github_email: String,
    pub github_website: String,
    pub accent_idx: usize,
    pub theme_is_dark: bool,
    /// Split ratio: request panel FillPortion (1-9). Response = 10 - panel_split.
    pub panel_split: u16,
    /// UI zoom level (0.7 – 2.0). Applied via iced scale_factor.
    pub ui_scale: f64,
}

impl AppState {
    fn active_env(&self) -> Option<&AppEnvironment> {
        self.environments.iter().find(|e| e.is_active)
    }
}

// ── iced Application trait ────────────────────────────────────────────────────

pub fn run() -> iced::Result {
    let icon = iced::window::icon::from_file_data(
        include_bytes!("../public/icon.png"),
        None,
    )
    .ok();
    let window = iced::window::Settings {
        size: Size::new(1280.0, 800.0),
        icon,
        ..iced::window::Settings::default()
    };
    iced::application("Rustman", update, view)
        .window(window)
        .subscription(subscription)
        .theme(app_theme)
        .scale_factor(|state: &AppState| state.ui_scale)
        .run_with(init)
}

fn app_theme(state: &AppState) -> iced::Theme {
    if state.theme_is_dark {
        iced::Theme::TokyoNightStorm
    } else {
        iced::Theme::GruvboxLight
    }
}

fn init() -> (AppState, Task<Message>) {
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
        use crate::state::tabs::RequestTabState;
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
                t.body_type = snap.body_type;
                t.body_editor = text_editor::Content::with_text(&snap.body);
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
                Ok(resp) if resp.status().is_success() => {
                    resp.bytes().await.ok().map(|b| b.to_vec())
                }
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

// ── Update ────────────────────────────────────────────────────────────────────

fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::Sidebar(msg) => handle_sidebar(state, msg),
        Message::Request(msg) => handle_request(state, msg),
        Message::Response(msg) => handle_response(state, msg),
        Message::Storage(msg) => handle_storage(state, msg),
        Message::WebSocket(msg) => handle_ws(state, msg),
        Message::Palette(msg) => handle_palette(state, msg),
        Message::SaveDialog(msg) => handle_save_dialog(state, msg),
        Message::Git(msg) => handle_git(state, msg),
        Message::Import(msg) => handle_import(state, msg),
        Message::App(msg) => handle_app(state, msg),
        Message::Settings(msg) => handle_settings(state, msg),
        Message::Layout(msg) => {
            use crate::message::LayoutMsg;
            match msg {
                LayoutMsg::ZoomIn => {
                    state.ui_scale = (state.ui_scale + 0.1).min(2.0);
                }
                LayoutMsg::ZoomOut => {
                    state.ui_scale = (state.ui_scale - 0.1).max(0.5);
                }
                LayoutMsg::ZoomReset => {
                    state.ui_scale = 1.0;
                }
            }
            Task::none()
        }
    }
}

fn subscription(state: &AppState) -> iced::Subscription<Message> {
    let global = keyboard::on_key_press(kbd_global);
    let kbd = if state.palette_open {
        iced::Subscription::batch([global, keyboard::on_key_press(kbd_palette_nav)])
    } else {
        global
    };

    let ws_subs: Vec<_> = state
        .tabs
        .tabs
        .iter()
        .filter(|t| t.ws.connecting || t.ws.connected)
        .map(|t| {
            let url = t.ws.url.clone();
            let tab_id = t.id.clone();
            iced::Subscription::run_with_id(
                tab_id.clone(),
                iced::stream::channel(256, move |mut output| async move {
                    use crate::services::websocket::WsEvent;
                    match websocket::connect(url).await {
                        Ok((handle, mut rx)) => {
                            let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<String>(64);
                            let _ = output
                                .send(Message::WebSocket(WsMsg::Handshake {
                                    tab_id: tab_id.clone(),
                                    sender: out_tx,
                                }))
                                .await;
                            loop {
                                tokio::select! {
                                    msg = out_rx.recv() => {
                                        match msg {
                                            Some(m) => handle.send_text(m).await,
                                            None => break,
                                        }
                                    }
                                    event = rx.recv() => {
                                        match event {
                                            Some(WsEvent::Text(t)) => {
                                                let _ = output.send(Message::WebSocket(WsMsg::TextFrame(t))).await;
                                            }
                                            Some(WsEvent::Error(e)) => {
                                                let _ = output.send(Message::WebSocket(WsMsg::Error(e))).await;
                                                break;
                                            }
                                            Some(WsEvent::Disconnected) | None => {
                                                let _ = output.send(Message::WebSocket(WsMsg::Disconnected)).await;
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = output.send(Message::WebSocket(WsMsg::Error(e))).await;
                        }
                    }
                }),
            )
        })
        .collect();

    let mut all = vec![kbd];
    all.extend(ws_subs);
    iced::Subscription::batch(all)
}

fn kbd_global(key: keyboard::Key, mods: keyboard::Modifiers) -> Option<Message> {
    use keyboard::key::Named;
    match key.as_ref() {
        keyboard::Key::Character("p") if mods.command() => Some(Message::Palette(PaletteMsg::Open)),
        keyboard::Key::Character("t") if mods.command() => Some(Message::Request(RequestMsg::NewTab)),
        keyboard::Key::Character("w") if mods.command() => Some(Message::Request(RequestMsg::CloseCurrentTab)),
        keyboard::Key::Character("s") if mods.command() => Some(Message::Request(RequestMsg::SaveRequest)),
        keyboard::Key::Character("e") if mods.command() => Some(Message::Request(RequestMsg::ExportCurl)),
        keyboard::Key::Character("f") if mods.command() => Some(Message::Response(ResponseMsg::ToggleSearch)),
        keyboard::Key::Character("/") if mods.command() => Some(Message::Request(RequestMsg::CommentToggle)),
        keyboard::Key::Character("=") if mods.command() => Some(Message::Layout(crate::message::LayoutMsg::ZoomIn)),
        keyboard::Key::Character("-") if mods.command() => Some(Message::Layout(crate::message::LayoutMsg::ZoomOut)),
        keyboard::Key::Character("0") if mods.command() => Some(Message::Layout(crate::message::LayoutMsg::ZoomReset)),
        keyboard::Key::Named(Named::Enter) if mods.command() => Some(Message::Request(RequestMsg::Send)),
        keyboard::Key::Character("1") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(0))),
        keyboard::Key::Character("2") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(1))),
        keyboard::Key::Character("3") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(2))),
        keyboard::Key::Character("4") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(3))),
        keyboard::Key::Character("5") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(4))),
        keyboard::Key::Character("6") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(5))),
        keyboard::Key::Character("7") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(6))),
        keyboard::Key::Character("8") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(7))),
        keyboard::Key::Character("9") if mods.alt() => Some(Message::Request(RequestMsg::SwitchTab(8))),
        _ => None,
    }
}

fn kbd_palette_nav(key: keyboard::Key, _mods: keyboard::Modifiers) -> Option<Message> {
    use keyboard::key::Named;
    match key.as_ref() {
        keyboard::Key::Named(Named::Escape) => Some(Message::Palette(PaletteMsg::Close)),
        keyboard::Key::Named(Named::ArrowDown) => Some(Message::Palette(PaletteMsg::MoveDown)),
        keyboard::Key::Named(Named::ArrowUp) => Some(Message::Palette(PaletteMsg::MoveUp)),
        keyboard::Key::Named(Named::Enter) => Some(Message::Palette(PaletteMsg::Confirm)),
        _ => None,
    }
}

fn handle_save_dialog(state: &mut AppState, msg: crate::message::SaveDialogMsg) -> Task<Message> {
    use crate::message::SaveDialogMsg;
    match msg {
        SaveDialogMsg::Open => {
            let tab = state.tabs.active_tab();
            state.save_dialog_name = if tab.url.is_empty() { tab.title.clone() } else { tab.url.clone() };
            state.save_dialog_collection_id = state.collections.first().map(|c| c.id.clone());
            state.save_dialog_new_col = false;
            state.save_dialog_new_col_name = String::new();
            state.save_dialog_open = true;
        }
        SaveDialogMsg::Close => {
            state.save_dialog_open = false;
            state.save_dialog_new_col = false;
            state.save_dialog_new_col_name = String::new();
        }
        SaveDialogMsg::NameChanged(s) => state.save_dialog_name = s,
        SaveDialogMsg::CollectionSelected(id) => {
            state.save_dialog_collection_id = Some(id);
            state.save_dialog_new_col = false;
        }
        SaveDialogMsg::ToggleNewCollection => {
            state.save_dialog_new_col = !state.save_dialog_new_col;
        }
        SaveDialogMsg::NewCollectionNameChanged(s) => state.save_dialog_new_col_name = s,
        SaveDialogMsg::Confirm => {
            state.save_dialog_open = false;
            let name = state.save_dialog_name.clone();

            if state.save_dialog_new_col {
                let col_name = if state.save_dialog_new_col_name.trim().is_empty() {
                    "New Collection".to_owned()
                } else {
                    state.save_dialog_new_col_name.trim().to_owned()
                };
                let new_col_id = uuid::Uuid::new_v4().to_string();
                let col = Collection {
                    id: new_col_id.clone(),
                    name: col_name,
                    created_at: chrono::Utc::now().timestamp_millis(),
                };
                if let Some(db) = &state.db {
                    let _ = storage::create_collection(db, &col);
                }
                state.collections.push(col);
                state.save_dialog_collection_id = Some(new_col_id);
                state.save_dialog_new_col = false;
                state.save_dialog_new_col_name = String::new();
            }

            let col_id = state
                .save_dialog_collection_id
                .clone()
                .or_else(|| state.collections.first().map(|c| c.id.clone()))
                .unwrap_or_default();

            if col_id.is_empty() {
                let new_col_id = uuid::Uuid::new_v4().to_string();
                let col = Collection {
                    id: new_col_id.clone(),
                    name: "My Requests".to_owned(),
                    created_at: chrono::Utc::now().timestamp_millis(),
                };
                if let Some(db) = &state.db {
                    let _ = storage::create_collection(db, &col);
                }
                state.collections.push(col);
                state.save_dialog_collection_id = Some(new_col_id);
            }

            let col_id = state.save_dialog_collection_id.clone().unwrap_or_default();
            let tab = state.tabs.active_tab_mut();
            let req_id = uuid::Uuid::new_v4().to_string();
            tab.title = name.clone();
            tab.saved_as = Some((col_id.clone(), req_id.clone()));

            return save_request(state);
        }
    }
    Task::none()
}

fn handle_git(state: &mut AppState, msg: crate::message::GitMsg) -> Task<Message> {
    use crate::message::GitMsg;
    match msg {
        GitMsg::LogLoaded(log) => state.git_log = log,
        GitMsg::CommitAll => {
            let data_dir = state.data_dir.clone();
            let collections = state.collections.clone();
            let requests = state.requests.clone();
            return Task::perform(
                async move {
                    use crate::services::vcs;
                    match vcs::open_repo(&data_dir) {
                        Ok(repo) => {
                            for col in &collections {
                                let reqs = requests.get(&col.id).map(|v| v.as_slice()).unwrap_or(&[]);
                                let _ = vcs::save_collection(&repo, col, reqs);
                            }
                            // Return the updated log
                            let log = if let Some(col) = collections.first() {
                                vcs::collection_log(&repo, &col.id)
                            } else {
                                vec![]
                            };
                            Ok(log)
                        }
                        Err(e) => Err(e),
                    }
                },
                |result| match result {
                    Ok(log) => Message::Git(GitMsg::LogLoaded(log)),
                    Err(e) => Message::Git(GitMsg::Error(e)),
                },
            );
        }
        GitMsg::Committed(msg) => state.status_message = Some(format!("Committed: {msg}")),
        GitMsg::Error(e) => state.status_message = Some(format!("Git error: {e}")),
    }
    Task::none()
}

fn handle_palette(state: &mut AppState, msg: PaletteMsg) -> Task<Message> {
    match msg {
        PaletteMsg::Open => {
            state.palette_open = true;
            state.palette_query = String::new();
            state.palette_selected = 0;
            return iced::widget::text_input::focus(
                iced::widget::text_input::Id::new("palette-search"),
            );
        }
        PaletteMsg::Close => {
            state.palette_open = false;
        }
        PaletteMsg::QueryChanged(s) => {
            state.palette_query = s;
            state.palette_selected = 0;
        }
        PaletteMsg::MoveDown => {
            state.palette_selected = state.palette_selected.saturating_add(1).min(11);
        }
        PaletteMsg::MoveUp => {
            state.palette_selected = state.palette_selected.saturating_sub(1);
        }
        PaletteMsg::Confirm => {
            state.palette_open = false;
        }
    }
    Task::none()
}

fn handle_ws(state: &mut AppState, msg: WsMsg) -> Task<Message> {
    match msg {
        WsMsg::Handshake { tab_id, sender } => {
            if let Some(tab) = state.tabs.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.ws.connecting = false;
                tab.ws.connected = true;
                tab.ws.outgoing_tx = Some(sender);
            }
        }
        WsMsg::TextFrame(text) => {
            let tab = state.tabs.active_tab_mut();
            tab.ws.messages.push(crate::state::tabs::WsMessage { text, is_outgoing: false });
        }
        WsMsg::Disconnected => {
            let tab = state.tabs.active_tab_mut();
            tab.ws.connected = false;
            tab.ws.connecting = false;
            tab.ws.outgoing_tx = None;
        }
        WsMsg::Error(e) => {
            let tab = state.tabs.active_tab_mut();
            tab.ws.connected = false;
            tab.ws.connecting = false;
            tab.ws.outgoing_tx = None;
            tab.ws.messages.push(crate::state::tabs::WsMessage {
                text: format!("Error: {e}"),
                is_outgoing: false,
            });
        }
        _ => {}
    }
    Task::none()
}

fn handle_sidebar(state: &mut AppState, msg: SidebarMsg) -> Task<Message> {
    match msg {
        SidebarMsg::PanelSelected(panel) => {
            if panel == crate::message::SidebarPanel::Git {
                let data_dir = state.data_dir.clone();
                let col_id = state.collections.first().map(|c| c.id.clone()).unwrap_or_default();
                let task = Task::perform(
                    async move {
                        use crate::services::vcs;
                        vcs::open_repo(&data_dir).map(|repo| vcs::collection_log(&repo, &col_id))
                    },
                    |result| Message::Git(crate::message::GitMsg::LogLoaded(result.unwrap_or_default())),
                );
                state.sidebar.panel = panel;
                return task;
            }
            state.sidebar.panel = panel;
        }
        SidebarMsg::CollectionToggled(id) => {
            if state.sidebar.expanded.contains(&id) {
                state.sidebar.expanded.remove(&id);
            } else {
                state.sidebar.expanded.insert(id);
            }
        }
        SidebarMsg::RequestOpened(req) => {
            state.sidebar.selected_request = Some(req.id.clone());
            state.tabs.open_request(&req);
        }
        SidebarMsg::HistoryEntryOpened(entry) => {
            state.tabs.open_request(&entry.request);
        }
        SidebarMsg::ClearHistory => {
            state.history.clear();
            if let Some(db) = &state.db {
                let _ = storage::clear_history(db);
            }
        }
        SidebarMsg::NewCollection => {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().timestamp_millis();
            let col = Collection { id: id.clone(), name: "New Collection".to_owned(), created_at: now };
            if let Some(db) = &state.db {
                let _ = storage::create_collection(db, &col);
            }
            state.collections.push(col);
        }
        SidebarMsg::DeleteCollection(id) => {
            state.collections.retain(|c| c.id != id);
            state.requests.remove(&id);
            if let Some(db) = &state.db {
                let _ = storage::delete_collection(db, &id);
            }
        }
        SidebarMsg::DeleteRequest { id, collection_id } => {
            if let Some(reqs) = state.requests.get_mut(&collection_id) {
                reqs.retain(|r| r.id != id);
            }
            if let Some(db) = &state.db {
                let _ = storage::delete_request(db, &id);
            }
        }
        SidebarMsg::RenameCollection { id, name } => {
            if let Some(c) = state.collections.iter_mut().find(|c| c.id == id) {
                c.name = name.clone();
                if let Some(db) = &state.db {
                    let _ = storage::update_collection(db, &id, &name);
                }
            }
        }
        SidebarMsg::EnvironmentCreated => {
            let id = uuid::Uuid::new_v4().to_string();
            let env = AppEnvironment {
                id: id.clone(),
                name: format!("Environment {}", state.environments.len() + 1),
                variables: std::collections::HashMap::new(),
                is_active: state.environments.is_empty(),
            };
            if let Some(db) = &state.db {
                let _ = storage::save_environment(db, &env);
            }
            state.environments.push(env);
        }
        SidebarMsg::EnvironmentSelected(id) => {
            for env in &mut state.environments {
                env.is_active = env.id == id;
            }
            if let Some(db) = &state.db {
                for env in &state.environments {
                    let _ = storage::save_environment(db, env);
                }
            }
        }
        SidebarMsg::EnvironmentDeleted(id) => {
            if state.sidebar.env_editing.as_deref() == Some(&id) {
                state.sidebar.env_editing = None;
            }
            state.environments.retain(|e| e.id != id);
            if let Some(db) = &state.db {
                let _ = storage::delete_environment(db, &id);
            }
        }
        SidebarMsg::EnvironmentToggleEdit(id) => {
            state.sidebar.env_editing = if state.sidebar.env_editing.as_deref() == Some(&id) {
                None
            } else {
                Some(id)
            };
        }
        SidebarMsg::EnvironmentNameChanged(id, name) => {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == id) {
                env.name = name;
                if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
            }
        }
        SidebarMsg::EnvironmentVarAdded(env_id) => {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                let key = format!("VAR_{}", env.variables.len() + 1);
                env.variables.insert(key, String::new());
                if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
            }
        }
        SidebarMsg::EnvironmentVarKeyChanged(env_id, idx, new_key) => {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                let entries: Vec<(String, String)> = env.variables.clone().into_iter().collect();
                if let Some((old_key, val)) = entries.get(idx) {
                    if old_key != &new_key {
                        env.variables.remove(old_key);
                        env.variables.insert(new_key, val.clone());
                        if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
                    }
                }
            }
        }
        SidebarMsg::EnvironmentVarValueChanged(env_id, idx, new_val) => {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                let keys: Vec<String> = env.variables.keys().cloned().collect();
                if let Some(k) = keys.get(idx) {
                    env.variables.insert(k.clone(), new_val);
                    if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
                }
            }
        }
        SidebarMsg::EnvironmentVarRemoved(env_id, idx) => {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                let keys: Vec<String> = env.variables.keys().cloned().collect();
                if let Some(k) = keys.get(idx) {
                    env.variables.remove(k);
                    if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
                }
            }
        }
    }
    Task::none()
}

fn handle_request(state: &mut AppState, msg: RequestMsg) -> Task<Message> {
    let tab = state.tabs.active_tab_mut();
    match msg {
        RequestMsg::UrlChanged(v) => {
            // When a URL with a query string is pasted (contains '?'), parse the
            // query params into the Params table and strip them from the URL field.
            if let Some(q_idx) = v.find('?') {
                let base = v[..q_idx].to_owned();
                let qs = &v[q_idx + 1..];
                if !qs.is_empty() {
                    let mut new_params: Vec<crate::domain::request::KeyValue> = qs
                        .split('&')
                        .filter(|pair| !pair.is_empty())
                        .map(|pair| {
                            let (k, val) = pair.split_once('=').unwrap_or((pair, ""));
                            crate::domain::request::KeyValue {
                                id: uuid::Uuid::new_v4().to_string(),
                                key: percent_decode(k),
                                value: percent_decode(val),
                                enabled: true,
                            }
                        })
                        .collect();
                    let existing_keys: std::collections::HashSet<String> =
                        new_params.iter().map(|p| p.key.clone()).collect();
                    for p in &tab.params {
                        if !existing_keys.contains(&p.key) && !p.key.is_empty() {
                            new_params.push(p.clone());
                        }
                    }
                    tab.params = new_params;
                    tab.url = base;
                    tab.modified = true;
                    return Task::none();
                }
            }
            tab.url = v;
            tab.modified = true;
        }
        RequestMsg::MethodChanged(v) => {
            if let Ok(m) = v.parse() { tab.method = m; tab.modified = true; }
        }
        RequestMsg::TabSelected(t) => tab.active_request_tab = t,
        RequestMsg::BodyEdited(action) => { tab.body_editor.perform(action); tab.modified = true; }
        RequestMsg::PreRequestScriptEdited(action) => { tab.pre_request_editor.perform(action); tab.modified = true; }
        RequestMsg::TestScriptEdited(action) => { tab.test_editor.perform(action); tab.modified = true; }
        RequestMsg::NewTab => { state.tabs.new_tab(); persist_session(state); return Task::none(); }
        RequestMsg::CloseTab(i) => { state.tabs.close_tab(i); persist_session(state); return Task::none(); }
        RequestMsg::CloseCurrentTab => { let i = state.tabs.active; state.tabs.close_tab(i); persist_session(state); return Task::none(); }
        RequestMsg::SwitchTab(i) => { state.tabs.switch_to(i); persist_session(state); return Task::none(); }
        RequestMsg::Send => return send_request(state),
        RequestMsg::HeaderAdded => { tab.headers.push(crate::domain::request::KeyValue::new_empty()); tab.modified = true; }
        RequestMsg::HeaderRemoved(i) => { tab.headers.remove(i); tab.modified = true; }
        RequestMsg::HeaderToggled(i) => { tab.headers[i].enabled = !tab.headers[i].enabled; tab.modified = true; }
        RequestMsg::HeaderKeyChanged(i, v) => { tab.headers[i].key = v; tab.modified = true; }
        RequestMsg::HeaderValueChanged(i, v) => { tab.headers[i].value = v; tab.modified = true; }
        RequestMsg::ParamAdded => { tab.params.push(crate::domain::request::KeyValue::new_empty()); tab.modified = true; }
        RequestMsg::ParamRemoved(i) => { tab.params.remove(i); tab.modified = true; }
        RequestMsg::ParamToggled(i) => { tab.params[i].enabled = !tab.params[i].enabled; tab.modified = true; }
        RequestMsg::ParamKeyChanged(i, v) => { tab.params[i].key = v; tab.modified = true; }
        RequestMsg::ParamValueChanged(i, v) => { tab.params[i].value = v; tab.modified = true; }
        RequestMsg::BearerTokenChanged(v) => tab.bearer_token = v,
        RequestMsg::BasicUserChanged(v) => tab.basic_user = v,
        RequestMsg::BasicPassChanged(v) => tab.basic_pass = v,
        RequestMsg::ApiKeyNameChanged(v) => tab.api_key_name = v,
        RequestMsg::ApiKeyValueChanged(v) => tab.api_key_value = v,
        RequestMsg::AuthTypeChanged(v) => {
            if let Ok(a) = v.parse() { tab.auth_type = a; }
        }
        RequestMsg::CookieStringChanged(v) => tab.cookie_string = v,
        RequestMsg::JwtSecretChanged(v) => { tab.jwt_secret = v; tab.modified = true; }
        RequestMsg::JwtSubjectChanged(v) => { tab.jwt_subject = v; tab.modified = true; }
        RequestMsg::JwtAlgoChanged(v) => { tab.jwt_algo = v; tab.modified = true; }
        RequestMsg::FormFieldTypeToggled(i) => {
            use crate::domain::request::FormFieldType;
            if i < tab.form_fields.len() {
                tab.form_fields[i].field_type = if tab.form_fields[i].field_type == FormFieldType::Text {
                    FormFieldType::File
                } else {
                    FormFieldType::Text
                };
                tab.modified = true;
            }
        }
        RequestMsg::FormFieldPickFile(i) => {
            return Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Select file")
                        .pick_file()
                        .await;
                    if let Some(f) = file {
                        let name = f.file_name();
                        let bytes = f.read().await;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        Some((i, name, b64))
                    } else {
                        None
                    }
                },
                |res| match res {
                    Some((idx, name, data)) => Message::Request(RequestMsg::FormFieldFilePicked(idx, name, data)),
                    None => Message::App(AppMsg::Noop),
                },
            );
        }
        RequestMsg::FormFieldFilePicked(i, fname, data) => {
            if i < tab.form_fields.len() {
                tab.form_fields[i].file_name = Some(fname);
                tab.form_fields[i].file_data = Some(data);
                tab.modified = true;
            }
        }
        RequestMsg::WsUrlChanged(v) => tab.ws.url = v,
        RequestMsg::WsMessageChanged(v) => tab.ws.draft = v,
        RequestMsg::WsConnect => {
            tab.ws.connecting = true;
            tab.ws.connected = false;
            tab.ws.messages.clear();
        }
        RequestMsg::WsDisconnect => {
            tab.ws.outgoing_tx = None; // dropping the sender signals the outgoing task to stop
            tab.ws.connected = false;
            tab.ws.connecting = false;
        }
        RequestMsg::WsSend => {
            if let Some(tx) = tab.ws.outgoing_tx.clone() {
                let msg = std::mem::take(&mut tab.ws.draft);
                tab.ws.messages.push(crate::state::tabs::WsMessage { text: msg.clone(), is_outgoing: true });
                return Task::perform(
                    async move { let _ = tx.send(msg).await; },
                    |_| Message::App(AppMsg::Noop),
                );
            }
        }
        RequestMsg::ImportCurl(cmd) => {
            let parsed = curl::parse(&cmd);
            let has_body = parsed.body.is_some();
            let has_headers = !parsed.header.is_empty();

            if let Some(url) = parsed.url {
                if let Some(q_idx) = url.find('?') {
                    tab.url = url[..q_idx].to_owned();
                    let qs = &url[q_idx + 1..];
                    if !qs.is_empty() {
                        tab.params = qs
                            .split('&')
                            .filter(|pair| !pair.is_empty())
                            .map(|pair| {
                                let (k, val) = pair.split_once('=').unwrap_or((pair, ""));
                                crate::domain::request::KeyValue {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    key: percent_decode(k),
                                    value: percent_decode(val),
                                    enabled: true,
                                }
                            })
                            .collect();
                    }
                } else {
                    tab.url = url;
                }
            }
            if let Some(method) = parsed.method {
                if let Ok(m) = method.to_uppercase().parse() {
                    tab.method = m;
                }
            }
            if has_headers {
                tab.headers = parsed.header.into_iter()
                    .map(|(k, v)| crate::domain::request::KeyValue {
                        id: uuid::Uuid::new_v4().to_string(),
                        key: k,
                        value: v,
                        enabled: true,
                    })
                    .collect();
            }
            if has_body {
                if let Some(body) = parsed.body {
                    let trimmed = body.trim();
                    tab.body_type = if trimmed.starts_with('{') || trimmed.starts_with('[') {
                        crate::domain::request::BodyType::Json
                    } else {
                        crate::domain::request::BodyType::Text
                    };
                    tab.body_editor = iced::widget::text_editor::Content::with_text(&body);
                }
                tab.active_request_tab = crate::message::RequestTab::Body;
            } else if !tab.params.is_empty() {
                tab.active_request_tab = crate::message::RequestTab::Params;
            } else if has_headers {
                tab.active_request_tab = crate::message::RequestTab::Headers;
            }
            if !parsed.cookies.is_empty() {
                tab.cookie_string = parsed.cookies.into_iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("; ");
            }
            tab.modified = true;
        }
        RequestMsg::SaveRequest => {
            let tab = state.tabs.active_tab();
            if tab.saved_as.is_none() {
                let name = if tab.url.is_empty() { tab.title.clone() } else { tab.url.clone() };
                state.save_dialog_name = name;
                state.save_dialog_collection_id = state.collections.first().map(|c| c.id.clone());
                state.save_dialog_open = true;
                return Task::none();
            }
            return save_request(state);
        }
        RequestMsg::FormatBody => {
            let use_tabs = tab.body_indent_tabs;
            let text = tab.body_editor.text();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                let formatted = if use_tabs {
                    let buf = Vec::new();
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
                    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
                    v.serialize(&mut ser).ok();
                    String::from_utf8(ser.into_inner()).unwrap_or_default()
                } else {
                    serde_json::to_string_pretty(&v).unwrap_or(text)
                };
                tab.body_editor = iced::widget::text_editor::Content::with_text(&formatted);
            }
        }
        RequestMsg::ToggleBodyIndentStyle => {
            tab.body_indent_tabs = !tab.body_indent_tabs;
        }
        RequestMsg::ExportCurl => {
            use crate::services::curl::{generate, GenerateCurlInput, KvPair};
            let body_text = tab.body_editor.text();
            let body = if body_text.trim().is_empty() { None } else { Some(body_text) };
            let headers: Vec<KvPair> = tab.headers.iter()
                .filter(|h| h.enabled && !h.key.is_empty())
                .map(|h| KvPair { key: h.key.clone(), value: h.value.clone() })
                .collect();
            let api_loc = match tab.api_key_location {
                crate::domain::request::ApiKeyLocation::Header => None,
                crate::domain::request::ApiKeyLocation::Query => Some("query".to_owned()),
            };
            let input = GenerateCurlInput {
                method: tab.method.as_str().to_owned(),
                url: tab.url.clone(),
                headers,
                body,
                cookies: vec![],
                auth_type: tab.auth_type.as_str().to_owned(),
                bearer_token: if tab.bearer_token.is_empty() { None } else { Some(tab.bearer_token.clone()) },
                basic_user: if tab.basic_user.is_empty() { None } else { Some(tab.basic_user.clone()) },
                basic_pass: if tab.basic_pass.is_empty() { None } else { Some(tab.basic_pass.clone()) },
                api_key_name: if tab.api_key_name.is_empty() { None } else { Some(tab.api_key_name.clone()) },
                api_key_value: if tab.api_key_value.is_empty() { None } else { Some(tab.api_key_value.clone()) },
                api_key_location: api_loc,
            };
            let curl_cmd = generate(&input);
            state.curl_modal_command = curl_cmd;
            state.curl_modal_open = true;
        }
        RequestMsg::CloseCurlModal => {
            state.curl_modal_open = false;
        }
        RequestMsg::CopyCurlToClipboard => {
            let cmd = state.curl_modal_command.clone();
            state.status_message = Some("cURL command copied!".to_owned());
            return iced::clipboard::write::<Message>(cmd);
        }
        RequestMsg::BodyTypeChanged(v) => {
            if let Ok(b) = v.parse() {
                tab.body_type = b;
                tab.modified = true;
            }
        }
        RequestMsg::FormFieldAdded => {
            use crate::domain::request::{FormField, FormFieldType};
            tab.form_fields.push(FormField {
                id: uuid::Uuid::new_v4().to_string(),
                key: String::new(),
                value: String::new(),
                field_type: FormFieldType::Text,
                enabled: true,
                file_name: None,
                file_data: None,
                mime_type: None,
            });
            tab.modified = true;
        }
        RequestMsg::FormFieldRemoved(i) => {
            if i < tab.form_fields.len() {
                tab.form_fields.remove(i);
                tab.modified = true;
            }
        }
        RequestMsg::FormFieldKeyChanged(i, v) => {
            if let Some(f) = tab.form_fields.get_mut(i) {
                f.key = v;
                tab.modified = true;
            }
        }
        RequestMsg::FormFieldValueChanged(i, v) => {
            if let Some(f) = tab.form_fields.get_mut(i) {
                f.value = v;
                tab.modified = true;
            }
        }
        RequestMsg::ApiKeyLocationChanged(v) => {
            if let Ok(loc) = v.parse() {
                tab.api_key_location = loc;
            }
        }
        RequestMsg::Abort => {
            tab.is_loading = false;
        }
        RequestMsg::CommentToggle => {
            use crate::message::RequestTab;
            match tab.active_request_tab {
                RequestTab::Scripts => {
                    let toggled = toggle_js_comments(&tab.pre_request_editor.text());
                    tab.pre_request_editor = iced::widget::text_editor::Content::with_text(&toggled);
                }
                RequestTab::Body => {
                    let toggled = toggle_js_comments(&tab.body_editor.text());
                    tab.body_editor = iced::widget::text_editor::Content::with_text(&toggled);
                }
                _ => {}
            }
        }
    }
    Task::none()
}

/// Toggle `// ` comments on every non-empty line.
/// If ALL non-empty lines already start with `//`, the comments are removed;
/// otherwise `// ` is prepended to every non-empty line.
fn toggle_js_comments(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let all_commented = lines.iter()
        .filter(|l| !l.trim().is_empty())
        .all(|l| l.trim_start().starts_with("//"));
    lines.iter().map(|l| {
        if l.trim().is_empty() {
            l.to_string()
        } else if all_commented {
            // Remove the leading `//` (with optional space)
            let trimmed = l.trim_start();
            let without = trimmed.strip_prefix("// ").or_else(|| trimmed.strip_prefix("//")).unwrap_or(trimmed);
            let leading = &l[..l.len() - trimmed.len()];
            format!("{leading}{without}")
        } else {
            format!("// {l}")
        }
    }).collect::<Vec<_>>().join("\n")
}

fn send_request(state: &mut AppState) -> Task<Message> {
    let tab = state.tabs.active_tab_mut();
    tab.is_loading = true;
    tab.response = None;
    tab.console.clear();
    tab.test_results.clear();

    // Local addresses (loopback/RFC-1918) get http://; everything else https://.
    let raw_url = tab.url.trim().to_owned();
    let url_with_scheme = if !raw_url.is_empty()
        && !raw_url.starts_with("http://")
        && !raw_url.starts_with("https://")
        && !raw_url.starts_with("ws://")
        && !raw_url.starts_with("wss://")
    {
        let scheme = if is_local_url(&raw_url) { "http" } else { "https" };
        format!("{scheme}://{raw_url}")
    } else {
        raw_url
    };

    let req = crate::domain::collection::SavedRequest {
        id: tab.id.clone(),
        collection_id: String::new(),
        name: tab.title.clone(),
        method: tab.method.clone(),
        url: url_with_scheme,
        headers: tab.headers.clone(),
        params: tab.params.clone(),
        body: tab.body_editor.text(),
        body_type: tab.body_type.clone(),
        auth_type: tab.auth_type.clone(),
        bearer_token: tab.bearer_token.clone(),
        basic_user: tab.basic_user.clone(),
        basic_pass: tab.basic_pass.clone(),
        api_key_name: tab.api_key_name.clone(),
        api_key_value: tab.api_key_value.clone(),
        api_key_location: tab.api_key_location.clone(),
        form_data_fields: tab.form_fields.clone(),
        cookie_string: tab.cookie_string.clone(),
        cookies: tab.cookies.clone(),
        jwt_secret: tab.jwt_secret.clone(),
        jwt_subject: tab.jwt_subject.clone(),
        jwt_algo: tab.jwt_algo.clone(),
        pre_request_script: tab.pre_request_editor.text(),
        test_script: tab.test_editor.text(),
    };

    let tab_id = tab.id.clone();
    let client = state.http_client.clone();
    let active_env = state.active_env().cloned();

    Task::perform(
        async move {
            http::send(&client, tab_id, &req, active_env.as_ref()).await
        },
        |result| Message::App(AppMsg::HttpResponse(result)),
    )
}

fn save_request(state: &mut AppState) -> Task<Message> {
    let tab = state.tabs.active_tab();
    let req = SavedRequest {
        id: tab.saved_as.as_ref().map(|(_, id)| id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        collection_id: tab.saved_as.as_ref().map(|(col, _)| col.clone())
            .or_else(|| state.collections.first().map(|c| c.id.clone()))
            .unwrap_or_default(),
        name: tab.title.clone(),
        method: tab.method.clone(),
        url: tab.url.clone(),
        headers: tab.headers.clone(),
        params: tab.params.clone(),
        body: tab.body_editor.text(),
        body_type: tab.body_type.clone(),
        auth_type: tab.auth_type.clone(),
        bearer_token: tab.bearer_token.clone(),
        basic_user: tab.basic_user.clone(),
        basic_pass: tab.basic_pass.clone(),
        api_key_name: tab.api_key_name.clone(),
        api_key_value: tab.api_key_value.clone(),
        api_key_location: tab.api_key_location.clone(),
        form_data_fields: tab.form_fields.clone(),
        cookie_string: tab.cookie_string.clone(),
        cookies: tab.cookies.clone(),
        jwt_secret: tab.jwt_secret.clone(),
        jwt_subject: tab.jwt_subject.clone(),
        jwt_algo: tab.jwt_algo.clone(),
        pre_request_script: tab.pre_request_editor.text(),
        test_script: tab.test_editor.text(),
    };

    if req.collection_id.is_empty() {
        let existing_col_id = state
            .collections
            .iter()
            .find(|c| c.name == "My Requests")
            .map(|c| c.id.clone());

        let col_id = existing_col_id.unwrap_or_else(|| {
            let id = uuid::Uuid::new_v4().to_string();
            let col = Collection {
                id: id.clone(),
                name: "My Requests".to_owned(),
                created_at: chrono::Utc::now().timestamp_millis(),
            };
            if let Some(db) = &state.db {
                let _ = storage::create_collection(db, &col);
            }
            state.collections.push(col);
            id
        });
        let tab = state.tabs.active_tab_mut();
        tab.saved_as = Some((col_id.clone(), req.id.clone()));
        let mut req = req;
        req.collection_id = col_id.clone();
        if let Some(db) = &state.db {
            let _ = storage::create_request(db, &req);
        }
        state.requests.entry(col_id).or_default().push(req);
    } else {
        let col_id = req.collection_id.clone();
        let req_id = req.id.clone();
        let is_new = tab.saved_as.is_none();
        if let Some(db) = &state.db {
            if is_new {
                let _ = storage::create_request(db, &req);
            } else {
                let _ = storage::update_request(db, &req);
            }
        }
        let tab = state.tabs.active_tab_mut();
        tab.saved_as = Some((col_id.clone(), req_id.clone()));
        let bucket = state.requests.entry(col_id).or_default();
        if let Some(existing) = bucket.iter_mut().find(|r| r.id == req_id) {
            *existing = req;
        } else {
            bucket.push(req);
        }
    }

    state.tabs.active_tab_mut().modified = false;
    state.status_message = Some("Saved!".to_owned());
    Task::none()
}

fn handle_response(state: &mut AppState, msg: ResponseMsg) -> Task<Message> {
    match msg {
        ResponseMsg::TabSelected(t) => {
            state.tabs.active_tab_mut().active_response_tab = t;
        }
        ResponseMsg::SearchChanged(q) => {
            state.tabs.active_tab_mut().search_query = q;
        }
        ResponseMsg::FormatBody => {
            let tab = state.tabs.active_tab_mut();
            let use_tabs = tab.body_indent_tabs;
            let pretty_opt: Option<String> = tab.response.as_ref().and_then(|resp| {
                serde_json::from_str::<serde_json::Value>(&resp.body).ok().map(|v| {
                    if use_tabs {
                        let buf = Vec::new();
                        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
                        let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
                        let _ = v.serialize(&mut ser);
                        String::from_utf8(ser.into_inner()).unwrap_or_default()
                    } else {
                        serde_json::to_string_pretty(&v).unwrap_or_default()
                    }
                })
            });
            if let (Some(pretty), Some(resp)) = (pretty_opt, tab.response.as_mut()) {
                resp.body = pretty;
            }
        }
        ResponseMsg::CopyBody => {
            let body = state.tabs.active_tab().response.as_ref().map(|r| r.body.clone()).unwrap_or_default();
            state.status_message = Some("Copied!".to_owned());
            return clipboard::write::<Message>(body);
        }
        ResponseMsg::CopyValue(v) => {
            state.status_message = Some("Value copied!".to_owned());
            return clipboard::write::<Message>(v);
        }
        ResponseMsg::ViewerAction(action) => {
            let tab = state.tabs.active_tab_mut();
            if action.is_edit() {
                // Read-only viewer — swallow edit actions
            } else {
                tab.response_viewer.perform(action);
                if let Some(selected) = tab.response_viewer.selection() {
                    if !selected.is_empty() {
                        return clipboard::write::<Message>(selected);
                    }
                }
            }
        }
        ResponseMsg::ToggleSearch => {
            let tab = state.tabs.active_tab_mut();
            tab.search_visible = !tab.search_visible;
            if tab.search_visible {
                return iced::widget::text_input::focus(
                    iced::widget::text_input::Id::new("response-search"),
                );
            } else {
                tab.search_query.clear();
            }
        }
        ResponseMsg::BodyViewToggled => {}
        ResponseMsg::ToggleJsonNode(path) => {
            let tab = state.tabs.active_tab_mut();
            if !tab.json_collapsed.remove(&path) {
                tab.json_collapsed.insert(path);
            }
        }
        ResponseMsg::ToggleJsonRaw => {
            let tab = state.tabs.active_tab_mut();
            tab.json_raw_mode = !tab.json_raw_mode;
        }
    }
    Task::none()
}

fn handle_storage(state: &mut AppState, msg: StorageMsg) -> Task<Message> {
    match msg {
        StorageMsg::Error(e) => state.status_message = Some(e),
        _ => {}
    }
    Task::none()
}

fn handle_app(state: &mut AppState, msg: AppMsg) -> Task<Message> {
    match msg {
        AppMsg::HttpResponse(result) => {
            if let Some(tab) = state.tabs.tabs.iter_mut().find(|t| t.id == result.tab_id) {
                tab.is_loading = false;
                tab.json_collapsed.clear();
                tab.json_raw_mode = false;
                tab.parsed_json = None;
                tab.viewer_processing = true;
                tab.response = Some(result.response.clone());

                let entry = HistoryEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    method: tab.method.as_str().to_owned(),
                    url: tab.url.clone(),
                    status: result.response.status as i32,
                    duration_ms: result.response.duration_ms as i64,
                    request: crate::domain::collection::SavedRequest {
                        id: tab.id.clone(),
                        collection_id: String::new(),
                        name: tab.title.clone(),
                        method: tab.method.clone(),
                        url: tab.url.clone(),
                        headers: tab.headers.clone(),
                        params: tab.params.clone(),
                        body: tab.body_editor.text(),
                        body_type: tab.body_type.clone(),
                        auth_type: tab.auth_type.clone(),
                        bearer_token: tab.bearer_token.clone(),
                        basic_user: tab.basic_user.clone(),
                        basic_pass: tab.basic_pass.clone(),
                        api_key_name: tab.api_key_name.clone(),
                        api_key_value: tab.api_key_value.clone(),
                        api_key_location: tab.api_key_location.clone(),
                        form_data_fields: tab.form_fields.clone(),
                        cookie_string: tab.cookie_string.clone(),
                        cookies: tab.cookies.clone(),
                        jwt_secret: tab.jwt_secret.clone(),
                        jwt_subject: tab.jwt_subject.clone(),
                        jwt_algo: tab.jwt_algo.clone(),
                        pre_request_script: tab.pre_request_editor.text(),
                        test_script: tab.test_editor.text(),
                    },
                };

                let body = result.response.body.clone();
                let tab_id = tab.id.clone();

                if let Some(db) = &state.db {
                    let _ = storage::add_history(db, &entry);
                }
                state.history.insert(0, entry);
                if state.history.len() > 100 {
                    state.history.truncate(100);
                }
                persist_session(state);

                // Always process body in a background thread — no size limits.
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let parsed = serde_json::from_str::<serde_json::Value>(&body)
                                .ok()
                                .map(Box::new);
                            AppMsg::ViewerReady { tab_id, content_text: body, parsed_json: parsed }
                        })
                        .await
                        .unwrap_or(AppMsg::ViewerReady {
                            tab_id: String::new(),
                            content_text: String::new(),
                            parsed_json: None,
                        })
                    },
                    Message::App,
                );
            }
        }
        AppMsg::AvatarLoaded(bytes) => {
            state.profile_avatar = Some(bytes);
        }
        AppMsg::ScriptConsoleLog(_) => {}
        AppMsg::ViewerReady { tab_id, content_text, parsed_json } => {
            if !tab_id.is_empty() {
                if let Some(tab) = state.tabs.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.response_viewer_lines = content_text.lines().count().max(1);
                        tab.response_viewer = iced::widget::text_editor::Content::with_text(&content_text);
                    tab.parsed_json = parsed_json.map(|b| *b);
                    tab.viewer_processing = false;
                }
            }
        }
        AppMsg::OpenUrl(url) => {
            let _ = open::that(url);
        }
        AppMsg::Noop => {}
    }
    Task::none()
}

// ── Import / Export ───────────────────────────────────────────────────────────

fn handle_import(state: &mut AppState, msg: ImportMsg) -> Task<Message> {
    match msg {
        ImportMsg::OpenPostmanDialog => Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_title("Import Postman Collection")
                    .pick_file()
                    .await;
                if let Some(f) = file {
                    let bytes = f.read().await;
                    Some(String::from_utf8_lossy(&bytes).to_string())
                } else {
                    None
                }
            },
            |content| match content {
                Some(json) => match crate::services::import::postman::import(&json) {
                    Ok(data) => Message::Import(ImportMsg::PostmanLoaded(data)),
                    Err(e) => Message::Import(ImportMsg::Error(e)),
                },
                None => Message::App(AppMsg::Noop),
            },
        ),

        ImportMsg::OpenOpenApiDialog => Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("JSON/YAML", &["json", "yaml", "yml"])
                    .set_title("Import OpenAPI Specification")
                    .pick_file()
                    .await;
                if let Some(f) = file {
                    let bytes = f.read().await;
                    Some(String::from_utf8_lossy(&bytes).to_string())
                } else {
                    None
                }
            },
            |content| match content {
                Some(json) => match crate::services::import::openapi::import(&json) {
                    Ok(data) => Message::Import(ImportMsg::OpenApiLoaded(data)),
                    Err(e) => Message::Import(ImportMsg::Error(e)),
                },
                None => Message::App(AppMsg::Noop),
            },
        ),

        ImportMsg::ExportCollection(col_id) => {
            let Some(col) = state.collections.iter().find(|c| c.id == col_id).cloned() else {
                return Task::none();
            };
            let reqs = state.requests.get(&col_id).cloned().unwrap_or_default();
            let json = crate::services::import::postman::export(&col, &reqs);
            let file_name = format!("{}.postman_collection.json", col.name);
            Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .set_file_name(&file_name)
                        .add_filter("JSON", &["json"])
                        .set_title("Export Postman Collection")
                        .save_file()
                        .await;
                    if let Some(f) = file {
                        let path = f.path().to_path_buf();
                        let _ = tokio::fs::write(&path, json.as_bytes()).await;
                        Some(path.to_string_lossy().to_string())
                    } else {
                        None
                    }
                },
                |path| match path {
                    Some(p) => Message::Import(ImportMsg::ExportDone(p)),
                    None => Message::App(AppMsg::Noop),
                },
            )
        }

        ImportMsg::PostmanLoaded(data) | ImportMsg::OpenApiLoaded(data) => {
            let count: usize = data.iter().map(|(_, reqs)| reqs.len()).sum();
            for (col, reqs) in data {
                if let Some(db) = &state.db {
                    let _ = storage::create_collection(db, &col);
                    for req in &reqs {
                        let _ = storage::create_request(db, req);
                    }
                }
                state.requests.insert(col.id.clone(), reqs);
                state.collections.push(col);
            }
            state.status_message = Some(format!("Imported {count} requests"));
            Task::none()
        }

        ImportMsg::ExportDone(path) => {
            state.status_message = Some(format!("Exported → {path}"));
            Task::none()
        }

        ImportMsg::Error(e) => {
            state.status_message = Some(format!("Import error: {e}"));
            Task::none()
        }
    }
}

// ── Settings ──────────────────────────────────────────────────────────────────

fn handle_settings(state: &mut AppState, msg: SettingsMsg) -> Task<Message> {
    match msg {
        SettingsMsg::GithubChanged(v) => state.github_username = v,
        SettingsMsg::EmailChanged(v) => state.github_email = v,
        SettingsMsg::WebsiteChanged(v) => state.github_website = v,
        SettingsMsg::AccentChanged(idx) => {
            state.accent_idx = idx;
            crate::ui::theme::Palette::set_accent_idx(idx);
        }
        SettingsMsg::ThemeDark => {
            state.theme_is_dark = true;
            crate::ui::theme::Palette::set_dark(true);
        }
        SettingsMsg::ThemeLight => {
            state.theme_is_dark = false;
            crate::ui::theme::Palette::set_dark(false);
        }
    }
    Task::none()
}

// ── Session persistence ────────────────────────────────────────────────────────

fn persist_session(state: &AppState) {
    let Some(db) = &state.db else { return };
    let snapshots: Vec<_> = state.tabs.tabs.iter().map(|t| t.into()).collect();
    let session = AppSession {
        tabs: snapshots,
        active_tab: state.tabs.active,
        active_env_id: state.active_env().map(|e| e.id.clone()),
        sidebar_panel: format!("{:?}", state.sidebar.panel),
    };
    let _ = storage::save_session(db, &session);
}

// ── View ──────────────────────────────────────────────────────────────────────

fn view(state: &AppState) -> Element<'_, Message> {
    crate::ui::layout::view(state)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Decode `%xx` percent-encoded sequences in a URL component.
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((hi * 16 + lo) as u8 as char);
                i += 3;
            } else {
                out.push('%');
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn is_local_url(url: &str) -> bool {
    let host = url.split('/').next().unwrap_or(url);
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("172.")  // covers 172.16-31.x.x private range
}
