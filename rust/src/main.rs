//! Command-line front end.  Subcommands are plain functions taking the arguments after the subcommand name.

use std::process::ExitCode;
use std::time::Instant;
use taxman::game::Optimal;
use taxman::numtheory::Tables;

const USAGE: &str = "\
usage: taxman <command> [args]

commands:
  verify-optimal [FILE]   replay every game in optimal.json (or FILE) and check its recorded score
  tables N K...           print the maximal factors and composites of each K on the board 1..=N
  help                    show this message
";

enum CliError {
    /// Bad command line: print the message and the usage, exit 2.
    Usage(String),
    /// The command ran and failed: print the message, exit 1.
    Failed(String),
}

type CliResult = Result<(), CliError>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(message)) => {
            eprintln!("error: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Err(CliError::Failed(message)) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> CliResult {
    let Some((command, rest)) = args.split_first() else {
        return Err(CliError::Usage("no command given".into()));
    };
    match command.as_str() {
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        "verify-optimal" => verify_optimal(rest),
        "tables" => tables(rest),
        // Later subcommands go here, each as a function like the ones above:
        //   "solve"   - solve one n or a range, with checkpointing (see DESIGN.md, "Per-game pipeline")
        //   "regress" - solve 1..N and compare every score with optimal.json
        //   "bench"   - the fixed benchmark set
        other => Err(CliError::Usage(format!("unknown command {other:?}"))),
    }
}

fn millis(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

fn verify_optimal(args: &[String]) -> CliResult {
    let path = match args {
        [] => Optimal::default_path(),
        [file] => file.into(),
        _ => return Err(CliError::Usage("verify-optimal takes at most one argument".into())),
    };
    let start = Instant::now();
    let optimal = Optimal::load(&path).map_err(CliError::Failed)?;
    let load_ms = millis(start);

    let start = Instant::now();
    let checked = optimal.verify_all().map_err(CliError::Failed)?;
    let replay_ms = millis(start);

    let total_moves: usize = optimal.games().map(|(_, _, moves)| moves.len()).sum();
    println!("verified {checked} games (n up to {}, {total_moves} moves)", optimal.max_n());
    println!("load+parse {load_ms:.1} ms, replay {replay_ms:.1} ms");
    Ok(())
}

fn tables(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: tables N K...".into());
    let (n, ks) = args.split_first().ok_or_else(usage)?;
    if ks.is_empty() {
        return Err(usage());
    }
    let n: u32 = n.parse().map_err(|_| CliError::Usage(format!("N must be a non-negative integer, got {n:?}")))?;
    let tables = Tables::new(n);
    for k in ks {
        let k: u32 = k.parse().map_err(|_| CliError::Usage(format!("K must be a non-negative integer, got {k:?}")))?;
        if k < 1 || k > n {
            return Err(CliError::Usage(format!("K = {k} is not on the board 1..={n}")));
        }
        println!(
            "{k}: maximal factors {:?}; maximal composites {:?}",
            tables.max_factors[k as usize], tables.max_composites[k as usize]
        );
    }
    Ok(())
}
