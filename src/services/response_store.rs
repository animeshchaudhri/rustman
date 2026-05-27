use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ResponseStore(pub Arc<Mutex<HashMap<String, String>>>);

impl Default for ResponseStore {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl Clone for ResponseStore {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

pub struct BodySlice {
    pub lines: Vec<String>,
    pub total_lines: usize,
}

impl ResponseStore {
    pub fn insert(&self, id: String, text: String) {
        if let Ok(mut map) = self.0.lock() {
            map.insert(id, text);
        }
    }

    pub fn get_full(&self, id: &str) -> String {
        self.0.lock().ok().and_then(|m| m.get(id).cloned()).unwrap_or_default()
    }

    pub fn get_slice(&self, id: &str, line_start: usize, line_count: usize) -> BodySlice {
        let map = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let text = map.get(id).map(|s| s.as_str()).unwrap_or("");
        let all_lines: Vec<&str> = text.split('\n').collect();
        let total_lines = all_lines.len();
        let start = line_start.min(total_lines);
        let end = (line_start + line_count).min(total_lines);
        BodySlice {
            lines: all_lines[start..end].iter().map(|s| s.to_string()).collect(),
            total_lines,
        }
    }

    pub fn search(&self, id: &str, query: &str) -> usize {
        if query.is_empty() {
            return 0;
        }
        let map = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let text = map.get(id).map(|s| s.as_str()).unwrap_or("");
        let lower_q = query.to_lowercase();
        text.to_lowercase().matches(lower_q.as_str()).count()
    }

    pub fn search_lines(&self, id: &str, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return vec![];
        }
        let map = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let text = map.get(id).map(|s| s.as_str()).unwrap_or("");
        let lower_q = query.to_lowercase();
        text.split('\n')
            .enumerate()
            .filter(|(_, l)| l.to_lowercase().contains(lower_q.as_str()))
            .map(|(i, _)| i)
            .take(50_000)
            .collect()
    }

    pub fn clear_prefix(&self, prefix: &str) {
        if let Ok(mut map) = self.0.lock() {
            map.retain(|k, _| !k.starts_with(prefix));
        }
    }
}
