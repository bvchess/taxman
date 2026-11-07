//! The finder: large neighborhood search (LNS) for good moves, and the thread that runs it beside a long search.
//!
//! # The search for good moves
//!
//! [`Lns`] keeps a random share of an incumbent's promotions fixed, searches the rest exactly as a unit of the engine
//! ([`run_unit`]), takes any better set, and repeats.  It is not sound: it may miss optimal moves, and nothing it
//! finds proves anything.  A lower bound needs only legal moves, so a caller offers what it finds as an incumbent,
//! after the checks of [`offer_found`].  The measurements that chose its defaults are in
//! solver/docs/design/trust.md.
//!
//! # Beside a long search
//!
//! Almost every search of the chain ends within a minute of its final best.  A few start well below it and take
//! hours, and what they spend before the start's deficit is made up is most of their cost: game 4875's search began
//! 153 below its final best and took 4.5 G steps to reach it, which LNS found in 1.2 to 10.6 seconds on one thread
//! (https://github.com/bvchess/taxman-notebook/blob/main/search/the-finder-on-long-searches-and-game-4875-2026-10/README.md).
//! So a search that proves long gets a finder, and one that ends first never pays for one.
//!
//! The flag `--finder-after` is the number of steps, over every run of a search, after which the search starts one
//! finder thread ([`AFTER`] when `solve` is not told, 0 for none; `--finder-seconds` is its budget, [`BUDGET`] when
//! `solve` is not told; both are `SolveOptions`, no tactics); [`Plan`] carries the two, and the seed of the search, if
//! it has one.  The thread ([`beside`]) runs *rounds* of LNS, each with a random seed of its own (the round's number)
//! for [`ROUND`] at most: the odd rounds start from the survivors of the search's seed, if it has one, the others from
//! the search's incumbent as it is when the round starts, and adopt a better incumbent that the search finds
//! meanwhile.  A set better than the search's incumbent is offered to the search at once
//! ([`offer_found`]), through the control, the way a worker's improvement reaches the incumbent; the thread ends when
//! its budget is spent or the search ends, whichever comes first.
//!
//! The finder only offers sets.  A set it offers becomes the incumbent after the check that a checkpoint's incumbent
//! passes on resuming (a set of candidates that can be promoted from the root, worth what it claims), and the search
//! stays exhaustive: it prunes against the incumbent as it would against a worker's, its tree is searched to the end,
//! and its proof is its own.  Nothing of the finder is in the header that a checkpoint is checked against, so a
//! checkpoint resumes whatever the finder's settings are.  What it did is the [`Report`] in the result line.
//!
//! # On many threads, alone
//!
//! The unsound mode (`taxman solve --unsound`, [`crate::solver::unsound`]) runs LNS with no search beside it, on every
//! thread, and its answer is the same on any number of threads and any machine ([`Lns::rounds`]).  It goes in rounds of
//! [`NEIGHBORHOODS`] neighborhoods, each a unit of at most `unit_nodes` steps.  Every neighborhood of a round starts
//! from the incumbent as the round starts, and neighborhood k of round r draws its random choices from a generator of
//! its own, keyed by the seed, n, r and k alone ([`Options::rng_of`]), with a control of its own, so that what it finds
//! and the steps it takes are a function of the incumbent and those four.  The threads take the round's neighborhoods
//! in any order; once all are done, the best set they found becomes the incumbent, the lowest k's on a tie
//! ([`Lns::round`]).  The rounds go on until their work reaches a budget, checked after each round (the work of a
//! round is its steps and [`NEIGHBORHOOD_WORK`] for each neighborhood), so the incumbent after r rounds depends on the
//! start, n, the seed and r alone, and a run of a larger budget does the rounds of a smaller one first.  These rounds
//! are not those of the finder beside a search, which are timed.
//!
//! The rounds may keep more than one incumbent, a *trajectory* each: the unsound mode keeps the start's, and one for
//! each distinct set that survives of the moves for the games before ([`surviving`]), as the finder beside a search
//! starts its seed rounds.  With T trajectories, neighborhood k of a round works from the incumbent of trajectory
//! k mod T as the round starts, each trajectory's next incumbent is the best set of its own neighborhoods, the lowest
//! k's on a tie, if that is heavier, and the game's set is the heaviest incumbent, the first trajectory's on a tie
//! ([`Lns::rounds`]).  The draws of neighborhood k do not depend on T, and the guarantees above hold for any T.
//!
//! Two options of [`Options`] change which promotions a neighborhood frees, and neither changes what the guarantees
//! rest on: a neighborhood's draws still come from its own generator, and its freeing depends on nothing else but its
//! trajectory's incumbent and stage.
//!
//! * **`tilt`** P makes P percent of a round's neighborhoods free the small promotions more often than the large
//!   ones, and the others free them as before (the uniform draw).  Which neighborhoods tilt is a rule of their place in
//!   their trajectory alone ([`draws_tilted`]), so that each trajectory has its share whatever the trajectories are.
//!   The number a tilted neighborhood frees is the same as a uniform one's, drawn from `fix` (the share fixed); which
//!   ones is a weighted draw without replacement, the weight of a promotion by its size x/n ([`tilt_weight`]), in
//!   whole numbers so that it is the same on every machine ([`tilted_freed`]).  The small promotions are where the
//!   optimal game changes from n − 1 to n ([`TILT`]).  A tilted neighborhood takes many more steps than a uniform
//!   one, so at a small budget of work the tilted rounds are few, while the uniform draw finds the easy games in its
//!   first cheap rounds; a share of each keeps the uniform draw's quick finds and the tilt's reach.
//! * **`widen`** moves a trajectory whose incumbent has not grown heavier in its last S rounds down a ladder of shares
//!   fixed ([`LADDER`]), one stage for each S rounds without growth, and back to the first stage when it grows
//!   ([`Standing::after`]).  A trajectory's stage in a round depends only on the rounds before it.

use super::incumbent::surviving;
use super::{best_so_far, offer_found, payload_of, set_of_payload, Prepared, SearchSpace, TaxSearcher, TaxUnit};
use crate::bits::Bits;
use crate::engine::{self, in_parallel, Control, EngineConfig, Start};
use crate::rng::Rng;
use std::fmt;
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// What a search of one unit came to.
pub struct Ran<const W: usize> {
    /// The best set it knows: the incumbent it started from, or better.
    pub set: Bits<W>,
    pub nodes: u64,
    /// The unit's subtree was searched to the end (or the incumbent was proven optimal).
    pub completed: bool,
}

/// The engine on one thread searching `unit` from the incumbent `start`, to the end unless `node_limit` or `control`
/// stops it.  A search of a unit finds the best set below it that beats `start`: a set of its own subtree.
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

/// The step limit of one unit by default (the engine counts in blocks of 1,024).
pub const UNIT_STEPS: u64 = 1024;

/// The ladder of `widen`: the range of the share of the incumbent's promotions kept fixed at each stage, from the
/// first, where a trajectory begins and returns when it grows, to the last, where it stays.  A smaller share leaves a
/// neighborhood more to search, and the steps of a round grow with it (the notebook's larger neighborhoods measured the
/// last two:
/// https://github.com/bvchess/taxman-notebook/blob/main/search/larger-neighborhoods-and-a-second-base-2026-11/README.md
/// ).
pub const LADDER: [(f64, f64); 3] = [(0.7, 0.9), (0.5, 0.7), (0.4, 0.6)];

/// The stages of the ladder.
pub const STAGES: usize = LADDER.len();

/// The weights of the tilted share by the size of a promotion, in [`TILT`]`.len()` bins of width n/40 from n/4: the
/// share of the drops in a bin over the share of the seed's promotions in it, fitted on the searched games of 1001-3440
/// and written times 100 (4.87, 4.70, 3.48, 2.41, 1.53, 1.12, 0.76, 0.45, 0.28, 0.33), as whole numbers, so that a
/// draw is the same on every machine (the notebook's study of how the optimal game changes with n, section 6, tilt.py:
/// https://github.com/bvchess/taxman-notebook/blob/main/search/how-the-optimal-game-changes-with-n-2026-11/README.md).
/// A promotion below n/4 takes the first weight, and one above the last bin's start the last.
pub const TILT: [u64; 10] = [487, 470, 348, 241, 153, 112, 76, 45, 28, 33];

/// The weight of the promotion `x` of game `n` in the tilted share: [`TILT`] of the bin of x/n.  The bin is
/// floor((x/n − 1/4) · 40) = floor((40x − 10n) / n), clamped to the table, worked in whole numbers so that a size
/// on the edge of a bin has no rounding.
pub fn tilt_weight(x: u32, n: u32) -> u64 {
    let bin = (40 * u64::from(x)).saturating_sub(10 * u64::from(n)) / u64::from(n);
    TILT[bin.min(TILT.len() as u64 - 1) as usize]
}

