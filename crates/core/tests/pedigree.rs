//! Pedigrees: the Parents of an inspected Organism, their Parents and so on,
//! kept for Recorded History under a memory budget.

use genetic_life_core::{
    Allele, MutationSettings, Origin, Pedigree, PedigreeOrganism, World, WorldError,
    MAX_PEDIGREE_BUDGET,
};
use proptest::prelude::*;

const RED: Allele = Allele {
    hue: 0.0,
    dominance: 1,
};
const GREEN: Allele = Allele {
    hue: 120.0,
    dominance: 1,
};

fn without_mutation(mut world: World) -> World {
    world.set_mutation(MutationSettings::new(0.0, 0.0).unwrap());
    world
}

/// A horizontal blinker around (5, 5). The middle Organism lives from
/// Generation 0; the end Cells are Born in turn, from two of the three
/// Organisms of the previous Generation.
fn blinker() -> World {
    World::from_cells(16, 16, 1, &[(4, 5), (5, 5), (6, 5)]).unwrap()
}

fn at(organism: &PedigreeOrganism) -> (usize, usize) {
    (organism.x, organism.y)
}

#[test]
fn a_born_organisms_parents_are_two_of_its_live_neighbours() {
    let mut world = blinker();
    world.step();
    world.inspect(5, 4).unwrap().unwrap();
    let pedigree = world.pedigree(1).unwrap();
    let root = &pedigree.organisms[0];
    assert_eq!(
        (at(root), root.birth_generation, root.depth),
        ((5, 4), Some(1), 0)
    );
    let Origin::Born {
        parents,
        from_allele_pool,
    } = &root.origin
    else {
        panic!("the root was Born: {:?}", root.origin);
    };
    assert_eq!(*from_allele_pool, 0);
    assert_eq!(parents.len(), 2);
    let mut cells: Vec<_> = parents
        .iter()
        .map(|parent| at(&pedigree.organisms[parent.organism]))
        .collect();
    cells.sort();
    cells.dedup();
    assert_eq!(cells.len(), 2, "the Parents are distinct");
    for parent in parents {
        let organism = &pedigree.organisms[parent.organism];
        assert!([(4, 5), (5, 5), (6, 5)].contains(&at(organism)));
        assert_eq!(organism.birth_generation, Some(0));
        assert_eq!(organism.depth, 1);
        assert_eq!(organism.origin, Origin::Placed);
    }
}

/// A lone RED/GREEN Parent at (5, 5) under B1/S: each of its Neighbours is
/// Born from it alone, its first Allele from the Parent and its second from
/// the Allele Pool.
#[test]
fn a_b1_birth_has_one_parent_whose_passed_allele_it_carries_and_one_allele_from_the_pool() {
    let mut passed = [false; 2];
    for seed in 0..8 {
        let mut world = without_mutation(
            World::from_genotypes(16, 16, seed, &[RED, GREEN], &[(5, 5, 0, 1)]).unwrap(),
        );
        world.set_rule("B1/S".parse().unwrap());
        world.step();
        let child = world.inspect(4, 4).unwrap().unwrap();
        let pedigree = world.pedigree(3).unwrap();
        let Origin::Born {
            parents,
            from_allele_pool,
        } = &pedigree.organisms[0].origin
        else {
            panic!("the child was Born: {:?}", pedigree.organisms[0].origin);
        };
        assert_eq!(*from_allele_pool, 1);
        let [parent] = parents[..] else {
            panic!("one Parent: {parents:?}");
        };
        let organism = &pedigree.organisms[parent.organism];
        assert_eq!(
            (at(organism), organism.origin.clone()),
            ((5, 5), Origin::Placed)
        );
        assert_eq!(child.alleles[0].allele, [RED, GREEN][parent.allele]);
        passed[parent.allele] = true;
    }
    assert_eq!(
        passed,
        [true, true],
        "the Parent passed on each of its Alleles"
    );
}

#[test]
fn a_b0_birth_has_no_parents_and_both_alleles_from_the_pool() {
    let mut world = World::from_genotypes(16, 16, 3, &[RED], &[(2, 2, 0, 0)]).unwrap();
    world.set_rule("B0/S".parse().unwrap());
    world.step();
    world.inspect(10, 10).unwrap().unwrap();
    let pedigree = world.pedigree(3).unwrap();
    assert_eq!(pedigree.organisms.len(), 1);
    assert_eq!(
        pedigree.organisms[0].origin,
        Origin::Born {
            parents: Vec::new(),
            from_allele_pool: 2
        }
    );
}

/// Six distinct Alleles, so every Allele a child carries names the Parent
/// Allele it came from.
fn hues(n: usize) -> Vec<Allele> {
    (0..n)
        .map(|i| Allele {
            hue: i as f64 * 50.0,
            dominance: 1,
        })
        .collect()
}

