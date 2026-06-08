use iced::Color;

/// The appearance of a code editor.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Main editor background color
    pub background: Color,
    /// Text content color
    pub text_color: Color,
    /// Line numbers gutter background color
    pub gutter_background: Color,
    /// Border color for the gutter
    pub gutter_border: Color,
    /// Color for line numbers text
    pub line_number_color: Color,
    /// Scrollbar background color
    pub scrollbar_background: Color,
    /// Scrollbar scroller (thumb) color
    pub scroller_color: Color,
    /// Highlight color for the current line where cursor is located
    pub current_line_highlight: Color,
}

/// The theme catalog of a code editor.
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// A styling function for a code editor.
///
/// This is a shorthand for a function that takes a reference to a
/// [`Theme`](iced::Theme) and returns a [`Style`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for iced::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(from_iced_theme)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// Creates a theme style automatically from any Iced theme.
///
/// This is the default styling function that adapts to all native Iced themes including:
/// - Basic themes: Light, Dark
/// - Popular themes: Dracula, Nord, Solarized, Gruvbox
/// - Catppuccin variants: Latte, Frappé, Macchiato, Mocha
/// - Tokyo Night variants: Tokyo Night, Storm, Light
/// - Kanagawa variants: Wave, Dragon, Lotus
/// - And more: Moonfly, Nightfly, Oxocarbon, Ferra
///
/// The function automatically detects if the theme is dark or light and adjusts
/// colors accordingly for optimal contrast and readability in code editing.
///
/// # Color Mapping
///
/// - `background`: Uses the theme's base background color
/// - `text_color`: Uses the theme's base text color
/// - `gutter_background`: Slightly darker/lighter than background
/// - `gutter_border`: Border between gutter and editor
/// - `line_number_color`: Dimmed text color for subtle line numbers
/// - `scrollbar_background`: Matches editor background
/// - `scroller_color`: Uses secondary color for visibility
/// - `current_line_highlight`: Subtle highlight using primary color
///
/// # Example
///
/// ```
/// use iced_code_editor::theme;
///
/// let tokyo_night = iced::Theme::TokyoNightStorm;
/// let style = theme::from_iced_theme(&tokyo_night);
///
/// // Or use with any theme variant
/// let dracula = iced::Theme::Dracula;
/// let style = theme::from_iced_theme(&dracula);
/// ```
pub fn from_iced_theme(theme: &iced::Theme) -> Style {
    let palette = theme.extended_palette();
    let is_dark = palette.is_dark;

    // Base colors from theme palette
    let background = palette.background.base.color;
    let text_color = palette.background.base.text;

    // Gutter colors: slightly offset from background for subtle distinction
    let gutter_background = palette.background.weak.color;
    let gutter_border = if is_dark {
        darken(palette.background.strong.color, 0.1)
    } else {
        lighten(palette.background.strong.color, 0.1)
    };

    // Line numbers: dimmed text color for subtlety
    // For dark themes: dim the bright text (make it darker)
    // For light themes: blend text towards background (make it lighter/grayer)
    let line_number_color = if is_dark {
        dim_color(text_color, 0.5)
    } else {
        // For light themes, blend text color towards background
        blend_colors(text_color, background, 0.5)
    };

    // Scrollbar colors: blend with background
    let scrollbar_background = background;
    let scroller_color = palette.secondary.weak.color;

    // Current line highlight: very subtle with primary color
    let current_line_highlight = with_alpha(
        palette.primary.weak.color,
        if is_dark { 0.15 } else { 0.25 },
    );

    Style {
        background,
        text_color,
        gutter_background,
        gutter_border,
        line_number_color,
        scrollbar_background,
        scroller_color,
        current_line_highlight,
    }
}

/// Darkens a color by a given factor (0.0 to 1.0).
fn darken(color: Color, factor: f32) -> Color {
    Color {
        r: color.r * (1.0 - factor),
        g: color.g * (1.0 - factor),
        b: color.b * (1.0 - factor),
        a: color.a,
    }
}

