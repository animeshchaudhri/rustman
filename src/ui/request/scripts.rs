use iced::{
    widget::{column, container, row, text, text_editor, Space},
    Background, Border, Color, Element, Length,
};

use crate::{message::{Message, RequestMsg}, state::tabs::RequestTabState, ui::theme::Palette};

pub fn view(tab: &RequestTabState) -> Element<'_, Message> {
    let pre_header = container(
        row![
            text("Pre-request Script").size(11).color(Palette::text_muted()),
            Space::with_width(Length::Fill),
            text("Runs before each request · access req object")
                .size(10)
                .color(Palette::text_subtle()),
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 10]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.08, g: 0.08, b: 0.10, a: 1.0 })),
        border: Border { color: Palette::border_subtle(), width: 0.0, radius: 0.0.into() },
        ..Default::default()
    })
    .width(Length::Fill);

    let pre_editor = text_editor(&tab.pre_request_editor)
        .on_action(|a| Message::Request(RequestMsg::PreRequestScriptEdited(a)))
        .height(Length::FillPortion(1))
        .font(iced::Font::MONOSPACE)
        .style(editor_style);

    let test_header = container(
        row![
            text("Test Script").size(11).color(Palette::text_muted()),
            Space::with_width(Length::Fill),
            text("Runs after response · use pm.test() to assert")
                .size(10)
                .color(Palette::text_subtle()),
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 10]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color { r: 0.08, g: 0.08, b: 0.10, a: 1.0 })),
        border: Border { color: Palette::border_subtle(), width: 0.0, radius: 0.0.into() },
        ..Default::default()
    })
    .width(Length::Fill);

    let test_editor = text_editor(&tab.test_editor)
        .on_action(|a| Message::Request(RequestMsg::TestScriptEdited(a)))
        .height(Length::FillPortion(1))
        .font(iced::Font::MONOSPACE)
        .style(editor_style);

    column![
        pre_header,
        pre_editor,
        test_header,
        test_editor,
    ]
    .spacing(0)
    .height(Length::Fill)
    .into()
}

fn editor_style(
    _theme: &iced::Theme,
    _status: iced::widget::text_editor::Status,
) -> iced::widget::text_editor::Style {
    iced::widget::text_editor::Style {
        background: Background::Color(Color { r: 0.07, g: 0.07, b: 0.09, a: 1.0 }),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        icon: Palette::text_muted(),
        placeholder: Palette::text_subtle(),
        value: Palette::text(),
        selection: Color { r: Palette::accent().r, g: Palette::accent().g, b: Palette::accent().b, a: 0.25 },
    }
}
