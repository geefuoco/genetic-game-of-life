//! Headless core of the genetic game of life: a torus World running a
//! configurable Rule (Conway's Life by default) from a seeded Soup.

mod rng;
mod rule;
mod world;

pub use rng::Rng;
pub use rule::{Rule, RuleError};
pub use world::{World, WorldError, MAX_SIZE, MIN_SIZE};
