//! Pedigrees, kept for Recorded History (ADR-0003).
//!
//! Each Generation keeps one record per Organism it created: first its
//! Births, from the Generation's step, then the Organisms painted or stamped
//! in it, or live in the World it started as, that were still live when the
//! next Generation was applied. Each part is in row-major order. A record is
//! a `u32` holding the Organism's row-major Cell index in its low 24 bits and,
//! for a Birth, its one-byte code in the high 8 bits.
//!
//! A Birth's code holds its Parent configuration in the low 6 bits and, in
//! bits 6 and 7, which Allele each Parent passed on. The configurations are
//! the 28 pairs of distinct Neighbour directions, lower direction first, then
//! the 8 single-Parent directions, then one for no Parents, where both
//! Alleles came from the Allele Pool. A lone Parent's Allele is in bit 6 and
//! the Allele Pool gave the other. A pair's lower direction Parent's Allele
//! is in bit 6 and the other Parent's in bit 7; which of the child's Alleles
//! came from which Parent is not kept.

use std::collections::{HashMap, VecDeque};

/// The default and largest memory budget for Recorded History, 200 MB.
pub const MAX_PEDIGREE_BUDGET: usize = 200_000_000;

/// Neighbour directions, in the order of the World's Neighbour scan.
const DIRECTIONS: usize = 8;
/// Distinct pairs of Neighbour directions.
const PAIRS: u8 = 28;
/// The configuration of a Birth with no Parents.
const NO_PARENTS: u8 = PAIRS + DIRECTIONS as u8;
/// Bits of a record holding the Cell index.
const INDEX_BITS: u32 = 24;
const INDEX_MASK: u32 = (1 << INDEX_BITS) - 1;

/// An Organism's Pedigree: its Parents, their Parents and so on, as far as a
/// requested depth and Recorded History reach.
#[derive(Debug, Clone, PartialEq)]
pub struct Pedigree {
    /// The queried Organism first, then its ancestors in order of depth. An
    /// ancestor shared by several descendants appears once.
    pub organisms: Vec<PedigreeOrganism>,
}

/// One Organism in a [`Pedigree`].
#[derive(Debug, Clone, PartialEq)]
pub struct PedigreeOrganism {
    /// Column of the Organism's Cell.
    pub x: usize,
    /// Row of the Organism's Cell.
    pub y: usize,
    /// The Organism's Birth Generation; `None` when it is beyond Recorded
    /// History and no longer live.
    pub birth_generation: Option<u64>,
    /// The fewest Births between it and the queried Organism: 0 for the
    /// queried Organism, 1 for its Parents.
    pub depth: usize,
    pub origin: Origin,
}

/// How an Organism in a [`Pedigree`] came to be.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    /// Born from `parents`, indices into [`Pedigree::organisms`], with its
    /// other Alleles, `from_allele_pool` of them, drawn from the Allele Pool.
    Born {
        parents: Vec<PedigreeParent>,
        from_allele_pool: usize,
    },
    /// Painted, stamped or live in the World it started as: it has no
    /// Parents.
    Placed,
    /// Created before Recorded History, so how is no longer known.
    BeyondRecordedHistory,
    /// Born, at the requested depth, so its Parents were not queried.
    Unexplored,
}

/// A Parent in a [`Pedigree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PedigreeParent {
    /// The Parent's index in [`Pedigree::organisms`].
    pub organism: usize,
    /// Which of the Parent's two Alleles, 0 or 1, it passed on.
    pub allele: usize,
}

/// The code of a Birth from `parents`, each its Neighbour direction and the
/// Allele, 0 or 1, it passed on; any Alleles past the Parents' came from the
/// Allele Pool.
pub(crate) fn birth_code(parents: &[(usize, usize)]) -> u8 {
    match *parents {
        [] => NO_PARENTS,
        [(direction, allele)] => (PAIRS + direction as u8) | ((allele as u8) << 6),
        [(a, allele_a), (b, allele_b)] => {
            let ((low, allele_low), (high, allele_high)) = if a < b {
                ((a, allele_a), (b, allele_b))
            } else {
                ((b, allele_b), (a, allele_a))
            };
            pair_index(low, high) | (allele_low as u8) << 6 | (allele_high as u8) << 7
        }
        _ => unreachable!("a Birth has at most two Parents"),
    }
}

