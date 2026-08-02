use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub body_size: usize,
    pub body_stored: bool,
    #[serde(default)]
    pub is_binary: bool,
    /// Raw bytes of a binary response, kept only for the live response so the
    /// UI can render a PDF/spreadsheet preview. Never persisted (history and
    /// collections would otherwise balloon with embedded binary blobs) —
    /// reloading a past response from history just falls back to the plain
    /// "Binary response" message instead of a rendered preview.
    #[serde(skip)]
    pub binary_data: Option<Vec<u8>>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// What kind of rich preview (if any) a binary response's content-type calls for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryPreviewKind {
    Pdf,
    Spreadsheet,
    /// A recognised binary type without a richer preview (image, zip, etc.) —
    /// falls back to the plain "Binary response" message.
    Other,
}

impl HttpResponse {
    /// Looks at the response's Content-Type header to decide which preview
    /// (if any) `binary_data` should be rendered with.
    pub fn binary_preview_kind(&self) -> BinaryPreviewKind {
        let content_type = self
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.to_ascii_lowercase())
            .unwrap_or_default();

        if content_type.contains("pdf") {
            BinaryPreviewKind::Pdf
        } else if content_type.contains("spreadsheet")
            || content_type.contains("ms-excel")
            || content_type.contains("opendocument.spreadsheet")
        {
            BinaryPreviewKind::Spreadsheet
        } else {
            BinaryPreviewKind::Other
        }
    }

    /// Whether this (non-binary) response's Content-Type is HTML.
    pub fn is_html(&self) -> bool {
        self.headers
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("content-type") && value.to_ascii_lowercase().contains("text/html"))
    }
}
