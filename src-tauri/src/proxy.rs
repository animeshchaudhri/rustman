use base64::{engine::general_purpose, Engine as _};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::time::Duration;

use crate::response_store::BodyStore;

pub struct HttpClient(pub Client);

const INLINE_BODY_THRESHOLD: usize = 50_000;

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
    body: String,
    body_size: usize,
    body_stored: bool,
    error: Option<String>,
}

fn build_header_map(headers_value: Value) -> Result<HeaderMap, String> {
    let mut map = HeaderMap::new();
    if let Value::Object(obj) = headers_value {
        for (key, value) in obj {
            if let Some(val_str) = value.as_str() {
                let name = HeaderName::from_str(&key)
                    .map_err(|_| format!("Invalid header name: {}", key))?;
                let val = HeaderValue::from_str(val_str)
                    .map_err(|_| format!("Invalid header value for {}: {}", key, val_str))?;
                map.insert(name, val);
            }
        }
    }
    Ok(map)
}

fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for (name, value) in headers.iter() {
        if let Ok(val_str) = value.to_str() {
            map.entry(name.to_string())
                .or_default()
                .push(val_str.to_owned());
        }
    }

    let obj: serde_json::Map<String, Value> = map
        .into_iter()
        .map(|(k, vals)| {
            let v = if vals.len() == 1 {
                Value::String(vals.into_iter().next().unwrap())
            } else {
                Value::String(vals.join("\n"))
            };
            (k, v)
        })
        .collect();

    Value::Object(obj)
}

fn maybe_pretty(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_owned()),
        Err(_) => raw.to_owned(),
    }
}

fn split_long_lines(text: &str, max_width: usize) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.len() <= max_width {
            out.push_str(line);
        } else {
            let chars: Vec<char> = line.chars().collect();
            let mut start = 0;
            while start < chars.len() {
                if start > 0 {
                    out.push('\n');
                }
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
    http_client: tauri::State<'_, HttpClient>,
    body_store: tauri::State<'_, BodyStore>,
    request: ProxyRequest,
) -> Result<ProxyResponse, String> {
    let client = &http_client.0;

    let method = Method::from_str(&request.method.to_ascii_uppercase())
        .map_err(|_| format!("Invalid HTTP method: {}", request.method))?;

    let mut req_builder = client.request(method, &request.url);

    let timeout_ms = request.timeout.unwrap_or(129_600_000); // 36 hours default
    req_builder = req_builder.timeout(Duration::from_millis(timeout_ms));

    if let Some(headers) = request.headers {
        let header_map = build_header_map(headers)?;
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
                let fname = field.file_name.unwrap_or_else(|| "file".to_string());
                let mut part = reqwest::multipart::Part::bytes(bytes).file_name(fname);
                if let Some(mime) = field.mime_type {
                    part = part
                        .mime_str(&mime)
                        .map_err(|e| format!("MIME type error: {}", e))?;
                }
                form = form.part(field.name, part);
            } else {
                form = form.text(field.name, field.value);
            }
        }
        req_builder = req_builder.multipart(form);
    } else if let Some(body) = request.body {
        req_builder = req_builder.body(body);
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

            if let Some(ref tab_id) = request.tab_id {
                let pretty = maybe_pretty(&raw_body);
                let raw_display = split_long_lines(&raw_body, 2000);

                if let Ok(mut map) = body_store.0.lock() {
                    map.insert(format!("{}:raw", tab_id), raw_body.clone());
                    map.insert(format!("{}:raw_display", tab_id), raw_display);
                    map.insert(format!("{}:pretty", tab_id), pretty);
                }
            }

            Ok(ProxyResponse {
                status,
                headers: headers_json,
                body: if is_large {
                    String::new()
                } else {
                    raw_body
                },
                body_size,
                body_stored: request.tab_id.is_some(),
                error: None,
            })
        }
        Err(e) => Ok(ProxyResponse {
            status: 0,
            headers: json!({}),
            body: String::new(),
            body_size: 0,
            body_stored: false,
            error: Some(format!("Request failed: {}", e)),
        }),
    }
}
