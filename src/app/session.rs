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
        theme_idx: state.theme_idx,
        default_timeout_ms: state.default_timeout_ms,
        global_pre_request_script: state.global_pre_request_editor.text(),
        global_test_script: state.global_test_editor.text(),
    };
    let _ = storage::save_session(db, &session);
}
