//! The sacrifice bound: an upper bound on game `n` from games on the board `n − 1` with one number taken off.
//!
//! # Status: a study, with a negative result
//!
//! **This is study code.  The solver does not use it.**  It is kept because it holds a theorem (proved below) and
//! a measured negative result, and because the tests use it as an independent check.
//!
//! * *The question.*  The tactic `reuse_prev` (since removed, docs/EXPERIMENTS.md, "Which tactics earn their keep")
//!   accepted a derived game unproven.  Could an upper bound on `opt(n)` computed from games of `n − 1` prove those
//!   games optimal, or settle other games faster than searching them?
//! * *The answer is no.*  The bound is a theorem, and the frame model computes it correctly (checked against
//!   exhaustive search, [`crate::brute`]), but it is tight on too few of the games that matter, and computing it
//!   costs about as much as solving the game: it is no replacement for the tactic, and putting it into the
//!   pipeline slowed the games it was tried on (numbers under "Findings"; `examples/sacrifice.rs` made them).
//! * *What stays in use.*  `tests/wide.rs` uses [`sacrifice_bound`] as an upper bound that does not come from the
//!   search under test, to prove one result at `W = 20` (game 1133), beyond what the Java can check.  The support
//!   for punctured boards (holes) exists for this study alone: [`crate::numtheory::Tables::punctured`],
//!   [`crate::game::replay_punctured`], and the hole cases of [`crate::frames`].
//! * *Not here.*  The proof that the prime shortcut is optimal, which the solver does rely on, is with the shortcut
//!   ([`crate::accelerate::prime_shortcut`]); the "Prime `n`" remark below is its case in this setting.
//!
//! # Notation
//!
//! `opt(n)` is the best score on the board `[n] = {1, …, n}`; `mf(n)` the maximal factors of `n` (`n/p` for each
//! prime `p | n`).  For a board `B ⊆ [n]` (the other numbers are *holes*: never in the pot, so never moves and
//! never tax), `opt_M(B)` is the best score on `B` under the *maximal-factor rule*: a move needs one of its
//! maximal factors in the pot, not merely some divisor ([`crate::game::replay_punctured`]).  On `[n]` itself
//! the rule changes nothing (lifting: a divisor `d` in the pot lies below a maximal factor `g`, `d | g | m`, and
//! whatever removed `g` would have removed `d`), so `opt_M([n]) = opt(n)`.  On a punctured board it is
//! stricter, so `opt_M(B) ≤ opt(B)`.  Write `B_f = [n − 1] ∖ {f}`.
//!
//! # Theorem
//!
//! For `n ≥ 2`:  `opt(n) ≤ max( opt(n − 1),  n + max_{f ∈ mf(n)} opt_M(B_f) )`.
//!
//! Equivalently, with the *sacrifice* `σ(n) = n + opt(n − 1) − opt(n)` and the *ban cost*
//! `c(f) = opt(n − 1) − opt_M(B_f)`:  `σ(n) ≥ min(n, min_f c(f))`.
//!
//! # Proof
//!
//! Let `S` be an optimal game on `[n]`.  If `n ∉ S`, `S` is a legal game on `[n − 1]`: `n` divides nothing else on
//! the board, so it is never tax and never swept, and removing it from the board changes no other move.  Then
//! `opt(n) ≤ opt(n − 1)`.
//!
//! Otherwise let `P` be the pot just before `n` is played.  `n` pays tax, so some proper divisor `d` of `n` is in
//! `P`.  Take a prime `p | n/d` and `f = n/p ∈ mf(n)`, so `d | f`.  Then `f ∈ P`: if `d = f` this is given;
//! otherwise `d < f`, and whatever removed `f` earlier (playing `f`, or playing a multiple of `f`) would have
//! removed its divisor `d` too.  `n`'s move removes `f`, so
//!
//! * (a) `f ∉ S`;
//! * (b) no move before `n` is a multiple of `f` (it would have removed `f`);
//! * (c) no move after `n` finds `f` in the pot.
//!
//! Let `S'` be `S` without `n`, in the same order, played on `B_f`.  Compare the pots before each move of `S'`:
//! `P_t` in the game `S` on `[n]`, `P'_t` in `S'` on `B_f`.  Invariant: `P_t ∖ {f, n} ⊆ P'_t`, with equality up
//! to `n`'s turn.  Before `n`'s turn, a move `m` divides by neither `f` (b) nor `n` (`m < n`), so its divisors miss
//! `{f, n}` and both games remove the same numbers apart from `f` and `n`.  At `n`'s turn `S` removes `n` and
//! its divisors and `S'` removes nothing, which keeps the inclusion.  Later moves remove the same numbers (the
//! move and its divisors) from both pots, which keeps it too.  Now every move `m` of `S'` is legal in `S`, so by
//! lifting on `[n]` some maximal factor `g` of `m` is in `P_t`.  `g ≠ f`: before `n`'s turn by (b), after it by
//! (c); and `g ≠ n` as `g < m < n`.  So `g ∈ P'_t`; likewise `m ∈ P'_t` (`m ≠ f` by (a), `m ≠ n`).  Each move
//! of `S'` is therefore legal on `B_f` under the maximal-factor rule, and `opt(n) − n = sum(S') ≤ opt_M(B_f)`.
//! Taking the `f` with the largest `opt_M(B_f)` gives the theorem.  For the second form, subtract from
//! `n + opt(n − 1)`.  ∎
//!
//! Remarks.
//!
//! * The proof gives `opt(n) − n ≤ opt_M(B_f)` for *every* maximal factor `f` of `n` that is in the pot at
//!   `n`'s turn (indeed with every divisor of `n` in the pot then taken off the board), but which those are is
//!   not known in advance, and `opt_M` only grows as holes are filled, hence the maximum over single holes.
//!   With the rules' `opt(B_f)` in place of `opt_M(B_f)` the theorem still holds but is weaker.
//! * It is an upper bound only.  In the terms of Theorem 1 of `heuristics/THEORY.md` (a set of moves is
//!   playable iff each can be paid a distinct maximal factor outside the set, with an acyclic precedence "`a`
//!   before `b` when `a`'s payment or `a` divides `b`"), `n + opt_M(B_f)` is the best set containing `n` with `n`
//!   paying `f`, except that the precedence edges at `n` are dropped: `n` before every multiple of `f`, and after
//!   every move that pays with a divisor of `n` or divides `n`.  So when `n` is played, the bound is tight exactly
//!   when, for some `f` with the largest `opt_M(B_f)`, some best game of `B_f` stays acyclic with `n` added.
//! * `c(f) = 0` whenever some optimal game of `n − 1` replays on `B_f` under the maximal-factor rule (it never
//!   plays `f` and every move has a maximal factor other than `f` in the pot).  If that holds for some `f`, the
//!   bound is the trivial `n + opt(n − 1)`.  The bound bites only when every maximal factor of `n` is
//!   load-bearing.
//! * Prime `n` (`mf(n) = {1}`).  On `B_1` no prime can be played, and the two rules agree (the other numbers have
//!   maximal factors above 1).  Let `P` be the largest prime below `n`.  Every optimal game of `n − 1` opens with
//!   `P` (Lemma A of [`crate::accelerate::prime_shortcut`], where it is proved).  Dropping that first move gives
//!   a game on `B_1`, so `opt_M(B_1) ≥ opt(n − 1) − P`; and `P` followed by any game on `B_1` is a game on
//!   `[n − 1]` (no multiple of `P` is on the board), so `opt_M(B_1) = opt(n − 1) − P`.  The bound is
//!   `n + opt(n − 1) − P`, which the prime shortcut reaches: it is tight for every prime.  (That the shortcut is
//!   optimal needs no frame-model assumption.  Its proof is a direct argument that does not go through this
//!   theorem, and it lives with the shortcut.)  [`sacrifice_bound`] uses this value for the hole 1 without any
//!   search (`without_one`).
//!
//! # Computing it
//!
//! [`punctured`] computes `opt_M(B)` for `B = [n] ∖ {hole}` with the frame model on [`Tables::punctured`]: the
//! hole is in no frame and no factor or composite list ([`BoardFrames`](crate::frames::BoardFrames), the board's
//! frames), so the frame model plays exactly the maximal-factor rule.  Two things had to be right for that, both found
//! by the brute force: the sources stay the numbers above `n/2` (a number whose only maximal composite is the hole is
//! optional, not a source, which the model plays), and the frames are ordered through the hole (a number still sweeps
//! divisors whose maximal-factor chain to it passes through the hole).  The frame model's own rules (greedy
//! impossible sources, mandatory moves, closed frames, one-frame promotions; DESIGN.md §2 and §5, and
//! [`crate::frames`]) apply to punctured boards as they do to `[n]`.  On `[n]` they are proven, given that the
//! numbering of the frames is clean (heuristics/THEORY.md, Theorem M), but a punctured board is not covered: the
//! proof is for `[n]`, and the numbering of a punctured board need not be clean (56 without 39 is not).  There they
//! are only validated: against the brute force ([`crate::brute`]) on every board `[N] ∖ {h}` with `N ≤ 52`.  A
//! bound computed this way is as sound as the frame model on the punctured board, no more.  Each punctured game is warm-started from an optimal game of the unpunctured board,
//! which is its ceiling: if that game replays on the punctured board, its score is the answer with no search.
//!
//! # Findings (the negative result)
//!
//! Measured with `examples/sacrifice.rs` (2026-10; 2 threads; its selection of the games `reuse_prev` derived went
//! with the tactic):
//!
//! * By exhaustive search, no frame model, `n = 2..66`: the theorem holds for every `n` and `f`.  The bound is
//!   tight on 61 of the 65 games (58 with the rules' `opt(B_f)`), and on 20 of the 24 that are neither prime nor
//!   exact (`opt(n) ≠ n + opt(n − 1)`); the misses, 21, 39, 51 and 57, are all `3p`.
//! * By the frame model, `n = 2..800`: never below `optimal.json`; tight on 638 of 799 games, 229 of the 390 that
//!   are neither prime nor exact, and only 57 of the 152 of those that `reuse_prev` derived.  The misses are
//!   mostly semiprimes `pq` (96 of 161), whose two ban costs are often equal; 146 of the 152 `reuse_prev` games
//!   were semiprimes.
//! * A punctured game is about as hard as game `n` itself (for `pq` the two searches nearly copy the direct one),
//!   so on the games it proves the bound costs 1.1–8.7 times the direct search (median 2.0, 1001–1199).  Putting
//!   it between the ceiling and the search (prime → ceiling → bound → search) made 93 games of 1001–1199 73% slower
//!   than prime → ceiling → search.  It is no sound replacement for `reuse_prev`: it proved too few of its games, and
//!   searching proves all of them sooner.

