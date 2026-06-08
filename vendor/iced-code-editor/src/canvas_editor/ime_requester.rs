//! IME (Input Method Editor) Requester Widget
//!
//! This module provides a specialized invisible widget that manages IME state communication
//! between the application and the operating system.
//!
//! # What is IME?
//!
//! IME (Input Method Editor) is a system component that allows users to input complex characters
//! and symbols that cannot be directly represented on a standard keyboard. This is essential for:
//! - Asian languages (Chinese, Japanese, Korean) that have thousands of characters
//! - Special symbols and accented characters
//! - Predictive text and autocomplete features
//!
//! The IME displays a "candidate window" showing possible character choices as the user types,
//! and maintains "preedit" text (the temporary input being composed before final conversion).
//!
//! # Design Pattern
//!
//! This widget implements a non-standard pattern where it acts as an invisible bridge rather
//! than a visual element. It:
//! - Returns `Length::Shrink` for both dimensions but produces a zero-size layout
//! - Exists solely to call `shell.request_input_method()` on each frame
//! - Synchronizes IME state (enabled/disabled, cursor position, preedit text) with the OS

use iced::advanced::input_method;
use iced::advanced::widget::{Widget, tree};
use iced::advanced::{Renderer, Shell};
use iced::{Event, Length, Rectangle, Size, Vector, mouse, window};

/// An invisible widget that manages Input Method Editor (IME) state.
///
/// This widget serves as a bridge between the application's text editor and the
/// operating system's IME infrastructure. It handles:
/// - Enabling/disabling IME based on focus state
/// - Positioning the IME candidate window near the text cursor
/// - Synchronizing preedit (composition) text state
///
/// # Design
///
/// The widget is intentionally invisible (zero-size layout) and only exists to
/// communicate IME state to the OS through `shell.request_input_method()` calls.
/// This ensures the OS knows where to display the IME candidate window and what
/// text is currently being composed.
#[derive(Debug, Clone)]
pub struct ImeRequester {
    // -------------------------------------------------------------------------
    // IME requester state fields
    // -------------------------------------------------------------------------
    /// Whether IME interaction is currently enabled.
    ///
    /// This is `true` only when the editor has both:
    /// - Iced focus (is_focused)
    /// - Internal canvas focus (has_canvas_focus)
    ///
    /// When `true`, IME is active and the candidate window may appear.
    /// When `false`, IME is disabled and any soft keyboard is hidden.
    /// Maps directly to the Enabled/Disabled state of `shell.request_input_method`.
    enabled: bool,

    /// The visual cursor (caret) position and dimensions.
    ///
    /// Specifies the exact screen location (x, y) and size (width, height) of the
    /// text cursor. The OS uses this rectangle to position the IME candidate window
    /// using the "over-the-spot" style, placing it near the cursor without obscuring it.
    ///
    /// Coordinates are widget-relative and converted to window-relative in `update()`.
    cursor: Rectangle,

    /// The current preedit (composition) text state.
    ///
    /// Contains the temporary text being composed before final conversion (e.g., "nihao"
    /// before converting to "你好"). This is sent back to the Shell to maintain consistent
    /// IME state across the application. `None` when no composition is in progress.
    preedit: Option<input_method::Preedit<String>>,
}

impl ImeRequester {
    /// Creates a new IME requester widget.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether IME interaction is enabled. Typically `true` when the editor
    ///   has both Iced focus and internal canvas focus, `false` otherwise.
    /// * `cursor` - The visual cursor position and size in widget-relative coordinates.
    ///   This will be converted to window-relative coordinates before being sent to the OS.
    /// * `preedit` - The current pre-edit (composition) text state. `None` if no text
    ///   is currently being composed.
    ///
    /// # Returns
    ///
    /// A new `ImeRequester` instance configured with the provided state.
    pub fn new(
        enabled: bool,
        cursor: Rectangle,
        preedit: Option<input_method::Preedit<String>>,
    ) -> Self {
        Self { enabled, cursor, preedit }
    }
}

