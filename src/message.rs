use crate::domain::{collection::SavedRequest, history::HistoryEntry};
use crate::services::http::HttpResult;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(SidebarMsg),
    Request(RequestMsg),
    Response(ResponseMsg),
    Storage(StorageMsg),
    WebSocket(WsMsg),
    Palette(PaletteMsg),
    SaveDialog(SaveDialogMsg),
    Git(GitMsg),
    Import(ImportMsg),
    App(AppMsg),
    Settings(SettingsMsg),
    Layout(LayoutMsg),
    Update(UpdateMsg),
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SidebarMsg {
    PanelSelected(SidebarPanel),
    CollectionToggled(String),
    RequestOpened(SavedRequest),
    NewCollection,
    RenameCollection { id: String, name: String },
    DeleteCollection(String),
    DeleteRequest { id: String, collection_id: String },
    HistoryEntryOpened(HistoryEntry),
    ClearHistory,
    EnvironmentSelected(String),
    EnvironmentCreated,
    EnvironmentDeleted(String),
    EnvironmentToggleEdit(String),
    EnvironmentNameChanged(String, String),
    EnvironmentVarAdded(String),
    EnvironmentVarKeyChanged(String, usize, String),
    EnvironmentVarValueChanged(String, usize, String),
    EnvironmentVarRemoved(String, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarPanel {
    Collections,
    History,
    Environments,
    Git,
    Settings,
}

// ── Request ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum RequestMsg {
    UrlChanged(String),
    MethodChanged(String),
    TabSelected(RequestTab),
    // Headers
    HeaderToggled(usize),
    HeaderKeyChanged(usize, String),
    HeaderValueChanged(usize, String),
    HeaderAdded,
    HeaderRemoved(usize),
    HeadersBulkToggle,
    HeadersBulkEdited(iced::widget::text_editor::Action),
    // Params
    ParamToggled(usize),
    ParamKeyChanged(usize, String),
    ParamValueChanged(usize, String),
    ParamAdded,
    ParamRemoved(usize),
    ParamsBulkToggle,
    ParamsBulkEdited(iced::widget::text_editor::Action),
    // Body
    BodyTypeChanged(String),
    BodyEdited(iced_code_editor::Message),
    FormFieldAdded,
    FormFieldRemoved(usize),
    FormFieldKeyChanged(usize, String),
    FormFieldValueChanged(usize, String),
    // Auth
    AuthTypeChanged(String),
    BearerTokenChanged(String),
    BasicUserChanged(String),
    BasicPassChanged(String),
    ApiKeyNameChanged(String),
    ApiKeyValueChanged(String),
    ApiKeyLocationChanged(String),
    CookieStringChanged(String),
    // JWT auth
    JwtSecretChanged(String),
    JwtSubjectChanged(String),
    JwtAlgoChanged(String),
    // Form file upload
    FormFieldTypeToggled(usize),
    FormFieldPickFile(usize),                   // open file dialog for row index
    FormFieldFilePicked(usize, String, String), // index, filename, base64_data
    // Scripts
    PreRequestScriptEdited(iced::widget::text_editor::Action),
    TestScriptEdited(iced::widget::text_editor::Action),
    // WebSocket
    WsUrlChanged(String),
    WsConnect,
    WsDisconnect,
    WsMessageChanged(String),
    WsSend,
    // Tabs
    NewTab,
    CloseTab(usize),
    CloseCurrentTab,
    SwitchTab(usize),
    // Actions
    Send,
    Abort,
    Undo,
    Redo,
    SaveRequest,
    ImportCurl(String),
    ExportCurl,
    CloseCurlModal,
    CopyCurlToClipboard,
    FormatBody,
    ToggleBodyIndentStyle,
    CommentToggle,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum RequestTab {
    #[default]
    Params,
    Headers,
    Body,
    Auth,
    Scripts,
    WebSocket,
}

// ── Response ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ResponseMsg {
    TabSelected(ResponseTab),
    CopyBody,
    CopyValue(String),
    ViewerEdited(iced_code_editor::Message),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
    Cookies,
    Tests,
    Console,
}

// ── Storage ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StorageMsg {
    Loaded(Box<crate::state::session::AppSession>),
    Saved,
    Error(String),
}

// ── WebSocket ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum WsMsg {
    Connected,
    TextFrame(String),
    BinaryFrame(Vec<u8>),
    Disconnected,
    Error(String),
    Handshake { tab_id: String, sender: mpsc::Sender<String> },
}

// ── Git panel ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum GitMsg {
    LogLoaded(Vec<crate::services::vcs::CommitInfo>),
    CommitAll,
    Committed(String),
    Error(String),
}

// ── Save dialog ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SaveDialogMsg {
    Open,
    Close,
    NameChanged(String),
    CollectionSelected(String),
    ToggleNewCollection,
    NewCollectionNameChanged(String),
    Confirm,
}

// ── Command palette ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PaletteMsg {
    Open,
    Close,
    QueryChanged(String),
    MoveDown,
    MoveUp,
    Confirm,
    ConfirmAt(usize),
}

// ── Import / Export ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ImportMsg {
    OpenPostmanDialog,
    OpenOpenApiDialog,
    ExportCollection(String),
    PostmanLoaded(Vec<(crate::domain::collection::Collection, Vec<crate::domain::collection::SavedRequest>)>),
    OpenApiLoaded(Vec<(crate::domain::collection::Collection, Vec<crate::domain::collection::SavedRequest>)>),
    ExportDone(String),
    Error(String),
}

// ── App-level ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LayoutMsg {
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

#[derive(Debug, Clone)]
pub enum AppMsg {
    /// A request completed. `generation` is the JobKind::Request stamp from the tab's
    /// JobManager at spawn time; the result is applied only if it is still current.
    HttpResponse { generation: u64, result: HttpResult },
    ScriptConsoleLog(String),
    AvatarLoaded(Vec<u8>),
    OpenUrl(String),
    /// Large-body viewer content ready after background build. `generation` is the
    /// JobKind::Parse stamp; a stale build (superseded by a newer response) is dropped.
    ViewerReady {
        generation: u64,
        tab_id: String,
        content_text: String,
        parsed_json: Option<Box<serde_json::Value>>,
    },
    /// Background pretty-print finished. Applied only if the JobKind::Format
    /// `generation` is still current for the tab.
    Formatted {
        generation: u64,
        tab_id: String,
        target: FormatTarget,
        text: String,
    },
    /// Window is closing — persist session before the process exits.
    WindowCloseRequested(iced::window::Id),
    Noop,
}

/// Which buffer a background [`AppMsg::Formatted`] result should be written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatTarget {
    RequestBody,
    ResponseBody,
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SettingsMsg {
    GithubChanged(String),
    EmailChanged(String),
    WebsiteChanged(String),
    AccentChanged(usize),
}

// ── Auto-update ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum UpdateMsg {
    /// Start a background check against GitHub releases.
    Check,
    /// Result of a check: `Some` when a newer version exists.
    Checked(Result<Option<crate::services::update::UpdateInfo>, String>),
    /// Download + replace the binary.
    Install,
    /// Result of an install: the new version on success.
    Installed(Result<String, String>),
    /// Hide the update banner.
    Dismiss,
    /// Relaunch into the updated binary.
    Restart,
}