#[test]
fn each_of_two_parents_passed_on_the_allele_its_pedigree_records() {
    let alleles = hues(6);
    for seed in 0..12 {
        let live = [(4, 5, 0, 1), (5, 5, 2, 3), (6, 5, 4, 5)];
        let mut world =
            without_mutation(World::from_genotypes(16, 16, seed, &alleles, &live).unwrap());
        world.step();
        let child = world.inspect(5, 4).unwrap().unwrap();
        let pedigree = world.pedigree(1).unwrap();
        let Origin::Born { parents, .. } = &pedigree.organisms[0].origin else {
            panic!("the child was Born");
        };
        let mut passed: Vec<f64> = parents
            .iter()
            .map(|parent| {
                let x = pedigree.organisms[parent.organism].x;
                alleles[(x - 4) * 2 + parent.allele].hue
            })
            .collect();
        let mut carried: Vec<f64> = child.alleles.iter().map(|a| a.allele.hue).collect();
        passed.sort_by(f64::total_cmp);
        carried.sort_by(f64::total_cmp);
        assert_eq!(passed, carried, "seed {seed}");
    }
}

#[test]
fn a_shared_ancestor_appears_once_and_depth_counts_births_back() {
    let mut world = blinker();
    world.run(9);
    world.inspect(5, 4).unwrap().unwrap();
    let pedigree = world.pedigree(6).unwrap();
    let mut identities: Vec<_> = pedigree
        .organisms
        .iter()
        .map(|o| (o.x, o.y, o.birth_generation))
        .collect();
    let count = identities.len();
    identities.sort();
    identities.dedup();
    assert_eq!(identities.len(), count, "no Organism appears twice");

    let mut links = 0;
    for organism in &pedigree.organisms {
        if let Origin::Born { parents, .. } = &organism.origin {
            for parent in parents {
                links += 1;
                let parent = &pedigree.organisms[parent.organism];
                assert!(parent.depth <= organism.depth + 1);
                assert!(parent.birth_generation < organism.birth_generation);
            }
        }
    }
    assert!(
        links >= count,
        "some ancestor is shared by several descendants"
    );
    let middle = pedigree
        .organisms
        .iter()
        .find(|o| at(o) == (5, 5))
        .expect("the middle Organism is an ancestor");
    assert_eq!(
        (middle.birth_generation, middle.origin.clone()),
        (Some(0), Origin::Placed)
    );
    for organism in &pedigree.organisms {
        assert!(organism.depth <= 6);
        assert_eq!(
            organism.origin == Origin::Unexplored,
            organism.depth == 6 && organism.birth_generation != Some(0),
            "{organism:?}"
        );
    }
}

#[test]
fn organisms_painted_in_a_later_generation_are_placed_parents_from_that_generation() {
    let mut world = blinker();
    world.run(4);
    for x in 10..13 {
        world.paint(x, 10, [RED, RED]).unwrap();
    }
    world.end_stroke();
    world.step();
    world.inspect(11, 9).unwrap().unwrap();
    let pedigree = world.pedigree(3).unwrap();
    assert_eq!(pedigree.organisms.len(), 3);
    for parent in &pedigree.organisms[1..] {
        assert_eq!(
            (parent.y, parent.birth_generation, parent.origin.clone()),
            (10, Some(4), Origin::Placed)
        );
    }
}

#[test]
fn a_painting_undone_before_the_next_generation_leaves_the_born_organism_in_the_pedigree() {
    let mut undone_cell_was_a_parent = false;
    for seed in 0..8 {
        let mut world = World::from_cells(16, 16, seed, &[(4, 5), (5, 5), (6, 5)]).unwrap();
        world.step();
        world.paint(5, 4, [RED, RED]).unwrap();
        world.end_stroke();
        world.undo();
        world.step();
        world.inspect(4, 5).unwrap().unwrap();
        let pedigree = world.pedigree(1).unwrap();
        for organism in &pedigree.organisms[1..] {
            let expected = if at(organism) == (5, 5) {
                (Some(0), Origin::Placed)
            } else {
                (Some(1), Origin::Unexplored)
            };
            assert_eq!(
                (organism.birth_generation, organism.origin.clone()),
                expected
            );
            undone_cell_was_a_parent |= at(organism) == (5, 4);
        }
    }
    assert!(undone_cell_was_a_parent);
}

#[test]
fn an_organism_painted_over_in_its_birth_generation_keeps_its_parents() {
    let mut world = blinker();
    world.step();
    world.inspect(5, 4).unwrap().unwrap();
    world.paint(5, 4, [RED, GREEN]).unwrap();
    world.end_stroke();
    world.step();
    let pedigree = world.pedigree(1).unwrap();
    assert!(matches!(
        &pedigree.organisms[0].origin,
        Origin::Born { parents, .. } if parents.len() == 2
    ));

    world.inspect(5, 4).unwrap();
    assert_eq!(world.pedigree(1), None, "the painted Organism has died");
}

