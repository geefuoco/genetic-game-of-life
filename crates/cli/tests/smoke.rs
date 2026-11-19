use genetic_life_core::World;
use std::path::PathBuf;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_genetic-life"))
        .args(args)
        .output()
        .unwrap()
}

fn stats(args: &[&str]) -> serde_json::Value {
    let output = run(args);
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn prints_population_and_generation_as_json() {
    let json = stats(&[
        "--seed",
        "7",
        "--width",
        "16",
        "--height",
        "16",
        "--density",
        "1.0",
        "--generations",
        "0",
    ]);
    assert_eq!(json["generation"], 0);
    assert_eq!(json["population"], 256);
}

#[test]
fn reports_the_requested_number_of_generations() {
    let json = stats(&["--seed", "7", "--generations", "10"]);
    assert_eq!(json["generation"], 10);
}

#[test]
fn same_seed_prints_identical_stats() {
    let args = ["--seed", "42", "--generations", "25"];
    assert_eq!(stats(&args), stats(&args));
}

#[test]
fn invalid_world_size_fails_with_a_message() {
    let output = run(&["--seed", "1", "--width", "4", "--generations", "1"]);
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn stats_include_the_census() {
    let json = stats(&[
        "--seed",
        "7",
        "--width",
        "16",
        "--height",
        "16",
        "--density",
        "1.0",
        "--alleles",
        "3",
        "--generations",
        "2",
    ]);
    let census = &json["census"];
    assert_eq!(census["generation"], 2);
    assert_eq!(census["population"], json["population"]);
    assert_eq!(census["population_history"].as_array().unwrap().len(), 3);
    assert_eq!(census["population_history"][0], 256);
    let copies: u64 = census["alleles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["copies"].as_u64().unwrap())
        .sum();
    assert_eq!(copies, 2 * json["population"].as_u64().unwrap());
    assert!(census["fully_hidden"].is_array());
}

#[test]
fn invalid_founding_allele_count_fails_with_a_message() {
    let output = run(&["--seed", "1", "--alleles", "0"]);
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn stats_count_the_living_alleles() {
    let json = stats(&[
        "--seed",
        "7",
        "--width",
        "16",
        "--height",
        "16",
        "--density",
        "1.0",
        "--alleles",
        "3",
        "--generations",
        "0",
    ]);
    assert_eq!(json["living_alleles"], 3);
}

#[test]
fn mutation_flags_change_the_run() {
    let base = [
        "--seed",
        "3",
        "--rule",
        "day-and-night",
        "--generations",
        "20",
    ];
    let off = stats(&[&base[..], &["--mutation-rate", "0"]].concat());
    let high = stats(
        &[
            &base[..],
            &["--mutation-rate", "0.5", "--mutation-magnitude", "90"],
        ]
        .concat(),
    );
    assert!(off["living_alleles"].as_u64().unwrap() <= 4);
    assert!(high["living_alleles"].as_u64().unwrap() > 4);
}

#[test]
fn invalid_mutation_settings_fail_with_a_message() {
    for flags in [["--mutation-rate", "1.5"], ["--mutation-magnitude", "200"]] {
        let output = run(&[&["--seed", "1"][..], &flags].concat());
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());
    }
}

fn temp_path(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_TARGET_TMPDIR"), name].iter().collect();
    path.to_str().unwrap().to_string()
}

#[test]
fn writes_a_valid_snapshot_of_the_resulting_world() {
    let out = temp_path("writes_a_valid_snapshot.snapshot");
    let json = stats(&["--seed", "7", "--generations", "10", "--out", &out]);
    let world = World::from_snapshot(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(world.generation(), 10);
    assert_eq!(json["population"], world.population());
}

#[test]
fn resuming_from_a_snapshot_continues_the_run() {
    let base = ["--rule", "highlife", "--mutation-rate", "0.2"];
    let straight = stats(&[&base[..], &["--seed", "42", "--generations", "25"]].concat());
    let out = temp_path("resuming_from_a_snapshot.snapshot");
    stats(
        &[
            &base[..],
            &["--seed", "42", "--generations", "10", "--out", &out],
        ]
        .concat(),
    );
    let resumed = stats(&["--snapshot", &out, "--generations", "15"]);
    assert_eq!(resumed, straight);
}

#[test]
fn a_corrupt_snapshot_fails_with_a_message() {
    let path = temp_path("a_corrupt_snapshot.snapshot");
    std::fs::write(&path, b"not a snapshot").unwrap();
    let output = run(&["--snapshot", &path]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("snapshot"));
}

#[test]
fn a_missing_snapshot_file_fails_with_a_message() {
    let output = run(&["--snapshot", &temp_path("no_such_file.snapshot")]);
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn a_snapshot_cannot_be_combined_with_seed_or_settings() {
    let path = temp_path("combined.snapshot");
    stats(&["--seed", "1", "--generations", "0", "--out", &path]);
    for flags in [["--seed", "1"], ["--width", "32"], ["--rule", "seeds"]] {
        let output = run(&[&["--snapshot", &path][..], &flags].concat());
        assert!(!output.status.success(), "{flags:?}");
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn either_a_seed_or_a_snapshot_is_required() {
    let output = run(&["--generations", "1"]);
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}
