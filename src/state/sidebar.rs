use crate::message::SidebarPanel;

#[derive(Debug, Clone)]
pub struct SidebarState {
    pub panel: SidebarPanel,
    pub expanded: std::collections::HashSet<String>,
    pub selected_request: Option<String>,
  
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
