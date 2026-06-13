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

#[cfg(test)]
mod tests {
    use super::{SplitOptions, split_at_delimiters};

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
}
