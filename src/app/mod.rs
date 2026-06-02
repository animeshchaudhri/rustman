use std::collections::HashMap;
use std::path::PathBuf;

use iced::{Element, Size};
use rusqlite::Connection;

use crate::{
    domain::{
        collection::{Collection, SavedRequest},
        environment::AppEnvironment,
        history::HistoryEntry,
    },
    message::Message,
    state::{sidebar::SidebarState, tabs::TabManager},
};

mod boot;
mod request_ops;
mod session;
mod subscription;
mod update;

pub struct AppState {
    pub tabs: TabManager,
    pub sidebar: SidebarState,
    pub collections: Vec<Collection>,
    pub requests: HashMap<String, Vec<SavedRequest>>,
    pub history: Vec<HistoryEntry>,
    pub environments: Vec<AppEnvironment>,
    pub http_client: reqwest::Client,
    pub db: Option<Connection>,
    pub data_dir: PathBuf,
    pub status_message: Option<String>,
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_selected: usize,
    pub profile_avatar: Option<iced::widget::image::Handle>,
    /// LRU cache for parsed response JSON — keyed by body hash, 20-slot cap.
    pub parsed_cache: crate::services::cache::ParsedBodyCache,
    pub save_dialog_open: bool,
    pub save_dialog_name: String,
    pub save_dialog_collection_id: Option<String>,
    pub save_dialog_new_col: bool,
    pub save_dialog_new_col_name: String,
    pub git_log: Vec<crate::services::vcs::CommitInfo>,
    pub curl_modal_open: bool,
    pub curl_modal_command: String,
    pub github_username: String,
    pub github_email: String,
    pub github_website: String,
    pub accent_idx: usize,
    pub theme_is_dark: bool,
    /// Split ratio: request panel FillPortion (1-9). Response = 10 - panel_split.
    pub panel_split: u16,
    /// UI zoom level (0.7 – 2.0). Applied via iced scale_factor.
    pub ui_scale: f64,
}

impl AppState {
    pub(crate) fn active_env(&self) -> Option<&AppEnvironment> {
        self.environments.iter().find(|e| e.is_active)
    }
}

pub fn run() -> iced::Result {
    let window = iced::window::Settings {
        size: Size::new(1280.0, 800.0),
        ..iced::window::Settings::default()
    };
    iced::application(boot::init, update::update, view)
        .title("Rustman")
        .font(include_bytes!("../../assets/fonts/lucide.ttf").as_slice())
        .font(include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf").as_slice())
        .window(window)
        .subscription(subscription::subscription)
        .theme(app_theme)
        .scale_factor(|state: &AppState| state.ui_scale as f32)
        .exit_on_close_request(false)
        .run()
}

fn app_theme(_state: &AppState) -> iced::Theme {
    iced::Theme::TokyoNightStorm
}

fn view(state: &AppState) -> Element<'_, Message> {
    crate::ui::layout::view(state)
}
