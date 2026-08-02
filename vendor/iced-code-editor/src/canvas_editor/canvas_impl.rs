//! Canvas rendering implementation using Iced's `canvas::Program`.

use crate::text_utils::char_range_to_byte_range;
use iced::advanced::input_method;
use iced::mouse;
use iced::widget::canvas::{self, Geometry};
use iced::{Color, Event, Point, Rectangle, Size, Theme, keyboard};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    HighlightIterator, HighlightState, Highlighter, Style, ThemeSet,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

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

/// Expands tabs and replaces whitespace with visible symbols: `\t` → `→` +
/// `·` fill, ` ` → `·`. The output has the same logical width as the
/// `expand_tabs` output, so existing width measurements remain valid.
fn expand_tabs_visible(text: &str, tab_width: usize) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        match ch {
            '\t' => {
                result.push('→');
                for _ in 1..tab_width {
                    result.push('·');
                }
            }
            ' ' => result.push('·'),
            other => result.push(other),
        }
    }
    result
}

/// Splits a string (already processed by [`expand_tabs_visible`]) into
/// alternating `(is_whitespace, segment)` pairs, where whitespace segments
/// consist exclusively of `·` and `→` characters.
fn split_whitespace_segments(text: &str) -> Vec<(bool, &str)> {
    if text.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();
    let mut seg_start = 0usize;
    let mut chars = text.char_indices().peekable();

    let is_ws_char = |c: char| c == '·' || c == '→';

    let first_ch = chars.peek().map(|(_, c)| *c).unwrap_or(' ');
    let mut current_is_ws = is_ws_char(first_ch);

    for (byte_idx, ch) in chars {
        let ch_is_ws = is_ws_char(ch);
        if ch_is_ws != current_is_ws {
            result.push((current_is_ws, &text[seg_start..byte_idx]));
            seg_start = byte_idx;
            current_is_ws = ch_is_ws;
        }
    }
    result.push((current_is_ws, &text[seg_start..]));
    result
}

/// Converts a syntect highlight [`Style`] into an iced [`Color`].
///
/// Only the foreground color is used; alpha is left fully opaque.
///
/// # Arguments
///
/// * `style` - The syntect style whose foreground color is converted.
fn color_from_style(style: Style) -> Color {
    Color::from_rgb(
        f32::from(style.foreground.r) / 255.0,
        f32::from(style.foreground.g) / 255.0,
        f32::from(style.foreground.b) / 255.0,
    )
}

/// Tokenizes a full logical line into colored spans using syntect.
///
/// The returned spans cover the entire line in order, each pairing an iced
/// [`Color`] with the owned token text. Each call highlights the line
/// independently from the syntax's initial state, so it does not handle
/// multi-line constructs; it is used for tests and benchmarks. Rendering uses
/// the sequential [`CodeEditor::highlighted_line_cached`] instead.
///
/// # Arguments
///
/// * `line` - The full logical line content (without trailing newline).
/// * `syntax` - The syntect syntax definition to tokenize with.
/// * `theme` - The syntect highlighting theme providing token colors.
/// * `syntax_set` - The syntax set the `syntax` belongs to.
///
/// # Returns
///
/// The ordered colored spans covering the entire line.
pub fn highlight_line_spans(
    line: &str,
    syntax: &syntect::parsing::SyntaxReference,
    theme: &syntect::highlighting::Theme,
    syntax_set: &SyntaxSet,
) -> Vec<(Color, String)> {
    let mut highlighter = HighlightLines::new(syntax, theme);
    let ranges = highlighter
        .highlight_line(line, syntax_set)
        .unwrap_or_else(|_| vec![(Style::default(), line)]);

    ranges
        .into_iter()
        .map(|(style, text)| (color_from_style(style), text.to_string()))
        .collect()
}

