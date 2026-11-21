use crate::allele_tree::{AlleleId, AlleleNode, AlleleStatus, AlleleTree, Slot};
use crate::edit::{CellState, EditHistory, Stroke};
use crate::genetics::{
    phenotype, shows, wrap, Allele, AlleleCensus, Census, Genotype, DEAD_PHENOTYPE, MAX_DOMINANCE,
};
use crate::inspection::{InspectedOrganism, Inspection};
use crate::mutation::MutationSettings;
use crate::pattern::Pattern;
use crate::pedigree::{birth_code, Pedigree, PedigreeQuery, RecordedHistory, MAX_PEDIGREE_BUDGET};
use crate::rng::Rng;
use crate::rule::Rule;
use crate::snapshot::{self, invalid, Reader, SnapshotError, Writer};
use std::cell::OnceCell;
use std::fmt;

/// Smallest allowed World width or height.
pub const MIN_SIZE: usize = 16;
/// Largest allowed World width or height.
pub const MAX_SIZE: usize = 1024;
/// Most Founding Alleles a Soup can have.
pub const MAX_FOUNDING_ALLELES: usize = 16;
/// Founding Alleles in a Soup made with [`World::soup`].
pub const DEFAULT_FOUNDING_ALLELES: usize = 4;

/// Reasons a World cannot be created.
#[derive(Debug, Clone, PartialEq)]
pub enum WorldError {
    /// Width or height is outside `MIN_SIZE..=MAX_SIZE`.
    SizeOutOfRange { width: usize, height: usize },
    /// Density is not within `0.0..=1.0`.
    DensityOutOfRange(f64),
    /// A live Cell lies outside the World.
    CellOutOfBounds { x: usize, y: usize },
    /// Founding Allele count is outside `1..=MAX_FOUNDING_ALLELES`.
    FoundingAllelesOutOfRange(usize),
    /// An Allele's hue is outside `0.0..360.0` or its Dominance Level is too high.
    InvalidAllele(Allele),
    /// A Cell refers to an Allele index beyond the World's Alleles.
    UnknownAllele(usize),
    /// A Pattern is wider or taller than the World it is stamped into.
    PatternTooLarge(Pattern),
    /// A Pedigree memory budget, in bytes, is above [`MAX_PEDIGREE_BUDGET`].
    PedigreeBudgetTooLarge(usize),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeOutOfRange { width, height } => write!(
                f,
                "world size {width}x{height} is outside {MIN_SIZE}..={MAX_SIZE} on each axis"
            ),
            Self::DensityOutOfRange(density) => {
                write!(f, "density {density} is outside 0.0..=1.0")
            }
            Self::CellOutOfBounds { x, y } => write!(f, "cell ({x}, {y}) is outside the world"),
            Self::FoundingAllelesOutOfRange(n) => write!(
                f,
                "founding allele count {n} is outside 1..={MAX_FOUNDING_ALLELES}"
            ),
            Self::InvalidAllele(a) => write!(
                f,
                "allele hue {} must be in 0..360 and dominance {} at most {MAX_DOMINANCE}",
                a.hue, a.dominance
            ),
            Self::UnknownAllele(id) => write!(f, "allele {id} is not one of the World's Alleles"),
            Self::PatternTooLarge(pattern) => {
                let (width, height) = pattern.size();
                write!(
                    f,
                    "pattern {} is {width}x{height} cells, larger than the world",
                    pattern.name()
                )
            }
            Self::PedigreeBudgetTooLarge(bytes) => write!(
                f,
                "pedigree memory budget {bytes} bytes is above the maximum of {MAX_PEDIGREE_BUDGET}"
            ),
        }
    }
}

impl std::error::Error for WorldError {}

