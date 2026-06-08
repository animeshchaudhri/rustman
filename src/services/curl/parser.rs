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
    pub form: Vec<CurlForm>,
}

#[derive(Debug, Serialize)]
pub struct CurlForm {
    pub key: String,
    pub value: String,
    pub is_file: bool,
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
                chars.next();
                match chars.peek().copied() {
                    // A backslash at the start of a token followed by whitespace is a
                    // line-continuation remnant (`cmd \<newline>` becomes `cmd \  ` once
                    // a single-line paste strips the newline). Drop it.
                    Some(c) if c.is_ascii_whitespace() && !had_content => {}
                    Some(_) => {
                        had_content = true;
                        token.push(chars.next().unwrap());
                    }
                    None => {}
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
                    if !d.starts_with('@') {
                        let piece = d.strip_prefix('=').unwrap_or(&d);
                        append_body(&mut result.body, piece);
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

            "-F" | "--form" => {
                if let Some(raw) = iter.next() {
                    apply_form(&raw, false, &mut result);
                }
            }

            "--form-string" => {
                if let Some(raw) = iter.next() {
                    apply_form(&raw, true, &mut result);
                }
            }

            flag if consumes_arg(flag) => {
                iter.next();
            }

            flag if flag.starts_with('-') && !flag.starts_with("--") && flag.len() > 2 => {
                handle_combined_flags(&flag[1..], &mut iter, &mut result);
            }

            tok => {
                if !tok.starts_with('-') && !tok.trim().is_empty() && result.url.is_none() {
                    result.url = Some(tok.to_string());
                }
            }
        }
    }

    match result.method.as_deref() {
        None | Some("") => {
            let has_payload = result.body.is_some() || !result.form.is_empty();
            result.method = Some(if has_payload { "POST".to_string() } else { "GET".to_string() });
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

fn apply_form(raw: &str, force_text: bool, result: &mut ParsedCurl) {
    if let Some(eq) = raw.find('=') {
        let key = raw[..eq].to_string();
        let val = &raw[eq + 1..];
        if !force_text {
            if let Some(path) = val.strip_prefix('@') {
                let path = path.split(';').next().unwrap_or(path).to_string();
                result.form.push(CurlForm { key, value: path, is_file: true });
                return;
            }
        }
        result.form.push(CurlForm { key, value: val.to_string(), is_file: false });
    }
}

fn append_body(body: &mut Option<String>, new_data: &str) {
    *body = Some(match body.take() {
        Some(existing) if !existing.is_empty() => format!("{}&{}", existing, new_data),
        _ => new_data.to_string(),
    });
}


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
            'F' => {
                if let Some(raw) = iter.next() {
                    apply_form(&raw, false, result);
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

#[cfg(test)]
mod tests {
    use super::*;

    const MULTILINE: &str = "curl \\\n  --url https://www.bajajfinservhealth.in/backend/hrx-search/v1/search/search?page=1&size=10 \\\n  --header 'x-origin: https://www.bajajfinservhealth.in' \\\n  --header 'incoming_source: hrx_dweb' \\\n  --header 'Accept: */*'";

    /// A multi-line curl pasted into a single-line input loses its newlines (iced
    /// strips control chars), leaving dangling `\` continuations. The URL and
    /// headers must still parse instead of the URL collapsing to a stray space.
    #[test]
    fn multiline_paste_without_newlines() {
        let pasted: String = MULTILINE.chars().filter(|c| !c.is_control()).collect();
        let p = parse(&pasted);
        assert_eq!(
            p.url.as_deref(),
            Some("https://www.bajajfinservhealth.in/backend/hrx-search/v1/search/search?page=1&size=10")
        );
        assert_eq!(p.header.get("x-origin").map(String::as_str), Some("https://www.bajajfinservhealth.in"));
        assert_eq!(p.header.get("Accept").map(String::as_str), Some("*/*"));
    }

    #[test]
    fn multiline_paste_with_newlines() {
        let p = parse(MULTILINE);
        assert_eq!(
            p.url.as_deref(),
            Some("https://www.bajajfinservhealth.in/backend/hrx-search/v1/search/search?page=1&size=10")
        );
    }

    // Only a *leading* backslash-whitespace is dropped; an escaped space inside a
    // token must still survive.
    #[test]
    fn escaped_space_inside_token_preserved() {
        let p = parse("curl 'https://example.com' --data foo\\ bar");
        assert_eq!(p.body.as_deref(), Some("foo bar"));
    }

    #[test]
    fn full_pasted_curl_round_trips() {
        const RAW: &str = "curl \\\n  --url https://www.bajajfinservhealth.in/backend/hrx-search/v1/search/search?page=1&size=10&index_type=Hospitals&pvdServices=Hospital&fetchEntities=false \\\n  --header 'x-origin: https://www.bajajfinservhealth.in' \\\n  --header 'sec-ch-ua: \"Not/A)Brand\";v=\"99\", \"Chromium\";v=\"148\"' \\\n  --header 'Accept: */*' \\\n  --cookie 'eventsObject={}; sharedLocation={\"city\":\"pune\"%2C\"lat\":\"18.520430\"}; locale=en'";
        let pasted: String = RAW.chars().filter(|c| !c.is_control()).collect();
        let p = parse(&pasted);

        assert_eq!(
            p.url.as_deref(),
            Some("https://www.bajajfinservhealth.in/backend/hrx-search/v1/search/search?page=1&size=10&index_type=Hospitals&pvdServices=Hospital&fetchEntities=false")
        );
        assert_eq!(p.header.get("Accept").map(String::as_str), Some("*/*"));
        assert_eq!(p.header.get("sec-ch-ua").map(String::as_str), Some("\"Not/A)Brand\";v=\"99\", \"Chromium\";v=\"148\""));
        assert_eq!(p.cookies.get("locale").map(String::as_str), Some("en"));
        assert_eq!(p.cookies.get("sharedLocation").map(String::as_str), Some("{\"city\":\"pune\",\"lat\":\"18.520430\"}"));
    }
}
