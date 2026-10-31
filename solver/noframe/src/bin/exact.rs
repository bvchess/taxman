//! The prover's proofs of optimal scores: for every game in a range, an upper bound on opt(n) proven without the
//! frame model, compared with the recorded optimum V(n) (the committed chain and the `--results` files; see
//! solver/noframe/docs/reproducing.md, "The recorded games").  One line per game, resumable:
//!
//!     exact --from A --to B [--list ...] [--threads T] [--seconds S] [--share S] [--budget NODES] [--all 1]
//!           [--parity 0] [--segments 1] [--lagrangian 0|1] [--fix-low 0 | --fix-upper 1] [--checkpoints DIR]
//!           [--optimal FILE] [--results FILE]... --out FILE
//!
//! Per game: the recorded game is replayed by the rules (a lower bound, opt(n) >= V(n)); the Franklín–Moniot bound and
//! the parity bound (W1, docs/theory/parity-bound.md; `parity.rs`) are computed at the root; the
//! ceiling opt(n) <= n + opt(n−1), for a prime n the prime formula opt(n) <= n + opt(n−1) − p, and for an odd n with
//! two prime factors that closes a cycle with U\*_2(n − 1) the semiprime formula opt(n) = opt(n−1) + n − x (J3,
//! docs/theory/arrival.md; `semiprime.rs`), are noted (the line's `relation` is `chain`, `prime`, `semiprime` or
//! `none`); and unless one of them already gives V(n) (or `--all`), the branch and bound of `game.rs` decides whether
//! any playable set beats V(n), with the parity bound at every node (`--parity 0`: the Franklín–Moniot bound alone,
//! as the searches of `data/exact_2_1231.jsonl` ran).  Which games end up proven, given their predecessors, is worked
//! out by `tools/summarize.py exact` from the file.
//!
//! The roles of F4 (`--fix-low 1`, the default; `--fix-low 0` starts from the plain root, as the committed files
//! `data/exact_*.jsonl` were made): the search starts from the roles `upper::low_roles` writes down, "B" below, the
//! largest prime forced and every other prime barred, the sources of layer 2 in U\* forced and the other numbers of
//! layer 2 above n/2 barred, and, on a board whose frames of layer 2 are closed (the function decides it from the
//! board's own U\*, not from n), every number of layer 2 up to n/2 barred.  The bottom-layers theorem (F4,
//! docs/theory/frame-model.md) says every optimal set obeys them, so a search from them that proves "no playable set
//! obeying them beats V" still proves opt(n) <= V (an optimal set beating V would obey them), and its line is written
//! `"proven"` like any other.  The recorded game must obey them (checked).  The sources of layer 3 and up stay free.
//! Against the plain root it takes half the time a node (the parity solves have fewer free elements) at the plain
//! search's nodes within a few percent (1575 and 1584).  Because the proof rests on F4 besides the other theorems,
//! every line records `"fix_low"`, 1 or 0, and with it `fixed_bound`, the parity bound at the roles searched from.
//!
//! The threads (`--threads T`, `src/team.rs`): each thread starts the next game in the order given, and when fewer
//! games are left than threads, the idle threads help the searches still running, taking their open subtrees from the
//! threads that hold the largest.  A line's `nodes` add up its threads' nodes, `secs` is its wall time, `thread_secs`
//! its threads' time added up, and `threads` the most threads its search ran on at once; all of them count every run.
//! Each thread keeps a nogood table of its own: a game's first thread in a run starts from the checkpoint's nogoods,
//! and a thread that joins it later starts empty; with `--share S`, a thread joining a game starts from the game's
//! nogoods, and every S seconds takes those its other threads learned.
//!
//! A search still running at `--seconds` pauses and saves the open subtrees of all its threads, and its nogoods, to a
//! checkpoint in `DIR` (default `work/checkpoints`); the next run resumes it on any number of threads, so a game may
//! take any number of runs.  A search of one kind never resumes another's: the checkpoint is `exact_N_low.ckpt`,
//! `exact_N.ckpt` (`--fix-low 0`) or `exact_N_upper.ckpt`.  Without the parity bound, a search that outlasts a
//! 20-second warm-up, or resumes, goes on with the Lagrangian bound fitted to the nogoods learned so far
//! (`--lagrangian 0` turns that off, `--lagrangian 1` turns it on with the parity bound too), on one thread.  Only a
//! search that will turn it on pauses for the warm-up; any other runs to its end or to `--seconds`, so that its tree
//! does not depend on the clock (a pause restarts the open subtrees without their parents' bounds, and a solve from
//! scratch can pick another of two heaviest sets that tie).
//!
//! A SIGINT or SIGTERM stops the run as `--seconds` does (`src/stop.rs`): every search pauses into its checkpoint, all
//! its threads' open subtrees and its nogoods, every finished game has its line, and `exact` exits with status 128 plus
//! the signal's number (130 for SIGINT, 143 for SIGTERM), the status that `solver/tools/cloud/prover.sh` takes for a
//! stop by a signal.  A second signal ends it at once, with the same status; the checkpoints and lines already written
//! stay valid.
//!
//! `--segments 1` turns on the segment cuts, which make a game's proof rest on the smaller games' bounds (recorded as
//! `"segments":1`); they are off by default, so that each search stands alone.  A proof's `"matching_nodes"` are its
//! nodes searched without the parity bound (a search resumed from a checkpoint of such a run); its `"nodes"` count all
//! of them.
//!
//! What the sources of layer 3 and up cost (study E6): `--fix-upper 1` (it needs `--fix-low 1`) also fixes the whole
//! upper part to U\*, every source in it forced and every other barred, "D" below, which is the frame model's problem
//! on the whole board.  It proves nothing about opt(n) (that some optimal set has the upper part U\* is the open
//! statement U of docs/theory/exactness.md), so a finished search of that kind is written `"search":"upper_fixed"` and
//! no tool counts it as a proof; it is a measurement, and its line has `"fix_upper":1`.  A playable set beating V is a
//! real game whatever the roles, so a search that finds one stops with an error in every mode.  Two counters show where
//! B spends on the sources of layer 3 and up, in every search (0 for a game not searched): `branched_on_sources`, the
//! nodes that branched on a set holding a free source of layer 3 or up, and `barred_upper`, the nodes whose roles, as
//! the node received them, bar a member of U\*_{>=3} (`game::Prover::watch`; they change nothing in the search).
//! `fm_bound`, `parity_bound` and the choice of the games to search stay the whole board's.
//! `tools/summarize.py sources` compares B with D.

