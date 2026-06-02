use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub body_size: usize,
    pub body_stored: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConsoleEntry {
    pub level: ConsoleLevel,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleLevel {
    Log,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}
