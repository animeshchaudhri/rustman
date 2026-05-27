use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
pub struct ParsedCurl {
    pub method: Option<String>,
    pub url: Option<String>,
    pub header: HashMap<String, String>,
    pub body: Option<String>,
    pub cookies: HashMap<String, String>,
}

pub fn parse(input: &str) -> ParsedCurl {
    let normalized = normalize_line_continuations(input);
    let cmd_body = strip_curl_prefix(&normalized);
    let raw_tokens = shell_tokenize(cmd_body);
    let tokens = expand_long_flag_assignments(raw_tokens);
    parse_tokens(tokens)
}

fn normalize_line_continuations(s: &str) -> String {
    s.replace("\\\r\n", " ").replace("\\\n", " ").replace('\r', "")
}

fn strip_curl_prefix(s: &str) -> &str {
    let trimmed = s.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("curl") {
        let after = &trimmed[4..];
        if after.is_empty() || after.starts_with(char::is_whitespace) {
            return after.trim_start();
        }
    }
    trimmed
}

// Expands `--flag=value` into ["--flag", "value"] so parse_tokens handles
// a single canonical form rather than the `=` variant in every match arm.
fn expand_long_flag_assignments(tokens: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(tokens.len() + 4);
    for tok in tokens {
        if tok.starts_with("--") {
            if let Some(eq) = tok.find('=') {
                out.push(tok[..eq].to_string());
                out.push(tok[eq + 1..].to_string());
                continue;
            }
        }
        out.push(tok);
    }
    out
}

fn shell_tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();
    loop {
        skip_whitespace(&mut chars);
        if chars.peek().is_none() {
            break;
        }
        if let Some(tok) = read_token(&mut chars) {
            tokens.push(tok);
        }
    }
    tokens
}

fn skip_whitespace(chars: &mut Peekable<Chars<'_>>) {
    while chars.peek().map_or(false, |c| c.is_ascii_whitespace()) {
        chars.next();
    }
}

fn read_token(chars: &mut Peekable<Chars<'_>>) -> Option<String> {
    let mut token = String::new();
    let mut had_content = false;

    loop {
        match chars.peek().copied() {
            None | Some(' ') | Some('\t') | Some('\n') => break,

            Some('\'') => {
                had_content = true;
                chars.next();
                loop {
                    match chars.next() {
                        None | Some('\'') => break,
                        Some(c) => token.push(c),
                    }
                }
            }

            Some('"') => {
                had_content = true;
                chars.next();
                loop {
                    match chars.peek().copied() {
                        None => break,
                        Some('"') => { chars.next(); break; }
                        Some('\\') => {
                            chars.next();
                            if let Some(c) = chars.next() {
                                token.push(c);
                            }
                        }
                        Some(_) => token.push(chars.next().unwrap()),
                    }
                }
            }

            Some('$') => {
                had_content = true;
                chars.next();
                if chars.peek() == Some(&'\'') {
                    // ANSI-C quoting $'...' — interpret escape sequences
                    chars.next();
                    loop {
                        match chars.next() {
                            None | Some('\'') => break,
                            Some('\\') => match chars.next() {
                                Some('n') => token.push('\n'),
                                Some('t') => token.push('\t'),
                                Some('r') => token.push('\r'),
                                Some('\\') => token.push('\\'),
                                Some('\'') => token.push('\''),
                                Some('"') => token.push('"'),
                                Some('a') => token.push('\x07'),
                                Some('b') => token.push('\x08'),
                                Some('f') => token.push('\x0C'),
                                Some('v') => token.push('\x0B'),
                                Some(c) => { token.push('\\'); token.push(c); }
                                None => break,
                            },
                            Some(c) => token.push(c),
                        }
                    }
                } else {
                    token.push('$');
                }
            }

            Some('\\') => {
                had_content = true;
                chars.next();
                if let Some(c) = chars.next() {
                    token.push(c);
                }
            }

            Some(c) => {
                had_content = true;
                chars.next();
                token.push(c);
            }
        }
    }

    if had_content { Some(token) } else { None }
}

