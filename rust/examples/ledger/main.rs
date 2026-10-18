//! The ledger: a lower bound L(n), the score of a game it holds, and an upper bound U(n) on opt(n) for every board
//! size of a stretch at once, tightened from both sides by limited searches and by the relations between boards,
//! with the effort spent where a proof looks cheapest.  A board is proven when L(n) = U(n).  The design and the
//! experiment are in docs/experiments/the-ledger-bounds-from-both-sides-2026-10.md.
//!
//! ```text
//! cargo run --release --example ledger -- run A-B --dir DIR --cpu-hours H [--workers P] [--base S] [--max-limit S]
//!     [--climb up|down|both|off] [--climb-limit S] [--lns-seconds S] [--gap-unit G] [--proven FILE] [--bounds FILE]
//!     [--save-every S]
//! cargo run --release --example ledger -- report DIR [--known FILE]...
//! cargo run --release --example ledger -- export DIR --bounds FILE --source NAME [--checkpoints PREFIX]
//!     [--strict FILE] [--date YYYY-MM-DD]
//! cargo run --release --example ledger -- verify-bounds FILE [--strict FILE] [--workers P]
//! cargo run --release --example ledger -- games FILE [--workers P]
//! ```
//!
//! **run** keeps the ledger of the boards A..B in DIR and works on it until every board is proven, the CPU budget of
//! `--cpu-hours` (the process's CPU time, all its threads, over all its runs) is spent, or SIGINT or SIGTERM stops it
//! (each search then writes its checkpoint).  Run it again with the same DIR to go on.
//!
//! * **The bounds and where they come from** (`book.rs`; the rules are the names the files use).  The *map*
//!   (`start`, `root`): each board searched is first set up as `taxman solve N` sets it up standalone (no game of
//!   n − 1), and its search stopped before its first node: L is the game it starts from (the greedy set or the
//!   matching dive, improved by the local search) and U the dual bound of that search, the root's GF(2) bound, as
//!   `wall landscape` has them.  *Searches* (`search`, `dual`, `exhausted`): L is a search's best game; U is the dual
//!   bound of a search a limit stopped (`taxman::search::dual_bound`: the largest of its incumbent and the bounds of
//!   its open units), or the game of a search that completed.  *Relations* (`monotone`, `ceiling`, `prime`,
//!   `repeat`): the bounds of one board give bounds of its neighbours, of the board past or before a prime, and of
//!   the boards with the same search problem, applied to a fixpoint after every change (heuristics/THEORY.md, "The
//!   ledger: bounds between boards").  *Given* games (`given`): the completed games of `--proven FILE` (a results
//!   file) in A − 1..B, each replayed, are fixed points.  `trivial`: a board with nothing to promote.
//! * **The searches.**  Of a repeat class (the boards whose search problems are equal) only its smallest board is
//!   searched, and no prime whose board before it the ledger has (the prime formula gives its bounds both ways).  A
//!   search is `taxman::search::search` on one thread, as `taxman solve` searches: seeded with the best of the games
//!   the ledger holds for n − 1, n and n + 1 (the one whose candidates keep the largest sum), with the ceiling U(n),
//!   so that a game that reaches U proves itself, and limited in time: `--base` seconds (default 2) the first time,
//!   twice as long each time after, at most `--max-limit` (default 3600) and at most the budget left over the workers.
//!   Each search goes on from the checkpoint of the last (DIR/ckpt/nN.ckpt), so its work adds up.
//! * **The order** (`schedule.rs`): first the map of every board searched, all of them on the workers at once, their
//!   bounds kept as they come, and when the last is in the relations carry them to the other boards in one go
//!   (`settle`: a map's cascade through the boards, one map at a time, would make every board many times over);
//!   then the climbs (below); then the boards whose bounds a proven neighbour has changed since their last search (the
//!   ceiling and the prime formula of a board next to a proven one are exact: such a board is often proven by a search
//!   at its ceiling in a few steps), smallest gap first; then the board of least `level + gap / gap_unit`, where
//!   `level` counts its searches (each doubles the limit) and `--gap-unit` (default 175) is the gap worth one doubling
//!   of the cost (the chain's steps rise a decade with about 580 of the standalone gap, `bench/landscape/`).
//!   `--workers` searches at once (default all CPUs).  When a board is proven, its search is stopped if one is going,
//!   and its checkpoint deleted; a search whose U falls is given the lower ceiling.
//! * **Wedge and climb.**  The order above finds wedges: boards proven with no help from a neighbour.  A search whose
//!   board below is proven is far cheaper (the ceiling opt(n) <= opt(n − 1) + n is exact in about a third of the
//!   games, and the proven game is the optimum's start about 88% of the time), so the ledger *climbs* from a wedge.  A
//!   board that is searched, not proven, not parked, and has its board below proven (however that was proven, and
//!   however many boards between were settled by the relations) gets a climb *up* ahead of every other job: its search,
//!   as above (so seeded with the proven game), for `--climb-limit` seconds (default 60; 0: no climbs), resumed from its
//!   checkpoint if it has one, and not counted among its ordinary searches (its `level` and so its next limit stay).
//!   If the climb proves the board, the board above it is climbed to next, and so on.  A board with its board *above*
//!   proven gets a climb *down*, after the climbs up: seeded with that game (the game below it that Lemma L2 makes from
//!   it is the optimum for n − 1 in about 35% of the composite pairs), but with the ledger's ceiling, which is loose
//!   (opt(n − 1) <= opt(n)); a proof brings the climb to the board below it.  `--climb` picks the ways (`up`, `down`,
//!   `both`, the default, or `off`).  If a climb stops at its limit the board is *parked* (`parked` in the ledger): it
//!   goes by the ordinary order, with its checkpoint, and the climb from that wedge ends.  Climbs from different wedges
//!   run side by side.  A board next to one that is being climbed to waits (no ordinary search of it starts), as the
//!   climb may prove that board within its limit.
//! * **The finder** (`common/lns.rs`, which `wall finders` measures; `--lns-seconds` seconds of one thread, default
//!   1; 0: none): large neighbourhood search, which keeps a random 70-90% of a game's promotions and searches the
//!   rest exactly, again and again.  It is not sound and proves nothing, but a lower bound needs only a legal game:
//!   its game is offered as the board's L (`lns`) after the referee has replayed it on the worker, like every other
//!   game.  It runs on the worker, after a board's map from the start incumbent, and before a search whose best seed
//!   has changed since it last ran on the board (`lns_score` in the ledger: the score of the best seed then, its own
//!   game included; a climb, whose seed is a proven neighbour's game, always counts as changed), from that seed, and
//!   not at all if the seed already reaches the ceiling.  The search is then seeded with its game if that is better.
//!   Its time counts in the CPU budget and in the board's `search_s`; the search's limit is the search's alone.
//! * **The main thread** applies every result, and a worker whose search has ended waits for it: so what it does for a
//!   result is small beside a search, `settle` aside.  A relation's upper bound is arithmetic; its lower bound is a game,
//!   made once for each lower bound of its source, not at all when its score cannot beat the target's (the prime
//!   exchange scores exactly p − q more or less, the game of n − 1 followed by n at most n more), and shown to the
//!   referee only if the sum of its moves beats the target's.  The run's status line and its `end` event say how long
//!   the main thread was busy and how long results waited for it.
//!
//! The files of DIR: `ledger.jsonl`, one line a board in increasing n, written every `--save-every` seconds (default
//! 30) and at the end: `n`, `in_the_bag`, `prime`, `class` (the smallest board of its repeat class, or null), `lower`
//! (`value`, `rule`, `from`: the board a relation took it from, `moves`, `promotions`: those of a game made from
//! promotions, else null), `upper` (`value`, `rule`, `from`), `gap`, `proven`, `proven_cpu_s` (the run's CPU seconds
//! when it was proven), `attempts`, `level`, `search_s` and `nodes` (its searches, climbs among them: their number,
//! the number of the ordinary ones, their seconds, the finder's included, and their steps over all runs), `lns_score`
//! (see the finder), `mapped` (its own map is made), `boosted` (a proven neighbour changed its bounds since its last
//! search) and `parked` (a climb to it ended unproven).  `events.jsonl`, one line an event, appended: each has `t`
//! (seconds since the run began) and `cpu` (the CPU seconds of all runs); a
//! run's `start` (its settings), an `init` or `search` of board `n` (`limit_s`, `seconds`, `nodes` of this search,
//! `completed`, `reached_cap`, `found` (the score of its game), `dual` (its dual bound, null if it completed), the
//! bounds `before` and `after`, each [L, U]; `lns_s` (the finder's seconds, 0 if it did not run), `lns_from` and
//! `lns_found` (the scores of the game it began from and of its game, null if it did not run), `seed_from` (which game
//! seeded the search: `n-1`, `n`, `n+1` or `lns`, null for none) and `below_proven` and `above_proven` (whether the
//! boards n − 1 and n + 1 were proven when the job started); a search also has its `job`, `ordinary` or `climb`, and a
//! climb its `way` (`up` or `down`, `up` if the board is next to a proven one both ways) and its `outcome`: `proved`
//! (its search proved the board), `parked`, or `moot` (a relation had proven the board while it
//! went on)), a `derive` (a relation changed `side`, `lower` or `upper`, of board
//! `n` from `before` to `after`, by `rule` from the board `from`; those of a result all carry its time), a `settle`
//! (the relations carried the `boards` of the maps to the others: `changes`, `seconds`), a `proven` board (`value`, and
//! the rules and sources of its two bounds), and the run's `end` (`reason`, `proven`, `games`, and the main thread's
//! `main_s` busy, `main_longest_s` for the longest piece, and `waited_s`: the seconds that results waited in all).
//! The events are written out after each result.  `ckpt/`: the checkpoints of the searches that have not finished.
//! One run at a time uses DIR (`taxman.lock`).
//!
//! A bound that the relations make is checked as it is made: a game must pass the referee (`replay_finished`), and
//! L > U anywhere ends the run with an error (a bug, or a counterexample to statement U, on which every U of the frame
//! model rests).  The run never reads optimal.json or a results file but `--proven`: the truth stays hidden from it.
//!
//! **report** prints what a run did: the boards proven and how (the rules of their two bounds), the curve of boards
//! proven against CPU, the gaps left, and the check of every board against the known optima (optimal.json and the
//! completed games of each `--known FILE`): every game replays to its L, L <= opt <= U, and a proven value is the
//! optimum.  It exits with status 1 if the check finds anything wrong.
//!
//! **The bounds store** (`store.rs` the line and its checks, `export.rs`, `verify.rs`; results/README.md, "bounds.jsonl",
//! has the format).  The bounds a run finds are kept in a file of their own, `results/bounds.jsonl`, one JSON line a
//! bound, appended and never changed; a line is added only when it improves the file's best bound of its board and
//! side, and a board is proven when its best lower bound is its best upper bound.  **export** appends to `--bounds`
//! (created if it is not there) the bounds of the run in DIR that improve on its best, for the boards that `--strict`
//! (default results/strict.jsonl) has not proven, and prints what it appended and which boards are newly proven: a
//! lower bound with its game (the promotion set when the game was made from one; cited, the line of the game it was
//! carried from, when a relation carried it), an upper bound with its rule, the board and value it was derived from,
//! and the boards at which it assumes statement U; `--source` names the run,
//! `--checkpoints` the place (a prefix, with `nN.ckpt` under it) of the checkpoints of the searches that gave the dual
//! bounds.  **run --bounds** starts a run from the file's best bounds of its stretch, each game replayed by the
//! referee, and a board with both bounds there is not mapped.  **verify-bounds** is the Rust referee of the file (see
//! `verify.rs`); **games** writes the moves of its promotion sets for the Python referee, tools/check_bounds.py.

