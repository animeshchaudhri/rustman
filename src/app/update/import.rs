use iced::Task;

use crate::app::AppState;
use crate::message::{AppMsg, ImportMsg, Message};
use crate::services::storage;

pub(super) fn handle(state: &mut AppState, msg: ImportMsg) -> Task<Message> {
    match msg {
        ImportMsg::OpenPostmanDialog => Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_title("Import Postman Collection")
                    .pick_file()
                    .await;
                if let Some(f) = file {
                    let bytes = f.read().await;
                    Some(String::from_utf8_lossy(&bytes).to_string())
                } else {
                    None
                }
            },
            |content| match content {
                Some(json) => match crate::services::import::postman::import(&json) {
                    Ok(data) => Message::Import(ImportMsg::PostmanLoaded(data)),
                    Err(e) => Message::Import(ImportMsg::Error(e)),
                },
                None => Message::App(AppMsg::Noop),
            },
        ),

        ImportMsg::OpenOpenApiDialog => Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .add_filter("JSON/YAML", &["json", "yaml", "yml"])
                    .set_title("Import OpenAPI Specification")
                    .pick_file()
                    .await;
                if let Some(f) = file {
                    let bytes = f.read().await;
                    Some(String::from_utf8_lossy(&bytes).to_string())
                } else {
                    None
                }
            },
            |content| match content {
                Some(json) => match crate::services::import::openapi::import(&json) {
                    Ok(data) => Message::Import(ImportMsg::OpenApiLoaded(data)),
                    Err(e) => Message::Import(ImportMsg::Error(e)),
                },
                None => Message::App(AppMsg::Noop),
            },
        ),

        ImportMsg::ExportCollection(col_id) => {
            let Some(col) = state.collections.iter().find(|c| c.id == col_id).cloned() else {
                return Task::none();
            };
            let reqs = state.requests.get(&col_id).cloned().unwrap_or_default();
            let json = crate::services::import::postman::export(&col, &reqs);
            let file_name = format!("{}.postman_collection.json", col.name);
            Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .set_file_name(&file_name)
                        .add_filter("JSON", &["json"])
                        .set_title("Export Postman Collection")
                        .save_file()
                        .await;
                    if let Some(f) = file {
                        let path = f.path().to_path_buf();
                        let _ = tokio::fs::write(&path, json.as_bytes()).await;
                        Some(path.to_string_lossy().to_string())
                    } else {
                        None
                    }
                },
                |path| match path {
                    Some(p) => Message::Import(ImportMsg::ExportDone(p)),
                    None => Message::App(AppMsg::Noop),
                },
            )
        }

        ImportMsg::PostmanLoaded(data) | ImportMsg::OpenApiLoaded(data) => {
            let count: usize = data.iter().map(|(_, reqs)| reqs.len()).sum();
            for (col, reqs) in data {
                if let Some(db) = &state.db {
                    let _ = storage::create_collection(db, &col);
                    for req in &reqs {
                        let _ = storage::create_request(db, req);
                    }
                }
                state.requests.insert(col.id.clone(), reqs);
                state.collections.push(col);
            }
            state.status_message = Some(format!("Imported {count} requests"));
            Task::none()
        }

        ImportMsg::ExportDone(path) => {
            state.status_message = Some(format!("Exported to {path}"));
            Task::none()
        }

        ImportMsg::Error(e) => {
            state.status_message = Some(format!("Import error: {e}"));
            Task::none()
        }
    }
}
