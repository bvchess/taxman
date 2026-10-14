//! The prover's proofs of optimal scores: for every game in a range, an upper bound on opt(n) proven without the
//! frame model, compared with the recorded optimum V(n) (optimal.json, which reaches 1300, and for a board size it
//! lacks the game of `--results`, by default the solver's `results/strict.jsonl`).  One line per game, resumable:
//!
//!     exact --from A --to B [--list ...] [--threads T] [--seconds S] [--budget NODES] [--all 1] [--parity 0]
//!           [--segments 1] [--lagrangian 0|1] [--checkpoints DIR] [--optimal FILE] [--results FILE] --out FILE
//!
//! Per game: the recorded game is replayed by the rules (a lower bound, opt(n) >= V(n)); the Franklín–Moniot
//! bound and the parity bound (THEORY.md, Theorem 10; `parity.rs`) are computed at the root; the ceiling
//! opt(n) <= n + opt(n−1) and, for a prime n, the prime formula opt(n) <= n + opt(n−1) − p are noted; and unless one
//! of them already gives V(n) (or `--all`), the branch and bound of `game.rs` decides whether any playable set beats
//! V(n), with the parity bound at every node (`--parity 0`: the Franklín–Moniot bound alone, as the searches of
//! `data/exact_2_1231.jsonl` ran).  Which games end up proven, given their predecessors, is worked out by
//! `tools/summarize.py exact` from the file.
//!
//! A search still running at `--seconds` pauses and saves its open subtrees and nogoods to
//! `DIR/exact_N.ckpt` (default `work/checkpoints`); the next run resumes it, so a game may take any number of
//! runs.  Without the parity bound, a search that outlasts a 20-second warm-up, or resumes, goes on with the
//! Lagrangian bound fitted to the nogoods learned so far (`--lagrangian 0` turns that off, `--lagrangian 1` turns
//! it on with the parity bound too).  `--segments 1` turns on the segment cuts, which make a game's proof rest on
//! the smaller games' bounds (recorded as `"segments":1`); they are off by default, so that each search stands
//! alone.  A proof's `"matching_nodes"` are its nodes searched without the parity bound (a search resumed from a
//! checkpoint of such a run); its `"nodes"` count all of them.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use taxman_prover::game::{relation, Board, Prover, Role, Verdict};
use taxman_prover::jsonl::{campaign, parse_items, recorded_games, Obj};
use taxman_prover::parity::{Meet, Outcome, Parity};
use taxman_prover::primes::smallest_prime_factors;

/// A paused search: its target, nodes so far (and how many of them without the parity bound), nogoods and open
/// units (roles as letters F, P, B).
struct Checkpoint {
    target: u64,
    nodes: u64,
    /// None in a checkpoint written before the parity bound existed: all its nodes were searched without it.
    matching_nodes: Option<u64>,
    /// Seconds spent in earlier runs.
    secs: f64,
    nogoods: Vec<Vec<u32>>,
    units: Vec<Vec<Role>>,
}

impl Checkpoint {
    fn save(&self, path: &Path) {
        let mut text = format!("target {}\nnodes {}\nsecs {}\n", self.target, self.nodes, self.secs);
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
        let mut cp =
            Checkpoint { target: 0, nodes: 0, matching_nodes: None, secs: 0.0, nogoods: Vec::new(), units: Vec::new() };
        for line in text.lines() {
            let (key, rest) = line.split_once(' ')?;
            match key {
                "target" => cp.target = rest.parse().ok()?,
                "nodes" => cp.nodes = rest.parse().ok()?,
                "matching_nodes" => cp.matching_nodes = Some(rest.parse().ok()?),
                "secs" => cp.secs = rest.parse().ok()?,
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

fn main() {
    let (items, flags) = parse_items(std::env::args().skip(1));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (mut threads, mut seconds, mut budget) = (1usize, f64::INFINITY, u64::MAX);
    let (mut all, mut parity, mut segments, mut lagrangian) = (false, true, false, None);
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
            "--checkpoints" => checkpoints = value.into(),
            "--optimal" => optimal = value.into(),
            "--results" => results = value.into(),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
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
        let parity_bound = {
            let weights: Vec<i64> = (0..=n as i64).collect();
            let mut roles = prover.root_roles();
            let Outcome::Bound(root_bound) = Meet::new(Parity::pair(&board, &weights)).solve(&roles) else {
                panic!("n={n}: the root's parity bound is not certified")
            };
            assert!(root_bound.value >= *score as i64, "n={n}: the parity bound is below a legal game's {score}");
            // The recorded game's moves are independent in both parity matroids (Theorem 10).
            moves.iter().for_each(|&c| roles[c as usize] = Role::Pick);
            let independent = Meet::new(Parity::pair(&board, &weights)).solve(&roles);
            assert!(matches!(independent, Outcome::Bound(_)), "n={n}: the recorded game is dependent");
            root_bound.value as u64
        };
        let ckpt = checkpoints.join(format!("exact_{n}.ckpt"));
        let (mut matching_nodes, mut loaded_nodes) = (0u64, 0u64);
        let bounded = fm == *score || (parity && parity_bound == *score);
        let (search, note) = if bounded || (relation != "none" && !all) {
            ("skipped", None)
        } else {
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
                    prover.take_units(pause_at);
                    Err(cp.units)
                }
                None => {
                    prover.take_units(warmup);
                    match prover.run() {
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
                Verdict::Proven => ("proven", None),
                Verdict::Budget => ("budget", Some(format!("budget exhausted after {} nodes", prover.nodes))),
                Verdict::Paused => {
                    let cp = Checkpoint {
                        target: *score,
                        nodes: prover.nodes,
                        matching_nodes: Some(matching_nodes),
                        secs: earlier.get() + started.elapsed().as_secs_f64(),
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
        let line = Obj::new()
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
            .finish();
        (Some(line), note)
    });
}
