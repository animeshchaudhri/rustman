use iced::{
    widget::{button, column, container, row, scrollable, text},
    Element, Length,
};

use crate::{
    app::AppState,
    message::{Message, SidebarMsg},
    ui::theme::Palette,
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let clear_btn = button(text("Clear").size(11).color(Palette::text_muted()))
        .on_press(Message::Sidebar(SidebarMsg::ClearHistory))
        .style(iced::widget::button::text)
        .padding([2, 6]);

    let header = container(
        row![
            text("History").size(12).color(Palette::text_muted()),
            iced::widget::Space::with_width(Length::Fill),
            clear_btn,
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 8]),
    )
    .width(Length::Fill);

    let mut col = column![header].spacing(0);

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
                    text(&entry.method).size(10).color(method_color).font(iced::Font::MONOSPACE),
                    text(&entry.url).size(12).color(Palette::text()),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
                row![
                    text(status_str).size(10).color(status_color),
                    text(dur_str).size(10).color(Palette::text_muted()),
                    text(ts).size(10).color(Palette::text_muted()),
                ]
                .spacing(8),
            ]
            .spacing(2),
        )
        .on_press(Message::Sidebar(SidebarMsg::HistoryEntryOpened(entry.clone())))
        .style(iced::widget::button::text)
        .width(Length::Fill)
        .padding([4, 8]);

        col = col.push(item);
    }

    if state.history.is_empty() {
        col = col.push(
            container(text("No history yet.").size(12).color(Palette::text_muted()))
                .padding([12, 8]),
        );
    }

    scrollable(col)
        .height(Length::Fill)
        .style(crate::ui::theme::thin_scrollbar)
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
