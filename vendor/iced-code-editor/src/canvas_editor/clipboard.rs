//! Clipboard operations (copy, paste, delete selection).

use iced::Task;

use super::command::{Command, DeleteRangeCommand, InsertTextCommand};
use super::{CodeEditor, Message};

// =========================================================================
// Position adjustment helpers (private to this module)
// =========================================================================

/// Adjusts `pos` after deleting the buffer range `[start, end)`.
///
/// Positions inside the range collapse to `start`; positions after `end`
/// shift back by the size of the deleted region.
fn adjust_pos_for_delete_range(
    pos: &mut (usize, usize),
    start: (usize, usize),
    end: (usize, usize),
) {
    if start == end || *pos < start {
        return; // empty range or position precedes it — unaffected
    }
    if *pos < end {
        *pos = start; // inside deleted range: collapse to start
        return;
    }
    // position is at or after `end`
    let (sl, sc) = start;
    let (el, ec) = end;
    let (pl, pc) = *pos;
    if sl == el {
        // single-line deletion on the same line
        if pl == sl {
            pos.1 = pc - (ec - sc);
        }
    } else {
        // multi-line deletion
        if pl > el {
            pos.0 = pl - (el - sl);
        } else if pl == el {
            // on the end line, at or after `ec`
            pos.0 = sl;
            pos.1 = sc + (pc - ec);
        }
    }
}

/// Adjusts `pos` after inserting text that began at `(edit_line, edit_col)`.
///
/// `line_delta` = number of `\n` chars in the inserted text.
/// `col_delta`  = number of chars on the last inserted line.
fn adjust_pos_for_insert(
    pos: &mut (usize, usize),
    edit_line: usize,
    edit_col: usize,
    line_delta: usize,
    col_delta: usize,
) {
    if line_delta == 0 {
        if pos.0 == edit_line && pos.1 >= edit_col {
            pos.1 += col_delta;
        }
    } else if pos.0 > edit_line {
        pos.0 += line_delta;
    } else if pos.0 == edit_line && pos.1 >= edit_col {
        pos.0 += line_delta;
        pos.1 = pos.1 - edit_col + col_delta;
    }
}

/// Computes the `(line_delta, col_delta)` for a text string.
///
/// Uses `split('\n')` to be consistent with [`InsertTextCommand`].
fn text_deltas(text: &str) -> (usize, usize) {
    let parts: Vec<&str> = text.split('\n').collect();
    let line_delta = parts.len() - 1;
    let col_delta = parts.last().map_or(0, |l| l.chars().count());
    (line_delta, col_delta)
}

// =========================================================================
// CodeEditor impl
// =========================================================================

impl CodeEditor {
    /// Copies selected text to clipboard.
    ///
    /// With multiple cursors that each have a selection, all selected texts
    /// are joined with `\n` and written to the clipboard as one string —
    /// matching the multi-cursor copy convention used by VS Code/Sublime Text.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // cursor 0 selects "foo", cursor 1 selects "bar"
    /// // → clipboard contains "foo\nbar"
    /// ```
    pub(crate) fn copy_selection(&self) -> Task<Message> {
        let texts: Vec<String> = self
            .cursors
            .iter()
            .filter_map(|c| {
                c.selection_range()
                    .map(|(start, end)| self.extract_text_range(start, end))
            })
            .collect();

        if texts.is_empty() {
            Task::none()
        } else {
            iced::clipboard::write(texts.join("\n"))
        }
    }

    /// Deletes the selected text across **all** cursors.
    ///
    /// Cursors are processed in descending document order so that each
    /// deletion does not invalidate the positions of cursors that come
    /// earlier in the document.  After every range deletion all other
    /// cursors (including ones without a selection) are adjusted.
    pub(crate) fn delete_selection(&mut self) {
        // Collect indices of cursors that have a non-empty selection.
        let mut indices: Vec<usize> = (0..self.cursors.len())
            .filter(|&i| self.cursors.as_slice()[i].has_selection())
            .collect();

        // Sort descending by the start of the selection range.
        indices.sort_by(|&a, &b| {
            let sa = self.cursors.as_slice()[a]
                .selection_range()
                .map_or((0, 0), |(s, _)| s);
            let sb = self.cursors.as_slice()[b]
                .selection_range()
                .map_or((0, 0), |(s, _)| s);
            sb.cmp(&sa) // descending
        });

        for idx in indices {
            let (start, end) =
                match self.cursors.as_slice()[idx].selection_range() {
                    Some(r) if r.0 != r.1 => r,
                    _ => continue,
                };

            let pos = self.cursors.as_slice()[idx].position;
            let mut cmd =
                DeleteRangeCommand::new(&self.buffer, start, end, pos);
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.as_mut_slice()[idx].position = cursor_pos;
            self.history.push(Box::new(cmd));

            // Adjust every other cursor for this deletion.
            let n = self.cursors.len();
            for other_idx in 0..n {
                if other_idx == idx {
                    continue;
                }
                let cursor = &mut self.cursors.as_mut_slice()[other_idx];
                adjust_pos_for_delete_range(&mut cursor.position, start, end);
                if let Some(ref mut anchor) = cursor.anchor {
                    adjust_pos_for_delete_range(anchor, start, end);
                }
            }
        }
        self.cursors.clear_all_selections();
    }

