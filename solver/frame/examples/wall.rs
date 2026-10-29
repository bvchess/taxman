//! The wall regime: how the solver's costs grow with `n` and with the size of a search tree.  What a
//! game of the chain costs is its search, which grows with the tree, plus everything around the search, which grows
//! with `n` alone (the tables, the frames, the model, the GF(2) frame vectors of the root, the replays); the commands
//! measure the two apart.  The study, its data and its conclusions are in
//! solver/notebook/search/the-wall-regime-2026-10/README.md.
//!
//! ```text
//! cargo run --release --example wall -- setup [--known FILE] [--repeat R] [--width W] N|A-B...
//! cargo run --release --example wall -- nodes [--known FILE] [--nodes K] [--seconds S] [--mode start|opt] [--delta D]
//!     [--width W] [--no-seed] [--tactic NAME=VALUE]... [--ckpt-dir DIR --every SECONDS] N...
//! cargo run --release --example wall -- grow [--known FILE] [--delta D] [--nodes K] [--seconds S] [--progress S]
//!     [--tactic NAME=VALUE]... [--ckpt-dir DIR --every SECONDS] [--threads T] [--cores C] [--mem-cap UNITS] N
//! cargo run --release --example wall -- start [--known FILE] N|A-B...
//! cargo run --release --example wall -- frames N|A-B...
//! cargo run --release --example wall -- choice-prepare [--cpu S] [--steps K] [--fixtures DIR] [--suite FILE]
//!     [--keep K] [--paths K] [--seed S] [--fix LO-HI] [--unit-nodes K] [--fit-seconds S] [--fits DIR]
//!     --prep DIR --known FILE N|A-B...
//! cargo run --release --example wall -- choice [--node-steps K] [--cap S] [--sub-nogoods] [--a1] [--a1-pricings K]
//!     [--a1-seconds S] --prep DIR [--results DIR --worker K] [--claims DIR] N|A-B...
//! cargo run --profile test --example wall --features studies -- decompose --known FILE [--fixtures DIR [--keep K]]
//!     [--pauses P --span STEPS] [--unit-cap STEPS] [--best-cap STEPS] [--and-or-cap K [--and-or-min STEPS]] N|A-B...
//! cargo run --profile test --example wall --features studies -- anatomy --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) [--learn [--closure]] [--trace DIR] [--prefer FILE] [--best-cpu S] [--nodes K]
//!     [--tactic NAME=VALUE]... N
//! cargo run --profile test --example wall --features studies -- cycles [--known FILE] [--suite FILE] [--units K]
//!     [--seconds S] [--out FILE] N...
//! cargo run --profile test --example wall --features studies -- exchanges --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) [--prune] [--below D] [--sample K] [--nodes K] [--tactic NAME=VALUE]... N|A-B...
//! cargo run --profile test --example wall --features studies -- near --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) --within D [--nodes K] [--probes K [--probe-gap G] [--seed S]] [--sets DIR] [--tactic NAME=VALUE]... N
//! cargo run --profile test --example wall --features studies -- images --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) [--learn [--closure] | --bare | --count] [--trace DIR] [--nodes K] [--tactic NAME=VALUE]... N
//! cargo run --profile test --example wall --features studies -- local --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) [--nodes K] [--checked SETS] [--per-unit K] [--hybrid-cap STEPS] [--compare] [--tactic NAME=VALUE]...
//!     N
//! cargo run --profile test --example wall --features studies -- local --compare N|A-B...
//! ```
//!
//! The example is a study tool: it is built with the feature `studies` (`--features studies`; Cargo.toml).
//!
//! Games are set up as `taxman solve` sets them up (`taxman::board::Board` at the set width `with_width!` picks for
//! `n`, the search tactics named as for `taxman solve --tactic`).  A board with nothing to promote prints
//! `{"n":N,"trivial":true}` and is skipped.  `--known FILE` (every command but `frames` and `choice`) is a results
//! file, such as `results/optimal.jsonl.gz`, whose completed games extend the known games beyond the table (as the
//! ladder's `--known` does), which is how a game gets its optimum, and the game of `n - 1` for its seed and ceiling.  A
//! range `A-B` stands for every `n` in it, none filtered out (every command but `nodes` and `grow`).  Each command
//! prints one JSON object per line on stdout; a mistake in the arguments is an error on stderr and exit status 2.
//!
//! * **setup**: what a game of the chain costs besides its search, piece by piece, each timed `--repeat` times
//!   (default 1) and reported as the fastest, for every `n` (primes too, `prime` says which).  The pieces: the
//!   number-theory tables (`tables_s`), the frames (`frames_s`), the game with no promotions that `Board::new` adds
//!   to them (`plain_s`) and the board as a whole (`board_s`); `prepare_s`, the search's model and greedy incumbent
//!   (`model_s` and `greedy_s` are its two phases); `problem_s`, the canonical search problem and its digest;
//!   `search_problem_s`, the same problem built from `n` alone, which is what the tactic `same_search`
//!   does for the earlier game and what the results check does for each repeat line and the game it repeats;
//!   `match_graph_s`, the matching graph; `improve_s`, the local search from the greedy set; `zero_search_s`, the
//!   search stopped before its first node (the search's own setup: matching graph, seed, dive, local search, the
//!   GF(2) graph, the engine started and stopped), whose result gives `start`, the incumbent a search starts from,
//!   and `capped`, whether the ceiling from the game of `n - 1` already ends the search; the root's GF(2) bound
//!   (below); `moves_s` and `replay_s`, the promotions made a game and the referee's replay of it (`replay_ok`);
//!   and `prime_path_s`, the prime shortcut's own work besides its replays.  Then the sizes (below), `opt` (the
//!   known optimum promotion sum, or null), `start`, `root_bound`, the gaps `gap_start` and `gap_opt` of the root's
//!   bound over `start` and over `opt`, and `max_rss` (the process's peak resident set, bytes).  `--width W` sets the
//!   game up at that set width instead of the one `with_width!` picks, as for `nodes`, to show what the width costs the
//!   setup.
//! * **nodes**: a one-thread search of each game, stopped after `--nodes K` steps (default 5000) or `--seconds S`,
//!   for the cost per node at the root-sized part of a tree.  `--mode start` (the default) is the solver's own
//!   search from its start incumbent (greedy set, seed from the game of `n - 1`, dive, local search; `--no-seed`
//!   drops the seed and the ceiling).  `--mode opt` is the proof that `prove` of the example `matroid` (removed from
//!   master) ran: the engine's incumbent is the value `opt + delta` (`--delta`, default 0) with no set behind it, no
//!   seed and no ceiling, so it needs the known optimum.  `--width W` runs the search at a set width of the list the
//!   solver is built for, instead of the one `with_width!` picks, to show that the width changes the time and nothing
//!   else: the same search at several widths has to give the same nodes and GF(2) counters.  `--ckpt-dir DIR --every
//!   SECONDS` checkpoints to `DIR/wall_n<N>_w<W>.ckpt` as often as that, and reports the size of the last checkpoint
//!   (`ckpt_bytes`; the final one, as a node limit stops the run) before deleting it.  Each line reports the sizes,
//!   the root's GF(2) bound (computed after the search, apart from it), the outcome and the search's cost: wall
//!   seconds, the engine's busy time, the cycles and instructions of the process, the promotion and GF(2) counters,
//!   the cost per node and per augmentation, and the engine's counters (below).
//! * **grow**: the emulation of a large tree.  A search whose incumbent starts `--delta D` below a known optimum
//!   (`D <= 0` usually) behaves like a search with a larger gap, so it makes a much larger tree on a known game:
//!   the proof of `nodes --mode opt` with the incumbent at `opt + D`, on `--threads T` threads (default 1; `--cores
//!   C` lets at most C of them search at once), until it completes or `--nodes K` or `--seconds S` stops it.
//!   `--tactic` sets search tactics as for `nodes`; `gf2=off,lagrange=on,symmetry=off` (the matching path, whose nodes
//!   are ten times cheaper and whose trees a hundred times larger) makes trees of ten million nodes in minutes, for
//!   what the engine and the nogood tables do in a large tree.  The engine prints its progress lines to stderr every
//!   `--progress S` seconds (default 2), and a monitor thread prints on stdout at the same pace
//!   `{"t":SECONDS,"max_rss":BYTES,"ckpt_bytes":SIZE,"spill_files":COUNT}` for the memory, the checkpoint and the spill
//!   directory.  With `--ckpt-dir DIR` the run checkpoints to `DIR/grow_n<N>.ckpt` every `--every` seconds and spills
//!   its queue to `DIR/spill` beyond `--mem-cap` units.  One summary line closes the run, with all the engine's
//!   counters.
//! * **start**: what the search's starting incumbent costs and gains, piece by piece, for each `n` that has a known
//!   game of `n - 1` (any other has the line `{"n":N,"no_previous":true}`).  `greedy` is the sum of the greedy set;
//!   `seed_kept` the sum of the seed that survives (the promotions of the game of `n - 1` that are candidates here,
//!   promoted one by one from the largest down, skipping the ones that fail) and `seed_kept_s` the time that took;
//!   `base` the sum of the better of the two, with `members` the number of promotions in it (the solver also tries a
//!   matching dive, which is private to the library and not reproduced here).  `ceiling` is the ceiling on the
//!   promotion sum that the game of `n - 1` gives, and `at_ceiling` says whether `base` already reaches it.  Then the
//!   local search from `base`: `singles` is the sum it finds when it takes out one member at a time, `pairs` the sum
//!   when it takes out two as well (the solver's call), each with its time in `singles_s` and `pairs_s`.  `opt` is the
//!   known optimum promotion sum, or null, and `max_rss` the process's peak resident set.
//! * **frames**: the frames of any board the solver supports (up to 8191), at the width it picks: the frames the
//!   board builder makes (before `FrameGraph::build` reduces them, which keeps every frame's freedoms), the time it
//!   took, and the sizes the later stages would need: per frame the factors, moves and freedoms, `max_freedoms`
//!   and the `block_words` of the GF(2) bound's vectors it needs (`max_freedoms.div_ceil(64)`).
//! * **landscape**: which games are easy and which will be expensive, game by game and without solving a chain in
//!   order.  Each game is taken standalone, as `taxman solve N` takes it: no game of `n - 1`, so no seed, no ceiling
//!   and no prime formula.  The line has the two ends of what its search would have to close, as scores: `upper`,
//!   `in_the_bag` plus the root's GF(2) bound with the default tactics, which is the bound the search computes at its
//!   first node before it branches; and `lower`, the game the search starts from (the better of the greedy set, whose
//!   score is `greedy`, and the matching dive, improved by the local search: `search::start_incumbent`).  Their
//!   difference is `gap`, the standalone root gap, and `meets` says whether they are equal, which settles the game at
//!   the first node.  Besides: `prime`, `trivial` (nothing to promote: both ends are `in_the_bag`), `frames`,
//!   `candidates` and `max_total`, as the setup line of `solve` counts them, and the times in seconds: `setup_s` (the
//!   tables, the frames, the model and the greedy set, `greedy_s` of it), `start_s` (the matching graph, the dive and
//!   the local search), `bound_s` (the GF(2) graph, `graph_s` of it, and the bound), `total_s` and `cpu_s` (the CPU
//!   time of the game's thread, which a load on the other threads moves less than the wall time).  Of a game with a
//!   known optimum (the table, and `--known FILE`'s completed games), `opt` is its score, `opt_proof` the proof of
//!   its line in `--known FILE` (null if that has none), `root_gap` is `upper - opt` and `shortfall` is `opt - lower`
//!   (the gap is their sum); all four are null for a game with no known optimum.  `--threads K` plays K games at
//!   once, one a thread (default 1), in the order given.  Each line is printed, and with `--out FILE` appended to it,
//!   as its game finishes, so in the order of finishing; the games that FILE has are skipped, so a run that was
//!   interrupted goes on where it was (a line cut short at the end of FILE is cut off).
//! * **choice-prepare** and **choice**: at which nodes is a slower, tighter bound (the Lagrangian bound over the
//!   nogoods, refitted at the node, and the multi-field bound of `wall/fields.rs` in two forms) worth computing besides
//!   today's?  `choice-prepare` makes a game's file (its nogood pool, its root multipliers, its A1 root fit, its
//!   sampled nodes) and `choice` measures each node of the file: its gap, the slower bounds and whether they cut, the
//!   subtree today's search makes of it, and the times.  Defined in `wall/choice.rs`, whose module comment has the
//!   options and the lines; `solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/` runs them in
//!   the cloud and reads the lines.
//! * **decompose**: do the conflicts of a node that heads a big subtree split, across some home, into two sides that do
//!   not interact (A5, the decomposition across the frames)?  For the nodes of a game's proof (the probe's fixtures,
//!   the open units of pauses of the proof, the root): how the conflicts of the node's heaviest set and the nogoods
//!   its subtree learns lie against each separator home, how the node's gap splits by home, the two sides solved
//!   apart under the separator of the node's best completion, and the AND/OR search itself.  Defined in
//!   `wall/decompose.rs`, whose module comment has the options and the lines;
//!   `solver/notebook/search/decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/` reads them.
//! * **anatomy**: units of the tuning suite, or whole games, solved with a hook on every step of the searcher, which
//!   writes what each step saw and did (its bound, conflict, branch and how it ended, and the residues of the frames),
//!   where each known conflict came from (a mirror of the nogood table), the nogoods the searcher learned, and
//!   optionally with a branching order of its own and the learning of `images`; defined in `wall/anatomy.rs`, whose
//!   module comment has the options and the files.
//!   `solver/notebook/search/the-anatomy-of-a-hard-subtree-2026-10/` reads them.
//! * **cycles**: do flat-cycle cuts close the gap the field-rank bounds leave?  The matching LP of a game's root and of
//!   its tuning-suite units, with cover, rank and flat-cycle cuts, against the GF(2) bound; defined in
//!   `wall/cycles.rs`, whose module comment has the options and the stages, for
//!   `solver/notebook/search/flat-cycle-cuts-on-the-matching-lp-2026-10/`.
//! * **exchanges**: units of the tuning suite, or whole games, solved with a hook that checks at every exclusion
//!   whether an exchange of two primes that maps the member onto another candidate is a symmetry of the node, and with
//!   `--prune` prunes by the ones that are; defined in `wall/exchanges.rs`, whose module comment has the check, the
//!   rule and the lines.  `solver/notebook/search/local-prime-exchanges-at-the-nodes-2026-10/` reads them.
//! * **near**: how many completions of a unit of the tuning suite, or of a whole game, come within `--within D` of
//!   P*, by value, counted exactly by the search made to count (`search::StepHook::counting`) or estimated from
//!   random descents; defined in `wall/near.rs`, whose module comment has the count, the estimate and the files.
//!   `solver/notebook/search/completions-near-the-optimum-in-the-hard-subtrees-2026-10/` reads them.
//! * **images**: units of the tuning suite, or whole games, solved with a hook that, wherever a node goes on without a
//!   member and its bound swaps in the member's twin (a twin swap or a paired exchange), asks whether the node's
//!   conflict with the same swap made is a conflict too, and with `--learn` makes the search learn those images;
//!   `--count` only counts the tree's steps and leaves; defined in `wall/images.rs`, whose module comment has the test,
//!   the learning, the lines and the files.
//!   `solver/notebook/search/conflicts-closed-under-cheap-exchanges-2026-10/` reads them.
//! * **local**: is the GF(2) bound's error local?  Units of the suite, or whole games, solved with a hook that records
//!   the nodes that hold the tree and how often the local situation of a few frames recurs; at a sample of those
//!   nodes, the *hybrid bound* that is exact in a set of frames and GF(2) in the others, found by a search of a relaxed
//!   model; defined in `wall/local.rs`, whose module comment has the bound, the options and the lines.
//!   `solver/notebook/search/a-bound-exact-in-a-few-frames-2026-10/` reads them.
//!
//! The commands `chain`, `sound`, `finders`, `nogoods`, `target` and `relaxed` of the finished experiments were removed
//! from master (solver/notebook/README.md, "How this notebook is kept", says where they build).
//!
//! **The sizes** (`setup`, `nodes` and `frames`): `frames` (the root's frame count), `candidates`, `max_total`
//! (the most promotions a set can have), `members` (the distinct numbers in the moves and factors of the root's
//! frames and in the candidates), `root_max` (the largest of them), `words_n` (`words_for(n)`), `width` (the words
//! per set the game ran at), `words_members`, `words_candidates` and `words_root_max` (the words a set of that
//! many numbers, or of numbers up to `root_max`, would need), and per frame the numbers of moves, factors and
//! freedoms (`frame_moves`, `frame_factors`, `frame_freedoms`; `moves`, `factors` and `freedoms` for `frames`).
//!
//! **The root's GF(2) bound** (`setup` and `nodes`): the time to build the root's frame vectors (`gf2_graph_s`), their
//! `elements`, per-frame `ranks` and `block_words`; the bound with no incumbent (`root_bound_s` for the reset and
//! the evaluation, `root_bound`, `root_chosen` the size of its heaviest set, `root_augmentations`); and the same
//! evaluation asked for the members forced by an incumbent of the start value (`root_split_s`, `root_forced`).
//!
//! **The engine's counters** (`nodes` and `grow`; each is defined at `engine::Counters`): the pauses, checkpoints and
//! queue (`pauses`, `pause_total_s`, `pause_max_s`, `park_max_s`, `checkpoints`, `max_queue`, `spilled_units`); the
//! units run (`runs`, `tiny_runs`); the workers started and retired after the first ones (`spawned`, `retired`) and
//! the workers' lifetimes summed (`capacity_s`, which is `seconds` × threads when their number did not change); and
//! how the time went besides the steps: the idle time before all workers first ran at once (`ramp_idle_s`), the wall
//! time and the idle time after the last moment a worker finished a unit while all were running (`tail_s`,
//! `tail_idle_s`), and the units' starts, the time to rebuild a unit's state (`rebuild_s`) and the time of the first
//! node of the units that had one (`first_step_s`, over `first_steps` units).

