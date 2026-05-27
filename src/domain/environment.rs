use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEnvironment {
    pub id: String,
    pub name: String,
    pub variables: HashMap<String, String>,
    pub is_active: bool,
}

impl AppEnvironment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            variables: HashMap::new(),
            is_active: false,
        }
    }
}

pub fn substitute(text: &str, env: Option<&AppEnvironment>) -> String {
    let Some(env) = env else { return text.to_owned() };
    let mut out = text.to_owned();
    for (k, v) in &env.variables {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}
