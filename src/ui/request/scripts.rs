use iced::{
    widget::{column, container, row, text, text_editor, Space},
    Element, Length,
};

use crate::{
    message::{Message, RequestMsg},
    state::tabs::RequestTabState,
    ui::{styles, theme::{Palette, MONO}},
};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    column![
        script_section_header("Pre-request Script", "Runs before each request · access req object"),
        text_editor(&tab.pre_request_editor)
            .on_action(|a| Message::Request(RequestMsg::PreRequestScriptEdited(a)))
            .height(Length::FillPortion(1))
            .font(MONO)
            .style(styles::scripts_editor),
        script_section_header("Test Script", "Runs after response · use pm.test() to assert"),
        text_editor(&tab.test_editor)
            .on_action(|a| Message::Request(RequestMsg::TestScriptEdited(a)))
            .height(Length::FillPortion(1))
            .font(MONO)
            .style(styles::scripts_editor),
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
