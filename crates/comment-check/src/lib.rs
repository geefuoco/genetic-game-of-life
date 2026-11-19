//! Lexer-based check that bans ordinary comments, and with them lint
//! suppression comments, in Rust sources. Doc comments are allowed.

use std::fmt;

/// An ordinary comment found in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// 1-based line on which the comment starts.
    pub line: usize,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ordinary comment", self.line)
    }
}

/// Returns every ordinary comment in `source`, in order of appearance.
pub fn check_source(source: &str) -> Vec<Violation> {
    let chars: Vec<char> = source.chars().collect();
    let mut violations = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\n' => {
                line += 1;
                i += 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                let doc = is_line_doc(&chars[i..]);
                if !doc {
                    violations.push(Violation { line });
                }
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if chars.get(i + 1) == Some(&'*') => {
                let doc = is_block_doc(&chars[i..]);
                if !doc {
                    violations.push(Violation { line });
                }
                let (end, newlines) = skip_block_comment(&chars, i);
                line += newlines;
                i = end;
            }
            '"' => {
                let (end, newlines) = skip_string(&chars, i + 1);
                line += newlines;
                i = end;
            }
            '\'' => i = skip_char_or_lifetime(&chars, i),
            c if c == '_' || c.is_alphabetic() => {
                let start = i;
                while i < chars.len() && (chars[i] == '_' || chars[i].is_alphanumeric()) {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                if matches!(ident.as_str(), "r" | "br" | "cr") {
                    if let Some((end, newlines)) = skip_raw_string(&chars, i) {
                        line += newlines;
                        i = end;
                    }
                }
            }
            _ => i += 1,
        }
    }
    violations
}

fn is_line_doc(rest: &[char]) -> bool {
    match rest.get(2) {
        Some('!') => true,
        Some('/') => rest.get(3) != Some(&'/'),
        _ => false,
    }
}

fn is_block_doc(rest: &[char]) -> bool {
    match rest.get(2) {
        Some('!') => true,
        Some('*') => !matches!(rest.get(3), Some('*') | Some('/')),
        _ => false,
    }
}

fn skip_block_comment(chars: &[char], start: usize) -> (usize, usize) {
    let mut depth = 0;
    let mut newlines = 0;
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            i += 2;
        } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                break;
            }
        } else {
            if chars[i] == '\n' {
                newlines += 1;
            }
            i += 1;
        }
    }
    (i, newlines)
}

fn skip_string(chars: &[char], start: usize) -> (usize, usize) {
    let mut newlines = 0;
    let mut i = start;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                if chars.get(i + 1) == Some(&'\n') {
                    newlines += 1;
                }
                i += 2;
            }
            '"' => return (i + 1, newlines),
            c => {
                if c == '\n' {
                    newlines += 1;
                }
                i += 1;
            }
        }
    }
    (chars.len(), newlines)
}

fn skip_raw_string(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let mut i = start;
    let mut hashes = 0;
    while chars.get(i) == Some(&'#') {
        hashes += 1;
        i += 1;
    }
    if chars.get(i) != Some(&'"') {
        return None;
    }
    i += 1;
    let mut newlines = 0;
    while i < chars.len() {
        if chars[i] == '"' && (0..hashes).all(|k| chars.get(i + 1 + k) == Some(&'#')) {
            return Some((i + 1 + hashes, newlines));
        }
        if chars[i] == '\n' {
            newlines += 1;
        }
        i += 1;
    }
    Some((chars.len(), newlines))
}

fn skip_char_or_lifetime(chars: &[char], start: usize) -> usize {
    let is_char_literal = match chars.get(start + 1) {
        Some('\\') => true,
        Some(_) => chars.get(start + 2) == Some(&'\''),
        None => false,
    };
    if !is_char_literal {
        return start + 1;
    }
    let mut i = start + 1;
    if chars[i] == '\\' {
        i += 2;
    } else {
        i += 1;
    }
    while i < chars.len() && chars[i] != '\'' && chars[i] != '\n' {
        i += 1;
    }
    (i + 1).min(chars.len())
}
