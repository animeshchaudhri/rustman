//! Canvas-based text editor widget for maximum performance.
//!
//! This module provides a custom Canvas widget that handles all text rendering
//! and input directly, bypassing Iced's higher-level widgets for optimal speed.

use iced::Color;
use iced::advanced::text::{
    Alignment, Paragraph, Renderer as TextRenderer, Text,
};
use iced::widget::operation::{RelativeOffset, snap_to};
use iced::widget::{Id, canvas};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering as CmpOrdering;
use std::collections::{BTreeMap, HashSet};
use std::ops::Range;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use syntect::highlighting::HighlightState;
use syntect::parsing::ParseState;
use unicode_width::UnicodeWidthChar;

use crate::i18n::Translations;
use crate::text_buffer::TextBuffer;
use crate::theme::Style;
pub use history::CommandHistory;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

/// Global counter for generating unique editor IDs (starts at 1)
static EDITOR_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// ID of the currently focused editor (0 = no editor focused)
static FOCUSED_EDITOR_ID: AtomicU64 = AtomicU64::new(0);

// Re-export submodules
mod canvas_impl;
mod clipboard;
pub mod command;
mod context_menu;
mod cursor;
pub(crate) mod cursor_set;
pub mod folding;
mod goto_line;
mod goto_line_dialog;
pub mod history;
pub mod ime_requester;
pub mod lsp;
#[cfg(all(feature = "lsp-process", not(target_arch = "wasm32")))]
pub mod lsp_process;
mod search;
mod search_dialog;
mod selection;
mod update;
mod view;
mod vim;
mod wrapping;

pub use context_menu::{ContextMenuEntry, ContextMenuItem};
pub use vim::VimMode;

/// Hidden re-exports for the benchmark harness in `benches/`.
///
/// This module is compiled only with the `bench` feature and is **not** part
/// of the public API. It exposes internal hot-path functions so the
/// `criterion` benchmarks (which run as a separate crate) can measure them.
#[doc(hidden)]
#[cfg(feature = "bench")]
pub mod bench_support {
    pub use super::canvas_impl::highlight_line_spans;
    pub use super::folding::compute_foldable_regions;
    pub use super::search::find_matches;
    pub use super::wrapping::WrappingCalculator;
    pub use crate::text_buffer::TextBuffer;

    /// Stateful harness for measuring the normal localized typing path.
    pub struct IncrementalEditBenchmark {
        editor: super::CodeEditor,
    }

    impl IncrementalEditBenchmark {
        /// Creates an editor, primes its visual-line cache, and places the
        /// cursor at `line`/`column`.
        pub fn new(content: &str, line: usize, column: usize) -> Self {
            let mut editor = super::CodeEditor::new(content, "rs")
                .with_wrap_column(Some(80));
            editor.request_focus();
            editor.has_canvas_focus = true;
            editor.focus_locked = false;
            editor.cursors.primary_mut().position = (line, column);
            let _ = editor.visual_lines_cached(800.0);
            Self { editor }
        }

        /// Inserts and removes one character, leaving content size stable for
        /// repeated Criterion iterations.
        pub fn insert_and_backspace(&mut self) -> u64 {
            let _ = self.editor.update(&super::Message::CharacterInput('x'));
            let _ = self.editor.update(&super::Message::Backspace);
            if self.editor.is_grouping {
                self.editor.history.end_group();
                self.editor.is_grouping = false;
            }
            self.editor.buffer_revision
        }
    }

    struct NoopLspClient;

    impl super::lsp::LspClient for NoopLspClient {}

    /// Stateful harness for the incremental LSP synchronization path.
    pub struct IncrementalLspEditBenchmark {
        editor: super::CodeEditor,
    }

    impl IncrementalLspEditBenchmark {
        /// Creates and primes a focused editor with a no-op LSP client.
        pub fn new(content: &str, line: usize, column: usize) -> Self {
            let mut editor = super::CodeEditor::new(content, "rs")
                .with_wrap_column(Some(80));
            editor.attach_lsp(
                Box::new(NoopLspClient),
                super::lsp::LspDocument::new("file:///benchmark.rs", "rust"),
            );
            editor.request_focus();
            editor.has_canvas_focus = true;
            editor.focus_locked = false;
            editor.cursors.primary_mut().position = (line, column);
            let _ = editor.visual_lines_cached(800.0);
            Self { editor }
        }

        /// Inserts and removes one character while sending incremental LSP
        /// changes for both edits.
        pub fn insert_and_backspace(&mut self) -> u64 {
            let _ = self.editor.update(&super::Message::CharacterInput('x'));
            let _ = self.editor.update(&super::Message::Backspace);
            if self.editor.is_grouping {
                self.editor.history.end_group();
                self.editor.is_grouping = false;
            }
            self.editor.buffer_revision
        }
    }

    /// Stateful harness for typing with wrapping disabled.
    pub struct IncrementalNoWrapEditBenchmark {
        editor: super::CodeEditor,
    }

    impl IncrementalNoWrapEditBenchmark {
        /// Creates an editor and primes both layout and horizontal-width caches.
        pub fn new(content: &str, line: usize, column: usize) -> Self {
            let mut editor = super::CodeEditor::new(content, "rs");
            editor.set_wrap_enabled(false);
            editor.request_focus();
            editor.has_canvas_focus = true;
            editor.focus_locked = false;
            editor.cursors.primary_mut().position = (line, column);
            let _ = editor.visual_lines_cached(800.0);
            let _ = editor.max_content_width();
            Self { editor }
        }

        /// Inserts and removes one character without triggering a whole-file
        /// maximum-width scan.
        pub fn insert_and_backspace(&mut self) -> u64 {
            let _ = self.editor.update(&super::Message::CharacterInput('x'));
            let _ = self.editor.update(&super::Message::Backspace);
            if self.editor.is_grouping {
                self.editor.history.end_group();
                self.editor.is_grouping = false;
            }
            self.editor.buffer_revision
        }
    }

    /// Stateful harness for typing while a large-file search is open.
    pub struct IncrementalSearchEditBenchmark {
        editor: super::CodeEditor,
    }

    impl IncrementalSearchEditBenchmark {
        /// Creates an editor with populated search results and a warm layout.
        pub fn new(
            content: &str,
            query: &str,
            line: usize,
            column: usize,
        ) -> Self {
            let mut editor = super::CodeEditor::new(content, "rs")
                .with_wrap_column(Some(80));
            editor.search_state.open_search();
            editor.search_state.set_query(query.to_owned(), &editor.buffer);
            editor.request_focus();
            editor.has_canvas_focus = true;
            editor.focus_locked = false;
            editor.cursors.primary_mut().position = (line, column);
            let _ = editor.visual_lines_cached(800.0);
            Self { editor }
        }

        /// Inserts and removes one character while maintaining search matches.
        pub fn insert_and_backspace(&mut self) -> u64 {
            let _ = self.editor.update(&super::Message::CharacterInput('x'));
            let _ = self.editor.update(&super::Message::Backspace);
            if self.editor.is_grouping {
                self.editor.history.end_group();
                self.editor.is_grouping = false;
            }
            self.editor.buffer_revision
        }
    }

    /// Measures the incremental wrapping path without exposing internal visual
    /// line types as public editor API.
    pub fn calculate_visual_line_range_len(
        calculator: &WrappingCalculator,
        buffer: &TextBuffer,
        viewport_width: f32,
        gutter_width: f32,
        start_line: usize,
        end_line: usize,
    ) -> usize {
        calculator
            .calculate_visual_lines_range(
                buffer,
                viewport_width,
                gutter_width,
                &std::collections::HashSet::new(),
                start_line..end_line,
            )
            .len()
    }
}

/// Canvas-based text editor constants
pub(crate) const FONT_SIZE: f32 = 14.0;
pub(crate) const LINE_HEIGHT: f32 = 20.0;
pub(crate) const CHAR_WIDTH: f32 = 8.4; // Monospace character width
pub(crate) const TAB_WIDTH: usize = 4;
pub(crate) const GUTTER_WIDTH: f32 = 45.0;
/// Width in pixels of the fold margin (chevron column) added to the gutter when
/// code folding is enabled.
pub(crate) const FOLD_MARGIN_WIDTH: f32 = 14.0;
pub(crate) const CURSOR_BLINK_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(530);
/// How long after construction the editor keeps re-verifying its char-width
/// measurement against the live font system (see `ensure_font_metrics`).
/// Fonts registered by the host application at startup are guaranteed to be
/// loaded well within this window.
pub(crate) const FONT_METRICS_SETTLE_DURATION: std::time::Duration =
    std::time::Duration::from_secs(5);
/// A draw after a pause longer than this triggers a char-width re-verification,
/// so editors that stayed hidden during startup still get corrected on their
/// first real render.
pub(crate) const FONT_METRICS_REVISIT_GAP: std::time::Duration =
    std::time::Duration::from_secs(1);
/// Minimum absolute width difference that counts as the real font having
/// replaced the fallback measurement.
pub(crate) const FONT_METRICS_EPSILON: f32 = 0.25;

/// Measures the width of a single character.
///
/// # Arguments
///
/// * `c` - The character to measure
/// * `full_char_width` - The width of a full-width character
/// * `char_width` - The width of the character
///
/// # Returns
///
/// The calculated width of the character as a `f32`
pub(crate) fn measure_char_width(
    c: char,
    full_char_width: f32,
    char_width: f32,
) -> f32 {
    if c == '\t' {
        return char_width * TAB_WIDTH as f32;
    }
    match c.width() {
        Some(w) if w > 1 => full_char_width,
        Some(_) => char_width,
        None => 0.0,
    }
}

/// Measures rendered text width, accounting for CJK wide characters.
///
/// - Wide characters (e.g. Chinese) use FONT_SIZE.
/// - Narrow characters (e.g. Latin) use CHAR_WIDTH.
/// - Control characters (except tab) have width 0.
///
/// # Arguments
///
/// * `text` - The text string to measure
/// * `full_char_width` - The width of a full-width character
/// * `char_width` - The width of a regular character
///
/// # Returns
///
/// The total calculated width of the text as a `f32`
pub(crate) fn measure_text_width(
    text: &str,
    full_char_width: f32,
    char_width: f32,
) -> f32 {
    text.chars()
        .map(|c| measure_char_width(c, full_char_width, char_width))
        .sum()
}

/// Epsilon value for floating-point comparisons in text layout.
pub(crate) const EPSILON: f32 = 0.001;
/// Multiplier used to extend the cached render window beyond the visible range.
/// The cache window margin is computed as:
///     margin = visible_lines_count * CACHE_WINDOW_MARGIN_MULTIPLIER
/// A larger margin reduces how often we clear and rebuild the canvas cache when
/// scrolling, improving performance on very large files while still ensuring
/// correct initial rendering during the first scroll.
pub(crate) const CACHE_WINDOW_MARGIN_MULTIPLIER: usize = 2;
/// Maximum number of previously unseen logical lines syntect may parse while
/// rebuilding one content frame.
///
/// Syntax state is sequential, so jumping far into a large file can otherwise
/// parse every line from the start in one blocking draw call. Lines beyond this
/// budget temporarily use the editor's plain text color and will be highlighted
/// as later content redraws advance the cached parser state.
pub(crate) const HIGHLIGHT_LINES_PER_FRAME: usize = 2_000;

