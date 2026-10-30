//! The `taxman` command: parses the subcommand and hands the rest of the command line to it.
//!
//! The subcommands are in `commands.rs`; the flag parser, the error type and the output macros they share are in
//! `cli.rs`.  Every subcommand is a plain function of the arguments after its name.  A command
//! that fails prints `error: ...` and exits with status 1; a bad command line prints the usage and exits with
//! status 2; a run that SIGINT or SIGTERM stopped (see `solve`) exits with 128 plus the signal; `certify` exits with
//! status 2 when it finds a game that beats a recorded score.  `verify-certificates` fails with status 1 on the first
//! line that is not what it says.

mod cli;
mod commands;

use cli::{errln, outln, CliError, CliResult};
use std::process::ExitCode;
use taxman::engine;
use taxman::search::{finder, gate_trial, CONFLICT_BRANCHING_GAP, MAX_GATE, PROFILE_DEPTH};
use taxman::tactics::Tactics;

/// The usage text with its `{...}` parts to fill in by [`usage`].
const USAGE: &str = "\
usage: taxman <command> [args]

commands:
  solve N|A-B [options]   solve game N or a range, each from the one before; the first from the game
                          of A-1 in --out FILE, or with --use-prev from the known games (with neither it is
                          solved standalone, which costs more: no seed from the game before and no ceiling).
                          Every score is checked against the table of optimal games, which is built into this program
      --threads K              worker threads (default: all CPUs)
      --background             run the workers at the lowest priority (Linux SCHED_IDLE): on a machine shared with
                               other work, with every hardware thread, the search takes the CPU the rest leaves
      --window K               games of a range in flight at once, sharing the threads: game n+1 is set up, and
                               searched from n's best game so far, while n searches (default 4; 1 is one game after
                               another, and it is always 1 with one thread).  Scores and proofs are those of a
                               window of 1
      --tactic NAME=VALUE      set a tactic (repeatable): VALUE is on or off.  Names: {tactics}
      --finder-after NODES     once a game's search has taken NODES steps (over all its runs), start a finder beside
                               it: a thread that looks for a better game than the incumbent, from the incumbent and
                               from the game of n-1, and offers it to the search (default {finder_after}; 0: none).
                               It changes no answer and is no part of a checkpoint, which resumes with any finder or
                               none
      --finder-seconds S       the finder thread's budget, from the moment it starts (default 60)
      --out FILE               append each result as a JSON line; games already completed in FILE are skipped
      --checkpoint-dir DIR     keep a resumable checkpoint nNNNN.ckpt per game in DIR
      --checkpoint-interval S  seconds between checkpoints (default 60)
      --progress S             seconds between progress lines on stderr (default 60, 0 for none)
      --time-limit S           stop a game's search after S seconds (keeps its checkpoint)
      --node-limit N           stop a game's search after N nodes (keeps its checkpoint)
                               SIGINT or SIGTERM stops the search the same way (exit status 130 or 143);
                               a second one kills the process.  One process at a time uses a checkpoint DIR.
      --profile DIR           keep a profile of each game's search in DIR/nNNNN.profile.jsonl: where its steps
                               go, by node near the root, by number of promotions and by checkpoint interval, and
                               its open units at each checkpoint (solver/tools/read_profile.py reads it).  It
                               changes no search and no result, and is no part of a checkpoint
      --profile-depth D        the nodes a profile names: those at most D branchings below the root (default
                               {profile_depth})
      --carried-dir DIR        carry nogoods into each game's search (the tactic carried_nogoods, which this
                               turns on): the stores DIR/nogoods-N.txt of its game and the games before it, the
                               latest first, checked at its root; each search writes its own there.  They only
                               prune, and change no result
      --gate GAP               the gap, from 0 to {max_gate}, from which conflict branching chooses its conflict, in
                               every game's search (default {gate}).  It changes no answer and is no part of a
                               checkpoint; refused with conflict_branching off
      --gate-trial GATES       once a game's search has taken {trial_after} steps, try each of the gates GATES (comma
                               separated, each from 0 to {max_gate}) and the search's own on a sample of its units,
                               and branch with the one that took the least CPU.  It needs --checkpoint-dir
      --gate-trial-plan SPEC   the trial's numbers, NAME=VALUE separated by commas: after ({trial_after}), units
                               ({trial_units}), steps ({trial_steps}, the most of each solve), threads
                               ({trial_threads}) and least_gap ({trial_least_gap})
      --mem-cap UNITS          search units kept in memory before spilling to disk
      --force-search           search even where the previous game or an earlier search gives the answer
      --use-prev               draw on the known games: the built-in table's, the committed chain's
                               (results/optimal.jsonl.gz) and the proven boards of the bounds store
                               (results/bounds.jsonl).  A board a source has proven is recorded without a
                               search (proof known), citing the source and what it assumes (the table's games
                               are assumed optimal; the store's, wherever statement U holds at the boards its
                               bound rests on); the game of A-1 from them, if --out FILE lacks it, gives the
                               ceiling and the seed; the store's other boards seed their searches with its lower
                               bound's game and stop at its upper bound.  --force-search leaves the boards to
                               their searches
      --show                   print each game's promotions and moves
  regress [--from A] [--to N] [--threads K] [--window K] [--tactic ...] [--use-prev]
                          solve A..N (default 1..400) in memory and check every score against the built-in table
  verify-results [--threads K] FILE...
                          check results files without changing them: every game replayed and its score checked
                          against the built-in table, the chain from n-1, each line's proof and provenance, and that
                          the files agree; counts games by proof and by what they assume.  A file whose name ends in
                          .gz is read through the gzip decoder (the committed results are one).  K threads set up the
                          searches of the repeats (default: all CPUs).  A line that cites a known source is checked
                          against the built-in table, the committed chain and the bounds store of results
  certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE [options]
                          prove the recorded scores of A..B (or of the boards of the list FILE, one n a
                          line in the order to certify them, blank lines and # comments skipped) that rest on the
                          frame model on the whole board, game by game: the extended problem with the open
                          members of the barred bounds (--upper, the prover's `upper` files) optional, searched
                          from the recorded game (--results, over the built-in table; a .gz file is read as gzip);
                          one line a game in --out, certified or beaten; a game whose extended problem an
                          earlier certified game of the last 8 games posed is certified without a search
                          (same_problem); a game that beats its score is also written to
                          OUT.counterexample.jsonl and the run exits with status 2.
                          Options: --threads K, --checkpoint-dir DIR, --checkpoint-interval S, --progress S,
                          --time-limit S, --node-limit N, --mem-cap UNITS, --background, as for solve; --no-claims,
                          search the extended problem without the claims of `unzip` (the formulation of the
                          certificates made before the kept-claims lemma, E4; the default keeps the claims it
                          allows).  A rerun skips the games --out has a verdict for and resumes a stopped search
                          from its checkpoint
  verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...] [--threads K]
                          check the files of certificates `certify --out` wrote, without changing them: every
                          line against the recorded games, the barred bounds and the extended problem set up
                          again, the games that rest on one another, and the counterexample files beside them;
                          the first line to fail ends it with status 1 (that a search explored its whole tree is
                          taken on trust).  Then says how far the proofs of the recorded scores reach: every
                          game to some board is proven by the prover (--exact, default every exact_*.jsonl of
                          solver/noframe/data), by a certificate, or by a ceiling above a proven game.  K threads
                          set the problems up (default: all CPUs)
  tables N K...           print the maximal factors and composites of each K on the board 1..=N
  help                    show this message
";

/// [`USAGE`] with the tactics' names and values, which the solver defines.
fn usage() -> String {
    let rows: Vec<String> = Tactics::names().chunks(4).map(|row| row.join(", ")).collect();
    USAGE
        .replace("{tactics}", &rows.join(&format!(",\n{:31}", "")))
        .replace("{finder_after}", &finder::AFTER.to_string())
        .replace("{profile_depth}", &PROFILE_DEPTH.to_string())
        .replace("{gate}", &CONFLICT_BRANCHING_GAP.to_string())
        .replace("{max_gate}", &MAX_GATE.to_string())
        .replace("{trial_after}", &gate_trial::AFTER.to_string())
        .replace("{trial_units}", &gate_trial::UNITS.to_string())
        .replace("{trial_steps}", &gate_trial::STEPS.to_string())
        .replace("{trial_threads}", &gate_trial::THREADS.to_string())
        .replace("{trial_least_gap}", &gate_trial::LEAST_GAP.to_string())
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
        Err(CliError::Counterexample(message)) => {
            errln!("counterexample: {message}");
            ExitCode::from(2)
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
        "verify-results" => commands::verify_results(rest),
        "tables" => commands::tables(rest),
        "certify" => commands::certify(rest),
        "verify-certificates" => commands::verify_certificates(rest),
        other => Err(CliError::Usage(format!("unknown command {other:?}"))),
    }
}
