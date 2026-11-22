//! Thin wasm-bindgen layer over the core: a Soup World the web app's worker
//! steps and reads Phenotypes from.

use genetic_life_core::{
    Allele, AlleleId, AlleleStatus, Inspection, MutationSettings, Origin, Pattern, Pedigree, Rule,
    Soup, World,
};
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

    /// Starts following the Organism at `(x, y)` for its inspector, or
    /// follows nothing if the Cell is dead.
    pub fn inspect(&mut self, x: usize, y: usize) -> Result<(), JsError> {
        self.world.inspect(x, y).map(|_| ()).map_err(js_error)
    }

    /// The inspection of the Organism being followed, if any, packed as
    /// `[x, y, birth Generation, death Generation or -1 while it lives,
    /// Phenotype hue]` and then, for each of its two Alleles, `[shown as 1
    /// or hidden as 0, id high, id low, lineage length]` and the lineage from
    /// the Allele back to its Founding Allele as `[hue, dominance, origin
    /// Generation, status]` per Allele, with status 0 living, 1 hidden and 2
    /// extinct. An Allele id is packed as the high and low 32 bits of the
    /// number `carryingCells` takes.
    pub fn inspection(&self) -> Option<Vec<f64>> {
        self.world.inspection().as_ref().map(packed_inspection)
    }

    /// The Pedigree of the Organism being followed, if any, to `depth` Births
    /// back, packed as, per Organism with the followed one first, `[x, y,
    /// Birth Generation or -1 when unknown, depth, origin, Alleles from the
    /// Allele Pool, Parent count]` and then `[Parent's Organism index, Allele
    /// it passed on]` per Parent, with origin 0 Born, 1 painted, stamped or
    /// live in the starting World, 2 beyond Recorded History and 3 Born but
    /// at `depth`, so its Parents were not queried.
    pub fn pedigree(&self, depth: usize) -> Option<Vec<f64>> {
        self.world.pedigree(depth).as_ref().map(packed_pedigree)
    }

    /// Changes the memory budget of Recorded History, in bytes; fails above
    /// the core's maximum.
    #[wasm_bindgen(js_name = setPedigreeBudget)]
    pub fn set_pedigree_budget(&mut self, bytes: usize) -> Result<(), JsError> {
        self.world.set_pedigree_budget(bytes).map_err(js_error)
    }

    /// Stops following an Organism for its inspector.
    #[wasm_bindgen(js_name = stopInspecting)]
    pub fn stop_inspecting(&mut self) {
        self.world.stop_inspecting();
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

    /// The census of living Alleles, packed as `[Generation, population]`
    /// and then, per Allele carried by a live Cell in id order, `[id high, id
    /// low, hue, dominance, copies, live Cells showing it, live Cells hiding
    /// it]`, with ids packed as for `inspection`.
    pub fn census(&self) -> Vec<f64> {
        packed_census(&self.world)
    }

    /// The Allele Tree packed as, per retained Allele with each before its
    /// descendants, `[id high, id low, source id high, source id low, hue,
    /// dominance, origin Generation, Generation last carried in, status]`,
    /// with ids and status packed as for `inspection` and the source id as
    /// `-1, -1` for a Founding Allele.
    #[wasm_bindgen(js_name = alleleTree)]
    pub fn allele_tree(&self) -> Vec<f64> {
        let mut packed = Vec::new();
        for node in self.world.allele_tree() {
            push_id(&mut packed, node.id);
            match node.source {
                Some(source) => push_id(&mut packed, source),
                None => packed.extend([-1.0, -1.0]),
            }
            packed.extend([
                node.allele.hue,
                f64::from(node.allele.dominance),
                node.origin as f64,
                node.last_carried as f64,
                packed_status(node.status),
            ]);
        }
        packed
    }

    /// Writes the Carrier highlight of Allele `id` into `out`, one byte per
    /// Cell in row-major order: 0 for a Cell not carrying it, 1 for one
    /// showing it and 2 for a Carrier hiding it. Returns whether the World
    /// retains Allele `id`; when it does not, every Cell is 0.
    #[wasm_bindgen(js_name = carryingCells)]
    pub fn carrying_cells(&self, id: u64, out: &mut [u8]) -> Result<bool, JsError> {
        let cells = self.world.width() * self.world.height();
        if out.len() != cells {
            return Err(JsError::new(&format!(
                "highlight buffer holds {} cells but the world has {cells}",
                out.len()
            )));
        }
        out.fill(0);
        let Some(carrying) = self.world.carrying_cells(AlleleId::from_bits(id)) else {
            return Ok(false);
        };
        for cell in carrying {
            out[cell.y * self.world.width() + cell.x] = if cell.shown { 1 } else { 2 };
        }
        Ok(true)
    }

    /// Number of live Cells at each Generation from 0 to the current one.
    #[wasm_bindgen(js_name = populationHistory)]
    pub fn population_history(&self) -> Vec<u32> {
        self.world
            .population_history()
            .iter()
            .map(|&population| population as u32)
            .collect()
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

/// `inspection` laid out as `WebWorld::inspection` documents.
fn packed_inspection(inspection: &Inspection) -> Vec<f64> {
    let mut packed = vec![
        inspection.x as f64,
        inspection.y as f64,
        inspection.birth_generation as f64,
        inspection.death_generation.map_or(-1.0, |died| died as f64),
        inspection.phenotype,
    ];
    for allele in &inspection.alleles {
        packed.push(f64::from(u8::from(allele.shown)));
        push_id(&mut packed, allele.id);
        packed.push(allele.lineage.len() as f64);
        for node in &allele.lineage {
            packed.extend([
                node.allele.hue,
                f64::from(node.allele.dominance),
                node.origin as f64,
                packed_status(node.status),
            ]);
        }
    }
    packed
}

/// Appends `id` as its high and low 32 bits.
fn push_id(packed: &mut Vec<f64>, id: AlleleId) {
    let bits = id.to_bits();
    packed.extend([(bits >> 32) as f64, (bits & u64::from(u32::MAX)) as f64]);
}

fn packed_status(status: AlleleStatus) -> f64 {
    match status {
        AlleleStatus::Living => 0.0,
        AlleleStatus::Hidden => 1.0,
        AlleleStatus::Extinct => 2.0,
    }
}

/// The census of `world` laid out as `WebWorld::census` documents.
fn packed_census(world: &World) -> Vec<f64> {
    let mut packed = vec![world.generation() as f64, world.population() as f64];
    for line in world.living_alleles() {
        push_id(&mut packed, line.id);
        packed.extend([
            line.allele.hue,
            f64::from(line.allele.dominance),
            line.copies as f64,
            line.shown as f64,
            line.hidden as f64,
        ]);
    }
    packed
}

/// `pedigree` laid out as `WebWorld::pedigree` documents.
fn packed_pedigree(pedigree: &Pedigree) -> Vec<f64> {
    let mut packed = Vec::new();
    for organism in &pedigree.organisms {
        let (origin, from_allele_pool, parents) = match &organism.origin {
            Origin::Born {
                parents,
                from_allele_pool,
            } => (0.0, *from_allele_pool, &parents[..]),
            Origin::Placed => (1.0, 0, &[][..]),
            Origin::BeyondRecordedHistory => (2.0, 0, &[][..]),
            Origin::Unexplored => (3.0, 0, &[][..]),
        };
        packed.extend([
            organism.x as f64,
            organism.y as f64,
            organism.birth_generation.map_or(-1.0, |born| born as f64),
            organism.depth as f64,
            origin,
            from_allele_pool as f64,
            parents.len() as f64,
        ]);
        for parent in parents {
            packed.extend([parent.organism as f64, parent.allele as f64]);
        }
    }
    packed
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
