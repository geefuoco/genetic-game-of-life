use comment_check::{check_source, Violation};

fn lines(source: &str) -> Vec<usize> {
    check_source(source)
        .iter()
        .map(|v: &Violation| v.line)
        .collect()
}

#[test]
fn rejects_line_comments() {
    assert_eq!(lines("fn a() {}\n// note\n"), [2]);
    assert_eq!(lines("let x = 1; // trailing"), [1]);
}

#[test]
fn rejects_block_comments_including_nested() {
    assert_eq!(lines("/* a */ fn a() {}"), [1]);
    assert_eq!(lines("fn a() {}\n/* outer /* inner */ still */\n"), [2]);
}

#[test]
fn rejects_lint_suppression_comments() {
    assert_eq!(lines("// allow(clippy::all)\nfn a() {}"), [1]);
    assert_eq!(lines("// noqa\nfn a() {}"), [1]);
}

#[test]
fn rejects_comment_lookalikes_that_are_not_doc_comments() {
    assert_eq!(lines("//// four slashes"), [1]);
    assert_eq!(lines("/**/ fn a() {}"), [1]);
    assert_eq!(lines("/*** three stars */"), [1]);
}

#[test]
fn allows_doc_comments() {
    let source =
        "//! crate docs\n/// item docs\n/** block docs */\n/*! inner block docs */\nfn a() {}\n";
    assert_eq!(lines(source), Vec::<usize>::new());
}

#[test]
fn ignores_comment_markers_inside_literals() {
    let source = r##"
let a = "// not a comment";
let b = "escaped \" // still string";
let c = r#"/* raw " // still raw */"#;
let d = b"/* bytes */";
let e = '/';
let f: &'static str = "x";
"##;
    assert_eq!(lines(source), Vec::<usize>::new());
}

#[test]
fn finds_a_comment_after_a_lifetime_or_char_literal() {
    assert_eq!(lines("fn a<'a>(x: &'a str) {} // c"), [1]);
    assert_eq!(lines("let c = '\"'; // c"), [1]);
}

#[test]
fn reports_each_comment_on_the_right_line() {
    assert_eq!(lines("// a\n\n// b\nfn x() {}\n/* c\nd */"), [1, 3, 5]);
}
