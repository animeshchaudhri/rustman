//! Message handling and update logic.

use iced::Task;
use iced::widget::operation::{focus, select_all};

use super::command::{
    Command, CompositeCommand, DeleteCharCommand, DeleteForwardCommand,
    InsertCharCommand, InsertNewlineCommand, ReplaceTextCommand,
};
use super::{
    ArrowDirection, CURSOR_BLINK_INTERVAL, CodeEditor, ImePreedit, IndentStyle,
    Message, cursor_set,
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
        self.buffer_revision = self.buffer_revision.wrapping_add(1);
        *self.visual_lines_cache.borrow_mut() = None;
        self.content_cache.clear();
        self.overlay_cache.clear();
        self.enqueue_lsp_change();
    }

    /// Performs common cleanup operations after navigation operations.
    ///
    /// This method should be called after cursor movement operations.
    /// It resets the cursor blink animation and invalidates only the overlay
    /// rendering cache. Cursor movement and selection changes do not modify the
    /// buffer content, so keeping the content cache intact avoids unnecessary
    /// re-rendering of syntax-highlighted text.
    fn finish_navigation_operation(&mut self) {
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

        // Multi-cursor: build a sorted index list (descending document order)
        // so that edits at higher positions don't invalidate lower positions.
        let mut order: Vec<usize> = (0..self.cursors.len()).collect();
        order.sort_by(|&a, &b| {
            self.cursors.as_slice()[b]
                .position
                .cmp(&self.cursors.as_slice()[a].position)
        });

        for &idx in &order {
            let cursor = &self.cursors.as_slice()[idx];
            // Insert at the selection start when the cursor has an active selection,
            // otherwise insert at the cursor position.
            let pos = match cursor.anchor {
                Some(anchor) if anchor < cursor.position => anchor,
                _ => cursor.position,
            };
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
        // End grouping on enter
        self.end_grouping_if_active();

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
        self.end_grouping_if_active();

        // If any cursor has a selection, delete all selections first
        if self.delete_selection_if_present() {
            return self.scroll_to_cursor();
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
        self.end_grouping_if_active();

        // If any cursor has a selection, delete all selections first
        if self.delete_selection_if_present() {
            return self.scroll_to_cursor();
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
        self.focus_locked = false;

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
        Task::none()
    }

    fn handle_double_click_msg(&mut self, point: iced::Point) -> Task<Message> {
        self.request_focus();
        self.has_canvas_focus = true;
        self.focus_locked = false;
        self.end_grouping_if_active();
        self.cursors.remove_all_but_primary();
        if let Some((line, col)) = self.calculate_cursor_from_point(point) {
            let line_str = self.buffer.line(line);
            let start = Self::word_start_in_line(line_str, col);
            let end = Self::word_end_in_line(line_str, col);
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
        Task::none()
    }

    fn handle_triple_click_msg(&mut self, point: iced::Point) -> Task<Message> {
        self.request_focus();
        self.has_canvas_focus = true;
        self.focus_locked = false;
        self.end_grouping_if_active();
        self.cursors.remove_all_but_primary();
        if let Some((line, _col)) = self.calculate_cursor_from_point(point) {
            let line_len = self.buffer.line(line).chars().count();
            let cursor = self.cursors.primary_mut();
            cursor.anchor = Some((line, 0));
            cursor.position = (line, line_len);
        }
        self.is_dragging = false;
        self.show_cursor = true;
        self.reset_cursor_blink();
        self.overlay_cache.clear();
        Task::none()
    }

    // =========================================================================
    // Clipboard Handlers
    // =========================================================================

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
            // We have the text, paste it (normalising Windows/Mac line endings,
            // which would otherwise leave stray \r glyphs inside the buffer)
            let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
            self.paste_text(&normalized);
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
        self.search_state.open_search();
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
        self.search_state.open_replace();
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

            // Update matches after replacement
            self.search_state.update_matches(&self.buffer);

            // Move to next match if available
            if !self.search_state.matches.is_empty()
                && let Some(next_match) = self.search_state.current_match()
            {
                self.cursors.primary_mut().position =
                    (next_match.line, next_match.col);
            }

            self.clear_selection();
            self.finish_edit_operation();
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

            // Update matches (should be empty now)
            self.search_state.update_matches(&self.buffer);
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

        if text.is_empty() {
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

        for line_offset in 0..=line_count {
            let line_idx = (start_line + line_offset) % line_count;
            let line_str = self.buffer.line(line_idx).to_string();
            let chars: Vec<char> = line_str.chars().collect();

            // On the first iteration, start after start_col; on wrap-around, start from 0
            let search_col = if line_offset == 0 { start_col } else { 0 };

            // Build substring from search_col onward (char-indexed)
            let prefix_bytes: usize =
                chars.iter().take(search_col).map(|c| c.len_utf8()).sum();
            let haystack = &line_str[prefix_bytes..];

            // The search_text is also char-based; find it as a substring
            if let Some(byte_offset) = haystack.find(search_text.as_str()) {
                // Convert byte_offset back to char offset
                let char_start =
                    search_col + haystack[..byte_offset].chars().count();
                let char_end = char_start + search_text.chars().count();

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
        // Self-heal font metrics that were measured before the configured font
        // became available (see the matching check in draw), so cursor and
        // hit-testing math stays in sync with what is rendered.
        let fresh = self.measure_single_char_width("a");
        if fresh.is_finite() && fresh > 0.0 && (fresh - self.char_width).abs() > 0.01 {
            self.recalculate_char_dimensions(false);
        }

        match message {
            // Text input operations
            Message::CharacterInput(ch) => self.handle_character_input_msg(*ch),
            Message::Tab => self.handle_tab(),
            Message::Enter => self.handle_enter(),

            // Deletion operations
            Message::Backspace => self.handle_backspace(),
            Message::Delete => self.handle_delete(),
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
            Message::PageUp => self.handle_page_up(),
            Message::PageDown => self.handle_page_down(),

            // Mouse and selection operations
            Message::MouseClick(point) => self.handle_mouse_click_msg(*point),
            Message::MouseDrag(point) => self.handle_mouse_drag_msg(*point),
            Message::MouseHover(point) => self.handle_mouse_drag_msg(*point),
            Message::MouseRelease => self.handle_mouse_release_msg(),
            Message::DoubleClick(point) => {
                self.handle_double_click_msg(*point)
            }
            Message::TripleClick(point) => {
                self.handle_triple_click_msg(*point)
            }

            // Clipboard operations
            Message::Cut => self.cut_selection(),
            Message::Copy => self.copy_selection(),
            Message::SelectAll => {
                self.select_all_text();
                Task::none()
            }
            Message::Paste(text) => self.handle_paste_msg(text),

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
            Message::ImeOpened => self.handle_ime_opened_msg(),
            Message::ImePreedit(content, selection) => {
                self.handle_ime_preedit_msg(content, selection)
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_editor::ArrowDirection;

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
        // Current behavior: character is inserted at cursor, selection is NOT automatically deleted
        // This is expected behavior - user must delete selection first (Backspace/Delete) or use Paste
        assert_eq!(editor.buffer.line(0), "Xhello world");
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
    fn test_enter_no_indent() {
        let mut editor = CodeEditor::new("hello", "rs");
        editor.cursors.primary_mut().position = (0, 5);
        let _ = editor.update(&Message::Enter);
        assert_eq!(editor.buffer.line(0), "hello");
        assert_eq!(editor.buffer.line(1), "");
        assert_eq!(editor.cursors.primary_position(), (1, 0));
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
}
