use iced::Element;

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::widgets::kv_table,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    kv_table::view(
        &tab.params,
        |i| Message::Request(RequestMsg::ParamToggled(i)),
        |i, s| Message::Request(RequestMsg::ParamKeyChanged(i, s)),
        |i, s| Message::Request(RequestMsg::ParamValueChanged(i, s)),
        |i| Message::Request(RequestMsg::ParamRemoved(i)),
        Message::Request(RequestMsg::ParamAdded),
        "Add param",
    )
}