/// A fixed-size torus of Cells running a Rule, Conway's Life (B3/S23) by default.
///
/// Cells are processed in row-major order. The seeded PRNG is part of the
/// World's state. The edit history, the inspected Organism and Recorded
/// History are not, so Worlds that differ only in what can be undone or
/// redone, in what is being inspected, or in the Pedigrees they keep, are
/// equal.
#[derive(Debug, Clone)]
pub struct World {
    width: usize,
    height: usize,
    cells: Vec<Option<Genotype>>,
    /// The Birth Generation of each live Cell's Organism; meaningless for a
    /// dead Cell. Kept apart from `cells` so the Rule's scan of `cells`
    /// reads no more memory than it did before Birth Generations existed.
    born: Vec<u64>,
    tree: AlleleTree,
    population_history: Vec<usize>,
    generation: u64,
    rule: Rule,
    mutation: MutationSettings,
    rng: Rng,
    edits: EditHistory,
    inspected: Option<InspectedOrganism>,
    history: RecordedHistory,
}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            width,
            height,
            cells,
            born,
            tree,
            population_history,
            generation,
            rule,
            mutation,
            rng,
            edits: _,
            inspected: _,
            history: _,
        } = self;
        *width == other.width
            && *height == other.height
            && *cells == other.cells
            && live_birth_generations(cells, born)
                .eq(live_birth_generations(&other.cells, &other.born))
            && *tree == other.tree
            && *population_history == other.population_history
            && *generation == other.generation
            && *rule == other.rule
            && *mutation == other.mutation
            && *rng == other.rng
    }
}

impl World {
    /// Creates a Soup with [`DEFAULT_FOUNDING_ALLELES`] Founding Alleles.
    pub fn soup(width: usize, height: usize, density: f64, seed: u64) -> Result<Self, WorldError> {
        Self::soup_with_alleles(width, height, density, seed, DEFAULT_FOUNDING_ALLELES)
    }

    /// Creates a Soup: each Cell is live with probability `density`.
    ///
    /// Draw order from the Seed: Cell liveness in row-major order, the hue
    /// offset, one Dominance Level per Founding Allele, then two Allele
    /// choices per live Cell in row-major order.
    pub fn soup_with_alleles(
        width: usize,
        height: usize,
        density: f64,
        seed: u64,
        founding_alleles: usize,
    ) -> Result<Self, WorldError> {
        check_soup(width, height, density, founding_alleles)?;
        let mut rng = Rng::new(seed);
        let live: Vec<bool> = (0..width * height)
            .map(|_| rng.next_f64() < density)
            .collect();
        let offset = rng.next_f64() * 360.0;
        let spacing = 360.0 / founding_alleles as f64;
        let alleles: Vec<Allele> = (0..founding_alleles)
            .map(|i| {
                let hue = offset + i as f64 * spacing;
                Allele {
                    hue: wrap(hue),
                    dominance: (rng.next_f64() * (MAX_DOMINANCE + 1) as f64) as u8,
                }
            })
            .collect();
        let cells = live
            .into_iter()
            .map(|alive| {
                alive.then(|| {
                    let a = pick(&mut rng, founding_alleles);
                    let b = pick(&mut rng, founding_alleles);
                    [Slot::at(a), Slot::at(b)]
                })
            })
            .collect();
        Ok(Self::assemble(width, height, cells, &alleles, rng))
    }

    /// Creates a World with exactly the given `(x, y)` Cells live, each
    /// homozygous for a single default Allele (hue 0, Dominance Level 0).
    pub fn from_cells(
        width: usize,
        height: usize,
        seed: u64,
        live: &[(usize, usize)],
    ) -> Result<Self, WorldError> {
        let default = Allele {
            hue: 0.0,
            dominance: 0,
        };
        let genotyped: Vec<_> = live.iter().map(|&(x, y)| (x, y, 0, 0)).collect();
        Self::from_genotypes(width, height, seed, &[default], &genotyped)
    }

    /// Creates a World with an explicit, non-empty list of Founding Alleles
    /// and exactly the given `(x, y, first, second)` Cells live, where
    /// `first` and `second` index into `alleles`.
    pub fn from_genotypes(
        width: usize,
        height: usize,
        seed: u64,
        alleles: &[Allele],
        live: &[(usize, usize, usize, usize)],
    ) -> Result<Self, WorldError> {
        check_size(width, height)?;
        if alleles.is_empty() || alleles.len() > u32::MAX as usize {
            return Err(WorldError::FoundingAllelesOutOfRange(alleles.len()));
        }
        for &allele in alleles {
            if !allele.is_valid() {
                return Err(WorldError::InvalidAllele(allele));
            }
        }
        let mut cells = vec![None; width * height];
        for &(x, y, a, b) in live {
            if x >= width || y >= height {
                return Err(WorldError::CellOutOfBounds { x, y });
            }
            if let Some(&id) = [a, b].iter().find(|&&id| id >= alleles.len()) {
                return Err(WorldError::UnknownAllele(id));
            }
            cells[y * width + x] = Some([Slot::at(a), Slot::at(b)]);
        }
        Ok(Self::assemble(
            width,
            height,
            cells,
            alleles,
            Rng::new(seed),
        ))
    }

