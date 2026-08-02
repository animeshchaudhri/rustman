//! Tokenizer for the script language.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(f64),
    String(String),
    Ident(String),
    True,
    False,
    Null,

    // Keywords
    Let,
    If,
    Else,

    // Punctuation
    Dot,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,

    // Operators
    Plus,
    Minus,
    Bang,
    Eq,       // =
    EqEq,     // ==
    NotEq,    // !=
    Lt,
    Gt,
    LtEq,
    GtEq,
    AndAnd,
    OrOr,

    Eof,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub line: usize,
}

pub fn tokenize(source: &str) -> Result<Vec<(Token, usize)>, LexError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut line = 1;

    while i < chars.len() {
        let c = chars[i];

        match c {
            ' ' | '\t' | '\r' => i += 1,
            '\n' => {
                line += 1;
                i += 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '.' => {
                tokens.push((Token::Dot, line));
                i += 1;
            }
            ',' => {
                tokens.push((Token::Comma, line));
                i += 1;
            }
            '(' => {
                tokens.push((Token::LParen, line));
                i += 1;
            }
            ')' => {
                tokens.push((Token::RParen, line));
                i += 1;
            }
            '{' => {
                tokens.push((Token::LBrace, line));
                i += 1;
            }
            '}' => {
                tokens.push((Token::RBrace, line));
                i += 1;
            }
            ';' => {
                tokens.push((Token::Semicolon, line));
                i += 1;
            }
            '+' => {
                tokens.push((Token::Plus, line));
                i += 1;
            }
            '-' => {
                tokens.push((Token::Minus, line));
                i += 1;
            }
            '=' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push((Token::EqEq, line));
                    i += 2;
                } else {
                    tokens.push((Token::Eq, line));
                    i += 1;
                }
            }
            '!' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push((Token::NotEq, line));
                    i += 2;
                } else {
                    tokens.push((Token::Bang, line));
                    i += 1;
                }
            }
            '<' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push((Token::LtEq, line));
                    i += 2;
                } else {
                    tokens.push((Token::Lt, line));
                    i += 1;
                }
            }
            '>' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push((Token::GtEq, line));
                    i += 2;
                } else {
                    tokens.push((Token::Gt, line));
                    i += 1;
                }
            }
            '&' if chars.get(i + 1) == Some(&'&') => {
                tokens.push((Token::AndAnd, line));
                i += 2;
            }
            '|' if chars.get(i + 1) == Some(&'|') => {
                tokens.push((Token::OrOr, line));
                i += 2;
            }
            '"' => {
                let (s, next_i, next_line) = read_string(&chars, i, line)?;
                tokens.push((Token::String(s), line));
                i = next_i;
                line = next_line;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let n: f64 = text.parse().map_err(|_| LexError {
                    message: format!("invalid number literal '{text}'"),
                    line,
                })?;
                tokens.push((Token::Number(n), line));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let token = match word.as_str() {
                    "let" => Token::Let,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "true" => Token::True,
                    "false" => Token::False,
                    "null" => Token::Null,
                    _ => Token::Ident(word),
                };
                tokens.push((token, line));
            }
            other => {
                return Err(LexError {
                    message: format!("unexpected character '{other}'"),
                    line,
                });
            }
        }
    }

    tokens.push((Token::Eof, line));
    Ok(tokens)
}

fn read_string(
    chars: &[char],
    start: usize,
    start_line: usize,
) -> Result<(String, usize, usize), LexError> {
    let mut i = start + 1; // skip opening quote
    let mut line = start_line;
    let mut out = String::new();
    loop {
        match chars.get(i) {
            None => {
                return Err(LexError { message: "unterminated string literal".to_owned(), line });
            }
            Some('"') => {
                i += 1;
                return Ok((out, i, line));
            }
            Some('\\') => match chars.get(i + 1) {
                Some('n') => {
                    out.push('\n');
                    i += 2;
                }
                Some('t') => {
                    out.push('\t');
                    i += 2;
                }
                Some('"') => {
                    out.push('"');
                    i += 2;
                }
                Some('\\') => {
                    out.push('\\');
                    i += 2;
                }
                _ => {
                    return Err(LexError {
                        message: "invalid escape sequence in string literal".to_owned(),
                        line,
                    });
                }
            },
            Some('\n') => {
                line += 1;
                out.push('\n');
                i += 1;
            }
            Some(c) => {
                out.push(*c);
                i += 1;
            }
        }
    }
}
