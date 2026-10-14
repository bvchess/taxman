//! The `taxman` command: parses the subcommand and hands the rest of the command line to it.
//!
//! The subcommands are in `commands.rs`; the flag parser, the error type and the output macros they share are in
//! `cli.rs`.  Every subcommand is a plain function of the arguments after its name.  A command
//! that fails prints `error: ...` and exits with status 1; a bad command line prints the usage and exits with
//! status 2; a run that SIGINT or SIGTERM stopped (see `solve`) exits with 128 plus the signal.

mod cli;
mod commands;

use cli::{errln, outln, CliError, CliResult};
use std::process::ExitCode;
use taxman::engine;
use taxman::search::SearchTactics;
use taxman::tactics::Tactics;

/// The usage text with its `{...}` parts to fill in by [`usage`].
const USAGE: &str = "\
usage: taxman <command> [args]

commands:
  solve N|A-B [options]   solve one board size or a range, each from the one before; the first from the game
                          of A-1 in --out FILE, or with --use-prev in optimal.json (with neither it is solved
                          standalone, which costs more: no seed from the game before and no ceiling).  Every
                          score is checked against optimal.json, which is built into this program
      --threads K              worker threads (default: all cores)
      --tactic NAME=VALUE      set a tactic (repeatable).  A switch is on or off, match_refine is 0-{max_refine}.
                               Names: {tactics}
      --out FILE               append each result as a JSON line; games already completed in FILE are skipped
      --checkpoint-dir DIR     keep a resumable checkpoint nNNNN.ckpt per game in DIR
      --checkpoint-interval S  seconds between checkpoints (default 60)
      --progress S             seconds between progress lines on stderr (default 60, 0 for none)
      --time-limit S           stop a game's search after S seconds (keeps its checkpoint)
      --node-limit N           stop a game's search after N nodes (keeps its checkpoint)
                               SIGINT or SIGTERM stops the search the same way (exit status 130 or 143);
                               a second one kills the process.  One process at a time uses a checkpoint DIR.
      --mem-cap UNITS          search units kept in memory before spilling to disk
      --force-search           search even where the previous game or an earlier search gives the answer
      --use-prev               take the game of A-1 from optimal.json if --out FILE lacks it; the results
                               whose proofs rest on it record that they assume it optimal
      --show                   print each game's promotions and moves
  regress [--from A] [--to N] [--threads K] [--tactic ...] [--use-prev]
                          solve A..N (default 1..400) in memory and check every score against optimal.json
  bench [--threads K] [--tactic ...] [--history FILE] [--known FILE] [N ...]
                          time the benchmark games (default 936 1050 1100 1133 1170 1184), each on its own
                          with the previous game from optimal.json or past it from the results file --known
                          (default rust/results/strict.jsonl), and no derivation; appends the run to
                          --history (default rust/bench/history.jsonl; both in the repository)
  verify-optimal [FILE]   replay every game of the built-in optimal.json (or of the table FILE) and check its
                          recorded score
  verify-results FILE...  check results files without changing them: every game replayed and its score checked
                          against optimal.json, the chain from n-1, each line's proof and provenance, and that the
                          files agree; counts games by proof and by what they assume
  extend-optimal RESULTS --out FILE [--table FILE]
                          write the built-in optimal.json (or the table FILE) to --out FILE with the games of the
                          results file RESULTS appended: from the board size after the table's last, each game that
                          RESULTS has completed, up to the first that it lacks.  A rebuild takes in a changed table
  tables N K...           print the maximal factors and composites of each K on the board 1..=N
  help                    show this message
";

/// [`USAGE`] with the tactics' names and values, which the solver defines.
fn usage() -> String {
    let rows: Vec<String> = Tactics::names().chunks(4).map(|row| row.join(", ")).collect();
    USAGE
        .replace("{tactics}", &rows.join(&format!(",\n{:31}", "")))
        .replace("{max_refine}", &SearchTactics::MAX_MATCH_REFINE.to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        // A run that a signal stopped (with its checkpoint, see `solve`) tells the shell so, as 128 + the signal.
        Ok(()) => engine::stop_signal().map_or(ExitCode::SUCCESS, |signal| ExitCode::from(128 + signal as u8)),
        Err(CliError::Usage(message)) => {
            errln!("error: {message}\n\n{}", usage());
            ExitCode::from(2)
        }
        Err(CliError::Failed(message)) => {
            errln!("error: {message}");
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
            outln!("{}", usage().trim_end());
            Ok(())
        }
        "solve" => commands::solve(rest),
        "regress" => commands::regress(rest),
        "bench" => commands::bench(rest),
        "verify-optimal" => commands::verify_optimal(rest),
        "verify-results" => commands::verify_results(rest),
        "extend-optimal" => commands::extend_optimal(rest),
        "tables" => commands::tables(rest),
        other => Err(CliError::Usage(format!("unknown command {other:?}"))),
    }
}