/// `anatomy`: a hard subtree's steps, told by a hook on the searcher.
#[path = "wall/anatomy.rs"]
mod anatomy;
/// The game that `choice-prepare` and `choice` play: their options, budgets and the play of a command.
#[path = "wall/case.rs"]
mod case;
/// `choice-prepare` and `choice`: the Lagrangian bound, refitted at a node, against the node's subtree in today's
/// search.
#[path = "wall/choice.rs"]
mod choice;
#[path = "common/counters.rs"]
mod counters;
/// `cycles`: the matching LP with cover, rank and flat-cycle cuts, against the GF(2) bound.
#[path = "wall/cycles.rs"]
mod cycles;
/// `decompose`: whether the conflicts of a node split across a separator home (A5's cheapest experiment).
#[path = "wall/decompose.rs"]
mod decompose;
/// `exchanges`: local prime exchanges at the nodes of a search, checked and pruned by.
#[path = "wall/exchanges.rs"]
mod exchanges;
/// The multi-field bound (A1: independence in every frame over GF(2) and under GF(3) signings, fitted multipliers) at a
/// node, for `choice`.
#[path = "wall/fields.rs"]
mod fields;
/// `sample` reads the probe's `.nodes` fixtures and the tuning suite's hex sets.
#[path = "common/fixtures.rs"]
mod fixtures;
/// `images`: whether a node's conflict stays a conflict under the twin swap its bound makes for a member left out.
#[path = "wall/images.rs"]
mod images;
#[path = "common/known.rs"]
mod known;
/// `local`: how much of a node's gap a bound exact in a few frames, and GF(2) in the others, closes.
#[path = "wall/local.rs"]
mod local;
/// `near`: the completions of a unit near P*, counted or estimated.
#[path = "wall/near.rs"]
mod near;
/// The nogoods that the finder and the sound search learn, and the Lagrangian bound fitted over them.
#[path = "wall/nogoods.rs"]
mod nogoods;
/// The nodes of a game that `choice-prepare` samples: the probe's fixtures, the tuning suite's units and nodes on the
/// way to an optimal set.
#[path = "wall/sample.rs"]
mod sample;
/// The helpers of the tests (temporary directories), shared with the other tests.
#[cfg(test)]
#[path = "../tests/common/support.rs"]
mod support;

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};
use taxman::accelerate::solution_for_promotions;
use taxman::bits::{words_for, Bits};
use taxman::board::{search_problem, Board};
use taxman::engine::{self, Control, Counters, EngineConfig, Start, Stats};
use taxman::frames::{build_frames, BoardFrames};
use taxman::game::{replay_finished, Optimal};
use taxman::gf2::{Eval, Gf2Bound, Gf2Graph};
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::model::Scratch;
use taxman::numtheory::Tables;
use taxman::rng;
use taxman::search::finder as lns;
use taxman::search::{
    search, start_incumbent, stat, Diagnostics, Neighborhood, Prepared, SearchSpace, SearchTactics, TaxSearcher,
    TaxUnit,
};
use taxman::solver::MAX_N;
use taxman::tactics::Tactics;

