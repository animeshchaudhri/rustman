use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct AppDb(pub Mutex<Connection>);

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbCollection {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbSavedRequest {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: String,
    pub params: String,
    pub body: String,
    pub body_type: String,
    pub auth_type: String,
    pub bearer_token: String,
    pub basic_user: String,
    pub basic_pass: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_location: String,
    pub form_data_fields: String,
    pub cookie_string: String,
    pub cookies: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbHistoryEntry {
    pub id: String,
    pub timestamp: i64,
    pub method: String,
    pub url: String,
    pub status: i32,
    pub duration: i64,
    pub request: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DbEnvironment {
    pub id: String,
    pub name: String,
    pub variables: String,
    pub is_active: bool,
}

pub fn init_db(app: &AppHandle) -> SqlResult<Connection> {
    let data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");
    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("rustman.db");
    let conn = Connection::open(&db_path)?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS requests (
            id TEXT PRIMARY KEY,
            collection_id TEXT NOT NULL,
            name TEXT NOT NULL,
            method TEXT NOT NULL DEFAULT 'GET',
            url TEXT NOT NULL DEFAULT '',
            headers TEXT NOT NULL DEFAULT '[]',
            params TEXT NOT NULL DEFAULT '[]',
            body TEXT NOT NULL DEFAULT '',
            body_type TEXT NOT NULL DEFAULT 'none',
            auth_type TEXT NOT NULL DEFAULT 'none',
            bearer_token TEXT NOT NULL DEFAULT '',
            basic_user TEXT NOT NULL DEFAULT '',
            basic_pass TEXT NOT NULL DEFAULT '',
            api_key_name TEXT NOT NULL DEFAULT '',
            api_key_value TEXT NOT NULL DEFAULT '',
            api_key_location TEXT NOT NULL DEFAULT 'header',
            form_data_fields TEXT NOT NULL DEFAULT '[]',
            cookie_string TEXT NOT NULL DEFAULT '',
            cookies TEXT NOT NULL DEFAULT '[]',
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS history (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            method TEXT NOT NULL,
            url TEXT NOT NULL,
            status INTEGER NOT NULL DEFAULT 0,
            duration INTEGER NOT NULL DEFAULT 0,
            request TEXT NOT NULL DEFAULT '{}'
        );

        CREATE TABLE IF NOT EXISTS environments (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            variables TEXT NOT NULL DEFAULT '{}',
            is_active INTEGER NOT NULL DEFAULT 0
        );
        ",
    )?;

    Ok(conn)
}

// ── Helper macro to collect rows without borrow checker issues ─────────────

macro_rules! query_rows {
    ($conn:expr, $sql:expr, $params:expr, $map:expr, $T:ty) => {{
        let mut stmt = $conn.prepare($sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query($params).map_err(|e| e.to_string())?;
        let mut results: Vec<$T> = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            results.push($map(row).map_err(|e: rusqlite::Error| e.to_string())?);
        }
        Ok(results)
    }};
}

// ── Collections ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn db_get_collections(state: tauri::State<'_, AppDb>) -> Result<Vec<DbCollection>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    query_rows!(
        conn,
        "SELECT id, name, created_at FROM collections ORDER BY created_at ASC",
        [],
        |row: &rusqlite::Row| -> SqlResult<DbCollection> {
            Ok(DbCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        },
        DbCollection
    )
}

#[tauri::command]
pub fn db_create_collection(
    state: tauri::State<'_, AppDb>,
    id: String,
    name: String,
    #[allow(non_snake_case)] createdAt: i64,
) -> Result<DbCollection, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO collections (id, name, created_at) VALUES (?1, ?2, ?3)",
        params![id, name, createdAt],
    )
    .map_err(|e| e.to_string())?;
    Ok(DbCollection {
        id,
        name,
        created_at: createdAt,
    })
}

