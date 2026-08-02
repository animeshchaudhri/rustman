//! Text search functionality for the code editor.
//!
//! This module provides efficient text search capabilities including:
//! - Case-sensitive and case-insensitive search
//! - Multiple match detection
//! - Position tracking for highlighting

use crate::text_buffer::TextBuffer;
use iced::widget::Id;
use std::borrow::Cow;
use std::thread;

/// Represents a search match position in the buffer.
///
/// Contains the line and column position of a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchMatch {
    /// Line index (0-based)
    pub line: usize,
    /// Column index (0-based, UTF-8 character offset)
    pub col: usize,
}

/// Which field in the search dialog currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFocusedField {
    /// Search input field has focus
    Search,
    /// Replace input field has focus
    Replace,
}

/// Search state management.
///
/// Tracks the current search query, options, and results.
#[derive(Debug, Clone)]
pub struct SearchState {
    /// Current search query
    pub query: String,
    /// Text to replace with
    pub replace_with: String,
    /// Case-sensitive search flag
    pub case_sensitive: bool,
    /// Whether the search dialog is visible
    pub is_open: bool,
    /// Whether replace mode is active (true) or just search (false)
    pub is_replace_mode: bool,
    /// List of all matches found
    pub matches: Vec<SearchMatch>,
    /// Index of the currently selected match
    pub current_match_index: Option<usize>,
    /// ID for the search text input (for focus management)
    pub search_input_id: Id,
    /// ID for the replace text input (for focus management)
    pub replace_input_id: Id,
    /// Which field currently has focus (for Tab navigation)
    pub focused_field: SearchFocusedField,
    /// Buffer line count represented by `matches`.
    buffer_line_count: usize,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            replace_with: String::new(),
            case_sensitive: false,
            is_open: false,
            is_replace_mode: false,
            matches: Vec::new(),
            current_match_index: None,
            search_input_id: Id::unique(),
            replace_input_id: Id::unique(),
            focused_field: SearchFocusedField::Search,
            buffer_line_count: 0,
        }
    }
}

impl SearchState {
    /// Creates a new search state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the search dialog in search-only mode.
    pub fn open_search(&mut self) {
        self.is_open = true;
        self.is_replace_mode = false;
        self.focused_field = SearchFocusedField::Search;
    }

    /// Opens the search dialog in search-and-replace mode.
    pub fn open_replace(&mut self) {
        self.is_open = true;
        self.is_replace_mode = true;
        self.focused_field = SearchFocusedField::Search;
    }

    /// Closes the search dialog.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Cycles focus to the next field (Tab).
    pub fn focus_next_field(&mut self) {
        if self.is_replace_mode {
            self.focused_field = match self.focused_field {
                SearchFocusedField::Search => SearchFocusedField::Replace,
                SearchFocusedField::Replace => SearchFocusedField::Search,
            };
        }
        // In search-only mode, do nothing
    }

    /// Cycles focus to the previous field (Shift+Tab).
    pub fn focus_previous_field(&mut self) {
        if self.is_replace_mode {
            self.focused_field = match self.focused_field {
                SearchFocusedField::Search => SearchFocusedField::Replace,
                SearchFocusedField::Replace => SearchFocusedField::Search,
            };
        }
        // In search-only mode, do nothing
    }

    /// Updates the search query and triggers a new search.
    pub fn set_query(&mut self, query: String, buffer: &TextBuffer) {
        self.query = query;
        self.update_matches(buffer);
    }

    /// Updates the replace text.
    pub fn set_replace_with(&mut self, replace_with: String) {
        self.replace_with = replace_with;
    }

    /// Toggles case sensitivity and re-runs the search.
    pub fn toggle_case_sensitive(&mut self, buffer: &TextBuffer) {
        self.case_sensitive = !self.case_sensitive;
        self.update_matches(buffer);
    }

    /// Updates the matches list based on current query and options.
    pub fn update_matches(&mut self, buffer: &TextBuffer) {
        self.matches = find_matches(
            buffer,
            &self.query,
            self.case_sensitive,
            Some(MAX_MATCHES),
        );
        self.buffer_line_count = buffer.line_count();

        // Update current match index
        if self.matches.is_empty() {
            self.current_match_index = None;
        } else if self.current_match_index.is_none() {
            self.current_match_index = Some(0);
        } else if let Some(idx) = self.current_match_index {
            // Clamp to valid range
            if idx >= self.matches.len() {
                self.current_match_index =
                    Some(self.matches.len().saturating_sub(1));
            }
        }
    }

