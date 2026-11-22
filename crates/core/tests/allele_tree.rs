//! When each Allele in the Allele Tree was last carried by a live Cell.

use genetic_life_core::{Allele, AlleleNode, AlleleStatus, MutationSettings, World};

const ORANGE: Allele = Allele {
    hue: 40.0,
    dominance: 2,
};
const JADE: Allele = Allele {
    hue: 160.0,
    dominance: 1,
};

fn node(world: &World, allele: Allele) -> AlleleNode {
    *world
        .allele_tree()
        .iter()
        .find(|node| node.allele == allele)
        .expect("the World retains the Allele")
}

/// An orange block, which lives on, and a lone jade Cell, which dies in the
/// first Generation.
fn block_and_lone_cell() -> World {
    World::from_genotypes(
        16,
        16,
        1,
        &[ORANGE, JADE],
        &[
            (4, 4, 0, 0),
            (5, 4, 0, 0),
            (4, 5, 0, 0),
            (5, 5, 0, 0),
            (12, 12, 1, 1),
        ],
    )
    .unwrap()
}

#[test]
fn a_carried_allele_was_last_carried_in_the_current_generation() {
    let mut world = block_and_lone_cell();
    world.run(5);
    assert_eq!(node(&world, ORANGE).last_carried, 5);
}

#[test]
fn an_extinct_allele_keeps_the_last_generation_it_was_carried_in() {
    let mut world = block_and_lone_cell();
    world.run(5);
    let jade = node(&world, JADE);
    assert_eq!(jade.status, AlleleStatus::Extinct);
    assert_eq!(jade.last_carried, 0);

    world.paint(12, 12, [JADE, JADE]).unwrap();
    world.end_stroke();
    assert_eq!(node(&world, JADE).last_carried, 5);
    world.run(3);
    assert_eq!(node(&world, JADE).last_carried, 5);
}

#[test]
fn an_extinct_ancestor_of_a_living_allele_keeps_when_it_was_last_carried() {
    let blinker = [(4, 5, 0, 0), (5, 5, 0, 0), (6, 5, 0, 0)];
    let mut world = World::from_genotypes(16, 16, 1, &[JADE], &blinker).unwrap();
    world.set_mutation(MutationSettings::new(1.0, 30.0).unwrap());
    world.run(2);
    let ancestors: Vec<_> = world
        .allele_tree()
        .into_iter()
        .filter(|node| node.status == AlleleStatus::Extinct)
        .collect();
    assert!(
        !ancestors.is_empty(),
        "the Cells Born in generation 2 carry mutants of the generation 1 mutants, which died out with their Cells"
    );
    for ancestor in ancestors {
        assert_eq!((ancestor.origin, ancestor.last_carried), (1, 1));
    }
}

#[test]
fn when_an_extinct_allele_was_last_carried_survives_a_snapshot() {
    let mut world = block_and_lone_cell();
    world.run(5);
    world.paint(12, 12, [JADE, JADE]).unwrap();
    world.end_stroke();
    world.run(3);
    let imported = World::from_snapshot(&world.snapshot()).unwrap();
    assert_eq!(node(&imported, JADE).last_carried, 5);
    assert_eq!(imported, world);
}
