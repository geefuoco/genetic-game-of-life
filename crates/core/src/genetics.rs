//! Pigment: the Gene whose Alleles are hues with a Dominance Level.
//!
//! Only add, subtract, multiply and divide are used on hues, so results are
//! identical natively and in WASM.

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

/// Index of an Allele in a World's Allele table.
pub(crate) type AlleleId = u16;

/// The pair of Alleles a live Cell carries.
pub(crate) type Genotype = [AlleleId; 2];

/// Census line for one living Allele.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlleleCensus {
    pub allele: Allele,
    /// Copies carried by live Cells; each Organism carries two.
    pub copies: usize,
    /// Organisms whose Phenotype shows this Allele (a blend shows both).
    pub shown: usize,
    /// Organisms carrying this Allele that the Phenotype does not show.
    pub hidden: usize,
}

/// Snapshot of the genetic state of a World.
#[derive(Debug, Clone, PartialEq)]
pub struct Census {
    pub population: usize,
    pub generation: u64,
    /// Every living Allele, in Founding order.
    pub alleles: Vec<AlleleCensus>,
    /// Living Alleles that no Organism shows.
    pub fully_hidden: Vec<Allele>,
    /// Population at Generations `0..=generation`.
    pub population_history: Vec<usize>,
}

fn wrap(hue: f64) -> f64 {
    if hue < 0.0 {
        hue + 360.0
    } else if hue >= 360.0 {
        hue - 360.0
    } else {
        hue
    }
}

/// Hue of an Organism carrying `a` and `b`.
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
