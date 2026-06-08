//! Command pattern implementation for undo/redo functionality.
//!
//! This module provides a trait-based command system that allows all text
//! modifications to be recorded and reversed, enabling robust undo/redo support.

use crate::text_buffer::TextBuffer;

/// Trait for reversible editor commands.
///
/// All text modifications should implement this trait to support undo/redo.
/// Commands must be both executable and reversible.
pub trait Command: Send + std::fmt::Debug {
    /// Executes the command, modifying the buffer and cursor.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The text buffer to modify
    /// * `cursor` - The cursor position (will be updated)
    fn execute(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize));

    /// Reverses the command, restoring previous state.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The text buffer to modify
    /// * `cursor` - The cursor position (will be restored)
    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize));
}

/// Command for inserting a single character.
#[derive(Debug, Clone)]
pub struct InsertCharCommand {
    line: usize,
    col: usize,
    ch: char,
    cursor_before: (usize, usize),
    cursor_after: (usize, usize),
}

impl InsertCharCommand {
    /// Creates a new insert character command.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index where to insert
    /// * `col` - Column position where to insert
    /// * `ch` - Character to insert
    /// * `cursor` - Current cursor position
    pub fn new(
        line: usize,
        col: usize,
        ch: char,
        cursor: (usize, usize),
    ) -> Self {
        Self {
            line,
            col,
            ch,
            cursor_before: cursor,
            cursor_after: (line, col + 1),
        }
    }
}

impl Command for InsertCharCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        buffer.insert_char(self.line, self.col, self.ch);
        *cursor = self.cursor_after;
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        // Delete the character we inserted
        buffer.delete_forward(self.line, self.col);
        *cursor = self.cursor_before;
    }
}

/// Command for deleting a character (backspace).
#[derive(Debug, Clone)]
pub struct DeleteCharCommand {
    line: usize,
    col: usize,
    deleted_char: Option<char>,
    merged_line: bool,
    merged_content: Option<String>,
    cursor_before: (usize, usize),
    cursor_after: (usize, usize),
}

impl DeleteCharCommand {
    /// Creates a new delete character command.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The text buffer (to read the character being deleted)
    /// * `line` - Line index
    /// * `col` - Column position
    /// * `cursor` - Current cursor position
    pub fn new(
        buffer: &TextBuffer,
        line: usize,
        col: usize,
        cursor: (usize, usize),
    ) -> Self {
        let (deleted_char, merged_line, merged_content, cursor_after) = if col
            > 0
        {
            // Deleting character before cursor
            let line_str = buffer.line(line);
            let ch = line_str.chars().nth(col - 1);
            (ch, false, None, (line, col - 1))
        } else if line > 0 {
            // Merging with previous line
            let prev_line_len = buffer.line_len(line - 1);
            let current_line_content = buffer.line(line).to_string();
            (None, true, Some(current_line_content), (line - 1, prev_line_len))
        } else {
            // At beginning of document, nothing to delete
            (None, false, None, cursor)
        };

        Self {
            line,
            col,
            deleted_char,
            merged_line,
            merged_content,
            cursor_before: cursor,
            cursor_after,
        }
    }
}

impl Command for DeleteCharCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        buffer.delete_char(self.line, self.col);
        *cursor = self.cursor_after;
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        if self.merged_line {
            // Restore the line that was merged
            if let Some(content) = &self.merged_content {
                buffer.insert_newline(self.cursor_after.0, self.cursor_after.1);
                // Replace the new line with the original content
                for (i, ch) in content.chars().enumerate() {
                    buffer.insert_char(self.line, i, ch);
                }
            }
        } else if let Some(ch) = self.deleted_char {
            // Re-insert the deleted character
            buffer.insert_char(self.line, self.col - 1, ch);
        }
        *cursor = self.cursor_before;
    }
}

/// Command for deleting forward (Delete key).
#[derive(Debug, Clone)]
pub struct DeleteForwardCommand {
    line: usize,
    col: usize,
    deleted_char: Option<char>,
    merged_next_line: bool,
    next_line_content: Option<String>,
    cursor_before: (usize, usize),
}