use std::path::PathBuf;
use std::time::{Duration, Instant};
use taxman_prover::game::{relation, Board, Prover, Role, Verdict};
use taxman_prover::jsonl::{committed_chain, parse_items, recorded_games_to, Obj};
use taxman_prover::parity::{Meet, Outcome, Parity};
use taxman_prover::primes::smallest_prime_factors;
use taxman_prover::stop;
use taxman_prover::team::{campaign, Plan, Search, Tally};
use taxman_prover::upper::{low_roles, obeys, recorded_ustar, upper_fixed_roles, watched};

/// The fields of a game's line that its setup decides.
struct Head {
    n: u64,
    score: u64,
    fm: u64,
    parity_bound: u64,
    relation: &'static str,
    parity: bool,
    segments: bool,
    fix_low: bool,
    fix_upper: bool,
    fixed_bound: Option<u64>,
}

impl Head {
    /// The game's line: `search` says what the search came to, and `tally` counts it.
    fn line(&self, search: &str, tally: &Tally) -> String {
        let searched = search != "skipped";
        let mut line = Obj::new()
            .num("n", self.n)
            .num("score", self.score)
            .num("fm_bound", self.fm)
            .num("parity_bound", self.parity_bound)
            .str("relation", self.relation)
            .str("search", search)
            .num("parity", u8::from(self.parity && searched))
            .num("segments", u8::from(self.segments && searched))
            .num("nodes", tally.nodes)
            .num("matching_nodes", tally.matching_nodes)
            .float("secs", tally.secs)
            .float("thread_secs", tally.thread_secs)
            .num("threads", tally.threads)
            .num("branched_on_sources", tally.branching)
            .num("barred_upper", tally.barring)
            .num("fix_low", u8::from(self.fix_low));
        if let Some(bound) = self.fixed_bound {
            line = line.num("fixed_bound", bound);
        }
        if self.fix_upper {
            line = line.num("fix_upper", 1);
        }
        line.finish()
    }
}