/// Compares two floating point numbers with a small epsilon tolerance.
///
/// # Arguments
///
/// * `a` - first float number
/// * `b` - second float number
///
/// # Returns
///
/// * `Ordering::Equal` if `abs(a - b) < EPSILON`
/// * `Ordering::Greater` if `a > b` (and not equal)
/// * `Ordering::Less` if `a < b` (and not equal)
pub(crate) fn compare_floats(a: f32, b: f32) -> CmpOrdering {
    if (a - b).abs() < EPSILON {
        CmpOrdering::Equal
    } else if a > b {
        CmpOrdering::Greater
    } else {
        CmpOrdering::Less
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ImePreedit {
    pub(crate) content: String,
    pub(crate) selection: Option<Range<usize>>,
}

/// Conservative logical-line range captured immediately before an edit.
///
/// It lets LSP synchronization send one incremental range replacement without
/// serializing and diffing the entire document after every keystroke.
pub(crate) struct LspEditSnapshot {
    pub(crate) start_line: usize,
    pub(crate) old_end_exclusive: usize,
    pub(crate) old_line_count: usize,
    pub(crate) old_end: lsp::LspPosition,
}

/// Canvas-based high-performance text editor.
pub struct CodeEditor {
    /// Unique ID for this editor instance (for focus management)
    pub(crate) editor_id: u64,
    /// Text buffer
    pub(crate) buffer: TextBuffer,
    /// All cursor positions (multi-cursor support).
    pub(crate) cursors: cursor_set::CursorSet,
    /// Horizontal scroll offset in pixels, only used when wrap_enabled = false
    pub(crate) horizontal_scroll_offset: f32,
    /// Editor theme style
    pub(crate) style: Style,
    /// Syntax highlighting language
    pub(crate) syntax: String,
    /// Last cursor blink time
    pub(crate) last_blink: Instant,
    /// Wall-clock creation time; used to bound the font-metrics re-check window
    /// (fonts can be registered asynchronously after the editor is built).
    pub(crate) created_at: Instant,
    /// When the editor was last drawn; used to re-verify font metrics after a
    /// long pause in rendering (see `ensure_font_metrics`).
    pub(crate) last_draw_at: Cell<Instant>,
    /// `true` once the char-width measurement has been observed to change from
    /// the value captured at construction (i.e. the real font loaded and the
    /// metrics are now final), so the per-frame re-check can stop.
    pub(crate) font_metrics_updated: Cell<bool>,
    /// Cursor visible state
    pub(crate) cursor_visible: bool,
    /// Mouse is currently dragging for selection
    pub(crate) is_dragging: bool,
    /// Cached geometry for the "content" layer.
    ///
    /// This layer includes expensive-to-build, mostly static visuals such as:
    /// - syntax-highlighted text glyphs
    /// - line numbers / gutter text
    ///
    /// It is intentionally kept stable across selection/cursor movement so
    /// that mouse-drag selection feels smooth.
    pub(crate) content_cache: canvas::Cache,
    /// Cached geometry for the "overlay" layer.
    ///
    /// This layer includes visuals that change frequently without modifying the
    /// underlying buffer, such as:
    /// - cursor and current-line highlight
    /// - selection highlight
    /// - search match highlights
    /// - IME preedit decorations
    ///
    /// Keeping overlays in a separate cache avoids invalidating the content
    /// layer on every cursor blink or selection drag.
    pub(crate) overlay_cache: canvas::Cache,
    /// Scrollable ID for programmatic scrolling
    pub(crate) scrollable_id: Id,
    /// ID for the horizontal scrollable widget (only used when wrap_enabled = false)
    pub(crate) horizontal_scrollable_id: Id,
    /// Incremental per-line width index for the horizontal scrollbar.
    pub(crate) max_content_width_cache: RefCell<Option<MaxContentWidthCache>>,
    /// Current viewport scroll position (Y offset)
    pub(crate) viewport_scroll: f32,
    /// Viewport height (visible area)
    pub(crate) viewport_height: f32,
    /// Viewport width (visible area)
    pub(crate) viewport_width: f32,
    /// Whether `viewport_height`/`viewport_width` have ever been set from a
    /// real `Scrolled` event. Until then they hold construction-time
    /// placeholder values (chosen so undirected logic like Page Up/Down has
    /// *something* sane to divide by before the first real layout), which
    /// `draw()` must NOT trust for deciding what's actually visible — it
    /// uses the real per-frame layout bounds instead until this is `true`.
    pub(crate) viewport_metrics_confirmed: bool,
    /// Command history for undo/redo
    pub(crate) history: CommandHistory,
    /// Whether we're currently grouping commands (for smart undo)
    pub(crate) is_grouping: bool,
    /// Line wrapping enabled
    pub(crate) wrap_enabled: bool,
    /// Auto-indentation enabled
    pub(crate) auto_indent_enabled: bool,
    /// Indentation style (spaces or tab)
    pub(crate) indent_style: IndentStyle,
    /// Wrap column (None = wrap at viewport width)
    pub(crate) wrap_column: Option<usize>,
    /// Whether code folding (collapse/expand blocks) is enabled.
    pub(crate) folding_enabled: bool,
    /// Header line indices of regions that are currently collapsed.
    pub(crate) collapsed_folds: HashSet<usize>,
    /// Monotonic revision counter for fold state.
    ///
    /// Bumped whenever the collapsed set or the folding toggle changes, so that
    /// derived layout caches (visual lines) are invalidated.
    pub(crate) fold_revision: u64,
    /// Cached foldable regions, keyed by `buffer_revision`.
    pub(crate) foldable_regions_cache:
        RefCell<Option<(u64, Rc<Vec<folding::FoldRegion>>)>>,
    /// Search state
    pub(crate) search_state: search::SearchState,
    /// Custom entries displayed before the built-in context-menu actions.
    custom_context_menu_entries: Vec<ContextMenuEntry>,
    /// Whether the built-in editing actions are shown in the context menu.
    default_context_menu_enabled: bool,
    /// Whether the built-in reveal-in-file-manager action is shown.
    reveal_in_file_manager_enabled: bool,
    /// Go-to-line dialog state
    pub(crate) goto_line_state: goto_line::GotoLineState,
    /// Whether Vim key handling is enabled for this editor instance.
    vim_enabled: bool,
    /// Per-editor Vim mode, parser prefixes and unnamed register.
    pub(crate) vim_state: vim::VimState,
    /// Translations for UI text
    pub(crate) translations: Translations,
    /// Whether search/replace functionality is enabled
    pub(crate) search_replace_enabled: bool,
    /// Whether line numbers are displayed
    pub(crate) line_numbers_enabled: bool,
    /// Whether to render whitespace characters visibly (spaces as `·`, tabs as `→`)
    pub(crate) show_whitespace: bool,
    /// Whether LSP support is enabled
    pub(crate) lsp_enabled: bool,
    /// Active LSP client connection, if configured.
    pub(crate) lsp_client: Option<Box<dyn lsp::LspClient>>,
    /// Metadata for the currently open LSP document.
    pub(crate) lsp_document: Option<lsp::LspDocument>,
    /// Pending incremental LSP text changes not yet flushed.
    pub(crate) lsp_pending_changes: Vec<lsp::LspTextChange>,
    /// Shadow copy of buffer content used to compute LSP deltas.
    pub(crate) lsp_shadow_text: String,
    /// Whether `lsp_shadow_text` still exactly matches the server document.
    pub(crate) lsp_shadow_is_current: bool,
    /// Current server-side line count, maintained incrementally.
    pub(crate) lsp_synced_line_count: usize,
    /// Length of the current server-side final line in Unicode scalar values.
    pub(crate) lsp_synced_last_line_len: usize,
    /// Pre-edit range used to build a bounded incremental LSP change.
    pub(crate) lsp_edit_snapshot: Option<LspEditSnapshot>,
    /// Whether to auto-flush LSP changes after edits.
    pub(crate) lsp_auto_flush: bool,
    /// Whether the canvas has user input focus (for keyboard events)
    pub(crate) has_canvas_focus: bool,
    /// Whether input processing is locked to prevent focus stealing
    pub(crate) focus_locked: bool,
    /// Whether to show the cursor (for rendering)
    pub(crate) show_cursor: bool,
    /// Current keyboard modifiers state (Ctrl, Alt, Shift, Logo).
    ///
    /// This is updated via subscription events and used to handle modifier-dependent
    /// interactions, such as "Ctrl+Click" for jumping to a definition.
    pub(crate) modifiers: Cell<iced::keyboard::Modifiers>,
    /// Last left-button press (time, position, consecutive count), used to
    /// detect double/triple clicks.
    pub(crate) last_click: Cell<Option<(Instant, iced::Point, u8)>>,
    /// The font used for rendering text
    pub(crate) font: iced::Font,
    /// IME pre-edit state (for CJK input)
    pub(crate) ime_preedit: Option<ImePreedit>,
    /// Font size in pixels
    pub(crate) font_size: f32,
    /// Full character width (wide chars like CJK) in pixels.
    ///
    /// Held in a `Cell` because it can be re-measured during rendering: the
    /// initial measurement happens before embedded fonts are loaded, so glyph
    /// metrics are re-verified on the first frames (see
    /// [`CodeEditor::ensure_font_metrics`]).
    pub(crate) full_char_width: Cell<f32>,
    /// Line height in pixels
    pub(crate) line_height: f32,
    /// Character width in pixels.
    ///
    /// Held in a `Cell` — see [`CodeEditor::full_char_width`].
    pub(crate) char_width: Cell<f32>,
    /// Cached render window: the first visual line index included in the cache.
    /// We keep a larger window than the currently visible range to avoid clearing
    /// the canvas cache on every small scroll. Only when scrolling crosses the
    /// window boundary do we re-window and clear the cache.
    pub(crate) last_first_visible_line: usize,
    /// Cached render window start line (inclusive)
    pub(crate) cache_window_start_line: usize,
    /// Cached render window end line (exclusive)
    pub(crate) cache_window_end_line: usize,
    /// Monotonic revision counter for buffer content.
    ///
    /// Any operation that changes the buffer must bump this counter to
    /// invalidate derived layout caches (e.g. wrapping / visual lines). The
    /// exact value is not semantically meaningful, so `wrapping_add` is used to
    /// avoid overflow panics while still producing a different key.
    pub(crate) buffer_revision: u64,
    /// Cached result of line wrapping ("visual lines") for the current layout key.
    ///
    /// This is stored behind a `RefCell` because wrapping is needed during
    /// rendering (where we only have `&self`), but we still want to memoize the
    /// expensive computation without forcing external mutability.
    visual_lines_cache: RefCell<Option<VisualLinesCache>>,
    /// Sequential per-line syntax-highlight cache (see [`HighlightCache`]).
    ///
    /// Stored behind a `RefCell` because highlighting is performed during
    /// rendering (where only `&self` is available) yet should be memoized.
    /// Spans are reused across wrapped visual segments and across scroll-only
    /// renders. On an edit the cache is truncated from the first changed line
    /// (tracked via `pre_edit_line`) rather than fully cleared, so multi-line
    /// constructs stay correct without re-parsing the whole file.
    pub(crate) highlight_cache: RefCell<Option<HighlightCache>>,
    /// Remaining syntax lines that may be parsed during the current content
    /// render. `usize::MAX` keeps direct non-render uses (notably tests) uncapped.
    pub(crate) highlight_lines_remaining: Cell<usize>,
    /// Topmost logical line touched by the cursors/selections before the
    /// current edit, captured at the top of `update()`.
    ///
    /// Used as a conservative lower bound for the first line an edit may
    /// change, to truncate `highlight_cache` precisely.
    pub(crate) pre_edit_line: usize,
    /// Bottommost logical line touched before the current edit.
    ///
    /// Together with `pre_edit_line`, this bounds the portion of the visual-line
    /// cache that must be rebuilt after a localized edit.
    pub(crate) pre_edit_last_line: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct VisualLinesKey {
    buffer_revision: u64,
    /// `f32::to_bits()` is used so the cache key is stable and exact:
    /// - no epsilon comparisons are required
    /// - NaN payloads (if any) do not collapse unexpectedly
    viewport_width_bits: u32,
    gutter_width_bits: u32,
    wrap_enabled: bool,
    wrap_column: Option<usize>,
    folding_enabled: bool,
    fold_revision: u64,
    full_char_width_bits: u32,
    char_width_bits: u32,
}

struct VisualLinesCache {
    key: VisualLinesKey,
    visual_lines: Rc<Vec<wrapping::VisualLine>>,
    buffer_line_count: usize,
}

/// Per-line widths plus a counted ordered index for O(log n) max updates.
pub(crate) struct MaxContentWidthCache {
    revision: u64,
    line_widths: Vec<f32>,
    width_counts: BTreeMap<u32, usize>,
}

impl MaxContentWidthCache {
    fn add_width(&mut self, width: f32) {
        *self.width_counts.entry(width.to_bits()).or_insert(0) += 1;
    }

    fn remove_width(&mut self, width: f32) {
        let bits = width.to_bits();
        let remove_entry = if let Some(count) = self.width_counts.get_mut(&bits)
        {
            *count = count.saturating_sub(1);
            *count == 0
        } else {
            false
        };
        if remove_entry {
            self.width_counts.remove(&bits);
        }
    }

    fn max_width(&self) -> f32 {
        self.width_counts
            .last_key_value()
            .map_or(0.0, |(bits, _)| f32::from_bits(*bits))
    }
}

/// One highlighted logical line together with the syntect parser state *after*
/// it, so highlighting can resume sequentially from any cached line.
///
/// Storing the post-line state is what makes multi-line constructs (block
/// comments, multi-line strings) highlight correctly: line `N` is highlighted
/// starting from the state left by line `N - 1`.
struct CachedHighlightLine {
    /// Colored token spans covering the full logical line.
    spans: Rc<Vec<(Color, String)>>,
    /// Syntect parse state after this line (start state for the next line).
    parse_state: ParseState,
    /// Syntect highlight state after this line (start state for the next line).
    highlight_state: HighlightState,
}

/// Sequential per-line syntax-highlight cache.
///
/// `lines` holds a dense, valid prefix: `lines[i]` is the highlight of logical
/// line `i`. The prefix is extended lazily as deeper lines become visible and
/// truncated from the first edited line on each edit (see
/// [`CodeEditor::invalidate_highlight_from`]), so an edit never forces a full
/// re-parse from the top of the file.
pub(crate) struct HighlightCache {
    /// Active syntax/language identifier these lines were highlighted with.
    syntax: String,
    /// Dense valid prefix of highlighted lines (vector index = logical line).
    lines: Vec<CachedHighlightLine>,
}

impl HighlightCache {
    /// Creates an empty cache for the given syntax identifier.
    ///
    /// # Arguments
    ///
    /// * `syntax` - Active syntax/language identifier the cache is built for.
    pub(crate) fn new(syntax: String) -> Self {
        Self { syntax, lines: Vec::new() }
    }

    /// Returns the syntax identifier these lines were highlighted with.
    pub(crate) fn syntax(&self) -> &str {
        &self.syntax
    }

    /// Returns the number of highlighted logical lines (valid prefix length).
    pub(crate) fn valid_len(&self) -> usize {
        self.lines.len()
    }

    /// Returns the cached spans for `logical_line`, if within the valid prefix.
    ///
    /// # Arguments
    ///
    /// * `logical_line` - Index of the logical line to look up.
    pub(crate) fn spans(
        &self,
        logical_line: usize,
    ) -> Option<Rc<Vec<(Color, String)>>> {
        self.lines.get(logical_line).map(|line| Rc::clone(&line.spans))
    }

    /// Returns the syntect state to resume highlighting the next line from.
    ///
    /// This is the state left after the last cached line, or `None` when the
    /// cache is empty (highlighting then starts from the syntax's initial
    /// state).
    pub(crate) fn resume_state(&self) -> Option<(ParseState, HighlightState)> {
        self.lines.last().map(|line| {
            (line.parse_state.clone(), line.highlight_state.clone())
        })
    }

    /// Appends one highlighted line and its post-line state to the prefix.
    ///
    /// # Arguments
    ///
    /// * `spans` - The colored token spans of the line.
    /// * `parse_state` - Syntect parse state after the line.
    /// * `highlight_state` - Syntect highlight state after the line.
    pub(crate) fn push_line(
        &mut self,
        spans: Rc<Vec<(Color, String)>>,
        parse_state: ParseState,
        highlight_state: HighlightState,
    ) {
        self.lines.push(CachedHighlightLine {
            spans,
            parse_state,
            highlight_state,
        });
    }

    /// Truncates the valid prefix to `line`, discarding lines at index `line`
    /// and beyond so they are re-highlighted on next access.
    ///
    /// # Arguments
    ///
    /// * `line` - First logical line to invalidate.
    pub(crate) fn truncate(&mut self, line: usize) {
        self.lines.truncate(line);
    }
}

/// Messages emitted by the code editor
#[derive(Debug, Clone)]
pub enum Message {
    /// Character typed
    CharacterInput(char),
    /// A printable key interpreted by the Vim state machine.
    VimKey(char),
    /// Toggle Vim behavior for this editor instance.
    ToggleVimMode,
    /// Requests that the host save this editor's current document.
    WriteRequested,
    /// Backspace pressed
    Backspace,
    /// Delete pressed
    Delete,
    /// Enter pressed
    Enter,
    /// Tab pressed (inserts 4 spaces)
    Tab,
    /// Arrow key pressed (direction, shift_pressed)
    ArrowKey(ArrowDirection, bool),
    /// Mouse clicked at position
    MouseClick(iced::Point),
    /// Mouse drag for selection
    MouseDrag(iced::Point),
    /// Mouse moved within the editor without dragging
    MouseHover(iced::Point),
    /// Mouse released
    MouseRelease,
    /// Double-click: select the word under the cursor
    DoubleClick(iced::Point),
    /// Triple-click: select the whole line under the cursor
    TripleClick(iced::Point),
    /// Right-clicked in the editor to position and open the context menu
    ContextMenuRequested(iced::Point),
    /// A configured context-menu action was selected.
    CustomContextMenuAction(String),
    /// Requests that the host reveal the editor's file in the system file manager.
    RevealInFileManager,
    /// Cut selected text
    Cut,
    /// Copy selected text (Ctrl+C)
    Copy,
    /// Paste text from clipboard (Ctrl+V)
    Paste(String),
    /// Delete selected text (Shift+Delete)
    DeleteSelection,
    /// Select the complete document
    SelectAll,
    /// Request redraw for cursor blink
    Tick,
    /// Page Up pressed
    PageUp,
    /// Page Down pressed
    PageDown,
    /// Home key pressed (move to start of line, shift_pressed)
    Home(bool),
    /// End key pressed (move to end of line, shift_pressed)
    End(bool),
    /// Ctrl+Home pressed (move to start of document)
    CtrlHome,
    /// Ctrl+End pressed (move to end of document)
    CtrlEnd,
    /// Go to an explicit logical position (line, column), both 0-based.
    GotoPosition(usize, usize),
    /// Open the go-to-line dialog (Cmd/Ctrl+G).
    OpenGotoLine,
    /// Close the go-to-line dialog.
    CloseGotoLine,
    /// Change the one-based line number shown in the go-to-line input.
    GotoLineChanged(String),
    /// Submit the current go-to-line input.
    SubmitGotoLine,
    /// Viewport scrolled - track scroll position
    Scrolled(iced::widget::scrollable::Viewport),
    /// Horizontal scrollbar scrolled (only when wrap is disabled)
    HorizontalScrolled(iced::widget::scrollable::Viewport),
    /// Undo last operation (Ctrl+Z)
    Undo,
    /// Redo last undone operation (Ctrl+Y)
    Redo,
    /// Open search dialog (Ctrl+F)
    OpenSearch,
    /// Open search and replace dialog (Ctrl+H)
    OpenSearchReplace,
    /// Close search dialog (Escape)
    CloseSearch,
    /// Search query text changed
    SearchQueryChanged(String),
    /// Replace text changed
    ReplaceQueryChanged(String),
    /// Toggle case sensitivity
    ToggleCaseSensitive,
    /// Find next match (F3)
    FindNext,
    /// Find previous match (Shift+F3)
    FindPrevious,
    /// Replace current match
    ReplaceNext,
    /// Replace all matches
    ReplaceAll,
    /// Tab pressed in search dialog (cycle forward)
    SearchDialogTab,
    /// Shift+Tab pressed in search dialog (cycle backward)
    SearchDialogShiftTab,
    /// Tab pressed for focus navigation (when search dialog is not open)
    FocusNavigationTab,
    /// Shift+Tab pressed for focus navigation (when search dialog is not open)
    FocusNavigationShiftTab,
    /// Canvas gained focus (mouse click)
    CanvasFocusGained,
    /// Canvas lost focus (external widget interaction)
    CanvasFocusLost,
    /// Triggered when the user performs a Ctrl+Click (or Cmd+Click on macOS)
    /// on the editor content, intending to jump to the definition of the symbol
    /// under the cursor.
    JumpClick(iced::Point),
    /// IME input method opened
    ImeOpened,
    /// IME pre-edit update (content, selection range)
    ImePreedit(String, Option<Range<usize>>),
    /// IME commit text
    ImeCommit(String),
    /// IME input method closed
    ImeClosed,
    /// Alt+Click: add a new cursor at the given canvas position
    AltClick(iced::Point),
    /// Ctrl+Alt+Up: add a cursor on the line above the primary cursor
    AddCursorAbove,
    /// Ctrl+Alt+Down: add a cursor on the line below the primary cursor
    AddCursorBelow,
    /// Ctrl+D: select the next occurrence of the currently selected text (or word under cursor)
    SelectNextOccurrence,
    /// Toggle the collapsed state of the fold whose header is the given logical line.
    ToggleFold(usize),
    /// Toggle the collapsed state of the innermost block containing the primary cursor.
    ToggleFoldAtCursor,
    /// Fold every foldable block in the buffer.
    FoldAll,
    /// Unfold every collapsed block in the buffer.
    UnfoldAll,
    /// Alt+Up: move the current line (or selected line range) up by one line.
    MoveLineUp,
    /// Alt+Down: move the current line (or selected line range) down by one line.
    MoveLineDown,
    /// Shift+Alt+Up: duplicate the current line (or selected line range) above.
    DuplicateLineUp,
    /// Shift+Alt+Down: duplicate the current line (or selected line range) below.
    DuplicateLineDown,
    /// Ctrl+/: toggle line comments on the current line or primary selection.
    ToggleComment,
}

/// Indentation style used when pressing the Tab key.
///
/// Controls whether indentation inserts spaces or a tab character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    /// Insert `n` space characters.
    Spaces(u8),
    /// Insert a single tab character (`\t`).
    Tab,
}

impl IndentStyle {
    /// All standard indentation styles available for selection.
    pub const ALL: [IndentStyle; 4] = [
        IndentStyle::Spaces(2),
        IndentStyle::Spaces(4),
        IndentStyle::Spaces(8),
        IndentStyle::Tab,
    ];
}

impl std::fmt::Display for IndentStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndentStyle::Spaces(1) => write!(f, "1 space"),
            IndentStyle::Spaces(n) => write!(f, "{n} spaces"),
            IndentStyle::Tab => write!(f, "Tab"),
        }
    }
}