#[allow(dead_code, unused_imports)] // the ledger reads only the process's CPU time
#[path = "../common/counters.rs"]
mod counters;
#[allow(dead_code)] // the ledger reads only `survivors`
#[path = "../common/known.rs"]
mod known;
/// The finder, shared with `wall finders`.
#[allow(dead_code)] // `wall finders` reads the counts of units that the ledger does not
#[path = "../common/lns.rs"]
mod lns;
#[path = "../common/rng.rs"]
mod rng;
/// The helpers of the tests (temporary directories), shared with the other tests.
#[cfg(test)]
#[path = "../../tests/common/support.rs"]
mod support;

mod book;
mod export;
mod pool;
mod report;
mod schedule;
mod store;
mod verify;

use schedule::Settings;
use std::path::PathBuf;

/// The committed chain, whose games the bounds store cites and leaves out.
const STRICT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/results/strict.jsonl");

fn usage() -> ! {
    eprintln!(
        "usage: ledger run A-B --dir DIR --cpu-hours H [--workers P] [--base S] [--max-limit S] \
         [--climb up|down|both|off] [--climb-limit S] [--lns-seconds S] [--gap-unit G] [--proven FILE] \
         [--bounds FILE] [--save-every S]\n       ledger report DIR [--known FILE]...\n       \
         ledger export DIR --bounds FILE --source NAME [--checkpoints PREFIX] [--strict FILE] [--date YYYY-MM-DD]\n       \
         ledger verify-bounds FILE [--strict FILE] [--workers P]\n       ledger games FILE [--workers P]"
    );
    std::process::exit(2)
}

