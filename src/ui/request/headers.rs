use iced::{
    widget::{button, column, row, text},
    Element,
};

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::widgets::kv_table,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let enabled = tab.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();

    let toolbar = row![
        text(if enabled > 0 {
            format!("{enabled} active")
        } else {
            "No active headers".to_owned()
        })
        .size(11),
        iced::widget::Space::with_width(iced::Length::Fill),
        button(text("+ Add").size(11))
            .on_press(Message::Request(RequestMsg::HeaderAdded))
            .style(iced::widget::button::text)
            .padding([2, 6]),
    ]
    .align_y(iced::Alignment::Center)
    .padding([4, 8])
    .spacing(4);

    column![
        toolbar,
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
    .height(iced::Length::Fill)
    .into()
}