// ---------------------------------------------------------------------------------------------------------
// Arguments

const COMMANDS: [&str; 15] = [
    "setup",
    "nodes",
    "grow",
    "start",
    "frames",
    "landscape",
    "choice-prepare",
    "choice",
    "decompose",
    "anatomy",
    "cycles",
    "exchanges",
    "near",
    "images",
    "local",
];

/// The options, with the commands each applies to (an option given to another command is an error).
const OPTIONS: [(&str, &[&str]); 64] = [
    (
        "--known",
        &[
            "setup",
            "nodes",
            "grow",
            "start",
            "landscape",
            "choice-prepare",
            "decompose",
            "anatomy",
            "cycles",
            "exchanges",
            "near",
            "images",
            "local",
        ],
    ),
    ("--cap", &["choice"]),
    ("--fixtures", &["choice-prepare", "decompose"]),
    ("--pauses", &["decompose"]),
    ("--span", &["decompose"]),
    ("--unit-cap", &["decompose"]),
    ("--best-cap", &["decompose"]),
    ("--and-or-cap", &["decompose"]),
    ("--and-or-min", &["decompose"]),
    ("--suite", &["choice-prepare", "anatomy", "cycles", "exchanges", "near", "images", "local"]),
    ("--units", &["cycles"]),
    ("--ids", &["anatomy", "exchanges", "near", "images", "local"]),
    ("--prune", &["exchanges"]),
    ("--below", &["exchanges"]),
    ("--root", &["anatomy", "exchanges", "near", "images", "local"]),
    ("--sample", &["exchanges"]),
    ("--learn", &["anatomy", "images"]),
    ("--bare", &["images"]),
    ("--count", &["images"]),
    ("--closure", &["anatomy", "images"]),
    ("--within", &["near"]),
    ("--probes", &["near"]),
    ("--probe-gap", &["near"]),
    ("--sets", &["near"]),
    ("--trace", &["anatomy", "images"]),
    ("--prefer", &["anatomy"]),
    ("--best-cpu", &["anatomy"]),
    ("--paths", &["choice-prepare"]),
    ("--keep", &["choice-prepare", "decompose"]),
    ("--prep", &["choice-prepare", "choice"]),
    ("--results", &["choice"]),
    ("--worker", &["choice"]),
    ("--claims", &["choice"]),
    ("--node-steps", &["choice"]),
    ("--sub-nogoods", &["choice"]),
    ("--a1", &["choice"]),
    ("--a1-pricings", &["choice"]),
    ("--a1-seconds", &["choice"]),
    ("--fit-seconds", &["choice-prepare"]),
    ("--fits", &["choice-prepare"]),
    ("--cpu", &["choice-prepare"]),
    ("--steps", &["choice-prepare"]),
    ("--seed", &["choice-prepare", "near"]),
    ("--fix", &["choice-prepare"]),
    ("--unit-nodes", &["choice-prepare"]),
    ("--repeat", &["setup"]),
    ("--nodes", &["nodes", "grow", "anatomy", "exchanges", "near", "images", "local"]),
    ("--seconds", &["nodes", "grow", "cycles"]),
    ("--mode", &["nodes"]),
    ("--delta", &["nodes", "grow"]),
    ("--width", &["setup", "nodes"]),
    ("--no-seed", &["nodes"]),
    ("--tactic", &["nodes", "grow", "anatomy", "exchanges", "near", "images", "local"]),
    ("--checked", &["local"]),
    ("--per-unit", &["local"]),
    ("--hybrid-cap", &["local"]),
    ("--compare", &["local"]),
    ("--ckpt-dir", &["nodes", "grow"]),
    ("--every", &["nodes", "grow"]),
    ("--progress", &["grow"]),
    ("--threads", &["grow", "landscape"]),
    ("--out", &["landscape", "cycles", "local"]),
    ("--cores", &["grow"]),
    ("--mem-cap", &["grow"]),
];

/// The node limit of `nodes` when `--nodes` is not given.
const DEFAULT_NODES: u64 = 5000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The solver's own search, from its start incumbent.
    Start,
    /// The proof of the known optimum (the incumbent a value).
    Opt,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Start => "start",
            Mode::Opt => "opt",
        }
    }
}

struct Options {
    command: String,
    /// A results file with more known games.
    known: Option<PathBuf>,
    repeat: usize,
    nodes: Option<u64>,
    seconds: Option<Duration>,
    mode: Mode,
    delta: Option<i64>,
    width: Option<usize>,
    no_seed: bool,
    tactics: SearchTactics,
    ckpt_dir: Option<PathBuf>,
    every: Option<Duration>,
    progress: Duration,
    threads: usize,
    cores: Option<usize>,
    mem_cap: Option<usize>,
    /// The file `landscape` appends its lines to.
    out: Option<PathBuf>,
    /// The suite units of a game that `cycles` measures (all of them without `--units`).
    units: Option<usize>,
    /// The options of `choice-prepare` and `choice`.
    params: case::Params,
    /// The options of `anatomy` (and its `--ids`, of `exchanges`; its `--root` is `exchanges'`, its `--learn` and
    /// `--closure` are `images'`).
    anatomy: anatomy::Params,
    /// The options of `exchanges`.
    exchanges: exchanges::Params,
    /// The options of `near` (its `--ids` are `anatomy`'s, its `--root` `exchanges'`, its `--seed` the finder's).
    near: near::Params,
    /// The options of `images` (its `--ids` and `--trace` are `anatomy`'s, its `--root` `exchanges'`).
    images: images::Params,
    /// The options of `local` (its `--suite` is `choice-prepare`'s, its `--ids` `anatomy`'s, its `--root`
    /// `exchanges'`).
    local: local::Params,
    games: Vec<u32>,
}

impl Options {
    /// The options of `command` with every option at its default and no games.
    fn new(command: String) -> Options {
        Options {
            command,
            known: None,
            repeat: 1,
            nodes: None,
            seconds: None,
            mode: Mode::Start,
            delta: None,
            width: None,
            no_seed: false,
            tactics: SearchTactics::default(),
            ckpt_dir: None,
            every: None,
            progress: Duration::from_secs(2),
            threads: 1,
            cores: None,
            mem_cap: None,
            out: None,
            units: None,
            params: case::Params::default(),
            anatomy: anatomy::Params::default(),
            exchanges: exchanges::Params::default(),
            near: near::Params::default(),
            images: images::Params::default(),
            local: local::Params::default(),
            games: Vec::new(),
        }
    }
}

fn number<T: std::str::FromStr>(name: &str, text: String) -> Result<T, String>
where
    T::Err: Display,
{
    text.parse().map_err(|e| format!("{name} {text}: {e}"))
}

/// A duration given in (fractional) seconds.
fn duration(name: &str, text: String) -> Result<Duration, String> {
    let seconds: f64 = number(name, text)?;
    Duration::try_from_secs_f64(seconds).map_err(|e| format!("{name} {seconds}: {e}"))
}

