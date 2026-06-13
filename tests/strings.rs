use r_vlr_util::strings::{SplitOptions, split_path};

#[test]
fn split_path_discards_empty_elements_by_default() {
    assert_eq!(
        split_path("/etc//something/", SplitOptions::default()),
        ["etc", "something"]
    );
}

#[test]
fn split_path_can_keep_each_kind_of_empty_element() {
    let options = SplitOptions {
        keep_leading_empty: true,
        keep_trailing_empty: true,
        keep_consecutive_empty: true,
    };

    assert_eq!(
        split_path("/etc//something/", options),
        ["", "etc", "", "something", ""]
    );
}
