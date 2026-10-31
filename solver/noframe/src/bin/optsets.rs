//! Every optimal set, on the whole board: for every game in a range, all playable sets of picks worth the recorded
//! optimum V(n) (the committed chain and the `--results` files; see solver/noframe/docs/reproducing.md, "The recorded
//! games").  One line per game, resumable:
//!
//!     optsets --from A --to B [--list ...] [--threads T] [--seconds S] [--budget NODES] [--limit SETS]
//!             [--upper all|fixed|other] [--derive 0] [--searched-only 1] [--segments 1] [--optimal FILE]
//!             [--results FILE]... --out FILE
//!
//! `--upper` splits the question in two: `fixed` lists only the optimal sets whose upper part (the numbers above
//! n/2) is U\*, descending greedy's upper set, by a search with the upper half fixed; `other` lists only those whose
//! upper part is not U\*, by a search whose root is split on the largest upper number where the set and U\*
//! disagree.  The two lists partition the full one (`all`, the default).  Derivations are made only for `all`.
//!
//! A game whose V(n) is at its ceiling or meets the prime or the semiprime formula gets its sets from those of n − 1
//! ([`derive_optimal_sets`]; the line says `"source":"chain"`, `"prime"` or `"semiprime"`, and rests on V(n − 1) being
//! the true optimum, proven on the whole board to 989); every other game, and every game with `--derive 0`, is
//! searched: the branch and bound of `game.rs` in enumeration mode with target V(n) − 1 lists every playable set
//! beating it (one beating V(n) would refute the recorded optimum and stops the run), up to `--limit` sets (default
//! unlimited) within `--budget` nodes.  The predecessors a derivation needs come from earlier lines of the output
//! file, or are worked out on the spot.  Threads working on neighboring games would work out the same predecessors,
//! so a long range is best run in two passes: `--searched-only 1` (only the games that need a search, in parallel),
//! then the whole range (the derivations, instantly, from the first pass's lines).
//!
//! The line records how the enumeration ended ("complete", "limit" or "budget"), how many sets it found, how many
//! distinct upper parts (numbers above n/2) and lower parts they have, how many have U\* (descending greedy's upper
//! set) as their upper part, the canonical set (the largest in decreasing lexicographic order: compare the largest
//! members, then the next, ...) when it is not the recorded game's, and every other set as its difference from
//! the canonical one, {"out": [...], "in": [...]} (at most 200).  `--segments 1` turns on the segment cuts, which
//! make the answer rest on the recorded optima of the smaller games being the true ones; the line says so.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use taxman_prover::game::{
    derive_optimal_sets, relation, the_prime_of_every_optimal_set, Board, Prover, Relation, Role, Verdict,
};
use taxman_prover::jsonl::{
    campaign, committed_chain, field_nums, field_str, field_u64, parse_items, recorded_games_to, Obj,
};
use taxman_prover::peel::u_star;
use taxman_prover::primes::smallest_prime_factors;

/// At most this many sets are written per line (besides the canonical one).
const WRITTEN: usize = 200;

/// A set in decreasing order: the canonical order compares these lexicographically.
fn descending(set: &[u32]) -> Vec<u32> {
    let mut d = set.to_vec();
    d.sort_unstable_by(|a, b| b.cmp(a));
    d
}

/// How a game's sets were found.
struct Outcome {
    /// Each set in decreasing order, the canonical one first.
    sets: Vec<Vec<u32>>,
    ended: &'static str,
    source: &'static str,
    nodes: u64,
}

struct Enumerator {
    known: std::collections::BTreeMap<u64, (u64, Vec<u32>)>,
    spf: Vec<u32>,
    budget: u64,
    limit: usize,
    derive: bool,
    segments: bool,
    /// "all", "fixed" or "other" (see the top).
    upper: String,
    /// The complete list of optimal sets of each game worked out so far, or None if it could not be had.
    cache: Mutex<HashMap<u64, Option<Vec<Vec<u32>>>>>,
}

impl Enumerator {
    fn relation(&self, n: u64) -> Relation {
        let prev = self.known.get(&(n - 1)).map_or(0, |g| g.0);
        relation(n, self.known[&n].0, prev, &self.spf)
    }

    /// Whether game n's sets are derived from game n − 1's rather than searched.
    fn derivable(&self, n: u64) -> bool {
        self.derive && self.upper == "all" && self.known.contains_key(&(n - 1)) && self.relation(n) != Relation::None
    }

