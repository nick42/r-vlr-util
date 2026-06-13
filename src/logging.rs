//! Lightweight logging context and callback-based dispatch.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum Level {
    Debug,
    Trace,
    Verbose,
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CodeContext {
    pub file: &'static str,
    pub line: u32,
    pub function: &'static str,
}

impl CodeContext {
    #[must_use]
    pub fn file_name(self) -> &'static str {
        Path::new(self.file)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(self.file)
    }

    #[must_use]
    pub fn position(self) -> String {
        if self.file.is_empty() {
            "[unknown]".to_owned()
        } else {
            format!("{}:{}", self.file_name(), self.line)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MessageContext {
    pub code: CodeContext,
    pub level: Level,
    pub flags: u16,
}

type Sink = dyn Fn(&MessageContext, &str) + Send + Sync;

#[derive(Clone)]
pub struct Logger {
    minimum_level: Level,
    sink: Arc<Sink>,
}

impl fmt::Debug for Logger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Logger")
            .field("minimum_level", &self.minimum_level)
            .finish_non_exhaustive()
    }
}

impl Logger {
    pub fn new(
        minimum_level: Level,
        sink: impl Fn(&MessageContext, &str) + Send + Sync + 'static,
    ) -> Self {
        Self {
            minimum_level,
            sink: Arc::new(sink),
        }
    }

    #[must_use]
    pub fn could_log(&self, context: &MessageContext) -> bool {
        context.level >= self.minimum_level
    }

    #[must_use]
    pub fn log(&self, context: &MessageContext, message: &str) -> bool {
        if !self.could_log(context) {
            return false;
        }
        (self.sink)(context, message);
        true
    }
}

#[macro_export]
macro_rules! code_context {
    () => {
        $crate::logging::CodeContext {
            file: file!(),
            line: line!(),
            function: module_path!(),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{Level, Logger, MessageContext};
    use std::sync::{Arc, Mutex};

    #[test]
    fn code_context_and_level_filter_work() {
        let context = crate::code_context!();
        assert!(context.file_name().ends_with("logging.rs"));
        let messages = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&messages);
        let logger = Logger::new(Level::Info, move |_, message| {
            captured.lock().unwrap().push(message.to_owned());
        });
        assert!(!logger.log(
            &MessageContext {
                level: Level::Debug,
                ..MessageContext::default()
            },
            "debug"
        ));
        assert!(logger.log(&MessageContext::default(), "info"));
        assert_eq!(*messages.lock().unwrap(), ["info"]);
    }
}
