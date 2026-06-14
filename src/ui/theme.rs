use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use iced::{Color, Font};

pub const MONO: Font = Font::with_name("JetBrains Mono");

static DARK_MODE: AtomicBool = AtomicBool::new(true);
static ACCENT_IDX: AtomicUsize = AtomicUsize::new(0);

pub const ACCENT_PALETTE: &[Color] = &[
    Color { r: 0.388, g: 0.400, b: 0.945, a: 1.0 },
    Color { r: 0.133, g: 0.773, b: 0.478, a: 1.0 },
    Color { r: 0.247, g: 0.631, b: 0.961, a: 1.0 },
    Color { r: 0.678, g: 0.400, b: 0.945, a: 1.0 },
    Color { r: 0.251, g: 0.878, b: 0.816, a: 1.0 },
    Color { r: 0.961, g: 0.267, b: 0.341, a: 1.0 },
    Color { r: 0.988, g: 0.580, b: 0.110, a: 1.0 },
    Color { r: 0.608, g: 0.349, b: 0.714, a: 1.0 },
];

pub fn thin_scrollbar(
    _theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let hovered = matches!(status, iced::widget::scrollable::Status::Hovered { .. });
    let dragging = matches!(status, iced::widget::scrollable::Status::Dragged { .. });
    let alpha = if dragging { 0.90 } else if hovered { 0.60 } else { 0.0 };
    let s = Palette::text_subtle();
    let scroller = Color { r: s.r, g: s.g, b: s.b, a: alpha };
    let rail_bg = Color { r: 0.5, g: 0.5, b: 0.5, a: if hovered || dragging { 0.06 } else { 0.0 } };
    let rail = iced::widget::scrollable::Rail {
        background: Some(iced::Background::Color(rail_bg)),
        border: iced::Border::default(),
        scroller: iced::widget::scrollable::Scroller {
            background: iced::Background::Color(scroller),
            border: iced::Border { radius: 3.0.into(), ..Default::default() },
        },
    };
    iced::widget::scrollable::Style {
        container: Default::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

pub struct Palette;

impl Palette {
    pub fn set_accent_idx(idx: usize) { ACCENT_IDX.store(idx, Ordering::Relaxed); }
    pub fn is_dark() -> bool { DARK_MODE.load(Ordering::Relaxed) }

    pub fn background() -> Color {
        if Self::is_dark() { Color::from_rgb(0.050, 0.052, 0.060) }
        else { Color::from_rgb(0.965, 0.972, 0.984) }
    }
    pub fn surface() -> Color {
        if Self::is_dark() { Color::from_rgb(0.086, 0.090, 0.104) }
        else { Color::from_rgb(0.996, 0.997, 1.000) }
    }
    pub fn surface_high() -> Color {
        if Self::is_dark() { Color::from_rgb(0.132, 0.139, 0.160) }
        else { Color::from_rgb(0.925, 0.941, 0.965) }
    }
    pub fn surface_raised() -> Color {
        if Self::is_dark() { Color::from_rgb(0.166, 0.176, 0.204) }
        else { Color::from_rgb(0.890, 0.914, 0.949) }
    }
    pub fn row_even() -> Color { Self::surface() }
    pub fn row_odd() -> Color {
        if Self::is_dark() { Color::from_rgb(0.073, 0.077, 0.091) }
        else { Color::from_rgb(0.977, 0.982, 0.991) }
    }
    pub fn border() -> Color {
        if Self::is_dark() { Color::from_rgb(0.234, 0.246, 0.283) }
        else { Color::from_rgb(0.737, 0.776, 0.835) }
    }
    pub fn border_subtle() -> Color {
        if Self::is_dark() { Color::from_rgb(0.153, 0.162, 0.190) }
        else { Color::from_rgb(0.847, 0.875, 0.918) }
    }
    pub fn text() -> Color {
        if Self::is_dark() { Color::from_rgb(0.965, 0.973, 0.988) }
        else { Color::from_rgb(0.105, 0.124, 0.158) }
    }
    pub fn text_muted() -> Color {
        if Self::is_dark() { Color::from_rgb(0.632, 0.659, 0.710) }
        else { Color::from_rgb(0.355, 0.400, 0.475) }
    }
    pub fn text_subtle() -> Color {
        if Self::is_dark() { Color::from_rgb(0.456, 0.483, 0.540) }
        else { Color::from_rgb(0.554, 0.604, 0.680) }
    }
    pub fn accent() -> Color {
        ACCENT_PALETTE
            .get(ACCENT_IDX.load(Ordering::Relaxed))
            .copied()
            .unwrap_or(Color::from_rgb(0.388, 0.400, 0.945))
    }
    pub fn accent_dim() -> Color {
        let a = Self::accent();
        Color { r: a.r * 0.62, g: a.g * 0.62, b: a.b * 0.63, a: 1.0 }
    }
    pub fn accent_soft() -> Color {
        let a = Self::accent();
        Self::soft(a)
    }
    pub fn success_soft() -> Color { Self::soft(Self::SUCCESS) }
    pub fn warning_soft() -> Color { Self::soft(Self::WARNING) }
    pub fn error_soft() -> Color { Self::soft(Self::ERROR) }

    fn soft(color: Color) -> Color {
        if Self::is_dark() {
            Color { r: color.r * 0.22, g: color.g * 0.22, b: color.b * 0.24, a: 1.0 }
        } else {
            Color {
                r: 1.0 - ((1.0 - color.r) * 0.18),
                g: 1.0 - ((1.0 - color.g) * 0.18),
                b: 1.0 - ((1.0 - color.b) * 0.18),
                a: 1.0,
            }
        }
    }
    pub fn hover() -> Color {
        if Self::is_dark() { Color::from_rgb(0.112, 0.119, 0.140) }
        else { Color::from_rgb(0.939, 0.950, 0.969) }
    }
    pub fn chrome() -> Color {
        if Self::is_dark() { Color::from_rgb(0.036, 0.038, 0.046) }
        else { Color::from_rgb(0.925, 0.940, 0.963) }
    }

    pub const SUCCESS: Color = Color::from_rgb(0.133, 0.773, 0.478);
    pub const WARNING: Color = Color::from_rgb(0.984, 0.749, 0.184);
    pub const ERROR:   Color = Color::from_rgb(0.961, 0.267, 0.341);
    pub const GET:     Color = Color::from_rgb(0.133, 0.773, 0.478);
    pub const POST:    Color = Color::from_rgb(0.988, 0.580, 0.110);
    pub const PUT:     Color = Color::from_rgb(0.388, 0.400, 0.945);
    pub const PATCH:   Color = Color::from_rgb(0.678, 0.400, 0.945);
    pub const DELETE:  Color = Color::from_rgb(0.961, 0.267, 0.341);
    pub const HEAD:    Color = Color::from_rgb(0.631, 0.631, 0.667);

    /// Code-editor style from Palette — makes editor bg match all other panels.
    pub fn code_editor_style() -> iced_code_editor::Style {
        let accent = Self::accent();
        iced_code_editor::Style {
            background: Self::surface(),
            text_color: Self::text(),
            gutter_background: Self::background(),
            gutter_border: Self::border_subtle(),
            line_number_color: Self::text_subtle(),
            scrollbar_background: Color::TRANSPARENT,
            scroller_color: Color { r: Self::text_subtle().r, g: Self::text_subtle().g, b: Self::text_subtle().b, a: 0.4 },
            current_line_highlight: Color { r: accent.r, g: accent.g, b: accent.b, a: 0.08 },
        }
    }
}