#[test]
fn an_organism_painted_in_the_current_generation_is_placed() {
    let mut world = blinker();
    world.run(3);
    world.paint(10, 10, [RED, GREEN]).unwrap();
    world.inspect(10, 10).unwrap().unwrap();
    let pedigree = world.pedigree(3).unwrap();
    assert_eq!(pedigree.organisms.len(), 1);
    assert_eq!(
        (
            pedigree.organisms[0].birth_generation,
            pedigree.organisms[0].origin.clone()
        ),
        (Some(3), Origin::Placed)
    );
}

#[test]
fn pedigrees_start_fresh_after_a_snapshot_import() {
    let mut original = blinker();
    original.run(5);
    let mut world = World::from_snapshot(&original.snapshot()).unwrap();
    world.inspect(5, 4).unwrap().unwrap();
    let imported = world.pedigree(3).unwrap();
    assert_eq!(imported.organisms.len(), 1);
    assert_eq!(
        (
            imported.organisms[0].birth_generation,
            imported.organisms[0].origin.clone()
        ),
        (Some(5), Origin::BeyondRecordedHistory)
    );

    world.step();
    world.inspect(4, 5).unwrap().unwrap();
    let pedigree = world.pedigree(3).unwrap();
    assert!(
        matches!(&pedigree.organisms[0].origin, Origin::Born { parents, .. } if parents.len() == 2)
    );
    for parent in &pedigree.organisms[1..] {
        assert_eq!(parent.origin, Origin::BeyondRecordedHistory);
        let still_live = (at(parent) == (5, 5)).then_some(0);
        assert_eq!(parent.birth_generation, still_live, "{parent:?}");
    }
}

#[test]
fn the_pedigree_budget_defaults_to_its_maximum_of_200_mb_and_cannot_exceed_it() {
    let mut world = blinker();
    assert_eq!(MAX_PEDIGREE_BUDGET, 200_000_000);
    assert_eq!(world.pedigree_budget(), MAX_PEDIGREE_BUDGET);
    world.set_pedigree_budget(50_000_000).unwrap();
    assert_eq!(world.pedigree_budget(), 50_000_000);
    assert_eq!(
        world.set_pedigree_budget(MAX_PEDIGREE_BUDGET + 1),
        Err(WorldError::PedigreeBudgetTooLarge(MAX_PEDIGREE_BUDGET + 1))
    );
    assert_eq!(world.pedigree_budget(), 50_000_000);
}

/// The origins of the Organisms whose Parents are not in `pedigree`.
fn leaves(pedigree: &Pedigree) -> Vec<Origin> {
    pedigree
        .organisms
        .iter()
        .filter(|o| !matches!(o.origin, Origin::Born { .. }))
        .map(|o| o.origin.clone())
        .collect()
}

#[test]
fn over_budget_the_oldest_records_are_dropped_first() {
    let mut full = blinker();
    full.run(301);
    full.inspect(5, 4).unwrap().unwrap();
    let pedigree = full.pedigree(1000).unwrap();
    assert!(leaves(&pedigree).iter().all(|o| *o == Origin::Placed));

    let mut small = blinker();
    small.set_pedigree_budget(2_000).unwrap();
    small.run(301);
    small.inspect(5, 4).unwrap().unwrap();
    let pedigree = small.pedigree(1000).unwrap();
    let leaves = leaves(&pedigree);
    assert!(!leaves.is_empty());
    assert!(leaves.iter().all(|o| *o == Origin::BeyondRecordedHistory));
    let recent = pedigree
        .organisms
        .iter()
        .filter(|o| o.birth_generation.is_some_and(|g| g > 290))
        .count();
    assert!(recent > 5, "the latest Generations are still recorded");
    assert!(small.pedigree_bytes() <= 2_000);
}

#[test]
fn lowering_the_budget_drops_records_at_once_and_zero_keeps_none() {
    let mut world = World::soup(64, 64, 0.3, 5).unwrap();
    world.run(20);
    assert!(world.pedigree_bytes() > 10_000);
    world.set_pedigree_budget(10_000).unwrap();
    assert!(world.pedigree_bytes() <= 10_000);
    world.set_pedigree_budget(0).unwrap();
    world.step();
    assert_eq!(world.pedigree_bytes(), 0);
    let born = world.live_cells()[0];
    world.inspect(born.0, born.1).unwrap().unwrap();
    let root = &world.pedigree(3).unwrap().organisms[0];
    assert_eq!(root.origin, Origin::BeyondRecordedHistory);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn the_pedigree_never_exceeds_its_memory_budget(
        seed in any::<u64>(),
        size in 16usize..80,
        density in 0.1f64..0.6,
        budgets in prop::collection::vec((0usize..60_000, 1u64..12), 1..6),
    ) {
        let mut world = World::soup(size, size, density, seed).unwrap();
        for (budget, generations) in budgets {
            world.set_pedigree_budget(budget).unwrap();
            prop_assert!(world.pedigree_bytes() <= budget);
            for _ in 0..generations {
                world.paint(0, 0, [RED, GREEN]).unwrap();
                world.step();
                prop_assert!(world.pedigree_bytes() <= budget);
            }
        }
    }
}
