//! The sacrifice bound: an upper bound on game `n` from games on the board `n − 1` with one number taken off.
//!
//! # Status: a study, with a negative result
//!
//! **This is study code.  The solver does not use it.**  It is kept because it holds a theorem (proved below) and
//! a measured negative result, and because the tests use it as an independent check.
//!
//! * *The question.*  The tactic `reuse_prev` (since removed,
//!   docs/experiments/which-tactics-earn-their-keep-2026-10.md) accepted a derived game unproven.  Could an upper bound
//!   on `opt(n)` computed from games of `n − 1` prove those games optimal, or settle other games faster than searching
//!   them?
//! * *The answer is no.*  The bound is a theorem, and the frame model computes it correctly (checked against
//!   exhaustive search, [`crate::brute`]), but it is tight on too few of the games that matter, and computing it
//!   costs about as much as solving the game: it is no replacement for the tactic, and putting it into the
//!   pipeline slowed the games it was tried on (numbers under "Findings"; the example `sacrifice` made them; removed
//!   from master, it builds at commit 8753189).
//! * *What stays in use.*  `tests/wide.rs` uses [`sacrifice_bound`] as an upper bound that does not come from the
//!   search under test, to prove one result at `W = 20` (game 1133), beyond what the Java can check.  The support
//!   for punctured boards (holes) exists for this study and for the hunt (next bullet):
//!   [`crate::numtheory::Tables::punctured`], [`crate::game::replay_punctured`], and the hole cases of
//!   [`crate::frames`].
//! * *The hunt.*  The punctured boards also serve a search for a counterexample to a claimed optimum: [`hunt`] looks
//!   on the board `[n] ∖ {u}` for a game that scores more than the claim, for a source `u` that the prover's bounds
//!   could not show every better game must play (the example `hunt`, removed from master: it builds at commit
//!   8753189), after the solver's own start there
//!   ([`PuncturedOptions::start`]).  A game of the punctured board is a game of `[n]` with the same moves and score,
//!   since a source is nobody's factor.
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
//! [`crate::frames`]) apply to punctured boards as they do to `[n]`, and so does their proof: the frames are numbered
//! by layer, so the frame model is exact on a board with holes as on `[n]`, given the sources setup keeps
//! (heuristics/THEORY.md, Theorem M and "Boards with holes": the heaviest promotion set gives the best game, under the
//! maximal-factor rule, among those that play every source setup keeps on the punctured board).  What a bound computed
//! this way rests on is therefore the upper part of each punctured board: that some best game of `B_f` plays every
//! source setup keeps there.  That is open, as U is on `[n]` (where Theorem B settles the sources of one and two prime
//! factors; it is not proven for punctured boards, so on them the statement takes in all the kept sources), and it is
//! validated against the brute force ([`crate::brute`]) on every board `[N] ∖ {h}` with `N ≤ 52`.  Each punctured
//! game is warm-started from an optimal game of the unpunctured board, which is its ceiling: if that game replays on
//! the punctured board, its score is the answer with no search.
//!
//! # Findings (the negative result)
//!
//! Measured with the example `sacrifice` (2026-10; 2 threads; its selection of the games `reuse_prev` derived went
//! with the tactic; the example is removed from master and builds at commit 8753189):
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
use crate::bits::Bits;
use crate::board::Board;
use crate::engine::{self, Control, EngineConfig, Start};
use crate::game::replay_punctured;
use crate::matchbound::MatchGraph;
use crate::numtheory::Tables;
use crate::search::{self, payload_of, set_of_payload, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};
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
    /// The limit of the exact search ([`hunt`]'s start is not counted in it).
    pub time_limit: Option<Duration>,
    /// [`hunt`] only: whether it makes the solver's own start on the punctured board before the exact search
    /// ([`search::start_incumbent`]: the greedy set, the survivors of `seed`, the dive and the local search) and
    /// reports a game of the start that beats the claim at once.
    pub start: bool,
    /// [`hunt`] only: the moves of a game of the unpunctured board, the seed of the start ([`Board::seed`]).
    pub seed: Option<Vec<u32>>,
}

