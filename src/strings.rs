//! String-oriented utilities.

/// Controls which empty elements [`split_at_delimiters`] retains.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SplitOptions {
    /// Retain an empty element before a delimiter at the start of the input.
    pub keep_leading_empty: bool,
    /// Retain an empty element after a delimiter at the end of the input.
    pub keep_trailing_empty: bool,
    /// Retain empty elements between consecutive delimiters.
    pub keep_consecutive_empty: bool,
}

/// Splits `input` at any character in `delimiters`.
///
/// Returned elements borrow from `input`; the strings themselves are not
/// copied or allocated.
///
/// # Examples
///
/// ```
/// use r_vlr_util::strings::{SplitOptions, split_at_delimiters};
///
/// let options = SplitOptions {
///     keep_consecutive_empty: true,
///     ..SplitOptions::default()
/// };
///
/// assert_eq!(
///     split_at_delimiters("/etc//hosts", &['/', '\\'], options),
///     ["etc", "", "hosts"],
/// );
/// ```
#[must_use]
pub fn split_at_delimiters<'input>(
    input: &'input str,
    delimiters: &[char],
    options: SplitOptions,
) -> Vec<&'input str> {
    if input.is_empty() || delimiters.is_empty() {
        return (!input.is_empty()).then_some(input).into_iter().collect();
    }

    let parts: Vec<_> = input
        .split(|character| delimiters.contains(&character))
        .collect();
    let last_index = parts.len() - 1;

    parts
        .into_iter()
        .enumerate()
        .filter_map(|(index, part)| {
            if !part.is_empty() {
                return Some(part);
            }

            let keep = match index {
                0 => options.keep_leading_empty,
                index if index == last_index => options.keep_trailing_empty,
                _ => options.keep_consecutive_empty,
            };
            keep.then_some(part)
        })
        .collect()
}

/// Splits a path-like string at either kind of path separator.
#[must_use]
pub fn split_path(input: &str, options: SplitOptions) -> Vec<&str> {
    split_at_delimiters(input, &['/', '\\'], options)
}

/// Removes `prefix` when it is present according to `case_sensitive`.
#[must_use]
pub fn without_prefix<'a>(value: &'a str, prefix: &str, case_sensitive: bool) -> &'a str {
    if case_sensitive {
        return value.strip_prefix(prefix).unwrap_or(value);
    }
    value
        .get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .and_then(|_| value.get(prefix.len()..))
        .unwrap_or(value)
}

/// Removes `suffix` when it is present according to `case_sensitive`.
#[must_use]
pub fn without_suffix<'a>(value: &'a str, suffix: &str, case_sensitive: bool) -> &'a str {
    if case_sensitive {
        return value.strip_suffix(suffix).unwrap_or(value);
    }
    value
        .len()
        .checked_sub(suffix.len())
        .and_then(|start| value.get(start..).map(|candidate| (start, candidate)))
        .filter(|(_, candidate)| candidate.eq_ignore_ascii_case(suffix))
        .and_then(|(start, _)| value.get(..start))
        .unwrap_or(value)
}

/// Trims every leading and trailing character contained in `characters`.
#[must_use]
pub fn trim_matches<'a>(value: &'a str, characters: &[char]) -> &'a str {
    value.trim_matches(|character| characters.contains(&character))
}

#[cfg(test)]
mod tests {
    use super::{SplitOptions, split_at_delimiters, trim_matches, without_prefix, without_suffix};

    #[test]
    fn returns_input_when_no_delimiter_is_configured() {
        assert_eq!(
            split_at_delimiters("one/two", &[], SplitOptions::default()),
            ["one/two"]
        );
    }

    #[test]
    fn supports_unicode_delimiters() {
        assert_eq!(
            split_at_delimiters("one→two→three", &['→'], SplitOptions::default()),
            ["one", "two", "three"]
        );
    }

    #[test]
    fn removes_optional_affixes_and_trims() {
        assert_eq!(without_prefix("/etc/hosts", "/etc", true), "/hosts");
        assert_eq!(without_prefix("PrefixValue", "prefix", false), "Value");
        assert_eq!(without_suffix("file.TXT", ".txt", false), "file");
        assert_eq!(
            trim_matches(" \tvalue\r\n", &[' ', '\t', '\r', '\n']),
            "value"
        );
    }
}
