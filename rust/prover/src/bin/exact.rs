//! The prover's proofs of optimal scores: for every game in a range, an upper bound on opt(n) proven without the
//! frame model, compared with the recorded optimum V(n) (optimal.json, which reaches 1300, and for a board size it
//! lacks the game of `--results`, by default the solver's `results/strict.jsonl`).  One line per game, resumable:
//!
//!     exact --from A --to B [--list ...] [--threads T] [--seconds S] [--budget NODES] [--all 1] [--parity 0]
//!           [--segments 1] [--lagrangian 0|1] [--fix-low 0 | --fix-upper 1] [--checkpoints DIR]
//!           [--optimal FILE] [--results FILE] --out FILE
//!
//! Per game: the recorded game is replayed by the rules (a lower bound, opt(n) >= V(n)); the Franklín–Moniot
//! bound and the parity bound (THEORY.md, Theorem 10; `parity.rs`) are computed at the root; the ceiling
//! opt(n) <= n + opt(n−1) and, for a prime n, the prime formula opt(n) <= n + opt(n−1) − p are noted; and unless one
//! of them already gives V(n) (or `--all`), the branch and bound of `game.rs` decides whether any playable set beats
//! V(n), with the parity bound at every node (`--parity 0`: the Franklín–Moniot bound alone, as the searches of
//! `data/exact_2_1231.jsonl` ran).  Which games end up proven, given their predecessors, is worked out by
//! `tools/summarize.py exact` from the file.
//!
//! The roles of Theorem B (`--fix-low 1`, the default; `--fix-low 0` starts from the plain root, as the committed
//! files `data/exact_*.jsonl` were made): the search starts from the roles `upper::low_roles` writes down, "B" below,
//! the largest prime forced and every other prime barred, the sources of layer 2 in U\* forced and the other numbers of
//! layer 2 above n/2 barred, and, on a board whose frames of layer 2 are closed (the function decides it from the
//! board's own U\*, not from a board size), every number of layer 2 up to n/2 barred.  THEORY.md's Theorem B says every
//! optimal set obeys them, so a search from them that proves "no playable set obeying them beats V" still proves
//! opt(n) <= V (an optimal set beating V would obey them), and its line is written `"proven"` like any other.  The
//! recorded game must obey them (checked).  The sources of layer 3 and up stay free.  Against the plain root it takes
//! half the time a node (the parity solves have fewer free elements) at the plain search's nodes within a few percent
//! (1575 and 1584).  Because the proof rests on Theorem B besides the other theorems, every line records `"fix_low"`,
//! 1 or 0, and with it `fixed_bound`, the parity bound at the roles searched from.
//!
//! A search still running at `--seconds` pauses and saves its open subtrees and nogoods to a checkpoint in
//! `DIR` (default `work/checkpoints`); the next run resumes it, so a game may take any number of runs.  A search of
//! one kind never resumes another's: the checkpoint is `exact_N_low.ckpt`, `exact_N.ckpt` (`--fix-low 0`) or
//! `exact_N_upper.ckpt`.  Without the parity bound, a search that outlasts a 20-second warm-up, or resumes, goes on
//! with the Lagrangian bound fitted to the nogoods learned so far (`--lagrangian 0` turns that off, `--lagrangian 1`
//! turns it on with the parity bound too).  `--segments 1` turns on the segment cuts, which make a game's proof rest
//! on the smaller games' bounds (recorded as `"segments":1`); they are off by default, so that each search stands
//! alone.  A proof's `"matching_nodes"` are its nodes searched without the parity bound (a search resumed from a
//! checkpoint of such a run); its `"nodes"` count all of them.
//!
//! What the sources of layer 3 and up cost (E6): `--fix-upper 1` (it needs `--fix-low 1`) also fixes the whole upper
//! part to U\*, every source in it forced and every other barred, "D" below, which is the frame model's problem on
//! the whole board.  It proves nothing about opt(n) (that some optimal set has the upper part U\* is the open U of
//! THEORY.md), so a finished search of that kind is written `"search":"upper_fixed"` and no tool counts it as a
//! proof; it is a measurement, and its line has `"fix_upper":1`.  A playable set beating V is a real game whatever
//! the roles, so a search that finds one stops with an error in every mode.  Two counters show where B spends on the
//! sources of layer 3 and up, in every search (0 for a game not searched): `branched_on_sources`, the nodes that
//! branched on a set holding a free source of layer 3 or up, and `barred_upper`, the nodes whose roles, as the node
//! received them, bar a member of U\*_{>=3} (`game::Prover::watch`; they change nothing in the search).  `fm_bound`,
//! `parity_bound` and the choice of the games to search stay the whole board's.  `tools/summarize.py sources`
//! compares B with D.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use taxman_prover::game::{relation, Board, Prover, Role, Verdict};
use taxman_prover::jsonl::{campaign, parse_items, recorded_games, Obj};
use taxman_prover::parity::{Meet, Outcome, Parity};
use taxman_prover::primes::smallest_prime_factors;
use taxman_prover::upper::{low_roles, obeys, recorded_ustar, upper_fixed_roles, watched};

