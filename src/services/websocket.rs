use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected,
    Text(String),
    Binary,
    Disconnected,
}

pub struct WsHandle {
    sender: mpsc::Sender<String>,
}

impl WsHandle {
    pub async fn send_text(&self, msg: String) {
        let _ = self.sender.send(msg).await;
    }
}


fn ensure_crypto_provider() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        // Fails only if a provider was already installed, which is fine: the
        // goal is just that *some* provider is in place before a handshake.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

pub async fn connect(url: String) -> Result<(WsHandle, mpsc::Receiver<WsEvent>), String> {
    ensure_crypto_provider();

    let (stream, _) = connect_async(&url).await.map_err(|e| e.to_string())?;
    let (mut write, mut read) = stream.split();

    let (event_tx, event_rx) = mpsc::channel::<WsEvent>(256);
    let (send_tx, mut send_rx) = mpsc::channel::<String>(64);

    let event_tx_clone = event_tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = send_rx.recv().await {
            if write.send(WsMessage::Text(msg.into())).await.is_err() {
                let _ = event_tx_clone.send(WsEvent::Disconnected).await;
                break;
            }
        }
    });

    tokio::spawn(async move {
        let _ = event_tx.send(WsEvent::Connected).await;
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(t)) => {
                    let _ = event_tx.send(WsEvent::Text(t.to_string())).await;
                }
                Ok(WsMessage::Binary(_)) => {
                    let _ = event_tx.send(WsEvent::Binary).await;
                }
                Ok(WsMessage::Close(_)) | Err(_) => {
                    let _ = event_tx.send(WsEvent::Disconnected).await;
                    break;
                }
                _ => {}
            }
        }
    });

    Ok((WsHandle { sender: send_tx }, event_rx))
}
