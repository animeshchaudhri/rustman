use serde::{Deserialize, Serialize};

use super::tabs::TabSnapshot;

/// Persisted session — survives app restarts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSession {
    pub tabs: Vec<TabSnapshot>,
    pub active_tab: usize,
    pub active_env_id: Option<String>,
    pub sidebar_panel: String,
    #[serde(default)]
    pub theme_idx: usize,
}
