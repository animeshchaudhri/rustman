use serde::{Deserialize, Serialize};

use super::collection::SavedRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: i64,
    pub method: String,
    pub url: String,
    pub status: i32,
    pub duration_ms: i64,
    pub request: SavedRequest,
}