impl DeleteForwardCommand {
    /// Creates a new delete forward command.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The text buffer
    /// * `line` - Line index
    /// * `col` - Column position
    /// * `cursor` - Current cursor position
    pub fn new(
        buffer: &TextBuffer,
        line: usize,
        col: usize,
        cursor: (usize, usize),
    ) -> Self {
        let line_len = buffer.line_len(line);
        let (deleted_char, merged_next_line, next_line_content) =
            if col < line_len {
                // Deleting character at cursor
                let ch = buffer.line(line).chars().nth(col);
                (ch, false, None)
            } else if line + 1 < buffer.line_count() {
                // Merging with next line
                let next_content = buffer.line(line + 1).to_string();
                (None, true, Some(next_content))
            } else {
                // At end of document
                (None, false, None)
            };

        Self {
            line,
            col,
            deleted_char,
            merged_next_line,
            next_line_content,
            cursor_before: cursor,
        }
    }
}

impl Command for DeleteForwardCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        buffer.delete_forward(self.line, self.col);
        *cursor = self.cursor_before; // Cursor doesn't move on delete forward
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        if self.merged_next_line {
            // Restore the newline and next line
            if let Some(content) = &self.next_line_content {
                buffer.insert_newline(self.line, self.col);
                // The content is already in the next line after insert_newline
                // We need to clear it and restore the original
                let next_line_len = buffer.line_len(self.line + 1);
                for _ in 0..next_line_len {
                    buffer.delete_forward(self.line + 1, 0);
                }
                for (i, ch) in content.chars().enumerate() {
                    buffer.insert_char(self.line + 1, i, ch);
                }
            }
        } else if let Some(ch) = self.deleted_char {
            // Re-insert the deleted character
            buffer.insert_char(self.line, self.col, ch);
        }
        *cursor = self.cursor_before;
    }
}

/// Command for inserting a newline, optionally with auto-indentation.
///
/// When `indent` is non-empty, the indentation string is inserted at the
/// beginning of the new line after the split, and the cursor is placed
/// after the indent. Undo removes the indent chars then joins the lines.
#[derive(Debug, Clone)]
pub struct InsertNewlineCommand {
    line: usize,
    col: usize,
    cursor_before: (usize, usize),
    cursor_after: (usize, usize),
    indent: String,
}

impl InsertNewlineCommand {
    /// Creates a new insert newline command with auto-indentation.
    ///
    /// The `indent` string (leading whitespace of the current line) is
    /// inserted at the start of the new line, and the cursor is placed
    /// after it.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index where to insert
    /// * `col` - Column position where to split
    /// * `cursor` - Current cursor position
    /// * `indent` - Leading whitespace to copy to the new line
    pub fn with_indent(
        line: usize,
        col: usize,
        cursor: (usize, usize),
        indent: String,
    ) -> Self {
        let indent_len = indent.chars().count();
        Self {
            line,
            col,
            cursor_before: cursor,
            cursor_after: (line + 1, indent_len),
            indent,
        }
    }
}

impl Command for InsertNewlineCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        buffer.insert_newline(self.line, self.col);
        for (i, c) in self.indent.chars().enumerate() {
            buffer.insert_char(self.line + 1, i, c);
        }
        *cursor = self.cursor_after;
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        // Remove indent chars inserted at start of new line
        for _ in 0..self.indent.chars().count() {
            buffer.delete_forward(self.line + 1, 0);
        }
        // Merge the two lines back together
        if self.line + 1 < buffer.line_count() {
            buffer.delete_char(self.line + 1, 0);
        }
        *cursor = self.cursor_before;
    }
}

/// Command for inserting multiple characters (paste).
#[derive(Debug, Clone)]
pub struct InsertTextCommand {
    line: usize,
    col: usize,
    text: String,
    cursor_before: (usize, usize),
    cursor_after: (usize, usize),
}

impl InsertTextCommand {
    /// Creates a new insert text command.
    ///
    /// # Arguments
    ///
    /// * `line` - Line index where to insert
    /// * `col` - Column position where to insert
    /// * `text` - Text to insert
    /// * `cursor` - Current cursor position
    pub fn new(
        line: usize,
        col: usize,
        text: String,
        cursor: (usize, usize),
    ) -> Self {
        // Calculate final cursor position
        let lines: Vec<&str> = text.split('\n').collect();
        let cursor_after = if lines.len() == 1 {
            (line, col + text.chars().count())
        } else {
            let last_line_len = lines.last().map_or(0, |l| l.chars().count());
            (line + lines.len() - 1, last_line_len)
        };

        Self { line, col, text, cursor_before: cursor, cursor_after }
    }
}