    fn assemble(
        width: usize,
        height: usize,
        cells: Vec<Option<Genotype>>,
        founding: &[Allele],
        rng: Rng,
    ) -> Self {
        let population = cells.iter().flatten().count();
        Self {
            width,
            height,
            born: vec![0; cells.len()],
            cells,
            tree: AlleleTree::founded(founding),
            population_history: vec![population],
            generation: 0,
            rule: Rule::default(),
            mutation: MutationSettings::default(),
            rng,
            edits: EditHistory::default(),
            inspected: None,
            history: RecordedHistory::new(0, true),
        }
    }

    /// Exports the World's full state as a compressed Snapshot in format
    /// [`crate::SNAPSHOT_VERSION`]; [`World::from_snapshot`] resumes the run
    /// exactly.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut writer = Writer::default();
        writer.index(self.width);
        writer.index(self.height);
        writer.u64(self.generation);
        let (birth, survival) = self.rule.masks();
        writer.u16(birth);
        writer.u16(survival);
        writer.f64(self.mutation.rate());
        writer.f64(self.mutation.magnitude());
        writer.u32(Rng::VERSION);
        writer.u64(self.rng.state());
        self.tree.encode(&mut writer);
        writer.index(self.population());
        for (index, cell) in self.cells.iter().enumerate() {
            if let Some([a, b]) = cell {
                writer.index(index);
                writer.index(a.index());
                writer.index(b.index());
                writer.u64(self.born[index]);
            }
        }
        for &population in &self.population_history {
            writer.index(population);
        }
        snapshot::seal(&writer.bytes)
    }

    /// Imports a Snapshot made by [`World::snapshot`]. Fails, without
    /// panicking, on an unsupported format or PRNG version, on truncated or
    /// corrupt data, and on a payload that describes no valid World.
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, SnapshotError> {
        let payload = snapshot::open(bytes)?;
        let mut reader = Reader::new(&payload);
        let (width, height) = (reader.index()?, reader.index()?);
        check_size(width, height).map_err(invalid)?;
        let generation = reader.u64()?;
        let rule = Rule::from_checked_masks(reader.u16()?, reader.u16()?)
            .ok_or_else(|| invalid("the rule names a neighbour count above 8"))?;
        let mutation = MutationSettings::new(reader.f64()?, reader.f64()?).map_err(invalid)?;
        let rng_version = reader.u32()?;
        if rng_version != Rng::VERSION {
            return Err(SnapshotError::UnsupportedRng(rng_version));
        }
        let rng = Rng::new(reader.u64()?);
        let tree = AlleleTree::decode(&mut reader)?;
        let population = reader.index()?;
        reader.expect_items(population as u64, 20)?;
        let mut cells = vec![None; width * height];
        let mut born = vec![0; width * height];
        let mut lowest_allowed = 0;
        for _ in 0..population {
            let index = reader.index()?;
            if index < lowest_allowed || index >= cells.len() {
                return Err(invalid(format!(
                    "live cell index {index} is out of order or outside the world"
                )));
            }
            let genotype = [reader.index()?, reader.index()?];
            if let Some(slot) = genotype.into_iter().find(|&slot| !tree.is_retained(slot)) {
                return Err(invalid(format!(
                    "live cell {index} carries allele slot {slot}, which is not retained"
                )));
            }
            cells[index] = Some(genotype.map(Slot::at));
            born[index] = reader.u64()?;
            if born[index] > generation {
                return Err(invalid(format!(
                    "live cell {index} was born in generation {}, after generation {generation}",
                    born[index]
                )));
            }
            lowest_allowed = index + 1;
        }
        reader.expect_items(generation, 4)?;
        let population_history = (0..=generation)
            .map(|_| reader.index())
            .collect::<Result<Vec<_>, _>>()?;
        if population_history.last() != Some(&cells.iter().flatten().count()) {
            return Err(invalid(
                "the population history does not end with the live cell count",
            ));
        }
        reader.finish()?;
        Ok(Self {
            width,
            height,
            cells,
            born,
            tree,
            population_history,
            generation,
            rule,
            mutation,
            rng,
            edits: EditHistory::default(),
            inspected: None,
            history: RecordedHistory::new(generation, false),
        })
    }

    /// Applies the Rule once to every Cell simultaneously, then prunes the
    /// Allele Tree and clears the edit history.
    ///
    /// A Birth draws its Parent and Allele choices, then any Allele Pool
    /// draws, then the Mutation draws for the first and then the second Allele
    /// passed on. With two or more live Neighbours the choices are the first
    /// Parent, the second (distinct) Parent, the first Parent's Allele and the
    /// second Parent's Allele, and there are no Allele Pool draws. With one
    /// live Neighbour (B1 Rules) the lone Parent's Allele is chosen and is the
    /// first Allele, and one Allele Pool draw gives the second. With none (B0
    /// Rules) two Allele Pool draws give both.
    ///
    /// An Allele Pool draw picks a random live Cell as the World stood at the
    /// start of the Generation, then one of its two Alleles, so Alleles are
    /// weighted by copy count. In an empty World it picks a random Founding
    /// Allele instead.
    ///
    /// Each Mutation draw decides whether the Allele mutates; a mutant then
    /// draws its hue shift and then its Dominance Level.
    pub fn step(&mut self) {
        self.history.end_generation(&self.placed_cells());
        let cells = std::mem::take(&mut self.cells);
        let pool = AllelePool::new(&cells, self.tree.founding().to_vec());
        let mut next = vec![None; cells.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let current = cells[y * self.width + x];
                let neighbours = self.neighbour_indices(x, y);
                let count = neighbours.iter().filter(|&&i| cells[i].is_some()).count();
                if self.rule.next_state(current.is_some(), count) {
                    let index = y * self.width + x;
                    next[index] = Some(match current {
                        Some(genotype) => genotype,
                        None => {
                            let parents: Vec<(usize, Genotype)> = neighbours
                                .iter()
                                .enumerate()
                                .filter_map(|(direction, &i)| cells[i].map(|g| (direction, g)))
                                .collect();
                            self.born[index] = self.generation + 1;
                            let (genotype, code) = self.birth(&parents, &pool);
                            self.history.record_birth(index, code);
                            genotype
                        }
                    });
                }
            }
        }
        self.cells = next;
        self.generation += 1;
        self.population_history.push(self.population());
        self.tree
            .prune(self.cells.iter().flatten().flatten().copied());
        self.edits.clear();
        self.history.keep_within_budget();
        self.follow_inspected(false);
    }

    /// The Genotype of a Birth from the live Neighbours `parents`, each with
    /// its Neighbour direction, and the Birth's Pedigree code.
    fn birth(&mut self, parents: &[(usize, Genotype)], pool: &AllelePool) -> (Genotype, u8) {
        let ([first, second], code) = match parents {
            [] => (
                [pool.draw(&mut self.rng), pool.draw(&mut self.rng)],
                birth_code(&[]),
            ),
            [(direction, only)] => {
                let allele = pick(&mut self.rng, 2);
                (
                    [only[allele], pool.draw(&mut self.rng)],
                    birth_code(&[(*direction, allele)]),
                )
            }
            _ => {
                let first = pick(&mut self.rng, parents.len());
                let mut second = pick(&mut self.rng, parents.len() - 1);
                if second >= first {
                    second += 1;
                }
                let first_allele = pick(&mut self.rng, 2);
                let second_allele = pick(&mut self.rng, 2);
                let ((first_direction, first), (second_direction, second)) =
                    (parents[first], parents[second]);
                (
                    [first[first_allele], second[second_allele]],
                    birth_code(&[
                        (first_direction, first_allele),
                        (second_direction, second_allele),
                    ]),
                )
            }
        };
        ([self.pass_on(first), self.pass_on(second)], code)
    }

    /// The live Cells, in row-major order, whose Organisms were placed in
    /// the current Generation: painted or stamped in it, or, in the World a
    /// run starts as, all of them.
    fn placed_cells(&self) -> Vec<usize> {
        if self.history.starting_world() {
            return (0..self.cells.len())
                .filter(|&index| self.cells[index].is_some())
                .collect();
        }
        let mut edited = self.edits.edited_cells();
        edited.retain(|&index| self.cells[index].is_some());
        edited
    }

    /// The Allele in `slot` as it reaches the child: unchanged, or with
    /// probability of the Mutation rate a new mutant of it.
    fn pass_on(&mut self, slot: Slot) -> Slot {
        if self.rng.next_f64() >= self.mutation.rate() {
            return slot;
        }
        let source = self.tree.allele(slot);
        let shift = (self.rng.next_f64() * 2.0 - 1.0) * self.mutation.magnitude();
        let mutant = Allele {
            hue: wrap(source.hue + shift),
            dominance: pick(&mut self.rng, MAX_DOMINANCE as usize + 1) as u8,
        };
        self.tree.add_mutant(slot, mutant, self.generation + 1)
    }

    /// Makes the Cell at `(x, y)` live with `genotype`, as part of the open
    /// stroke. Its Alleles are brush Alleles: Founding Alleles originating in
    /// the current Generation, reusing any Founding Allele with the same hue
    /// and Dominance Level.
    pub fn paint(&mut self, x: usize, y: usize, genotype: [Allele; 2]) -> Result<(), WorldError> {
        let index = self.index_of(x, y)?;
        let slots = self.brush(genotype)?;
        self.edit_cell(index, Some(slots));
        Ok(())
    }

    /// Makes the Cell at `(x, y)` dead, as part of the open stroke.
    pub fn erase(&mut self, x: usize, y: usize) -> Result<(), WorldError> {
        let index = self.index_of(x, y)?;
        self.edit_cell(index, None);
        Ok(())
    }

    /// Stamps `pattern` in `genotype` as part of the open stroke, with the
    /// Pattern's middle Cell (half its width and height, rounded down) at
    /// `(x, y)`. Every Cell of the Pattern's bounding box takes the
    /// Pattern's state, wrapping across the World's edges. Its Alleles are
    /// brush Alleles, as for [`World::paint`]. A Pattern wider or taller than
    /// the World is refused.
    pub fn stamp(
        &mut self,
        pattern: Pattern,
        x: usize,
        y: usize,
        genotype: [Allele; 2],
    ) -> Result<(), WorldError> {
        self.index_of(x, y)?;
        let (width, height) = pattern.size();
        if width > self.width || height > self.height {
            return Err(WorldError::PatternTooLarge(pattern));
        }
        let slots = self.brush(genotype)?;
        let left = x + self.width - width / 2;
        let top = y + self.height - height / 2;
        for (dx, dy, live) in pattern.cells() {
            let index = (top + dy) % self.height * self.width + (left + dx) % self.width;
            self.edit_cell(index, live.then_some(slots));
        }
        Ok(())
    }

    /// Closes the open stroke, so the next edit starts a new one. Undo and
    /// redo act on whole strokes.
    pub fn end_stroke(&mut self) {
        self.edits.end_stroke();
    }

    /// Reverts the latest stroke not yet undone in this Generation, closing
    /// the open stroke first. Returns whether there was one.
    pub fn undo(&mut self) -> bool {
        let Some(mut stroke) = self.edits.take_undo() else {
            return false;
        };
        self.swap_in(&mut stroke);
        self.edits.push_redo(stroke);
        true
    }

    /// Reapplies the latest undone stroke. Returns whether there was one.
    pub fn redo(&mut self) -> bool {
        let Some(mut stroke) = self.edits.take_redo() else {
            return false;
        };
        self.swap_in(&mut stroke);
        self.edits.push_undo(stroke);
        true
    }

    /// Whether [`World::undo`] would revert a stroke. Advancing a Generation
    /// clears the edit history.
    pub fn can_undo(&self) -> bool {
        self.edits.can_undo()
    }

    /// Whether [`World::redo`] would reapply a stroke.
    pub fn can_redo(&self) -> bool {
        self.edits.can_redo()
    }

    fn swap_in(&mut self, stroke: &mut Stroke) {
        for (&index, cell) in &mut stroke.cells {
            *cell = self.replace_cell(index, *cell);
        }
        if let Some(tree) = &mut stroke.tree {
            std::mem::swap(tree, &mut self.tree);
        }
        self.follow_inspected(true);
    }

    /// Sets the Cell at `index` as part of the open stroke; a new live
    /// Genotype is a new Organism whose Birth Generation is the current one.
    fn edit_cell(&mut self, index: usize, cell: Option<Genotype>) {
        if self.cells[index] != cell {
            let state = CellState {
                genotype: cell,
                born: self.generation,
            };
            let before = self.replace_cell(index, state);
            self.edits.record_cell(index, before);
            if self.inspected.as_ref().is_some_and(|o| o.index == index) {
                self.follow_inspected(false);
            }
        }
    }

    /// Sets the Cell at `index`, keeping the current Generation's population
    /// up to date, and returns what it held.
    fn replace_cell(&mut self, index: usize, state: CellState) -> CellState {
        let old = CellState {
            genotype: std::mem::replace(&mut self.cells[index], state.genotype),
            born: std::mem::replace(&mut self.born[index], state.born),
        };
        let population = self
            .population_history
            .last_mut()
            .expect("the history holds the current Generation");
        match (old.genotype.is_some(), state.genotype.is_some()) {
            (false, true) => *population += 1,
            (true, false) => *population -= 1,
            _ => {}
        }
        old
    }

    /// The slots of the brush Alleles in `genotype`.
    fn brush(&mut self, genotype: [Allele; 2]) -> Result<Genotype, WorldError> {
        if let Some(&invalid) = genotype.iter().find(|allele| !allele.is_valid()) {
            return Err(WorldError::InvalidAllele(invalid));
        }
        Ok(genotype.map(|allele| {
            self.tree.find_founding(allele).unwrap_or_else(|| {
                self.edits.record_tree(&self.tree);
                self.tree.add_founding(allele, self.generation)
            })
        }))
    }

    fn index_of(&self, x: usize, y: usize) -> Result<usize, WorldError> {
        if x < self.width && y < self.height {
            Ok(y * self.width + x)
        } else {
            Err(WorldError::CellOutOfBounds { x, y })
        }
    }

    /// Applies the Rule `generations` times.
    pub fn run(&mut self, generations: u64) {
        for _ in 0..generations {
            self.step();
        }
    }

    /// The Rule applied by [`World::step`].
    pub fn rule(&self) -> Rule {
        self.rule
    }

    /// Changes the Rule; Cells and Generation are kept and later Generations
    /// follow the new Rule.
    pub fn set_rule(&mut self, rule: Rule) {
        self.rule = rule;
    }

    /// How Alleles mutate as they are passed on at a Birth.
    pub fn mutation(&self) -> MutationSettings {
        self.mutation
    }

    /// Changes the Mutation settings; later Births use the new settings.
    pub fn set_mutation(&mut self, mutation: MutationSettings) {
        self.mutation = mutation;
    }

    /// Number of Generations applied since creation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Number of live Cells.
    pub fn population(&self) -> usize {
        self.cells.iter().flatten().count()
    }

    /// World width in Cells.
    pub fn width(&self) -> usize {
        self.width
    }

    /// World height in Cells.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Live `(x, y)` Cells in row-major order.
    pub fn live_cells(&self) -> Vec<(usize, usize)> {
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.is_some())
            .map(|(i, _)| (i % self.width, i / self.width))
            .collect()
    }

    /// The World's PRNG, part of its state.
    pub fn rng(&self) -> &Rng {
        &self.rng
    }

    /// Cell indices of the Neighbours of `(x, y)` in a fixed scan order.
    fn neighbour_indices(&self, x: usize, y: usize) -> [usize; 8] {
        let (w, h) = (self.width, self.height);
        let (left, right) = ((x + w - 1) % w, (x + 1) % w);
        let (up, down) = ((y + h - 1) % h * w, (y + 1) % h * w);
        let row = y * w;
        [
            up + left,
            up + x,
            up + right,
            row + left,
            row + right,
            down + left,
            down + x,
            down + right,
        ]
    }

    /// The Allele Tree: every retained Allele, each listed before its
    /// descendants.
    pub fn allele_tree(&self) -> Vec<AlleleNode> {
        let tally = self.tally();
        self.tree.nodes(|slot| tally[slot.index()].status())
    }

    /// The chain from `id` back to its Founding Allele, starting with `id`;
    /// `None` if the World retains no Allele `id`.
    pub fn lineage(&self, id: AlleleId) -> Option<Vec<AlleleNode>> {
        let tally = self.tally();
        self.tree.lineage(id, |slot| tally[slot.index()].status())
    }

    /// Starts following the Organism at `(x, y)` for its inspector and
    /// returns its inspection; `None`, following nothing, if the Cell is dead.
    pub fn inspect(&mut self, x: usize, y: usize) -> Result<Option<Inspection>, WorldError> {
        let index = self.index_of(x, y)?;
        let born = self.born[index];
        let placed = if born == self.generation {
            self.placed_cells().binary_search(&index).is_ok()
        } else {
            self.history.was_placed(born, index)
        };
        self.inspected = self.cells[index].map(|genotype| InspectedOrganism {
            index,
            birth_generation: born,
            placed,
            lineages: genotype.map(|slot| {
                self.lineage(self.tree.id(slot))
                    .expect("a live Cell's Alleles are retained")
            }),
            death_generation: None,
        });
        Ok(self.inspection())
    }

    /// Stops following an Organism for its inspector.
    pub fn stop_inspecting(&mut self) {
        self.inspected = None;
    }

    /// Notes the Generation the inspected Organism died in once its Cell no
    /// longer holds it. With `revive`, after undo or redo, an Organism its
    /// Cell holds again lives again; an edit instead always makes a new
    /// Organism, even one painted with the same Genotype in the same
    /// Generation.
    fn follow_inspected(&mut self, revive: bool) {
        let Some(organism) = &mut self.inspected else {
            return;
        };
        let lives = self.cells[organism.index].is_some_and(|genotype| {
            self.born[organism.index] == organism.birth_generation
                && genotype.map(|slot| self.tree.allele(slot)) == organism.alleles()
        });
        if !lives {
            organism.death_generation.get_or_insert(self.generation);
        } else if revive {
            organism.death_generation = None;
        }
    }

    /// The inspection of the Organism being followed, if any.
    pub fn inspection(&self) -> Option<Inspection> {
        let organism = self.inspected.as_ref()?;
        let tally = self.tally();
        Some(organism.inspection(self.width, |id| {
            self.tree.slot(id).map(|slot| tally[slot.index()].status())
        }))
    }

    /// The Pedigree of the Organism being followed, if any, to `depth`
    /// Births back: depth 1 holds its Parents, 2 their Parents.
    pub fn pedigree(&self, depth: usize) -> Option<Pedigree> {
        let organism = self.inspected.as_ref()?;
        let live_since = |index: usize| self.cells[index].map(|_| self.born[index]);
        let query = PedigreeQuery {
            history: &self.history,
            width: self.width,
            height: self.height,
            live_since: &live_since,
        };
        Some(query.pedigree(
            organism.index,
            organism.birth_generation,
            organism.placed,
            depth,
        ))
    }

    /// Most bytes Recorded History may take; [`MAX_PEDIGREE_BUDGET`] unless
    /// changed. Not part of a Snapshot.
    pub fn pedigree_budget(&self) -> usize {
        self.history.budget()
    }

    /// Changes the memory budget of Recorded History, at once dropping the
    /// oldest records that no longer fit; a budget above
    /// [`MAX_PEDIGREE_BUDGET`] is refused.
    pub fn set_pedigree_budget(&mut self, bytes: usize) -> Result<(), WorldError> {
        if bytes > MAX_PEDIGREE_BUDGET {
            return Err(WorldError::PedigreeBudgetTooLarge(bytes));
        }
        self.history.set_budget(bytes);
        Ok(())
    }

    /// Bytes Recorded History takes, at most [`World::pedigree_budget`].
    pub fn pedigree_bytes(&self) -> usize {
        self.history.bytes()
    }

    /// One Phenotype hue per Cell in row-major order; dead Cells are
    /// [`DEAD_PHENOTYPE`].
    pub fn phenotypes(&self) -> Vec<f64> {
        self.cells
            .iter()
            .map(|cell| match cell {
                Some([a, b]) => phenotype(self.tree.allele(*a), self.tree.allele(*b)),
                None => DEAD_PHENOTYPE,
            })
            .collect()
    }

    /// The genetic state of the World.
    pub fn census(&self) -> Census {
        let alleles: Vec<AlleleCensus> = self
            .tally()
            .into_iter()
            .enumerate()
            .filter(|(_, tally)| tally.copies > 0)
            .map(|(index, tally)| {
                let slot = Slot::at(index);
                AlleleCensus {
                    id: self.tree.id(slot),
                    allele: self.tree.allele(slot),
                    copies: tally.copies,
                    shown: tally.shown,
                    hidden: tally.hidden,
                }
            })
            .collect();
        Census {
            population: self.population(),
            generation: self.generation,
            fully_hidden: alleles
                .iter()
                .filter(|a| a.shown == 0)
                .map(|a| a.allele)
                .collect(),
            alleles,
            population_history: self.population_history.clone(),
        }
    }

    /// Copies, showing Cells and hiding Cells per Allele slot index.
    fn tally(&self) -> Vec<Tally> {
        let mut tally = vec![Tally::default(); self.tree.slot_bound()];
        for &[a, b] in self.cells.iter().flatten() {
            let (shows_a, shows_b) = shows(self.tree.allele(a), self.tree.allele(b));
            tally[a.index()].copies += 1;
            tally[b.index()].copies += 1;
            if a == b {
                tally[a.index()].shown += 1;
                continue;
            }
            for (id, shown) in [(a, shows_a), (b, shows_b)] {
                if shown {
                    tally[id.index()].shown += 1;
                } else {
                    tally[id.index()].hidden += 1;
                }
            }
        }
        tally
    }
}

