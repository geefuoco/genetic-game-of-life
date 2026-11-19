//! A Soup World's Seed, Rule and settings, and their Share Link.

use crate::mutation::{MutationError, MutationSettings};
use crate::rule::{Rule, RuleError};
use crate::world::{check_soup, World, WorldError};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Share Link format version written by [`Soup::share_link`].
const SHARE_LINK_VERSION: &str = "1";

/// Share Link field names, in the order [`Soup::share_link`] writes them.
const FIELDS: [&str; 9] = [
    "v",
    "seed",
    "rule",
    "width",
    "height",
    "density",
    "alleles",
    "mutation-rate",
    "mutation-magnitude",
];

/// Reasons a Share Link cannot be decoded.
#[derive(Debug, Clone, PartialEq)]
pub enum ShareLinkError {
    /// A part between `&` separators is not `field=value`.
    MalformedPair(String),
    /// The `v` field is not a version this build reads.
    UnsupportedVersion(String),
    /// A required field is absent.
    MissingField(&'static str),
    /// A field appears more than once.
    DuplicateField(String),
    /// A field is not part of the format.
    UnknownField(String),
    /// A numeric field's value does not parse.
    InvalidValue { field: &'static str, value: String },
    /// The Rule does not parse.
    Rule(RuleError),
    /// The Mutation settings are out of range.
    Mutation(MutationError),
    /// The World settings are out of range.
    World(WorldError),
}

impl fmt::Display for ShareLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedPair(part) => {
                write!(f, "share link part '{part}' is not a field=value pair")
            }
            Self::UnsupportedVersion(version) => write!(
                f,
                "share link version '{version}' is not supported, this build reads version {SHARE_LINK_VERSION}"
            ),
            Self::MissingField(field) => write!(f, "share link is missing the '{field}' field"),
            Self::DuplicateField(field) => {
                write!(f, "share link gives the '{field}' field more than once")
            }
            Self::UnknownField(field) => write!(f, "share link has an unknown field '{field}'"),
            Self::InvalidValue { field, value } => write!(
                f,
                "share link field '{field}' has value '{value}', which is not a valid number"
            ),
            Self::Rule(error) => write!(f, "share link rule is invalid: {error}"),
            Self::Mutation(error) => write!(f, "share link mutation settings are invalid: {error}"),
            Self::World(error) => write!(f, "share link world settings are invalid: {error}"),
        }
    }
}

impl std::error::Error for ShareLinkError {}

/// Everything that fixes a Soup World run: its Seed, Rule and settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Soup {
    pub seed: u64,
    pub rule: Rule,
    /// World width in Cells.
    pub width: usize,
    /// World height in Cells.
    pub height: usize,
    /// Probability that a Cell starts live.
    pub density: f64,
    /// Number of Founding Alleles.
    pub founding_alleles: usize,
    pub mutation: MutationSettings,
}

impl Soup {
    /// The Soup World at Generation 0, running this Rule and Mutation.
    pub fn world(&self) -> Result<World, WorldError> {
        let mut world = World::soup_with_alleles(
            self.width,
            self.height,
            self.density,
            self.seed,
            self.founding_alleles,
        )?;
        world.set_rule(self.rule);
        world.set_mutation(self.mutation);
        Ok(world)
    }

    /// The Share Link: `field=value` pairs joined by `&`, starting with the
    /// format version `v`, suitable for a URL fragment. Numbers are written so
    /// that [`Soup::from_share_link`] reads back exactly the same values.
    pub fn share_link(&self) -> String {
        let values = [
            SHARE_LINK_VERSION.to_string(),
            self.seed.to_string(),
            self.rule.to_string(),
            self.width.to_string(),
            self.height.to_string(),
            self.density.to_string(),
            self.founding_alleles.to_string(),
            self.mutation.rate().to_string(),
            self.mutation.magnitude().to_string(),
        ];
        FIELDS
            .iter()
            .zip(values)
            .map(|(field, value)| format!("{field}={value}"))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Decodes a Share Link, alone or as the fragment of a URL (everything
    /// after its last `#`). Fields may come in any order; each must appear
    /// exactly once and every value must be in range. The version is checked
    /// before any other field.
    pub fn from_share_link(link: &str) -> Result<Self, ShareLinkError> {
        let fragment = link.rsplit_once('#').map_or(link, |(_, fragment)| fragment);
        let pairs = fragment
            .split('&')
            .map(|part| {
                part.split_once('=')
                    .ok_or_else(|| ShareLinkError::MalformedPair(part.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let version = pairs
            .iter()
            .find(|(field, _)| *field == "v")
            .ok_or(ShareLinkError::MissingField("v"))?
            .1;
        if version != SHARE_LINK_VERSION {
            return Err(ShareLinkError::UnsupportedVersion(version.to_string()));
        }
        let mut values = HashMap::new();
        for (field, value) in pairs {
            if !FIELDS.contains(&field) {
                return Err(ShareLinkError::UnknownField(field.to_string()));
            }
            if values.insert(field, value).is_some() {
                return Err(ShareLinkError::DuplicateField(field.to_string()));
            }
        }
        let soup = Self {
            seed: number(&values, "seed")?,
            rule: Rule::parse(value(&values, "rule")?).map_err(ShareLinkError::Rule)?,
            width: number(&values, "width")?,
            height: number(&values, "height")?,
            density: number(&values, "density")?,
            founding_alleles: number(&values, "alleles")?,
            mutation: MutationSettings::new(
                number(&values, "mutation-rate")?,
                number(&values, "mutation-magnitude")?,
            )
            .map_err(ShareLinkError::Mutation)?,
        };
        check_soup(soup.width, soup.height, soup.density, soup.founding_alleles)
            .map_err(ShareLinkError::World)?;
        Ok(soup)
    }
}

fn value<'a>(
    values: &HashMap<&str, &'a str>,
    field: &'static str,
) -> Result<&'a str, ShareLinkError> {
    values
        .get(field)
        .copied()
        .ok_or(ShareLinkError::MissingField(field))
}

fn number<T: FromStr>(
    values: &HashMap<&str, &str>,
    field: &'static str,
) -> Result<T, ShareLinkError> {
    let text = value(values, field)?;
    text.parse().map_err(|_| ShareLinkError::InvalidValue {
        field,
        value: text.to_string(),
    })
}