/// A paused search: its target, nodes so far (and how many of them without the parity bound), nogoods and open
/// units (roles as letters F, P, B).
struct Checkpoint {
    target: u64,
    nodes: u64,
    /// None in a checkpoint written before the parity bound existed: all its nodes were searched without it.
    matching_nodes: Option<u64>,
    /// Seconds spent in earlier runs.
    secs: f64,
    /// The counters of [`Prover::watch`] so far (0 in a checkpoint written before they existed).
    branching: u64,
    barring: u64,
    nogoods: Vec<Vec<u32>>,
    units: Vec<Vec<Role>>,
}

impl Checkpoint {
    fn save(&self, path: &Path) {
        let mut text = format!("target {}\nnodes {}\nsecs {}\n", self.target, self.nodes, self.secs);
        text += &format!("branching {}\nbarring {}\n", self.branching, self.barring);
        if let Some(m) = self.matching_nodes {
            text += &format!("matching_nodes {m}\n");
        }
        for k in &self.nogoods {
            text += &format!("nogood {}\n", k.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" "));
        }
        for u in &self.units {
            let letters: String = u
                .iter()
                .map(|r| match r {
                    Role::Free => 'F',
                    Role::Pick => 'P',
                    Role::Barred => 'B',
                })
                .collect();
            text += &format!("unit {letters}\n");
        }
        let tmp = path.with_extension("tmp");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&tmp, text).unwrap();
        fs::File::open(&tmp).unwrap().sync_all().unwrap();
        fs::rename(&tmp, path).unwrap();
    }

    fn load(path: &Path) -> Option<Checkpoint> {
        let text = fs::read_to_string(path).ok()?;
        let mut cp = Checkpoint {
            target: 0,
            nodes: 0,
            matching_nodes: None,
            secs: 0.0,
            branching: 0,
            barring: 0,
            nogoods: Vec::new(),
            units: Vec::new(),
        };
        for line in text.lines() {
            let (key, rest) = line.split_once(' ')?;
            match key {
                "target" => cp.target = rest.parse().ok()?,
                "nodes" => cp.nodes = rest.parse().ok()?,
                "matching_nodes" => cp.matching_nodes = Some(rest.parse().ok()?),
                "secs" => cp.secs = rest.parse().ok()?,
                "branching" => cp.branching = rest.parse().ok()?,
                "barring" => cp.barring = rest.parse().ok()?,
                "nogood" => cp.nogoods.push(rest.split(' ').map(|c| c.parse().unwrap()).collect()),
                "unit" => cp.units.push(
                    rest.chars()
                        .map(|ch| match ch {
                            'F' => Role::Free,
                            'P' => Role::Pick,
                            _ => Role::Barred,
                        })
                        .collect(),
                ),
                _ => return None,
            }
        }
        Some(cp)
    }
}

/// The counters of the prover's watch: (nodes that branched on a source of layer 3 or up, nodes that barred a member
/// of U\*_{>=3}); zeros for a prover that is not watching (a game that was not searched).
fn counters(prover: &Prover<'_>) -> (u64, u64) {
    prover.watch.as_ref().map_or((0, 0), |w| (w.branching, w.barring))
}

