//! Reusable caches.

use regex::{Error as RegexError, Regex};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, RwLock};

/// A bounded most-recently-used cache.
#[derive(Clone, Debug)]
pub struct MruCache<K, V> {
    capacity: usize,
    order: VecDeque<K>,
    values: HashMap<K, V>,
}

impl<K: Clone + Eq + Hash, V> MruCache<K, V> {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::new(),
            values: HashMap::new(),
        }
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        self.truncate();
    }

    #[must_use]
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.values.contains_key(key) {
            self.order.retain(|existing| existing != key);
            self.order.push_front(key.clone());
        }
        self.values.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.order.retain(|existing| existing != &key);
        self.order.push_front(key.clone());
        let previous = self.values.insert(key, value);
        self.truncate();
        previous
    }

    fn truncate(&mut self) {
        while self.order.len() > self.capacity {
            if let Some(key) = self.order.pop_back() {
                self.values.remove(&key);
            }
        }
    }
}

impl<K: Clone + Eq + Hash, V> Default for MruCache<K, V> {
    fn default() -> Self {
        Self::new(100)
    }
}

/// A thread-safe cache of compiled regular expressions.
#[derive(Debug, Default)]
pub struct RegexCache {
    values: RwLock<HashMap<String, Arc<Regex>>>,
}

impl RegexCache {
    pub fn get(&self, pattern: &str) -> Result<Arc<Regex>, RegexError> {
        if let Some(value) = self
            .values
            .read()
            .expect("regex cache poisoned")
            .get(pattern)
        {
            return Ok(Arc::clone(value));
        }
        let compiled = Arc::new(Regex::new(pattern)?);
        self.values
            .write()
            .expect("regex cache poisoned")
            .insert(pattern.to_owned(), Arc::clone(&compiled));
        Ok(compiled)
    }
}

#[cfg(test)]
mod tests {
    use super::{MruCache, RegexCache};
    use std::sync::Arc;

    #[test]
    fn mru_cache_updates_and_evicts() {
        let mut cache = MruCache::new(2);
        cache.insert("one", 1);
        cache.insert("two", 2);
        assert_eq!(cache.get(&"one"), Some(&1));
        cache.insert("three", 3);
        assert_eq!(cache.get(&"two"), None);
        assert_eq!(cache.get(&"one"), Some(&1));
    }

    #[test]
    fn regex_cache_reuses_compiled_value_and_reports_errors() {
        let cache = RegexCache::default();
        let first = cache.get(r"^\w+$").unwrap();
        let second = cache.get(r"^\w+$").unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(cache.get("(").is_err());
    }
}
