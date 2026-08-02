//! State for the go-to-line dialog.

use iced::widget::Id;

/// State owned by the compact go-to-line input.
#[derive(Debug, Clone)]
pub(crate) struct GotoLineState {
    /// User-entered, one-based line number.
    pub(crate) query: String,
    /// Whether the dialog is visible.
    pub(crate) is_open: bool,
    /// Stable input ID used for focus and selection operations.
    pub(crate) input_id: Id,
}

impl Default for GotoLineState {
    fn default() -> Self {
        Self { query: String::new(), is_open: false, input_id: Id::unique() }
    }
}

impl GotoLineState {
    /// Creates a closed go-to-line state.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Opens the dialog and pre-fills the current one-based line number.
    pub(crate) fn open(&mut self, current_line: usize) {
        self.query = current_line.saturating_add(1).to_string();
        self.is_open = true;
    }

    /// Closes the dialog without changing its current query.
    pub(crate) fn close(&mut self) {
        self.is_open = false;
    }

    /// Returns the entered one-based line number when it is a positive integer.
    pub(crate) fn target_line(&self) -> Option<usize> {
        self.query.trim().parse::<usize>().ok().filter(|line| *line > 0)
    }
}
