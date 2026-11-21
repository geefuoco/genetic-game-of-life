//! Editing a World: painting, erasing and stamping Patterns, in undoable
//! strokes, with brush Alleles entering the Allele Tree as Founding Alleles.

use genetic_life_core::{Allele, AlleleNode, MutationSettings, Pattern, World, DEAD_PHENOTYPE};

const ORANGE: Allele = Allele {
    hue: 40.0,
    dominance: 2,
};

fn empty(size: usize) -> World {
    World::from_cells(size, size, 1, &[]).unwrap()
}

fn phenotype_at(world: &World, x: usize, y: usize) -> f64 {
    world.phenotypes()[y * world.width() + x]
}

#[test]
fn painting_a_cell_makes_it_live_showing_the_brush_hue() {
    let mut world = empty(16);
    world.paint(3, 4, [ORANGE, ORANGE]).unwrap();
    assert_eq!(world.live_cells(), [(3, 4)]);
    assert_eq!(phenotype_at(&world, 3, 4), 40.0);
    assert_eq!(phenotype_at(&world, 4, 4), DEAD_PHENOTYPE);
}

/// The Allele [`World::from_cells`] gives every live Cell.
const DEFAULT: Allele = Allele {
    hue: 0.0,
    dominance: 0,
};

/// Founding Alleles other than the default one, which in these Worlds can
/// only have come from a brush.
fn brush_alleles(world: &World) -> Vec<AlleleNode> {
    world
        .allele_tree()
        .into_iter()
        .filter(|node| node.source.is_none() && node.allele != DEFAULT)
        .collect()
}

#[test]
fn brush_alleles_are_founding_alleles_originating_in_the_current_generation() {
    let mut world = empty(16);
    world.run(7);
    world.paint(3, 4, [ORANGE, ORANGE]).unwrap();
    let painted = brush_alleles(&world);
    assert_eq!(painted.len(), 1);
    assert_eq!(painted[0].allele, ORANGE);
    assert_eq!(painted[0].origin, 7);
}

#[test]
fn a_brush_allele_is_reused_for_the_same_hue_and_dominance_level() {
    let mut world = empty(16);
    world.run(2);
    world.paint(3, 4, [ORANGE, ORANGE]).unwrap();
    world.run(3);
    world.paint(8, 8, [ORANGE, ORANGE]).unwrap();
    let weaker = Allele {
        dominance: 1,
        ..ORANGE
    };
    world.paint(9, 8, [weaker, weaker]).unwrap();
    let painted = brush_alleles(&world);
    assert_eq!(painted.len(), 2);
    assert_eq!((painted[0].allele, painted[0].origin), (ORANGE, 2));
    assert_eq!((painted[1].allele, painted[1].origin), (weaker, 5));
}

const BLUE: Allele = Allele {
    hue: 230.0,
    dominance: 0,
};

#[test]
fn painting_two_different_alleles_makes_a_carrier() {
    let mut world = empty(16);
    world.paint(3, 4, [ORANGE, BLUE]).unwrap();
    assert_eq!(phenotype_at(&world, 3, 4), ORANGE.hue);
    let census = world.census();
    let blue = census.alleles.iter().find(|a| a.allele == BLUE).unwrap();
    assert_eq!((blue.copies, blue.shown, blue.hidden), (1, 0, 1));
}

#[test]
fn erasing_a_cell_makes_it_dead() {
    let mut world = World::from_cells(16, 16, 1, &[(2, 2), (3, 2)]).unwrap();
    world.erase(2, 2).unwrap();
    assert_eq!(world.live_cells(), [(3, 2)]);
    assert_eq!(world.census().population_history, [1]);
}

#[test]
fn editing_outside_the_world_or_with_an_invalid_allele_fails_and_changes_nothing() {
    let mut world = World::from_cells(16, 16, 1, &[(2, 2)]).unwrap();
    let before = world.clone();
    assert!(world.paint(16, 0, [ORANGE, ORANGE]).is_err());
    assert!(world.erase(0, 16).is_err());
    let invalid = Allele {
        hue: 360.0,
        dominance: 0,
    };
    assert!(world.paint(1, 1, [ORANGE, invalid]).is_err());
    assert_eq!(world, before);
}

#[test]
fn undo_reverts_whole_strokes_and_redo_reapplies_them() {
    let mut world = World::from_cells(16, 16, 1, &[(2, 2), (3, 2)]).unwrap();
    let original = world.clone();
    world.paint(5, 5, [ORANGE, ORANGE]).unwrap();
    world.paint(6, 5, [ORANGE, BLUE]).unwrap();
    world.end_stroke();
    let painted = world.clone();
    world.erase(2, 2).unwrap();
    world.paint(5, 5, [BLUE, BLUE]).unwrap();
    let edited = world.clone();

    assert!(world.undo());
    assert_eq!(world, painted);
    assert!(world.undo());
    assert_eq!(world, original);
    assert!(brush_alleles(&world).is_empty());
    assert!(!world.undo());

    assert!(world.redo());
    assert_eq!(world, painted);
    assert!(world.redo());
    assert_eq!(world, edited);
    assert!(!world.redo());
}

#[test]
fn a_new_edit_after_undo_discards_the_strokes_that_could_be_redone() {
    let mut world = empty(16);
    world.paint(5, 5, [ORANGE, ORANGE]).unwrap();
    world.end_stroke();
    assert!(world.undo());
    assert!(world.can_redo());
    world.paint(6, 6, [BLUE, BLUE]).unwrap();
    assert!(!world.can_redo());
    assert!(!world.redo());
    assert_eq!(world.live_cells(), [(6, 6)]);
}