use crate::accelerate::solution_for_promotions;
use crate::board::Board;
use crate::engine::EngineConfig;
use crate::game::replay_punctured;
use crate::numtheory::Tables;
use crate::search::{self, SearchTactics};
use std::time::{Duration, Instant};

/// How [`punctured`] found its game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// The unpunctured board's game replays on the punctured one: the ban costs nothing.
    Replay,
    /// No promotion candidates: the game with no promotions is the only one.
    Trivial,
    /// The frame model's search.
    Search,
    /// The hole is 1 (`n` prime): the previous game without its first move, by the remark on primes.
    Prime,
}

impl How {
    pub fn as_str(self) -> &'static str {
        match self {
            How::Replay => "replay",
            How::Trivial => "trivial",
            How::Search => "search",
            How::Prime => "prime",
        }
    }
}

/// The best game on `[n] ∖ {hole}` under the maximal-factor rule, as the frame model finds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Punctured {
    pub hole: u32,
    /// The best score found; exact when `exact`.
    pub score: u64,
    pub moves: Vec<u32>,
    pub how: How,
    /// False when the search was stopped by a limit, or by a ceiling that is not proven (`score` is then only a
    /// lower bound).
    pub exact: bool,
    pub nodes: u64,
    pub time: Duration,
}

/// What to search with.
#[derive(Clone, Debug)]
pub struct PuncturedOptions {
    pub tactics: SearchTactics,
    pub threads: usize,
    pub time_limit: Option<Duration>,
}