    /// Pastes text from clipboard at all cursor positions.
    ///
    /// **Multi-cursor behaviour**:
    /// - If the clipboard text has exactly as many `\n`-separated segments as
    ///   there are cursors, each segment is pasted at the corresponding cursor
    ///   in ascending document order (segment *i* → cursor *i*).
    /// - Otherwise the full text is pasted at every cursor.
    ///
    /// Any active selection is deleted before the paste.
    /// Cursors are processed in descending document order so that each
    /// insertion does not shift the positions of unprocessed cursors.
    /// After every insertion all other cursors (including already-processed
    /// ones) are adjusted to account for the buffer change.
    pub(crate) fn paste_text(&mut self, text: &str) {
        // Remove all selections before inserting.
        if self.cursors.iter().any(|c| c.has_selection()) {
            self.delete_selection();
        }

        let cursor_count = self.cursors.len();

        // Fast path: single cursor.
        if cursor_count == 1 {
            let pos = self.cursors.primary_position();
            let mut cmd =
                InsertTextCommand::new(pos.0, pos.1, text.to_string(), pos);
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.primary_mut().position = cursor_pos;
            self.history.push(Box::new(cmd));
            return;
        }

        // Multi-cursor path.
        // Split the clipboard on '\n' to detect per-cursor mode.
        let clip_lines: Vec<&str> = text.split('\n').collect();
        let per_cursor = clip_lines.len() == cursor_count;

        // Build a descending-order index list (highest position first).
        let mut order: Vec<usize> = (0..cursor_count).collect();
        order.sort_by(|&a, &b| {
            self.cursors.as_slice()[b]
                .position
                .cmp(&self.cursors.as_slice()[a].position)
        });

        for (rank, &idx) in order.iter().enumerate() {
            // `rank` 0 = highest position, `cursor_count - 1` = lowest.
            // For per-cursor paste: ascending document order → ascending line index.
            let paste_str: &str = if per_cursor {
                let asc_rank = cursor_count - 1 - rank;
                clip_lines[asc_rank]
            } else {
                text
            };

            let pos = self.cursors.as_slice()[idx].position;
            let (line_delta, col_delta) = text_deltas(paste_str);

            let mut cmd = InsertTextCommand::new(
                pos.0,
                pos.1,
                paste_str.to_string(),
                pos,
            );
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.as_mut_slice()[idx].position = cursor_pos;
            self.history.push(Box::new(cmd));

            // Adjust every other cursor for this insertion.
            for other_idx in 0..cursor_count {
                if other_idx == idx {
                    continue;
                }
                let cursor = &mut self.cursors.as_mut_slice()[other_idx];
                adjust_pos_for_insert(
                    &mut cursor.position,
                    pos.0,
                    pos.1,
                    line_delta,
                    col_delta,
                );
                if let Some(ref mut anchor) = cursor.anchor {
                    adjust_pos_for_insert(
                        anchor, pos.0, pos.1, line_delta, col_delta,
                    );
                }
            }
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // adjust_pos_for_delete_range
    // ------------------------------------------------------------------

    #[test]
    fn test_delete_range_before_range() {
        let mut pos = (0, 1);
        adjust_pos_for_delete_range(&mut pos, (0, 3), (0, 6));
        assert_eq!(pos, (0, 1)); // unaffected
    }

    #[test]
    fn test_delete_range_inside_range() {
        let mut pos = (0, 4);
        adjust_pos_for_delete_range(&mut pos, (0, 3), (0, 6));
        assert_eq!(pos, (0, 3)); // collapses to start
    }

    #[test]
    fn test_delete_range_after_single_line() {
        let mut pos = (0, 8);
        adjust_pos_for_delete_range(&mut pos, (0, 3), (0, 6));
        assert_eq!(pos, (0, 5)); // shifted back by 3
    }

    #[test]
    fn test_delete_range_multiline_later_line() {
        let mut pos = (3, 2);
        adjust_pos_for_delete_range(&mut pos, (1, 0), (2, 5));
        assert_eq!(pos, (2, 2)); // line shifted up by 1
    }

    #[test]
    fn test_delete_range_end_line_merges() {
        // Deleting from (1, 0) to (2, 3): cursor at (2, 5) moves to (1, 0 + 5-3 = 2)
        let mut pos = (2, 5);
        adjust_pos_for_delete_range(&mut pos, (1, 0), (2, 3));
        assert_eq!(pos, (1, 2));
    }

    // ------------------------------------------------------------------
    // adjust_pos_for_insert
    // ------------------------------------------------------------------

    #[test]
    fn test_insert_same_line_shift() {
        let mut pos = (0, 5);
        adjust_pos_for_insert(&mut pos, 0, 3, 0, 2); // insert 2 chars at (0,3)
        assert_eq!(pos, (0, 7));
    }

    #[test]
    fn test_insert_before_pos_unaffected() {
        let mut pos = (0, 2);
        adjust_pos_for_insert(&mut pos, 0, 5, 0, 3); // insert after pos
        assert_eq!(pos, (0, 2)); // unaffected
    }

    #[test]
    fn test_insert_multiline_shifts_line() {
        let mut pos = (2, 3);
        adjust_pos_for_insert(&mut pos, 1, 0, 2, 4); // insert 2 newlines at (1,0)
        assert_eq!(pos, (4, 3)); // shifted up by 2
    }

    // ------------------------------------------------------------------
    // CodeEditor clipboard operations
    // ------------------------------------------------------------------

    #[test]
    fn test_delete_selection_single_line() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        editor.delete_selection();
        assert_eq!(editor.buffer.line(0), " world");
    }

    #[test]
    fn test_delete_selection_multi_cursor() {
        // "hello world" — cursor 0 selects "hel", cursor 1 selects "wor"
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 3);
        editor.cursors.add_cursor((0, 9));
        editor.cursors.as_mut_slice()[1].anchor = Some((0, 6));
        editor.cursors.as_mut_slice()[1].position = (0, 9);

        editor.delete_selection();
        // After deleting "hel": "lo world" (cursor 1 selection shifts left by 3)
        // After deleting "wor": "lo ld"
        assert_eq!(editor.buffer.line(0), "lo ld");
    }