impl Default for PuncturedOptions {
    fn default() -> Self {
        PuncturedOptions { tactics: SearchTactics::default(), threads: 1, time_limit: None, start: false, seed: None }
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
    if board.is_trivial() {
        let score = referee(n, hole, &board.plain)?;
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
    let (moves, score) = game_of(&board, hole, found.promotions)?;
    // Reaching the unpunctured optimum is proof; reaching a lower `stop_at` proves only that much.
    let exact = found.completed && (!found.reached_cap || score >= full_score);
    Ok(Punctured { hole, score, moves, how: How::Search, exact, nodes: found.stats.nodes, time: Duration::ZERO })
}

/// The score of `moves` as a game of `[n] ∖ {hole}` under the maximal-factor rule, or why it is not one.
fn referee(n: u32, hole: u32, moves: &[u32]) -> Result<u64, String> {
    replay_punctured(n, &[hole], moves).map_err(|e| format!("illegal game: {e}"))
}

/// The game of the punctured `board` (its one hole `hole`) that plays `promotions`, refereed there: its moves and
/// score.  The score must be what the frame model says, `in_the_bag` plus the promotions' sum.
fn game_of<const W: usize>(board: &Board<W>, hole: u32, promotions: Bits<W>) -> Result<(Vec<u32>, u64), String> {
    let moves = solution_for_promotions::<W>(&board.tables, promotions);
    let score = referee(board.n(), hole, &moves)?;
    if score != board.in_the_bag + promotions.sum() {
        let (in_the_bag, sum) = (board.in_the_bag, promotions.sum());
        return Err(format!("the game scores {score}, not {in_the_bag} + the promotions' {sum}"));
    }
    Ok((moves, score))
}

/// What [`hunt`] concluded about the board `[n] ∖ {hole}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hunted {
    /// A game of the punctured board beating the claim, refereed there: its score and moves.
    Found { score: u64, moves: Vec<u32> },
    /// The search finished: no promotion set of the punctured board's model beats the claim.
    Nothing,
    /// The time limit stopped the search first.
    Timeout,
}

/// A hunt on one punctured board.
#[derive(Clone, Debug)]
pub struct Hunt {
    /// The number taken off the board `[n]`.
    pub hole: u32,
    /// The score to beat.
    pub claim: u64,
    /// What the punctured board's game with no promotions scores: the part of every score that no promotion changes.
    pub in_the_bag: u64,
    /// The score of the game of the start (`PuncturedOptions::start`), the best the heuristics found before the exact
    /// search; `None` without the start, or when the claim was settled without it.
    pub start_score: Option<u64>,
    /// What the hunt concluded.
    pub outcome: Hunted,
    /// The search's steps (0 when the claim was settled without one).
    pub nodes: u64,
    /// The whole call, board setup included.
    pub time: Duration,
}

/// Whether some game of `[n] ∖ {hole}` (maximal-factor rule) scores more than `claim`, by the frame model on the
/// punctured board.
///
/// For a source `hole` (a number above `n/2`: nobody's factor) a game of the punctured board is a game of `[n]` with
/// the same moves and score, so a game there that beats a recorded optimum of `n` is a counterexample to it.  The
/// search starts from a handicapped incumbent worth `claim − in_the_bag` with an empty payload
/// (the method of `tests/soundness.rs`): it looks only for promotion sets that beat the claim, and if it finds one
/// the engine's incumbent is a real set, which is turned into a game and refereed on the punctured board
/// ([`replay_punctured`]), its score checked against `in_the_bag` plus the promotions' sum.  A claim below
/// `in_the_bag` is beaten by the game with no promotions, with no search.
///
/// With `opts.start` the hunt first makes the solver's own start on the punctured board ([`search::start_incumbent`],
/// seeded with `opts.seed` if given): a game of it above the claim ends the hunt, found with no search, and whatever it
/// reaches is in [`Hunt::start_score`].  The exact search that follows has the time limit, and is the only part of
/// the hunt that can conclude "nothing".
///
/// A found game is a real game.  "Nothing" is only as complete as the frame model on the punctured board: there it is
/// validated against the brute force on small boards, not proven (see "Computing it" in the module documentation), so
/// it is evidence, not a proof.  "Timeout" says nothing.
///
/// # Errors
///
/// If `hole` is not on the board, the board is beyond the widest set, the search cannot be set up or fails, or a game
/// does not referee as the frame model says (a bug).
pub fn hunt(n: u32, hole: u32, claim: u64, opts: &PuncturedOptions) -> Result<Hunt, String> {
    let start = Instant::now();
    if !(1..=n).contains(&hole) {
        return Err(format!("n={n} without {hole}: the hole is not on the board"));
    }
    let result = crate::with_width!(n, W => hunt_with_width::<W>(n, hole, claim, opts),
        else return Err(format!("n={n} is too large")));
    let mut result = result.map_err(|e| format!("n={n} without {hole}: {e}"))?;
    result.time = start.elapsed();
    Ok(result)
}

/// [`hunt`] at the set width `W`.
fn hunt_with_width<const W: usize>(n: u32, hole: u32, claim: u64, opts: &PuncturedOptions) -> Result<Hunt, String> {
    let board = Board::<W>::new(Tables::punctured(n, &[hole]));
    let in_the_bag = board.in_the_bag;
    let hunted = |outcome: Hunted, nodes: u64, start_score: Option<u64>| Hunt {
        hole,
        claim,
        in_the_bag,
        start_score,
        outcome,
        nodes,
        time: Duration::ZERO,
    };
    if in_the_bag > claim {
        let (moves, score) = game_of(&board, hole, Bits::EMPTY)?;
        return Ok(hunted(Hunted::Found { score, moves }, 0, None));
    }
    if board.is_trivial() {
        return Ok(hunted(Hunted::Nothing, 0, None));
    }

    let prepared = board.prepare(opts.tactics)?;
    let graph = MatchGraph::new(&prepared.model, &prepared.root);
    // The solver's own start, if asked: a game of it above the claim is a game of the punctured board, found without
    // a search.  It has no ceiling (nothing is known about a game above the claim), so the local search runs to its end.
    let mut start_score = None;
    if opts.start {
        let seed = opts.seed.as_deref().map(|moves| board.seed(moves));
        let incumbent = search::start_incumbent(&prepared, &graph, &Control::new(), opts.threads, seed.as_ref());
        start_score = Some(in_the_bag + incumbent.sum());
        if in_the_bag + incumbent.sum() > claim {
            let (moves, score) = game_of(&board, hole, incumbent)?;
            return Ok(hunted(Hunted::Found { score, moves }, 0, start_score));
        }
    }
    let space = SearchSpace::new(&prepared, &graph);
    let cfg = EngineConfig {
        threads: opts.threads,
        time_limit: opts.time_limit,
        label: format!("hunt n={n}-{hole}"),
        ..EngineConfig::default()
    };
    // The incumbent is worth the target and holds no set: the search keeps it unless a promotion set beats it, and
    // then the incumbent it ends with is that set.
    let target = claim - in_the_bag;
    let root = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
    let start = Start::Fresh { units: vec![root], best: target, payload: payload_of(&Bits::<W>::EMPTY) };
    let outcome =
        engine::run(&cfg, start, b"hunt", |_| TaxSearcher::new(&space)).map_err(|e| format!("search failed: {e}"))?;
    let nodes = outcome.stats.nodes;
    if outcome.best > target {
        let promotions = set_of_payload::<W>(&outcome.payload);
        if promotions.sum() != outcome.best {
            return Err(format!("the incumbent is worth {}, its set {}", outcome.best, promotions.sum()));
        }
        let (moves, score) = game_of(&board, hole, promotions)?;
        return Ok(hunted(Hunted::Found { score, moves }, nodes, start_score));
    }
    Ok(hunted(if outcome.completed { Hunted::Nothing } else { Hunted::Timeout }, nodes, start_score))
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
    /// the brute force being exponential).  The larger run that the module documentation cites (N <= 52) was
    /// `cargo run --release --example sacrifice -- validate 52` of the example `sacrifice`, removed from master (it
    /// builds at commit 8753189).
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
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is the_bound_holds_and_is_tight_for_primes_to_100")]
    fn the_bound_holds_and_is_tight_for_primes() {
        the_bound_holds_and_is_tight_for_primes_on((2..=200u32).chain([500, 600, 609, 611, 660, 700]), 100);
    }

    /// [`the_bound_holds_and_is_tight_for_primes`] on the boards 2 to 100 and 500.
    #[test]
    fn the_bound_holds_and_is_tight_for_primes_to_100() {
        the_bound_holds_and_is_tight_for_primes_on((2..=100u32).chain([500]), 40);
    }

    /// The bound holds on the boards `ns`, and is tight at more than `fewest` of them.
    fn the_bound_holds_and_is_tight_for_primes_on(ns: impl Iterator<Item = u32>, fewest: u32) {
        let optimal = Optimal::built_in();
        let opts = PuncturedOptions::default();
        let mut tight = 0;
        for n in ns {
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
        assert!(tight > fewest, "the bound is tight at {tight} boards");
    }

    /// The game that a hunt found, checked: it beats the claim, plays no hole, and referees on the punctured board to
    /// the score the hunt says.  Returns the score and the moves.
    fn found_game(n: u32, found: &Hunt) -> (u64, &[u32]) {
        let hole = found.hole;
        let Hunted::Found { score, moves } = &found.outcome else {
            panic!("n={n} without {hole}: {:?}", found.outcome)
        };
        assert!(*score > found.claim && !moves.contains(&hole), "n={n} without {hole}: {moves:?}");
        assert_eq!(replay_punctured(n, &[hole], moves), Ok(*score), "n={n} without {hole}");
        (*score, moves)
    }

    /// Planted answers.  For each source `u` of the boards 30 to 150 (every 7th) the best game of `[n] ∖ {u}` is
    /// known exactly ([`punctured`]), worth `s`: a hunt with the claim `s − 1` must find a game (the best one, which
    /// replays on `[n]` to the same score), and a hunt with the claim `s` must find nothing.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is a_hunt_finds_the_best_game_of_some_punctured_boards_and_nothing_above_it"
    )]
    fn a_hunt_finds_the_best_game_of_a_punctured_board_and_nothing_above_it() {
        a_hunt_finds_the_best_game_of_the_punctured_boards_of(7, 500);
    }

