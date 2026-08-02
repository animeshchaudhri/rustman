//! Efficient text buffer for storing and manipulating editor content.
//!
//! This module provides a line-based text buffer optimized for:
//! - Fast line access for virtual scrolling
//! - Efficient insertions and deletions
//! - Memory-efficient storage

use crate::text_utils::{char_range_to_byte_range, char_to_byte_index};

/// A line-based text buffer optimized for editor operations.
///
/// Lines are stored around a movable gap:
///
/// - `lines_before` is in document order.
/// - `lines_after` is in reverse document order.
///
/// Random line reads remain O(1). Once the gap has moved to an edit location,
/// inserting/removing nearby lines is O(1) instead of shifting the entire tail
/// of a `Vec<String>`. This mirrors the locality principle behind piece-table
/// editors while preserving the editor's existing borrowed `&str` line API.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    /// Lines before the gap, in document order.
    lines_before: Vec<String>,
    /// Lines after the gap, in reverse document order.
    lines_after: Vec<String>,
}

impl TextBuffer {
    /// Creates a new text buffer from a string.
    ///
    /// # Arguments
    ///
    /// * `content` - Initial text content (will be split into lines)
    ///
    /// # Returns
    ///
    /// A new `TextBuffer` instance
    pub fn new(content: &str) -> Self {
        let mut lines_after: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        lines_after.reverse();

        Self { lines_before: Vec::new(), lines_after }
    }

