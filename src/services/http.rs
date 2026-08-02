use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose, Engine as _};
use hmac::{digest::KeyInit, Hmac, Mac};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde_json::Value;
use sha2::Sha256;
use std::str::FromStr;

use crate::domain::collection::SavedRequest;
use crate::domain::request::{ApiKeyLocation, AuthType, BodyType};
use crate::domain::response::HttpResponse;
use crate::domain::environment::{substitute, AppEnvironment};

type HmacSha256 = Hmac<Sha256>;

fn base64url(data: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn make_jwt(subject: &str, secret: &str) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let header = base64url(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let payload_json = format!(
        "{{\"sub\":\"{}\",\"iat\":{},\"exp\":{}}}",
        subject.replace('"', "\\\""),
        now,
        now + 3600
    );
    let payload = base64url(payload_json.as_bytes());
    let signing_input = format!("{header}.{payload}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| e.to_string())?;
    mac.update(signing_input.as_bytes());
    let sig = base64url(&mac.finalize().into_bytes());
    Ok(format!("{signing_input}.{sig}"))
}

const INLINE_BODY_THRESHOLD: usize = 10_000_000;

#[derive(Debug, Clone)]
pub struct HttpResult {
    pub tab_id: String,
    pub response: HttpResponse,
}

pub async fn send(
    client: &Client,
    tab_id: String,
    req: &SavedRequest,
    env: Option<&AppEnvironment>,
    default_timeout_ms: u64,
) -> HttpResult {
    let start = Instant::now();
    match do_send(client, &tab_id, req, env, default_timeout_ms).await {
        Ok(mut resp) => {
            resp.duration_ms = start.elapsed().as_millis() as u64;
            HttpResult { tab_id, response: resp }
        }
        Err(e) => HttpResult {
            tab_id,
            response: HttpResponse { error: Some(e), ..Default::default() },
        },
    }
}

async fn do_send(
    client: &Client,
    _tab_id: &str,
    req: &SavedRequest,
    env: Option<&AppEnvironment>,
    default_timeout_ms: u64,
) -> Result<HttpResponse, String> {
    let url = substitute(&req.url, env);
    // The query is sent from `params`; if the URL also carries one (e.g. older
    // imports), drop it so each value isn't sent twice (which APIs read as an array).
    let will_add_query = req.params.iter().any(|p| p.enabled && !p.key.is_empty())
        || (req.auth_type == AuthType::ApiKey
            && req.api_key_location == ApiKeyLocation::Query
            && !req.api_key_name.is_empty());
    let url = match url.find('?') {
        Some(i) if will_add_query => url[..i].to_string(),
        _ => url,
    };
    let method = Method::from_str(req.method.as_str())
        .map_err(|_| format!("Invalid HTTP method: {}", req.method))?;

    let mut builder = client.request(method, &url);
    let timeout_ms = if default_timeout_ms < 1000 { 30_000 } else { default_timeout_ms };
    builder = builder.timeout(Duration::from_millis(timeout_ms));

    let mut header_map = HeaderMap::new();
    for h in &req.headers {
        if !h.enabled || h.key.is_empty() {
            continue;
        }
        let k = substitute(&h.key, env);
        let v = substitute(&h.value, env);
        let name = HeaderName::from_str(&k).map_err(|_| format!("Invalid header: {k}"))?;
        let val = HeaderValue::from_str(&v).map_err(|_| format!("Invalid header value: {v}"))?;
        header_map.insert(name, val);
    }

    match &req.auth_type {
        AuthType::Bearer if !req.bearer_token.is_empty() => {
            let val = HeaderValue::from_str(&format!("Bearer {}", req.bearer_token))
                .map_err(|e| e.to_string())?;
            header_map.insert(HeaderName::from_static("authorization"), val);
        }
        AuthType::Basic if !req.basic_user.is_empty() => {
            let credentials = format!("{}:{}", req.basic_user, req.basic_pass);
            let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
            let val = HeaderValue::from_str(&format!("Basic {encoded}"))
                .map_err(|e| e.to_string())?;
            header_map.insert(HeaderName::from_static("authorization"), val);
        }
        AuthType::ApiKey
            if !req.api_key_name.is_empty()
                && req.api_key_location == ApiKeyLocation::Header =>
        {
            let name = HeaderName::from_str(&req.api_key_name)
                .map_err(|_| format!("Invalid API key header name: {}", req.api_key_name))?;
            let val = HeaderValue::from_str(&req.api_key_value).map_err(|e| e.to_string())?;
            header_map.insert(name, val);
        }
        AuthType::Cookie if !req.cookie_string.is_empty() => {
            let val =
                HeaderValue::from_str(&req.cookie_string).map_err(|e| e.to_string())?;
            header_map.insert(HeaderName::from_static("cookie"), val);
        }
        AuthType::JwtUser if !req.jwt_secret.is_empty() => {
            let token = make_jwt(&req.jwt_subject, &req.jwt_secret)?;
            let val = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| e.to_string())?;
            header_map.insert(HeaderName::from_static("authorization"), val);
        }
        _ => {}
    }

    if matches!(req.body_type, BodyType::Json)
        && !header_map.contains_key(reqwest::header::CONTENT_TYPE)
    {
        header_map.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }

    builder = builder.headers(header_map);

    let mut query: Vec<(String, String)> = req
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.is_empty())
        .map(|p| (substitute(&p.key, env), substitute(&p.value, env)))
        .collect();

    if req.auth_type == AuthType::ApiKey
        && req.api_key_location == ApiKeyLocation::Query
        && !req.api_key_name.is_empty()
    {
        query.push((req.api_key_name.clone(), req.api_key_value.clone()));
    }

    if !query.is_empty() {
        builder = builder.query(&query);
    }

    match &req.body_type {
        BodyType::FormData => {
            let has_file = req
                .form_data_fields
                .iter()
                .any(|f| f.enabled && f.field_type == crate::domain::request::FormFieldType::File);

            if has_file {
                let mut form = reqwest::multipart::Form::new();
                for field in &req.form_data_fields {
                    if !field.enabled {
                        continue;
                    }
                    if let crate::domain::request::FormFieldType::File = field.field_type {
                        let file_data = field.file_data.as_ref().ok_or_else(|| {
                            format!("File field '{}' has no data — pick a file first", field.key)
                        })?;
                        let bytes = general_purpose::STANDARD
                            .decode(file_data)
                            .map_err(|e| format!("Base64 decode: {e}"))?;
                        let fname = field.file_name.clone().unwrap_or_else(|| "file".to_owned());
                        let mut part = reqwest::multipart::Part::bytes(bytes).file_name(fname);
                        if let Some(mime) = &field.mime_type {
                            part = part.mime_str(mime).map_err(|e| e.to_string())?;
                        }
                        form = form.part(field.key.clone(), part);
                    } else {
                        form = form.text(field.key.clone(), field.value.clone());
                    }
                }
                builder = builder.multipart(form);
            } else {
                // No file fields: send as application/x-www-form-urlencoded,
                // which is what plain HTML-style forms (logins, etc.) expect —
                // multipart/form-data would be syntactically wrong for them.
                let pairs: Vec<(String, String)> = req
                    .form_data_fields
                    .iter()
                    .filter(|f| f.enabled)
                    .map(|f| (f.key.clone(), f.value.clone()))
                    .collect();
                builder = builder.form(&pairs);
            }
        }
        BodyType::Json | BodyType::Text => {
            let body = substitute(&req.body, env);
            if !body.is_empty() {
                builder = builder.body(body);
            }
        }
        BodyType::None => {}
    }

    let response = builder.send().await.map_err(|e| describe_send_error(&e))?;

    let status = response.status().as_u16();
    let status_text = response.status().canonical_reason().unwrap_or("").to_owned();

    let mut headers = HashMap::new();
    for (name, value) in response.headers().iter() {
        if let Ok(v) = value.to_str() {
            headers
                .entry(name.to_string())
                .and_modify(|e: &mut String| {
                    e.push('\n');
                    e.push_str(v);
                })
                .or_insert_with(|| v.to_owned());
        }
    }

    let raw_bytes = response.bytes().await.map_err(|e| format!("Read body: {e}"))?;
    let body_size = raw_bytes.len();
    let is_large = body_size > INLINE_BODY_THRESHOLD;

    // Non-UTF-8 bodies (PDFs, images, other binary downloads) can't be shown
    // as text. Detect them up front instead of lossily decoding them into a
    // string full of replacement characters and feeding that into the text
    // editor / JSON pretty-printer.
    match String::from_utf8(raw_bytes.to_vec()) {
        Ok(raw_body) => Ok(HttpResponse {
            status,
            status_text,
            headers,
            body: if is_large { String::new() } else { maybe_pretty(&raw_body) },
            body_size,
            body_stored: is_large,
            is_binary: false,
            binary_data: None,
            duration_ms: 0, // filled by caller
            error: None,
        }),
        Err(_) => Ok(HttpResponse {
            status,
            status_text,
            headers,
            body: String::new(),
            body_size,
            body_stored: false,
            is_binary: true,
            binary_data: if is_large { None } else { Some(raw_bytes.to_vec()) },
            duration_ms: 0, // filled by caller
            error: None,
        }),
    }
}

