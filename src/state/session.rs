use serde::{Deserialize, Serialize};

use super::tabs::TabSnapshot;

/// Persisted session — survives app restarts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSession {
    pub tabs: Vec<TabSnapshot>,
    pub active_tab: usize,
    pub active_env_id: Option<String>,
    pub sidebar_panel: String,
    #[serde(default)]
    pub theme_idx: usize,
    #[serde(default = "default_timeout_ms")]
    pub default_timeout_ms: u64,
    #[serde(default)]
    pub global_pre_request_script: String,
    #[serde(default)]
    pub global_test_script: String,
    /// Accept invalid/self-signed TLS certificates (issue #40).
    #[serde(default)]
    pub tls_accept_invalid_certs: bool,
    /// Offer only HTTP/1.1, avoiding servers that break on an h2 ALPN offer.
    #[serde(default)]
    pub tls_http1_only: bool,
    /// Pin the handshake to TLS 1.2.
    #[serde(default)]
    pub tls_force_tls12: bool,
    /// Pin the handshake to TLS 1.3.
    #[serde(default)]
    pub tls_force_tls13: bool,
}

fn default_timeout_ms() -> u64 { 30_000 }
