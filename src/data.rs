//! Adaptors for externally defined data layouts.

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultiSzError {
    MissingDoubleTerminator,
    EmbeddedEmptyValue,
}

impl fmt::Display for MultiSzError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDoubleTerminator => {
                formatter.write_str("MULTI_SZ is not double terminated")
            }
            Self::EmbeddedEmptyValue => formatter.write_str("MULTI_SZ values cannot be empty"),
        }
    }
}

impl std::error::Error for MultiSzError {}

/// Parses a double-NUL-terminated sequence into borrowed values.
pub fn parse_multi_sz<T: Default + Eq>(input: &[T]) -> Result<Vec<&[T]>, MultiSzError> {
    let zero = T::default();
    if input.len() < 2 || input[input.len() - 2] != zero || input[input.len() - 1] != zero {
        return Err(MultiSzError::MissingDoubleTerminator);
    }
    let mut values = Vec::new();
    let mut start = 0;
    for (index, value) in input.iter().enumerate() {
        if value != &zero {
            continue;
        }
        if index == start {
            if index == input.len() - 1 || input.get(index + 1) == Some(&zero) {
                return Ok(values);
            }
            return Err(MultiSzError::EmbeddedEmptyValue);
        }
        values.push(&input[start..index]);
        start = index + 1;
    }
    Err(MultiSzError::MissingDoubleTerminator)
}

/// Encodes values using a double-NUL-terminated sequence.
pub fn encode_multi_sz<T: Clone + Default + Eq>(
    values: &[impl AsRef<[T]>],
) -> Result<Vec<T>, MultiSzError> {
    let mut encoded = Vec::new();
    for value in values {
        let value = value.as_ref();
        if value.is_empty() {
            return Err(MultiSzError::EmbeddedEmptyValue);
        }
        encoded.extend_from_slice(value);
        encoded.push(T::default());
    }
    encoded.push(T::default());
    if values.is_empty() {
        encoded.push(T::default());
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::{encode_multi_sz, parse_multi_sz};

    #[test]
    fn multi_sz_round_trips() {
        let encoded = encode_multi_sz(&[b"one".as_slice(), b"two".as_slice()]).unwrap();
        assert_eq!(encoded, b"one\0two\0\0");
        assert_eq!(parse_multi_sz(&encoded).unwrap(), [b"one", b"two"]);
        let empty = encode_multi_sz::<u8>(&[] as &[&[u8]]).unwrap();
        assert_eq!(empty, b"\0\0");
        assert!(parse_multi_sz(&empty).unwrap().is_empty());
    }

    #[test]
    fn malformed_multi_sz_is_rejected() {
        assert!(parse_multi_sz(b"one\0two\0").is_err());
        assert!(parse_multi_sz(b"one\0\0two\0\0").is_err());
    }
}
