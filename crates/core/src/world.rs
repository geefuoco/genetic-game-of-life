use crate::rng::Rng;
use crate::rule::Rule;
use std::fmt;

/// Smallest allowed World width or height.
pub const MIN_SIZE: usize = 16;
/// Largest allowed World width or height.
pub const MAX_SIZE: usize = 1024;

/// Reasons a World cannot be created.
#[derive(Debug, Clone, PartialEq)]
pub enum WorldError {
    /// Width or height is outside `MIN_SIZE..=MAX_SIZE`.
    SizeOutOfRange { width: usize, height: usize },
    /// Density is not within `0.0..=1.0`.
    DensityOutOfRange(f64),
    /// A live Cell lies outside the World.
    CellOutOfBounds { x: usize, y: usize },
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
    cells: Vec<bool>,
    generation: u64,
    rule: Rule,
    rng: Rng,
}

impl World {
    /// Creates a Soup: each Cell is live with probability `density`.
    pub fn soup(width: usize, height: usize, density: f64, seed: u64) -> Result<Self, WorldError> {
        check_size(width, height)?;
        if !(0.0..=1.0).contains(&density) {
            return Err(WorldError::DensityOutOfRange(density));
        }
        let mut rng = Rng::new(seed);
        let cells = (0..width * height)
            .map(|_| rng.next_f64() < density)
            .collect();
        Ok(Self {
            width,
            height,
            cells,
            generation: 0,
            rule: Rule::default(),
            rng,
        })
    }

    /// Creates a World with exactly the given `(x, y)` Cells live.
    pub fn from_cells(
        width: usize,
        height: usize,
        seed: u64,
        live: &[(usize, usize)],
    ) -> Result<Self, WorldError> {
        check_size(width, height)?;
        let mut cells = vec![false; width * height];
        for &(x, y) in live {
            if x >= width || y >= height {
                return Err(WorldError::CellOutOfBounds { x, y });
            }
            cells[y * width + x] = true;
        }
        Ok(Self {
            width,
            height,
            cells,
            generation: 0,
            rule: Rule::default(),
            rng: Rng::new(seed),
        })
    }

    /// Applies the Rule once to every Cell simultaneously.
    pub fn step(&mut self) {
        let mut next = vec![false; self.cells.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let neighbours = self.live_neighbours(x, y);
                let alive = self.cells[y * self.width + x];
                next[y * self.width + x] = self.rule.next_state(alive, neighbours);
            }
        }
        self.cells = next;
        self.generation += 1;
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

    /// Number of Generations applied since creation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Number of live Cells.
    pub fn population(&self) -> usize {
        self.cells.iter().filter(|&&alive| alive).count()
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
            .filter(|(_, &alive)| alive)
            .map(|(i, _)| (i % self.width, i / self.width))
            .collect()
    }

    /// The World's PRNG, part of its state.
    pub fn rng(&self) -> &Rng {
        &self.rng
    }

    fn live_neighbours(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        for dy in [self.height - 1, 0, 1] {
            for dx in [self.width - 1, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (x + dx) % self.width;
                let ny = (y + dy) % self.height;
                count += usize::from(self.cells[ny * self.width + nx]);
            }
        }
        count
    }
}

fn check_size(width: usize, height: usize) -> Result<(), WorldError> {
    let range = MIN_SIZE..=MAX_SIZE;
    if range.contains(&width) && range.contains(&height) {
        Ok(())
    } else {
        Err(WorldError::SizeOutOfRange { width, height })
    }
}
