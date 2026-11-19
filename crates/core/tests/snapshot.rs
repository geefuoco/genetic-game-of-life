//! Snapshots: export a World's full state and import it to resume the run.

use genetic_life_core::{AlleleStatus, MutationSettings, Rule, SnapshotError, World};
use proptest::prelude::*;

fn soup(seed: u64) -> World {
    let mut world = World::soup_with_alleles(20, 18, 0.4, seed, 5).unwrap();
    world.set_rule(Rule::resolve("highlife").unwrap());
    world.set_mutation(MutationSettings::new(0.2, 45.0).unwrap());
    world.run(12);
    world
}

/// B/S notation for the neighbour counts whose bits are set in each mask.
fn notation(birth: u16, survival: u16) -> String {
    let digits = |mask: u16| -> String {
        (0..=8)
            .filter(|n| mask >> n & 1 == 1)
            .map(|n| n.to_string())
            .collect()
    };
    format!("B{}/S{}", digits(birth), digits(survival))
}

/// The Snapshot header: the marker and the little-endian format version.
const HEADER: usize = 8;

/// Byte offset of the PRNG algorithm version in a version 1 payload.
const RNG_VERSION_OFFSET: usize = 36;

fn payload(snapshot: &[u8]) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec_zlib(&snapshot[HEADER..]).unwrap()
}

fn reframe(snapshot: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut bytes = snapshot[..HEADER].to_vec();
    bytes.extend(miniz_oxide::deflate::compress_to_vec_zlib(payload, 6));
    bytes
}

#[test]
fn an_imported_snapshot_is_the_exported_world() {
    let world = soup(9);
    let imported = World::from_snapshot(&world.snapshot()).unwrap();
    assert_eq!(imported, world);
}

#[test]
fn a_snapshot_is_compressed() {
    let world = World::soup(256, 256, 0.0, 1).unwrap();
    assert!(world.snapshot().len() < 1024);
}

#[test]
fn a_snapshot_starts_with_its_marker_and_format_version() {
    let bytes = soup(1).snapshot();
    assert_eq!(&bytes[..4], b"GGLS");
    assert_eq!(bytes[4..HEADER], 1u32.to_le_bytes());
    assert_eq!(genetic_life_core::SNAPSHOT_VERSION, 1);
}

#[test]
fn an_unknown_format_version_is_rejected() {
    let mut bytes = soup(2).snapshot();
    bytes[4..HEADER].copy_from_slice(&2u32.to_le_bytes());
    let error = World::from_snapshot(&bytes).unwrap_err();
    assert_eq!(error, SnapshotError::UnsupportedVersion(2));
    assert!(error.to_string().contains("version 2"), "{error}");
}

#[test]
fn an_unknown_prng_version_is_rejected() {
    let bytes = soup(2).snapshot();
    let mut payload = payload(&bytes);
    payload[RNG_VERSION_OFFSET..RNG_VERSION_OFFSET + 4].copy_from_slice(&99u32.to_le_bytes());
    let error = World::from_snapshot(&reframe(&bytes, &payload)).unwrap_err();
    assert_eq!(error, SnapshotError::UnsupportedRng(99));
    assert!(error.to_string().contains("99"), "{error}");
}

#[test]
fn bytes_that_are_not_a_snapshot_are_rejected() {
    for bytes in [&b""[..], b"GG", b"hello world", b"{\"generation\": 0}"] {
        assert_eq!(
            World::from_snapshot(bytes),
            Err(SnapshotError::NotASnapshot)
        );
    }
}

