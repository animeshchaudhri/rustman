use futures_util::SinkExt as _;
use iced::event::{self, Event};
use iced::{keyboard, window, Subscription};

use crate::{
    message::{Message, PaletteMsg, RequestMsg, WsMsg},
    services::websocket,
};

use super::AppState;

pub(crate) fn subscription(state: &AppState) -> Subscription<Message> {
    let global = event::listen_with(global_keys);
    let kbd = if state.palette_open {
        Subscription::batch([global, event::listen_with(palette_keys)])
    } else {
        global
    };

    let ws_subs: Vec<_> = state
        .tabs
        .tabs
        .iter()
        .filter(|t| t.ws.connecting || t.ws.connected)
        .map(|t| Subscription::run_with(WsConn { tab_id: t.id.clone(), url: t.ws.url.clone() }, ws_stream))
        .collect();

    let mut all = vec![kbd];
    all.extend(ws_subs);
    Subscription::batch(all)
}

/// Identity + inputs for a tab's WebSocket subscription. `Hash` drives subscription
/// identity, so the stream is kept alive while a tab stays connected.
#[derive(Hash)]
struct WsConn {
    tab_id: String,
    url: String,
}

fn ws_stream(conn: &WsConn) -> impl iced::futures::Stream<Item = Message> {
    let url = conn.url.clone();
    let tab_id = conn.tab_id.clone();
    iced::stream::channel(256, move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
        use crate::services::websocket::WsEvent;
        match websocket::connect(url).await {
            Ok((handle, mut rx)) => {
                let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<String>(64);
                let _ = output
                    .send(Message::WebSocket(WsMsg::Handshake { tab_id: tab_id.clone(), sender: out_tx }))
                    .await;
                loop {
                    tokio::select! {
                        msg = out_rx.recv() => {
                            match msg {
                                Some(m) => handle.send_text(m).await,
                                None => break,
                            }
                        }
                        event = rx.recv() => {
                            match event {
                                Some(WsEvent::Text(t)) => {
                                    let _ = output.send(Message::WebSocket(WsMsg::TextFrame(t))).await;
                                }
                                Some(WsEvent::Error(e)) => {
                                    let _ = output.send(Message::WebSocket(WsMsg::Error(e))).await;
                                    break;
                                }
                                Some(WsEvent::Disconnected) | None => {
                                    let _ = output.send(Message::WebSocket(WsMsg::Disconnected)).await;
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = output.send(Message::WebSocket(WsMsg::Error(e))).await;
            }
        }
    })
}

fn global_keys(event: Event, status: event::Status, window_id: window::Id) -> Option<Message> {
    if let Event::Window(window::Event::CloseRequested) = event {
        return Some(Message::App(crate::message::AppMsg::WindowCloseRequested(window_id)));
    }
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };

    if let keyboard::Key::Character(z) = key.as_ref() {
        if z == "z" && modifiers.command() && !modifiers.shift() {
            return if matches!(status, event::Status::Ignored) {
                Some(Message::Request(RequestMsg::UndoUrl))
            } else {
                None
            };
        }
        
        if z == "a" && modifiers.command() {
            return if matches!(status, event::Status::Ignored) {
                Some(Message::Request(RequestMsg::CopyBodyToClipboard))
            } else {
                None
            };
        }
    }

    use keyboard::key::Named;
    match key.as_ref() {
        keyboard::Key::Character("p") if modifiers.command() => Some(Message::Palette(PaletteMsg::Open)),
        keyboard::Key::Character("t") if modifiers.command() => Some(Message::Request(RequestMsg::NewTab)),
        keyboard::Key::Character("w") if modifiers.command() => Some(Message::Request(RequestMsg::CloseCurrentTab)),
        keyboard::Key::Character("s") if modifiers.command() => Some(Message::Request(RequestMsg::SaveRequest)),
        keyboard::Key::Character("e") if modifiers.command() => Some(Message::Request(RequestMsg::ExportCurl)),
        keyboard::Key::Character("f") if modifiers.command() => None,
        keyboard::Key::Character("/") if modifiers.command() => Some(Message::Request(RequestMsg::CommentToggle)),
        keyboard::Key::Character("=") if modifiers.command() => Some(Message::Layout(crate::message::LayoutMsg::ZoomIn)),
        keyboard::Key::Character("-") if modifiers.command() => Some(Message::Layout(crate::message::LayoutMsg::ZoomOut)),
        keyboard::Key::Character("0") if modifiers.command() => Some(Message::Layout(crate::message::LayoutMsg::ZoomReset)),
        keyboard::Key::Named(Named::Enter) if modifiers.command() => Some(Message::Request(RequestMsg::Send)),
        keyboard::Key::Character("1") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(0))),
        keyboard::Key::Character("2") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(1))),
        keyboard::Key::Character("3") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(2))),
        keyboard::Key::Character("4") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(3))),
        keyboard::Key::Character("5") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(4))),
        keyboard::Key::Character("6") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(5))),
        keyboard::Key::Character("7") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(6))),
        keyboard::Key::Character("8") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(7))),
        keyboard::Key::Character("9") if modifiers.alt() => Some(Message::Request(RequestMsg::SwitchTab(8))),
        _ => None,
    }
}

fn palette_keys(event: Event, _status: event::Status, _id: window::Id) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event else {
        return None;
    };
    use keyboard::key::Named;
    match key.as_ref() {
        keyboard::Key::Named(Named::Escape) => Some(Message::Palette(PaletteMsg::Close)),
        keyboard::Key::Named(Named::ArrowDown) => Some(Message::Palette(PaletteMsg::MoveDown)),
        keyboard::Key::Named(Named::ArrowUp) => Some(Message::Palette(PaletteMsg::MoveUp)),
        keyboard::Key::Named(Named::Enter) => Some(Message::Palette(PaletteMsg::Confirm)),
        _ => None,
    }
}
