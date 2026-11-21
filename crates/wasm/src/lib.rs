//! Thin wasm-bindgen layer over the core: a Soup World the web app's worker
//! steps and reads Phenotypes from.

use genetic_life_core::{Allele, MutationSettings, Pattern, Rule, Soup, World};
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

    /// Makes the Cell at `(x, y)` live as part of the open stroke with
    /// `genotype`: its two Alleles as `[hue, dominance, hue, dominance]`.
    pub fn paint(&mut self, x: usize, y: usize, genotype: &[f64]) -> Result<(), JsError> {
        let genotype = alleles(genotype)?;
        self.world.paint(x, y, genotype).map_err(js_error)
    }

    /// Makes the Cell at `(x, y)` dead as part of the open stroke.
    pub fn erase(&mut self, x: usize, y: usize) -> Result<(), JsError> {
        self.world.erase(x, y).map_err(js_error)
    }

    /// Stamps the Pattern called `pattern` centred on `(x, y)` as part of the
    /// open stroke, its live Cells carrying `genotype` as for `paint`.
    pub fn stamp(
        &mut self,
        pattern: &str,
        x: usize,
        y: usize,
        genotype: &[f64],
    ) -> Result<(), JsError> {
        let named = Pattern::named(pattern)
            .ok_or_else(|| JsError::new(&format!("there is no Pattern called '{pattern}'")))?;
        let genotype = alleles(genotype)?;
        self.world.stamp(named, x, y, genotype).map_err(js_error)
    }

    /// Closes the open stroke, so the next edit starts a new one.
    #[wasm_bindgen(js_name = endStroke)]
    pub fn end_stroke(&mut self) {
        self.world.end_stroke();
    }

    /// Reverts the latest stroke of this Generation; returns whether there was one.
    pub fn undo(&mut self) -> bool {
        self.world.undo()
    }

    /// Reapplies the latest undone stroke; returns whether there was one.
    pub fn redo(&mut self) -> bool {
        self.world.redo()
    }

    /// Whether `undo` would revert a stroke.
    #[wasm_bindgen(js_name = canUndo)]
    pub fn can_undo(&self) -> bool {
        self.world.can_undo()
    }

    /// Whether `redo` would reapply a stroke.
    #[wasm_bindgen(js_name = canRedo)]
    pub fn can_redo(&self) -> bool {
        self.world.can_redo()
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

/// The two Alleles in `[hue, dominance, hue, dominance]`.
fn alleles(genotype: &[f64]) -> Result<[Allele; 2], JsError> {
    let &[first_hue, first_dominance, second_hue, second_dominance] = genotype else {
        return Err(JsError::new(&format!(
            "a genotype is 4 numbers, hue and dominance for each allele, not {}",
            genotype.len()
        )));
    };
    Ok([
        allele(first_hue, first_dominance)?,
        allele(second_hue, second_dominance)?,
    ])
}

fn allele(hue: f64, dominance: f64) -> Result<Allele, JsError> {
    if dominance.fract() != 0.0 || !(0.0..=f64::from(u8::MAX)).contains(&dominance) {
        return Err(JsError::new(&format!(
            "dominance {dominance} is not a whole number from 0 to 255"
        )));
    }
    Ok(Allele {
        hue,
        dominance: dominance as u8,
    })
}

fn js_error(error: impl Display) -> JsError {
    JsError::new(&error.to_string())
}
