//! Minimal LSP types and helpers used by the editor.

/// A zero-based position in an LSP document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspPosition {
    /// Zero-based line index.
    pub line: u32,
    /// Zero-based character index on the line.
    pub character: u32,
}

/// Metadata describing the currently edited document.
#[derive(Debug, Clone)]
pub struct LspDocument {
    /// Document URI.
    pub uri: String,
    /// Language identifier for syntax services.
    pub language_id: String,
    /// Version number used for LSP change notifications.
    pub version: i32,
}

impl LspDocument {
    /// Creates a new LSP document descriptor with version set to 0.
    pub fn new(uri: impl Into<String>, language_id: impl Into<String>) -> Self {
        Self { uri: uri.into(), language_id: language_id.into(), version: 0 }
    }
}

/// A text range in an LSP document.
#[derive(Debug, Clone, Copy)]
pub struct LspRange {
    /// Range start (inclusive).
    pub start: LspPosition,
    /// Range end (exclusive).
    pub end: LspPosition,
}

/// A text change described by a range replacement.
#[derive(Debug, Clone)]
pub struct LspTextChange {
    /// Range replaced by the change.
    pub range: LspRange,
    /// Inserted text.
    pub text: String,
}

/// LSP client hooks invoked by the editor.
pub trait LspClient {
    /// Notifies the client that a document was opened.
    fn did_open(&mut self, _document: &LspDocument, _text: &str) {}
    /// Notifies the client that the document changed.
    fn did_change(
        &mut self,
        _document: &LspDocument,
        _changes: &[LspTextChange],
    ) {
    }
    /// Notifies the client that the document was saved.
    fn did_save(&mut self, _document: &LspDocument, _text: &str) {}
    /// Notifies the client that the document was closed.
    fn did_close(&mut self, _document: &LspDocument) {}
    /// Requests hover information at the given position.
    fn request_hover(
        &mut self,
        _document: &LspDocument,
        _position: LspPosition,
    ) {
    }
    /// Requests completion items at the given position.
    fn request_completion(
        &mut self,
        _document: &LspDocument,
        _position: LspPosition,
    ) {
    }
    /// Requests the definition location(s) for the symbol at the given position.
    ///
    /// This method is called when the user triggers a "Go to Definition" action
    /// (e.g., via Ctrl+Click or a context menu). The client implementation should
    /// send a `textDocument/definition` request to the LSP server.
    fn request_definition(
        &mut self,
        _document: &LspDocument,
        _position: LspPosition,
    ) {
    }
}

/// Computes a minimal text change between two snapshots.
///
/// Returns `None` when the input strings are identical.
pub fn compute_text_change(old: &str, new: &str) -> Option<LspTextChange> {
    if old == new {
        return None;
    }

    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let old_len = old_chars.len();
    let new_len = new_chars.len();

    let mut prefix = 0;
    while prefix < old_len
        && prefix < new_len
        && old_chars[prefix] == new_chars[prefix]
    {
        prefix += 1;
    }

    let mut suffix = 0;
    while suffix < old_len.saturating_sub(prefix)
        && suffix < new_len.saturating_sub(prefix)
        && old_chars[old_len - 1 - suffix] == new_chars[new_len - 1 - suffix]
    {
        suffix += 1;
    }

    let removed_len = old_len.saturating_sub(prefix + suffix);
    let inserted: String =
        new_chars[prefix..new_len.saturating_sub(suffix)].iter().collect();

    let start = position_for_char_index(old, prefix);
    let end = position_for_char_index(old, prefix + removed_len);

    Some(LspTextChange { range: LspRange { start, end }, text: inserted })
}

/// Converts a character index into a line/character position.
fn position_for_char_index(text: &str, target_index: usize) -> LspPosition {
    let mut line: u32 = 0;
    let mut character: u32 = 0;
    for (index, ch) in text.chars().enumerate() {
        if index == target_index {
            return LspPosition { line, character };
        }
        if ch == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(1);
        }
    }

    LspPosition { line, character }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_text_change_none_when_equal() {
        let change = compute_text_change("abc", "abc");
        assert!(change.is_none());
    }

    #[test]
    fn test_compute_text_change_insertion() {
        let change = compute_text_change("abc", "abXc");
        assert!(change.is_some());
        if let Some(change) = change {
            assert_eq!(change.text, "X");
            assert_eq!(
                change.range.start,
                LspPosition { line: 0, character: 2 }
            );
            assert_eq!(change.range.end, LspPosition { line: 0, character: 2 });
        }
    }

    #[test]
    fn test_compute_text_change_deletion_across_lines() {
        let change = compute_text_change("a\nbc", "a\nc");
        assert!(change.is_some());
        if let Some(change) = change {
            assert_eq!(change.text, "");
            assert_eq!(
                change.range.start,
                LspPosition { line: 1, character: 0 }
            );
            assert_eq!(change.range.end, LspPosition { line: 1, character: 1 });
        }
    }

    #[test]
    fn test_position_for_char_index_end_of_text() {
        let pos = position_for_char_index("a\nb", 3);
        assert_eq!(pos, LspPosition { line: 1, character: 1 });
    }
}
