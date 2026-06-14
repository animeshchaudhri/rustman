use iced::Task;

use crate::{
    domain::collection::{Collection, SavedRequest},
    jobs::JobKind,
    message::{AppMsg, FormatTarget, Message},
    services::{format, http, storage},
    state::tabs::RequestTabState,
};

use super::AppState;


pub(crate) fn format_body(tab: &mut RequestTabState, text: String, target: FormatTarget) -> Task<Message> {
    let use_tabs = tab.body_indent_tabs;
    let tab_id = tab.id.clone();
    let (generation, cancel) = tab.jobs.start(JobKind::Format);
    Task::perform(
        async move {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => AppMsg::Noop,
                out = tokio::task::spawn_blocking(move || format::pretty_json(&text, use_tabs)) => {
                    match out {
                        Ok(Some(text)) => AppMsg::Formatted { generation, tab_id, target, text },
                        _ => AppMsg::Noop,
                    }
                }
            }
        },
        Message::App,
    )
}

pub(crate) fn send_request(state: &mut AppState) -> Task<Message> {
    let tab = state.tabs.active_tab_mut();
    tab.is_loading = true;
    tab.response = None;
    let (generation, cancel) = tab.jobs.start(JobKind::Request);

    // Local addresses (loopback/RFC-1918) get http://; everything else https://.
    let raw_url = tab.url.trim().to_owned();
    let url_with_scheme = if !raw_url.is_empty()
        && !raw_url.starts_with("http://")
        && !raw_url.starts_with("https://")
        && !raw_url.starts_with("ws://")
        && !raw_url.starts_with("wss://")
    {
        let scheme = if is_local_url(&raw_url) { "http" } else { "https" };
        format!("{scheme}://{raw_url}")
    } else {
        raw_url
    };

    let req = SavedRequest {
        id: tab.id.clone(),
        collection_id: String::new(),
        name: tab.title.clone(),
        method: tab.method.clone(),
        url: url_with_scheme,
        headers: tab.headers.clone(),
        params: tab.params.clone(),
        body: tab.body_editor.content(),
        body_type: tab.body_type.clone(),
        auth_type: tab.auth_type.clone(),
        bearer_token: tab.bearer_token.clone(),
        basic_user: tab.basic_user.clone(),
        basic_pass: tab.basic_pass.clone(),
        api_key_name: tab.api_key_name.clone(),
        api_key_value: tab.api_key_value.clone(),
        api_key_location: tab.api_key_location.clone(),
        form_data_fields: tab.form_fields.clone(),
        cookie_string: tab.cookie_string.clone(),
        cookies: tab.cookies.clone(),
        jwt_secret: tab.jwt_secret.clone(),
        jwt_subject: tab.jwt_subject.clone(),
        jwt_algo: tab.jwt_algo.clone(),
        pre_request_script: tab.pre_request_editor.text(),
        test_script: tab.test_editor.text(),
        timeout_ms: tab.timeout_ms,
    };

    let tab_id = tab.id.clone();
    let client = state.http_client.clone();
    let active_env = state.active_env().cloned();

    Task::perform(
        async move {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => None,
                result = http::send(&client, tab_id, &req, active_env.as_ref()) => Some(result),
            }
        },
        move |outcome| match outcome {
            Some(result) => Message::App(AppMsg::HttpResponse { generation, result }),
            None => Message::App(AppMsg::Noop),
        },
    )
}

