//! Generations per second for stepping genetic Soups under Conway's Life.
//!
//! Each Soup is run for [`SETTLE_GENERATIONS`] before measuring, so the
//! numbers reflect a running Soup rather than the die-off after Generation 0.
//! Every measured iteration steps a fresh copy of that settled World once, so
//! all samples time the same Generation.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use genetic_life_core::{Rule, World};

/// Seed of every benchmarked Soup.
const SEED: u64 = 8;
/// Probability that a Cell starts live, the CLI default.
const DENSITY: f64 = 0.3;
/// Generations run before measuring.
const SETTLE_GENERATIONS: u64 = 100;
/// Square World sizes benchmarked, in Cells per side.
const SIZES: [usize; 2] = [256, 1024];

fn step(c: &mut Criterion) {
    let mut group = c.benchmark_group("generation");
    group.throughput(Throughput::Elements(1));
    for size in SIZES {
        let mut world = World::soup(size, size, DENSITY, SEED).expect("benchmark Soup is valid");
        world.set_rule(Rule::CONWAY);
        world.run(SETTLE_GENERATIONS);
        group.bench_function(format!("{size}x{size}"), |b| {
            b.iter_batched_ref(|| world.clone(), World::step, BatchSize::LargeInput)
        });
    }
    group.finish();
}

criterion_group!(benches, step);
criterion_main!(benches);
