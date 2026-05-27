

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptInput {
    pub script: String,
    pub body: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub response: Option<ResponseData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseData {
    pub status: Option<u16>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ScriptOutput {
    pub vars: HashMap<String, String>,
    pub body: Option<String>,
    pub logs: Vec<LogEntry>,
    pub results: Vec<ScriptTestResult>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub level: String,
    pub args: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptTestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub fn run(_input: ScriptInput) -> ScriptOutput {
   
    ScriptOutput {
        error: Some("Script runner not yet implemented in this build".to_owned()),
        ..Default::default()
    }
}