impl Command for InsertTextCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        let mut current_line = self.line;
        let mut current_col = self.col;

        for ch in self.text.chars() {
            if ch == '\n' {
                buffer.insert_newline(current_line, current_col);
                current_line += 1;
                current_col = 0;
            } else {
                buffer.insert_char(current_line, current_col, ch);
                current_col += 1;
            }
        }

        *cursor = self.cursor_after;
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        // Delete characters in reverse
        let mut current_line = self.cursor_after.0;
        let mut current_col = self.cursor_after.1;

        for ch in self.text.chars().rev() {
            if ch == '\n' {
                // Merge lines
                if current_line > 0 {
                    let prev_line_len = buffer.line_len(current_line - 1);
                    buffer.delete_char(current_line, 0);
                    current_line -= 1;
                    current_col = prev_line_len;
                }
            } else {
                // Delete character
                if current_col > 0 {
                    buffer.delete_char(current_line, current_col);
                    current_col -= 1;
                }
            }
        }

        *cursor = self.cursor_before;
    }
}

/// Command for deleting a range of text (selection).
#[derive(Debug, Clone)]
pub struct DeleteRangeCommand {
    start: (usize, usize),
    end: (usize, usize),
    deleted_text: String,
    cursor_before: (usize, usize),
}

impl DeleteRangeCommand {
    /// Creates a new delete range command.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The text buffer
    /// * `start` - Start position (line, col)
    /// * `end` - End position (line, col)
    /// * `cursor` - Current cursor position
    pub fn new(
        buffer: &TextBuffer,
        start: (usize, usize),
        end: (usize, usize),
        cursor: (usize, usize),
    ) -> Self {
        // Extract the text being deleted
        let mut deleted_text = String::new();

        if start.0 == end.0 {
            // Single line
            let line = buffer.line(start.0);
            let chars: Vec<char> = line.chars().collect();
            for ch in chars.iter().skip(start.1).take(
                end.1
                    .saturating_sub(start.1)
                    .min(chars.len().saturating_sub(start.1)),
            ) {
                deleted_text.push(*ch);
            }
        } else {
            // Multiple lines
            for line_idx in start.0..=end.0 {
                let line = buffer.line(line_idx);
                let chars: Vec<char> = line.chars().collect();

                if line_idx == start.0 {
                    // First line: from start.1 to end
                    for ch in chars.iter().skip(start.1) {
                        deleted_text.push(*ch);
                    }
                    deleted_text.push('\n');
                } else if line_idx == end.0 {
                    // Last line: from 0 to end.1
                    for ch in chars.iter().take(end.1.min(chars.len())) {
                        deleted_text.push(*ch);
                    }
                } else {
                    // Middle lines: entire line
                    deleted_text.push_str(line);
                    deleted_text.push('\n');
                }
            }
        }

        Self { start, end, deleted_text, cursor_before: cursor }
    }
}

impl Command for DeleteRangeCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        // Delete from start to end
        if self.start == self.end {
            *cursor = self.start;
            return;
        }

        // Calculate how many characters to delete
        let mut chars_to_delete = 0;
        if self.start.0 == self.end.0 {
            // Single line: just delete the characters between start and end
            chars_to_delete = self.end.1 - self.start.1;
        } else {
            // Multi-line: calculate total characters including newlines
            // First line: from start.1 to end of line
            chars_to_delete += buffer.line_len(self.start.0) - self.start.1 + 1; // +1 for newline

            // Middle lines: entire lines
            for line_idx in (self.start.0 + 1)..self.end.0 {
                chars_to_delete += buffer.line_len(line_idx) + 1; // +1 for newline
            }

            // Last line: from 0 to end.1
            chars_to_delete += self.end.1;
        }

        // Delete all characters forward from start position
        for _ in 0..chars_to_delete {
            buffer.delete_forward(self.start.0, self.start.1);
        }

        *cursor = self.start;
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        // Re-insert the deleted text
        let mut current_line = self.start.0;
        let mut current_col = self.start.1;

        for ch in self.deleted_text.chars() {
            if ch == '\n' {
                buffer.insert_newline(current_line, current_col);
                current_line += 1;
                current_col = 0;
            } else {
                buffer.insert_char(current_line, current_col, ch);
                current_col += 1;
            }
        }

        *cursor = self.cursor_before;
    }
}

