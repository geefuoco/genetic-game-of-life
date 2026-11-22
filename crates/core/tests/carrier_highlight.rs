//! The Carrier highlight: every live Cell carrying an Allele, telling Cells
//! that show it from Carriers that hide it.

use genetic_life_core::{Allele, AlleleId, CarryingCell, MutationSettings, World};

const ORANGE: Allele = Allele {
    hue: 40.0,
    dominance: 2,
};
const JADE: Allele = Allele {
    hue: 160.0,
    dominance: 1,
};
const SKY: Allele = Allele {
    hue: 230.0,
    dominance: 1,
};

/// The id of the Founding Allele `allele` in `world`.
fn id_of(world: &World, allele: Allele) -> AlleleId {
    world
        .allele_tree()
        .iter()
        .find(|node| node.allele == allele)
        .expect("the World retains the Allele")
        .id
}

fn cell(x: usize, y: usize, shown: bool) -> CarryingCell {
    CarryingCell { x, y, shown }
}

/// Isolated Cells: pure orange, an orange Carrier hiding jade, a jade and
/// sky blend, and pure sky.
fn mixed() -> World {
    World::from_genotypes(
        16,
        16,
        1,
        &[ORANGE, JADE, SKY],
        &[(1, 1, 0, 0), (5, 1, 0, 1), (9, 1, 1, 2), (13, 1, 2, 2)],
    )
    .unwrap()
}

#[test]
fn the_highlight_tells_cells_that_show_an_allele_from_carriers_that_hide_it() {
    let world = mixed();
    assert_eq!(
        world.carrying_cells(id_of(&world, JADE)),
        Some(vec![cell(5, 1, false), cell(9, 1, true)])
    );
    assert_eq!(
        world.carrying_cells(id_of(&world, ORANGE)),
        Some(vec![cell(1, 1, true), cell(5, 1, true)])
    );
    assert_eq!(
        world.carrying_cells(id_of(&world, SKY)),
        Some(vec![cell(9, 1, true), cell(13, 1, true)])
    );
}

#[test]
fn an_extinct_allele_is_carried_by_no_cell() {
    let mut world = mixed();
    world.erase(5, 1).unwrap();
    world.erase(9, 1).unwrap();
    world.end_stroke();
    assert_eq!(world.carrying_cells(id_of(&world, JADE)), Some(vec![]));
}

#[test]
fn a_pruned_allele_is_not_found() {
    let trio = [(4, 4, 0, 0), (5, 4, 0, 0), (4, 5, 0, 0)];
    let mut world = World::from_genotypes(16, 16, 1, &[JADE], &trio).unwrap();
    world.set_mutation(MutationSettings::new(1.0, 30.0).unwrap());
    world.step();
    let mutant = world.inspect(5, 5).unwrap().unwrap().alleles[0].id;
    assert_eq!(
        world.carrying_cells(mutant).map(|cells| cells.len()),
        Some(1)
    );

    for (x, y) in world.live_cells() {
        world.erase(x, y).unwrap();
    }
    world.step();
    assert_eq!(world.carrying_cells(mutant), None);
}

#[test]
fn the_highlight_follows_the_run() {
    let block = [(4, 4, 0, 1), (5, 4, 0, 1), (4, 5, 0, 1), (5, 5, 0, 1)];
    let lone = [(12, 12, 1, 1)];
    let mut world =
        World::from_genotypes(16, 16, 1, &[ORANGE, JADE], &[&block[..], &lone].concat()).unwrap();
    let jade = id_of(&world, JADE);
    assert_eq!(world.carrying_cells(jade).unwrap().len(), 5);
    world.step();
    assert_eq!(
        world.carrying_cells(jade),
        Some(vec![
            cell(4, 4, false),
            cell(5, 4, false),
            cell(4, 5, false),
            cell(5, 5, false),
        ])
    );
}