/// Whether neighborhood `k` of a round of `trajectories` trajectories draws tilted ([`tilted_freed`]) when `percent`
/// of the round's neighborhoods do (`Options::tilt`), and uniform otherwise.  Neighborhood k is the jth of its
/// trajectory, j = k div T (its trajectory is k mod T, as in [`Lns::round`]), and it draws tilted if and only if
/// floor((j + 1) P / 100) > floor(j P / 100), P the percent.  So 0 tilts none and 100 every neighborhood, 50 the odd j
/// and 25 every fourth, from j = 3; the first m neighborhoods of a trajectory tilt floor(m P / 100) of them, so every
/// trajectory has its share to within one neighborhood, whatever T is.
pub fn draws_tilted(k: u64, trajectories: usize, percent: u32) -> bool {
    let (j, percent) = (k / trajectories as u64, u64::from(percent));
    (j + 1) * percent / 100 > j * percent / 100
}

/// How LNS searches, with the defaults the finders' measurements chose.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// The share of the incumbent's promotions kept fixed: drawn from this range for each unit.
    pub fix: (f64, f64),
    /// The step limit of one unit (the engine counts in blocks of 1,024).
    pub unit_nodes: u64,
    /// The seed of the random choices (mixed with n, so that each game has its own: [`rng`](Self::rng)).
    pub seed: u64,
    /// The percent, from 0 to 100, of a round's neighborhoods that free the small promotions more often than the large
    /// ones ([`draws_tilted`] says which, [`Lns::neighborhood`] the draw); the others draw uniformly.  The finder beside
    /// a proof search never tilts.
    pub tilt: u32,
    /// The rounds on many threads move a trajectory that has not grown heavier in this many rounds one stage down the
    /// [`LADDER`], which then replaces `fix`.  The finder beside a proof search never does.
    pub widen: Option<u64>,
}

impl Default for Options {
    fn default() -> Options {
        Options { fix: LADDER[0], unit_nodes: UNIT_STEPS, seed: 1, tilt: 0, widen: None }
    }
}

impl Options {
    /// These options for a neighborhood at `stage` of the ladder: with `widen`, the share fixed of that stage; else the
    /// options as they are.
    pub fn at_stage(&self, stage: usize) -> Options {
        match self.widen {
            Some(_) => Options { fix: LADDER[stage], ..*self },
            None => *self,
        }
    }

    /// The generator of the board `n`.
    pub fn rng(&self, n: u32) -> Rng {
        Rng(self.seed ^ u64::from(n).wrapping_mul(0xD1B5_4A32_D192_ED03))
    }

    /// The generator of neighborhood `k` of round `round` of the board `n` ([`Lns::round`]): a function of the seed,
    /// `n`, `round` and `k` alone.  Each is mixed in by a step of the generator, so that two keys that differ anywhere
    /// start far apart.
    pub fn rng_of(&self, n: u32, round: u64, k: u64) -> Rng {
        let mix = |state: u64, value: u64| Rng(state ^ value).next();
        Rng(mix(mix(mix(self.seed, u64::from(n)), round), k))
    }
}

/// What a search found and what it cost.
pub struct Outcome<const W: usize> {
    /// The best set it knows: the incumbent it started from, or better.
    pub set: Bits<W>,
    /// The engine's steps.
    pub nodes: u64,
    /// The units searched, the number of them searched to the end within the step limit, and the number that found a
    /// better set.
    pub units: u64,
    pub finished: u64,
    pub improved: u64,
    /// The time, on the caller's clock, at which the last better set was found (0 if none was).
    pub found: f64,
}

/// The neighborhoods of a round of the rounds on many threads ([`Lns::round`]), K: a constant of the unsound mode, the
/// same on every machine, since the answer depends on it.  A round keeps at most K threads busy, and the incumbent
/// moves only between rounds, so a larger K feeds more threads and a smaller one adopts a better set sooner.  64 feeds
/// every thread of a machine of up to 64 threads, and 64 of a larger one's.
pub const NEIGHBORHOODS: u64 = 64;

/// What a neighborhood costs besides its steps, in steps' worth: the work of a neighborhood is its steps plus this.
/// A neighborhood has a fixed cost (building its unit and starting the engine on it), so that rounds of small
/// neighborhoods take longer than their steps say, and a budget of steps would give an option that frees more of the
/// game far fewer rounds for the same time.  Effort 1 at n = 3000 and 5000, from the table's moves, on 4 threads,
/// with and without `tilt` and at the shares fixed 0.7,0.9 and 0.5,0.7, took on average 3.3 to 68.6 ms a neighborhood
/// at 18.5 to 1,452 steps at n = 3000, and 8.6 to 274 ms at 36 to 1,935 steps at n = 5000.  A line through the cheapest
/// and the dearest of each size gives a fixed cost of about 54 steps' worth at n = 3000 and 25 at n = 5000 (a least
/// squares line through all four, 94 and 33), and 32 steps' worth a neighborhood makes the wall time a work comes to
/// 0.046 to 0.066 ms at n = 3000 and 0.126 to 0.143 ms at n = 5000 whatever the options.
pub const NEIGHBORHOOD_WORK: u64 = 32;

/// What a round of the rounds on many threads came to ([`Lns::round`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round<const W: usize> {
    /// For each trajectory, the heaviest set its neighborhoods found, the one of the lowest k among equals: the
    /// incumbent it started from, or better.
    pub sets: Vec<Bits<W>>,
    /// The engine's steps, over its neighborhoods.
    pub nodes: u64,
    /// Its neighborhoods searched to the end within the step limit.
    pub finished: u64,
}

impl<const W: usize> Round<W> {
    /// The work of the round: its steps and [`NEIGHBORHOOD_WORK`] for each of its [`NEIGHBORHOODS`].
    pub fn work(&self) -> u64 {
        self.nodes + NEIGHBORHOODS * NEIGHBORHOOD_WORK
    }
}

/// A better set that a round of [`Lns::rounds`] found: the round (from 1; 0 is a start heavier than the first, before
/// any round), the trajectory that made it (0 is the first start's) and the stage of the ladder its neighborhoods
/// worked at in that round, the engine's steps and the work of the rounds up to and including this one, the game's new
/// promotion sum, and the time on the caller's clock when the round ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Improvement {
    pub round: u64,
    pub trajectory: usize,
    pub stage: usize,
    pub steps: u64,
    pub work: u64,
    pub sum: u64,
    pub seconds: f64,
}

/// Where a trajectory stands on the ladder of `widen`: its stage, and the rounds since its incumbent grew heavier or
/// its stage changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    pub stage: usize,
    pub stalled: u64,
}

impl Standing {
    /// The standing after a round of the trajectory, in which its incumbent grew heavier or not (`grew`), when `widen`
    /// rounds without growth move it one stage down the [`LADDER`] (it stays at the last, and counts the rounds again
    /// after each move), and growth returns it to the first.
    pub fn after(self, grew: bool, widen: u64) -> Standing {
        if grew {
            Standing::default()
        } else if self.stalled + 1 >= widen {
            Standing { stage: (self.stage + 1).min(STAGES - 1), stalled: 0 }
        } else {
            Standing { stalled: self.stalled + 1, ..self }
        }
    }
}

/// What [`Lns::rounds`] came to.
#[derive(Clone, Debug, PartialEq)]
pub struct Rounds<const W: usize> {
    /// The game's set after the last round: the heaviest incumbent of the trajectories, the first's on a tie.
    pub set: Bits<W>,
    /// The rounds done, their engine's steps, and their work (the steps and [`NEIGHBORHOOD_WORK`] for each of their
    /// neighborhoods).
    pub rounds: u64,
    pub nodes: u64,
    pub work: u64,
    /// For each trajectory, the rounds it worked at each stage of the ladder.  Without `widen` they are all at the
    /// first.
    pub stage_rounds: Vec<[u64; STAGES]>,
    /// Each round that found a better set, in order, after round 0 if a start was heavier than the first.
    pub improvements: Vec<Improvement>,
    /// A stop signal ended the rounds before their work reached the budget or `enough` said so.
    pub interrupted: bool,
}

/// A game to search: its prepared setup, and the search space of the engine's searchers on it.
pub struct Lns<'a, const W: usize> {
    pub p: &'a Prepared<W>,
    pub space: &'a SearchSpace<'a, W>,
    pub options: Options,
}

