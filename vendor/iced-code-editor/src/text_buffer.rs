//! Efficient text buffer for storing and manipulating editor content.
//!
//! This module provides a line-based text buffer optimized for:
//! - Fast line access for virtual scrolling
//! - Efficient insertions and deletions
//! - Memory-efficient storage

/// A line-based text buffer optimized for editor operations.
///
/// Stores text as a vector of lines for fast random access needed by virtual scrolling.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    /// Lines of text (without newline characters)
    lines: Vec<String>,
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
        let lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };

        Self { lines }
    }

    /// Returns the number of lines in the buffer.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
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
        self.lines.get(index).map_or("", |s| s.as_str())
    }

    /// Inserts a character at the specified position.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    /// * `column` - Column position (UTF-8 character index)
    /// * `ch` - Character to insert
    pub fn insert_char(&mut self, line: usize, column: usize, ch: char) {
        if line >= self.lines.len() {
            return;
        }

        let line_str = &mut self.lines[line];
        let byte_pos = Self::char_to_byte_index(line_str, column);
        line_str.insert(byte_pos, ch);
    }

    /// Inserts a newline at the specified position, splitting the line.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index
    /// * `column` - Column position where to split
    pub fn insert_newline(&mut self, line: usize, column: usize) {
        if line >= self.lines.len() {
            return;
        }

        let line_str = self.lines[line].clone();
        let byte_pos = Self::char_to_byte_index(&line_str, column);

        let left = line_str[..byte_pos].to_string();
        let right = line_str[byte_pos..].to_string();

        self.lines[line] = left;
        self.lines.insert(line + 1, right);
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
            if line < self.lines.len() {
                let line_str = &mut self.lines[line];
                let byte_pos = Self::char_to_byte_index(line_str, column);
                if byte_pos > 0 {
                    let char_start =
                        Self::char_to_byte_index(line_str, column - 1);
                    line_str.drain(char_start..byte_pos);
                }
            }
            false
        } else if line > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(line);
            self.lines[line - 1].push_str(&current_line);
            true
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
        if line >= self.lines.len() {
            return;
        }

        let line_str = &mut self.lines[line];
        let char_count = line_str.chars().count();

        if column < char_count {
            // Delete character at cursor
            let byte_pos = Self::char_to_byte_index(line_str, column);
            let next_byte_pos = Self::char_to_byte_index(line_str, column + 1);
            line_str.drain(byte_pos..next_byte_pos);
        } else if line + 1 < self.lines.len() {
            // Merge with next line
            let next_line = self.lines.remove(line + 1);
            self.lines[line].push_str(&next_line);
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
        if line >= self.lines.len() {
            return;
        }

        let line_str = &mut self.lines[line];
        let start_byte = Self::char_to_byte_index(line_str, col_start);
        let end_byte = Self::char_to_byte_index(line_str, col_start + length);

        line_str.replace_range(start_byte..end_byte, new_text);
    }

    /// Converts a character index to a byte index in a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The string
    /// * `char_index` - Character index
    ///
    /// # Returns
    ///
    /// The byte index
    fn char_to_byte_index(s: &str, char_index: usize) -> usize {
        s.char_indices().nth(char_index).map_or(s.len(), |(idx, _)| idx)
    }

    /// Returns the entire buffer content as a single string.
    #[must_use]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.lines.join("\n")
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
        self.lines.get(line).map_or(0, |s| s.chars().count())
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
}
