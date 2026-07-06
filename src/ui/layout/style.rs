use iced::{Background, Border, Color, Shadow};

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
        background: Some(Background::Color(Palette::chrome())),
        border: Border { color: Palette::border_subtle(), width: 1.0, radius: 0.0.into() },
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
            Some(Background::Color(Palette::accent_soft()))
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            Some(Background::Color(Palette::hover()))
        } else {
            None
        },
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
        background: Some(Background::Color(Palette::surface())),
        border: Border {
            color: Palette::border(),
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

pub fn tab_bar_container(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::chrome())),
        border: Border {
            color: Palette::border_subtle(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn tab_container_style(_theme: &iced::Theme, active: bool, dragging: bool) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: if dragging {
            Some(Background::Color(Palette::accent_soft()))
        } else if active {
            Some(Background::Color(Palette::surface()))
        } else {
            None
        },
        border: Border {
            color: if dragging || active { Palette::accent() } else { Color::TRANSPARENT },
            width: if dragging || active { 1.0 } else { 0.0 },
            radius: 8.0.into(),
        },
        shadow: if active || dragging {
            let accent = Palette::accent();
            Shadow { color: Color { r: accent.r, g: accent.g, b: accent.b, a: 0.3 }, offset: iced::Vector::new(0.0, 0.0), blur_radius: 10.0 }
        } else {
            Shadow::default()
        },
        ..Default::default()
    }
}
