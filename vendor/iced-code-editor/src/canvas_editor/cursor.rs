//! Cursor movement and positioning logic.

use iced::widget::operation::scroll_to;
use iced::widget::scrollable;
use iced::{Point, Task};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use super::measure_text_width;

use super::wrapping::{VisualLine, WrappingCalculator};
use super::{ArrowDirection, CodeEditor, Message};
use crate::text_buffer::TextBuffer;

/// Computes the next logical `(line, col)` position for a cursor at `pos` moving in `direction`.
///
/// Returns `None` if the cursor is already at the boundary and cannot move further.
fn compute_next_position(
    pos: (usize, usize),
    direction: ArrowDirection,
    buffer: &TextBuffer,
    visual_lines: &[VisualLine],
) -> Option<(usize, usize)> {
    let (line, col) = pos;
    match direction {
        ArrowDirection::Up | ArrowDirection::Down => {
            let current_visual =
                WrappingCalculator::logical_to_visual(visual_lines, line, col)?;

            let target_visual = match direction {
                ArrowDirection::Up => current_visual.checked_sub(1)?,
                ArrowDirection::Down => {
                    let next = current_visual + 1;
                    if next < visual_lines.len() {
                        next
                    } else {
                        return None;
                    }
                }
                _ => return None,
            };

            let target_vl = &visual_lines[target_visual];
            let current_vl = &visual_lines[current_visual];

            let new_col = if target_vl.logical_line == line {
                let offset_in_current =
                    col.saturating_sub(current_vl.start_col);
                let target_col = target_vl.start_col + offset_in_current;
                if target_col >= target_vl.end_col {
                    target_vl.end_col.saturating_sub(1).max(target_vl.start_col)
                } else {
                    target_col
                }
            } else {
                let target_line_len = buffer.line_len(target_vl.logical_line);
                (target_vl.start_col + col.min(target_vl.len()))
                    .min(target_line_len)
            };

            Some((target_vl.logical_line, new_col))
        }
        ArrowDirection::Left => {
            if col > 0 {
                Some((line, col - 1))
            } else if line > 0 {
                Some((line - 1, buffer.line_len(line - 1)))
            } else {
                None
            }
        }
        ArrowDirection::Right => {
            let line_len = buffer.line_len(line);
            if col < line_len {
                Some((line, col + 1))
            } else if line + 1 < buffer.line_count() {
                Some((line + 1, 0))
            } else {
                None
            }
        }
    }
}

impl CodeEditor {
    /// Sets the cursor position to the specified line and column.
    ///
    /// This method ensures the new position is within the bounds of the text buffer.
    /// It also resets the blinking animation, clears the overlay cache (to redraw
    /// the cursor immediately), and scrolls the view to make the cursor visible.
    ///
    /// # Arguments
    ///
    /// * `line` - The target line index (0-based).
    /// * `col` - The target column index (0-based).
    ///
    /// # Returns
    ///
    /// A `Task` that may produce a `Message` (e.g., if scrolling is needed).
    pub fn set_cursor(&mut self, line: usize, col: usize) -> Task<Message> {
        let line = line.min(self.buffer.line_count().saturating_sub(1));
        let line_len = self.buffer.line(line).chars().count();
        let col = col.min(line_len);

        self.cursors.set_single((line, col));
        // Programmatic jumps should end any drag gesture. Otherwise, a stale
        // drag state may let subsequent hover events move the caret away.
        self.is_dragging = false;

        // Reset blink
        self.last_blink = Instant::now();

        self.overlay_cache.clear();
        self.scroll_to_cursor()
    }

    /// Moves all cursors one step in `direction`.
    ///
    /// Visual lines are computed once and shared across all cursor movements.
    /// After moving, overlapping cursors are merged via `sort_and_merge`.
    pub(crate) fn move_cursor(&mut self, direction: ArrowDirection) {
        // Compute visual lines once — used by Up/Down movement for all cursors.
        // Reuse the memoized layout so that lines hidden by collapsed folds are
        // skipped during vertical navigation, exactly like in rendering.
        let visual_lines = self.visual_lines_cached(self.viewport_width);

        for cursor in self.cursors.as_mut_slice() {
            if let Some(new_pos) = compute_next_position(
                cursor.position,
                direction,
                &self.buffer,
                &visual_lines,
            ) {
                cursor.position = new_pos;
            }
        }

        // Deduplicate cursors that landed on the same position after movement.
        self.cursors.sort_and_merge();

        // Cursor movement affects only overlay visuals (caret, current-line highlight),
        // so avoid invalidating the expensive content cache.
        self.overlay_cache.clear();
    }

