//! The `taxman` command: parses the subcommand and hands the rest of the command line to it.
//!
//! The subcommands are in `commands.rs`; the flag parser, the error type and the output macros they share are in
//! `cli.rs`.  Every subcommand is a plain function of the arguments after its name.  A command
//! that fails prints `error: ...` and exits with status 1; a bad command line prints the usage and exits with
//! status 2; a run that SIGINT or SIGTERM stopped (see `solve`) exits with 128 plus the signal; a `solve` that a game
//! passing the wall (`--game-hours`) ended exits with status 4, and one that a lab's `redeploy` ended (`--lab-dir`)
//! with status 5; `certify` exits with status 2 when it finds a game that beats a recorded score.  `verify-certificates` fails with status 1 on the first line that is not what it says.

mod cli;
mod commands;

use cli::{errln, outln, CliError, CliResult, REDEPLOY_STATUS, WALL_STATUS};
use std::process::ExitCode;
use taxman::engine;
use taxman::search::{finder, SearchTactics, PROFILE_DEPTH};
use taxman::tactics::Tactics;

/// The usage text with its `{...}` parts to fill in by [`usage`].
const USAGE: &str = "\
usage: taxman <command> [args]

commands:
  solve N|A-B [options]   solve game N or a range, each from the one before; the first from the game
                          of A-1 in --out FILE, or with --use-prev from the known games (with neither it is
                          solved standalone, which costs more: no seed from the game before and no ceiling).
                          Every score is checked against optimal.json, which is built into this program
      --threads K              worker threads (default: all CPUs)
      --background             run the workers at the lowest priority (Linux SCHED_IDLE): on a machine shared with
                               other work, with every hardware thread, the search takes the CPU the rest leaves
      --window K               games of a range in flight at once, sharing the threads: game n+1 is set up, and
                               searched from n's best game so far, while n searches (default 4; 1 is one game after
                               another, and it is always 1 with one thread).  Scores and proofs are those of a
                               window of 1
      --tactic NAME=VALUE      set a tactic (repeatable).  A switch is on or off, match_refine is 0-{max_refine},
                               nogood_period and nogood_limit are whole numbers, nogood_keep is none, hit, sizeN
                               or topN.  Names: {tactics}
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
      --keep-going             a game that a limit stops does not end the range: its best game so far is recorded
                               as an unfinished line, and the next game goes on from it as a seed alone (no
                               ceiling, no prime formula, nothing that rests on its being optimal); the games are
                               solved one at a time.  A rerun passes over the games --out holds only unfinished
                               lines of; a signal still ends the range, with nothing recorded for its game
      --retry                  with --keep-going, search again the games --out holds only unfinished lines of
                               (each from its checkpoint), as a pass with a longer limit does
      --retry-improved         with --keep-going, search again only the games --out holds only unfinished lines of
                               whose game before it now has more to give than when they were tried: it is
                               proven, or has a better best game, or they had none (the others are passed over)
      --pass N                 record N as the pass in the options of every line written (for a script that
                               goes over the games more than once); it changes nothing else
      --game-hours H           the wall: stop the range at the first game whose own search (the time it is the
                               first of the window, summed over runs and kept in DIR/nNNNN.clock) passes H hours:
                               every search stops with a final checkpoint, nothing is recorded for that game, the
                               line `wall at N after H hours` is printed and the exit status is 4
      --lab-dir DIR            the lab's command folder: at each progress tick the run takes the files of DIR/in
                               (one JSON command each: checkpoint, offer, branching, restart, threads, finder,
                               redeploy), applies or refuses each, writes the outcome to DIR/out and moves the
                               command to DIR/done.  A lab changes how the games are searched, never their
                               results.  A redeploy ends the run with every search checkpointed, the line
                               `redeploy to COMMIT` and the exit status 5
      --hook PROGRAM           run PROGRAM, detached, with one JSON argument: the event long when a game's own
                               search passes --hook-after (once a game), and done, with its line, when such a game
                               has its line
      --hook-after S           seconds of a game's own search before the hook is told it runs long (default 3600)
      --profile DIR            keep a profile of each game's search in DIR/nNNNN.profile.jsonl: where its steps
                               go, by node near the root, by number of promotions and by checkpoint interval, and
                               its open units at each checkpoint (solver/tools/read_profile.py reads it).  It
                               changes no search and no result, and is no part of a checkpoint
      --profile-depth D        the nodes a profile names: those at most D branchings below the root (default
                               {profile_depth})
      --carried-dir DIR        carry nogoods into each game's search (the tactic carried_nogoods, which this
                               turns on): the stores DIR/nogoods-N.txt of its game and the games before it, the
                               latest first, checked at its root.  They only prune, and change no result
      --mem-cap UNITS          search units kept in memory before spilling to disk
      --force-search           search even where the previous game or an earlier search gives the answer
      --use-prev               draw on the known games: optimal.json's, the committed chain's
                               (results/strict.jsonl) and the proven boards of the bounds store
                               (results/bounds.jsonl).  A board a source has proven is recorded without a
                               search (proof known), citing the source and what it assumes (optimal.json's games
                               are assumed optimal; the store's, wherever statement U holds at the boards its
                               bound rests on); the game of A-1 from them, if --out FILE lacks it, gives the
                               ceiling and the seed; the store's other boards seed their searches with its lower
                               bound's game and stop at its upper bound.  --force-search leaves the boards to
                               their searches
      --show                   print each game's promotions and moves
  regress [--from A] [--to N] [--threads K] [--window K] [--tactic ...] [--use-prev]
                          solve A..N (default 1..400) in memory and check every score against optimal.json
  bench [--threads K] [--tactic ...] [--history FILE] [--known FILE] [N ...]
                          time the benchmark games (default 936 1050 1100 1133 1170 1184), each on its own
                          with the previous game from optimal.json or past it from the results file --known
                          (default results/strict.jsonl), and no derivation; appends the run to
                          --history (default solver/frame/bench/history.jsonl; both in the repository)
  verify-optimal [FILE]   replay every game of the built-in optimal.json (or of the table FILE) and check its
                          recorded score
  verify-results [--threads K] FILE...
                          check results files without changing them: every game replayed and its score checked
                          against optimal.json, the chain from n-1, each line's proof and provenance, and that the
                          files agree; counts games by proof and by what they assume.  K threads set up the
                          searches of the repeats (default: all CPUs).  A line that cites a known source is checked
                          against optimal.json, the committed chain and the bounds store of results
  extend-optimal RESULTS --out FILE [--table FILE]
                          write the built-in optimal.json (or the table FILE) to --out FILE with the games of the
                          results file RESULTS appended: from the game after the table's last, each game that
                          RESULTS has completed, up to the first that it lacks.  A rebuild takes in a changed table
  certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE [options]
                          prove the recorded scores of A..B (or of the boards of the list FILE, one n a
                          line in the order to certify them, blank lines and # comments skipped) that rest on the
                          frame model on the whole board, game by game: the extended problem with the open
                          members of the barred bounds (--upper, the prover's `upper` files) optional, searched
                          from the recorded game (--results, over optimal.json); one line a game in --out,
                          certified or beaten; a game whose extended problem an earlier certified game of the
                          last 8 games posed is certified without a search (same_problem); a game that
                          beats its score is also written to OUT.counterexample.jsonl and the run exits with
                          status 2.
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
        .replace("{max_refine}", &SearchTactics::MAX_MATCH_REFINE.to_string())
        .replace("{finder_after}", &finder::AFTER.to_string())
        .replace("{profile_depth}", &PROFILE_DEPTH.to_string())
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
        // `solve` has said where on stdout.
        Err(CliError::Wall) => ExitCode::from(WALL_STATUS),
        Err(CliError::Redeploy) => ExitCode::from(REDEPLOY_STATUS),
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
        "certify" => commands::certify(rest),
        "verify-certificates" => commands::verify_certificates(rest),
        other => Err(CliError::Usage(format!("unknown command {other:?}"))),
    }
}
