//! Code folding logic for the text editor.
//!
//! This module detects foldable regions (collapsible blocks) in the buffer and
//! computes which logical lines must be hidden when a block is collapsed.
//!
//! Detection is **indentation-based**: a line is a fold header when the next
//! non-blank line is more deeply indented. This is language-agnostic and matches
//! the fallback strategy used by editors such as VS Code. The collapsed state and
//! the on/off toggle live on [`super::CodeEditor`]; this module is pure logic so
//! it can be unit-tested in isolation.

use std::collections::HashSet;

use crate::text_buffer::TextBuffer;

use super::TAB_WIDTH;

/// A region of the buffer that can be collapsed into a single header line.
///
/// `start_line` is the header line, which always stays visible. When the region
/// is collapsed, the lines `start_line + 1 ..= end_line` are hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldRegion {
    /// Index of the header line (stays visible when collapsed).
    pub start_line: usize,
    /// Index of the last line belonging to the region (hidden when collapsed).
    pub end_line: usize,
}

impl FoldRegion {
    /// Creates a new fold region.
    ///
    /// # Arguments
    ///
    /// * `start_line` - Index of the header line
    /// * `end_line` - Index of the last line in the region
    pub fn new(start_line: usize, end_line: usize) -> Self {
        Self { start_line, end_line }
    }
}

/// Computes the visual indentation width of a line, expanding tabs.
///
/// Returns `None` for blank lines (empty or whitespace-only), which have no
/// meaningful indentation and act as "transparent" lines during detection.
///
/// # Arguments
///
/// * `line` - The line content (without the trailing newline)
fn indent_width(line: &str) -> Option<usize> {
    let mut width = 0;
    for c in line.chars() {
        match c {
            '\t' => width += TAB_WIDTH,
            ' ' => width += 1,
            _ if c.is_whitespace() => width += 1,
            _ => return Some(width),
        }
    }
    // Reached end of line without a non-whitespace character: blank line.
    None
}

/// Detects all indentation-based foldable regions in the buffer.
///
/// A line `i` is a fold header when the next non-blank line is indented more
/// deeply than `i`. The region extends over every following line that is either
/// blank or more deeply indented than the header; trailing blank lines are
/// trimmed so a collapsed block does not swallow the gap before the next block.
///
/// Nested blocks each yield their own region, so they can be folded
/// independently.
///
/// # Arguments
///
/// * `buffer` - The text buffer to analyze
///
/// # Returns
///
/// Fold regions in ascending order of `start_line`. Only regions hiding at least
/// one line are returned.
pub fn compute_foldable_regions(buffer: &TextBuffer) -> Vec<FoldRegion> {
    let line_count = buffer.line_count();
    // Precompute indentation once: O(n) instead of re-scanning per header.
    let indents: Vec<Option<usize>> =
        (0..line_count).map(|i| indent_width(buffer.line(i))).collect();

    let mut regions = Vec::new();

    for i in 0..line_count {
        let Some(header_indent) = indents[i] else {
            continue; // Blank lines are never headers.
        };

        // Scan forward, tracking the last non-blank line more deeply indented
        // than the header. Blank lines are skipped without ending the region.
        let mut last_content = i;
        let mut j = i + 1;
        while j < line_count {
            match indents[j] {
                None => j += 1, // Blank line: tentatively inside the region.
                Some(indent) if indent > header_indent => {
                    last_content = j;
                    j += 1;
                }
                Some(_) => break, // Sibling/outer line ends the region.
            }
        }

        if last_content > i {
            regions.push(FoldRegion::new(i, last_content));
        }
    }

    regions
}

/// Returns whether `line` is the header of a foldable region.
///
/// # Arguments
///
/// * `regions` - Pre-computed fold regions
/// * `line` - The logical line index to test
pub fn is_fold_header(regions: &[FoldRegion], line: usize) -> bool {
    regions.iter().any(|r| r.start_line == line)
}

