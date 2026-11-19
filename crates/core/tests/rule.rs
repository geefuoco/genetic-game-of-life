use genetic_life_core::Rule;

fn parsed(notation: &str) -> Rule {
    Rule::parse(notation).unwrap()
}

#[test]
fn parses_conways_life() {
    let rule = parsed("B3/S23");
    assert_eq!(rule.birth_counts(), vec![3]);
    assert_eq!(rule.survival_counts(), vec![2, 3]);
}

#[test]
fn parses_the_full_range_of_counts_including_b0() {
    let rule = parsed("B012345678/S012345678");
    assert_eq!(rule.birth_counts(), (0..=8).collect::<Vec<u8>>());
    assert_eq!(rule.survival_counts(), (0..=8).collect::<Vec<u8>>());
}

#[test]
fn parses_empty_sets() {
    let rule = parsed("B2/S");
    assert_eq!(rule.birth_counts(), vec![2]);
    assert!(rule.survival_counts().is_empty());
    assert!(parsed("B/S").birth_counts().is_empty());
}

#[test]
fn parsing_is_case_insensitive_and_ignores_digit_order() {
    assert_eq!(parsed("b32/s32"), parsed("B23/S23"));
}

#[test]
fn display_round_trips_to_canonical_notation() {
    assert_eq!(parsed("b63/s32").to_string(), "B36/S23");
    assert_eq!(parsed("B/S").to_string(), "B/S");
}

#[test]
fn invalid_notation_is_rejected_with_a_descriptive_error() {
    let cases = [
        ("", "B"),
        ("B3", "S"),
        ("S23/B3", "B"),
        ("B3/S23/X", "two"),
        ("B9/S23", "9"),
        ("B3/S2x", "x"),
        ("B33/S23", "twice"),
        ("B3/S2 3", "' '"),
    ];
    for (input, hint) in cases {
        let error = Rule::parse(input).unwrap_err().to_string();
        assert!(error.contains(hint), "{input:?} gave {error:?}");
    }
}