    /// Computes the cursor logical position (line, col) from a screen point.
    ///
    /// This method considers:
    /// 1. Whether the click is inside the gutter area.
    /// 2. Visual line mapping after wrapping.
    /// 3. CJK character widths (wide characters use FONT_SIZE, narrow use CHAR_WIDTH).
    pub(crate) fn calculate_cursor_from_point(
        &self,
        point: Point,
    ) -> Option<(usize, usize)> {
        // Account for gutter width
        if point.x < self.gutter_width() {
            return None; // Clicked in gutter
        }

        // Calculate visual line number - point.y is already in canvas coordinates
        let visual_line_idx = (point.y / self.line_height) as usize;

        // Reuse memoized wrapping result for hit-testing. This avoids recomputing
        // visual lines on every mouse move/drag.
        let visual_lines = self.visual_lines_cached(self.viewport_width);

        if visual_line_idx >= visual_lines.len() {
            // Clicked beyond last line - move to end of document
            let last_line = self.buffer.line_count().saturating_sub(1);
            let last_col = self.buffer.line_len(last_line);
            return Some((last_line, last_col));
        }

        let visual_line = &visual_lines[visual_line_idx];

        // Calculate column within the segment, accounting for horizontal scroll
        let x_in_text =
            point.x - self.gutter_width() - 5.0 + self.horizontal_scroll_offset;

        // Use correct width calculation for CJK support
        let line_content = self.buffer.line(visual_line.logical_line);

        let mut current_width = 0.0;
        let mut col_offset = 0;

        // Iterate the visual slice directly to avoid allocating a temporary String.
        for c in line_content
            .chars()
            .skip(visual_line.start_col)
            .take(visual_line.end_col - visual_line.start_col)
        {
            let char_width = super::measure_char_width(
                c,
                self.full_char_width,
                self.char_width,
            );

            if current_width + char_width / 2.0 > x_in_text {
                break;
            }
            current_width += char_width;
            col_offset += 1;
        }

        let col = visual_line.start_col + col_offset;
        Some((visual_line.logical_line, col))
    }

    /// Handles mouse clicks to position the cursor.
    ///
    /// Reuses `calculate_cursor_from_point` to compute the position and updates the cache.
    pub(crate) fn handle_mouse_click(&mut self, point: Point) {
        let before = self.cursors.primary_position();
        if let Some(pos) = self.calculate_cursor_from_point(point) {
            self.cursors.primary_mut().position = pos;
            if self.cursors.primary_position() != before {
                // Only clear overlay when the caret actually moved.
                self.overlay_cache.clear();
            }
        }
    }

