use genetic_life_core::World;

fn world_with(width: usize, height: usize, cells: &[(usize, usize)]) -> World {
    World::from_cells(width, height, 1, cells).unwrap()
}

#[test]
fn block_stays_still() {
    let block = [(5, 5), (6, 5), (5, 6), (6, 6)];
    let mut world = world_with(16, 16, &block);
    world.step();
    assert_eq!(world.live_cells(), block);
    assert_eq!(world.generation(), 1);
}

#[test]
fn blinker_repeats_every_two_generations() {
    let horizontal = [(4, 5), (5, 5), (6, 5)];
    let vertical = [(5, 4), (5, 5), (5, 6)];
    let mut world = world_with(16, 16, &horizontal);
    world.step();
    assert_eq!(world.live_cells(), vertical);
    world.step();
    assert_eq!(world.live_cells(), horizontal);
}

const GLIDER: [(usize, usize); 5] = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];

#[test]
fn glider_moves_one_cell_diagonally_every_four_generations() {
    let mut world = world_with(16, 16, &GLIDER);
    world.run(4);
    let mut moved = GLIDER.map(|(x, y)| (x + 1, y + 1));
    moved.sort_by_key(|&(x, y)| (y, x));
    assert_eq!(world.live_cells(), moved);
}

#[test]
fn glider_wraps_around_the_torus() {
    let mut world = world_with(16, 16, &[(15, 14), (0, 15), (14, 0), (15, 0), (0, 0)]);
    world.run(4);
    let mut expected = vec![(0, 15), (1, 0), (15, 1), (0, 1), (1, 1)];
    expected.sort_by_key(|&(x, y)| (y, x));
    assert_eq!(world.live_cells(), expected);
}

#[test]
fn glider_returns_to_start_after_64_generations_on_16x16() {
    let mut world = world_with(16, 16, &GLIDER);
    world.run(64);
    assert_eq!(world.live_cells(), [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)]);
    assert_eq!(world.generation(), 64);
}

#[test]
fn sizes_outside_16_to_1024_are_rejected() {
    assert!(World::soup(15, 16, 0.5, 1).is_err());
    assert!(World::soup(16, 1025, 0.5, 1).is_err());
    assert!(World::soup(16, 16, 0.5, 1).is_ok());
    assert!(World::soup(1024, 1024, 0.0, 1).is_ok());
}

#[test]
fn density_outside_unit_interval_is_rejected() {
    assert!(World::soup(16, 16, -0.1, 1).is_err());
    assert!(World::soup(16, 16, 1.1, 1).is_err());
}

#[test]
fn density_extremes_fill_none_or_all_cells() {
    assert_eq!(World::soup(16, 20, 0.0, 9).unwrap().population(), 0);
    assert_eq!(World::soup(16, 20, 1.0, 9).unwrap().population(), 320);
}

#[test]
fn live_cells_outside_the_world_are_rejected() {
    assert!(World::from_cells(16, 16, 1, &[(16, 0)]).is_err());
}

mod determinism {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn same_seed_and_settings_give_identical_runs(
            seed in any::<u64>(),
            width in 16usize..40,
            height in 16usize..40,
            density in 0.0f64..=1.0,
            generations in 0u64..20,
        ) {
            let mut a = World::soup(width, height, density, seed).unwrap();
            let mut b = World::soup(width, height, density, seed).unwrap();
            a.run(generations);
            b.run(generations);
            prop_assert_eq!(a, b);
        }
    }
}

#[test]
fn rng_version_1_sequence_is_pinned() {
    let mut rng = genetic_life_core::Rng::new(0);
    assert_eq!(genetic_life_core::Rng::VERSION, 1);
    assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
}

#[test]
fn different_seeds_give_different_soups() {
    let a = World::soup(32, 32, 0.5, 1).unwrap();
    let b = World::soup(32, 32, 0.5, 2).unwrap();
    assert_ne!(a.live_cells(), b.live_cells());
}
