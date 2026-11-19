//! Pigment: the Gene whose Alleles are hues with a Dominance Level.
//!
//! Only add, subtract, multiply and divide are used on hues, so results are
//! identical natively and in WASM.

use crate::allele_tree::{AlleleId, Slot};

/// Highest Dominance Level an Allele can have.
pub const MAX_DOMINANCE: u8 = 3;

/// Phenotype value reported for a dead Cell. Live hues are in `0.0..360.0`.
pub const DEAD_PHENOTYPE: f64 = -1.0;

/// One variant of the Pigment Gene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Allele {
    /// Hue in degrees, `0.0..360.0`.
    pub hue: f64,
    /// Dominance Level, `0..=MAX_DOMINANCE`.
    pub dominance: u8,
}

impl Allele {
    /// Whether the hue is in `0.0..360.0` and the Dominance Level at most
    /// [`MAX_DOMINANCE`].
    pub(crate) fn is_valid(&self) -> bool {
        (0.0..360.0).contains(&self.hue) && self.dominance <= MAX_DOMINANCE
    }
}

/// The pair of Alleles a live Cell carries.
pub(crate) type Genotype = [Slot; 2];

/// Census line for one living Allele.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlleleCensus {
    pub id: AlleleId,
    pub allele: Allele,
    /// Copies carried by live Cells; each live Cell carries two.
    pub copies: usize,
    /// live Cells whose Phenotype shows this Allele (a blend shows both).
    pub shown: usize,
    /// live Cells carrying this Allele that the Phenotype does not show.
    pub hidden: usize,
}

/// Snapshot of the genetic state of a World.
#[derive(Debug, Clone, PartialEq)]
pub struct Census {
    pub population: usize,
    pub generation: u64,
    /// Every living Allele, in id order.
    pub alleles: Vec<AlleleCensus>,
    /// Living Alleles that no live Cell shows.
    pub fully_hidden: Vec<Allele>,
    /// Population at Generations `0..=generation`.
    pub population_history: Vec<usize>,
}

/// Wraps a hue within one turn of the wheel into `0.0..360.0`.
pub(crate) fn wrap(hue: f64) -> f64 {
    let wrapped = if hue < 0.0 {
        hue + 360.0
    } else if hue >= 360.0 {
        hue - 360.0
    } else {
        hue
    };
    if wrapped >= 360.0 {
        0.0
    } else {
        wrapped
    }
}

/// Hue of a live Cell carrying `a` and `b`.
pub(crate) fn phenotype(a: Allele, b: Allele) -> f64 {
    if a.hue == b.hue {
        return a.hue;
    }
    if a.dominance != b.dominance {
        return if a.dominance > b.dominance {
            a.hue
        } else {
            b.hue
        };
    }
    let d = wrap(b.hue - a.hue);
    if d == 180.0 {
        return wrap(a.hue.min(b.hue) + 90.0);
    }
    if d < 180.0 {
        wrap(a.hue + d / 2.0)
    } else {
        wrap(a.hue - (360.0 - d) / 2.0)
    }
}

/// Whether the Phenotype of `a`/`b` shows (`.0`) / (`.1`) each Allele.
pub(crate) fn shows(a: Allele, b: Allele) -> (bool, bool) {
    if a.hue == b.hue || a.dominance == b.dominance {
        (true, true)
    } else {
        (a.dominance > b.dominance, b.dominance > a.dominance)
    }
}
