//! Births with fewer than two Parents (B0 and B1 Rules) draw their missing
//! Alleles from the Allele Pool.

use genetic_life_core::{Allele, MutationSettings, Rule, World};
use proptest::prelude::*;

fn allele(hue: f64, dominance: u8) -> Allele {
    Allele { hue, dominance }
}

fn rule(notation: &str) -> Rule {
    notation.parse().unwrap()
}

fn without_mutation(mut world: World) -> World {
    world.set_mutation(MutationSettings::new(0.0, 0.0).unwrap());
    world
}

/// Equal Dominance Levels, so a heterozygote shows the blend, hue 60.
const RED: Allele = Allele {
    hue: 0.0,
    dominance: 1,
};
const GREEN: Allele = Allele {
    hue: 120.0,
    dominance: 1,
};

/// A lone Parent homozygous for RED at (5, 5) among a row of GREEN Cells far
/// enough away that each of the Parent's eight Neighbours has it as its only
/// live Neighbour. Under B1 the Allele Pool holds 2 RED and 8 GREEN copies.
#[test]
fn a_b1_birth_takes_one_allele_from_its_parent_and_one_from_the_allele_pool() {
    let mut blends = 0;
    for seed in 0..100 {
        let mut live = vec![(5, 5, 0, 0)];
        live.extend((10..14).map(|x| (x, 12, 1, 1)));
        let mut world =
            without_mutation(World::from_genotypes(16, 16, seed, &[RED, GREEN], &live).unwrap());
        world.set_rule(rule("B1/S"));
        world.step();
        let phenotypes = world.phenotypes();
        for (x, y) in [
            (4, 4),
            (5, 4),
            (6, 4),
            (4, 5),
            (6, 5),
            (4, 6),
            (5, 6),
            (6, 6),
        ] {
            let child = phenotypes[y * 16 + x];
            assert!(
                child == 0.0 || child == 60.0,
                "seed {seed} ({x}, {y}): {child} lacks the Parent's Allele"
            );
            blends += usize::from(child == 60.0);
        }
    }
    let share = blends as f64 / 800.0;
    assert!((share - 0.8).abs() < 0.06, "GREEN from the pool: {share}");
}

/// BLUE is the first Founding Allele but no Cell carries it, so it is not in
/// the Allele Pool. Every Cell away from the two pool Cells has no live
/// Neighbours and is born under B0.
#[test]
fn a_b0_birth_draws_both_alleles_from_the_allele_pool() {
    let blue = allele(240.0, 1);
    let mut seen = [false; 3];
    for seed in 0..20 {
        let live = [(2, 2, 1, 1), (9, 9, 2, 2)];
        let mut world = without_mutation(
            World::from_genotypes(16, 16, seed, &[blue, RED, GREEN], &live).unwrap(),
        );
        world.set_rule(rule("B0/S"));
        world.step();
        assert_eq!(world.population(), 16 * 16 - 2 * 9);
        for child in world.phenotypes().into_iter().filter(|&h| h >= 0.0) {
            let kind = [0.0, 60.0, 120.0].iter().position(|&h| h == child);
            let kind = kind.unwrap_or_else(|| panic!("seed {seed}: {child} is not from the pool"));
            seen[kind] = true;
        }
    }
    assert_eq!(
        seen, [true; 3],
        "RED/RED, RED/GREEN and GREEN/GREEN all occur"
    );
}

/// About 40,000 B0 Births from an empty World each draw two Founding Alleles;
/// one standard error of a share is at most 0.0025, so the tolerance is
/// about eight.
#[test]
fn an_empty_world_draws_from_the_founding_alleles() {
    let mut world =
        without_mutation(World::from_genotypes(200, 200, 7, &[RED, GREEN], &[]).unwrap());
    world.set_rule(rule("B0/S"));
    world.step();
    assert_eq!(world.population(), 200 * 200);
    let phenotypes = world.phenotypes();
    for (hue, expected) in [(0.0, 0.25), (60.0, 0.5), (120.0, 0.25)] {
        let share =
            phenotypes.iter().filter(|&&h| h == hue).count() as f64 / phenotypes.len() as f64;
        assert!((share - expected).abs() < 0.02, "hue {hue}: {share}");
    }
}

