//! Message handling and update logic.

use iced::Task;
use iced::widget::operation::{focus, select_all};

use crate::text_utils::char_to_byte_index;

use super::command::{
    Command, CompositeCommand, DeleteCharCommand, DeleteForwardCommand,
    DeleteRangeCommand, DuplicateLinesCommand, InsertCharCommand,
    InsertNewlineCommand, InsertTextCommand, MoveLinesCommand,
    ReplaceTextCommand, ToggleCommentCommand, line_comment_token,
};
use super::vim::{
    VimAction, VimInsertPosition, VimMotion, VimOperator, VimPastePosition,
    VimRegister, VimRegisterKind,
};
use super::{
    ArrowDirection, CURSOR_BLINK_INTERVAL, CodeEditor, ImePreedit, IndentStyle,
    LspEditSnapshot, Message, VimMode, cursor_set, lsp,
};

// =========================================================================
// Cursor adjustment helpers for multi-cursor editing
// =========================================================================

/// Describes the kind of edit applied to a single position.
#[derive(Clone, Copy)]
enum EditType {
    /// Insert one char at `(edit_line, edit_col)`.
    InsertChar,
    /// Backspace: delete char at `(edit_line, edit_col - 1)`.
    DeleteCharBack,
    /// Delete-forward: delete char at `(edit_line, edit_col)`.
    DeleteCharForward,
    /// Enter: split `edit_line` at `edit_col`; new line has `extra` indent chars.
    InsertNewline { indent_len: usize },
    /// Backspace-at-col-0: merge `edit_line` into `edit_line - 1`.
    /// `extra` = length of the previous line before merge.
    MergePrev { prev_line_len: usize },
    /// Delete-at-end-of-line: merge `edit_line + 1` into `edit_line`.
    /// `extra` = length of `edit_line` before merge.
    MergeNext { edit_line_len: usize },
}

/// Adjusts a single `(line, col)` pair after an edit.
fn adjust_pos(
    pos: &mut (usize, usize),
    edit_line: usize,
    edit_col: usize,
    kind: EditType,
) {
    match kind {
        EditType::InsertChar => {
            if pos.0 == edit_line && pos.1 >= edit_col {
                pos.1 += 1;
            }
        }
        EditType::DeleteCharBack => {
            if edit_col > 0 && pos.0 == edit_line && pos.1 > edit_col - 1 {
                pos.1 -= 1;
            }
        }
        EditType::DeleteCharForward => {
            if pos.0 == edit_line && pos.1 > edit_col {
                pos.1 -= 1;
            }
        }
        EditType::InsertNewline { indent_len } => {
            if pos.0 > edit_line {
                pos.0 += 1;
            } else if pos.0 == edit_line && pos.1 >= edit_col {
                pos.0 += 1;
                pos.1 = pos.1 - edit_col + indent_len;
            }
        }
        EditType::MergePrev { prev_line_len } => {
            if pos.0 == edit_line {
                pos.0 -= 1;
                pos.1 += prev_line_len;
            } else if pos.0 > edit_line {
                pos.0 -= 1;
            }
        }
        EditType::MergeNext { edit_line_len } => {
            if pos.0 == edit_line + 1 {
                pos.0 = edit_line;
                pos.1 += edit_line_len;
            } else if pos.0 > edit_line + 1 {
                pos.0 -= 1;
            }
        }
    }
}

/// Adjusts all cursors except `skip_idx` after an edit at `(edit_line, edit_col)`.
fn adjust_other_cursors(
    cursors: &mut [cursor_set::Cursor],
    skip_idx: usize,
    edit_line: usize,
    edit_col: usize,
    kind: EditType,
) {
    for (i, cursor) in cursors.iter_mut().enumerate() {
        if i == skip_idx {
            continue;
        }
        adjust_pos(&mut cursor.position, edit_line, edit_col, kind);
        if let Some(ref mut anchor) = cursor.anchor {
            adjust_pos(anchor, edit_line, edit_col, kind);
        }
    }
}

impl CodeEditor {
    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Performs common cleanup operations after edit operations.
    ///
    /// This method should be called after any operation that modifies the buffer content.
    /// It resets the cursor blink animation, refreshes search matches if search is active,
    /// and invalidates all caches that depend on buffer content or layout:
    /// - `buffer_revision` is bumped to invalidate layout-derived caches
    /// - `visual_lines_cache` is cleared so wrapping is recalculated on next use
    /// - `content_cache` and `overlay_cache` are cleared to rebuild canvas geometry
    fn finish_edit_operation(&mut self) {
        self.reset_cursor_blink();
        self.refresh_search_matches_if_needed();
        // The exact revision value is not semantically meaningful; it only needs
        // to change on edits, so `wrapping_add` is sufficient and overflow-safe.
        let previous_revision = self.buffer_revision;
        self.buffer_revision = self.buffer_revision.wrapping_add(1);
        self.refresh_visual_lines_after_edit(previous_revision);
        self.refresh_max_content_width_after_edit(previous_revision);
        // Truncate the syntax-highlight cache from the first line the edit may
        // have changed. `pre_edit_line` is the topmost active line captured
        // before the edit; the extra line of margin covers edits that merge
        // with the preceding line (e.g. backspace at column 0).
        self.invalidate_highlight_from(self.pre_edit_line.saturating_sub(1));
        self.content_cache.clear();
        self.overlay_cache.clear();
        self.enqueue_incremental_lsp_change();
    }

    /// Returns the topmost logical line currently touched by any cursor or its
    /// selection anchor.
    ///
    /// This is captured before an edit to bound which highlight-cache lines may
    /// change. With no cursors it defaults to line `0`.
    pub(crate) fn min_active_line(&self) -> usize {
        self.cursors
            .iter()
            .map(|cursor| match cursor.anchor {
                Some(anchor) => cursor.position.0.min(anchor.0),
                None => cursor.position.0,
            })
            .min()
            .unwrap_or(0)
    }

    /// Returns the bottommost logical line touched by a cursor or selection.
    fn max_active_line(&self) -> usize {
        self.cursors
            .iter()
            .map(|cursor| match cursor.anchor {
                Some(anchor) => cursor.position.0.max(anchor.0),
                None => cursor.position.0,
            })
            .max()
            .unwrap_or(0)
    }

    /// Captures a conservative old-document line range for an incremental LSP
    /// replacement. Non-editing messages do not allocate or retain a snapshot.
    fn capture_lsp_edit_snapshot(&mut self, message: &Message) {
        if self.lsp_document.is_none() {
            self.lsp_edit_snapshot = None;
            return;
        }

        let is_local_edit = matches!(
            message,
            Message::CharacterInput(_)
                | Message::Tab
                | Message::Enter
                | Message::Backspace
                | Message::Delete
                | Message::DeleteSelection
                | Message::Paste(_)
                | Message::ImeCommit(_)
                | Message::MoveLineUp
                | Message::MoveLineDown
                | Message::DuplicateLineUp
                | Message::DuplicateLineDown
                | Message::ToggleComment
        );
        let is_global_edit = matches!(
            message,
            Message::Undo | Message::Redo | Message::ReplaceAll
        );
        let is_replace_next = matches!(message, Message::ReplaceNext);
        if !is_local_edit && !is_global_edit && !is_replace_next {
            self.lsp_edit_snapshot = None;
            return;
        }

        let line_count = self.buffer.line_count();
        let (mut first_line, mut last_line) = if is_global_edit {
            (0, line_count.saturating_sub(1))
        } else {
            (self.pre_edit_line, self.pre_edit_last_line)
        };
        if is_replace_next
            && let Some(search_match) = self.search_state.current_match()
        {
            first_line = first_line.min(search_match.line);
            last_line = last_line.max(search_match.line);
        }

        let start_line =
            if is_global_edit { 0 } else { first_line.saturating_sub(1) };
        let old_end_exclusive = if is_global_edit {
            line_count
        } else {
            last_line.saturating_add(2).min(line_count)
        };
        let old_end = if old_end_exclusive < line_count {
            lsp::LspPosition {
                line: u32::try_from(old_end_exclusive).unwrap_or(u32::MAX),
                character: 0,
            }
        } else {
            let last_line = line_count.saturating_sub(1);
            lsp::LspPosition {
                line: u32::try_from(last_line).unwrap_or(u32::MAX),
                character: u32::try_from(self.buffer.line_len(last_line))
                    .unwrap_or(u32::MAX),
            }
        };

        self.lsp_edit_snapshot = Some(LspEditSnapshot {
            start_line,
            old_end_exclusive,
            old_line_count: line_count,
            old_end,
        });
    }

    /// Truncates the syntax-highlight cache so logical lines `>= line` are
    /// re-highlighted on next access.
    ///
    /// Lines before the first edited line are unaffected, so the cached prefix
    /// is preserved and edits never trigger a full re-parse from the top of the
    /// file. Has no effect when the cache is empty.
    ///
    /// # Arguments
    ///
    /// * `line` - First logical line to invalidate.
    pub(crate) fn invalidate_highlight_from(&self, line: usize) {
        if let Some(cache) = self.highlight_cache.borrow_mut().as_mut() {
            cache.truncate(line);
        }
    }

    /// Performs common cleanup operations after navigation operations.
    ///
    /// This method should be called after cursor movement operations.
    /// It resets the cursor blink animation and invalidates only the overlay
    /// rendering cache. Cursor movement and selection changes do not modify the
    /// buffer content, so keeping the content cache intact avoids unnecessary
    /// re-rendering of syntax-highlighted text.
    fn finish_navigation_operation(&mut self) {
        self.sync_search_match_from_primary_cursor();
        self.reset_cursor_blink();
        self.overlay_cache.clear();
    }

    /// Starts command grouping with the given label if not already grouping.
    ///
    /// This is used for smart undo functionality, allowing multiple related
    /// operations to be undone as a single unit.
    ///
    /// # Arguments
    ///
    /// * `label` - A descriptive label for the group of commands
    fn ensure_grouping_started(&mut self, label: &str) {
        if !self.is_grouping {
            self.history.begin_group(label);
            self.is_grouping = true;
        }
    }

    /// Ends command grouping if currently active.
    ///
    /// This should be called when a series of related operations is complete,
    /// or when starting a new type of operation that shouldn't be grouped
    /// with previous operations.
    fn end_grouping_if_active(&mut self) {
        if self.is_grouping {
            self.history.end_group();
            self.is_grouping = false;
        }
    }

    fn keep_vim_insert_group(&self) -> bool {
        self.vim_enabled
            && self.vim_state.mode() == VimMode::Insert
            && self.is_grouping
    }

    /// Deletes all active selections across every cursor and performs cleanup.
    ///
    /// # Returns
    ///
    /// `true` if at least one selection was deleted, `false` if no cursor had a selection
    fn delete_selection_if_present(&mut self) -> bool {
        if self.cursors.iter().any(|c| c.has_selection()) {
            self.delete_selection();
            self.finish_edit_operation();
            true
        } else {
            false
        }
    }

    // =========================================================================
    // Text Input Handlers
    // =========================================================================

