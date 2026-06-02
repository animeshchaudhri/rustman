use iced::Task;

use crate::app::AppState;
use crate::domain::{collection::Collection, environment::AppEnvironment};
use crate::message::{Message, SidebarMsg};
use crate::services::storage;

pub(super) fn handle(state: &mut AppState, msg: SidebarMsg) -> Task<Message> {
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
