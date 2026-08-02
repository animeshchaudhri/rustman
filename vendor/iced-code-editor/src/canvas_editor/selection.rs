//! Text selection logic.

use super::CodeEditor;
use crate::text_utils::{char_range_to_byte_range, char_to_byte_index};

impl CodeEditor {
    /// Clears the current selection on all cursors.
    pub(crate) fn clear_selection(&mut self) {
        self.cursors.clear_all_selections();
        // Selection affects only overlay visuals (highlight rectangles), so avoid
        // invalidating the expensive content cache.
        self.overlay_cache.clear();
    }

    /// Returns the primary cursor's selected text range in normalized order (start before end).
    pub(crate) fn get_selection_range(
        &self,
    ) -> Option<((usize, usize), (usize, usize))> {
        self.cursors.primary().selection_range()
    }

    /// Returns the selected text of the primary cursor as a string.
    pub(crate) fn get_selected_text(&self) -> Option<String> {
        let (start, end) = self.get_selection_range()?;

        if start == end {
            return None; // No selection
        }

        Some(self.extract_text_range(start, end))
    }

    /// Extracts text between two positions from the buffer.
    pub(crate) fn extract_text_range(
        &self,
        start: (usize, usize),
        end: (usize, usize),
    ) -> String {
        let mut result = String::new();

        if start.0 == end.0 {
            // Single line selection
            let line = self.buffer.line(start.0);
            let (start_byte, end_byte) =
                char_range_to_byte_range(line, start.1, end.1);
            result.push_str(&line[start_byte..end_byte]);
        } else {
            // Multi-line selection
            // First line
            let first_line = self.buffer.line(start.0);
            let start_byte = char_to_byte_index(first_line, start.1);
            result.push_str(&first_line[start_byte..]);
            result.push('\n');

            // Middle lines
            for line_idx in (start.0 + 1)..end.0 {
                result.push_str(self.buffer.line(line_idx));
                result.push('\n');
            }

            // Last line
            let last_line = self.buffer.line(end.0);
            let end_byte = char_to_byte_index(last_line, end.1);
            result.push_str(&last_line[..end_byte]);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_single_line() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        let text = editor.get_selected_text();
        assert_eq!(text, Some("hello".to_string()));
    }

    #[test]
    fn test_selection_multiline() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().anchor = Some((0, 2));
        editor.cursors.primary_mut().position = (2, 3);

        let text = editor.get_selected_text();
        assert_eq!(text, Some("ne1\nline2\nlin".to_string()));
    }

    #[test]
    fn test_selection_range_normalization() {
        let mut editor = CodeEditor::new("hello world", "py");
        // Set selection in reverse order (end before start)
        editor.cursors.primary_mut().anchor = Some((0, 5));
        editor.cursors.primary_mut().position = (0, 0);

        let range = editor.get_selection_range();
        // Should normalize to (0,0) -> (0,5)
        assert_eq!(range, Some(((0, 0), (0, 5))));
    }

    #[test]
    fn test_clear_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        editor.clear_selection();
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_selection_out_of_bounds() {
        let mut editor = CodeEditor::new("hello", "py");
        // Start out of bounds (column 10)
        editor.cursors.primary_mut().anchor = Some((0, 10));
        editor.cursors.primary_mut().position = (0, 15);

        let text = editor.get_selected_text();
        // With the fix, start is out of bounds, so we get empty string.
        assert_eq!(text, Some("".to_string()));
    }

    #[test]
    fn test_selection_multiline_out_of_bounds() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        // Start out of bounds on first line
        editor.cursors.primary_mut().anchor = Some((0, 10));
        // End normal on last line
        editor.cursors.primary_mut().position = (2, 3);

        let text = editor.get_selected_text();
        // An out-of-bounds start column is clamped to the end of the first
        // line, so the selection starts there and includes the newline that
        // separates it from the next line.
        assert_eq!(text, Some("\nline2\nlin".to_string()));

        // Now test end out of bounds
        editor.cursors.primary_mut().anchor = Some((0, 2));
        editor.cursors.primary_mut().position = (2, 10);
        let text = editor.get_selected_text();
        assert_eq!(text, Some("ne1\nline2\nline3".to_string()));
    }

    #[test]
    fn test_selection_unicode() {
        let mut editor = CodeEditor::new("你好\n世界", "txt");

        editor.cursors.primary_mut().anchor = Some((0, 1));
        editor.cursors.primary_mut().position = (1, 1);

        let text = editor.get_selected_text();
        assert_eq!(text, Some("好\n世".to_string()));
    }

    #[test]
    fn test_selection_with_empty_lines() {
        let mut editor = CodeEditor::new("line1\n\nline3", "txt");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (2, 5);

        let text = editor.get_selected_text();
        assert_eq!(text, Some("line1\n\nline3".to_string()));
    }

    #[test]
    fn test_selection_emoji() {
        let mut editor = CodeEditor::new("a😀b", "txt");

        editor.cursors.primary_mut().anchor = Some((0, 1));
        editor.cursors.primary_mut().position = (0, 2);

        let text = editor.get_selected_text();
        assert_eq!(text, Some("😀".to_string()));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_selection_complex_emoji() {
        let complex_emoji = "👨‍👩‍👧‍👦";
        let mut editor = CodeEditor::new(complex_emoji, "txt");

        let char_count = complex_emoji.chars().count();

        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, char_count);

        let text = editor.get_selected_text();
        assert_eq!(text, Some(complex_emoji.to_string()));

        if char_count > 1 {
            editor.cursors.primary_mut().anchor = Some((0, 0));
            editor.cursors.primary_mut().position = (0, 1);
            let text = editor.get_selected_text();
            let first_char = complex_emoji.chars().next().unwrap().to_string();
            assert_eq!(text, Some(first_char));
        }
    }
}
