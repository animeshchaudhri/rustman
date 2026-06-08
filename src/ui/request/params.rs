use iced::{widget::column, Element, Length};

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::widgets::kv_table,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    match &tab.params_bulk {
        Some(content) => kv_table::bulk_panel(
            content,
            |a| Message::Request(RequestMsg::ParamsBulkEdited(a)),
            Message::Request(RequestMsg::ParamsBulkToggle),
        ),
        None => column![
            kv_table::bulk_toggle_bar(Message::Request(RequestMsg::ParamsBulkToggle), "Bulk Edit"),
            kv_table::view(
                &tab.params,
                |i| Message::Request(RequestMsg::ParamToggled(i)),
                |i, s| Message::Request(RequestMsg::ParamKeyChanged(i, s)),
                |i, s| Message::Request(RequestMsg::ParamValueChanged(i, s)),
                |i| Message::Request(RequestMsg::ParamRemoved(i)),
                Message::Request(RequestMsg::ParamAdded),
                "Add param",
            ),
        ]
        .height(Length::Fill)
        .into(),
    }
}
