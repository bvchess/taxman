//! Large neighbourhood search (LNS) for a good game: keep a random share of the incumbent's promotions fixed, search the
//! rest exactly as a unit of the engine, take any better game, and repeat (`#[path = "common/lns.rs"] mod lns;` beside
//! `rng`; `wall finders` measures it and the ledger uses it, so that they share one copy).  It is not sound: it may miss
//! the optimum, and nothing it finds proves anything.  A lower bound needs only a legal game, so a caller may offer
//! what it finds as one, after the referee (`taxman::game::replay_finished`) has replayed it at its score.  The
//! measurements are in docs/experiments/finders-for-good-games-that-are-not-sound-2026-10.md.

use crate::rng::Rng;
use std::sync::Arc;
use taxman::bits::Bits;
use taxman::engine::{self, Control, EngineConfig, Start};
use taxman::search::{payload_of, set_of_payload, Prepared, SearchSpace, TaxSearcher, TaxUnit};

/// What a search of one unit came to.
pub struct Ran<const W: usize> {
    /// The best game it knows: the incumbent it started from, or better.
    pub set: Bits<W>,
    pub nodes: u64,
    /// The unit's subtree was searched to the end (or the incumbent was proven optimal).
    pub completed: bool,
}

/// The engine on one thread searching `unit` from the incumbent `start`, to the end unless `node_limit` or `control`
/// stops it.  A search of a unit finds the best game below it that beats `start`: a game of its own subtree.
pub fn run_unit<const W: usize>(
    space: &SearchSpace<'_, W>,
    control: &Arc<Control>,
    unit: TaxUnit<W>,
    start: &Bits<W>,
    node_limit: Option<u64>,
) -> Ran<W> {
    let cfg = EngineConfig { threads: 1, node_limit, control: Some(Arc::clone(control)), ..EngineConfig::default() };
    let begin = Start::Fresh { units: vec![unit], best: start.sum(), payload: payload_of(start) };
    let out = engine::run(&cfg, begin, b"finders", |_| TaxSearcher::new(space)).expect("the engine");
    Ran { set: set_of_payload(&out.payload), nodes: out.stats.nodes, completed: out.completed }
}

/// How LNS searches, with the defaults the finders' measurements chose.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// The share of the incumbent's promotions kept fixed: drawn from this range for each unit.
    pub fix: (f64, f64),
    /// The step limit of one unit (the engine counts in blocks of 1,024).
    pub unit_nodes: u64,
    /// The seed of the random choices (mixed with the board size, so that each game has its own: [`rng`](Self::rng)).
    pub seed: u64,
}

impl Default for Options {
    fn default() -> Options {
        Options { fix: (0.7, 0.9), unit_nodes: 1024, seed: 1 }
    }
}

impl Options {
    /// The generator of the board `n`.
    pub fn rng(&self, n: u32) -> Rng {
        Rng(self.seed ^ u64::from(n).wrapping_mul(0xD1B5_4A32_D192_ED03))
    }
}

/// What a search found and what it cost.
pub struct Outcome<const W: usize> {
    /// The best game it knows: the incumbent it started from, or better.
    pub set: Bits<W>,
    /// The engine's steps.
    pub nodes: u64,
    /// The units searched, the number of them searched to the end within the step limit, and the number that found a
    /// better game.
    pub units: u64,
    pub finished: u64,
    pub improved: u64,
    /// The time, on the caller's clock, at which the last better game was found (0 if none was).
    pub found: f64,
}

/// A game to search: its prepared setup, and the search space of the engine's searchers on it.
pub struct Lns<'a, const W: usize> {
    pub p: &'a Prepared<W>,
    pub space: &'a SearchSpace<'a, W>,
    pub options: Options,
}

impl<const W: usize> Lns<'_, W> {
    /// Searches from the incumbent `start` until `over` says to stop (asked before each unit).  Each unit fixes a random
    /// share of the incumbent's promotions and searches the rest exactly, with every other candidate, for at most
    /// `unit_nodes` steps; a better game becomes the incumbent.  `control` steers the units (stopping it ends the one in
    /// progress); `clock` is the caller's seconds so far, read when a better game is found.
    pub fn search(
        &self,
        start: &Bits<W>,
        rng: &mut Rng,
        control: &Arc<Control>,
        clock: &dyn Fn() -> f64,
        over: &dyn Fn() -> bool,
    ) -> Outcome<W> {
        let Options { fix, unit_nodes, .. } = self.options;
        let (mut best, mut nodes) = (*start, 0);
        let (mut units, mut finished, mut improved, mut found) = (0, 0, 0, 0.0);
        while !over() {
            let mut members = best.to_vec();
            rng.shuffle(&mut members);
            let share = fix.0 + rng.unit() * (fix.1 - fix.0);
            let fixed: Bits<W> = members[..(members.len() as f64 * share).round() as usize].iter().copied().collect();
            let unit = TaxUnit { promoted: fixed, cands: self.p.candidates.difference(&fixed) };
            let ran = run_unit(self.space, control, unit, &best, Some(unit_nodes));
            (units, nodes, finished) = (units + 1, nodes + ran.nodes, finished + u64::from(ran.completed));
            if ran.set.sum() > best.sum() {
                (best, improved, found) = (ran.set, improved + 1, clock());
            }
        }
        Outcome { set: best, nodes, units, finished, improved, found }
    }
}