/// Composite command that groups multiple commands together.
#[derive(Debug)]
pub struct CompositeCommand {
    commands: Vec<Box<dyn Command>>,
}

impl CompositeCommand {
    /// Creates a new composite command.
    pub fn new(_description: String) -> Self {
        Self { commands: Vec::new() }
    }

    /// Adds a command to this composite.
    pub fn add(&mut self, command: Box<dyn Command>) {
        self.commands.push(command);
    }

    /// Returns whether this composite is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Command for CompositeCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        for cmd in &mut self.commands {
            cmd.execute(buffer, cursor);
        }
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        // Undo in reverse order
        for cmd in self.commands.iter_mut().rev() {
            cmd.undo(buffer, cursor);
        }
    }
}

/// Command for replacing text (used in search/replace functionality).
#[derive(Debug, Clone)]
pub struct ReplaceTextCommand {
    position: (usize, usize),
    old_text: String,
    new_text: String,
    cursor_before: (usize, usize),
    cursor_after: (usize, usize),
}

impl ReplaceTextCommand {
    /// Creates a new replace text command.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The text buffer (to read the old text)
    /// * `position` - Start position (line, col) of text to replace
    /// * `old_text_len` - Length of text to replace (in characters)
    /// * `new_text` - Text to insert in place
    /// * `cursor` - Current cursor position
    pub fn new(
        buffer: &TextBuffer,
        position: (usize, usize),
        old_text_len: usize,
        new_text: String,
        cursor: (usize, usize),
    ) -> Self {
        // Extract the old text being replaced
        let line = buffer.line(position.0);
        let chars: Vec<char> = line.chars().collect();
        let old_text: String =
            chars.iter().skip(position.1).take(old_text_len).collect();

        let cursor_after = (position.0, position.1 + new_text.chars().count());

        Self {
            position,
            old_text,
            new_text,
            cursor_before: cursor,
            cursor_after,
        }
    }
}

impl Command for ReplaceTextCommand {
    fn execute(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: &mut (usize, usize),
    ) {
        // Optimized replacement using replace_range
        buffer.replace_range(
            self.position.0,
            self.position.1,
            self.old_text.chars().count(),
            &self.new_text,
        );

        *cursor = self.cursor_after;
    }

