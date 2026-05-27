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
    let enabled = tab.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();

    let toolbar = row![
        text(if enabled > 0 {
            format!("{enabled} active")
        } else {
            "No active params".to_owned()
        })
        .size(11),
        iced::widget::Space::with_width(iced::Length::Fill),
        button(text("+ Add").size(11))
            .on_press(Message::Request(RequestMsg::ParamAdded))
            .style(iced::widget::button::text)
            .padding([2, 6]),
    ]
    .align_y(iced::Alignment::Center)
    .padding([4, 8])
    .spacing(4);

    column![
        toolbar,
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
    .height(iced::Length::Fill)
    .into()
}
