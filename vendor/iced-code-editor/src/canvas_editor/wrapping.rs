//! Line wrapping logic for the text editor.
//!
//! This module handles the calculation of visual lines from logical lines
//! when line wrapping is enabled. It supports both viewport-based wrapping
//! (dynamic) and fixed column wrapping.

use crate::text_buffer::TextBuffer;
use std::cmp::Ordering;
use std::collections::HashSet;

use super::compare_floats;

/// Represents a visual line segment in the editor.
///
/// When line wrapping is enabled, a single logical line may be split into
/// multiple visual line segments that are displayed sequentially.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualLine {
    /// Index of the logical line in the text buffer
    pub logical_line: usize,
    /// Segment index (0 for first segment, 1+ for wrapped segments)
    pub segment_index: usize,
    /// Start column in the logical line (inclusive)
    pub start_col: usize,
    /// End column in the logical line (exclusive)
    pub end_col: usize,
}

impl VisualLine {
    /// Creates a new visual line segment.
    ///
    /// # Arguments
    ///
    /// * `logical_line` - Index of the logical line
    /// * `segment_index` - Index of the segment within the line
    /// * `start_col` - Start column (inclusive)
    /// * `end_col` - End column (exclusive)
    pub fn new(
        logical_line: usize,
        segment_index: usize,
        start_col: usize,
        end_col: usize,
    ) -> Self {
        Self { logical_line, segment_index, start_col, end_col }
    }

    /// Returns whether this is the first segment of the logical line.
    pub fn is_first_segment(&self) -> bool {
        self.segment_index == 0
    }

    /// Returns the length of this segment in characters.
    pub fn len(&self) -> usize {
        self.end_col - self.start_col
    }
}

/// Calculator for line wrapping operations.
///
/// Handles the conversion between logical lines (as stored in the text buffer)
/// and visual lines (as displayed on screen with wrapping applied).
pub struct WrappingCalculator {
    /// Whether wrapping is enabled
    wrap_enabled: bool,
    /// Fixed wrap column (None = wrap at viewport width)
    wrap_column: Option<usize>,
    /// Full chat with for wide characters
    full_char_width: f32,
    /// Character width for narrow characters
    char_width: f32,
}

impl WrappingCalculator {
    /// Creates a new wrapping calculator.
    ///
    /// # Arguments
    ///
    /// * `wrap_enabled` - Whether line wrapping is enabled
    /// * `wrap_column` - Fixed wrap column, or None for viewport-based wrapping
    /// * `full_char_width` - Full chat with in pixels
    /// * `char_width` - Character width in pixels
    ///
    /// # Example
    ///
    /// ```ignore
    /// use iced_code_editor::canvas_editor::wrapping::WrappingCalculator;
    ///
    /// // Wrap at viewport width
    /// let calc = WrappingCalculator::new(true, None, 14.0, 8.4);
    ///
    /// // Wrap at 80 characters
    /// let calc = WrappingCalculator::new(true, Some(80), 14.0, 8.4);
    /// ```
    pub fn new(
        wrap_enabled: bool,
        wrap_column: Option<usize>,
        full_char_width: f32,
        char_width: f32,
    ) -> Self {
        Self { wrap_enabled, wrap_column, full_char_width, char_width }
    }

