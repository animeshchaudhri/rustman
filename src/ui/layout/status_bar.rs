use iced::{
    widget::{container, row, text, Space},
    Background, Border, Element, Length,
};

use crate::{
    app::AppState,
    message::Message,
    ui::{icons, theme::Palette},
};

pub(super) fn status_bar(state: &AppState) -> Element<'_, Message> {
    let env_el: Element<Message> = match state.environments.iter().find(|e| e.is_active) {
        Some(e) => row![
            icons::environments().size(10).color(Palette::text_subtle()),
            text(e.name.as_str()).size(10).color(Palette::text_subtle()),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into(),
        None => text("No environment").size(10).color(Palette::text_subtle()).into(),
    };

    let msg = state.status_message.as_deref().unwrap_or("");

    container(
        row![
            text(msg).size(10).color(Palette::SUCCESS).width(Length::Fill),
            env_el,
            Space::new().width(14),
            row![
                text("Ctrl+P").size(10).color(Palette::text_subtle()),
                icons::command().size(10).color(Palette::text_subtle()),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([2, 10]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::chrome())),
        border: Border { color: Palette::border_subtle(), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    })
    .height(26)
    .width(Length::Fill)
    .into()
}
