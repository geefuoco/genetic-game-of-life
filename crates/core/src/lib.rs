//! Headless core of the genetic game of life: a torus World running a
//! configurable Rule (Conway's Life by default) from a seeded Soup.

mod allele_tree;
mod genetics;
mod mutation;
mod rng;
mod rule;
mod snapshot;
mod soup;
mod world;

pub use allele_tree::{AlleleId, AlleleNode, AlleleStatus};
pub use genetics::{Allele, AlleleCensus, Census, DEAD_PHENOTYPE, MAX_DOMINANCE};
pub use mutation::{MutationError, MutationSettings};
pub use rng::Rng;
pub use rule::{Rule, RuleError};
pub use snapshot::{SnapshotError, SNAPSHOT_VERSION};
pub use soup::{ShareLinkError, Soup};
pub use world::{
    World, WorldError, DEFAULT_FOUNDING_ALLELES, MAX_FOUNDING_ALLELES, MAX_SIZE, MIN_SIZE,
};