    /// Calculates all visual lines from the text buffer.
    ///
    /// # Arguments
    ///
    /// * `text_buffer` - The text buffer to wrap
    /// * `viewport_width` - Width of the viewport in pixels (used if wrap_column is None)
    /// * `gutter_width` - Width of the line number gutter in pixels (subtracted from available width)
    /// * `hidden` - Logical lines hidden by collapsed code folds; these produce no visual lines
    ///
    /// # Returns
    ///
    /// A vector of visual line segments
    pub fn calculate_visual_lines(
        &self,
        text_buffer: &TextBuffer,
        viewport_width: f32,
        gutter_width: f32,
        hidden: &HashSet<usize>,
    ) -> Vec<VisualLine> {
        if !self.wrap_enabled {
            // No wrapping: one visual line per (visible) logical line
            return (0..text_buffer.line_count())
                .filter(|line| !hidden.contains(line))
                .map(|line| {
                    VisualLine::new(line, 0, 0, text_buffer.line_len(line))
                })
                .collect();
        }

        // Calculate wrap width in pixels
        // If wrap_column is set, width is columns * character width.
        // Otherwise, use viewport width minus gutter width.
        let wrap_width_pixels = if let Some(cols) = self.wrap_column {
            cols as f32 * self.char_width
        } else {
            (viewport_width - gutter_width).max(self.char_width)
        };

        let mut visual_lines = Vec::new();

        for logical_line in 0..text_buffer.line_count() {
            if hidden.contains(&logical_line) {
                continue; // Hidden by a collapsed fold: emit no visual lines.
            }

            let line_content = text_buffer.line(logical_line);

            if line_content.is_empty() {
                visual_lines.push(VisualLine::new(logical_line, 0, 0, 0));
                continue;
            }

            let mut segment_index = 0;
            let mut current_width = 0.0;
            let mut current_segment_start_col = 0;

            for (i, c) in line_content.chars().enumerate() {
                // Compute pixel width for the current character
                let char_width = super::measure_char_width(
                    c,
                    self.full_char_width,
                    self.char_width,
                );

                // If adding the current character exceeds wrap width, wrap at the previous char.
                // Ensure at least one character per segment even if a single char exceeds wrap_width.
                // Use epsilon to handle floating-point error.
                if compare_floats(current_width + char_width, wrap_width_pixels)
                    == Ordering::Greater
                    && i > current_segment_start_col
                {
                    // Create a new visual segment
                    visual_lines.push(VisualLine::new(
                        logical_line,
                        segment_index,
                        current_segment_start_col,
                        i, // end_col is exclusive (current char belongs to next line)
                    ));

                    segment_index += 1;
                    current_segment_start_col = i;
                    current_width = 0.0;
                }

                current_width += char_width;
            }

            // Push remaining segment
            // Add the last segment of the logical line
            visual_lines.push(VisualLine::new(
                logical_line,
                segment_index,
                current_segment_start_col,
                line_content.chars().count(),
            ));
        }

        visual_lines
    }

    /// Converts a logical position to a visual line index.
    ///
    /// # Arguments
    ///
    /// * `visual_lines` - Pre-calculated visual lines
    /// * `line` - Logical line index
    /// * `col` - Column in the logical line
    ///
    /// # Returns
    ///
    /// The visual line index containing this position
    pub fn logical_to_visual(
        visual_lines: &[VisualLine],
        line: usize,
        col: usize,
    ) -> Option<usize> {
        visual_lines
            .iter()
            .position(|vl| {
                vl.logical_line == line
                    && col >= vl.start_col
                    && col < vl.end_col
            })
            .or_else(|| {
                // Handle cursor at end of line (col == end_col)
                visual_lines.iter().position(|vl| {
                    vl.logical_line == line && col == vl.end_col && {
                        // Check if this is the last segment for this line
                        visual_lines
                            .iter()
                            .filter(|v| v.logical_line == line)
                            .max_by_key(|v| v.segment_index)
                            .map(|v| v.segment_index == vl.segment_index)
                            .unwrap_or(false)
                    }
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_editor::{CHAR_WIDTH, FONT_SIZE};

    #[test]
    fn test_no_wrap_when_disabled() {
        let buffer = TextBuffer::new("line 1\nline 2\nline 3");
        let calc = WrappingCalculator::new(false, None, FONT_SIZE, CHAR_WIDTH);
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &HashSet::new());

        assert_eq!(visual_lines.len(), 3);
        assert_eq!(visual_lines[0].logical_line, 0);
        assert_eq!(visual_lines[1].logical_line, 1);
        assert_eq!(visual_lines[2].logical_line, 2);
    }

    #[test]
    fn test_hidden_lines_are_skipped() {
        let buffer = TextBuffer::new("line 1\nline 2\nline 3\nline 4");
        let calc = WrappingCalculator::new(false, None, FONT_SIZE, CHAR_WIDTH);
        // Hide logical lines 1 and 2 (e.g. a collapsed fold).
        let hidden: HashSet<usize> = [1, 2].into_iter().collect();
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &hidden);

        // Only lines 0 and 3 remain visible.
        assert_eq!(visual_lines.len(), 2);
        assert_eq!(visual_lines[0].logical_line, 0);
        assert_eq!(visual_lines[1].logical_line, 3);
    }

    #[test]
    fn test_hidden_lines_are_skipped_when_wrapping() {
        let buffer = TextBuffer::new("aaaa\nbbbb\ncccc");
        let calc =
            WrappingCalculator::new(true, Some(80), FONT_SIZE, CHAR_WIDTH);
        let hidden: HashSet<usize> = [1].into_iter().collect();
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &hidden);

        assert_eq!(visual_lines.len(), 2);
        assert_eq!(visual_lines[0].logical_line, 0);
        assert_eq!(visual_lines[1].logical_line, 2);
    }

    #[test]
    fn test_wrap_at_fixed_column() {
        let buffer =
            TextBuffer::new("this is a very long line that should be wrapped");
        let calc =
            WrappingCalculator::new(true, Some(10), FONT_SIZE, CHAR_WIDTH);
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &HashSet::new());

        // Line is 47 chars, should wrap into 5 segments (10+10+10+10+7)
        assert_eq!(visual_lines.len(), 5);
        assert_eq!(visual_lines[0].start_col, 0);
        assert_eq!(visual_lines[0].end_col, 10);
        assert_eq!(visual_lines[1].start_col, 10);
        assert_eq!(visual_lines[1].end_col, 20);
        assert_eq!(visual_lines[4].start_col, 40);
        assert_eq!(visual_lines[4].end_col, 47);
    }

    #[test]
    fn test_logical_to_visual_mapping() {
        let buffer =
            TextBuffer::new("short\nthis is a very long line that wraps\nend");
        let calc =
            WrappingCalculator::new(true, Some(15), FONT_SIZE, CHAR_WIDTH);
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &HashSet::new());

        // First line (short) - no wrap
        assert_eq!(
            WrappingCalculator::logical_to_visual(&visual_lines, 0, 0),
            Some(0)
        );

        // Second line (long) - wraps
        assert_eq!(
            WrappingCalculator::logical_to_visual(&visual_lines, 1, 0),
            Some(1)
        );
        assert_eq!(
            WrappingCalculator::logical_to_visual(&visual_lines, 1, 14),
            Some(1)
        );
        assert_eq!(
            WrappingCalculator::logical_to_visual(&visual_lines, 1, 15),
            Some(2)
        );
        assert_eq!(
            WrappingCalculator::logical_to_visual(&visual_lines, 1, 30),
            Some(3)
        );
    }

