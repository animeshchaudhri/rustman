use iced::{clipboard, Task};

use crate::app::AppState;
use crate::message::{Message, ResponseMsg};

pub(super) fn handle(state: &mut AppState, msg: ResponseMsg) -> Task<Message> {
    match msg {
        ResponseMsg::TabSelected(t) => {
            state.tabs.active_tab_mut().active_response_tab = t;
        }
        ResponseMsg::CopyBody => {
            let tab = state.tabs.active_tab();
            let mut body = tab.response_editor.content();
            if body.is_empty() {
                body = tab.response.as_ref().map(|r| r.body.clone()).unwrap_or_default();
            }
            state.status_message = Some("Copied!".to_owned());
            return clipboard::write::<Message>(body);
        }
        ResponseMsg::CopyValue(v) => {
            state.status_message = Some("Value copied!".to_owned());
            return clipboard::write::<Message>(v);
        }
        ResponseMsg::ViewerEdited(msg) => {
            let tab = state.tabs.active_tab_mut();
            return tab.response_editor.update(&msg)
                .map(|m| Message::Response(ResponseMsg::ViewerEdited(m)));
        }
    }
    Task::none()
}
