use iced::Task;

use crate::app::AppState;
use crate::message::{Message, SettingsMsg};

pub(super) fn handle(state: &mut AppState, msg: SettingsMsg) -> Task<Message> {
    match msg {
        SettingsMsg::AccentChanged(idx) => {
            state.accent_idx = idx;
            crate::ui::theme::Palette::set_accent_idx(idx);
            for tab in &mut state.tabs.tabs { tab.sync_editor_themes(); }
        }
    }
    Task::none()
}