/// The Allele Pool: the live Cells as the World stood at the start of a
/// Generation, gathered on the first draw.
struct AllelePool<'a> {
    cells: &'a [Option<Genotype>],
    founding: Vec<Slot>,
    live: OnceCell<Vec<Genotype>>,
}

impl<'a> AllelePool<'a> {
    /// The pool of `cells`, falling back to the `founding` slots, the
    /// Founding Alleles, when no Cell is live.
    fn new(cells: &'a [Option<Genotype>], founding: Vec<Slot>) -> Self {
        Self {
            cells,
            founding,
            live: OnceCell::new(),
        }
    }

    /// One Allele copy at random: a random live Cell, then one of its two
    /// Alleles, so each Allele is weighted by its copy count. With no live
    /// Cells, a random Founding Allele instead.
    fn draw(&self, rng: &mut Rng) -> Slot {
        let live = self
            .live
            .get_or_init(|| self.cells.iter().flatten().copied().collect());
        if live.is_empty() {
            return self.founding[pick(rng, self.founding.len())];
        }
        let genotype = live[pick(rng, live.len())];
        genotype[pick(rng, 2)]
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    copies: usize,
    shown: usize,
    hidden: usize,
}

impl Tally {
    fn status(&self) -> AlleleStatus {
        if self.copies == 0 {
            AlleleStatus::Extinct
        } else if self.shown > 0 {
            AlleleStatus::Living
        } else {
            AlleleStatus::Hidden
        }
    }
}

/// The Birth Generation of each live Cell, in row-major order.
fn live_birth_generations<'a>(
    cells: &'a [Option<Genotype>],
    born: &'a [u64],
) -> impl Iterator<Item = u64> + 'a {
    cells
        .iter()
        .zip(born)
        .filter(|(cell, _)| cell.is_some())
        .map(|(_, &born)| born)
}

/// Uniform index in `0..n` using one draw.
fn pick(rng: &mut Rng, n: usize) -> usize {
    ((rng.next_f64() * n as f64) as usize).min(n - 1)
}

/// Fails unless [`World::soup_with_alleles`] accepts these settings.
pub(crate) fn check_soup(
    width: usize,
    height: usize,
    density: f64,
    founding_alleles: usize,
) -> Result<(), WorldError> {
    check_size(width, height)?;
    if !(0.0..=1.0).contains(&density) {
        return Err(WorldError::DensityOutOfRange(density));
    }
    if !(1..=MAX_FOUNDING_ALLELES).contains(&founding_alleles) {
        return Err(WorldError::FoundingAllelesOutOfRange(founding_alleles));
    }
    Ok(())
}

fn check_size(width: usize, height: usize) -> Result<(), WorldError> {
    let range = MIN_SIZE..=MAX_SIZE;
    if range.contains(&width) && range.contains(&height) {
        Ok(())
    } else {
        Err(WorldError::SizeOutOfRange { width, height })
    }
}