fn main() {
    // First, so that a signal during the start-up (the recorded games take seconds to read) is a clean stop too.
    stop::on_signals().expect("install the handlers of SIGINT and SIGTERM");
    let (items, flags) = parse_items(std::env::args().skip(1));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (mut threads, mut seconds, mut share, mut budget) = (1usize, f64::INFINITY, None, u64::MAX);
    let (mut all, mut parity, mut segments, mut lagrangian) = (false, true, false, None);
    // The roles of F4 (docs/theory/frame-model.md) are the default; --fix-low 0 starts from the plain root.
    let (mut fix_low, mut fix_upper) = (true, false);
    let mut files = vec![committed_chain()];
    let mut checkpoints = root.join("work/checkpoints");
    let mut out = PathBuf::new();
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--seconds" => seconds = value.parse().unwrap(),
            "--share" => share = Some(Duration::from_secs_f64(value.parse().unwrap())),
            "--budget" => budget = value.parse().unwrap(),
            "--all" => all = value == "1" || value == "true",
            "--parity" => parity = value == "1" || value == "true",
            "--segments" => segments = value == "1" || value == "true",
            "--lagrangian" => lagrangian = Some(value == "1" || value == "true"),
            "--fix-low" => fix_low = value == "1" || value == "true",
            "--fix-upper" => fix_upper = value == "1" || value == "true",
            "--checkpoints" => checkpoints = value.into(),
            "--optimal" => files[0] = value.into(),
            "--results" => files.push(value.into()),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    assert!(fix_low || !fix_upper, "--fix-upper needs the roles of F4 (--fix-low 1, the default)");
    // A search of one kind never resumes another's.
    let kind = if fix_upper {
        "_upper"
    } else if fix_low {
        "_low"
    } else {
        ""
    };
    // The Lagrangian bound cut nothing beside the parity bound at 990 (the same 44,411 nodes, 7% more time).
    let lagrangian = lagrangian.unwrap_or(!parity);
    let known = recorded_games_to(&files, items.iter().copied().max().unwrap_or(0));
    let items: Vec<u64> = items.into_iter().filter(|n| *n >= 2 && known.contains_key(n)).collect();
    let spf = smallest_prime_factors(items.iter().copied().max().unwrap_or(2) as usize);
    campaign("exact", &items, threads, seconds, &out, share, |n| {
        let started = Instant::now();
        let (score, moves) = &known[&n];
        let score = *score;
        let board = Board::new(n as u32, &spf);
        let replayed = board.replay(moves);
        assert_eq!(replayed, Ok(score), "n={n}: the recorded game does not replay to its score");
        let prev = known.get(&(n - 1)).map_or(0, |(s, _)| *s);
        let relation = relation(n, score, prev, &spf).name();
        let mut root_prover = Prover::new(&board, score, budget);
        let fm = root_prover.root_bound();
        assert!(fm >= score, "n={n}: the matching bound {fm} is below a legal game's {score}");
        let weights: Vec<i64> = (0..=n as i64).collect();
        let parity_at = |roles: &[Role]| Meet::new(Parity::pair(&board, &weights)).solve(roles);
        let parity_bound = {
            let mut roles = root_prover.root_roles();
            let Outcome::Bound(root_bound) = parity_at(&roles) else {
                panic!("n={n}: the root's parity bound is not certified")
            };
            assert!(root_bound.value >= score as i64, "n={n}: the parity bound is below a legal game's {score}");
            // The recorded game's moves are independent at both parities (W1, docs/theory/parity-bound.md).
            moves.iter().for_each(|&c| roles[c as usize] = Role::Pick);
            let independent = parity_at(&roles);
            assert!(matches!(independent, Outcome::Bound(_)), "n={n}: the recorded game is dependent");
            root_bound.value as u64
        };
        let bounded = fm == score || (parity && parity_bound == score);
        let skip = bounded || (relation != "none" && !all);
        // U* fixes the roles and names the numbers the counters watch; a game that is left alone needs neither.
        let ustar = if fix_low || !skip { recorded_ustar(n, moves, &spf) } else { Vec::new() };
        // The roles the search starts from (those of F4, docs/theory/frame-model.md, unless --fix-low 0 says the plain
        // root): the recorded game obeys them, so their parity bound is at least its score.
        let unit = if fix_upper {
            upper_fixed_roles(n as u32, &spf, &ustar)
        } else if fix_low {
            low_roles(n as u32, &spf, &ustar)
        } else {
            root_prover.root_roles()
        };
        if fix_low {
            obeys(&unit, moves)
                .unwrap_or_else(|e| panic!("n={n}: the recorded game breaks the roles searched from: {e}"));
        }
        let fixed_bound = fix_low.then(|| {
            let Outcome::Bound(bound) = parity_at(&unit) else {
                panic!("n={n}: the parity bound at the roles searched from is not certified")
            };
            assert!(bound.value >= score as i64, "n={n}: the bound at the roles searched from is below {score}");
            bound.value as u64
        });
        let head = Head { n, score, fm, parity_bound, relation, parity, segments, fix_low, fix_upper, fixed_bound };
        if skip {
            let secs = started.elapsed().as_secs_f64();
            let tally = Tally {
                verdict: Verdict::Proven,
                nodes: 0,
                matching_nodes: 0,
                secs,
                thread_secs: secs,
                threads: 1,
                branching: 0,
                barring: 0,
                given: 0,
                open: 0,
                found: Vec::new(),
            };
            return Plan::Line(head.line("skipped", &tally));
        }
        let (branch, barred) = watched(n as u32, &spf, &ustar);
        let smaller: Vec<u64> =
            if segments { (0..n).map(|m| known.get(&m).map_or(0, |(s, _)| *s)).collect() } else { Vec::new() };
        let search = Search {
            board,
            target: score,
            budget,
            root: unit,
            checkpoint: checkpoints.join(format!("exact_{n}{kind}.ckpt")),
            parity,
            // A search that outlasts a short warm-up (or resumes from a checkpoint) goes on with the Lagrangian bound
            // fitted to the nogoods learned so far.  Without that bound there is no warm-up: a pause restarts the open
            // units without their parents' matchings and certificates, so the search runs to its end or to the
            // deadline.
            warmup: lagrangian.then_some(Duration::from_secs(20)),
            prover: Box::new(move |board, budget| {
                let mut prover = Prover::new(board, score, budget);
                if parity {
                    prover = prover.with_parity();
                }
                if segments {
                    prover = prover.with_smaller_games(&smaller);
                }
                prover.watch(branch.clone(), barred.clone())
            }),
        };
        let finish = Box::new(move |tally: &Tally| {
            let (search, note) = match &tally.verdict {
                // A search from the roles of the frame model's problem proves nothing about opt(n).
                Verdict::Proven => {
                    let (nodes, threads, given) = (tally.nodes, tally.threads, tally.given);
                    let note = format!("{nodes} nodes, on {threads} threads at most, {given} units moved");
                    (if fix_upper { "upper_fixed" } else { "proven" }, Some(note))
                }
                Verdict::Budget => ("budget", Some(format!("budget exhausted after {} nodes", tally.nodes))),
                Verdict::Paused => {
                    let (nodes, open, given) = (tally.nodes, tally.open, tally.given);
                    return (None, Some(format!("paused after {nodes} nodes, {open} open units, {given} units moved")));
                }
                Verdict::Beaten(picks) => {
                    let sum: u64 = picks.iter().map(|&c| u64::from(c)).sum();
                    panic!("n={n}: a playable set worth {sum} beats the recorded {score}: {picks:?}");
                }
            };
            (Some(head.line(search, tally)), note)
        });
        Plan::Search(search, finish)
    });
    // Every search is paused in its checkpoint and every finished game has its line.
    if let Some(status) = stop::status() {
        std::process::exit(status);
    }
}
