use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::str::FromStr;
use std::time::Duration;
use crate::response_store::BodyStore;

const INLINE_BODY_THRESHOLD: usize = 50_000; // bytes: below this, return body inline too

#[derive(Debug, Deserialize)]
pub struct ProxyFormField {
    pub name: String,
    pub value: String,
    pub is_file: bool,
    pub file_name: Option<String>,
    pub file_data_base64: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProxyRequest {
    url: String,
    method: String,
    headers: Option<Value>,
    body: Option<String>,
    form_fields: Option<Vec<ProxyFormField>>,
    timeout: Option<u64>,
    tab_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProxyResponse {
    status: u16,
    headers: Value,
    /// Full body text for small responses; empty string for large ones (body is in BodyStore).
    body: String,
    /// Raw byte length of the response body.
    body_size: usize,
    /// True when the body was stored in the Rust BodyStore (frontend should fetch via body_get_slice).
    body_stored: bool,
    error: Option<String>,
}

fn convert_headers(headers_value: Value) -> Result<HeaderMap> {
    let mut header_map = HeaderMap::new();
    
    if let Value::Object(map) = headers_value {
        for (key, value) in map {
            if let Some(val_str) = value.as_str() {
                let header_name = HeaderName::from_str(&key)
                    .with_context(|| format!("Invalid header name: {}", key))?;
                    
                let header_value = HeaderValue::from_str(val_str)
                    .with_context(|| format!("Invalid header value for {}: {}", key, val_str))?;
                    
                header_map.insert(header_name, header_value);
            }
        }
    }
    
    Ok(header_map)
}

fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut map = serde_json::Map::new();
    
    for (name, value) in headers.iter() {
        if let Ok(val_str) = value.to_str() {
            map.insert(
                name.to_string(),
                Value::String(val_str.to_owned()),
            );
        }
    }
    
    Value::Object(map)
}

/// Pretty-print `raw` if it is valid JSON; otherwise return raw as-is.
fn maybe_pretty(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_owned()),
        Err(_) => raw.to_owned(),
    }
}

/// Split any lines that exceed `max_width` chars into multiple shorter lines.
/// This ensures virtual scroll + the syntax highlighter never process huge single-line blobs.
fn split_long_lines(text: &str, max_width: usize) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 { out.push('\n'); }
        if line.len() <= max_width {
            out.push_str(line);
        } else {
            let chars: Vec<char> = line.chars().collect();
            let mut start = 0;
            while start < chars.len() {
                if start > 0 { out.push('\n'); }
                let end = (start + max_width).min(chars.len());
                out.extend(&chars[start..end]);
                start = end;
            }
        }
    }
    out
}

#[tauri::command]
pub async fn proxy_request(
    body_store: tauri::State<'_, BodyStore>,
    request: ProxyRequest,
) -> Result<ProxyResponse, String> {
    
    let mut client_builder = Client::builder();
    if let Some(timeout) = request.timeout {
        client_builder = client_builder.timeout(Duration::from_millis(timeout));
    }
    
    let client = client_builder
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    
    let method = Method::from_str(&request.method.to_uppercase())
        .map_err(|e| format!("Invalid HTTP method '{}': {}", request.method, e))?;
    
    let mut req_builder = client.request(method.clone(), &request.url);
    
    if let Some(headers) = &request.headers {
        let header_map = convert_headers(headers.clone())
            .map_err(|e| format!("Header conversion error: {}", e))?;
        req_builder = req_builder.headers(header_map);
    }
    
    if let Some(fields) = request.form_fields {
        let mut form = reqwest::multipart::Form::new();
        for field in fields {
            if field.is_file {
                let bytes = match field.file_data_base64 {
                    Some(ref data) => general_purpose::STANDARD
                        .decode(data)
                        .map_err(|e| format!("Base64 decode error: {}", e))?,
                    None => Vec::new(),
                };
                let fname = field.file_name.clone().unwrap_or_else(|| "file".to_string());
                let mut part = reqwest::multipart::Part::bytes(bytes).file_name(fname);
                if let Some(mime) = field.mime_type {
                    part = part.mime_str(&mime)
                        .map_err(|e| format!("MIME type error: {}", e))?;
                }
                form = form.part(field.name, part);
            } else {
                form = form.text(field.name, field.value);
            }
        }
        req_builder = req_builder.multipart(form);
    } else if let Some(body) = &request.body {
        req_builder = req_builder.body(body.clone());
    }
    
    match req_builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let headers_json = headers_to_json(response.headers());

            let raw_body = response
                .text()
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?;

            let body_size = raw_body.len();
            let is_large = body_size > INLINE_BODY_THRESHOLD;

            // Always store in Rust BodyStore when a tab_id is provided.
            // Pretty-printing happens here in Rust (serde_json) — no JS main-thread work.
            if let Some(ref tab_id) = request.tab_id {
                let pretty = maybe_pretty(&raw_body);
                let raw_key = format!("{}:raw", tab_id);
                // raw_display splits long lines so virtual scroll + highlighter never see huge blobs
                let raw_display_key = format!("{}:raw_display", tab_id);
                let pretty_key = format!("{}:pretty", tab_id);
                if let Ok(mut map) = body_store.0.lock() {
                    map.insert(raw_display_key, split_long_lines(&raw_body, 2000));
                    map.insert(raw_key, raw_body.clone()); // original, used for copy
                    map.insert(pretty_key, pretty);
                }
            }

            Ok(ProxyResponse {
                status,
                headers: headers_json,
                // For large bodies send empty string — frontend uses BodyStore
                body: if is_large { String::new() } else { raw_body },
                body_size,
                body_stored: request.tab_id.is_some(),
                error: None,
            })
        },
        Err(e) => {
            Ok(ProxyResponse {
                status: 0,
                headers: json!({}),
                body: String::new(),
                body_size: 0,
                body_stored: false,
                error: Some(format!("Request failed: {}", e)),
            })
        }
    }
}

