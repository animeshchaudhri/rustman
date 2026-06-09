//! Canvas rendering implementation using Iced's `canvas::Program`.

use iced::advanced::input_method;
use iced::mouse;
use iced::widget::canvas::{self, Geometry};
use iced::{Color, Event, Point, Rectangle, Size, Theme, keyboard};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

/// Computes geometry (x start and width) for a text segment used in rendering or highlighting.
///
/// # Arguments
///
/// * `line_content`: full text content of the current line.
/// * `visual_start_col`: start column index of the current visual line.
/// * `segment_start_col`: start column index of the target segment (e.g. highlight).
/// * `segment_end_col`: end column index of the target segment.
/// * `base_offset`: base X offset (usually gutter_width + padding).
///
/// # Returns
///
/// x_start, width
///
/// # Remark
///
/// This function handles CJK character widths correctly to keep highlights accurate.
fn calculate_segment_geometry(
    line_content: &str,
    visual_start_col: usize,
    segment_start_col: usize,
    segment_end_col: usize,
    base_offset: f32,
    full_char_width: f32,
    char_width: f32,
) -> (f32, f32) {
    // Clamp the segment to the current visual line so callers can safely pass
    // logical selection/match columns without worrying about wrapping boundaries.
    let segment_start_col = segment_start_col.max(visual_start_col);
    let segment_end_col = segment_end_col.max(segment_start_col);

    let mut prefix_width = 0.0;
    let mut segment_width = 0.0;

    // Compute widths directly from the source string to avoid allocating
    // intermediate `String` slices for prefix/segment.
    for (i, c) in line_content.chars().enumerate() {
        if i >= segment_end_col {
            break;
        }

        let w = super::measure_char_width(c, full_char_width, char_width);

        if i >= visual_start_col && i < segment_start_col {
            prefix_width += w;
        } else if i >= segment_start_col {
            segment_width += w;
        }
    }

    (base_offset + prefix_width, segment_width)
}

fn expand_tabs(text: &str, tab_width: usize) -> Cow<'_, str> {
    if !text.contains('\t') {
        return Cow::Borrowed(text);
    }

    let mut expanded = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch == '\t' {
            for _ in 0..tab_width {
                expanded.push(' ');
            }
        } else {
            expanded.push(ch);
        }
    }

    Cow::Owned(expanded)
}

use super::folding;
use super::wrapping::{VisualLine, WrappingCalculator};
use super::{ArrowDirection, CodeEditor, Message, measure_text_width};
use iced::widget::canvas::Action;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

/// Context for canvas rendering operations.
///
/// This struct packages commonly used rendering parameters to reduce
/// method signature complexity and improve code maintainability.
struct RenderContext<'a> {
    /// Visual lines calculated from wrapping
    visual_lines: &'a [VisualLine],
    /// Width of the canvas bounds
    bounds_width: f32,
    /// Width of the line number gutter
    gutter_width: f32,
    /// Height of each line in pixels
    line_height: f32,
    /// Font size in pixels
    font_size: f32,
    /// Full character width for wide characters (e.g., CJK)
    full_char_width: f32,
    /// Character width for narrow characters
    char_width: f32,
    /// Font to use for rendering text
    font: iced::Font,
    /// Horizontal scroll offset in pixels (subtracted from text X positions)
    horizontal_scroll_offset: f32,
}

impl CodeEditor {
    /// Draws line numbers and wrap indicators in the gutter area.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    /// * `visual_line` - The visual line to render
    /// * `y` - Y position for rendering
    fn draw_line_numbers(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        visual_line: &VisualLine,
        y: f32,
    ) {
        // The line-number area is the left part of the gutter; the fold margin
        // (when folding is enabled) is the right strip adjacent to the text.
        let number_area_width = self.line_number_gutter_width();

        if self.line_numbers_enabled {
            if visual_line.is_first_segment() {
                // Draw line number for first segment, centered in the number area.
                let line_num = visual_line.logical_line + 1;
                let line_num_text = format!("{}", line_num);
                let text_width = measure_text_width(
                    &line_num_text,
                    ctx.full_char_width,
                    ctx.char_width,
                );
                let x_pos = (number_area_width - text_width) / 2.0;
                frame.fill_text(canvas::Text {
                    content: line_num_text,
                    position: Point::new(x_pos, y + 2.0),
                    color: self.style.line_number_color,
                    size: ctx.font_size.into(),
                    font: ctx.font,
                    ..canvas::Text::default()
                });
            } else {
                // Draw wrap indicator for continuation lines.
                frame.fill_text(canvas::Text {
                    content: "↪".to_string(),
                    position: Point::new(number_area_width - 20.0, y + 2.0),
                    color: self.style.line_number_color,
                    size: ctx.font_size.into(),
                    font: ctx.font,
                    ..canvas::Text::default()
                });
            }
        }

        self.draw_fold_chevron(frame, ctx, visual_line, y, number_area_width);
    }

    /// Draws the fold chevron in the fold margin for a foldable header line.
    ///
    /// Draws nothing when folding is disabled, on continuation (wrapped)
    /// segments, or on lines that are not fold headers.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing metrics
    /// * `visual_line` - The visual line to render
    /// * `y` - Y position for rendering
    /// * `number_area_width` - Width of the line-number area (start of the fold margin)
    fn draw_fold_chevron(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        visual_line: &VisualLine,
        y: f32,
        number_area_width: f32,
    ) {
        if !self.folding_enabled || !visual_line.is_first_segment() {
            return;
        }

        let regions = self.foldable_regions();
        if !folding::is_fold_header(&regions, visual_line.logical_line) {
            return;
        }

        // `▶` when collapsed, `▼` when expanded.
        let chevron = if self.is_folded(visual_line.logical_line) {
            "▶"
        } else {
            "▼"
        };
        frame.fill_text(canvas::Text {
            content: chevron.to_string(),
            position: Point::new(number_area_width + 1.0, y + 2.0),
            color: self.style.line_number_color,
            size: ctx.font_size.into(),
            font: ctx.font,
            ..canvas::Text::default()
        });
    }

    /// Draws a `⋯` marker after the text of a collapsed fold header, signalling
    /// that lines are hidden below it (VS Code-style cue).
    ///
    /// Draws nothing unless folding is enabled and `visual_line` is the header
    /// of a currently collapsed region. Intended to be called inside the clipped
    /// code area so the marker cannot bleed into the gutter.
    fn draw_fold_collapsed_marker(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        visual_line: &VisualLine,
        y: f32,
    ) {
        if !self.folding_enabled
            || !visual_line.is_first_segment()
            || !self.is_folded(visual_line.logical_line)
        {
            return;
        }

        let line_content = self.buffer.line(visual_line.logical_line);
        let line_width = measure_text_width(
            line_content,
            ctx.full_char_width,
            ctx.char_width,
        );
        let x = ctx.gutter_width + 5.0 - ctx.horizontal_scroll_offset
            + line_width
            + 6.0;
        frame.fill_text(canvas::Text {
            content: "⋯".to_string(),
            position: Point::new(x, y + 2.0),
            color: self.style.line_number_color,
            size: ctx.font_size.into(),
            font: ctx.font,
            ..canvas::Text::default()
        });
    }

    /// Draws the background highlight for the current line.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    /// * `visual_line` - The visual line to check
    /// * `y` - Y position for rendering
    fn draw_current_line_highlight(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        visual_line: &VisualLine,
        y: f32,
    ) {
        if self.cursors.iter().any(|c| c.position.0 == visual_line.logical_line)
        {
            frame.fill_rectangle(
                Point::new(ctx.gutter_width, y),
                Size::new(ctx.bounds_width - ctx.gutter_width, ctx.line_height),
                self.style.current_line_highlight,
            );
        }
    }