impl<const W: usize> Lns<'_, W> {
    /// The unit that searches a neighborhood of the incumbent `best`: a random share of its promotions (drawn from
    /// `options.fix`) fixed, and every other candidate free.  The promotions fixed are the first of a shuffle of the
    /// members; if the neighborhood is `tilted` (the rule is [`draws_tilted`]) the promotions freed, as many as the share
    /// leaves, are a draw that favors the small ones ([`tilted_freed`]).
    pub fn neighborhood(&self, best: &Bits<W>, rng: &mut Rng, tilted: bool) -> TaxUnit<W> {
        let fix = self.options.fix;
        let mut members = best.to_vec();
        if !tilted {
            rng.shuffle(&mut members);
        }
        let share = fix.0 + rng.unit() * (fix.1 - fix.0);
        let kept = (members.len() as f64 * share).round() as usize;
        let fixed: Bits<W> = if tilted {
            let freed: Bits<W> = tilted_freed(&members, self.p.n, members.len() - kept, rng).into_iter().collect();
            best.difference(&freed)
        } else {
            members[..kept].iter().copied().collect()
        };
        TaxUnit { promoted: fixed, cands: self.p.candidates.difference(&fixed) }
    }

    /// A neighborhood of the incumbent `best` ([`neighborhood`](Self::neighborhood), drawn with `rng`, `tilted` or not)
    /// searched from it as a unit, for at most `unit_nodes` steps, steered by `control` ([`run_unit`]).
    pub fn unit(&self, best: &Bits<W>, rng: &mut Rng, control: &Arc<Control>, tilted: bool) -> Ran<W> {
        run_unit(self.space, control, self.neighborhood(best, rng, tilted), best, Some(self.options.unit_nodes))
    }

    /// Searches from the incumbent `start` until `over` says to stop (asked before each unit).  Each unit fixes a
    /// random share of the incumbent's promotions, uniformly drawn, and searches the rest exactly, with every other
    /// candidate, for at most `unit_nodes` steps; a better set becomes the incumbent.  `control` steers the units
    /// (stopping it ends the one in progress); `clock` is the caller's seconds so far, read when a better set is found.
    pub fn search(
        &self,
        start: &Bits<W>,
        rng: &mut Rng,
        control: &Arc<Control>,
        clock: &dyn Fn() -> f64,
        over: &dyn Fn() -> bool,
    ) -> Outcome<W> {
        let unit_nodes = self.options.unit_nodes;
        let run = |unit, best: &Bits<W>| run_unit(self.space, control, unit, best, Some(unit_nodes));
        self.search_with(start, rng, clock, over, run)
    }

    /// [`search`](Self::search) with `run` searching each unit from the incumbent it is given (as [`run_unit`] does
    /// within the step limit, `options.unit_nodes`), for a caller that wants what the searchers learned.
    pub fn search_with(
        &self,
        start: &Bits<W>,
        rng: &mut Rng,
        clock: &dyn Fn() -> f64,
        over: &dyn Fn() -> bool,
        mut run: impl FnMut(TaxUnit<W>, &Bits<W>) -> Ran<W>,
    ) -> Outcome<W> {
        let (mut best, mut nodes) = (*start, 0);
        let (mut units, mut finished, mut improved, mut found) = (0, 0, 0, 0.0);
        while !over() {
            let ran = run(self.neighborhood(&best, rng, false), &best);
            (units, nodes, finished) = (units + 1, nodes + ran.nodes, finished + u64::from(ran.completed));
            if ran.set.sum() > best.sum() {
                (best, improved, found) = (ran.set, improved + 1, clock());
            }
        }
        Outcome { set: best, nodes, units, finished, improved, found }
    }

    /// Round `round` (counted from 1) of the rounds on many threads (see the module documentation, "On many threads,
    /// alone"), from the incumbents `bases`, at least one, on `threads` threads: the [`NEIGHBORHOODS`] neighborhoods,
    /// the kth of the incumbent `bases[k % bases.len()]`, at the stage of the ladder `stages[k % bases.len()]`
    /// ([`Options::at_stage`]), drawn from [`Options::rng_of`], tilted or not as [`draws_tilted`] says of
    /// `options.tilt`, and searched as a unit ([`unit`](Self::unit)) with a control of its own, since a control passes
    /// the incumbent of one run on to the next.  The answer is the same on any number of threads.
    pub fn round(&self, bases: &[Bits<W>], stages: &[usize], round: u64, threads: usize) -> Round<W> {
        debug_assert_eq!(bases.len(), stages.len(), "a stage for each trajectory");
        let n = self.p.n;
        let ran = in_parallel(NEIGHBORHOODS as usize, threads, |k| {
            let trajectory = k % bases.len();
            let at = Lns { options: self.options.at_stage(stages[trajectory]), ..*self };
            let tilted = draws_tilted(k as u64, bases.len(), self.options.tilt);
            at.unit(&bases[trajectory], &mut self.options.rng_of(n, round, k as u64), &Control::new(), tilted)
        });
        let mut out = Round { sets: bases.to_vec(), nodes: 0, finished: 0 };
        for (k, ran) in ran.into_iter().enumerate() {
            (out.nodes, out.finished) = (out.nodes + ran.nodes, out.finished + u64::from(ran.completed));
            // The heaviest of a trajectory's, the first of equals: a later set replaces it only if it is strictly
            // heavier.
            let set = &mut out.sets[k % bases.len()];
            if ran.set.sum() > set.sum() {
                *set = ran.set;
            }
        }
        out
    }

    /// Rounds 1, 2, ... of the rounds on many threads ([`round`](Self::round)), from the incumbents `starts`, at least
    /// one, a trajectory each, on `threads` threads: until their work reaches `budget` (the engine's steps and
    /// [`NEIGHBORHOOD_WORK`] for each neighborhood, checked after each round, so the round that reaches it is the
    /// last, and a budget of 0 allows none), and none once `enough` says that the game's promotion sum is enough (asked
    /// of the starts, and after each round).  A round's work is at least [`NEIGHBORHOODS`] times
    /// [`NEIGHBORHOOD_WORK`], so the rounds end.  A trajectory's incumbent becomes the best set of its own
    /// neighborhoods if that is heavier.  With `options.widen`, a trajectory's stage moves down the ladder and back as
    /// its [`Standing`] says, from the rounds before.  The game's set is the heaviest incumbent, the first's on a
    /// tie, and a round that makes it heavier than it was is an [`Improvement`], whose time is the caller's `clock`.
    /// A start that is heavier than the first one makes the game's set before any round: an improvement of round 0.
    /// A stop signal ([`engine::stop_signal`]) ends the rounds before the next, or in the middle of one, whose units
    /// it stops: then the rounds are `interrupted`, and the round it stopped is not counted.
    pub fn rounds(
        &self,
        starts: &[Bits<W>],
        budget: u64,
        threads: usize,
        clock: &dyn Fn() -> f64,
        enough: &dyn Fn(u64) -> bool,
    ) -> Rounds<W> {
        let mut incumbents = starts.to_vec();
        let (trajectory, set) = heaviest(&incumbents);
        let mut out = Rounds {
            set,
            rounds: 0,
            nodes: 0,
            work: 0,
            stage_rounds: vec![[0; STAGES]; starts.len()],
            improvements: Vec::new(),
            interrupted: false,
        };
        if set.sum() > starts[0].sum() {
            let sum = set.sum();
            let none = Improvement { round: 0, trajectory, stage: 0, steps: 0, work: 0, sum, seconds: clock() };
            out.improvements.push(none);
        }
        let mut standing = vec![Standing::default(); starts.len()];
        let signaled = || engine::stop_signal().is_some();
        while out.work < budget && !enough(out.set.sum()) {
            if signaled() {
                out.interrupted = true;
                break;
            }
            let round = out.rounds + 1;
            let stages: Vec<usize> = standing.iter().map(|standing| standing.stage).collect();
            let ran = self.round(&incumbents, &stages, round, threads);
            // A signal in the middle of the round stopped some of its units.
            if signaled() {
                out.interrupted = true;
                break;
            }
            (out.rounds, out.nodes, out.work) = (round, out.nodes + ran.nodes, out.work + ran.work());
            for (t, &stage) in stages.iter().enumerate() {
                out.stage_rounds[t][stage] += 1;
                if let Some(widen) = self.options.widen {
                    standing[t] = standing[t].after(ran.sets[t].sum() > incumbents[t].sum(), widen);
                }
            }
            incumbents = ran.sets;
            let (trajectory, set) = heaviest(&incumbents);
            if set.sum() > out.set.sum() {
                let (stage, steps, work, sum) = (stages[trajectory], out.nodes, out.work, set.sum());
                out.improvements.push(Improvement { round, trajectory, stage, steps, work, sum, seconds: clock() });
            }
            out.set = set;
        }
        out
    }
}

