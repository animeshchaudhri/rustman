use iced::Task;

use crate::app::AppState;
use crate::message::{Message, SettingsMsg};

pub(super) fn handle(state: &mut AppState, msg: SettingsMsg) -> Task<Message> {
    match msg {
        SettingsMsg::GithubChanged(v) => state.github_username = v,
        SettingsMsg::EmailChanged(v) => state.github_email = v,
        SettingsMsg::WebsiteChanged(v) => state.github_website = v,
        SettingsMsg::AccentChanged(idx) => {
            state.accent_idx = idx;
            crate::ui::theme::Palette::set_accent_idx(idx);
            for tab in &mut state.tabs.tabs { tab.sync_editor_themes(); }
        }
    }
    Task::none()
}
