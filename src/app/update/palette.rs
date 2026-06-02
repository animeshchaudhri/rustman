use iced::Task;

use crate::app::AppState;
use crate::message::{Message, PaletteMsg};

pub(super) fn handle(state: &mut AppState, msg: PaletteMsg) -> Task<Message> {
    match msg {
        PaletteMsg::Open => {
            state.palette_open = true;
            state.palette_query = String::new();
            state.palette_selected = 0;
            return iced::widget::operation::focus("palette-search");
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
