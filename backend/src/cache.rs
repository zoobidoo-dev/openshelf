use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

const DEFAULT_MAX_SIZE: usize = 12;

/// LRU cache for in-memory EPUB bytes, bounded to prevent unbounded memory growth.
/// Capacity is controlled by the `EPUB_CACHE_SIZE` env var (default: 12 books).
pub struct EpubCache {
    map: HashMap<String, Arc<Vec<u8>>>,
    order: VecDeque<String>,
    max_size: usize,
}

impl EpubCache {
    pub fn new() -> Self {
        let max_size = std::env::var("EPUB_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(DEFAULT_MAX_SIZE);
        Self {
            map: HashMap::with_capacity(max_size + 1),
            order: VecDeque::with_capacity(max_size + 1),
            max_size,
        }
    }

    /// Returns cached bytes for `id`, promoting it to most-recently-used.
    pub fn get(&mut self, id: &str) -> Option<Arc<Vec<u8>>> {
        let val = self.map.get(id).cloned()?;
        self.order.retain(|k| k != id);
        self.order.push_back(id.to_string());
        Some(val)
    }

    /// Inserts bytes for `id`, evicting the least-recently-used entry if at capacity.
    pub fn insert(&mut self, id: String, bytes: Arc<Vec<u8>>) {
        if self.map.contains_key(&id) {
            self.order.retain(|k| k != &id);
            self.order.push_back(id.clone());
            self.map.insert(id, bytes);
            return;
        }
        while self.map.len() >= self.max_size {
            if let Some(lru_key) = self.order.pop_front() {
                self.map.remove(&lru_key);
            }
        }
        self.order.push_back(id.clone());
        self.map.insert(id, bytes);
    }
}
