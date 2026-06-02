use serde::Deserialize;
use std::collections::HashMap;

use crate::domain::{
    collection::{Collection, SavedRequest},
    request::{ApiKeyLocation, AuthType, BodyType, HttpMethod, KeyValue},
};


#[derive(Deserialize)]
struct OpenApiSpec {
    info: OpenApiInfo,
    paths: Option<HashMap<String, HashMap<String, OpenApiOperation>>>,
    servers: Option<Vec<OpenApiServer>>,
}

#[derive(Deserialize)]
struct OpenApiInfo {
    title: String,
}

#[derive(Deserialize)]
struct OpenApiServer {
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenApiOperation {
    summary: Option<String>,
    operation_id: Option<String>,
    parameters: Option<Vec<OpenApiParameter>>,
    request_body: Option<OpenApiRequestBody>,
}

#[derive(Deserialize)]
struct OpenApiParameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    #[serde(default)]
    required: bool,
}

#[derive(Deserialize)]
struct OpenApiRequestBody {
    content: HashMap<String, OpenApiMediaType>,
}

#[derive(Deserialize)]
struct OpenApiMediaType {
    example: Option<serde_json::Value>,
}

// ── Import ────────────────────────────────────────────────────────────────────

pub fn import(json: &str) -> Result<Vec<(Collection, Vec<SavedRequest>)>, String> {
    let spec: OpenApiSpec =
        serde_json::from_str(json).map_err(|e| format!("Invalid OpenAPI JSON: {e}"))?;

    let base_url = spec
        .servers
        .as_ref()
        .and_then(|s| s.first())
        .map(|s| s.url.trim_end_matches('/').to_owned())
        .unwrap_or_default();

    let collection = Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: spec.info.title,
        created_at: chrono::Utc::now().timestamp_millis(),
    };

    let mut reqs = Vec::new();
    if let Some(paths) = &spec.paths {
        let mut sorted_paths: Vec<_> = paths.iter().collect();
        sorted_paths.sort_by_key(|(p, _)| p.as_str());

        for (path, methods) in sorted_paths {
            let mut sorted_methods: Vec<_> = methods.iter().collect();
            sorted_methods.sort_by_key(|(m, _)| m.as_str());

            for (method_str, op) in sorted_methods {
                if matches!(method_str.as_str(), "parameters" | "summary" | "description") {
                    continue;
                }

                let method: HttpMethod = method_str.parse().unwrap_or(HttpMethod::Get);
                let url = format!("{}{}", base_url, path);
                let name = op
                    .summary
                    .clone()
                    .or_else(|| op.operation_id.clone())
                    .unwrap_or_else(|| format!("{} {}", method_str.to_uppercase(), path));

                let mut query_params: Vec<KeyValue> = Vec::new();
                let mut header_params: Vec<KeyValue> = Vec::new();

                for param in op.parameters.as_deref().unwrap_or(&[]) {
                    match param.location.as_str() {
                        "query" => query_params.push(KeyValue {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: param.name.clone(),
                            value: String::new(),
                            enabled: param.required,
                        }),
                        "header" => header_params.push(KeyValue {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: param.name.clone(),
                            value: String::new(),
                            enabled: param.required,
                        }),
                        _ => {}
                    }
                }

                let (body, body_type) = if let Some(rb) = &op.request_body {
                    if let Some(json_type) = rb.content.get("application/json") {
                        let example = json_type
                            .example
                            .as_ref()
                            .map(|e| serde_json::to_string_pretty(e).unwrap_or_default())
                            .unwrap_or_default();
                        (example, BodyType::Json)
                    } else {
                        (String::new(), BodyType::None)
                    }
                } else {
                    (String::new(), BodyType::None)
                };

                reqs.push(SavedRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    collection_id: collection.id.clone(),
                    name,
                    method,
                    url,
                    headers: header_params,
                    params: query_params,
                    body,
                    body_type,
                    auth_type: AuthType::None,
                    bearer_token: String::new(),
                    basic_user: String::new(),
                    basic_pass: String::new(),
                    api_key_name: String::new(),
                    api_key_value: String::new(),
                    api_key_location: ApiKeyLocation::Header,
                    form_data_fields: vec![],
                    cookie_string: String::new(),
                    cookies: vec![],
                    jwt_secret: String::new(),
                    jwt_subject: String::new(),
                    jwt_algo: "HS256".to_owned(),
                    pre_request_script: String::new(),
                    test_script: String::new(),
                });
            }
        }
    }

    Ok(vec![(collection, reqs)])
}
