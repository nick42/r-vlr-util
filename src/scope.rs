//! Scope-bound cleanup helpers.

/// Runs a closure when dropped unless it has been dismissed.
pub struct ScopeGuard<F: FnOnce()> {
    action: Option<F>,
}

impl<F: FnOnce()> ScopeGuard<F> {
    #[must_use]
    pub const fn new(action: F) -> Self {
        Self {
            action: Some(action),
        }
    }

    pub fn run_now(&mut self) {
        if let Some(action) = self.action.take() {
            action();
        }
    }

    pub fn dismiss(&mut self) {
        self.action = None;
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(action) = self.action.take() {
            action();
        }
    }
}

/// Temporarily assigns a value and restores the original when dropped.
pub struct RevertGuard<'a, T> {
    target: &'a mut T,
    original: Option<T>,
}

impl<'a, T> RevertGuard<'a, T> {
    #[must_use]
    pub fn new(target: &'a mut T, temporary: T) -> Self {
        let original = std::mem::replace(target, temporary);
        Self {
            target,
            original: Some(original),
        }
    }

    pub fn current(&self) -> &T {
        self.target
    }

    pub fn current_mut(&mut self) -> &mut T {
        self.target
    }

    pub fn keep(mut self) {
        self.original = None;
    }
}

impl<T> Drop for RevertGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(original) = self.original.take() {
            *self.target = original;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RevertGuard, ScopeGuard};
    use std::cell::Cell;

    #[test]
    fn scope_guard_runs_once_or_can_be_dismissed() {
        let calls = Cell::new(0);
        {
            let mut guard = ScopeGuard::new(|| calls.set(calls.get() + 1));
            guard.run_now();
        }
        assert_eq!(calls.get(), 1);
        {
            let mut guard = ScopeGuard::new(|| calls.set(calls.get() + 1));
            guard.dismiss();
        }
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn revert_guard_restores_value() {
        let mut value = 1;
        {
            let mut guard = RevertGuard::new(&mut value, 2);
            assert_eq!(*guard.current(), 2);
            *guard.current_mut() = 3;
        }
        assert_eq!(value, 1);
    }
}
