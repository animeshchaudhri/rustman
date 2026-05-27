use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use iced::Color;

// ── Runtime palette state ─────────────────────────────────────────────────────

static DARK_MODE: AtomicBool = AtomicBool::new(true);
static ACCENT_IDX: AtomicUsize = AtomicUsize::new(0);

pub const ACCENT_PALETTE: &[Color] = &[
    Color { r: 0.388, g: 0.400, b: 0.945, a: 1.0 }, // indigo (default)
    Color { r: 0.133, g: 0.773, b: 0.478, a: 1.0 }, // emerald
    Color { r: 0.247, g: 0.631, b: 0.961, a: 1.0 }, // blue
    Color { r: 0.678, g: 0.400, b: 0.945, a: 1.0 }, // violet
    Color { r: 0.251, g: 0.878, b: 0.816, a: 1.0 }, // teal
    Color { r: 0.961, g: 0.267, b: 0.341, a: 1.0 }, // rose
    Color { r: 0.988, g: 0.580, b: 0.110, a: 1.0 }, // amber
    Color { r: 0.608, g: 0.349, b: 0.714, a: 1.0 }, // purple
];

// ── Scrollbar ─────────────────────────────────────────────────────────────────

pub fn thin_scrollbar(
    _theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let hovered = matches!(status, iced::widget::scrollable::Status::Hovered { .. });
    let dragging = matches!(status, iced::widget::scrollable::Status::Dragged { .. });
    let alpha = if dragging { 0.90 } else if hovered { 0.70 } else { 0.40 };
    let scroller_color = if Palette::is_dark() {
        Color { r: 0.45, g: 0.45, b: 0.55, a: alpha }
    } else {
        Color { r: 0.40, g: 0.38, b: 0.34, a: alpha }
    };
    let rail_bg_alpha = if hovered || dragging { 0.08 } else { 0.0 };
    let rail_bg = Color { r: 0.5, g: 0.5, b: 0.5, a: rail_bg_alpha };
    let make_rail = |color: Color| iced::widget::scrollable::Rail {
        background: Some(iced::Background::Color(rail_bg)),
        border: iced::Border::default(),
        scroller: iced::widget::scrollable::Scroller {
            color,
            border: iced::Border { radius: 3.0.into(), ..Default::default() },
        },
    };
    iced::widget::scrollable::Style {
        container: Default::default(),
        vertical_rail: make_rail(scroller_color),
        horizontal_rail: make_rail(scroller_color),
        gap: None,
    }
}

// ── Palette ───────────────────────────────────────────────────────────────────

pub struct Palette;

impl Palette {
    // ── Control ───────────────────────────────────────────────────────────────

    pub fn set_dark(dark: bool) {
        DARK_MODE.store(dark, Ordering::Relaxed);
    }
    pub fn set_accent_idx(idx: usize) {
        ACCENT_IDX.store(idx, Ordering::Relaxed);
    }
    pub fn is_dark() -> bool {
        DARK_MODE.load(Ordering::Relaxed)
    }

    // ── Backgrounds (theme-aware) ─────────────────────────────────────────────

    pub fn background() -> Color {
        if Self::is_dark() { Color::from_rgb(0.055, 0.055, 0.063) }
        else { Color::from_rgb(0.984, 0.945, 0.780) }
    }
    pub fn surface() -> Color {
        if Self::is_dark() { Color::from_rgb(0.094, 0.094, 0.106) }
        else { Color::from_rgb(0.957, 0.914, 0.745) }
    }
    pub fn surface_high() -> Color {
        if Self::is_dark() { Color::from_rgb(0.153, 0.153, 0.165) }
        else { Color::from_rgb(0.922, 0.859, 0.698) }
    }
    pub fn surface_raised() -> Color {
        if Self::is_dark() { Color::from_rgb(0.196, 0.196, 0.212) }
        else { Color::from_rgb(0.898, 0.835, 0.671) }
    }

    // ── Borders ───────────────────────────────────────────────────────────────

    pub fn border() -> Color {
        if Self::is_dark() { Color::from_rgb(0.247, 0.247, 0.275) }
        else { Color::from_rgb(0.741, 0.682, 0.576) }
    }
    pub fn border_subtle() -> Color {
        if Self::is_dark() { Color::from_rgb(0.157, 0.157, 0.173) }
        else { Color::from_rgb(0.855, 0.800, 0.686) }
    }

    // ── Text ─────────────────────────────────────────────────────────────────

    pub fn text() -> Color {
        if Self::is_dark() { Color::from_rgb(0.980, 0.980, 0.980) }
        else { Color::from_rgb(0.235, 0.220, 0.212) }
    }
    pub fn text_muted() -> Color {
        if Self::is_dark() { Color::from_rgb(0.631, 0.631, 0.667) }
        else { Color::from_rgb(0.400, 0.361, 0.329) }
    }
    pub fn text_subtle() -> Color {
        if Self::is_dark() { Color::from_rgb(0.443, 0.443, 0.475) }
        else { Color::from_rgb(0.588, 0.533, 0.451) }
    }

    // ── Accent (dynamic — reads from ACCENT_IDX atomic) ──────────────────────

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

    // ── Const fallbacks kept for any internal use ─────────────────────────────

    pub const ACCENT: Color = Color { r: 0.388, g: 0.400, b: 0.945, a: 1.0 };
    pub const ACCENT_DIM: Color = Color { r: 0.239, g: 0.247, b: 0.596, a: 1.0 };

    // ── Semantic ──────────────────────────────────────────────────────────────

    pub const SUCCESS: Color = Color::from_rgb(0.133, 0.773, 0.478);
    pub const WARNING: Color = Color::from_rgb(0.984, 0.749, 0.184);
    pub const ERROR: Color = Color::from_rgb(0.961, 0.267, 0.341);

    // ── HTTP Method colors ────────────────────────────────────────────────────

    pub const GET: Color = Color::from_rgb(0.133, 0.773, 0.478);
    pub const POST: Color = Color::from_rgb(0.988, 0.580, 0.110);
    pub const PUT: Color = Color::from_rgb(0.388, 0.400, 0.945);
    pub const PATCH: Color = Color::from_rgb(0.678, 0.400, 0.945);
    pub const DELETE: Color = Color::from_rgb(0.961, 0.267, 0.341);
    pub const HEAD: Color = Color::from_rgb(0.631, 0.631, 0.667);
    pub const OPTIONS: Color = Color::from_rgb(0.631, 0.631, 0.667);
}
