mod proxy;
mod storage;

use std::sync::Mutex;
use storage::AppDb;
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let conn = storage::init_db(app.handle()).expect("Failed to init database");
            app.manage(AppDb(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            proxy::proxy_request,
            storage::db_get_collections,
            storage::db_create_collection,
            storage::db_update_collection,
            storage::db_delete_collection,
            storage::db_get_requests,
            storage::db_save_request,
            storage::db_delete_request,
            storage::db_get_history,
            storage::db_add_history,
            storage::db_clear_history,
            storage::db_get_environments,
            storage::db_save_environment,
            storage::db_delete_environment,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