    /// Every optimal set of game m, if they can all be had (from the cache, by derivation or by search).
    fn all_sets(&self, m: u64) -> Option<Vec<Vec<u32>>> {
        if m < 2 {
            return Some(vec![Vec::new()]);
        }
        if let Some(sets) = self.cache.lock().unwrap().get(&m) {
            return sets.clone();
        }
        let outcome = self.solve(m);
        (outcome.ended == "complete").then_some(outcome.sets)
    }

    fn solve(&self, n: u64) -> Outcome {
        let (score, _) = &self.known[&n];
        let board = Board::new(n as u32, &self.spf);
        let rel = self.relation(n);
        let derived = if self.derivable(n) {
            self.all_sets(n - 1).map(|prev| derive_optimal_sets(&board, rel, &prev, &self.spf))
        } else {
            None
        };
        let mut outcome = match derived {
            Some(sets) => Outcome { sets, ended: "complete", source: rel.name(), nodes: 0 },
            None => {
                let mut prover = Prover::new(&board, score - 1, self.budget).enumerate(self.limit);
                if self.segments {
                    let smaller: Vec<u64> = (0..n).map(|m| self.known.get(&m).map_or(0, |g| g.0)).collect();
                    prover = prover.with_smaller_games(&smaller);
                }
                let root = prover.root_roles();
                let units = match self.upper.as_str() {
                    "all" => vec![root],
                    upper => {
                        // Agree with U* above c, and (fixed) at c too, or (other) disagree at c.
                        let star = u_star(n, &self.spf);
                        let role = |c: u32| if star.contains(&c) { Role::Pick } else { Role::Barred };
                        let flip = |r: Role| if r == Role::Pick { Role::Barred } else { Role::Pick };
                        let agree_above = |c: u32| {
                            let mut roles = root.clone();
                            (c + 1..=n as u32).for_each(|d| roles[d as usize] = role(d));
                            roles
                        };
                        match upper {
                            "fixed" => vec![agree_above(n as u32 / 2)],
                            _ => (n as u32 / 2 + 1..=n as u32)
                                .map(|c| {
                                    let mut roles = agree_above(c);
                                    roles[c as usize] = flip(role(c));
                                    roles
                                })
                                .collect(),
                        }
                    }
                };
                let ended = match prover.run_units(units) {
                    Verdict::Proven => "complete",
                    Verdict::Beaten(_) => "limit",
                    Verdict::Budget => "budget",
                    Verdict::Paused => unreachable!("no pause deadline"),
                };
                Outcome { sets: prover.found, ended, source: "search", nodes: prover.nodes }
            }
        };
        for s in outcome.sets.iter_mut() {
            *s = descending(s);
            let sum: u64 = s.iter().map(|&c| u64::from(c)).sum();
            assert!(sum <= *score, "n={n}: a playable set worth {sum} beats the recorded {score}: {s:?}");
            assert_eq!(sum, *score);
            the_prime_of_every_optimal_set(s, n as u32, &self.spf).unwrap();
        }
        outcome.sets.sort_unstable_by(|a, b| b.cmp(a));
        let complete = (outcome.ended == "complete").then(|| outcome.sets.clone());
        self.cache.lock().unwrap().insert(n, complete);
        outcome
    }

    /// The lists of optimal sets recorded in earlier lines of the output: complete (with every set written), or
    /// known not to be.
    fn prefill(&self, lines: &[String]) {
        for line in lines {
            let (Some(n), Some(count)) = (field_u64(line, "n"), field_u64(line, "sets")) else { continue };
            if field_str(line, "upper").is_some_and(|u| u != "all") {
                continue;
            }
            if field_str(line, "ended") != Some("complete") || count as usize > WRITTEN + 1 {
                self.cache.lock().unwrap().insert(n, None);
                continue;
            }
            let Some((_, moves)) = self.known.get(&n) else { continue };
            let canonical: Vec<u32> = match field_nums(line, "canonical") {
                Some(c) => c.into_iter().map(|c| c as u32).collect(),
                None => descending(moves),
            };
            let mut sets = vec![canonical.clone()];
            let others = line.find("\"others\":[").map_or("", |at| &line[at..]);
            for diff in others.split("{\"out\":").skip(1) {
                let diff = format!("\"out\":{diff}");
                let gone: Vec<u32> = field_nums(&diff, "out").unwrap().into_iter().map(|c| c as u32).collect();
                let new = field_nums(&diff, "in").unwrap().into_iter().map(|c| c as u32);
                let set: Vec<u32> = canonical.iter().copied().filter(|c| !gone.contains(c)).chain(new).collect();
                sets.push(descending(&set));
            }
            assert_eq!(sets.len() as u64, count, "n={n}: the line's sets do not add up");
            self.cache.lock().unwrap().insert(n, Some(sets));
        }
    }
}

