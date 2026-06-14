use iced::{
    widget::{container, row, text, Space},
    Background, Border, Color, Element, Length,
};

use crate::{domain::response::HttpResponse, message::Message, ui::theme::Palette};

pub fn view(resp: &HttpResponse) -> Element<'static, Message> {
    if resp.status == 0 {
        
        return row![
            status_pill("ERR".to_owned(), Palette::ERROR, Palette::error_soft()),
            Space::new().width(Length::Fill),
        ]
        .spacing(8)
        .padding([6, 12])
        .align_y(iced::Alignment::Center)
        .into();
    }

    let (pill_fg, pill_bg) = status_colors(resp.status);
    let status_label = format!("{} {}", resp.status, resp.status_text);

    let dur_str = format_duration(resp.duration_ms);
    let size_str = if resp.body_size < 1024 {
        format!("{} B", resp.body_size)
    } else {
        format!("{:.1} KB", resp.body_size as f64 / 1024.0)
    };

    row![
        status_pill(status_label, pill_fg, pill_bg),
        meta_chip(dur_str),
        meta_chip(size_str),
        Space::new().width(Length::Fill),
    ]
    .spacing(8)
    .padding([6, 12])
    .align_y(iced::Alignment::Center)
    .into()
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.2} s", ms as f64 / 1000.0)
    }
}

fn status_colors(code: u16) -> (Color, Color) {
    match code {
        200..=299 => (Palette::SUCCESS, Palette::success_soft()),
        300..=399 => (Palette::WARNING, Palette::warning_soft()),
        400..=499 => (Palette::POST, Palette::warning_soft()),
        500..=599 => (Palette::ERROR, Palette::error_soft()),
        _ => (Palette::text_muted(), Palette::surface_high()),
    }
}

fn status_pill(label: String, fg: Color, bg: Color) -> Element<'static, Message> {
    container(
        text(label)
            .size(11)
            .color(fg)
            .font(crate::ui::theme::MONO),
    )
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: fg, width: 1.0, radius: 100.0.into() },
        ..Default::default()
    })
    .padding([3, 10])
    .into()
}

fn meta_chip(label: String) -> Element<'static, Message> {
    container(text(label).size(11).color(Palette::text_muted()))
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(Palette::surface_high())),
            border: Border {
                color: Palette::border_subtle(),
                width: 1.0,
                radius: 100.0.into(),
            },
            ..Default::default()
        })
        .padding([3, 10])
        .into()
}
