//! Application options and simple `name = value` file ingestion.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OptionSource {
    ExplicitViaCode,
    CommandLine,
    BinaryRelatedConfigFile(String),
    SystemConfigRepository,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionValue {
    String(String),
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
    Bytes(Vec<u8>),
    Strings(Vec<String>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueConversionError;

impl fmt::Display for ValueConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("option value cannot be converted to the requested type")
    }
}

impl std::error::Error for ValueConversionError {}

impl OptionValue {
    pub fn as_string(&self) -> Result<String, ValueConversionError> {
        match self {
            Self::String(value) => Ok(value.clone()),
            Self::I64(value) => Ok(value.to_string()),
            Self::U64(value) => Ok(value.to_string()),
            Self::F64(value) => Ok(value.to_string()),
            Self::Bool(value) => Ok(value.to_string()),
            Self::Bytes(_) | Self::Strings(_) => Err(ValueConversionError),
        }
    }

    pub fn as_i64(&self) -> Result<i64, ValueConversionError> {
        match self {
            Self::String(value) => value.parse().map_err(|_| ValueConversionError),
            Self::I64(value) => Ok(*value),
            Self::U64(value) => i64::try_from(*value).map_err(|_| ValueConversionError),
            Self::F64(value)
                if value.is_finite() && *value >= i64::MIN as f64 && *value <= i64::MAX as f64 =>
            {
                Ok(*value as i64)
            }
            Self::Bool(value) => Ok(i64::from(*value)),
            Self::F64(_) | Self::Bytes(_) | Self::Strings(_) => Err(ValueConversionError),
        }
    }

    pub fn as_u64(&self) -> Result<u64, ValueConversionError> {
        match self {
            Self::String(value) => value.parse().map_err(|_| ValueConversionError),
            Self::I64(value) => u64::try_from(*value).map_err(|_| ValueConversionError),
            Self::U64(value) => Ok(*value),
            Self::F64(value) if value.is_finite() && *value >= 0.0 && *value <= u64::MAX as f64 => {
                Ok(*value as u64)
            }
            Self::Bool(value) => Ok(u64::from(*value)),
            Self::F64(_) | Self::Bytes(_) | Self::Strings(_) => Err(ValueConversionError),
        }
    }

    pub fn as_f64(&self) -> Result<f64, ValueConversionError> {
        match self {
            Self::String(value) => value.parse().map_err(|_| ValueConversionError),
            Self::I64(value) => Ok(*value as f64),
            Self::U64(value) => Ok(*value as f64),
            Self::F64(value) => Ok(*value),
            Self::Bool(value) => Ok(if *value { 1.0 } else { 0.0 }),
            Self::Bytes(_) | Self::Strings(_) => Err(ValueConversionError),
        }
    }

    pub fn as_bool(&self) -> Result<bool, ValueConversionError> {
        match self {
            Self::String(value) if value.eq_ignore_ascii_case("true") => Ok(true),
            Self::String(value) if value.eq_ignore_ascii_case("false") => Ok(false),
            Self::String(value) => value
                .parse::<i64>()
                .map(|value| value != 0)
                .map_err(|_| ValueConversionError),
            Self::I64(value) => Ok(*value != 0),
            Self::U64(value) => Ok(*value != 0),
            Self::F64(value) => Ok(*value != 0.0),
            Self::Bool(value) => Ok(*value),
            Self::Bytes(_) | Self::Strings(_) => Err(ValueConversionError),
        }
    }
}

/// Conversion from an untyped stored option value into a typed option.
pub trait FromOptionValue: Sized {
    fn from_option_value(value: &OptionValue) -> Result<Self, ValueConversionError>;
}

impl FromOptionValue for String {
    fn from_option_value(value: &OptionValue) -> Result<Self, ValueConversionError> {
        value.as_string()
    }
}

impl FromOptionValue for i64 {
    fn from_option_value(value: &OptionValue) -> Result<Self, ValueConversionError> {
        value.as_i64()
    }
}

