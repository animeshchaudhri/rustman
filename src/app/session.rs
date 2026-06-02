use crate::{services::storage, state::session::AppSession};

use super::AppState;

pub(crate) fn persist_session(state: &AppState) {
    let Some(db) = &state.db else { return };
    let snapshots: Vec<_> = state.tabs.tabs.iter().map(|t| t.into()).collect();
    let session = AppSession {
        tabs: snapshots,
        active_tab: state.tabs.active,
        active_env_id: state.active_env().map(|e| e.id.clone()),
        sidebar_panel: format!("{:?}", state.sidebar.panel),
    };
    let _ = storage::save_session(db, &session);
}
