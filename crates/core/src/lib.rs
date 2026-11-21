//! Headless core of the genetic game of life: a torus World running a
//! configurable Rule (Conway's Life by default) from a seeded Soup.

mod allele_tree;
mod edit;
mod genetics;
mod inspection;
mod mutation;
mod pattern;
mod pedigree;
mod rng;
mod rule;
mod snapshot;
mod soup;
mod world;

pub use allele_tree::{AlleleId, AlleleNode, AlleleStatus};
pub use genetics::{Allele, AlleleCensus, Census, DEAD_PHENOTYPE, MAX_DOMINANCE};
pub use inspection::{InspectedAllele, Inspection};
pub use mutation::{MutationError, MutationSettings};
pub use pattern::Pattern;
pub use pedigree::{Origin, Pedigree, PedigreeOrganism, PedigreeParent, MAX_PEDIGREE_BUDGET};
pub use rng::Rng;
pub use rule::{Rule, RuleError};
pub use snapshot::{SnapshotError, SNAPSHOT_VERSION};
pub use soup::{ShareLinkError, Soup};
pub use world::{
    World, WorldError, DEFAULT_FOUNDING_ALLELES, MAX_FOUNDING_ALLELES, MAX_SIZE, MIN_SIZE,
};
