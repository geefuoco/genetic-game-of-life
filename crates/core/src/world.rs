use crate::allele_tree::{AlleleId, AlleleNode, AlleleStatus, AlleleTree, Slot};
use crate::genetics::{
    phenotype, shows, wrap, Allele, AlleleCensus, Census, Genotype, DEAD_PHENOTYPE, MAX_DOMINANCE,
};
use crate::mutation::MutationSettings;
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
        }
    }
}

impl std::error::Error for WorldError {}

/// A fixed-size torus of Cells running a Rule, Conway's Life (B3/S23) by default.
///
/// Cells are processed in row-major order. The seeded PRNG is part of the
/// World's state.
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    width: usize,
    height: usize,
    cells: Vec<Option<Genotype>>,
    tree: AlleleTree,
    population_history: Vec<usize>,
    generation: u64,
    rule: Rule,
    mutation: MutationSettings,
    rng: Rng,
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
            cells,
            tree: AlleleTree::founded(founding),
            population_history: vec![population],
            generation: 0,
            rule: Rule::default(),
            mutation: MutationSettings::default(),
            rng,
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
        reader.expect_items(population as u64, 12)?;
        let mut cells = vec![None; width * height];
        let mut first_free = 0;
        for _ in 0..population {
            let index = reader.index()?;
            if index < first_free || index >= cells.len() {
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
            first_free = index + 1;
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
            tree,
            population_history,
            generation,
            rule,
            mutation,
            rng,
        })
    }

    /// Applies the Rule once to every Cell simultaneously, then prunes the
    /// Allele Tree.
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
        let cells = std::mem::take(&mut self.cells);
        let pool = AllelePool::new(&cells, self.tree.founding_count());
        let mut next = vec![None; cells.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let current = cells[y * self.width + x];
                let neighbours = self.neighbour_indices(x, y);
                let count = neighbours.iter().filter(|&&i| cells[i].is_some()).count();
                if self.rule.next_state(current.is_some(), count) {
                    next[y * self.width + x] = Some(match current {
                        Some(genotype) => genotype,
                        None => {
                            let parents: Vec<Genotype> =
                                neighbours.iter().filter_map(|&i| cells[i]).collect();
                            self.birth(&parents, &pool)
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
    }

    fn birth(&mut self, parents: &[Genotype], pool: &AllelePool) -> Genotype {
        let [first, second] = match parents {
            [] => [pool.draw(&mut self.rng), pool.draw(&mut self.rng)],
            [only] => {
                let from_parent = only[pick(&mut self.rng, 2)];
                [from_parent, pool.draw(&mut self.rng)]
            }
            _ => {
                let first = pick(&mut self.rng, parents.len());
                let mut second = pick(&mut self.rng, parents.len() - 1);
                if second >= first {
                    second += 1;
                }
                let from_first = parents[first][pick(&mut self.rng, 2)];
                let from_second = parents[second][pick(&mut self.rng, 2)];
                [from_first, from_second]
            }
        };
        [self.pass_on(first), self.pass_on(second)]
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
    founding_count: usize,
    live: OnceCell<Vec<Genotype>>,
}

impl<'a> AllelePool<'a> {
    /// The pool of `cells`, falling back to the first `founding_count`
    /// slots, the Founding Alleles, when no Cell is live.
    fn new(cells: &'a [Option<Genotype>], founding_count: usize) -> Self {
        Self {
            cells,
            founding_count,
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
            return Slot::at(pick(rng, self.founding_count));
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