/// Arrow key directions
#[derive(Debug, Clone, Copy)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}

impl CodeEditor {
    /// Creates a new canvas-based text editor.
    ///
    /// # Arguments
    ///
    /// * `content` - Initial text content
    /// * `syntax` - Syntax highlighting language (e.g., "py", "lua", "rs")
    ///
    /// # Returns
    ///
    /// A new `CodeEditor` instance
    pub fn new(content: &str, syntax: &str) -> Self {
        // Generate a unique ID for this editor instance
        let editor_id = EDITOR_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Give focus to the first editor created (ID == 1)
        if editor_id == 1 {
            FOCUSED_EDITOR_ID.store(editor_id, Ordering::Relaxed);
        }

        let mut editor = Self {
            editor_id,
            buffer: TextBuffer::new(content),
            cursors: cursor_set::CursorSet::new((0, 0)),
            horizontal_scroll_offset: 0.0,
            style: crate::theme::from_iced_theme(&iced::Theme::TokyoNightStorm),
            syntax: syntax.to_string(),
            last_blink: Instant::now(),
            created_at: Instant::now(),
            last_draw_at: Cell::new(Instant::now()),
            font_metrics_updated: Cell::new(false),
            cursor_visible: true,
            is_dragging: false,
            content_cache: canvas::Cache::default(),
            overlay_cache: canvas::Cache::default(),
            scrollable_id: Id::unique(),
            horizontal_scrollable_id: Id::unique(),
            max_content_width_cache: RefCell::new(None),
            viewport_scroll: 0.0,
            viewport_height: 600.0, // Default, will be updated
            viewport_width: 800.0,  // Default, will be updated
            viewport_metrics_confirmed: false,
            history: CommandHistory::new(100),
            is_grouping: false,
            wrap_enabled: true,
            auto_indent_enabled: true,
            indent_style: IndentStyle::Spaces(4),
            wrap_column: None,
            folding_enabled: true,
            collapsed_folds: HashSet::new(),
            fold_revision: 0,
            foldable_regions_cache: RefCell::new(None),
            search_state: search::SearchState::new(),
            custom_context_menu_entries: Vec::new(),
            default_context_menu_enabled: true,
            reveal_in_file_manager_enabled: false,
            goto_line_state: goto_line::GotoLineState::new(),
            vim_enabled: false,
            vim_state: vim::VimState::default(),
            translations: Translations::default(),
            search_replace_enabled: true,
            line_numbers_enabled: true,
            show_whitespace: true,
            lsp_enabled: true,
            lsp_client: None,
            lsp_document: None,
            lsp_pending_changes: Vec::new(),
            lsp_shadow_text: String::new(),
            lsp_shadow_is_current: true,
            lsp_synced_line_count: 1,
            lsp_synced_last_line_len: 0,
            lsp_edit_snapshot: None,
            lsp_auto_flush: true,
            has_canvas_focus: false,
            focus_locked: false,
            show_cursor: false,
            modifiers: Cell::new(iced::keyboard::Modifiers::default()),
            last_click: Cell::new(None),
            font: iced::Font::MONOSPACE,
            ime_preedit: None,
            font_size: FONT_SIZE,
            full_char_width: Cell::new(CHAR_WIDTH * 2.0),
            line_height: LINE_HEIGHT,
            char_width: Cell::new(CHAR_WIDTH),
            // Initialize render window tracking for virtual scrolling:
            // these indices define the cached visual line window. The window is
            // expanded beyond the visible range to amortize redraws and keep scrolling smooth.
            last_first_visible_line: 0,
            cache_window_start_line: 0,
            cache_window_end_line: 0,
            buffer_revision: 0,
            visual_lines_cache: RefCell::new(None),
            highlight_cache: RefCell::new(None),
            highlight_lines_remaining: Cell::new(usize::MAX),
            pre_edit_line: 0,
            pre_edit_last_line: 0,
        };

        // Perform initial character dimension calculation
        editor.recalculate_char_dimensions(false);

        editor
    }

    /// Replaces the custom context-menu entries.
    pub fn set_custom_context_menu_entries(
        &mut self,
        entries: Vec<ContextMenuEntry>,
    ) {
        self.custom_context_menu_entries = entries;
    }

    /// Replaces the custom context-menu entries using the builder pattern.
    #[must_use]
    pub fn with_custom_context_menu_entries(
        mut self,
        entries: Vec<ContextMenuEntry>,
    ) -> Self {
        self.set_custom_context_menu_entries(entries);
        self
    }

    /// Returns the custom context-menu entries in display order.
    pub fn custom_context_menu_entries(&self) -> &[ContextMenuEntry] {
        &self.custom_context_menu_entries
    }

    /// Sets whether the built-in editing actions appear in the context menu.
    pub fn set_default_context_menu_enabled(&mut self, enabled: bool) {
        self.default_context_menu_enabled = enabled;
    }

    /// Sets built-in context-menu visibility using the builder pattern.
    #[must_use]
    pub fn with_default_context_menu_enabled(mut self, enabled: bool) -> Self {
        self.set_default_context_menu_enabled(enabled);
        self
    }

    /// Returns whether the built-in context-menu actions are enabled.
    pub fn default_context_menu_enabled(&self) -> bool {
        self.default_context_menu_enabled
    }

    /// Sets whether the built-in reveal-in-file-manager action is shown.
    pub fn set_reveal_in_file_manager_enabled(&mut self, enabled: bool) {
        self.reveal_in_file_manager_enabled = enabled;
    }

    /// Sets reveal-in-file-manager visibility using the builder pattern.
    #[must_use]
    pub fn with_reveal_in_file_manager_enabled(
        mut self,
        enabled: bool,
    ) -> Self {
        self.set_reveal_in_file_manager_enabled(enabled);
        self
    }

    /// Returns whether the reveal-in-file-manager action is shown.
    pub fn reveal_in_file_manager_enabled(&self) -> bool {
        self.reveal_in_file_manager_enabled
    }

    /// Sets the font used by the editor
    ///
    /// # Arguments
    ///
    /// * `font` - The iced font to set for the editor
    pub fn set_font(&mut self, font: iced::Font) {
        self.font = font;
        self.recalculate_char_dimensions(false);
    }

    /// Sets the font size and recalculates character dimensions.
    ///
    /// If `auto_adjust_line_height` is true, `line_height` will also be scaled to maintain
    /// the default proportion (Line Height ~ 1.43x).
    ///
    /// # Arguments
    ///
    /// * `size` - The font size in pixels
    /// * `auto_adjust_line_height` - Whether to automatically adjust the line height
    pub fn set_font_size(&mut self, size: f32, auto_adjust_line_height: bool) {
        self.font_size = size;
        self.recalculate_char_dimensions(auto_adjust_line_height);
    }

    /// Recalculates character dimensions based on current font and size.
    fn recalculate_char_dimensions(&mut self, auto_adjust_line_height: bool) {
        self.char_width
            .set(self.measure_single_char_width("a"));
        // Use '汉' as a standard reference for CJK (Chinese, Japanese, Korean) wide characters
        self.full_char_width
            .set(self.measure_single_char_width("汉"));

        // Fallback for infinite width measurements
        if self.char_width.get().is_infinite() {
            self.char_width
                .set(self.font_size / 2.0); // Rough estimate for monospace
        }

        if self.full_char_width.get().is_infinite() {
            self.full_char_width.set(self.font_size);
        }

        if auto_adjust_line_height {
            let line_height_ratio = LINE_HEIGHT / FONT_SIZE;
            self.line_height = self.font_size * line_height_ratio;
        }

        self.content_cache.clear();
        self.overlay_cache.clear();
        *self.max_content_width_cache.borrow_mut() = None;
    }

    /// Re-verifies the character-width measurement against the live font
    /// system.
    ///
    /// A `CodeEditor` is often constructed before iced has finished loading
    /// the embedded fonts (e.g. when an app restores its session state during
    /// startup). The initial `char_width` is then measured with a fallback
    /// font, so every token position, wrap point and cursor offset — computed
    /// from that width — is wrong once the real font renders, which shows up
    /// as misaligned glyphs (punctuation tokens visibly "warped" or
    /// overlapping while colors/highlighting stay correct).
    ///
    /// This is a self-healing check. A measurement happens:
    ///
    /// - every frame while the editor is young (the fonts are registered by
    ///   the host at startup, so they are guaranteed to have loaded within
    ///   `FONT_METRICS_SETTLE_DURATION`), and
    /// - whenever the editor is drawn again after a `FONT_METRICS_REVISIT_GAP`
    ///   pause (e.g. a session-restored tab that stayed hidden and only
    ///   rendered for the first time long after startup).
    ///
    /// The moment the measured width differs from the cached one, the real
    /// font has been registered: the metrics are refreshed and all geometry
    /// caches dropped so the next frame lays the text out correctly, and the
    /// check is disabled for good. While no change is seen, the only ongoing
    /// cost outside the young window is the single comparison above — a
    /// continuously drawn editor never re-measures after it has settled.
    fn ensure_font_metrics(&self) {
        if self.font_metrics_updated.get() {
            return;
        }
        let now = Instant::now();
        let young =
            now.duration_since(self.created_at) <= FONT_METRICS_SETTLE_DURATION;
        let revisit =
            now.duration_since(self.last_draw_at.get()) > FONT_METRICS_REVISIT_GAP;
        self.last_draw_at.set(now);
        if !young && !revisit {
            return;
        }
        let measured = self.measure_single_char_width("a");
        if (measured - self.char_width.get()).abs() > FONT_METRICS_EPSILON {
            self.char_width.set(measured);
            self.full_char_width
                .set(self.measure_single_char_width("汉"));
            self.font_metrics_updated.set(true);
            self.content_cache.clear();
            self.overlay_cache.clear();
            *self.max_content_width_cache.borrow_mut() = None;
        }
    }