#[tauri::command]
pub fn db_update_collection(
    state: tauri::State<'_, AppDb>,
    id: String,
    name: String,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE collections SET name = ?1 WHERE id = ?2",
        params![name, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_delete_collection(state: tauri::State<'_, AppDb>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM requests WHERE collection_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM collections WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Requests ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn db_get_requests(
    state: tauri::State<'_, AppDb>,
    #[allow(non_snake_case)] collectionId: String,
) -> Result<Vec<DbSavedRequest>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let sql = "SELECT id,collection_id,name,method,url,headers,params,body,body_type,auth_type,\
               bearer_token,basic_user,basic_pass,api_key_name,api_key_value,api_key_location,\
               form_data_fields,cookie_string,cookies \
               FROM requests WHERE collection_id=?1 ORDER BY name ASC";
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![collectionId]).map_err(|e| e.to_string())?;
    let mut results: Vec<DbSavedRequest> = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        results.push(DbSavedRequest {
            id: row.get(0).map_err(|e: rusqlite::Error| e.to_string())?,
            collection_id: row.get(1).map_err(|e: rusqlite::Error| e.to_string())?,
            name: row.get(2).map_err(|e: rusqlite::Error| e.to_string())?,
            method: row.get(3).map_err(|e: rusqlite::Error| e.to_string())?,
            url: row.get(4).map_err(|e: rusqlite::Error| e.to_string())?,
            headers: row.get(5).map_err(|e: rusqlite::Error| e.to_string())?,
            params: row.get(6).map_err(|e: rusqlite::Error| e.to_string())?,
            body: row.get(7).map_err(|e: rusqlite::Error| e.to_string())?,
            body_type: row.get(8).map_err(|e: rusqlite::Error| e.to_string())?,
            auth_type: row.get(9).map_err(|e: rusqlite::Error| e.to_string())?,
            bearer_token: row.get(10).map_err(|e: rusqlite::Error| e.to_string())?,
            basic_user: row.get(11).map_err(|e: rusqlite::Error| e.to_string())?,
            basic_pass: row.get(12).map_err(|e: rusqlite::Error| e.to_string())?,
            api_key_name: row.get(13).map_err(|e: rusqlite::Error| e.to_string())?,
            api_key_value: row.get(14).map_err(|e: rusqlite::Error| e.to_string())?,
            api_key_location: row.get(15).map_err(|e: rusqlite::Error| e.to_string())?,
            form_data_fields: row.get(16).map_err(|e: rusqlite::Error| e.to_string())?,
            cookie_string: row.get(17).map_err(|e: rusqlite::Error| e.to_string())?,
            cookies: row.get(18).map_err(|e: rusqlite::Error| e.to_string())?,
        });
    }
    Ok(results)
}

#[tauri::command]
pub fn db_save_request(
    state: tauri::State<'_, AppDb>,
    req: DbSavedRequest,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO requests \
         (id,collection_id,name,method,url,headers,params,body,body_type,auth_type,\
          bearer_token,basic_user,basic_pass,api_key_name,api_key_value,api_key_location,\
          form_data_fields,cookie_string,cookies) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
        params![
            req.id,
            req.collection_id,
            req.name,
            req.method,
            req.url,
            req.headers,
            req.params,
            req.body,
            req.body_type,
            req.auth_type,
            req.bearer_token,
            req.basic_user,
            req.basic_pass,
            req.api_key_name,
            req.api_key_value,
            req.api_key_location,
            req.form_data_fields,
            req.cookie_string,
            req.cookies
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_delete_request(state: tauri::State<'_, AppDb>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM requests WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── History ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn db_get_history(state: tauri::State<'_, AppDb>) -> Result<Vec<DbHistoryEntry>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let sql = "SELECT id,timestamp,method,url,status,duration,request \
               FROM history ORDER BY timestamp DESC LIMIT 100";
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut results: Vec<DbHistoryEntry> = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        results.push(DbHistoryEntry {
            id: row.get(0).map_err(|e: rusqlite::Error| e.to_string())?,
            timestamp: row.get(1).map_err(|e: rusqlite::Error| e.to_string())?,
            method: row.get(2).map_err(|e: rusqlite::Error| e.to_string())?,
            url: row.get(3).map_err(|e: rusqlite::Error| e.to_string())?,
            status: row.get(4).map_err(|e: rusqlite::Error| e.to_string())?,
            duration: row.get(5).map_err(|e: rusqlite::Error| e.to_string())?,
            request: row.get(6).map_err(|e: rusqlite::Error| e.to_string())?,
        });
    }
    Ok(results)
}

#[tauri::command]
pub fn db_add_history(
    state: tauri::State<'_, AppDb>,
    entry: DbHistoryEntry,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO history (id,timestamp,method,url,status,duration,request) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            entry.id,
            entry.timestamp,
            entry.method,
            entry.url,
            entry.status,
            entry.duration,
            entry.request
        ],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM history WHERE id NOT IN \
         (SELECT id FROM history ORDER BY timestamp DESC LIMIT 100)",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_clear_history(state: tauri::State<'_, AppDb>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Environments ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn db_get_environments(state: tauri::State<'_, AppDb>) -> Result<Vec<DbEnvironment>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let sql = "SELECT id,name,variables,is_active FROM environments \
               ORDER BY is_active DESC, name ASC";
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let mut results: Vec<DbEnvironment> = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let is_active_i: i32 = row.get(3).map_err(|e: rusqlite::Error| e.to_string())?;
        results.push(DbEnvironment {
            id: row.get(0).map_err(|e: rusqlite::Error| e.to_string())?,
            name: row.get(1).map_err(|e: rusqlite::Error| e.to_string())?,
            variables: row.get(2).map_err(|e: rusqlite::Error| e.to_string())?,
            is_active: is_active_i != 0,
        });
    }
    Ok(results)
}

#[tauri::command]
pub fn db_save_environment(
    state: tauri::State<'_, AppDb>,
    env: DbEnvironment,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    if env.is_active {
        conn.execute("UPDATE environments SET is_active=0", [])
            .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO environments (id,name,variables,is_active) VALUES (?1,?2,?3,?4)",
        params![env.id, env.name, env.variables, env.is_active as i32],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn db_delete_environment(state: tauri::State<'_, AppDb>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM environments WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
