//! `wall exchanges`: whether the local prime exchanges among a node's substitutes are symmetries of the node, and what
//! pruning by them saves.  Units of the tuning suite (or whole games) are solved as `wall anatomy` solves them, with a
//! hook on every step (`search::StepHook`) that checks, wherever a node goes on without a member, each exchange of two
//! primes that maps the member onto another candidate; with `--prune` it also prunes by the exchanges that pass.  The
//! notebook entry solver/notebook/local-prime-exchanges-at-the-nodes-2026-10/README.md states the checks and proves the
//! rule; this comment says what the code does.
//!
//! ```text
//! cargo run --profile test --features studies --example wall -- exchanges --suite FILE --known FILE --ids ID[,ID...]
//!     [--prune] [--below D] [--sample K] [--nodes K] [--tactic NAME=VALUE]... N
//! cargo run --profile test --features studies --example wall -- exchanges --known FILE --root [--prune] [--below D]
//!     [--sample K] [--nodes K] [--tactic NAME=VALUE]... N|A-B...
//! ```
//!
//! **The checks.**  For primes `p < q`, `π` exchanges their exponents in every number (`symmetry::swap_primes`).  At a
//! node (its frames, its candidates `C`), a number has a *place* ([`Place`]).  The *movers* of frame `f` are its moves
//! and the members of `C` of home `f + 1`; a mover and a factor of frame `f` are *adjacent* when the factor is a maximal
//! factor of the mover.  The *component* of a candidate `x` is the smallest set `A` of numbers that `π` moves which
//! holds `x`, holds `π(a)` with every `a`, and holds every number that `π` moves and that is adjacent to one of its
//! members.  Three checks, from the strictest:
//!
//! * *whole*: every number of the node's frames that `π` moves has its image in its place (Theorem 13's conditions, at
//!   the node);
//! * *component*: every member of the component has its image in its place;
//! * *reduced*: the same, with the frames *reduced* first: a factor of frame `f` that is no candidate and is a maximal
//!   factor of exactly one mover of `f` is *private*, and that mover *inert* in `f` (it can be played last, with it, in
//!   every completion), and their edges are left out; a number and its image must also agree on being inert and
//!   private ([`View::status`]).
//!
//! Each implies the next.  Where the reduced check passes, `π` on `A` and the identity elsewhere map every completion
//! of the node to a completion worth `Σ (π(a) − a)` over its members in `A` more.
//!
//! **The rule** (`--prune`).  Where the reduced check passes at a node `N₀`, a heaviest completion `F` of `N₀` has
//! `w(F) = Σ_{a ∈ F ∩ A} (π(a) − a) ≤ 0`, and so does every heaviest promotion set in the subtree of `N₀` (the node's
//! later steps and their children): the subtree needs only the completions that keep it.  At a node below, with
//! promotions `P` and candidates `C`, the members of `P` in `A` add `b` to `w` and the candidates of negative weight
//! can take at most `r` off: the node is cut if `b > r`, and a candidate of positive weight `w(x)` is dropped if
//! `b + w(x) > r`, until nothing changes over all the constraints the path holds ([`Constraint`]).
//!
//! **The samples** (`--sample K`), not sound: at an exchange swap (a node went on without the member `x` and its
//! heaviest set took the member's image `y`) whose reduced check failed and whose image is the lighter, `K` random
//! maximal completions of the node that hold `y` are tried with `y` exchanged for `x`.  If every one can be promoted,
//! the pair's dominance *held*: an estimate of where the node might go on without `y` as well.
//!
//! **The lines.**  One a unit, or a game with `--root`: `id`, `mode` (`check` or `prune`), `below` (the incumbent is
//! P* less it, as a floor with no set, when it is positive), `tactics`, `nodes` (the engine's steps), `steps`
//! (the hook's: the same), `tree_nodes` and `leaves` (nodes with no child), `completed`, `best` (the sum of the set the
//! search ended with: the incumbent's or one it found, 0 from a floor if it found none), `opt`, `cpu_s`; `exclusions`
//! (steps that went on without a member), `with_exchange` (of those, the ones where an exchange of two primes maps the
//! member onto another candidate), `whole`, `component` and `reduced` (where that check passes for one of them), and
//! `beyond_root` (where the reduced check passes for a pair that is no symmetric pair of the root); `priced`
//! (exclusions whose node checked its bound again at its next step), `exchange_swaps` (of those, the ones whose
//! heaviest set took the member's image under one of its exchanges) and `whole_swaps`, `component_swaps`,
//! `reduced_swaps` and `beyond_root_swaps` (where that exchange passed); `cheap_priced`, `cheap_exchange`,
//! `cheap_reduced` and `cheap_beyond_root`, the same of the exclusions priced under 50; `sample` (K), `sampled` and
//! `sampled_held` (the exchange swaps sampled, and those whose dominance held), `cheap_sampled` and
//! `cheap_sampled_held`; `constraints`, `dropped` and `cut` (what the rule did), and `dropped_beyond_root` and
//! `cut_beyond_root` (what it did by constraints whose pair is no symmetric pair of the root); `swaps` (the exchange
//! swaps met most:
//! the member, its image, the two primes, the times, the median price, how many times the whole, the component and the
//! reduced check passed, whether the pair is one of the root's, and how many were sampled and held), `cheap` (the cheap
//! exclusions met most: the member, what came in, the times, the median price, whether an exchange maps the member
//! into what came in, and how many times its reduced check passed); and the reduced check's failures, by kind over all
//! the exchanges tried (`fails`) and at the exchange swaps (`swap_fails`), and the commonest at the swaps
//! (`fail_examples`: the kind, the primes, the member, the number that failed and its image, the times).  A kind is the
//! places of the number and of its image, `M`, `F`, `C` for a move, a factor and a candidate, `-` for none and `>`
//! for an image past `n`, with `*` if their frames differ (a candidate and a move of the frame in front of its home,
//! where its promoted twin is, count as one frame) and `~` if only being inert or private does.