/// The index of the pair of directions `low < high` among the 28 pairs.
fn pair_index(low: usize, high: usize) -> u8 {
    let before: usize = (0..low).map(|d| DIRECTIONS - 1 - d).sum();
    (before + high - low - 1) as u8
}

/// The Parents a Birth `code` records, as Neighbour direction and Allele.
fn birth_parents(code: u8) -> Vec<(usize, usize)> {
    let configuration = code & 0x3f;
    let first = usize::from(code >> 6 & 1);
    let second = usize::from(code >> 7 & 1);
    if configuration < PAIRS {
        let mut index = configuration as usize;
        for low in 0..DIRECTIONS {
            let span = DIRECTIONS - 1 - low;
            if index < span {
                return vec![(low, first), (low + 1 + index, second)];
            }
            index -= span;
        }
        unreachable!("pair index below {PAIRS}")
    } else if configuration < NO_PARENTS {
        vec![(usize::from(configuration - PAIRS), first)]
    } else {
        Vec::new()
    }
}

/// What a Generation's records say an Organism was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Created {
    Born(u8),
    Placed,
}

/// The records of one Generation: its Births, then from `placed_from` on
/// its placed Organisms.
#[derive(Debug, Clone)]
struct Block {
    records: Box<[u32]>,
    placed_from: u32,
}

impl Block {
    fn births(&self) -> &[u32] {
        &self.records[..self.placed_from as usize]
    }

    fn placed(&self) -> &[u32] {
        &self.records[self.placed_from as usize..]
    }
}

/// The record for Cell `index` in `records`, sorted by Cell index.
fn find(records: &[u32], index: usize) -> Option<u32> {
    records
        .binary_search_by_key(&(index as u32), |record| record & INDEX_MASK)
        .ok()
        .map(|at| records[at])
}

/// The Pedigree records of Recorded History, kept within a memory budget by
/// dropping the oldest Generations' records first.
#[derive(Debug, Clone)]
pub(crate) struct RecordedHistory {
    /// Most bytes the records and their containers may take.
    budget: usize,
    /// Bytes taken by the records in `blocks`.
    block_bytes: usize,
    /// The Generation the World is in.
    current: u64,
    /// The records of the Generations just before `current`, oldest first.
    blocks: VecDeque<Block>,
    /// The Births of `current`, as they are recorded.
    births: Vec<u32>,
    /// Whether `current` is the Generation of the World a run starts as, in
    /// which every live Organism was placed rather than Born.
    starting_world: bool,
}

impl RecordedHistory {
    /// Recorded History of a World started in `generation`; with
    /// `starting_world` its live Organisms were placed in it, as in the World
    /// a run starts as, otherwise their Pedigrees are beyond Recorded History.
    pub(crate) fn new(generation: u64, starting_world: bool) -> Self {
        Self {
            budget: MAX_PEDIGREE_BUDGET,
            block_bytes: 0,
            current: generation,
            blocks: VecDeque::new(),
            births: Vec::new(),
            starting_world,
        }
    }

    /// Whether the current Generation is that of the World a run starts as,
    /// in which every live Organism was placed.
    pub(crate) fn starting_world(&self) -> bool {
        self.starting_world
    }

    /// Records a Birth into the Generation after the current one, at Cell
    /// `index`, after every Birth at a lower index.
    pub(crate) fn record_birth(&mut self, index: usize, code: u8) {
        self.births
            .push(index as u32 | u32::from(code) << INDEX_BITS);
    }

    /// Ends the current Generation, whose placed Organisms still live are at
    /// the Cells `placed`, in row-major order.
    pub(crate) fn end_generation(&mut self, placed: &[usize]) {
        let placed_from = self.births.len() as u32;
        let mut records = Vec::with_capacity(self.births.len() + placed.len());
        records.extend_from_slice(&self.births);
        records.extend(placed.iter().map(|&index| index as u32));
        self.births.clear();
        self.make_room_for_a_block();
        self.block_bytes += records.len() * size_of::<u32>();
        self.blocks.push_back(Block {
            records: records.into_boxed_slice(),
            placed_from,
        });
        self.current += 1;
        self.starting_world = false;
    }

