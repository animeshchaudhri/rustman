//! Multi-cursor support: `Cursor` and `CursorSet` types.
//!
//! A `CursorSet` is an ordered, deduplicated collection of cursors.
//! It always contains at least one cursor (the *primary* cursor that
//! the viewport follows).

use std::cmp::Ordering;

/// A single cursor with an optional selection anchor.
///
/// When `anchor` is `Some`, the selection spans from `anchor` to `position`
/// (in whichever direction the user dragged / shifted).
///
/// # Example
///
/// ```ignore
/// let c = Cursor::new((0, 5));
/// assert_eq!(c.position, (0, 5));
/// assert!(c.anchor.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// Current position `(line, col)`.
    pub position: (usize, usize),
    /// Selection anchor — the point where the selection started.
    /// `None` means no active selection for this cursor.
    pub anchor: Option<(usize, usize)>,
}

impl Cursor {
    /// Creates a new cursor at the given position with no selection.
    pub fn new(position: (usize, usize)) -> Self {
        Self { position, anchor: None }
    }

    /// Returns `true` if this cursor has an active selection.
    pub fn has_selection(&self) -> bool {
        match self.anchor {
            Some(a) => a != self.position,
            None => false,
        }
    }

    /// Clears the selection anchor.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Sets the selection anchor to the current position (start selecting).
    pub fn set_anchor(&mut self) {
        self.anchor = Some(self.position);
    }

    /// Returns the normalised selection range `(start, end)` where
    /// `start <= end` in document order, or `None` if there is no selection.
    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.anchor?;
        if anchor == self.position {
            return None;
        }
        Some(normalise(anchor, self.position))
    }
}

/// An ordered, deduplicated collection of cursors.
///
/// **Invariant**: always contains at least one cursor.
/// Cursors are kept sorted by position in document order after every
/// mutation that may change ordering.
///
/// The *primary* cursor is the one the viewport follows and that
/// receives IME input.
///
/// # Example
///
/// ```ignore
/// let mut cs = CursorSet::new((0, 0));
/// assert_eq!(cs.len(), 1);
/// cs.add_cursor((1, 0));
/// assert_eq!(cs.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct CursorSet {
    /// Cursors sorted by position in document order.
    cursors: Vec<Cursor>,
    /// Index of the primary cursor inside `cursors`.
    primary_idx: usize,
}

impl CursorSet {
    /// Creates a `CursorSet` with a single cursor at `pos`.
    pub fn new(pos: (usize, usize)) -> Self {
        Self { cursors: vec![Cursor::new(pos)], primary_idx: 0 }
    }

    // -----------------------------------------------------------------
    // Primary cursor access
    // -----------------------------------------------------------------