    fn undo(&mut self, buffer: &mut TextBuffer, cursor: &mut (usize, usize)) {
        // Restore old text using replace_range
        buffer.replace_range(
            self.position.0,
            self.position.1,
            self.new_text.chars().count(),
            &self.old_text,
        );

        *cursor = self.cursor_before;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_char_command() {
        let mut buffer = TextBuffer::new("hello");
        let mut cursor = (0, 5);
        let mut cmd = InsertCharCommand::new(0, 5, '!', cursor);

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello!");
        assert_eq!(cursor, (0, 6));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello");
        assert_eq!(cursor, (0, 5));
    }

    #[test]
    fn test_delete_char_command() {
        let mut buffer = TextBuffer::new("hello");
        let mut cursor = (0, 5);
        let mut cmd = DeleteCharCommand::new(&buffer, 0, 5, cursor);

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hell");
        assert_eq!(cursor, (0, 4));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello");
        assert_eq!(cursor, (0, 5));
    }

    #[test]
    fn test_insert_newline_command() {
        let mut buffer = TextBuffer::new("hello world");
        let mut cursor = (0, 5);
        let mut cmd =
            InsertNewlineCommand::with_indent(0, 5, cursor, String::new());

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello");
        assert_eq!(buffer.line(1), " world");
        assert_eq!(cursor, (1, 0));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello world");
        assert_eq!(cursor, (0, 5));
    }

    #[test]
    fn test_insert_newline_with_indent_spaces() {
        let mut buffer = TextBuffer::new("    hello");
        let mut cursor = (0, 9);
        let mut cmd =
            InsertNewlineCommand::with_indent(0, 9, cursor, "    ".to_string());

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "    hello");
        assert_eq!(buffer.line(1), "    ");
        assert_eq!(cursor, (1, 4));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "    hello");
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(cursor, (0, 9));
    }

    #[test]
    fn test_insert_newline_with_indent_mid_line() {
        let mut buffer = TextBuffer::new("    hello world");
        let mut cursor = (0, 9);
        let mut cmd =
            InsertNewlineCommand::with_indent(0, 9, cursor, "    ".to_string());

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "    hello");
        assert_eq!(buffer.line(1), "     world"); // 4 spaces indent + " world"
        assert_eq!(cursor, (1, 4));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "    hello world");
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(cursor, (0, 9));
    }

    #[test]
    fn test_insert_newline_with_indent_tab() {
        let mut buffer = TextBuffer::new("\thello");
        let mut cursor = (0, 6);
        let mut cmd =
            InsertNewlineCommand::with_indent(0, 6, cursor, "\t".to_string());

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "\thello");
        assert_eq!(buffer.line(1), "\t");
        assert_eq!(cursor, (1, 1));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "\thello");
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(cursor, (0, 6));
    }

    #[test]
    fn test_insert_text_command() {
        let mut buffer = TextBuffer::new("hello");
        let mut cursor = (0, 5);
        let mut cmd =
            InsertTextCommand::new(0, 5, " world".to_string(), cursor);

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello world");
        assert_eq!(cursor, (0, 11));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello");
        assert_eq!(cursor, (0, 5));
    }

    #[test]
    fn test_delete_range_command() {
        let mut buffer = TextBuffer::new("hello world");
        let mut cursor = (0, 0);
        let mut cmd = DeleteRangeCommand::new(&buffer, (0, 0), (0, 5), cursor);

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), " world");
        assert_eq!(cursor, (0, 0));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello world");
        assert_eq!(cursor, (0, 0));
    }

    #[test]
    fn test_composite_command() {
        let mut buffer = TextBuffer::new("hello");
        let mut cursor = (0, 5);
        let mut composite = CompositeCommand::new("Multiple edits".to_string());

        composite.add(Box::new(InsertCharCommand::new(0, 5, ' ', cursor)));
        cursor.1 += 1;
        composite.add(Box::new(InsertCharCommand::new(0, 6, 'w', cursor)));
        cursor.1 += 1;
        composite.add(Box::new(InsertCharCommand::new(0, 7, 'o', cursor)));

        composite.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello wo");

        composite.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello");
    }

    #[test]
    fn test_replace_text_command() {
        let mut buffer = TextBuffer::new("hello world");
        let mut cursor = (0, 0);
        let mut cmd = ReplaceTextCommand::new(
            &buffer,
            (0, 0),
            5,
            "goodbye".to_string(),
            cursor,
        );

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "goodbye world");
        assert_eq!(cursor, (0, 7));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "hello world");
        assert_eq!(cursor, (0, 0));
    }

    #[test]
    fn test_replace_text_different_lengths() {
        let mut buffer = TextBuffer::new("foo bar baz");
        let mut cursor = (0, 4);

        // Replace "bar" (3 chars) with "x" (1 char)
        let mut cmd = ReplaceTextCommand::new(
            &buffer,
            (0, 4),
            3,
            "x".to_string(),
            cursor,
        );

        cmd.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "foo x baz");
        assert_eq!(cursor, (0, 5));

        cmd.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "foo bar baz");
        assert_eq!(cursor, (0, 4));
    }

    #[test]
    fn test_replace_all_composite() {
        let mut buffer = TextBuffer::new("foo foo foo");
        let mut cursor = (0, 0);
        let mut composite = CompositeCommand::new("Replace all".to_string());

        // Replace all "foo" with "bar" (in reverse order to preserve positions)
        composite.add(Box::new(ReplaceTextCommand::new(
            &buffer,
            (0, 8),
            3,
            "bar".to_string(),
            cursor,
        )));
        composite.add(Box::new(ReplaceTextCommand::new(
            &buffer,
            (0, 4),
            3,
            "bar".to_string(),
            cursor,
        )));
        composite.add(Box::new(ReplaceTextCommand::new(
            &buffer,
            (0, 0),
            3,
            "bar".to_string(),
            cursor,
        )));

        composite.execute(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "bar bar bar");

        composite.undo(&mut buffer, &mut cursor);
        assert_eq!(buffer.line(0), "foo foo foo");
    }
}