/// The Allele Pool holds RED/RED, RED/RED and RED/GREEN, so 5 of its 6 copies
/// are RED: a draw weighted by copy count gives RED with probability 5/6,
/// where a draw over distinct Alleles would give 1/2. About 40,000 B0 Births
/// make about 80,000 draws; one standard error is about 0.0013 and the
/// tolerance is about seven.
#[test]
fn allele_pool_draws_are_weighted_by_copy_count() {
    let live = [(10, 10, 0, 0), (100, 100, 0, 0), (150, 50, 0, 1)];
    let mut world =
        without_mutation(World::from_genotypes(200, 200, 3, &[RED, GREEN], &live).unwrap());
    world.set_rule(rule("B0/S"));
    world.step();
    let births = world.population();
    assert_eq!(births, 200 * 200 - 3 * 9);
    let phenotypes = world.phenotypes();
    let count = |hue: f64| phenotypes.iter().filter(|&&h| h == hue).count();
    assert_eq!(count(0.0) + count(60.0) + count(120.0), births);
    let red = (2 * count(0.0) + count(60.0)) as f64 / (2 * births) as f64;
    assert!((red - 5.0 / 6.0).abs() < 0.01, "RED share {red}");
}

/// Total Allele copies in the census, which must be two per live Cell.
fn census_copies(world: &World) -> usize {
    world.census().alleles.iter().map(|a| a.copies).sum()
}

#[test]
fn every_rule_preset_keeps_two_allele_copies_per_live_cell() {
    for (name, preset) in Rule::PRESETS {
        for seed in 0..3 {
            let mut world = World::soup(32, 32, 0.35, seed).unwrap();
            world.set_rule(preset);
            world.set_mutation(MutationSettings::new(0.05, 30.0).unwrap());
            for _ in 0..10 {
                world.run(20);
                assert_eq!(
                    census_copies(&world),
                    2 * world.population(),
                    "{name} seed {seed} generation {}",
                    world.generation()
                );
            }
        }
    }
}

/// B/S notation for a Rule whose birth set includes 0 or 1 (or both).
fn b0_or_b1_rule() -> impl Strategy<Value = Rule> {
    (1u16..=3, any::<u16>(), any::<u16>()).prop_map(|(low, birth, survival)| {
        let digits = |mask: u16| -> String {
            (0..=8)
                .filter(|n| mask >> n & 1 == 1)
                .map(|n| char::from(b'0' + n as u8))
                .collect()
        };
        rule(&format!("B{}/S{}", digits(birth | low), digits(survival)))
    })
}

proptest! {
    #[test]
    fn b0_and_b1_rules_keep_two_allele_copies_per_live_cell(
        rule in b0_or_b1_rule(), seed in any::<u64>(), density in 0.0f64..=1.0,
        rate in 0.0f64..=1.0, generations in 1u64..60,
    ) {
        let mut world = World::soup(24, 24, density, seed).unwrap();
        world.set_rule(rule);
        world.set_mutation(MutationSettings::new(rate, 45.0).unwrap());
        for _ in 0..generations {
            world.step();
            prop_assert_eq!(census_copies(&world), 2 * world.population());
        }
    }

    #[test]
    fn same_seed_and_settings_give_identical_runs_under_b0_and_b1_rules(
        rule in b0_or_b1_rule(), seed in any::<u64>(), density in 0.0f64..=1.0,
        rate in 0.0f64..=1.0, generations in 0u64..30,
    ) {
        let run = || {
            let mut world = World::soup(24, 24, density, seed).unwrap();
            world.set_rule(rule);
            world.set_mutation(MutationSettings::new(rate, 45.0).unwrap());
            world.run(generations);
            world
        };
        let (a, b) = (run(), run());
        prop_assert_eq!(a.phenotypes(), b.phenotypes());
        prop_assert_eq!(a.allele_tree(), b.allele_tree());
        prop_assert_eq!(a, b);
    }
}
