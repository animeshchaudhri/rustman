use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KvPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct FormFieldInput {
    pub key: String,
    pub value: String,
    pub is_file: bool,
    pub file_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateCurlInput {
    pub method: String,
    pub url: String,
    pub headers: Vec<KvPair>,
    pub body: Option<String>,
    pub form_fields: Vec<FormFieldInput>,
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

    if !input.form_fields.is_empty() {
        let has_file = input.form_fields.iter().any(|f| f.is_file);
        for field in &input.form_fields {
            if field.key.is_empty() {
                continue;
            }
            if has_file {
                // Uploaded file data lives in memory as base64, not on disk,
                // so there's no real path to point curl at. Reference the
                // filename instead (the same convention Postman/Insomnia
                // use) — the user fills in the path.
                let value = if field.is_file {
                    format!("@{}", field.file_name.as_deref().unwrap_or("file"))
                } else {
                    field.value.clone()
                };
                parts.push(format!(
                    "--form {}",
                    shell_escape(&format!("{}={}", field.key, value))
                ));
            } else {
                // No file fields: matches rustman's own behavior of sending
                // this as application/x-www-form-urlencoded.
                parts.push(format!(
                    "--data-urlencode {}",
                    shell_escape(&format!("{}={}", field.key, field.value))
                ));
            }
        }
    } else if let Some(body) = &input.body {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> GenerateCurlInput {
        GenerateCurlInput {
            method: "POST".to_owned(),
            url: "https://example.com/upload".to_owned(),
            headers: vec![],
            body: None,
            form_fields: vec![],
            cookies: vec![],
            auth_type: "none".to_owned(),
            bearer_token: None,
            basic_user: None,
            basic_pass: None,
            api_key_name: None,
            api_key_value: None,
            api_key_location: None,
        }
    }

    #[test]
    fn form_data_fields_produce_form_flags() {
        let mut input = base_input();
        input.form_fields = vec![
            FormFieldInput {
                key: "name".to_owned(),
                value: "rustman".to_owned(),
                is_file: false,
                file_name: None,
            },
            FormFieldInput {
                key: "avatar".to_owned(),
                value: String::new(),
                is_file: true,
                file_name: Some("avatar.png".to_owned()),
            },
        ];

        let cmd = generate(&input);

        assert!(cmd.contains("--form name=rustman"), "cmd was: {cmd}");
        assert!(cmd.contains("--form avatar=@avatar.png"), "cmd was: {cmd}");
        assert!(!cmd.contains("--data-raw"));
    }

    #[test]
    fn text_only_form_fields_produce_urlencoded_flags() {
        // No file field present: rustman sends this as
        // application/x-www-form-urlencoded, not multipart — the curl
        // command should match.
        let mut input = base_input();
        input.form_fields = vec![
            FormFieldInput {
                key: "conta".to_owned(),
                value: "sync_dep_1".to_owned(),
                is_file: false,
                file_name: None,
            },
            FormFieldInput {
                key: "modulo".to_owned(),
                value: "SYNC".to_owned(),
                is_file: false,
                file_name: None,
            },
        ];

        let cmd = generate(&input);

        assert!(cmd.contains("--data-urlencode conta=sync_dep_1"), "cmd was: {cmd}");
        assert!(cmd.contains("--data-urlencode modulo=SYNC"), "cmd was: {cmd}");
        assert!(!cmd.contains("--form"));
        assert!(!cmd.contains("--data-raw"));
    }

    #[test]
    fn form_data_takes_priority_over_stale_body_text() {
        // A body-editor leftover shouldn't leak into a form-data command.
        let mut input = base_input();
        input.body = Some("{\"leftover\":true}".to_owned());
        input.form_fields = vec![FormFieldInput {
            key: "key".to_owned(),
            value: "value".to_owned(),
            is_file: false,
            file_name: None,
        }];

        let cmd = generate(&input);

        assert!(cmd.contains("--data-urlencode key=value"));
        assert!(!cmd.contains("--data-raw"));
        assert!(!cmd.contains("leftover"));
    }

    #[test]
    fn empty_form_fields_fall_back_to_raw_body() {
        let mut input = base_input();
        input.body = Some("{\"a\":1}".to_owned());

        let cmd = generate(&input);

        assert!(cmd.contains("--data-raw"));
        assert!(!cmd.contains("--form"));
    }

    #[test]
    fn disabled_and_empty_key_form_fields_are_skipped() {
        let mut input = base_input();
        input.form_fields = vec![FormFieldInput {
            key: String::new(),
            value: "ignored".to_owned(),
            is_file: false,
            file_name: None,
        }];

        let cmd = generate(&input);

        assert!(!cmd.contains("ignored"));
    }
}