    /// Handles character input message operations.
    ///
    /// Inserts a character at the current cursor position and adds it to the
    /// undo history. Characters are grouped together for smart undo.
    /// Only processes input when the editor has active focus and is not locked.
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to insert
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible (including
    /// horizontal scroll when wrap is disabled)
    fn handle_character_input_msg(&mut self, ch: char) -> Task<Message> {
        // Guard clause: only process character input if editor has focus and is not locked
        if !self.has_focus() {
            return Task::none();
        }

        // Start grouping if not already grouping (for smart undo)
        self.ensure_grouping_started("Typing");

        // Typing replaces active selections, matching paste and IME commit
        // behavior. Keep the deletion and insertion in the same history group
        // so a single undo restores the replaced text.
        if self.cursors.iter().any(|cursor| cursor.has_selection()) {
            self.delete_selection();
        } else {
            // A plain click leaves a zero-length anchor in place (see
            // `handle_enter`); clear it so it isn't mistaken for a real
            // selection by a later edit.
            self.clear_selection();
        }

        // Multi-cursor: build a sorted index list (descending document order)
        // so that edits at higher positions don't invalidate lower positions.
        let mut order: Vec<usize> = (0..self.cursors.len()).collect();
        order.sort_by(|&a, &b| {
            self.cursors.as_slice()[b]
                .position
                .cmp(&self.cursors.as_slice()[a].position)
        });

        for &idx in &order {
            // Any active selection was deleted above, which also moves the
            // cursor to the original selection start and clears its anchor.
            // The current cursor position is therefore the insertion point.
            let pos = self.cursors.as_slice()[idx].position;
            let mut cmd = InsertCharCommand::new(pos.0, pos.1, ch, pos);
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.as_mut_slice()[idx].position = cursor_pos;
            adjust_other_cursors(
                self.cursors.as_mut_slice(),
                idx,
                pos.0,
                pos.1,
                EditType::InsertChar,
            );
            self.history.push(Box::new(cmd));
        }

        self.finish_edit_operation();

        // Auto-trigger LSP completion for identifier characters and trigger characters
        if ch.is_alphanumeric() || ch == '_' || ch == '.' {
            self.lsp_flush_pending_changes();
            self.lsp_request_completion();
        }

        self.scroll_to_cursor()
    }

    /// Handles Tab key press (inserts 4 spaces).
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible (including
    /// horizontal scroll when wrap is disabled)
    fn handle_tab(&mut self) -> Task<Message> {
        self.ensure_grouping_started("Tab");

        // A plain click leaves a zero-length anchor in place (see
        // `handle_enter`); clear it so it isn't mistaken for a real selection
        // by a later edit. Tab only reaches here when there is no real
        // selection to indent (see `IndentLines`).
        self.clear_selection();

        // Multi-cursor: process in descending document order
        let mut order: Vec<usize> = (0..self.cursors.len()).collect();
        order.sort_by(|&a, &b| {
            self.cursors.as_slice()[b]
                .position
                .cmp(&self.cursors.as_slice()[a].position)
        });

        for &idx in &order {
            let pos = self.cursors.as_slice()[idx].position;
            match self.indent_style {
                IndentStyle::Spaces(n) => {
                    let mut cursor_pos = pos;
                    for _i in 0..n as usize {
                        let current_col = cursor_pos.1;
                        let mut cmd = InsertCharCommand::new(
                            pos.0,
                            current_col,
                            ' ',
                            cursor_pos,
                        );
                        cmd.execute(&mut self.buffer, &mut cursor_pos);
                        adjust_other_cursors(
                            self.cursors.as_mut_slice(),
                            idx,
                            pos.0,
                            current_col,
                            EditType::InsertChar,
                        );
                        self.history.push(Box::new(cmd));
                    }
                    self.cursors.as_mut_slice()[idx].position = cursor_pos;
                }
                IndentStyle::Tab => {
                    let mut cmd =
                        InsertCharCommand::new(pos.0, pos.1, '\t', pos);
                    let mut cursor_pos = pos;
                    cmd.execute(&mut self.buffer, &mut cursor_pos);
                    adjust_other_cursors(
                        self.cursors.as_mut_slice(),
                        idx,
                        pos.0,
                        pos.1,
                        EditType::InsertChar,
                    );
                    self.cursors.as_mut_slice()[idx].position = cursor_pos;
                    self.history.push(Box::new(cmd));
                }
            }
        }

        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    /// Handles Tab key press for focus navigation (when search dialog is not open).
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that may navigate focus to another editor
    fn handle_focus_navigation_tab(&mut self) -> Task<Message> {
        // Only handle focus navigation if search dialog is not open
        if !self.search_state.is_open {
            // Lose focus from current editor
            self.has_canvas_focus = false;
            self.show_cursor = false;

            // Return a task that could potentially focus another editor
            // This implements focus chain management by allowing the parent application
            // to handle focus navigation between multiple editors
            Task::none()
        } else {
            Task::none()
        }
    }

    /// Handles Shift+Tab key press for focus navigation (when search dialog is not open).
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that may navigate focus to another editor
    fn handle_focus_navigation_shift_tab(&mut self) -> Task<Message> {
        // Only handle focus navigation if search dialog is not open
        if !self.search_state.is_open {
            // Lose focus from current editor
            self.has_canvas_focus = false;
            self.show_cursor = false;

            // Return a task that could potentially focus another editor
            // This implements focus chain management by allowing the parent application
            // to handle focus navigation between multiple editors
            Task::none()
        } else {
            Task::none()
        }
    }

    /// Handles Enter key press (inserts newline).
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_enter(&mut self) -> Task<Message> {
        // Standard editing treats Enter as a boundary. In Vim Insert mode the
        // newline belongs to the same insertion session and is closed by Esc.
        let keep_vim_group = self.keep_vim_insert_group();
        if !keep_vim_group {
            self.end_grouping_if_active();
        }

        // A mouse click leaves a zero-length anchor in place so a following
        // drag can extend the selection. Enter must clear that anchor before
        // moving the caret to the new line; otherwise the inserted newline
        // becomes selected and the next typed character deletes it.
        //
        // For a real selection, Enter replaces the selected text with a
        // newline. Group both commands so one undo restores the selection.
        let replaces_selection =
            self.cursors.iter().any(|cursor| cursor.has_selection());
        if replaces_selection {
            self.ensure_grouping_started("Enter");
            self.delete_selection();
        } else {
            self.clear_selection();
        }

        // Multi-cursor: process in descending document order
        let mut order: Vec<usize> = (0..self.cursors.len()).collect();
        order.sort_by(|&a, &b| {
            self.cursors.as_slice()[b]
                .position
                .cmp(&self.cursors.as_slice()[a].position)
        });

        for &idx in &order {
            let pos = self.cursors.as_slice()[idx].position;

            // Copy leading whitespace of the current line to the new line (if enabled)
            let indent: String = if self.auto_indent_enabled {
                self.buffer
                    .line(pos.0)
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect()
            } else {
                String::new()
            };
            let indent_len = indent.chars().count();

            let mut cmd =
                InsertNewlineCommand::with_indent(pos.0, pos.1, pos, indent);
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.as_mut_slice()[idx].position = cursor_pos;
            adjust_other_cursors(
                self.cursors.as_mut_slice(),
                idx,
                pos.0,
                pos.1,
                EditType::InsertNewline { indent_len },
            );
            self.history.push(Box::new(cmd));
        }

        if replaces_selection && !keep_vim_group {
            self.end_grouping_if_active();
        }

        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    // =========================================================================
    // Line Manipulation Handlers
    // =========================================================================

    /// Returns the inclusive line range affected by the primary cursor.
    ///
    /// When the primary cursor has a selection, the range covers every line it
    /// spans. A selection that ends at column 0 of a line does not include that
    /// trailing line (VS Code convention). Without a selection, the range is the
    /// single line the cursor sits on.
    fn primary_line_range(&self) -> (usize, usize) {
        let primary = self.cursors.primary();
        match primary.selection_range() {
            Some((sel_start, sel_end)) => {
                let end_line = if sel_end.1 == 0 && sel_end.0 > sel_start.0 {
                    sel_end.0 - 1
                } else {
                    sel_end.0
                };
                (sel_start.0, end_line)
            }
            None => {
                let line = primary.position.0;
                (line, line)
            }
        }
    }

    /// Shifts the primary cursor's position and selection anchor by `delta`
    /// whole lines (positive moves downward) so the selection follows an edit.
    fn shift_primary_cursor_lines(&mut self, delta: isize) {
        let primary = self.cursors.primary_mut();
        primary.position.0 = primary.position.0.saturating_add_signed(delta);
        if let Some(anchor) = primary.anchor.as_mut() {
            anchor.0 = anchor.0.saturating_add_signed(delta);
        }
    }

    /// Moves the current line, or the lines spanned by the primary selection,
    /// up or down by one line (Alt+Up / Alt+Down).
    ///
    /// Secondary cursors are collapsed onto the primary one. The move is a no-op
    /// when the affected range is already at the corresponding edge of the
    /// buffer.
    ///
    /// # Arguments
    ///
    /// * `down` - `true` to move the range down, `false` to move it up
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn move_lines(&mut self, down: bool) -> Task<Message> {
        self.end_grouping_if_active();
        self.cursors.remove_all_but_primary();

        let (start, end) = self.primary_line_range();

        // Reject moves that would push the range past the buffer edges.
        if down {
            if end + 1 >= self.buffer.line_count() {
                return Task::none();
            }
        } else if start == 0 {
            return Task::none();
        }

        let pos = self.cursors.primary_position();
        let mut cmd = MoveLinesCommand::new(start, end, down, pos);
        let mut cursor_pos = pos;
        cmd.execute(&mut self.buffer, &mut cursor_pos);
        self.shift_primary_cursor_lines(if down { 1 } else { -1 });
        self.history.push(Box::new(cmd));

        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    /// Duplicates the current line, or the lines spanned by the primary
    /// selection, above or below (Shift+Alt+Up / Shift+Alt+Down).
    ///
    /// Secondary cursors are collapsed onto the primary one. A downward
    /// duplication moves the cursor onto the new copy; an upward one leaves it
    /// on the (upper) copy.
    ///
    /// # Arguments
    ///
    /// * `down` - `true` to insert the copy below, `false` to insert it above
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn duplicate_lines(&mut self, down: bool) -> Task<Message> {
        self.end_grouping_if_active();
        self.cursors.remove_all_but_primary();

        let (start, end) = self.primary_line_range();
        let pos = self.cursors.primary_position();
        let mut cmd = DuplicateLinesCommand::new(start, end, down, pos);
        let mut cursor_pos = pos;
        cmd.execute(&mut self.buffer, &mut cursor_pos);
        if down {
            let block_len = (end - start + 1) as isize;
            self.shift_primary_cursor_lines(block_len);
        }
        self.history.push(Box::new(cmd));

        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    /// Toggles line comments on the current line, or the lines spanned by the
    /// primary selection (Ctrl+/).
    ///
    /// Secondary cursors are collapsed onto the primary one. If every non-blank
    /// line in the range is already commented, the range is uncommented;
    /// otherwise every non-blank line is commented. The operation is a no-op
    /// when the active syntax has no line-comment token (e.g. HTML) or the range
    /// holds only blank lines.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn toggle_comment(&mut self) -> Task<Message> {
        self.end_grouping_if_active();
        self.cursors.remove_all_but_primary();

        let Some(token) = line_comment_token(&self.syntax) else {
            return Task::none();
        };

        let (start, end) = self.primary_line_range();
        let pos = self.cursors.primary_position();
        let mut cmd =
            ToggleCommentCommand::new(&self.buffer, start, end, token, pos);
        if cmd.is_noop() {
            return Task::none();
        }

        // Track the selection anchor across the column shift before executing.
        let new_anchor =
            self.cursors.primary().anchor.map(|a| cmd.adjust_position(a));

        let mut cursor_pos = pos;
        cmd.execute(&mut self.buffer, &mut cursor_pos);
        let primary = self.cursors.primary_mut();
        primary.position = cursor_pos;
        primary.anchor = new_anchor;
        self.history.push(Box::new(cmd));

        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    // =========================================================================
    // Deletion Handlers
    // =========================================================================

    /// Handles Backspace key press.
    ///
    /// If there's a selection, deletes the selection. Otherwise, deletes the
    /// character before the cursor.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible if selection was deleted
    fn handle_backspace(&mut self) -> Task<Message> {
        // End grouping on backspace (separate from typing)
        if !self.keep_vim_insert_group() {
            self.end_grouping_if_active();
        }

        // If any cursor has a selection, delete all selections first
        if self.delete_selection_if_present() {
            return self.scroll_to_cursor();
        }

        // A mouse click leaves a zero-length anchor in place so a following
        // drag can extend the selection (see `handle_enter`). Backspace must
        // clear it before moving the caret; otherwise the anchor is left
        // behind at the pre-edit position and a phantom one-character
        // selection appears next to the cursor, which the next Backspace or
        // Delete then eats instead of a single character.
        self.clear_selection();

        // Multi-cursor: process in descending document order
        let mut order: Vec<usize> = (0..self.cursors.len()).collect();
        order.sort_by(|&a, &b| {
            self.cursors.as_slice()[b]
                .position
                .cmp(&self.cursors.as_slice()[a].position)
        });

        for &idx in &order {
            let pos = self.cursors.as_slice()[idx].position;
            // Determine edit type for adjusting other cursors
            let edit_kind = if pos.1 > 0 {
                EditType::DeleteCharBack
            } else if pos.0 > 0 {
                let prev_line_len = self.buffer.line_len(pos.0 - 1);
                EditType::MergePrev { prev_line_len }
            } else {
                // At very start of document: nothing to delete
                continue;
            };
            let mut cmd =
                DeleteCharCommand::new(&self.buffer, pos.0, pos.1, pos);
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.as_mut_slice()[idx].position = cursor_pos;
            adjust_other_cursors(
                self.cursors.as_mut_slice(),
                idx,
                pos.0,
                pos.1,
                edit_kind,
            );
            self.history.push(Box::new(cmd));
        }

        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    /// Handles Delete key press.
    ///
    /// If there's a selection, deletes the selection. Otherwise, deletes the
    /// character after the cursor.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible if selection was deleted
    fn handle_delete(&mut self) -> Task<Message> {
        // End grouping on delete
        if !self.keep_vim_insert_group() {
            self.end_grouping_if_active();
        }

        // If any cursor has a selection, delete all selections first
        if self.delete_selection_if_present() {
            return self.scroll_to_cursor();
        }

        // See the matching comment in `handle_backspace`: clear any
        // zero-length anchor left by a plain click before editing, so it
        // can't be mistaken for a real selection on a later edit.
        self.clear_selection();

        // Multi-cursor: process in descending document order
        let mut order: Vec<usize> = (0..self.cursors.len()).collect();
        order.sort_by(|&a, &b| {
            self.cursors.as_slice()[b]
                .position
                .cmp(&self.cursors.as_slice()[a].position)
        });

        for &idx in &order {
            let pos = self.cursors.as_slice()[idx].position;
            let line_len = self.buffer.line_len(pos.0);
            let edit_kind = if pos.1 < line_len {
                EditType::DeleteCharForward
            } else if pos.0 + 1 < self.buffer.line_count() {
                EditType::MergeNext { edit_line_len: line_len }
            } else {
                // At very end of document: nothing to delete
                continue;
            };
            let mut cmd =
                DeleteForwardCommand::new(&self.buffer, pos.0, pos.1, pos);
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.as_mut_slice()[idx].position = cursor_pos;
            adjust_other_cursors(
                self.cursors.as_mut_slice(),
                idx,
                pos.0,
                pos.1,
                edit_kind,
            );
            self.history.push(Box::new(cmd));
        }

        self.finish_edit_operation();
        Task::none()
    }

    /// Handles explicit selection deletion (Shift+Delete).
    ///
    /// Deletes the selected text if a selection exists.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_delete_selection(&mut self) -> Task<Message> {
        // End grouping on delete selection
        self.end_grouping_if_active();

        if self.cursors.iter().any(|c| c.has_selection()) {
            self.delete_selection();
            self.finish_edit_operation();
            self.scroll_to_cursor()
        } else {
            Task::none()
        }
    }

    // =========================================================================
    // Navigation Handlers
    // =========================================================================

    fn vim_accepts_insert_input(&self) -> bool {
        !self.vim_enabled || self.vim_state.mode() == VimMode::Insert
    }

    fn handle_vim_key_msg(&mut self, key: char) -> Task<Message> {
        if !self.vim_enabled {
            return Task::none();
        }

        let previous_mode = self.vim_state.mode();
        let action = self.vim_state.parse_key(key);
        match action {
            Some(VimAction::Mode(mode)) => {
                self.handle_vim_mode(mode, previous_mode)
            }
            Some(VimAction::Motion { motion, count }) => {
                self.handle_vim_motion(motion, count)
            }
            Some(VimAction::Insert { position, count }) => {
                self.handle_vim_insert(position, count)
            }
            Some(VimAction::Operator { operator, motion, count }) => {
                self.handle_vim_motion_operator(operator, motion, count)
            }
            Some(VimAction::LineOperator { operator, count }) => {
                let start_line = self.cursors.primary_position().0;
                let end_line = start_line
                    .saturating_add(count.saturating_sub(1))
                    .min(self.buffer.line_count().saturating_sub(1));
                self.handle_vim_line_operator(
                    operator, start_line, end_line, false,
                )
            }
            Some(VimAction::VisualOperator(operator)) => {
                self.handle_vim_visual_operator(operator)
            }
            Some(VimAction::DeleteCharacters { count }) => {
                self.handle_vim_delete_characters(count)
            }
            Some(VimAction::Paste { position, count }) => {
                self.handle_vim_paste(position, count)
            }
            Some(VimAction::Undo { count }) => {
                self.handle_vim_history(false, count)
            }
            Some(VimAction::Redo { count }) => {
                self.handle_vim_history(true, count)
            }
            Some(VimAction::RepeatSearch { reverse }) => {
                self.handle_vim_repeat_search(reverse)
            }
            Some(VimAction::SubmitSearch(query)) => {
                self.handle_vim_search(&query)
            }
            Some(VimAction::SubmitGotoLine(line)) => {
                self.handle_goto_position(line.saturating_sub(1), 0)
            }
            Some(VimAction::WriteFile { exit_vim }) => {
                if exit_vim {
                    self.set_vim_enabled(false);
                }
                Task::done(Message::WriteRequested)
            }
            Some(VimAction::ExitVimMode) => {
                self.set_vim_enabled(false);
                Task::none()
            }
            Some(VimAction::CommandLineChanged) => {
                self.overlay_cache.clear();
                Task::none()
            }
            None => Task::none(),
        }
    }

    fn handle_vim_search(&mut self, query: &str) -> Task<Message> {
        if !self.search_replace_enabled || query.is_empty() {
            return Task::none();
        }

        self.search_state.close();
        self.search_state.set_query(query.to_owned(), &self.buffer);
        if self.search_state.matches.is_empty() {
            self.overlay_cache.clear();
            return Task::none();
        }

        let cursor = self.cursors.primary_position();
        let next_index = self
            .search_state
            .matches
            .partition_point(|item| (item.line, item.col) <= cursor);
        self.search_state.current_match_index =
            Some(if next_index == self.search_state.matches.len() {
                0
            } else {
                next_index
            });

        if let Some(search_match) = self.search_state.current_match() {
            self.cursors.set_single((search_match.line, search_match.col));
        }
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    fn handle_vim_repeat_search(&mut self, reverse: bool) -> Task<Message> {
        let Some(last_search) = self.vim_state.last_search().map(str::to_owned)
        else {
            return Task::none();
        };
        if self.search_state.query != last_search {
            self.search_state.set_query(last_search, &self.buffer);
            self.search_state
                .select_match_near_cursor(self.cursors.primary_position());
        }
        if self.search_state.matches.is_empty() {
            return Task::none();
        }

        if reverse {
            self.search_state.previous_match();
        } else {
            self.search_state.next_match();
        }
        if let Some(search_match) = self.search_state.current_match() {
            self.cursors.set_single((search_match.line, search_match.col));
        }
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    fn handle_vim_delete_characters(&mut self, count: usize) -> Task<Message> {
        let start = self.vim_normal_position(self.cursors.primary_position());
        let line_len = self.buffer.line_len(start.0);
        let end = (start.0, start.1.saturating_add(count).min(line_len));
        self.handle_vim_character_operator(
            VimOperator::Delete,
            start,
            end,
            false,
        )
    }

    fn handle_vim_motion_operator(
        &mut self,
        operator: VimOperator,
        motion: VimMotion,
        count: usize,
    ) -> Task<Message> {
        let start = self.vim_normal_position(self.cursors.primary_position());
        if matches!(
            motion,
            VimMotion::Up
                | VimMotion::Down
                | VimMotion::DocumentStart
                | VimMotion::DocumentEnd
        ) {
            let target = self.vim_motion_target(start, motion, count);
            return self.handle_vim_line_operator(
                operator,
                start.0.min(target.0),
                start.0.max(target.0),
                false,
            );
        }

        let target = self.vim_motion_target(start, motion, count);
        let (range_start, range_end) = match motion {
            VimMotion::Right => (
                start,
                (
                    start.0,
                    start
                        .1
                        .saturating_add(count)
                        .min(self.buffer.line_len(start.0)),
                ),
            ),
            VimMotion::Left => {
                ((start.0, start.1.saturating_sub(count)), start)
            }
            VimMotion::WordEnd | VimMotion::LineEnd => {
                let end = if motion == VimMotion::LineEnd {
                    (start.0, self.buffer.line_len(start.0))
                } else {
                    (
                        target.0,
                        target
                            .1
                            .saturating_add(1)
                            .min(self.buffer.line_len(target.0)),
                    )
                };
                (start.min(end), start.max(end))
            }
            VimMotion::WordForward => {
                let end = if target > start {
                    target
                } else {
                    (start.0, self.buffer.line_len(start.0))
                };
                (start.min(end), start.max(end))
            }
            VimMotion::WordBackward
            | VimMotion::LineStart
            | VimMotion::FirstNonBlank => {
                (start.min(target), start.max(target))
            }
            VimMotion::Up
            | VimMotion::Down
            | VimMotion::DocumentStart
            | VimMotion::DocumentEnd => return Task::none(),
        };

        self.handle_vim_character_operator(
            operator,
            range_start,
            range_end,
            false,
        )
    }

    fn handle_vim_visual_operator(
        &mut self,
        operator: VimOperator,
    ) -> Task<Message> {
        if self.vim_state.mode() == VimMode::VisualLine {
            let (anchor, active) =
                self.vim_state.visual_positions().unwrap_or_else(|| {
                    let position = self.cursors.primary_position();
                    (position, position)
                });
            self.handle_vim_line_operator(
                operator,
                anchor.0.min(active.0),
                anchor.0.max(active.0),
                true,
            )
        } else {
            let Some((start, end)) = self.cursors.primary().selection_range()
            else {
                return Task::none();
            };
            self.handle_vim_character_operator(operator, start, end, true)
        }
    }

    fn handle_vim_character_operator(
        &mut self,
        operator: VimOperator,
        start: (usize, usize),
        end: (usize, usize),
        from_visual: bool,
    ) -> Task<Message> {
        if start == end {
            return Task::none();
        }
        let register = VimRegister {
            text: self.extract_text_range(start, end),
            kind: VimRegisterKind::Characterwise,
        };
        self.apply_vim_operator(operator, start, end, register, from_visual)
    }

    fn handle_vim_line_operator(
        &mut self,
        operator: VimOperator,
        start_line: usize,
        end_line: usize,
        from_visual: bool,
    ) -> Task<Message> {
        let last_line = self.buffer.line_count().saturating_sub(1);
        let start_line = start_line.min(last_line);
        let end_line = end_line.min(last_line).max(start_line);
        let mut text = String::new();
        for line in start_line..=end_line {
            text.push_str(self.buffer.line(line));
            text.push('\n');
        }

        let (start, end) = if end_line < last_line {
            ((start_line, 0), (end_line + 1, 0))
        } else if start_line > 0 {
            (
                (start_line - 1, self.buffer.line_len(start_line - 1)),
                (end_line, self.buffer.line_len(end_line)),
            )
        } else {
            ((0, 0), (end_line, self.buffer.line_len(end_line)))
        };
        self.apply_vim_operator(
            operator,
            start,
            end,
            VimRegister { text, kind: VimRegisterKind::Linewise },
            from_visual,
        )
    }

    fn apply_vim_operator(
        &mut self,
        operator: VimOperator,
        start: (usize, usize),
        end: (usize, usize),
        register: VimRegister,
        from_visual: bool,
    ) -> Task<Message> {
        self.vim_state.register = register;

        if operator == VimOperator::Yank {
            if from_visual {
                self.cursors.set_single(self.vim_normal_position(start));
            } else {
                let position =
                    self.vim_normal_position(self.cursors.primary_position());
                self.cursors.set_single(position);
            }
            self.vim_state.enter_clean_normal_mode();
            self.finish_navigation_operation();
            return self.scroll_to_cursor();
        }

        self.end_grouping_if_active();
        if operator == VimOperator::Change {
            self.ensure_grouping_started("Vim change");
        }

        self.pre_edit_line = start.0.min(end.0);
        self.pre_edit_last_line = start.0.max(end.0);
        self.capture_lsp_edit_snapshot(&Message::DeleteSelection);

        let cursor_before = self.cursors.primary_position();
        let mut command =
            DeleteRangeCommand::new(&self.buffer, start, end, cursor_before);
        let mut cursor_after = cursor_before;
        command.execute(&mut self.buffer, &mut cursor_after);
        self.history.push(Box::new(command));
        self.cursors.set_single(self.vim_normal_position(cursor_after));

        if operator == VimOperator::Change {
            self.vim_state.enter_insert_mode();
        } else {
            self.vim_state.enter_clean_normal_mode();
        }
        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    fn handle_vim_paste(
        &mut self,
        position: VimPastePosition,
        count: usize,
    ) -> Task<Message> {
        let register = self.vim_state.register.clone();
        if register.text.is_empty() {
            return Task::none();
        }
        self.end_grouping_if_active();

        let current = self.vim_normal_position(self.cursors.primary_position());
        let (insert_at, text, cursor_after) = match register.kind {
            VimRegisterKind::Characterwise => {
                let insert_at = match position {
                    VimPastePosition::BeforeCursor => current,
                    VimPastePosition::AfterCursor => (
                        current.0,
                        current
                            .1
                            .saturating_add(usize::from(
                                self.buffer.line_len(current.0) > 0,
                            ))
                            .min(self.buffer.line_len(current.0)),
                    ),
                };
                (insert_at, register.text.repeat(count.max(1)), insert_at)
            }
            VimRegisterKind::Linewise => {
                let repeated = register.text.repeat(count.max(1));
                match position {
                    VimPastePosition::BeforeCursor => {
                        ((current.0, 0), repeated, (current.0, 0))
                    }
                    VimPastePosition::AfterCursor
                        if current.0 + 1 < self.buffer.line_count() =>
                    {
                        ((current.0 + 1, 0), repeated, (current.0 + 1, 0))
                    }
                    VimPastePosition::AfterCursor => {
                        let text = format!(
                            "\n{}",
                            repeated.strip_suffix('\n').unwrap_or(&repeated)
                        );
                        (
                            (current.0, self.buffer.line_len(current.0)),
                            text,
                            (current.0 + 1, 0),
                        )
                    }
                }
            }
        };

        self.pre_edit_line = insert_at.0;
        self.pre_edit_last_line = insert_at.0;
        self.capture_lsp_edit_snapshot(&Message::Paste(text.clone()));
        let mut command =
            InsertTextCommand::new(insert_at.0, insert_at.1, text, current)
                .with_cursor_after(cursor_after);
        let mut command_cursor = current;
        command.execute(&mut self.buffer, &mut command_cursor);
        self.history.push(Box::new(command));
        self.cursors.set_single(self.vim_normal_position(command_cursor));
        self.vim_state.enter_clean_normal_mode();
        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    fn handle_vim_history(
        &mut self,
        redo: bool,
        count: usize,
    ) -> Task<Message> {
        self.end_grouping_if_active();
        self.pre_edit_line = 0;
        self.pre_edit_last_line = usize::MAX;
        self.capture_lsp_edit_snapshot(if redo {
            &Message::Redo
        } else {
            &Message::Undo
        });

        let mut cursor = self.cursors.primary_position();
        let mut changed = false;
        for _ in 0..count.max(1) {
            let applied = if redo {
                self.history.redo(&mut self.buffer, &mut cursor)
            } else {
                self.history.undo(&mut self.buffer, &mut cursor)
            };
            if !applied {
                break;
            }
            changed = true;
        }
        if !changed {
            return Task::none();
        }

        self.cursors.set_single(self.vim_normal_position(cursor));
        self.vim_state.enter_clean_normal_mode();
        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    fn handle_vim_mode(
        &mut self,
        mode: VimMode,
        previous_mode: VimMode,
    ) -> Task<Message> {
        self.end_grouping_if_active();
        match mode {
            VimMode::Normal => {
                let mut active = self
                    .vim_state
                    .visual_positions()
                    .map(|(_, active)| active)
                    .unwrap_or_else(|| self.cursors.primary_position());
                if previous_mode == VimMode::Insert {
                    active.1 = active.1.saturating_sub(1);
                }
                self.vim_state.clear_visual();
                self.cursors.set_single(self.vim_normal_position(active));
            }
            VimMode::Visual | VimMode::VisualLine => {
                let position =
                    self.vim_normal_position(self.cursors.primary_position());
                self.vim_state.begin_visual(position);
                self.apply_vim_visual_selection(
                    position,
                    position,
                    mode == VimMode::VisualLine,
                );
            }
            VimMode::Insert => {}
        }
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    fn handle_vim_motion(
        &mut self,
        motion: VimMotion,
        count: usize,
    ) -> Task<Message> {
        self.end_grouping_if_active();
        match self.vim_state.mode() {
            VimMode::Visual | VimMode::VisualLine => {
                let (anchor, active) =
                    self.vim_state.visual_positions().unwrap_or_else(|| {
                        let position = self.vim_normal_position(
                            self.cursors.primary_position(),
                        );
                        (position, position)
                    });
                let target = self.vim_motion_target(active, motion, count);
                self.vim_state.set_visual_active(target);
                self.apply_vim_visual_selection(
                    anchor,
                    target,
                    self.vim_state.mode() == VimMode::VisualLine,
                );
            }
            VimMode::Normal => {
                let target = self.vim_motion_target(
                    self.cursors.primary_position(),
                    motion,
                    count,
                );
                self.cursors.set_single(target);
                self.overlay_cache.clear();
            }
            VimMode::Insert => return Task::none(),
        }
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    fn handle_vim_insert(
        &mut self,
        position: VimInsertPosition,
        count: usize,
    ) -> Task<Message> {
        self.end_grouping_if_active();
        let current = self
            .vim_state
            .visual_positions()
            .map(|(_, active)| active)
            .unwrap_or_else(|| self.cursors.primary_position());
        self.vim_state.clear_visual();
        let current = self.vim_normal_position(current);
        self.cursors.set_single(current);
        self.ensure_grouping_started("Vim insert");

        match position {
            VimInsertPosition::BeforeCursor => {}
            VimInsertPosition::AfterCursor => {
                let line_len = self.buffer.line_len(current.0);
                self.cursors.primary_mut().position.1 =
                    current.1.saturating_add(1).min(line_len);
            }
            VimInsertPosition::FirstNonBlank => {
                self.cursors.primary_mut().position.1 = self
                    .buffer
                    .line(current.0)
                    .chars()
                    .position(|ch| !ch.is_whitespace())
                    .unwrap_or(0);
            }
            VimInsertPosition::EndOfLine => {
                self.cursors.primary_mut().position.1 =
                    self.buffer.line_len(current.0);
            }
            VimInsertPosition::NewLineBelow => {
                self.cursors.primary_mut().position.1 =
                    self.buffer.line_len(current.0);
                for _ in 0..count.max(1) {
                    let _ = self.update(&Message::Enter);
                }
            }
            VimInsertPosition::NewLineAbove => {
                self.cursors.primary_mut().position.1 = 0;
                let line = current.0;
                for _ in 0..count.max(1) {
                    let _ = self.update(&Message::Enter);
                    self.cursors.set_single((line, 0));
                }
            }
        }

        self.overlay_cache.clear();
        self.reset_cursor_blink();
        self.scroll_to_cursor()
    }

    /// Handles arrow key navigation.
    ///
    /// # Arguments
    ///
    /// * `direction` - The direction of movement
    /// * `shift_pressed` - Whether Shift is held (for selection)
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_arrow_key(
        &mut self,
        direction: ArrowDirection,
        shift_pressed: bool,
    ) -> Task<Message> {
        // End grouping on navigation
        self.end_grouping_if_active();

        if shift_pressed {
            // Set anchor on ALL cursors that don't yet have one
            for cursor in self.cursors.as_mut_slice() {
                if cursor.anchor.is_none() {
                    cursor.set_anchor();
                }
            }
            self.move_cursor(direction);
        } else {
            // Clear all selections, then move all cursors
            self.clear_selection();
            self.move_cursor(direction);
        }
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    /// Handles Home key press.
    ///
    /// Moves the cursor to the start of the current line.
    ///
    /// # Arguments
    ///
    /// * `shift_pressed` - Whether Shift is held (for selection)
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible (including
    /// horizontal scroll back to x=0 when wrap is disabled)
    fn handle_home(&mut self, shift_pressed: bool) -> Task<Message> {
        if shift_pressed {
            for cursor in self.cursors.as_mut_slice() {
                if cursor.anchor.is_none() {
                    cursor.set_anchor();
                }
                cursor.position.1 = 0;
            }
        } else {
            self.clear_selection();
            for cursor in self.cursors.as_mut_slice() {
                cursor.position.1 = 0;
            }
        }
        self.cursors.sort_and_merge();
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    /// Handles End key press.
    ///
    /// Moves the cursor to the end of the current line.
    ///
    /// # Arguments
    ///
    /// * `shift_pressed` - Whether Shift is held (for selection)
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible (including
    /// horizontal scroll to end of line when wrap is disabled)
    fn handle_end(&mut self, shift_pressed: bool) -> Task<Message> {
        if shift_pressed {
            for cursor in self.cursors.as_mut_slice() {
                if cursor.anchor.is_none() {
                    cursor.set_anchor();
                }
                cursor.position.1 = self.buffer.line_len(cursor.position.0);
            }
        } else {
            self.clear_selection();
            for cursor in self.cursors.as_mut_slice() {
                cursor.position.1 = self.buffer.line_len(cursor.position.0);
            }
        }
        self.cursors.sort_and_merge();
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    /// Handles Ctrl+Home key press.
    ///
    /// Moves the cursor to the beginning of the document.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_ctrl_home(&mut self) -> Task<Message> {
        // Move cursor to the beginning of the document
        self.clear_selection();
        self.cursors.set_single((0, 0));
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    /// Handles Ctrl+End key press.
    ///
    /// Moves the cursor to the end of the document.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_ctrl_end(&mut self) -> Task<Message> {
        // Move cursor to the end of the document
        self.clear_selection();
        let last_line = self.buffer.line_count().saturating_sub(1);
        let last_col = self.buffer.line_len(last_line);
        self.cursors.set_single((last_line, last_col));
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    /// Handles Page Up key press.
    ///
    /// Scrolls the view up by one page.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_page_up(&mut self) -> Task<Message> {
        self.page_up();
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    /// Handles Page Down key press.
    ///
    /// Scrolls the view down by one page.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_page_down(&mut self) -> Task<Message> {
        self.page_down();
        self.finish_navigation_operation();
        self.scroll_to_cursor()
    }

    /// Handles direct navigation to an explicit logical position.
    ///
    /// # Arguments
    ///
    /// * `line` - Target line index (0-based)
    /// * `col` - Target column index (0-based)
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to keep the cursor visible
    fn handle_goto_position(
        &mut self,
        line: usize,
        col: usize,
    ) -> Task<Message> {
        // End grouping on navigation command
        self.end_grouping_if_active();
        self.set_cursor(line, col)
    }

    // =========================================================================
    // Mouse and Selection Handlers
    // =========================================================================

    /// Synchronises the active search result with a manual primary-cursor
    /// position or selection.
    fn sync_search_match_from_primary_cursor(&mut self) {
        if !self.search_matches_visible() || self.search_state.query.is_empty()
        {
            return;
        }

        let primary = self.cursors.primary();
        let cursor = primary.position;
        let selection = primary.selection_range();
        if self.search_state.select_match_at_cursor(cursor, selection) {
            self.overlay_cache.clear();
        }
    }

    /// Handles mouse click operations.
    ///
    /// Sets focus, ends command grouping, positions cursor, starts selection tracking.
    ///
    /// # Arguments
    ///
    /// * `point` - The click position
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none() as no scrolling is needed)
    fn handle_mouse_click_msg(&mut self, point: iced::Point) -> Task<Message> {
        // Capture focus when clicked using the new focus method
        self.request_focus();

        // Set internal canvas focus state
        self.has_canvas_focus = true;

        // End grouping on mouse click
        self.end_grouping_if_active();

        // Regular click collapses any multi-cursor state to a single cursor
        // positioned at the click location.
        self.cursors.remove_all_but_primary();

        self.handle_mouse_click(point);
        self.reset_cursor_blink();
        // Clear selection on click, then set anchor for potential drag selection
        self.clear_selection();
        self.is_dragging = true;
        self.cursors.primary_mut().set_anchor();
        self.sync_search_match_from_primary_cursor();

        // Show cursor when focused
        self.show_cursor = true;

        Task::none()
    }

    /// Handles mouse drag operations for selection.
    ///
    /// # Arguments
    ///
    /// * `point` - The drag position
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none() as no scrolling is needed)
    fn handle_mouse_drag_msg(&mut self, point: iced::Point) -> Task<Message> {
        if self.is_dragging {
            let before_pos = self.cursors.primary_position();
            self.handle_mouse_drag(point);
            if self.cursors.primary_position() != before_pos {
                // Mouse move events can be very frequent. Only invalidate the
                // overlay cache if the drag actually changed selection/cursor.
                self.overlay_cache.clear();
            }
        }
        Task::none()
    }

    /// Handles mouse release operations.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none() as no scrolling is needed)
    fn handle_mouse_release_msg(&mut self) -> Task<Message> {
        self.is_dragging = false;
        if self.vim_enabled {
            if self.cursors.primary().has_selection() {
                let anchor = self.cursors.primary().anchor.unwrap_or_default();
                let position = self.cursors.primary_position();
                let active = if position >= anchor && position.1 > 0 {
                    (position.0, position.1 - 1)
                } else {
                    position
                };
                let anchor = self.vim_normal_position(anchor);
                let active = self.vim_normal_position(active);
                self.vim_state.set_mode_from_mouse(VimMode::Visual);
                self.vim_state.begin_visual(anchor);
                self.vim_state.set_visual_active(active);
            } else {
                let position =
                    self.vim_normal_position(self.cursors.primary_position());
                self.cursors.set_single(position);
            }
            self.overlay_cache.clear();
        }
        self.sync_search_match_from_primary_cursor();
        Task::none()
    }

    /// Handles a double-click: selects the word under the cursor.
    ///
    /// If the click lands outside any word (e.g. on whitespace), the
    /// selection is cleared and the caret is simply placed there.
    fn handle_double_click_msg(&mut self, point: iced::Point) -> Task<Message> {
        self.request_focus();
        self.has_canvas_focus = true;
        self.end_grouping_if_active();
        self.cursors.remove_all_but_primary();
        if let Some((line, col)) = self.calculate_cursor_from_point(point) {
            let line_content = self.buffer.line(line);
            let start = Self::word_start_in_line(line_content, col);
            let end = Self::word_end_in_line(line_content, col);
            let cursor = self.cursors.primary_mut();
            if start < end {
                cursor.anchor = Some((line, start));
                cursor.position = (line, end);
            } else {
                cursor.anchor = None;
                cursor.position = (line, col);
            }
        }
        self.is_dragging = false;
        self.show_cursor = true;
        self.reset_cursor_blink();
        self.overlay_cache.clear();
        self.sync_search_match_from_primary_cursor();
        Task::none()
    }

    /// Handles a triple-click: selects the whole line under the cursor.
    fn handle_triple_click_msg(&mut self, point: iced::Point) -> Task<Message> {
        self.request_focus();
        self.has_canvas_focus = true;
        self.end_grouping_if_active();
        self.cursors.remove_all_but_primary();
        if let Some((line, _col)) = self.calculate_cursor_from_point(point) {
            let line_len = self.buffer.line_len(line);
            let cursor = self.cursors.primary_mut();
            cursor.anchor = Some((line, 0));
            cursor.position = (line, line_len);
        }
        self.is_dragging = false;
        self.show_cursor = true;
        self.reset_cursor_blink();
        self.overlay_cache.clear();
        self.sync_search_match_from_primary_cursor();
        Task::none()
    }

    /// Handles a right-click before the context menu is displayed.
    ///
    /// A click inside any existing selection preserves it so Cut and Copy act
    /// on the selected text. A click elsewhere collapses the selection and
    /// moves the caret to the clicked position.
    fn handle_context_menu_requested_msg(
        &mut self,
        point: iced::Point,
    ) -> Task<Message> {
        self.request_focus();
        self.has_canvas_focus = true;
        self.focus_locked = false;
        self.show_cursor = true;
        self.is_dragging = false;
        self.end_grouping_if_active();

        if let Some(position) = self.calculate_cursor_from_point(point) {
            let inside_selection = self.cursors.iter().any(|cursor| {
                cursor.selection_range().is_some_and(|(start, end)| {
                    (start..=end).contains(&position)
                })
            });

            if !inside_selection {
                self.cursors.set_single(position);
                self.overlay_cache.clear();
            }
        }

        self.reset_cursor_blink();
        Task::none()
    }

    // =========================================================================
    // Clipboard Handlers
    // =========================================================================

    /// Cuts all selected ranges to the clipboard as a single undoable edit.
    fn handle_cut_msg(&mut self) -> Task<Message> {
        if !self.cursors.iter().any(|cursor| cursor.has_selection()) {
            return Task::none();
        }

        self.end_grouping_if_active();
        self.ensure_grouping_started("Cut");
        let clipboard_task = self.copy_selection();
        self.delete_selection();
        self.end_grouping_if_active();
        self.finish_edit_operation();

        Task::batch([clipboard_task, self.scroll_to_cursor()])
    }

    /// Selects the complete document.
    fn handle_select_all_msg(&mut self) -> Task<Message> {
        self.end_grouping_if_active();

        let last_line = self.buffer.line_count().saturating_sub(1);
        let end = (last_line, self.buffer.line_len(last_line));
        self.cursors.set_single(end);
        self.cursors.primary_mut().anchor = Some((0, 0));
        self.overlay_cache.clear();
        self.reset_cursor_blink();

        self.scroll_to_cursor()
    }

    /// Handles paste operations.
    ///
    /// If the provided text is empty, reads from clipboard. Otherwise pastes
    /// the provided text at the cursor position.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to paste (empty string triggers clipboard read)
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that may read clipboard or scroll to cursor
    fn handle_paste_msg(&mut self, text: &str) -> Task<Message> {
        // End grouping on paste
        self.end_grouping_if_active();

        // If text is empty, we need to read from clipboard
        if text.is_empty() {
            // Return a task that reads clipboard and chains to paste
            iced::clipboard::read().and_then(|clipboard_text| {
                Task::done(Message::Paste(clipboard_text))
            })
        } else {
            // We have the text, paste it
            self.paste_text(text);
            self.finish_edit_operation();
            self.scroll_to_cursor()
        }
    }

    // =========================================================================
    // History (Undo/Redo) Handlers
    // =========================================================================

    /// Handles undo operations.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to cursor if undo succeeded
    fn handle_undo_msg(&mut self) -> Task<Message> {
        // End any current grouping before undoing
        self.end_grouping_if_active();

        let mut cursor_pos = self.cursors.primary_position();
        if self.history.undo(&mut self.buffer, &mut cursor_pos) {
            self.cursors.primary_mut().position = cursor_pos;
            self.clear_selection();
            // An undone command (especially a composite like "Replace All") may
            // touch lines anywhere in the document, so reset the highlight cache
            // entirely rather than trusting the cursor as the change origin.
            self.pre_edit_line = 0;
            self.pre_edit_last_line = usize::MAX;
            self.finish_edit_operation();
            self.scroll_to_cursor()
        } else {
            Task::none()
        }
    }

    /// Handles redo operations.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to cursor if redo succeeded
    fn handle_redo_msg(&mut self) -> Task<Message> {
        let mut cursor_pos = self.cursors.primary_position();
        if self.history.redo(&mut self.buffer, &mut cursor_pos) {
            self.cursors.primary_mut().position = cursor_pos;
            self.clear_selection();
            // A redone command may touch lines anywhere; reset the highlight
            // cache entirely (see `handle_undo_msg`).
            self.pre_edit_line = 0;
            self.pre_edit_last_line = usize::MAX;
            self.finish_edit_operation();
            self.scroll_to_cursor()
        } else {
            Task::none()
        }
    }

    // =========================================================================
    // Search and Replace Handlers
    // =========================================================================

    /// Handles opening the search dialog.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that focuses and selects all in the search input
    fn handle_open_search_msg(&mut self) -> Task<Message> {
        self.goto_line_state.close();
        self.search_state.open_search();
        if !self.search_state.query.is_empty() {
            self.search_state.update_matches(&self.buffer);
            self.search_state
                .select_match_near_cursor(self.cursors.primary_position());
        }
        self.overlay_cache.clear();

        // Focus the search input and select all text if any
        Task::batch([
            focus(self.search_state.search_input_id.clone()),
            select_all(self.search_state.search_input_id.clone()),
        ])
    }

    /// Handles opening the search and replace dialog.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that focuses and selects all in the search input
    fn handle_open_search_replace_msg(&mut self) -> Task<Message> {
        self.goto_line_state.close();
        self.search_state.open_replace();
        if !self.search_state.query.is_empty() {
            self.search_state.update_matches(&self.buffer);
            self.search_state
                .select_match_near_cursor(self.cursors.primary_position());
        }
        self.overlay_cache.clear();

        // Focus the search input and select all text if any
        Task::batch([
            focus(self.search_state.search_input_id.clone()),
            select_all(self.search_state.search_input_id.clone()),
        ])
    }

    /// Handles closing the search dialog.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_close_search_msg(&mut self) -> Task<Message> {
        // Escape with multiple cursors and no open search: collapse to primary cursor
        if self.cursors.is_multi() && !self.search_state.is_open {
            self.cursors.remove_all_but_primary();
            self.overlay_cache.clear();
            return Task::none();
        }
        self.search_state.close();
        self.overlay_cache.clear();
        Task::none()
    }

    /// Opens the go-to-line input and selects the current one-based line.
    fn handle_open_goto_line_msg(&mut self) -> Task<Message> {
        self.search_state.close();
        self.goto_line_state.open(self.cursors.primary_position().0);
        self.overlay_cache.clear();

        Task::batch([
            focus(self.goto_line_state.input_id.clone()),
            select_all(self.goto_line_state.input_id.clone()),
        ])
    }

    /// Closes the go-to-line input without moving the cursor.
    fn handle_close_goto_line_msg(&mut self) -> Task<Message> {
        self.goto_line_state.close();
        self.overlay_cache.clear();
        Task::none()
    }

    /// Updates the one-based line number entered by the user.
    fn handle_goto_line_changed_msg(&mut self, query: &str) -> Task<Message> {
        self.goto_line_state.query = query.to_string();
        Task::none()
    }

    /// Moves to the submitted one-based line and closes the input.
    fn handle_submit_goto_line_msg(&mut self) -> Task<Message> {
        let Some(one_based_line) = self.goto_line_state.target_line() else {
            return Task::none();
        };

        let target_line = one_based_line
            .saturating_sub(1)
            .min(self.buffer.line_count().saturating_sub(1));
        while self.hidden_lines_set().contains(&target_line) {
            let collapsed_count = self.collapsed_folds.len();
            self.unfold_at(target_line);
            if self.collapsed_folds.len() == collapsed_count {
                break;
            }
        }

        self.goto_line_state.close();
        self.handle_goto_position(target_line, 0)
    }

    /// Handles search query text changes.
    ///
    /// # Arguments
    ///
    /// * `query` - The new search query
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to first match if any
    fn handle_search_query_changed_msg(
        &mut self,
        query: &str,
    ) -> Task<Message> {
        self.search_state.set_query(query.to_string(), &self.buffer);
        self.overlay_cache.clear();

        // Move cursor to first match if any
        if let Some(match_pos) = self.search_state.current_match() {
            self.cursors.primary_mut().position =
                (match_pos.line, match_pos.col);
            self.clear_selection();
            return self.scroll_to_cursor();
        }
        Task::none()
    }

    /// Handles replace query text changes.
    ///
    /// # Arguments
    ///
    /// * `replace_text` - The new replacement text
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_replace_query_changed_msg(
        &mut self,
        replace_text: &str,
    ) -> Task<Message> {
        self.search_state.set_replace_with(replace_text.to_string());
        Task::none()
    }

    /// Handles toggling case-sensitive search.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to first match if any
    fn handle_toggle_case_sensitive_msg(&mut self) -> Task<Message> {
        self.search_state.toggle_case_sensitive(&self.buffer);
        self.overlay_cache.clear();

        // Move cursor to first match if any
        if let Some(match_pos) = self.search_state.current_match() {
            self.cursors.primary_mut().position =
                (match_pos.line, match_pos.col);
            self.clear_selection();
            return self.scroll_to_cursor();
        }
        Task::none()
    }

    /// Handles finding the next match.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to the next match if any
    fn handle_find_next_msg(&mut self) -> Task<Message> {
        if !self.search_state.matches.is_empty() {
            self.search_state.next_match();
            if let Some(match_pos) = self.search_state.current_match() {
                self.cursors.primary_mut().position =
                    (match_pos.line, match_pos.col);
                self.clear_selection();
                self.overlay_cache.clear();
                return self.scroll_to_cursor();
            }
        }
        Task::none()
    }

    /// Handles finding the previous match.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to the previous match if any
    fn handle_find_previous_msg(&mut self) -> Task<Message> {
        if !self.search_state.matches.is_empty() {
            self.search_state.previous_match();
            if let Some(match_pos) = self.search_state.current_match() {
                self.cursors.primary_mut().position =
                    (match_pos.line, match_pos.col);
                self.clear_selection();
                self.overlay_cache.clear();
                return self.scroll_to_cursor();
            }
        }
        Task::none()
    }

    /// Handles replacing the current match and moving to the next.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to the next match if any
    fn handle_replace_next_msg(&mut self) -> Task<Message> {
        // Replace current match and move to next
        if let Some(match_pos) = self.search_state.current_match() {
            let query_len = self.search_state.query.chars().count();
            let replace_text = self.search_state.replace_with.clone();

            // Create and execute replace command
            let pos = self.cursors.primary_position();
            let mut cmd = ReplaceTextCommand::new(
                &self.buffer,
                (match_pos.line, match_pos.col),
                query_len,
                replace_text,
                pos,
            );
            let mut cursor_pos = pos;
            cmd.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.primary_mut().position = cursor_pos;
            self.history.push(Box::new(cmd));

            // The replacement starts at the matched line; invalidate highlight
            // from there regardless of where the cursor moved next.
            self.pre_edit_line = self.pre_edit_line.min(match_pos.line);
            self.pre_edit_last_line =
                self.pre_edit_last_line.max(match_pos.line);

            self.clear_selection();
            self.finish_edit_operation();

            // Move to the closest remaining match after the replacement.
            if !self.search_state.matches.is_empty()
                && let Some(next_match) = self.search_state.current_match()
            {
                self.cursors.primary_mut().position =
                    (next_match.line, next_match.col);
            }

            return self.scroll_to_cursor();
        }
        Task::none()
    }

    /// Handles replacing all matches.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to cursor after replacement
    fn handle_replace_all_msg(&mut self) -> Task<Message> {
        // Perform a fresh search to find ALL matches (ignoring the display limit)
        let all_matches = super::search::find_matches(
            &self.buffer,
            &self.search_state.query,
            self.search_state.case_sensitive,
            None, // No limit for Replace All
        );

        if !all_matches.is_empty() {
            let query_len = self.search_state.query.chars().count();
            let replace_text = self.search_state.replace_with.clone();

            // Create composite command for undo
            let mut composite =
                CompositeCommand::new("Replace All".to_string());

            // Process matches in reverse order (to preserve positions)
            for match_pos in all_matches.iter().rev() {
                let pos = self.cursors.primary_position();
                let cmd = ReplaceTextCommand::new(
                    &self.buffer,
                    (match_pos.line, match_pos.col),
                    query_len,
                    replace_text.clone(),
                    pos,
                );
                composite.add(Box::new(cmd));
            }

            // Execute all replacements
            let mut cursor_pos = self.cursors.primary_position();
            composite.execute(&mut self.buffer, &mut cursor_pos);
            self.cursors.primary_mut().position = cursor_pos;
            self.history.push(Box::new(composite));

            // Replace All touches matches anywhere in the document, so reset
            // the highlight cache entirely.
            self.pre_edit_line = 0;
            self.pre_edit_last_line = usize::MAX;

            self.clear_selection();
            self.finish_edit_operation();
            self.scroll_to_cursor()
        } else {
            Task::none()
        }
    }

    /// Handles Tab key in search dialog (cycle forward).
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that focuses the next field
    fn handle_search_dialog_tab_msg(&mut self) -> Task<Message> {
        // Cycle focus forward (Search → Replace → Search)
        self.search_state.focus_next_field();

        // Focus the appropriate input based on new focused_field
        match self.search_state.focused_field {
            crate::canvas_editor::search::SearchFocusedField::Search => {
                focus(self.search_state.search_input_id.clone())
            }
            crate::canvas_editor::search::SearchFocusedField::Replace => {
                focus(self.search_state.replace_input_id.clone())
            }
        }
    }

    /// Handles Shift+Tab key in search dialog (cycle backward).
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that focuses the previous field
    fn handle_search_dialog_shift_tab_msg(&mut self) -> Task<Message> {
        // Cycle focus backward (Replace → Search → Replace)
        self.search_state.focus_previous_field();

        // Focus the appropriate input based on new focused_field
        match self.search_state.focused_field {
            crate::canvas_editor::search::SearchFocusedField::Search => {
                focus(self.search_state.search_input_id.clone())
            }
            crate::canvas_editor::search::SearchFocusedField::Replace => {
                focus(self.search_state.replace_input_id.clone())
            }
        }
    }

    // =========================================================================
    // Focus and IME Handlers
    // =========================================================================

    /// Handles canvas focus gained event.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_canvas_focus_gained_msg(&mut self) -> Task<Message> {
        self.has_canvas_focus = true;
        self.focus_locked = false; // Unlock focus when gained
        self.show_cursor = true;
        self.reset_cursor_blink();
        self.overlay_cache.clear();
        Task::none()
    }

    /// Handles canvas focus lost event.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_canvas_focus_lost_msg(&mut self) -> Task<Message> {
        self.has_canvas_focus = false;
        self.focus_locked = true; // Lock focus when lost to prevent focus stealing
        self.show_cursor = false;
        self.ime_preedit = None;
        self.overlay_cache.clear();
        Task::none()
    }

    /// Handles IME opened event.
    ///
    /// Clears current preedit content to accept new input.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_ime_opened_msg(&mut self) -> Task<Message> {
        self.ime_preedit = None;
        self.overlay_cache.clear();
        Task::none()
    }

    /// Handles IME preedit event.
    ///
    /// Updates the preedit text and selection while the user is composing.
    ///
    /// # Arguments
    ///
    /// * `content` - The preedit text content
    /// * `selection` - The selection range within the preedit text
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_ime_preedit_msg(
        &mut self,
        content: &str,
        selection: &Option<std::ops::Range<usize>>,
    ) -> Task<Message> {
        if content.is_empty() {
            self.ime_preedit = None;
        } else {
            self.ime_preedit = Some(ImePreedit {
                content: content.to_string(),
                selection: selection.clone(),
            });
        }

        self.overlay_cache.clear();
        Task::none()
    }

    /// Handles IME commit event.
    ///
    /// Inserts the committed text at the cursor position.
    ///
    /// # Arguments
    ///
    /// * `text` - The committed text
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that scrolls to cursor after insertion
    fn handle_ime_commit_msg(&mut self, text: &str) -> Task<Message> {
        self.ime_preedit = None;

        if text.is_empty() || !self.vim_accepts_insert_input() {
            self.overlay_cache.clear();
            return Task::none();
        }

        self.ensure_grouping_started("Typing");

        self.paste_text(text);
        self.finish_edit_operation();
        self.scroll_to_cursor()
    }

    /// Handles IME closed event.
    ///
    /// Clears preedit state to return to normal input mode.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_ime_closed_msg(&mut self) -> Task<Message> {
        self.ime_preedit = None;
        self.overlay_cache.clear();
        Task::none()
    }

    // =========================================================================
    // Complex Standalone Handlers
    // =========================================================================

    /// Handles cursor blink tick event.
    ///
    /// Updates cursor visibility for blinking animation.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_tick_msg(&mut self) -> Task<Message> {
        // Handle cursor blinking only if editor has focus
        if self.has_focus()
            && self.last_blink.elapsed() >= CURSOR_BLINK_INTERVAL
        {
            self.cursor_visible = !self.cursor_visible;
            self.last_blink = super::Instant::now();
            self.overlay_cache.clear();
        }

        // Hide cursor if editor doesn't have focus
        if !self.has_focus() {
            self.show_cursor = false;
        }

        Task::none()
    }

    /// Handles viewport scrolled event.
    ///
    /// Manages the virtual scrolling cache window to optimize rendering
    /// for large files. Only clears the cache when scrolling crosses the
    /// cached window boundary or when viewport dimensions change.
    ///
    /// # Arguments
    ///
    /// * `viewport` - The viewport information after scrolling
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently Task::none())
    fn handle_scrolled_msg(
        &mut self,
        viewport: iced::widget::scrollable::Viewport,
    ) -> Task<Message> {
        // Virtual-scrolling cache window:
        // Instead of clearing the canvas cache for every small scroll,
        // we maintain a larger "render window" of visual lines around
        // the visible range. We only clear the cache and re-window
        // when the scroll crosses the window boundary or the viewport
        // size changes significantly. This prevents frequent re-highlighting
        // and layout recomputation for very large files while ensuring
        // the first scroll renders correctly without requiring a click.
        let new_scroll = viewport.absolute_offset().y;
        let new_height = viewport.bounds().height;
        let new_width = viewport.bounds().width;
        let scroll_changed = (self.viewport_scroll - new_scroll).abs() > 0.1;
        let visible_lines_count =
            (new_height / self.line_height).ceil() as usize + 2;
        let first_visible_line =
            (new_scroll / self.line_height).floor() as usize;
        let last_visible_line = first_visible_line + visible_lines_count;
        let margin = visible_lines_count
            * crate::canvas_editor::CACHE_WINDOW_MARGIN_MULTIPLIER;
        let window_start = first_visible_line.saturating_sub(margin);
        let window_end = last_visible_line + margin;
        // Decide whether we need to re-window the cache.
        // Special-case top-of-file: when window_start == 0, allow small forward scrolls
        // without forcing a rewindow, to avoid thrashing when the visible range is near 0.
        let need_rewindow =
            if self.cache_window_end_line > self.cache_window_start_line {
                let lower_boundary_trigger = self.cache_window_start_line > 0
                    && first_visible_line
                        < self
                            .cache_window_start_line
                            .saturating_add(visible_lines_count / 2);
                let upper_boundary_trigger = last_visible_line
                    > self
                        .cache_window_end_line
                        .saturating_sub(visible_lines_count / 2);
                lower_boundary_trigger || upper_boundary_trigger
            } else {
                true
            };
        // Clear cache when viewport dimensions change significantly
        // to ensure proper redraw (e.g., window resize)
        if (self.viewport_height - new_height).abs() > 1.0
            || (self.viewport_width - new_width).abs() > 1.0
            || (scroll_changed && need_rewindow)
        {
            self.cache_window_start_line = window_start;
            self.cache_window_end_line = window_end;
            self.last_first_visible_line = first_visible_line;
            self.content_cache.clear();
            self.overlay_cache.clear();
        }
        self.viewport_scroll = new_scroll;
        self.viewport_height = new_height;
        self.viewport_width = new_width;
        Task::none()
    }

    /// Handles horizontal scrollbar scrolled event (only active when wrap is disabled).
    ///
    /// Updates `horizontal_scroll_offset` and clears render caches when the offset
    /// changes by more than 0.1 pixels to avoid unnecessary redraws.
    ///
    /// # Arguments
    ///
    /// * `viewport` - The viewport information after scrolling
    ///
    /// # Returns
    ///
    /// A `Task<Message>` (currently `Task::none()`)
    fn handle_horizontal_scrolled_msg(
        &mut self,
        viewport: iced::widget::scrollable::Viewport,
    ) -> Task<Message> {
        let new_x = viewport.absolute_offset().x;
        if (self.horizontal_scroll_offset - new_x).abs() > 0.1 {
            self.horizontal_scroll_offset = new_x;
            self.content_cache.clear();
            self.overlay_cache.clear();
        }
        Task::none()
    }

    // =========================================================================
    // Multi-cursor operations
    // =========================================================================

    /// Handles Alt+Click: adds a new cursor at the clicked position without
    /// disturbing existing cursors.
    ///
    /// # Arguments
    ///
    /// * `point` - Canvas-local position of the click
    ///
    /// # Returns
    ///
    /// `Task::none()` — no async work needed
    fn handle_alt_click_msg(&mut self, point: iced::Point) -> Task<Message> {
        if self.vim_enabled {
            return Task::none();
        }
        if let Some(pos) = self.calculate_cursor_from_point(point) {
            self.cursors.add_cursor(pos);
            self.overlay_cache.clear();
            self.reset_cursor_blink();
        }
        Task::none()
    }

    /// Handles Ctrl+Alt+Up: adds a cursor on the line above the primary cursor,
    /// at the same column (clamped to line length).
    ///
    /// # Returns
    ///
    /// `Task::none()`
    fn handle_add_cursor_above_msg(&mut self) -> Task<Message> {
        if self.vim_enabled {
            return Task::none();
        }
        let (line, col) = self.cursors.primary_position();
        if line == 0 {
            return Task::none();
        }
        let new_line = line - 1;
        let new_col = col.min(self.buffer.line_len(new_line));
        self.cursors.add_cursor((new_line, new_col));
        self.overlay_cache.clear();
        self.reset_cursor_blink();
        Task::none()
    }

    /// Handles Ctrl+Alt+Down: adds a cursor on the line below the primary cursor,
    /// at the same column (clamped to line length).
    ///
    /// # Returns
    ///
    /// `Task::none()`
    fn handle_add_cursor_below_msg(&mut self) -> Task<Message> {
        if self.vim_enabled {
            return Task::none();
        }
        let (line, col) = self.cursors.primary_position();
        let last_line = self.buffer.line_count().saturating_sub(1);
        if line >= last_line {
            return Task::none();
        }
        let new_line = line + 1;
        let new_col = col.min(self.buffer.line_len(new_line));
        self.cursors.add_cursor((new_line, new_col));
        self.overlay_cache.clear();
        self.reset_cursor_blink();
        Task::none()
    }

    /// Handles Ctrl+D: selects the next occurrence of the text currently selected
    /// by the primary cursor, or the word under the primary cursor if there is no
    /// selection. A new cursor with that selection is added.
    ///
    /// # Returns
    ///
    /// `Task::none()`
    fn handle_select_next_occurrence_msg(&mut self) -> Task<Message> {
        if self.vim_enabled {
            return Task::none();
        }
        // Determine the search text: selected text on primary cursor, or word under cursor
        let search_text = if let Some(text) = self.get_selected_text() {
            text
        } else {
            // Select word under primary cursor first
            let (line, col) = self.cursors.primary_position();
            let line_str = self.buffer.line(line).to_string();
            let word_start = Self::word_start_in_line(&line_str, col);
            let word_end = Self::word_end_in_line(&line_str, col);
            if word_start == word_end {
                return Task::none();
            }
            // Apply selection to primary cursor and stop: the next Ctrl+D call
            // will find the next occurrence (selection will be non-empty then).
            self.cursors.primary_mut().anchor = Some((line, word_start));
            self.cursors.primary_mut().position = (line, word_end);
            self.overlay_cache.clear();
            return Task::none();
        };

        if search_text.is_empty() {
            return Task::none();
        }

        // Find the search start position: just after the last cursor's selection end
        let search_start = self
            .cursors
            .as_slice()
            .last()
            .map(|last| {
                last.selection_range()
                    .map(|(_, end)| end)
                    .unwrap_or(last.position)
            })
            .unwrap_or((0, 0));

        // Search forward from search_start for the next occurrence
        let (start_line, start_col) = search_start;
        let line_count = self.buffer.line_count();
        let search_char_len = search_text.chars().count();

        for line_offset in 0..=line_count {
            let line_idx = (start_line + line_offset) % line_count;
            let line_str = self.buffer.line(line_idx);

            // On the first iteration, start after start_col; on wrap-around, start from 0
            let search_col = if line_offset == 0 { start_col } else { 0 };

            // Build substring from search_col onward (char-indexed)
            let prefix_bytes = char_to_byte_index(line_str, search_col);
            let haystack = &line_str[prefix_bytes..];

            // The search_text is also char-based; find it as a substring
            if let Some(byte_offset) = haystack.find(search_text.as_str()) {
                // Convert byte_offset back to char offset
                let char_start =
                    search_col + haystack[..byte_offset].chars().count();
                let char_end = char_start + search_char_len;

                // Build cursor with selection for the found occurrence
                let found_cursor = cursor_set::Cursor {
                    position: (line_idx, char_end),
                    anchor: Some((line_idx, char_start)),
                };
                self.cursors.add_cursor_with_selection(found_cursor);
                self.overlay_cache.clear();
                self.reset_cursor_blink();
                return self.scroll_to_cursor();
            }
        }

        Task::none()
    }

    // =========================================================================
    // Main Update Method
    // =========================================================================

    /// Updates the editor state based on messages and returns scroll commands.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to process for updating the editor state
    ///
    /// # Returns
    /// A `Task<Message>` for any asynchronous operations, such as scrolling to keep the cursor visible after state updates
    pub fn update(&mut self, message: &Message) -> Task<Message> {
        // Capture the topmost active line before any edit mutates the buffer,
        // so `finish_edit_operation` can truncate the highlight cache precisely.
        self.pre_edit_line = self.min_active_line();
        self.pre_edit_last_line = self.max_active_line();
        self.capture_lsp_edit_snapshot(message);
        match message {
            // Text input operations
            Message::CharacterInput(ch) if self.vim_accepts_insert_input() => {
                self.handle_character_input_msg(*ch)
            }
            Message::CharacterInput(_) => Task::none(),
            Message::VimKey(ch) => self.handle_vim_key_msg(*ch),
            Message::ToggleVimMode => {
                self.set_vim_enabled(!self.vim_enabled);
                Task::none()
            }
            Message::Tab if self.vim_accepts_insert_input() => {
                self.handle_tab()
            }
            Message::Enter if self.vim_accepts_insert_input() => {
                self.handle_enter()
            }
            Message::Tab | Message::Enter => Task::none(),

            // Deletion operations
            Message::Backspace if self.vim_accepts_insert_input() => {
                self.handle_backspace()
            }
            Message::Delete if self.vim_accepts_insert_input() => {
                self.handle_delete()
            }
            Message::Backspace | Message::Delete => Task::none(),
            Message::DeleteSelection => self.handle_delete_selection(),

            // Navigation operations
            Message::ArrowKey(direction, shift) => {
                self.handle_arrow_key(*direction, *shift)
            }
            Message::Home(shift) => self.handle_home(*shift),
            Message::End(shift) => self.handle_end(*shift),
            Message::CtrlHome => self.handle_ctrl_home(),
            Message::CtrlEnd => self.handle_ctrl_end(),
            Message::GotoPosition(line, col) => {
                self.handle_goto_position(*line, *col)
            }
            Message::OpenGotoLine => self.handle_open_goto_line_msg(),
            Message::CloseGotoLine => self.handle_close_goto_line_msg(),
            Message::GotoLineChanged(query) => {
                self.handle_goto_line_changed_msg(query)
            }
            Message::SubmitGotoLine => self.handle_submit_goto_line_msg(),
            Message::PageUp => self.handle_page_up(),
            Message::PageDown => self.handle_page_down(),

            // Mouse and selection operations
            Message::MouseClick(point) => self.handle_mouse_click_msg(*point),
            Message::MouseDrag(point) => self.handle_mouse_drag_msg(*point),
            Message::MouseHover(point) => self.handle_mouse_drag_msg(*point),
            Message::MouseRelease => self.handle_mouse_release_msg(),
            Message::DoubleClick(point) => self.handle_double_click_msg(*point),
            Message::TripleClick(point) => self.handle_triple_click_msg(*point),
            Message::ContextMenuRequested(point) => {
                self.handle_context_menu_requested_msg(*point)
            }
            Message::WriteRequested
            | Message::CustomContextMenuAction(_)
            | Message::RevealInFileManager => Task::none(),

            // Clipboard operations
            Message::Cut => self.handle_cut_msg(),
            Message::Copy => self.copy_selection(),
            Message::Paste(text) => self.handle_paste_msg(text),
            Message::SelectAll => self.handle_select_all_msg(),

            // History operations
            Message::Undo => self.handle_undo_msg(),
            Message::Redo => self.handle_redo_msg(),

            // Search and replace operations
            Message::OpenSearch => self.handle_open_search_msg(),
            Message::OpenSearchReplace => self.handle_open_search_replace_msg(),
            Message::CloseSearch => self.handle_close_search_msg(),
            Message::SearchQueryChanged(query) => {
                self.handle_search_query_changed_msg(query)
            }
            Message::ReplaceQueryChanged(text) => {
                self.handle_replace_query_changed_msg(text)
            }
            Message::ToggleCaseSensitive => {
                self.handle_toggle_case_sensitive_msg()
            }
            Message::FindNext => self.handle_find_next_msg(),
            Message::FindPrevious => self.handle_find_previous_msg(),
            Message::ReplaceNext => self.handle_replace_next_msg(),
            Message::ReplaceAll => self.handle_replace_all_msg(),
            Message::SearchDialogTab => self.handle_search_dialog_tab_msg(),
            Message::SearchDialogShiftTab => {
                self.handle_search_dialog_shift_tab_msg()
            }
            Message::FocusNavigationTab => self.handle_focus_navigation_tab(),
            Message::FocusNavigationShiftTab => {
                self.handle_focus_navigation_shift_tab()
            }

            // Focus and IME operations
            Message::CanvasFocusGained => self.handle_canvas_focus_gained_msg(),
            Message::CanvasFocusLost => self.handle_canvas_focus_lost_msg(),
            Message::ImeOpened if self.vim_accepts_insert_input() => {
                self.handle_ime_opened_msg()
            }
            Message::ImeOpened => Task::none(),
            Message::ImePreedit(content, selection) => {
                if self.vim_accepts_insert_input() {
                    self.handle_ime_preedit_msg(content, selection)
                } else {
                    Task::none()
                }
            }
            Message::ImeCommit(text) => self.handle_ime_commit_msg(text),
            Message::ImeClosed => self.handle_ime_closed_msg(),

            // UI update operations
            Message::Tick => self.handle_tick_msg(),
            Message::Scrolled(viewport) => self.handle_scrolled_msg(*viewport),
            Message::HorizontalScrolled(viewport) => {
                self.handle_horizontal_scrolled_msg(*viewport)
            }

            // Handle the "Jump to Definition" action triggered by Ctrl+Click.
            // Currently, this returns `Task::none()` as the actual navigation logic
            // is delegated to the `LspClient` implementation or handled elsewhere.
            Message::JumpClick(_point) => Task::none(),

            // Multi-cursor operations
            Message::AltClick(point) => self.handle_alt_click_msg(*point),
            Message::AddCursorAbove => self.handle_add_cursor_above_msg(),
            Message::AddCursorBelow => self.handle_add_cursor_below_msg(),
            Message::SelectNextOccurrence => {
                self.handle_select_next_occurrence_msg()
            }
            Message::ToggleFold(header_line) => {
                self.toggle_fold(*header_line);
                Task::none()
            }
            Message::ToggleFoldAtCursor => {
                self.toggle_fold_at(self.cursors.primary_position().0);
                Task::none()
            }
            Message::FoldAll => {
                self.fold_all();
                Task::none()
            }
            Message::UnfoldAll => {
                self.unfold_all();
                Task::none()
            }

            // Line manipulation operations
            Message::MoveLineUp => self.move_lines(false),
            Message::MoveLineDown => self.move_lines(true),
            Message::DuplicateLineUp => self.duplicate_lines(false),
            Message::DuplicateLineDown => self.duplicate_lines(true),
            Message::ToggleComment => self.toggle_comment(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_editor::{ArrowDirection, VimMode};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn vim_keys(editor: &mut CodeEditor, keys: &str) {
        for key in keys.chars() {
            let _ = editor.update(&Message::VimKey(key));
        }
    }

    fn focus_editor(editor: &mut CodeEditor) {
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
    }

    fn assert_vim_delete(
        content: &str,
        cursor: (usize, usize),
        keys: &str,
        expected: &str,
        register: &str,
    ) {
        let mut editor = CodeEditor::new(content, "txt").with_vim_enabled(true);
        editor.cursors.set_single(cursor);
        vim_keys(&mut editor, keys);
        assert_eq!(editor.content(), expected, "keys: {keys}");
        assert_eq!(editor.vim_state.register.text, register, "keys: {keys}");
    }

    #[derive(Default)]
    struct VimTestLspClient {
        changes: Rc<RefCell<Vec<Vec<lsp::LspTextChange>>>>,
    }

    impl lsp::LspClient for VimTestLspClient {
        fn did_change(
            &mut self,
            _document: &lsp::LspDocument,
            changes: &[lsp::LspTextChange],
        ) {
            self.changes.borrow_mut().push(changes.to_vec());
        }
    }

    #[test]
    fn test_vim_navigation_normal_key_does_not_insert() {
        let mut editor = CodeEditor::new("abc", "txt").with_vim_enabled(true);

        vim_keys(&mut editor, "l");

        assert_eq!(editor.content(), "abc");
        assert_eq!(editor.cursors.primary_position(), (0, 1));

        let mut standard = CodeEditor::new("abc", "txt");
        focus_editor(&mut standard);
        let _ = standard.update(&Message::CharacterInput('l'));
        assert_eq!(standard.content(), "labc");
    }

    #[test]
    fn test_vim_navigation_insert_and_escape_round_trip() {
        let mut editor = CodeEditor::new("abc", "txt").with_vim_enabled(true);
        focus_editor(&mut editor);

        vim_keys(&mut editor, "i");
        assert_eq!(editor.vim_mode(), Some(VimMode::Insert));
        let _ = editor.update(&Message::CharacterInput('X'));
        assert_eq!(editor.content(), "Xabc");

        let _ = editor.update(&Message::VimKey('\u{1b}'));
        assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
        vim_keys(&mut editor, "l");
        assert_eq!(editor.content(), "Xabc");
        assert_eq!(editor.cursors.primary_position(), (0, 1));
    }

    #[test]
    fn test_vim_navigation_counted_word_and_line_motions() {
        let mut editor =
            CodeEditor::new("one two\nthree four\nfive six", "txt")
                .with_vim_enabled(true);

        vim_keys(&mut editor, "2w");
        assert_eq!(editor.cursors.primary_position(), (1, 0));
        vim_keys(&mut editor, "e");
        assert_eq!(editor.cursors.primary_position(), (1, 4));
        vim_keys(&mut editor, "b");
        assert_eq!(editor.cursors.primary_position(), (1, 0));
        vim_keys(&mut editor, "G");
        assert_eq!(editor.cursors.primary_position(), (2, 0));
        vim_keys(&mut editor, "gg");
        assert_eq!(editor.cursors.primary_position(), (0, 0));
        vim_keys(&mut editor, "2j");
        assert_eq!(editor.cursors.primary_position(), (2, 0));
        vim_keys(&mut editor, "k$");
        assert_eq!(editor.cursors.primary_position(), (1, 9));
        vim_keys(&mut editor, "0");
        assert_eq!(editor.cursors.primary_position(), (1, 0));

        let mut folded = CodeEditor::new(
            "fn main() {\n    let x = 1;\n    if x > 0 {\n        print();\n    }\n}",
            "rs",
        )
        .with_vim_enabled(true);
        folded.toggle_fold(0);
        vim_keys(&mut folded, "j");
        assert_eq!(folded.cursors.primary_position(), (5, 0));
    }

    #[test]
    fn test_vim_navigation_visual_and_visual_line_ranges() {
        let mut editor = CodeEditor::new("abcd\nefgh\nijkl\nmnop", "txt")
            .with_vim_enabled(true);
        editor.cursors.set_single((0, 1));

        vim_keys(&mut editor, "vl");
        assert_eq!(editor.vim_mode(), Some(VimMode::Visual));
        assert_eq!(
            editor.cursors.primary().selection_range(),
            Some(((0, 1), (0, 3)))
        );

        let _ = editor.update(&Message::VimKey('\u{1b}'));
        assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
        assert!(editor.cursors.primary().anchor.is_none());

        editor.cursors.set_single((1, 2));
        vim_keys(&mut editor, "Vj");
        assert_eq!(editor.vim_mode(), Some(VimMode::VisualLine));
        assert_eq!(
            editor.cursors.primary().selection_range(),
            Some(((1, 0), (3, 0)))
        );
    }

    #[test]
    fn test_vim_navigation_unicode_and_empty_line_bounds() {
        let mut editor =
            CodeEditor::new("你🙂好\n\nz", "txt").with_vim_enabled(true);

        vim_keys(&mut editor, "lll");
        assert_eq!(editor.cursors.primary_position(), (0, 2));
        vim_keys(&mut editor, "j");
        assert_eq!(editor.cursors.primary_position(), (1, 0));
        vim_keys(&mut editor, "j");
        assert_eq!(editor.cursors.primary_position(), (2, 0));
        vim_keys(&mut editor, "k");
        assert_eq!(editor.cursors.primary_position(), (1, 0));
        vim_keys(&mut editor, "k$");
        assert_eq!(editor.cursors.primary_position(), (0, 2));
        vim_keys(&mut editor, "0");
        assert_eq!(editor.cursors.primary_position(), (0, 0));
    }

    #[test]
    fn test_vim_navigation_ime_only_commits_in_insert() {
        let mut editor = CodeEditor::new("abc", "txt").with_vim_enabled(true);

        let _ = editor.update(&Message::ImeCommit("中".to_owned()));
        assert_eq!(editor.content(), "abc");

        vim_keys(&mut editor, "i");
        let _ = editor.update(&Message::ImeCommit("中".to_owned()));
        assert_eq!(editor.content(), "中abc");

        let mut standard = CodeEditor::new("abc", "txt");
        let _ = standard.update(&Message::ImeCommit("中".to_owned()));
        assert_eq!(standard.content(), "中abc");
    }

    #[test]
    fn test_vim_navigation_collapses_and_blocks_extra_cursors() {
        let mut editor = CodeEditor::new("same\nsame\nsame", "txt");
        editor.cursors.add_cursor((1, 0));
        assert_eq!(editor.cursors.len(), 2);

        editor.set_vim_enabled(true);
        assert_eq!(editor.cursors.len(), 1);

        let _ = editor.update(&Message::AddCursorBelow);
        let _ = editor.update(&Message::SelectNextOccurrence);
        let _ = editor.update(&Message::SelectNextOccurrence);
        let _ = editor.update(&Message::AltClick(iced::Point::new(
            editor.gutter_width() + 5.0,
            editor.line_height,
        )));
        assert_eq!(editor.cursors.len(), 1);
    }

    #[test]
    fn test_vim_editing_x_and_count() {
        let mut editor =
            CodeEditor::new("abcdef", "txt").with_vim_enabled(true);

        vim_keys(&mut editor, "x");
        assert_eq!(editor.content(), "bcdef");
        assert_eq!(editor.vim_state.register.text, "a");
        assert_eq!(
            editor.vim_state.register.kind,
            super::super::vim::VimRegisterKind::Characterwise
        );

        vim_keys(&mut editor, "2x");
        assert_eq!(editor.content(), "def");
        assert_eq!(editor.vim_state.register.text, "bc");
        assert_eq!(editor.cursors.primary_position(), (0, 0));
    }

    #[test]
    fn test_vim_editing_delete_change_yank_motions() {
        let mut deleted =
            CodeEditor::new("one two three", "txt").with_vim_enabled(true);
        vim_keys(&mut deleted, "dw");
        assert_eq!(deleted.content(), "two three");
        assert_eq!(deleted.vim_state.register.text, "one ");

        let mut yanked =
            CodeEditor::new("one two", "txt").with_vim_enabled(true);
        vim_keys(&mut yanked, "yw");
        assert_eq!(yanked.content(), "one two");
        assert_eq!(yanked.vim_state.register.text, "one ");

        let mut changed =
            CodeEditor::new("one two", "txt").with_vim_enabled(true);
        focus_editor(&mut changed);
        vim_keys(&mut changed, "ce");
        assert_eq!(changed.content(), " two");
        assert_eq!(changed.vim_state.register.text, "one");
        assert_eq!(changed.vim_mode(), Some(VimMode::Insert));
        let _ = changed.update(&Message::CharacterInput('X'));
        vim_keys(&mut changed, "\u{1b}");
        assert_eq!(changed.content(), "X two");

        assert_vim_delete("abc", (0, 2), "dh", "ac", "b");
        assert_vim_delete("abc", (0, 1), "dl", "ac", "b");
        assert_vim_delete("abc def", (0, 6), "db", "abc f", "de");
        assert_vim_delete("abcdef", (0, 3), "d0", "def", "abc");
        assert_vim_delete("  abc", (0, 4), "d^", "  c", "ab");
        assert_vim_delete("abcde", (0, 2), "d$", "ab", "cde");
        assert_vim_delete(
            "one\ntwo\nthree\nfour",
            (2, 0),
            "dgg",
            "four",
            "one\ntwo\nthree\n",
        );
        assert_vim_delete(
            "one\ntwo\nthree\nfour",
            (1, 0),
            "dG",
            "one",
            "two\nthree\nfour\n",
        );
    }

    #[test]
    fn test_vim_editing_doubled_line_operators() {
        let mut deleted =
            CodeEditor::new("one\ntwo\nthree", "txt").with_vim_enabled(true);
        vim_keys(&mut deleted, "dd");
        assert_eq!(deleted.content(), "two\nthree");
        assert_eq!(deleted.vim_state.register.text, "one\n");
        assert_eq!(
            deleted.vim_state.register.kind,
            super::super::vim::VimRegisterKind::Linewise
        );

        let mut yanked =
            CodeEditor::new("one\ntwo\nthree", "txt").with_vim_enabled(true);
        yanked.cursors.set_single((1, 1));
        vim_keys(&mut yanked, "yy");
        assert_eq!(yanked.content(), "one\ntwo\nthree");
        assert_eq!(yanked.vim_state.register.text, "two\n");
        assert_eq!(
            yanked.vim_state.register.kind,
            super::super::vim::VimRegisterKind::Linewise
        );

        let mut changed =
            CodeEditor::new("one\ntwo\nthree", "txt").with_vim_enabled(true);
        changed.cursors.set_single((1, 1));
        vim_keys(&mut changed, "cc");
        assert_eq!(changed.content(), "one\nthree");
        assert_eq!(changed.vim_state.register.text, "two\n");
        assert_eq!(changed.vim_mode(), Some(VimMode::Insert));

        assert_vim_delete(
            "one\ntwo\nthree\nfour",
            (1, 0),
            "dk",
            "three\nfour",
            "one\ntwo\n",
        );
        assert_vim_delete(
            "one\ntwo\nthree\nfour",
            (1, 0),
            "dj",
            "one\nfour",
            "two\nthree\n",
        );
        assert_vim_delete(
            "one\ntwo\nthree",
            (0, 0),
            "2dd",
            "three",
            "one\ntwo\n",
        );
    }

    #[test]
    fn test_vim_editing_visual_operators() {
        let mut deleted =
            CodeEditor::new("abcd\nefgh", "txt").with_vim_enabled(true);
        deleted.cursors.set_single((0, 1));
        vim_keys(&mut deleted, "vld");
        assert_eq!(deleted.content(), "ad\nefgh");
        assert_eq!(deleted.vim_state.register.text, "bc");
        assert_eq!(
            deleted.vim_state.register.kind,
            super::super::vim::VimRegisterKind::Characterwise
        );
        assert_eq!(deleted.vim_mode(), Some(VimMode::Normal));

        let mut yanked =
            CodeEditor::new("one\ntwo\nthree", "txt").with_vim_enabled(true);
        yanked.cursors.set_single((1, 1));
        vim_keys(&mut yanked, "Vjy");
        assert_eq!(yanked.content(), "one\ntwo\nthree");
        assert_eq!(yanked.vim_state.register.text, "two\nthree\n");
        assert_eq!(
            yanked.vim_state.register.kind,
            super::super::vim::VimRegisterKind::Linewise
        );
        assert_eq!(yanked.vim_mode(), Some(VimMode::Normal));

        let mut changed = CodeEditor::new("abcd", "txt").with_vim_enabled(true);
        focus_editor(&mut changed);
        changed.cursors.set_single((0, 1));
        vim_keys(&mut changed, "vlc");
        let _ = changed.update(&Message::CharacterInput('X'));
        vim_keys(&mut changed, "\u{1b}");
        assert_eq!(changed.content(), "aXd");
        assert_eq!(changed.vim_state.register.text, "bc");
        assert_eq!(changed.history.undo_count(), 1);
    }

    #[test]
    fn test_vim_editing_characterwise_and_linewise_paste() {
        let mut characterwise =
            CodeEditor::new("abc", "txt").with_vim_enabled(true);
        vim_keys(&mut characterwise, "yl2lp");
        assert_eq!(characterwise.content(), "abca");
        assert_eq!(characterwise.cursors.primary_position(), (0, 3));

        let mut characterwise_before =
            CodeEditor::new("abc", "txt").with_vim_enabled(true);
        vim_keys(&mut characterwise_before, "yl2lP");
        assert_eq!(characterwise_before.content(), "abac");
        assert_eq!(characterwise_before.cursors.primary_position(), (0, 2));

        let mut linewise =
            CodeEditor::new("one\ntwo", "txt").with_vim_enabled(true);
        vim_keys(&mut linewise, "yyp");
        assert_eq!(linewise.content(), "one\none\ntwo");
        assert_eq!(linewise.cursors.primary_position(), (1, 0));

        let mut linewise_before =
            CodeEditor::new("one\ntwo", "txt").with_vim_enabled(true);
        linewise_before.cursors.set_single((1, 0));
        vim_keys(&mut linewise_before, "yyP");
        assert_eq!(linewise_before.content(), "one\ntwo\ntwo");
        assert_eq!(linewise_before.cursors.primary_position(), (1, 0));
    }

    #[test]
    fn test_vim_editing_operator_counts_multiply() {
        let mut editor =
            CodeEditor::new("one two three four five six seven", "txt")
                .with_vim_enabled(true);

        vim_keys(&mut editor, "2d3w");

        assert_eq!(editor.content(), "seven");
        assert_eq!(
            editor.vim_state.register.text,
            "one two three four five six "
        );
    }

    #[test]
    fn test_vim_editing_undo_redo_is_one_command() {
        let original = "one two three";
        let mut editor =
            CodeEditor::new(original, "txt").with_vim_enabled(true);
        focus_editor(&mut editor);

        vim_keys(&mut editor, "cw");
        let _ = editor.update(&Message::CharacterInput('X'));
        let _ = editor.update(&Message::CharacterInput('Y'));
        vim_keys(&mut editor, "\u{1b}");
        assert_eq!(editor.content(), "XYtwo three");
        assert_eq!(editor.history.undo_count(), 1);

        vim_keys(&mut editor, "u");
        assert_eq!(editor.content(), original);
        assert_eq!(editor.history.redo_count(), 1);

        vim_keys(&mut editor, "\u{12}");
        assert_eq!(editor.content(), "XYtwo three");
        assert_eq!(editor.history.undo_count(), 1);

        let mut opened = CodeEditor::new("one", "txt").with_vim_enabled(true);
        focus_editor(&mut opened);
        vim_keys(&mut opened, "o");
        let _ = opened.update(&Message::CharacterInput('X'));
        vim_keys(&mut opened, "\u{1b}");
        assert_eq!(opened.content(), "one\nX");
        assert_eq!(opened.history.undo_count(), 1);
        vim_keys(&mut opened, "u");
        assert_eq!(opened.content(), "one");
    }

    #[test]
    fn test_vim_editing_emits_incremental_lsp_change() {
        let changes = Rc::new(RefCell::new(Vec::new()));
        let client = VimTestLspClient { changes: Rc::clone(&changes) };
        let content = (0..10)
            .map(|line| format!("line{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = CodeEditor::new(&content, "rs").with_vim_enabled(true);
        editor.attach_lsp(
            Box::new(client),
            lsp::LspDocument::new("file:///vim.rs", "rust"),
        );
        editor.cursors.set_single((5, 2));

        vim_keys(&mut editor, "x");

        let changes = changes.borrow();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].len(), 1);
        let change = &changes[0][0];
        assert_eq!(change.range.start.line, 4);
        assert_eq!(change.range.start.character, 0);
        assert_eq!(change.range.end.line, 7);
        assert_eq!(change.range.end.character, 0);
        assert_eq!(change.text, "line4\nlie5\nline6\n");
    }

    #[test]
    fn test_horizontal_scroll_initial_state() {
        let editor = CodeEditor::new("short line", "rs");
        assert!(
            (editor.horizontal_scroll_offset - 0.0).abs() < f32::EPSILON,
            "Initial horizontal scroll offset should be 0"
        );
    }

    #[test]
    fn test_set_wrap_enabled_resets_horizontal_offset() {
        let mut editor = CodeEditor::new("long line", "rs");
        editor.wrap_enabled = false;
        // Simulate a non-zero horizontal scroll
        editor.horizontal_scroll_offset = 100.0;

        // Re-enabling wrap should reset horizontal offset
        editor.set_wrap_enabled(true);
        assert!(
            (editor.horizontal_scroll_offset - 0.0).abs() < f32::EPSILON,
            "Horizontal scroll offset should be reset when wrap is re-enabled"
        );
    }

    #[test]
    fn test_canvas_focus_lost() {
        let mut editor = CodeEditor::new("test", "rs");
        editor.has_canvas_focus = true;

        let _ = editor.update(&Message::CanvasFocusLost);

        assert!(!editor.has_canvas_focus);
        assert!(!editor.show_cursor);
        assert!(editor.focus_locked, "Focus should be locked when lost");
    }

    #[test]
    fn test_canvas_focus_gained_resets_lock() {
        let mut editor = CodeEditor::new("test", "rs");
        editor.has_canvas_focus = false;
        editor.focus_locked = true;

        let _ = editor.update(&Message::CanvasFocusGained);

        assert!(editor.has_canvas_focus);
        assert!(
            !editor.focus_locked,
            "Focus lock should be reset when focus is gained"
        );
    }

    #[test]
    fn test_focus_lock_state() {
        let mut editor = CodeEditor::new("test", "rs");

        // Initially, focus should not be locked
        assert!(!editor.focus_locked);

        // When focus is lost, it should be locked
        let _ = editor.update(&Message::CanvasFocusLost);
        assert!(editor.focus_locked, "Focus should be locked when lost");

        // When focus is regained, it should be unlocked
        editor.request_focus();
        let _ = editor.update(&Message::CanvasFocusGained);
        assert!(!editor.focus_locked, "Focus should be unlocked when regained");

        // Can manually reset focus lock
        editor.focus_locked = true;
        editor.reset_focus_lock();
        assert!(!editor.focus_locked, "Focus lock should be resetable");
    }

    #[test]
    fn test_reset_focus_lock() {
        let mut editor = CodeEditor::new("test", "rs");
        editor.focus_locked = true;

        editor.reset_focus_lock();

        assert!(!editor.focus_locked);
    }

    #[test]
    fn test_home_key() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().position = (0, 5); // Move to middle of line
        let _ = editor.update(&Message::Home(false));
        assert_eq!(editor.cursors.primary_position(), (0, 0));
    }

    #[test]
    fn test_end_key() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().position = (0, 0);
        let _ = editor.update(&Message::End(false));
        assert_eq!(editor.cursors.primary_position(), (0, 11)); // Length of "hello world"
    }

    #[test]
    fn test_arrow_key_with_shift_creates_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().position = (0, 0);

        // Shift+Right should start selection
        let _ = editor.update(&Message::ArrowKey(ArrowDirection::Right, true));
        assert!(editor.cursors.primary().anchor.is_some());
        assert!(editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_arrow_key_without_shift_clears_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        // Regular arrow key should clear selection
        let _ = editor.update(&Message::ArrowKey(ArrowDirection::Right, false));
        assert!(editor.cursors.primary().anchor.is_none());
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_typing_with_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        // Ensure editor has focus for character input
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::CharacterInput('X'));
        assert_eq!(editor.buffer.line(0), "X world");
        assert_eq!(editor.cursors.primary_position(), (0, 1));
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_typing_digit_with_reversed_selection_replaces_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().anchor = Some((0, 5));
        editor.cursors.primary_mut().position = (0, 0);

        let _ = editor.update(&Message::CharacterInput('7'));
        assert_eq!(editor.buffer.line(0), "7 world");
        assert_eq!(editor.cursors.primary_position(), (0, 1));
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_typing_with_selection_undoes_as_single_edit() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::CharacterInput('X'));
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello world");
    }

    #[test]
    fn test_ctrl_home() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().position = (2, 5); // Start at line 3, column 5
        let _ = editor.update(&Message::CtrlHome);
        assert_eq!(editor.cursors.primary_position(), (0, 0)); // Should move to beginning of document
    }

    #[test]
    fn test_ctrl_end() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().position = (0, 0); // Start at beginning
        let _ = editor.update(&Message::CtrlEnd);
        assert_eq!(editor.cursors.primary_position(), (2, 5)); // Should move to end of last line (line3 has 5 chars)
    }

    #[test]
    fn test_ctrl_home_clears_selection() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().position = (2, 5);
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (2, 5);

        let _ = editor.update(&Message::CtrlHome);
        assert_eq!(editor.cursors.primary_position(), (0, 0));
        assert!(editor.cursors.primary().anchor.is_none());
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_ctrl_end_clears_selection() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().position = (0, 0);
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (1, 3);

        let _ = editor.update(&Message::CtrlEnd);
        assert_eq!(editor.cursors.primary_position(), (2, 5));
        assert!(editor.cursors.primary().anchor.is_none());
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_goto_position_sets_cursor_and_clears_selection() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (1, 2);

        let _ = editor.update(&Message::GotoPosition(1, 3));

        assert_eq!(editor.cursors.primary_position(), (1, 3));
        assert!(editor.cursors.primary().anchor.is_none());
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_goto_position_clamps_out_of_range() {
        let mut editor = CodeEditor::new("a\nbb", "py");

        let _ = editor.update(&Message::GotoPosition(99, 99));

        // Clamped to last line (index 1) and end of that line (len = 2)
        assert_eq!(editor.cursors.primary_position(), (1, 2));
    }

    #[test]
    fn test_scroll_sets_initial_cache_window() {
        let content =
            (0..200).map(|i| format!("line{}\n", i)).collect::<String>();
        let mut editor = CodeEditor::new(&content, "py");

        // Simulate initial viewport
        let height = 400.0;
        let width = 800.0;
        let scroll = 0.0;

        // Expected derived ranges
        let visible_lines_count =
            (height / editor.line_height).ceil() as usize + 2;
        let first_visible_line = (scroll / editor.line_height).floor() as usize;
        let last_visible_line = first_visible_line + visible_lines_count;
        let margin = visible_lines_count * 2;
        let window_start = first_visible_line.saturating_sub(margin);
        let window_end = last_visible_line + margin;

        // Apply logic similar to Message::Scrolled branch
        editor.viewport_height = height;
        editor.viewport_width = width;
        editor.viewport_scroll = -1.0;
        let scroll_changed = (editor.viewport_scroll - scroll).abs() > 0.1;
        let need_rewindow = true;
        if (editor.viewport_height - height).abs() > 1.0
            || (editor.viewport_width - width).abs() > 1.0
            || (scroll_changed && need_rewindow)
        {
            editor.cache_window_start_line = window_start;
            editor.cache_window_end_line = window_end;
            editor.last_first_visible_line = first_visible_line;
        }
        editor.viewport_scroll = scroll;

        assert_eq!(editor.last_first_visible_line, first_visible_line);
        assert!(editor.cache_window_end_line > editor.cache_window_start_line);
        assert_eq!(editor.cache_window_start_line, window_start);
        assert_eq!(editor.cache_window_end_line, window_end);
    }

    #[test]
    fn test_small_scroll_keeps_window() {
        let content =
            (0..200).map(|i| format!("line{}\n", i)).collect::<String>();
        let mut editor = CodeEditor::new(&content, "py");
        let height = 400.0;
        let width = 800.0;
        let initial_scroll = 0.0;
        let visible_lines_count =
            (height / editor.line_height).ceil() as usize + 2;
        let first_visible_line =
            (initial_scroll / editor.line_height).floor() as usize;
        let last_visible_line = first_visible_line + visible_lines_count;
        let margin = visible_lines_count * 2;
        let window_start = first_visible_line.saturating_sub(margin);
        let window_end = last_visible_line + margin;
        editor.cache_window_start_line = window_start;
        editor.cache_window_end_line = window_end;
        editor.viewport_height = height;
        editor.viewport_width = width;
        editor.viewport_scroll = initial_scroll;

        // Small scroll inside window
        let small_scroll =
            editor.line_height * (visible_lines_count as f32 / 4.0);
        let first_visible_line2 =
            (small_scroll / editor.line_height).floor() as usize;
        let last_visible_line2 = first_visible_line2 + visible_lines_count;
        let lower_boundary_trigger = editor.cache_window_start_line > 0
            && first_visible_line2
                < editor
                    .cache_window_start_line
                    .saturating_add(visible_lines_count / 2);
        let upper_boundary_trigger = last_visible_line2
            > editor
                .cache_window_end_line
                .saturating_sub(visible_lines_count / 2);
        let need_rewindow = lower_boundary_trigger || upper_boundary_trigger;

        assert!(!need_rewindow, "Small scroll should be inside the window");
        // Window remains unchanged
        assert_eq!(editor.cache_window_start_line, window_start);
        assert_eq!(editor.cache_window_end_line, window_end);
    }

    #[test]
    fn test_large_scroll_rewindows() {
        let content =
            (0..1000).map(|i| format!("line{}\n", i)).collect::<String>();
        let mut editor = CodeEditor::new(&content, "py");
        let height = 400.0;
        let width = 800.0;
        let initial_scroll = 0.0;
        let visible_lines_count =
            (height / editor.line_height).ceil() as usize + 2;
        let first_visible_line =
            (initial_scroll / editor.line_height).floor() as usize;
        let last_visible_line = first_visible_line + visible_lines_count;
        let margin = visible_lines_count * 2;
        editor.cache_window_start_line =
            first_visible_line.saturating_sub(margin);
        editor.cache_window_end_line = last_visible_line + margin;
        editor.viewport_height = height;
        editor.viewport_width = width;
        editor.viewport_scroll = initial_scroll;

        // Large scroll beyond window boundary
        let large_scroll =
            editor.line_height * ((visible_lines_count * 4) as f32);
        let first_visible_line2 =
            (large_scroll / editor.line_height).floor() as usize;
        let last_visible_line2 = first_visible_line2 + visible_lines_count;
        let window_start2 = first_visible_line2.saturating_sub(margin);
        let window_end2 = last_visible_line2 + margin;
        let need_rewindow = first_visible_line2
            < editor
                .cache_window_start_line
                .saturating_add(visible_lines_count / 2)
            || last_visible_line2
                > editor
                    .cache_window_end_line
                    .saturating_sub(visible_lines_count / 2);
        assert!(need_rewindow, "Large scroll should trigger window update");

        // Apply rewindow
        editor.cache_window_start_line = window_start2;
        editor.cache_window_end_line = window_end2;
        editor.last_first_visible_line = first_visible_line2;

        assert_eq!(editor.cache_window_start_line, window_start2);
        assert_eq!(editor.cache_window_end_line, window_end2);
        assert_eq!(editor.last_first_visible_line, first_visible_line2);
    }

    #[test]
    fn test_delete_selection_message() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().position = (0, 0);
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::DeleteSelection);
        assert_eq!(editor.buffer.line(0), " world");
        assert_eq!(editor.cursors.primary_position(), (0, 0));
        assert!(editor.cursors.primary().anchor.is_none());
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_delete_selection_multiline() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().position = (0, 2);
        editor.cursors.primary_mut().anchor = Some((0, 2));
        editor.cursors.primary_mut().position = (2, 2);

        let _ = editor.update(&Message::DeleteSelection);
        assert_eq!(editor.buffer.line(0), "line3");
        assert_eq!(editor.cursors.primary_position(), (0, 2));
        assert!(editor.cursors.primary().anchor.is_none());
    }

    #[test]
    fn test_delete_selection_no_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::DeleteSelection);
        // Should do nothing if there's no selection
        assert_eq!(editor.buffer.line(0), "hello world");
        assert_eq!(editor.cursors.primary_position(), (0, 5));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_ime_preedit_and_commit_chinese() {
        let mut editor = CodeEditor::new("", "py");
        // Simulate IME opened
        let _ = editor.update(&Message::ImeOpened);
        assert!(editor.ime_preedit.is_none());

        // Preedit with Chinese content and a selection range
        let content = "安全与合规".to_string();
        let selection = Some(0..3); // range aligned to UTF-8 character boundary
        let _ = editor
            .update(&Message::ImePreedit(content.clone(), selection.clone()));

        assert!(editor.ime_preedit.is_some());
        assert_eq!(
            editor.ime_preedit.as_ref().unwrap().content.clone(),
            content
        );
        assert_eq!(
            editor.ime_preedit.as_ref().unwrap().selection.clone(),
            selection
        );

        // Commit should insert the text and clear preedit
        let _ = editor.update(&Message::ImeCommit("安全与合规".to_string()));
        assert!(editor.ime_preedit.is_none());
        assert_eq!(editor.buffer.line(0), "安全与合规");
        assert_eq!(
            editor.cursors.primary_position(),
            (0, "安全与合规".chars().count())
        );
    }

    #[test]
    fn test_undo_char_insert() {
        let mut editor = CodeEditor::new("hello", "py");
        // Ensure editor has focus for character input
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().position = (0, 5);

        // Type a character
        let _ = editor.update(&Message::CharacterInput('!'));
        assert_eq!(editor.buffer.line(0), "hello!");
        assert_eq!(editor.cursors.primary_position(), (0, 6));

        // Undo should remove it (but first end the grouping)
        editor.history.end_group();
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello");
        assert_eq!(editor.cursors.primary_position(), (0, 5));
    }

    #[test]
    fn test_undo_redo_char_insert() {
        let mut editor = CodeEditor::new("hello", "py");
        // Ensure editor has focus for character input
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().position = (0, 5);

        // Type a character
        let _ = editor.update(&Message::CharacterInput('!'));
        editor.history.end_group();

        // Undo
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello");

        // Redo
        let _ = editor.update(&Message::Redo);
        assert_eq!(editor.buffer.line(0), "hello!");
        assert_eq!(editor.cursors.primary_position(), (0, 6));
    }

    #[test]
    fn test_undo_backspace() {
        let mut editor = CodeEditor::new("hello", "py");
        editor.cursors.primary_mut().position = (0, 5);

        // Backspace
        let _ = editor.update(&Message::Backspace);
        assert_eq!(editor.buffer.line(0), "hell");
        assert_eq!(editor.cursors.primary_position(), (0, 4));

        // Undo
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello");
        assert_eq!(editor.cursors.primary_position(), (0, 5));
    }

    #[test]
    fn test_undo_newline() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().position = (0, 5);

        // Insert newline
        let _ = editor.update(&Message::Enter);
        assert_eq!(editor.buffer.line(0), "hello");
        assert_eq!(editor.buffer.line(1), " world");
        assert_eq!(editor.cursors.primary_position(), (1, 0));

        // Undo
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello world");
        assert_eq!(editor.cursors.primary_position(), (0, 5));
    }

    #[test]
    fn test_undo_grouped_typing() {
        let mut editor = CodeEditor::new("hello", "py");
        // Ensure editor has focus for character input
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().position = (0, 5);

        // Type multiple characters (they should be grouped)
        let _ = editor.update(&Message::CharacterInput(' '));
        let _ = editor.update(&Message::CharacterInput('w'));
        let _ = editor.update(&Message::CharacterInput('o'));
        let _ = editor.update(&Message::CharacterInput('r'));
        let _ = editor.update(&Message::CharacterInput('l'));
        let _ = editor.update(&Message::CharacterInput('d'));

        assert_eq!(editor.buffer.line(0), "hello world");

        // End the group
        editor.history.end_group();

        // Single undo should remove all grouped characters
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello");
        assert_eq!(editor.cursors.primary_position(), (0, 5));
    }

    #[test]
    fn test_navigation_ends_grouping() {
        let mut editor = CodeEditor::new("hello", "py");
        // Ensure editor has focus for character input
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().position = (0, 5);

        // Type a character (starts grouping)
        let _ = editor.update(&Message::CharacterInput('!'));
        assert!(editor.is_grouping);

        // Move cursor (ends grouping)
        let _ = editor.update(&Message::ArrowKey(ArrowDirection::Left, false));
        assert!(!editor.is_grouping);

        // Type another character (starts new group)
        let _ = editor.update(&Message::CharacterInput('?'));
        assert!(editor.is_grouping);

        editor.history.end_group();

        // Two separate undo operations
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello!");

        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "hello");
    }

