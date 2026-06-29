use iced::Task;

use crate::app::AppState;
use crate::domain::{collection::Collection, environment::AppEnvironment};
use crate::message::{Message, SidebarMsg};
use crate::services::storage;

pub(super) fn handle(state: &mut AppState, msg: SidebarMsg) -> Task<Message> {
    match msg {
        SidebarMsg::PanelSelected(panel) => {
            let is_git = panel == crate::message::SidebarPanel::Git;
            state.sidebar.panel = panel;
            if is_git {
                return Task::done(Message::Git(crate::message::GitMsg::Refresh));
            }
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
        SidebarMsg::ToggleRenameCollection(id) => {
            state.sidebar.col_renaming =
                if state.sidebar.col_renaming.as_deref() == Some(&id) { None } else { Some(id) };
        }
        SidebarMsg::ToggleRenameRequest(id) => {
            state.sidebar.req_renaming =
                if state.sidebar.req_renaming.as_deref() == Some(&id) { None } else { Some(id) };
        }
        SidebarMsg::RenameRequest { id, collection_id, name } => {
            if let Some(reqs) = state.requests.get_mut(&collection_id) {
                if let Some(req) = reqs.iter_mut().find(|r| r.id == id) {
                    req.name = name.clone();
                    if let Some(db) = &state.db {
                        let _ = storage::create_request(db, req);
                    }
                }
            }
            for tab in state.tabs.tabs.iter_mut() {
                if tab.saved_as.as_ref().map(|(_, rid)| rid.as_str()) == Some(id.as_str()) {
                    tab.title = name.clone();
                }
            }
        }
        SidebarMsg::NewRequestIn(collection_id) => {
            let req = crate::domain::collection::SavedRequest::new_in(
                collection_id.clone(),
                "New Request".to_owned(),
            );
            if let Some(db) = &state.db {
                let _ = storage::create_request(db, &req);
            }
            state.sidebar.expanded.insert(collection_id.clone());
            state.requests.entry(collection_id).or_default().push(req.clone());
            state.sidebar.selected_request = Some(req.id.clone());
            state.tabs.open_request(&req);
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
            if state.sidebar.env_editing.as_deref() == Some(&id) {
                state.sidebar.env_editing = None;
                state.sidebar.env_edit_rows.clear();
            } else {
                let mut rows: Vec<(String, String)> = state
                    .environments
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.variables.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                rows.sort();
                state.sidebar.env_editing = Some(id);
                state.sidebar.env_edit_rows = rows;
            }
        }
        SidebarMsg::EnvironmentNameChanged(id, name) => {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == id) {
                env.name = name;
                if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
            }
        }
        SidebarMsg::EnvironmentVarAdded(env_id) => {
            if state.sidebar.env_editing.as_deref() == Some(&env_id) {
                state.sidebar.env_edit_rows.push((String::new(), String::new()));
            }
        }
        SidebarMsg::EnvironmentVarKeyChanged(env_id, idx, new_key) => {
            if let Some(row) = state.sidebar.env_edit_rows.get_mut(idx) {
                let old_key = std::mem::replace(&mut row.0, new_key);
                if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                    let trimmed_old = old_key.trim().to_owned();
                    if !trimmed_old.is_empty() {
                        env.variables.remove(&trimmed_old);
                    }
                    let trimmed = row.0.trim().to_owned();
                    if !trimmed.is_empty() {
                        env.variables.insert(trimmed, row.1.clone());
                    }
                    if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
                }
            }
        }
        SidebarMsg::EnvironmentVarValueChanged(env_id, idx, new_val) => {
            if let Some(row) = state.sidebar.env_edit_rows.get_mut(idx) {
                row.1 = new_val;
                if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                    let trimmed = row.0.trim().to_owned();
                    if !trimmed.is_empty() {
                        env.variables.insert(trimmed, row.1.clone());
                    }
                    if let Some(db) = &state.db { let _ = storage::save_environment(db, env); }
                }
            }
        }
        SidebarMsg::EnvironmentVarRemoved(env_id, idx) => {
            if idx < state.sidebar.env_edit_rows.len() {
                state.sidebar.env_edit_rows.remove(idx);
            }
            rebuild_env_vars(state, &env_id);
        }
    }
    Task::none()
}

fn rebuild_env_vars(state: &mut AppState, env_id: &str) {
    let vars: std::collections::HashMap<String, String> = state
        .sidebar
        .env_edit_rows
        .iter()
        .filter(|(k, _)| !k.trim().is_empty())
        .map(|(k, v)| (k.trim().to_owned(), v.clone()))
        .collect();
    if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
        env.variables = vars;
        if let Some(db) = &state.db {
            let _ = storage::save_environment(db, env);
        }
    }
}
