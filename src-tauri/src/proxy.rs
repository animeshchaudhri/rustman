use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::str::FromStr;
use std::time::Duration;

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
}

#[derive(Debug, Serialize)]
pub struct ProxyResponse {
    status: u16,
    headers: Value,
    body: String,
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

#[tauri::command]
pub async fn proxy_request(request: ProxyRequest) -> Result<ProxyResponse, String> {
    
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
            println!("{}", headers_json);
            
            let body = response
                .text()
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?;
            
            Ok(ProxyResponse {
                status,
                headers: headers_json,
                body,
                error: None,
            })
        },
        Err(e) => {
            Ok(ProxyResponse {
                status: 0, 
                headers: json!({}),
                body: String::new(),
                error: Some(format!("Request failed: {}", e)),
            })
        }
    }
}