use super::anatomy::suite_units;
use super::{beyond, Line, Options};
use crate::counters::cpu_seconds;
use crate::known::known_incumbent;
use crate::rng::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Control, EngineConfig, Start};
use taxman::game::Optimal;
use taxman::matchbound::MatchGraph;
use taxman::model::{FactorProvider, Frame, FrameGraph, FrameSet, Scratch as ModelScratch};
use taxman::numtheory;
use taxman::search::symmetry::{root_pairs, swap_primes};
use taxman::search::{
    payload_of, set_of_payload, MakeHook, Prepared, SearchSpace, StepEnd, StepHook, StepRecord, TaxSearcher, TaxUnit,
};
use taxman::tactics::Tactics;

/// The options of `exchanges`.
#[derive(Clone, Copy, Default)]
pub struct Params {
    /// Prune by the exchanges that pass (`--prune`); otherwise only check them, and the tree is the search's own.
    pub prune: bool,
    /// The incumbent is P* less this, as a floor with no set (`--below`); 0: the known optimum's set.
    pub below: u64,
    /// Search each game from its root (`--root`) instead of units of the suite.
    pub root: bool,
    /// At each exchange swap whose reduced check failed, try the pair's dominance on this many random maximal
    /// completions (`--sample K`; 0: none).
    pub sample: usize,
}

/// An exclusion priced under this many points is *cheap* (the anatomy's line).
const CHEAP: i64 = 50;

/// How many of the swaps, cheap exclusions and failures a line lists.
const LISTED: usize = 16;

// ---------------------------------------------------------------------------------------------------------
// The checks

/// Where a number is at a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Place {
    /// A move of the frame.
    Move(usize),
    /// A factor of the frame that is no candidate of the node.
    Factor(usize),
    /// A candidate of the node, a factor of its home frame.
    Candidate(usize),
}

impl Place {
    /// The frame where the number is a mover: a move's, or the one in front of a candidate's home.
    fn mover_frame(self) -> Option<usize> {
        match self {
            Place::Move(f) => Some(f),
            Place::Candidate(h) => h.checked_sub(1),
            Place::Factor(_) => None,
        }
    }

    /// The frame where the number is a factor.
    fn factor_frame(self) -> Option<usize> {
        match self {
            Place::Factor(f) | Place::Candidate(f) => Some(f),
            Place::Move(_) => None,
        }
    }
}

/// What the checks compare of a number and its image: its place and, for the reduced check, whether it is inert as a
/// mover and private as a factor.
type Status = (Place, bool, bool);

/// Why a check failed: the first number met whose image's status is not its own, the image, and both statuses.
#[derive(Clone, Copy, Debug)]
struct Failure {
    number: u32,
    image: u32,
    from: Option<Status>,
    to: Option<Status>,
}

impl Failure {
    /// `M/F`, `C/-`, ... for the places of the number and of its image (`>` for an image past `n`), with `~` when only
    /// being inert or private differs, and `*` when they are in different frames, a candidate's promoted twin (a move
    /// of the frame in front of its home) aside.
    fn kind(&self, n: u32) -> String {
        let letter = |s: Option<Status>, x: u32| match s.map(|s| s.0) {
            Some(Place::Move(_)) => 'M',
            Some(Place::Factor(_)) => 'F',
            Some(Place::Candidate(_)) => 'C',
            None if x > n => '>',
            None => '-',
        };
        let near = |a: Place, b: Place| match (a, b) {
            (Place::Candidate(h), Place::Move(f)) | (Place::Move(f), Place::Candidate(h)) => f + 1 == h,
            (
                Place::Move(f) | Place::Factor(f) | Place::Candidate(f),
                Place::Move(g) | Place::Factor(g) | Place::Candidate(g),
            ) => f == g,
        };
        let mark = match (self.from, self.to) {
            (Some(a), Some(b)) if a.0 == b.0 => "~",
            (Some(a), Some(b)) if !near(a.0, b.0) => "*",
            _ => "",
        };
        format!("{}/{}{mark}", letter(self.from, self.number), letter(self.to, self.image))
    }
}

/// What every searcher's hook of a game shares: the model, the primes, and the options.
struct Game {
    /// The game's n, and its model.
    n: u32,
    model: Arc<FrameGraph>,
    /// The smallest prime factor of each number up to `top`, and the primes up to it.
    spf: Vec<u32>,
    primes: Vec<u32>,
    /// The largest number of the root's frames.
    top: u32,
    /// The root's symmetric pairs (`symmetry::root_pairs`), whose rule the search has with the tactic `symmetry`.
    root_pairs: Vec<(u32, u32)>,
    /// `--prune` and `--sample`.
    prune: bool,
    sample: usize,
}

impl Game {
    fn new<const W: usize>(n: u32, p: &Prepared<W>, params: Params) -> Game {
        let top = p.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(1);
        let (spf, primes) = numtheory::sieve(top);
        let root_pairs = root_pairs(&p.model, &p.root, &p.candidates);
        let (prune, sample) = (params.prune, params.sample);
        Game { n, model: Arc::clone(&p.model), spf, primes, top, root_pairs, prune, sample }
    }

    fn graph(&self) -> &FactorProvider {
        self.model.graph()
    }

