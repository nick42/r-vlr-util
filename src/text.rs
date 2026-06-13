//! Text comparison and similarity utilities.

use std::cmp::Ordering;

/// Controls whether comparisons distinguish ASCII letter case.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CaseSensitivity {
    #[default]
    Sensitive,
    Insensitive,
}

/// A reusable string comparator.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Comparator {
    pub case_sensitivity: CaseSensitivity,
}

impl Comparator {
    #[must_use]
    pub const fn case_sensitive() -> Self {
        Self {
            case_sensitivity: CaseSensitivity::Sensitive,
        }
    }

    #[must_use]
    pub const fn case_insensitive() -> Self {
        Self {
            case_sensitivity: CaseSensitivity::Insensitive,
        }
    }

    #[must_use]
    pub fn is_blank(self, value: &str) -> bool {
        value.is_empty()
    }

    #[must_use]
    pub fn equals(self, left: &str, right: &str) -> bool {
        match self.case_sensitivity {
            CaseSensitivity::Sensitive => left == right,
            CaseSensitivity::Insensitive => left.eq_ignore_ascii_case(right),
        }
    }

    #[must_use]
    pub fn compare(self, left: &str, right: &str) -> Ordering {
        match self.case_sensitivity {
            CaseSensitivity::Sensitive => left.cmp(right),
            CaseSensitivity::Insensitive => left
                .bytes()
                .map(|byte| byte.to_ascii_lowercase())
                .cmp(right.bytes().map(|byte| byte.to_ascii_lowercase())),
        }
    }

    #[must_use]
    pub fn has_prefix(self, value: &str, prefix: &str) -> bool {
        value
            .get(..prefix.len())
            .is_some_and(|candidate| self.equals(candidate, prefix))
    }

    #[must_use]
    pub fn has_suffix(self, value: &str, suffix: &str) -> bool {
        value
            .len()
            .checked_sub(suffix.len())
            .and_then(|start| value.get(start..))
            .is_some_and(|candidate| self.equals(candidate, suffix))
    }

    #[must_use]
    pub fn find(self, value: &str, needle: &str) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        value
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(value.len()))
            .find(|&index| {
                value
                    .get(index..index.saturating_add(needle.len()))
                    .is_some_and(|candidate| self.equals(candidate, needle))
            })
    }
}

/// Calculates a generalized Levenshtein distance over Unicode scalar values.
#[must_use]
pub fn levenshtein_with_costs(
    source: &str,
    target: &str,
    insert_cost: usize,
    delete_cost: usize,
    replace_cost: usize,
) -> usize {
    let source: Vec<_> = source.chars().collect();
    let target: Vec<_> = target.chars().collect();
    let mut previous: Vec<_> = (0..=target.len()).map(|n| n * insert_cost).collect();
    let mut current = vec![0; target.len() + 1];

    for (source_index, source_char) in source.iter().enumerate() {
        current[0] = (source_index + 1) * delete_cost;
        for (target_index, target_char) in target.iter().enumerate() {
            let replacement = usize::from(source_char != target_char) * replace_cost;
            current[target_index + 1] = (current[target_index] + insert_cost)
                .min(previous[target_index + 1] + delete_cost)
                .min(previous[target_index] + replacement);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[target.len()]
}

#[must_use]
pub fn levenshtein(source: &str, target: &str) -> usize {
    levenshtein_with_costs(source, target, 1, 1, 1)
}

/// Evaluates candidates by edit distance, lowest cost first.
#[must_use]
pub fn closest_candidates<'a>(
    starting_value: &str,
    candidates: impl IntoIterator<Item = &'a str>,
    case_sensitivity: CaseSensitivity,
) -> Vec<(&'a str, usize)> {
    let normalize = |value: &str| match case_sensitivity {
        CaseSensitivity::Sensitive => value.to_owned(),
        CaseSensitivity::Insensitive => value.to_ascii_lowercase(),
    };
    let starting_value = normalize(starting_value);
    let mut results: Vec<_> = candidates
        .into_iter()
        .map(|candidate| {
            (
                candidate,
                levenshtein(&starting_value, &normalize(candidate)),
            )
        })
        .collect();
    results.sort_by_key(|(_, distance)| *distance);
    results
}

#[cfg(test)]
mod tests {
    use super::{CaseSensitivity, Comparator, closest_candidates, levenshtein};
    use std::cmp::Ordering;

    #[test]
    fn comparator_supports_core_operations() {
        let comparator = Comparator::case_insensitive();
        assert!(comparator.equals("Value", "value"));
        assert_eq!(comparator.compare("a", "B"), Ordering::Less);
        assert!(comparator.has_prefix("SomeValue", "some"));
        assert!(comparator.has_suffix("SomeValue", "VALUE"));
        assert_eq!(comparator.find("SomeValue", "MEV"), Some(2));
    }

    #[test]
    fn levenshtein_and_candidate_ordering_work() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("café", "cafe"), 1);
        let results = closest_candidates(
            "colour",
            ["color", "collar", "completely"],
            CaseSensitivity::Sensitive,
        );
        assert_eq!(results[0], ("color", 1));
    }
}
