use crate::message::SidebarPanel;

#[derive(Debug, Clone)]
pub struct SidebarState {
    pub panel: SidebarPanel,
    pub expanded: std::collections::HashSet<String>,
    pub selected_request: Option<String>,
    pub col_renaming: Option<String>,
    pub req_renaming: Option<String>,
    pub env_editing: Option<String>,
    pub env_edit_rows: Vec<(String, String)>,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            panel: SidebarPanel::Collections,
            expanded: std::collections::HashSet::new(),
            selected_request: None,
            col_renaming: None,
            req_renaming: None,
            env_editing: None,
            env_edit_rows: Vec::new(),
        }
    }
}