    /// The exchanges of two primes that map the candidate `x` onto another candidate: `(p, q, π(x))` with `p < q`.
    fn exchanges_of<const W: usize>(
        &self,
        cands: &Bits<W>,
        x: u32,
        factors: &mut Vec<(u32, u32)>,
    ) -> Vec<(u32, u32, u32)> {
        factors.clear();
        let mut rest = x;
        while rest > 1 {
            let r = self.spf[rest as usize];
            let mut e = 0;
            while rest.is_multiple_of(r) {
                rest /= r;
                e += 1;
            }
            factors.push((r, e));
        }
        let mut found = Vec::new();
        // A prime r of x for one that does not divide it: r^a·m becomes s^a·m.
        for &(r, a) in factors.iter() {
            let m = x / r.pow(a);
            for &s in &self.primes {
                let Some(y) = s.checked_pow(a).and_then(|s_a| s_a.checked_mul(m)).filter(|&y| y <= self.top) else {
                    break;
                };
                if s != r && !m.is_multiple_of(s) && cands.contains(y) {
                    found.push((r.min(s), r.max(s), y));
                }
            }
        }
        // Two primes of x with different exponents.
        for (i, &(r, a)) in factors.iter().enumerate() {
            for &(s, b) in &factors[i + 1..] {
                let y = swap_primes(x, r, s);
                if a != b && cands.contains(y) {
                    found.push((r.min(s), r.max(s), y));
                }
            }
        }
        found
    }
}

/// A node as the checks see it: its frames and its candidates.
struct View<'a, const W: usize> {
    game: &'a Game,
    frames: &'a [Frame<W>],
    cands: &'a Bits<W>,
}

impl<const W: usize> View<'_, W> {
    /// The place of `x`, or `None` if it is in no frame.
    fn place(&self, x: u32) -> Option<Place> {
        for (f, frame) in self.frames.iter().enumerate() {
            if frame.moves.contains(x) {
                return Some(Place::Move(f));
            }
            if frame.factors.contains(x) {
                return Some(if self.cands.contains(x) { Place::Candidate(f) } else { Place::Factor(f) });
            }
        }
        None
    }

    /// Whether `c` is a mover of frame `f`: a move of it, or a candidate of home `f + 1`.
    fn is_mover(&self, f: usize, c: u32) -> bool {
        self.frames[f].moves.contains(c)
            || (self.cands.contains(c) && self.frames.get(f + 1).is_some_and(|behind| behind.factors.contains(c)))
    }

    /// The factors of frame `f` that are maximal factors of `u`.  The factor graph lists every pair that can meet in a
    /// frame.
    fn factors_in(&self, f: usize, u: u32) -> impl Iterator<Item = u32> + '_ {
        self.game.graph().factors(u).iter().map(|&y| u32::from(y)).filter(move |&y| self.frames[f].factors.contains(y))
    }

    /// The movers of frame `f` that `y` is a maximal factor of.
    fn listers(&self, f: usize, y: u32) -> impl Iterator<Item = u32> + '_ {
        self.game.graph().composites(y).iter().map(|&c| u32::from(c)).filter(move |&c| self.is_mover(f, c))
    }

    /// Whether `y`, a factor of frame `f`, is private: no candidate, and a maximal factor of exactly one mover of `f`.
    fn private(&self, f: usize, y: u32) -> bool {
        !self.cands.contains(y) && self.listers(f, y).take(2).count() == 1
    }

    /// Whether `u`, a mover of frame `f`, is inert there: it has a private factor.
    fn inert(&self, f: usize, u: u32) -> bool {
        self.factors_in(f, u).any(|y| self.private(f, y))
    }

    /// The status of `x`: its place and, if `reduce`, whether it is inert where it moves and private where it is a
    /// factor (a candidate never is: a completion may promote it).
    fn status(&self, x: u32, reduce: bool) -> Option<Status> {
        let at = self.place(x)?;
        if !reduce {
            return Some((at, false, false));
        }
        let inert = at.mover_frame().is_some_and(|f| self.inert(f, x));
        let private = matches!(at, Place::Factor(f) if self.private(f, x));
        Some((at, inert, private))
    }

    /// The numbers adjacent to `x`, at `at`: as a mover, its maximal factors among the frame's factors; as a factor,
    /// the frame's movers it is a maximal factor of; a candidate is both.  With `reduce`, without the edges of the inert
    /// movers and the private factors.
    fn neighbours(&self, x: u32, at: Place, reduce: bool, out: &mut Vec<u32>) {
        if let Some(f) = at.mover_frame().filter(|&f| !(reduce && self.inert(f, x))) {
            out.extend(self.factors_in(f, x).filter(|&y| !(reduce && self.private(f, y))));
        }
        if let Some(f) = at.factor_frame().filter(|&f| !(reduce && self.private(f, x))) {
            out.extend(self.listers(f, x).filter(|&c| !(reduce && self.inert(f, c))));
        }
    }

    /// The component check of the exchange of `p` and `q` from the candidate `seed` (reduced if `reduce`; see the module
    /// docs): the component, or the first failure.
    fn component(
        &self,
        (p, q): (u32, u32),
        seed: u32,
        reduce: bool,
        scratch: &mut Scratch,
    ) -> Result<Bits<W>, Failure> {
        let (queue, near) = (&mut scratch.queue, &mut scratch.near);
        queue.clear();
        queue.push(seed);
        let mut moved = Bits::EMPTY;
        while let Some(x) = queue.pop() {
            if moved.contains(x) {
                continue;
            }
            let y = swap_primes(x, p, q);
            let (from, to) = (self.status(x, reduce), self.status(y, reduce));
            let (Some((at, _, _)), true) = (from, from == to) else {
                return Err(Failure { number: x, image: y, from, to });
            };
            moved.insert(x);
            moved.insert(y);
            for z in [x, y] {
                near.clear();
                self.neighbours(z, at, reduce, near);
                queue.extend(near.iter().filter(|&&w| !moved.contains(w) && swap_primes(w, p, q) != w));
            }
        }
        Ok(moved)
    }

    /// The whole-node check: every number of the node's frames that the exchange moves has its image in its place.
    fn whole(&self, (p, q): (u32, u32)) -> bool {
        [p, q].into_iter().all(|r| {
            (r..=self.game.top).step_by(r as usize).all(|x| {
                let y = swap_primes(x, p, q);
                y == x || self.place(x) == self.place(y)
            })
        })
    }
}

