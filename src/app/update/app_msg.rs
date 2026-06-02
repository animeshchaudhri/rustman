use iced::Task;

use crate::app::session::persist_session;
use crate::app::AppState;
use crate::domain::history::HistoryEntry;
use crate::jobs::JobKind;
use crate::message::{AppMsg, FormatTarget, Message};
use crate::services::storage;

pub(super) fn handle(state: &mut AppState, msg: AppMsg) -> Task<Message> {
    match msg {
        AppMsg::HttpResponse { generation, result } => {
            if let Some(tab) = state.tabs.tabs.iter_mut().find(|t| t.id == result.tab_id) {
                if !tab.jobs.is_current(JobKind::Request, generation) {
                    return Task::none();
                }
                tab.is_loading = false;
                tab.parsed_json = None;
                tab.viewer_processing = true;
                tab.response = Some(result.response.clone());

                let entry = HistoryEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    method: tab.method.as_str().to_owned(),
                    url: tab.url.clone(),
                    status: result.response.status as i32,
                    duration_ms: result.response.duration_ms as i64,
                    request: crate::domain::collection::SavedRequest {
                        id: tab.id.clone(),
                        collection_id: String::new(),
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
                    },
                };

                let body = result.response.body.clone();
                let tab_id = tab.id.clone();
                let body_hash = crate::services::cache::ParsedBodyCache::body_hash(&body);

                if let Some(db) = &state.db {
                    let _ = storage::add_history(db, &entry);
                }
                state.history.insert(0, entry);
                if state.history.len() > 100 {
                    state.history.truncate(100);
                }
                persist_session(state);
                if state.parsed_cache.inner_get_by_hash(body_hash).is_some() {
                    let cached_clone = state.parsed_cache.inner_get_by_hash(body_hash).unwrap().clone();
                    let display = serde_json::to_string_pretty(&cached_clone)
                        .unwrap_or_else(|_| body.clone());
                    if let Some(t) = state.tabs.tabs.iter_mut().find(|t| t.id == tab_id) {
                        t.parsed_json = Some(cached_clone);
                        t.set_viewer_content(&display, true);
                        t.viewer_processing = false;
                        t.jobs.cancel(JobKind::Parse);
                    }
                    return Task::none();
                }

                let parse_generation;
                let parse_cancel;
                {
                    let t = state.tabs.tabs.iter_mut().find(|t| t.id == tab_id);
                    if let Some(t) = t {
                        let (gen, cancel) = t.jobs.start(JobKind::Parse);
                        parse_generation = gen;
                        parse_cancel = cancel;
                    } else {
                        return Task::none();
                    }
                }
                return Task::perform(
                    async move {
                        tokio::select! {
                            biased;
                            _ = parse_cancel.cancelled() => AppMsg::Noop,
                            built = tokio::task::spawn_blocking(move || {
                                let parsed = serde_json::from_str::<serde_json::Value>(&body).ok();
                                let display = parsed.as_ref()
                                    .and_then(|j| serde_json::to_string_pretty(j).ok())
                                    .unwrap_or(body);
                                (display, parsed.map(Box::new))
                            }) => match built {
                                Ok((content_text, parsed_json)) => AppMsg::ViewerReady {
                                    generation: parse_generation,
                                    tab_id,
                                    content_text,
                                    parsed_json,
                                },
                                Err(_) => AppMsg::Noop,
                            },
                        }
                    },
                    Message::App,
                );
            }
        }
        AppMsg::AvatarLoaded(bytes) => {
            state.profile_avatar = Some(iced::widget::image::Handle::from_bytes(bytes));
        }
        AppMsg::ScriptConsoleLog(_) => {}
        AppMsg::ViewerReady { generation, tab_id, content_text, parsed_json } => {
            if let Some(tab) = state.tabs.tabs.iter_mut().find(|t| t.id == tab_id) {
                if tab.jobs.is_current(JobKind::Parse, generation) {
                    let is_json = parsed_json.is_some();
                    if let (Some(parsed), Some(raw_body)) = (parsed_json.as_deref(), tab.response.as_ref().map(|r| r.body.as_str())) {
                        state.parsed_cache.insert(raw_body, parsed.clone());
                    }
                    tab.parsed_json = parsed_json.map(|b| *b);
                    tab.set_viewer_content(&content_text, is_json);
                    tab.viewer_processing = false;
                }
            }
        }
        AppMsg::Formatted { generation, tab_id, target, text } => {
            if let Some(tab) = state.tabs.tabs.iter_mut().find(|t| t.id == tab_id) {
                if tab.jobs.is_current(JobKind::Format, generation) {
                    match target {
                        FormatTarget::RequestBody => {
                            tab.reset_body_editor(&text);
                            tab.modified = true;
                        }
                        FormatTarget::ResponseBody => {
                            if let Some(r) = tab.response.as_mut() {
                                r.body = text.clone();
                            }
                            let is_json = tab.parsed_json.is_some();
                            tab.set_viewer_content(&text, is_json);
                        }
                    }
                }
            }
        }
        AppMsg::WindowCloseRequested(id) => {
            persist_session(state);
            return iced::window::close(id);
        }
        AppMsg::OpenUrl(url) => {
            let _ = open::that(url);
        }
        AppMsg::Noop => {}
    }
    Task::none()
}
