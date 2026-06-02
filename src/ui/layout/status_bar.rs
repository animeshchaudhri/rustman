use iced::{
    widget::{column, container, row, scrollable, text, Space},
    Background, Color, Element, Length,
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
        .padding([0, 8]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Color {
            r: 0.040, g: 0.040, b: 0.048, a: 1.0,
        })),
        ..Default::default()
    })
    .height(22)
    .width(Length::Fill)
    .into()
}

pub(super) fn response_tests(tab: &crate::state::tabs::RequestTabState) -> Element<'_, Message> {
    let mut col = column![].spacing(4).padding(8);
    if tab.test_results.is_empty() {
        col = col.push(text("No test results.").size(13).color(Palette::text_muted()));
    }
    for r in &tab.test_results {
        let icon = if r.passed { icons::check() } else { icons::close() };
        let color = if r.passed { Palette::SUCCESS } else { Palette::ERROR };
        col = col.push(
            row![icon.size(12).color(color), text(&r.name).size(12)].spacing(6),
        );
    }
    scrollable(col).height(Length::Fill).into()
}