    /// [`a_hunt_finds_the_best_game_of_a_punctured_board_and_nothing_above_it`] for the boards 30 to 150 (every 28th).
    #[test]
    fn a_hunt_finds_the_best_game_of_some_punctured_boards_and_nothing_above_it() {
        a_hunt_finds_the_best_game_of_the_punctured_boards_of(28, 100);
    }

    /// The hunts for the punctured boards of the boards 30 to 150, every `step`th; more than `fewest` pairs of a board
    /// and a source.
    fn a_hunt_finds_the_best_game_of_the_punctured_boards_of(step: usize, fewest: usize) {
        let optimal = Optimal::built_in();
        let opts = PuncturedOptions::default();
        let (mut pairs, mut searched, mut proved_by_search) = (0, 0, 0);
        for n in (30..=150u32).step_by(step) {
            let (full, full_score) = (optimal.moves(n).unwrap(), optimal.score(n).unwrap());
            let sources: Vec<u32> = Board::<8>::for_n(n).plain.into_iter().filter(|&u| 2 * u > n).collect();
            assert!(!sources.is_empty(), "n={n}");
            for u in sources {
                let best = punctured(n, u, full, full_score, None, &opts).unwrap();
                assert!(best.exact, "n={n} without {u}");
                let s = best.score;

                let beaten = hunt(n, u, s - 1, &opts).unwrap();
                assert_eq!((beaten.hole, beaten.claim), (u, s - 1));
                let (score, moves) = found_game(n, &beaten);
                assert_eq!(score, s, "n={n} without {u}: the best game scores {s}");
                assert_eq!(replay(n, moves), Ok(s), "n={n} without {u}: the game does not replay on the full board");

                let settled = hunt(n, u, s, &opts).unwrap();
                assert_eq!(settled.outcome, Hunted::Nothing, "n={n} without {u}: nothing beats {s}");

                pairs += 1;
                searched += usize::from(beaten.nodes > 0);
                proved_by_search += usize::from(settled.nodes > 0);
            }
        }
        // Most pairs take a search, to find the best game and to rule out a better one (a claim below the game with
        // no promotions, or a board with no candidates, is settled without).
        let most = pairs / 2;
        assert!(
            pairs > fewest && searched > most && proved_by_search > most,
            "{pairs} pairs: {searched}, {proved_by_search}"
        );
    }

