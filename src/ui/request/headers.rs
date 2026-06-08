use iced::{widget::column, Element, Length};

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::widgets::kv_table,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    match &tab.headers_bulk {
        Some(content) => kv_table::bulk_panel(
            content,
            |a| Message::Request(RequestMsg::HeadersBulkEdited(a)),
            Message::Request(RequestMsg::HeadersBulkToggle),
        ),
        None => column![
            kv_table::bulk_toggle_bar(Message::Request(RequestMsg::HeadersBulkToggle), "Bulk Edit"),
            kv_table::view(
                &tab.headers,
                |i| Message::Request(RequestMsg::HeaderToggled(i)),
                |i, s| Message::Request(RequestMsg::HeaderKeyChanged(i, s)),
                |i, s| Message::Request(RequestMsg::HeaderValueChanged(i, s)),
                |i| Message::Request(RequestMsg::HeaderRemoved(i)),
                Message::Request(RequestMsg::HeaderAdded),
                "Add header",
            ),
        ]
        .height(Length::Fill)
        .into(),
    }
}
