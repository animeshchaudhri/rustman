//! Manages a single native OS webview (via `wry`), embedded as a child of
//! the main window, used to render HTML response bodies inline.
//!
//! `wry::WebView` is not `Send` on Linux (the underlying WebKitGTK bindings
//! use raw, non-thread-safe pointers), so it can't be returned as a value
//! out of `iced::window::run`'s `Send`-bounded callback. Instead it's kept
//! in a thread-local: iced's window actions and its own `update()` calls
//! both execute on the same (main) thread in practice, so a thread-local is
//! sufficient and avoids any `unsafe impl Send`.
//!
//! Creation needs the native window handle (only obtainable via
//! `iced::window::run`), so it happens once, lazily, the first time an HTML
//! response needs previewing. Every later resize/content/visibility change
//! is a direct, synchronous call from `update()` — no `Task` required.

use std::cell::{Cell, RefCell};

use iced::window;
use wry::dpi::{LogicalPosition, LogicalSize, Position, Size};
use wry::{Rect, WebView, WebViewBuilder};

thread_local! {
    static WEBVIEW: RefCell<Option<WebView>> = const { RefCell::new(None) };
    /// Tracks what's currently loaded so a periodic bounds/visibility sync
    /// doesn't reload (and reset scroll position / flash) on every tick.
    static LAST_HTML: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Set once creation fails (e.g. no child-window support under native
    /// Wayland). Callers use this to fall back to a plain text/source view
    /// instead of leaving a permanently-empty placeholder.
    static CREATION_FAILED: Cell<bool> = const { Cell::new(false) };
}

/// Whether the embedded webview failed to attach (e.g. the platform doesn't
/// support child windows in this configuration) — callers should fall back
/// to a plain text preview instead of the (permanently empty) placeholder.
pub fn creation_failed() -> bool {
    CREATION_FAILED.with(Cell::get)
}

/// Ensures a webview exists (creating it on first call) and points it at
/// `html`, positioned over `bounds` (logical pixels) and visible.
///
/// Must be called from within `iced::window::run`'s callback the *first*
/// time (to get a native window handle to attach to); after that, updating
/// content/bounds no longer needs the window handle at all — call
/// `set_bounds`/`load_html`/`set_visible` directly instead.
fn create(window: &dyn window::Window, bounds: iced::Rectangle, html: &str) {
    let handle = match window.window_handle() {
        Ok(handle) => handle,
        Err(err) => {
            eprintln!("failed to get native window handle for embedded webview: {err}");
            return;
        }
    };
    WEBVIEW.with(|cell| {
        let webview = WebViewBuilder::new()
            .with_bounds(to_wry_rect(bounds))
            .with_html(html)
            .build_as_child(&handle);
        match webview {
            Ok(webview) => {
                *cell.borrow_mut() = Some(webview);
                LAST_HTML.with(|last| *last.borrow_mut() = Some(html.to_owned()));
            }
            Err(err) => {
                eprintln!("failed to create embedded webview: {err}");
                CREATION_FAILED.with(|failed| failed.set(true));
            }
        }
    });
}

/// Returns a one-time `Task` that creates the webview if it doesn't exist
/// yet. A no-op if it already exists, or if an earlier attempt already
/// failed (see `creation_failed`) — no point retrying every tick once we
/// know this platform/configuration can't attach a child webview.
pub fn ensure_created<Message: Send + 'static>(
    window_id: window::Id,
    bounds: iced::Rectangle,
    html: String,
) -> iced::Task<Message> {
    if exists() || creation_failed() {
        return iced::Task::none();
    }
    iced::window::run(window_id, move |window| {
        create(window, bounds, &html);
    })
    .discard()
}

pub fn exists() -> bool {
    WEBVIEW.with(|cell| cell.borrow().is_some())
}

/// Loads `html` only if it differs from what's already showing, so a
/// periodic sync doesn't repeatedly reload the same content (which would
/// reset scroll position and flash on every tick).
pub fn load_html_if_changed(html: &str) {
    let already_loaded =
        LAST_HTML.with(|last| last.borrow().as_deref() == Some(html));
    if already_loaded {
        return;
    }
    WEBVIEW.with(|cell| {
        if let Some(webview) = cell.borrow().as_ref() {
            let _ = webview.load_html(html);
        }
    });
    LAST_HTML.with(|last| *last.borrow_mut() = Some(html.to_owned()));
}

pub fn set_bounds(bounds: iced::Rectangle) {
    WEBVIEW.with(|cell| {
        if let Some(webview) = cell.borrow().as_ref() {
            let _ = webview.set_bounds(to_wry_rect(bounds));
        }
    });
}

pub fn set_visible(visible: bool) {
    WEBVIEW.with(|cell| {
        if let Some(webview) = cell.borrow().as_ref() {
            let _ = webview.set_visible(visible);
        }
    });
}

fn to_wry_rect(bounds: iced::Rectangle) -> Rect {
    Rect {
        position: Position::Logical(LogicalPosition::new(bounds.x as f64, bounds.y as f64)),
        size: Size::Logical(LogicalSize::new(bounds.width as f64, bounds.height as f64)),
    }
}
