use iced::{
    widget::{container, row, text, Space},
    Alignment, Background, Border, Color, Element, Length,
};

use crate::{
    domain::response::HttpResponse,
    message::Message,
    ui::{icons, theme::{Palette, TEXT_SM, TEXT_XS}},
};

pub fn view(resp: &HttpResponse) -> Element<'static, Message> {
    if resp.status == 0 {
        return container(
            row![
                status_pill("ERR".to_owned(), Palette::ERROR, Palette::error_soft()),
                text("Request failed").size(TEXT_SM).color(Palette::text_muted()),
                Space::new().width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([4, 12]),
        )
        .width(Length::Fill)
        .into();
    }

    let (pill_fg, pill_bg) = status_colors(resp.status);
    let status_label = format!("{} {}", resp.status, resp.status_text);

    let dur_str = format_duration(resp.duration_ms);
    let size_str = if resp.body_size < 1024 {
        format!("{} B", resp.body_size)
    } else if resp.body_size < 1024 * 1024 {
        format!("{:.1} KB", resp.body_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", resp.body_size as f64 / (1024.0 * 1024.0))
    };

    container(
        row![
            status_pill(status_label, pill_fg, pill_bg),
            meta_chip(icons::timer().size(11).color(Palette::text_subtle()), dur_str),
            meta_chip(icons::zap().size(11).color(Palette::text_subtle()), size_str),
            Space::new().width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([4, 12]),
    )
    .width(Length::Fill)
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

/// Colored dot + label on a soft tinted pill — reads at a glance.
fn status_pill(label: String, fg: Color, bg: Color) -> Element<'static, Message> {
    container(
        row![
            container(Space::new().width(7).height(7))
                .style(move |_| container::Style {
                    background: Some(Background::Color(fg)),
                    border: Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
            text(label)
                .size(TEXT_SM)
                .color(fg)
                .font(crate::ui::theme::UI_FONT_MEDIUM),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(bg)),
        border: Border { radius: 100.0.into(), ..Default::default() },
        ..Default::default()
    })
    .padding([5, 12])
    .into()
}

fn meta_chip(icon: iced::widget::Text<'static>, label: String) -> Element<'static, Message> {
    container(
        row![
            icon,
            text(label).size(TEXT_XS).color(Palette::text_muted()).font(crate::ui::theme::MONO),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface_high())),
        border: Border {
            color: Palette::border_subtle(),
            width: 1.0,
            radius: 100.0.into(),
        },
        ..Default::default()
    })
    .padding([4, 10])
    .into()
}