    #[test]
    fn test_edit_increments_revision_and_clears_visual_lines_cache() {
        let mut editor = CodeEditor::new("hello", "rs");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.visual_lines_cached(800.0);
        assert!(
            editor.visual_lines_cache.borrow().is_some(),
            "visual_lines_cached should populate the cache"
        );

        let previous_revision = editor.buffer_revision;

        let _ = editor.update(&Message::CharacterInput('!'));
        assert_eq!(
            editor.buffer_revision,
            previous_revision.wrapping_add(1),
            "buffer_revision should change on buffer edits"
        );
        // `scroll_to_cursor` repopulates the cache after the edit with the new
        // revision, so the cache may be `Some`.  What must never happen is that
        // stale data (an old revision) survives an edit.
        assert!(
            editor
                .visual_lines_cache
                .borrow()
                .as_ref()
                .is_none_or(|c| c.key.buffer_revision == editor.buffer_revision),
            "buffer edits should not leave stale data in the visual lines cache"
        );
    }

    #[test]
    fn test_edit_refreshes_only_affected_search_matches() {
        let mut editor = CodeEditor::new("foo\nfoo\nfoo", "rs");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        editor.search_state.open_search();
        editor.search_state.set_query("foo".to_owned(), &editor.buffer);
        editor.cursors.primary_mut().position = (1, 1);

        let _ = editor.update(&Message::CharacterInput('x'));

        let match_lines: Vec<usize> =
            editor.search_state.matches.iter().map(|item| item.line).collect();
        assert_eq!(match_lines, vec![0, 2]);
    }

