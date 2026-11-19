use genetic_life_core::{Allele, AlleleNode, AlleleStatus, MutationSettings, Rule, World};
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

fn allele(hue: f64, dominance: u8) -> Allele {
    Allele { hue, dominance }
}

fn mutation(rate: f64, magnitude: f64) -> MutationSettings {
    MutationSettings::new(rate, magnitude).unwrap()
}

/// Shortest distance between two hues around the color wheel.
fn hue_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).abs();
    d.min(360.0 - d)
}

/// A 16x16 World holding a blinker homozygous for `founding`, so every
/// Generation has exactly two Births.
fn blinker(founding: Allele, seed: u64) -> World {
    let live = [(5, 4, 0, 0), (5, 5, 0, 0), (5, 6, 0, 0)];
    World::from_genotypes(16, 16, seed, &[founding], &live).unwrap()
}

fn mutants(world: &World) -> Vec<AlleleNode> {
    world
        .allele_tree()
        .into_iter()
        .filter(|node| node.source.is_some())
        .collect()
}

#[test]
fn a_new_world_mutates_at_one_percent_by_up_to_thirty_degrees() {
    let world = World::soup(32, 32, 0.3, 1).unwrap();
    assert_eq!(world.mutation().rate(), 0.01);
    assert_eq!(world.mutation().magnitude(), 30.0);
    assert_eq!(world.mutation(), MutationSettings::default());
}

#[test]
fn mutation_settings_reject_out_of_range_values() {
    for (rate, magnitude) in [
        (-0.1, 30.0),
        (1.1, 30.0),
        (f64::NAN, 30.0),
        (0.5, -1.0),
        (0.5, 180.1),
        (0.5, f64::NAN),
    ] {
        assert!(
            MutationSettings::new(rate, magnitude).is_err(),
            "{rate} {magnitude}"
        );
    }
    for (rate, magnitude) in [(0.0, 0.0), (1.0, 180.0), (0.25, 45.0)] {
        let settings = MutationSettings::new(rate, magnitude).unwrap();
        assert_eq!((settings.rate(), settings.magnitude()), (rate, magnitude));
    }
}

#[test]
fn mutation_settings_can_change_mid_run() {
    let mut world = World::soup(32, 32, 0.3, 1).unwrap();
    world.run(3);
    let settings = MutationSettings::new(0.2, 90.0).unwrap();
    world.set_mutation(settings);
    assert_eq!(world.mutation(), settings);
    assert_eq!(world.generation(), 3);
}

/// The source hue sits near the 0/360 seam, so shifts in either direction
/// must wrap.
#[test]
fn a_mutant_records_its_source_and_origin_and_stays_within_the_magnitude() {
    for seed in 0..50 {
        let mut world = blinker(allele(355.0, 1), seed);
        world.set_mutation(mutation(0.0, 10.0));
        world.run(2);
        world.set_mutation(mutation(1.0, 10.0));
        world.step();
        let founding = world.allele_tree()[0];
        assert_eq!((founding.source, founding.origin), (None, 0));
        let mutants = mutants(&world);
        assert_eq!(mutants.len(), 4, "seed {seed}: two Births pass on four");
        for mutant in mutants {
            assert_ne!(mutant.id, founding.id);
            assert_eq!(mutant.source, Some(founding.id));
            assert_eq!(mutant.origin, 3);
            assert!((0.0..360.0).contains(&mutant.allele.hue), "{mutant:?}");
            assert!(hue_distance(mutant.allele.hue, 355.0) <= 10.0, "{mutant:?}");
        }
    }
}

/// Every Allele passed on in the first Generation of a single-Allele Soup
/// either stays that Allele or becomes a new mutant carried by the newborn,
/// so mutants over twice the Births is the observed rate. With about 14,000
/// Alleles passed on per Seed, one standard error is about 0.0008 at 1% and
/// 0.0018 at 5%; the tolerances are about four standard errors.
#[test]
fn observed_mutation_rate_matches_the_configured_rate() {
    for (rate, tolerance) in [(0.01, 0.0035), (0.05, 0.0075)] {
        for seed in [1, 2, 3] {
            let mut world = World::soup_with_alleles(200, 200, 0.3, seed, 1).unwrap();
            world.set_mutation(mutation(rate, 30.0));
            let alive_before: HashSet<_> = world.live_cells().into_iter().collect();
            world.step();
            let births = world
                .live_cells()
                .iter()
                .filter(|cell| !alive_before.contains(cell))
                .count();
            assert!(births > 5000, "sample too small: {births}");
            let observed = mutants(&world).len() as f64 / (2 * births) as f64;
            assert!(
                (observed - rate).abs() < tolerance,
                "rate {rate} seed {seed}: observed {observed}"
            );
        }
    }
}

