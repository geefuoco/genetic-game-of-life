//! Headless core of the genetic game of life: a torus World running a
//! configurable Rule (Conway's Life by default) from a seeded Soup.

mod genetics;
mod rng;
mod rule;
mod world;

pub use genetics::{Allele, AlleleCensus, Census, DEAD_PHENOTYPE, MAX_DOMINANCE};
pub use rng::Rng;
pub use rule::{Rule, RuleError};
pub use world::{
    World, WorldError, DEFAULT_FOUNDING_ALLELES, MAX_FOUNDING_ALLELES, MAX_SIZE, MIN_SIZE,
};
