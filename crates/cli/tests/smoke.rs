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