    /// Returns the number of lines in the buffer.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines_before.len() + self.lines_after.len()
    }

    /// Returns a reference to a specific line.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based line index
    ///
    /// # Returns
    ///
    /// The line content, or an empty string if index is out of bounds
    #[must_use]
    pub fn line(&self, index: usize) -> &str {
        if let Some(line) = self.lines_before.get(index) {
            return line;
        }

        let relative_index = index.saturating_sub(self.lines_before.len());
        self.lines_after
            .len()
            .checked_sub(relative_index.saturating_add(1))
            .and_then(|after_index| self.lines_after.get(after_index))
            .map_or("", String::as_str)
    }

    /// Moves the line gap to the logical boundary at `index`.
    fn move_gap_to(&mut self, index: usize) {
        let target = index.min(self.line_count());
        while self.lines_before.len() < target {
            let Some(line) = self.lines_after.pop() else { break };
            self.lines_before.push(line);
        }
        while self.lines_before.len() > target {
            let Some(line) = self.lines_before.pop() else { break };
            self.lines_after.push(line);
        }
    }

    /// Returns a mutable line after moving the gap immediately after it.
    fn line_mut(&mut self, index: usize) -> Option<&mut String> {
        if index >= self.line_count() {
            return None;
        }
        self.move_gap_to(index.saturating_add(1));
        self.lines_before.last_mut()
    }

    /// Iterates all lines in document order.
    fn iter_lines(&self) -> impl Iterator<Item = &String> {
        self.lines_before.iter().chain(self.lines_after.iter().rev())
    }

    /// Inserts a character at the specified position.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    /// * `column` - Column position (UTF-8 character index)
    /// * `ch` - Character to insert
    pub fn insert_char(&mut self, line: usize, column: usize, ch: char) {
        let Some(line_str) = self.line_mut(line) else { return };
        let byte_pos = char_to_byte_index(line_str, column);
        line_str.insert(byte_pos, ch);
    }

    /// Inserts a newline at the specified position, splitting the line.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    /// * `column` - Column position where to split
    pub fn insert_newline(&mut self, line: usize, column: usize) {
        let Some(line_str) = self.line_mut(line) else { return };
        let byte_pos = char_to_byte_index(line_str, column);
        let right = line_str.split_off(byte_pos);
        // `line_mut` leaves the gap after `line`; pushing here inserts the new
        // right half directly after it without moving the document tail.
        self.lines_before.push(right);
    }

    /// Deletes a character before the cursor (backspace).
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    /// * `column` - Column position
    ///
    /// # Returns
    ///
    /// `true` if a line merge occurred, `false` otherwise
    pub fn delete_char(&mut self, line: usize, column: usize) -> bool {
        if column > 0 {
            // Delete character in current line
            if let Some(line_str) = self.line_mut(line) {
                let byte_pos = char_to_byte_index(line_str, column);
                if byte_pos > 0 {
                    let char_start = char_to_byte_index(line_str, column - 1);
                    line_str.drain(char_start..byte_pos);
                }
            }
            false
        } else if line > 0 && line < self.line_count() {
            // Merge with previous line
            self.move_gap_to(line.saturating_add(1));
            if let Some(current_line) = self.lines_before.pop()
                && let Some(previous_line) = self.lines_before.last_mut()
            {
                previous_line.push_str(&current_line);
                return true;
            }
            false
        } else {
            false
        }
    }

    /// Deletes a character at the cursor (delete key).
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    /// * `column` - Column position
    pub fn delete_forward(&mut self, line: usize, column: usize) {
        if line >= self.line_count() {
            return;
        }

        let char_count = self.line(line).chars().count();

        if column < char_count {
            // Delete character at cursor
            if let Some(line_str) = self.line_mut(line) {
                let byte_pos = char_to_byte_index(line_str, column);
                let next_byte_pos = char_to_byte_index(line_str, column + 1);
                line_str.drain(byte_pos..next_byte_pos);
            }
        } else if line + 1 < self.line_count() {
            // Merge with next line
            self.move_gap_to(line.saturating_add(1));
            if let Some(next_line) = self.lines_after.pop()
                && let Some(line_str) = self.lines_before.last_mut()
            {
                line_str.push_str(&next_line);
            }
        }
    }

    /// Replaces a range of characters in a line with new text.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    /// * `col_start` - Column position to start replacing
    /// * `length` - Number of characters to replace
    /// * `new_text` - The text to insert
    pub fn replace_range(
        &mut self,
        line: usize,
        col_start: usize,
        length: usize,
        new_text: &str,
    ) {
        let Some(line_str) = self.line_mut(line) else { return };
        let (start_byte, end_byte) =
            char_range_to_byte_range(line_str, col_start, col_start + length);

        line_str.replace_range(start_byte..end_byte, new_text);
    }

    /// Returns the entire buffer content as a single string.
    #[must_use]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let line_count = self.line_count();
        let content_len = self.iter_lines().map(String::len).sum::<usize>()
            + line_count.saturating_sub(1);
        let mut content = String::with_capacity(content_len);
        for (index, line) in self.iter_lines().enumerate() {
            if index > 0 {
                content.push('\n');
            }
            content.push_str(line);
        }
        content
    }

    /// Returns a contiguous logical-line range as replacement text.
    ///
    /// When `end_exclusive` is before the end of the buffer, the returned text
    /// includes the newline that separates the range from the following line.
    /// This form maps directly to an LSP range ending at column zero.
    pub(crate) fn line_range_to_string(
        &self,
        start: usize,
        end_exclusive: usize,
    ) -> String {
        let line_count = self.line_count();
        let start = start.min(line_count);
        let end_exclusive = end_exclusive.min(line_count).max(start);
        let mut content = String::new();
        for line_index in start..end_exclusive {
            if line_index > start {
                content.push('\n');
            }
            content.push_str(self.line(line_index));
        }
        if end_exclusive < line_count && start < end_exclusive {
            content.push('\n');
        }
        content
    }

    /// Returns the character count of a specific line.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    ///
    /// # Returns
    ///
    /// The number of characters in the line
    #[must_use]
    pub fn line_len(&self, line: usize) -> usize {
        self.line(line).chars().count()
    }

    /// Inserts a full line at the given index, shifting following lines down.
    ///
    /// The new line is inserted *before* the line currently at `index`. An
    /// `index` equal to (or greater than) the line count appends the line at
    /// the end of the buffer.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based position where the line is inserted
    /// * `content` - The line content (without trailing newline)
    pub fn insert_line(&mut self, index: usize, content: String) {
        self.move_gap_to(index);
        self.lines_before.push(content);
    }

    /// Removes the line at the given index, returning its content.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index of the line to remove
    ///
    /// # Returns
    ///
    /// The removed line content, or `None` if `index` is out of bounds
    pub fn remove_line(&mut self, index: usize) -> Option<String> {
        if index >= self.line_count() {
            return None;
        }
        self.move_gap_to(index);
        self.lines_after.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = TextBuffer::new("line1\nline2\nline3");
        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line(0), "line1");
        assert_eq!(buffer.line(1), "line2");
        assert_eq!(buffer.line(2), "line3");
    }

    #[test]
    fn test_empty_buffer() {
        let buffer = TextBuffer::new("");
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.line(0), "");
    }

    #[test]
    fn test_insert_char() {
        let mut buffer = TextBuffer::new("hello");
        buffer.insert_char(0, 5, '!');
        assert_eq!(buffer.line(0), "hello!");
    }

    #[test]
    fn test_insert_newline() {
        let mut buffer = TextBuffer::new("hello world");
        buffer.insert_newline(0, 5);
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line(0), "hello");
        assert_eq!(buffer.line(1), " world");
    }

    #[test]
    fn test_delete_char() {
        let mut buffer = TextBuffer::new("hello");
        let merged = buffer.delete_char(0, 5);
        assert!(!merged);
        assert_eq!(buffer.line(0), "hell");
    }

    #[test]
    fn test_delete_char_merge() {
        let mut buffer = TextBuffer::new("line1\nline2");
        let merged = buffer.delete_char(1, 0);
        assert!(merged);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.line(0), "line1line2");
    }

    #[test]
    fn test_to_string() {
        let buffer = TextBuffer::new("line1\nline2\nline3");
        assert_eq!(buffer.to_string(), "line1\nline2\nline3");
    }

    #[test]
    fn test_replace_range() {
        let mut buffer = TextBuffer::new("hello world");
        // Replace "world" with "rust"
        buffer.replace_range(0, 6, 5, "rust");
        assert_eq!(buffer.line(0), "hello rust");

        // Replace "hello" with "hi"
        buffer.replace_range(0, 0, 5, "hi");
        assert_eq!(buffer.line(0), "hi rust");

        // Insert at end
        buffer.replace_range(0, 7, 0, "!");
        assert_eq!(buffer.line(0), "hi rust!");
    }

    #[test]
    fn test_insert_line() {
        let mut buffer = TextBuffer::new("line1\nline3");

        // Insert in the middle
        buffer.insert_line(1, "line2".to_string());
        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line(0), "line1");
        assert_eq!(buffer.line(1), "line2");
        assert_eq!(buffer.line(2), "line3");

        // Insert at the start
        buffer.insert_line(0, "line0".to_string());
        assert_eq!(buffer.line(0), "line0");

        // Insert at the end (index beyond bounds is clamped)
        buffer.insert_line(99, "last".to_string());
        assert_eq!(buffer.line(buffer.line_count() - 1), "last");
    }

    #[test]
    fn test_remove_line() {
        let mut buffer = TextBuffer::new("line1\nline2\nline3");

        // Remove from the middle
        assert_eq!(buffer.remove_line(1), Some("line2".to_string()));
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line(0), "line1");
        assert_eq!(buffer.line(1), "line3");

        // Out of bounds returns None
        assert_eq!(buffer.remove_line(5), None);
        assert_eq!(buffer.line_count(), 2);
    }

    #[test]
    fn test_gap_buffer_preserves_order_across_distant_edits() {
        let content = (0..100)
            .map(|line| format!("line-{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut buffer = TextBuffer::new(&content);

        buffer.insert_line(50, "middle".to_string());
        buffer.insert_char(50, 6, '!');
        buffer.insert_line(0, "first".to_string());
        buffer.insert_line(buffer.line_count(), "last".to_string());

        assert_eq!(buffer.line(0), "first");
        assert_eq!(buffer.line(51), "middle!");
        assert_eq!(buffer.line(52), "line-50");
        assert_eq!(buffer.line(buffer.line_count() - 1), "last");

        assert_eq!(buffer.remove_line(51), Some("middle!".to_string()));
        assert_eq!(buffer.remove_line(0), Some("first".to_string()));
        assert_eq!(buffer.line(50), "line-50");
        assert_eq!(
            buffer.remove_line(buffer.line_count() - 1),
            Some("last".to_string())
        );
        assert_eq!(buffer.to_string(), content);
    }

    #[test]
    fn test_line_range_to_string_matches_line_boundary_replacement() {
        let buffer = TextBuffer::new("zero\none\ntwo\nthree");
        assert_eq!(buffer.line_range_to_string(1, 3), "one\ntwo\n");
        assert_eq!(buffer.line_range_to_string(2, 4), "two\nthree");
        assert_eq!(buffer.line_range_to_string(99, 100), "");
    }
}