/// The text of a flag's value that must not be missing.
fn text(value: Option<String>) -> String {
    value.unwrap_or_else(|| usage())
}

fn number<T: std::str::FromStr>(name: &str, value: Option<String>) -> T {
    let value = value.unwrap_or_else(|| usage());
    value.parse().unwrap_or_else(|_| {
        eprintln!("{name}: cannot read {value}");
        std::process::exit(2)
    })
}

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| usage());
    let mut positional = Vec::new();
    let (mut dir, mut proven, mut known, mut cpu_hours) = (None, None, Vec::new(), None);
    let (mut bounds, mut source, mut checkpoints, mut date) = (None, None, None, None);
    let mut strict = PathBuf::from(STRICT);
    let mut settings = Settings { signals: true, ..Settings::default() };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dir" => dir = args.next().map(PathBuf::from),
            "--proven" => proven = args.next().map(PathBuf::from),
            "--known" => known.push(PathBuf::from(text(args.next()))),
            "--bounds" => bounds = args.next().map(PathBuf::from),
            "--strict" => strict = PathBuf::from(text(args.next())),
            "--source" => source = args.next(),
            "--checkpoints" => checkpoints = args.next(),
            "--date" => date = args.next(),
            "--cpu-hours" => cpu_hours = Some(number::<f64>(&arg, args.next())),
            "--workers" => settings.workers = number(&arg, args.next()),
            "--base" => settings.base = number(&arg, args.next()),
            "--max-limit" => settings.max_limit = number(&arg, args.next()),
            "--climb-limit" => settings.climb_limit = number(&arg, args.next()),
            "--lns-seconds" => settings.lns_seconds = number(&arg, args.next()),
            "--climb" => {
                (settings.climb_up, settings.climb_down) = match args.next().as_deref() {
                    Some("up") => (true, false),
                    Some("down") => (false, true),
                    Some("both") => (true, true),
                    Some("off") => (false, false),
                    _ => usage(),
                }
            }
            "--gap-unit" => settings.gap_unit = number(&arg, args.next()),
            "--save-every" => settings.save_every = number(&arg, args.next()),
            flag if flag.starts_with("--") => usage(),
            _ => positional.push(arg),
        }
    }
    let result = match command.as_str() {
        "run" => {
            let ([range], Some(dir), Some(hours)) = (positional.as_slice(), dir, cpu_hours) else { usage() };
            let (first, last) = match range.split_once('-') {
                Some((a, b)) => (number::<u32>("A", Some(a.to_owned())), number::<u32>("B", Some(b.to_owned()))),
                None => (number::<u32>("N", Some(range.clone())), number::<u32>("N", Some(range.clone()))),
            };
            if first < 2
                || last < first
                || settings.workers == 0
                || settings.base <= 0.0
                || settings.climb_limit < 0.0
                || settings.lns_seconds < 0.0
                || settings.gap_unit <= 0.0
            {
                usage();
            }
            settings.cpu_budget = hours * 3600.0;
            settings.bounds = bounds;
            schedule::run(first, last, &dir, &settings, proven.as_deref()).map(|reason| println!("ledger: {reason}"))
        }
        "report" => {
            let [dir] = positional.as_slice() else { usage() };
            match report::report(&PathBuf::from(dir), &known) {
                Ok((text, ok)) => {
                    print!("{text}");
                    if !ok {
                        std::process::exit(1);
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        "export" => {
            let ([run_dir], Some(bounds), Some(source)) = (positional.as_slice(), bounds, source) else { usage() };
            let date = date.unwrap_or_else(export::today);
            if !store::is_date(&date) {
                usage();
            }
            let options = export::Options { source: &source, date: &date, checkpoints: checkpoints.as_deref() };
            export::export(&PathBuf::from(run_dir), &bounds, &strict, &options)
                .map(|summary| println!("ledger export: appended to {}: {summary}", bounds.display()))
        }
        "verify-bounds" => {
            let [file] = positional.as_slice() else { usage() };
            verify::verify(&PathBuf::from(file), &strict, settings.workers).map(|(text, ok)| {
                print!("{text}");
                if !ok {
                    std::process::exit(1);
                }
            })
        }
        "games" => {
            let [file] = positional.as_slice() else { usage() };
            let mut out = std::io::BufWriter::new(std::io::stdout().lock());
            verify::games(&PathBuf::from(file), &mut out, settings.workers)
                .map(|count| eprintln!("ledger games: {count} games"))
        }
        _ => usage(),
    };
    if let Err(e) = result {
        eprintln!("ledger: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;