    /// Returns a scroll command to make the cursor visible.
    pub(crate) fn scroll_to_cursor(&self) -> Task<Message> {
        // Reuse memoized wrapping result so repeated scroll computations do not
        // trigger repeated visual line calculation.
        let visual_lines = self.visual_lines_cached(self.viewport_width);

        let pos = self.cursors.primary_position();
        let cursor_visual =
            WrappingCalculator::logical_to_visual(&visual_lines, pos.0, pos.1);

        let cursor_y = if let Some(visual_idx) = cursor_visual {
            visual_idx as f32 * self.line_height
        } else {
            // Fallback to logical line if visual not found
            pos.0 as f32 * self.line_height
        };

        let viewport_top = self.viewport_scroll;
        let viewport_bottom = self.viewport_scroll + self.viewport_height;

        // Add margins to avoid cursor being exactly at edge
        let top_margin = self.line_height * 2.0;
        let bottom_margin = self.line_height * 2.0;

        // Calculate new vertical scroll position if cursor is outside visible area
        let new_v_scroll = if cursor_y < viewport_top + top_margin {
            // Cursor is above viewport - scroll up
            Some((cursor_y - top_margin).max(0.0))
        } else if cursor_y + self.line_height > viewport_bottom - bottom_margin
        {
            // Cursor is below viewport - scroll down
            Some(
                cursor_y + self.line_height + bottom_margin
                    - self.viewport_height,
            )
        } else {
            None
        };

        let vertical_task = if let Some(new_scroll) = new_v_scroll {
            scroll_to(
                self.scrollable_id.clone(),
                scrollable::AbsoluteOffset { x: 0.0, y: new_scroll },
            )
        } else {
            Task::none()
        };

        // Horizontal scroll: only when wrap is disabled
        let h_task = if !self.wrap_enabled {
            // Compute cursor content-space X position
            let cursor_content_x = if let Some(visual_idx) = cursor_visual {
                let vl = &visual_lines[visual_idx];
                let line_content = self.buffer.line(vl.logical_line);
                let prefix: String = line_content
                    .chars()
                    .skip(vl.start_col)
                    .take(pos.1.saturating_sub(vl.start_col))
                    .collect();
                self.gutter_width()
                    + 5.0
                    + measure_text_width(
                        &prefix,
                        self.full_char_width,
                        self.char_width,
                    )
            } else {
                self.gutter_width() + 5.0
            };

            let left_boundary = self.gutter_width() + self.char_width;
            let right_boundary = self.viewport_width - self.char_width * 2.0;
            let cursor_viewport_x =
                cursor_content_x - self.horizontal_scroll_offset;

            let new_h_offset = if cursor_viewport_x < left_boundary {
                (cursor_content_x - left_boundary).max(0.0)
            } else if cursor_viewport_x > right_boundary {
                cursor_content_x - right_boundary
            } else {
                self.horizontal_scroll_offset // no change
            };

            if (new_h_offset - self.horizontal_scroll_offset).abs() > 0.5 {
                scroll_to(
                    self.horizontal_scrollable_id.clone(),
                    scrollable::AbsoluteOffset { x: new_h_offset, y: 0.0 },
                )
            } else {
                Task::none()
            }
        } else {
            Task::none()
        };

        Task::batch([vertical_task, h_task])
    }

    /// Moves all cursors up by one page (approximately viewport height).
    pub(crate) fn page_up(&mut self) {
        let lines_per_page = (self.viewport_height / self.line_height) as usize;
        for cursor in self.cursors.as_mut_slice() {
            let new_line = cursor.position.0.saturating_sub(lines_per_page);
            let line_len = self.buffer.line_len(new_line);
            cursor.position = (new_line, cursor.position.1.min(line_len));
        }
        self.cursors.sort_and_merge();
        self.overlay_cache.clear();
    }

    /// Moves all cursors down by one page (approximately viewport height).
    pub(crate) fn page_down(&mut self) {
        let lines_per_page = (self.viewport_height / self.line_height) as usize;
        let max_line = self.buffer.line_count().saturating_sub(1);
        for cursor in self.cursors.as_mut_slice() {
            let new_line = (cursor.position.0 + lines_per_page).min(max_line);
            let line_len = self.buffer.line_len(new_line);
            cursor.position = (new_line, cursor.position.1.min(line_len));
        }
        self.cursors.sort_and_merge();
        self.overlay_cache.clear();
    }