/// The games of the arguments: numbers, and for every command but `nodes` and `grow` also ranges `A-B`.
fn games(command: &str, texts: &[String]) -> Result<Vec<u32>, String> {
    let ranges = !matches!(command, "nodes" | "grow");
    let mut games = Vec::new();
    for text in texts {
        let number = |part: &str| part.parse::<u32>().map_err(|e| format!("{text}: {e}"));
        match text.split_once('-') {
            Some((a, b)) if ranges => {
                let (a, b) = (number(a)?, number(b)?);
                if a > b {
                    return Err(format!("{text}: the range is empty"));
                }
                games.extend(a..=b);
            }
            _ => games.push(number(text)?),
        }
    }
    if games.is_empty() {
        return Err("no games given".into());
    }
    match games.iter().find(|&&n| n < 2) {
        Some(n) => Err(format!("n={n}: the boards start at 2")),
        None => Ok(games),
    }
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or_else(|| format!("expected a command: {}", COMMANDS.join(", ")))?;
    if !COMMANDS.contains(&command.as_str()) {
        return Err(format!("unknown command {command:?}: {}", COMMANDS.join(", ")));
    }
    let mut o = Options::new(command);
    let mut texts = Vec::new();
    while let Some(arg) = args.next() {
        if arg.starts_with("--") {
            match OPTIONS.iter().find(|(name, _)| *name == arg) {
                None => return Err(format!("unknown option {arg}")),
                Some((_, commands)) if !commands.contains(&o.command.as_str()) => {
                    return Err(format!("{arg} does not apply to {}", o.command));
                }
                Some(_) => {}
            }
        }
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--known" => o.known = Some(value("--known")?.into()),
            "--repeat" => o.repeat = number::<usize>("--repeat", value("--repeat")?)?.max(1),
            "--nodes" => o.nodes = Some(number("--nodes", value("--nodes")?)?),
            "--seconds" => o.seconds = Some(duration("--seconds", value("--seconds")?)?),
            "--mode" => {
                o.mode = match value("--mode")?.as_str() {
                    "start" => Mode::Start,
                    "opt" => Mode::Opt,
                    other => return Err(format!("--mode {other}: start or opt")),
                }
            }
            "--delta" => o.delta = Some(number("--delta", value("--delta")?)?),
            "--width" => o.width = Some(number("--width", value("--width")?)?),
            "--no-seed" => o.no_seed = true,
            "--tactic" => {
                // The solver's parser, so that the names and values are those of `taxman solve --tactic`.
                let spec = value("--tactic")?;
                let mut tactics = Tactics { search: o.tactics, ..Tactics::default() };
                tactics.set(&spec).map_err(|e| format!("--tactic: {e}"))?;
                if tactics != (Tactics { search: tactics.search, ..Tactics::default() }) {
                    return Err(format!("--tactic {spec}: only search tactics apply"));
                }
                o.tactics = tactics.search;
            }
            "--ckpt-dir" => o.ckpt_dir = Some(value("--ckpt-dir")?.into()),
            "--every" => o.every = Some(duration("--every", value("--every")?)?),
            "--progress" => o.progress = duration("--progress", value("--progress")?)?,
            "--threads" => o.threads = number::<usize>("--threads", value("--threads")?)?.max(1),
            "--cores" => o.cores = Some(number("--cores", value("--cores")?)?),
            "--mem-cap" => o.mem_cap = Some(number("--mem-cap", value("--mem-cap")?)?),
            "--out" => o.out = Some(value("--out")?.into()),
            "--units" => o.units = Some(number("--units", value("--units")?)?),
            "--cap" => o.params.cap = Some(number("--cap", value("--cap")?)?),
            "--fixtures" => o.params.fixtures = Some(value("--fixtures")?.into()),
            "--suite" => o.params.suite = Some(value("--suite")?.into()),
            "--paths" => o.params.paths = number("--paths", value("--paths")?)?,
            "--keep" => o.params.keep = number::<usize>("--keep", value("--keep")?)?.max(1),
            "--pauses" => o.params.pauses = number("--pauses", value("--pauses")?)?,
            "--span" => o.params.span = number::<u64>("--span", value("--span")?)?.max(1),
            "--unit-cap" => o.params.unit_cap = Some(number("--unit-cap", value("--unit-cap")?)?),
            "--best-cap" => o.params.best_cap = number("--best-cap", value("--best-cap")?)?,
            "--and-or-cap" => o.params.and_or_cap = number("--and-or-cap", value("--and-or-cap")?)?,
            "--and-or-min" => o.params.and_or_min = number("--and-or-min", value("--and-or-min")?)?,
            "--prep" => o.params.prep = Some(value("--prep")?.into()),
            "--results" => o.params.results = Some(value("--results")?.into()),
            "--worker" => o.params.worker = number("--worker", value("--worker")?)?,
            "--claims" => o.params.claims = Some(value("--claims")?.into()),
            "--node-steps" => o.params.node_steps = number::<usize>("--node-steps", value("--node-steps")?)?.max(1),
            "--sub-nogoods" => o.params.sub = true,
            "--a1" => o.params.a1 = true,
            "--a1-pricings" => o.params.a1_pricings = number("--a1-pricings", value("--a1-pricings")?)?,
            "--a1-seconds" => o.params.a1_seconds = number("--a1-seconds", value("--a1-seconds")?)?,
            "--fit-seconds" => o.params.fit_seconds = number("--fit-seconds", value("--fit-seconds")?)?,
            "--fits" => o.params.fits = Some(value("--fits")?.into()),
            "--cpu" => o.params.cpu = number("--cpu", value("--cpu")?)?,
            "--steps" => o.params.steps = number::<usize>("--steps", value("--steps")?)?.max(1),
            "--seed" => o.params.lns.seed = number("--seed", value("--seed")?)?,
            "--fix" => o.params.lns.fix = case::fix_range(&value("--fix")?)?,
            "--unit-nodes" => o.params.lns.unit_nodes = number("--unit-nodes", value("--unit-nodes")?)?,
            "--ids" => o.anatomy.ids = value("--ids")?.split(',').map(str::to_owned).collect(),
            "--trace" => o.anatomy.trace = Some(value("--trace")?.into()),
            "--prefer" => o.anatomy.prefer = Some(value("--prefer")?.into()),
            "--best-cpu" => o.anatomy.best_cpu = number("--best-cpu", value("--best-cpu")?)?,
            "--prune" => o.exchanges.prune = true,
            "--below" => o.exchanges.below = number("--below", value("--below")?)?,
            "--root" => o.exchanges.root = true,
            "--sample" => o.exchanges.sample = number("--sample", value("--sample")?)?,
            "--learn" => o.images.learn = true,
            "--bare" => o.images.bare = true,
            "--count" => o.images.count = true,
            "--closure" => o.images.closure = true,
            "--within" => o.near.within = number("--within", value("--within")?)?,
            "--probes" => o.near.probes = number("--probes", value("--probes")?)?,
            "--probe-gap" => o.near.probe_gap = number("--probe-gap", value("--probe-gap")?)?,
            "--sets" => o.near.sets = Some(value("--sets")?.into()),
            "--checked" => o.local.checked = local::parse_checked(&value("--checked")?)?,
            "--per-unit" => o.local.per_unit = number("--per-unit", value("--per-unit")?)?,
            "--hybrid-cap" => o.local.hybrid_cap = number("--hybrid-cap", value("--hybrid-cap")?)?,
            "--compare" => o.local.compare = true,
            _ => texts.push(arg.clone()),
        }
    }
    if o.ckpt_dir.is_some() != o.every.is_some() {
        return Err("--ckpt-dir and --every go together".into());
    }
    if o.progress.is_zero() {
        return Err("--progress must be positive".into());
    }
    if o.command == "nodes" && o.mode == Mode::Start && o.delta.is_some() {
        return Err("--delta needs --mode opt".into());
    }
    o.games = games(&o.command, &texts)?;
    if o.command == "grow" && o.games.len() != 1 {
        return Err("grow takes one game".into());
    }
    Ok(o)
}

fn fail(error: &str) -> ! {
    eprintln!("error: {error}");
    std::process::exit(2);
}

fn beyond(n: u32) -> String {
    format!("n={n} is beyond the widest set the solver is built for")
}

/// The arms of a match on the set width `$width`: `$run::<w>($n, $o, $optimal)` for each listed width `w`, and an
/// error naming the widths for any other.
macro_rules! match_width {
    ($width:expr, $run:ident, $n:expr, $o:expr, $optimal:expr; $($w:literal)*) => {
        match $width {
            $($w => $run::<$w>($n, $o, $optimal),)*
            width => Err(format!("--width {width}: the widths are {:?}", [$($w),*])),
        }
    };
}

/// `$run` for game `$n` at the set width `--width` (`$o.width`) if it is given, and otherwise at the width that
/// `with_width!` picks for `$n`.  The widths `--width` may name are listed here, once, for `setup` and `nodes` alike.
macro_rules! at_width {
    ($run:ident, $n:expr, $o:expr, $optimal:expr) => {{
        let (n, o, optimal) = ($n, $o, $optimal);
        match o.width {
            None => taxman::with_width!(n, W => $run::<W>(n, o, optimal), else Err(beyond(n))),
            Some(width) if width < words_for(n) => {
                Err(format!("--width {width} is below the {} words that n={n} needs", words_for(n)))
            }
            Some(width) => match_width!(width, $run, n, o, optimal; 8 16 20 24 28 32 36 40 44 48 56 64 128),
        }
    }};
}

