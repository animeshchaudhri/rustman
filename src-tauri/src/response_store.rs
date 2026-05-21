use std::collections::HashMap;
use std::sync::Mutex;
use serde::Serialize;

pub struct BodyStore(pub Mutex<HashMap<String, String>>);

#[derive(Serialize)]
pub struct BodySlice {
    pub lines: Vec<String>,
    #[serde(rename = "totalLines")]
    pub total_lines: usize,
}

#[tauri::command]
pub fn body_store(
    state: tauri::State<'_, BodyStore>,
    id: String,
    text: String,
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    map.insert(id, text);
    Ok(())
}

#[tauri::command]
pub fn body_get_slice(
    state: tauri::State<'_, BodyStore>,
    id: String,
    #[allow(non_snake_case)] lineStart: usize,
    #[allow(non_snake_case)] lineCount: usize,
) -> Result<BodySlice, String> {
    let map = state.0.lock().map_err(|e| e.to_string())?;
    let text = map.get(&id).map(|s| s.as_str()).unwrap_or("");
    let all_lines: Vec<&str> = text.split('\n').collect();
    let total_lines = all_lines.len();
    let start = lineStart.min(total_lines);
    let end = (lineStart + lineCount).min(total_lines);
    let lines = all_lines[start..end].iter().map(|s| s.to_string()).collect();
    Ok(BodySlice { lines, total_lines })
}

#[tauri::command]
pub fn body_search(
    state: tauri::State<'_, BodyStore>,
    id: String,
    query: String,
) -> Result<usize, String> {
    let map = state.0.lock().map_err(|e| e.to_string())?;
    let text = map.get(&id).map(|s| s.as_str()).unwrap_or("");
    if query.is_empty() {
        return Ok(0);
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    Ok(lower_text.matches(lower_query.as_str()).count())
}

/// Returns the 0-based line indices of every line that contains `query` (case-insensitive).
/// Capped at 50_000 results to avoid sending huge payloads.
#[tauri::command]
pub fn body_search_lines(
    state: tauri::State<'_, BodyStore>,
    id: String,
    query: String,
) -> Result<Vec<usize>, String> {
    let map = state.0.lock().map_err(|e| e.to_string())?;
    let text = map.get(&id).map(|s| s.as_str()).unwrap_or("");
    if query.is_empty() {
        return Ok(vec![]);
    }
    let lower_query = query.to_lowercase();
    let results: Vec<usize> = text
        .split('\n')
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains(lower_query.as_str()))
        .map(|(i, _)| i)
        .take(50_000)
        .collect();
    Ok(results)
}

/// Remove all keys that start with `prefix` (e.g. the tab ID).
#[tauri::command]
pub fn body_clear_prefix(
    state: tauri::State<'_, BodyStore>,
    prefix: String,
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    map.retain(|k, _| !k.starts_with(&prefix));
    Ok(())
}

/// Return the full stored text for a key (used for copy-to-clipboard).
#[tauri::command]
pub fn body_get_full(
    state: tauri::State<'_, BodyStore>,
    id: String,
) -> Result<String, String> {
    let map = state.0.lock().map_err(|e| e.to_string())?;
    Ok(map.get(&id).cloned().unwrap_or_default())
}