    /// Draws text content with syntax highlighting or plain text fallback.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    /// * `visual_line` - The visual line to render
    /// * `y` - Y position for rendering
    /// * `syntax_ref` - Optional syntax reference for highlighting
    /// * `syntax_set` - Syntax set for highlighting
    /// * `syntax_theme` - Theme for syntax highlighting
    #[allow(clippy::too_many_arguments)]
    fn draw_text_with_syntax_highlighting(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        visual_line: &VisualLine,
        y: f32,
        syntax_ref: Option<&syntect::parsing::SyntaxReference>,
        syntax_set: &SyntaxSet,
        syntax_theme: Option<&syntect::highlighting::Theme>,
    ) {
        let full_line_content = self.buffer.line(visual_line.logical_line);

        // Convert character indices to byte indices for UTF-8 string slicing
        let start_byte = full_line_content
            .char_indices()
            .nth(visual_line.start_col)
            .map_or(full_line_content.len(), |(idx, _)| idx);
        let end_byte = full_line_content
            .char_indices()
            .nth(visual_line.end_col)
            .map_or(full_line_content.len(), |(idx, _)| idx);
        let line_segment = &full_line_content[start_byte..end_byte];

        if let (Some(syntax), Some(syntax_theme)) = (syntax_ref, syntax_theme) {
            let mut highlighter = HighlightLines::new(syntax, syntax_theme);

            // Highlight the full line to get correct token colors
            let full_line_ranges = highlighter
                .highlight_line(full_line_content, syntax_set)
                .unwrap_or_else(|_| {
                    vec![(Style::default(), full_line_content)]
                });

            // Extract only the ranges that fall within our segment
            let mut x_offset =
                ctx.gutter_width + 5.0 - ctx.horizontal_scroll_offset;
            let mut char_pos = 0;

            for (style, text) in full_line_ranges {
                let text_len = text.chars().count();
                let text_end = char_pos + text_len;

                // Check if this token intersects with our segment
                if text_end > visual_line.start_col
                    && char_pos < visual_line.end_col
                {
                    // Calculate the intersection
                    let segment_start = char_pos.max(visual_line.start_col);
                    let segment_end = text_end.min(visual_line.end_col);

                    let text_start_offset =
                        segment_start.saturating_sub(char_pos);
                    let text_end_offset =
                        text_start_offset + (segment_end - segment_start);

                    // Convert character offsets to byte offsets for UTF-8 slicing
                    let start_byte = text
                        .char_indices()
                        .nth(text_start_offset)
                        .map_or(text.len(), |(idx, _)| idx);
                    let end_byte = text
                        .char_indices()
                        .nth(text_end_offset)
                        .map_or(text.len(), |(idx, _)| idx);

                    let segment_text = &text[start_byte..end_byte];
                    let display_text =
                        expand_tabs(segment_text, super::TAB_WIDTH)
                            .into_owned();
                    let display_width = measure_text_width(
                        &display_text,
                        ctx.full_char_width,
                        ctx.char_width,
                    );

                    let color = Color::from_rgb(
                        f32::from(style.foreground.r) / 255.0,
                        f32::from(style.foreground.g) / 255.0,
                        f32::from(style.foreground.b) / 255.0,
                    );

                    frame.fill_text(canvas::Text {
                        content: display_text,
                        position: Point::new(x_offset, y + 2.0),
                        color,
                        size: ctx.font_size.into(),
                        font: ctx.font,
                        ..canvas::Text::default()
                    });

                    x_offset += display_width;
                }

                char_pos = text_end;
            }
        } else {
            // Fallback to plain text
            let display_text =
                expand_tabs(line_segment, super::TAB_WIDTH).into_owned();
            frame.fill_text(canvas::Text {
                content: display_text,
                position: Point::new(
                    ctx.gutter_width + 5.0 - ctx.horizontal_scroll_offset,
                    y + 2.0,
                ),
                color: self.style.text_color,
                size: ctx.font_size.into(),
                font: ctx.font,
                ..canvas::Text::default()
            });
        }
    }

