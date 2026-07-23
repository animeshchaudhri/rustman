use iced::{
    widget::{container, row, text, Space},
    Alignment, Background, Border, Element, Length,
};

use crate::{
    app::AppState,
    message::Message,
    ui::{icons, theme::{Palette, MONO, TEXT_XS}},
};

pub(super) fn status_bar(state: &AppState) -> Element<'_, Message> {
    let env_el: Element<Message> = match state.environments.iter().find(|e| e.is_active) {
        Some(e) => row![
            icons::globe().size(11).color(Palette::accent()),
            text(e.name.as_str()).size(TEXT_XS).color(Palette::text_muted()),
        ]
        .spacing(5)
        .align_y(Alignment::Center)
        .into(),
        None => row![
            icons::globe().size(11).color(Palette::text_subtle()),
            text("No environment").size(TEXT_XS).color(Palette::text_subtle()),
        ]
        .spacing(5)
        .align_y(Alignment::Center)
        .into(),
    };

    let msg = state.status_message.as_deref().unwrap_or("");

    container(
        row![
            if msg.is_empty() {
                text("").size(TEXT_XS)
            } else {
                text(msg).size(TEXT_XS).color(Palette::SUCCESS)
            },
            Space::new().width(Length::Fill),
            env_el,
            separator(),
            row![
                text("Ctrl+P").size(TEXT_XS).color(Palette::text_subtle()).font(MONO),
                text("Command").size(TEXT_XS).color(Palette::text_subtle()),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([4, 12]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::chrome())),
        border: Border { color: Palette::border_subtle(), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    })
    .height(28)
    .width(Length::Fill)
    .into()
}

fn separator() -> Element<'static, Message> {
    container(Space::new().width(1).height(12))
        .style(|_| container::Style {
            background: Some(Background::Color(Palette::border())),
            ..Default::default()
        })
        .into()
}
