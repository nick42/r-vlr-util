//! Numeric, bit, checksum, and conversion helpers.

use std::fmt;

/// Returns whether every bit in `mask` is present in `value`.
#[must_use]
pub const fn is_bit_set(value: u64, mask: u64) -> bool {
    value & mask == mask
}

/// Returns whether `value` contains exactly one set bit.
#[must_use]
pub const fn is_single_bit(value: u64) -> bool {
    value.is_power_of_two()
}

/// Combines the low and high halves of a 64-bit value.
#[must_use]
pub const fn combine_u32(low: u32, high: u32) -> u64 {
    (high as u64) << 32 | low as u64
}

/// A failed checked numeric conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutOfRange;

impl fmt::Display for OutOfRange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("numeric value is outside the destination type's range")
    }
}

impl std::error::Error for OutOfRange {}

/// Converts between numeric types without the silent truncation of C++ casts.
pub fn checked_cast<T, U>(value: U) -> Result<T, OutOfRange>
where
    T: TryFrom<U>,
{
    T::try_from(value).map_err(|_| OutOfRange)
}

/// Computes a standard IEEE CRC-32 checksum.
#[must_use]
pub const fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    let mut index = 0;
    while index < bytes.len() {
        crc ^= bytes[index] as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }
        index += 1;
    }
    crc ^ 0xffff_ffff
}

/// Couples a resource-format string with its stable lookup checksum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceString<'a> {
    pub format: &'a str,
    pub crc32: u32,
}

impl<'a> ResourceString<'a> {
    #[must_use]
    pub const fn new(format: &'a str) -> Self {
        Self {
            format,
            crc32: crc32(format.as_bytes()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ResourceString, checked_cast, combine_u32, crc32, is_bit_set, is_single_bit};

    #[test]
    fn bit_and_combine_helpers_work() {
        assert!(is_bit_set(0b1011, 0b0011));
        assert!(!is_bit_set(0b1001, 0b0011));
        assert!(is_single_bit(8));
        assert!(!is_single_bit(0));
        assert!(!is_single_bit(3));
        assert_eq!(combine_u32(0x89ab_cdef, 0x0123_4567), 0x0123_4567_89ab_cdef);
    }

    #[test]
    fn checked_cast_reports_range_errors() {
        assert_eq!(checked_cast::<u8, _>(42_u16), Ok(42));
        assert!(checked_cast::<u8, _>(256_u16).is_err());
        assert!(checked_cast::<u32, _>(-1_i32).is_err());
    }

    #[test]
    fn checksum_matches_standard_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(ResourceString::new("value").crc32, crc32(b"value"));
    }
}