/// A shift just below zero from hue 0 rounds to exactly 360 when wrapped
/// naively.
#[test]
fn a_tiny_shift_below_hue_zero_wraps_into_range() {
    for seed in 0..20 {
        let mut world = blinker(allele(0.0, 0), seed);
        world.set_mutation(mutation(1.0, 1e-14));
        world.step();
        for mutant in mutants(&world) {
            assert!((0.0..360.0).contains(&mutant.allele.hue), "{mutant:?}");
        }
    }
}

/// With every passed Allele mutating, about 14,000 mutants each draw a
/// Dominance Level; one standard error of a level's share is about 0.004.
#[test]
fn mutant_dominance_levels_are_uniformly_random() {
    let mut world = World::soup_with_alleles(200, 200, 0.3, 4, 1).unwrap();
    world.set_mutation(mutation(1.0, 30.0));
    world.step();
    let mutants = mutants(&world);
    assert!(
        mutants.len() > 10_000,
        "sample too small: {}",
        mutants.len()
    );
    for level in 0..=3 {
        let share = mutants
            .iter()
            .filter(|m| m.allele.dominance == level)
            .count() as f64
            / mutants.len() as f64;
        assert!((share - 0.25).abs() < 0.02, "level {level}: {share}");
    }
}

#[test]
fn lineage_runs_from_an_allele_back_to_its_founding_allele() {
    let mut world = blinker(allele(100.0, 1), 3);
    world.set_mutation(mutation(1.0, 20.0));
    world.run(12);
    let founding = world.allele_tree()[0];
    assert_eq!(world.lineage(founding.id), Some(vec![founding]));
    let mut deepest = 0;
    for node in world.allele_tree() {
        let chain = world.lineage(node.id).unwrap();
        assert_eq!(chain[0], node);
        assert_eq!(chain.last().unwrap().id, founding.id);
        for pair in chain.windows(2) {
            assert_eq!(pair[0].source, Some(pair[1].id));
            assert!(pair[0].origin > pair[1].origin);
        }
        deepest = deepest.max(chain.len());
    }
    assert!(deepest >= 4, "deepest lineage {deepest}");
}

/// The dominant Allele shows, the recessive one is only carried, and an
/// uncarried Founding Allele is retained as extinct.
#[test]
fn allele_tree_marks_living_hidden_and_extinct_alleles() {
    let alleles = [allele(0.0, 3), allele(120.0, 0), allele(240.0, 0)];
    let world = World::from_genotypes(16, 16, 1, &alleles, &[(1, 1, 0, 1)]).unwrap();
    let statuses: Vec<_> = world
        .allele_tree()
        .iter()
        .map(|node| (node.allele, node.status))
        .collect();
    assert_eq!(
        statuses,
        vec![
            (alleles[0], AlleleStatus::Living),
            (alleles[1], AlleleStatus::Hidden),
            (alleles[2], AlleleStatus::Extinct),
        ]
    );
}

/// An L-tromino grows into a block with one newborn carrying two mutants;
/// a Rule with no Birth or Survival then kills every Cell.
#[test]
fn mutants_with_no_carriers_and_no_carried_descendants_are_pruned() {
    let live = [(4, 4, 0, 0), (5, 4, 0, 0), (4, 5, 0, 0)];
    let mut world = World::from_genotypes(16, 16, 1, &[allele(50.0, 0)], &live).unwrap();
    world.set_mutation(mutation(1.0, 30.0));
    world.step();
    let born = mutants(&world);
    assert_eq!(born.len(), 2);
    world.set_rule("B/S".parse().unwrap());
    world.step();
    assert_eq!(world.population(), 0);
    for mutant in born {
        assert_eq!(world.lineage(mutant.id), None);
    }
    let tree = world.allele_tree();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].source, None);
    assert_eq!(tree[0].status, AlleleStatus::Extinct);
}

