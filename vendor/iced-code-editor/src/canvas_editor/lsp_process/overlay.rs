//! LSP overlay UI components for displaying hover tooltips and completion menus.
//!
//! Provides [`LspOverlayState`] for storing overlay display state and
//! [`view_lsp_overlay`] for rendering it on top of a [`CodeEditor`].

use crate::CodeEditor;
use iced::widget::{
    Id, Space, button, column, container, markdown, mouse_area, row,
    scrollable, stack, text,
};
use iced::{Background, Border, Color, Element, Length, Point, Shadow, Theme};

/// Maximum number of completion items shown at once in the menu.
const MAX_COMPLETION_ITEMS: usize = 8;
/// Height in pixels of each completion item row.
const COMPLETION_ITEM_HEIGHT: f32 = 20.0;
/// Height in pixels of the completion menu header area.
const COMPLETION_HEADER_HEIGHT: f32 = 24.0;
/// Padding in pixels around the completion item list.
const COMPLETION_PADDING: f32 = 4.0;
/// Maximum width in pixels of the completion menu.
const COMPLETION_MENU_WIDTH: f32 = 250.0;
/// Border radius in pixels applied to scrollable rail and scroller borders.
const SCROLLABLE_BORDER_RADIUS: f32 = 4.0;

/// State for the LSP overlay display (hover tooltips and completion menus).
///
/// This struct aggregates all display-related LSP state. Instantiate once in
/// your application and pass it to [`view_lsp_overlay`] for rendering.
///
/// # Example
///
/// ```
/// use iced_code_editor::LspOverlayState;
///
/// let mut state = LspOverlayState::new();
/// assert!(!state.hover_visible);
/// assert!(!state.completion_visible);
/// ```
pub struct LspOverlayState {
    /// The hover text received from the LSP server.
    pub hover_text: Option<String>,
    /// Parsed markdown items derived from `hover_text`.
    pub hover_items: Vec<iced::widget::markdown::Item>,
    /// Whether the hover tooltip is currently visible.
    pub hover_visible: bool,
    /// Screen position where the hover tooltip should be rendered.
    pub hover_position: Option<Point>,
    /// Whether the mouse cursor is currently over the hover tooltip.
    pub hover_interactive: bool,
    /// All completion items received from the LSP server.
    pub all_completions: Vec<String>,
    /// Current filter string applied to completion items.
    pub completion_filter: String,
    /// Filtered completion items to display.
    pub completion_items: Vec<String>,
    /// Whether the completion menu is currently visible.
    pub completion_visible: bool,
    /// Index of the currently selected completion item.
    pub completion_selected: usize,
    /// Whether completion has been suppressed after applying an item.
    pub completion_suppressed: bool,
    /// Screen position of the completion menu anchor.
    pub completion_position: Option<Point>,
}

impl LspOverlayState {
    /// Creates a new [`LspOverlayState`] with all fields at their default values.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let state = LspOverlayState::new();
    /// assert!(!state.hover_visible);
    /// assert!(!state.completion_visible);
    /// ```
    pub fn new() -> Self {
        Self {
            hover_text: None,
            hover_items: Vec::new(),
            hover_visible: false,
            hover_position: None,
            hover_interactive: false,
            all_completions: Vec::new(),
            completion_filter: String::new(),
            completion_items: Vec::new(),
            completion_visible: false,
            completion_selected: 0,
            completion_suppressed: false,
            completion_position: None,
        }
    }

    /// Sets the screen position where the hover tooltip should appear.
    ///
    /// Call this when dispatching a hover request to the LSP server.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Point;
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.set_hover_position(Point::new(100.0, 200.0));
    /// assert_eq!(state.hover_position, Some(Point::new(100.0, 200.0)));
    /// ```
    pub fn set_hover_position(&mut self, point: Point) {
        self.hover_position = Some(point);
    }

    /// Displays a hover tooltip with the given text.
    ///
    /// Parses the text as markdown and marks the tooltip as visible.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.show_hover("**bold** text".to_string());
    /// assert!(state.hover_visible);
    /// assert!(state.hover_text.is_some());
    /// ```
    pub fn show_hover(&mut self, text: String) {
        self.hover_items = iced::widget::markdown::parse(&text).collect();
        self.hover_text = Some(text);
        self.hover_visible = true;
    }