fn main() {
    let o = parse().unwrap_or_else(|e| fail(&e));
    let optimal = Optimal::built_in().with_results_files(o.known.as_slice()).unwrap_or_else(|e| fail(&e));
    if o.command == "landscape" {
        landscape(&o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    if o.command == "anatomy" {
        anatomy::run(&o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    if o.command == "cycles" {
        cycles::run(&o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    if o.command == "exchanges" {
        exchanges::run(&o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    if o.command == "near" {
        near::run(&o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    if o.command == "images" {
        images::run(&o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    if o.command == "local" {
        local::run(&o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    if let Some(command) = case::Command::named(&o.command) {
        case::run(command, &o, &optimal).unwrap_or_else(|e| fail(&e));
        return;
    }
    for &n in &o.games {
        let done = match o.command.as_str() {
            "setup" => at_width!(setup, n, &o, &optimal),
            "nodes" => at_width!(nodes, n, &o, &optimal),
            "grow" => taxman::with_width!(n, W => grow::<W>(n, &o, &optimal), else Err(beyond(n))),
            "start" => taxman::with_width!(n, W => start::<W>(n, &o, &optimal), else Err(beyond(n))),
            _ => frames_at_width(n),
        };
        done.unwrap_or_else(|e| fail(&e));
    }
}

// ---------------------------------------------------------------------------------------------------------
// What the commands share

/// One JSON object under construction (`{"n":N,...}`): each method appends a field and returns the line, to chain.
struct Line(Vec<String>);

impl Line {
    fn new() -> Line {
        Line(Vec::new())
    }

    /// A line that starts with `n`.
    fn of(n: u32) -> Line {
        let mut line = Line::new();
        line.field("n", n);
        line
    }

    fn field(&mut self, name: &str, value: impl Display) -> &mut Line {
        self.0.push(format!("\"{name}\":{value}"));
        self
    }

    fn text(&mut self, name: &str, value: &str) -> &mut Line {
        self.field(name, format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")))
    }

    /// A time in seconds.
    fn seconds(&mut self, name: &str, value: f64) -> &mut Line {
        self.field(name, format!("{value:.6}"))
    }

    /// The value, or `null`.
    fn maybe(&mut self, name: &str, value: Option<impl Display>) -> &mut Line {
        match value {
            Some(value) => self.field(name, value),
            None => self.field(name, "null"),
        }
    }

    /// `numerator / denominator`, or `null` for a zero denominator.
    fn ratio(&mut self, name: &str, numerator: f64, denominator: u64) -> &mut Line {
        let value = (denominator > 0).then(|| format!("{:.3}", numerator / denominator as f64));
        self.maybe(name, value)
    }

    fn list<T: Display>(&mut self, name: &str, values: impl IntoIterator<Item = T>) -> &mut Line {
        let items: Vec<String> = values.into_iter().map(|v| v.to_string()).collect();
        self.field(name, format!("[{}]", items.join(",")))
    }

    /// The line as JSON text.
    fn render(&self) -> String {
        format!("{{{}}}", self.0.join(","))
    }

    fn print(&self) {
        println!("{}", self.render());
    }
}

/// A game set up as the chain sets it up, with what is known of it.
struct Game<const W: usize> {
    board: Board<W>,
    p: Prepared<W>,
    /// The known optimum promotion sum (the known score minus `in_the_bag`).
    opt: Option<u64>,
    /// The promotions of the known game of `n - 1` that are candidates here.
    seed: Option<Bits<W>>,
    /// The ceiling on the promotion sum that the known game of `n - 1` gives.
    ceiling: Option<u64>,
}

impl<const W: usize> Game<W> {
    /// The game of `n`, or `None` (after printing `{"n":N,"trivial":true}`) for a board with nothing to promote.
    /// Without `with_seed` the game has neither a seed nor a ceiling.
    fn new(n: u32, tactics: SearchTactics, optimal: &Optimal, with_seed: bool) -> Option<Game<W>> {
        let board = Board::<W>::for_n(n);
        if board.is_trivial() {
            Line::of(n).field("trivial", true).print();
            return None;
        }
        let p = board.prepare(tactics).expect("prepare");
        let opt = optimal.score(n).map(|score| score - board.in_the_bag);
        let seed = optimal.moves(n - 1).filter(|_| with_seed).map(|moves| board.seed(moves));
        let ceiling = optimal.score(n - 1).filter(|_| with_seed).and_then(|score| board.promotion_cap(score));
        Some(Game { board, p, opt, seed, ceiling })
    }
}

/// The fastest of `repeat` runs of `f`, in seconds, and the value of that run.
fn fastest<T>(repeat: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut best: Option<(Duration, T)> = None;
    for _ in 0..repeat {
        let t0 = Instant::now();
        let value = f();
        let time = t0.elapsed();
        if best.as_ref().is_none_or(|(fastest, _)| time < *fastest) {
            best = Some((time, value));
        }
    }
    let (time, value) = best.expect("at least one run");
    (time.as_secs_f64(), value)
}

/// What a call cost: its wall time, and the cycles and instructions of the process during it (0 where the system
/// does not count them).
struct Call {
    wall: Duration,
    cycles: u64,
    instructions: u64,
}

fn measure<T>(f: impl FnOnce() -> T) -> (T, Call) {
    let before = counters::now();
    let t0 = Instant::now();
    let value = f();
    let wall = t0.elapsed();
    let after = counters::now();
    let (instructions, cycles) = match (before, after) {
        (Some((i0, c0)), Some((i1, c1))) => (i1 - i0, c1 - c0),
        _ => (0, 0),
    };
    (value, Call { wall, cycles, instructions })
}

/// A search call's outcome as far as the reports go, and what it cost.
struct Run {
    completed: bool,
    best: u64,
    stats: Stats,
    counters: Counters,
    call: Call,
}

/// The proof that `prove` of the example `matroid` (removed from master) ran: the engine searches `p` from the
/// incumbent value `best`, which has no set behind it, with no seed and no ceiling.  Only the engine's run is timed,
/// not the graphs it starts from.
fn proof<const W: usize>(p: &Prepared<W>, cfg: &EngineConfig, best: u64) -> Run {
    proof_collecting(p, cfg, best, None)
}

/// [`proof`], collecting the search's [`Diagnostics`] into `diagnostics` if given (which slows the search).
fn proof_collecting<const W: usize>(
    p: &Prepared<W>,
    cfg: &EngineConfig,
    best: u64,
    diagnostics: Option<&Mutex<Diagnostics>>,
) -> Run {
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace { diagnostics, ..SearchSpace::new(p, &graph) };
    let units = vec![TaxUnit { promoted: Bits::<W>::EMPTY, cands: p.candidates }];
    // The incumbent is a value only: its payload, an empty set, is never read back here.
    let start = Start::Fresh { units, best, payload: vec![0u8; 8 * W] };
    let (out, call) = measure(|| engine::run(cfg, start, b"wall", |_| TaxSearcher::new(&space)).expect("search"));
    Run { completed: out.completed, best: out.best, stats: out.stats, counters: out.counters, call }
}

/// The sizes of a root: what the sets, the GF(2) vectors and the frame count have to hold.
struct Sizes {
    n: u32,
    /// Words per set.
    width: usize,
    /// Per frame, the numbers of moves, factors and freedoms.
    frames: Vec<(u32, u32, i32)>,
    candidates: u32,
    members: u32,
    root_max: u32,
}

impl Sizes {
    /// The sizes of the root whose frames have the given moves, factors and freedoms.
    fn new<const W: usize>(
        n: u32,
        frames: impl IntoIterator<Item = (Bits<W>, Bits<W>, i32)>,
        candidates: &Bits<W>,
    ) -> Sizes {
        let mut members = *candidates;
        let mut counts = Vec::new();
        for (moves, factors, freedoms) in frames {
            members |= moves | factors;
            counts.push((moves.len(), factors.len(), freedoms));
        }
        Sizes {
            n,
            width: W,
            frames: counts,
            candidates: candidates.len(),
            members: members.len(),
            root_max: members.max().unwrap_or(0),
        }
    }

    fn of_root<const W: usize>(n: u32, p: &Prepared<W>) -> Sizes {
        Sizes::new(n, p.root.frames().iter().map(|f| (f.moves, f.factors, f.freedoms)), &p.candidates)
    }

    fn max_freedoms(&self) -> u32 {
        self.frames.iter().map(|&(_, _, freedoms)| freedoms.max(0) as u32).max().unwrap_or(0)
    }

    /// Adds the sizes to `line`; the per-frame lists are named with `prefix` (`frame_moves`, or `moves` for none).
    fn add_to(&self, line: &mut Line, prefix: &str) {
        line.field("frames", self.frames.len())
            .field("candidates", self.candidates)
            .field("members", self.members)
            .field("root_max", self.root_max)
            .field("words_n", words_for(self.n))
            .field("width", self.width)
            .field("words_members", self.members.div_ceil(64))
            .field("words_candidates", self.candidates.div_ceil(64))
            .field("words_root_max", words_for(self.root_max))
            .list(&format!("{prefix}moves"), self.frames.iter().map(|f| f.0))
            .list(&format!("{prefix}factors"), self.frames.iter().map(|f| f.1))
            .list(&format!("{prefix}freedoms"), self.frames.iter().map(|f| f.2));
    }
}

/// The root's GF(2) graph (the vectors of the candidates in the root's frames), and the time it took to build.
fn root_graph<const W: usize>(p: &Prepared<W>) -> (Gf2Graph, Duration) {
    let t0 = Instant::now();
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    (graph, t0.elapsed())
}

/// The root's GF(2) bound with no incumbent, as the search computes it at its first node, before any branching
/// (`Stepper::check_gf2` at the root unit: the bound of the root with all the candidates and at most `max_total`
/// promotions; the rule of the tactic `symmetry` drops nothing there, as a test checks), the engine's work for it
/// (augmentations), and the time of the reset and the evaluation.
fn root_bound<const W: usize>(p: &Prepared<W>, graph: &Gf2Graph) -> (Eval<W>, u64, Duration) {
    let mut bound = Gf2Bound::<W>::new(graph);
    let t0 = Instant::now();
    bound.reset(&Bits::EMPTY, &p.candidates);
    let eval = bound.evaluate(p.max_total, None);
    (eval, bound.stats().augmentations, t0.elapsed())
}

/// The root's GF(2) bound: building the frame vectors, the bound with no incumbent, and the same evaluation asked for
/// the members forced by an incumbent worth `start`.  Adds the gaps of the bound over `start` and over `opt`.
fn root_gf2<const W: usize>(line: &mut Line, p: &Prepared<W>, start: u64, opt: Option<u64>) {
    let (graph, graph_time) = root_graph(p);
    let (eval, augmentations, bound_time) = root_bound(p, &graph);

    let mut split = Gf2Bound::<W>::new(&graph);
    let t2 = Instant::now();
    split.reset(&Bits::EMPTY, &p.candidates);
    let forced = split.evaluate(p.max_total, Some(start + 1)).forced;
    let split_time = t2.elapsed();

    let gap = |floor: u64| eval.value as i64 - floor as i64;
    line.seconds("gf2_graph_s", graph_time.as_secs_f64())
        .field("elements", graph.len())
        .list("ranks", graph.ranks())
        .field("block_words", graph.words())
        .seconds("root_bound_s", bound_time.as_secs_f64())
        .field("root_bound", eval.value)
        .field("root_chosen", eval.chosen.len())
        .field("root_augmentations", augmentations)
        .seconds("root_split_s", split_time.as_secs_f64())
        .field("root_forced", forced.len())
        .field("gap_start", gap(start))
        .maybe("gap_opt", opt.map(gap));
}

/// The engine's counters for pausing, checkpointing and queueing, for the workers' number and lifetimes, and for the
/// ramp, the tail and the units' starts.
fn engine_fields(line: &mut Line, c: &Counters) {
    line.field("pauses", c.pauses)
        .seconds("pause_total_s", c.pause_total.as_secs_f64())
        .seconds("pause_max_s", c.pause_max.as_secs_f64())
        .seconds("park_max_s", c.park_max.as_secs_f64())
        .field("checkpoints", c.checkpoints)
        .field("max_queue", c.max_queue)
        .field("spilled_units", c.spilled_units)
        .field("runs", c.runs)
        .field("tiny_runs", c.tiny_runs)
        .field("spawned", c.spawned)
        .field("retired", c.retired)
        .seconds("capacity_s", c.capacity.as_secs_f64())
        .seconds("ramp_idle_s", c.ramp_idle.as_secs_f64())
        .seconds("tail_s", c.tail.as_secs_f64())
        .seconds("tail_idle_s", c.tail_idle.as_secs_f64())
        .seconds("rebuild_s", c.rebuild.as_secs_f64())
        .seconds("first_step_s", c.first_step.as_secs_f64())
        .field("first_steps", c.first_steps);
}

/// What a search came to and cost: outcome, time, the promotion and GF(2) counters, the cost per node and per
/// augmentation, and the engine's counters.
fn run_fields(line: &mut Line, run: &Run) {
    let (x, c) = (&run.stats.extra, &run.counters);
    let busy_us = c.busy.as_secs_f64() * 1e6;
    line.field("completed", run.completed)
        .field("best", run.best)
        .field("nodes", run.stats.nodes)
        .seconds("seconds", run.call.wall.as_secs_f64())
        .seconds("busy_s", c.busy.as_secs_f64())
        .field("cycles", run.call.cycles)
        .field("instructions", run.call.instructions)
        .field("promote_attempts", x[stat::PROMOTE_ATTEMPTS])
        .field("promotions", x[stat::PROMOTIONS])
        .field("children", x[stat::CHILDREN])
        .field("gf2_solves", x[stat::GF2_SOLVES])
        .field("gf2_augmentations", x[stat::GF2_AUGMENTATIONS])
        .field("gf2_strong_solves", x[stat::GF2_STRONG_SOLVES])
        .field("gf2_strong_augmentations", x[stat::GF2_STRONG_AUGMENTATIONS])
        .ratio("us_per_node", busy_us, run.stats.nodes)
        .ratio("us_per_aug", busy_us, x[stat::GF2_AUGMENTATIONS])
        .ratio("cycles_per_node", run.call.cycles as f64, run.stats.nodes);
    engine_fields(line, c);
}

/// The checkpoint a run writes, from `--ckpt-dir` and `--every`: the file `name` in the directory (which is made if
/// missing, and where a stale file of that name is deleted) and the interval.
fn checkpoint_in(o: &Options, name: &str) -> Result<Option<(PathBuf, Duration)>, String> {
    let (Some(dir), Some(every)) = (&o.ckpt_dir, o.every) else { return Ok(None) };
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(name);
    engine::remove_checkpoint(&path).map_err(|e| e.to_string())?;
    Ok(Some((path, every)))
}

fn file_len(path: Option<&Path>) -> Option<u64> {
    fs::metadata(path?).ok().map(|m| m.len())
}

/// The size of the run's checkpoint file, if it wrote one; the file is deleted.
fn take_checkpoint(checkpoint: &Option<(PathBuf, Duration)>) -> Result<Option<u64>, String> {
    let Some((path, _)) = checkpoint else { return Ok(None) };
    let len = file_len(Some(path));
    engine::remove_checkpoint(path).map_err(|e| e.to_string())?;
    Ok(len)
}

/// The incumbent value `opt + delta` of a proof, if it is not below zero.
fn incumbent(n: u32, opt: u64, delta: Option<i64>) -> Result<u64, String> {
    let delta = delta.unwrap_or(0);
    opt.checked_add_signed(delta).ok_or_else(|| format!("n={n}: --delta {delta} puts the incumbent below zero"))
}

// ---------------------------------------------------------------------------------------------------------
// setup

fn setup<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, SearchTactics::default(), optimal, true) else { return Ok(()) };
    let Game { board, p, opt, seed, ceiling } = &game;
    let (r, tables) = (o.repeat, &board.tables);
    let mut line = Line::of(n);
    line.field("prime", tables.is_prime(n));

    // Building the board, piece by piece and whole.
    line.seconds("tables_s", fastest(r, || Tables::new(n)).0)
        .seconds("frames_s", fastest(r, || build_frames::<W>(tables)).0)
        .seconds("plain_s", fastest(r, || solution_for_promotions::<W>(tables, Bits::EMPTY)).0)
        .seconds("board_s", fastest(r, || Board::<W>::new(Tables::new(n))).0);

    // What the search is given.
    let (prepare_s, prepared) = fastest(r, || board.prepare(SearchTactics::default()).expect("prepare"));
    line.seconds("prepare_s", prepare_s)
        .seconds("model_s", prepared.timings.build.as_secs_f64())
        .seconds("greedy_s", prepared.timings.greedy.as_secs_f64())
        .seconds("problem_s", fastest(r, || p.problem().digest()).0)
        .seconds("search_problem_s", fastest(r, || search_problem(n).expect("problem")).0)
        .seconds("match_graph_s", fastest(r, || MatchGraph::new(&p.model, &p.root)).0);
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    line.seconds("improve_s", fastest(r, || hood.improve(&mut Scratch::new(), &p.greedy, true)).0);

    // The search's own setup: it is stopped before its first node.
    let cfg = EngineConfig { threads: 1, time_limit: Some(Duration::ZERO), ..EngineConfig::default() };
    let (zero_s, zero) = fastest(r, || search(p, &cfg, seed.as_ref(), *ceiling, false).expect("search"));
    let start = zero.start.sum();
    line.seconds("zero_search_s", zero_s).field("capped", zero.reached_cap);
    root_gf2(&mut line, p, start, *opt);

    // Turning the promotions into a game, and checking it.
    let (moves_s, moves) = fastest(r, || solution_for_promotions::<W>(tables, zero.start));
    let (replay_s, replay) = fastest(r, || replay_finished(n, &moves));
    line.seconds("moves_s", moves_s)
        .seconds("replay_s", replay_s)
        .field("replay_ok", replay.is_ok())
        .seconds("prime_path_s", fastest(r, || BoardFrames::<W>::new(tables, Bits::EMPTY).promotion_candidates()).0);

    line.field("max_total", p.max_total);
    Sizes::of_root(n, p).add_to(&mut line, "frame_");
    line.maybe("opt", *opt).field("start", start).maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// nodes

fn nodes<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, o.tactics, optimal, !o.no_seed) else { return Ok(()) };
    let Game { p, opt, seed, ceiling, .. } = &game;
    if o.mode == Mode::Opt && opt.is_none() {
        Line::of(n).text("error", "--mode opt needs the known optimum (--known FILE)").print();
        return Ok(());
    }
    let node_limit = o.nodes.unwrap_or(DEFAULT_NODES);
    let checkpoint = checkpoint_in(o, &format!("wall_n{n}_w{W}.ckpt"))?;
    let cfg = EngineConfig {
        threads: 1,
        node_limit: Some(node_limit),
        time_limit: o.seconds,
        checkpoint: checkpoint.clone(),
        keep_completed_checkpoint: true,
        ..EngineConfig::default()
    };
    let (run, start, delta) = match (o.mode, opt) {
        (Mode::Opt, Some(opt)) => {
            let start = incumbent(n, *opt, o.delta)?;
            (proof(p, &cfg, start), start, o.delta.or(Some(0)))
        }
        _ => {
            let (result, call) = measure(|| search(p, &cfg, seed.as_ref(), *ceiling, false).expect("search"));
            let run = Run {
                completed: result.completed,
                best: result.sum,
                stats: result.stats,
                counters: result.counters,
                call,
            };
            (run, result.start.sum(), None)
        }
    };
    let ckpt_bytes = take_checkpoint(&checkpoint)?;

    let mut line = Line::of(n);
    line.text("mode", o.mode.name())
        .maybe("delta", delta)
        .text("tactics", &Tactics { search: o.tactics, ..Tactics::default() }.description())
        .field("node_limit", node_limit)
        .field("max_total", p.max_total);
    Sizes::of_root(n, p).add_to(&mut line, "frame_");
    root_gf2(&mut line, p, start, *opt);
    line.field("start", start).maybe("opt", *opt);
    run_fields(&mut line, &run);
    line.maybe("ckpt_bytes", ckpt_bytes).maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// grow

/// Prints a line of the process's memory, the checkpoint's size and the spill directory's files every `period`
/// until `done`; it looks every 50 ms, so it ends soon after.
fn monitor(done: &AtomicBool, period: Duration, checkpoint: Option<&Path>, spill: Option<&Path>) {
    let started = Instant::now();
    let mut next = period;
    while !done.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(50));
        if started.elapsed() < next {
            continue;
        }
        next += period;
        let spill_files =
            spill.and_then(|dir| fs::read_dir(dir).ok()).map_or(0, |dir| dir.filter_map(Result::ok).count());
        let mut line = Line::new();
        line.seconds("t", started.elapsed().as_secs_f64())
            .maybe("max_rss", counters::max_rss())
            .maybe("ckpt_bytes", file_len(checkpoint))
            .field("spill_files", spill_files);
        line.print();
    }
}

fn grow<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, o.tactics, optimal, false) else { return Ok(()) };
    let p = &game.p;
    let opt = game.opt.ok_or_else(|| format!("n={n}: grow needs the known optimum (--known FILE)"))?;
    let start = incumbent(n, opt, o.delta)?;
    let checkpoint = checkpoint_in(o, &format!("grow_n{n}.ckpt"))?;
    let mut cfg = EngineConfig {
        threads: o.threads,
        cores: o.cores,
        node_limit: o.nodes,
        time_limit: o.seconds,
        progress: Some(o.progress),
        label: format!("n={n}"),
        checkpoint: checkpoint.clone(),
        keep_completed_checkpoint: true,
        spill_dir: o.ckpt_dir.as_ref().map(|dir| dir.join("spill")),
        ..EngineConfig::default()
    };
    if let Some(units) = o.mem_cap {
        cfg.queue_mem_cap = units;
    }

    let done = AtomicBool::new(false);
    let checkpoint_path = checkpoint.as_ref().map(|(path, _)| path.as_path());
    let run = std::thread::scope(|scope| {
        scope.spawn(|| monitor(&done, o.progress, checkpoint_path, cfg.spill_dir.as_deref()));
        let run = proof(p, &cfg, start);
        done.store(true, Ordering::Release);
        run
    });
    let ckpt_bytes = take_checkpoint(&checkpoint)?;

    let c = &run.counters;
    let mut line = Line::of(n);
    line.field("opt", opt).field("delta", o.delta.unwrap_or(0)).field("threads", o.threads).maybe("cores", o.cores);
    run_fields(&mut line, &run);
    line.field("splits", c.splits)
        .field("split_units", c.split_units)
        .field("steals", c.steals)
        .field("spill_segments", c.spill_segments)
        .field("reloaded_segments", c.reloaded_segments)
        .maybe("ckpt_bytes", ckpt_bytes)
        .maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// start

/// The promotions of `seed` that are candidates and still promote one by one from the largest down, skipping the
/// ones that fail: what `search::incumbent::surviving` (private to the library) keeps of the solver's seed.
fn surviving_seed<const W: usize>(p: &Prepared<W>, seed: &Bits<W>) -> Bits<W> {
    let mut state = p.root;
    let mut kept = Bits::EMPTY;
    for x in (*seed & p.candidates).iter_desc() {
        if p.model.promote_in_place(&mut state, x) {
            kept.insert(x);
        }
    }
    kept
}

fn start<const W: usize>(n: u32, _: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, SearchTactics::default(), optimal, true) else { return Ok(()) };
    let Game { p, opt, seed, ceiling, .. } = &game;
    let Some(seed) = seed else {
        Line::of(n).field("no_previous", true).print();
        return Ok(());
    };

    // The two incumbents the solver starts from, and the better of them (it also tries a matching dive, not here).
    let greedy = p.greedy.sum();
    let (seed_kept_s, kept) = fastest(1, || surviving_seed(p, seed));
    let base = if kept.sum() > greedy { kept } else { p.greedy };
    let base_sum = base.sum();

    // The local search from it, as the solver calls it (with pairs), and without them.
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    let (singles_s, singles) = fastest(1, || hood.improve(&mut Scratch::new(), &base, false).sum());
    let (pairs_s, pairs) = fastest(1, || hood.improve(&mut Scratch::new(), &base, true).sum());

    let mut line = Line::of(n);
    line.field("greedy", greedy)
        .field("seed_kept", kept.sum())
        .seconds("seed_kept_s", seed_kept_s)
        .field("base", base_sum)
        .field("members", base.len())
        .maybe("ceiling", *ceiling)
        .maybe("at_ceiling", ceiling.map(|ceiling| base_sum >= ceiling))
        .seconds("singles_s", singles_s)
        .field("singles", singles)
        .seconds("pairs_s", pairs_s)
        .field("pairs", pairs)
        .maybe("opt", *opt)
        .maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// frames

/// `frames` for board `n`, at the set width the solver picks for it.
fn frames_at_width(n: u32) -> Result<(), String> {
    if n > MAX_N {
        return Err(format!("n={n}: frames goes up to {MAX_N}"));
    }
    taxman::with_width!(n, W => frames::<W>(n), else unreachable!("n={n} is within {MAX_N}"));
    Ok(())
}

fn frames<const W: usize>(n: u32) {
    let tables = Tables::new(n);
    let t0 = Instant::now();
    let built = build_frames::<W>(&tables);
    let frames_s = t0.elapsed().as_secs_f64();
    let shapes = built.frames.iter().map(|f| (f.moves, f.factors, f.freedoms));
    let sizes = Sizes::new(n, shapes, &built.candidates);
    let mut line = Line::of(n);
    line.seconds("frames_s", frames_s);
    sizes.add_to(&mut line, "");
    line.field("max_freedoms", sizes.max_freedoms()).field("block_words", sizes.max_freedoms().div_ceil(64));
    line.print();
}

// ---------------------------------------------------------------------------------------------------------
// landscape

/// Game `n` set up as `solve` sets it up: the board and, unless nothing can be promoted, the search's `prepare` with
/// the default tactics.
fn set_up<const W: usize>(n: u32) -> (Board<W>, Option<Prepared<W>>) {
    let board = Board::<W>::for_n(n);
    let prepared = (!board.is_trivial()).then(|| board.prepare(SearchTactics::default()).expect("prepare"));
    (board, prepared)
}

/// The start incumbent of the search of `p` (`start_incumbent`: the better of the greedy set and the matching dive,
/// improved by the local search), with no seed and no ceiling.
fn start_of<const W: usize>(p: &Prepared<W>) -> Bits<W> {
    let graph = MatchGraph::new(&p.model, &p.root);
    start_incumbent(p, &graph, &Control::new(), 1, &[])
}

/// A game standalone: the two ends of what its search has to close, and what finding them cost.  The sums are
/// promotion sums; the line reports scores.
#[derive(Default)]
struct Landscape {
    n: u32,
    prime: bool,
    /// Nothing to promote: the game with no promotions is the only game.
    trivial: bool,
    in_the_bag: u64,
    /// The board's frames and candidates, as `solve`'s setup line counts them, and `max_total`.
    frames: usize,
    candidates: u32,
    max_total: u32,
    /// The greedy set's sum, the start incumbent's (the lower end) and the root's GF(2) bound (the upper end).
    greedy: u64,
    start: u64,
    bound: u64,
    /// Seconds: the board and the search's setup (`greedy_s`, the greedy set, is part of it), the start incumbent,
    /// the GF(2) graph (`graph_s`) and the bound with it, and the game as a whole; and the CPU seconds of its thread.
    setup_s: f64,
    greedy_s: f64,
    start_s: f64,
    graph_s: f64,
    bound_s: f64,
    total_s: f64,
    cpu_s: Option<f64>,
}

impl Landscape {
    /// The upper end as a score: no game of the board scores more.
    fn upper(&self) -> u64 {
        self.in_the_bag + self.bound
    }

    /// The lower end as a score: the game a standalone `solve` starts from.
    fn lower(&self) -> u64 {
        self.in_the_bag + self.start
    }

    /// The standalone root gap, which can be negative only if a bound is wrong.
    fn gap(&self) -> i64 {
        self.upper() as i64 - self.lower() as i64
    }

    /// Whether the two ends meet, which settles the game at the search's first node.
    fn meets(&self) -> bool {
        self.upper() == self.lower()
    }

    /// The line of the game; `opt` is its known optimum score, and `proof` how its results line proves it.
    fn line(&self, opt: Option<u64>, proof: Option<&str>) -> Line {
        let mut line = Line::of(self.n);
        line.field("prime", self.prime)
            .field("trivial", self.trivial)
            .field("in_the_bag", self.in_the_bag)
            .field("upper", self.upper())
            .field("lower", self.lower())
            .field("gap", self.gap())
            .field("meets", self.meets())
            .field("greedy", self.in_the_bag + self.greedy)
            .field("frames", self.frames)
            .field("candidates", self.candidates)
            .field("max_total", self.max_total)
            .seconds("setup_s", self.setup_s)
            .seconds("greedy_s", self.greedy_s)
            .seconds("start_s", self.start_s)
            .seconds("graph_s", self.graph_s)
            .seconds("bound_s", self.bound_s)
            .seconds("total_s", self.total_s)
            .maybe("cpu_s", self.cpu_s.map(|cpu| format!("{cpu:.6}")))
            .maybe("opt", opt);
        match proof {
            Some(proof) => line.text("opt_proof", proof),
            None => line.field("opt_proof", "null"),
        };
        line.maybe("root_gap", opt.map(|opt| self.upper() as i64 - opt as i64))
            .maybe("shortfall", opt.map(|opt| opt as i64 - self.lower() as i64));
        line
    }
}

/// Game `n` standalone, at the set width `W` that `solve` picks for it: set up as `solve` sets it up (the board and
/// `prepare` with the default tactics), the start incumbent as the search makes it with no seed and no ceiling
/// (`start_incumbent`: the better of the greedy set and the matching dive, improved by the local search), and the
/// root's GF(2) bound.
fn landscape_of<const W: usize>(n: u32) -> Landscape {
    let (cpu0, t0) = (counters::thread_cpu_seconds(), Instant::now());
    let (board, prepared) = set_up::<W>(n);
    let mut land = Landscape {
        n,
        prime: board.tables.is_prime(n),
        trivial: prepared.is_none(),
        in_the_bag: board.in_the_bag,
        frames: board.frames.frames.len(),
        candidates: board.candidates().len(),
        ..Landscape::default()
    };
    if let Some(p) = prepared {
        land.setup_s = t0.elapsed().as_secs_f64();
        (land.greedy_s, land.greedy, land.max_total) = (p.timings.greedy.as_secs_f64(), p.greedy.sum(), p.max_total);

        let t1 = Instant::now();
        land.start = start_of(&p).sum();
        land.start_s = t1.elapsed().as_secs_f64();

        let (gf2, graph_time) = root_graph(&p);
        let (eval, _, eval_time) = root_bound(&p, &gf2);
        land.bound = eval.value;
        land.graph_s = graph_time.as_secs_f64();
        land.bound_s = (graph_time + eval_time).as_secs_f64();
    }
    land.total_s = t0.elapsed().as_secs_f64();
    land.cpu_s = cpu0.zip(counters::thread_cpu_seconds()).map(|(before, after)| after - before);
    land
}

/// Game `n` standalone, at the set width `with_width!` picks for it.
fn landscape_at(n: u32) -> Result<Landscape, String> {
    taxman::with_width!(n, W => Ok(landscape_of::<W>(n)), else Err(beyond(n)))
}

/// The proof of each game that the completed lines of the results file `path` have (the last line of a game counts).
fn known_proofs(path: &Path) -> Result<HashMap<u32, String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut proofs = HashMap::new();
    for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let game = Json::parse(line).map_err(|e| format!("{}: line {}: {e}", path.display(), i + 1))?;
        let field = |name: &str| game.get(name);
        if field("completed").and_then(Json::as_bool) == Some(true) {
            if let (Some(n), Some(proof)) = (field("n").and_then(Json::as_u32), field("proof").and_then(Json::as_str)) {
                proofs.insert(n, proof.to_owned());
            }
        }
    }
    Ok(proofs)
}

/// The games of the lines of an output file's text, and the length in bytes of its whole lines.  A text that does
/// not end in a newline ends in a line cut short (a run was killed in the middle of writing it), which counts for
/// nothing and is cut off before a run goes on.
fn finished_games(text: &str) -> Result<(HashSet<u32>, usize), String> {
    let whole = text.rfind('\n').map_or(0, |at| at + 1);
    let mut games = HashSet::new();
    for (i, line) in text[..whole].lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let n = Json::parse(line).ok().and_then(|game| game.get("n").and_then(Json::as_u32));
        games.insert(n.ok_or_else(|| format!("line {}: not a game's line", i + 1))?);
    }
    Ok((games, whole))
}

/// `landscape`: the games of `o.games` that `--out` does not have yet, `--threads` at a time, one game a thread, each
/// line printed and appended as its game finishes.
fn landscape(o: &Options, optimal: &Optimal) -> Result<(), String> {
    let proofs = o.known.as_deref().map(known_proofs).transpose()?.unwrap_or_default();
    // The games the output file has, and the length to cut it to if it ends in a line cut short.
    let (mut done, mut cut_to) = (HashSet::new(), None);
    if let Some(path) = o.out.as_deref().filter(|path| path.exists()) {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (games, whole) = finished_games(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        (done, cut_to) = (games, Some(whole).filter(|&whole| whole < text.len()));
    }
    let mut seen = HashSet::new();
    let todo: Vec<u32> = o.games.iter().copied().filter(|n| seen.insert(*n)).collect();
    let todo: Vec<u32> = todo.into_iter().filter(|n| !done.contains(n)).collect();
    if let Some(path) = &o.out {
        let skipped = seen.len() - todo.len();
        if skipped > 0 {
            eprintln!("skipped {skipped} games already done in {}", path.display());
        }
    }
    let sink = match &o.out {
        Some(path) => {
            let at = |e: std::io::Error| format!("{}: {e}", path.display());
            let file = fs::OpenOptions::new().append(true).create(true).open(path).map_err(at)?;
            if let Some(length) = cut_to {
                eprintln!("cut off a line cut short at the end of {}", path.display());
                file.set_len(length as u64).map_err(at)?;
            }
            Some(Mutex::new(file))
        }
        None => None,
    };

    let next = AtomicUsize::new(0);
    let failure = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..o.threads.min(todo.len()) {
            scope.spawn(|| {
                while let Some(&n) = todo.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let land = landscape_at(n).unwrap_or_else(|e| fail(&e));
                    let line = land.line(optimal.score(n), proofs.get(&n).map(String::as_str)).render();
                    println!("{line}");
                    if let Some(sink) = &sink {
                        let mut file = sink.lock().unwrap_or_else(PoisonError::into_inner);
                        if let Err(e) = file.write_all(format!("{line}\n").as_bytes()) {
                            failure.lock().unwrap_or_else(PoisonError::into_inner).get_or_insert(e.to_string());
                        }
                    }
                }
            });
        }
    });
    match failure.into_inner().unwrap_or_else(PoisonError::into_inner) {
        Some(e) => Err(format!("--out: {e}")),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------------------------------------
// Tests: `landscape`'s (`cargo test --features studies` runs them: the example is `test = true` in Cargo.toml)

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::search::cut;
    use taxman::search::symmetry::Symmetries;
    use taxman::solver::{self, GameResult, SolveOptions};

    /// The games from 2 to `to`, every `step`-th.
    fn sample(step: usize, to: u32) -> impl Iterator<Item = u32> {
        (2..=to).step_by(step)
    }

    /// `solve` on game `n` alone (no previous game: standalone), on one thread, with the time limit given.
    fn solve_alone(n: u32, time_limit: Option<Duration>) -> GameResult {
        let opts = SolveOptions { threads: 1, window: 1, time_limit, ..SolveOptions::default() };
        let mut result = None;
        solver::solve_range_each(n, n, &opts, None, |game| result = Some(game.clone())).expect("solve");
        result.expect("a result for the game")
    }

    /// The ends of game `n` are what `solve` reports of it: its setup line's sizes and greedy sum, the start incumbent
    /// that a search stopped at once is left with (`best_so_far`), and with `full` the score of a search to the end,
    /// which lies between the ends, and which takes no more than the first node if they meet.
    fn agree_with_solve(n: u32, full: bool) {
        let land = landscape_at(n).expect("a board the solver supports");
        let stopped = solve_alone(n, Some(Duration::ZERO));
        let setup = stopped.setup.expect("a game that is searched");
        let sizes = (setup.frames, setup.candidates, setup.max_total, setup.greedy_sum);
        assert_eq!(sizes, (land.frames, land.candidates, land.max_total, land.greedy), "n={n}: solve's setup line");
        assert!(!stopped.completed, "n={n}: a search stopped before its first node is not complete");
        assert_eq!(stopped.score, land.lower(), "n={n}: the incumbent a standalone search starts from");
        if full {
            let done = solve_alone(n, None);
            assert!(done.completed && land.lower() <= done.score && done.score <= land.upper(), "n={n}");
            assert!(!land.meets() || done.nodes <= 1, "n={n}: the ends meet, and the search took {} steps", done.nodes);
        }
    }

    #[test]
    fn the_ends_of_a_few_small_games_are_what_solve_starts_from_and_finds() {
        for n in [18, 20, 60, 98, 120, 250] {
            agree_with_solve(n, true);
        }
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is the_ends_of_a_few_small_games_are_what_solve_starts_from_and_finds"
    )]
    fn the_ends_of_medium_games_are_what_solve_starts_from_and_finds() {
        for n in [480, 700] {
            agree_with_solve(n, true);
        }
        for n in [936, 1215, 1300] {
            agree_with_solve(n, false);
        }
    }

    /// The root bound is the first node's: a search from an incumbent worth the bound is over at its first step, which
    /// a bound ends (the node needs more than the GF(2) bound, or than the sum of its largest candidates, which no
    /// GF(2) bound exceeds); and one from an incumbent worth one less does not end at its first step by a bound (the
    /// bounds reach what the node needs; the first step can still end by the forced members or the nogoods, or the
    /// node can be branched on).  And the rule of the tactic `symmetry`, which the first step applies before the
    /// bound, drops no candidate at the root.
    fn first_node<const W: usize>(n: u32) {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).expect("prepare");
        let (graph, _) = root_graph(&p);
        let bound = root_bound(&p, &graph).0.value;
        let cfg = EngineConfig { threads: 1, node_limit: Some(2048), ..EngineConfig::default() };
        // The search from `best`, and how many node loops a bound ended.
        let from = |best: u64| {
            let diagnostics = Mutex::new(Diagnostics::default());
            let run = proof_collecting(&p, &cfg, best, Some(&diagnostics));
            let ends = diagnostics.into_inner().unwrap_or_else(PoisonError::into_inner).loop_ends;
            (run.completed, run.stats.nodes, ends[cut::GF2] + ends[cut::LARGEST])
        };
        assert_eq!(from(bound), (true, 1, 1), "n={n}: a bound of {bound} does not end the first node at {bound}");
        let (_, nodes, ended) = from(bound - 1);
        assert!(nodes > 1 || ended == 0, "n={n}: the first node's bound is below {bound}");

        let symmetries = Symmetries::of(&p.model, &p.root, &p.candidates);
        let mut candidates = p.candidates;
        assert!(!symmetries.exclude(&mut symmetries.scratch(), &Bits::EMPTY, &mut candidates), "n={n}");
        assert_eq!(candidates, p.candidates, "n={n}: the rule drops candidates at the root");
    }

    fn first_nodes(games: &[u32]) {
        for &n in games {
            taxman::with_width!(n, W => first_node::<W>(n), else panic!("{}", beyond(n)));
        }
    }

    #[test]
    fn the_root_bound_is_the_bound_of_the_searchs_first_node() {
        first_nodes(&[18, 30, 60, 98, 120, 250, 480]);
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is the_root_bound_is_the_bound_of_the_searchs_first_node"
    )]
    fn the_root_bound_is_the_bound_of_the_searchs_first_node_of_larger_games() {
        first_nodes(&[700, 936, 1215, 1300, 1400, 2112]);
    }

    /// Every sampled game's ends hold opt(n): lower at most it, upper at least.
    fn bracket_the_optimum(games: impl Iterator<Item = u32>) {
        for n in games {
            let land = landscape_at(n).expect("a board the solver supports");
            let opt = Optimal::built_in().score(n).unwrap_or_else(|| panic!("n={n}: the table has no game"));
            let ends = (land.lower(), land.upper());
            assert!(ends.0 <= opt && opt <= ends.1, "n={n}: lower {} optimum {opt} upper {}", ends.0, ends.1);
        }
    }

    #[test]
    fn the_ends_bracket_the_optimum_of_games_to_300() {
        bracket_the_optimum(sample(7, 300));
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is the_ends_bracket_the_optimum_of_games_to_300")]
    fn the_ends_bracket_the_optimum_of_games_to_1300() {
        bracket_the_optimum(sample(5, 1300));
    }

    /// The lines of a `landscape` output file as JSON.
    fn lines(path: &Path) -> Vec<Json> {
        fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
    }

    fn board_sizes(lines: &[Json]) -> Vec<u32> {
        lines.iter().map(|line| line.get("n").and_then(Json::as_u32).expect("n")).collect()
    }

    #[test]
    fn the_output_file_is_appended_to_and_a_rerun_skips_what_it_has() {
        let dir = support::TempDir::new("wall-landscape");
        let out = dir.join("landscape.jsonl");
        let known = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/optimal.jsonl.gz");
        let optimal = Optimal::built_in().with_results_files(std::slice::from_ref(&known)).expect("the chain");
        let mut o = Options::new("landscape".to_owned());
        (o.out, o.known, o.threads) = (Some(out.clone()), Some(known), 3);

        // Thirteen games on three threads, in the order they finish.
        o.games = (18..=30).collect();
        landscape(&o, &optimal).expect("landscape");
        let first = fs::read_to_string(&out).unwrap();
        let mut seen = board_sizes(&lines(&out));
        seen.sort_unstable();
        assert_eq!(seen, (18..=30).collect::<Vec<u32>>());
        for line in lines(&out) {
            let field = |name: &str| line.get(name).and_then(Json::as_i64).expect(name);
            let (lower, upper, opt) = (field("lower"), field("upper"), field("opt"));
            assert!(lower <= opt && opt <= upper && field("gap") == upper - lower, "{line}");
            assert_eq!((field("root_gap"), field("shortfall")), (upper - opt, opt - lower), "{line}");
            let proven = line.get("opt_proof").and_then(Json::as_str);
            assert!(proven.is_some(), "{line}: the chain has the proof of every game");
        }

        // The same games again add nothing; more games add only themselves.
        landscape(&o, &optimal).expect("landscape");
        assert_eq!(fs::read_to_string(&out).unwrap(), first);
        o.games = (18..=34).collect();
        landscape(&o, &optimal).expect("landscape");
        let more = fs::read_to_string(&out).unwrap();
        assert!(more.starts_with(&first) && lines(&out).len() == 17);

        // A run killed in the middle of a line leaves a piece of it, which the next run cuts off.
        fs::write(&out, format!("{more}{{\"n\":35,\"up")).unwrap();
        o.games = (18..=36).collect();
        landscape(&o, &optimal).expect("landscape");
        let mut seen = board_sizes(&lines(&out));
        seen.sort_unstable();
        assert_eq!(seen, (18..=36).collect::<Vec<u32>>());
        assert!(fs::read_to_string(&out).unwrap().starts_with(&more));
    }

    #[test]
    fn a_game_without_a_known_optimum_has_null_where_the_optimum_is_needed() {
        let line = Json::parse(&landscape_at(250).expect("a board").line(None, None).render()).expect("JSON");
        for name in ["opt", "opt_proof", "root_gap", "shortfall"] {
            assert!(matches!(line.get(name), Some(Json::Null)), "{name}");
        }
        assert!(line.get("upper").and_then(Json::as_u64) >= line.get("lower").and_then(Json::as_u64));
    }
}
