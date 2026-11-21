//! Thin wasm-bindgen layer over the core: a Soup World the web app's worker
//! steps and reads Phenotypes from.

use genetic_life_core::{MutationSettings, Rule, Soup, World};
use std::fmt::Display;
use wasm_bindgen::prelude::*;

/// A Soup World, driven from JavaScript.
#[wasm_bindgen]
pub struct WebWorld {
    world: World,
}

#[wasm_bindgen]
impl WebWorld {
    /// Creates a Soup under Conway's Life with the default Mutation settings;
    /// `setRule` and `setMutation` change them before or during the run.
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
        let world = soup.world().map_err(js_error)?;
        Ok(Self { world })
    }

    /// Switches to `rule`, given in B/S notation, from the next Generation on,
    /// and returns it in canonical notation. Invalid notation leaves the Rule
    /// unchanged and fails with the core's explanation.
    #[wasm_bindgen(js_name = setRule)]
    pub fn set_rule(&mut self, rule: &str) -> Result<String, JsError> {
        let rule = Rule::parse(rule).map_err(js_error)?;
        self.world.set_rule(rule);
        Ok(rule.to_string())
    }

    /// Switches the Mutation settings from the next Generation on.
    #[wasm_bindgen(js_name = setMutation)]
    pub fn set_mutation(&mut self, rate: f64, magnitude: f64) -> Result<(), JsError> {
        let mutation = MutationSettings::new(rate, magnitude).map_err(js_error)?;
        self.world.set_mutation(mutation);
        Ok(())
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

fn js_error(error: impl Display) -> JsError {
    JsError::new(&error.to_string())
}