    /// Clears all hover-related state.
    ///
    /// Resets hover text, items, visibility, position, and interaction flags.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.show_hover("some text".to_string());
    /// state.clear_hover();
    /// assert!(!state.hover_visible);
    /// assert!(state.hover_text.is_none());
    /// ```
    pub fn clear_hover(&mut self) {
        self.hover_text = None;
        self.hover_items.clear();
        self.hover_visible = false;
        self.hover_position = None;
        self.hover_interactive = false;
    }

    /// Sets the completion items and their display position.
    ///
    /// Resets the selection to index 0 and applies the current filter.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Point;
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.set_completions(
    ///     vec!["foo".to_string(), "bar".to_string()],
    ///     Point::ORIGIN,
    /// );
    /// assert_eq!(state.completion_items.len(), 2);
    /// ```
    pub fn set_completions(&mut self, items: Vec<String>, position: Point) {
        self.all_completions = items;
        self.completion_selected = 0;
        self.completion_position = Some(position);
        self.filter_completions();
    }

    /// Clears all completion-related state.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Point;
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.set_completions(vec!["foo".to_string()], Point::ORIGIN);
    /// state.clear_completions();
    /// assert!(!state.completion_visible);
    /// assert!(state.all_completions.is_empty());
    /// ```
    pub fn clear_completions(&mut self) {
        self.all_completions.clear();
        self.completion_items.clear();
        self.completion_filter.clear();
        self.completion_visible = false;
        self.completion_suppressed = false;
    }

    /// Filters `all_completions` into `completion_items` using `completion_filter`.
    ///
    /// Updates `completion_visible` and clamps `completion_selected` if needed.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Point;
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.set_completions(
    ///     vec!["foo".to_string(), "bar".to_string()],
    ///     Point::ORIGIN,
    /// );
    /// state.completion_filter = "fo".to_string();
    /// state.filter_completions();
    /// assert_eq!(state.completion_items, vec!["foo".to_string()]);
    /// ```
    pub fn filter_completions(&mut self) {
        let filter = self.completion_filter.to_lowercase();
        if filter.is_empty() {
            self.completion_items = self.all_completions.clone();
        } else {
            self.completion_items = self
                .all_completions
                .iter()
                .filter(|item| item.to_lowercase().contains(&filter))
                .cloned()
                .collect();
        }
        self.completion_visible = !self.completion_items.is_empty();
        if self.completion_selected >= self.completion_items.len() {
            self.completion_selected =
                self.completion_items.len().saturating_sub(1);
        }
    }

    /// Navigates through the completion list by `delta` steps, wrapping at boundaries.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Point;
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.set_completions(
    ///     vec!["a".to_string(), "b".to_string(), "c".to_string()],
    ///     Point::ORIGIN,
    /// );
    /// state.navigate(1);
    /// assert_eq!(state.completion_selected, 1);
    /// state.navigate(-1);
    /// assert_eq!(state.completion_selected, 0);
    /// ```
    pub fn navigate(&mut self, delta: i32) {
        if self.completion_items.is_empty() {
            return;
        }
        let len = self.completion_items.len();
        let current = self.completion_selected as i32;
        self.completion_selected =
            ((current + delta).rem_euclid(len as i32)) as usize;
    }

    /// Returns the currently selected completion item, if any.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Point;
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.set_completions(vec!["foo".to_string()], Point::ORIGIN);
    /// assert_eq!(state.selected_item(), Some("foo"));
    /// ```
    pub fn selected_item(&self) -> Option<&str> {
        self.completion_items.get(self.completion_selected).map(String::as_str)
    }

    /// Returns the vertical scroll offset in pixels to keep the selected
    /// completion item visible when navigating with the keyboard.
    ///
    /// Pass the returned value to `scrollable::AbsoluteOffset::y`.
    ///
    /// # Example
    ///
    /// ```
    /// use iced::Point;
    /// use iced_code_editor::LspOverlayState;
    ///
    /// let mut state = LspOverlayState::new();
    /// state.set_completions(
    ///     vec!["a".to_string(), "b".to_string(), "c".to_string()],
    ///     Point::ORIGIN,
    /// );
    /// state.navigate(2);
    /// assert_eq!(state.scroll_offset_for_selected(), 40.0);
    /// ```
    pub fn scroll_offset_for_selected(&self) -> f32 {
        self.completion_selected as f32 * COMPLETION_ITEM_HEIGHT
    }
}

impl Default for LspOverlayState {
    fn default() -> Self {
        Self::new()
    }
}

/// Messages produced by LSP overlay UI interactions.
///
/// Use these in your application's `update` function to handle hover
/// and completion interactions.
#[derive(Debug, Clone)]
pub enum LspOverlayMessage {
    /// The mouse cursor entered the hover tooltip area.
    HoverEntered,
    /// The mouse cursor exited the hover tooltip area.
    HoverExited,
    /// A completion item at the given index was clicked.
    CompletionSelected(usize),
    /// The completion menu was dismissed by clicking outside it.
    CompletionClosed,
    /// Navigate up in the completion list (e.g., keyboard Up arrow).
    CompletionNavigateUp,
    /// Navigate down in the completion list (e.g., keyboard Down arrow).
    CompletionNavigateDown,
    /// Confirm the currently highlighted completion item (e.g., Enter key).
    CompletionConfirm,
}

/// Measures the maximum pixel width of any line in the given text.
fn measure_hover_width(editor: &CodeEditor, text: &str) -> f32 {
    text.lines().map(|line| editor.measure_text_width(line)).fold(0.0, f32::max)
}

/// Renders LSP overlay elements (hover tooltip and completion menu) on top of a [`CodeEditor`].
///
/// Returns an [`Element`] containing the overlays positioned relative to the editor viewport.
/// The function maps [`LspOverlayMessage`] values to the application message type `M` via `f`.
///
/// # Arguments
///
/// * `state` — current overlay display state
/// * `editor` — the editor this overlay is associated with (used for viewport measurements)
/// * `theme` — the active Iced theme for styling
/// * `font_size` — font size in points, used for markdown rendering
/// * `line_height` — line height in pixels, used for vertical positioning
/// * `f` — mapping function from [`LspOverlayMessage`] to the app's message type
///
/// # Example
///
/// ```no_run
/// use iced_code_editor::{
///     CodeEditor, LspOverlayMessage, LspOverlayState, view_lsp_overlay,
/// };
///
/// struct App {
///     editor: CodeEditor,
///     overlay: LspOverlayState,
/// }
///
/// #[derive(Clone)]
/// enum Message {
///     Overlay(LspOverlayMessage),
/// }
///
/// fn view(app: &App) -> iced::Element<'_, Message> {
///     view_lsp_overlay(
///         &app.overlay,
///         &app.editor,
///         &iced::Theme::Dark,
///         14.0,
///         20.0,
///         Message::Overlay,
///     )
/// }
/// ```
pub fn view_lsp_overlay<'a, M: Clone + 'a>(
    state: &'a LspOverlayState,
    editor: &'a CodeEditor,
    theme: &'a Theme,
    font_size: f32,
    line_height: f32,
    f: impl Fn(LspOverlayMessage) -> M + 'a,
) -> Element<'a, M> {
    // Pre-compute messages so we can clone them freely
    let msg_hover_entered = f(LspOverlayMessage::HoverEntered);
    let msg_hover_exited = f(LspOverlayMessage::HoverExited);
    let msg_completion_closed = f(LspOverlayMessage::CompletionClosed);
    let msg_completion_selected: Vec<M> = (0..state.completion_items.len())
        .map(|i| f(LspOverlayMessage::CompletionSelected(i)))
        .collect();

    let mut has_overlay = false;

    // Build the hover tooltip layer
    let hover_layer: Element<'a, M> = build_hover_layer(
        state,
        editor,
        theme,
        (font_size, line_height),
        msg_hover_entered,
        msg_hover_exited,
        &mut has_overlay,
    );

    // Build the auto-completion menu layer
    let completion_layer: Element<'a, M> = build_completion_layer(
        state,
        editor,
        line_height,
        msg_completion_closed,
        msg_completion_selected,
        &mut has_overlay,
    );