pub(crate) fn save_request(state: &mut AppState) -> Task<Message> {
    let tab = state.tabs.active_tab();
    let req = SavedRequest {
        id: tab.saved_as.as_ref().map(|(_, id)| id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        collection_id: tab.saved_as.as_ref().map(|(col, _)| col.clone())
            .or_else(|| state.collections.first().map(|c| c.id.clone()))
            .unwrap_or_default(),
        name: tab.title.clone(),
        method: tab.method.clone(),
        url: tab.url.clone(),
        headers: tab.headers.clone(),
        params: tab.params.clone(),
        body: tab.body_editor.content(),
        body_type: tab.body_type.clone(),
        auth_type: tab.auth_type.clone(),
        bearer_token: tab.bearer_token.clone(),
        basic_user: tab.basic_user.clone(),
        basic_pass: tab.basic_pass.clone(),
        api_key_name: tab.api_key_name.clone(),
        api_key_value: tab.api_key_value.clone(),
        api_key_location: tab.api_key_location.clone(),
        form_data_fields: tab.form_fields.clone(),
        cookie_string: tab.cookie_string.clone(),
        cookies: tab.cookies.clone(),
        jwt_secret: tab.jwt_secret.clone(),
        jwt_subject: tab.jwt_subject.clone(),
        jwt_algo: tab.jwt_algo.clone(),
        pre_request_script: tab.pre_request_editor.text(),
        test_script: tab.test_editor.text(),
        timeout_ms: tab.timeout_ms,
    };

    if req.collection_id.is_empty() {
        let existing_col_id = state
            .collections
            .iter()
            .find(|c| c.name == "My Requests")
            .map(|c| c.id.clone());

        let col_id = existing_col_id.unwrap_or_else(|| {
            let id = uuid::Uuid::new_v4().to_string();
            let col = Collection {
                id: id.clone(),
                name: "My Requests".to_owned(),
                created_at: chrono::Utc::now().timestamp_millis(),
            };
            if let Some(db) = &state.db {
                let _ = storage::create_collection(db, &col);
            }
            state.collections.push(col);
            id
        });
        let tab = state.tabs.active_tab_mut();
        tab.saved_as = Some((col_id.clone(), req.id.clone()));
        let mut req = req;
        req.collection_id = col_id.clone();
        if let Some(db) = &state.db {
            let _ = storage::create_request(db, &req);
        }
        state.requests.entry(col_id).or_default().push(req);
    } else {
        let col_id = req.collection_id.clone();
        let req_id = req.id.clone();
        if let Some(db) = &state.db {
            let _ = storage::create_request(db, &req);
        }
        let tab = state.tabs.active_tab_mut();
        tab.saved_as = Some((col_id.clone(), req_id.clone()));
        let bucket = state.requests.entry(col_id).or_default();
        if let Some(existing) = bucket.iter_mut().find(|r| r.id == req_id) {
            *existing = req;
        } else {
            bucket.push(req);
        }
    }

    state.tabs.active_tab_mut().modified = false;
    state.status_message = Some("Saved!".to_owned());
    Task::none()
}

pub(crate) fn saved_request_from_tab(
    tab: &RequestTabState,
    collection_id: String,
    id: String,
) -> SavedRequest {
    SavedRequest {
        id,
        collection_id,
        name: tab.title.clone(),
        method: tab.method.clone(),
        url: tab.url.clone(),
        headers: tab.headers.clone(),
        params: tab.params.clone(),
        body: tab.body_editor.content(),
        body_type: tab.body_type.clone(),
        auth_type: tab.auth_type.clone(),
        bearer_token: tab.bearer_token.clone(),
        basic_user: tab.basic_user.clone(),
        basic_pass: tab.basic_pass.clone(),
        api_key_name: tab.api_key_name.clone(),
        api_key_value: tab.api_key_value.clone(),
        api_key_location: tab.api_key_location.clone(),
        form_data_fields: tab.form_fields.clone(),
        cookie_string: tab.cookie_string.clone(),
        cookies: tab.cookies.clone(),
        jwt_secret: tab.jwt_secret.clone(),
        jwt_subject: tab.jwt_subject.clone(),
        jwt_algo: tab.jwt_algo.clone(),
        pre_request_script: tab.pre_request_editor.text(),
        test_script: tab.test_editor.text(),
        timeout_ms: tab.timeout_ms,
    }
}

pub(crate) fn flush_modified_tabs(state: &mut AppState) {
    let pending: Vec<SavedRequest> = state
        .tabs
        .tabs
        .iter()
        .filter(|t| t.modified && t.saved_as.is_some())
        .map(|t| {
            let (collection_id, req_id) = t.saved_as.clone().unwrap();
            saved_request_from_tab(t, collection_id, req_id)
        })
        .collect();

    for req in pending {
        if let Some(db) = &state.db {
            let _ = storage::create_request(db, &req);
        }
        let bucket = state.requests.entry(req.collection_id.clone()).or_default();
        if let Some(existing) = bucket.iter_mut().find(|r| r.id == req.id) {
            *existing = req;
        } else {
            bucket.push(req);
        }
    }

    for tab in state.tabs.tabs.iter_mut() {
        if tab.saved_as.is_some() {
            tab.modified = false;
        }
    }
}

fn is_local_url(url: &str) -> bool {
    let host = url.split('/').next().unwrap_or(url);
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("172.") // covers 172.16-31.x.x private range
}