/// A random maximal completion of the node `state` among `cands`: `first` (if given) promoted first, then the others
/// tried in a random order; `None` if `first` cannot be promoted there.
fn completion<const W: usize>(
    model: &FrameGraph,
    scratch: &mut ModelScratch<W>,
    rng: &mut Rng,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    first: Option<u32>,
) -> Option<Bits<W>> {
    let (mut now, mut set) = (*state, Bits::EMPTY);
    if let Some(first) = first {
        if !model.promote_in_place_using(scratch, &mut now, first) {
            return None;
        }
        set.insert(first);
    }
    let mut order = (*cands - set).to_vec();
    rng.shuffle(&mut order);
    for x in order {
        if model.promote_in_place_using(scratch, &mut now, x) {
            set.insert(x);
        }
    }
    Some(set)
}

/// Whether `set` can be promoted from `state`, leaving no frame with negative freedoms.
fn promotable<const W: usize>(
    model: &FrameGraph,
    scratch: &mut ModelScratch<W>,
    state: &FrameSet<W>,
    set: &Bits<W>,
) -> bool {
    let after = model.promote_all_using(scratch, state, set);
    after.is_some_and(|s| s.frames().iter().all(|f| f.freedoms >= 0))
}

/// The lists a component check works in, reused.
#[derive(Default)]
struct Scratch {
    queue: Vec<u32>,
    near: Vec<u32>,
    factors: Vec<(u32, u32)>,
}

// ---------------------------------------------------------------------------------------------------------
// The hook

/// A constraint of the rule: derived at the node at `depth`, it holds for the node's subtree.  `weight` has the
/// candidates of the component then, each with `π(a) − a`; none of them was a promotion of that node, so those
/// promoted at a node below it are its completion's.  `root`: its primes are a symmetric pair of the root.
struct Constraint {
    depth: usize,
    root: bool,
    weight: Vec<(u32, i64)>,
}

/// Which checks passed; `beyond_root`: the reduced check, for a pair that is no symmetric pair of the root.
#[derive(Clone, Copy, Default)]
struct Passed {
    whole: bool,
    component: bool,
    reduced: bool,
    beyond_root: bool,
}

/// An exchange tried at an exclusion: the primes, the member's image, the checks it passed, and why the reduced check
/// failed if it did.
#[derive(Clone, Copy)]
struct Tried {
    pair: (u32, u32),
    image: u32,
    passed: Passed,
    failure: Option<Failure>,
}

/// An exclusion made at a node's step, for its next step to price.
struct Pending {
    member: u32,
    tried: Vec<Tried>,
}

/// The node at one depth of the stack.
struct Level<const W: usize> {
    /// Its last bound check: the value and the heaviest set.
    check: Option<(u64, Bits<W>)>,
    /// Whether it has made a child that was explored.
    parent: bool,
    pending: Option<Pending>,
}

impl<const W: usize> Default for Level<W> {
    fn default() -> Self {
        Level { check: None, parent: false, pending: None }
    }
}

/// How often each check passed, out of a count of tries.
#[derive(Clone, Copy, Default)]
struct Passes {
    whole: u64,
    component: u64,
    reduced: u64,
    beyond_root: u64,
}

impl Passes {
    fn add(&mut self, passed: Passed) {
        self.whole += u64::from(passed.whole);
        self.component += u64::from(passed.component);
        self.reduced += u64::from(passed.reduced);
        self.beyond_root += u64::from(passed.beyond_root);
    }

    fn merge(&mut self, other: Passes) {
        self.whole += other.whole;
        self.component += other.component;
        self.reduced += other.reduced;
        self.beyond_root += other.beyond_root;
    }
}

/// Of the exchange swaps sampled (`--sample`), how many held on every sample.
#[derive(Clone, Copy, Default)]
struct Sampled {
    tried: u64,
    held: u64,
}

impl Sampled {
    fn add(&mut self, held: Option<bool>) {
        self.tried += u64::from(held.is_some());
        self.held += u64::from(held == Some(true));
    }

    fn merge(&mut self, other: Sampled) {
        self.tried += other.tried;
        self.held += other.held;
    }
}

/// An exchange swap: its primes, its prices, how often each check passed, and what the samples said.
#[derive(Clone, Default)]
struct Swap {
    pair: (u32, u32),
    prices: Vec<i64>,
    passes: Passes,
    sampled: Sampled,
}

/// A cheap exclusion: its prices, whether an exchange maps the member into what came in, and how often that exchange
/// passed the reduced check.
#[derive(Clone, Default)]
struct Cheap {
    prices: Vec<i64>,
    exchange: bool,
    reduced: u64,
}

/// A failure of the reduced check at an exchange swap: its kind, the primes, the member, and the number and image
/// that failed.
type Example = (String, (u32, u32), u32, u32, u32);

/// What a unit's search counted.
#[derive(Default)]
struct Tally {
    steps: u64,
    nodes: u64,
    leaves: u64,
    exclusions: u64,
    with_exchange: u64,
    /// Exclusions where some exchange passed each check.
    passes: Passes,
    priced: u64,
    /// Priced exclusions that swap the member for its image under one of its exchanges, and how often that exchange
    /// passed each check.
    exchange_swaps: u64,
    swap_passes: Passes,
    /// Priced exclusions under [`CHEAP`], those of them that are exchange swaps, and those whose exchange passed the
    /// reduced check.
    cheap_priced: u64,
    cheap_exchange: u64,
    cheap_reduced: u64,
    cheap_beyond_root: u64,
    /// The exchange swaps sampled (`--sample`), and the cheap ones.
    sampled: Sampled,
    cheap_sampled: Sampled,
    constraints: u64,
    dropped: u64,
    cut: u64,
    /// Of the candidates dropped and the nodes cut, those by a constraint whose primes are no symmetric pair of the root.
    dropped_beyond_root: u64,
    cut_beyond_root: u64,
    swaps: HashMap<(u32, u32), Swap>,
    cheap: HashMap<(u32, Vec<u32>), Cheap>,
    /// The reduced check's failures by kind; and at the exchange swaps, by kind, and by kind, primes, member, and the
    /// number and image that failed.
    fails: HashMap<String, u64>,
    swap_fails: HashMap<String, u64>,
    examples: HashMap<Example, u64>,
}

