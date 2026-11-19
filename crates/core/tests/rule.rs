use genetic_life_core::{Rule, RuleError};

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
fn display_gives_canonical_notation() {
    assert_eq!(parsed("b63/s32").to_string(), "B36/S23");
    assert_eq!(parsed("B/S").to_string(), "B/S");
}

#[test]
fn invalid_notation_is_rejected_with_the_matching_error() {
    let cases = [
        ("", RuleError::WrongPartCount(1)),
        ("B3", RuleError::WrongPartCount(1)),
        ("B3/S23/X", RuleError::WrongPartCount(3)),
        ("S23/B3", RuleError::MissingBirth),
        ("3/S23", RuleError::MissingBirth),
        ("B3/23", RuleError::MissingSurvival),
        ("B9/S23", RuleError::CountTooLarge('9')),
        ("B3/S2x", RuleError::NotADigit('x')),
        ("B3/S2 3", RuleError::NotADigit(' ')),
        ("B33/S23", RuleError::DuplicateCount('3')),
    ];
    for (input, expected) in cases {
        assert_eq!(Rule::parse(input), Err(expected), "{input:?}");
    }
}

#[test]
fn error_messages_explain_the_problem() {
    let message = |input| Rule::parse(input).unwrap_err().to_string();
    assert!(message("B9/S23").contains("maximum is 8"));
    assert!(message("B33/S23").contains("listed twice"));
    assert!(message("S23/B3").contains("start with 'B'"));
}
