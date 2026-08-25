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

/// How many times to attempt creation before giving up for good.
///
/// Creation can fail *transiently* — most importantly on Windows, where the
/// WebView2 runtime may still be starting up (or installing) when the first
/// HTML response arrives. Latching the very first failure forever meant one
/// unlucky moment disabled HTML preview for the rest of the process, which is
/// indistinguishable to the user from "the webview is broken". A small retry
/// budget recovers from that while still bounding the cost on platforms that
/// genuinely cannot host a child webview (e.g. native Wayland), where every
/// attempt would otherwise be retried forever.
const MAX_CREATION_ATTEMPTS: u32 = 5;

thread_local! {
    static WEBVIEW: RefCell<Option<WebView>> = const { RefCell::new(None) };
    /// Tracks what's currently loaded so a periodic bounds/visibility sync
    /// doesn't reload (and reset scroll position / flash) on every tick.
    static LAST_HTML: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Number of failed creation attempts so far.
    static CREATION_ATTEMPTS: Cell<u32> = const { Cell::new(0) };
    /// Why the last attempt failed, for reporting in the UI instead of only
    /// to stderr (a GUI build on Windows has no console attached, so an
    /// `eprintln!` there is invisible).
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Whether the embedded webview has permanently failed to attach — callers
/// should fall back to a plain text preview instead of the (permanently empty)
/// placeholder.
///
/// Only true once the retry budget is exhausted, so a transient failure (very
/// common on Windows while WebView2 initialises) no longer disables preview.
pub fn creation_failed() -> bool {
    CREATION_ATTEMPTS.with(Cell::get) >= MAX_CREATION_ATTEMPTS
}

/// The reason the last creation attempt failed, if any — shown in the preview
/// panel so the failure is diagnosable without a console.
pub fn last_error() -> Option<String> {
    LAST_ERROR.with(|err| err.borrow().clone())
}

/// Ensures a webview exists (creating it on first call) and points it at
/// `html`, positioned over `bounds` (already scaled to physical-ish logical
/// pixels by the caller — see `scaled_bounds`) and visible.
fn create(window: &dyn window::Window, bounds: iced::Rectangle, html: &str) {
    let handle = match window.window_handle() {
        Ok(handle) => handle,
        Err(err) => {
            let message = format!("no native window handle: {err}");
            eprintln!("embedded webview: {message}");
            record_failure(message);
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
                // A later success clears the record of earlier stumbles.
                CREATION_ATTEMPTS.with(|attempts| attempts.set(0));
                LAST_ERROR.with(|err| *err.borrow_mut() = None);
            }
            Err(err) => {
                let message = err.to_string();
                eprintln!("failed to create embedded webview: {message}");
                record_failure(message);
            }
        }
    });
}

fn record_failure(message: String) {
    CREATION_ATTEMPTS.with(|attempts| attempts.set(attempts.get().saturating_add(1)));
    LAST_ERROR.with(|err| *err.borrow_mut() = Some(message));
}

/// Returns a one-time `Task` that creates the webview if it doesn't exist
/// yet. A no-op if it already exists, or once the retry budget is spent (see
/// `creation_failed`) — no point retrying every tick on a platform that
/// cannot attach a child webview at all.
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


pub fn scaled_bounds(bounds: iced::Rectangle, ui_scale: f64) -> iced::Rectangle {
    let scale = if ui_scale.is_finite() && ui_scale > 0.0 {
        ui_scale as f32
    } else {
        1.0
    };
    iced::Rectangle {
        x: bounds.x * scale,
        y: bounds.y * scale,
        width: bounds.width * scale,
        height: bounds.height * scale,
    }
}

fn to_wry_rect(bounds: iced::Rectangle) -> Rect {
    Rect {
        position: Position::Logical(LogicalPosition::new(bounds.x as f64, bounds.y as f64)),
        size: Size::Logical(LogicalSize::new(bounds.width as f64, bounds.height as f64)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_bounds_applies_app_scale_factor() {
        let bounds = iced::Rectangle { x: 100.0, y: 50.0, width: 400.0, height: 300.0 };

        let scaled = scaled_bounds(bounds, 1.5);

        assert_eq!(scaled.x, 150.0);
        assert_eq!(scaled.y, 75.0);
        assert_eq!(scaled.width, 600.0);
        assert_eq!(scaled.height, 450.0);
    }

    #[test]
    fn scaled_bounds_is_identity_at_unit_scale() {
        let bounds = iced::Rectangle { x: 10.0, y: 20.0, width: 30.0, height: 40.0 };

        assert_eq!(scaled_bounds(bounds, 1.0), bounds);
    }

    #[test]
    fn scaled_bounds_rejects_nonsense_scales() {
        let bounds = iced::Rectangle { x: 10.0, y: 20.0, width: 30.0, height: 40.0 };

        // A zero, negative or non-finite scale must not collapse the webview
        // to nothing (which would look exactly like "it didn't open").
        assert_eq!(scaled_bounds(bounds, 0.0), bounds);
        assert_eq!(scaled_bounds(bounds, -2.0), bounds);
        assert_eq!(scaled_bounds(bounds, f64::NAN), bounds);
        assert_eq!(scaled_bounds(bounds, f64::INFINITY), bounds);
    }
}