    #[test]
    fn test_copy_selection_multi_cursor() {
        let editor_text = "foo\nbar";
        let mut editor = CodeEditor::new(editor_text, "py");
        // Select "foo" with primary cursor
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 3);
        // Add second cursor selecting "bar"
        editor.cursors.add_cursor((1, 3));
        editor.cursors.as_mut_slice()[1].anchor = Some((1, 0));

        // We can't easily test the clipboard write task result,
        // but we can verify extract_text_range works for both ranges.
        let text0 = editor.extract_text_range((0, 0), (0, 3));
        let text1 = editor.extract_text_range((1, 0), (1, 3));
        assert_eq!(text0, "foo");
        assert_eq!(text1, "bar");
        assert_eq!(format!("{text0}\n{text1}"), "foo\nbar");
    }

    #[test]
    fn test_paste_text_single_cursor() {
        let mut editor = CodeEditor::new("hello", "py");
        editor.cursors.set_single((0, 5)); // move cursor to end of "hello"
        editor.paste_text(" world");
        assert_eq!(editor.buffer.line(0), "hello world");
        assert_eq!(editor.cursors.primary_position(), (0, 11));
    }

    #[test]
    fn test_paste_text_multi_cursor_same_text() {
        // Two cursors on different lines; paste same text at both
        let mut editor = CodeEditor::new("ab\ncd", "py");
        editor.cursors.set_single((0, 2));
        editor.cursors.add_cursor((1, 2));

        editor.paste_text("X");

        // Descending: process (1,2) first → "cdX", then (0,2) → "abX"
        assert_eq!(editor.buffer.line(0), "abX");
        assert_eq!(editor.buffer.line(1), "cdX");
    }

    #[test]
    fn test_paste_text_per_cursor() {
        // Two cursors, clipboard has two lines → per-cursor paste
        let mut editor = CodeEditor::new("ab\ncd", "py");
        editor.cursors.set_single((0, 2));
        editor.cursors.add_cursor((1, 2));

        // Clipboard text "foo\nbar": 2 lines, 2 cursors → per-cursor
        editor.paste_text("foo\nbar");

        // Cursor 0 (lower position, ascending rank 0) → "foo"
        // Cursor 1 (higher position, ascending rank 1) → "bar"
        assert_eq!(editor.buffer.line(0), "abfoo");
        assert_eq!(editor.buffer.line(1), "cdbar");
    }
}
