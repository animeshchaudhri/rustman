use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use lru::LruCache;
use std::num::NonZeroUsize;

/// In-memory LRU cache for parsed response JSON, capped at 20 entries.
///
/// Keyed by a hash of the raw body string so two tabs with identical responses
/// share one cached parse. On eviction the tab falls back to displaying raw text —
/// no data is lost, just the parsed tree (the raw body always lives in HttpResponse).
pub struct ParsedBodyCache {
    inner: LruCache<u64, serde_json::Value>,
}

impl ParsedBodyCache {
    pub fn new() -> Self {
        Self {
            inner: LruCache::new(NonZeroUsize::new(20).unwrap()),
        }
    }

    pub fn body_hash(body: &str) -> u64 {
        let mut h = DefaultHasher::new();
        body.hash(&mut h);
        h.finish()
    }

    pub fn get(&mut self, body: &str) -> Option<&serde_json::Value> {
        let key = Self::body_hash(body);
        self.inner.get(&key)
    }

    /// Look up by pre-computed hash — avoids re-hashing when the caller already
    /// computed the key before taking a mutable borrow of surrounding state.
    pub fn inner_get_by_hash(&mut self, hash: u64) -> Option<&serde_json::Value> {
        self.inner.get(&hash)
    }

    pub fn insert(&mut self, body: &str, value: serde_json::Value) {
        let key = Self::body_hash(body);
        self.inner.put(key, value);
    }
}

impl Default for ParsedBodyCache {
    fn default() -> Self { Self::new() }
}
