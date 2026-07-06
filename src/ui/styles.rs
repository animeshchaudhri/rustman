

use iced::{Background, Border, Color};

use crate::ui::theme::Palette;

// ── Text input ────────────────────────────────────────────────────────────────

/// For kv-table cells (params, headers, cookies).
pub fn cell_input(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let accent = Palette::accent();
    iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: match status {
                iced::widget::text_input::Status::Focused { .. } =>
                    Color { r: accent.r, g: accent.g, b: accent.b, a: 0.5 },
                iced::widget::text_input::Status::Hovered =>
                    Palette::border_subtle(),
                _ => Color::TRANSPARENT,
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        icon: Palette::text_muted(),
        placeholder: Palette::text_subtle(),
        value: Palette::text(),
        selection: Color { r: accent.r, g: accent.g, b: accent.b, a: 0.25 },
    }
}

/// For form fields (auth inputs, sidebar inputs, URL bar).
pub fn field_input(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let accent = Palette::accent();
    iced::widget::text_input::Style {
        background: Background::Color(Palette::surface_high()),
        border: Border {
            color: match status {
                iced::widget::text_input::Status::Focused { .. } => accent,
                iced::widget::text_input::Status::Hovered => Palette::border(),
                _ => Palette::border_subtle(),
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        icon: Palette::text_muted(),
        placeholder: Palette::text_subtle(),
        value: Palette::text(),
        selection: Color { r: accent.r, g: accent.g, b: accent.b, a: 0.25 },
    }
}

// ── text_editor (scripts panel uses iced text_editor, not CodeEditor) ─────────

pub fn scripts_editor(
    _theme: &iced::Theme,
    _status: iced::widget::text_editor::Status,
) -> iced::widget::text_editor::Style {
    let accent = Palette::accent();
    iced::widget::text_editor::Style {
        background: Background::Color(Palette::surface()),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        placeholder: Palette::text_subtle(),
        value: Palette::text(),
        selection: Color { r: accent.r, g: accent.g, b: accent.b, a: 0.25 },
    }
}

pub fn divider(_theme: &iced::Theme) -> iced::widget::rule::Style {
    iced::widget::rule::Style {
        color: Palette::border_subtle(),
        radius: 0.0.into(),
        fill_mode: iced::widget::rule::FillMode::Full,
        snap: true,
    }
}

// ── Container helpers ─────────────────────────────────────────────────────────


/// A slightly-raised container for section headers inside panels.
pub fn section_header(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Palette::surface_high())),
        border: Border {
            color: Palette::border_subtle(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