impl FromOptionValue for u64 {
    fn from_option_value(value: &OptionValue) -> Result<Self, ValueConversionError> {
        value.as_u64()
    }
}

impl FromOptionValue for f64 {
    fn from_option_value(value: &OptionValue) -> Result<Self, ValueConversionError> {
        value.as_f64()
    }
}

impl FromOptionValue for bool {
    fn from_option_value(value: &OptionValue) -> Result<Self, ValueConversionError> {
        value.as_bool()
    }
}

impl From<&str> for OptionValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for OptionValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OptionQualifiers {
    pub sensitive: bool,
    pub return_only_default_value: bool,
    pub application_flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpecifiedValue {
    pub source: OptionSource,
    pub native_name: String,
    pub value: OptionValue,
}

impl SpecifiedValue {
    #[must_use]
    pub fn new(
        source: OptionSource,
        native_name: impl Into<String>,
        value: impl Into<OptionValue>,
    ) -> Self {
        Self {
            source,
            native_name: native_name.into(),
            value: value.into(),
        }
    }

    #[must_use]
    pub fn matches(&self, normalized_name: &str) -> bool {
        option_names_match(&self.native_name, normalized_name)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppOptions {
    specified: HashMap<String, SpecifiedValue>,
    qualifiers: HashMap<String, OptionQualifiers>,
    metadata: HashMap<String, String>,
}

impl AppOptions {
    pub fn add(&mut self, value: SpecifiedValue) -> Option<SpecifiedValue> {
        self.specified.insert(value.native_name.clone(), value)
    }

    #[must_use]
    pub fn find_by_native_name(&self, name: &str) -> Option<&SpecifiedValue> {
        self.specified.get(name)
    }

    #[must_use]
    pub fn find_matching(&self, normalized_name: &str) -> Vec<&SpecifiedValue> {
        self.specified
            .values()
            .filter(|value| value.matches(normalized_name))
            .collect()
    }

    #[must_use]
    pub fn get(&self, normalized_name: &str) -> Option<&OptionValue> {
        if self
            .qualifiers
            .get(normalized_name)
            .is_some_and(|qualifiers| qualifiers.return_only_default_value)
        {
            return None;
        }
        self.find_matching(normalized_name)
            .first()
            .map(|specified| &specified.value)
    }

    pub fn get_typed<T: FromOptionValue>(
        &self,
        normalized_name: &str,
    ) -> Result<Option<T>, ValueConversionError> {
        self.get(normalized_name)
            .map(T::from_option_value)
            .transpose()
    }

    pub fn clear(&mut self, native_name: &str) -> bool {
        self.specified.remove(native_name).is_some()
    }

    pub fn clear_all(&mut self) {
        self.specified.clear();
    }

    pub fn set_qualifiers(
        &mut self,
        normalized_name: impl Into<String>,
        qualifiers: OptionQualifiers,
    ) {
        self.qualifiers.insert(normalized_name.into(), qualifiers);
    }

    #[must_use]
    pub fn qualifiers(&self, normalized_name: &str) -> Option<&OptionQualifiers> {
        self.qualifiers.get(normalized_name)
    }

    pub fn set_metadata(
        &mut self,
        normalized_name: impl Into<String>,
        metadata: impl Into<String>,
    ) {
        self.metadata
            .insert(normalized_name.into(), metadata.into());
    }

    #[must_use]
    pub fn metadata(&self, normalized_name: &str) -> Option<&str> {
        self.metadata.get(normalized_name).map(String::as_str)
    }

    pub fn read_basic_file(&mut self, path: impl AsRef<Path>) -> io::Result<usize> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        let mut count = 0;
        for line in contents.lines() {
            if let Some((name, value)) = parse_basic_line(line) {
                self.add(SpecifiedValue::new(
                    OptionSource::BinaryRelatedConfigFile(path.display().to_string()),
                    name,
                    value,
                ));
                count += 1;
            }
        }
        Ok(count)
    }
}

/// A typed option definition with an application-provided default.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionDefinition<T> {
    pub name: String,
    pub default: T,
}

impl<T: Clone + FromOptionValue> OptionDefinition<T> {
    #[must_use]
    pub fn new(name: impl Into<String>, default: T) -> Self {
        Self {
            name: name.into(),
            default,
        }
    }

