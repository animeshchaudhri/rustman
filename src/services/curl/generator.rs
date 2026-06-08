use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KvPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerateCurlInput {
    pub method: String,
    pub url: String,
    pub headers: Vec<KvPair>,
    pub body: Option<String>,
    pub cookies: Vec<KvPair>,
    pub auth_type: String,
    pub bearer_token: Option<String>,
    pub basic_user: Option<String>,
    pub basic_pass: Option<String>,
    pub api_key_name: Option<String>,
    pub api_key_value: Option<String>,
    pub api_key_location: Option<String>,
}

pub fn generate(input: &GenerateCurlInput) -> String {
    let mut parts: Vec<String> = Vec::new();

    let method = if input.method.trim().is_empty() { "GET" } else { input.method.trim() };

    if method != "GET" {
        parts.push(format!("--request {}", method));
    }

    let effective_url = build_url_with_apikey(input);
    parts.push(format!("--url {}", shell_escape(&effective_url)));

    match input.auth_type.as_str() {
        "bearer" => {
            if let Some(token) = &input.bearer_token {
                if !token.is_empty() {
                    parts.push(format!(
                        "--header {}",
                        shell_escape(&format!("Authorization: Bearer {}", token))
                    ));
                }
            }
        }
        "basic" => {
            if let Some(user) = &input.basic_user {
                if !user.is_empty() {
                    let pass = input.basic_pass.as_deref().unwrap_or("");
                    parts.push(format!(
                        "--user {}",
                        shell_escape(&format!("{}:{}", user, pass))
                    ));
                }
            }
        }
        "apikey" => {
            if let (Some(name), Some(value), Some(loc)) = (
                &input.api_key_name,
                &input.api_key_value,
                &input.api_key_location,
            ) {
                if !name.is_empty() && !value.is_empty() && loc.as_str() == "header" {
                    parts.push(format!(
                        "--header {}",
                        shell_escape(&format!("{}: {}", name, value))
                    ));
                }
            }
        }
        _ => {}
    }

    for kv in &input.headers {
        if !kv.key.is_empty() {
            parts.push(format!(
                "--header {}",
                shell_escape(&format!("{}: {}", kv.key, kv.value))
            ));
        }
    }

    let active_cookies: Vec<_> = input.cookies.iter().filter(|kv| !kv.key.is_empty()).collect();
    if !active_cookies.is_empty() {
        let cookie_str = active_cookies
            .iter()
            .map(|kv| format!("{}={}", kv.key, cookie_encode_value(&kv.value)))
            .collect::<Vec<_>>()
            .join("; ");
        parts.push(format!("--cookie {}", shell_escape(&cookie_str)));
    }

    if let Some(body) = &input.body {
        if !body.is_empty() {
            parts.push(format!("--data-raw {}", shell_escape(body)));
        }
    }

    if parts.is_empty() {
        return "curl".to_string();
    }

    format!("curl \\\n  {}", parts.join(" \\\n  "))
}

fn build_url_with_apikey(input: &GenerateCurlInput) -> String {
    if input.auth_type == "apikey" {
        if let (Some(name), Some(value), Some(loc)) = (
            input.api_key_name.as_deref(),
            input.api_key_value.as_deref(),
            input.api_key_location.as_deref(),
        ) {
            if !name.is_empty() && !value.is_empty() && loc == "query" {
                let sep = if input.url.contains('?') { '&' } else { '?' };
                return format!("{}{}{name}={value}", input.url, sep);
            }
        }
    }
    input.url.clone()
}

fn cookie_encode_value(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            ';' => vec!['%', '3', 'B'],
            ',' => vec!['%', '2', 'C'],
            _ => vec![c],
        })
        .collect()
}

fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }

    let safe = s.bytes().all(|b| {
        matches!(b,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'/' | b':' | b'@'
            | b',' | b'+' | b'%' | b'='
        )
    });

    if safe {
        return s.to_string();
    }

    format!("'{}'", s.replace('\'', "'\\''"))
}
