//! Vertically centered button content.
//!
//! iced's `button` lays its content out with `layout::padded`, which places the
//! child at exactly `(padding.left, padding.top)` and adds no vertical centering
//! step (`iced_core::layout::positioned`). A label therefore sits at the top
//! padding edge, and the button's height is just content + padding. That looks
//! centered only when the top and bottom padding happen to match the label's own
//! line box — and text defaults to a 1.3x line height, so the optical centre
//! usually sits low. With small vertical padding (the codebase is full of
//! `padding([2, 6])`-style buttons) the text reads as noticeably high or low,
//! which is issue #29.
//!
//! Wrapping the label in a container that fills the button and centres on both
//! axes fixes it. That is the pattern the icon-rail buttons already use; this
//! module makes it reusable instead of re-derived per call site.

use iced::widget::{button, container, Button};
use iced::{Element, Length};

/// Wraps `content` so it is centred within the button's box.
///
/// Use with an explicit `height` for a predictable hit target:
///
/// ```ignore
/// button(centered(text("Send").size(12)))
///     .height(32)
///     .padding([0, 16])
/// ```
pub fn centered<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    // Only the vertical axis is centred, and the width deliberately stays
    // `Shrink`.
    //
    // Using `center_x(Length::Fill)` here would make this wrapper greedily
    // claim all available width; the button takes its width hint from its
    // content, so it would stretch to fill its row instead of hugging its
    // label. `center_y(Length::Fill)` fills only the height the button already
    // has (its fixed height), which is exactly what centres the text.
    container(content)
        .center_y(Length::Fill)
        .into()
}

/// A button whose label is vertically and horizontally centred within a fixed
/// height.
///
/// Prefer this over `button(...).padding([v, h])` for anything with a visible
/// background: the fixed height makes the control's size independent of the
/// label's line metrics, so a row of buttons lines up and each one is a
/// consistent, comfortably sized target.
pub fn centered_button<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    height: f32,
    horizontal_padding: f32,
) -> Button<'a, Message> {
    button(centered(content))
        .height(Length::Fixed(height))
        .padding(iced::Padding {
            top: 0.0,
            right: horizontal_padding,
            bottom: 0.0,
            left: horizontal_padding,
        })
}

/// Comfortable default height for a primary action button.
///
/// 36px matches what the padding-only buttons already measured
/// (a 15px label at the default 1.3x line height, plus 8px above and below), so
/// switching a button to this helper corrects its vertical alignment without
/// changing the size of the control.
pub const BUTTON_HEIGHT: f32 = 36.0;

/// Height for compact/secondary buttons (toolbars, inline row actions).
pub const BUTTON_HEIGHT_SM: f32 = 26.0;

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::Widget;

    /// The wrapper must not claim horizontal space.
    ///
    /// `Button::new` derives its own width from `content.size_hint().width.fluid()`,
    /// and `Length::fluid` maps `Fill -> Fill`. So a wrapper reporting `Fill`
    /// (which is what `center_x(Length::Fill)` does) silently turns the button
    /// into a full-width one — the Send button stretched across the whole URL
    /// bar. Only the vertical axis may fill.
    #[test]
    fn centered_content_does_not_claim_horizontal_space() {
        let element: iced::Element<'_, ()> = centered(iced::widget::text("Send"));
        let size = element.as_widget().size_hint();

        assert_eq!(
            size.width,
            iced::Length::Shrink,
            "must shrink horizontally, or the button stretches to fill its row"
        );
        assert_eq!(
            size.height,
            iced::Length::Fill,
            "should fill the button's fixed height so the label centres"
        );
    }

    /// And the resulting button must therefore also hug its label.
    #[test]
    fn centered_button_hugs_its_label_horizontally() {
        let btn = centered_button(iced::widget::text("Send"), BUTTON_HEIGHT, 20.0);
        let element: iced::Element<'_, ()> = btn.into();
        let size = element.as_widget().size_hint();

        assert_eq!(size.width, iced::Length::Shrink, "button must not be full-width");
    }
}