fn parse_tokens(tokens: Vec<String>) -> ParsedCurl {
    let mut result = ParsedCurl::default();
    let mut iter = tokens.into_iter().peekable();

    while let Some(tok) = iter.next() {
        match tok.as_str() {
            "-X" | "--request" => {
                if let Some(m) = iter.next() {
                    result.method = Some(m.to_ascii_uppercase());
                }
            }

            "--url" => {
                if let Some(u) = iter.next() {
                    if result.url.is_none() {
                        result.url = Some(u);
                    }
                }
            }

            "-H" | "--header" => {
                if let Some(raw) = iter.next() {
                    apply_header(&raw, &mut result);
                }
            }

            "-b" | "--cookie" => {
                if let Some(raw) = iter.next() {
                    parse_cookie_str(&raw, &mut result.cookies);
                }
            }

            "-d" | "--data" | "--data-ascii" => {
                if let Some(d) = iter.next() {
                    if !d.starts_with('@') {
                        append_body(&mut result.body, &d);
                    }
                }
            }

            "--data-raw" => {
                if let Some(d) = iter.next() {
                    append_body(&mut result.body, &d);
                }
            }

            "--data-binary" => {
                if let Some(d) = iter.next() {
                    if !d.starts_with('@') {
                        append_body(&mut result.body, &d);
                    }
                }
            }

            "--data-urlencode" => {
                if let Some(d) = iter.next() {
                    if d.starts_with('@') {
                        // '@file' references are not inlineable — skip
                    } else {
                        let value = if let Some(pos) = d.find('=') {
                            d[pos + 1..].to_string()
                        } else {
                            d
                        };
                        append_body(&mut result.body, &value);
                    }
                }
            }

            "--json" => {
                if let Some(d) = iter.next() {
                    result.body = Some(d);
                    result.header.entry("Content-Type".to_string())
                        .or_insert_with(|| "application/json".to_string());
                    result.header.entry("Accept".to_string())
                        .or_insert_with(|| "application/json".to_string());
                }
            }

            "-u" | "--user" => {
                if let Some(creds) = iter.next() {
                    let encoded = general_purpose::STANDARD.encode(creds.as_bytes());
                    result.header.insert("Authorization".to_string(), format!("Basic {}", encoded));
                }
            }

            "--oauth2-bearer" => {
                if let Some(token) = iter.next() {
                    result.header.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
            }

            "-G" | "--get" => {
                result.method = Some("GET".to_string());
            }

            "-I" | "--head" => {
                result.method = Some("HEAD".to_string());
            }

            "-F" | "--form" | "--form-string" => {
                iter.next();
            }

            flag if consumes_arg(flag) => {
                iter.next();
            }

            flag if flag.starts_with('-') && !flag.starts_with("--") && flag.len() > 2 => {
                handle_combined_flags(&flag[1..], &mut iter, &mut result);
            }

            tok => {
                if !tok.starts_with('-') && result.url.is_none() {
                    result.url = Some(tok.to_string());
                }
            }
        }
    }

    match result.method.as_deref() {
        None | Some("") => {
            result.method = Some(if result.body.is_some() { "POST".to_string() } else { "GET".to_string() });
        }
        _ => {}
    }

    result
}

fn apply_header(raw: &str, result: &mut ParsedCurl) {
    if let Some(pos) = raw.find(':') {
        let key = raw[..pos].trim();
        let val = raw[pos + 1..].trim();
        if key.is_empty() {
            return;
        }
        if key.eq_ignore_ascii_case("cookie") {
            parse_cookie_str(val, &mut result.cookies);
        } else {
            result.header.insert(key.to_string(), val.to_string());
        }
    }
}

fn append_body(body: &mut Option<String>, new_data: &str) {
    *body = Some(match body.take() {
        Some(existing) if !existing.is_empty() => format!("{}&{}", existing, new_data),
        _ => new_data.to_string(),
    });
}

// Cookie attributes appear in Set-Cookie headers and sometimes get pasted
// verbatim into -b arguments; we must not treat them as cookie name=value pairs.
fn is_cookie_attribute(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "path" | "domain" | "expires" | "max-age" | "secure" | "httponly"
            | "samesite" | "partitioned" | "priority"
    )
}

