use iced::{
    widget::{container, row, text, Space},
    Background, Border, Color, Element, Length,
};

use crate::{domain::response::HttpResponse, message::Message, ui::theme::Palette};

pub fn view(resp: &HttpResponse) -> Element<'static, Message> {
    if resp.status == 0 {
        let err = resp
            .error
            .clone()
            .unwrap_or_else(|| "Unknown error".to_owned());
        return row![
            status_pill("ERR".to_owned(), Palette::ERROR, Color { r: 0.15, g: 0.05, b: 0.05, a: 1.0 }),
            text(err).size(12).color(Palette::text_muted()),
        ]
        .spacing(10)
        .padding([6, 12])
        .align_y(iced::Alignment::Center)
        .into();
    }

    let (pill_fg, pill_bg) = status_colors(resp.status);
    let status_label = format!("{} {}", resp.status, resp.status_text);

    let dur_str = format!("{}ms", resp.duration_ms);
    let size_str = if resp.body_size < 1024 {
        format!("{} B", resp.body_size)
    } else {
        format!("{:.1} KB", resp.body_size as f64 / 1024.0)
    };

    row![
        status_pill(status_label, pill_fg, pill_bg),
        meta_chip(dur_str),
        meta_chip(size_str),
        Space::with_width(Length::Fill),
    ]
    .spacing(8)
    .padding([6, 12])
    .align_y(iced::Alignment::Center)
    .into()
}

fn status_colors(code: u16) -> (Color, Color) {
    match code {
        200..=299 => (Palette::SUCCESS, Color { r: 0.05, g: 0.25, b: 0.14, a: 1.0 }),
        300..=399 => (Palette::WARNING, Color { r: 0.25, g: 0.20, b: 0.04, a: 1.0 }),
        400..=499 => (
            Color { r: 0.988, g: 0.580, b: 0.110, a: 1.0 },
            Color { r: 0.25, g: 0.14, b: 0.02, a: 1.0 },
        ),
        500..=599 => (Palette::ERROR, Color { r: 0.25, g: 0.05, b: 0.07, a: 1.0 }),
        _ => (Palette::text_muted(), Palette::surface_high()),
    }
}

fn status_pill(label: String, fg: Color, bg: Color) -> Element<'static, Message> {
    container(
        text(label)
            .size(11)
            .color(fg)
            .font(iced::Font::MONOSPACE),
    )
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border { color: fg, width: 1.0, radius: 4.0.into() },
        ..Default::default()
    })
    .padding([3, 8])
    .into()
}

fn meta_chip(label: String) -> Element<'static, Message> {
    container(text(label).size(11).color(Palette::text_muted()))
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(Palette::surface_high())),
            border: Border {
                color: Palette::border_subtle(),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .padding([3, 8])
        .into()
}