    /// Measures the width of a single character string using the current font settings.
    fn measure_single_char_width(&self, content: &str) -> f32 {
        let text = Text {
            content,
            font: self.font,
            size: iced::Pixels(self.font_size),
            line_height: iced::advanced::text::LineHeight::default(),
            bounds: iced::Size::new(f32::INFINITY, f32::INFINITY),
            align_x: Alignment::Left,
            align_y: iced::alignment::Vertical::Top,
            shaping: iced::advanced::text::Shaping::Advanced,
            wrapping: iced::advanced::text::Wrapping::default(),
        };
        let p = <iced::Renderer as TextRenderer>::Paragraph::with_text(text);
        p.min_width()
    }

    /// Returns the current font size.
    ///
    /// # Returns
    ///
    /// The font size in pixels
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Returns the width of a standard narrow character in pixels.
    ///
    /// # Returns
    ///
    /// The character width in pixels
    pub fn char_width(&self) -> f32 {
        self.char_width.get()
    }

    /// Returns the width of a wide character (e.g. CJK) in pixels.
    ///
    /// # Returns
    ///
    /// The full character width in pixels
    pub fn full_char_width(&self) -> f32 {
        self.full_char_width.get()
    }

    /// Measures the rendered width for a given text snippet using editor metrics.
    pub fn measure_text_width(&self, text: &str) -> f32 {
        measure_text_width(text, self.full_char_width.get(), self.char_width.get())
    }

    /// Sets the line height used by the editor
    ///
    /// # Arguments
    ///
    /// * `height` - The line height in pixels
    pub fn set_line_height(&mut self, height: f32) {
        self.line_height = height;
        self.content_cache.clear();
        self.overlay_cache.clear();
    }

    /// Returns the current line height.
    ///
    /// # Returns
    ///
    /// The line height in pixels
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Returns the current viewport height in pixels.
    pub fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    /// Returns the current viewport width in pixels.
    pub fn viewport_width(&self) -> f32 {
        self.viewport_width
    }

    /// Returns the current vertical scroll offset in pixels.
    pub fn viewport_scroll(&self) -> f32 {
        self.viewport_scroll
    }

    /// Returns the current text content as a string.
    ///
    /// # Returns
    ///
    /// The complete text content of the editor
    pub fn content(&self) -> String {
        self.buffer.to_string()
    }

    /// Enables or disables Vim behavior for this editor instance.
    ///
    /// Changing this setting enters a clean Normal mode without modifying the
    /// buffer or command history.
    pub fn set_vim_enabled(&mut self, enabled: bool) {
        if self.is_grouping {
            self.history.end_group();
            self.is_grouping = false;
        }
        self.vim_enabled = enabled;
        self.vim_state.enter_clean_normal_mode();
        self.cursors.remove_all_but_primary();
        let position = if enabled {
            self.vim_normal_position(self.cursors.primary_position())
        } else {
            self.cursors.primary_position()
        };
        self.cursors.set_single(position);
        self.is_dragging = false;
        self.overlay_cache.clear();
    }

    /// Sets whether Vim behavior is enabled using the builder pattern.
    #[must_use]
    pub fn with_vim_enabled(mut self, enabled: bool) -> Self {
        self.set_vim_enabled(enabled);
        self
    }

    /// Returns whether Vim behavior is enabled for this editor instance.
    pub fn vim_enabled(&self) -> bool {
        self.vim_enabled
    }

    /// Returns the active Vim mode, or `None` when Vim behavior is disabled.
    pub fn vim_mode(&self) -> Option<VimMode> {
        self.vim_enabled.then(|| self.vim_state.mode())
    }

    /// Sets the viewport height for the editor.
    ///
    /// This determines the minimum height of the canvas, ensuring proper
    /// background rendering even when content is smaller than the viewport.
    ///
    /// # Arguments
    ///
    /// * `height` - The viewport height in pixels
    ///
    /// # Returns
    ///
    /// Self for method chaining
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs")
    ///     .with_viewport_height(500.0);
    /// ```
    #[must_use]
    pub fn with_viewport_height(mut self, height: f32) -> Self {
        self.viewport_height = height;
        self
    }

    /// Sets the theme style for the editor.
    ///
    /// # Arguments
    ///
    /// * `style` - The style to apply to the editor
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::{CodeEditor, theme};
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_theme(theme::from_iced_theme(&iced::Theme::TokyoNightStorm));
    /// ```
    pub fn set_theme(&mut self, style: Style) {
        self.style = style;
        self.content_cache.clear();
        self.overlay_cache.clear();
    }

    /// Sets the language for UI translations.
    ///
    /// This changes the language used for all UI text elements in the editor,
    /// including search dialog tooltips, placeholders, and labels.
    ///
    /// # Arguments
    ///
    /// * `language` - The language to use for UI text
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::{CodeEditor, Language};
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_language(Language::French);
    /// ```
    pub fn set_language(&mut self, language: crate::i18n::Language) {
        self.translations.set_language(language);
        self.overlay_cache.clear();
    }

    /// Returns the current UI language.
    ///
    /// # Returns
    ///
    /// The currently active language for UI text
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::{CodeEditor, Language};
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs");
    /// let current_lang = editor.language();
    /// ```
    pub fn language(&self) -> crate::i18n::Language {
        self.translations.language()
    }

    /// Attaches an LSP client and opens a document for the current buffer.
    ///
    /// This sends an initial `did_open` with the current buffer contents and
    /// resets any pending LSP change state.
    ///
    /// # Arguments
    ///
    /// * `client` - The LSP client to notify
    /// * `document` - Document metadata describing the buffer
    pub fn attach_lsp(
        &mut self,
        mut client: Box<dyn lsp::LspClient>,
        mut document: lsp::LspDocument,
    ) {
        if !self.lsp_enabled {
            return;
        }
        document.version = 1;
        let text = self.buffer.to_string();
        client.did_open(&document, &text);
        self.lsp_client = Some(client);
        self.lsp_document = Some(document);
        self.lsp_shadow_text = text;
        self.lsp_shadow_is_current = true;
        self.update_lsp_synced_extent();
        self.lsp_edit_snapshot = None;
        self.lsp_pending_changes.clear();
    }

    /// Opens a new document on the attached LSP client.
    ///
    /// If a document is already open, this will close it before opening the new
    /// one and reset pending change tracking.
    ///
    /// # Arguments
    ///
    /// * `document` - Document metadata describing the buffer
    pub fn lsp_open_document(&mut self, mut document: lsp::LspDocument) {
        let Some(client) = self.lsp_client.as_mut() else { return };
        if let Some(current) = self.lsp_document.as_ref() {
            client.did_close(current);
        }
        document.version = 1;
        let text = self.buffer.to_string();
        client.did_open(&document, &text);
        self.lsp_document = Some(document);
        self.lsp_shadow_text = text;
        self.lsp_shadow_is_current = true;
        self.update_lsp_synced_extent();
        self.lsp_edit_snapshot = None;
        self.lsp_pending_changes.clear();
    }