/// Computes the set of logical lines hidden by the currently collapsed regions.
///
/// A region contributes its hidden lines (`start_line + 1 ..= end_line`) only
/// when its header is present in `collapsed`. Overlapping (nested) regions are
/// handled naturally because the result is a union.
///
/// # Arguments
///
/// * `regions` - Pre-computed fold regions
/// * `collapsed` - Header line indices that are currently collapsed
///
/// # Returns
///
/// The set of logical line indices that must not be rendered.
pub fn hidden_lines(
    regions: &[FoldRegion],
    collapsed: &HashSet<usize>,
) -> HashSet<usize> {
    let mut hidden = HashSet::new();
    for region in regions {
        if collapsed.contains(&region.start_line) {
            hidden.extend((region.start_line + 1)..=region.end_line);
        }
    }
    hidden
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_width_blank_lines() {
        assert_eq!(indent_width(""), None);
        assert_eq!(indent_width("   "), None);
        assert_eq!(indent_width("\t"), None);
    }

    #[test]
    fn test_indent_width_expands_tabs() {
        assert_eq!(indent_width("code"), Some(0));
        assert_eq!(indent_width("  code"), Some(2));
        assert_eq!(indent_width("\tcode"), Some(TAB_WIDTH));
        assert_eq!(indent_width("\t  code"), Some(TAB_WIDTH + 2));
    }

    #[test]
    fn test_no_regions_for_flat_text() {
        let buffer = TextBuffer::new("a\nb\nc");
        assert!(compute_foldable_regions(&buffer).is_empty());
    }

    #[test]
    fn test_simple_block() {
        // `fn main` header at line 0, body at lines 1-2, closing brace dedented.
        let buffer =
            TextBuffer::new("fn main() {\n    let x = 1;\n    let y = 2;\n}");
        let regions = compute_foldable_regions(&buffer);
        assert_eq!(regions, vec![FoldRegion::new(0, 2)]);
    }

    #[test]
    fn test_nested_blocks() {
        // Two nesting levels produce two independent regions.
        let buffer = TextBuffer::new(
            "outer:\n    inner:\n        deep\n        deeper\n    after_inner",
        );
        let regions = compute_foldable_regions(&buffer);
        assert_eq!(regions, vec![FoldRegion::new(0, 4), FoldRegion::new(1, 3)]);
    }

    #[test]
    fn test_blank_lines_inside_and_trailing() {
        // Blank line (idx 2) stays inside the block; trailing blank (idx 4)
        // before a dedented line is trimmed.
        let buffer =
            TextBuffer::new("def f():\n    a = 1\n\n    b = 2\n\ng = 3");
        let regions = compute_foldable_regions(&buffer);
        // Region covers lines 1..=3 (the blank at 2 is absorbed), but not the
        // trailing blank at line 4.
        assert_eq!(regions, vec![FoldRegion::new(0, 3)]);
    }

    #[test]
    fn test_hidden_lines_single_collapsed() {
        let regions = vec![FoldRegion::new(0, 2)];
        let collapsed: HashSet<usize> = [0].into_iter().collect();
        let hidden = hidden_lines(&regions, &collapsed);
        assert_eq!(hidden, [1, 2].into_iter().collect());
    }

    #[test]
    fn test_hidden_lines_nested_union() {
        let regions = vec![FoldRegion::new(0, 4), FoldRegion::new(1, 3)];
        // Only the inner region is collapsed.
        let collapsed: HashSet<usize> = [1].into_iter().collect();
        assert_eq!(
            hidden_lines(&regions, &collapsed),
            [2, 3].into_iter().collect()
        );

        // Both collapsed: union covers 1..=4.
        let collapsed: HashSet<usize> = [0, 1].into_iter().collect();
        assert_eq!(
            hidden_lines(&regions, &collapsed),
            [1, 2, 3, 4].into_iter().collect()
        );
    }

    #[test]
    fn test_hidden_lines_ignores_unknown_collapsed() {
        // A stale collapsed entry without a matching region contributes nothing.
        let regions = vec![FoldRegion::new(0, 2)];
        let collapsed: HashSet<usize> = [99].into_iter().collect();
        assert!(hidden_lines(&regions, &collapsed).is_empty());
    }

    #[test]
    fn test_is_fold_header() {
        let regions = vec![FoldRegion::new(0, 2), FoldRegion::new(5, 7)];
        assert!(is_fold_header(&regions, 0));
        assert!(is_fold_header(&regions, 5));
        assert!(!is_fold_header(&regions, 1));
        assert!(!is_fold_header(&regions, 3));
    }
}