impl Default for PuncturedOptions {
    fn default() -> Self {
        PuncturedOptions { tactics: SearchTactics::default(), threads: 1, time_limit: None }
    }
}

/// The best game on `[n] ∖ {hole}` under the maximal-factor rule (`opt_M`), by the frame model.
///
/// `full` is an optimal game of `[n]` and `full_score` its score: since `opt_M([n] ∖ {hole}) ≤ opt(n)`, it is the
/// search's ceiling, and its moves seed it (those still promotable).  If `full` replays on the punctured board it is
/// the answer.  `stop_at`, if given, ends the search as soon as a game scoring that much is found (a decision
/// search: "is `opt_M` at least `stop_at`?"); the result is then exact only if no such game exists.
pub fn punctured(
    n: u32,
    hole: u32,
    full: &[u32],
    full_score: u64,
    stop_at: Option<u64>,
    opts: &PuncturedOptions,
) -> Result<Punctured, String> {
    if let Some(free) = replayed(n, hole, full, full_score) {
        return Ok(free);
    }
    let start = Instant::now();
    if n == 0 {
        return Err("n must be at least 1".into());
    }
    let result = crate::with_width!(n, W => punctured_with_width::<W>(n, hole, full, full_score, stop_at, opts),
        else return Err(format!("n={n} is too large")));
    let mut result = result.map_err(|e| format!("n={n} without {hole}: {e}"))?;
    result.time = start.elapsed();
    Ok(result)
}