    /// Refreshes only the logical-line range affected by an editor operation.
    ///
    /// Search queries are matched independently within each line, so unchanged
    /// lines retain their results. Matches after inserted or removed lines only
    /// need their logical line number shifted.
    pub(crate) fn update_matches_after_edit(
        &mut self,
        buffer: &TextBuffer,
        start_line: usize,
        old_end_exclusive: usize,
    ) {
        if self.query.is_empty() || self.buffer_line_count == 0 {
            self.update_matches(buffer);
            return;
        }

        let old_line_count = self.buffer_line_count;
        let new_line_count = buffer.line_count();
        let start_line = start_line.min(old_line_count).min(new_line_count);
        let old_end_exclusive =
            old_end_exclusive.min(old_line_count).max(start_line);
        let new_end_exclusive = if new_line_count >= old_line_count {
            old_end_exclusive
                .saturating_add(new_line_count - old_line_count)
                .min(new_line_count)
        } else {
            old_end_exclusive
                .saturating_sub(old_line_count - new_line_count)
                .max(start_line)
                .min(new_line_count)
        };

        let replace_start =
            self.matches.partition_point(|item| item.line < start_line);
        let replace_end =
            self.matches.partition_point(|item| item.line < old_end_exclusive);
        let replacement = find_matches_in_range(
            buffer,
            &self.query,
            self.case_sensitive,
            start_line,
            new_end_exclusive,
            Some(MAX_MATCHES),
        );
        let replacement_len = replacement.len();

        self.matches.splice(replace_start..replace_end, replacement);
        let shifted_suffix_start = replace_start + replacement_len;
        for item in &mut self.matches[shifted_suffix_start..] {
            item.line = if new_line_count >= old_line_count {
                item.line.saturating_add(new_line_count - old_line_count)
            } else {
                item.line.saturating_sub(old_line_count - new_line_count)
            };
        }
        self.matches.truncate(MAX_MATCHES);
        self.buffer_line_count = new_line_count;

        if self.matches.is_empty() {
            self.current_match_index = None;
        }
    }

    /// Moves to the next match (circular).
    pub fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        self.current_match_index = Some(match self.current_match_index {
            Some(idx) => {
                if idx + 1 >= self.matches.len() {
                    0 // Wrap to first
                } else {
                    idx + 1
                }
            }
            None => 0,
        });
    }

    /// Moves to the previous match (circular).
    pub fn previous_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        self.current_match_index = Some(match self.current_match_index {
            Some(idx) => {
                if idx == 0 {
                    self.matches.len() - 1 // Wrap to last
                } else {
                    idx - 1
                }
            }
            None => self.matches.len() - 1,
        });
    }

    /// Returns the current match position if available.
    #[must_use]
    pub fn current_match(&self) -> Option<SearchMatch> {
        self.current_match_index.and_then(|idx| self.matches.get(idx).copied())
    }

    /// Returns the number of matches found.
    #[must_use]
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Selects the search match identified by a manually positioned cursor or
    /// selection.
    ///
    /// An exact single-line selection takes precedence over the active cursor
    /// endpoint. A cursor on either boundary of a match counts as being on that
    /// match so a selection ending immediately after the query is recognised.
    /// Otherwise, the closest match on the cursor's logical line is selected.
    /// If that line has no matches, the current match remains unchanged.
    ///
    /// Returns `true` when the current match index changed.
    pub fn select_match_at_cursor(
        &mut self,
        cursor: (usize, usize),
        selection: Option<((usize, usize), (usize, usize))>,
    ) -> bool {
        if self.matches.is_empty() || self.query.is_empty() {
            return false;
        }

        let query_len = self.query.chars().count();
        let exact_selection_index = selection.and_then(|(start, end)| {
            if start.0 != end.0 {
                return None;
            }

            self.matches_on_line(start.0).find(|&index| {
                let match_item = self.matches[index];
                match_item.col == start.1
                    && match_item.col.saturating_add(query_len) == end.1
            })
        });
        let cursor_index = exact_selection_index.or_else(|| {
            let line_matches = self.matches_on_line(cursor.0);
            line_matches
                .clone()
                .find(|&index| {
                    let match_item = self.matches[index];
                    (match_item.col..=match_item.col.saturating_add(query_len))
                        .contains(&cursor.1)
                })
                .or_else(|| {
                    line_matches.min_by_key(|&index| {
                        let match_item = self.matches[index];
                        let match_end =
                            match_item.col.saturating_add(query_len);
                        if cursor.1 < match_item.col {
                            match_item.col - cursor.1
                        } else {
                            cursor.1.saturating_sub(match_end)
                        }
                    })
                })
        });

        let Some(index) = cursor_index else {
            return false;
        };
        if self.current_match_index == Some(index) {
            return false;
        }

        self.current_match_index = Some(index);
        true
    }

    /// Returns the indices of matches on one logical line.
    fn matches_on_line(&self, line: usize) -> std::ops::Range<usize> {
        let start = self.matches.partition_point(|item| item.line < line);
        let end = self.matches.partition_point(|item| item.line <= line);
        start..end
    }

    /// Selects the match closest to the given cursor position.
    ///
    /// This is useful after buffer modifications to maintain context.
    /// If no matches exist, sets current_match_index to None.
    pub fn select_match_near_cursor(&mut self, cursor: (usize, usize)) {
        if self.matches.is_empty() {
            self.current_match_index = None;
            return;
        }

        let (cursor_line, cursor_col) = cursor;
        let insertion = self.matches.partition_point(|item| {
            (item.line, item.col) < (cursor_line, cursor_col)
        });
        let mut left = insertion.checked_sub(1);
        let mut right = (insertion < self.matches.len()).then_some(insertion);
        let mut closest_index = insertion.min(self.matches.len() - 1);
        let mut closest_distance = usize::MAX;

        while left.is_some() || right.is_some() {
            let left_line_distance = left.map_or(usize::MAX, |index| {
                self.matches[index]
                    .line
                    .abs_diff(cursor_line)
                    .saturating_mul(1000)
            });
            let right_line_distance = right.map_or(usize::MAX, |index| {
                self.matches[index]
                    .line
                    .abs_diff(cursor_line)
                    .saturating_mul(1000)
            });

            if left_line_distance.min(right_line_distance) > closest_distance {
                break;
            }

            let index = match (left, right) {
                (Some(index), Some(_))
                    if left_line_distance <= right_line_distance =>
                {
                    left = index.checked_sub(1);
                    index
                }
                (_, Some(index)) => {
                    right =
                        (index + 1 < self.matches.len()).then_some(index + 1);
                    index
                }
                (Some(index), None) => {
                    left = index.checked_sub(1);
                    index
                }
                (None, None) => break,
            };
            let item = self.matches[index];
            let distance = item
                .line
                .abs_diff(cursor_line)
                .saturating_mul(1000)
                .saturating_add(item.col.abs_diff(cursor_col));
            if distance < closest_distance {
                closest_distance = distance;
                closest_index = index;
                if distance == 0 {
                    break;
                }
            }
        }

        self.current_match_index = Some(closest_index);
    }
}