// The ImeRequester widget implements a size of Length::Shrink for both dimensions
// but returns a zero-size layout. This creates an invisible widget that only exists
// to call shell.request_input_method. Consider documenting this design pattern more
// explicitly in the struct-level documentation, as it's a non-standard use of the
// Widget trait where the widget serves as a bridge rather than a visual element.
impl<Message> Widget<Message, iced::Theme, iced::Renderer> for ImeRequester
where
    iced::Renderer: Renderer,
{
    /// Returns the size strategy for this widget.
    ///
    /// # Returns
    ///
    /// A `Size` with both width and height set to `Length::Shrink`, indicating
    /// this widget should take up minimal space. The actual layout will be zero-sized.
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    /// Computes the layout for this widget.
    ///
    /// # Arguments
    ///
    /// * `_tree` - The widget tree (unused for stateless widgets)
    /// * `_renderer` - The renderer instance (unused)
    /// * `_limits` - Layout constraints (unused)
    ///
    /// # Returns
    ///
    /// A layout node with zero dimensions, making this widget invisible.
    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &iced::Renderer,
        _limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::Node::new(Size::new(0.0, 0.0))
    }

    /// Draws the widget.
    ///
    /// This is intentionally empty as the widget is invisible and serves only
    /// to manage IME state, not to render any visual content.
    ///
    /// # Arguments
    ///
    /// * `_tree` - The widget tree (unused)
    /// * `_renderer` - The renderer instance (unused)
    /// * `_theme` - The theme (unused)
    /// * `_style` - The rendering style (unused)
    /// * `_layout` - The computed layout (unused)
    /// * `_cursor` - The mouse cursor state (unused)
    /// * `_viewport` - The visible viewport (unused)
    fn draw(
        &self,
        _tree: &tree::Tree,
        _renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &iced::advanced::renderer::Style,
        _layout: iced::advanced::layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }

    /// Returns the widget's tag for state management.
    ///
    /// # Returns
    ///
    /// A stateless tag, indicating this widget maintains no internal state.
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    /// Returns the widget's state.
    ///
    /// # Returns
    ///
    /// `tree::State::None`, as this widget requires no internal state management.
    fn state(&self) -> tree::State {
        tree::State::None
    }

    /// Handles events and updates IME state.
    ///
    /// This is the core logic of the widget. On each `RedrawRequested` event, it sends
    /// the current IME state (enabled/disabled, cursor position, preedit text) to the
    /// operating system via `shell.request_input_method()`.
    ///
    /// # Why RedrawRequested?
    ///
    /// - Iced's IME protocol requires explicit state updates each frame or when changes occur
    /// - `RedrawRequested` marks the start of the render cycle, ensuring the OS receives
    ///   the latest cursor position so the candidate window tracks cursor movement accurately
    /// - Updating on input events (like `KeyPressed`) would use stale cursor positions from
    ///   the previous frame, since the widget hasn't been rebuilt with new state yet
    /// - `RedrawRequested` guarantees we're using the fresh cursor position calculated in
    ///   the latest `view()` pass
    ///
    /// # Arguments
    ///
    /// * `_tree` - The widget tree (unused)
    /// * `event` - The event to handle
    /// * `layout` - The widget's layout, used to get window-relative position
    /// * `_cursor` - The mouse cursor state (unused)
    /// * `_renderer` - The renderer instance (unused)
    /// * `_clipboard` - The clipboard interface (unused)
    /// * `shell` - The shell interface for making IME requests
    /// * `_viewport` - The visible viewport (unused)
    fn update(
        &mut self,
        _tree: &mut tree::Tree,
        event: &Event,
        layout: iced::advanced::layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // Core IME request logic
        // ---------------------------------------------------------------------
        // When enabled = true: Editor is active and focused. Send `InputMethod::Enabled`
        //   with the cursor rectangle and preedit content to activate IME and position
        //   the candidate window.
        //
        // When enabled = false: Editor is unfocused. Send `InputMethod::Disabled`
        //   to close the soft keyboard and reset IME state.
        // ---------------------------------------------------------------------
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            if self.enabled {
                // Convert widget-relative cursor position to window-relative coordinates.
                // This is required because the OS IME API expects window-relative positions,
                // not widget-relative ones. Without this conversion, the candidate window
                // would appear at the top-left of the window instead of near the actual cursor.
                let position = layout.bounds().position();
                let cursor_rect = Rectangle {
                    x: self.cursor.x + position.x,
                    y: self.cursor.y + position.y,
                    width: self.cursor.width,
                    height: self.cursor.height,
                };

                let ime = input_method::InputMethod::Enabled {
                    cursor: cursor_rect,
                    purpose: input_method::Purpose::Normal,
                    preedit: self
                        .preedit
                        .as_ref()
                        .map(input_method::Preedit::as_ref),
                };
                shell.request_input_method(&ime);
            } else {
                // Disable IME when the editor loses focus
                let disabled: input_method::InputMethod<&str> =
                    input_method::InputMethod::Disabled;
                shell.request_input_method(&disabled);
            }
        }
    }

    /// Returns the mouse interaction style for this widget.
    ///
    /// # Arguments
    ///
    /// * `_tree` - The widget tree (unused)
    /// * `_layout` - The widget's layout (unused)
    /// * `_cursor` - The mouse cursor state (unused)
    /// * `_viewport` - The visible viewport (unused)
    /// * `_renderer` - The renderer instance (unused)
    ///
    /// # Returns
    ///
    /// `mouse::Interaction::None`, as this invisible widget doesn't interact with the mouse.
    fn mouse_interaction(
        &self,
        _tree: &tree::Tree,
        _layout: iced::advanced::layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    /// Returns an overlay element for this widget.
    ///
    /// # Arguments
    ///
    /// * `_tree` - The widget tree (unused)
    /// * `_layout` - The widget's layout (unused)
    /// * `_renderer` - The renderer instance (unused)
    /// * `_viewport` - The visible viewport (unused)
    /// * `_translation` - The translation vector (unused)
    ///
    /// # Returns
    ///
    /// `None`, as this widget has no overlay.
    fn overlay<'a>(
        &'a mut self,
        _tree: &'a mut tree::Tree,
        _layout: iced::advanced::layout::Layout<'a>,
        _renderer: &iced::Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<iced::overlay::Element<'a, Message, iced::Theme, iced::Renderer>>
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Length, Point, Size};

    /// Tests the initialization of ImeRequester.
    ///
    /// Verifies that:
    /// 1. The enabled state is correctly stored
    /// 2. The cursor rectangle is preserved
    /// 3. The preedit content is correctly passed through
    #[test]
    fn test_ime_requester_initialization() {
        // Setup test data
        let cursor =
            Rectangle::new(Point::new(10.0, 10.0), Size::new(2.0, 20.0));
        let preedit = Some(input_method::Preedit {
            content: "test".to_string(),
            selection: None,
            text_size: None,
        });

        // Create instance
        let requester = ImeRequester::new(true, cursor, preedit);

        // Assertions
        assert!(requester.enabled, "Should be enabled");
        assert_eq!(requester.cursor, cursor, "Cursor rect should match");

        // Verify preedit content matches
        if let Some(p) = requester.preedit {
            assert_eq!(p.content, "test", "Preedit content should match");
        } else {
            assert!(requester.preedit.is_some(), "Preedit should be Some");
        }
    }

    /// Tests the Widget trait implementation details.
    ///
    /// Verifies that:
    /// 1. `size()` returns `Shrink/Shrink` (invisible widget design)
    /// 2. `tag()` returns a stateless tag (no state management needed)
    /// 3. `state()` returns `None` (no internal state tracking)
    #[test]
    fn test_ime_requester_layout_properties() {
        let cursor = Rectangle::new(Point::new(0.0, 0.0), Size::new(0.0, 0.0));
        let requester = ImeRequester::new(false, cursor, None);

        // Test size strategy - should be Shrink/Shrink for invisible widget
        let size =
            <ImeRequester as Widget<(), iced::Theme, iced::Renderer>>::size(
                &requester,
            );
        assert_eq!(size.width, Length::Shrink, "Width should be Shrink");
        assert_eq!(size.height, Length::Shrink, "Height should be Shrink");

        // Test widget tag - should be stateless since no state is managed
        assert_eq!(
            <ImeRequester as Widget<(), iced::Theme, iced::Renderer>>::tag(
                &requester
            ),
            tree::Tag::stateless(),
            "Widget should be stateless"
        );

        // Test widget state - should be None since no internal state exists
        assert!(matches!(
            <ImeRequester as Widget<(), iced::Theme, iced::Renderer>>::state(&requester),
            tree::State::None
        ), "Widget state should be None");
    }
}
