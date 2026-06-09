use iced::{
    widget::{column, container, row, text, text_input, Space},
    Element, Length,
};

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::theme::Palette,
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let timeout_input = text_input("30000", &tab.timeout_text)
        .on_input(|v| Message::Request(RequestMsg::TimeoutChanged(v)))
        .size(13)
        .padding([7, 10])
        .width(160);

    let timeout_row = row![
        column![
            text("Request timeout (ms)").size(12).color(Palette::text()),
            text("Time to wait before the request is aborted. 0 falls back to 30000.")
                .size(10)
                .color(Palette::text_subtle()),
        ]
        .spacing(3),
        Space::new().width(Length::Fill),
        timeout_input,
    ]
    .align_y(iced::Alignment::Center)
    .spacing(12)
    .padding([14, 16]);

    container(column![timeout_row].spacing(0))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
