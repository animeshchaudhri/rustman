use iced::{Background, Border, Color};

use crate::ui::theme::Palette;

pub fn method_color(method: &crate::domain::request::HttpMethod) -> Color {
    use crate::domain::request::HttpMethod as M;
    match method {
        M::Get => Palette::GET,
        M::Post => Palette::POST,
        M::Put => Palette::PUT,
        M::Patch => Palette::PATCH,
        M::Delete => Palette::DELETE,
        M::Head | M::Options => Palette::HEAD,
    }
}

pub fn icon_rail_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::background())),
        ..Default::default()
    }
}

pub fn icon_btn_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: if active {
            Some(Background::Color(Palette::surface_high()))
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            Some(Background::Color(Palette::surface()))
        } else {
            None
        },
        border: Border::default(),
        text_color: Palette::text(),
        ..Default::default()
    }
}

pub fn sidebar_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        border: Border { color: Palette::border(), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

pub fn surface_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::background())),
        ..Default::default()
    }
}

pub fn tab_bar_container(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface())),
        ..Default::default()
    }
}

pub fn tab_container_style(_theme: &iced::Theme, active: bool) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: if active { Some(Background::Color(Palette::surface_high())) } else { None },
        border: Border {
            color: if active { Palette::border() } else { Color::TRANSPARENT },
            width: if active { 1.0 } else { 0.0 },
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

pub fn tab_btn_style(
    _theme: &iced::Theme,
    _status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        text_color: if active { Palette::text() } else { Palette::text_muted() },
        ..Default::default()
    }
}
