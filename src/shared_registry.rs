//! Typed shared-instance registration.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct SharedRegistry {
    values: Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl SharedRegistry {
    pub fn get_or_insert<T>(&self, name: &str) -> Result<Arc<T>, Arc<dyn Any + Send + Sync>>
    where
        T: Any + Default + Send + Sync,
    {
        let mut values = self.values.lock().expect("shared registry poisoned");
        let value = values
            .entry(name.to_owned())
            .or_insert_with(|| Arc::new(T::default()))
            .clone();
        Arc::downcast(value)
    }

    pub fn set<T: Any + Send + Sync>(&self, name: impl Into<String>, value: Arc<T>) {
        self.values
            .lock()
            .expect("shared registry poisoned")
            .insert(name.into(), value);
    }

    pub fn clear(&self, name: &str) -> bool {
        self.values
            .lock()
            .expect("shared registry poisoned")
            .remove(name)
            .is_some()
    }

    pub fn clear_all(&self) {
        self.values
            .lock()
            .expect("shared registry poisoned")
            .clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.values.lock().expect("shared registry poisoned").len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::SharedRegistry;
    use std::sync::Arc;

    #[test]
    fn values_are_shared_typed_and_clearable() {
        let registry = SharedRegistry::default();
        let first = registry.get_or_insert::<String>("value").unwrap();
        let second = registry.get_or_insert::<String>("value").unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(registry.get_or_insert::<usize>("value").is_err());
        assert_eq!(registry.len(), 1);
        assert!(registry.clear("value"));
        assert!(registry.is_empty());
    }
}
