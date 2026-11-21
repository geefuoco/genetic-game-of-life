use genetic_life_core::{Allele, MutationSettings, Rule, World, DEAD_PHENOTYPE, MAX_DOMINANCE};
use proptest::prelude::*;

fn allele(hue: f64, dominance: u8) -> Allele {
    Allele { hue, dominance }
}

fn without_mutation(mut world: World) -> World {
    world.set_mutation(MutationSettings::new(0.0, 0.0).unwrap());
    world
}

fn founding_alleles(world: &World) -> Vec<Allele> {
    world
        .allele_tree()
        .iter()
        .filter(|node| node.source.is_none())
        .map(|node| node.allele)
        .collect()
}

/// A 16x16 World with an L-shaped trio of Parents around the dead Cell (5, 5),
/// so exactly one Birth happens there. Genotypes are `(first, second)` indices.
fn trio(alleles: &[Allele], genotype: (usize, usize)) -> World {
    let live: Vec<_> = [(4, 4), (5, 4), (4, 5)]
        .iter()
        .map(|&(x, y)| (x, y, genotype.0, genotype.1))
        .collect();
    without_mutation(World::from_genotypes(16, 16, 1, alleles, &live).unwrap())
}

/// The Phenotype of a child whose Parents are both `(first, second)`-typed.
fn child_phenotype(alleles: &[Allele], genotype: (usize, usize)) -> f64 {
    let mut world = trio(alleles, genotype);
    world.step();
    world.phenotypes()[5 * 16 + 5]
}

fn phenotype_of(a: Allele, b: Allele) -> f64 {
    let world = World::from_genotypes(16, 16, 1, &[a, b], &[(3, 3, 0, 1)]).unwrap();
    world.phenotypes()[3 * 16 + 3]
}

#[test]
fn homozygous_cell_shows_its_hue() {
    assert_eq!(phenotype_of(allele(100.0, 1), allele(100.0, 1)), 100.0);
}

#[test]
fn higher_dominance_level_masks_the_other_allele() {
    assert_eq!(phenotype_of(allele(10.0, 2), allele(200.0, 1)), 10.0);
    assert_eq!(phenotype_of(allele(10.0, 0), allele(200.0, 3)), 200.0);
}

/// Includes a blend across the 0/360 seam: 350 and 30 meet at 10.
#[test]
fn equal_dominance_blends_along_the_shorter_arc_in_both_directions() {
    assert_eq!(phenotype_of(allele(10.0, 1), allele(70.0, 1)), 40.0);
    assert_eq!(phenotype_of(allele(70.0, 1), allele(10.0, 1)), 40.0);
    assert_eq!(phenotype_of(allele(350.0, 2), allele(30.0, 2)), 10.0);
    assert_eq!(phenotype_of(allele(30.0, 2), allele(350.0, 2)), 10.0);
}

#[test]
fn exactly_opposite_equal_dominance_hues_blend_to_lower_hue_plus_90() {
    assert_eq!(phenotype_of(allele(30.0, 1), allele(210.0, 1)), 120.0);
    assert_eq!(phenotype_of(allele(210.0, 1), allele(30.0, 1)), 120.0);
}

#[test]
fn phenotype_buffer_has_one_value_per_cell_and_marks_dead_cells() {
    let world = World::from_genotypes(16, 16, 1, &[allele(50.0, 0)], &[(2, 1, 0, 0)]).unwrap();
    let buffer = world.phenotypes();
    assert_eq!(buffer.len(), 16 * 16);
    assert_eq!(buffer[16 + 2], 50.0);
    assert_eq!(buffer.iter().filter(|&&v| v == DEAD_PHENOTYPE).count(), 255);
}

/// Blends are only ever displayed: after many Generations of crosses without
/// Mutation, the Alleles still alive are exactly Founding Alleles, so no blend
/// was inherited.
#[test]
fn blended_cells_pass_on_original_alleles_never_the_blend() {
    for seed in 0..20 {
        let mut world = without_mutation(World::soup_with_alleles(32, 32, 0.4, seed, 3).unwrap());
        let founding = founding_alleles(&world);
        world.run(40);
        let census = world.census();
        assert!(census.alleles.iter().all(|a| founding.contains(&a.allele)));
    }
}

/// Three Parents each homozygous for a different Allele: a child of two
/// distinct Parents is never homozygous, which fails if one Parent is picked twice.
#[test]
fn a_birth_never_takes_both_alleles_from_the_same_parent() {
    let alleles = [allele(0.0, 0), allele(100.0, 0), allele(200.0, 0)];
    for seed in 0..200 {
        let live = [(4, 4, 0, 0), (5, 4, 1, 1), (4, 5, 2, 2)];
        let mut world =
            without_mutation(World::from_genotypes(16, 16, seed, &alleles, &live).unwrap());
        world.step();
        let child = world.phenotypes()[5 * 16 + 5];
        let hues = alleles.map(|a| a.hue);
        assert!(!hues.contains(&child), "seed {seed}: homozygous child");
    }
}

