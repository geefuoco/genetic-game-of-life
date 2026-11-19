//! Births with fewer than two Parents (B0 and B1 Rules) draw their missing
//! Alleles from the Allele Pool.

use genetic_life_core::{Allele, MutationSettings, Rule, World};
use proptest::prelude::*;
use std::collections::HashMap;

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
const BLUE: Allele = Allele {
    hue: 240.0,
    dominance: 1,
};

/// A lone Parent homozygous for RED at (5, 5), and a row of four GREEN
/// homozygotes far enough away that each of the Parent's eight Neighbours has
/// the Parent as its only live Neighbour. The Allele Pool holds 2 RED and 8
/// GREEN copies, so a child gets GREEN from the pool with probability 0.8.
/// Over 800 children one standard error is about 0.014; the tolerance is
/// about four.
#[test]
fn a_b1_birth_takes_one_allele_from_its_parent_and_one_from_the_allele_pool() {
    const SIZE: usize = 16;
    let mut children = 0;
    let mut blends = 0;
    for seed in 0..100 {
        let mut live = vec![(5, 5, 0, 0)];
        live.extend((10..14).map(|x| (x, 12, 1, 1)));
        let mut world = without_mutation(
            World::from_genotypes(SIZE, SIZE, seed, &[RED, GREEN], &live).unwrap(),
        );
        world.set_rule(rule("B1/S"));
        world.step();
        let phenotypes = world.phenotypes();
        for (x, y) in (4..=6).flat_map(|y| (4..=6).map(move |x| (x, y))) {
            if (x, y) == (5, 5) {
                continue;
            }
            let child = phenotypes[y * SIZE + x];
            assert!(
                child == 0.0 || child == 60.0,
                "seed {seed} ({x}, {y}): {child} lacks the Parent's Allele"
            );
            children += 1;
            blends += usize::from(child == 60.0);
        }
    }
    let share = blends as f64 / children as f64;
    assert!((share - 0.8).abs() < 0.06, "GREEN from the pool: {share}");
}

/// BLUE is the first Founding Allele but no Cell carries it, so it is not in
/// the Allele Pool. Every Cell away from the two pool Cells has no live
/// Neighbours and is born under B0.
#[test]
fn a_b0_birth_draws_both_alleles_from_the_allele_pool() {
    let mut seen = [false; 3];
    for seed in 0..20 {
        let live = [(2, 2, 1, 1), (9, 9, 2, 2)];
        let mut world = without_mutation(
            World::from_genotypes(16, 16, seed, &[BLUE, RED, GREEN], &live).unwrap(),
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

/// About 40,000 B0 Births from an empty World each draw two Founding Alleles,
/// so 1/4 are RED/RED, 1/2 RED/GREEN and 1/4 GREEN/GREEN. One standard error
/// of a share is at most 0.0025; the tolerance is about eight standard
/// errors, still far from the shares of any other fallback.
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
/// tolerance is about seven standard errors.
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

/// Checks that the census counts two Allele copies per live Cell and that
/// every carried Allele is retained in the Allele Tree with a lineage back to
/// a Founding Allele.
fn check_census(world: &World) -> Result<(), String> {
    let census = world.census();
    let copies: usize = census.alleles.iter().map(|a| a.copies).sum();
    if copies != 2 * census.population {
        return Err(format!("{copies} copies in {} Cells", census.population));
    }
    let sources: HashMap<_, _> = world
        .allele_tree()
        .into_iter()
        .map(|node| (node.id, node.source))
        .collect();
    for carried in &census.alleles {
        let mut next = Some(carried.id);
        while let Some(id) = next {
            next = *sources
                .get(&id)
                .ok_or_else(|| format!("{id:?}, carried or an ancestor, is not retained"))?;
        }
    }
    Ok(())
}

#[test]
fn every_rule_preset_runs_many_generations_with_a_sound_census() {
    for (name, preset) in Rule::PRESETS {
        for seed in 0..3 {
            let mut world = World::soup(32, 32, 0.35, seed).unwrap();
            world.set_rule(preset);
            world.set_mutation(MutationSettings::new(0.05, 30.0).unwrap());
            for _ in 0..10 {
                world.run(20);
                if let Err(problem) = check_census(&world) {
                    panic!(
                        "{name} seed {seed} generation {}: {problem}",
                        world.generation()
                    );
                }
            }
        }
    }
}

/// B/S notation for a Rule whose birth set includes 0 or 1 (or both).
fn b0_or_b1_rule() -> impl Strategy<Value = Rule> {
    (1u16..=3, any::<u16>(), any::<u16>()).prop_map(|(b0_b1, birth, survival)| {
        let digits = |mask: u16| -> String {
            (0..=8)
                .filter(|n| mask >> n & 1 == 1)
                .map(|n| char::from(b'0' + n as u8))
                .collect()
        };
        rule(&format!("B{}/S{}", digits(birth | b0_b1), digits(survival)))
    })
}

proptest! {
    #[test]
    fn b0_and_b1_rules_run_many_generations_with_a_sound_census(
        rule in b0_or_b1_rule(), seed in any::<u64>(), density in 0.0f64..=1.0,
        rate in 0.0f64..=1.0, generations in 1u64..60,
    ) {
        let mut world = World::soup(24, 24, density, seed).unwrap();
        world.set_rule(rule);
        world.set_mutation(MutationSettings::new(rate, 45.0).unwrap());
        for _ in 0..generations {
            world.step();
            prop_assert_eq!(check_census(&world), Ok(()));
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
        prop_assert_eq!(run(), run());
    }
}
