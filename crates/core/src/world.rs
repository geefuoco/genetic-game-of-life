use crate::allele_tree::{AlleleId, AlleleNode, AlleleStatus, AlleleTree};
use crate::genetics::{
    phenotype, shows, wrap, Allele, AlleleCensus, Census, Genotype, DEAD_PHENOTYPE, MAX_DOMINANCE,
};
use crate::mutation::MutationSettings;
use crate::rng::Rng;
use crate::rule::Rule;
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
        check_size(width, height)?;
        if !(0.0..=1.0).contains(&density) {
            return Err(WorldError::DensityOutOfRange(density));
        }
        if !(1..=MAX_FOUNDING_ALLELES).contains(&founding_alleles) {
            return Err(WorldError::FoundingAllelesOutOfRange(founding_alleles));
        }
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
                    [AlleleId::from_index(a), AlleleId::from_index(b)]
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

    /// Creates a World with an explicit list of Alleles and exactly the given
    /// `(x, y, first, second)` Cells live, where `first` and `second` index
    /// into `alleles`.
    pub fn from_genotypes(
        width: usize,
        height: usize,
        seed: u64,
        alleles: &[Allele],
        live: &[(usize, usize, usize, usize)],
    ) -> Result<Self, WorldError> {
        check_size(width, height)?;
        if alleles.len() > u32::MAX as usize {
            return Err(WorldError::FoundingAllelesOutOfRange(alleles.len()));
        }
        for &allele in alleles {
            if !(0.0..360.0).contains(&allele.hue) || allele.dominance > MAX_DOMINANCE {
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
            cells[y * width + x] = Some([AlleleId::from_index(a), AlleleId::from_index(b)]);
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

    /// Applies the Rule once to every Cell simultaneously, then prunes the
    /// Allele Tree.
    ///
    /// A Birth with two or more live Neighbours draws, in order: the first
    /// Parent, the second (distinct) Parent, the first Parent's Allele, the
    /// second Parent's Allele, then the Mutation draws for the first and then
    /// the second Allele passed on. Births with fewer Parents (B0 and B1
    /// Rules) are unspecified: they draw nothing and copy the lone Parent, or
    /// take the first Founding Allele twice when there is none.
    ///
    /// Each Mutation draw decides whether the Allele mutates; a mutant then
    /// draws its hue shift and then its Dominance Level.
    pub fn step(&mut self) {
        let mut next = vec![None; self.cells.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let current = self.cells[y * self.width + x];
                if self
                    .rule
                    .next_state(current.is_some(), self.live_neighbours(x, y))
                {
                    next[y * self.width + x] = Some(match current {
                        Some(genotype) => genotype,
                        None => {
                            let (parents, count) = self.neighbour_genotypes(x, y);
                            self.birth(&parents[..count])
                        }
                    });
                }
            }
        }
        self.cells = next;
        self.generation += 1;
        self.population_history.push(self.population());
        let mut carried = vec![false; self.tree.capacity()];
        for &[a, b] in self.cells.iter().flatten() {
            carried[a.index()] = true;
            carried[b.index()] = true;
        }
        self.tree.prune(&carried);
    }

    fn birth(&mut self, parents: &[Genotype]) -> Genotype {
        match parents {
            [] => [AlleleId::from_index(0); 2],
            [only] => *only,
            _ => {
                let first = pick(&mut self.rng, parents.len());
                let mut second = pick(&mut self.rng, parents.len() - 1);
                if second >= first {
                    second += 1;
                }
                let from_first = parents[first][pick(&mut self.rng, 2)];
                let from_second = parents[second][pick(&mut self.rng, 2)];
                [self.pass_on(from_first), self.pass_on(from_second)]
            }
        }
    }

    /// `id` as it reaches the child: unchanged, or with probability of the
    /// Mutation rate a new mutant of it.
    fn pass_on(&mut self, id: AlleleId) -> AlleleId {
        if self.rng.next_f64() >= self.mutation.rate() {
            return id;
        }
        let source = self.tree.allele(id);
        let shift = (self.rng.next_f64() * 2.0 - 1.0) * self.mutation.magnitude();
        let mutant = Allele {
            hue: wrap(source.hue + shift),
            dominance: pick(&mut self.rng, MAX_DOMINANCE as usize + 1) as u8,
        };
        self.tree.add_mutant(id, mutant, self.generation + 1)
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

    /// Number of live Neighbours of `(x, y)`.
    fn live_neighbours(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        for dy in [self.height - 1, 0, 1] {
            for dx in [self.width - 1, 0, 1] {
                let nx = (x + dx) % self.width;
                let ny = (y + dy) % self.height;
                count += usize::from(self.cells[ny * self.width + nx].is_some());
            }
        }
        count - usize::from(self.cells[y * self.width + x].is_some())
    }

    /// Genotypes of the live Neighbours of `(x, y)` in a fixed scan order.
    fn neighbour_genotypes(&self, x: usize, y: usize) -> ([Genotype; 8], usize) {
        let mut found = [[AlleleId::from_index(0); 2]; 8];
        let mut count = 0;
        for dy in [self.height - 1, 0, 1] {
            for dx in [self.width - 1, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (x + dx) % self.width;
                let ny = (y + dy) % self.height;
                if let Some(genotype) = self.cells[ny * self.width + nx] {
                    found[count] = genotype;
                    count += 1;
                }
            }
        }
        (found, count)
    }

    /// The Allele Tree: every retained Allele, each listed before its
    /// descendants.
    pub fn allele_tree(&self) -> Vec<AlleleNode> {
        let tally = self.tally();
        self.tree.nodes(|i| tally[i].status())
    }

    /// The chain from `id` back to its Founding Allele, starting with `id`;
    /// `None` if the World retains no Allele `id`.
    pub fn lineage(&self, id: AlleleId) -> Option<Vec<AlleleNode>> {
        let tally = self.tally();
        self.tree.lineage(id, |i| tally[i].status())
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
                let id = AlleleId::from_index(index);
                AlleleCensus {
                    id,
                    allele: self.tree.allele(id),
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

    /// Copies, showing Cells and hiding Cells per Allele index.
    fn tally(&self) -> Vec<Tally> {
        let mut tally = vec![Tally::default(); self.tree.capacity()];
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

fn check_size(width: usize, height: usize) -> Result<(), WorldError> {
    let range = MIN_SIZE..=MAX_SIZE;
    if range.contains(&width) && range.contains(&height) {
        Ok(())
    } else {
        Err(WorldError::SizeOutOfRange { width, height })
    }
}