    /// Draws search match highlights for all visible matches.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    /// * `first_visible_line` - First visible visual line index
    /// * `last_visible_line` - Last visible visual line index
    fn draw_search_highlights(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        start_visual_idx: usize,
        end_visual_idx: usize,
    ) {
        if !self.search_state.is_open || self.search_state.query.is_empty() {
            return;
        }

        let query_len = self.search_state.query.chars().count();

        let start_visual_idx = start_visual_idx.min(ctx.visual_lines.len());
        let end_visual_idx = end_visual_idx.min(ctx.visual_lines.len());

        let end_visual_inclusive = end_visual_idx
            .saturating_sub(1)
            .min(ctx.visual_lines.len().saturating_sub(1));

        if let (Some(start_vl), Some(end_vl)) = (
            ctx.visual_lines.get(start_visual_idx),
            ctx.visual_lines.get(end_visual_inclusive),
        ) {
            let min_logical_line = start_vl.logical_line;
            let max_logical_line = end_vl.logical_line;

            // Optimization: Use get_visible_match_range to find matches in view
            // This uses binary search + early termination for O(log N) performance
            let match_range = super::search::get_visible_match_range(
                &self.search_state.matches,
                min_logical_line,
                max_logical_line,
            );

            for (match_idx, search_match) in self
                .search_state
                .matches
                .iter()
                .enumerate()
                .skip(match_range.start)
                .take(match_range.len())
            {
                // Determine if this is the current match
                let is_current =
                    self.search_state.current_match_index == Some(match_idx);

                let highlight_color = if is_current {
                    // Orange for current match
                    Color { r: 1.0, g: 0.6, b: 0.0, a: 0.4 }
                } else {
                    // Yellow for other matches
                    Color { r: 1.0, g: 1.0, b: 0.0, a: 0.3 }
                };

                // Convert logical position to visual line
                let start_visual = WrappingCalculator::logical_to_visual(
                    ctx.visual_lines,
                    search_match.line,
                    search_match.col,
                );
                let end_visual = WrappingCalculator::logical_to_visual(
                    ctx.visual_lines,
                    search_match.line,
                    search_match.col + query_len,
                );

                if let (Some(start_v), Some(end_v)) = (start_visual, end_visual)
                {
                    if start_v == end_v {
                        // Match within same visual line
                        let y = start_v as f32 * ctx.line_height;
                        let vl = &ctx.visual_lines[start_v];
                        let line_content = self.buffer.line(vl.logical_line);

                        // Use calculate_segment_geometry to compute match position and width
                        let (x_start, match_width) = calculate_segment_geometry(
                            line_content,
                            vl.start_col,
                            search_match.col,
                            search_match.col + query_len,
                            ctx.gutter_width + 5.0,
                            ctx.full_char_width,
                            ctx.char_width,
                        );
                        let x_start = x_start - ctx.horizontal_scroll_offset;
                        let x_end = x_start + match_width;

                        frame.fill_rectangle(
                            Point::new(x_start, y + 2.0),
                            Size::new(x_end - x_start, ctx.line_height - 4.0),
                            highlight_color,
                        );
                    } else {
                        // Match spans multiple visual lines
                        for (v_idx, vl) in ctx
                            .visual_lines
                            .iter()
                            .enumerate()
                            .skip(start_v)
                            .take(end_v - start_v + 1)
                        {
                            let y = v_idx as f32 * ctx.line_height;

                            let match_start_col = search_match.col;
                            let match_end_col = search_match.col + query_len;

                            let sel_start_col = if v_idx == start_v {
                                match_start_col
                            } else {
                                vl.start_col
                            };
                            let sel_end_col = if v_idx == end_v {
                                match_end_col
                            } else {
                                vl.end_col
                            };

                            let line_content =
                                self.buffer.line(vl.logical_line);

                            let (x_start, sel_width) =
                                calculate_segment_geometry(
                                    line_content,
                                    vl.start_col,
                                    sel_start_col,
                                    sel_end_col,
                                    ctx.gutter_width + 5.0,
                                    ctx.full_char_width,
                                    ctx.char_width,
                                );
                            let x_start =
                                x_start - ctx.horizontal_scroll_offset;
                            let x_end = x_start + sel_width;

                            frame.fill_rectangle(
                                Point::new(x_start, y + 2.0),
                                Size::new(
                                    x_end - x_start,
                                    ctx.line_height - 4.0,
                                ),
                                highlight_color,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draws the selection highlight for a single cursor range.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    /// * `start` - Selection start (line, col)
    /// * `end` - Selection end (line, col), must be >= start
    fn draw_single_selection(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        start: (usize, usize),
        end: (usize, usize),
    ) {
        let selection_color = Color { r: 0.3, g: 0.5, b: 0.8, a: 0.3 };

        if start.0 == end.0 {
            // Single line selection - need to handle wrapped segments
            let start_visual = WrappingCalculator::logical_to_visual(
                ctx.visual_lines,
                start.0,
                start.1,
            );
            let end_visual = WrappingCalculator::logical_to_visual(
                ctx.visual_lines,
                end.0,
                end.1,
            );

            if let (Some(start_v), Some(end_v)) = (start_visual, end_visual) {
                if start_v == end_v {
                    // Selection within same visual line
                    let y = start_v as f32 * ctx.line_height;
                    let vl = &ctx.visual_lines[start_v];
                    let line_content = self.buffer.line(vl.logical_line);

                    let (x_start, sel_width) = calculate_segment_geometry(
                        line_content,
                        vl.start_col,
                        start.1,
                        end.1,
                        ctx.gutter_width + 5.0,
                        ctx.full_char_width,
                        ctx.char_width,
                    );
                    let x_start = x_start - ctx.horizontal_scroll_offset;
                    let x_end = x_start + sel_width;

                    frame.fill_rectangle(
                        Point::new(x_start, y + 2.0),
                        Size::new(x_end - x_start, ctx.line_height - 4.0),
                        selection_color,
                    );
                } else {
                    // Selection spans multiple visual lines (same logical line)
                    for (v_idx, vl) in ctx
                        .visual_lines
                        .iter()
                        .enumerate()
                        .skip(start_v)
                        .take(end_v - start_v + 1)
                    {
                        let y = v_idx as f32 * ctx.line_height;

                        let sel_start_col = if v_idx == start_v {
                            start.1
                        } else {
                            vl.start_col
                        };
                        let sel_end_col =
                            if v_idx == end_v { end.1 } else { vl.end_col };

                        let line_content = self.buffer.line(vl.logical_line);

                        let (x_start, sel_width) = calculate_segment_geometry(
                            line_content,
                            vl.start_col,
                            sel_start_col,
                            sel_end_col,
                            ctx.gutter_width + 5.0,
                            ctx.full_char_width,
                            ctx.char_width,
                        );
                        let x_start = x_start - ctx.horizontal_scroll_offset;
                        let x_end = x_start + sel_width;

                        frame.fill_rectangle(
                            Point::new(x_start, y + 2.0),
                            Size::new(x_end - x_start, ctx.line_height - 4.0),
                            selection_color,
                        );
                    }
                }
            }
        } else {
            // Multi-line selection
            let start_visual = WrappingCalculator::logical_to_visual(
                ctx.visual_lines,
                start.0,
                start.1,
            );
            let end_visual = WrappingCalculator::logical_to_visual(
                ctx.visual_lines,
                end.0,
                end.1,
            );

            if let (Some(start_v), Some(end_v)) = (start_visual, end_visual) {
                for (v_idx, vl) in ctx
                    .visual_lines
                    .iter()
                    .enumerate()
                    .skip(start_v)
                    .take(end_v - start_v + 1)
                {
                    let y = v_idx as f32 * ctx.line_height;

                    let sel_start_col =
                        if vl.logical_line == start.0 && v_idx == start_v {
                            start.1
                        } else {
                            vl.start_col
                        };

                    let sel_end_col =
                        if vl.logical_line == end.0 && v_idx == end_v {
                            end.1
                        } else {
                            vl.end_col
                        };

                    let line_content = self.buffer.line(vl.logical_line);

                    let (x_start, sel_width) = calculate_segment_geometry(
                        line_content,
                        vl.start_col,
                        sel_start_col,
                        sel_end_col,
                        ctx.gutter_width + 5.0,
                        ctx.full_char_width,
                        ctx.char_width,
                    );
                    let x_start = x_start - ctx.horizontal_scroll_offset;
                    let x_end = x_start + sel_width;

                    frame.fill_rectangle(
                        Point::new(x_start, y + 2.0),
                        Size::new(x_end - x_start, ctx.line_height - 4.0),
                        selection_color,
                    );
                }
            }
        }
    }

    /// Draws text selection highlights for all cursors.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    fn draw_selection_highlight(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
    ) {
        for cursor in self.cursors.iter() {
            if let Some((start, end)) = cursor.selection_range()
                && start != end
            {
                self.draw_single_selection(frame, ctx, start, end);
            }
        }
    }

    /// Draws the cursor (normal caret or IME preedit cursor).
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    fn draw_cursor(&self, frame: &mut canvas::Frame, ctx: &RenderContext) {
        // Cursor drawing logic (only when the editor has focus)
        // -------------------------------------------------------------------------
        // Core notes:
        // 1. Choose the drawing path based on whether IME preedit is present.
        // 2. Require both `is_focused()` (Iced focus) and `has_canvas_focus()` (internal focus)
        //    so the cursor is drawn only in the active editor, avoiding multiple cursors.
        // 3. Use `WrappingCalculator` to map logical (line, col) to visual (x, y)
        //    for correct cursor positioning with line wrapping.
        // -------------------------------------------------------------------------
        if self.show_cursor
            && self.cursor_visible
            && self.has_focus()
            && self.ime_preedit.is_some()
        {
            // [Branch A] IME preedit rendering mode
            // ---------------------------------------------------------------------
            // When the user is composing with an IME (e.g. pinyin before commit),
            // draw a preedit region instead of the normal caret, including:
            // - preedit background (highlighting the composing text)
            // - preedit text content (preedit.content)
            // - preedit selection (underline or selection background)
            // - preedit caret
            // ---------------------------------------------------------------------
            if let Some(cursor_visual) = WrappingCalculator::logical_to_visual(
                ctx.visual_lines,
                self.cursors.primary_position().0,
                self.cursors.primary_position().1,
            ) {
                let vl = &ctx.visual_lines[cursor_visual];
                let line_content = self.buffer.line(vl.logical_line);

                // Compute the preedit region start X
                // Use calculate_segment_geometry to ensure correct CJK width handling
                let (cursor_x_content, _) = calculate_segment_geometry(
                    line_content,
                    vl.start_col,
                    self.cursors.primary_position().1,
                    self.cursors.primary_position().1,
                    ctx.gutter_width + 5.0,
                    ctx.full_char_width,
                    ctx.char_width,
                );
                let cursor_x = cursor_x_content - ctx.horizontal_scroll_offset;
                let cursor_y = cursor_visual as f32 * ctx.line_height;

                if let Some(preedit) = self.ime_preedit.as_ref() {
                    let preedit_width = measure_text_width(
                        &preedit.content,
                        ctx.full_char_width,
                        ctx.char_width,
                    );

                    // 1. Draw preedit background (light translucent)
                    // This indicates the text is not committed yet
                    frame.fill_rectangle(
                        Point::new(cursor_x, cursor_y + 2.0),
                        Size::new(preedit_width, ctx.line_height - 4.0),
                        Color { r: 1.0, g: 1.0, b: 1.0, a: 0.08 },
                    );

                    // 2. Draw preedit selection (if any)
                    // IME may mark a selection inside preedit text (e.g. segmentation)
                    // The range uses UTF-8 byte indices, so slices must be safe
                    if let Some(range) = preedit.selection.as_ref()
                        && range.start != range.end
                    {
                        // Validate indices before slicing to prevent panic
                        if let Some((start, end)) = validate_selection_indices(
                            &preedit.content,
                            range.start,
                            range.end,
                        ) {
                            let selected_prefix = &preedit.content[..start];
                            let selected_text = &preedit.content[start..end];

                            let selection_x = cursor_x
                                + measure_text_width(
                                    selected_prefix,
                                    ctx.full_char_width,
                                    ctx.char_width,
                                );
                            let selection_w = measure_text_width(
                                selected_text,
                                ctx.full_char_width,
                                ctx.char_width,
                            );

                            frame.fill_rectangle(
                                Point::new(selection_x, cursor_y + 2.0),
                                Size::new(selection_w, ctx.line_height - 4.0),
                                Color { r: 0.3, g: 0.5, b: 0.8, a: 0.3 },
                            );
                        }
                    }

                    // 3. Draw preedit text itself
                    frame.fill_text(canvas::Text {
                        content: preedit.content.clone(),
                        position: Point::new(cursor_x, cursor_y + 2.0),
                        color: self.style.text_color,
                        size: ctx.font_size.into(),
                        font: ctx.font,
                        ..canvas::Text::default()
                    });

                    // 4. Draw bottom underline (IME state indicator)
                    frame.fill_rectangle(
                        Point::new(cursor_x, cursor_y + ctx.line_height - 3.0),
                        Size::new(preedit_width, 1.0),
                        self.style.text_color,
                    );

                    // 5. Draw preedit caret
                    // If IME provides a caret position (usually selection end), draw a thin bar
                    if let Some(range) = preedit.selection.as_ref() {
                        let caret_end = range.end.min(preedit.content.len());

                        // Validate caret position to avoid panic on invalid UTF-8 boundary
                        if caret_end <= preedit.content.len()
                            && preedit.content.is_char_boundary(caret_end)
                        {
                            let caret_prefix = &preedit.content[..caret_end];
                            let caret_x = cursor_x
                                + measure_text_width(
                                    caret_prefix,
                                    ctx.full_char_width,
                                    ctx.char_width,
                                );

                            frame.fill_rectangle(
                                Point::new(caret_x, cursor_y + 2.0),
                                Size::new(2.0, ctx.line_height - 4.0),
                                self.style.text_color,
                            );
                        }
                    }
                }
            }
        } else if self.show_cursor && self.cursor_visible && self.has_focus() {
            // [Branch B] Normal caret rendering mode
            // ---------------------------------------------------------------------
            // Draw a caret for every cursor in the set.
            // The primary cursor is drawn exactly like secondary ones — the viewport
            // follows the primary, but visually all carets look the same.
            // ---------------------------------------------------------------------
            for cursor in self.cursors.iter() {
                self.draw_single_caret(frame, ctx, cursor.position);
            }
        }
    }

    /// Draws a single 2px vertical caret at the given logical (line, col) position.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    /// * `position` - Logical cursor position (line, col)
    fn draw_single_caret(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        position: (usize, usize),
    ) {
        // Map logical cursor position (line, col) to visual line index
        if let Some(cursor_visual) = WrappingCalculator::logical_to_visual(
            ctx.visual_lines,
            position.0,
            position.1,
        ) {
            let vl = &ctx.visual_lines[cursor_visual];
            let line_content = self.buffer.line(vl.logical_line);

            // Compute exact caret X position
            let (cursor_x_content, _) = calculate_segment_geometry(
                line_content,
                vl.start_col,
                position.1,
                position.1,
                ctx.gutter_width + 5.0,
                ctx.full_char_width,
                ctx.char_width,
            );
            let cursor_x = cursor_x_content - ctx.horizontal_scroll_offset;
            let cursor_y = cursor_visual as f32 * ctx.line_height;

            // Draw standard caret (2px vertical bar)
            frame.fill_rectangle(
                Point::new(cursor_x, cursor_y + 2.0),
                Size::new(2.0, ctx.line_height - 4.0),
                self.style.text_color,
            );
        }
    }

    /// Checks if the editor has focus (both Iced focus and internal canvas focus).
    ///
    /// # Returns
    ///
    /// `true` if the editor has both Iced focus and internal canvas focus and is not focus-locked; `false` otherwise
    pub(crate) fn has_focus(&self) -> bool {
        // Check if this editor has Iced focus
        let focused_id =
            super::FOCUSED_EDITOR_ID.load(std::sync::atomic::Ordering::Relaxed);
        focused_id == self.editor_id
            && self.has_canvas_focus
            && !self.focus_locked
    }

    /// Handles keyboard shortcut combinations (Ctrl+C, Ctrl+Z, etc.).
    ///
    /// This implementation includes focus chain management for Tab and Shift+Tab
    /// navigation between editors.
    ///
    /// # Arguments
    ///
    /// * `key` - The keyboard key that was pressed
    /// * `modifiers` - The keyboard modifiers (Ctrl, Shift, Alt, etc.)
    ///
    /// # Returns
    ///
    /// `Some(Action<Message>)` if a shortcut was matched, `None` otherwise
    fn handle_keyboard_shortcuts(
        &self,
        key: &keyboard::Key,
        modifiers: &keyboard::Modifiers,
    ) -> Option<Action<Message>> {
        // Shift+Tab: focus navigation backward (Tab alone inserts indentation)
        if matches!(key, keyboard::Key::Named(keyboard::key::Named::Tab))
            && modifiers.shift()
            && !self.search_state.is_open
        {
            return Some(
                Action::publish(Message::FocusNavigationShiftTab).and_capture(),
            );
        }

        // Handle Ctrl+C / Ctrl+Insert (copy)
        if (modifiers.command()
            && matches!(key, keyboard::Key::Character(c) if c.as_str() == "c"))
            || (modifiers.command()
                && matches!(
                    key,
                    keyboard::Key::Named(keyboard::key::Named::Insert)
                ))
        {
            return Some(Action::publish(Message::Copy).and_capture());
        }

        // Select all (Ctrl/Cmd+A)
        if modifiers.command()
            && matches!(key, keyboard::Key::Character(a) if a.as_str() == "a")
        {
            return Some(Action::publish(Message::SelectAll).and_capture());
        }

        // Handle Ctrl+Z (undo)
        if modifiers.command()
            && matches!(key, keyboard::Key::Character(z) if z.as_str() == "z")
        {
            return Some(Action::publish(Message::Undo).and_capture());
        }

        // Handle Ctrl+Y (redo)
        if modifiers.command()
            && matches!(key, keyboard::Key::Character(y) if y.as_str() == "y")
        {
            return Some(Action::publish(Message::Redo).and_capture());
        }

        // Handle Ctrl+F (open search)
        if modifiers.command()
            && matches!(key, keyboard::Key::Character(f) if f.as_str() == "f")
            && self.search_replace_enabled
        {
            return Some(Action::publish(Message::OpenSearch).and_capture());
        }

        // Handle Ctrl+H (open search and replace)
        if modifiers.command()
            && matches!(key, keyboard::Key::Character(h) if h.as_str() == "h")
            && self.search_replace_enabled
        {
            return Some(
                Action::publish(Message::OpenSearchReplace).and_capture(),
            );
        }

        // Handle Escape — handled by CloseSearch message, which also collapses multi-cursor
        if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
            return Some(Action::publish(Message::CloseSearch).and_capture());
        }

        // Handle Ctrl+D (select next occurrence)
        if modifiers.command()
            && matches!(key, keyboard::Key::Character(d) if d.as_str() == "d")
        {
            return Some(
                Action::publish(Message::SelectNextOccurrence).and_capture(),
            );
        }

        // Handle Ctrl+Alt+Up (add cursor above)
        if modifiers.command()
            && modifiers.alt()
            && matches!(
                key,
                keyboard::Key::Named(keyboard::key::Named::ArrowUp)
            )
        {
            return Some(
                Action::publish(Message::AddCursorAbove).and_capture(),
            );
        }

        // Handle Ctrl+Alt+Down (add cursor below)
        if modifiers.command()
            && modifiers.alt()
            && matches!(
                key,
                keyboard::Key::Named(keyboard::key::Named::ArrowDown)
            )
        {
            return Some(
                Action::publish(Message::AddCursorBelow).and_capture(),
            );
        }

        // Handle Tab (cycle forward in search dialog if open)
        if matches!(key, keyboard::Key::Named(keyboard::key::Named::Tab))
            && self.search_state.is_open
        {
            if modifiers.shift() {
                // Shift+Tab: cycle backward
                return Some(
                    Action::publish(Message::SearchDialogShiftTab)
                        .and_capture(),
                );
            } else {
                // Tab: cycle forward
                return Some(
                    Action::publish(Message::SearchDialogTab).and_capture(),
                );
            }
        }

        // Handle F3 (find next) and Shift+F3 (find previous)
        if matches!(key, keyboard::Key::Named(keyboard::key::Named::F3))
            && self.search_replace_enabled
        {
            if modifiers.shift() {
                return Some(
                    Action::publish(Message::FindPrevious).and_capture(),
                );
            } else {
                return Some(Action::publish(Message::FindNext).and_capture());
            }
        }

        // Handle Ctrl+V / Shift+Insert (paste) - read clipboard and send paste message
        if (modifiers.command()
            && matches!(key, keyboard::Key::Character(v) if v.as_str() == "v"))
            || (modifiers.shift()
                && matches!(
                    key,
                    keyboard::Key::Named(keyboard::key::Named::Insert)
                ))
        {
            // Return an action that requests clipboard read
            return Some(Action::publish(Message::Paste(String::new())));
        }

        // Handle Ctrl+Home (go to start of document)
        if modifiers.command()
            && matches!(key, keyboard::Key::Named(keyboard::key::Named::Home))
        {
            return Some(Action::publish(Message::CtrlHome).and_capture());
        }

        // Handle Ctrl+End (go to end of document)
        if modifiers.command()
            && matches!(key, keyboard::Key::Named(keyboard::key::Named::End))
        {
            return Some(Action::publish(Message::CtrlEnd).and_capture());
        }

        // Handle Shift+Delete (delete selection)
        if modifiers.shift()
            && matches!(key, keyboard::Key::Named(keyboard::key::Named::Delete))
        {
            return Some(
                Action::publish(Message::DeleteSelection).and_capture(),
            );
        }

        // Code folding shortcuts (only when folding is enabled).
        if self.folding_enabled {
            // Ctrl+. : toggle the fold of the block at the cursor.
            if modifiers.command()
                && matches!(key, keyboard::Key::Character(c) if c.as_str() == ".")
            {
                return Some(
                    Action::publish(Message::ToggleFoldAtCursor).and_capture(),
                );
            }

            // Ctrl+K : fold all blocks.
            if modifiers.command()
                && !modifiers.shift()
                && matches!(key, keyboard::Key::Character(c) if c.as_str() == "k")
            {
                return Some(Action::publish(Message::FoldAll).and_capture());
            }

            // Ctrl+J : unfold all blocks.
            if modifiers.command()
                && !modifiers.shift()
                && matches!(key, keyboard::Key::Character(c) if c.as_str() == "j")
            {
                return Some(Action::publish(Message::UnfoldAll).and_capture());
            }
        }

        None
    }

    /// Handles character input and special navigation keys.
    ///
    /// This implementation includes focus event propagation and focus chain management
    /// for proper focus handling without mouse bounds checking.
    ///
    /// # Arguments
    ///
    /// * `key` - The keyboard key that was pressed
    /// * `modifiers` - The keyboard modifiers (Ctrl, Shift, Alt, etc.)
    /// * `text` - Optional text content from the keyboard event
    ///
    /// # Returns
    ///
    /// `Some(Action<Message>)` if input should be processed, `None` otherwise
    #[allow(clippy::unused_self)]
    fn handle_character_input(
        &self,
        key: &keyboard::Key,
        modifiers: &keyboard::Modifiers,
        text: Option<&str>,
    ) -> Option<Action<Message>> {
        // Early exit: Only process character input when editor has focus
        // This prevents focus stealing where characters typed in other input fields
        // appear in the editor
        if !self.has_focus() {
            return None;
        }

        // PRIORITY 1: Check if 'text' field has valid printable character
        // This handles:
        // - Numpad keys with NumLock ON (key=Named(ArrowDown), text=Some("2"))
        // - Regular typing with shift, accents, international layouts
        if let Some(text_content) = text
            && !text_content.is_empty()
            && !modifiers.control()
            && !modifiers.command()
            && !modifiers.alt()
        {
            // Check if it's a printable character (not a control character)
            // This filters out Enter (\n), Tab (\t), Delete (U+007F), etc.
            if let Some(first_char) = text_content.chars().next()
                && !first_char.is_control()
            {
                return Some(
                    Action::publish(Message::CharacterInput(first_char))
                        .and_capture(),
                );
            }
        }

        // PRIORITY 2: Handle special named keys (navigation, editing)
        // These are only processed if text didn't contain a printable character
        let message = match key {
            keyboard::Key::Named(keyboard::key::Named::Backspace) => {
                Some(Message::Backspace)
            }
            keyboard::Key::Named(keyboard::key::Named::Delete) => {
                Some(Message::Delete)
            }
            keyboard::Key::Named(keyboard::key::Named::Enter) => {
                Some(Message::Enter)
            }
            keyboard::Key::Named(keyboard::key::Named::Tab) => {
                // Handle Tab for focus navigation or text insertion
                // This implements focus event propagation and focus chain management
                if modifiers.shift() {
                    // Shift+Tab: focus navigation backward through widget hierarchy
                    Some(Message::FocusNavigationShiftTab)
                } else {
                    // Regular Tab: check if search dialog is open
                    if self.search_state.is_open {
                        Some(Message::SearchDialogTab)
                    } else {
                        // Insert 4 spaces for Tab when not in search dialog
                        Some(Message::Tab)
                    }
                }
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                Some(Message::ArrowKey(ArrowDirection::Up, modifiers.shift()))
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                Some(Message::ArrowKey(ArrowDirection::Down, modifiers.shift()))
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                Some(Message::ArrowKey(ArrowDirection::Left, modifiers.shift()))
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowRight) => Some(
                Message::ArrowKey(ArrowDirection::Right, modifiers.shift()),
            ),
            keyboard::Key::Named(keyboard::key::Named::PageUp) => {
                Some(Message::PageUp)
            }
            keyboard::Key::Named(keyboard::key::Named::PageDown) => {
                Some(Message::PageDown)
            }
            keyboard::Key::Named(keyboard::key::Named::Home) => {
                Some(Message::Home(modifiers.shift()))
            }
            keyboard::Key::Named(keyboard::key::Named::End) => {
                Some(Message::End(modifiers.shift()))
            }
            // PRIORITY 3: Fallback to extracting from 'key' if text was empty/control char
            // This handles edge cases where text field is not populated
            _ => {
                if !modifiers.control()
                    && !modifiers.command()
                    && !modifiers.alt()
                    && let keyboard::Key::Character(c) = key
                    && !c.is_empty()
                {
                    return c
                        .chars()
                        .next()
                        .map(Message::CharacterInput)
                        .map(|msg| Action::publish(msg).and_capture());
                }
                None
            }
        };

        message.map(|msg| Action::publish(msg).and_capture())
    }

    /// Handles keyboard events with focus event propagation through widget hierarchy.
    ///
    /// This implementation completes focus handling without mouse bounds checking
    /// and ensures proper focus chain management.
    ///
    /// # Arguments
    ///
    /// * `key` - The keyboard key that was pressed
    /// * `modifiers` - The keyboard modifiers (Ctrl, Shift, Alt, etc.)
    /// * `text` - Optional text content from the keyboard event
    /// * `bounds` - The rectangle bounds of the canvas widget (unused in this implementation)
    /// * `cursor` - The current mouse cursor position and status (unused in this implementation)
    ///
    /// # Returns
    ///
    /// `Some(Action<Message>)` if the event was handled, `None` otherwise
    fn handle_keyboard_event(
        &self,
        key: &keyboard::Key,
        modifiers: &keyboard::Modifiers,
        text: &Option<iced::advanced::graphics::core::SmolStr>,
        _bounds: Rectangle,
        _cursor: &mouse::Cursor,
    ) -> Option<Action<Message>> {
        // Early exit: Check if editor has focus and is not focus-locked
        // This prevents focus stealing where keyboard input meant for other widgets
        // is incorrectly processed by this editor during focus transitions
        if !self.has_focus() || self.focus_locked {
            return None;
        }

        // Skip if IME is active (unless Ctrl/Command is pressed)
        if self.ime_preedit.is_some()
            && !(modifiers.control() || modifiers.command())
        {
            return None;
        }

        // Try keyboard shortcuts first
        if let Some(action) = self.handle_keyboard_shortcuts(key, modifiers) {
            return Some(action);
        }

        // Handle character input and special keys
        // Convert Option<SmolStr> to Option<&str>
        let text_str = text.as_ref().map(|s| s.as_str());
        self.handle_character_input(key, modifiers, text_str)
    }

    /// Handles mouse events (button presses, movement, releases).
    ///
    /// # Arguments
    ///
    /// * `event` - The mouse event to handle
    /// * `bounds` - The rectangle bounds of the canvas widget
    /// * `cursor` - The current mouse cursor position and status
    ///
    /// # Returns
    ///
    /// `Some(Action<Message>)` if the event was handled, `None` otherwise
    #[allow(clippy::unused_self)]
    /// Returns the logical line of the fold header whose chevron is at `point`,
    /// if any.
    ///
    /// Returns `None` when folding is disabled, when the point is outside the
    /// fold margin, or when the targeted line is not a fold header.
    ///
    /// # Arguments
    ///
    /// * `point` - The click position in canvas coordinates
    pub(crate) fn fold_header_at_point(&self, point: Point) -> Option<usize> {
        if !self.folding_enabled {
            return None;
        }

        // The fold margin is the strip between the line-number area and the text.
        let margin_start = self.line_number_gutter_width();
        if point.x < margin_start || point.x >= self.gutter_width() {
            return None;
        }

        let visual_line_idx = (point.y / self.line_height) as usize;
        let visual_lines = self.visual_lines_cached(self.viewport_width);
        let visual_line = visual_lines.get(visual_line_idx)?;
        if !visual_line.is_first_segment() {
            return None;
        }

        let regions = self.foldable_regions();
        folding::is_fold_header(&regions, visual_line.logical_line)
            .then_some(visual_line.logical_line)
    }

    fn handle_mouse_event(
        &self,
        event: &mouse::Event,
        bounds: Rectangle,
        cursor: &mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                cursor.position_in(bounds).map(|position| {
                    // Clicking a fold chevron toggles the block instead of
                    // moving the caret.
                    if let Some(header) = self.fold_header_at_point(position) {
                        return Action::publish(Message::ToggleFold(header))
                            .and_capture();
                    }

                    // Check for Ctrl (or Command on macOS) + Click
                    #[cfg(target_os = "macos")]
                    let is_jump_click = self.modifiers.get().command();
                    #[cfg(not(target_os = "macos"))]
                    let is_jump_click = self.modifiers.get().control();

                    if is_jump_click {
                        return Action::publish(Message::JumpClick(position));
                    }

                    // Alt+Click: add a new cursor at the clicked position
                    if self.modifiers.get().alt() {
                        return Action::publish(Message::AltClick(position))
                            .and_capture();
                    }

                    let count = self.classify_click(position);
                    let msg = match count {
                        2 => Message::DoubleClick(position),
                        3 => Message::TripleClick(position),
                        _ => Message::MouseClick(position),
                    };
                    Action::publish(msg)
                })
                .or_else(|| {
                    // Clicking outside the editor blurs it (releases the caret).
                    self.has_canvas_focus
                        .then(|| Action::publish(Message::CanvasFocusLost))
                })
            }
            mouse::Event::CursorMoved { .. } => {
                cursor.position_in(bounds).map(|position| {
                    if self.is_dragging {
                        // Handle mouse drag for selection only when cursor is within bounds
                        Action::publish(Message::MouseDrag(position))
                            .and_capture()
                    } else {
                        // Forward hover events when not dragging to enable LSP hover.
                        Action::publish(Message::MouseHover(position))
                    }
                })
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                // Only handle mouse release when cursor is within bounds
                // This prevents capturing events meant for other widgets
                if cursor.is_over(bounds) {
                    Some(Action::publish(Message::MouseRelease).and_capture())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Handles IME (Input Method Editor) events for complex text input.
    ///
    /// # Arguments
    ///
    /// * `event` - The IME event to handle
    /// * `bounds` - The rectangle bounds of the canvas widget
    /// * `cursor` - The current mouse cursor position and status
    ///
    /// # Returns
    ///
    /// `Some(Action<Message>)` if the event was handled, `None` otherwise
    fn handle_ime_event(
        &self,
        event: &input_method::Event,
        _bounds: Rectangle,
        _cursor: &mouse::Cursor,
    ) -> Option<Action<Message>> {
        // Early exit: Check if editor has focus and is not focus-locked
        // This prevents focus stealing where IME events meant for other widgets
        // are incorrectly processed by this editor during focus transitions
        if !self.has_focus() || self.focus_locked {
            return None;
        }

        // IME event handling
        // ---------------------------------------------------------------------
        // Core mapping: convert Iced IME events into editor Messages
        //
        // Flow:
        // 1. Opened: IME activated (e.g. switching input method). Clear old preedit state.
        // 2. Preedit: User is composing (e.g. typing "nihao" before commit).
        //    - content: current candidate text
        //    - selection: selection range within the text, in bytes
        // 3. Commit: User confirms a candidate and commits text into the buffer.
        // 4. Closed: IME closed or lost focus.
        //
        // Safety checks:
        // - handle only when `focused_id` matches this editor ID
        // - handle only when `has_canvas_focus` is true
        // This ensures IME events are not delivered to the wrong widget.
        // ---------------------------------------------------------------------
        let message = match event {
            input_method::Event::Opened => Message::ImeOpened,
            input_method::Event::Preedit(content, selection) => {
                Message::ImePreedit(content.clone(), selection.clone())
            }
            input_method::Event::Commit(content) => {
                Message::ImeCommit(content.clone())
            }
            input_method::Event::Closed => Message::ImeClosed,
        };

        Some(Action::publish(message).and_capture())
    }
}

impl CodeEditor {
    /// Draws underlines for jumpable links when modifier is held.
    fn draw_jump_link_highlight(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) {
        #[cfg(target_os = "macos")]
        let modifier_active = self.modifiers.get().command();
        #[cfg(not(target_os = "macos"))]
        let modifier_active = self.modifiers.get().control();

        if !modifier_active {
            return;
        }

        let Some(point) = cursor.position_in(bounds) else {
            return;
        };

        if let Some((line, col)) = self.calculate_cursor_from_point(point) {
            let line_content = self.buffer.line(line);

            let start_col = Self::word_start_in_line(line_content, col);
            let end_col = Self::word_end_in_line(line_content, col);

            if start_col >= end_col {
                return;
            }

            // Find the first visual line for this logical line
            if let Some(mut idx) =
                WrappingCalculator::logical_to_visual(ctx.visual_lines, line, 0)
            {
                // Iterate all visual lines belonging to this logical line
                while idx < ctx.visual_lines.len() {
                    let visual_line = &ctx.visual_lines[idx];
                    if visual_line.logical_line != line {
                        break;
                    }

                    // Check intersection
                    let seg_start = visual_line.start_col.max(start_col);
                    let seg_end = visual_line.end_col.min(end_col);

                    if seg_start < seg_end {
                        let (x, width) = calculate_segment_geometry(
                            line_content,
                            visual_line.start_col,
                            seg_start,
                            seg_end,
                            ctx.gutter_width + 5.0
                                - ctx.horizontal_scroll_offset,
                            ctx.full_char_width,
                            ctx.char_width,
                        );

                        let y = idx as f32 * ctx.line_height + ctx.line_height; // Underline at bottom

                        // Draw underline
                        let path = canvas::Path::line(
                            Point::new(x, y),
                            Point::new(x + width, y),
                        );

                        frame.stroke(
                            &path,
                            canvas::Stroke::default()
                                .with_color(self.style.text_color) // Use text color or link color
                                .with_width(1.0),
                        );
                    }

                    idx += 1;
                }
            }
        }
    }
}

impl canvas::Program<Message> for CodeEditor {
    type State = ();

    /// Renders the code editor's visual elements on the canvas, including text layout, syntax highlighting,
    /// cursor positioning, and other graphical aspects.
    ///
    /// # Arguments
    ///
    /// * `state` - The current state of the canvas
    /// * `renderer` - The renderer used for drawing
    /// * `theme` - The theme for styling
    /// * `bounds` - The rectangle bounds of the canvas
    /// * `cursor` - The mouse cursor position
    ///
    /// # Returns
    ///
    /// A vector of `Geometry` objects representing the drawn elements
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        // Font metrics may have been computed before the configured font was
        // loaded into the renderer's font system (e.g. editors built during
        // application boot measure against a fallback font). Re-measure here
        // and redraw if the live value diverges from the cached one, so token
        // x-offsets always match the font actually used for rendering.
        let fresh_char_width = self.measure_single_char_width("a");
        let fresh_full_char_width = self.measure_single_char_width("汉");
        let live_char_width =
            if fresh_char_width.is_finite() && fresh_char_width > 0.0 {
                fresh_char_width
            } else {
                self.char_width
            };
        let live_full_char_width = if fresh_full_char_width.is_finite()
            && fresh_full_char_width > 0.0
        {
            fresh_full_char_width
        } else {
            self.full_char_width
        };
        if (live_char_width - self.char_width).abs() > 0.01
            || (live_full_char_width - self.full_char_width).abs() > 0.01
        {
            self.content_cache.clear();
            self.overlay_cache.clear();
        }

        let visual_lines: Rc<Vec<VisualLine>> =
            self.visual_lines_cached(bounds.width);

        // Prefer the tracked viewport height when available, but fall back to
        // the current bounds during initial layout when viewport metrics have
        // not been populated yet.
        let effective_viewport_height = if self.viewport_height > 0.0 {
            self.viewport_height
        } else {
            bounds.height
        };
        let first_visible_line =
            (self.viewport_scroll / self.line_height).floor() as usize;
        let visible_lines_count =
            (effective_viewport_height / self.line_height).ceil() as usize + 2;
        let last_visible_line =
            (first_visible_line + visible_lines_count).min(visual_lines.len());

        let (start_idx, end_idx) =
            if self.cache_window_end_line > self.cache_window_start_line {
                let s = self.cache_window_start_line.min(visual_lines.len());
                let e = self.cache_window_end_line.min(visual_lines.len());
                (s, e)
            } else {
                (first_visible_line, last_visible_line)
            };

        // Split rendering into two cached layers:
        // - content: expensive, mostly static text/gutter rendering
        // - overlay: frequently changing highlights/cursor/IME
        //
        // This keeps selection dragging and cursor blinking smooth by avoiding
        // invalidation of the text layer on every overlay update.
        let visual_lines_for_content = visual_lines.clone();
        let content_geometry =
            self.content_cache.draw(renderer, bounds.size(), |frame| {
                // syntect initialization is relatively expensive; keep it global.
                let syntax_set = SYNTAX_SET.get_or_init(|| {
                    #[cfg(feature = "two-face")]
                    {
                        two_face::syntax::extra_newlines()
                    }
                    #[cfg(not(feature = "two-face"))]
                    {
                        SyntaxSet::load_defaults_newlines()
                    }
                });
                let theme_set = THEME_SET.get_or_init(ThemeSet::load_defaults);
                let syntax_theme = theme_set
                    .themes
                    .get("base16-ocean.dark")
                    .or_else(|| theme_set.themes.values().next());

                // Normalize common language aliases/extensions used by consumers.
                let syntax_ref = match self.syntax.as_str() {
                    "python" => syntax_set.find_syntax_by_extension("py"),
                    "rust" => syntax_set.find_syntax_by_extension("rs"),
                    "javascript" => syntax_set.find_syntax_by_extension("js"),
                    "htm" => syntax_set.find_syntax_by_extension("html"),
                    "svg" => syntax_set.find_syntax_by_extension("xml"),
                    "markdown" => syntax_set.find_syntax_by_extension("md"),
                    "text" => Some(syntax_set.find_syntax_plain_text()),
                    _ => syntax_set
                        .find_syntax_by_extension(self.syntax.as_str()),
                }
                .or(Some(syntax_set.find_syntax_plain_text()));

                let ctx = RenderContext {
                    visual_lines: visual_lines_for_content.as_ref(),
                    bounds_width: bounds.width,
                    gutter_width: self.gutter_width(),
                    line_height: self.line_height,
                    font_size: self.font_size,
                    full_char_width: live_full_char_width,
                    char_width: live_char_width,
                    font: self.font,
                    horizontal_scroll_offset: self.horizontal_scroll_offset,
                };

                // Clip code text to the code area (right of gutter) so that
                // horizontal scrolling cannot cause text to bleed into the gutter.
                // Note: iced renders ALL text on top of ALL geometry, so a
                // fill_rectangle cannot mask text bleed — with_clip is required.
                let code_clip = Rectangle {
                    x: ctx.gutter_width,
                    y: 0.0,
                    width: (bounds.width - ctx.gutter_width).max(0.0),
                    height: bounds.height,
                };
                frame.with_clip(code_clip, |f| {
                    for (idx, visual_line) in visual_lines_for_content
                        .iter()
                        .enumerate()
                        .skip(start_idx)
                        .take(end_idx.saturating_sub(start_idx))
                    {
                        let y = idx as f32 * self.line_height;
                        self.draw_text_with_syntax_highlighting(
                            f,
                            &ctx,
                            visual_line,
                            y,
                            syntax_ref,
                            syntax_set,
                            syntax_theme,
                        );
                        self.draw_fold_collapsed_marker(
                            f,
                            &ctx,
                            visual_line,
                            y,
                        );
                    }
                });

                // Draw line numbers in the gutter (no clip — fixed position)
                for (idx, visual_line) in visual_lines_for_content
                    .iter()
                    .enumerate()
                    .skip(start_idx)
                    .take(end_idx.saturating_sub(start_idx))
                {
                    let y = idx as f32 * self.line_height;
                    self.draw_line_numbers(frame, &ctx, visual_line, y);
                }
            });

        let visual_lines_for_overlay = visual_lines;
        let overlay_geometry =
            self.overlay_cache.draw(renderer, bounds.size(), |frame| {
                // The overlay layer shares the same visual lines, but draws only
                // elements that change without modifying the buffer content.
                let ctx = RenderContext {
                    visual_lines: visual_lines_for_overlay.as_ref(),
                    bounds_width: bounds.width,
                    gutter_width: self.gutter_width(),
                    line_height: self.line_height,
                    font_size: self.font_size,
                    full_char_width: live_full_char_width,
                    char_width: live_char_width,
                    font: self.font,
                    horizontal_scroll_offset: self.horizontal_scroll_offset,
                };

                for (idx, visual_line) in visual_lines_for_overlay
                    .iter()
                    .enumerate()
                    .skip(start_idx)
                    .take(end_idx.saturating_sub(start_idx))
                {
                    let y = idx as f32 * self.line_height;
                    self.draw_current_line_highlight(
                        frame,
                        &ctx,
                        visual_line,
                        y,
                    );
                }

                self.draw_search_highlights(frame, &ctx, start_idx, end_idx);
                self.draw_selection_highlight(frame, &ctx);
                self.draw_jump_link_highlight(frame, &ctx, bounds, _cursor);
                self.draw_cursor(frame, &ctx);
            });

        vec![content_geometry, overlay_geometry]
    }

    /// Handles Canvas trait events, specifically keyboard input events and focus management for the code editor widget.
    ///
    /// # Arguments
    ///
    /// * `_state` - The mutable state of the canvas (unused in this implementation)
    /// * `event` - The input event to handle, such as keyboard presses
    /// * `bounds` - The rectangle bounds of the canvas widget
    /// * `cursor` - The current mouse cursor position and status
    ///
    /// # Returns
    ///
    /// An optional `Action<Message>` to perform, such as sending a message or redrawing the canvas
    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                self.modifiers.set(*modifiers);
                None
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) => {
                self.modifiers.set(*modifiers);
                self.handle_keyboard_event(
                    key, modifiers, text, bounds, &cursor,
                )
            }
            Event::Keyboard(keyboard::Event::KeyReleased {
                modifiers, ..
            }) => {
                self.modifiers.set(*modifiers);
                None
            }
            Event::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event, bounds, &cursor)
            }
            Event::InputMethod(ime_event) => {
                self.handle_ime_event(ime_event, bounds, &cursor)
            }
            _ => None,
        }
    }
}

/// Validates that the selection indices fall on valid UTF-8 character boundaries
/// to prevent panics during string slicing.
///
/// # Arguments
///
/// * `content` - The string content to check against
/// * `start` - The start byte index
/// * `end` - The end byte index
///
/// # Returns
///
/// `Some((start, end))` if indices are valid, `None` otherwise.
fn validate_selection_indices(
    content: &str,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let len = content.len();
    // Clamp indices to content length
    let start = start.min(len);
    let end = end.min(len);

    // Ensure start is not greater than end
    if start > end {
        return None;
    }

    // Verify that indices fall on valid UTF-8 character boundaries
    if content.is_char_boundary(start) && content.is_char_boundary(end) {
        Some((start, end))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_editor::{CHAR_WIDTH, FONT_SIZE, compare_floats};
    use std::cmp::Ordering;

    #[test]
    fn test_calculate_segment_geometry_ascii() {
        // "Hello World"
        // "Hello " (6 chars) -> prefix
        // "World" (5 chars) -> segment
        // width("Hello ") = 6 * CHAR_WIDTH
        // width("World") = 5 * CHAR_WIDTH
        let content = "Hello World";
        let (x, w) = calculate_segment_geometry(
            content, 0, 6, 11, 0.0, FONT_SIZE, CHAR_WIDTH,
        );

        let expected_x = CHAR_WIDTH * 6.0;
        let expected_w = CHAR_WIDTH * 5.0;

        assert_eq!(
            compare_floats(x, expected_x),
            Ordering::Equal,
            "X position mismatch for ASCII"
        );
        assert_eq!(
            compare_floats(w, expected_w),
            Ordering::Equal,
            "Width mismatch for ASCII"
        );
    }

    #[test]
    fn test_calculate_segment_geometry_cjk() {
        // "你好世界"
        // "你好" (2 chars) -> prefix
        // "世界" (2 chars) -> segment
        // width("你好") = 2 * FONT_SIZE
        // width("世界") = 2 * FONT_SIZE
        let content = "你好世界";
        let (x, w) = calculate_segment_geometry(
            content, 0, 2, 4, 10.0, FONT_SIZE, CHAR_WIDTH,
        );

        let expected_x = 10.0 + FONT_SIZE * 2.0;
        let expected_w = FONT_SIZE * 2.0;

        assert_eq!(
            compare_floats(x, expected_x),
            Ordering::Equal,
            "X position mismatch for CJK"
        );
        assert_eq!(
            compare_floats(w, expected_w),
            Ordering::Equal,
            "Width mismatch for CJK"
        );
    }

    #[test]
    fn test_calculate_segment_geometry_mixed() {
        // "Hi你好"
        // "Hi" (2 chars) -> prefix
        // "你好" (2 chars) -> segment
        // width("Hi") = 2 * CHAR_WIDTH
        // width("你好") = 2 * FONT_SIZE
        let content = "Hi你好";
        let (x, w) = calculate_segment_geometry(
            content, 0, 2, 4, 0.0, FONT_SIZE, CHAR_WIDTH,
        );

        let expected_x = CHAR_WIDTH * 2.0;
        let expected_w = FONT_SIZE * 2.0;

        assert_eq!(
            compare_floats(x, expected_x),
            Ordering::Equal,
            "X position mismatch for mixed content"
        );
        assert_eq!(
            compare_floats(w, expected_w),
            Ordering::Equal,
            "Width mismatch for mixed content"
        );
    }

    #[test]
    fn test_calculate_segment_geometry_empty_range() {
        let content = "Hello";
        let (x, w) = calculate_segment_geometry(
            content, 0, 0, 0, 0.0, FONT_SIZE, CHAR_WIDTH,
        );
        assert!((x - 0.0).abs() < f32::EPSILON);
        assert!((w - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_segment_geometry_with_visual_offset() {
        // content: "0123456789"
        // visual_start_col: 2 (starts at '2')
        // segment: "34" (indices 3 to 5)
        // prefix: from visual start (2) to segment start (3) -> "2" (length 1)
        // prefix width: 1 * CHAR_WIDTH
        // segment width: 2 * CHAR_WIDTH
        let content = "0123456789";
        let (x, w) = calculate_segment_geometry(
            content, 2, 3, 5, 5.0, FONT_SIZE, CHAR_WIDTH,
        );

        let expected_x = 5.0 + CHAR_WIDTH * 1.0;
        let expected_w = CHAR_WIDTH * 2.0;

        assert_eq!(
            compare_floats(x, expected_x),
            Ordering::Equal,
            "X position mismatch with visual offset"
        );
        assert_eq!(
            compare_floats(w, expected_w),
            Ordering::Equal,
            "Width mismatch with visual offset"
        );
    }

    #[test]
    fn test_calculate_segment_geometry_out_of_bounds() {
        // Content length is 5 ("Hello")
        // Request start at 10, end at 15
        // visual_start 0
        // Prefix should consume whole string ("Hello") and stop.
        // Segment should be empty.
        let content = "Hello";
        let (x, w) = calculate_segment_geometry(
            content, 0, 10, 15, 0.0, FONT_SIZE, CHAR_WIDTH,
        );

        let expected_x = CHAR_WIDTH * 5.0; // Width of "Hello"
        let expected_w = 0.0;

        assert_eq!(
            compare_floats(x, expected_x),
            Ordering::Equal,
            "X position mismatch for out of bounds start"
        );
        assert!(
            (w - expected_w).abs() < f32::EPSILON,
            "Width should be 0 for out of bounds segment"
        );
    }

    #[test]
    fn test_calculate_segment_geometry_special_chars() {
        // Emoji "👋" (width > 1 => FONT_SIZE)
        // Tab "\t" (width = 4 * CHAR_WIDTH)
        let content = "A👋\tB";
        // Measure "👋" (index 1 to 2)
        // Indices in chars: 'A' (0), '👋' (1), '\t' (2), 'B' (3)

        // Segment covering Emoji
        let (x, w) = calculate_segment_geometry(
            content, 0, 1, 2, 0.0, FONT_SIZE, CHAR_WIDTH,
        );
        let expected_x_emoji = CHAR_WIDTH; // 'A'
        let expected_w_emoji = FONT_SIZE; // '👋'

        assert_eq!(
            compare_floats(x, expected_x_emoji),
            Ordering::Equal,
            "X pos for emoji"
        );
        assert_eq!(
            compare_floats(w, expected_w_emoji),
            Ordering::Equal,
            "Width for emoji"
        );

        // Segment covering Tab
        let (x_tab, w_tab) = calculate_segment_geometry(
            content, 0, 2, 3, 0.0, FONT_SIZE, CHAR_WIDTH,
        );
        let expected_x_tab = CHAR_WIDTH + FONT_SIZE; // 'A' + '👋'
        let expected_w_tab =
            CHAR_WIDTH * crate::canvas_editor::TAB_WIDTH as f32;

        assert_eq!(
            compare_floats(x_tab, expected_x_tab),
            Ordering::Equal,
            "X pos for tab"
        );
        assert_eq!(
            compare_floats(w_tab, expected_w_tab),
            Ordering::Equal,
            "Width for tab"
        );
    }

    #[test]
    fn test_calculate_segment_geometry_inverted_range() {
        // Start 5, End 3
        // Should result in empty segment at start 5
        let content = "0123456789";
        let (x, w) = calculate_segment_geometry(
            content, 0, 5, 3, 0.0, FONT_SIZE, CHAR_WIDTH,
        );

        let expected_x = CHAR_WIDTH * 5.0;
        let expected_w = 0.0;

        assert_eq!(
            compare_floats(x, expected_x),
            Ordering::Equal,
            "X pos for inverted range"
        );
        assert!(
            (w - expected_w).abs() < f32::EPSILON,
            "Width for inverted range"
        );
    }

    #[test]
    fn test_validate_selection_indices() {
        // Test valid ASCII indices
        let content = "Hello";
        assert_eq!(validate_selection_indices(content, 0, 5), Some((0, 5)));
        assert_eq!(validate_selection_indices(content, 1, 3), Some((1, 3)));

        // Test valid multi-byte indices (Chinese "你好")
        // "你" is 3 bytes (0-3), "好" is 3 bytes (3-6)
        let content = "你好";
        assert_eq!(validate_selection_indices(content, 0, 6), Some((0, 6)));
        assert_eq!(validate_selection_indices(content, 0, 3), Some((0, 3)));
        assert_eq!(validate_selection_indices(content, 3, 6), Some((3, 6)));

        // Test invalid indices (splitting multi-byte char)
        assert_eq!(validate_selection_indices(content, 1, 3), None); // Split first char
        assert_eq!(validate_selection_indices(content, 0, 4), None); // Split second char

        // Test out of bounds (should be clamped if on boundary, but here len is 6)
        // If we pass start=0, end=100 -> clamped to 0, 6. 6 is boundary.
        assert_eq!(validate_selection_indices(content, 0, 100), Some((0, 6)));

        // Test inverted range
        assert_eq!(validate_selection_indices(content, 3, 0), None);
    }
}