impl Tally {
    fn add(&mut self, other: Tally) {
        let counts = [
            (&mut self.steps, other.steps),
            (&mut self.nodes, other.nodes),
            (&mut self.leaves, other.leaves),
            (&mut self.exclusions, other.exclusions),
            (&mut self.with_exchange, other.with_exchange),
            (&mut self.priced, other.priced),
            (&mut self.exchange_swaps, other.exchange_swaps),
            (&mut self.cheap_priced, other.cheap_priced),
            (&mut self.cheap_exchange, other.cheap_exchange),
            (&mut self.cheap_reduced, other.cheap_reduced),
            (&mut self.cheap_beyond_root, other.cheap_beyond_root),
            (&mut self.constraints, other.constraints),
            (&mut self.dropped, other.dropped),
            (&mut self.cut, other.cut),
            (&mut self.dropped_beyond_root, other.dropped_beyond_root),
            (&mut self.cut_beyond_root, other.cut_beyond_root),
        ];
        for (mine, theirs) in counts {
            *mine += theirs;
        }
        self.passes.merge(other.passes);
        self.swap_passes.merge(other.swap_passes);
        self.sampled.merge(other.sampled);
        self.cheap_sampled.merge(other.cheap_sampled);
        for (key, s) in other.swaps {
            let mine = self.swaps.entry(key).or_default();
            mine.pair = s.pair;
            mine.prices.extend(s.prices);
            mine.passes.merge(s.passes);
            mine.sampled.merge(s.sampled);
        }
        for (key, c) in other.cheap {
            let mine = self.cheap.entry(key).or_default();
            mine.prices.extend(c.prices);
            mine.exchange |= c.exchange;
            mine.reduced += c.reduced;
        }
        for (key, count) in other.fails {
            *self.fails.entry(key).or_default() += count;
        }
        for (key, count) in other.swap_fails {
            *self.swap_fails.entry(key).or_default() += count;
        }
        for (key, count) in other.examples {
            *self.examples.entry(key).or_default() += count;
        }
    }
}

/// The hook of one searcher: checks the exchanges at each exclusion and, with `prune`, keeps the constraints of the
/// path and narrows by them.  What it counts goes to `shared` when the searcher is dropped.
struct Local<const W: usize> {
    game: Arc<Game>,
    shared: Arc<Mutex<Tally>>,
    tally: Tally,
    depth: usize,
    levels: Vec<Level<W>>,
    constraints: Vec<Constraint>,
    /// The exclusion of the step in progress, for the level once the step is told.
    fresh: Option<Pending>,
    scratch: Scratch,
    /// For the samples (`--sample`).
    rng: Rng,
    promote: ModelScratch<W>,
}

impl<const W: usize> Local<W> {
    fn new(game: Arc<Game>, shared: Arc<Mutex<Tally>>) -> Self {
        Local {
            game,
            shared,
            tally: Tally { nodes: 1, ..Tally::default() },
            depth: 0,
            levels: vec![Level::default()],
            constraints: Vec::new(),
            fresh: None,
            scratch: Scratch::default(),
            rng: Rng(1),
            promote: ModelScratch::new(),
        }
    }

    /// Whether, on `--sample` random maximal completions `F` of the node `state` among `cands` that hold `lighter`,
    /// `F` with `lighter` exchanged for `heavier` can be promoted too: an estimate, not a proof, that the node may go
    /// on without `lighter` once it goes on without `heavier`.  (A set is promotable when a maximal one holding it is
    /// with the exchange, so the maximal sets are the ones to try.)
    fn dominated(&mut self, state: &FrameSet<W>, cands: &Bits<W>, heavier: u32, lighter: u32) -> bool {
        let model = &self.game.model;
        for _ in 0..self.game.sample {
            let Some(set) = completion(model, &mut self.promote, &mut self.rng, state, cands, Some(lighter)) else {
                return true;
            };
            if !promotable(model, &mut self.promote, state, &set.without(lighter).with(heavier)) {
                return false;
            }
        }
        true
    }

    /// The exclusion priced at the node's next step: `before` and `now` its bound checks; `state` and `cands` the node
    /// then.
    fn price(
        &mut self,
        pending: Pending,
        before: (u64, Bits<W>),
        now: (u64, Bits<W>),
        state: &FrameSet<W>,
        cands: &Bits<W>,
    ) {
        let price = before.0 as i64 - now.0 as i64;
        let gained = now.1 - before.1;
        let swap = pending.tried.iter().find(|tried| gained.contains(tried.image)).copied();
        // Where the check failed and the image is the lighter: did the pair's dominance hold on the samples?
        let held = match swap {
            Some(tried) if self.game.sample > 0 && !tried.passed.reduced && tried.image < pending.member => {
                Some(self.dominated(state, cands, pending.member, tried.image))
            }
            _ => None,
        };
        let t = &mut self.tally;
        t.priced += 1;
        if let Some(tried) = swap {
            t.exchange_swaps += 1;
            t.swap_passes.add(tried.passed);
            t.sampled.add(held);
            if price < CHEAP {
                t.cheap_sampled.add(held);
            }
            let s = t.swaps.entry((pending.member, tried.image)).or_default();
            s.pair = tried.pair;
            s.prices.push(price);
            s.passes.add(tried.passed);
            s.sampled.add(held);
            if let Some(failure) = tried.failure {
                *t.swap_fails.entry(failure.kind(self.game.n)).or_default() += 1;
                let example = (failure.kind(self.game.n), tried.pair, pending.member, failure.number, failure.image);
                *t.examples.entry(example).or_default() += 1;
            }
        }
        if price < CHEAP {
            let reduced = swap.is_some_and(|tried| tried.passed.reduced);
            t.cheap_priced += 1;
            t.cheap_exchange += u64::from(swap.is_some());
            t.cheap_reduced += u64::from(reduced);
            t.cheap_beyond_root += u64::from(swap.is_some_and(|tried| tried.passed.beyond_root));
            let c = t.cheap.entry((pending.member, gained.to_vec())).or_default();
            c.prices.push(price);
            c.exchange |= swap.is_some();
            c.reduced += u64::from(reduced);
        }
    }
}