    #[test]
    fn test_wrap_empty_lines() {
        let buffer = TextBuffer::new("line1\n\nline3");
        let calc =
            WrappingCalculator::new(true, Some(10), FONT_SIZE, CHAR_WIDTH);
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &HashSet::new());

        assert_eq!(visual_lines.len(), 3);
        assert_eq!(visual_lines[1].logical_line, 1);
        assert_eq!(visual_lines[1].len(), 0);
    }

    #[test]
    fn test_wrap_very_long_line() {
        let long_text = "a".repeat(100);
        let buffer = TextBuffer::new(&long_text);
        let calc =
            WrappingCalculator::new(true, Some(20), FONT_SIZE, CHAR_WIDTH);
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &HashSet::new());

        // 100 chars / 20 per line = 5 lines
        assert_eq!(visual_lines.len(), 5);
        assert!(visual_lines.iter().all(|vl| vl.logical_line == 0));
    }

    #[test]
    fn test_visual_line_is_first_segment() {
        let vl1 = VisualLine::new(0, 0, 0, 10);
        let vl2 = VisualLine::new(0, 1, 10, 20);

        assert!(vl1.is_first_segment());
        assert!(!vl2.is_first_segment());
    }

    #[test]
    fn test_wrap_cjk() {
        // CJK characters are wide (FONT_SIZE = 14.0)
        // Latin characters are narrow (CHAR_WIDTH = 8.4)
        // Wrap width = 10 columns * 8.4 = 84.0 pixels

        // 6 CJK characters = 6 * 14.0 = 84.0 pixels. Matches exactly.
        let text = "你好世界你好"; // 6 chars
        let buffer = TextBuffer::new(text);
        let calc =
            WrappingCalculator::new(true, Some(10), FONT_SIZE, CHAR_WIDTH); // 84.0 px
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &HashSet::new());

        assert_eq!(visual_lines.len(), 1);
        assert_eq!(visual_lines[0].len(), 6);

        // 7 CJK characters = 7 * 14.0 = 98.0 pixels.
        // Wrap width is 84.0 pixels.
        // First 6 chars = 6 * 14.0 = 84.0 pixels. They fit exactly (84.0 <= 84.0).
        // 7th char adds 14.0, total 98.0 > 84.0. Triggers wrap before 7th char.
        let text = "你好世界你好世"; // 7 chars
        let buffer = TextBuffer::new(text);
        let visual_lines =
            calc.calculate_visual_lines(&buffer, 800.0, 60.0, &HashSet::new());

        assert_eq!(visual_lines.len(), 2);
        assert_eq!(visual_lines[0].len(), 6); // First 6 fit
        assert_eq!(visual_lines[1].len(), 1); // 7th wraps
        assert_eq!(visual_lines[1].start_col, 6); // Starts at 7th char (index 6)
    }
}
