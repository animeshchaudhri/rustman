use std::collections::HashMap;
use std::path::PathBuf;

use rusqlite::{params, Connection, Result as SqlResult};
use serde_json;

use crate::domain::{
    collection::{Collection, SavedRequest},
    environment::AppEnvironment,
    history::HistoryEntry,
};
use crate::state::session::AppSession;

// ── DB init ───────────────────────────────────────────────────────────────────

pub fn open(data_dir: &PathBuf) -> SqlResult<Connection> {
    std::fs::create_dir_all(data_dir).ok();
    let path = data_dir.join("rustman.db");
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;

         CREATE TABLE IF NOT EXISTS collections (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             created_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS environments (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             variables TEXT NOT NULL DEFAULT '{}',
             is_active INTEGER NOT NULL DEFAULT 0
         );

         CREATE TABLE IF NOT EXISTS history (
             id TEXT PRIMARY KEY,
             timestamp INTEGER NOT NULL,
             method TEXT NOT NULL,
             url TEXT NOT NULL,
             status INTEGER NOT NULL DEFAULT 0,
             duration INTEGER NOT NULL DEFAULT 0,
             request_json TEXT NOT NULL DEFAULT '{}'
         );

         CREATE TABLE IF NOT EXISTS session (
             key TEXT PRIMARY KEY DEFAULT 'current',
             data TEXT NOT NULL DEFAULT '{}',
             saved_at INTEGER NOT NULL DEFAULT 0
         );

         CREATE TABLE IF NOT EXISTS requests (
             id TEXT PRIMARY KEY,
             collection_id TEXT NOT NULL,
             data TEXT NOT NULL DEFAULT '{}'
         );",
    )?;
    Ok(conn)
}

// ── Collections ───────────────────────────────────────────────────────────────

pub fn get_collections(conn: &Connection) -> Result<Vec<Collection>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, created_at FROM collections ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Collection { id: r.get(0)?, name: r.get(1)?, created_at: r.get(2)? })
        })
        .map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

pub fn create_collection(conn: &Connection, c: &Collection) -> Result<(), String> {
    conn.execute(
        "INSERT INTO collections (id, name, created_at) VALUES (?1, ?2, ?3)",
        params![c.id, c.name, c.created_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_collection(conn: &Connection, id: &str, name: &str) -> Result<(), String> {
    conn.execute("UPDATE collections SET name=?1 WHERE id=?2", params![name, id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_collection(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM collections WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Environments ──────────────────────────────────────────────────────────────

pub fn get_environments(conn: &Connection) -> Result<Vec<AppEnvironment>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, variables, is_active FROM environments ORDER BY name ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let vars_json: String = r.get(2)?;
            let is_active: i32 = r.get(3)?;
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, vars_json, is_active))
        })
        .map_err(|e| e.to_string())?;

    rows.map(|r| {
        r.map_err(|e| e.to_string()).and_then(|(id, name, vars_json, is_active)| {
            let variables: HashMap<String, String> =
                serde_json::from_str(&vars_json).unwrap_or_default();
            Ok(AppEnvironment { id, name, variables, is_active: is_active != 0 })
        })
    })
    .collect()
}

pub fn save_environment(conn: &Connection, env: &AppEnvironment) -> Result<(), String> {
    if env.is_active {
        conn.execute("UPDATE environments SET is_active=0", []).map_err(|e| e.to_string())?;
    }
    let vars = serde_json::to_string(&env.variables).unwrap_or_else(|_| "{}".to_owned());
    conn.execute(
        "INSERT OR REPLACE INTO environments (id,name,variables,is_active) VALUES (?1,?2,?3,?4)",
        params![env.id, env.name, vars, env.is_active as i32],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_environment(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM environments WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── History ───────────────────────────────────────────────────────────────────

pub fn get_history(conn: &Connection) -> Result<Vec<HistoryEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id,timestamp,method,url,status,duration,request_json \
             FROM history ORDER BY timestamp DESC LIMIT 100",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i32>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    rows.map(|r| {
        r.map_err(|e| e.to_string()).and_then(
            |(id, timestamp, method, url, status, duration, req_json)| {
                let request: SavedRequest =
                    serde_json::from_str(&req_json).map_err(|e| e.to_string())?;
                Ok(HistoryEntry { id, timestamp, method, url, status, duration_ms: duration, request })
            },
        )
    })
    .collect()
}

pub fn add_history(conn: &Connection, entry: &HistoryEntry) -> Result<(), String> {
    let req_json = serde_json::to_string(&entry.request).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO history (id,timestamp,method,url,status,duration,request_json) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            entry.id,
            entry.timestamp,
            entry.method,
            entry.url,
            entry.status,
            entry.duration_ms,
            req_json
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

pub fn clear_history(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM history", []).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Session ───────────────────────────────────────────────────────────────────

pub fn save_session(conn: &Connection, session: &AppSession) -> Result<(), String> {
    let data = serde_json::to_string(session).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT OR REPLACE INTO session (key, data, saved_at) VALUES ('current', ?1, ?2)",
        params![data, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_session(conn: &Connection) -> Result<Option<AppSession>, String> {
    match conn.query_row(
        "SELECT data FROM session WHERE key='current'",
        [],
        |r| r.get::<_, String>(0),
    ) {
        Ok(data) => {
            let session = serde_json::from_str(&data).map_err(|e| e.to_string())?;
            Ok(Some(session))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

// ── Requests ──────────────────────────────────────────────────────────────────

pub fn get_requests(conn: &Connection, collection_id: &str) -> Result<Vec<SavedRequest>, String> {
    let mut stmt = conn
        .prepare("SELECT data FROM requests WHERE collection_id=?1 ORDER BY rowid ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![collection_id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.map(|r| {
        r.map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str(&data).map_err(|e| e.to_string()))
    })
    .collect()
}

pub fn create_request(conn: &Connection, req: &SavedRequest) -> Result<(), String> {
    let data = serde_json::to_string(req).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO requests (id, collection_id, data) VALUES (?1, ?2, ?3)",
        params![req.id, req.collection_id, data],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_request(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM requests WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
