use iced::{
    widget::{button, column, container, row, scrollable, text},
    Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, SidebarMsg},
    ui::{theme::{Palette, TEXT_LG, TEXT_SM}, widgets::kv_table},
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let clear_btn = button(text("Clear").size(TEXT_SM).color(Palette::text_muted()))
        .on_press(Message::Sidebar(SidebarMsg::ClearHistory))
        .style(iced::widget::button::text)
        .padding([2, 6]);

    let header = container(
        row![
            text("History").size(TEXT_LG).color(Palette::text()).font(crate::ui::theme::UI_FONT_MEDIUM),
            iced::widget::Space::new().width(Length::Fill),
            clear_btn,
        ]
        .align_y(iced::Alignment::Center)
        .padding([10, 10]),
    )
    .width(Length::Fill);

    if state.history.is_empty() {
        return column![header, kv_table::empty_state("No history yet.")]
            .spacing(0)
            .height(Length::Fill)
            .into();
    }

    let mut col = column![header].spacing(2);

    for entry in &state.history {
        let method_color = method_color(&entry.method);
        let status_color = status_color(entry.status as u16);
        let ts = chrono::DateTime::from_timestamp_millis(entry.timestamp)
            .map(|dt| dt.format("%H:%M:%S").to_string())
            .unwrap_or_default();
        let status_str = format!("{}", entry.status);
        let dur_str = format!("{}ms", entry.duration_ms);

        let item = button(
            column![
                row![
                    text(&entry.method).size(10).color(method_color).font(crate::ui::theme::MONO),
                    text(&entry.url).size(12).color(Palette::text()),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
                row![
                    status_dot(status_color),
                    text(status_str).size(10).color(status_color),
                    text(dur_str).size(10).color(Palette::text_muted()),
                    text(ts).size(10).color(Palette::text_muted()),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(3),
        )
        .on_press(Message::Sidebar(SidebarMsg::HistoryEntryOpened(entry.clone())))
        .style(|_t, status| {
            let hovered = matches!(status, iced::widget::button::Status::Hovered);
            iced::widget::button::Style {
                background: if hovered { Some(iced::Background::Color(Palette::hover())) } else { None },
                text_color: Palette::text(),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .width(Length::Fill)
        .padding([6, 8]);

        col = col.push(container(item).padding(iced::Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 6.0 }));
    }

    scrollable(col)
        .height(Length::Fill)
        .direction(iced::widget::scrollable::Direction::Vertical(
            crate::ui::theme::grabbable_scrollbar(),
        ))
        .style(crate::ui::theme::thin_scrollbar)
        .into()
}

fn status_dot(color: iced::Color) -> Element<'static, Message> {
    container(iced::widget::Space::new().width(6).height(6))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn method_color(method: &str) -> iced::Color {
    match method {
        "GET" => Palette::GET,
        "POST" => Palette::POST,
        "PUT" => Palette::PUT,
        "PATCH" => Palette::PATCH,
        "DELETE" => Palette::DELETE,
        _ => Palette::HEAD,
    }
}

fn status_color(status: u16) -> iced::Color {
    match status {
        200..=299 => Palette::SUCCESS,
        300..=399 => Palette::accent(),
        400..=499 => Palette::WARNING,
        500..=599 => Palette::ERROR,
        _ => Palette::text_muted(),
    }
}
