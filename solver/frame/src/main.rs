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
use taxman::search::{finder, gate_trial, CONFLICT_BRANCHING_GAP, MAX_GATE, PROFILE_DEPTH};
use taxman::solver::unsound::{
    DEFAULT_BEST_KNOWN, DEFAULT_STOP_GROWTH, MAX_ABOVE, MAX_EFFORT, MIN_EFFORT, ROUND_WORK, STOP_QUIET,
};
use taxman::tactics::Tactics;

/// The usage text with its `{...}` parts to fill in by [`usage`].
const USAGE: &str = "\
usage: taxman <command> [args]

commands:
  solve N|A-B [options]   solve game N or a range, each from the one before; the first from the moves
                          for A-1 in --out FILE (without them it is solved standalone, which costs more: no
                          seed from the moves before and no ceiling).
                          Every score is checked against the table of optimal moves, which is built into this program
      --threads K              worker threads (default: all CPUs)
      --background             run the workers at the lowest priority (Linux SCHED_IDLE): on a machine shared with
                               other work, with every hardware thread, the search takes the CPU the rest leaves
      --window K               games of a range in flight at once, sharing the threads: game n+1 is set up, and
                               searched from the best moves found for n so far, while n searches (default 4; 1 is
                               one game after another, and it is always 1 with one thread).  Scores and proof kinds are
                               those of a window of 1
      --tactic NAME=VALUE      set a tactic (repeatable): VALUE is on or off.  Names: {tactics}
      --finder-after NODES     once a game's search has taken NODES steps (over all its runs), start a finder beside
                               it: a thread that looks for better moves than the incumbent, from the incumbent and
                               from the moves for n-1, and offers it to the search (default {finder_after}; 0: none).
                               It changes no answer and is no part of a checkpoint, which resumes with any finder or
                               none
      --finder-seconds S       the finder thread's budget, from the moment it starts (default {finder_seconds})
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
                               and branch with the one that took the least CPU (default none: no trial, and a
                               search that keeps its gate).  It runs only with --checkpoint-dir
      --gate-trial-plan SPEC   the trial's numbers, NAME=VALUE separated by commas: after ({trial_after}), units
                               ({trial_units}), steps ({trial_steps}, the most of each solve), threads
                               ({trial_threads}) and least_gap ({trial_least_gap}).  Without --gate-trial it asks
                               for a trial of the gates {trial_gates}
      --mem-cap UNITS          search units kept in memory before spilling to disk
      --force-search           search even where the previous game or an earlier search gives the answer
      --show                   print each game's promotions and moves
      --unsound                play each game as well as --effort finds, with no proof, on all the threads: each
                               line, appended to --out (needed), holds legal moves and their score, a lower bound,
                               and is marked unsound, which every reader of proven lines refuses.  It takes
                               --threads, --tactic, --show and the flags below, and refuses the others.  A game
                               follows trajectories from the games before it, and a game whose gap to its bound
                               grows is played twice, also unseeded; the repair replaces a line that a line above
                               it shows short
      --effort L               with --unsound: the level of effort, from {min_effort} to {max_effort}.
                               Level L runs rounds of {neighborhoods} neighborhoods of the finder's search until
                               their work (their steps and {neighborhood_work} for each neighborhood) reaches
                               2^L x {round_work}; a higher level does the rounds of a lower one first
      --seed S                 with --unsound: the seed of the rounds' random choices (default {seed})
      --above J                with --unsound: besides the games before, follow an incumbent from what survives of the
                               moves for each of the games n+1 .. n+J that the run, --out or the best-known file has
                               (J from 0 to {max_above}, default 0: none)
      --stop-growth T|off      with --unsound: end a searched game's rounds once its gap is within T of the gap of the
                               moves for n-1 it starts from (T >= 0) and the last {stop_quiet} rounds raised nothing,
                               from the end of the rounds of level {min_effort} on (default {stop_growth}).  The gap is
                               the game's bound less its score, and its growth the gap less that of the moves for n-1
      --best-known FILE|off    with --unsound: the best-known file of unproven lines, the best-known line of each game
                               past the built-in table, from any run (default {default_best_known} in the directory of
                               --out; off: none).  A game starts from the file's moves for the games around it when
                               they score more than the run's own, every line of --out and every line the run records
                               is offered to the file, and the walk of the repair goes down into its lines of the
                               games below the range.  Runs may share a file: it is locked only to read what is new
                               and to append
      --known-to N             with --unsound: take the built-in table as holding the games up to N only, so that a
                               game above N is as if unproven: no game starts from the table's moves for a game above
                               N, the best-known file may hold the games above N and refuses those up to N, and the
                               checks against the table apply up to N.  For a rehearsal on games whose opt(n) is
                               proven.  The line records N
  regress [--from A] [--to N] [--threads K] [--window K] [--tactic ...]
                          solve A..N (default 1..400) in memory and check every score against the built-in table
  verify-results [--threads K] FILE...
                          check results files without changing them: every line's moves replayed and its score checked
                          against the built-in table, the chain from n-1, each line's proof and provenance, and that
                          the files agree; counts lines by proof kind and by what they assume.  A file whose name ends
                          in .gz is read through the gzip decoder (the committed results are one).  K threads set up the
                          searches of the repeats (default: all CPUs)
  tables N K...           print the maximal factors and composites of each K on the board 1..=N
  help                    show this message
";

/// [`USAGE`] with the tactics' names and values, which the solver defines.
fn usage() -> String {
    let rows: Vec<String> = Tactics::names().chunks(4).map(|row| row.join(", ")).collect();
    let gates = gate_trial::GATES.map(|gate| gate.to_string()).join(",");
    USAGE
        .replace("{tactics}", &rows.join(&format!(",\n{:31}", "")))
        .replace("{finder_after}", &finder::AFTER.to_string())
        .replace("{finder_seconds}", &finder::BUDGET.as_secs().to_string())
        .replace("{profile_depth}", &PROFILE_DEPTH.to_string())
        .replace("{gate}", &CONFLICT_BRANCHING_GAP.to_string())
        .replace("{max_gate}", &MAX_GATE.to_string())
        .replace("{trial_gates}", &gates)
        .replace("{trial_after}", &gate_trial::AFTER.to_string())
        .replace("{trial_units}", &gate_trial::UNITS.to_string())
        .replace("{trial_steps}", &gate_trial::STEPS.to_string())
        .replace("{trial_threads}", &gate_trial::THREADS.to_string())
        .replace("{trial_least_gap}", &gate_trial::LEAST_GAP.to_string())
        .replace("{min_effort}", &MIN_EFFORT.to_string())
        .replace("{max_effort}", &MAX_EFFORT.to_string())
        .replace("{neighborhoods}", &finder::NEIGHBORHOODS.to_string())
        .replace("{round_work}", &ROUND_WORK.to_string())
        .replace("{neighborhood_work}", &finder::NEIGHBORHOOD_WORK.to_string())
        .replace("{seed}", &finder::Options::default().seed.to_string())
        .replace("{max_above}", &MAX_ABOVE.to_string())
        .replace("{stop_growth}", &DEFAULT_STOP_GROWTH.map_or("off".to_owned(), |t| t.to_string()))
        .replace("{stop_quiet}", &STOP_QUIET.to_string())
        .replace("{default_best_known}", DEFAULT_BEST_KNOWN)
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
        "verify-results" => commands::verify_results(rest),
        "tables" => commands::tables(rest),
        other => Err(CliError::Usage(format!("unknown command {other:?}"))),
    }
}
