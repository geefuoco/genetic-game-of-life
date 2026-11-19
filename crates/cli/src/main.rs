use clap::Parser;
use genetic_life_core::{Census, MutationSettings, Rule, Soup, World, DEFAULT_FOUNDING_ALLELES};
use serde::Serialize;
use std::path::PathBuf;
use std::process::ExitCode;

/// Runs a seeded Soup, or resumes a Snapshot, for a number of Generations and
/// prints stats as JSON.
#[derive(Parser)]
struct Args {
    /// Seed fixing every random choice in the run.
    #[arg(long, required_unless_present = "snapshot")]
    seed: Option<u64>,
    /// Snapshot file to resume instead of starting a seeded Soup.
    #[arg(long, conflicts_with_all = SOUP_FLAGS)]
    snapshot: Option<PathBuf>,
    /// File to write the Snapshot of the resulting World to.
    #[arg(long)]
    out: Option<PathBuf>,
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

/// Flags that describe a seeded Soup and so cannot be combined with a Snapshot.
const SOUP_FLAGS: [&str; 8] = [
    "seed",
    "width",
    "height",
    "density",
    "rule",
    "alleles",
    "mutation_rate",
    "mutation_magnitude",
];

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
    let mut world = match start(&args) {
        Ok(world) => world,
        Err(error) => return fail(error),
    };
    world.run(args.generations);
    if let Some(out) = &args.out {
        if let Err(error) = std::fs::write(out, world.snapshot()) {
            return fail(format!("cannot write snapshot {}: {error}", out.display()));
        }
    }
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

/// The World to run: the Snapshot if one is given, otherwise the seeded Soup.
fn start(args: &Args) -> Result<World, String> {
    if let Some(path) = &args.snapshot {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("cannot read snapshot {}: {error}", path.display()))?;
        return World::from_snapshot(&bytes).map_err(|error| error.to_string());
    }
    let soup = Soup {
        seed: args.seed.expect("clap requires a seed without a snapshot"),
        rule: args.rule,
        width: args.width,
        height: args.height,
        density: args.density,
        founding_alleles: args.alleles,
        mutation: MutationSettings::new(args.mutation_rate, args.mutation_magnitude)
            .map_err(|error| error.to_string())?,
    };
    soup.world().map_err(|error| error.to_string())
}

fn fail(error: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {error}");
    ExitCode::FAILURE
}
