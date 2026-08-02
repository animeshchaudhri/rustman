//! A high-performance code editor widget for Iced.
//!
//! This crate provides a canvas-based code editor with syntax highlighting,
//! line numbers, and text selection capabilities for the Iced GUI framework.
//!
//! # Features
//!
//! - **Syntax highlighting** for multiple programming languages
//! - **Line numbers** with styled gutter
//! - **Text selection** via mouse drag and keyboard
//! - **Clipboard operations** (copy, paste)
//! - **Custom scrollbars** with themed styling
//! - **Focus management** for multiple editors
//! - **Dark & light themes** support with customizable colors
//! - **Undo/Redo** with command history
//! - **Optional Vim mode** with Normal, Insert, Visual, and Visual Line modes
//!
//! # Example
//!
//! ```no_run
//! use iced::widget::container;
//! use iced::{Element, Task};
//! use iced_code_editor::{CodeEditor, Message as EditorMessage};
//!
//! struct MyApp {
//!     editor: CodeEditor,
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     EditorEvent(EditorMessage),
//! }
//!
//! impl Default for MyApp {
//!     fn default() -> Self {
//!         let code = r#"fn main() {
//!     println!("Hello, world!");
//! }
//! "#;
//!
//!         Self { editor: CodeEditor::new(code, "rust") }
//!     }
//! }
//!
//! impl MyApp {
//!     fn update(&mut self, message: Message) -> Task<Message> {
//!         match message {
//!             Message::EditorEvent(event) => {
//!                 self.editor.update(&event).map(Message::EditorEvent)
//!             }
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         container(self.editor.view().map(Message::EditorEvent))
//!             .padding(20)
//!             .into()
//!     }
//! }
//!
//! fn main() -> iced::Result {
//!     iced::run(MyApp::update, MyApp::view)
//! }
//! ```
//!
//! # Themes
//!
//! The editor supports all native Iced themes with automatic color adaptation:
//!
//! ```no_run
//! use iced_code_editor::{CodeEditor, theme};
//!
//! // Create an editor (defaults to Tokyo Night Storm theme)
//! let mut editor = CodeEditor::new("fn main() {}", "rs");
//!
//! // Switch to any Iced theme
//! editor.set_theme(theme::from_iced_theme(&iced::Theme::Dracula));
//! editor.set_theme(theme::from_iced_theme(&iced::Theme::CatppuccinMocha));
//! editor.set_theme(theme::from_iced_theme(&iced::Theme::Nord));
//! ```
//!
//! # Vim Mode
//!
//! Vim behavior is disabled by default and configured per editor instance:
//!
//! ```no_run
//! use iced_code_editor::{CodeEditor, VimMode};
//!
//! let mut editor =
//!     CodeEditor::new("fn main() {}", "rs").with_vim_enabled(true);
//! assert!(editor.vim_enabled());
//! assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
//!
//! editor.set_vim_enabled(false);
//! assert_eq!(editor.vim_mode(), None);
//! ```
//!
//! A focused editor can toggle Vim behavior with `Ctrl+Alt+V`, or
//! `Command+Alt+V` on macOS. Regular `Ctrl`/`Command+V` system paste is
//! unchanged. Counts support both target lines (`5G`/`5gg` jump to logical
//! line 5) and line operators (`5yy`, `5dd`, and `5cc` affect five lines).
//! A fixed bottom status line shows the current mode, pending keys, and active
//! `/pattern` or `:N` input. Submit with Enter, edit with Backspace, or cancel
//! with Escape. `n` and `N` repeat the last search forward or backward.
//!
//! The Vim MVP provides Normal, Insert, Visual, and Visual Line modes; counts;
//! `h/j/k/l`, word and line motions; `d/c/y`, `x`, `p/P`, and `u`/`Ctrl+R`;
//! `/` search with `n`/`N`, `:N` line jumps; and a per-editor unnamed register.
//! It is single-cursor and does not include general Ex commands, regex/search
//! history, text objects, macros, named registers, marks, `.` repeat, or
//! configurable mappings. Platform clipboard shortcuts continue to use the
//! system clipboard instead of the Vim register.
//!
//! # Keyboard Shortcuts
//!
//! The editor supports a comprehensive set of keyboard shortcuts:
//!
//! ## Navigation
//!
//! | Shortcut | Action |
//! |----------|--------|
//! | **Arrow Keys** (Up, Down, Left, Right) | Move cursor |
//! | **Shift + Arrows** | Move cursor with selection |
//! | **Home** / **End** | Jump to start/end of line |
//! | **Shift + Home** / **Shift + End** | Select to start/end of line |
//! | **Ctrl + Home** / **Ctrl + End** | Jump to start/end of document |
//! | **Page Up** / **Page Down** | Scroll one page up/down |
//!
//! ## Editing
//!
//! | Shortcut | Action |
//! |----------|--------|
//! | **Backspace** | Delete character before cursor (or delete selection if text is selected) |
//! | **Delete** | Delete character after cursor (or delete selection if text is selected) |
//! | **Shift + Delete** | Delete selected text (same as Delete when selection exists) |
//! | **Enter** | Insert new line |
//!
//! ## Clipboard
//!
//! | Shortcut | Action |
//! |----------|--------|
//! | **Ctrl + C** or **Ctrl + Insert** | Copy selected text |
//! | **Ctrl + V** or **Shift + Insert** | Paste from clipboard |
//!
//! # Supported Languages
//!
//! The editor supports syntax highlighting through the `syntect` crate:
//! - Python (`"py"` or `"python"`)
//! - Lua (`"lua"`)
//! - Rust (`"rs"` or `"rust"`)
//! - JavaScript (`"js"` or `"javascript"`)
//! - And many more...
//!
//! For a complete list, refer to the `syntect` crate documentation.
//!
//! # Command History Management
//!
//! The [`CommandHistory`] type provides fine-grained control over undo/redo operations.
//! While the editor handles history automatically, you can access it directly for
//! advanced use cases:
//!
//! ## Monitoring History State
//!
//! ```no_run
//! use iced_code_editor::CommandHistory;
//!
//! let history = CommandHistory::new(100);
//!
//! // Check how many operations are available
//! println!("Undo operations: {}", history.undo_count());
//! println!("Redo operations: {}", history.redo_count());
//!
//! // Check if operations are possible
//! if history.can_undo() {
//!     println!("Can undo!");
//! }
//! ```
//!
//! ## Adjusting History Size
//!
//! You can dynamically adjust the maximum number of operations kept in history:
//!
//! ```no_run
//! use iced_code_editor::CommandHistory;
//!
//! let history = CommandHistory::new(100);
//!
//! // Get current maximum
//! assert_eq!(history.max_size(), 100);
//!
//! // Increase limit for memory-rich environments
//! history.set_max_size(500);
//!
//! // Or decrease for constrained environments
//! history.set_max_size(50);
//! ```
//!
//! ## Clearing History
//!
//! You can reset the entire history when needed:
//!
//! ```no_run
//! use iced_code_editor::CommandHistory;
//!
//! let history = CommandHistory::new(100);
//!
//! // Clear all undo/redo operations
//! history.clear();
//!
//! assert_eq!(history.undo_count(), 0);
//! assert_eq!(history.redo_count(), 0);
//! ```
//!
//! ## Save Point Tracking
//!
//! Track whether the document has been modified since the last save:
//!
//! ```no_run
//! use iced_code_editor::CommandHistory;
//!
//! let history = CommandHistory::new(100);
//!
//! // After loading or saving a file
//! history.mark_saved();
//!
//! // Check if there are unsaved changes
//! if history.is_modified() {
//!     println!("Document has unsaved changes!");
//! }
//! ```

