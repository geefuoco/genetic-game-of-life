//! Inspecting one Organism: one continuous life at a Cell, identified by its
//! Cell and its Birth Generation.

use crate::allele_tree::{AlleleId, AlleleNode, AlleleStatus};
use crate::genetics::{phenotype, shows, Allele};

/// What the inspector shows about an Organism.
#[derive(Debug, Clone, PartialEq)]
pub struct Inspection {
    /// Column of the Organism's Cell.
    pub x: usize,
    /// Row of the Organism's Cell.
    pub y: usize,
    /// The Generation of the Organism's Birth, or of its painting or stamping.
    pub birth_generation: u64,
    /// The first Generation the Organism was no longer live in; `None` while
    /// it lives.
    pub death_generation: Option<u64>,
    /// The Organism's Genotype.
    pub alleles: [InspectedAllele; 2],
    /// The Organism's Phenotype hue.
    pub phenotype: f64,
}

/// One Allele of an inspected Organism's Genotype.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectedAllele {
    pub id: AlleleId,
    pub allele: Allele,
    /// Whether the Organism's Phenotype shows this Allele; a blend shows both.
    pub shown: bool,
    /// This Allele and the Alleles it descends from, back to its Founding
    /// Allele. Alleles pruned since the Organism died are extinct, last
    /// carried no later than when they were last seen retained.
    pub lineage: Vec<AlleleNode>,
}

/// The Organism a World follows for its inspector.
#[derive(Debug, Clone)]
pub(crate) struct InspectedOrganism {
    /// Row-major index of its Cell.
    pub(crate) index: usize,
    pub(crate) birth_generation: u64,
    /// Whether it was painted, stamped or live in the World it started as,
    /// rather than Born.
    pub(crate) placed: bool,
    /// Each Allele's lineage, starting with the Allele, as it stood when
    /// inspection began; Alleles never change their source or origin, only
    /// their status and when they were last carried.
    pub(crate) lineages: [Vec<AlleleNode>; 2],
    pub(crate) death_generation: Option<u64>,
}

impl InspectedOrganism {
    /// The Organism's Genotype.
    pub(crate) fn alleles(&self) -> [Allele; 2] {
        self.lineages.each_ref().map(|lineage| lineage[0].allele)
    }

    /// The inspection of this Organism in a World `width` Cells wide, where
    /// `node` gives each Allele's node as it stands now, or `None` once it is
    /// pruned.
    pub(crate) fn inspection(
        &self,
        width: usize,
        node: impl Fn(AlleleId) -> Option<AlleleNode>,
    ) -> Inspection {
        let [a, b] = self.alleles();
        let (shows_a, shows_b) = shows(a, b);
        let allele = |i: usize, shown: bool| InspectedAllele {
            id: self.lineages[i][0].id,
            allele: self.lineages[i][0].allele,
            shown,
            lineage: self.lineages[i]
                .iter()
                .map(|then| {
                    node(then.id).unwrap_or(AlleleNode {
                        status: AlleleStatus::Extinct,
                        ..*then
                    })
                })
                .collect(),
        };
        Inspection {
            x: self.index % width,
            y: self.index / width,
            birth_generation: self.birth_generation,
            death_generation: self.death_generation,
            alleles: [allele(0, shows_a), allele(1, shows_b)],
            phenotype: phenotype(a, b),
        }
    }
}