/// Finds all matches of a query in the text buffer.
///
/// # Arguments
///
/// * `buffer` - The text buffer to search in
/// * `query` - The search string
/// * `case_sensitive` - Whether to perform case-sensitive search
/// * `limit` - Optional maximum number of matches to return
///
/// # Returns
///
/// A vector of all match positions found
#[must_use]
pub fn find_matches(
    buffer: &TextBuffer,
    query: &str,
    case_sensitive: bool,
    limit: Option<usize>,
) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let line_count = buffer.line_count();

    // Use parallel search for larger files
    // Threshold can be tuned, but PARALLEL_SEARCH_THRESHOLD lines is a reasonable start to offset thread creation overhead
    if line_count > PARALLEL_SEARCH_THRESHOLD {
        let num_threads =
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);

        if num_threads > 1 {
            let chunk_size = line_count.div_ceil(num_threads);

            return thread::scope(|s| {
                let mut handles = Vec::with_capacity(num_threads);

                for i in 0..num_threads {
                    let start = i * chunk_size;
                    let end = (start + chunk_size).min(line_count);

                    if start >= end {
                        break;
                    }

                    handles.push(s.spawn(move || {
                        find_matches_in_range(
                            buffer,
                            query,
                            case_sensitive,
                            start,
                            end,
                            limit,
                        )
                    }));
                }

                let mut matches = Vec::new();
                for handle in handles {
                    if let Ok(mut chunk_matches) = handle.join() {
                        matches.append(&mut chunk_matches);
                        if let Some(l) = limit
                            && matches.len() >= l
                        {
                            matches.truncate(l);
                            break;
                        }
                    }
                }
                matches
            });
        }
    }

    find_matches_in_range(buffer, query, case_sensitive, 0, line_count, limit)
}

/// Threshold for line count to trigger parallel search.
const PARALLEL_SEARCH_THRESHOLD: usize = 1000;

/// Maximum number of matches to return to prevent UI performance issues.
pub const MAX_MATCHES: usize = 10_000;