/// `full` as the answer on `[n] ∖ {hole}`, if it replays there and scores `full_score` (the ceiling).
fn replayed(n: u32, hole: u32, full: &[u32], full_score: u64) -> Option<Punctured> {
    let start = Instant::now();
    let score = replay_punctured(n, &[hole], full).ok().filter(|&s| s == full_score)?;
    Some(Punctured {
        hole,
        score,
        moves: full.to_vec(),
        how: How::Replay,
        exact: true,
        nodes: 0,
        time: start.elapsed(),
    })
}

/// [`punctured`] at the set width `W`: the board (tables, frames, `in_the_bag`) is a [`Board`] on the punctured
/// tables, set up for the search as the solver sets up a game, but with the ceiling and seed of the unpunctured
/// board's game instead of the previous game's.
fn punctured_with_width<const W: usize>(
    n: u32,
    hole: u32,
    full: &[u32],
    full_score: u64,
    stop_at: Option<u64>,
    opts: &PuncturedOptions,
) -> Result<Punctured, String> {
    let board = Board::<W>::new(Tables::punctured(n, &[hole]));
    let referee = |moves: &[u32]| replay_punctured(n, &[hole], moves).map_err(|e| format!("illegal game: {e}"));
    if board.is_trivial() {
        let score = referee(&board.plain)?;
        return Ok(Punctured {
            hole,
            score,
            moves: board.plain,
            how: How::Trivial,
            exact: true,
            nodes: 0,
            time: Duration::ZERO,
        });
    }

    let prepared = board.prepare(opts.tactics)?;
    // The ceiling is the unpunctured board's score (or the decision's target, if lower), not `n + opt(n − 1)`, so
    // `Board::promotion_cap` does not apply.  The seed is the game's moves that are candidates here; the hole is
    // in no frame, so it is never one.
    let ceiling = stop_at.map_or(full_score, |s| s.min(full_score));
    let cap = Some(ceiling.saturating_sub(board.in_the_bag));
    let seed = board.seed(full);
    let cfg = EngineConfig {
        threads: opts.threads,
        time_limit: opts.time_limit,
        label: format!("n={n}-{hole}"),
        ..EngineConfig::default()
    };
    let found = search::search(&prepared, &cfg, Some(&seed), cap, false).map_err(|e| format!("search failed: {e}"))?;
    let moves = solution_for_promotions::<W>(&board.tables, found.promotions);
    let score = referee(&moves)?;
    if score != board.in_the_bag + found.sum {
        return Err(format!("the game scores {score}, not {} + the promotions' {}", board.in_the_bag, found.sum));
    }
    // Reaching the unpunctured optimum is proof; reaching a lower `stop_at` proves only that much.
    let exact = found.completed && (!found.reached_cap || score >= full_score);
    Ok(Punctured { hole, score, moves, how: How::Search, exact, nodes: found.stats.nodes, time: Duration::ZERO })
}

/// The sacrifice bound of game `n`, with what it took.
#[derive(Clone, Debug)]
pub struct Bound {
    pub n: u32,
    /// `opt(n − 1)`.
    pub prev_score: u64,
    /// `opt_M(B_f)` for each maximal factor `f` of `n`, ascending in `f`.
    pub holes: Vec<Punctured>,
    /// `max(opt(n − 1), n + max_f opt_M(B_f))`; an upper bound on `opt(n)` when every hole is exact.
    pub value: u64,
}

impl Bound {
    /// The ban cost `c(f) = opt(n − 1) − opt_M(B_f)` of each maximal factor.
    pub fn costs(&self) -> Vec<(u32, u64)> {
        self.holes.iter().map(|h| (h.hole, self.prev_score - h.score)).collect()
    }

    /// Whether every punctured game was solved exactly, so that `value` is proven (within the frame model).
    pub fn exact(&self) -> bool {
        self.holes.iter().all(|h| h.exact)
    }

    /// Whether the bound proves `opt(n) ≤ target`.  (A decision that stopped early has a hole beating the target, so
    /// its value is above it.)
    pub fn proves(&self, target: u64) -> bool {
        self.exact() && self.value <= target
    }

