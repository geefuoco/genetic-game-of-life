use clap::Parser;
use genetic_life_core::{Rule, World};
use serde::Serialize;
use std::process::ExitCode;

/// Runs a seeded Soup for a number of Generations and prints stats as JSON.
#[derive(Parser)]
struct Args {
    /// Seed fixing every random choice in the run.
    #[arg(long)]
    seed: u64,
    /// World width in Cells (16 to 1024).
    #[arg(long, default_value_t = 64)]
    width: usize,
    /// World height in Cells (16 to 1024).
    #[arg(long, default_value_t = 64)]
    height: usize,
    /// Probability that a Cell starts live (0.0 to 1.0).
    #[arg(long, default_value_t = 0.3)]
    density: f64,
    /// Rule as B/S notation (e.g. B36/S23) or a Preset: conway, highlife,
    /// day-and-night, seeds, gnarl, life-without-death.
    #[arg(long, default_value = "conway", value_parser = Rule::resolve)]
    rule: Rule,
    /// Number of Generations to run.
    #[arg(long, default_value_t = 100)]
    generations: u64,
}

#[derive(Serialize)]
struct Stats {
    generation: u64,
    population: usize,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let mut world = match World::soup(args.width, args.height, args.density, args.seed) {
        Ok(world) => world,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    world.set_rule(args.rule);
    world.run(args.generations);
    let stats = Stats {
        generation: world.generation(),
        population: world.population(),
    };
    println!(
        "{}",
        serde_json::to_string(&stats).expect("stats serialise")
    );
    ExitCode::SUCCESS
}