    /// Handles mouse drag for text selection.
    ///
    /// Reuses `calculate_cursor_from_point` to compute the position and update selection end.
    pub(crate) fn handle_mouse_drag(&mut self, point: Point) {
        if let Some(pos) = self.calculate_cursor_from_point(point) {
            self.cursors.primary_mut().position = pos;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_movement() {
        let mut editor = CodeEditor::new("line1\nline2", "py");
        editor.move_cursor(ArrowDirection::Down);
        assert_eq!(editor.cursors.primary_position().0, 1);
        editor.move_cursor(ArrowDirection::Right);
        assert_eq!(editor.cursors.primary_position().1, 1);
    }

    #[test]
    fn test_page_down() {
        // Create editor with many lines
        let content = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = CodeEditor::new(&content, "py");

        editor.page_down();
        // Should move approximately 30 lines (600px / 20px per line)
        assert!(editor.cursors.primary_position().0 >= 25);
        assert!(editor.cursors.primary_position().0 <= 35);
    }

    #[test]
    fn test_page_up() {
        // Create editor with many lines
        let content = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = CodeEditor::new(&content, "py");

        // Move to line 50
        editor.cursors.primary_mut().position = (50, 0);
        editor.page_up();

        // Should move approximately 30 lines up
        assert!(editor.cursors.primary_position().0 >= 15);
        assert!(editor.cursors.primary_position().0 <= 25);
    }

    #[test]
    fn test_page_down_at_end() {
        let content =
            (0..10).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let mut editor = CodeEditor::new(&content, "py");

        editor.page_down();
        // Should be at last line (line 9)
        assert_eq!(editor.cursors.primary_position().0, 9);
    }

    #[test]
    fn test_page_up_at_start() {
        let content = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = CodeEditor::new(&content, "py");

        // Already at start
        editor.cursors.primary_mut().position = (0, 0);
        editor.page_up();
        assert_eq!(editor.cursors.primary_position().0, 0);
    }

    #[test]
    fn test_cursor_click_cjk() {
        use iced::Point;
        let mut editor = CodeEditor::new("你好", "txt");
        editor.set_line_numbers_enabled(false);
        // Disable folding so the gutter (line numbers + fold margin) is
        // zero-width; otherwise the fold margin shifts click coordinates.
        editor.set_folding_enabled(false);

        let full_char_width = editor.full_char_width();
        let half_width = full_char_width / 2.0;
        let padding = 5.0;

        // Assume each CJK character is `full_char_width` wide.
        // "你" is 0..full_char_width. "好" is full_char_width..2*full_char_width.
        //
        // Case 1: Click inside "你", at less than half its width.
        // Expect col 0
        editor
            .handle_mouse_click(Point::new((half_width - 2.0) + padding, 10.0));

        assert_eq!(editor.cursors.primary_position(), (0, 0));

        // Case 2: Click inside "你", at more than half its width.
        // Expect col 1
        editor
            .handle_mouse_click(Point::new((half_width + 2.0) + padding, 10.0));
        assert_eq!(editor.cursors.primary_position(), (0, 1));

        // Case 3: Click inside "好", at less than half its width.
        // "好" starts at full_char_width. Offset into "好" is < half_width.
        // Expect col 1 (start of "好")
        editor.handle_mouse_click(Point::new(
            (full_char_width + half_width - 2.0) + padding,
            10.0,
        ));
        assert_eq!(editor.cursors.primary_position(), (0, 1));

        // Case 4: Click inside "好", at more than half its width.
        // "好" starts at full_char_width. Offset into "好" is > half_width.
        // Expect col 2 (end of "好")
        editor.handle_mouse_click(Point::new(
            (full_char_width + half_width + 2.0) + padding,
            10.0,
        ));
        assert_eq!(editor.cursors.primary_position(), (0, 2));
    }

    #[test]
    fn test_multi_cursor_move_left() {
        let mut editor = CodeEditor::new("abc\ndef", "rs");
        editor.cursors.primary_mut().position = (0, 2);
        editor.cursors.add_cursor((1, 2));

        editor.move_cursor(ArrowDirection::Left);

        // Both cursors should have moved left by one
        let positions: Vec<(usize, usize)> =
            editor.cursors.iter().map(|c| c.position).collect();
        assert!(positions.contains(&(0, 1)));
        assert!(positions.contains(&(1, 1)));
    }

    #[test]
    fn test_multi_cursor_move_right() {
        let mut editor = CodeEditor::new("abc\ndef", "rs");
        editor.cursors.primary_mut().position = (0, 1);
        editor.cursors.add_cursor((1, 1));

        editor.move_cursor(ArrowDirection::Right);

        let positions: Vec<(usize, usize)> =
            editor.cursors.iter().map(|c| c.position).collect();
        assert!(positions.contains(&(0, 2)));
        assert!(positions.contains(&(1, 2)));
    }

    #[test]
    fn test_multi_cursor_move_deduplicates() {
        let mut editor = CodeEditor::new("abc", "rs");
        // Place two cursors adjacent, moving right will merge them
        editor.cursors.primary_mut().position = (0, 0);
        editor.cursors.add_cursor((0, 1));
        assert_eq!(editor.cursors.len(), 2);

        editor.move_cursor(ArrowDirection::Right);

        // Both moved right: (0,1) and (0,2). Still 2 distinct positions.
        assert_eq!(editor.cursors.len(), 2);
    }
}