impl<const W: usize> Drop for Local<W> {
    fn drop(&mut self) {
        let tally = std::mem::take(&mut self.tally);
        self.shared.lock().unwrap_or_else(PoisonError::into_inner).add(tally);
    }
}

impl<const W: usize> StepHook<W> for Local<W> {
    fn step(&mut self, r: &StepRecord<W>, state: &FrameSet<W>, _child: Option<&FrameSet<W>>) {
        self.tally.steps += 1;
        let d = self.depth;
        if self.levels.len() < d + 2 {
            self.levels.resize_with(d + 2, Level::default);
        }
        let now = r.node.map(|c| (c.value, c.chosen));
        let first = r.inherited || self.tally.steps == 1;
        if let Some(pending) = self.levels[d].pending.take().filter(|_| !first) {
            if let (Some(before), Some(now)) = (self.levels[d].check, now) {
                self.price(pending, before, now, state, &r.cands);
            }
        }
        if now.is_some() {
            self.levels[d].check = now;
        }
        self.levels[d].pending = self.fresh.take();
        match r.end {
            StepEnd::Child => {
                self.tally.nodes += 1;
                self.levels[d].parent = true;
                self.levels[d + 1] = Level { check: r.child.map(|c| (c.value, c.chosen)), ..Level::default() };
                self.depth = d + 1;
            }
            StepEnd::LoopEnd(_) => {
                self.tally.leaves += u64::from(!self.levels[d].parent);
                self.levels[d] = Level::default();
                while self.constraints.last().is_some_and(|c| c.depth >= d) {
                    self.constraints.pop();
                }
                self.depth = d.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn excluding(&mut self, state: &FrameSet<W>, _promoted: &Bits<W>, cands: &Bits<W>, member: u32) {
        self.tally.exclusions += 1;
        let game = Arc::clone(&self.game);
        let view = View { game: &game, frames: state.frames(), cands };
        let exchanges = game.exchanges_of(cands, member, &mut self.scratch.factors);
        let mut tried = Vec::with_capacity(exchanges.len());
        for (p, q, image) in exchanges {
            let component = view.component((p, q), member, false, &mut self.scratch).is_ok();
            let reduced = view.component((p, q), member, true, &mut self.scratch);
            if let Err(failure) = reduced {
                *self.tally.fails.entry(failure.kind(self.game.n)).or_default() += 1;
            }
            let root = game.root_pairs.contains(&(p, q));
            if let (Ok(moved), true) = (reduced, game.prune) {
                let weight =
                    (moved & *cands).iter().map(|a| (a, i64::from(swap_primes(a, p, q)) - i64::from(a))).collect();
                self.constraints.push(Constraint { depth: self.depth, root, weight });
                self.tally.constraints += 1;
            }
            let whole = component && view.whole((p, q));
            let beyond_root = reduced.is_ok() && !root;
            let passed = Passed { whole, component, reduced: reduced.is_ok(), beyond_root };
            tried.push(Tried { pair: (p, q), image, passed, failure: reduced.err() });
        }
        let t = &mut self.tally;
        t.with_exchange += u64::from(!tried.is_empty());
        t.passes.add(Passed {
            whole: tried.iter().any(|tried| tried.passed.whole),
            component: tried.iter().any(|tried| tried.passed.component),
            reduced: tried.iter().any(|tried| tried.passed.reduced),
            beyond_root: tried.iter().any(|tried| tried.passed.beyond_root),
        });
        self.fresh = Some(Pending { member, tried });
    }

    fn narrow(&mut self, promoted: &Bits<W>, cands: &mut Bits<W>) -> bool {
        loop {
            let mut changed = false;
            for c in &self.constraints {
                // What the promotions below the constraint's node add to its sum, and what the candidates of negative
                // weight can take off.
                let (mut added, mut room) = (0, 0);
                for &(a, w) in &c.weight {
                    if promoted.contains(a) {
                        added += w;
                    } else if w < 0 && cands.contains(a) {
                        room -= w;
                    }
                }
                if added > room {
                    self.tally.cut += 1;
                    self.tally.cut_beyond_root += u64::from(!c.root);
                    return true;
                }
                for &(a, w) in &c.weight {
                    if w > 0 && cands.contains(a) && added + w > room {
                        cands.remove(a);
                        self.tally.dropped += 1;
                        self.tally.dropped_beyond_root += u64::from(!c.root);
                        changed = true;
                    }
                }
            }
            if !changed {
                return false;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The command

/// What solving a unit gave.
struct Solved<const W: usize> {
    nodes: u64,
    completed: bool,
    /// The best set the search ended with: the incumbent's, or better.
    set: Bits<W>,
    cpu_s: f64,
    tally: Tally,
}

/// Solves `unit` on one thread from the incumbent worth `best` (its set `start`, or none: a floor), with fresh tables
/// and a hook made for each searcher from `game`.
fn solve<const W: usize>(
    p: &Prepared<W>,
    graph: &MatchGraph,
    game: &Arc<Game>,
    unit: TaxUnit<W>,
    (best, start): (u64, Bits<W>),
    node_limit: Option<u64>,
) -> Solved<W> {
    let shared = Arc::new(Mutex::new(Tally::default()));
    let make = {
        let (game, shared) = (Arc::clone(game), Arc::clone(&shared));
        move || Box::new(Local::<W>::new(Arc::clone(&game), Arc::clone(&shared))) as Box<dyn StepHook<W>>
    };
    let space = SearchSpace { hooks: Some(&make as &MakeHook<W>), ..SearchSpace::new(p, graph) };
    let cfg = EngineConfig { threads: 1, node_limit, control: Some(Control::new()), ..EngineConfig::default() };
    let begin = Start::Fresh { units: vec![unit], best, payload: payload_of(&start) };
    let before = cpu_seconds();
    let out = engine::run(&cfg, begin, b"exchanges", |_| TaxSearcher::new(&space)).expect("the engine");
    let cpu_s = before.zip(cpu_seconds()).map_or(0.0, |(a, b)| b - a);
    let tally = std::mem::take(&mut *shared.lock().unwrap_or_else(PoisonError::into_inner));
    Solved { nodes: out.stats.nodes, completed: out.completed, set: set_of_payload(&out.payload), cpu_s, tally }
}

fn median(xs: &[i64]) -> i64 {
    let mut xs = xs.to_vec();
    xs.sort_unstable();
    xs.get(xs.len() / 2).copied().unwrap_or(0)
}

fn json_list<T: ToString>(items: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", items.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}

/// The line of a solved unit.
fn line<const W: usize>(n: u32, id: &str, o: &Options, opt: u64, game: &Game, s: &Solved<W>) -> Line {
    let t = &s.tally;
    let mut swaps: Vec<_> = t.swaps.iter().collect();
    swaps.sort_by_key(|(key, s)| (std::cmp::Reverse(s.prices.len()), **key));
    let mut cheap: Vec<_> = t.cheap.iter().collect();
    cheap.sort_by(|a, b| b.1.prices.len().cmp(&a.1.prices.len()).then(a.0.cmp(b.0)));
    let kinds = |map: &HashMap<String, u64>| {
        by_count(map).into_iter().take(LISTED).map(|(kind, count)| format!("[\"{kind}\",{count}]")).collect::<Vec<_>>()
    };
    let mut out = Line::of(n);
    out.text("id", id)
        .text("mode", if o.exchanges.prune { "prune" } else { "check" })
        .field("below", o.exchanges.below)
        .text("tactics", &Tactics { search: o.tactics, ..Tactics::default() }.description())
        .field("nodes", s.nodes)
        .field("steps", t.steps)
        .field("tree_nodes", t.nodes)
        .field("leaves", t.leaves)
        .field("completed", s.completed)
        .field("best", s.set.sum())
        .field("opt", opt)
        .seconds("cpu_s", s.cpu_s)
        .field("exclusions", t.exclusions)
        .field("with_exchange", t.with_exchange)
        .field("whole", t.passes.whole)
        .field("component", t.passes.component)
        .field("reduced", t.passes.reduced)
        .field("beyond_root", t.passes.beyond_root)
        .field("priced", t.priced)
        .field("exchange_swaps", t.exchange_swaps)
        .field("whole_swaps", t.swap_passes.whole)
        .field("component_swaps", t.swap_passes.component)
        .field("reduced_swaps", t.swap_passes.reduced)
        .field("beyond_root_swaps", t.swap_passes.beyond_root)
        .field("cheap_priced", t.cheap_priced)
        .field("cheap_exchange", t.cheap_exchange)
        .field("cheap_reduced", t.cheap_reduced)
        .field("cheap_beyond_root", t.cheap_beyond_root)
        .field("sample", o.exchanges.sample)
        .field("sampled", t.sampled.tried)
        .field("sampled_held", t.sampled.held)
        .field("cheap_sampled", t.cheap_sampled.tried)
        .field("cheap_sampled_held", t.cheap_sampled.held)
        .field("constraints", t.constraints)
        .field("dropped", t.dropped)
        .field("cut", t.cut)
        .field("dropped_beyond_root", t.dropped_beyond_root)
        .field("cut_beyond_root", t.cut_beyond_root)
        .list(
            "swaps",
            swaps.iter().take(LISTED).map(|((x, y), s)| {
                let ((p, q), passes, times, price) = (s.pair, s.passes, s.prices.len(), median(&s.prices));
                let (whole, component, reduced) = (passes.whole, passes.component, passes.reduced);
                let root = game.root_pairs.contains(&s.pair);
                let (sampled, held) = (s.sampled.tried, s.sampled.held);
                format!("[{x},{y},{p},{q},{times},{price},{whole},{component},{reduced},{root},{sampled},{held}]")
            }),
        )
        .list(
            "cheap",
            cheap.iter().take(LISTED).map(|((x, gained), c)| {
                let (times, price) = (c.prices.len(), median(&c.prices));
                format!("[{x},{},{times},{price},{},{}]", json_list(gained.iter()), c.exchange, c.reduced)
            }),
        )
        .list("fails", kinds(&t.fails))
        .list("swap_fails", kinds(&t.swap_fails))
        .list(
            "fail_examples",
            by_count(&t.examples)
                .into_iter()
                .take(LISTED)
                .map(|((kind, (p, q), member, x, y), count)| format!("[\"{kind}\",{p},{q},{member},{x},{y},{count}]")),
        );
    out
}

/// The entries of `map`, the most frequent first (ties by key).
fn by_count<K: Ord>(map: &HashMap<K, u64>) -> Vec<(&K, &u64)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    entries
}

/// `exchanges` on game `n` at the set width `W`.
fn play<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return Ok(());
    }
    let p = board.prepare(o.tactics)?;
    let Some((set, opt)) = known_incumbent(&board, &p, optimal)? else {
        return Err(format!("n={n}: needs its known optimum (--known)"));
    };
    let units = if o.exchanges.root {
        vec![(n.to_string(), TaxUnit { promoted: Bits::EMPTY, cands: p.candidates })]
    } else {
        let suite = o.params.suite.as_ref().ok_or("exchanges needs --suite or --root")?;
        suite_units::<W>(suite, n, &o.anatomy.ids)?
    };
    let below = o.exchanges.below;
    let start = if below == 0 { (opt, set) } else { (opt.saturating_sub(below), Bits::EMPTY) };
    let graph = MatchGraph::new(&p.model, &p.root);
    let game = Arc::new(Game::new(n, &p, o.exchanges));
    for (id, unit) in units {
        let solved = solve(&p, &graph, &game, unit, start, o.nodes);
        line(n, &id, o, opt, &game, &solved).print();
    }
    Ok(())
}

/// `exchanges` on each game of `o`.
pub fn run(o: &Options, optimal: &Optimal) -> Result<(), String> {
    for &n in &o.games {
        taxman::with_width!(n, W => play::<W>(n, o, optimal), else Err(beyond(n)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::search::SearchTactics;

    /// How many random maximal completions each exchange that passes is tried on.
    const SAMPLES: usize = 4;

    /// At every exclusion of a search, each exchange of the member: the checks imply one another (whole, component,
    /// reduced), and where the reduced check passes, the image of random maximal completions of the node under `π` on
    /// the component is a completion of the node too, worth the formula's more.  Counts the exchanges it verified.
    struct Verifier<const W: usize> {
        game: Arc<Game>,
        rng: Rng,
        scratch: Scratch,
        promote: ModelScratch<W>,
        verified: Arc<Mutex<u64>>,
    }

    impl<const W: usize> StepHook<W> for Verifier<W> {
        fn step(&mut self, _: &StepRecord<W>, _: &FrameSet<W>, _: Option<&FrameSet<W>>) {}

        fn excluding(&mut self, state: &FrameSet<W>, _: &Bits<W>, cands: &Bits<W>, member: u32) {
            let game = Arc::clone(&self.game);
            let view = View { game: &game, frames: state.frames(), cands };
            for (p, q, _) in game.exchanges_of(cands, member, &mut self.scratch.factors) {
                let component = view.component((p, q), member, false, &mut self.scratch);
                let reduced = view.component((p, q), member, true, &mut self.scratch);
                assert!(!view.whole((p, q)) || component.is_ok(), "{p}, {q} at {member}: whole but not component");
                assert!(component.is_err() || reduced.is_ok(), "{p}, {q} at {member}: component but not reduced");
                let Ok(moved) = reduced else { continue };
                let pi = |a: u32| if moved.contains(a) { swap_primes(a, p, q) } else { a };
                for _ in 0..SAMPLES {
                    let model = &game.model;
                    let set = completion(model, &mut self.promote, &mut self.rng, state, cands, None).expect("a set");
                    let image: Bits<W> = set.iter().map(pi).collect();
                    assert!(image.is_subset(cands), "{p}, {q} at {member}: an image is no candidate");
                    assert!(
                        promotable(model, &mut self.promote, state, &image),
                        "{p}, {q} at {member}: the image of {set:?} cannot be promoted"
                    );
                    let gain: i64 = (set & moved).iter().map(|a| i64::from(pi(a)) - i64::from(a)).sum();
                    assert_eq!(image.sum() as i64 - set.sum() as i64, gain);
                }
                *self.verified.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            }
        }
    }

    /// The game `n` prepared with `tactics`, its known optimum and its set, from the built-in table.
    fn game<const W: usize>(n: u32, tactics: SearchTactics) -> (Prepared<W>, u64, Bits<W>) {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(tactics).expect("prepare");
        let (set, opt) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        (p, opt, set)
    }

    fn verify<const W: usize>(n: u32) -> u64 {
        let (p, opt, set) = game::<W>(n, SearchTactics::default());
        let graph = MatchGraph::new(&p.model, &p.root);
        let game = Arc::new(Game::new(n, &p, Params::default()));
        let verified = Arc::new(Mutex::new(0));
        let make = {
            let verified = Arc::clone(&verified);
            move || {
                let (game, verified) = (Arc::clone(&game), Arc::clone(&verified));
                let (rng, scratch, promote) = (Rng(u64::from(n)), Scratch::default(), ModelScratch::new());
                Box::new(Verifier { game, rng, scratch, promote, verified }) as Box<dyn StepHook<W>>
            }
        };
        let space = SearchSpace { hooks: Some(&make as &MakeHook<W>), ..SearchSpace::new(&p, &graph) };
        let cfg = EngineConfig { threads: 1, control: Some(Control::new()), ..EngineConfig::default() };
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let begin = Start::Fresh { units: vec![root], best: opt, payload: payload_of(&set) };
        engine::run(&cfg, begin, b"exchanges", |_| TaxSearcher::new(&space)).expect("the engine");
        let count = *verified.lock().unwrap_or_else(PoisonError::into_inner);
        count
    }

    #[test]
    fn an_exchange_that_passes_maps_completions_to_completions_worth_the_formula() {
        let verified = taxman::with_width!(1215, W => verify::<W>(1215), else panic!("{}", beyond(1215)));
        assert!(verified > 0, "no exchange passed at 1215");
    }

    /// The root search of game `n` from a floor `below` under P*, with the rule, finds P*; returns
    /// the candidates the rule dropped.
    fn prune<const W: usize>(n: u32, below: u64, symmetry: bool) -> u64 {
        let (p, opt, _) = game::<W>(n, SearchTactics { symmetry, ..SearchTactics::default() });
        let graph = MatchGraph::new(&p.model, &p.root);
        let game = Arc::new(Game::new(n, &p, Params { prune: true, ..Params::default() }));
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let solved = solve(&p, &graph, &game, root, (opt - below, Bits::EMPTY), None);
        assert!(solved.completed);
        assert_eq!(solved.set.sum(), opt, "n={n}, from {below} below, symmetry {symmetry}");
        assert!(solved.tally.constraints > 0, "n={n}: the rule never applied");
        solved.tally.dropped
    }

    #[test]
    fn the_rule_finds_the_optimum_from_a_floor_with_the_rule_of_symmetric_pairs_and_without() {
        let mut dropped = 0;
        for symmetry in [true, false] {
            for n in [1000, 1215] {
                dropped += taxman::with_width!(n, W => prune::<W>(n, 60, symmetry), else panic!("{}", beyond(n)));
            }
        }
        assert!(dropped > 0, "the rule dropped nothing");
    }
}