    pub(crate) fn budget(&self) -> usize {
        self.budget
    }

    /// Changes the budget, dropping records at once to keep within it.
    pub(crate) fn set_budget(&mut self, budget: usize) {
        self.budget = budget;
        self.keep_within_budget();
    }

    /// Bytes the records and their containers take.
    pub(crate) fn bytes(&self) -> usize {
        self.block_bytes
            + self.blocks.capacity() * size_of::<Block>()
            + self.births.capacity() * size_of::<u32>()
    }

    /// Makes room in `blocks` for one more block: by growing it while the
    /// grown queue fits the budget, otherwise by dropping the oldest block,
    /// so once the budget is full each Generation reuses the slot of the one
    /// it drops instead of reallocating the queue.
    fn make_room_for_a_block(&mut self) {
        let capacity = self.blocks.capacity();
        if self.blocks.len() < capacity {
            return;
        }
        let grown = (capacity * 2).max(16);
        let fits = self.bytes() + (grown - capacity) * size_of::<Block>() <= self.budget;
        if fits || !self.drop_oldest() {
            self.blocks.reserve_exact(grown - self.blocks.len());
        }
    }

    /// Drops the oldest block, if any, returning whether there was one.
    fn drop_oldest(&mut self) -> bool {
        let Some(block) = self.blocks.pop_front() else {
            return false;
        };
        self.block_bytes -= block.records.len() * size_of::<u32>();
        true
    }

    /// Drops the oldest Generations' records, and last the current
    /// Generation's Births, until within the budget, between Generations.
    ///
    /// Capacity freed by dropping is reused by later Generations, so it is
    /// given back only once most of it is spare, as after the budget is
    /// lowered. A Generation whose Births are dropped this way is the oldest
    /// left, so lookups reaching it end there, beyond Recorded History.
    pub(crate) fn keep_within_budget(&mut self) {
        while self.bytes() > self.budget {
            let capacity = self.blocks.capacity();
            if capacity > 2 * self.blocks.len() + 16 {
                self.blocks.shrink_to_fit();
                if self.blocks.capacity() < capacity {
                    continue;
                }
            }
            if !self.drop_oldest() {
                self.blocks = VecDeque::new();
                self.births = Vec::new();
            }
        }
    }

    /// The oldest Generation whose records are retained.
    fn first_retained(&self) -> u64 {
        self.current - self.blocks.len() as u64
    }

    /// The block of `generation`, if retained.
    fn block(&self, generation: u64) -> Option<&Block> {
        let offset = generation.checked_sub(self.first_retained())?;
        self.blocks.get(offset as usize)
    }

    /// Whether the Organism at Cell `index` at the end of `generation`, a
    /// Generation already over, was placed in that Generation.
    pub(crate) fn was_placed(&self, generation: u64, index: usize) -> bool {
        self.block(generation)
            .is_some_and(|block| find(block.placed(), index).is_some())
    }

    /// The code of the Birth at Cell `index` in `generation`, if recorded.
    fn birth(&self, generation: u64, index: usize) -> Option<u8> {
        let births = if generation == self.current {
            &self.births[..]
        } else {
            self.block(generation)?.births()
        };
        find(births, index).map(|record| (record >> INDEX_BITS) as u8)
    }

    /// How the Organism at Cell `index` at the end of `generation`, a
    /// Generation already over, was created in it, if it was.
    fn created_in(&self, generation: u64, index: usize) -> Option<Created> {
        if self.was_placed(generation, index) {
            Some(Created::Placed)
        } else {
            self.birth(generation, index).map(Created::Born)
        }
    }