    if !has_overlay {
        return container(
            Space::new().width(Length::Shrink).height(Length::Shrink),
        )
        .into();
    }

    let base = container(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill);

    // Hover appears on top of completion
    stack![base, completion_layer, hover_layer].into()
}

/// Builds the hover tooltip layer.
fn build_hover_layer<'a, M: Clone + 'a>(
    state: &'a LspOverlayState,
    editor: &'a CodeEditor,
    theme: &'a Theme,
    text_metrics: (f32, f32),
    msg_entered: M,
    msg_exited: M,
    has_overlay: &mut bool,
) -> Element<'a, M> {
    let (font_size, line_height) = text_metrics;
    if !state.hover_visible {
        return empty_overlay();
    }

    let Some(hover) =
        state.hover_text.as_ref().filter(|t| !t.trim().is_empty())
    else {
        return empty_overlay();
    };

    let line_count = hover.lines().count().max(1);
    let visible_lines = line_count.min(10);
    let hover_padding = 8.0;
    let scroll_height = line_height * visible_lines as f32
        + (line_height * 0.75).max(10.0)
        + hover_padding * 2.0;

    let viewport_width = editor.viewport_width();
    let max_line_width = measure_hover_width(editor, hover);
    let max_width = (viewport_width - 24.0).max(0.0);
    let content_max_width = if max_width > hover_padding * 2.0 {
        max_width - hover_padding * 2.0
    } else {
        max_width
    };
    let content_width = if content_max_width > 0.0 {
        max_line_width.min(content_max_width)
    } else {
        max_line_width
    };
    let hover_width = content_width + hover_padding * 2.0;

    let palette = theme.palette();
    let markdown_settings = markdown::Settings::with_text_size(
        font_size,
        markdown::Style::from_palette(palette),
    );

    let entered_for_map = msg_entered.clone();
    let entered_for_enter = msg_entered.clone();
    let entered_for_move = msg_entered;

    let hover_content = scrollable(
        container(
            markdown::view(&state.hover_items, markdown_settings)
                .map(move |_| entered_for_map.clone()),
        )
        .width(Length::Fixed(hover_width))
        .padding(hover_padding),
    )
    .height(Length::Fixed(scroll_height))
    .width(Length::Fixed(hover_width))
    .style(|theme: &Theme, _status| {
        let palette = theme.extended_palette();
        scrollable::Style {
            container: container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..container::Style::default()
            },
            vertical_rail: lsp_scrollable_rail(palette),
            horizontal_rail: lsp_scrollable_rail(palette),
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Color::TRANSPARENT.into(),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: Color::TRANSPARENT,
            },
        }
    });

    let hover_box = container(column![hover_content])
        .width(Length::Shrink)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(
                    palette.background.weak.color,
                )),
                border: iced::Border {
                    color: palette.primary.weak.color,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        });

    let hover_box: Element<'_, M> = mouse_area(hover_box)
        .on_enter(entered_for_enter)
        .on_move(move |_| entered_for_move.clone())
        .on_exit(msg_exited)
        .into();

    let hover_pos = state.hover_position.unwrap_or(Point::new(4.0, 4.0));
    let viewport_scroll = editor.viewport_scroll();
    let hover_pos =
        Point::new(hover_pos.x, (hover_pos.y - viewport_scroll).max(0.0));
    let viewport_height = editor.viewport_height();
    let gap = 1.0;
    let show_above = hover_pos.y >= scroll_height + gap;

    let gap_x = (editor.char_width() * 0.5).max(2.0);
    let right_x = hover_pos.x + gap_x;
    let left_x = hover_pos.x - hover_width - gap_x;
    let max_x = (viewport_width - hover_width - 4.0).max(0.0);

    let offset_x = if right_x <= max_x {
        right_x
    } else if left_x >= 0.0 {
        left_x
    } else {
        right_x.clamp(0.0, max_x)
    };

    let offset_y = if show_above {
        (hover_pos.y - scroll_height - gap).max(0.0)
    } else {
        (hover_pos.y + line_height + gap).max(0.0).min(viewport_height)
    };

    *has_overlay = true;

    container(
        column![
            Space::new().height(Length::Fixed(offset_y)),
            row![Space::new().width(Length::Fixed(offset_x)), hover_box]
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Builds the auto-completion menu layer.
fn build_completion_layer<'a, M: Clone + 'a>(
    state: &'a LspOverlayState,
    editor: &'a CodeEditor,
    line_height: f32,
    msg_closed: M,
    msg_selected: Vec<M>,
    has_overlay: &mut bool,
) -> Element<'a, M> {
    if !state.completion_visible
        || state.completion_items.is_empty()
        || state.completion_suppressed
    {
        return empty_overlay();
    }

    let visible_count = state.completion_items.len().min(MAX_COMPLETION_ITEMS);
    let menu_height = COMPLETION_HEADER_HEIGHT
        + (visible_count as f32 * COMPLETION_ITEM_HEIGHT)
        + (COMPLETION_PADDING * 2.0);

    let cursor_pos = state.completion_position.unwrap_or(Point::new(4.0, 4.0));
    let viewport_width = editor.viewport_width();
    let viewport_height = editor.viewport_height();
    let viewport_scroll = editor.viewport_scroll();

    let menu_width = COMPLETION_MENU_WIDTH.min(viewport_width - 8.0);
    let adjusted_y = (cursor_pos.y - viewport_scroll).max(0.0);
    let space_below = viewport_height - adjusted_y - line_height;
    let show_above =
        space_below < menu_height + 4.0 && adjusted_y >= menu_height + 4.0;

    let offset_x = cursor_pos.x.min(viewport_width - menu_width - 4.0).max(4.0);
    let offset_y = if show_above {
        (adjusted_y - menu_height - 4.0).max(0.0)
    } else {
        adjusted_y + line_height + 4.0
    };

    let completion_elements: Vec<Element<'_, M>> = state
        .completion_items
        .iter()
        .enumerate()
        .zip(msg_selected)
        .map(|((index, item), msg)| {
            let is_selected = index == state.completion_selected;
            button(
                text(item.clone())
                    .size(12)
                    .line_height(iced::widget::text::LineHeight::Relative(1.5)),
            )
            .padding([2, 8])
            .width(Length::Fill)
            .on_press(msg)
            .style(move |theme: &Theme, _status| {
                let palette = theme.extended_palette();
                if is_selected {
                    button::Style {
                        background: Some(iced::Background::Color(
                            palette.primary.weak.color,
                        )),
                        text_color: Color::WHITE,
                        ..Default::default()
                    }
                } else {
                    button::Style {
                        background: Some(iced::Background::Color(
                            palette.background.weak.color,
                        )),
                        text_color: Color::WHITE,
                        ..Default::default()
                    }
                }
            })
            .into()
        })
        .collect();

    let completion_box = scrollable(column(completion_elements).spacing(0))
        .height(Length::Fixed(menu_height))
        .width(Length::Fixed(menu_width))
        .id(Id::new("completion_scrollable"))
        .style(|theme: &Theme, _status| {
            let palette = theme.extended_palette();
            scrollable::Style {
                container: container::Style {
                    background: Some(iced::Background::Color(
                        palette.background.weak.color,
                    )),
                    border: iced::Border {
                        color: palette.primary.weak.color,
                        width: 1.0,
                        radius: SCROLLABLE_BORDER_RADIUS.into(),
                    },
                    ..Default::default()
                },
                vertical_rail: lsp_scrollable_rail(palette),
                horizontal_rail: lsp_scrollable_rail(palette),
                gap: None,
                auto_scroll: scrollable::AutoScroll {
                    background: Color::TRANSPARENT.into(),
                    border: Border::default(),
                    shadow: Shadow::default(),
                    icon: Color::TRANSPARENT,
                },
            }
        });

    *has_overlay = true;

    let click_outside =
        button(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(msg_closed)
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..Default::default()
            });

    let completion_content = container(
        column![
            Space::new().height(Length::Fixed(offset_y)),
            row![Space::new().width(Length::Fixed(offset_x)), completion_box]
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    stack![click_outside, completion_content].into()
}

/// Creates the scrollable rail style used in LSP overlay panels.
///
/// Both the hover tooltip and the completion menu share the same rail appearance:
/// a translucent background track with a primary-coloured scroller and no border.
fn lsp_scrollable_rail(
    palette: &iced::theme::palette::Extended,
) -> scrollable::Rail {
    scrollable::Rail {
        background: Some(palette.background.weak.color.into()),
        border: Border {
            radius: SCROLLABLE_BORDER_RADIUS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        scroller: scrollable::Scroller {
            background: palette.primary.weak.color.into(),
            border: Border {
                radius: SCROLLABLE_BORDER_RADIUS.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
    }
}

/// Returns a zero-size placeholder element.
fn empty_overlay<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(Length::Shrink).height(Length::Shrink)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    #[test]
    fn test_lsp_overlay_state_new() {
        let state = LspOverlayState::new();
        assert!(!state.hover_visible);
        assert!(!state.completion_visible);
        assert!(state.hover_text.is_none());
        assert!(state.completion_items.is_empty());
    }

    #[test]
    fn test_show_hover() {
        let mut state = LspOverlayState::new();
        state.show_hover("hello".to_string());
        assert!(state.hover_visible);
        assert_eq!(state.hover_text, Some("hello".to_string()));
    }

    #[test]
    fn test_clear_hover() {
        let mut state = LspOverlayState::new();
        state.show_hover("hello".to_string());
        state.hover_interactive = true;
        state.hover_position = Some(Point::ORIGIN);
        state.clear_hover();
        assert!(!state.hover_visible);
        assert!(state.hover_text.is_none());
        assert!(!state.hover_interactive);
        assert!(state.hover_position.is_none());
    }

    #[test]
    fn test_set_hover_position() {
        let mut state = LspOverlayState::new();
        state.set_hover_position(Point::new(10.0, 20.0));
        assert_eq!(state.hover_position, Some(Point::new(10.0, 20.0)));
    }

    #[test]
    fn test_set_completions() {
        let mut state = LspOverlayState::new();
        state.set_completions(
            vec!["foo".to_string(), "bar".to_string()],
            Point::ORIGIN,
        );
        assert_eq!(state.completion_items.len(), 2);
        assert!(state.completion_visible);
        assert_eq!(state.completion_selected, 0);
    }

    #[test]
    fn test_clear_completions() {
        let mut state = LspOverlayState::new();
        state.set_completions(vec!["foo".to_string()], Point::ORIGIN);
        state.clear_completions();
        assert!(!state.completion_visible);
        assert!(state.all_completions.is_empty());
        assert!(state.completion_items.is_empty());
    }

    #[test]
    fn test_filter_completions() {
        let mut state = LspOverlayState::new();
        state.set_completions(
            vec!["foo".to_string(), "bar".to_string(), "baz".to_string()],
            Point::ORIGIN,
        );
        state.completion_filter = "ba".to_string();
        state.filter_completions();
        assert_eq!(state.completion_items.len(), 2);
        assert!(state.completion_items.contains(&"bar".to_string()));
        assert!(state.completion_items.contains(&"baz".to_string()));
    }

    #[test]
    fn test_navigate() {
        let mut state = LspOverlayState::new();
        state.set_completions(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            Point::ORIGIN,
        );
        state.navigate(1);
        assert_eq!(state.completion_selected, 1);
        state.navigate(-1);
        assert_eq!(state.completion_selected, 0);
        // Wrap around going up
        state.navigate(-1);
        assert_eq!(state.completion_selected, 2);
        // Wrap around going down
        state.navigate(1);
        assert_eq!(state.completion_selected, 0);
    }

    #[test]
    fn test_scroll_offset_for_selected() {
        let mut state = LspOverlayState::new();
        assert_eq!(state.scroll_offset_for_selected(), 0.0);
        state.set_completions(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            Point::ORIGIN,
        );
        assert_eq!(state.scroll_offset_for_selected(), 0.0);
        state.navigate(1);
        assert_eq!(state.scroll_offset_for_selected(), COMPLETION_ITEM_HEIGHT);
        state.navigate(1);
        assert_eq!(
            state.scroll_offset_for_selected(),
            2.0 * COMPLETION_ITEM_HEIGHT
        );
    }

    #[test]
    fn test_selected_item() {
        let mut state = LspOverlayState::new();
        assert_eq!(state.selected_item(), None);
        state.set_completions(
            vec!["first".to_string(), "second".to_string()],
            Point::ORIGIN,
        );
        assert_eq!(state.selected_item(), Some("first"));
        state.navigate(1);
        assert_eq!(state.selected_item(), Some("second"));
    }
}
