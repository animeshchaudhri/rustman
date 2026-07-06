use std::sync::atomic::{AtomicUsize, Ordering};

use iced::{Color, Font};
use iced::font::Weight;

pub const MONO: Font = Font::with_name("JetBrains Mono");


pub const UI_FONT: Font = Font::with_name("Noto Sans");
pub const UI_FONT_MEDIUM: Font = Font { weight: Weight::Medium, ..UI_FONT };

// ── Type scale ──────────────────────────────────────────────────────────────
// Named sizes so panels share one hierarchy instead of ad-hoc literals.
pub const TEXT_XS: f32 = 10.0; // timestamps, hashes, meta
pub const TEXT_SM: f32 = 12.0; // secondary labels, buttons, table cells
pub const TEXT_MD: f32 = 13.0; // primary body text
pub const TEXT_LG: f32 = 15.0; // section/card headers
pub const TEXT_XL: f32 = 19.0; // panel titles, empty-state headlines

static THEME_IDX: AtomicUsize = AtomicUsize::new(0);


pub struct ThemeSpec {
    pub name: &'static str,
    pub dark: bool,
    pub background: Color,
    pub surface: Color,
    pub surface_high: Color,
    pub surface_raised: Color,
    pub row_odd: Color,
    pub border: Color,
    pub border_subtle: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_subtle: Color,
    pub accent: Color,
    pub hover: Color,
    pub chrome: Color,
}

pub const THEMES: &[ThemeSpec] = &[
    ThemeSpec {
        name: "Amber",
        dark: true,
        background: Color::from_rgb(0.071, 0.067, 0.063),
        surface: Color::from_rgb(0.114, 0.106, 0.098),
        surface_high: Color::from_rgb(0.161, 0.149, 0.137),
        surface_raised: Color::from_rgb(0.208, 0.192, 0.176),
        row_odd: Color::from_rgb(0.090, 0.084, 0.078),
        border: Color::from_rgb(0.320, 0.293, 0.261),
        border_subtle: Color::from_rgb(0.190, 0.176, 0.161),
        text: Color::from_rgb(0.973, 0.965, 0.953),
        text_muted: Color::from_rgb(0.650, 0.620, 0.580),
        text_subtle: Color::from_rgb(0.460, 0.435, 0.405),
        accent: Color::from_rgb(0.988, 0.580, 0.110),
        hover: Color::from_rgb(0.145, 0.135, 0.125),
        chrome: Color::from_rgb(0.050, 0.047, 0.043),
    },
    ThemeSpec {
        name: "Violet",
        dark: true,
        background: Color::from_rgb(0.055, 0.059, 0.072),
        surface: Color::from_rgb(0.095, 0.101, 0.123),
        surface_high: Color::from_rgb(0.140, 0.149, 0.178),
        surface_raised: Color::from_rgb(0.185, 0.196, 0.230),
        row_odd: Color::from_rgb(0.076, 0.081, 0.099),
        border: Color::from_rgb(0.280, 0.300, 0.350),
        border_subtle: Color::from_rgb(0.175, 0.186, 0.220),
        text: Color::from_rgb(0.965, 0.973, 0.988),
        text_muted: Color::from_rgb(0.620, 0.645, 0.695),
        text_subtle: Color::from_rgb(0.440, 0.470, 0.530),
        accent: Color::from_rgb(0.557, 0.420, 0.965),
        hover: Color::from_rgb(0.125, 0.133, 0.160),
        chrome: Color::from_rgb(0.040, 0.043, 0.053),
    },
    ThemeSpec {
        name: "Nord",
        dark: true,
        background: Color::from_rgb(0.145, 0.161, 0.192),
        surface: Color::from_rgb(0.180, 0.204, 0.243),
        surface_high: Color::from_rgb(0.216, 0.243, 0.283),
        surface_raised: Color::from_rgb(0.263, 0.298, 0.369),
        row_odd: Color::from_rgb(0.160, 0.180, 0.216),
        border: Color::from_rgb(0.320, 0.356, 0.420),
        border_subtle: Color::from_rgb(0.230, 0.255, 0.300),
        text: Color::from_rgb(0.925, 0.937, 0.957),
        text_muted: Color::from_rgb(0.700, 0.725, 0.765),
        text_subtle: Color::from_rgb(0.500, 0.525, 0.565),
        accent: Color::from_rgb(0.533, 0.753, 0.816),
        hover: Color::from_rgb(0.200, 0.224, 0.263),
        chrome: Color::from_rgb(0.122, 0.137, 0.163),
    },
    ThemeSpec {
        name: "Dracula",
        dark: true,
        background: Color::from_rgb(0.157, 0.165, 0.212),
        surface: Color::from_rgb(0.184, 0.192, 0.239),
        surface_high: Color::from_rgb(0.208, 0.220, 0.267),
        surface_raised: Color::from_rgb(0.267, 0.278, 0.353),
        row_odd: Color::from_rgb(0.145, 0.153, 0.196),
        border: Color::from_rgb(0.330, 0.345, 0.420),
        border_subtle: Color::from_rgb(0.230, 0.242, 0.300),
        text: Color::from_rgb(0.973, 0.973, 0.949),
        text_muted: Color::from_rgb(0.700, 0.700, 0.760),
        text_subtle: Color::from_rgb(0.500, 0.500, 0.560),
        accent: Color::from_rgb(1.000, 0.475, 0.776),
        hover: Color::from_rgb(0.220, 0.230, 0.290),
        chrome: Color::from_rgb(0.125, 0.133, 0.173),
    },
    ThemeSpec {
        name: "Solarized",
        dark: true,
        background: Color::from_rgb(0.000, 0.169, 0.212),
        surface: Color::from_rgb(0.027, 0.212, 0.259),
        surface_high: Color::from_rgb(0.043, 0.243, 0.293),
        surface_raised: Color::from_rgb(0.070, 0.280, 0.330),
        row_odd: Color::from_rgb(0.000, 0.145, 0.184),
        border: Color::from_rgb(0.220, 0.370, 0.400),
        border_subtle: Color::from_rgb(0.100, 0.260, 0.300),
        text: Color::from_rgb(0.933, 0.910, 0.835),
        text_muted: Color::from_rgb(0.514, 0.580, 0.588),
        text_subtle: Color::from_rgb(0.345, 0.431, 0.459),
        accent: Color::from_rgb(0.149, 0.545, 0.824),
        hover: Color::from_rgb(0.055, 0.230, 0.275),
        chrome: Color::from_rgb(0.000, 0.130, 0.165),
    },
    ThemeSpec {
        name: "Light",
        dark: false,
        background: Color::from_rgb(0.965, 0.972, 0.984),
        surface: Color::from_rgb(0.996, 0.997, 1.000),
        surface_high: Color::from_rgb(0.925, 0.941, 0.965),
        surface_raised: Color::from_rgb(0.890, 0.914, 0.949),
        row_odd: Color::from_rgb(0.977, 0.982, 0.991),
        border: Color::from_rgb(0.737, 0.776, 0.835),
        border_subtle: Color::from_rgb(0.847, 0.875, 0.918),
        text: Color::from_rgb(0.105, 0.124, 0.158),
        text_muted: Color::from_rgb(0.355, 0.400, 0.475),
        text_subtle: Color::from_rgb(0.554, 0.604, 0.680),
        accent: Color::from_rgb(0.557, 0.420, 0.965),
        hover: Color::from_rgb(0.939, 0.950, 0.969),
        chrome: Color::from_rgb(0.925, 0.940, 0.963),
    },
];

