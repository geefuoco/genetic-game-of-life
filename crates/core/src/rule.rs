use std::fmt;
use std::str::FromStr;

/// Reasons a Rule cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleError {
    /// The notation is not two `/`-separated parts.
    WrongPartCount(usize),
    /// The first part does not start with `B`.
    MissingBirth,
    /// The second part does not start with `S`.
    MissingSurvival,
    /// A character after `B` or `S` is not a digit.
    NotADigit(char),
    /// A digit is greater than 8, the most Neighbours a Cell has.
    CountTooLarge(char),
    /// A count appears more than once in the same set.
    DuplicateCount(char),
    /// The name matches no Rule Preset.
    UnknownPreset(String),
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPartCount(parts) => write!(
                f,
                "expected two parts separated by '/' like B3/S23, found {parts}"
            ),
            Self::MissingBirth => write!(f, "the first part must start with 'B', like B3"),
            Self::MissingSurvival => write!(f, "the second part must start with 'S', like S23"),
            Self::NotADigit(c) => write!(f, "'{c}' is not a digit from 0 to 8"),
            Self::CountTooLarge(c) => {
                write!(f, "neighbour count {c} is too large, the maximum is 8")
            }
            Self::DuplicateCount(c) => write!(f, "neighbour count {c} is listed twice"),
            Self::UnknownPreset(name) => write!(
                f,
                "'{name}' is neither a Rule Preset ({}) nor B/S notation like B3/S23",
                Rule::PRESETS.map(|(name, _)| name).join(", ")
            ),
        }
    }
}

impl std::error::Error for RuleError {}

/// An outer-totalistic Moore-neighbourhood Rule: which live Neighbour counts
/// cause Birth and which allow Survival.
///
/// Each set is a bitmask where bit `n` means "a count of `n` qualifies".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    birth: u16,
    survival: u16,
}

impl Default for Rule {
    fn default() -> Self {
        Self::CONWAY
    }
}

impl Rule {
    /// Conway's Life, B3/S23.
    pub const CONWAY: Rule = Rule::from_masks(1 << 3, (1 << 2) | (1 << 3));

    /// Named Rule Presets as `(name, Rule)`, Conway's Life first.
    pub const PRESETS: [(&'static str, Rule); 6] = [
        ("conway", Self::CONWAY),
        ("highlife", Rule::from_masks(0b0_0100_1000, 0b0_0000_1100)),
        (
            "day-and-night",
            Rule::from_masks(0b1_1100_1000, 0b1_1101_1000),
        ),
        ("seeds", Rule::from_masks(0b0_0000_0100, 0)),
        ("gnarl", Rule::from_masks(0b0_0000_0010, 0b0_0000_0010)),
        (
            "life-without-death",
            Rule::from_masks(0b0_0000_1000, 0b1_1111_1111),
        ),
    ];

    const fn from_masks(birth: u16, survival: u16) -> Self {
        Self { birth, survival }
    }

    /// Parses B/S notation such as `B3/S23`. Case-insensitive; `B0` and empty
    /// sets (`B2/S`) are allowed.
    pub fn parse(notation: &str) -> Result<Self, RuleError> {
        let parts: Vec<&str> = notation.split('/').collect();
        let [birth, survival] = parts[..] else {
            return Err(RuleError::WrongPartCount(parts.len()));
        };
        let birth = parse_counts(birth, 'B').ok_or(RuleError::MissingBirth)??;
        let survival = parse_counts(survival, 'S').ok_or(RuleError::MissingSurvival)??;
        Ok(Self { birth, survival })
    }

    /// Looks up a Rule Preset by name, ignoring case; `&` reads as "and" and
    /// any non-alphanumeric character is a word break, so "Day & Night" finds `day-and-night`.
    pub fn preset(name: &str) -> Option<Self> {
        let slug = name
            .to_lowercase()
            .replace('&', " and ")
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        Self::PRESETS
            .iter()
            .find(|(preset, _)| *preset == slug)
            .map(|&(_, rule)| rule)
    }

    /// Resolves user input that is either a Rule Preset name or B/S notation.
    pub fn resolve(input: &str) -> Result<Self, RuleError> {
        if let Some(rule) = Self::preset(input) {
            return Ok(rule);
        }
        if input.contains('/') {
            return Self::parse(input);
        }
        Err(RuleError::UnknownPreset(input.to_string()))
    }

    /// Neighbour counts that cause Birth, ascending.
    pub fn birth_counts(&self) -> Vec<u8> {
        counts(self.birth)
    }

    /// Neighbour counts that allow Survival, ascending.
    pub fn survival_counts(&self) -> Vec<u8> {
        counts(self.survival)
    }

    pub(crate) fn next_state(&self, alive: bool, neighbours: usize) -> bool {
        let mask = if alive { self.survival } else { self.birth };
        mask >> neighbours & 1 == 1
    }
}

impl FromStr for Rule {
    type Err = RuleError;

    fn from_str(notation: &str) -> Result<Self, RuleError> {
        Self::parse(notation)
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B")?;
        for count in self.birth_counts() {
            write!(f, "{count}")?;
        }
        write!(f, "/S")?;
        for count in self.survival_counts() {
            write!(f, "{count}")?;
        }
        Ok(())
    }
}

fn counts(mask: u16) -> Vec<u8> {
    (0..=8).filter(|n| mask >> n & 1 == 1).collect()
}

fn parse_counts(part: &str, prefix: char) -> Option<Result<u16, RuleError>> {
    let digits = part
        .strip_prefix(prefix)
        .or_else(|| part.strip_prefix(prefix.to_ascii_lowercase()))?;
    let mut mask = 0u16;
    for c in digits.chars() {
        let Some(count) = c.to_digit(10) else {
            return Some(Err(RuleError::NotADigit(c)));
        };
        if count > 8 {
            return Some(Err(RuleError::CountTooLarge(c)));
        }
        if mask >> count & 1 == 1 {
            return Some(Err(RuleError::DuplicateCount(c)));
        }
        mask |= 1 << count;
    }
    Some(Ok(mask))
}
