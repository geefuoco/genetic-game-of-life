//! Inspecting an Organism: its Genotype, Phenotype and Birth Generation, and
//! the Generation it died in, followed through death.

use genetic_life_core::{Allele, AlleleStatus, MutationSettings, World};

const ORANGE: Allele = Allele {
    hue: 40.0,
    dominance: 2,
};
const JADE: Allele = Allele {
    hue: 160.0,
    dominance: 1,
};

#[test]
fn inspecting_a_carrier_shows_its_genotype_phenotype_and_which_allele_is_hidden() {
    let mut world = World::from_genotypes(16, 16, 1, &[ORANGE, JADE], &[(3, 4, 1, 0)]).unwrap();
    let inspection = world.inspect(3, 4).unwrap().unwrap();
    assert_eq!((inspection.x, inspection.y), (3, 4));
    assert_eq!(inspection.alleles[0].allele, JADE);
    assert!(!inspection.alleles[0].shown);
    assert_eq!(inspection.alleles[1].allele, ORANGE);
    assert!(inspection.alleles[1].shown);
    assert_eq!(inspection.phenotype, 40.0);
    assert_eq!(inspection.birth_generation, 0);
    assert_eq!(inspection.death_generation, None);
}

/// A horizontal blinker around (5, 5). The Organisms at (5, 4) and (5, 6)
/// are Born in odd Generations and die in even ones; those at (4, 5) and
/// (6, 5) die in odd Generations and are Born in even ones.
fn blinker() -> World {
    World::from_cells(16, 16, 1, &[(4, 5), (5, 5), (6, 5)]).unwrap()
}

#[test]
fn an_organism_born_in_a_generation_has_that_birth_generation() {
    let mut world = blinker();
    world.run(3);
    let top = world.inspect(5, 4).unwrap().unwrap();
    assert_eq!(top.birth_generation, 3);
    let middle = world.inspect(5, 5).unwrap().unwrap();
    assert_eq!(middle.birth_generation, 0);
}

#[test]
fn an_inspected_organisms_death_is_reported_and_a_later_organism_in_its_cell_is_not_it() {
    let mut world = blinker();
    world.step();
    world.inspect(5, 4).unwrap().unwrap();
    world.step();
    let died = world.inspection().unwrap();
    assert_eq!((died.x, died.y), (5, 4));
    assert_eq!(died.birth_generation, 1);
    assert_eq!(died.death_generation, Some(2));

    world.step();
    assert!(
        world.phenotypes()[4 * 16 + 5] >= 0.0,
        "a new Organism lives at (5, 4)"
    );
    let still = world.inspection().unwrap();
    assert_eq!(still.birth_generation, 1);
    assert_eq!(still.death_generation, Some(2));
}

#[test]
fn a_painted_organism_is_born_in_the_generation_it_was_painted_in() {
    let mut world = blinker();
    world.run(4);
    world.paint(10, 10, [ORANGE, JADE]).unwrap();
    let painted = world.inspect(10, 10).unwrap().unwrap();
    assert_eq!(painted.birth_generation, 4);
    assert_eq!(painted.alleles.map(|a| a.allele), [ORANGE, JADE]);
}

#[test]
fn erasing_the_inspected_organism_kills_it_in_the_current_generation_and_undo_revives_it() {
    let mut world = blinker();
    world.run(2);
    world.inspect(5, 5).unwrap().unwrap();
    world.erase(5, 5).unwrap();
    world.end_stroke();
    assert_eq!(world.inspection().unwrap().death_generation, Some(2));
    world.undo();
    assert_eq!(world.inspection().unwrap().death_generation, None);
    world.redo();
    assert_eq!(world.inspection().unwrap().death_generation, Some(2));
}

#[test]
fn painting_over_the_inspected_organism_replaces_it_even_in_the_generation_it_was_painted_in() {
    let mut world = blinker();
    world.paint(10, 10, [ORANGE, ORANGE]).unwrap();
    world.inspect(10, 10).unwrap().unwrap();
    world.paint(10, 10, [JADE, JADE]).unwrap();
    let replaced = world.inspection().unwrap();
    assert_eq!(replaced.death_generation, Some(0));
    assert_eq!(replaced.alleles.map(|a| a.allele), [ORANGE, ORANGE]);
}

#[test]
fn each_allele_lineage_runs_back_to_its_founding_allele_and_outlasts_the_organism() {
    let trio = [(4, 4, 0, 0), (5, 4, 0, 0), (4, 5, 0, 0)];
    let mut world = World::from_genotypes(16, 16, 1, &[JADE], &trio).unwrap();
    world.set_mutation(MutationSettings::new(1.0, 30.0).unwrap());
    world.step();
    let child = world.inspect(5, 5).unwrap().unwrap();
    let founding = world.allele_tree()[0];
    for allele in &child.alleles {
        let chain: Vec<_> = allele
            .lineage
            .iter()
            .map(|node| (node.id, node.origin))
            .collect();
        assert_eq!(chain, [(allele.id, 1), (founding.id, 0)]);
        assert_ne!(allele.lineage[0].status, AlleleStatus::Extinct);
        assert_eq!(allele.lineage[1].status, AlleleStatus::Living);
    }

    for (x, y) in world.live_cells() {
        world.erase(x, y).unwrap();
    }
    world.step();
    let dead = world.inspection().unwrap();
    assert!(dead.alleles.iter().all(|allele| allele.lineage.len() == 2));
    for allele in &dead.alleles {
        assert!(allele
            .lineage
            .iter()
            .all(|node| node.status == AlleleStatus::Extinct));
    }
}

#[test]
fn inspecting_a_dead_cell_finds_nothing_and_stops_following_and_outside_the_world_fails() {
    let mut world = blinker();
    world.inspect(5, 5).unwrap().unwrap();
    assert_eq!(world.inspect(0, 0).unwrap(), None);
    assert_eq!(world.inspection(), None);
    assert!(world.inspect(16, 0).is_err());
}

#[test]
fn stopping_inspection_follows_nothing() {
    let mut world = blinker();
    world.inspect(5, 5).unwrap().unwrap();
    world.stop_inspecting();
    assert_eq!(world.inspection(), None);
}
