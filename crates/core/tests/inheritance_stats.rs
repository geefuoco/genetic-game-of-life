//! Statistical tests of Mendelian inheritance with fixed Seeds. Tolerances are
//! several standard errors wide for the sample sizes used, so they hold for
//! any correct implementation but fail on a biased one.

use genetic_life_core::{Allele, World, DEAD_PHENOTYPE};

const SIZE: usize = 200;

/// A deterministic scatter of live Cells (about 30%), every one typed
/// `genotype`.
fn uniform_world(alleles: &[Allele], genotype: (usize, usize), seed: u64) -> World {
    let mut state = seed ^ 0x1234_5678_9ABC_DEF0;
    let mut live = vec![];
    for y in 0..SIZE {
        for x in 0..SIZE {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            if (state >> 33) % 10 < 3 {
                live.push((x, y, genotype.0, genotype.1));
            }
        }
    }
    World::from_genotypes(SIZE, SIZE, seed, alleles, &live).unwrap()
}

/// Phenotypes of Cells born in the first Generation.
fn first_generation_births(mut world: World) -> Vec<f64> {
    let before = world.phenotypes();
    world.step();
    world
        .phenotypes()
        .into_iter()
        .zip(before)
        .filter(|&(after, before)| before == DEAD_PHENOTYPE && after != DEAD_PHENOTYPE)
        .map(|(after, _)| after)
        .collect()
}

fn fraction(births: &[f64], hue: f64) -> f64 {
    births.iter().filter(|&&h| h == hue).count() as f64 / births.len() as f64
}

#[test]
fn two_carriers_produce_about_a_quarter_showing_the_hidden_allele() {
    let alleles = [
        Allele {
            hue: 0.0,
            dominance: 3,
        },
        Allele {
            hue: 120.0,
            dominance: 0,
        },
    ];
    for seed in [1, 2, 3] {
        let births = first_generation_births(uniform_world(&alleles, (0, 1), seed));
        assert!(births.len() > 2000, "sample too small: {}", births.len());
        let revealed = fraction(&births, 120.0);
        assert!((revealed - 0.25).abs() < 0.03, "seed {seed}: {revealed}");
    }
}

#[test]
fn equal_dominance_cross_gives_about_one_two_one() {
    let alleles = [
        Allele {
            hue: 0.0,
            dominance: 1,
        },
        Allele {
            hue: 120.0,
            dominance: 1,
        },
    ];
    for seed in [1, 2, 3] {
        let births = first_generation_births(uniform_world(&alleles, (0, 1), seed));
        assert!(births.len() > 2000, "sample too small: {}", births.len());
        for (hue, expected) in [(0.0, 0.25), (60.0, 0.5), (120.0, 0.25)] {
            let got = fraction(&births, hue);
            assert!(
                (got - expected).abs() < 0.035,
                "seed {seed} hue {hue}: {got}"
            );
        }
    }
}