use super::folding;
use super::wrapping::{VisualLine, WrappingCalculator};
use super::{
    ArrowDirection, CodeEditor, Message, measure_char_width, measure_text_width,
};
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

        if !folding::is_line_fold_header(&self.buffer, visual_line.logical_line)
        {
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

    /// Returns the memoized syntax-highlighted spans for a logical line.
    ///
    /// Highlighting is performed sequentially: lines `0..=logical_line` are
    /// tokenized in order, each resuming from the syntect state left by the
    /// previous line, so multi-line constructs (block comments, multi-line
    /// strings) are colored correctly. The result is stored as a dense valid
    /// prefix in [`HighlightCache`] and reused across wrapped visual segments
    /// and across renders; an edit truncates the prefix from the changed line
    /// (see [`CodeEditor::invalidate_highlight_from`]) instead of clearing it,
    /// so deep lines are not re-parsed from the top on every keystroke. The
    /// cache is reset only when the active syntax changes.
    ///
    /// # Arguments
    ///
    /// * `logical_line` - Index of the logical line in the buffer.
    /// * `syntax` - The syntect syntax definition to tokenize with.
    /// * `theme` - The syntect highlighting theme providing token colors.
    /// * `syntax_set` - The syntax set the `syntax` belongs to.
    ///
    /// # Returns
    ///
    /// A shared handle to the line's colored token spans.
    fn highlighted_line_cached(
        &self,
        logical_line: usize,
        syntax: &syntect::parsing::SyntaxReference,
        theme: &syntect::highlighting::Theme,
        syntax_set: &SyntaxSet,
    ) -> Rc<Vec<(Color, String)>> {
        let mut guard = self.highlight_cache.borrow_mut();

        // Reset the whole cache only when the active syntax changes.
        let needs_reset =
            guard.as_ref().is_none_or(|cache| cache.syntax() != self.syntax);
        if needs_reset {
            *guard = Some(super::HighlightCache::new(self.syntax.clone()));
        }

        let Some(cache) = guard.as_mut() else {
            // Unreachable: populated just above. `unwrap`/`panic` are denied,
            // so fall back to a single independent highlight without caching.
            return Rc::new(highlight_line_spans(
                self.buffer.line(logical_line),
                syntax,
                theme,
                syntax_set,
            ));
        };

        if let Some(spans) = cache.spans(logical_line) {
            return spans;
        }

        // Extend the valid prefix sequentially up to `logical_line`, carrying
        // the parser/highlight state forward across lines.
        let highlighter = Highlighter::new(theme);
        let (mut parse_state, mut highlight_state) =
            cache.resume_state().unwrap_or_else(|| {
                (
                    ParseState::new(syntax),
                    HighlightState::new(&highlighter, ScopeStack::new()),
                )
            });

        let line_count = self.buffer.line_count();
        let target = logical_line.min(line_count.saturating_sub(1));
        let missing_lines =
            target.saturating_add(1).saturating_sub(cache.valid_len());
        let lines_to_parse =
            missing_lines.min(self.highlight_lines_remaining.get());
        let parse_end = cache
            .valid_len()
            .saturating_add(lines_to_parse)
            .saturating_sub(1)
            .min(target);
        let mut result = None;
        if lines_to_parse > 0 {
            for index in cache.valid_len()..=parse_end {
                // syntect's `_newlines` syntaxes expect a trailing '\n' for correct
                // end-of-line context handling; the stored buffer line has none.
                let mut line = self.buffer.line(index).to_string();
                line.push('\n');

                let ops = parse_state
                    .parse_line(&line, syntax_set)
                    .unwrap_or_default();
                let spans: Vec<(Color, String)> = HighlightIterator::new(
                    &mut highlight_state,
                    &ops,
                    &line,
                    &highlighter,
                )
                .filter_map(|(style, text)| {
                    let text = text.strip_suffix('\n').unwrap_or(text);
                    if text.is_empty() {
                        None
                    } else {
                        Some((color_from_style(style), text.to_string()))
                    }
                })
                .collect();

                let spans = Rc::new(spans);
                cache.push_line(
                    Rc::clone(&spans),
                    parse_state.clone(),
                    highlight_state.clone(),
                );
                if index == logical_line {
                    result = Some(spans);
                }
            }
        }
        self.highlight_lines_remaining.set(
            self.highlight_lines_remaining.get().saturating_sub(lines_to_parse),
        );

        result.or_else(|| cache.spans(logical_line)).unwrap_or_else(|| {
            Rc::new(vec![(
                self.style.text_color,
                self.buffer.line(logical_line).to_string(),
            )])
        })
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
        if let (Some(syntax), Some(syntax_theme)) = (syntax_ref, syntax_theme) {
            // Reuse the memoized full-line spans; only the visible segment of
            // the (possibly wrapped) line is positioned and drawn here.
            let spans = self.highlighted_line_cached(
                visual_line.logical_line,
                syntax,
                syntax_theme,
                syntax_set,
            );

            let mut x_offset =
                ctx.gutter_width + 5.0 - ctx.horizontal_scroll_offset;
            let mut char_pos = 0;

            for (color, text) in spans.iter() {
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

                    let (start_byte, end_byte) = char_range_to_byte_range(
                        text,
                        text_start_offset,
                        text_end_offset,
                    );

                    let segment_text = &text[start_byte..end_byte];
                    let display_text = if self.show_whitespace {
                        expand_tabs_visible(segment_text, super::TAB_WIDTH)
                    } else {
                        expand_tabs(segment_text, super::TAB_WIDTH).into_owned()
                    };
                    let display_width = measure_text_width(
                        &display_text,
                        ctx.full_char_width,
                        ctx.char_width,
                    );

                    if self.show_whitespace {
                        let ws_color = self.style.whitespace_color;
                        let mut seg_x = x_offset;
                        for (is_ws, seg) in
                            split_whitespace_segments(&display_text)
                        {
                            let seg_color =
                                if is_ws { ws_color } else { *color };
                            let seg_width = measure_text_width(
                                seg,
                                ctx.full_char_width,
                                ctx.char_width,
                            );
                            frame.fill_text(canvas::Text {
                                content: seg.to_string(),
                                position: Point::new(seg_x, y + 2.0),
                                color: seg_color,
                                size: ctx.font_size.into(),
                                font: ctx.font,
                                ..canvas::Text::default()
                            });
                            seg_x += seg_width;
                        }
                    } else {
                        frame.fill_text(canvas::Text {
                            content: display_text,
                            position: Point::new(x_offset, y + 2.0),
                            color: *color,
                            size: ctx.font_size.into(),
                            font: ctx.font,
                            ..canvas::Text::default()
                        });
                    }

                    x_offset += display_width;
                }

                char_pos = text_end;
            }
        } else {
            // Fallback to plain text
            let full_line_content = self.buffer.line(visual_line.logical_line);
            let (start_byte, end_byte) = char_range_to_byte_range(
                full_line_content,
                visual_line.start_col,
                visual_line.end_col,
            );
            let line_segment = &full_line_content[start_byte..end_byte];
            let display_text = if self.show_whitespace {
                expand_tabs_visible(line_segment, super::TAB_WIDTH)
            } else {
                expand_tabs(line_segment, super::TAB_WIDTH).into_owned()
            };
            let base_x = ctx.gutter_width + 5.0 - ctx.horizontal_scroll_offset;
            if self.show_whitespace {
                let ws_color = self.style.whitespace_color;
                let text_color = self.style.text_color;
                let mut seg_x = base_x;
                for (is_ws, seg) in split_whitespace_segments(&display_text) {
                    let seg_color = if is_ws { ws_color } else { text_color };
                    let seg_width = measure_text_width(
                        seg,
                        ctx.full_char_width,
                        ctx.char_width,
                    );
                    frame.fill_text(canvas::Text {
                        content: seg.to_string(),
                        position: Point::new(seg_x, y + 2.0),
                        color: seg_color,
                        size: ctx.font_size.into(),
                        font: ctx.font,
                        ..canvas::Text::default()
                    });
                    seg_x += seg_width;
                }
            } else {
                frame.fill_text(canvas::Text {
                    content: display_text,
                    position: Point::new(base_x, y + 2.0),
                    color: self.style.text_color,
                    size: ctx.font_size.into(),
                    font: ctx.font,
                    ..canvas::Text::default()
                });
            }
        }
    }

    /// Fills a single highlight rectangle for a column range within one visual
    /// line.
    ///
    /// Computes the CJK-aware segment geometry, applies the horizontal scroll
    /// offset, and draws the rectangle inset vertically to match the editor's
    /// highlight styling. Shared by selection and search-match rendering.
    ///
    /// # Arguments
    ///
    /// * `frame` - The canvas frame to draw on
    /// * `ctx` - Rendering context containing visual lines and metrics
    /// * `visual_idx` - Index of the visual line being drawn (drives the Y position)
    /// * `vl` - The visual line whose segment is highlighted
    /// * `cols` - Inclusive start and exclusive end columns of the segment
    /// * `color` - Fill color of the highlight rectangle
    fn fill_highlight_segment(
        &self,
        frame: &mut canvas::Frame,
        ctx: &RenderContext,
        visual_idx: usize,
        vl: &VisualLine,
        cols: (usize, usize),
        color: Color,
    ) {
        let y = visual_idx as f32 * ctx.line_height;
        let line_content = self.buffer.line(vl.logical_line);
        let (x_start, width) = calculate_segment_geometry(
            line_content,
            vl.start_col,
            cols.0,
            cols.1,
            ctx.gutter_width + 5.0,
            ctx.full_char_width,
            ctx.char_width,
        );
        let x_start = x_start - ctx.horizontal_scroll_offset;
        frame.fill_rectangle(
            Point::new(x_start, y + 2.0),
            Size::new(width, ctx.line_height - 4.0),
            color,
        );
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
        if !self.search_matches_visible() || self.search_state.query.is_empty()
        {
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
                        let vl = &ctx.visual_lines[start_v];
                        self.fill_highlight_segment(
                            frame,
                            ctx,
                            start_v,
                            vl,
                            (search_match.col, search_match.col + query_len),
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
                            let sel_start_col = if v_idx == start_v {
                                search_match.col
                            } else {
                                vl.start_col
                            };
                            let sel_end_col = if v_idx == end_v {
                                search_match.col + query_len
                            } else {
                                vl.end_col
                            };

                            self.fill_highlight_segment(
                                frame,
                                ctx,
                                v_idx,
                                vl,
                                (sel_start_col, sel_end_col),
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
        let selection_color = self.style.selection_color;

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
                    let vl = &ctx.visual_lines[start_v];
                    self.fill_highlight_segment(
                        frame,
                        ctx,
                        start_v,
                        vl,
                        (start.1, end.1),
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
                        let sel_start_col = if v_idx == start_v {
                            start.1
                        } else {
                            vl.start_col
                        };
                        let sel_end_col =
                            if v_idx == end_v { end.1 } else { vl.end_col };

                        self.fill_highlight_segment(
                            frame,
                            ctx,
                            v_idx,
                            vl,
                            (sel_start_col, sel_end_col),
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

                    self.fill_highlight_segment(
                        frame,
                        ctx,
                        v_idx,
                        vl,
                        (sel_start_col, sel_end_col),
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
                                self.style.selection_color,
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
            // Vim mode is single-cursor and Visual selections use an inclusive
            // active position that differs from the editor's half-open cursor.
            // Standard editing continues to render every cursor in the set.
            // ---------------------------------------------------------------------
            if self.vim_enabled {
                let position = self
                    .vim_state
                    .visual_positions()
                    .map(|(_, active)| active)
                    .unwrap_or_else(|| self.cursors.primary_position());
                self.draw_single_caret(frame, ctx, position);
            } else {
                for cursor in self.cursors.iter() {
                    self.draw_single_caret(frame, ctx, cursor.position);
                }
            }
        }
    }

    /// Returns the cursor size for a logical position using current font metrics.
    ///
    /// Standard editing and Vim Insert mode use the existing 2px bar. Vim
    /// Normal and Visual modes use the width of the character under the cursor;
    /// an empty line or end-of-line position uses one narrow character width.
    fn cursor_size_for_position(&self, position: (usize, usize)) -> Size {
        let uses_block =
            self.vim_enabled && self.vim_state.mode() != super::VimMode::Insert;
        let width = if uses_block {
            self.buffer
                .line(position.0)
                .chars()
                .nth(position.1)
                .map(|ch| {
                    measure_char_width(
                        ch,
                        self.full_char_width,
                        self.char_width,
                    )
                })
                .filter(|width| *width > 0.0)
                .unwrap_or(self.char_width)
        } else {
            2.0
        };

        Size::new(width, (self.line_height - 4.0).max(1.0))
    }

    /// Draws one cursor at the given logical (line, col) position.
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

            let cursor_size = self.cursor_size_for_position(position);
            let mut cursor_color = self.style.text_color;
            if cursor_size.width > 2.0 {
                cursor_color.a *= 0.55;
            }

            frame.fill_rectangle(
                Point::new(cursor_x, cursor_y + 2.0),
                cursor_size,
                cursor_color,
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
        modified_key: &keyboard::Key,
        modifiers: &keyboard::Modifiers,
    ) -> Option<Action<Message>> {
        // `command()` maps to Command on macOS and Control elsewhere. Keep
        // accepting Control on macOS for backwards compatibility.
        let command_pressed = modifiers.command() || modifiers.control();

        // Toggle Vim behavior without conflicting with the platform paste
        // shortcut (Ctrl/Cmd+V).
        if command_pressed
            && modifiers.alt()
            && !modifiers.shift()
            && matches!(key, keyboard::Key::Character(v) if v.as_str() == "v")
        {
            return Some(Action::publish(Message::ToggleVimMode).and_capture());
        }

        // Handle Ctrl/Cmd+S through the same host-owned save request as Vim
        // `:w`.
        if command_pressed
            && !modifiers.alt()
            && !modifiers.shift()
            && matches!(key, keyboard::Key::Character(s) if s.as_str() == "s")
        {
            return Some(
                Action::publish(Message::WriteRequested).and_capture(),
            );
        }

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
        if (command_pressed
            && matches!(key, keyboard::Key::Character(c) if c.as_str() == "c"))
            || (modifiers.control()
                && matches!(
                    key,
                    keyboard::Key::Named(keyboard::key::Named::Insert)
                ))
        {
            return Some(Action::publish(Message::Copy).and_capture());
        }

        // Handle Ctrl/Cmd+X (cut)
        if command_pressed
            && matches!(key, keyboard::Key::Character(x) if x.as_str() == "x")
        {
            return Some(Action::publish(Message::Cut).and_capture());
        }

        // Handle Ctrl/Cmd+A (select all)
        if command_pressed
            && matches!(key, keyboard::Key::Character(a) if a.as_str() == "a")
        {
            return Some(Action::publish(Message::SelectAll).and_capture());
        }

        // Handle Ctrl/Cmd+Z (undo). Shift+Cmd+Z is redo on macOS.
        if command_pressed
            && !modifiers.shift()
            && matches!(key, keyboard::Key::Character(z) if z.as_str() == "z")
        {
            return Some(Action::publish(Message::Undo).and_capture());
        }

        // Handle Ctrl/Cmd+Y and Shift+Cmd+Z (redo)
        if command_pressed
            && (matches!(key, keyboard::Key::Character(y) if y.as_str() == "y")
                || (modifiers.shift()
                    && matches!(key, keyboard::Key::Character(z) if z.as_str() == "z")))
        {
            return Some(Action::publish(Message::Redo).and_capture());
        }

        // Vim's redo binding is Ctrl+R in Normal mode. Keep the existing
        // platform redo shortcuts above available in every editor mode.
        if self.vim_enabled
            && self.vim_state.mode() == super::VimMode::Normal
            && modifiers.control()
            && !modifiers.shift()
            && matches!(key, keyboard::Key::Character(r) if r.as_str() == "r")
        {
            return Some(Action::publish(Message::Redo).and_capture());
        }

        // Handle Ctrl+F (open search)
        if command_pressed
            && matches!(key, keyboard::Key::Character(f) if f.as_str() == "f")
            && self.search_replace_enabled
        {
            return Some(Action::publish(Message::OpenSearch).and_capture());
        }

        // Handle Ctrl+H (open search and replace)
        if command_pressed
            && matches!(key, keyboard::Key::Character(h) if h.as_str() == "h")
            && self.search_replace_enabled
        {
            return Some(
                Action::publish(Message::OpenSearchReplace).and_capture(),
            );
        }

        // Handle Cmd/Ctrl+G (open go-to-line input)
        if command_pressed
            && matches!(key, keyboard::Key::Character(g) if g.as_str() == "g")
        {
            return Some(Action::publish(Message::OpenGotoLine).and_capture());
        }

        // Handle Escape — close the active overlay, or collapse multi-cursor.
        if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
            let message = if self.goto_line_state.is_open {
                Message::CloseGotoLine
            } else if self.search_state.is_open {
                Message::CloseSearch
            } else if self.vim_enabled {
                Message::VimKey('\u{1b}')
            } else {
                Message::CloseSearch
            };
            return Some(Action::publish(message).and_capture());
        }

        // Handle Ctrl+D (select next occurrence)
        if command_pressed
            && matches!(key, keyboard::Key::Character(d) if d.as_str() == "d")
        {
            return Some(
                Action::publish(Message::SelectNextOccurrence).and_capture(),
            );
        }

        // Handle Ctrl+/ (toggle line comment).
        //
        // Match against both the base key and `modified_key` so the shortcut
        // works regardless of layout: on US/QWERTY `/` is unshifted (in `key`),
        // while on French AZERTY it is Shift+`:` and only appears in
        // `modified_key`.
        if command_pressed
            && (matches!(key, keyboard::Key::Character(c) if c.as_str() == "/")
                || matches!(modified_key, keyboard::Key::Character(c) if c.as_str() == "/"))
        {
            return Some(Action::publish(Message::ToggleComment).and_capture());
        }

        // Handle Ctrl+Alt+Up (add cursor above)
        if modifiers.control()
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
        if modifiers.control()
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

        // Handle Alt+Up / Alt+Down (move line) and Shift+Alt+Up / Shift+Alt+Down
        // (duplicate line). Exclude Control to avoid clashing with the
        // Ctrl+Alt+Up/Down multi-cursor shortcuts above.
        if modifiers.alt() && !modifiers.control() {
            if matches!(
                key,
                keyboard::Key::Named(keyboard::key::Named::ArrowUp)
            ) {
                let message = if modifiers.shift() {
                    Message::DuplicateLineUp
                } else {
                    Message::MoveLineUp
                };
                return Some(Action::publish(message).and_capture());
            }
            if matches!(
                key,
                keyboard::Key::Named(keyboard::key::Named::ArrowDown)
            ) {
                let message = if modifiers.shift() {
                    Message::DuplicateLineDown
                } else {
                    Message::MoveLineDown
                };
                return Some(Action::publish(message).and_capture());
            }
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
        if (command_pressed
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
        if command_pressed
            && matches!(key, keyboard::Key::Named(keyboard::key::Named::Home))
        {
            return Some(Action::publish(Message::CtrlHome).and_capture());
        }

        // Handle Ctrl+End (go to end of document)
        if command_pressed
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
            if modifiers.control()
                && matches!(key, keyboard::Key::Character(c) if c.as_str() == ".")
            {
                return Some(
                    Action::publish(Message::ToggleFoldAtCursor).and_capture(),
                );
            }

            // Ctrl+K : fold all blocks.
            if modifiers.control()
                && !modifiers.shift()
                && matches!(key, keyboard::Key::Character(c) if c.as_str() == "k")
            {
                return Some(Action::publish(Message::FoldAll).and_capture());
            }

            // Ctrl+J : unfold all blocks.
            if modifiers.control()
                && !modifiers.shift()
                && matches!(key, keyboard::Key::Character(c) if c.as_str() == "j")
            {
                return Some(Action::publish(Message::UnfoldAll).and_capture());
            }
        }

        None
    }

    fn printable_input_message(&self, ch: char) -> Message {
        if self.vim_enabled && self.vim_state.mode() != super::VimMode::Insert {
            Message::VimKey(ch)
        } else {
            Message::CharacterInput(ch)
        }
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
            && !modifiers.alt()
        {
            // Check if it's a printable character (not a control character)
            // This filters out Enter (\n), Tab (\t), Delete (U+007F), etc.
            if let Some(first_char) = text_content.chars().next()
                && !first_char.is_control()
            {
                return Some(
                    Action::publish(self.printable_input_message(first_char))
                        .and_capture(),
                );
            }
        }

        // PRIORITY 2: Handle special named keys (navigation, editing)
        // These are only processed if text didn't contain a printable character
        let message = match key {
            keyboard::Key::Named(keyboard::key::Named::Backspace)
                if !self.vim_enabled
                    || self.vim_state.mode() == super::VimMode::Insert
                    || self.vim_state.command_line_active() =>
            {
                if self.vim_state.command_line_active() {
                    Some(Message::VimKey('\u{8}'))
                } else {
                    Some(Message::Backspace)
                }
            }
            keyboard::Key::Named(keyboard::key::Named::Delete)
                if !self.vim_enabled
                    || self.vim_state.mode() == super::VimMode::Insert =>
            {
                Some(Message::Delete)
            }
            keyboard::Key::Named(keyboard::key::Named::Enter)
                if !self.vim_enabled
                    || self.vim_state.mode() == super::VimMode::Insert
                    || self.vim_state.command_line_active() =>
            {
                if self.vim_state.command_line_active() {
                    Some(Message::VimKey('\n'))
                } else {
                    Some(Message::Enter)
                }
            }
            keyboard::Key::Named(keyboard::key::Named::Tab)
                if !self.vim_enabled
                    || self.vim_state.mode() == super::VimMode::Insert =>
            {
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
                    && !modifiers.alt()
                    && let keyboard::Key::Character(c) = key
                    && !c.is_empty()
                {
                    return c
                        .chars()
                        .next()
                        .map(|ch| self.printable_input_message(ch))
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
    /// * `key` - The keyboard key that was pressed (base key, no modifiers applied)
    /// * `modified_key` - The key with all modifiers applied except Ctrl; used
    ///   for character shortcuts so they work on layouts where the glyph needs
    ///   Shift (e.g. `/` on French AZERTY)
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
        modified_key: &keyboard::Key,
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
        if let Some(action) =
            self.handle_keyboard_shortcuts(key, modified_key, modifiers)
        {
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

        folding::is_line_fold_header(&self.buffer, visual_line.logical_line)
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
                        let message = if self.vim_enabled {
                            Message::MouseClick(position)
                        } else {
                            Message::AltClick(position)
                        };
                        return Action::publish(message).and_capture();
                    }

                    let click_count = self.classify_click(position);
                    match click_count {
                        2 => Action::publish(Message::DoubleClick(position))
                            .and_capture(),
                        3 => Action::publish(Message::TripleClick(position))
                            .and_capture(),
                        // Don't capture the event so it can bubble up for focus management
                        // This implements focus event propagation through the widget hierarchy
                        _ => Action::publish(Message::MouseClick(position)),
                    }
                })
            }
            mouse::Event::ButtonPressed(mouse::Button::Right) => {
                cursor.position_in(bounds).map(|position| {
                    Action::publish(Message::ContextMenuRequested(position))
                        .and_capture()
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
        if self.vim_enabled && self.vim_state.mode() != super::VimMode::Insert {
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
        let visual_lines: Rc<Vec<VisualLine>> =
            self.visual_lines_cached(bounds.width);

        // Prefer the tracked viewport height once a real Scrolled event has
        // confirmed it, but fall back to the real per-frame layout bounds
        // before that — `viewport_height` starts out as a construction-time
        // placeholder (see its field doc) that has no relationship to this
        // editor's actual rendered size, and trusting it here is what used
        // to make a freshly restored/constructed editor render blank (or
        // with a wrong line window) until the user happened to scroll it.
        let effective_viewport_height = if self.viewport_metrics_confirmed {
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
                // Bound sequential syntect catch-up work for this frame. This
                // keeps a deep jump or a cache truncation in a huge file from
                // blocking the UI while parsing every preceding line.
                self.highlight_lines_remaining
                    .set(super::HIGHLIGHT_LINES_PER_FRAME);

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
                // Pick a syntax theme whose tone matches the editor background:
                // dark syntax colors on a light editor (or vice versa) are unreadable.
                let bg = self.style.background;
                let bg_is_dark = bg.r * 0.299 + bg.g * 0.587 + bg.b * 0.114 < 0.5;
                let preferred: &[&str] = if let Some(name) = &self.style.syntax_theme_name {
                    std::slice::from_ref(name)
                } else if bg_is_dark {
                    &["base16-mocha.dark", "base16-ocean.dark", "Solarized (dark)"]
                } else {
                    &["InspiredGitHub", "Solarized (light)"]
                };
                let syntax_theme = preferred
                    .iter()
                    .find_map(|name| theme_set.themes.get(*name))
                    .or_else(|| theme_set.themes.values().next());

                // Normalize common language aliases/extensions used by consumers.
                let syntax_ref = match self.syntax.as_str() {
                    "python" => syntax_set.find_syntax_by_extension("py"),
                    "rust" => syntax_set.find_syntax_by_extension("rs"),
                    "javascript" => syntax_set.find_syntax_by_extension("js"),
                    "htm" => syntax_set.find_syntax_by_extension("html"),
                    "svg" => syntax_set.find_syntax_by_extension("xml"),
                    "markdown" => syntax_set.find_syntax_by_extension("md"),
                    "text" | "txt" => Some(syntax_set.find_syntax_plain_text()),
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
                    full_char_width: self.full_char_width,
                    char_width: self.char_width,
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
                    full_char_width: self.full_char_width,
                    char_width: self.char_width,
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
                modified_key,
                modifiers,
                text,
                ..
            }) => {
                self.modifiers.set(*modifiers);
                self.handle_keyboard_event(
                    key,
                    modified_key,
                    modifiers,
                    text,
                    bounds,
                    &cursor,
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

    /// Uses the text-selection cursor over the editable code area.
    ///
    /// The gutter keeps the default cursor, except for an interactive fold
    /// chevron. Checking the cursor against `bounds` is important because a
    /// canvas program's interaction can otherwise remain active out of bounds.
    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };

        if self.fold_header_at_point(position).is_some() {
            mouse::Interaction::Pointer
        } else if position.x >= self.gutter_width() {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
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

    fn editor_mouse_interaction(
        editor: &CodeEditor,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        canvas::Program::<Message>::mouse_interaction(
            editor,
            &(),
            bounds,
            cursor,
        )
    }

    #[test]
    fn test_vim_navigation_keyboard_route_uses_dedicated_message() {
        let mut editor = CodeEditor::new("abc", "txt").with_vim_enabled(true);

        assert!(matches!(
            editor.printable_input_message('l'),
            Message::VimKey('l')
        ));

        let _ = editor.vim_state.parse_key('i');
        assert!(matches!(
            editor.printable_input_message('x'),
            Message::CharacterInput('x')
        ));

        editor.set_vim_enabled(false);
        assert!(matches!(
            editor.printable_input_message('x'),
            Message::CharacterInput('x')
        ));

        editor.set_vim_enabled(true);
        let key = keyboard::Key::Character("r".into());
        let message = editor
            .handle_keyboard_shortcuts(&key, &key, &keyboard::Modifiers::CTRL)
            .map(|action| action.into_inner().0);
        assert!(matches!(message, Some(Some(Message::Redo))));
    }

    #[test]
    fn test_vim_command_line_routes_enter_and_backspace() {
        let mut editor = CodeEditor::new("abc", "txt").with_vim_enabled(true);
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        let _ = editor.vim_state.parse_key('/');

        let backspace = editor
            .handle_character_input(
                &keyboard::Key::Named(keyboard::key::Named::Backspace),
                &keyboard::Modifiers::NONE,
                None,
            )
            .map(|action| action.into_inner().0);
        assert!(matches!(backspace, Some(Some(Message::VimKey('\u{8}')))));

        let enter = editor
            .handle_character_input(
                &keyboard::Key::Named(keyboard::key::Named::Enter),
                &keyboard::Modifiers::NONE,
                None,
            )
            .map(|action| action.into_inner().0);
        assert!(matches!(enter, Some(Some(Message::VimKey('\n')))));
    }

    #[test]
    fn test_vim_cursor_rendering_insert_uses_bar() {
        let mut editor = CodeEditor::new("a", "txt").with_vim_enabled(true);
        let _ = editor.vim_state.parse_key('i');

        let size = editor.cursor_size_for_position((0, 0));

        assert_eq!(compare_floats(size.width, 2.0), Ordering::Equal);
        assert_eq!(
            compare_floats(size.height, editor.line_height() - 4.0),
            Ordering::Equal
        );
    }

    #[test]
    fn test_vim_cursor_rendering_normal_uses_ascii_block() {
        let editor = CodeEditor::new("a", "txt").with_vim_enabled(true);

        let size = editor.cursor_size_for_position((0, 0));

        assert_eq!(
            compare_floats(size.width, editor.char_width()),
            Ordering::Equal
        );
    }

    #[test]
    fn test_vim_cursor_rendering_normal_uses_cjk_width() {
        let editor = CodeEditor::new("汉", "txt").with_vim_enabled(true);

        let size = editor.cursor_size_for_position((0, 0));

        assert_eq!(
            compare_floats(size.width, editor.full_char_width()),
            Ordering::Equal
        );
    }

    #[test]
    fn test_vim_cursor_rendering_empty_line_has_visible_block() {
        let editor = CodeEditor::new("", "txt").with_vim_enabled(true);

        let size = editor.cursor_size_for_position((0, 0));

        assert_eq!(
            compare_floats(size.width, editor.char_width()),
            Ordering::Equal
        );
        assert!(size.width > 2.0);
    }

    #[test]
    fn test_mouse_interaction_uses_text_cursor_in_editable_area() {
        let editor = CodeEditor::new("fn main() {}", "rs");
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0));
        let cursor = mouse::Cursor::Available(Point::new(
            editor.gutter_width() + 10.0,
            10.0,
        ));

        assert_eq!(
            editor_mouse_interaction(&editor, bounds, cursor),
            mouse::Interaction::Text
        );
    }

    #[test]
    fn test_mouse_interaction_keeps_default_cursor_in_gutter_and_outside() {
        let editor = CodeEditor::new("fn main() {}", "rs");
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0));

        assert_eq!(
            editor_mouse_interaction(
                &editor,
                bounds,
                mouse::Cursor::Available(Point::new(5.0, 10.0)),
            ),
            mouse::Interaction::default()
        );
        assert_eq!(
            editor_mouse_interaction(
                &editor,
                bounds,
                mouse::Cursor::Available(Point::new(900.0, 10.0)),
            ),
            mouse::Interaction::default()
        );
    }

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

    #[test]
    fn test_highlight_line_spans_covers_full_line() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = syntax_set.find_syntax_plain_text();
        let theme = syntect::highlighting::Theme::default();

        let line = "fn main() {}";
        let spans = highlight_line_spans(line, syntax, &theme, &syntax_set);

        assert!(!spans.is_empty(), "expected at least one span");
        let combined: String =
            spans.iter().map(|(_, text)| text.as_str()).collect();
        assert_eq!(combined, line, "spans must cover the entire line");
    }

    #[test]
    fn test_highlighted_line_cached_reuses_until_invalidated() {
        let editor = CodeEditor::new("fn main() {}\nlet x = 1;", "rs");
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = syntax_set.find_syntax_plain_text();
        let theme = syntect::highlighting::Theme::default();

        let first =
            editor.highlighted_line_cached(0, syntax, &theme, &syntax_set);
        let second =
            editor.highlighted_line_cached(0, syntax, &theme, &syntax_set);
        assert!(
            Rc::ptr_eq(&first, &second),
            "a cached line should be reused as the same Rc"
        );

        editor.invalidate_highlight_from(0);
        let third =
            editor.highlighted_line_cached(0, syntax, &theme, &syntax_set);
        assert!(
            !Rc::ptr_eq(&first, &third),
            "invalidation should force the line to be recomputed"
        );
    }

    #[test]
    fn test_select_all_delete_paste_reflects_new_content_in_highlight_cache() {
        let mut editor = CodeEditor::new("{\"a\": 1}", "json");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = syntax_set
            .find_syntax_by_extension("json")
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
        let theme = syntect::highlighting::Theme::default();

        // Prime the cache exactly like a real render would.
        let _ = editor.highlighted_line_cached(0, syntax, &theme, &syntax_set);

        let _ = editor.update(&Message::SelectAll);
        let _ = editor.update(&Message::DeleteSelection);
        let _ = editor.update(&Message::Paste("{\"b\": 2}".to_string()));

        let spans = editor.highlighted_line_cached(0, syntax, &theme, &syntax_set);
        let combined: String =
            spans.iter().map(|(_, text)| text.as_str()).collect();

        assert_eq!(editor.buffer.line(0), "{\"b\": 2}");
        assert_eq!(combined, editor.buffer.line(0));
        assert!(spans.len() > 1, "expected multiple syntax-highlighted spans, not a single flat fallback span");
    }

    #[test]
    fn test_edit_resets_stale_scroll_window_so_content_stays_visible() {
        // Build a long document and simulate having scrolled deep into it,
        // which sets a non-trivial `cache_window_start_line..cache_window_end_line`
        // (see `handle_scrolled_msg`). Nothing resets that window on edits —
        // only another Scrolled message does.
        let long_content: String =
            (0..20).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let mut editor = CodeEditor::new(&long_content, "txt");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        // Simulate a scroll deep into the 20-line document.
        editor.cache_window_start_line = 10;
        editor.cache_window_end_line = 18;

        // Now select-all, delete, and type much shorter content (like
        // clearing a request body and typing a new one).
        let _ = editor.update(&Message::SelectAll);
        let _ = editor.update(&Message::DeleteSelection);
        let _ = editor.update(&Message::CharacterInput('x'));
        let _ = editor.update(&Message::CharacterInput('y'));
        let _ = editor.update(&Message::CharacterInput('z'));
        assert_eq!(editor.buffer.line(0), "xyz");

        // Replicate exactly the windowing logic `draw()` uses to pick which
        // visual lines to actually render.
        let visual_lines = editor.visual_lines_cached(800.0);
        let (start_idx, end_idx) =
            if editor.cache_window_end_line > editor.cache_window_start_line {
                let s = editor.cache_window_start_line.min(visual_lines.len());
                let e = editor.cache_window_end_line.min(visual_lines.len());
                (s, e)
            } else {
                (0, visual_lines.len())
            };

        assert!(
            start_idx < end_idx && end_idx <= visual_lines.len() && start_idx < visual_lines.len(),
            "render window ({start_idx}..{end_idx}) does not cover any of the \
             {} actual visual line(s) after the document shrank — the typed \
             text would render as blank",
            visual_lines.len()
        );
    }

    #[test]
    fn test_fresh_editor_select_all_delete_type_renders_correctly() {
        // No manual window tampering here at all — this is a brand-new
        // editor exactly as `make_code_editor` builds one in rustman, that
        // has never been scrolled.
        let mut editor = CodeEditor::new(
            "{\n  \"a\": 1,\n  \"b\": 2\n}",
            "json",
        );
        editor.set_font(iced::Font::MONOSPACE);
        editor.set_font_size(12.0, true);
        editor.set_line_height(18.5);
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        let _ = editor.update(&Message::SelectAll);
        let _ = editor.update(&Message::Delete);
        let _ = editor.update(&Message::CharacterInput('x'));
        let _ = editor.update(&Message::CharacterInput('y'));
        let _ = editor.update(&Message::CharacterInput('z'));

        let visual_lines = editor.visual_lines_cached(800.0);
        let (start_idx, end_idx) =
            if editor.cache_window_end_line > editor.cache_window_start_line {
                let s = editor.cache_window_start_line.min(visual_lines.len());
                let e = editor.cache_window_end_line.min(visual_lines.len());
                (s, e)
            } else {
                (0, visual_lines.len())
            };

        assert_eq!(editor.buffer.line(0), "xyz");
        assert!(start_idx < end_idx, "render range is empty!");
    }

    #[test]
    fn test_edit_clamps_stale_viewport_scroll_after_document_shrinks() {
        // Simulate having auto-scrolled deep into a longer document (the
        // cursor was down near line 19 of 20, so `scroll_to_cursor` would
        // have pushed `viewport_scroll` down accordingly) before clearing it.
        let long_content: String =
            (0..20).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let mut editor = CodeEditor::new(&long_content, "txt");
        editor.set_line_height(18.5);
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.viewport_height = 200.0;
        editor.viewport_scroll = 300.0; // deep into the 20-line document

        let _ = editor.update(&Message::SelectAll);
        let _ = editor.update(&Message::DeleteSelection);
        let _ = editor.update(&Message::CharacterInput('x'));
        let _ = editor.update(&Message::CharacterInput('y'));
        let _ = editor.update(&Message::CharacterInput('z'));
        assert_eq!(editor.buffer.line(0), "xyz");

        let visual_lines = editor.visual_lines_cached(editor.viewport_width);
        let first_visible_line =
            (editor.viewport_scroll / editor.line_height).floor() as usize;

        assert!(
            first_visible_line < visual_lines.len(),
            "viewport_scroll ({}) still points past the document's only \
             {} visual line(s) after it shrank — the typed text would \
             render as blank",
            editor.viewport_scroll,
            visual_lines.len()
        );
    }

    #[test]
    fn test_select_all_delete_type_preserves_syntax_highlighting() {
        use crate::theme::Style;

        // Mirror rustman's real setup as closely as possible: real font,
        // real line height, real theme with a named syntax theme override.
        let mut editor = CodeEditor::new("{\"a\": 1}", "json");
        editor.set_font(iced::Font::MONOSPACE);
        editor.set_font_size(12.0, true);
        editor.set_line_height(18.5);
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        let style = Style {
            background: Color::from_rgb(0.1, 0.1, 0.1),
            text_color: Color::WHITE,
            gutter_background: Color::from_rgb(0.1, 0.1, 0.1),
            gutter_border: Color::from_rgb(0.2, 0.2, 0.2),
            line_number_color: Color::from_rgb(0.5, 0.5, 0.5),
            scrollbar_background: Color::TRANSPARENT,
            scroller_color: Color::from_rgb(0.5, 0.5, 0.5),
            current_line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
            whitespace_color: Color::from_rgba(1.0, 1.0, 1.0, 0.4),
            selection_color: Color::from_rgba(0.3, 0.5, 0.8, 0.3),
            syntax_theme_name: Some("base16-mocha.dark"),
        };
        editor.set_theme(style);

        let syntax_set = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
        let theme_set = THEME_SET.get_or_init(ThemeSet::load_defaults);
        eprintln!(
            "does 'base16-mocha.dark' exist in default theme set? {}",
            theme_set.themes.contains_key("base16-mocha.dark")
        );
        let syntax = syntax_set
            .find_syntax_by_extension("json")
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
        let bg = editor.style.background;
        let bg_is_dark = bg.r * 0.299 + bg.g * 0.587 + bg.b * 0.114 < 0.5;
        let preferred: &[&str] = if let Some(name) = &editor.style.syntax_theme_name {
            std::slice::from_ref(name)
        } else if bg_is_dark {
            &["base16-mocha.dark", "base16-ocean.dark", "Solarized (dark)"]
        } else {
            &["InspiredGitHub", "Solarized (light)"]
        };
        let syntax_theme = preferred
            .iter()
            .find_map(|name| theme_set.themes.get(*name))
            .or_else(|| theme_set.themes.values().next())
            .unwrap();

        // Prime cache like a real render.
        let _ = editor.highlighted_line_cached(0, syntax, syntax_theme, syntax_set);

        let _ = editor.update(&Message::SelectAll);
        let _ = editor.update(&Message::Delete);
        let _ = editor.update(&Message::CharacterInput('{'));
        let _ = editor.update(&Message::CharacterInput('"'));
        let _ = editor.update(&Message::CharacterInput('a'));
        let _ = editor.update(&Message::CharacterInput('"'));
        let _ = editor.update(&Message::CharacterInput(':'));
        let _ = editor.update(&Message::CharacterInput('1'));
        let _ = editor.update(&Message::CharacterInput('}'));

        let spans = editor.highlighted_line_cached(0, syntax, syntax_theme, syntax_set);
        let colors: std::collections::HashSet<_> =
            spans.iter().map(|(c, _)| format!("{:?}", c)).collect();
        eprintln!(
            "buffer={:?} span_count={} distinct_colors={} spans={:?}",
            editor.buffer.line(0),
            spans.len(),
            colors.len(),
            spans
        );

        assert_eq!(editor.buffer.line(0), "{\"a\":1}");
        assert!(
            colors.len() > 1,
            "expected multiple distinct colors (real syntax highlighting), got only {} distinct color(s): {:?}",
            colors.len(),
            spans
        );
    }

    #[test]
    fn test_highlight_budget_uses_plain_fallback_without_scanning_to_target() {
        let editor = CodeEditor::new("zero\none\ntwo\nthree\nfour", "txt");
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = syntax_set.find_syntax_plain_text();
        let theme = syntect::highlighting::Theme::default();
        editor.highlight_lines_remaining.set(2);

        let spans =
            editor.highlighted_line_cached(4, syntax, &theme, &syntax_set);
        let combined: String =
            spans.iter().map(|(_, text)| text.as_str()).collect();

        assert_eq!(combined, "four");
        assert_eq!(
            editor
                .highlight_cache
                .borrow()
                .as_ref()
                .map(super::super::HighlightCache::valid_len),
            Some(2)
        );
        assert_eq!(editor.highlight_lines_remaining.get(), 0);
    }

    #[test]
    fn test_highlighted_line_cached_handles_multiline_comments() {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = syntax_set
            .find_syntax_by_extension("rs")
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
        let theme = ThemeSet::load_defaults()
            .themes
            .get("base16-ocean.dark")
            .cloned()
            .unwrap_or_default();

        // Line index 2 ("still inside") sits within a `/* ... */` block.
        let code = "let a = 1;\n/* open\nstill inside\n*/\nlet b = 2;";
        let editor = CodeEditor::new(code, "rs");

        // Sequential highlighting resumes inside the block comment.
        let sequential =
            editor.highlighted_line_cached(2, syntax, &theme, &syntax_set);
        // Independent highlighting wrongly treats the line as ordinary code.
        let independent = highlight_line_spans(
            editor.buffer.line(2),
            syntax,
            &theme,
            &syntax_set,
        );

        let sequential_color = sequential.first().map(|(color, _)| *color);
        let independent_color = independent.first().map(|(color, _)| *color);
        assert!(sequential_color.is_some());
        assert!(independent_color.is_some());
        assert_ne!(
            sequential_color, independent_color,
            "a line inside a block comment must be colored as a comment"
        );
    }

    #[test]
    fn test_expand_tabs_visible_spaces() {
        assert_eq!(expand_tabs_visible("a b", 4), "a·b");
        assert_eq!(expand_tabs_visible("  x  ", 4), "··x··");
    }

    #[test]
    fn test_expand_tabs_visible_tabs() {
        // tab_width = 4: '\t' → '→' + 3 × '·'
        assert_eq!(expand_tabs_visible("\t", 4), "→···");
        assert_eq!(expand_tabs_visible("a\tb", 4), "a→···b");
    }

    #[test]
    fn test_expand_tabs_visible_no_whitespace() {
        assert_eq!(expand_tabs_visible("hello", 4), "hello");
    }

    #[test]
    fn test_split_whitespace_segments_mixed() {
        let segs = split_whitespace_segments("a·b");
        assert_eq!(segs, vec![(false, "a"), (true, "·"), (false, "b")]);
    }

    #[test]
    fn test_split_whitespace_segments_leading_ws() {
        let segs = split_whitespace_segments("··x");
        assert_eq!(segs, vec![(true, "··"), (false, "x")]);
    }

    #[test]
    fn test_split_whitespace_segments_all_ws() {
        let segs = split_whitespace_segments("···");
        assert_eq!(segs, vec![(true, "···")]);
    }

    #[test]
    fn test_split_whitespace_segments_empty() {
        let segs = split_whitespace_segments("");
        assert!(segs.is_empty());
    }

    #[test]
    fn test_command_g_opens_goto_line_dialog() {
        let editor = CodeEditor::new("one\ntwo", "rs");
        let key = keyboard::Key::Character("g".into());

        let message = editor
            .handle_keyboard_shortcuts(
                &key,
                &key,
                &keyboard::Modifiers::COMMAND,
            )
            .map(|action| action.into_inner().0);

        assert!(matches!(message, Some(Some(Message::OpenGotoLine))));
    }
}
