use crate::message::SidebarPanel;

#[derive(Debug, Clone)]
pub struct SidebarState {
    pub panel: SidebarPanel,
    /// Expanded collection IDs
    pub expanded: std::collections::HashSet<String>,
    /// Selected request ID (highlighted in tree)
    pub selected_request: Option<String>,
    /// Environment currently open for variable editing
    pub env_editing: Option<String>,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            panel: SidebarPanel::Collections,
            expanded: std::collections::HashSet::new(),
            selected_request: None,
            env_editing: None,
        }
    }
}