/// Returns the range of matches that fall within the specified logical line range (inclusive).
///
/// This function uses binary search to efficiently find the starting match
/// and iterates to find the end match, avoiding full iteration of the matches vector.
pub fn get_visible_match_range(
    matches: &[SearchMatch],
    min_logical_line: usize,
    max_logical_line: usize,
) -> std::ops::Range<usize> {
    if matches.is_empty() {
        return 0..0;
    }

    // Find the first match that is on or after min_logical_line
    let start_idx = matches.partition_point(|m| m.line < min_logical_line);

    // Find the end index (exclusive)
    // We start searching from start_idx since we know everything before is < min_logical_line
    let mut end_idx = start_idx;
    for match_item in matches.iter().skip(start_idx) {
        if match_item.line > max_logical_line {
            break;
        }
        end_idx += 1;
    }

    start_idx..end_idx
}

fn find_matches_in_range(
    buffer: &TextBuffer,
    query: &str,
    case_sensitive: bool,
    start_line: usize,
    end_line: usize,
    limit: Option<usize>,
) -> Vec<SearchMatch> {
    let mut matches = Vec::new();
    let search_query = if case_sensitive {
        Cow::Borrowed(query)
    } else {
        Cow::Owned(query.to_lowercase())
    };

    for line_idx in start_line..end_line {
        // Stop if we have enough matches
        if let Some(l) = limit
            && matches.len() >= l
        {
            break;
        }

        let line = buffer.line(line_idx);

        // Optimization: skip lines shorter than query
        if line.len() < query.len() {
            continue;
        }

        let search_line = if case_sensitive {
            Cow::Borrowed(line)
        } else {
            Cow::Owned(line.to_lowercase())
        };

        // Find all occurrences in this line
        let mut start_pos = 0;
        while let Some(relative_pos) =
            search_line[start_pos..].find(search_query.as_ref())
        {
            let absolute_pos = start_pos + relative_pos;

            // Convert byte position to character position
            // Note: In case-insensitive mode, absolute_pos is in the lowercased string.
            // Using it to slice the original line is only safe if byte lengths match.
            // We use get() to be safe against panics for weird unicode cases.
            let col = if let Some(slice) = line.get(..absolute_pos) {
                slice.chars().count()
            } else {
                // Fallback: use the position in the search line if mapping fails
                // This assumes column in search_line is "close enough"
                search_line[..absolute_pos].chars().count()
            };

            matches.push(SearchMatch { line: line_idx, col });

            // Move past this match to find next occurrence
            // Use search_query.len() to avoid overlapping matches and ensure we land on UTF-8 character boundary
            start_pos = absolute_pos + search_query.len();
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_matches_case_sensitive() {
        let buffer = TextBuffer::new("Hello World\nhello world");
        let matches = find_matches(&buffer, "hello", true, None);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[0].col, 0);
    }

    #[test]
    fn test_find_matches_case_insensitive() {
        let buffer = TextBuffer::new("Hello World\nhello world");
        let matches = find_matches(&buffer, "hello", false, None);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[0].col, 0);
        assert_eq!(matches[1].line, 1);
        assert_eq!(matches[1].col, 0);
    }

    #[test]
    fn test_find_matches_multiple_occurrences() {
        let buffer = TextBuffer::new("foo bar foo baz foo");
        let matches = find_matches(&buffer, "foo", false, None);

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].col, 0);
        assert_eq!(matches[1].col, 8);
        assert_eq!(matches[2].col, 16);
    }

    #[test]
    fn test_find_matches_multiline() {
        let buffer = TextBuffer::new("line1\nfoo\nline3\nfoo");
        let matches = find_matches(&buffer, "foo", false, None);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[1].line, 3);
    }

    #[test]
    fn test_find_matches_empty_query() {
        let buffer = TextBuffer::new("Hello World");
        let matches = find_matches(&buffer, "", false, None);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_find_matches_no_results() {
        let buffer = TextBuffer::new("Hello World");
        let matches = find_matches(&buffer, "xyz", false, None);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_search_state_navigation() {
        let buffer = TextBuffer::new("foo bar foo baz foo");
        let mut state = SearchState::new();
        state.set_query("foo".to_string(), &buffer);

        assert_eq!(state.match_count(), 3);
        assert_eq!(state.current_match_index, Some(0));

        state.next_match();
        assert_eq!(state.current_match_index, Some(1));

        state.next_match();
        assert_eq!(state.current_match_index, Some(2));

        // Test wrap-around
        state.next_match();
        assert_eq!(state.current_match_index, Some(0));

        // Test previous
        state.previous_match();
        assert_eq!(state.current_match_index, Some(2));
    }

    #[test]
    fn test_select_match_near_cursor_uses_sorted_candidates() {
        let buffer = TextBuffer::new("foo\nnone\nfoo bar foo\nnone\nfoo");
        let mut state = SearchState::new();
        state.set_query("foo".to_string(), &buffer);

        state.select_match_near_cursor((2, 7));
        assert_eq!(state.current_match_index, Some(2));
        assert_eq!(
            state.current_match(),
            Some(SearchMatch { line: 2, col: 8 })
        );

        state.select_match_near_cursor((4, 0));
        assert_eq!(
            state.current_match(),
            Some(SearchMatch { line: 4, col: 0 })
        );
    }

    #[test]
    fn test_search_state_toggle_case() {
        let buffer = TextBuffer::new("Hello hello");
        let mut state = SearchState::new();
        state.set_query("hello".to_string(), &buffer);

        assert_eq!(state.match_count(), 2);

        state.toggle_case_sensitive(&buffer);
        assert_eq!(state.match_count(), 1);

        state.toggle_case_sensitive(&buffer);
        assert_eq!(state.match_count(), 2);
    }

    #[test]
    fn test_incremental_match_update_replaces_only_affected_lines() {
        let mut buffer = TextBuffer::new("foo\nfoo\nfoo");
        let mut state = SearchState::new();
        state.set_query("foo".to_string(), &buffer);

        buffer.insert_char(1, 1, 'x');
        state.update_matches_after_edit(&buffer, 1, 2);

        assert_eq!(
            state.matches,
            vec![
                SearchMatch { line: 0, col: 0 },
                SearchMatch { line: 2, col: 0 },
            ]
        );
    }

    #[test]
    fn test_incremental_match_update_shifts_suffix_after_newline() {
        let mut buffer = TextBuffer::new("foo\nbar\nfoo");
        let mut state = SearchState::new();
        state.set_query("foo".to_string(), &buffer);

        buffer.insert_newline(0, 0);
        state.update_matches_after_edit(&buffer, 0, 2);

        assert_eq!(
            state.matches,
            vec![
                SearchMatch { line: 1, col: 0 },
                SearchMatch { line: 3, col: 0 },
            ]
        );
    }

    #[test]
    fn test_find_matches_large_buffer_parallel() {
        // Create a buffer with PARALLEL_SEARCH_THRESHOLD * 2 lines (triggers parallel path)
        let mut content = String::new();
        let num_lines = PARALLEL_SEARCH_THRESHOLD * 2;
        for i in 0..num_lines {
            content.push_str(&format!("line {} foo\n", i));
        }
        let buffer = TextBuffer::new(&content);

        let matches = find_matches(&buffer, "foo", false, None);

        assert_eq!(matches.len(), num_lines);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[num_lines - 1].line, num_lines - 1);

        // Verify order is preserved (important!)
        for (i, m) in matches.iter().enumerate() {
            assert_eq!(m.line, i);
        }
    }

    #[test]
    fn test_find_matches_limit() {
        // Create a buffer with more than MAX_MATCHES (10,000) matches
        // We'll put 11,000 "foo"s, one per line
        let mut content = String::new();
        for _ in 0..11_000 {
            content.push_str("foo\n");
        }
        let buffer = TextBuffer::new(&content);

        let matches = find_matches(&buffer, "foo", false, Some(MAX_MATCHES));

        // Should be capped at MAX_MATCHES
        assert_eq!(matches.len(), MAX_MATCHES);
    }

    #[test]
    fn test_get_visible_match_range() {
        let matches = vec![
            SearchMatch { line: 1, col: 0 },
            SearchMatch { line: 2, col: 0 },
            SearchMatch { line: 5, col: 0 },
            SearchMatch { line: 5, col: 5 },
            SearchMatch { line: 10, col: 0 },
        ];

        // All visible
        assert_eq!(get_visible_match_range(&matches, 0, 15), 0..5);

        // None visible (before)
        assert_eq!(get_visible_match_range(&matches, 0, 0), 0..0);

        // None visible (after)
        assert_eq!(get_visible_match_range(&matches, 11, 20), 5..5);

        // None visible (middle gap)
        assert_eq!(get_visible_match_range(&matches, 3, 4), 2..2);

        // Partial visible (start)
        assert_eq!(get_visible_match_range(&matches, 2, 10), 1..5);

        // Partial visible (end)
        assert_eq!(get_visible_match_range(&matches, 0, 4), 0..2);

        // Partial visible (middle)
        assert_eq!(get_visible_match_range(&matches, 2, 5), 1..4);

        // Exact match line
        assert_eq!(get_visible_match_range(&matches, 5, 5), 2..4);
    }

    #[test]
    fn test_get_visible_match_range_empty() {
        let matches = vec![];
        assert_eq!(get_visible_match_range(&matches, 0, 100), 0..0);
    }
}
