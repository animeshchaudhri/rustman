use std::ops::Range;

use iced::Color;

// ── Syntax colours (matching response tree view) ──────────────────────────────

const KEY_COLOR: Color = Color { r: 0.72, g: 0.61, b: 0.97, a: 1.0 };   // purple
const STRING_COLOR: Color = Color { r: 0.56, g: 0.86, b: 0.50, a: 1.0 }; // green
const NUMBER_COLOR: Color = Color { r: 0.95, g: 0.78, b: 0.42, a: 1.0 }; // amber
const BOOL_COLOR: Color = Color { r: 0.47, g: 0.73, b: 0.98, a: 1.0 };   // blue
const NULL_COLOR: Color = Color { r: 0.90, g: 0.52, b: 0.52, a: 1.0 };   // rose
const PUNCT_COLOR: Color = Color { r: 0.65, g: 0.65, b: 0.70, a: 1.0 };  // gray

// ── Highlight token ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JsonHighlight(pub Color);

impl JsonHighlight {
    pub fn color(&self) -> Color {
        self.0
    }
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct JsonHighlightSettings {
    pub enabled: bool,
}

// ── Highlighter implementation ────────────────────────────────────────────────

pub struct JsonHighlighter {
    current_line: usize,
    enabled: bool,
}

impl iced::advanced::text::Highlighter for JsonHighlighter {
    type Settings = JsonHighlightSettings;
    type Highlight = JsonHighlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, JsonHighlight)>;

    fn new(settings: &Self::Settings) -> Self {
        Self { current_line: 0, enabled: settings.enabled }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.enabled = new_settings.enabled;
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn current_line(&self) -> usize {
        self.current_line
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        if !self.enabled {
            return vec![].into_iter();
        }
        tokenize_json_line(line).into_iter()
    }
}

// ── Line tokenizer ────────────────────────────────────────────────────────────

fn tokenize_json_line(line: &str) -> Vec<(Range<usize>, JsonHighlight)> {
    let mut spans: Vec<(Range<usize>, JsonHighlight)> = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        let (byte_pos, ch) = chars[i];
        match ch {
                ' ' | '\t' => { i += 1; }

            '"' => {
                let start_byte = byte_pos;
                i += 1;
                while i < n {
                    let (_, c) = chars[i];
                    if c == '\\' {
                        i += 2; // skip escape + next char
                        continue;
                    }
                    i += 1;
                    if c == '"' { break; }
                }
                let end_byte = if i < n { chars[i].0 } else { line.len() };

                let mut j = i;
                while j < n && chars[j].1 == ' ' { j += 1; }
                let is_key = j < n && chars[j].1 == ':';

                let color = if is_key { KEY_COLOR } else { STRING_COLOR };
                spans.push((start_byte..end_byte, JsonHighlight(color)));
            }

            '{' | '}' | '[' | ']' | ':' | ',' => {
                let end_byte = if i + 1 < n { chars[i + 1].0 } else { byte_pos + ch.len_utf8() };
                spans.push((byte_pos..end_byte, JsonHighlight(PUNCT_COLOR)));
                i += 1;
            }

            't' if line[byte_pos..].starts_with("true") => {
                spans.push((byte_pos..byte_pos + 4, JsonHighlight(BOOL_COLOR)));
                i += 4;
            }
            'f' if line[byte_pos..].starts_with("false") => {
                spans.push((byte_pos..byte_pos + 5, JsonHighlight(BOOL_COLOR)));
                i += 5;
            }
            'n' if line[byte_pos..].starts_with("null") => {
                spans.push((byte_pos..byte_pos + 4, JsonHighlight(NULL_COLOR)));
                i += 4;
            }

            '-' | '0'..='9' => {
                let start_byte = byte_pos;
                while i < n {
                    let c = chars[i].1;
                    if c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E' || c == '+' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let end_byte = if i < n { chars[i].0 } else { line.len() };
                spans.push((start_byte..end_byte, JsonHighlight(NUMBER_COLOR)));
            }

            _ => { i += 1; }
        }
    }
    spans
}