    #[test]
    fn test_manual_search_match_selection_updates_current_index() {
        let mut editor =
            CodeEditor::new("foo bar foo baz foo\nno result", "txt");
        editor.search_state.open_search();
        editor.search_state.set_query("foo".to_owned(), &editor.buffer);
        assert_eq!(editor.search_state.current_match_index, Some(0));

        let text_start = editor.gutter_width() + 5.0;
        let char_width = editor.char_width;
        let line_y = editor.line_height / 2.0;
        let point_at_col = |col: usize| {
            iced::Point::new(text_start + char_width * col as f32, line_y)
        };

        let _ = editor.update(&Message::MouseClick(point_at_col(8)));
        let _ = editor.update(&Message::MouseDrag(point_at_col(11)));
        let _ = editor.update(&Message::MouseRelease);

        assert_eq!(
            editor.cursors.primary().selection_range(),
            Some(((0, 8), (0, 11)))
        );
        assert_eq!(editor.search_state.current_match_index, Some(1));

        let no_match_line = iced::Point::new(
            text_start + char_width * 4.0,
            editor.line_height * 1.5,
        );
        let _ = editor.update(&Message::MouseClick(no_match_line));
        let _ = editor.update(&Message::MouseRelease);
        assert_eq!(editor.search_state.current_match_index, Some(1));

        let _ = editor.update(&Message::FindNext);
        assert_eq!(editor.search_state.current_match_index, Some(2));
        let _ = editor.update(&Message::FindPrevious);
        assert_eq!(editor.search_state.current_match_index, Some(1));
    }