    pub fn time(&self) -> Duration {
        self.holes.iter().map(|h| h.time).sum()
    }

    pub fn nodes(&self) -> u64 {
        self.holes.iter().map(|h| h.nodes).sum()
    }
}

/// The value of the bound from `opt(n − 1)` and the punctured optima.
pub fn bound_value(n: u32, prev_score: u64, punctured_scores: impl IntoIterator<Item = u64>) -> u64 {
    let best = punctured_scores.into_iter().max().map_or(0, |s| u64::from(n) + s);
    best.max(prev_score)
}

/// The sacrifice bound of game `n ≥ 2` from an optimal game of `n − 1` (`prev`, scoring `prev_score`).
///
/// With `target`, each punctured game only has to decide whether it reaches `target − n + 1` (it stops as soon as
/// it does): the bound then proves `opt(n) ≤ target` exactly when every hole is exact, and fails fast otherwise.
pub fn sacrifice_bound(
    n: u32,
    prev: &[u32],
    prev_score: u64,
    target: Option<u64>,
    opts: &PuncturedOptions,
) -> Result<Bound, String> {
    if n < 2 {
        return Err("the bound needs n ≥ 2".into());
    }
    let stop_at = target.map(|t| (t + 1).saturating_sub(u64::from(n)));
    let beaten = |p: &Punctured| stop_at.is_some_and(|s| p.score >= s);
    // The answers that need no search first (hole 1 by the remark on primes, and the holes the previous game
    // replays on), so that a decision can fail before any search.
    let mut holes = Vec::new();
    let mut to_search = Vec::new();
    for f in Tables::new(n).max_factors[n as usize].iter().copied() {
        let free =
            if f == 1 { Some(without_one(n - 1, prev, prev_score)?) } else { replayed(n - 1, f, prev, prev_score) };
        match free {
            Some(p) => holes.push(p),
            None => to_search.push(f),
        }
    }
    if !holes.iter().any(beaten) {
        for f in to_search {
            let p = punctured(n - 1, f, prev, prev_score, stop_at, opts)?;
            let failed = beaten(&p);
            holes.push(p);
            if failed {
                break; // this hole beats the target, so the bound cannot prove it
            }
        }
    }
    holes.sort_by_key(|h| h.hole);
    if let Some(p) = holes.iter().find(|p| p.score > prev_score) {
        return Err(format!("{} without {} scores {}: the game of {} is not optimal", n - 1, p.hole, p.score, n - 1));
    }
    let value = bound_value(n, prev_score, holes.iter().map(|h| h.score));
    Ok(Bound { n, prev_score, holes, value })
}