fn main() {
    let (items, flags) = parse_items(std::env::args().skip(1));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (mut threads, mut seconds, mut budget) = (1usize, f64::INFINITY, u64::MAX);
    let (mut all, mut parity, mut segments, mut lagrangian) = (false, true, false, None);
    // The roles of Theorem B are the default; --fix-low 0 starts from the plain root.
    let (mut fix_low, mut fix_upper) = (true, false);
    let mut optimal = root.join("../../src/main/resources/optimal.json");
    let mut results = root.join("../results/strict.jsonl");
    let mut checkpoints = root.join("work/checkpoints");
    let mut out = PathBuf::new();
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--seconds" => seconds = value.parse().unwrap(),
            "--budget" => budget = value.parse().unwrap(),
            "--all" => all = value == "1" || value == "true",
            "--parity" => parity = value == "1" || value == "true",
            "--segments" => segments = value == "1" || value == "true",
            "--lagrangian" => lagrangian = Some(value == "1" || value == "true"),
            "--fix-low" => fix_low = value == "1" || value == "true",
            "--fix-upper" => fix_upper = value == "1" || value == "true",
            "--checkpoints" => checkpoints = value.into(),
            "--optimal" => optimal = value.into(),
            "--results" => results = value.into(),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    assert!(fix_low || !fix_upper, "--fix-upper needs the roles of Theorem B (--fix-low 1, the default)");
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
    let known = recorded_games(&[&optimal, &results]);
    let items: Vec<u64> = items.into_iter().filter(|n| *n >= 2 && known.contains_key(n)).collect();
    let spf = smallest_prime_factors(items.iter().copied().max().unwrap_or(2) as usize);
    let pause_at = Instant::now() + Duration::from_secs_f64(seconds.min(1e9));
    campaign("exact", &items, threads, seconds, &out, |n| {
        let started = Instant::now();
        let earlier = std::cell::Cell::new(0.0f64);
        let (score, moves) = &known[&n];
        let board = Board::new(n as u32, &spf);
        let replayed = board.replay(moves);
        assert_eq!(replayed, Ok(*score), "n={n}: the recorded game does not replay to its score");
        let prev = known.get(&(n - 1)).map_or(0, |(s, _)| *s);
        let relation = relation(n, *score, prev, &spf).name();
        let mut prover = Prover::new(&board, *score, budget).pause_at(pause_at);
        if parity {
            prover = prover.with_parity();
        }
        if segments {
            let smaller: Vec<u64> = (0..n).map(|m| known.get(&m).map_or(0, |(s, _)| *s)).collect();
            prover = prover.with_smaller_games(&smaller);
        }
        let fm = prover.root_bound();
        assert!(fm >= *score, "n={n}: the matching bound {fm} is below a legal game's {score}");
        let weights: Vec<i64> = (0..=n as i64).collect();
        let parity_at = |roles: &[Role]| Meet::new(Parity::pair(&board, &weights)).solve(roles);
        let parity_bound = {
            let mut roles = prover.root_roles();
            let Outcome::Bound(root_bound) = parity_at(&roles) else {
                panic!("n={n}: the root's parity bound is not certified")
            };
            assert!(root_bound.value >= *score as i64, "n={n}: the parity bound is below a legal game's {score}");
            // The recorded game's moves are independent in both parity matroids (Theorem 10).
            moves.iter().for_each(|&c| roles[c as usize] = Role::Pick);
            let independent = parity_at(&roles);
            assert!(matches!(independent, Outcome::Bound(_)), "n={n}: the recorded game is dependent");
            root_bound.value as u64
        };
        let ckpt = checkpoints.join(format!("exact_{n}{kind}.ckpt"));
        let (mut matching_nodes, mut loaded_nodes) = (0u64, 0u64);
        let bounded = fm == *score || (parity && parity_bound == *score);
        let skip = bounded || (relation != "none" && !all);
        // U* fixes the roles and names the numbers the counters watch; a game that is left alone needs neither.
        let ustar = if fix_low || !skip { recorded_ustar(n, moves, &spf) } else { Vec::new() };
        // The roles the search starts from (those of Theorem B unless --fix-low 0 says the plain root): the recorded
        // game obeys them, so their parity bound is at least its score.
        let unit = if fix_upper {
            upper_fixed_roles(n as u32, &spf, &ustar)
        } else if fix_low {
            low_roles(n as u32, &spf, &ustar)
        } else {
            prover.root_roles()
        };
        if fix_low {
            obeys(&unit, moves)
                .unwrap_or_else(|e| panic!("n={n}: the recorded game breaks the roles searched from: {e}"));
        }
        let fixed_bound = fix_low.then(|| {
            let Outcome::Bound(bound) = parity_at(&unit) else {
                panic!("n={n}: the parity bound at the roles searched from is not certified")
            };
            assert!(bound.value >= *score as i64, "n={n}: the bound at the roles searched from is below {score}");
            bound.value as u64
        });
        let (search, note) = if skip {
            ("skipped", None)
        } else {
            let (branch, barred) = watched(n as u32, &spf, &ustar);
            prover = prover.watch(branch, barred);
            // A search that outlasts a short warm-up (or resumes from a checkpoint) goes on with the Lagrangian
            // bound fitted to the nogoods learned so far.
            let warmup = (Instant::now() + Duration::from_secs(20)).min(pause_at);
            let first = match Checkpoint::load(&ckpt) {
                Some(cp) => {
                    assert_eq!(cp.target, *score, "n={n}: the checkpoint is for another target");
                    prover.nodes = cp.nodes;
                    loaded_nodes = cp.nodes;
                    matching_nodes = cp.matching_nodes.unwrap_or(cp.nodes);
                    earlier.set(cp.secs);
                    prover.nogoods = cp.nogoods;
                    if let Some(watch) = prover.watch.as_mut() {
                        (watch.branching, watch.barring) = (cp.branching, cp.barring);
                    }
                    prover.take_units(pause_at);
                    Err(cp.units)
                }
                None => {
                    prover.take_units(warmup);
                    match prover.run_units(vec![unit]) {
                        Verdict::Paused => Err(prover.take_units(pause_at)),
                        other => Ok(other),
                    }
                }
            };
            let verdict = match first {
                Ok(verdict) => verdict,
                Err(units) => {
                    if lagrangian {
                        prover.enable_lagrangian(200);
                    }
                    prover.run_units(units)
                }
            };
            if !parity {
                matching_nodes += prover.nodes - loaded_nodes;
            }
            match verdict {
                // A search from the roles of the frame model's problem proves nothing about opt(n).
                Verdict::Proven => (if fix_upper { "upper_fixed" } else { "proven" }, None),
                Verdict::Budget => ("budget", Some(format!("budget exhausted after {} nodes", prover.nodes))),
                Verdict::Paused => {
                    let (branching, barring) = counters(&prover);
                    let cp = Checkpoint {
                        target: *score,
                        nodes: prover.nodes,
                        matching_nodes: Some(matching_nodes),
                        secs: earlier.get() + started.elapsed().as_secs_f64(),
                        branching,
                        barring,
                        nogoods: prover.nogoods.clone(),
                        units: prover.open_units().to_vec(),
                    };
                    cp.save(&ckpt);
                    let note = format!("paused after {} nodes, {} open units", prover.nodes, cp.units.len());
                    return (None, Some(note));
                }
                Verdict::Beaten(picks) => {
                    let sum: u64 = picks.iter().map(|&c| u64::from(c)).sum();
                    panic!("n={n}: a playable set worth {sum} beats the recorded {score}: {picks:?}");
                }
            }
        };
        let _ = fs::remove_file(&ckpt);
        let (branching, barring) = counters(&prover);
        let mut line = Obj::new()
            .num("n", n)
            .num("score", *score)
            .num("fm_bound", fm)
            .num("parity_bound", parity_bound)
            .str("relation", relation)
            .str("search", search)
            .num("parity", u8::from(parity && search != "skipped"))
            .num("segments", u8::from(segments && search != "skipped"))
            .num("nodes", prover.nodes)
            .num("matching_nodes", matching_nodes)
            .float("secs", earlier.get() + started.elapsed().as_secs_f64())
            .num("branched_on_sources", branching)
            .num("barred_upper", barring)
            .num("fix_low", u8::from(fix_low));
        if let Some(bound) = fixed_bound {
            line = line.num("fixed_bound", bound);
        }
        if fix_upper {
            line = line.num("fix_upper", 1);
        }
        (Some(line.finish()), note)
    });
}
