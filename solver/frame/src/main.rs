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
    DEFAULT_BACK, DEFAULT_PUSH_GROWTH, DEFAULT_STOP_GROWTH, DEFAULT_STOP_QUIET, DEFAULT_TILT, DEFAULT_UNSEEDED_EVERY,
    DEFAULT_WIDEN, MAX_ABOVE, MAX_BACK, MAX_EFFORT, MAX_TILT, MIN_EFFORT, REPAIR_REACH, ROUND_WORK,
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
                               --threads, --tactic, --show and the flags below, and refuses the others.  The rounds
                               are shaped by --tilt, --back and --widen, which are on unless given 0
      --effort L               with --unsound: the level of effort, from {min_effort} to {max_effort}.
                               Level L runs rounds of {neighborhoods} neighborhoods of the finder's search until
                               their work (their steps and {neighborhood_work} for each neighborhood) reaches
                               2^L x {round_work}; a higher level does the rounds of a lower one first
      --seed S                 with --unsound: the seed of the rounds' random choices (default {seed})
      --fix LO,HI              with --unsound: the share of the incumbent's promotions that each neighborhood keeps
                               fixed, drawn from LO to HI, 0 <= LO <= HI <= 1 (default {fix}); turns widening off,
                               and is refused with --widen S of 1 or more
      --tilt P                 with --unsound: P percent of a round's neighborhoods (P from 0 to {max_tilt}, default
                               {tilt}), the same share of each incumbent's, free the small promotions more often than
                               the large ones (the number freed is the same); the others draw uniformly.  0: none
      --back J                 with --unsound: besides the start's, follow an incumbent from what survives of the moves
                               for each of the games n-1 .. n-J (J from 0 to {max_back}, default {back}) that differs
                               from the ones before, each with its share of a round's neighborhoods.  0: the start's
                               alone
      --above J                with --unsound: follow an incumbent from what survives of the moves for each of the
                               games n+1 .. n+J that the run or --out has (the built-in table's with --from-known),
                               J from 0 to {max_above}, as --back does (default 0: none)
      --widen S                with --unsound: an incumbent that has not grown heavier for S rounds (S >= 1, default
                               {widen}) moves its neighborhoods down a ladder of the shares fixed,
                               {ladder}.  0: no widening
      --unseeded-every K       with --unsound: play each game whose n is a multiple of K twice, as any game and also
                               unseeded, as --unseeded plays it, and keep the better line (default {unseeded_every}).
                               0: no game twice
      --unseeded-effort L      with --unsound: the level of effort of the unseeded play of a game played twice (default:
                               --effort's)
      --stop-growth T|off      with --unsound: end a searched game's rounds once its gap is within T of the gap of the
                               moves for n-1 it starts from (T >= 0), from the end of the rounds of level {min_effort}
                               on (default {stop_growth}).  The gap is the game's bound less its score, and its growth
                               the gap less that of the moves for n-1
      --stop-quiet K           with --unsound: the stop of --stop-growth ends a game's rounds only when the last K
                               rounds raised nothing (default {stop_quiet}); the start counts as round 0.  0: the gap
                               alone
      --push-growth T|off      with --unsound: play a game twice, as --unseeded-every does, if the growth of its gap
                               after its play is above T (default {push_growth})
      --past-ceiling           with --unsound: do not end a game's rounds at the run's ceiling, n + the score of the
                               moves for n-1 it starts from (which they do by default, when it has such moves)
      --no-repair              with --unsound: do not repair lines: by default, what survives of each line in each of
                               the {repair_reach} games below it replaces the line of that game if it scores more, and
                               so on down, with the line appended again
      --stop-at-known          with --unsound: end a game's rounds once its score is the built-in table's
      --from-known             with --unsound: start every game from the built-in table's moves for n-1
      --unseeded               with --unsound: take nothing from the games below: search every game, with no seed,
                               no trajectory of --back and no run's ceiling (for measuring)
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
    let (lo, hi) = finder::Options::default().fix;
    let ladder: Vec<String> = finder::LADDER.iter().map(|(lo, hi)| format!("{lo},{hi}")).collect();
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
        .replace("{fix}", &format!("{lo},{hi}"))
        .replace("{max_tilt}", &MAX_TILT.to_string())
        .replace("{tilt}", &DEFAULT_TILT.to_string())
        .replace("{max_back}", &MAX_BACK.to_string())
        .replace("{back}", &DEFAULT_BACK.to_string())
        .replace("{max_above}", &MAX_ABOVE.to_string())
        .replace("{repair_reach}", &REPAIR_REACH.to_string())
        .replace("{unseeded_every}", &DEFAULT_UNSEEDED_EVERY.to_string())
        .replace("{stop_growth}", &DEFAULT_STOP_GROWTH.map_or("off".to_owned(), |t| t.to_string()))
        .replace("{stop_quiet}", &DEFAULT_STOP_QUIET.to_string())
        .replace("{push_growth}", &DEFAULT_PUSH_GROWTH.map_or("off".to_owned(), |t| t.to_string()))
        .replace("{widen}", &DEFAULT_WIDEN.to_string())
        .replace("{ladder}", &ladder.join(" -> "))
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
