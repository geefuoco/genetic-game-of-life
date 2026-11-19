//! Headless core of the genetic game of life: a torus World running Conway's
//! Life (B3/S23) from a seeded Soup.

mod rng;
mod world;

pub use rng::Rng;
pub use world::{World, WorldError, MAX_SIZE, MIN_SIZE};
