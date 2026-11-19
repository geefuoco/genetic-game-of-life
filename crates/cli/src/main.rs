use clap::Parser;
use genetic_life_core::{Census, MutationSettings, Rule, World, DEFAULT_FOUNDING_ALLELES};
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
    /// Rule as B/S notation (e.g. B36/S23) or a Rule Preset: conway, highlife,
    /// day-and-night, seeds, gnarl, life-without-death.
    #[arg(long, default_value = "conway", value_parser = Rule::resolve)]
    rule: Rule,
    /// Number of Founding Alleles (1 to 16).
    #[arg(long, default_value_t = DEFAULT_FOUNDING_ALLELES)]
    alleles: usize,
    /// Probability that each Allele passed on at a Birth mutates (0.0 to 1.0).
    #[arg(long, default_value_t = MutationSettings::default().rate())]
    mutation_rate: f64,
    /// Largest hue shift of a mutant from its source Allele, in degrees (0 to 180).
    #[arg(long, default_value_t = MutationSettings::default().magnitude())]
    mutation_magnitude: f64,
    /// Number of Generations to run.
    #[arg(long, default_value_t = 100)]
    generations: u64,
}

#[derive(Serialize)]
struct Stats {
    generation: u64,
    population: usize,
    /// Alleles carried by at least one live Cell, shown or hidden.
    living_alleles: usize,
    census: CensusJson,
}

#[derive(Serialize)]
struct CensusJson {
    population: usize,
    generation: u64,
    alleles: Vec<AlleleJson>,
    fully_hidden: Vec<AlleleRef>,
    population_history: Vec<usize>,
}

#[derive(Serialize)]
struct AlleleRef {
    hue: f64,
    dominance: u8,
}

#[derive(Serialize)]
struct AlleleJson {
    hue: f64,
    dominance: u8,
    copies: usize,
    shown: usize,
    hidden: usize,
}

impl From<Census> for CensusJson {
    fn from(census: Census) -> Self {
        Self {
            population: census.population,
            generation: census.generation,
            alleles: census
                .alleles
                .into_iter()
                .map(|a| AlleleJson {
                    hue: a.allele.hue,
                    dominance: a.allele.dominance,
                    copies: a.copies,
                    shown: a.shown,
                    hidden: a.hidden,
                })
                .collect(),
            fully_hidden: census
                .fully_hidden
                .into_iter()
                .map(|a| AlleleRef {
                    hue: a.hue,
                    dominance: a.dominance,
                })
                .collect(),
            population_history: census.population_history,
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    let mutation = match MutationSettings::new(args.mutation_rate, args.mutation_magnitude) {
        Ok(mutation) => mutation,
        Err(error) => return fail(error),
    };
    let mut world = match World::soup_with_alleles(
        args.width,
        args.height,
        args.density,
        args.seed,
        args.alleles,
    ) {
        Ok(world) => world,
        Err(error) => return fail(error),
    };
    world.set_rule(args.rule);
    world.set_mutation(mutation);
    world.run(args.generations);
    let census = world.census();
    let stats = Stats {
        generation: world.generation(),
        population: world.population(),
        living_alleles: census.alleles.len(),
        census: census.into(),
    };
    println!(
        "{}",
        serde_json::to_string(&stats).expect("stats serialise")
    );
    ExitCode::SUCCESS
}

fn fail(error: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {error}");
    ExitCode::FAILURE
}