// Initialize rust-i18n for the entire crate
rust_i18n::i18n!("locales", fallback = "en");

mod canvas_editor;
mod text_buffer;
mod text_utils;

pub mod i18n;
pub mod theme;

/// Hidden re-exports for the `criterion` benchmark harness (`benches/`).
///
/// Compiled only with the `bench` feature; not part of the public API.
#[doc(hidden)]
#[cfg(feature = "bench")]
pub use canvas_editor::bench_support;
pub use canvas_editor::folding::FoldRegion;
/// LSP integration types and traits for editor clients.
pub use canvas_editor::lsp::{
    LspClient, LspDocument, LspPosition, LspRange, LspTextChange,
};
pub use canvas_editor::{
    ArrowDirection, CodeEditor, CommandHistory, ContextMenuEntry,
    ContextMenuItem, IndentStyle, Message, VimMode,
};
pub use i18n::{Language, Translations};
pub use theme::{Catalog, Style, StyleFn, from_iced_theme};

#[cfg(all(feature = "lsp-process", not(target_arch = "wasm32")))]
pub use canvas_editor::lsp_process::{LspEvent, LspProcessClient};

#[cfg(all(feature = "lsp-process", not(target_arch = "wasm32")))]
pub use canvas_editor::lsp_process::config::{
    LspCommand, LspLanguage, LspServerConfig, ensure_rust_analyzer_config,
    lsp_language_for_extension, lsp_language_for_path, lsp_server_config,
    resolve_lsp_command,
};

#[cfg(all(feature = "lsp-process", not(target_arch = "wasm32")))]
pub use canvas_editor::lsp_process::overlay::{
    LspOverlayMessage, LspOverlayState, view_lsp_overlay,
};