    /// The Generation and manner in which the Organism live at Cell `index`
    /// at the end of `generation` was created; `None` if before Recorded
    /// History. `live_since` is the Birth Generation of the Organism live
    /// there now, if any.
    fn latest_creation(
        &self,
        index: usize,
        generation: u64,
        live_since: Option<u64>,
    ) -> Option<(u64, Created)> {
        if let Some(born) = live_since.filter(|&born| born <= generation) {
            return self.created_in(born, index).map(|created| (born, created));
        }
        (self.first_retained()..=generation)
            .rev()
            .find_map(|g| self.created_in(g, index).map(|created| (g, created)))
    }
}

/// The World as a Pedigree query sees it.
pub(crate) struct PedigreeQuery<'a> {
    pub(crate) history: &'a RecordedHistory,
    pub(crate) width: usize,
    pub(crate) height: usize,
    /// The Birth Generation of the Organism live at each Cell, if any.
    pub(crate) live_since: &'a dyn Fn(usize) -> Option<u64>,
}

/// What makes two ancestors the same Organism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AncestorKey {
    Created(usize, u64, Created),
    Beyond(usize),
}

impl PedigreeQuery<'_> {
    /// The Cell of the Neighbour of Cell `index` in `direction`.
    fn neighbour(&self, index: usize, direction: usize) -> usize {
        let (w, h) = (self.width, self.height);
        let (x, y) = (index % w, index / w);
        let (dx, dy) = [
            (w - 1, h - 1),
            (0, h - 1),
            (1, h - 1),
            (w - 1, 0),
            (1, 0),
            (w - 1, 1),
            (0, 1),
            (1, 1),
        ][direction];
        (y + dy) % h * w + (x + dx) % w
    }

    /// The Pedigree, to `depth` Births back, of the Organism at Cell `index`
    /// with Birth Generation `born`, which was placed rather than Born if
    /// `placed`.
    pub(crate) fn pedigree(&self, index: usize, born: u64, placed: bool, depth: usize) -> Pedigree {
        let root = if placed {
            Some(Created::Placed)
        } else {
            self.history.birth(born, index).map(Created::Born)
        };
        let mut organisms = vec![self.organism(index, Some(born), root, 0, depth)];
        let mut keys = HashMap::new();
        let mut queue = VecDeque::from([(0, index, born, root)]);
        while let Some((at, index, born, created)) = queue.pop_front() {
            let Some(Created::Born(code)) = created else {
                continue;
            };
            let depth_here = organisms[at].depth;
            if depth_here == depth {
                continue;
            }
            let mut parents = Vec::new();
            for (direction, allele) in birth_parents(code) {
                let cell = self.neighbour(index, direction);
                let live_since = (self.live_since)(cell);
                let creation = self.history.latest_creation(cell, born - 1, live_since);
                let key = match creation {
                    Some((generation, created)) => AncestorKey::Created(cell, generation, created),
                    None => AncestorKey::Beyond(cell),
                };
                let organism = *keys.entry(key).or_insert_with(|| {
                    let birth_generation = match creation {
                        Some((generation, _)) => Some(generation),
                        None => live_since.filter(|&since| since < born),
                    };
                    organisms.push(self.organism(
                        cell,
                        birth_generation,
                        creation.map(|(_, created)| created),
                        depth_here + 1,
                        depth,
                    ));
                    if let Some((generation, created)) = creation {
                        queue.push_back((organisms.len() - 1, cell, generation, Some(created)));
                    }
                    organisms.len() - 1
                });
                parents.push(PedigreeParent { organism, allele });
            }
            if let Origin::Born { parents: found, .. } = &mut organisms[at].origin {
                *found = parents;
            }
        }
        Pedigree { organisms }
    }

    fn organism(
        &self,
        index: usize,
        birth_generation: Option<u64>,
        created: Option<Created>,
        depth: usize,
        max_depth: usize,
    ) -> PedigreeOrganism {
        let origin = match created {
            None => Origin::BeyondRecordedHistory,
            Some(Created::Placed) => Origin::Placed,
            Some(Created::Born(_)) if depth == max_depth => Origin::Unexplored,
            Some(Created::Born(code)) => Origin::Born {
                parents: Vec::new(),
                from_allele_pool: 2 - birth_parents(code).len(),
            },
        };
        PedigreeOrganism {
            x: index % self.width,
            y: index / self.width,
            birth_generation,
            depth,
            origin,
        }
    }
}