#[test]
fn an_edit_that_changes_nothing_is_not_a_stroke() {
    let mut world = World::from_cells(16, 16, 1, &[(2, 2)]).unwrap();
    world.erase(9, 9).unwrap();
    world.end_stroke();
    assert!(!world.can_undo());
}

#[test]
fn advancing_a_generation_clears_the_edit_history() {
    let mut world = empty(16);
    world.paint(5, 5, [ORANGE, ORANGE]).unwrap();
    world.end_stroke();
    world.paint(9, 9, [ORANGE, ORANGE]).unwrap();
    world.end_stroke();
    assert!(world.undo());
    assert!(world.can_undo() && world.can_redo());
    world.step();
    assert!(!world.can_undo() && !world.can_redo());
    assert!(!world.undo());
}

fn sorted(mut cells: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    cells.sort_by_key(|&(x, y)| (y, x));
    cells
}

#[test]
fn the_pattern_library_names_its_patterns() {
    let names: Vec<_> = Pattern::ALL.iter().map(|p| p.name()).collect();
    assert_eq!(names, ["glider", "lwss", "r-pentomino", "gosper-gun"]);
    for pattern in Pattern::ALL {
        assert_eq!(Pattern::named(pattern.name()), Some(pattern));
    }
    assert_eq!(Pattern::named("puffer"), None);
}

#[test]
fn a_stamped_pattern_is_centred_on_the_position_in_the_brush_alleles() {
    let mut world = empty(16);
    world
        .stamp(Pattern::RPentomino, 8, 8, [ORANGE, ORANGE])
        .unwrap();
    assert_eq!(world.live_cells(), [(8, 7), (9, 7), (7, 8), (8, 8), (8, 9)]);
    assert!(world
        .live_cells()
        .iter()
        .all(|&(x, y)| phenotype_at(&world, x, y) == ORANGE.hue));
}

#[test]
fn a_stamped_pattern_replaces_the_cells_it_covers_and_wraps_across_edges() {
    let inside_but_dead_in_glider = (0, 0);
    let outside = (5, 5);
    let mut world = World::from_cells(16, 16, 1, &[inside_but_dead_in_glider, outside]).unwrap();
    world
        .stamp(Pattern::Glider, 0, 0, [ORANGE, ORANGE])
        .unwrap();
    let glider = [(0, 15), (1, 0), (15, 1), (0, 1), (1, 1)];
    let mut expected = glider.to_vec();
    expected.push(outside);
    assert_eq!(world.live_cells(), sorted(expected));
}

#[test]
fn a_stamped_glider_moves_one_cell_diagonally_every_four_generations() {
    let mut world = empty(16);
    world
        .stamp(Pattern::Glider, 8, 8, [ORANGE, ORANGE])
        .unwrap();
    let start = world.live_cells();
    world.run(4);
    let moved = start.iter().map(|&(x, y)| (x + 1, y + 1)).collect();
    assert_eq!(world.live_cells(), sorted(moved));
}

#[test]
fn a_stamped_lwss_moves_two_cells_every_four_generations() {
    let mut world = empty(32);
    world
        .stamp(Pattern::Lwss, 16, 16, [ORANGE, ORANGE])
        .unwrap();
    let start = world.live_cells();
    assert_eq!(start.len(), 9);
    world.run(4);
    let left: Vec<_> = start.iter().map(|&(x, y)| (x - 2, y)).collect();
    let right: Vec<_> = start.iter().map(|&(x, y)| (x + 2, y)).collect();
    let now = world.live_cells();
    assert!(now == sorted(left) || now == sorted(right), "{now:?}");
}

/// Once its first glider is free, the gun returns to the same state every
/// 30 Generations with one more glider of 5 Cells.
#[test]
fn a_stamped_gosper_gun_adds_a_glider_every_thirty_generations() {
    let mut world = empty(128);
    world
        .stamp(Pattern::GosperGun, 40, 40, [ORANGE, ORANGE])
        .unwrap();
    assert_eq!(world.population(), 36);
    world.run(60);
    let mut populations = vec![world.population()];
    for _ in 0..3 {
        world.run(30);
        populations.push(world.population());
    }
    let growth: Vec<_> = populations.windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(growth, [5, 5, 5]);
}

#[test]
fn a_stamp_is_one_stroke() {
    let mut world = World::from_cells(16, 16, 1, &[(7, 7)]).unwrap();
    let before = world.clone();
    world.stamp(Pattern::Glider, 8, 8, [ORANGE, BLUE]).unwrap();
    world.end_stroke();
    assert!(world.undo());
    assert_eq!(world, before);
    assert!(!world.can_undo());
}

/// Under B0/S every Cell of an empty World is born from two Allele Pool
/// draws, which fall back to the Founding Alleles: the default Allele and
/// the brush Allele left behind by an erased painting.
#[test]
fn an_empty_world_draws_brush_alleles_as_founding_alleles() {
    let mut world = empty(16);
    world.set_mutation(MutationSettings::new(0.0, 0.0).unwrap());
    world.paint(3, 3, [ORANGE, ORANGE]).unwrap();
    world.erase(3, 3).unwrap();
    world.set_rule("B0/S".parse().unwrap());
    world.step();
    let census = world.census();
    let orange = census.alleles.iter().find(|a| a.allele == ORANGE).unwrap();
    assert!(orange.copies > 0);
    assert!(orange.copies < 2 * world.population());
}

#[test]
fn a_pattern_larger_than_the_world_is_refused_and_changes_nothing() {
    let mut world = World::from_cells(30, 30, 1, &[(2, 2)]).unwrap();
    let before = world.clone();
    let error = world
        .stamp(Pattern::GosperGun, 15, 15, [ORANGE, ORANGE])
        .unwrap_err();
    assert!(error.to_string().contains("36x9"), "{error}");
    assert_eq!(world, before);
    assert!(!world.can_undo());
}