    #[test]
    fn test_manual_line_selection_updates_current_search_index() {
        let mut editor =
            CodeEditor::new("foo\nprefix foo suffix\nlast foo", "txt");
        editor.search_state.open_search();
        editor.search_state.set_query("foo".to_owned(), &editor.buffer);
        assert_eq!(editor.search_state.current_match_index, Some(0));

        let line_start = iced::Point::new(
            editor.gutter_width() + 5.0,
            editor.line_height * 1.5,
        );
        let _ = editor.update(&Message::MouseClick(line_start));
        let _ = editor.update(&Message::MouseRelease);

        assert_eq!(editor.cursors.primary_position(), (1, 0));
        assert_eq!(editor.search_state.current_match_index, Some(1));

        let _ = editor.update(&Message::FindNext);
        assert_eq!(editor.search_state.current_match_index, Some(2));

        let mut keyboard_editor =
            CodeEditor::new("foo\nprefix foo suffix\nlast foo", "txt");
        keyboard_editor.search_state.open_search();
        keyboard_editor
            .search_state
            .set_query("foo".to_owned(), &keyboard_editor.buffer);

        let _ = keyboard_editor
            .update(&Message::ArrowKey(ArrowDirection::Down, false));

        assert_eq!(keyboard_editor.cursors.primary_position(), (1, 0));
        assert_eq!(keyboard_editor.search_state.current_match_index, Some(1));
    }