fn parse_cookie_str(s: &str, cookies: &mut HashMap<String, String>) {
    for part in s.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(pos) = part.find('=') {
            let name = part[..pos].trim();
            let value = part[pos + 1..].trim();
            if !name.is_empty() && !is_cookie_attribute(name) {
                cookies.insert(name.to_string(), url_decode(value));
            }
        } else if !part.is_empty() && !is_cookie_attribute(part) {
            cookies.insert(part.to_string(), String::new());
        }
    }
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                char::from(bytes[i + 1]).to_digit(16),
                char::from(bytes[i + 2]).to_digit(16),
            ) {
                out.push(((hi as u8) << 4) | (lo as u8));
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

fn consumes_arg(flag: &str) -> bool {
    matches!(
        flag,
        "-A" | "--user-agent"
            | "--connect-timeout"
            | "-m"
            | "--max-time"
            | "--retry"
            | "--retry-delay"
            | "--retry-max-time"
            | "-o"
            | "--output"
            | "--proxy"
            | "-x"
            | "--cert"
            | "--key"
            | "-e"
            | "--referer"
            | "--referrer"
            | "--cacert"
            | "--capath"
            | "--resolve"
            | "--interface"
            | "--dns-servers"
            | "--max-filesize"
            | "--limit-rate"
            | "--keepalive-time"
            | "--proto"
            | "--proto-redir"
            | "-c"
            | "--cookie-jar"
            | "-D"
            | "--dump-header"
            | "-C"
            | "--continue-at"
            | "--local-port"
            | "-T"
            | "--upload-file"
            | "--trace"
            | "--trace-ascii"
            | "--unix-socket"
            | "--abstract-unix-socket"
            | "-w"
            | "--write-out"
            | "-y"
            | "--speed-time"
            | "-Y"
            | "--speed-limit"
            | "-r"
            | "--range"
            | "--preproxy"
            | "--doh-url"
            | "--noproxy"
            | "--socks4"
            | "--socks4a"
            | "--socks5"
            | "--socks5-hostname"
            | "-Q"
            | "--quote"
            | "--tlsuser"
            | "--tlspassword"
            | "--tlsauthtype"
            | "--tls-max"
            | "--tls13-ciphers"
            | "--ciphers"
            | "--curves"
            | "--delegation"
            | "-K"
            | "--config"
            | "--netrc-file"
            | "--pinnedpubkey"
            | "--login-options"
            | "--sasl-mech"
            | "--service-name"
            | "--mail-from"
            | "--mail-rcpt"
            | "--mail-auth"
            | "--smtp-auth"
            | "--ftp-port"
            | "-P"
            | "--ftp-alternative-to-user"
    )
}

fn handle_combined_flags<I>(flags: &str, iter: &mut std::iter::Peekable<I>, result: &mut ParsedCurl)
where
    I: Iterator<Item = String>,
{
    let chars: Vec<char> = flags.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            'X' => {
                let rest: String = chars[i + 1..].iter().collect();
                if !rest.is_empty() {
                    result.method = Some(rest.to_ascii_uppercase());
                } else if let Some(m) = iter.next() {
                    result.method = Some(m.to_ascii_uppercase());
                }
                return;
            }
            'H' => {
                if let Some(raw) = iter.next() {
                    apply_header(&raw, result);
                }
                return;
            }
            'd' => {
                let rest: String = chars[i + 1..].iter().collect();
                let data = if !rest.is_empty() { rest } else { iter.next().unwrap_or_default() };
                if !data.starts_with('@') {
                    append_body(&mut result.body, &data);
                }
                return;
            }
            'b' => {
                if let Some(raw) = iter.next() {
                    parse_cookie_str(&raw, &mut result.cookies);
                }
                return;
            }
            'u' => {
                if let Some(creds) = iter.next() {
                    let encoded = general_purpose::STANDARD.encode(creds.as_bytes());
                    result.header.insert("Authorization".to_string(), format!("Basic {}", encoded));
                }
                return;
            }
            'G' => result.method = Some("GET".to_string()),
            'I' => result.method = Some("HEAD".to_string()),
            's' | 'S' | 'v' | 'L' | 'k' | 'i' | 'f' | 'N' | 'g' | 'O' | 'J' | 'R' => {}
            _ => {}
        }
        i += 1;
    }
}
