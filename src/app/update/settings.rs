use iced::Task;

use crate::app::AppState;
use crate::message::{Message, SettingsMsg};
use crate::services::vcs;

pub(super) fn handle(state: &mut AppState, msg: SettingsMsg) -> Task<Message> {
    match msg {
        SettingsMsg::ThemeChanged(idx) => {
            state.theme_idx = idx;
            crate::ui::theme::Palette::set_theme_idx(idx);
            for tab in &mut state.tabs.tabs { tab.sync_editor_themes(); }
        }
        SettingsMsg::GitNameChanged(name) => {
            state.git_user_name = name.clone();
            let repo_path = active_repo_path(state);
            if let Some(path) = repo_path {
                let _ = vcs::set_identity(&path, &name, &state.git_user_email);
            }
        }
        SettingsMsg::GitEmailChanged(email) => {
            state.git_user_email = email.clone();
            let repo_path = active_repo_path(state);
            if let Some(path) = repo_path {
                let _ = vcs::set_identity(&path, &state.git_user_name, &email);
            }
        }
    }
    Task::none()
}

fn active_repo_path(state: &AppState) -> Option<std::path::PathBuf> {
    state
        .git_repos
        .iter()
        .find(|r| r.id == state.git_active_repo)
        .map(|r| r.path.clone())
        .or_else(|| Some(state.data_dir.join("collections")))
}
