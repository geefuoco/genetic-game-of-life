//! Thin wasm-bindgen layer over the core: a Soup World the web app's worker
//! steps and reads Phenotypes from.

use genetic_life_core::{MutationSettings, Rule, Soup, World};
use wasm_bindgen::prelude::*;

/// A Soup World running Conway's Life, driven from JavaScript.
#[wasm_bindgen]
pub struct WebWorld {
    world: World,
}

#[wasm_bindgen]
impl WebWorld {
    /// Creates a Soup under Conway's Life with the default Mutation settings.
    #[wasm_bindgen(constructor)]
    pub fn new(
        seed: u64,
        width: usize,
        height: usize,
        density: f64,
        founding_alleles: usize,
    ) -> Result<WebWorld, JsError> {
        let soup = Soup {
            seed,
            rule: Rule::CONWAY,
            width,
            height,
            density,
            founding_alleles,
            mutation: MutationSettings::default(),
        };
        let world = soup
            .world()
            .map_err(|error| JsError::new(&error.to_string()))?;
        Ok(Self { world })
    }

    /// Applies the Rule `generations` times.
    pub fn step(&mut self, generations: u32) {
        self.world.run(u64::from(generations));
    }

    /// Generations applied since the Soup was created.
    pub fn generation(&self) -> f64 {
        self.world.generation() as f64
    }

    /// Number of live Cells.
    pub fn population(&self) -> u32 {
        self.world.population() as u32
    }

    /// Writes one Phenotype hue per Cell into `out` in row-major order, with
    /// dead Cells as the core's dead Phenotype value.
    pub fn phenotypes(&self, out: &mut [f32]) -> Result<(), JsError> {
        let hues = self.world.phenotypes();
        if out.len() != hues.len() {
            return Err(JsError::new(&format!(
                "phenotype buffer holds {} cells but the world has {}",
                out.len(),
                hues.len()
            )));
        }
        for (slot, hue) in out.iter_mut().zip(hues) {
            *slot = hue as f32;
        }
        Ok(())
    }
}
