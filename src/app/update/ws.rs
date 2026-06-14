use iced::Task;

use crate::app::AppState;
use crate::message::{Message, WsMsg};

pub(super) fn handle(state: &mut AppState, msg: WsMsg) -> Task<Message> {
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
    }
    Task::none()
}