pub const SHADOW_LIGHT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.14 };

pub fn thin_scrollbar(
    _theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let hovered = matches!(status, iced::widget::scrollable::Status::Hovered { .. });
    let dragging = matches!(status, iced::widget::scrollable::Status::Dragged { .. });
    // Hidden at rest (content scrolls fine with the wheel without a
    // persistent bar sitting there); appears clearly on hover/drag so it's
    // still easy to find and grab when you actually want to drag it.
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

/// Unlike `thin_scrollbar`, this stays faintly visible at rest — used for the
/// tab strip's horizontal scroll, which (unlike content panes) is the *only*
/// way to reach overflowed tabs, so it needs to be findable without hovering.
pub fn tab_scrollbar(
    _theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let hovered = matches!(status, iced::widget::scrollable::Status::Hovered { .. });
    let dragging = matches!(status, iced::widget::scrollable::Status::Dragged { .. });
    let alpha = if dragging { 0.85 } else if hovered { 0.55 } else { 0.22 };
    let s = Palette::text_subtle();
    let scroller = Color { r: s.r, g: s.g, b: s.b, a: alpha };
    let rail = iced::widget::scrollable::Rail {
        background: None,
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

/// Fully invisible in every state — the panel still scrolls with the wheel,
/// there's just never a rail/thumb drawn over the content.
pub fn hidden_scrollbar(
    _theme: &iced::Theme,
    _status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let rail = iced::widget::scrollable::Rail {
        background: None,
        border: iced::Border::default(),
        scroller: iced::widget::scrollable::Scroller {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: iced::Border::default(),
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
    pub fn set_theme_idx(idx: usize) { THEME_IDX.store(idx.min(THEMES.len() - 1), Ordering::Relaxed); }
    pub fn theme_idx() -> usize { THEME_IDX.load(Ordering::Relaxed) }
    fn current() -> &'static ThemeSpec { &THEMES[Self::theme_idx()] }

    pub fn is_dark() -> bool { Self::current().dark }
    pub fn background() -> Color { Self::current().background }
    pub fn surface() -> Color { Self::current().surface }
    pub fn surface_high() -> Color { Self::current().surface_high }
    pub fn surface_raised() -> Color { Self::current().surface_raised }
    pub fn row_even() -> Color { Self::surface() }
    pub fn row_odd() -> Color { Self::current().row_odd }
    pub fn border() -> Color { Self::current().border }
    pub fn border_subtle() -> Color { Self::current().border_subtle }
    pub fn text() -> Color { Self::current().text }
    pub fn text_muted() -> Color { Self::current().text_muted }
    pub fn text_subtle() -> Color { Self::current().text_subtle }
    pub fn accent() -> Color { Self::current().accent }
    pub fn hover() -> Color { Self::current().hover }
    /// Recessed chrome (icon rail, tab strip) — deliberately a step darker
    /// than `background` so it reads as a distinct bar, not the same void.
    pub fn chrome() -> Color { Self::current().chrome }

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
