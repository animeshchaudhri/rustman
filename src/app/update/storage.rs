use iced::Task;

use crate::app::AppState;
use crate::message::{Message, StorageMsg};

pub(super) fn handle(state: &mut AppState, msg: StorageMsg) -> Task<Message> {
    if let StorageMsg::Error(e) = msg {
        state.status_message = Some(e);
    }
    Task::none()
}