#[test]
fn every_truncation_of_a_snapshot_is_rejected() {
    let bytes = soup(3).snapshot();
    for len in 0..bytes.len() {
        let error = World::from_snapshot(&bytes[..len]).unwrap_err();
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn a_corrupt_byte_in_the_compressed_payload_is_rejected() {
    let bytes = soup(4).snapshot();
    for i in HEADER..bytes.len() {
        let mut corrupt = bytes.clone();
        corrupt[i] ^= 0xFF;
        let error = World::from_snapshot(&corrupt).unwrap_err();
        assert!(!error.to_string().is_empty(), "byte {i}");
    }
}

#[test]
fn a_payload_with_trailing_bytes_is_rejected() {
    let bytes = soup(5).snapshot();
    let mut payload = payload(&bytes);
    payload.push(0);
    assert_eq!(
        World::from_snapshot(&reframe(&bytes, &payload)),
        Err(SnapshotError::TrailingBytes)
    );
}

#[test]
fn a_truncated_payload_is_rejected() {
    let bytes = soup(5).snapshot();
    let payload = payload(&bytes);
    for len in 0..payload.len() {
        assert_eq!(
            World::from_snapshot(&reframe(&bytes, &payload[..len])),
            Err(SnapshotError::Truncated),
            "payload length {len}"
        );
    }
}

/// One Allele Tree slot: its id version and, if retained, its hue,
/// Dominance Level, source slot and origin Generation.
type SlotSpec = (u32, Option<(f64, u8, Option<u32>, u64)>);

/// A version 1 payload built field by field from the documented layout,
/// independently of the World's own encoder.
#[derive(Clone)]
struct Payload {
    width: u32,
    height: u32,
    generation: u64,
    birth: u16,
    survival: u16,
    rate: f64,
    magnitude: f64,
    rng_version: u32,
    rng_state: u64,
    founding: u32,
    slots: Vec<SlotSpec>,
    free: Vec<u32>,
    cells: Vec<(u32, u32, u32)>,
    history: Vec<u32>,
}

impl Payload {
    /// A 16x16 Conway World at Generation 1 with one Founding Allele, one
    /// mutant of it, one freed slot and two live Cells.
    fn valid() -> Self {
        Self {
            width: 16,
            height: 16,
            generation: 1,
            birth: 1 << 3,
            survival: 1 << 2 | 1 << 3,
            rate: 0.25,
            magnitude: 40.0,
            rng_version: 1,
            rng_state: 42,
            founding: 1,
            slots: vec![
                (0, Some((10.0, 0, None, 0))),
                (3, Some((130.0, 2, Some(0), 1))),
                (1, None),
            ],
            free: vec![2],
            cells: vec![(17, 0, 1), (18, 1, 1)],
            history: vec![5, 2],
        }
    }

    fn snapshot(&self) -> Vec<u8> {
        let mut p = vec![];
        p.extend(self.width.to_le_bytes());
        p.extend(self.height.to_le_bytes());
        p.extend(self.generation.to_le_bytes());
        p.extend(self.birth.to_le_bytes());
        p.extend(self.survival.to_le_bytes());
        p.extend(self.rate.to_le_bytes());
        p.extend(self.magnitude.to_le_bytes());
        p.extend(self.rng_version.to_le_bytes());
        p.extend(self.rng_state.to_le_bytes());
        p.extend(self.founding.to_le_bytes());
        p.extend((self.slots.len() as u32).to_le_bytes());
        for (index, (version, record)) in self.slots.iter().enumerate() {
            p.extend(version.to_le_bytes());
            p.push(u8::from(record.is_some()));
            if let Some((hue, dominance, source, origin)) = record {
                p.extend(hue.to_le_bytes());
                p.push(*dominance);
                if index >= self.founding as usize {
                    p.extend(source.unwrap_or(u32::MAX).to_le_bytes());
                }
                p.extend(origin.to_le_bytes());
            }
        }
        p.extend((self.free.len() as u32).to_le_bytes());
        for slot in &self.free {
            p.extend(slot.to_le_bytes());
        }
        p.extend((self.cells.len() as u32).to_le_bytes());
        for (index, a, b) in &self.cells {
            p.extend(index.to_le_bytes());
            p.extend(a.to_le_bytes());
            p.extend(b.to_le_bytes());
        }
        for population in &self.history {
            p.extend(population.to_le_bytes());
        }
        let mut bytes = b"GGLS".to_vec();
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(miniz_oxide::deflate::compress_to_vec_zlib(&p, 6));
        bytes
    }
}

#[test]
fn a_snapshot_built_from_the_documented_layout_imports() {
    let world = World::from_snapshot(&Payload::valid().snapshot()).unwrap();
    assert_eq!((world.width(), world.height()), (16, 16));
    assert_eq!(world.generation(), 1);
    assert_eq!(world.rule(), Rule::CONWAY);
    assert_eq!(world.mutation(), MutationSettings::new(0.25, 40.0).unwrap());
    assert_eq!(world.live_cells(), vec![(1, 1), (2, 1)]);
    assert_eq!(world.census().population_history, vec![5, 2]);
    let tree = world.allele_tree();
    assert_eq!(tree.len(), 2);
    assert_eq!((tree[0].allele.hue, tree[0].source), (10.0, None));
    assert_eq!(tree[0].status, AlleleStatus::Hidden);
    assert_eq!(tree[1].allele.hue, 130.0);
    assert_eq!(tree[1].allele.dominance, 2);
    assert_eq!((tree[1].source, tree[1].origin), (Some(tree[0].id), 1));
    assert_eq!(tree[1].status, AlleleStatus::Living);
}

#[test]
fn a_payload_describing_no_valid_world_is_rejected() {
    type Damage = fn(&mut Payload);
    let cases: [(&str, Damage); 16] = [
        ("world too small", |p| p.width = 8),
        ("rule count above 8", |p| p.birth |= 1 << 9),
        ("mutation rate above 1", |p| p.rate = 2.0),
        ("no founding alleles", |p| p.founding = 0),
        ("founding allele not retained", |p| {
            p.founding = 2;
            p.slots[1].1 = None;
            p.free = vec![1, 2];
            p.cells = vec![(17, 0, 0), (18, 0, 0)];
        }),
        ("allele hue out of range", |p| {
            p.slots[1].1 = Some((360.0, 2, Some(0), 1))
        }),
        ("allele dominance too high", |p| {
            p.slots[1].1 = Some((130.0, 4, Some(0), 1))
        }),
        ("source beyond the slots", |p| {
            p.slots[1].1 = Some((130.0, 2, Some(9), 1))
        }),
        ("source not retained", |p| {
            p.slots[1].1 = Some((130.0, 2, Some(2), 1))
        }),
        ("allele descends from itself", |p| {
            p.slots[1].1 = Some((130.0, 2, Some(1), 1))
        }),
        ("retained slot listed free", |p| p.free = vec![1]),
        ("free slot listed twice", |p| {
            p.slots.push((0, None));
            p.free = vec![2, 2];
        }),
        ("empty slot not freed", |p| p.free = vec![]),
        ("live cells out of order", |p| {
            p.cells = vec![(18, 1, 1), (17, 0, 1)]
        }),
        ("live cell carries an empty slot", |p| {
            p.cells[0] = (17, 0, 2)
        }),
        ("history ends with another population", |p| {
            p.history = vec![5, 3]
        }),
    ];
    for (case, damage) in cases {
        let mut payload = Payload::valid();
        damage(&mut payload);
        match World::from_snapshot(&payload.snapshot()) {
            Err(SnapshotError::Invalid(reason)) => assert!(!reason.is_empty(), "{case}"),
            other => panic!("{case}: expected Invalid, got {other:?}"),
        }
    }
}

#[test]
fn a_live_cell_listed_twice_is_rejected() {
    let mut payload = Payload::valid();
    payload.cells = vec![(17, 0, 1), (17, 0, 1)];
    assert!(World::from_snapshot(&payload.snapshot()).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn an_imported_snapshot_continues_the_run_identically(
        width in 16usize..24,
        height in 16usize..24,
        density in 0.0f64..=1.0,
        seed in any::<u64>(),
        alleles in 1usize..=16,
        birth in 0u16..512,
        survival in 0u16..512,
        rate in 0.0f64..=1.0,
        magnitude in 0.0f64..=180.0,
        before in 0u64..30,
        after in 1u64..30,
    ) {
        let mut world = World::soup_with_alleles(width, height, density, seed, alleles).unwrap();
        world.set_rule(notation(birth, survival).parse().unwrap());
        world.set_mutation(MutationSettings::new(rate, magnitude).unwrap());
        world.run(before);
        let mut imported = World::from_snapshot(&world.snapshot()).unwrap();
        world.run(after);
        imported.run(after);
        prop_assert_eq!(imported.live_cells(), world.live_cells());
        prop_assert_eq!(imported.phenotypes(), world.phenotypes());
        prop_assert_eq!(imported.census(), world.census());
        prop_assert_eq!(imported.allele_tree(), world.allele_tree());
        prop_assert_eq!(imported.rng(), world.rng());
        prop_assert_eq!(imported, world);
    }

    /// Damaged payload bytes behind a valid frame must never panic: import
    /// either fails or yields a World that runs normally.
    #[test]
    fn a_damaged_payload_never_panics(
        seed in any::<u64>(),
        damage in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..4),
    ) {
        let mut world = World::soup_with_alleles(16, 16, 0.5, seed, 3).unwrap();
        world.set_mutation(MutationSettings::new(0.3, 60.0).unwrap());
        world.run(8);
        let bytes = world.snapshot();
        let mut payload = payload(&bytes);
        for (index, value) in damage {
            let i = index.index(payload.len());
            payload[i] = value;
        }
        if let Ok(mut imported) = World::from_snapshot(&reframe(&bytes, &payload)) {
            let tree = imported.allele_tree();
            for node in &tree {
                imported.lineage(node.id);
            }
            imported.census();
            imported.phenotypes();
            imported.run(5);
            imported.census();
        }
    }
}