fn main() {
    let (items, flags) = parse_items(std::env::args().skip(1));
    let (mut threads, mut seconds, mut budget, mut limit) = (1usize, f64::INFINITY, u64::MAX, 0);
    let (mut derive, mut searched_only, mut segments, mut upper) = (true, false, false, String::from("all"));
    let mut files = vec![committed_chain()];
    let mut out = PathBuf::new();
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--seconds" => seconds = value.parse().unwrap(),
            "--budget" => budget = value.parse().unwrap(),
            "--limit" => limit = value.parse().unwrap(),
            "--derive" => derive = value == "1" || value == "true",
            "--searched-only" => searched_only = value == "1" || value == "true",
            "--segments" => segments = value == "1" || value == "true",
            "--upper" => upper = value,
            "--optimal" => files[0] = value.into(),
            "--results" => files.push(value.into()),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    assert!(["all", "fixed", "other"].contains(&upper.as_str()), "--upper is all, fixed or other");
    let known = recorded_games_to(&files, items.iter().copied().max().unwrap_or(0));
    let mut items: Vec<u64> = items.into_iter().filter(|n| *n >= 2 && known.contains_key(n)).collect();
    let spf = smallest_prime_factors(items.iter().copied().max().unwrap_or(2) as usize);
    let limit = if limit == 0 { usize::MAX } else { limit };
    let e = Enumerator { known, spf, budget, limit, derive, segments, upper, cache: Mutex::new(HashMap::new()) };
    if searched_only {
        items.retain(|&n| !e.derivable(n));
    }
    if out.exists() {
        let text = std::fs::read_to_string(&out).expect("read the output file");
        e.prefill(&text.lines().map(String::from).collect::<Vec<_>>());
    }
    campaign("optsets", &items, threads, seconds, &out, |n| {
        let started = Instant::now();
        let (score, moves) = &e.known[&n];
        let board = Board::new(n as u32, &e.spf);
        assert_eq!(board.replay(moves), Ok(*score), "n={n}: the recorded game does not replay to its score");
        let Outcome { sets, ended, source, nodes } = e.solve(n);
        let half = (n / 2) as u32;
        let upper_part = |s: &[u32]| -> Vec<u32> { s.iter().copied().filter(|&c| c > half).collect() };
        let u_star = u_star(n, &e.spf);
        let recorded = descending(moves);
        let in_class = match e.upper.as_str() {
            "all" => true,
            "fixed" => upper_part(&recorded) == u_star,
            _ => upper_part(&recorded) != u_star,
        };
        assert!(
            ended != "complete" || !in_class || sets.contains(&recorded),
            "n={n}: the recorded game is not among the sets"
        );
        let distinct = |above: bool| {
            let mut parts: Vec<Vec<u32>> =
                sets.iter().map(|s| s.iter().copied().filter(|&c| (c > half) == above).collect()).collect();
            parts.sort_unstable();
            parts.dedup();
            parts.len()
        };
        let upper_is_greedy = sets.iter().filter(|s| upper_part(s) == u_star).count();
        let canonical = sets.first().cloned().unwrap_or_default();
        let mut line = Obj::new()
            .num("n", n)
            .num("score", *score)
            .str("relation", e.relation(n).name())
            .str("upper", &e.upper)
            .str("source", source)
            .str("ended", ended)
            .num("segments", u8::from(e.segments && source == "search"))
            .num("sets", sets.len())
            .num("upper_parts", distinct(true))
            .num("lower_parts", distinct(false))
            .num("upper_is_greedy", upper_is_greedy)
            .num("recorded_is_canonical", u8::from(canonical == recorded));
        if !sets.is_empty() && canonical != recorded {
            line = line.nums("canonical", &canonical);
        }
        let diffs: Vec<Obj> = sets
            .iter()
            .skip(1)
            .take(WRITTEN)
            .map(|s| {
                let gone: Vec<u32> = canonical.iter().copied().filter(|c| !s.contains(c)).collect();
                let new: Vec<u32> = s.iter().copied().filter(|c| !canonical.contains(c)).collect();
                Obj::new().nums("out", &gone).nums("in", &new)
            })
            .collect();
        let line =
            line.objs("others", diffs).num("nodes", nodes).float("secs", started.elapsed().as_secs_f64()).finish();
        let note = (sets.len() > 1 || ended != "complete").then(|| format!("{} sets, {ended}, {source}", sets.len()));
        (Some(line), note)
    });
}
