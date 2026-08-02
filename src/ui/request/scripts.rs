use iced::{
    widget::{column, container, row, text, Space},
    Element, Length,
};

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::{styles, theme::Palette},
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let pre_request: Element<Message> = tab
        .pre_request_editor
        .view()
        .map(|m| Message::Request(RequestMsg::PreRequestScriptEdited(m)));
    let test: Element<Message> = tab
        .test_editor
        .view()
        .map(|m| Message::Request(RequestMsg::TestScriptEdited(m)));

    column![
        script_section_header("Pre-request Script", "Runs before each request · env(), set_header()"),
        container(pre_request).height(Length::FillPortion(1)),
        script_section_header("Test Script", "Runs after response · use test(name, condition)"),
        container(test).height(Length::FillPortion(1)),
    ]
    .spacing(0)
    .height(Length::Fill)
    .into()
}

fn script_section_header<'a>(title: &'a str, hint: &'a str) -> Element<'a, Message> {
    container(
        row![
            text(title).size(11).color(Palette::text_muted()),
            Space::new().width(Length::Fill),
            text(hint).size(10).color(Palette::text_subtle()),
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 10]),
    )
    .style(styles::section_header)
    .width(Length::Fill)
    .into()
}