    #[test]
    fn test_incremental_visual_lines_match_full_recalculation_after_newline() {
        use std::collections::HashSet;

        let mut editor = CodeEditor::new("zero\nabcdefgh\nlast", "rs")
            .with_wrap_column(Some(4));
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        editor.cursors.primary_mut().position = (1, 4);

        let _ = editor.visual_lines_cached(800.0);
        let _ = editor.update(&Message::Enter);
        let incremental = editor.visual_lines_cached(800.0);

        let calculator = super::super::wrapping::WrappingCalculator::new(
            editor.wrap_enabled,
            editor.wrap_column,
            editor.full_char_width,
            editor.char_width,
        );
        let expected = calculator.calculate_visual_lines(
            &editor.buffer,
            800.0,
            editor.gutter_width(),
            &HashSet::new(),
        );

        assert_eq!(incremental.as_ref(), &expected);
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut editor = CodeEditor::new("a", "py");
        // Ensure editor has focus for character input
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        editor.cursors.primary_mut().position = (0, 1);

        // Make several changes
        let _ = editor.update(&Message::CharacterInput('b'));
        editor.history.end_group();

        let _ = editor.update(&Message::CharacterInput('c'));
        editor.history.end_group();

        let _ = editor.update(&Message::CharacterInput('d'));
        editor.history.end_group();

        assert_eq!(editor.buffer.line(0), "abcd");

        // Undo all
        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "abc");

        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "ab");

        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "a");

        // Redo all
        let _ = editor.update(&Message::Redo);
        assert_eq!(editor.buffer.line(0), "ab");

        let _ = editor.update(&Message::Redo);
        assert_eq!(editor.buffer.line(0), "abc");

        let _ = editor.update(&Message::Redo);
        assert_eq!(editor.buffer.line(0), "abcd");
    }

    #[test]
    fn test_delete_key_with_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::Delete);

        assert_eq!(editor.buffer.line(0), " world");
        assert_eq!(editor.cursors.primary_position(), (0, 0));
        assert!(editor.cursors.primary().anchor.is_none());
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_delete_key_without_selection() {
        let mut editor = CodeEditor::new("hello", "py");
        editor.cursors.primary_mut().position = (0, 0);

        let _ = editor.update(&Message::Delete);

        // Should delete the 'h'
        assert_eq!(editor.buffer.line(0), "ello");
        assert_eq!(editor.cursors.primary_position(), (0, 0));
    }

    #[test]
    fn test_backspace_with_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 6));
        editor.cursors.primary_mut().position = (0, 11);
        editor.cursors.primary_mut().position = (0, 11);

        let _ = editor.update(&Message::Backspace);

        assert_eq!(editor.buffer.line(0), "hello ");
        assert_eq!(editor.cursors.primary_position(), (0, 6));
        assert!(editor.cursors.primary().anchor.is_none());
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_backspace_without_selection() {
        let mut editor = CodeEditor::new("hello", "py");
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::Backspace);

        // Should delete the 'o'
        assert_eq!(editor.buffer.line(0), "hell");
        assert_eq!(editor.cursors.primary_position(), (0, 4));
    }

    #[test]
    fn test_delete_multiline_selection() {
        let mut editor = CodeEditor::new("line1\nline2\nline3", "py");
        editor.cursors.primary_mut().anchor = Some((0, 2));
        editor.cursors.primary_mut().position = (2, 2);
        editor.cursors.primary_mut().position = (2, 2);

        let _ = editor.update(&Message::Delete);

        assert_eq!(editor.buffer.line(0), "line3");
        assert_eq!(editor.cursors.primary_position(), (0, 2));
        assert!(editor.cursors.primary().anchor.is_none());
    }

    #[test]
    fn test_canvas_focus_gained() {
        let mut editor = CodeEditor::new("hello world", "py");
        assert!(!editor.has_canvas_focus);
        assert!(!editor.show_cursor);

        let _ = editor.update(&Message::CanvasFocusGained);

        assert!(editor.has_canvas_focus);
        assert!(editor.show_cursor);
    }

    #[test]
    fn test_mouse_click_gains_focus() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.has_canvas_focus = false;
        editor.show_cursor = false;

        let _ =
            editor.update(&Message::MouseClick(iced::Point::new(100.0, 10.0)));

        assert!(editor.has_canvas_focus);
        assert!(editor.show_cursor);
    }

    #[test]
    fn test_context_click_inside_selection_preserves_selection() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);
        let point = iced::Point::new(
            editor.gutter_width() + 5.0 + editor.char_width * 2.0,
            editor.line_height / 2.0,
        );

        let _ = editor.update(&Message::ContextMenuRequested(point));

        assert_eq!(
            editor.cursors.primary().selection_range(),
            Some(((0, 0), (0, 5)))
        );
    }

    #[test]
    fn test_context_click_outside_selection_moves_caret() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);
        let point = iced::Point::new(
            editor.gutter_width() + 5.0 + editor.char_width * 8.0,
            editor.line_height / 2.0,
        );

        let _ = editor.update(&Message::ContextMenuRequested(point));

        assert_eq!(editor.cursors.primary_position(), (0, 8));
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_context_menu_cut_and_select_all() {
        let mut editor = CodeEditor::new("hello world", "py");
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::Cut);
        assert_eq!(editor.content(), " world");

        let _ = editor.update(&Message::SelectAll);
        assert_eq!(editor.get_selected_text(), Some(" world".to_string()));

        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.content(), "hello world");
    }

    #[test]
    fn test_enter_no_indent() {
        let mut editor = CodeEditor::new("hello", "rs");
        editor.cursors.primary_mut().position = (0, 5);
        let _ = editor.update(&Message::Enter);
        assert_eq!(editor.buffer.line(0), "hello");
        assert_eq!(editor.buffer.line(1), "");
        assert_eq!(editor.cursors.primary_position(), (1, 0));
    }

    #[test]
    fn test_typing_after_enter_does_not_delete_newline_from_click_anchor() {
        let mut editor = CodeEditor::new("hello", "rs");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;

        // A regular click starts drag tracking with a zero-length anchor.
        editor.cursors.primary_mut().position = (0, 5);
        editor.cursors.primary_mut().anchor = Some((0, 5));

        let _ = editor.update(&Message::Enter);
        let _ = editor.update(&Message::CharacterInput('X'));

        assert_eq!(editor.buffer.line_count(), 2);
        assert_eq!(editor.buffer.line(0), "hello");
        assert_eq!(editor.buffer.line(1), "X");
        assert_eq!(editor.cursors.primary_position(), (1, 1));
        assert!(!editor.cursors.primary().has_selection());
    }

    #[test]
    fn test_enter_replaces_selection_and_undo_restores_text() {
        let mut editor = CodeEditor::new("hello world", "rs");
        editor.set_auto_indent_enabled(false);
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 5);

        let _ = editor.update(&Message::Enter);

        assert_eq!(editor.buffer.line(0), "");
        assert_eq!(editor.buffer.line(1), " world");
        assert_eq!(editor.cursors.primary_position(), (1, 0));
        assert!(!editor.cursors.primary().has_selection());

        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.content(), "hello world");
    }

    #[test]
    fn test_enter_auto_indent_spaces() {
        let mut editor = CodeEditor::new("    hello", "rs");
        editor.cursors.primary_mut().position = (0, 9);
        let _ = editor.update(&Message::Enter);
        assert_eq!(editor.buffer.line(0), "    hello");
        assert_eq!(editor.buffer.line(1), "    ");
        assert_eq!(editor.cursors.primary_position(), (1, 4));
    }

    #[test]
    fn test_enter_auto_indent_tab() {
        let mut editor = CodeEditor::new("\thello", "rs");
        editor.cursors.primary_mut().position = (0, 6);
        let _ = editor.update(&Message::Enter);
        assert_eq!(editor.buffer.line(0), "\thello");
        assert_eq!(editor.buffer.line(1), "\t");
        assert_eq!(editor.cursors.primary_position(), (1, 1));
    }

    #[test]
    fn test_enter_auto_indent_undo() {
        let mut editor = CodeEditor::new("    hello", "rs");
        editor.cursors.primary_mut().position = (0, 9);
        let _ = editor.update(&Message::Enter);
        assert_eq!(editor.buffer.line_count(), 2);

        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line_count(), 1);
        assert_eq!(editor.buffer.line(0), "    hello");
        assert_eq!(editor.cursors.primary_position(), (0, 9));
    }

    // =========================================================================
    // Multi-cursor tests
    // =========================================================================

    #[test]
    fn test_multi_cursor_char_input_different_lines() {
        let mut editor = CodeEditor::new("aaa\nbbb", "rs");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        // Place cursors at (0, 1) and (1, 1)
        editor.cursors.primary_mut().position = (0, 1);
        editor.cursors.add_cursor((1, 1));

        let _ = editor.update(&Message::CharacterInput('X'));

        // Both lines should have 'X' inserted at col 1
        assert_eq!(editor.buffer.line(0), "aXaa");
        assert_eq!(editor.buffer.line(1), "bXbb");
    }

    #[test]
    fn test_multi_cursor_char_input_same_line() {
        let mut editor = CodeEditor::new("abcd", "rs");
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        // Place cursors at col 1 and col 3 (same line)
        editor.cursors.primary_mut().position = (0, 1);
        editor.cursors.add_cursor((0, 3));

        let _ = editor.update(&Message::CharacterInput('X'));

        // Process descending: col 3 first → "abcXd"; then col 1 → "aXbcXd"
        // Col 1 cursor adjustment: insert at col 3 does not affect col 1 (col 1 < 3)
        assert_eq!(editor.buffer.line(0), "aXbcXd");
    }

    #[test]
    fn test_add_cursor_above() {
        let mut editor = CodeEditor::new("line0\nline1\nline2", "rs");
        editor.cursors.primary_mut().position = (1, 3);

        let _ = editor.update(&Message::AddCursorAbove);

        assert!(editor.cursors.is_multi());
        // New cursor should be at line 0, col 3
        assert_eq!(editor.cursors.as_slice()[0].position, (0, 3));
    }

    #[test]
    fn test_add_cursor_below() {
        let mut editor = CodeEditor::new("line0\nline1\nline2", "rs");
        editor.cursors.primary_mut().position = (1, 3);

        let _ = editor.update(&Message::AddCursorBelow);

        assert!(editor.cursors.is_multi());
        // New cursor should be at line 2, col 3
        assert_eq!(
            editor
                .cursors
                .as_slice()
                .iter()
                .find(|c| c.position.0 == 2)
                .map(|c| c.position),
            Some((2, 3))
        );
    }

    #[test]
    fn test_escape_collapses_multi_cursor() {
        let mut editor = CodeEditor::new("line0\nline1", "rs");
        editor.cursors.primary_mut().position = (0, 0);
        editor.cursors.add_cursor((1, 0));
        assert!(editor.cursors.is_multi());

        let _ = editor.update(&Message::CloseSearch);

        assert!(!editor.cursors.is_multi());
    }

    #[test]
    fn test_select_next_occurrence_selects_word() {
        let mut editor = CodeEditor::new("foo bar foo", "rs");
        editor.cursors.primary_mut().position = (0, 1); // inside "foo"

        let _ = editor.update(&Message::SelectNextOccurrence);

        // Primary cursor should now have "foo" selected
        let range = editor.cursors.primary().selection_range();
        assert_eq!(range, Some(((0, 0), (0, 3))));
    }

    #[test]
    fn test_select_next_occurrence_adds_cursor_for_second_occurrence() {
        let mut editor = CodeEditor::new("foo bar foo", "rs");
        // Set up primary cursor with "foo" selected
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (0, 3);

        let _ = editor.update(&Message::SelectNextOccurrence);

        // Should now have 2 cursors: primary at "foo" (0..3) and new at "foo" (8..11)
        assert_eq!(editor.cursors.len(), 2);
    }

    #[test]
    fn test_multi_cursor_backspace() {
        let mut editor = CodeEditor::new("abc\ndef", "rs");
        editor.cursors.primary_mut().position = (0, 2);
        editor.cursors.add_cursor((1, 2));

        let _ = editor.update(&Message::Backspace);

        assert_eq!(editor.buffer.line(0), "ac");
        assert_eq!(editor.buffer.line(1), "df");
    }

    #[test]
    fn test_toggle_comment_selection() {
        let mut editor = CodeEditor::new("a\nb\nc", "rs");
        // Select lines 0..=2.
        editor.cursors.primary_mut().anchor = Some((0, 0));
        editor.cursors.primary_mut().position = (2, 1);

        let _ = editor.update(&Message::ToggleComment);
        assert_eq!(editor.buffer.to_string(), "// a\n// b\n// c");
        assert_eq!(editor.cursors.primary_position(), (2, 4));

        // Toggling again uncomments the whole range.
        let _ = editor.update(&Message::ToggleComment);
        assert_eq!(editor.buffer.to_string(), "a\nb\nc");
    }

    #[test]
    fn test_toggle_comment_noop_without_token() {
        let mut editor = CodeEditor::new("<div>", "html");
        let _ = editor.update(&Message::ToggleComment);
        // HTML has no line-comment token, so the buffer is unchanged.
        assert_eq!(editor.buffer.line(0), "<div>");
    }

    #[test]
    fn test_toggle_comment_undo() {
        let mut editor = CodeEditor::new("    let x = 1;", "rs");
        editor.cursors.primary_mut().position = (0, 8);

        let _ = editor.update(&Message::ToggleComment);
        assert_eq!(editor.buffer.line(0), "    // let x = 1;");

        let _ = editor.update(&Message::Undo);
        assert_eq!(editor.buffer.line(0), "    let x = 1;");
        assert_eq!(editor.cursors.primary_position(), (0, 8));
    }

    #[test]
    fn test_open_goto_line_prefills_current_one_based_line() {
        let mut editor = CodeEditor::new("one\ntwo\nthree", "rs");
        editor.cursors.primary_mut().position = (1, 2);
        editor.search_state.open_search();

        let _ = editor.update(&Message::OpenGotoLine);

        assert!(editor.goto_line_state.is_open);
        assert_eq!(editor.goto_line_state.query, "2");
        assert!(!editor.search_state.is_open);
    }

    #[test]
    fn test_submit_goto_line_moves_to_one_based_line_and_closes_dialog() {
        let mut editor = CodeEditor::new("one\ntwo\nthree", "rs");
        let _ = editor.update(&Message::OpenGotoLine);
        let _ = editor.update(&Message::GotoLineChanged("3".to_string()));

        let _ = editor.update(&Message::SubmitGotoLine);

        assert_eq!(editor.cursors.primary_position(), (2, 0));
        assert!(!editor.goto_line_state.is_open);
    }

    #[test]
    fn test_submit_goto_line_clamps_to_last_line() {
        let mut editor = CodeEditor::new("one\ntwo\nthree", "rs");
        let _ = editor.update(&Message::OpenGotoLine);
        let _ = editor.update(&Message::GotoLineChanged("99".to_string()));

        let _ = editor.update(&Message::SubmitGotoLine);

        assert_eq!(editor.cursors.primary_position(), (2, 0));
        assert!(!editor.goto_line_state.is_open);
    }

    #[test]
    fn test_submit_goto_line_reveals_folded_target() {
        let mut editor =
            CodeEditor::new("root\n    child\n        nested\ntail", "rs");
        editor.fold_all();
        assert!(editor.hidden_lines_set().contains(&1));
        let _ = editor.update(&Message::OpenGotoLine);
        let _ = editor.update(&Message::GotoLineChanged("2".to_string()));

        let _ = editor.update(&Message::SubmitGotoLine);

        assert_eq!(editor.cursors.primary_position(), (1, 0));
        assert!(!editor.hidden_lines_set().contains(&1));
    }

    #[test]
    fn test_submit_goto_line_keeps_dialog_open_for_invalid_input() {
        let mut editor = CodeEditor::new("one\ntwo\nthree", "rs");
        editor.cursors.primary_mut().position = (1, 1);
        let _ = editor.update(&Message::OpenGotoLine);
        let _ = editor.update(&Message::GotoLineChanged("invalid".to_string()));

        let _ = editor.update(&Message::SubmitGotoLine);

        assert_eq!(editor.cursors.primary_position(), (1, 1));
        assert!(editor.goto_line_state.is_open);
    }
}