/// `opt_M([m] ∖ {1})` from an optimal game of `[m]` (`prev`, scoring `prev_score`), with no search: by the remark on
/// primes it is `opt(m) − P` for the largest prime `P ≤ m`, and `prev` without its first move (which is `P`)
/// reaches it.  Fails if `prev` does not start with `P`, which an optimal game always does.
fn without_one(m: u32, prev: &[u32], prev_score: u64) -> Result<Punctured, String> {
    let start = Instant::now();
    let largest = Tables::new(m).primes().last().copied();
    let rest = match (largest, prev.split_first()) {
        (None, _) => &[][..],
        (Some(p), Some((&first, rest))) if first == p => rest,
        (Some(p), _) => return Err(format!("the game of {m} does not start with its largest prime {p}")),
    };
    let score = replay_punctured(m, &[1], rest).map_err(|e| format!("the game of {m} without 1: {e}"))?;
    if score != prev_score - largest.map_or(0, u64::from) {
        return Err(format!("the game of {m} without its first move scores {score}"));
    }
    Ok(Punctured {
        hole: 1,
        score,
        moves: rest.to_vec(),
        how: How::Prime,
        exact: true,
        nodes: 0,
        time: start.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::Bits;
    use crate::brute::{self, Rule};
    use crate::frames::BoardFrames;
    use crate::game::{replay, Optimal};

    #[test]
    fn the_frames_of_a_punctured_board_leave_the_hole_out() {
        let t = Tables::punctured(30, &[6]);
        assert!(t.is_hole(6) && !t.is_hole(5) && t.holes == [6]);
        let a = BoardFrames::<1>::new(&t, Bits::EMPTY);
        let everything = a.frames().iter().fold(Bits::<1>::EMPTY, |acc, h| acc | h.sources() | h.factors());
        assert_eq!(everything | a.impossible(), Bits::from_range(1, 30).without(6));
        assert!(!a.solution().contains(&6));
        let plain = Tables::new(30);
        assert!(plain.holes.is_empty() && !plain.is_hole(6));
    }

    /// Every board `[n]` minus one number, for n up to 36 (0.3 s; up to 40 takes 1.2 s, to 44 3.4 s and to 48 8.7 s,
    /// the brute force being exponential).  The larger run that the module documentation cites (N <= 52) is
    /// `cargo run --release --example sacrifice -- validate 52`.
    #[test]
    fn small_punctured_boards_match_the_brute_force() {
        let optimal = Optimal::built_in();
        let opts = PuncturedOptions::default();
        for n in 1..=36u32 {
            let (full, full_score) = (optimal.moves(n).unwrap(), optimal.score(n).unwrap());
            for hole in 1..=n {
                let exact = brute::solve(n, &[hole], Rule::MaximalFactor).score;
                let p = punctured(n, hole, full, full_score, None, &opts).unwrap();
                assert!(p.exact, "n={n} hole={hole}");
                assert_eq!(p.score, exact, "n={n} hole={hole}: frame model {:?}", p.moves);
                assert_eq!(replay(n, &p.moves), Ok(p.score), "n={n} hole={hole}");
            }
        }
    }

    /// The two boards on which a first version of the punctured frame model went wrong.
    #[test]
    fn punctured_boards_that_once_failed() {
        let optimal = Optimal::built_in();
        let opts = PuncturedOptions::default();
        // Without 16, 8 has no maximal composite; were it a source (a move the model always plays) the model would
        // play 8 and miss 9: [17, 9, 15, 10, 18, 14, 12] scores 95.
        // Without 8, the frame of 4 is not in front of the frame of 32 (paid by 16) by maximal factors, but 32
        // sweeps 4 and 2: ordered first, it left 22 nothing to pay with.
        for (n, hole) in [(18, 16), (38, 8)] {
            let (full, full_score) = (optimal.moves(n).unwrap(), optimal.score(n).unwrap());
            let p = punctured(n, hole, full, full_score, None, &opts).unwrap();
            assert_eq!(p.score, brute::solve(n, &[hole], Rule::MaximalFactor).score, "n={n} hole={hole}");
        }
    }

    #[test]
    fn the_bound_holds_exhaustively() {
        let opt: Vec<u64> = (0..=36).map(|n| brute::solve(n, &[], Rule::Divisor).score).collect();
        for n in 2..=36u32 {
            let factors = Tables::new(n).max_factors[n as usize].clone();
            let best = factors.iter().map(|&f| brute::solve(n - 1, &[f], Rule::MaximalFactor).score);
            let bound = bound_value(n, opt[n as usize - 1], best);
            assert!(bound >= opt[n as usize], "n={n}: bound {bound} < opt {}", opt[n as usize]);
            // The first game it misses is 21 = 3·7 (σ = 1, but both bans cost nothing).
            assert_eq!(bound == opt[n as usize], n != 21, "n={n}");
        }
    }

    /// The bound is an oracle for the punctured frame model beyond the brute force's reach: it is at least the
    /// optimum for every n, and each punctured game is exact.  Up to 200 (the punctured searches of W = 8), and a
    /// sample of boards that need W = 16 (0.5 s in all).
    #[test]
    fn the_bound_holds_and_is_tight_for_primes() {
        let optimal = Optimal::built_in();
        let opts = PuncturedOptions::default();
        let mut tight = 0;
        for n in (2..=200u32).chain([500, 600, 609, 611, 660, 700]) {
            let (prev, prev_score) = (optimal.moves(n - 1).unwrap(), optimal.score(n - 1).unwrap());
            let b = sacrifice_bound(n, prev, prev_score, None, &opts).unwrap();
            let opt = optimal.score(n).unwrap();
            assert!(b.exact() && b.value >= opt, "n={n}: bound {} < opt {opt}", b.value);
            if Tables::new(n).is_prime(n) {
                assert_eq!(b.value, opt, "n={n}");
            }
            // A decision proves the optimum exactly when the bound is tight.
            let decided = sacrifice_bound(n, prev, prev_score, Some(opt), &opts).unwrap();
            assert_eq!(decided.proves(opt), b.value == opt, "n={n}");
            tight += u32::from(b.value == opt);
        }
        assert!(tight > 100, "the bound is tight at {tight} boards");
    }
}
