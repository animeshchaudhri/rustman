use iced::Element;

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::widgets::kv_table,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    kv_table::view(
        &tab.headers,
        |i| Message::Request(RequestMsg::HeaderToggled(i)),
        |i, s| Message::Request(RequestMsg::HeaderKeyChanged(i, s)),
        |i, s| Message::Request(RequestMsg::HeaderValueChanged(i, s)),
        |i| Message::Request(RequestMsg::HeaderRemoved(i)),
        Message::Request(RequestMsg::HeaderAdded),
        "Add header",
    )
}