    /// The start of a hunt (the greedy set, the seed's survivors, the dive and the local search) finds real games and
    /// no more: with it on, with a seed (the unpunctured board's game) or without, a hunt still finds the best game of a
    /// punctured board at one below its score and nothing at it, and the start's own game never scores more than the
    /// best one.  It settles most of these hunts with no search, seeded or not.  Sources of the boards 30 to 150 (every
    /// 14th), as the test above.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is a_hunt_with_the_start_agrees_with_the_hunt_without_on_some_boards"
    )]
    fn a_hunt_with_the_start_agrees_with_the_hunt_without() {
        a_hunt_with_the_start_agrees_with_the_hunt_without_on_every(14);
    }

    /// [`a_hunt_with_the_start_agrees_with_the_hunt_without`] for the boards 30 to 150 (every 42nd).
    #[test]
    fn a_hunt_with_the_start_agrees_with_the_hunt_without_on_some_boards() {
        a_hunt_with_the_start_agrees_with_the_hunt_without_on_every(42);
    }

    /// The hunts with the start and without agree on the boards 30 to 150, every `step`th.
    fn a_hunt_with_the_start_agrees_with_the_hunt_without_on_every(step: usize) {
        let optimal = Optimal::built_in();
        let (mut pairs, mut by_start, mut by_seeded_start) = (0, 0, 0);
        for n in (30..=150u32).step_by(step) {
            let (full, full_score) = (optimal.moves(n).unwrap(), optimal.score(n).unwrap());
            let sources: Vec<u32> = Board::<8>::for_n(n).plain.into_iter().filter(|&u| 2 * u > n).collect();
            for u in sources {
                let s = punctured(n, u, full, full_score, None, &PuncturedOptions::default()).unwrap().score;
                for seed in [None, Some(full.to_vec())] {
                    let seeded = seed.is_some();
                    let opts = PuncturedOptions { start: true, seed, ..PuncturedOptions::default() };
                    let beaten = hunt(n, u, s - 1, &opts).unwrap();
                    let (score, moves) = found_game(n, &beaten);
                    assert_eq!(score, s, "n={n} without {u}, seeded {seeded}: the best game scores {s}");
                    assert_eq!(
                        replay(n, moves),
                        Ok(s),
                        "n={n} without {u}: the game does not replay on the full board"
                    );
                    assert!(beaten.start_score.is_none_or(|x| x <= s), "n={n} without {u}: {:?}", beaten.start_score);
                    let by = usize::from(beaten.nodes == 0 && beaten.start_score == Some(s));
                    if seeded {
                        by_seeded_start += by;
                    } else {
                        by_start += by;
                    }

                    let settled = hunt(n, u, s, &opts).unwrap();
                    assert_eq!(settled.outcome, Hunted::Nothing, "n={n} without {u}: nothing beats {s}");
                    assert!(settled.start_score.is_none_or(|x| x <= s), "n={n} without {u}: {:?}", settled.start_score);
                }
                pairs += 1;
            }
        }
        eprintln!("{pairs} pairs: the start alone finds the best game on {by_start}, seeded on {by_seeded_start}");
        assert!(by_start > 0 && by_seeded_start > 0, "{pairs} pairs: {by_start}, {by_seeded_start}");
    }

    /// A hunt agrees with the brute force on every small punctured board, trivial ones included: nothing beats the best
    /// score, and something beats one less.
    #[test]
    fn a_hunt_agrees_with_the_brute_force_on_small_boards() {
        let opts = PuncturedOptions::default();
        for n in 2..=30u32 {
            for hole in 1..=n {
                let exact = brute::solve(n, &[hole], Rule::MaximalFactor).score;
                let settled = hunt(n, hole, exact, &opts).unwrap();
                assert_eq!(settled.outcome, Hunted::Nothing, "n={n} without {hole}: nothing beats {exact}");
                if exact > 0 {
                    let beaten = hunt(n, hole, exact - 1, &opts).unwrap();
                    assert_eq!(found_game(n, &beaten).0, exact, "n={n} without {hole}");
                }
            }
        }
    }

    /// A claim below the score of the game with no promotions is beaten by that game, with no search.
    #[test]
    fn a_claim_below_the_game_with_no_promotions_is_beaten_at_once() {
        let opts = PuncturedOptions::default();
        let (n, hole) = (100, 61);
        let plain = hunt(n, hole, 0, &opts).unwrap();
        assert_eq!((plain.nodes, found_game(n, &plain).0), (0, plain.in_the_bag));
        let just_below = hunt(n, hole, plain.in_the_bag - 1, &opts).unwrap();
        assert_eq!((just_below.nodes, found_game(n, &just_below).0), (0, plain.in_the_bag));
        // At the score itself it takes a search to tell: the board has candidates, and its best game scores more.
        let at = hunt(n, hole, plain.in_the_bag, &opts).unwrap();
        assert!(at.nodes > 0 && matches!(at.outcome, Hunted::Found { .. }), "{at:?}");
    }

    #[test]
    fn a_hunt_refuses_a_hole_that_is_not_on_the_board() {
        let opts = PuncturedOptions::default();
        assert!(hunt(30, 0, 0, &opts).is_err() && hunt(30, 31, 0, &opts).is_err() && hunt(0, 1, 0, &opts).is_err());
    }
}