/// The `count` promotions that the tilted share frees among `members`, the promotions of game `n` in ascending order:
/// a weighted draw without replacement, one member at a time.  Each draw is a number from `rng` below the total weight
/// ([`tilt_weight`]) of the members left, and takes the member whose cumulative weight, over the members in order,
/// first passes it.  It is the distribution of Efraimidis and Spirakis' keys u^(1/w), worked in whole numbers, so that
/// the draw is the same on every machine.
fn tilted_freed(members: &[u32], n: u32, count: usize, rng: &mut Rng) -> Vec<u32> {
    let mut left: Vec<(u32, u64)> = members.iter().map(|&x| (x, tilt_weight(x, n))).collect();
    let mut total: u64 = left.iter().map(|&(_, weight)| weight).sum();
    let mut freed = Vec::with_capacity(count);
    for _ in 0..count {
        let mut draw = rng.below(total as usize) as u64;
        let at = left.iter().position(|&(_, weight)| match draw.checked_sub(weight) {
            Some(rest) => {
                draw = rest;
                false
            }
            None => true,
        });
        let (x, weight) = left.remove(at.expect("a draw below the total weight of the members left"));
        total -= weight;
        freed.push(x);
    }
    freed
}

/// The heaviest of `sets`, at least one, and its index: the first of equals.
fn heaviest<const W: usize>(sets: &[Bits<W>]) -> (usize, Bits<W>) {
    let mut best = (0, sets[0]);
    for (i, set) in sets.iter().enumerate() {
        if set.sum() > best.1.sum() {
            best = (i, *set);
        }
    }
    best
}

// ---------------------------------------------------------------------------------------------------------
// Beside a long search

/// How long a round of LNS lasts at most: [`Plan::seconds`] is spent in rounds, each from a start of its own with a
/// random seed of its own.  Game 4875's finder reached its final best in 1.2 to 10.6 seconds from the seed's start
/// (https://github.com/bvchess/taxman-notebook/blob/main/search/the-finder-on-long-searches-and-game-4875-2026-10/README.md).
pub const ROUND: Duration = Duration::from_secs(15);

/// The steps after which `solve` starts a finder beside a search when `--finder-after` does not say: a few seconds of
/// a search on many threads, so that a search far below its final best gets one early.
pub const AFTER: u64 = 1_000_000;

/// How long a finder runs, from the moment it starts, when `--finder-seconds` does not say: the chain's re-run gave
/// each 900 seconds.
pub const BUDGET: Duration = Duration::from_secs(900);

/// How often the thread looks at the search's steps while it waits to start, and for the search's end.
const POLL: Duration = Duration::from_millis(20);

/// When and how a search runs its finder (the flags `--finder-after` and `--finder-seconds`).
#[derive(Clone, Debug)]
pub struct Plan<const W: usize> {
    /// The finder starts when the search has taken this many steps, over every run of it (a resumed search counts the
    /// runs before the resume).
    pub after: u64,
    /// The thread's budget, from the moment it starts.
    pub budget: Duration,
    /// The seed the search starts from, if it has one (the moves for `n − 1`, say): the finder's odd rounds start from
    /// what survives of it (its candidates that can still be promoted, from the largest down, as a search seeded with
    /// it starts).
    pub seed: Option<Bits<W>>,
}

/// What a finder did: the record that goes in the result line (`engine.finder`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report {
    /// The search's steps, over every run of it, when the finder started.
    pub at_nodes: u64,
    /// The incumbent's promotion sum then.
    pub from: u64,
    /// The best promotion sum the finder reached: `from` if it found nothing better.
    pub best: u64,
    /// How many of its sets became the search's incumbent.
    pub improved: u64,
    /// How many of its sets [`offer_found`] refused (a set that cannot be promoted, or is not worth what it claims:
    /// none, unless there is a bug).
    pub refused: u64,
    /// How long the thread ran, in seconds of the clock.
    pub seconds: f64,
    /// The seconds from its start at which it reached `best` (0 if it never passed `from`).
    pub found: f64,
}

/// The set that round `round` (counted from 1) starts from if it is a seed round: on the odd rounds, what survives of
/// the seed.  None on the even rounds, which start from the search's incumbent, and when there is no seed.
fn seed_start<const W: usize>(survivors: Option<Bits<W>>, round: u64) -> Option<Bits<W>> {
    survivors.filter(|_| !round.is_multiple_of(2))
}

/// Whether the search has ended (the sender of `ended` is gone, or has sent).
fn has_ended(ended: &Receiver<()>) -> bool {
    !matches!(ended.try_recv(), Err(TryRecvError::Empty))
}

/// Says something on stderr, if the search shows its progress (`label` is its progress lines'): ignoring a stderr that
/// cannot be written to, as the engine does.
fn say(label: Option<&str>, what: fmt::Arguments<'_>) {
    if let Some(label) = label {
        let _ = writeln!(io::stderr(), "{label} finder: {what}");
    }
}

/// The finder of `plan`, beside the search of `prepared` that `control` steers: waits until the search has taken
/// `plan.after` steps (`before` of them in the runs before this one), then runs rounds of LNS ([`rounds`]) until the
/// budget is spent or `ended` (its sender gone) says the search is over.  The finder of a resumed search whose runs
/// before this one took the steps starts at once, before the engine runs; any other looks at the steps the engine
/// reports every [`POLL`].  Returns what it did, or `None` if the search ended before the finder saw its steps, as one
/// that ends within a poll of reaching them may.
pub(super) fn beside<const W: usize>(
    prepared: &Prepared<W>,
    control: &Control,
    plan: &Plan<W>,
    before: u64,
    ended: &Receiver<()>,
    label: Option<&str>,
) -> Option<Report> {
    // The engine reports its steps once the run is attached to the control, and in blocks of 1,024 per worker.
    let at_nodes = loop {
        let nodes = before + control.status().map_or(0, |status| status.nodes);
        if nodes >= plan.after {
            break nodes;
        }
        if !matches!(ended.recv_timeout(POLL), Err(RecvTimeoutError::Timeout)) {
            return None;
        }
    };
    Some(rounds(prepared, control, plan, at_nodes, ended, label))
}

/// The rounds of LNS that [`beside`] runs from the moment the search has taken `at_nodes` steps (see the module
/// documentation).  They end when the budget is spent, `ended` says the search is over, or a set is refused.  Also the
/// finder alone: a `control` that holds the incumbent, and a channel whose sender the caller keeps.
fn rounds<const W: usize>(
    prepared: &Prepared<W>,
    control: &Control,
    plan: &Plan<W>,
    at_nodes: u64,
    ended: &Receiver<()>,
    label: Option<&str>,
) -> Report {
    let began = Instant::now();
    let deadline = began + plan.budget;
    let incumbent = || best_so_far::<W>(control).unwrap_or(prepared.greedy);
    let from = incumbent().sum();
    let mut report = Report { at_nodes, from, best: from, improved: 0, refused: 0, seconds: 0.0, found: 0.0 };
    say(label, format_args!("started at {at_nodes} nodes, from {from}, for {:.0} s", plan.budget.as_secs_f64()));

    // The finder's own space: the searches of its units must not touch the search's counters and records.
    let space = SearchSpace::new(prepared);
    let lns = Lns { p: prepared, space: &space, options: Options::default() };
    let units = Control::new();
    let survivors = plan.seed.as_ref().map(|seed| surviving(prepared, seed));
    let over = |until: Instant| has_ended(ended) || Instant::now() >= until;
    // Offers a set to the search; false if it was refused, which ends the finder.
    let mut offer = |set: &Bits<W>, round: u64, from_seed: bool| -> bool {
        if set.sum() > report.best {
            (report.best, report.found) = (set.sum(), began.elapsed().as_secs_f64());
        }
        match offer_found(prepared, control, set, set.sum()) {
            Ok(accepted) => {
                if accepted {
                    report.improved += 1;
                    let (value, seconds) = (set.sum(), began.elapsed().as_secs_f64());
                    let start = if from_seed { "the seed" } else { "the incumbent" };
                    say(label, format_args!("{value} after {seconds:.1} s (round {round}, from {start})"));
                }
                true
            }
            Err(why) => {
                report.refused += 1;
                say(label, format_args!("a set was refused ({why}); stopping"));
                false
            }
        }
    };
    let mut round = 0u64;
    'rounds: while !over(deadline) {
        round += 1;
        let seed_start = seed_start(survivors, round);
        let from_seed = seed_start.is_some();
        let mut best = seed_start.unwrap_or_else(incumbent);
        // What survives of the seed is a set in itself, which the search may not have.
        if from_seed && !offer(&best, round, true) {
            break;
        }
        let mut rng = Options { seed: round, ..lns.options }.rng(prepared.n);
        let until = (Instant::now() + ROUND).min(deadline);
        while !over(until) {
            // A round from the incumbent goes on from a better one that the search finds meanwhile; a round from the
            // seed is another start, which keeps its own.
            if !from_seed {
                let theirs = incumbent();
                if theirs.sum() > best.sum() {
                    best = theirs;
                }
            }
            let ran = lns.unit(&best, &mut rng, &units, false);
            if ran.set.sum() <= best.sum() {
                continue;
            }
            best = ran.set;
            if !offer(&best, round, from_seed) {
                break 'rounds;
            }
        }
    }
    report.seconds = began.elapsed().as_secs_f64();
    say(
        label,
        format_args!("ended after {:.1} s: best {}, {} sets accepted", report.seconds, report.best, report.improved),
    );
    report
}

