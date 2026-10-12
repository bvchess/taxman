//! Command-line front end.  Subcommands are plain functions taking the arguments after the subcommand name.

mod cli;
mod commands;

use cli::{CliError, CliResult};
use std::process::ExitCode;
use std::time::Instant;
use taxman::game::Optimal;
use taxman::numtheory::Tables;
use taxman::search::{Branching, SearchTactics};
use taxman::solver::Tactics;

/// The usage text with its `{...}` parts to fill in by [`usage`].
const USAGE: &str = "\
usage: taxman <command> [args]

commands:
  solve N|A-B [options]   solve one board size or a range, each from the one before
      --threads K              worker threads (default: all cores)
      --tactic NAME=VALUE      set a tactic (repeatable).  A switch is on or off, match_refine is 0-{max_refine},
                               branching is {branchings}.
                               Names: {tactics}
      --out FILE               append each result as a JSON line; games already completed in FILE are skipped
      --checkpoint-dir DIR     keep a resumable checkpoint nDDDD.ckpt per game in DIR
      --checkpoint-interval S  seconds between checkpoints (default 60)
      --progress S             seconds between progress lines on stderr (default 60, 0 for none)
      --time-limit S           stop a game's search after S seconds (keeps its checkpoint)
      --node-limit N           stop a game's search after N nodes (keeps its checkpoint)
      --mem-cap UNITS          search units kept in memory before spilling to disk
      --force-search           search even where the previous game gives the answer
      --optimal FILE           optimal.json to compare with and to take previous games from (none: no file)
      --show                   print each game's promotions and moves
  regress [--from A] [--to N] [--threads K] [--tactic ...]
                          solve A..N (default 1..400) in memory and check every score against optimal.json
  bench [--threads K] [--tactic ...] [--history FILE] [N ...]
                          time the benchmark games (default 480 500 600 640 660 700 720 760), each on its own
                          with the previous game from optimal.json and no derivation; appends to FILE (default
                          bench/history.jsonl)
  verify-optimal [FILE]   replay every game in optimal.json (or FILE) and check its recorded score
  tables N K...           print the maximal factors and composites of each K on the board 1..=N
  help                    show this message
";

/// [`USAGE`] with the tactics' names and values, which the solver defines.
fn usage() -> String {
    let rows: Vec<String> = Tactics::names().chunks(4).map(|row| row.join(", ")).collect();
    USAGE
        .replace("{tactics}", &rows.join(&format!(",\n{:31}", "")))
        .replace("{branchings}", &Branching::ALL.map(Branching::name).join(", "))
        .replace("{max_refine}", &SearchTactics::MAX_MATCH_REFINE.to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(message)) => {
            eprintln!("error: {message}\n\n{}", usage());
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
            print!("{}", usage());
            Ok(())
        }
        "solve" => commands::solve(rest),
        "regress" => commands::regress(rest),
        "bench" => commands::bench(rest),
        "verify-optimal" => verify_optimal(rest),
        "tables" => tables(rest),
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