    /// Detaches the current LSP client and closes any open document.
    ///
    /// This clears all LSP-related state on the editor instance.
    pub fn detach_lsp(&mut self) {
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            client.did_close(document);
        }
        self.lsp_client = None;
        self.lsp_document = None;
        self.lsp_shadow_text = String::new();
        self.lsp_shadow_is_current = true;
        self.lsp_synced_line_count = 1;
        self.lsp_synced_last_line_len = 0;
        self.lsp_edit_snapshot = None;
        self.lsp_pending_changes.clear();
    }

    /// Sends a `did_save` notification with the current buffer contents.
    pub fn lsp_did_save(&mut self) {
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            let text = self.buffer.to_string();
            client.did_save(document, &text);
        }
    }

    /// Requests hover information at the current cursor position.
    pub fn lsp_request_hover(&mut self) {
        let position = self.lsp_position_from_cursor();
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            client.request_hover(document, position);
        }
    }

    /// Requests hover information at a canvas point.
    ///
    /// Returns `true` if the point maps to a valid buffer position and the
    /// request was sent.
    pub fn lsp_request_hover_at(&mut self, point: iced::Point) -> bool {
        let Some(position) = self.lsp_position_from_point(point) else {
            return false;
        };
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            client.request_hover(document, position);
            return true;
        }
        false
    }

    /// Requests hover information at an explicit LSP position.
    ///
    /// Returns `true` if an LSP client is attached and the request was sent.
    pub fn lsp_request_hover_at_position(
        &mut self,
        position: lsp::LspPosition,
    ) -> bool {
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            client.request_hover(document, position);
            return true;
        }
        false
    }

    /// Converts a canvas point to an LSP position, if possible.
    pub fn lsp_position_at_point(
        &self,
        point: iced::Point,
    ) -> Option<lsp::LspPosition> {
        self.lsp_position_from_point(point)
    }

    /// Returns the hover anchor position and its canvas point for a given
    /// cursor location.
    ///
    /// The anchor is the start of the word under the cursor, which is useful
    /// for LSP hover and definition requests.
    pub fn lsp_hover_anchor_at_point(
        &self,
        point: iced::Point,
    ) -> Option<(lsp::LspPosition, iced::Point)> {
        let (line, col) = self.calculate_cursor_from_point(point)?;
        let line_content = self.buffer.line(line);
        let anchor_col = Self::word_start_in_line(line_content, col);
        let anchor_point =
            self.point_from_position(line, anchor_col).unwrap_or(point);
        let line = u32::try_from(line).unwrap_or(u32::MAX);
        let character = u32::try_from(anchor_col).unwrap_or(u32::MAX);
        Some((lsp::LspPosition { line, character }, anchor_point))
    }

    /// Requests completion items at the current cursor position.
    pub fn lsp_request_completion(&mut self) {
        let position = self.lsp_position_from_cursor();
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            client.request_completion(document, position);
        }
    }

    /// Flushes pending LSP text changes to the attached client.
    ///
    /// This increments the document version and sends `did_change` with all
    /// queued changes.
    pub fn lsp_flush_pending_changes(&mut self) {
        if self.lsp_pending_changes.is_empty() {
            return;
        }

        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_mut())
        {
            let changes = std::mem::take(&mut self.lsp_pending_changes);
            document.version = document.version.saturating_add(1);
            client.did_change(document, &changes);
        }
    }

    /// Sets whether LSP changes are flushed automatically after edits.
    pub fn set_lsp_auto_flush(&mut self, auto_flush: bool) {
        self.lsp_auto_flush = auto_flush;
    }

    /// Requests focus for this editor.
    ///
    /// This method programmatically sets the focus to this editor instance,
    /// allowing it to receive keyboard events. Other editors will automatically
    /// lose focus.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor1 = CodeEditor::new("fn main() {}", "rs");
    /// let mut editor2 = CodeEditor::new("fn test() {}", "rs");
    ///
    /// // Give focus to editor2
    /// editor2.request_focus();
    /// ```
    pub fn request_focus(&self) {
        FOCUSED_EDITOR_ID.store(self.editor_id, Ordering::Relaxed);
    }

    /// Checks if this editor currently has focus.
    ///
    /// Returns `true` if this editor will receive keyboard events,
    /// `false` otherwise.
    ///
    /// # Returns
    ///
    /// `true` if focused, `false` otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs");
    /// if editor.is_focused() {
    ///     println!("Editor has focus");
    /// }
    /// ```
    pub fn is_focused(&self) -> bool {
        FOCUSED_EDITOR_ID.load(Ordering::Relaxed) == self.editor_id
    }

    /// Whether this editor currently receives keyboard input (Iced focus +
    /// internal canvas focus, and not focus-locked).
    pub fn has_keyboard_focus(&self) -> bool {
        self.has_focus()
    }

    /// Forces a full re-render on the next frame, discarding any cached
    /// content/overlay geometry and the cached visible-line window. Content,
    /// cursor, selection, and undo history are untouched — this only affects
    /// what gets *redrawn*, not what's in the buffer.
    ///
    /// Call this after restoring an editor's content from persisted state
    /// (a fresh app launch, or reading a saved session/collection off disk):
    /// a newly-constructed editor's viewport metrics start out unset and
    /// only ever get corrected by a real `Scrolled` event, so without ever
    /// having been scrolled at least once, it can render blank, with a
    /// stale line window, or with glyph-spacing artifacts until the user
    /// happens to scroll it — this clears the stale cached state up front
    /// instead of waiting on that.
    pub fn invalidate_render_cache(&mut self) {
        self.content_cache.clear();
        self.overlay_cache.clear();
        self.cache_window_start_line = 0;
        self.cache_window_end_line = 0;
    }

    /// Resets the editor with new content.
    ///
    /// This method replaces the buffer content and resets all editor state
    /// (cursor position, selection, scroll, history) to initial values.
    /// Use this instead of creating a new `CodeEditor` instance to ensure
    /// proper widget tree updates in iced.
    ///
    /// Returns a `Task` that scrolls the editor to the top, which also
    /// forces a redraw of the canvas.
    ///
    /// # Arguments
    ///
    /// * `content` - The new text content
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that should be returned from your update function
    ///
    /// # Example
    ///
    /// ```ignore
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("initial content", "lua");
    /// // Later, reset with new content and get the task
    /// let task = editor.reset("new content");
    /// // Return task.map(YourMessage::Editor) from your update function
    /// ```
    pub fn reset(&mut self, content: &str) -> iced::Task<Message> {
        self.buffer = TextBuffer::new(content);
        self.cursors.set_single((0, 0));
        self.vim_state.reset();
        self.horizontal_scroll_offset = 0.0;
        self.is_dragging = false;
        self.viewport_scroll = 0.0;
        self.history = CommandHistory::new(100);
        self.is_grouping = false;
        self.last_blink = Instant::now();
        self.cursor_visible = true;
        self.content_cache = canvas::Cache::default();
        self.overlay_cache = canvas::Cache::default();
        self.buffer_revision = self.buffer_revision.wrapping_add(1);
        *self.visual_lines_cache.borrow_mut() = None;
        // The buffer is fully replaced, so discard the whole highlight prefix.
        self.pre_edit_line = 0;
        self.pre_edit_last_line = usize::MAX;
        self.invalidate_highlight_from(0);
        self.enqueue_lsp_change();

        // Scroll to top to force a redraw
        snap_to(self.scrollable_id.clone(), RelativeOffset::START)
    }

    /// Resets the cursor blink animation.
    pub(crate) fn reset_cursor_blink(&mut self) {
        self.last_blink = Instant::now();
        self.cursor_visible = true;
    }

    /// Converts the current cursor position into an LSP position.
    fn lsp_position_from_cursor(&self) -> lsp::LspPosition {
        let pos = self.cursors.primary_position();
        let line = u32::try_from(pos.0).unwrap_or(u32::MAX);
        let character = u32::try_from(pos.1).unwrap_or(u32::MAX);
        lsp::LspPosition { line, character }
    }

    /// Converts a canvas point into an LSP position, if it hits the buffer.
    fn lsp_position_from_point(
        &self,
        point: iced::Point,
    ) -> Option<lsp::LspPosition> {
        let (line, col) = self.calculate_cursor_from_point(point)?;
        let line = u32::try_from(line).unwrap_or(u32::MAX);
        let character = u32::try_from(col).unwrap_or(u32::MAX);
        Some(lsp::LspPosition { line, character })
    }

    /// Converts a logical buffer position into a canvas point, if visible.
    fn point_from_position(
        &self,
        line: usize,
        col: usize,
    ) -> Option<iced::Point> {
        let visual_lines = self.visual_lines_cached(self.viewport_width);
        let visual_index = wrapping::WrappingCalculator::logical_to_visual(
            &visual_lines,
            line,
            col,
        )?;
        let visual_line = &visual_lines[visual_index];
        let line_content = self.buffer.line(visual_line.logical_line);
        let prefix_len = col.saturating_sub(visual_line.start_col);
        let prefix_text: String = line_content
            .chars()
            .skip(visual_line.start_col)
            .take(prefix_len)
            .collect();
        let x = self.gutter_width()
            + 5.0
            + measure_text_width(
                &prefix_text,
                self.full_char_width.get(),
                self.char_width.get(),
            );
        let y = visual_index as f32 * self.line_height;
        Some(iced::Point::new(x, y))
    }

    /// Returns the word-start column in a line for a given column.
    ///
    /// Word characters include ASCII alphanumerics and underscore.
    pub(crate) fn word_start_in_line(line: &str, col: usize) -> usize {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return 0;
        }
        let mut idx = col.min(chars.len());
        if idx == chars.len() {
            idx = idx.saturating_sub(1);
        }
        if !Self::is_word_char(chars[idx]) {
            if idx > 0 && Self::is_word_char(chars[idx - 1]) {
                idx -= 1;
            } else {
                return col.min(chars.len());
            }
        }
        while idx > 0 && Self::is_word_char(chars[idx - 1]) {
            idx -= 1;
        }
        idx
    }

    /// Returns the word-end column in a line for a given column.
    pub(crate) fn word_end_in_line(line: &str, col: usize) -> usize {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return 0;
        }
        let mut idx = col.min(chars.len());
        if idx == chars.len() {
            idx = idx.saturating_sub(1);
        }

        // If current char is not a word char, check if previous was (we might be just after the word)
        if !Self::is_word_char(chars[idx]) {
            if idx > 0 && Self::is_word_char(chars[idx - 1]) {
                // We are just after a word, so idx is the end (exclusive)
                // But wait, if we are at the space after "foo", idx points to space.
                // "foo " -> ' ' is at 3. word_end should be 3.
                // So if chars[idx] is not word char, and chars[idx-1] IS, then idx is the end.
                return idx;
            } else {
                // Not on a word
                return col.min(chars.len());
            }
        }

        // If we are on a word char, scan forward
        while idx < chars.len() && Self::is_word_char(chars[idx]) {
            idx += 1;
        }
        idx
    }

    /// Returns true when the character is part of an identifier-style word.
    pub(crate) fn is_word_char(ch: char) -> bool {
        ch == '_' || ch.is_alphanumeric()
    }

    /// Computes and queues the latest LSP text change for the buffer.
    ///
    /// When auto-flush is enabled, this immediately sends changes.
    fn enqueue_lsp_change(&mut self) {
        if self.lsp_document.is_none() {
            return;
        }

        let new_text = self.buffer.to_string();
        let change = if self.lsp_shadow_is_current {
            lsp::compute_text_change(&self.lsp_shadow_text, &new_text)
        } else {
            let end_line = self.lsp_synced_line_count.saturating_sub(1);
            Some(lsp::LspTextChange {
                range: lsp::LspRange {
                    start: lsp::LspPosition { line: 0, character: 0 },
                    end: lsp::LspPosition {
                        line: u32::try_from(end_line).unwrap_or(u32::MAX),
                        character: u32::try_from(self.lsp_synced_last_line_len)
                            .unwrap_or(u32::MAX),
                    },
                },
                text: new_text.clone(),
            })
        };
        if let Some(change) = change {
            self.lsp_pending_changes.push(change);
        }
        self.lsp_shadow_text = new_text;
        self.lsp_shadow_is_current = true;
        self.update_lsp_synced_extent();
        if self.lsp_auto_flush {
            self.lsp_flush_pending_changes();
        }
    }

    /// Queues the bounded range replacement captured before a normal editor
    /// command. Unlike `enqueue_lsp_change`, this never serializes or diffs the
    /// complete document.
    pub(crate) fn enqueue_incremental_lsp_change(&mut self) {
        if self.lsp_document.is_none() {
            self.lsp_edit_snapshot = None;
            return;
        }

        let Some(snapshot) = self.lsp_edit_snapshot.take() else {
            self.enqueue_lsp_change();
            return;
        };

        let new_line_count = self.buffer.line_count();
        let start_line =
            snapshot.start_line.min(new_line_count.saturating_sub(1));
        let new_end_exclusive = if new_line_count >= snapshot.old_line_count {
            snapshot
                .old_end_exclusive
                .saturating_add(new_line_count - snapshot.old_line_count)
                .min(new_line_count)
        } else {
            snapshot
                .old_end_exclusive
                .saturating_sub(snapshot.old_line_count - new_line_count)
                .max(start_line.saturating_add(1))
                .min(new_line_count)
        };
        let text =
            self.buffer.line_range_to_string(start_line, new_end_exclusive);
        self.lsp_pending_changes.push(lsp::LspTextChange {
            range: lsp::LspRange {
                start: lsp::LspPosition {
                    line: u32::try_from(snapshot.start_line)
                        .unwrap_or(u32::MAX),
                    character: 0,
                },
                end: snapshot.old_end,
            },
            text,
        });

        // The shadow string is intentionally not rewritten here: doing so
        // would reintroduce an O(document size) copy. The compact extent below
        // is sufficient for a rare future full-document fallback.
        self.lsp_shadow_text = String::new();
        self.lsp_shadow_is_current = false;
        self.update_lsp_synced_extent();
        if self.lsp_auto_flush {
            self.lsp_flush_pending_changes();
        }
    }

    /// Updates the compact extent of the document state represented by queued
    /// and already-flushed LSP changes.
    fn update_lsp_synced_extent(&mut self) {
        self.lsp_synced_line_count = self.buffer.line_count();
        self.lsp_synced_last_line_len =
            self.buffer.line_len(self.lsp_synced_line_count.saturating_sub(1));
    }

    /// Refreshes search matches after buffer modification.
    ///
    /// Should be called after any operation that modifies the buffer.
    /// If search is active, recalculates only the affected logical lines and
    /// selects the match closest to the current cursor position.
    pub(crate) fn refresh_search_matches_if_needed(&mut self) {
        if self.search_matches_visible() && !self.search_state.query.is_empty()
        {
            let start_line = self.pre_edit_line.saturating_sub(1);
            let old_end_exclusive = self.pre_edit_last_line.saturating_add(2);
            self.search_state.update_matches_after_edit(
                &self.buffer,
                start_line,
                old_end_exclusive,
            );

            // Select match closest to cursor to maintain context
            self.search_state
                .select_match_near_cursor(self.cursors.primary_position());
        }
    }

    pub(crate) fn search_matches_visible(&self) -> bool {
        self.search_state.is_open
            || (self.vim_enabled && self.vim_state.last_search().is_some())
    }

    /// Returns whether the editor has unsaved changes.
    ///
    /// # Returns
    ///
    /// `true` if there are unsaved modifications, `false` otherwise
    pub fn is_modified(&self) -> bool {
        self.history.is_modified()
    }

    /// Marks the current state as saved.
    ///
    /// Call this after successfully saving the file to reset the modified state.
    pub fn mark_saved(&mut self) {
        self.history.mark_saved();
    }

    /// Returns whether undo is available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Returns whether redo is available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Sets whether line wrapping is enabled.
    ///
    /// When enabled, long lines will wrap at the viewport width or at a
    /// configured column width.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable line wrapping
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_wrap_enabled(false); // Disable wrapping
    /// ```
    pub fn set_wrap_enabled(&mut self, enabled: bool) {
        if self.wrap_enabled != enabled {
            self.wrap_enabled = enabled;
            if enabled {
                self.horizontal_scroll_offset = 0.0;
            }
            self.content_cache.clear();
            self.overlay_cache.clear();
        }
    }

    /// Returns whether line wrapping is enabled.
    ///
    /// # Returns
    ///
    /// `true` if line wrapping is enabled, `false` otherwise
    pub fn wrap_enabled(&self) -> bool {
        self.wrap_enabled
    }

    /// Enables or disables visible whitespace rendering.
    ///
    /// When enabled, space characters are rendered as `·` and tab characters
    /// as `→`, both drawn in a dimmed color to remain non-intrusive. Toggling
    /// this setting clears the content cache to trigger an immediate redraw.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to show whitespace characters
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_show_whitespace(true);
    /// ```
    pub fn set_show_whitespace(&mut self, enabled: bool) {
        if self.show_whitespace != enabled {
            self.show_whitespace = enabled;
            self.content_cache.clear();
        }
    }

    /// Returns whether visible whitespace rendering is enabled.
    pub fn show_whitespace(&self) -> bool {
        self.show_whitespace
    }

    /// Enables or disables code folding (collapse/expand blocks).
    ///
    /// When disabled, no fold chevrons are drawn and all lines are shown
    /// regardless of the collapsed state (which is preserved, so re-enabling
    /// restores the previously collapsed blocks).
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable code folding
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_folding_enabled(true);
    /// ```
    pub fn set_folding_enabled(&mut self, enabled: bool) {
        if self.folding_enabled != enabled {
            self.folding_enabled = enabled;
            self.bump_fold_revision();
        }
    }

    /// Returns whether code folding is enabled.
    pub fn folding_enabled(&self) -> bool {
        self.folding_enabled
    }

    /// Returns whether the region whose header is `header_line` is collapsed.
    pub fn is_folded(&self, header_line: usize) -> bool {
        self.collapsed_folds.contains(&header_line)
    }

    /// Toggles the collapsed state of the foldable region whose header is
    /// `header_line`.
    ///
    /// The call is a no-op if `header_line` is not currently a fold header.
    /// When collapsing, any cursor that would land on a hidden line is moved up
    /// to the header line so the caret stays visible.
    ///
    /// # Arguments
    ///
    /// * `header_line` - Logical line index of the region header
    pub fn toggle_fold(&mut self, header_line: usize) {
        let regions = self.foldable_regions();
        if !folding::is_fold_header(&regions, header_line) {
            return; // Not a fold header: nothing to toggle.
        }

        if self.collapsed_folds.contains(&header_line) {
            self.collapsed_folds.remove(&header_line);
        } else {
            self.collapsed_folds.insert(header_line);
        }
        self.after_fold_change();
    }

    /// Toggles the collapsed state of the innermost foldable region containing
    /// `line`.
    ///
    /// Folds the region if it is expanded, unfolds it if it is collapsed. Does
    /// nothing if `line` is not inside any foldable region. This is the
    /// cursor-driven primitive used by the keyboard shortcut and mirrors a click
    /// on the fold chevron.
    ///
    /// # Arguments
    ///
    /// * `line` - A logical line inside (or heading) the region to toggle
    pub fn toggle_fold_at(&mut self, line: usize) {
        let regions = self.foldable_regions();
        let header = regions
            .iter()
            .filter(|r| r.start_line <= line && line <= r.end_line)
            .map(|r| r.start_line)
            .max();
        if let Some(header) = header {
            if self.collapsed_folds.contains(&header) {
                self.collapsed_folds.remove(&header);
            } else {
                self.collapsed_folds.insert(header);
            }
            self.after_fold_change();
        }
    }

    /// Folds the innermost foldable region containing `line`.
    ///
    /// Does nothing if `line` is not inside any foldable region or the region
    /// is already collapsed. This is the cursor-driven counterpart to
    /// [`Self::toggle_fold`].
    ///
    /// # Arguments
    ///
    /// * `line` - A logical line inside (or heading) the region to fold
    pub fn fold_at(&mut self, line: usize) {
        let regions = self.foldable_regions();
        // Innermost containing region: the one with the greatest start line.
        let header = regions
            .iter()
            .filter(|r| r.start_line <= line && line <= r.end_line)
            .map(|r| r.start_line)
            .max();
        if let Some(header) = header
            && self.collapsed_folds.insert(header)
        {
            self.after_fold_change();
        }
    }

    /// Unfolds the innermost collapsed region containing `line`.
    ///
    /// Does nothing if no collapsed region contains `line`.
    ///
    /// # Arguments
    ///
    /// * `line` - A logical line inside (or heading) the region to unfold
    pub fn unfold_at(&mut self, line: usize) {
        let regions = self.foldable_regions();
        let header = regions
            .iter()
            .filter(|r| {
                r.start_line <= line
                    && line <= r.end_line
                    && self.collapsed_folds.contains(&r.start_line)
            })
            .map(|r| r.start_line)
            .max();
        if let Some(header) = header
            && self.collapsed_folds.remove(&header)
        {
            self.after_fold_change();
        }
    }

    /// Folds every foldable block in the buffer.
    pub fn fold_all(&mut self) {
        let regions = self.foldable_regions();
        let mut changed = false;
        for region in regions.iter() {
            changed |= self.collapsed_folds.insert(region.start_line);
        }
        if changed {
            self.after_fold_change();
        }
    }

    /// Unfolds every collapsed block in the buffer.
    pub fn unfold_all(&mut self) {
        if !self.collapsed_folds.is_empty() {
            self.collapsed_folds.clear();
            self.after_fold_change();
        }
    }

    /// Finalizes a change to the collapsed set: keeps every cursor on a visible
    /// line and invalidates fold-dependent caches.
    fn after_fold_change(&mut self) {
        let hidden = self.hidden_lines_set();
        self.move_cursors_out_of_hidden(&hidden);
        self.bump_fold_revision();
    }

    /// Moves any cursor sitting on a hidden line up to the nearest visible line
    /// above it (the header of the enclosing collapsed block).
    fn move_cursors_out_of_hidden(&mut self, hidden: &HashSet<usize>) {
        if hidden.is_empty() {
            return;
        }
        for cursor in self.cursors.as_mut_slice() {
            let mut line = cursor.position.0;
            while line > 0 && hidden.contains(&line) {
                line -= 1;
            }
            if line != cursor.position.0 {
                cursor.position = (line, 0);
            }
        }
        self.cursors.sort_and_merge();
    }

    /// Invalidates fold-dependent caches after a fold-state change.
    fn bump_fold_revision(&mut self) {
        self.fold_revision = self.fold_revision.wrapping_add(1);
        self.content_cache.clear();
        self.overlay_cache.clear();
    }

    /// Returns the foldable regions for the current buffer, memoized by
    /// `buffer_revision`.
    ///
    /// Returns an empty list when folding is disabled.
    pub(crate) fn foldable_regions(&self) -> Rc<Vec<folding::FoldRegion>> {
        if !self.folding_enabled {
            return Rc::new(Vec::new());
        }

        let mut cache = self.foldable_regions_cache.borrow_mut();
        if let Some((revision, regions)) = cache.as_ref()
            && *revision == self.buffer_revision
        {
            return regions.clone();
        }

        let regions = Rc::new(folding::compute_foldable_regions(&self.buffer));
        *cache = Some((self.buffer_revision, regions.clone()));
        regions
    }

    /// Returns the set of logical lines hidden by the currently collapsed folds.
    ///
    /// Empty when folding is disabled or nothing is collapsed.
    pub(crate) fn hidden_lines_set(&self) -> HashSet<usize> {
        if !self.folding_enabled || self.collapsed_folds.is_empty() {
            return HashSet::new();
        }
        let regions = self.foldable_regions();
        folding::hidden_lines(&regions, &self.collapsed_folds)
    }

    /// Enables or disables automatic indentation on Enter.
    ///
    /// When enabled, pressing Enter copies the leading whitespace of the
    /// current line to the new line. When disabled, the cursor is placed
    /// at column 0 on the new line.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable auto-indentation, `false` to disable
    pub fn set_auto_indent_enabled(&mut self, enabled: bool) {
        self.auto_indent_enabled = enabled;
    }

    /// Returns whether auto-indentation is enabled.
    ///
    /// # Returns
    ///
    /// `true` if auto-indentation is enabled, `false` otherwise
    pub fn auto_indent_enabled(&self) -> bool {
        self.auto_indent_enabled
    }

    /// Sets the indentation style used when pressing the Tab key.
    ///
    /// # Arguments
    ///
    /// * `style` - The indentation style (`IndentStyle::Spaces(n)` or `IndentStyle::Tab`)
    pub fn set_indent_style(&mut self, style: IndentStyle) {
        self.indent_style = style;
    }

    /// Returns the current indentation style.
    ///
    /// # Returns
    ///
    /// The current [`IndentStyle`] configured for this editor
    pub fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    /// Enables or disables the search/replace functionality.
    ///
    /// When disabled, search/replace keyboard shortcuts (Ctrl+F, Ctrl+H, F3)
    /// will be ignored. If the search dialog is currently open, it will be closed.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable search/replace functionality
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_search_replace_enabled(false); // Disable search/replace
    /// ```
    pub fn set_search_replace_enabled(&mut self, enabled: bool) {
        self.search_replace_enabled = enabled;
        if !enabled && self.search_state.is_open {
            self.search_state.close();
        }
    }

    /// Returns whether search/replace functionality is enabled.
    ///
    /// # Returns
    ///
    /// `true` if search/replace is enabled, `false` otherwise
    pub fn search_replace_enabled(&self) -> bool {
        self.search_replace_enabled
    }

    /// Sets whether LSP support is enabled.
    ///
    /// When set to `false`, any attached LSP client is detached automatically.
    /// Calling [`attach_lsp`] while disabled is a no-op.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_lsp_enabled(false);
    /// ```
    ///
    /// [`attach_lsp`]: CodeEditor::attach_lsp
    pub fn set_lsp_enabled(&mut self, enabled: bool) {
        self.lsp_enabled = enabled;
        if !enabled {
            self.detach_lsp();
        }
    }

    /// Returns whether LSP support is enabled.
    ///
    /// `true` if LSP is enabled, `false` otherwise
    pub fn lsp_enabled(&self) -> bool {
        self.lsp_enabled
    }

    /// Returns the syntax highlighting language identifier for this editor.
    ///
    /// This is the language key passed at construction (e.g., `"lua"`, `"rs"`, `"py"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    /// let editor = CodeEditor::new("fn main() {}", "rs");
    /// assert_eq!(editor.syntax(), "rs");
    /// ```
    pub fn syntax(&self) -> &str {
        &self.syntax
    }

    /// Opens the search dialog programmatically.
    ///
    /// This is useful when wiring your own UI button instead of relying on
    /// keyboard shortcuts.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that focuses the search input.
    pub fn open_search_dialog(&mut self) -> iced::Task<Message> {
        self.update(&Message::OpenSearch)
    }

    /// Opens the search-and-replace dialog programmatically.
    ///
    /// This is useful when wiring your own UI button instead of relying on
    /// keyboard shortcuts.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` that focuses the search input.
    pub fn open_search_replace_dialog(&mut self) -> iced::Task<Message> {
        self.update(&Message::OpenSearchReplace)
    }

    /// Closes the search dialog programmatically.
    ///
    /// # Returns
    ///
    /// A `Task<Message>` for any follow-up UI work.
    pub fn close_search_dialog(&mut self) -> iced::Task<Message> {
        self.update(&Message::CloseSearch)
    }

    /// Opens the go-to-line dialog programmatically.
    ///
    /// The input is pre-filled with the current one-based line number.
    pub fn open_goto_line_dialog(&mut self) -> iced::Task<Message> {
        self.update(&Message::OpenGotoLine)
    }

    /// Closes the go-to-line dialog programmatically.
    pub fn close_goto_line_dialog(&mut self) -> iced::Task<Message> {
        self.update(&Message::CloseGotoLine)
    }

    /// Sets the line wrapping with builder pattern.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable line wrapping
    ///
    /// # Returns
    ///
    /// Self for method chaining
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs")
    ///     .with_wrap_enabled(false);
    /// ```
    #[must_use]
    pub fn with_wrap_enabled(mut self, enabled: bool) -> Self {
        self.wrap_enabled = enabled;
        self
    }

    /// Enables or disables code folding using the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable code folding
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs")
    ///     .with_folding_enabled(true);
    /// ```
    #[must_use]
    pub fn with_folding_enabled(mut self, enabled: bool) -> Self {
        self.folding_enabled = enabled;
        self
    }

    /// Sets the wrap column (fixed width wrapping).
    ///
    /// When set to `Some(n)`, lines will wrap at column `n`.
    /// When set to `None`, lines will wrap at the viewport width.
    ///
    /// # Arguments
    ///
    /// * `column` - The column to wrap at, or None for viewport-based wrapping
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs")
    ///     .with_wrap_column(Some(80)); // Wrap at 80 characters
    /// ```
    #[must_use]
    pub fn with_wrap_column(mut self, column: Option<usize>) -> Self {
        self.wrap_column = column;
        self
    }

    /// Sets whether line numbers are displayed.
    ///
    /// When disabled, the gutter is completely removed (0px width),
    /// providing more space for code display.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to display line numbers
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.set_line_numbers_enabled(false); // Hide line numbers
    /// ```
    pub fn set_line_numbers_enabled(&mut self, enabled: bool) {
        if self.line_numbers_enabled != enabled {
            self.line_numbers_enabled = enabled;
            self.content_cache.clear();
            self.overlay_cache.clear();
        }
    }

    /// Returns whether line numbers are displayed.
    ///
    /// # Returns
    ///
    /// `true` if line numbers are displayed, `false` otherwise
    pub fn line_numbers_enabled(&self) -> bool {
        self.line_numbers_enabled
    }

    /// Sets the line numbers display with builder pattern.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to display line numbers
    ///
    /// # Returns
    ///
    /// Self for method chaining
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs")
    ///     .with_line_numbers_enabled(false);
    /// ```
    #[must_use]
    pub fn with_line_numbers_enabled(mut self, enabled: bool) -> Self {
        self.line_numbers_enabled = enabled;
        self
    }

    /// Returns the total gutter width, including the line-number area and the
    /// fold margin.
    ///
    /// The fold margin is added when folding is enabled, independently of line
    /// numbers, so fold chevrons remain clickable even without line numbers.
    pub(crate) fn gutter_width(&self) -> f32 {
        self.line_number_gutter_width() + self.fold_margin_width()
    }

    /// Returns the width of the line-number area (excluding the fold margin).
    pub(crate) fn line_number_gutter_width(&self) -> f32 {
        if self.line_numbers_enabled { GUTTER_WIDTH } else { 0.0 }
    }

    /// Returns the width of the fold margin (the chevron column), or `0.0` when
    /// folding is disabled.
    pub(crate) fn fold_margin_width(&self) -> f32 {
        if self.folding_enabled { FOLD_MARGIN_WIDTH } else { 0.0 }
    }

    /// Removes canvas focus from this editor.
    ///
    /// This method programmatically removes focus from the canvas, preventing
    /// it from receiving keyboard events. The cursor will be hidden, but the
    /// selection will remain visible.
    ///
    /// Call this when focus should move to another widget (e.g., text input).
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.lose_focus();
    /// ```
    pub fn lose_focus(&mut self) {
        self.has_canvas_focus = false;
        self.show_cursor = false;
        self.ime_preedit = None;
    }

    /// Resets the focus lock state.
    ///
    /// This method can be called to manually unlock focus processing
    /// after a focus transition has completed. This is useful when
    /// you want to allow the editor to process input again after
    /// programmatic focus changes.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let mut editor = CodeEditor::new("fn main() {}", "rs");
    /// editor.reset_focus_lock();
    /// ```
    pub fn reset_focus_lock(&mut self) {
        self.focus_locked = false;
    }

    /// Returns the screen position of the cursor.
    ///
    /// This method returns the (x, y) coordinates of the current cursor position
    /// relative to the editor canvas, accounting for gutter width and line height.
    ///
    /// # Returns
    ///
    /// An `Option<iced::Point>` containing the cursor position, or `None` if
    /// the cursor position cannot be determined.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs");
    /// if let Some(point) = editor.cursor_screen_position() {
    ///     println!("Cursor at: ({}, {})", point.x, point.y);
    /// }
    /// ```
    pub fn cursor_screen_position(&self) -> Option<iced::Point> {
        let pos = self.cursors.primary_position();
        self.point_from_position(pos.0, pos.1)
    }

    /// Returns the current cursor position as (line, column).
    ///
    /// This method returns the logical cursor position in the buffer,
    /// where line and column are both 0-indexed.
    ///
    /// # Returns
    ///
    /// A tuple `(line, column)` representing the cursor position.
    ///
    /// # Example
    ///
    /// ```
    /// use iced_code_editor::CodeEditor;
    ///
    /// let editor = CodeEditor::new("fn main() {}", "rs");
    /// let (line, col) = editor.cursor_position();
    /// println!("Cursor at line {}, column {}", line, col);
    /// ```
    pub fn cursor_position(&self) -> (usize, usize) {
        self.cursors.primary_position()
    }

    /// Returns the maximum content width across all lines, in pixels.
    ///
    /// Used to size the horizontal scrollbar when `wrap_enabled = false`.
    /// The result is cached keyed by `buffer_revision` so repeated calls are cheap.
    ///
    /// # Returns
    ///
    /// Total width in pixels including gutter, padding and a right margin.
    pub(crate) fn max_content_width(&self) -> f32 {
        let mut cache = self.max_content_width_cache.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|existing| existing.revision != self.buffer_revision)
        {
            let line_widths: Vec<f32> = (0..self.buffer.line_count())
                .map(|line| {
                    measure_text_width(
                        self.buffer.line(line),
                        self.full_char_width.get(),
                        self.char_width.get(),
                    )
                })
                .collect();
            let mut width_counts = BTreeMap::new();
            for width in &line_widths {
                *width_counts.entry(width.to_bits()).or_insert(0) += 1;
            }
            *cache = Some(MaxContentWidthCache {
                revision: self.buffer_revision,
                line_widths,
                width_counts,
            });
        }

        let gutter = self.gutter_width();
        let max_line_width =
            cache.as_ref().map_or(0.0, MaxContentWidthCache::max_width);

        // gutter + left padding + text + right margin
        gutter + 5.0 + max_line_width + 20.0
    }

    /// Returns wrapped "visual lines" for the current buffer and layout, with memoization.
    ///
    /// The editor frequently needs the wrapped view of the buffer:
    /// - hit-testing (mouse selection, cursor placement)
    /// - mapping logical ↔ visual positions
    /// - rendering (text, line numbers, highlights)
    ///
    /// Computing visual lines is relatively expensive for large files, so we
    /// cache the result keyed by:
    /// - `buffer_revision` (buffer content changes)
    /// - viewport width / gutter width (layout changes)
    /// - wrapping settings (wrap enabled / wrap column)
    /// - measured character widths (font / size changes)
    ///
    /// The returned `Rc<Vec<VisualLine>>` is cheap to clone and allows multiple
    /// rendering passes (content + overlay layers) to share the same computed
    /// layout without extra allocation.
    pub(crate) fn visual_lines_cached(
        &self,
        viewport_width: f32,
    ) -> Rc<Vec<wrapping::VisualLine>> {
        let key = VisualLinesKey {
            buffer_revision: self.buffer_revision,
            viewport_width_bits: viewport_width.to_bits(),
            gutter_width_bits: self.gutter_width().to_bits(),
            wrap_enabled: self.wrap_enabled,
            wrap_column: self.wrap_column,
            folding_enabled: self.folding_enabled,
            fold_revision: self.fold_revision,
            full_char_width_bits: self.full_char_width.get().to_bits(),
            char_width_bits: self.char_width.get().to_bits(),
        };

        let mut cache = self.visual_lines_cache.borrow_mut();
        if let Some(existing) = cache.as_ref()
            && existing.key == key
        {
            return existing.visual_lines.clone();
        }

        let hidden = self.hidden_lines_set();
        let wrapping_calc = wrapping::WrappingCalculator::new(
            self.wrap_enabled,
            self.wrap_column,
            self.full_char_width.get(),
            self.char_width.get(),
        );
        let visual_lines = wrapping_calc.calculate_visual_lines(
            &self.buffer,
            viewport_width,
            self.gutter_width(),
            &hidden,
        );
        let visual_lines = Rc::new(visual_lines);

        *cache = Some(VisualLinesCache {
            key,
            visual_lines: visual_lines.clone(),
            buffer_line_count: self.buffer.line_count(),
        });
        visual_lines
    }

    /// Rebuilds only the logical-line slice affected by the latest edit.
    ///
    /// The common typing path changes one line. Reusing the unchanged prefix
    /// and suffix prevents wrapping work from scaling with total file size.
    /// Collapsed folds intentionally fall back to a full rebuild because an
    /// indentation edit can change which distant lines are hidden.
    pub(crate) fn refresh_visual_lines_after_edit(
        &self,
        previous_revision: u64,
    ) {
        if !self.collapsed_folds.is_empty() {
            *self.visual_lines_cache.borrow_mut() = None;
            return;
        }

        let mut cache_guard = self.visual_lines_cache.borrow_mut();
        let Some(cache) = cache_guard.as_mut() else { return };
        if cache.key.buffer_revision != previous_revision {
            *cache_guard = None;
            return;
        }

        let same_layout = cache.key.gutter_width_bits
            == self.gutter_width().to_bits()
            && cache.key.wrap_enabled == self.wrap_enabled
            && cache.key.wrap_column == self.wrap_column
            && cache.key.folding_enabled == self.folding_enabled
            && cache.key.fold_revision == self.fold_revision
            && cache.key.full_char_width_bits == self.full_char_width.get().to_bits()
            && cache.key.char_width_bits == self.char_width.get().to_bits();
        if !same_layout {
            *cache_guard = None;
            return;
        }

        let old_line_count = cache.buffer_line_count;
        let new_line_count = self.buffer.line_count();
        let start_line =
            self.pre_edit_line.saturating_sub(1).min(old_line_count);
        let old_end_line =
            self.pre_edit_last_line.saturating_add(2).min(old_line_count);
        let new_end_line = if new_line_count >= old_line_count {
            old_end_line
                .saturating_add(new_line_count - old_line_count)
                .min(new_line_count)
        } else {
            old_end_line
                .saturating_sub(old_line_count - new_line_count)
                .max(start_line)
                .min(new_line_count)
        };

        let prefix_end = cache
            .visual_lines
            .partition_point(|visual| visual.logical_line < start_line);
        let suffix_start = cache
            .visual_lines
            .partition_point(|visual| visual.logical_line < old_end_line);

        let wrapping_calc = wrapping::WrappingCalculator::new(
            self.wrap_enabled,
            self.wrap_column,
            self.full_char_width.get(),
            self.char_width.get(),
        );
        let changed_visual_lines = wrapping_calc.calculate_visual_lines_range(
            &self.buffer,
            f32::from_bits(cache.key.viewport_width_bits),
            f32::from_bits(cache.key.gutter_width_bits),
            &HashSet::new(),
            start_line..new_end_line,
        );

        let old_segment_count = suffix_start.saturating_sub(prefix_end);
        let new_segment_count = changed_visual_lines.len();
        let visual_lines = Rc::make_mut(&mut cache.visual_lines);

        // The overwhelmingly common typing case keeps both the logical-line
        // count and the number of wrapped segments stable. Update that tiny
        // slice in place, without allocating or moving the rest of the file.
        if new_line_count == old_line_count
            && old_segment_count == new_segment_count
        {
            visual_lines[prefix_end..suffix_start]
                .clone_from_slice(&changed_visual_lines);
        } else {
            visual_lines.splice(prefix_end..suffix_start, changed_visual_lines);

            let shifted_suffix_start = prefix_end + new_segment_count;
            for visual in &mut visual_lines[shifted_suffix_start..] {
                visual.logical_line = if new_line_count >= old_line_count {
                    visual
                        .logical_line
                        .saturating_add(new_line_count - old_line_count)
                } else {
                    visual
                        .logical_line
                        .saturating_sub(old_line_count - new_line_count)
                };
            }
        }

        cache.key.buffer_revision = self.buffer_revision;
        cache.buffer_line_count = new_line_count;
    }

    /// Updates the horizontal-width index for the lines affected by an edit.
    ///
    /// This removes the final whole-file pass that used to happen after every
    /// keystroke when wrapping was disabled.
    pub(crate) fn refresh_max_content_width_after_edit(
        &self,
        previous_revision: u64,
    ) {
        let mut cache_guard = self.max_content_width_cache.borrow_mut();
        let Some(cache) = cache_guard.as_mut() else { return };
        if cache.revision != previous_revision {
            *cache_guard = None;
            return;
        }

        let old_line_count = cache.line_widths.len();
        let new_line_count = self.buffer.line_count();
        let start_line =
            self.pre_edit_line.saturating_sub(1).min(old_line_count);
        let old_end_line =
            self.pre_edit_last_line.saturating_add(2).min(old_line_count);
        if start_line == 0 && old_end_line == old_line_count {
            *cache_guard = None;
            return;
        }

        let new_end_line = if new_line_count >= old_line_count {
            old_end_line
                .saturating_add(new_line_count - old_line_count)
                .min(new_line_count)
        } else {
            old_end_line
                .saturating_sub(old_line_count - new_line_count)
                .max(start_line)
                .min(new_line_count)
        };
        let old_widths = cache.line_widths[start_line..old_end_line].to_vec();
        let new_widths: Vec<f32> = (start_line..new_end_line)
            .map(|line| {
                measure_text_width(
                    self.buffer.line(line),
                    self.full_char_width.get(),
                    self.char_width.get(),
                )
            })
            .collect();

        for width in old_widths {
            cache.remove_width(width);
        }
        for width in &new_widths {
            cache.add_width(*width);
        }
        cache.line_widths.splice(start_line..old_end_line, new_widths);
        cache.revision = self.buffer_revision;
    }

    /// Initiates a "Go to Definition" request for the symbol at the current cursor position.
    ///
    /// This method converts the current cursor coordinates into an LSP-compatible position
    /// and delegates the request to the active `LspClient`, if one is attached.
    pub fn lsp_request_definition(&mut self) {
        let position = self.lsp_position_from_cursor();
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            client.request_definition(document, position);
        }
    }

    /// Initiates a "Go to Definition" request for the symbol at the specified screen coordinates.
    ///
    /// This is typically used for mouse interactions (e.g., Ctrl+Click). It first resolves
    /// the screen coordinates to a text position and then sends the request.
    ///
    /// # Returns
    ///
    /// `true` if the request was successfully sent (i.e., a valid position was found and an LSP client is active),
    /// `false` otherwise.
    pub fn lsp_request_definition_at(&mut self, point: iced::Point) -> bool {
        let Some(position) = self.lsp_position_from_point(point) else {
            return false;
        };
        if let (Some(client), Some(document)) =
            (self.lsp_client.as_mut(), self.lsp_document.as_ref())
        {
            client.request_definition(document, position);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_custom_context_menu_configuration() {
        let custom_entries = vec![
            ContextMenuEntry::item("format", "Format document")
                .with_shortcut("Shift+Alt+F"),
            ContextMenuEntry::separator(),
            ContextMenuEntry::Item(
                ContextMenuItem::new("rename", "Rename symbol")
                    .with_enabled(false),
            ),
        ];

        let editor = CodeEditor::new("", "rs")
            .with_custom_context_menu_entries(custom_entries.clone())
            .with_default_context_menu_enabled(false);

        assert_eq!(editor.custom_context_menu_entries(), custom_entries);
        assert!(!editor.default_context_menu_enabled());

        let default_editor = CodeEditor::new("", "rs");
        assert!(default_editor.custom_context_menu_entries().is_empty());
        assert!(default_editor.default_context_menu_enabled());
    }

    #[test]
    fn test_reveal_in_file_manager_configuration() {
        let mut editor = CodeEditor::new("", "rs");
        assert!(!editor.reveal_in_file_manager_enabled());

        editor.set_reveal_in_file_manager_enabled(true);
        assert!(editor.reveal_in_file_manager_enabled());

        let editor =
            CodeEditor::new("", "rs").with_reveal_in_file_manager_enabled(true);
        assert!(editor.reveal_in_file_manager_enabled());
    }

    #[test]
    fn vim_disabled_by_default() {
        let editor = CodeEditor::new("unchanged", "rs");

        assert!(!editor.vim_enabled());
        assert_eq!(editor.vim_mode(), None);
        assert_eq!(editor.content(), "unchanged");
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn vim_enable_enters_clean_normal_mode() {
        let mut editor = CodeEditor::new("unchanged", "rs");
        assert_eq!(editor.vim_state.parse_key('9'), None);
        assert_eq!(editor.vim_state.parse_key('d'), None);

        editor.set_vim_enabled(true);

        assert!(editor.vim_enabled());
        assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
        assert_eq!(
            editor.vim_state.parse_key('l'),
            Some(vim::VimAction::Motion {
                motion: vim::VimMotion::Right,
                count: 1,
            })
        );
        assert_eq!(editor.content(), "unchanged");
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn vim_disable_clears_pending_state() {
        let mut editor =
            CodeEditor::new("unchanged", "rs").with_vim_enabled(true);
        assert_eq!(editor.vim_state.parse_key('4'), None);
        assert_eq!(editor.vim_state.parse_key('d'), None);

        editor.set_vim_enabled(false);
        assert!(!editor.vim_enabled());
        assert_eq!(editor.vim_mode(), None);

        editor.set_vim_enabled(true);
        assert_eq!(
            editor.vim_state.parse_key('w'),
            Some(vim::VimAction::Motion {
                motion: vim::VimMotion::WordForward,
                count: 1,
            })
        );
        assert_eq!(editor.content(), "unchanged");
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn vim_reset_clears_pending_state() {
        let mut editor = CodeEditor::new("before", "rs").with_vim_enabled(true);
        assert_eq!(editor.vim_state.parse_key('3'), None);
        assert_eq!(editor.vim_state.parse_key('g'), None);

        let _ = editor.reset("after");

        assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
        assert_eq!(editor.vim_state.parse_key('g'), None);
        assert_eq!(
            editor.vim_state.parse_key('g'),
            Some(vim::VimAction::Motion {
                motion: vim::VimMotion::DocumentStart,
                count: 1,
            })
        );
        assert_eq!(editor.content(), "after");
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn test_compare_floats() {
        // Equal cases
        assert_eq!(
            compare_floats(1.0, 1.0),
            CmpOrdering::Equal,
            "Exact equality"
        );
        assert_eq!(
            compare_floats(1.0, 1.0 + 0.0001),
            CmpOrdering::Equal,
            "Within epsilon (positive)"
        );
        assert_eq!(
            compare_floats(1.0, 1.0 - 0.0001),
            CmpOrdering::Equal,
            "Within epsilon (negative)"
        );

        // Greater cases
        assert_eq!(
            compare_floats(1.0 + 0.002, 1.0),
            CmpOrdering::Greater,
            "Definitely greater"
        );
        assert_eq!(
            compare_floats(1.0011, 1.0),
            CmpOrdering::Greater,
            "Just above epsilon"
        );

        // Less cases
        assert_eq!(
            compare_floats(1.0, 1.0 + 0.002),
            CmpOrdering::Less,
            "Definitely less"
        );
        assert_eq!(
            compare_floats(1.0, 1.0011),
            CmpOrdering::Less,
            "Just below negative epsilon"
        );
    }

    #[test]
    fn test_measure_text_width_ascii() {
        // "abc" (3 chars) -> 3 * CHAR_WIDTH
        let text = "abc";
        let width = measure_text_width(text, FONT_SIZE, CHAR_WIDTH);
        let expected = CHAR_WIDTH * 3.0;
        assert_eq!(
            compare_floats(width, expected),
            CmpOrdering::Equal,
            "Width mismatch for ASCII"
        );
    }

    #[test]
    fn test_measure_text_width_cjk() {
        // "你好" (2 chars) -> 2 * FONT_SIZE
        // Chinese characters are typically full-width.
        // width = 2 * FONT_SIZE
        let text = "你好";
        let width = measure_text_width(text, FONT_SIZE, CHAR_WIDTH);
        let expected = FONT_SIZE * 2.0;
        assert_eq!(
            compare_floats(width, expected),
            CmpOrdering::Equal,
            "Width mismatch for CJK"
        );
    }

    #[test]
    fn test_measure_text_width_mixed() {
        // "Hi" (2 chars) -> 2 * CHAR_WIDTH
        // "你好" (2 chars) -> 2 * FONT_SIZE
        let text = "Hi你好";
        let width = measure_text_width(text, FONT_SIZE, CHAR_WIDTH);
        let expected = CHAR_WIDTH * 2.0 + FONT_SIZE * 2.0;
        assert_eq!(
            compare_floats(width, expected),
            CmpOrdering::Equal,
            "Width mismatch for mixed content"
        );
    }

    #[test]
    fn test_measure_text_width_control_chars() {
        // "\t\n" (2 chars)
        // width = 4 * CHAR_WIDTH (tab) + 0 (newline)
        let text = "\t\n";
        let width = measure_text_width(text, FONT_SIZE, CHAR_WIDTH);
        let expected = CHAR_WIDTH * TAB_WIDTH as f32;
        assert_eq!(
            compare_floats(width, expected),
            CmpOrdering::Equal,
            "Width mismatch for control chars"
        );
    }

    #[test]
    fn test_measure_text_width_empty() {
        let text = "";
        let width = measure_text_width(text, FONT_SIZE, CHAR_WIDTH);
        assert!(
            (width - 0.0).abs() < f32::EPSILON,
            "Width should be 0 for empty string"
        );
    }

    #[test]
    fn test_measure_text_width_emoji() {
        // "👋" (1 char, width > 1) -> FONT_SIZE
        let text = "👋";
        let width = measure_text_width(text, FONT_SIZE, CHAR_WIDTH);
        let expected = FONT_SIZE;
        assert_eq!(
            compare_floats(width, expected),
            CmpOrdering::Equal,
            "Width mismatch for emoji"
        );
    }

    #[test]
    fn test_measure_text_width_korean() {
        // "안녕하세요" (5 chars)
        // Korean characters are typically full-width.
        // width = 5 * FONT_SIZE
        let text = "안녕하세요";
        let width = measure_text_width(text, FONT_SIZE, CHAR_WIDTH);
        let expected = FONT_SIZE * 5.0;
        assert_eq!(
            compare_floats(width, expected),
            CmpOrdering::Equal,
            "Width mismatch for Korean"
        );
    }

    #[test]
    fn test_measure_text_width_japanese() {
        // "こんにちは" (Hiragana, 5 chars) -> 5 * FONT_SIZE
        // "カタカナ" (Katakana, 4 chars) -> 4 * FONT_SIZE
        // "漢字" (Kanji, 2 chars) -> 2 * FONT_SIZE

        let text_hiragana = "こんにちは";
        let width_hiragana =
            measure_text_width(text_hiragana, FONT_SIZE, CHAR_WIDTH);
        let expected_hiragana = FONT_SIZE * 5.0;
        assert_eq!(
            compare_floats(width_hiragana, expected_hiragana),
            CmpOrdering::Equal,
            "Width mismatch for Hiragana"
        );

        let text_katakana = "カタカナ";
        let width_katakana =
            measure_text_width(text_katakana, FONT_SIZE, CHAR_WIDTH);
        let expected_katakana = FONT_SIZE * 4.0;
        assert_eq!(
            compare_floats(width_katakana, expected_katakana),
            CmpOrdering::Equal,
            "Width mismatch for Katakana"
        );

        let text_kanji = "漢字";
        let width_kanji = measure_text_width(text_kanji, FONT_SIZE, CHAR_WIDTH);
        let expected_kanji = FONT_SIZE * 2.0;
        assert_eq!(
            compare_floats(width_kanji, expected_kanji),
            CmpOrdering::Equal,
            "Width mismatch for Kanji"
        );
    }

    #[test]
    fn test_set_font_size() {
        let mut editor = CodeEditor::new("", "rs");

        // Initial state (defaults)
        assert!((editor.font_size() - 14.0).abs() < f32::EPSILON);
        assert!((editor.line_height() - 20.0).abs() < f32::EPSILON);

        // Test auto adjust = true
        editor.set_font_size(28.0, true);
        assert!((editor.font_size() - 28.0).abs() < f32::EPSILON);
        // Line height should double: 20.0 * (28.0/14.0) = 40.0
        assert_eq!(
            compare_floats(editor.line_height(), 40.0),
            CmpOrdering::Equal
        );

        // Test auto adjust = false
        // First set line height to something custom
        editor.set_line_height(50.0);
        // Change font size but keep line height
        editor.set_font_size(14.0, false);
        assert!((editor.font_size() - 14.0).abs() < f32::EPSILON);
        // Line height should stay 50.0
        assert_eq!(
            compare_floats(editor.line_height(), 50.0),
            CmpOrdering::Equal
        );
        // Char width should have scaled back to roughly default (but depends on measurement)
        // We check if it is close to the expected value, but since measurement can vary,
        // we just ensure it is positive and close to what we expect (around 8.4)
        assert!(editor.char_width() > 0.0);
        assert!((editor.char_width() - CHAR_WIDTH).abs() < 0.5);
    }

    #[test]
    fn test_measure_single_char_width() {
        let editor = CodeEditor::new("", "rs");

        // Measure 'a'
        let width_a = editor.measure_single_char_width("a");
        assert!(width_a > 0.0, "Width of 'a' should be positive");

        // Measure Chinese char
        let width_cjk = editor.measure_single_char_width("汉");
        assert!(width_cjk > 0.0, "Width of '汉' should be positive");

        assert!(
            width_cjk > width_a,
            "Width of '汉' should be greater than 'a'"
        );

        // Check that width_cjk is roughly double of width_a (common in terminal fonts)
        // but we just check it is significantly larger
        assert!(width_cjk >= width_a * 1.5);
    }

    #[test]
    fn test_ensure_font_metrics_self_heals_stale_width() {
        let editor = CodeEditor::new("", "rs");

        // The real measurement for the editor's font:
        let real = editor.measure_single_char_width("a");

        // Simulate the boot race: the width was captured before the real font
        // was registered, so it is wrong by more than the epsilon.
        let stale = real + 2.0;
        editor.char_width.set(stale);

        // A stale measurement must be corrected by the self-heal check.
        editor.ensure_font_metrics();
        assert!(
            (editor.char_width() - real).abs() <= FONT_METRICS_EPSILON,
            "char_width should be re-measured to the real font width \
             (got {}, expected {real})",
            editor.char_width()
        );

        // Once corrected, the check is disabled for good.
        editor.font_metrics_updated.set(true);
        editor.char_width.set(stale);
        editor.ensure_font_metrics();
        assert!(
            (editor.char_width() - stale).abs() < f32::EPSILON,
            "no further re-measurement after the metrics settled"
        );
    }

    #[test]
    fn test_set_line_height() {
        let mut editor = CodeEditor::new("", "rs");

        // Initial state
        assert!((editor.line_height() - LINE_HEIGHT).abs() < f32::EPSILON);

        // Set custom line height
        editor.set_line_height(35.0);
        assert!((editor.line_height() - 35.0).abs() < f32::EPSILON);

        // Font size should remain unchanged
        assert!((editor.font_size() - FONT_SIZE).abs() < f32::EPSILON);
    }

    #[test]
    fn test_visual_lines_cached_reuses_cache_for_same_key() {
        let editor = CodeEditor::new("a\nb\nc", "rs");

        let first = editor.visual_lines_cached(800.0);
        let second = editor.visual_lines_cached(800.0);

        assert!(
            Rc::ptr_eq(&first, &second),
            "visual_lines_cached should reuse the cached Rc for identical keys"
        );
    }

    #[derive(Default)]
    struct TestLspClient {
        changes: Rc<RefCell<Vec<Vec<lsp::LspTextChange>>>>,
    }

    impl lsp::LspClient for TestLspClient {
        fn did_change(
            &mut self,
            _document: &lsp::LspDocument,
            changes: &[lsp::LspTextChange],
        ) {
            self.changes.borrow_mut().push(changes.to_vec());
        }
    }

    #[test]
    fn test_word_start_in_line() {
        let line = "foo_bar baz";
        assert_eq!(CodeEditor::word_start_in_line(line, 0), 0);
        assert_eq!(CodeEditor::word_start_in_line(line, 2), 0);
        assert_eq!(CodeEditor::word_start_in_line(line, 4), 0);
        assert_eq!(CodeEditor::word_start_in_line(line, 7), 0);
        assert_eq!(CodeEditor::word_start_in_line(line, 9), 8);
    }

    #[test]
    fn test_enqueue_lsp_change_auto_flush() {
        let changes = Rc::new(RefCell::new(Vec::new()));
        let client = TestLspClient { changes: Rc::clone(&changes) };
        let mut editor = CodeEditor::new("hello", "rs");
        editor.attach_lsp(
            Box::new(client),
            lsp::LspDocument::new("file:///test.rs", "rust"),
        );
        editor.set_lsp_auto_flush(true);

        editor.buffer.insert_char(0, 5, '!');
        editor.enqueue_lsp_change();

        let changes = changes.borrow();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].len(), 1);
        let change = &changes[0][0];
        assert_eq!(change.text, "!");
        assert_eq!(change.range.start.line, 0);
        assert_eq!(change.range.start.character, 5);
        assert_eq!(change.range.end.line, 0);
        assert_eq!(change.range.end.character, 5);
    }

    #[test]
    fn test_editor_update_sends_bounded_incremental_lsp_change() {
        let changes = Rc::new(RefCell::new(Vec::new()));
        let client = TestLspClient { changes: Rc::clone(&changes) };
        let content = (0..10)
            .map(|line| format!("line{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = CodeEditor::new(&content, "rs");
        editor.attach_lsp(
            Box::new(client),
            lsp::LspDocument::new("file:///large.rs", "rust"),
        );
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        editor.cursors.primary_mut().position = (5, 2);

        let _ = editor.update(&Message::CharacterInput('X'));

        let changes = changes.borrow();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].len(), 1);
        let change = &changes[0][0];
        assert_eq!(change.range.start.line, 4);
        assert_eq!(change.range.start.character, 0);
        assert_eq!(change.range.end.line, 7);
        assert_eq!(change.range.end.character, 0);
        assert_eq!(change.text, "line4\nliXne5\nline6\n");
        assert!(!editor.lsp_shadow_is_current);
        assert!(editor.lsp_shadow_text.is_empty());
    }

    #[test]
    fn test_visual_lines_cached_changes_on_viewport_width_change() {
        let editor = CodeEditor::new("a\nb\nc", "rs");

        let first = editor.visual_lines_cached(800.0);
        let second = editor.visual_lines_cached(801.0);

        assert!(
            !Rc::ptr_eq(&first, &second),
            "visual_lines_cached should recompute when viewport width changes"
        );
    }

    #[test]
    fn test_visual_lines_cached_changes_on_buffer_revision_change() {
        let mut editor = CodeEditor::new("a\nb\nc", "rs");

        let first = editor.visual_lines_cached(800.0);
        editor.buffer_revision = editor.buffer_revision.wrapping_add(1);
        let second = editor.visual_lines_cached(800.0);

        assert!(
            !Rc::ptr_eq(&first, &second),
            "visual_lines_cached should recompute when buffer_revision changes"
        );
    }

    #[test]
    fn test_max_content_width_increases_with_longer_lines() {
        let short = CodeEditor::new("ab", "rs");
        let long =
            CodeEditor::new("abcdefghijklmnopqrstuvwxyz0123456789", "rs");

        assert!(
            long.max_content_width() > short.max_content_width(),
            "Longer lines should produce a greater max_content_width"
        );
    }

    #[test]
    fn test_max_content_width_cached_by_revision() {
        let mut editor = CodeEditor::new("hello", "rs");
        let w1 = editor.max_content_width();

        // Same revision → cache hit
        let w2 = editor.max_content_width();
        assert!(
            (w1 - w2).abs() < f32::EPSILON,
            "Repeated calls with same revision should return identical value"
        );

        // Bump revision to simulate edit
        editor.buffer_revision = editor.buffer_revision.wrapping_add(1);
        // Update the buffer to reflect a longer line
        editor.buffer = crate::text_buffer::TextBuffer::new(
            "hello world with extra content",
        );
        let w3 = editor.max_content_width();
        assert!(
            w3 > w1,
            "After revision bump with longer content, width should increase"
        );
    }

    #[test]
    fn test_max_content_width_cache_updates_incrementally_after_newline() {
        let mut editor =
            CodeEditor::new("short\nthis is the longest line\ntail", "rs");
        editor.set_wrap_enabled(false);
        editor.request_focus();
        editor.has_canvas_focus = true;
        editor.focus_locked = false;
        editor.cursors.primary_mut().position = (1, 7);
        let _ = editor.max_content_width();

        let _ = editor.update(&Message::Enter);
        let incremental = editor.max_content_width();
        let expected = CodeEditor::new(&editor.content(), "rs");

        assert!(
            (incremental - expected.max_content_width()).abs() < f32::EPSILON
        );
        let cache = editor.max_content_width_cache.borrow();
        assert_eq!(
            cache.as_ref().map(|cache| cache.line_widths.len()),
            Some(editor.buffer.line_count())
        );
        assert_eq!(
            cache.as_ref().map(|cache| cache.revision),
            Some(editor.buffer_revision)
        );
    }

    #[test]
    fn test_syntax_getter() {
        let editor = CodeEditor::new("", "lua");
        assert_eq!(editor.syntax(), "lua");
    }

    /// Buffer with one outer block (lines 0..=4) and a nested inner block
    /// (lines 2..=3), used by the folding tests.
    fn folding_editor() -> CodeEditor {
        CodeEditor::new(
            "fn main() {\n    let x = 1;\n    if x > 0 {\n        print();\n    }\n}",
            "rs",
        )
    }

    #[test]
    fn test_folding_enabled_by_default() {
        let editor = CodeEditor::new("fn main() {}", "rs");
        assert!(editor.folding_enabled());
    }

    #[test]
    fn test_foldable_regions_detected() {
        let editor = folding_editor();
        let regions = editor.foldable_regions();
        assert_eq!(
            *regions,
            vec![
                folding::FoldRegion::new(0, 4),
                folding::FoldRegion::new(2, 3)
            ]
        );
    }

    #[test]
    fn test_toggle_fold_hides_and_shows_lines() {
        let mut editor = folding_editor();
        let width = editor.viewport_width;
        let total = editor.visual_lines_cached(width).len();

        editor.toggle_fold(0);
        assert!(editor.is_folded(0));
        // Outer block hides lines 1..=4: only lines 0 and 5 remain.
        assert_eq!(editor.visual_lines_cached(width).len(), 2);

        editor.toggle_fold(0);
        assert!(!editor.is_folded(0));
        assert_eq!(editor.visual_lines_cached(width).len(), total);
    }

    #[test]
    fn test_toggle_fold_ignores_non_header() {
        let mut editor = folding_editor();
        editor.toggle_fold(3); // line 3 is not a header
        assert!(!editor.is_folded(3));
        assert!(editor.collapsed_folds.is_empty());
    }

    #[test]
    fn test_fold_at_picks_innermost_region() {
        let mut editor = folding_editor();
        // Line 3 is inside both (0,4) and (2,3); the innermost (header 2) folds.
        editor.fold_at(3);
        assert!(editor.is_folded(2));
        assert!(!editor.is_folded(0));
        assert_eq!(editor.hidden_lines_set(), [3].into_iter().collect());
    }

    #[test]
    fn test_unfold_at_expands_innermost_region() {
        let mut editor = folding_editor();
        editor.fold_at(3);
        editor.unfold_at(2);
        assert!(!editor.is_folded(2));
        assert!(editor.hidden_lines_set().is_empty());
    }

    #[test]
    fn test_toggle_fold_at_cursor_folds_then_unfolds() {
        let mut editor = folding_editor();
        // Line 3 is inside the innermost region (header 2).
        editor.toggle_fold_at(3);
        assert!(editor.is_folded(2));

        // Toggling again on the same line expands it back.
        editor.toggle_fold_at(2);
        assert!(!editor.is_folded(2));
    }

    #[test]
    fn test_toggle_fold_at_ignores_unfoldable_line() {
        let mut editor = CodeEditor::new("a\nb\nc", "rs");
        editor.toggle_fold_at(1);
        assert!(editor.collapsed_folds.is_empty());
    }

    #[test]
    fn test_fold_all_and_unfold_all() {
        let mut editor = folding_editor();
        editor.fold_all();
        assert!(editor.is_folded(0));
        assert!(editor.is_folded(2));
        // Outer fold hides 1..=4, inner hides 3: union is 1..=4.
        assert_eq!(
            editor.hidden_lines_set(),
            [1, 2, 3, 4].into_iter().collect()
        );

        editor.unfold_all();
        assert!(editor.collapsed_folds.is_empty());
        assert!(editor.hidden_lines_set().is_empty());
    }

    #[test]
    fn test_fold_moves_cursor_out_of_hidden_lines() {
        let mut editor = folding_editor();
        editor.cursors.set_single((3, 2));
        editor.fold_all();
        // Line 3 is hidden; the cursor moves up to the nearest visible line (0).
        assert_eq!(editor.cursors.primary_position(), (0, 0));
    }

    #[test]
    fn test_disabled_folding_yields_no_regions() {
        let mut editor = folding_editor();
        editor.set_folding_enabled(false);
        assert!(editor.foldable_regions().is_empty());
        // Collapsed state is preserved but produces no hidden lines while off.
        editor.collapsed_folds.insert(0);
        assert!(editor.hidden_lines_set().is_empty());
    }
}
