//! Retry policies and execution helpers.

use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RetryDisposition {
    #[default]
    NoRetry,
    LinearDelay(Duration),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RetrySettings {
    /// Number of retries after the initial call.
    pub max_retries: usize,
    pub disposition: RetryDisposition,
}

/// Calls `operation` until `should_retry` returns false or retries are exhausted.
pub fn call_with_retry<T>(
    settings: RetrySettings,
    mut operation: impl FnMut() -> T,
    should_retry: impl Fn(&T) -> bool,
) -> T {
    let mut retry_count = 0;
    loop {
        let result = operation();
        if retry_count >= settings.max_retries || !should_retry(&result) {
            return result;
        }
        retry_count += 1;
        if let RetryDisposition::LinearDelay(delay) = settings.disposition {
            thread::sleep(delay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RetrySettings, call_with_retry};

    #[test]
    fn retries_until_success() {
        let mut attempts = 0;
        let result = call_with_retry(
            RetrySettings {
                max_retries: 3,
                ..RetrySettings::default()
            },
            || {
                attempts += 1;
                attempts
            },
            |result| *result < 3,
        );
        assert_eq!(result, 3);
        assert_eq!(attempts, 3);
    }
}