    /// Returns a reference to the primary cursor.
    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary_idx]
    }

    /// Returns a mutable reference to the primary cursor.
    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary_idx]
    }

    /// Shorthand: returns the primary cursor's `(line, col)`.
    pub fn primary_position(&self) -> (usize, usize) {
        self.cursors[self.primary_idx].position
    }

    // -----------------------------------------------------------------
    // Collection access
    // -----------------------------------------------------------------

    /// Number of cursors.
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    /// Returns `true` if there are multiple cursors.
    pub fn is_multi(&self) -> bool {
        self.cursors.len() > 1
    }

    /// Immutable iterator over all cursors (document order).
    pub fn iter(&self) -> impl Iterator<Item = &Cursor> {
        self.cursors.iter()
    }

    /// Returns a slice of all cursors.
    pub fn as_slice(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Returns a mutable slice of all cursors.
    pub fn as_mut_slice(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    // -----------------------------------------------------------------
    // Mutation
    // -----------------------------------------------------------------

    /// Adds a new cursor at `pos` (no selection) and makes it primary.
    ///
    /// If a cursor already exists at `pos`, this is a no-op (the existing
    /// cursor becomes primary).
    pub fn add_cursor(&mut self, pos: (usize, usize)) {
        let cursor = Cursor::new(pos);
        self.cursors.push(cursor);
        // The newly pushed cursor is last; mark it primary before sort.
        self.primary_idx = self.cursors.len() - 1;
        self.sort_and_merge();
    }

    /// Adds a cursor that already has a selection and makes it primary.
    pub fn add_cursor_with_selection(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
        self.primary_idx = self.cursors.len() - 1;
        self.sort_and_merge();
    }

    /// Removes all cursors except the primary one.
    pub fn remove_all_but_primary(&mut self) {
        let primary = self.cursors[self.primary_idx].clone();
        self.cursors.clear();
        self.cursors.push(primary);
        self.primary_idx = 0;
    }

    /// Replaces the entire cursor set with a single cursor at `pos`.
    pub fn set_single(&mut self, pos: (usize, usize)) {
        self.cursors.clear();
        self.cursors.push(Cursor::new(pos));
        self.primary_idx = 0;
    }

    /// Clears the selection anchor on every cursor.
    pub fn clear_all_selections(&mut self) {
        for c in &mut self.cursors {
            c.clear_selection();
        }
    }

    /// Sorts cursors by position and merges any that overlap.
    ///
    /// Two cursors merge when:
    /// - they have the same position, or
    /// - their selection ranges overlap.
    ///
    /// After merging, the primary index is updated so it still points to
    /// the same logical cursor (or the merged result).
    pub fn sort_and_merge(&mut self) {
        if self.cursors.len() <= 1 {
            return;
        }

        // Tag each cursor with its original index so we can track the primary.
        let primary_orig = self.primary_idx;
        let mut tagged: Vec<(usize, Cursor)> =
            self.cursors.drain(..).enumerate().collect();

        // Sort by the *minimum* position (considering anchor) so overlapping
        // selections are adjacent.
        tagged.sort_by(|a, b| {
            let a_min = min_pos(&a.1);
            let b_min = min_pos(&b.1);
            cmp_pos(a_min, b_min)
        });

        let mut merged: Vec<(usize, Cursor)> = Vec::with_capacity(tagged.len());
        for entry in tagged {
            if let Some(last) = merged.last_mut()
                && cursors_overlap(&last.1, &entry.1)
            {
                // Merge: keep whichever was primary, union the ranges.
                let keep_new = entry.0 == primary_orig;
                merge_into(&mut last.1, &entry.1);
                if keep_new {
                    last.0 = entry.0;
                    // Position of merged cursor: keep the new one's position
                    // (the one the user just added/moved).
                    last.1.position = entry.1.position;
                }
                continue;
            }
            merged.push(entry);
        }

        // Rebuild cursors and find the new primary index.
        self.primary_idx = 0;
        self.cursors.clear();
        for (i, (orig_idx, cursor)) in merged.into_iter().enumerate() {
            if orig_idx == primary_orig {
                self.primary_idx = i;
            }
            self.cursors.push(cursor);
        }
    }
}

// =====================================================================
// Helper functions
// =====================================================================

/// Compares two `(line, col)` positions in document order.
fn cmp_pos(a: (usize, usize), b: (usize, usize)) -> Ordering {
    a.0.cmp(&b.0).then(a.1.cmp(&b.1))
}

/// Returns `(start, end)` with `start <= end`.
fn normalise(
    a: (usize, usize),
    b: (usize, usize),
) -> ((usize, usize), (usize, usize)) {
    if cmp_pos(a, b) == Ordering::Greater { (b, a) } else { (a, b) }
}

/// Minimum position covered by a cursor (position or anchor, whichever is earlier).
fn min_pos(c: &Cursor) -> (usize, usize) {
    match c.anchor {
        Some(a) => {
            if cmp_pos(a, c.position) == Ordering::Less {
                a
            } else {
                c.position
            }
        }
        None => c.position,
    }
}

/// Maximum position covered by a cursor.
fn max_pos(c: &Cursor) -> (usize, usize) {
    match c.anchor {
        Some(a) => {
            if cmp_pos(a, c.position) == Ordering::Greater {
                a
            } else {
                c.position
            }
        }
        None => c.position,
    }
}

/// Returns `true` if two cursors overlap (same position or overlapping selections).
fn cursors_overlap(a: &Cursor, b: &Cursor) -> bool {
    let a_max = max_pos(a);
    let b_min = min_pos(b);
    // Since `a` is sorted before `b`, overlap iff a_max >= b_min.
    cmp_pos(a_max, b_min) != Ordering::Less
}

/// Merges `src` into `dst`, unioning their covered ranges.
fn merge_into(dst: &mut Cursor, src: &Cursor) {
    let combined_min = {
        let a = min_pos(dst);
        let b = min_pos(src);
        if cmp_pos(a, b) == Ordering::Less { a } else { b }
    };
    let combined_max = {
        let a = max_pos(dst);
        let b = max_pos(src);
        if cmp_pos(a, b) == Ordering::Greater { a } else { b }
    };

    // If either cursor had a selection, the merged cursor keeps the union.
    if dst.has_selection() || src.has_selection() {
        // Anchor at the combined min, position at the combined max.
        dst.anchor = Some(combined_min);
        dst.position = combined_max;
    }
    // If neither had a selection, they share the same position (already set in dst).
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_cursor() {
        let cs = CursorSet::new((3, 5));
        assert_eq!(cs.len(), 1);
        assert!(!cs.is_multi());
        assert_eq!(cs.primary_position(), (3, 5));
    }

    #[test]
    fn test_add_cursor() {
        let mut cs = CursorSet::new((0, 0));
        cs.add_cursor((2, 3));
        assert_eq!(cs.len(), 2);
        assert!(cs.is_multi());
        // Primary should be the newly added cursor.
        assert_eq!(cs.primary_position(), (2, 3));
    }

    #[test]
    fn test_add_duplicate_cursor_merges() {
        let mut cs = CursorSet::new((1, 5));
        cs.add_cursor((1, 5));
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn test_sort_order() {
        let mut cs = CursorSet::new((5, 0));
        cs.add_cursor((1, 0));
        cs.add_cursor((3, 0));
        let positions: Vec<_> = cs.iter().map(|c| c.position).collect();
        assert_eq!(positions, vec![(1, 0), (3, 0), (5, 0)]);
    }

    #[test]
    fn test_remove_all_but_primary() {
        let mut cs = CursorSet::new((0, 0));
        cs.add_cursor((1, 0));
        cs.add_cursor((2, 0));
        assert_eq!(cs.len(), 3);
        cs.remove_all_but_primary();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.primary_position(), (2, 0));
    }

    #[test]
    fn test_cursor_selection_range() {
        let mut c = Cursor::new((1, 5));
        assert!(c.selection_range().is_none());

        c.anchor = Some((1, 2));
        assert_eq!(c.selection_range(), Some(((1, 2), (1, 5))));
    }

    #[test]
    fn test_cursor_selection_range_reversed() {
        let mut c = Cursor::new((1, 2));
        c.anchor = Some((1, 8));
        assert_eq!(c.selection_range(), Some(((1, 2), (1, 8))));
    }

    #[test]
    fn test_overlapping_selections_merge() {
        let mut cs = CursorSet::new((0, 0));
        // First cursor: selection from (0,0) to (0,5)
        cs.primary_mut().anchor = Some((0, 0));
        cs.primary_mut().position = (0, 5);
        // Add cursor with overlapping selection from (0,3) to (0,8)
        let mut c2 = Cursor::new((0, 8));
        c2.anchor = Some((0, 3));
        cs.add_cursor_with_selection(c2);
        // Should merge into one cursor spanning (0,0)-(0,8).
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.primary().selection_range(), Some(((0, 0), (0, 8))));
    }

    #[test]
    fn test_clear_all_selections() {
        let mut cs = CursorSet::new((0, 0));
        cs.primary_mut().anchor = Some((0, 5));
        let mut c2 = Cursor::new((1, 0));
        c2.anchor = Some((1, 3));
        cs.add_cursor_with_selection(c2);
        cs.clear_all_selections();
        for c in cs.iter() {
            assert!(!c.has_selection());
        }
    }

    #[test]
    fn test_set_single() {
        let mut cs = CursorSet::new((0, 0));
        cs.add_cursor((1, 0));
        cs.add_cursor((2, 0));
        cs.set_single((5, 5));
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.primary_position(), (5, 5));
    }

    #[test]
    fn test_primary_tracked_through_sort() {
        let mut cs = CursorSet::new((5, 0)); // index 0 initially, primary
        cs.add_cursor((1, 0)); // becomes primary
        // After sort: [(1,0), (5,0)] — primary should be at index 0 (the (1,0) cursor).
        assert_eq!(cs.primary_position(), (1, 0));
        // Verify primary cursor is the most recently added one
        assert_eq!(cs.primary().position, (1, 0));
    }
}