    pub fn get(&self, options: &AppOptions) -> Result<T, ValueConversionError> {
        Ok(options
            .get_typed(&self.name)?
            .unwrap_or_else(|| self.default.clone()))
    }
}

#[must_use]
pub fn option_names_match(left: &str, right: &str) -> bool {
    let elements = |value: &str| {
        value
            .split([':', '.'])
            .filter(|element| !element.is_empty())
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>()
    };
    elements(left) == elements(right)
}

#[must_use]
pub fn parse_basic_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (name, raw_value) = line.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let raw_value = raw_value.trim();
    if let Some(quoted) = raw_value.strip_prefix('"') {
        let end = quoted
            .char_indices()
            .find(|(index, character)| *character == '"' && !quoted[..*index].ends_with('\\'))
            .map_or(quoted.len(), |(index, _)| index);
        return Some((name, &quoted[..end]));
    }
    Some((
        name,
        raw_value
            .split_once('#')
            .map_or(raw_value, |(value, _)| value)
            .trim(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        AppOptions, OptionDefinition, OptionQualifiers, OptionSource, OptionValue, SpecifiedValue,
        option_names_match, parse_basic_line,
    };
    use std::fs;

    #[test]
    fn values_convert_with_checked_semantics() {
        assert_eq!(OptionValue::from("42").as_i64().unwrap(), 42);
        assert!(OptionValue::from("-1").as_u64().is_err());
        assert!(OptionValue::from("TRUE").as_bool().unwrap());
        assert_eq!(OptionValue::Bool(false).as_string().unwrap(), "false");
    }

    #[test]
    fn names_match_by_case_insensitive_tree_elements() {
        assert!(option_names_match("Logging:Level", "logging.level"));
        assert!(!option_names_match("Logging.Level", "Logging"));
    }

    #[test]
    fn store_handles_values_qualifiers_and_metadata() {
        let mut options = AppOptions::default();
        options.add(SpecifiedValue::new(
            OptionSource::CommandLine,
            "logging:level",
            "debug",
        ));
        assert_eq!(
            options.get("Logging.Level"),
            Some(&OptionValue::from("debug"))
        );
        options.set_metadata("Logging.Level", "display name");
        assert_eq!(options.metadata("Logging.Level"), Some("display name"));
        options.set_qualifiers(
            "Logging.Level",
            OptionQualifiers {
                return_only_default_value: true,
                ..OptionQualifiers::default()
            },
        );
        assert_eq!(options.get("Logging.Level"), None);
    }

    #[test]
    fn typed_definition_returns_stored_or_default_value() {
        let mut options = AppOptions::default();
        let definition = OptionDefinition::new("worker.count", 4_i64);
        assert_eq!(definition.get(&options).unwrap(), 4);
        options.add(SpecifiedValue::new(
            OptionSource::CommandLine,
            "worker:count",
            "8",
        ));
        assert_eq!(definition.get(&options).unwrap(), 8);
    }

    #[test]
    fn basic_file_lines_and_file_ingestion_work() {
        assert_eq!(
            parse_basic_line(" key = value # comment"),
            Some(("key", "value"))
        );
        assert_eq!(
            parse_basic_line("key = \"quoted # value\" # ignored"),
            Some(("key", "quoted # value"))
        );
        assert_eq!(parse_basic_line("# comment"), None);

        let path =
            std::env::temp_dir().join(format!("r-vlr-util-options-{}.txt", std::process::id()));
        fs::write(&path, "one = 1\ntwo = \"second\"\n").unwrap();
        let mut options = AppOptions::default();
        assert_eq!(options.read_basic_file(&path).unwrap(), 2);
        assert_eq!(options.get("one").unwrap().as_i64().unwrap(), 1);
        fs::remove_file(path).unwrap();
    }
}