/// Runs `search`, the engine's run, with the finder of `plan` beside it if there is a plan: a thread of its own, which
/// [`beside`] runs and which ends with the search.  Returns what `search` returns and what the finder did, if it started.
pub(super) fn run_beside<const W: usize, T>(
    prepared: &Prepared<W>,
    control: &Control,
    plan: Option<&Plan<W>>,
    before: u64,
    label: Option<&str>,
    search: impl FnOnce() -> T,
) -> (T, Option<Report>) {
    let Some(plan) = plan else { return (search(), None) };
    let (done, ended) = mpsc::channel::<()>();
    thread::scope(|scope| {
        let finder = scope.spawn(move || beside(prepared, control, plan, before, &ended, label));
        let found = search();
        // The sender's end tells the thread that the search is over.
        drop(done);
        (found, finder.join().expect("the finder's thread"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::game::Optimal;
    use crate::numtheory::Tables;
    use crate::search::{search, search_beside, SearchTactics};
    use std::cell::Cell;

    const W: usize = 16;

    fn board(n: u32) -> Board<W> {
        Board::new(Tables::new(n))
    }

    /// The best promotion sum of game `n`, from the table.
    fn known(board: &Board<W>) -> u64 {
        Optimal::built_in().score(board.n()).expect("a known entry") - board.in_the_bag
    }

    #[test]
    fn lns_reaches_the_best_known_game_on_small_games() {
        for n in [200, 300, 400, 600, 700, 900] {
            let board = board(n);
            let p = board.prepare(SearchTactics::default()).unwrap();
            let space = SearchSpace::new(&p);
            let lns = Lns { p: &p, space: &space, options: Options::default() };
            let units = Cell::new(0u64);
            let over = || {
                units.set(units.get() + 1);
                units.get() > 400
            };
            let out = lns.search(&p.greedy, &mut lns.options.rng(n), &Control::new(), &|| 0.0, &over);
            assert!(p.can_promote(&out.set), "n={n}: {:?} cannot be promoted", out.set);
            assert_eq!((out.set.sum(), out.units), (known(&board), 400), "n={n}: from {}", p.greedy.sum());
        }
    }

    #[test]
    fn lns_is_the_same_every_time() {
        let board = board(800);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let space = SearchSpace::new(&p);
        let lns = Lns { p: &p, space: &space, options: Options::default() };
        let run = || {
            let units = Cell::new(0u64);
            let over = || {
                units.set(units.get() + 1);
                units.get() > 100
            };
            let out = lns.search(&p.greedy, &mut lns.options.rng(800), &Control::new(), &|| 0.0, &over);
            (out.set, out.nodes, out.improved)
        };
        assert_eq!(run(), run());
        let other = Options { seed: 2, ..lns.options };
        assert_ne!(lns.options.rng(800).next(), other.rng(800).next());
        assert_ne!(lns.options.rng(800).next(), lns.options.rng(801).next());
    }

    /// The rounds on many threads on the prepared game `p` from the sets `starts`, a trajectory each, with `options`:
    /// until their steps reach `budget`, on `threads` threads, ending once the game's set reaches `target` if there is
    /// one.
    fn rounds_with(
        p: &Prepared<W>,
        options: Options,
        starts: &[Bits<W>],
        budget: u64,
        threads: usize,
        target: Option<u64>,
    ) -> Rounds<W> {
        let space = SearchSpace::new(p);
        let lns = Lns { p, space: &space, options };
        lns.rounds(starts, budget, threads, &|| 0.0, &|sum| target.is_some_and(|target| sum >= target))
    }

    /// [`rounds_with`] the default options.
    fn rounds_from(p: &Prepared<W>, starts: &[Bits<W>], budget: u64, threads: usize, target: Option<u64>) -> Rounds<W> {
        rounds_with(p, Options::default(), starts, budget, threads, target)
    }

    /// [`rounds_from`] the greedy set of `p`, one trajectory.
    fn rounds_of(p: &Prepared<W>, budget: u64, threads: usize, target: Option<u64>) -> Rounds<W> {
        rounds_from(p, &[p.greedy], budget, threads, target)
    }

    /// The game of the tests of the rounds on many threads, whose greedy set its rounds 1 and 2 improve.
    const ROUNDS_GAME: u32 = 400;

    /// A budget of work that every round reaches, so that the rounds it allows are one.
    const ONE_ROUND: u64 = 1;

    /// Whether the jth neighborhood of a trajectory draws tilted: the places of a trajectory that a tilt names.
    type Places = fn(u64) -> bool;

    #[test]
    fn the_rounds_on_many_threads_are_the_same_on_any_number_of_threads() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        // The work of the first round and one more: the budget of the first two rounds.
        let two = rounds_of(&p, ONE_ROUND, 1, None).work + 1;
        let one = rounds_of(&p, two, 1, None);
        let rounds: Vec<u64> = one.improvements.iter().map(|improvement| improvement.round).collect();
        assert_eq!(rounds, [1, 2], "each round improves the greedy set: {:?}", one.improvements);
        assert!(p.can_promote(&one.set) && one.set.sum() == one.improvements[1].sum);
        assert_eq!((one.rounds, one.interrupted), (2, false));
        assert_eq!(one.stage_rounds, [[2, 0, 0]], "without widen a trajectory stays at the first stage");
        for threads in [2, 4] {
            assert_eq!(rounds_of(&p, two, threads, None), one, "{threads} threads");
        }
    }

    #[test]
    fn the_rounds_run_until_their_work_reaches_the_budget_and_the_round_that_reaches_it_is_the_last() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let first = rounds_of(&p, ONE_ROUND, 4, None);
        assert_eq!(first.rounds, 1);
        // A round's work is its steps and the work of each neighborhood, and every neighborhood takes a step at least.
        let overhead = NEIGHBORHOODS * NEIGHBORHOOD_WORK;
        assert_eq!(first.work, first.nodes + overhead);
        assert!(first.nodes >= NEIGHBORHOODS, "every neighborhood takes a step at least: {}", first.nodes);
        // A budget the first round reaches exactly ends there, and one more takes a second round.
        assert_eq!(rounds_of(&p, first.work, 4, None), first);
        let more = rounds_of(&p, first.work + 1, 4, None);
        assert!(more.rounds >= 2 && more.work > first.work, "{} rounds, {} work", more.rounds, more.work);
        assert_eq!(more.work, more.nodes + more.rounds * overhead);
        // The rounds stop at the first round whose total reaches the budget: one less of budget than the total is
        // reached by the same rounds, since the round before fell short of it.
        assert_eq!(rounds_of(&p, more.work - 1, 4, None), more);
        // Each improvement records the steps and the work of the rounds up to its own, which grow with the rounds.
        let steps: Vec<u64> = more.improvements.iter().map(|improvement| improvement.steps).collect();
        assert!(steps.windows(2).all(|pair| pair[0] < pair[1]) && steps.iter().all(|&s| (1..=more.nodes).contains(&s)));
        for improvement in &more.improvements {
            assert_eq!(improvement.work, improvement.steps + improvement.round * overhead, "{improvement:?}");
        }
        assert_eq!(more.improvements[0].steps, first.nodes, "the first round improves the greedy set");
    }

    #[test]
    fn more_work_does_the_rounds_of_less_first_and_stops_once_it_is_enough() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let short = rounds_of(&p, ONE_ROUND, 4, None);
        let long = rounds_of(&p, short.work + 1, 4, None);
        assert!(
            long.improvements.starts_with(&short.improvements),
            "{:?}, {:?}",
            short.improvements,
            long.improvements
        );
        assert!(long.set.sum() > short.set.sum() && !short.improvements.is_empty());
        // Enough at the first improvement: the rounds end with it.
        let first = long.improvements[0];
        let stopped = rounds_of(&p, long.work, 4, Some(first.sum));
        assert_eq!((stopped.rounds, stopped.set.sum(), stopped.improvements), (first.round, first.sum, vec![first]));
        // A start that is enough runs no round, and neither does no budget.
        for none in [rounds_of(&p, long.work, 4, Some(0)), rounds_of(&p, 0, 4, None)] {
            assert_eq!((none.set, none.rounds, none.nodes, none.work), (p.greedy, 0, 0, 0));
            assert!(none.improvements.is_empty());
        }
    }

    #[test]
    fn the_heaviest_set_is_the_first_of_equals() {
        let set = |numbers: &[u32]| numbers.iter().copied().collect::<Bits<W>>();
        let (small, same, large) = (set(&[1, 5]), set(&[2, 4]), set(&[7]));
        assert_eq!((small.sum(), same.sum(), large.sum()), (6, 6, 7));
        assert_eq!(heaviest(&[small, same]), (0, small));
        assert_eq!(heaviest(&[same, small]), (0, same));
        assert_eq!(heaviest(&[small, large, same, large]), (1, large));
    }

    #[test]
    fn neighborhood_k_of_a_round_works_from_the_incumbent_of_trajectory_k_mod_t() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let space = SearchSpace::new(&p);
        let lns = Lns { p: &p, space: &space, options: Options::default() };
        let poorer = p.greedy.without(p.greedy.max().unwrap());
        // With widening, each trajectory's neighborhoods work at the stage of their trajectory; with a tilt of 50, the
        // odd j of each trajectory draw tilted, and with 100 every one does.
        let widened = Lns { options: Options { widen: Some(1), tilt: 50, ..lns.options }, ..lns };
        let every = Lns { options: Options { tilt: 100, ..lns.options }, ..lns };
        let cases: [(&Lns<'_, W>, [usize; 3], Places); 3] =
            [(&lns, [0, 0, 0], |_| false), (&widened, [0, 2, 1], |j| j % 2 == 1), (&every, [0, 0, 0], |_| true)];
        for (lns, stages, tilts) in cases {
            for bases in [vec![p.greedy], vec![p.greedy, poorer], vec![poorer, p.greedy, poorer]] {
                let stages = &stages[..bases.len()];
                let ran = lns.round(&bases, stages, 3, 4);
                // Each trajectory's best set, the lowest k's among equals, from the neighborhoods that k mod T names.
                let mut expected = bases.clone();
                for k in 0..NEIGHBORHOODS {
                    let t = k as usize % bases.len();
                    let at = Lns { options: lns.options.at_stage(stages[t]), ..*lns };
                    let tilted = tilts(k / bases.len() as u64);
                    let unit = at.unit(&bases[t], &mut lns.options.rng_of(p.n, 3, k), &Control::new(), tilted);
                    if unit.set.sum() > expected[t].sum() {
                        expected[t] = unit.set;
                    }
                }
                assert_eq!(ran.sets, expected, "{} trajectories, stages {stages:?}", bases.len());
            }
        }
    }

    #[test]
    fn trajectories_are_the_same_on_any_number_of_threads_and_the_game_follows_the_heaviest() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let space = SearchSpace::new(&p);
        let lns = Lns { p: &p, space: &space, options: Options::default() };
        // The greedy set with half its promotions, the other trajectory's start, in either order.
        let half = p.greedy.to_vec();
        let poor: Bits<W> = half[..half.len() / 2].iter().copied().collect();
        assert!(p.can_promote(&poor) && poor.sum() < p.greedy.sum());
        for starts in [[p.greedy, poor], [poor, p.greedy]] {
            let short = rounds_from(&p, &starts, ONE_ROUND, 4, None);
            let two = short.work + 1;
            let one = rounds_from(&p, &starts, two, 1, None);
            assert!(p.can_promote(&one.set) && one.set.sum() == one.improvements.last().unwrap().sum);
            assert_eq!(rounds_from(&p, &starts, two, 4, None), one, "4 threads");
            assert!(one.improvements.starts_with(&short.improvements), "{:?}", one.improvements);
            // A start heavier than the first raises the game's set before any round, as an improvement of round 0.
            // The improvement of a round names the trajectory of the heaviest incumbent after it, and the round's
            // steps and work.
            let (first, best) = (starts[0].sum(), heaviest(&starts));
            let mut expected = Vec::new();
            if best.1.sum() > first {
                let sum = best.1.sum();
                expected.push(Improvement {
                    round: 0,
                    trajectory: best.0,
                    stage: 0,
                    steps: 0,
                    work: 0,
                    sum,
                    seconds: 0.0,
                });
            }
            let ran = lns.round(&starts, &[0, 0], 1, 1);
            let (trajectory, set) = heaviest(&ran.sets);
            assert!(set.sum() > best.1.sum(), "the first round raises the heaviest start");
            let (steps, work, sum) = (ran.nodes, ran.work(), set.sum());
            expected.push(Improvement { round: 1, trajectory, stage: 0, steps, work, sum, seconds: 0.0 });
            assert_eq!(short.improvements, expected, "starts {starts:?}");
            assert_eq!(
                expected.len() == 2,
                starts[1].sum() > first,
                "only the second order begins with a heavier start"
            );
        }
        // Two equal starts tie, and the first trajectory's set is the game's.
        let tied = rounds_from(&p, &[p.greedy, p.greedy], 0, 1, None);
        assert_eq!((tied.set, tied.rounds), (p.greedy, 0));
    }

    #[test]
    fn each_key_of_a_neighborhood_draws_its_own_numbers() {
        let options = Options::default();
        let first = |options: Options, n, round, k| options.rng_of(n, round, k).next();
        let one = first(options, 800, 1, 0);
        assert_eq!(one, first(options, 800, 1, 0));
        let others = [
            first(Options { seed: 2, ..options }, 800, 1, 0),
            first(options, 801, 1, 0),
            first(options, 800, 2, 0),
            first(options, 800, 1, 1),
        ];
        assert!(others.iter().all(|&other| other != one), "{one} {others:?}");
    }

    /// A neighborhood with no option fixes the promotions that the first form of the mode (commit fa05cac) fixed: the
    /// first of a shuffle of the incumbent's, as many as a share drawn from `fix` says.  The draws are of the greedy
    /// set of game 400, 21 promotions, of which 16 are fixed each time.
    #[test]
    fn a_neighborhood_with_no_option_fixes_what_it_fixed_before_the_options() {
        let options = Options::default();
        assert!(options.tilt == 0 && options.widen.is_none(), "the finder beside a search draws as it did");
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let space = SearchSpace::new(&p);
        let lns = Lns { p: &p, space: &space, options };
        assert_eq!((p.greedy.len(), p.greedy.sum()), (21, 3575));
        let pinned: [(u64, u64, u64, [u32; 6]); 4] = [
            (1, 0, 2641, [75, 98, 150, 153, 156, 165]),
            (1, 1, 2730, [75, 98, 153, 165, 171, 176]),
            (2, 5, 2660, [75, 98, 150, 153, 156, 168]),
            (7, 63, 2763, [98, 150, 153, 156, 165, 171]),
        ];
        for (round, k, sum, first) in pinned {
            let unit = lns.neighborhood(&p.greedy, &mut options.rng_of(p.n, round, k), false);
            let fixed = unit.promoted.to_vec();
            assert_eq!(
                (fixed.len(), unit.promoted.sum(), fixed[..6].to_vec()),
                (16, sum, first.to_vec()),
                "{round} {k}"
            );
            assert_eq!(unit.cands, p.candidates.difference(&unit.promoted));
        }
    }

    #[test]
    fn the_tilt_weight_is_the_table_of_the_bin_of_the_size() {
        // x/n from 0.25 in bins of 0.025: the first bin takes everything below, the last everything above.
        for (x, bin) in
            [(0, 0), (99, 0), (100, 0), (109, 0), (110, 1), (119, 1), (120, 2), (189, 8), (190, 9), (399, 9), (400, 9)]
        {
            assert_eq!(tilt_weight(x, 400), TILT[bin], "x={x}");
        }
        // A size on the edge of a bin takes the bin that starts there, with no rounding: 11/40 = 0.275.
        assert_eq!((tilt_weight(10, 40), tilt_weight(11, 40), tilt_weight(20, 40)), (TILT[0], TILT[1], TILT[9]));
    }

    /// The neighborhoods k of a round of `trajectories` trajectories that draw tilted at `percent`.
    fn tilted_ks(trajectories: usize, percent: u32) -> Vec<u64> {
        (0..NEIGHBORHOODS).filter(|&k| draws_tilted(k, trajectories, percent)).collect()
    }

    /// The jth neighborhood of every trajectory draws tilted for the j that the rule names, whatever the trajectories
    /// are, and each trajectory has its percent of tilted neighborhoods to within one.
    #[test]
    fn the_neighborhoods_that_draw_tilted_are_the_same_places_of_every_trajectory() {
        // The places j of a trajectory that tilt, for the percents the rule names plainly: none, every fourth from the
        // third, the odd ones, and all.
        let places: [(u32, Places); 4] = [(0, |_| false), (25, |j| j % 4 == 3), (50, |j| j % 2 == 1), (100, |_| true)];
        for (percent, tilts) in places {
            for trajectories in [1usize, 2, 9] {
                let expected: Vec<u64> = (0..NEIGHBORHOODS).filter(|&k| tilts(k / trajectories as u64)).collect();
                assert_eq!(tilted_ks(trajectories, percent), expected, "{percent} percent of {trajectories}");
            }
        }
        // Half the neighborhoods of two trajectories: k = 2 and 3, 6 and 7, ... (j = 1, 3, ... of both).
        assert_eq!(&tilted_ks(2, 50)[..6], [2, 3, 6, 7, 10, 11]);
        assert_eq!(tilted_ks(1, 50).len(), 32);
        for percent in [0, 1, 25, 33, 50, 99, 100] {
            for trajectories in [1usize, 2, 9] {
                for t in 0..trajectories {
                    // The neighborhoods of trajectory t, in order: the first m of them tilt floor(m P / 100) in all.
                    let ks = (t as u64..NEIGHBORHOODS).step_by(trajectories);
                    let mut tilted = 0u64;
                    for (m, k) in (1u64..).zip(ks) {
                        tilted += u64::from(draws_tilted(k, trajectories, percent));
                        let share = m * u64::from(percent);
                        assert_eq!(tilted, share / 100, "{percent} percent: {m} of trajectory {t} of {trajectories}");
                        assert!((tilted * 100).abs_diff(share) < 100);
                    }
                }
            }
        }
    }

    /// The tilted draw is in whole numbers, so it is the same on every machine: these are the promotions that four
    /// tilted neighborhoods of the greedy set of game 400 (21 promotions) free, the draws of the untilted test above.
    #[test]
    fn a_tilted_neighborhood_frees_the_same_promotions_on_every_machine() {
        let options = Options::default();
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let space = SearchSpace::new(&p);
        let lns = Lns { p: &p, space: &space, options };
        let freed: Vec<Vec<u32>> = [(1, 0), (1, 1), (2, 5), (7, 63)]
            .into_iter()
            .map(|(round, k)| {
                let unit = lns.neighborhood(&p.greedy, &mut options.rng_of(p.n, round, k), true);
                assert_eq!(unit.cands, p.candidates.difference(&unit.promoted));
                p.greedy.difference(&unit.promoted).to_vec()
            })
            .collect();
        let pinned = [vec![75, 98, 153, 165], vec![75, 150, 156, 168], vec![75, 98, 182], vec![75, 98, 150, 165, 184]];
        assert_eq!(freed, pinned);
    }

    /// `tilted_freed` is the successive draw it says, whatever the number freed: the same draws taken with the
    /// cumulative weights of the members left.
    #[test]
    fn the_tilted_draw_takes_the_member_whose_cumulative_weight_covers_each_number() {
        let p = board(1000).prepare(SearchTactics::default()).unwrap();
        let members = p.candidates.to_vec();
        for (seed, count) in
            [(1, 0), (2, 1), (3, 7), (4, members.len() / 2), (5, members.len() - 1), (6, members.len())]
        {
            let (mut rng, mut other) = (Rng(seed), Rng(seed));
            let freed = tilted_freed(&members, p.n, count, &mut rng);
            let mut left = members.clone();
            let mut expected = Vec::new();
            for _ in 0..count {
                let ends: Vec<u64> = left
                    .iter()
                    .scan(0, |sum, &x| {
                        *sum += tilt_weight(x, p.n);
                        Some(*sum)
                    })
                    .collect();
                let draw = other.next() % ends[ends.len() - 1];
                expected.push(left.remove(ends.partition_point(|&end| end <= draw)));
            }
            assert_eq!(freed, expected, "{count} freed");
            assert_eq!(rng.next(), other.next(), "the same numbers drawn");
        }
    }

    /// The members freed by `lns` in `draws` neighborhoods of `best`, all `tilted` or all uniform: how many, and the sum
    /// of their sizes x/n.
    fn freed_by(lns: &Lns<'_, W>, best: &Bits<W>, draws: u64, tilted: bool) -> (u64, f64) {
        let (mut count, mut sizes) = (0, 0.0);
        for k in 0..draws {
            let fixed = lns.neighborhood(best, &mut lns.options.rng_of(lns.p.n, 1, k), tilted).promoted;
            for x in best.difference(&fixed).iter() {
                count += 1;
                sizes += f64::from(x) / f64::from(lns.p.n);
            }
        }
        (count, sizes)
    }

    #[test]
    fn a_tilted_neighborhood_frees_as_many_promotions_as_a_uniform_one_and_favors_the_small_ones() {
        let p = board(1000).prepare(SearchTactics::default()).unwrap();
        let space = SearchSpace::new(&p);
        // The candidates stand in for an incumbent: the draw only splits a set in two.
        let best = p.candidates;
        assert!(best.len() > 100);
        let lns = Lns { p: &p, space: &space, options: Options::default() };
        // With a share fixed that is one number, the number freed is the same.
        for share in [0.0, 0.3, 0.8, 1.0] {
            let options = Options { fix: (share, share), ..lns.options };
            let lns = Lns { options, ..lns };
            let expected = u64::from(best.len()) - (f64::from(best.len()) * share).round() as u64;
            for k in 0..20 {
                let counts: Vec<u32> = [false, true]
                    .into_iter()
                    .map(|tilted| {
                        let fixed = lns.neighborhood(&best, &mut options.rng_of(1000, 1, k), tilted).promoted;
                        best.len() - fixed.len()
                    })
                    .collect();
                assert_eq!(counts, [expected as u32; 2], "share {share}, draw {k}");
            }
        }
        // With the default share, the numbers freed average alike, and the tilted ones are smaller.
        let (uniform_count, uniform_sizes) = freed_by(&lns, &best, 300, false);
        let (tilted_count, tilted_sizes) = freed_by(&lns, &best, 300, true);
        let mean = |count: u64, sizes: f64| sizes / count as f64;
        let per_draw = |count: u64| count as f64 / 300.0;
        assert!((per_draw(uniform_count) - per_draw(tilted_count)).abs() < 0.1 * per_draw(uniform_count));
        let (small, large) = (mean(tilted_count, tilted_sizes), mean(uniform_count, uniform_sizes));
        assert!(
            small < large - 0.02,
            "the tilted draw frees promotions of mean size {small:.3} n, the uniform {large:.3} n"
        );
    }

    #[test]
    fn the_stage_moves_down_after_s_rounds_without_growth_and_back_on_growth() {
        let run = |widen: u64, grew: &[bool]| {
            let mut standing = Standing::default();
            grew.iter()
                .map(|&grew| {
                    standing = standing.after(grew, widen);
                    standing.stage
                })
                .collect::<Vec<_>>()
        };
        let (no, yes) = (false, true);
        // Three rounds without growth for each stage down, the last stage staying.
        assert_eq!(run(3, &[no; 10]), [0, 0, 1, 1, 1, 2, 2, 2, 2, 2]);
        // Growth returns it to the first stage, and the rounds are counted again from it.
        assert_eq!(run(3, &[no, no, no, no, no, yes, no, no, no]), [0, 0, 1, 1, 1, 0, 0, 0, 1]);
        assert_eq!(run(3, &[no, no, yes, no, no, no, yes, yes]), [0, 0, 0, 0, 0, 1, 0, 0]);
        assert_eq!(run(1, &[no, no, no, yes, no, yes, yes]), [1, 2, 2, 0, 1, 0, 0]);
        assert_eq!(Standing::default(), Standing { stage: 0, stalled: 0 });
    }

    /// Cases of the options that shape the rounds, each with the starts of its trajectories: the tilted share, the
    /// widening (after a round without growth), two trajectories, all three, and half the neighborhoods tilted, of two
    /// trajectories.
    fn shaped_cases(p: &Prepared<W>) -> Vec<(&'static str, Options, Vec<Bits<W>>)> {
        let half = p.greedy.to_vec();
        let poor: Bits<W> = half[..half.len() / 2].iter().copied().collect();
        let options = Options::default();
        let (tilt, widen) = (Options { tilt: 100, ..options }, Options { widen: Some(1), ..options });
        let all = Options { tilt: 100, widen: Some(1), ..options };
        vec![
            ("tilt", tilt, vec![p.greedy]),
            ("widen", widen, vec![p.greedy]),
            ("two trajectories", options, vec![p.greedy, poor]),
            ("all three", all, vec![p.greedy, poor]),
            ("half tilt", Options { tilt: 50, ..options }, vec![p.greedy, poor]),
        ]
    }

    /// The module `$module` of five tests, one for each case of [`shaped_cases`], each calling `$check` with the index
    /// of its case: the cases together use more CPU than a test may.
    macro_rules! for_each_case {
        ($module:ident, $check:ident) => {
            mod $module {
                #[test]
                fn tilt() {
                    super::$check(0);
                }
                #[test]
                fn widen() {
                    super::$check(1);
                }
                #[test]
                fn two_trajectories() {
                    super::$check(2);
                }
                #[test]
                fn all_three() {
                    super::$check(3);
                }
                #[test]
                fn half_tilt() {
                    super::$check(4);
                }
            }
        };
    }

    /// The rounds of case `case` of [`shaped_cases`] are the same on one thread and on four.
    fn check_same_on_any_number_of_threads(case: usize) {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let (what, options, starts) = shaped_cases(&p).swap_remove(case);
        let budget = 2 * rounds_with(&p, options, &starts, ONE_ROUND, 4, None).work;
        let one = rounds_with(&p, options, &starts, budget, 1, None);
        assert!(one.rounds >= 2 && p.can_promote(&one.set), "{what}: {} rounds", one.rounds);
        assert_eq!(rounds_with(&p, options, &starts, budget, 4, None), one, "{what}: 4 threads");
        // Every round of every trajectory is at one stage, and without widen at the first.
        let at: Vec<u64> = one.stage_rounds.iter().map(|stages| stages.iter().sum()).collect();
        assert_eq!(at, vec![one.rounds; starts.len()], "{what}");
        if options.widen.is_none() {
            assert!(one.stage_rounds.iter().all(|stages| stages[1..] == [0, 0]), "{what}: {:?}", one.stage_rounds);
        }
    }
    for_each_case!(same_on_any_number_of_threads, check_same_on_any_number_of_threads);

    /// The improvements of case `case` of [`shaped_cases`] at a budget are a prefix of those at a larger one.
    fn check_prefix_property(case: usize) {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let (what, options, starts) = shaped_cases(&p).swap_remove(case);
        let less = rounds_with(&p, options, &starts, ONE_ROUND, 4, None);
        let more = rounds_with(&p, options, &starts, 3 * less.work, 4, None);
        assert!(more.rounds > less.rounds, "{what}: {} rounds, then {}", less.rounds, more.rounds);
        assert!(more.improvements.starts_with(&less.improvements), "{what}: {less:?}, {more:?}");
    }
    for_each_case!(prefix_property, check_prefix_property);

    #[test]
    fn a_trajectory_that_stalls_widens_its_neighborhoods_and_one_that_grows_does_not() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let options = Options { widen: Some(1), ..Options::default() };
        let budget = 12 * rounds_with(&p, options, &[p.greedy], ONE_ROUND, 4, None).work;
        let ran = rounds_with(&p, options, &[p.greedy], budget, 4, None);
        let [stages] = ran.stage_rounds[..] else { panic!("one trajectory") };
        assert_eq!(stages.iter().sum::<u64>(), ran.rounds);
        assert!(stages[0] >= 1 && stages[1] >= 1 && stages[2] >= 1, "{stages:?} in {} rounds", ran.rounds);
        // The stage an improvement records is the stage its round worked at, so the first round's is the first.
        assert_eq!(ran.improvements[0].stage, 0);
        assert!(ran.improvements.iter().all(|improvement| improvement.stage < STAGES));
    }

    #[test]
    fn an_offered_set_is_checked_before_it_becomes_the_incumbent() {
        let board = board(300);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let control = Control::new();
        // All the candidates cannot be promoted together, and a number outside them is no promotion.
        assert!(!p.can_promote(&p.candidates));
        let outside = Bits::<W>::EMPTY.with(299);
        assert!(!p.candidates.contains(299) && !p.can_promote(&outside));
        for set in [p.candidates, outside, p.greedy.with(299)] {
            let refused = offer_found(&p, &control, &set, set.sum()).unwrap_err();
            assert!(refused.contains("not a set of candidates"), "{refused}");
            assert_eq!(control.best(), None, "a refused set is no incumbent");
        }
        // A set worth other than it claims is refused too.
        let refused = offer_found(&p, &control, &p.greedy, p.greedy.sum() + 1).unwrap_err();
        assert!(refused.contains("not the"), "{refused}");
        assert_eq!(control.best(), None);
        // A promotable set worth what it claims is the incumbent, once; a lighter one is not.
        assert_eq!(offer_found(&p, &control, &p.greedy, p.greedy.sum()), Ok(true));
        assert_eq!(offer_found(&p, &control, &p.greedy, p.greedy.sum()), Ok(false));
        let lighter = p.greedy.without(p.greedy.max().unwrap());
        assert!(p.can_promote(&lighter));
        assert_eq!(offer_found(&p, &control, &lighter, lighter.sum()), Ok(false));
        assert_eq!(best_so_far::<W>(&control), Some(p.greedy));
    }

    /// The finder's rounds on game `n` from the greedy set, for `budget`, with the seed `seed`: what it reports and
    /// the control it offered to.
    fn rounds_for(n: u32, seed: Option<Bits<W>>, budget: Duration) -> (Report, Arc<Control>, Prepared<W>) {
        let p = board(n).prepare(SearchTactics::default()).unwrap();
        let control = Control::new();
        control.offer(p.greedy.sum(), &payload_of(&p.greedy));
        let (_alive, ended) = mpsc::channel::<()>();
        let plan = Plan { after: 1, budget, seed };
        let report = rounds(&p, &control, &plan, 4096, &ended, None);
        (report, control, p)
    }

    #[test]
    fn the_finder_improves_the_incumbent_and_offers_only_sets_that_can_be_promoted() {
        let (report, control, p) = rounds_for(800, None, Duration::from_millis(400));
        let best = best_so_far::<W>(&control).unwrap();
        assert!(p.can_promote(&best));
        assert_eq!((report.at_nodes, report.from, report.refused), (4096, p.greedy.sum(), 0));
        assert!(report.improved >= 1 && report.best > report.from, "{report:?}");
        assert_eq!(report.best, best.sum(), "the best it reached is the incumbent it offered");
        assert!(report.seconds >= 0.4, "{report:?}");
    }

    #[test]
    fn the_finder_starts_its_first_round_from_the_seed() {
        // The seed is the promotions of the known moves; the incumbent the finder is given is the greedy set, below it.
        // The first round starts from what survives of the seed, so the finder reaches the best set at once.
        let board = board(800);
        let seed = board.seed(Optimal::built_in().moves(800).unwrap());
        let (report, control, p) = rounds_for(800, Some(seed), Duration::from_millis(50));
        assert_eq!(known(&board), report.best, "{report:?}");
        assert_eq!(best_so_far::<W>(&control).unwrap().sum(), known(&board));
        assert!(report.from < report.best && p.can_promote(&best_so_far::<W>(&control).unwrap()));
    }

    #[test]
    fn the_odd_rounds_start_from_the_seed() {
        let set = Some(Bits::<W>::EMPTY.with(1));
        let starts = |survivors: Option<Bits<W>>| (1..=8).map(|round| seed_start(survivors, round)).collect::<Vec<_>>();
        assert_eq!(starts(set), [set, None, set, None, set, None, set, None], "a seed has every odd round");
        assert_eq!(starts(None), [None; 8], "no seed, no seed rounds");
    }

    #[test]
    fn the_finder_ends_with_the_search() {
        let p = board(800).prepare(SearchTactics::default()).unwrap();
        let control = Control::new();
        control.offer(p.greedy.sum(), &payload_of(&p.greedy));
        let (alive, ended) = mpsc::channel::<()>();
        let plan = Plan { after: 1, budget: Duration::from_secs(600), seed: None };
        let began = Instant::now();
        let (prepared, steered, planned) = (&p, &control, &plan);
        let report = thread::scope(|scope| {
            let finder = scope.spawn(move || rounds(prepared, steered, planned, 1, &ended, None));
            thread::sleep(Duration::from_millis(100));
            assert!(!finder.is_finished(), "the finder ended before the search did");
            drop(alive);
            finder.join().unwrap()
        });
        assert!(began.elapsed() < Duration::from_secs(20), "{:?}", began.elapsed());
        assert!(report.seconds < 20.0, "{report:?}");
        // The finder of a search that ended before it started reports nothing.
        let (alive, ended) = mpsc::channel::<()>();
        drop(alive);
        assert_eq!(beside(&p, &control, &plan, 0, &ended, None), None);
    }

    fn two_threads() -> EngineConfig {
        EngineConfig { threads: 2, progress: None, ..EngineConfig::default() }
    }

    #[test]
    fn a_search_with_a_finder_has_the_answer_of_a_search_without() {
        // A tree of thousands of steps, so the search lasts until the finder looks.
        let board = board(1000);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let plain = search(&p, &two_threads(), None, None, false).unwrap();
        assert!(plain.completed && plain.finder.is_none());
        assert_eq!(plain.sum, known(&board));
        let seed = board.seed(Optimal::built_in().moves(999).unwrap());
        let plan = Plan { after: 1, budget: Duration::from_secs(30), seed: Some(seed) };
        let beside = search_beside(&p, &two_threads(), None, None, false, Some(&plan)).unwrap();
        assert!(beside.completed);
        assert_eq!(beside.sum, plain.sum);
        assert!(p.can_promote(&beside.promotions) && beside.promotions.sum() == beside.sum);
        // It started if the search lasted until the finder looked (a search that ends first reports nothing).
        if let Some(report) = beside.finder {
            assert!(report.at_nodes >= 1 && report.best >= report.from && report.refused == 0, "{report:?}");
            assert!(report.seconds < 30.0, "the search ended, so the finder did: {report:?}");
        }
    }
}
