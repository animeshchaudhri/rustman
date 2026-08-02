//! UTF-8 indexing helpers shared across the editor.
//!
//! Rust strings are UTF-8, so a character offset (the "column" the editor works
//! in) is **not** a byte offset. Slicing a `String` with a character offset
//! would panic on a non-`char` boundary (accents, CJK, emoji). These helpers
//! convert character offsets to byte offsets in a single, well-tested place so
//! that every slicing site stays multi-byte safe.

/// Converts a character index to a byte index in a UTF-8 string.
///
/// Character offsets beyond the end of the string clamp to its byte length,
/// yielding an empty trailing slice rather than panicking.
///
/// # Arguments
///
/// * `s` - The string to index into.
/// * `char_index` - The offset, in characters.
///
/// # Returns
///
/// The byte index suitable for slicing `s`.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(char_to_byte_index("héllo", 2), 3); // 'é' is two bytes
/// assert_eq!(char_to_byte_index("abc", 10), 3); // out of bounds -> len
/// ```
pub(crate) fn char_to_byte_index(s: &str, char_index: usize) -> usize {
    // Source code is overwhelmingly ASCII. In that common case character and
    // byte offsets are identical, so avoid walking from the start of a long
    // line for every cursor edit.
    if s.is_ascii() {
        return char_index.min(s.len());
    }

    s.char_indices().nth(char_index).map_or(s.len(), |(idx, _)| idx)
}

/// Converts a `[start_char, end_char)` character range into the corresponding
/// `[start_byte, end_byte)` byte range within a UTF-8 string.
///
/// The string is walked a single time, so converting both boundaries costs
/// `O(end_char)` instead of the `O(n)` per boundary incurred by repeated
/// [`char_to_byte_index`] calls. Character offsets beyond the end of the string
/// clamp to its byte length, yielding an empty trailing slice rather than
/// panicking.
///
/// # Arguments
///
/// * `text` - The string to index into.
/// * `start_char` - Inclusive start offset, in characters.
/// * `end_char` - Exclusive end offset, in characters.
///
/// # Returns
///
/// The `(start_byte, end_byte)` byte offsets suitable for slicing `text`.
///
/// # Examples
///
/// ```ignore
/// let (s, e) = char_range_to_byte_range("héllo", 1, 3); // 'é','l'
/// assert_eq!(&"héllo"[s..e], "él");
/// ```
pub(crate) fn char_range_to_byte_range(
    text: &str,
    start_char: usize,
    end_char: usize,
) -> (usize, usize) {
    if text.is_ascii() {
        return (start_char.min(text.len()), end_char.min(text.len()));
    }

    let mut start_byte = text.len();
    let mut end_byte = text.len();

    for (char_idx, (byte_idx, _)) in text.char_indices().enumerate() {
        if char_idx == start_char {
            start_byte = byte_idx;
        }
        if char_idx == end_char {
            end_byte = byte_idx;
            break;
        }
    }

    (start_byte, end_byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_byte_index_ascii() {
        assert_eq!(char_to_byte_index("Hello", 0), 0);
        assert_eq!(char_to_byte_index("Hello", 3), 3);
        assert_eq!(char_to_byte_index("Hello", 5), 5);
    }

    #[test]
    fn test_char_to_byte_index_multibyte() {
        // 'é' is two bytes, '汉' is three bytes.
        assert_eq!(char_to_byte_index("héllo", 0), 0);
        assert_eq!(char_to_byte_index("héllo", 1), 1);
        assert_eq!(char_to_byte_index("héllo", 2), 3);
        assert_eq!(char_to_byte_index("汉字", 1), 3);
    }

    #[test]
    fn test_char_to_byte_index_out_of_bounds() {
        assert_eq!(char_to_byte_index("abc", 10), 3);
        assert_eq!(char_to_byte_index("", 0), 0);
    }

    #[test]
    fn test_char_range_to_byte_range_ascii() {
        assert_eq!(char_range_to_byte_range("Hello", 0, 5), (0, 5));
        assert_eq!(char_range_to_byte_range("Hello", 1, 3), (1, 3));
    }

    #[test]
    fn test_char_range_to_byte_range_multibyte() {
        let text = "héllo";
        let (start, end) = char_range_to_byte_range(text, 1, 3);
        assert_eq!(&text[start..end], "él");
    }

    #[test]
    fn test_char_range_to_byte_range_out_of_bounds() {
        let text = "héllo";
        let (start, end) = char_range_to_byte_range(text, 1, 10);
        assert_eq!(&text[start..end], "éllo");
    }

    #[test]
    fn test_char_range_to_byte_range_empty() {
        assert_eq!(char_range_to_byte_range("", 0, 0), (0, 0));
    }
}