fn maybe_pretty(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_owned()),
        Err(_) => raw.to_owned(),
    }
}

pub fn build_client() -> Client {
    Client::builder()
        .cookie_store(true)
        .user_agent(concat!("rustman/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("Failed to build HTTP client")
}

/// Categorise a send failure into a short, human label.
fn classify_send_error(e: &reqwest::Error) -> &'static str {
    if e.is_timeout() {
        "Request timed out"
    } else if e.is_connect() {
        "Connection failed"
    } else if e.is_redirect() {
        "Too many redirects"
    } else if e.is_body() || e.is_decode() {
        "Failed to read response"
    } else {
        "Request failed"
    }
}

/// The deepest message in an error's `source()` chain. reqwest's own `Display`
/// stops at "error sending request for url (…)" and hides the real reason (TLS
/// trust failure, DNS error, connection refused, …), so walk down to the leaf.
fn root_cause(err: &dyn std::error::Error) -> String {
    let mut cur = err;
    while let Some(src) = cur.source() {
        cur = src;
    }
    cur.to_string()
}

/// Build an actionable one-line message from a send failure: a category plus the
/// underlying cause. The leaf cause is used rather than reqwest's top-level
/// `Display` so the (possibly secret-bearing) request URL isn't echoed back.
fn describe_send_error(e: &reqwest::Error) -> String {
    format!("{}: {}", classify_send_error(e), root_cause(e))
}

#[cfg(test)]
mod error_tests {
    use super::root_cause;
    use std::fmt;

    #[derive(Debug)]
    struct Err(&'static str, Option<Box<Err>>);
    impl fmt::Display for Err {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }
    impl std::error::Error for Err {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.1.as_deref().map(|e| e as &dyn std::error::Error)
        }
    }

    #[test]
    fn walks_to_leaf_cause() {
        let chain = Err(
            "error sending request",
            Some(Box::new(Err(
                "client error (Connect)",
                Some(Box::new(Err("invalid peer certificate: UnknownIssuer", None))),
            ))),
        );
        assert_eq!(root_cause(&chain), "invalid peer certificate: UnknownIssuer");
    }

    #[test]
    fn single_error_returns_itself() {
        assert_eq!(root_cause(&Err("boom", None)), "boom");
    }
}

/// Diagnostic tests that hit the real local test server
/// (`scripts/test_file_server.py`, port 8899) to check whether a mixed
/// text+file multipart form actually goes out over the wire correctly.
/// `#[ignore]`d by default since they need that server running:
///   python3 scripts/test_file_server.py 8899 &
///   cargo test --lib -- --ignored --nocapture live_form_upload
#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::domain::collection::SavedRequest;
    use crate::domain::request::{BodyType, FormField, FormFieldType, HttpMethod};

    #[tokio::test]
    #[ignore]
    async fn live_form_upload_with_mixed_text_and_file_fields() {
        let client = build_client();
        let mut req = SavedRequest::new_in(String::new(), "test".to_owned());
        req.method = HttpMethod::Post;
        req.url = "http://localhost:8899/upload".to_owned();
        req.body_type = BodyType::FormData;
        req.form_data_fields = vec![
            FormField {
                id: "1".to_owned(),
                key: "name".to_owned(),
                value: "rustman".to_owned(),
                field_type: FormFieldType::Text,
                enabled: true,
                file_name: None,
                file_data: None,
                mime_type: None,
            },
            FormField {
                id: "2".to_owned(),
                key: "avatar".to_owned(),
                value: String::new(),
                field_type: FormFieldType::File,
                enabled: true,
                file_name: Some("avatar.png".to_owned()),
                file_data: Some(general_purpose::STANDARD.encode(b"fake png bytes")),
                mime_type: Some("image/png".to_owned()),
            },
        ];

        let result = send(&client, "tab-1".to_owned(), &req, None, 30_000).await;
        eprintln!("response: {:?}", result.response);
        assert_eq!(result.response.error, None, "request failed: {:?}", result.response.error);
        assert_eq!(result.response.status, 200);

        let parsed: serde_json::Value = serde_json::from_str(&result.response.body)
            .expect("server response should be JSON");
        eprintln!("parsed server response: {parsed:#}");

        let files = parsed["files"].as_array().expect("files array");
        let fields = parsed["fields"].as_object().expect("fields object");

        assert_eq!(fields.get("name").and_then(|v| v.as_str()), Some("rustman"));
        assert_eq!(files.len(), 1, "expected exactly one uploaded file, got: {files:?}");
        assert_eq!(files[0]["filename"], "avatar.png");
        assert_eq!(files[0]["size"], 14); // "fake png bytes".len()
    }

    #[tokio::test]
    #[ignore]
    async fn live_form_upload_sends_correct_excel_mime_type() {
        let client = build_client();
        let mut req = SavedRequest::new_in(String::new(), "test".to_owned());
        req.method = HttpMethod::Post;
        req.url = "http://localhost:8899/upload".to_owned();
        req.body_type = BodyType::FormData;
        req.form_data_fields = vec![FormField {
            id: "1".to_owned(),
            key: "report".to_owned(),
            value: String::new(),
            field_type: FormFieldType::File,
            enabled: true,
            file_name: Some("report.xlsx".to_owned()),
            file_data: Some(general_purpose::STANDARD.encode(b"fake xlsx bytes")),
            mime_type: Some(
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_owned(),
            ),
        }];

        let result = send(&client, "tab-1".to_owned(), &req, None, 30_000).await;
        assert_eq!(result.response.error, None, "request failed: {:?}", result.response.error);
        assert_eq!(result.response.status, 200);

        let parsed: serde_json::Value = serde_json::from_str(&result.response.body)
            .expect("server response should be JSON");
        eprintln!("parsed server response: {parsed:#}");

        let files = parsed["files"].as_array().expect("files array");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["filename"], "report.xlsx");
        assert_eq!(
            files[0]["content_type"],
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "the multipart part's Content-Type on the wire should be the Excel \
             MIME type, not a generic fallback like application/octet-stream"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_text_only_form_sends_urlencoded_not_multipart() {
        // Reproduces the real bug report: a plain login form (no file
        // fields) sent to a strict server that only accepts
        // application/x-www-form-urlencoded and 400s on anything else
        // (like multipart/form-data).
        let client = build_client();
        let mut req = SavedRequest::new_in(String::new(), "test".to_owned());
        req.method = HttpMethod::Post;
        req.url = "http://localhost:8899/login".to_owned();
        req.body_type = BodyType::FormData;
        req.form_data_fields = vec![
            FormField {
                id: "1".to_owned(),
                key: "conta".to_owned(),
                value: "sync_dep_1".to_owned(),
                field_type: FormFieldType::Text,
                enabled: true,
                file_name: None,
                file_data: None,
                mime_type: None,
            },
            FormField {
                id: "2".to_owned(),
                key: "modulo".to_owned(),
                value: "SYNC".to_owned(),
                field_type: FormFieldType::Text,
                enabled: true,
                file_name: None,
                file_data: None,
                mime_type: None,
            },
        ];

        let result = send(&client, "tab-1".to_owned(), &req, None, 30_000).await;
        eprintln!("response: {:?}", result.response);
        assert_eq!(result.response.error, None, "request failed: {:?}", result.response.error);
        assert_eq!(
            result.response.status, 200,
            "server rejected the request — body: {}",
            result.response.body
        );

        let parsed: serde_json::Value = serde_json::from_str(&result.response.body)
            .expect("server response should be JSON");
        assert_eq!(parsed["received_fields"]["conta"], "sync_dep_1");
        assert_eq!(parsed["received_fields"]["modulo"], "SYNC");
    }
}
