use std::fmt;

/// Reasons Mutation settings are rejected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MutationError {
    /// The rate is not within `0.0..=1.0`.
    RateOutOfRange(f64),
    /// The magnitude is not within `0.0..=180.0` degrees.
    MagnitudeOutOfRange(f64),
}

impl fmt::Display for MutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateOutOfRange(rate) => write!(f, "mutation rate {rate} is outside 0.0..=1.0"),
            Self::MagnitudeOutOfRange(magnitude) => write!(
                f,
                "mutation magnitude {magnitude} is outside 0.0..=180.0 degrees"
            ),
        }
    }
}

impl std::error::Error for MutationError {}

/// How often and how far Mutation changes an Allele as it is passed on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MutationSettings {
    rate: f64,
    magnitude: f64,
}

impl Default for MutationSettings {
    /// A 1% rate and a 30 degree magnitude.
    fn default() -> Self {
        Self {
            rate: 0.01,
            magnitude: 30.0,
        }
    }
}

impl MutationSettings {
    /// `rate` is the probability, `0.0..=1.0`, that each Allele passed on
    /// mutates; a mutant's hue moves by up to `magnitude` degrees,
    /// `0.0..=180.0`, either way.
    pub fn new(rate: f64, magnitude: f64) -> Result<Self, MutationError> {
        if !(0.0..=1.0).contains(&rate) {
            return Err(MutationError::RateOutOfRange(rate));
        }
        if !(0.0..=180.0).contains(&magnitude) {
            return Err(MutationError::MagnitudeOutOfRange(magnitude));
        }
        Ok(Self { rate, magnitude })
    }

    /// Probability that an Allele mutates as it is passed on.
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Largest hue shift of a mutant from its source Allele, in degrees.
    pub fn magnitude(&self) -> f64 {
        self.magnitude
    }
}