#[test]
fn an_extinct_mutant_is_retained_while_a_descendant_is_carried() {
    let mut world = World::soup_with_alleles(48, 48, 0.4, 11, 2).unwrap();
    world.set_rule(Rule::preset("day-and-night").unwrap());
    world.set_mutation(mutation(0.3, 30.0));
    world.run(30);
    let tree = world.allele_tree();
    let extinct_ancestor = tree
        .iter()
        .find(|node| node.source.is_some() && node.status == AlleleStatus::Extinct)
        .expect("a high Mutation rate leaves extinct ancestors");
    assert!(tree.iter().any(|node| {
        node.status != AlleleStatus::Extinct
            && world.lineage(node.id).unwrap().contains(extinct_ancestor)
    }));
}

proptest! {
    #[test]
    fn mutant_hues_stay_within_the_magnitude_of_their_source(
        seed in any::<u64>(), hue in 0.0f64..360.0, magnitude in 0.0f64..=180.0,
    ) {
        let mut world = blinker(allele(hue, 2), seed);
        world.set_mutation(mutation(1.0, magnitude));
        world.step();
        for mutant in mutants(&world) {
            prop_assert!((0.0..360.0).contains(&mutant.allele.hue), "{:?}", mutant);
            prop_assert!(hue_distance(mutant.allele.hue, hue) <= magnitude + 1e-9, "{:?}", mutant);
        }
    }

    #[test]
    fn pruning_never_removes_an_ancestor_of_a_carried_allele_and_keeps_nothing_else(
        seed in any::<u64>(), rate in 0.0f64..=1.0, generations in 1u64..25,
    ) {
        let mut world = World::soup_with_alleles(24, 24, 0.4, seed, 3).unwrap();
        world.set_rule(Rule::preset("day-and-night").unwrap());
        world.set_mutation(mutation(rate, 40.0));
        for _ in 0..generations {
            world.step();
            let tree = world.allele_tree();
            let by_id: HashMap<_, _> = tree.iter().map(|node| (node.id, node)).collect();
            let mut needed: HashSet<_> = tree
                .iter()
                .filter(|node| node.source.is_none())
                .map(|node| node.id)
                .collect();
            for carried in world.census().alleles {
                let mut next = Some(carried.id);
                while let Some(id) = next {
                    let node = by_id.get(&id);
                    prop_assert!(node.is_some(), "ancestor {:?} of {:?} pruned", id, carried.id);
                    needed.insert(id);
                    next = node.unwrap().source;
                }
            }
            let retained: HashSet<_> = tree.iter().map(|node| node.id).collect();
            prop_assert_eq!(retained, needed);
        }
    }

    #[test]
    fn allele_status_agrees_with_the_census(
        seed in any::<u64>(), rate in 0.0f64..=1.0, generations in 0u64..15,
    ) {
        let mut world = World::soup_with_alleles(24, 24, 0.4, seed, 4).unwrap();
        world.set_mutation(mutation(rate, 60.0));
        world.run(generations);
        let census = world.census();
        for node in world.allele_tree() {
            let line = census.alleles.iter().find(|a| a.id == node.id);
            let expected = match line {
                None => AlleleStatus::Extinct,
                Some(a) if a.shown > 0 => AlleleStatus::Living,
                Some(_) => AlleleStatus::Hidden,
            };
            prop_assert_eq!(node.status, expected);
        }
    }

    #[test]
    fn same_seed_and_settings_give_identical_runs_with_mutation_on(
        seed in any::<u64>(), rate in 0.0f64..=1.0, magnitude in 0.0f64..=180.0,
        generations in 0u64..20,
    ) {
        let run = || {
            let mut world = World::soup_with_alleles(24, 24, 0.4, seed, 4).unwrap();
            world.set_mutation(mutation(rate, magnitude));
            world.run(generations);
            (world.phenotypes(), world.census(), world.allele_tree())
        };
        prop_assert_eq!(run(), run());
    }

    #[test]
    fn mutation_never_changes_which_cells_live(
        seed in any::<u64>(), rate in 0.0f64..=1.0, generations in 0u64..20,
    ) {
        let mut mutating = World::soup_with_alleles(24, 24, 0.4, seed, 4).unwrap();
        mutating.set_mutation(mutation(rate, 90.0));
        let mut reference = World::from_cells(24, 24, seed, &mutating.live_cells()).unwrap();
        mutating.run(generations);
        reference.run(generations);
        prop_assert_eq!(mutating.live_cells(), reference.live_cells());
    }
}
