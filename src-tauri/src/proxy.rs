use anyhow::{Context, Result};
use reqwest::{Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::str::FromStr;
use std::time::Duration;

// Request structure that will be sent from the frontend
#[derive(Debug, Deserialize)]
pub struct ProxyRequest {
    url: String,
    method: String,
    headers: Option<Value>, // Using Value for flexibility in header format
    body: Option<String>,
    timeout: Option<u64>, // timeout in milliseconds
}

// Response structure that will be sent back to the frontend
#[derive(Debug, Serialize)]
pub struct ProxyResponse {
    status: u16,
    headers: Value,
    body: String,
    error: Option<String>,
}

// Helper function to convert serde_json::Value headers to reqwest::HeaderMap
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

// Helper function to convert reqwest::HeaderMap to a Value for serialization
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

// The main proxy function that will be exposed as a Tauri command
#[tauri::command]
pub async fn proxy_request(request: ProxyRequest) -> Result<ProxyResponse, String> {
    // Build client with optional timeout
    let mut client_builder = Client::builder();
    if let Some(timeout) = request.timeout {
        client_builder = client_builder.timeout(Duration::from_millis(timeout));
    }
    
    let client = client_builder
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    
    // Parse HTTP method
    let method = Method::from_str(&request.method.to_uppercase())
        .map_err(|e| format!("Invalid HTTP method '{}': {}", request.method, e))?;
    
    // Build request
    let mut req_builder = client.request(method.clone(), &request.url);
    
    // Add headers if provided
    if let Some(headers) = &request.headers {
        let header_map = convert_headers(headers.clone())
            .map_err(|e| format!("Header conversion error: {}", e))?;
        req_builder = req_builder.headers(header_map);
    }
    
    // Add body for appropriate methods
    if let Some(body) = &request.body {
        if matches!(
            method,
            Method::POST | Method::PUT | Method::PATCH | Method::DELETE
        ) {
            req_builder = req_builder.body(body.clone());
        }
    }
    
    // Execute request
    match req_builder.send().await {
        Ok(response) => {
            // Extract status code
            let status = response.status().as_u16();
            
            // Extract headers
            let headers_json = headers_to_json(response.headers());
            println!("{}", headers_json);
            // Get response body as text
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
                status: 0, // Use 0 to indicate a connection error
                headers: json!({}),
                body: String::new(),
                error: Some(format!("Request failed: {}", e)),
            })
        }
    }
}