/// Lightens a color by a given factor (0.0 to 1.0).
fn lighten(color: Color, factor: f32) -> Color {
    Color {
        r: color.r + (1.0 - color.r) * factor,
        g: color.g + (1.0 - color.g) * factor,
        b: color.b + (1.0 - color.b) * factor,
        a: color.a,
    }
}

/// Dims a color by reducing its intensity.
fn dim_color(color: Color, factor: f32) -> Color {
    Color {
        r: color.r * factor,
        g: color.g * factor,
        b: color.b * factor,
        a: color.a,
    }
}

/// Blends two colors together by a given factor (0.0 = first color, 1.0 = second color).
fn blend_colors(color1: Color, color2: Color, factor: f32) -> Color {
    Color {
        r: color1.r + (color2.r - color1.r) * factor,
        g: color1.g + (color2.g - color1.g) * factor,
        b: color1.b + (color2.b - color1.b) * factor,
        a: color1.a + (color2.a - color1.a) * factor,
    }
}

/// Applies an alpha transparency to a color.
fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { r: color.r, g: color.g, b: color.b, a: alpha }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_iced_theme_dark() {
        let theme = iced::Theme::Dark;
        let style = from_iced_theme(&theme);

        // Dark theme should have dark background
        let brightness =
            (style.background.r + style.background.g + style.background.b)
                / 3.0;
        assert!(brightness < 0.5, "Dark theme should have dark background");

        // Text should be bright for contrast
        let text_brightness =
            (style.text_color.r + style.text_color.g + style.text_color.b)
                / 3.0;
        assert!(text_brightness > 0.5, "Dark theme should have bright text");
    }

    #[test]
    fn test_from_iced_theme_light() {
        let theme = iced::Theme::Light;
        let style = from_iced_theme(&theme);

        // Light theme should have bright background
        let brightness =
            (style.background.r + style.background.g + style.background.b)
                / 3.0;
        assert!(brightness > 0.5, "Light theme should have bright background");

        // Text should be dark for contrast
        let text_brightness =
            (style.text_color.r + style.text_color.g + style.text_color.b)
                / 3.0;
        assert!(text_brightness < 0.5, "Light theme should have dark text");
    }

    #[test]
    fn test_all_iced_themes_produce_valid_styles() {
        // Test all native Iced themes
        for theme in iced::Theme::ALL {
            let style = from_iced_theme(theme);

            // All color components should be valid (0.0 to 1.0)
            assert!(style.background.r >= 0.0 && style.background.r <= 1.0);
            assert!(style.text_color.r >= 0.0 && style.text_color.r <= 1.0);
            assert!(
                style.gutter_background.r >= 0.0
                    && style.gutter_background.r <= 1.0
            );
            assert!(
                style.line_number_color.r >= 0.0
                    && style.line_number_color.r <= 1.0
            );

            // Current line highlight should have transparency
            assert!(
                style.current_line_highlight.a < 1.0,
                "Current line highlight should be semi-transparent for theme: {:?}",
                theme
            );
        }
    }

    #[test]
    fn test_tokyo_night_themes() {
        // Test Tokyo Night variants specifically
        let tokyo_night = iced::Theme::TokyoNight;
        let style = from_iced_theme(&tokyo_night);
        assert!(style.background.r >= 0.0 && style.background.r <= 1.0);

        let tokyo_storm = iced::Theme::TokyoNightStorm;
        let style = from_iced_theme(&tokyo_storm);
        assert!(style.background.r >= 0.0 && style.background.r <= 1.0);

        let tokyo_light = iced::Theme::TokyoNightLight;
        let style = from_iced_theme(&tokyo_light);
        let brightness =
            (style.background.r + style.background.g + style.background.b)
                / 3.0;
        assert!(
            brightness > 0.5,
            "Tokyo Night Light should have bright background"
        );
    }

    #[test]
    fn test_catppuccin_themes() {
        // Test Catppuccin variants
        let themes = [
            iced::Theme::CatppuccinLatte,
            iced::Theme::CatppuccinFrappe,
            iced::Theme::CatppuccinMacchiato,
            iced::Theme::CatppuccinMocha,
        ];

        for theme in themes {
            let style = from_iced_theme(&theme);
            // All should produce valid styles
            assert!(style.background.r >= 0.0 && style.background.r <= 1.0);
            assert!(style.text_color.r >= 0.0 && style.text_color.r <= 1.0);
        }
    }

    #[test]
    fn test_gutter_colors_distinct_from_background() {
        let theme = iced::Theme::Dark;
        let style = from_iced_theme(&theme);

        // Gutter background should be different from editor background
        let gutter_diff = (style.gutter_background.r - style.background.r)
            .abs()
            + (style.gutter_background.g - style.background.g).abs()
            + (style.gutter_background.b - style.background.b).abs();

        assert!(
            gutter_diff > 0.0,
            "Gutter should be visually distinct from background"
        );
    }

    #[test]
    fn test_line_numbers_visible_but_subtle() {
        for theme in [iced::Theme::Dark, iced::Theme::Light] {
            let style = from_iced_theme(&theme);
            let palette = theme.extended_palette();

            // Line numbers should be dimmed compared to text
            let line_num_brightness = (style.line_number_color.r
                + style.line_number_color.g
                + style.line_number_color.b)
                / 3.0;

            let text_brightness =
                (style.text_color.r + style.text_color.g + style.text_color.b)
                    / 3.0;

            let bg_brightness =
                (style.background.r + style.background.g + style.background.b)
                    / 3.0;

            // Line numbers should be between text and background (more subtle than text)
            // For dark themes: text is bright, line numbers dimmer, background dark
            // For light themes: text is dark, line numbers lighter (gray), background bright
            if palette.is_dark {
                // Dark theme: line numbers should be less bright than text
                assert!(
                    line_num_brightness < text_brightness,
                    "Dark theme line numbers should be dimmer than text. Line num: {}, Text: {}",
                    line_num_brightness,
                    text_brightness
                );
            } else {
                // Light theme: line numbers should be between text (dark) and background (bright)
                assert!(
                    line_num_brightness > text_brightness
                        && line_num_brightness < bg_brightness,
                    "Light theme line numbers should be between text and background. Text: {}, Line num: {}, Bg: {}",
                    text_brightness,
                    line_num_brightness,
                    bg_brightness
                );
            }
        }
    }

    #[test]
    fn test_color_helper_functions() {
        let color = Color::from_rgb(0.5, 0.5, 0.5);

        // Test darken
        let darker = darken(color, 0.5);
        assert!(darker.r < color.r);
        assert!(darker.g < color.g);
        assert!(darker.b < color.b);

        // Test lighten
        let lighter = lighten(color, 0.5);
        assert!(lighter.r > color.r);
        assert!(lighter.g > color.g);
        assert!(lighter.b > color.b);

        // Test dim_color
        let dimmed = dim_color(color, 0.5);
        assert!(dimmed.r < color.r);

        // Test with_alpha
        let transparent = with_alpha(color, 0.3);
        assert!((transparent.a - 0.3).abs() < f32::EPSILON);
        assert!((transparent.r - color.r).abs() < f32::EPSILON);
    }

    #[test]
    fn test_style_copy() {
        let theme = iced::Theme::Dark;
        let style1 = from_iced_theme(&theme);
        let style2 = style1;

        // Verify colors are approximately equal (using epsilon for float comparison)
        assert!(
            (style1.background.r - style2.background.r).abs() < f32::EPSILON
        );
        assert!(
            (style1.text_color.r - style2.text_color.r).abs() < f32::EPSILON
        );
        assert!(
            (style1.gutter_background.r - style2.gutter_background.r).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn test_catalog_default() {
        let theme = iced::Theme::Dark;
        let class = <iced::Theme as Catalog>::default();
        let style = theme.style(&class);

        // Should produce a valid style
        assert!(style.background.r >= 0.0 && style.background.r <= 1.0);
        assert!(style.text_color.r >= 0.0 && style.text_color.r <= 1.0);
    }
}