#[test]
fn founding_alleles_are_evenly_spaced_and_deterministic_from_the_seed() {
    let a = founding_alleles(&World::soup_with_alleles(32, 32, 0.4, 99, 4).unwrap());
    let b = founding_alleles(&World::soup_with_alleles(32, 32, 0.4, 99, 4).unwrap());
    assert_eq!(a, b);
    assert_eq!(a.len(), 4);
    for pair in 0..4 {
        let gap = a[(pair + 1) % 4].hue - a[pair].hue;
        let gap = if gap <= 0.0 { gap + 360.0 } else { gap };
        assert!((gap - 90.0).abs() < 1e-9, "gap {gap}");
    }
    assert!(a.iter().all(|a| a.dominance <= MAX_DOMINANCE));
    let c = founding_alleles(&World::soup_with_alleles(32, 32, 0.4, 100, 4).unwrap());
    assert_ne!(a, c);
}

#[test]
fn founding_allele_count_must_be_in_range() {
    assert!(World::soup_with_alleles(32, 32, 0.4, 1, 0).is_err());
    assert!(World::soup_with_alleles(32, 32, 0.4, 1, 17).is_err());
    assert!(World::soup_with_alleles(32, 32, 0.4, 1, 1).is_ok());
    assert!(World::from_genotypes(16, 16, 1, &[], &[]).is_err());
}

#[test]
fn same_seed_gives_identical_genetic_runs() {
    let run = || {
        let mut w = World::soup_with_alleles(40, 40, 0.35, 5, 5).unwrap();
        w.run(30);
        (w.phenotypes(), w.census())
    };
    assert_eq!(run(), run());
}

/// Allele 0 dominates 1; Allele 2 is carried by nobody so is not reported.
#[test]
fn census_reports_copies_shown_hidden_and_fully_hidden_alleles() {
    let alleles = [allele(0.0, 3), allele(120.0, 0), allele(240.0, 0)];
    let world = World::from_genotypes(
        16,
        16,
        1,
        &alleles,
        &[(1, 1, 0, 1), (8, 8, 0, 0), (12, 3, 1, 1)],
    )
    .unwrap();
    let census = world.census();
    assert_eq!(census.population, 3);
    assert_eq!(census.generation, 0);
    assert_eq!(census.population_history, vec![3]);
    assert_eq!(census.alleles.len(), 2);
    let dominant = census.alleles[0];
    assert_eq!(
        (dominant.copies, dominant.shown, dominant.hidden),
        (3, 2, 0)
    );
    let recessive = census.alleles[1];
    assert_eq!(
        (recessive.copies, recessive.shown, recessive.hidden),
        (3, 1, 1)
    );
    assert!(census.fully_hidden.is_empty());
}

#[test]
fn an_allele_only_carried_under_a_dominant_one_is_fully_hidden() {
    let alleles = [allele(0.0, 3), allele(120.0, 0)];
    let world = World::from_genotypes(16, 16, 1, &alleles, &[(1, 1, 0, 1)]).unwrap();
    assert_eq!(world.census().fully_hidden, vec![alleles[1]]);
}

#[test]
fn census_tracks_population_history_across_generations() {
    let mut world = World::from_cells(16, 16, 1, &[(4, 5), (5, 5), (6, 5)]).unwrap();
    world.run(3);
    let census = world.census();
    assert_eq!(census.generation, 3);
    assert_eq!(census.population_history, vec![3, 3, 3, 3]);
    assert_eq!(world.population_history(), [3, 3, 3, 3]);
}

#[test]
fn single_allele_world_always_births_that_allele() {
    assert_eq!(child_phenotype(&[allele(77.0, 2)], (0, 0)), 77.0);
}

proptest! {
    #[test]
    fn census_counts_exactly_two_allele_copies_per_live_cell(
        seed in any::<u64>(), alleles in 1usize..=16, generations in 0u64..20,
    ) {
        let mut world = World::soup_with_alleles(24, 24, 0.4, seed, alleles).unwrap();
        world.run(generations);
        let census = world.census();
        let copies: usize = census.alleles.iter().map(|a| a.copies).sum();
        prop_assert_eq!(copies, 2 * census.population);
        for a in &census.alleles {
            prop_assert!(a.shown + a.hidden <= a.copies);
        }
    }

    #[test]
    fn genetic_run_keeps_the_same_live_cells_as_a_rule_only_run(
        seed in any::<u64>(), alleles in 1usize..=16, generations in 0u64..20,
    ) {
        let rule: Rule = "B36/S23".parse().unwrap();
        let mut genetic = World::soup_with_alleles(24, 24, 0.4, seed, alleles).unwrap();
        genetic.set_rule(rule);
        let mut reference = World::from_cells(24, 24, seed, &genetic.live_cells()).unwrap();
        reference.set_rule(rule);
        genetic.run(generations);
        reference.run(generations);
        prop_assert_eq!(genetic.live_cells(), reference.live_cells());
    }
}
