/// Versioned, seeded pseudo-random number generator (SplitMix64).
///
/// The sequence for a given seed is fixed for a given [`Rng::VERSION`]; any
/// change to the algorithm must bump the version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Version of the algorithm that produces the sequence.
    pub const VERSION: u32 = 1;

    /// Creates a generator whose whole sequence is fixed by `seed`.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The whole state; [`Rng::new`] with it continues the same sequence.
    pub(crate) fn state(&self) -> u64 {
        self.state
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Next value uniformly distributed in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
