//! Human-readable formatting helpers.

/// Formats a byte count using the C++ library's binary unit thresholds.
#[must_use]
pub fn approximate_data_size(bytes: i64) -> String {
    const UNITS: [(&str, i64); 4] = [
        ("bytes", 1),
        ("KB", 1024),
        ("MB", 1024 * 1024),
        ("GB", 1024 * 1024 * 1024),
    ];
    for (name, count) in UNITS {
        if bytes <= count * 1024 {
            return format!("{:.2} {name}", bytes as f64 / count as f64);
        }
    }
    format!("{:.2} GB(big)", bytes as f64 / (1024_i64.pow(3)) as f64)
}

#[cfg(test)]
mod tests {
    use super::approximate_data_size;

    #[test]
    fn formats_sizes() {
        assert_eq!(approximate_data_size(512), "512.00 bytes");
        assert_eq!(approximate_data_size(2048), "2.00 KB");
    }
}
