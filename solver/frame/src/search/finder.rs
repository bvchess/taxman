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
//! is a function of the incumbent and those four.  The threads take the round's neighborhoods in any order; once all
//! are done, the best set they found becomes the incumbent, the lowest k's on a tie ([`Lns::round`]).  So the incumbent
//! after r rounds depends on the start, n, the seed and r alone, and a run of more rounds does those of a run of fewer
//! first.  These rounds are not those of the finder beside a search, which are timed.

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

/// How LNS searches, with the defaults the finders' measurements chose.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// The share of the incumbent's promotions kept fixed: drawn from this range for each unit.
    pub fix: (f64, f64),
    /// The step limit of one unit (the engine counts in blocks of 1,024).
    pub unit_nodes: u64,
    /// The seed of the random choices (mixed with n, so that each game has its own: [`rng`](Self::rng)).
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
/// every thread of a machine of up to 64 threads, and 64 of a larger one's; and since a level of effort is a power of 2
/// of rounds, every level is a power of 2 of neighborhoods.
pub const NEIGHBORHOODS: u64 = 64;

/// What a round of the rounds on many threads came to ([`Lns::round`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Round<const W: usize> {
    /// The heaviest set its neighborhoods found, the one of the lowest k among equals: the incumbent it started from,
    /// or better.
    pub set: Bits<W>,
    /// The engine's steps, over its neighborhoods.
    pub nodes: u64,
    /// Its neighborhoods searched to the end within the step limit.
    pub finished: u64,
}

/// A better set that a round of [`Lns::rounds`] found: the round (from 1), the incumbent's new promotion sum, and the
/// time on the caller's clock when the round ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Improvement {
    pub round: u64,
    pub sum: u64,
    pub seconds: f64,
}

/// What [`Lns::rounds`] came to.
#[derive(Clone, Debug, PartialEq)]
pub struct Rounds<const W: usize> {
    /// The incumbent after the last round: the start, or better.
    pub set: Bits<W>,
    /// The rounds done, and their engine's steps.
    pub rounds: u64,
    pub nodes: u64,
    /// Each round that found a better set, in order.
    pub improvements: Vec<Improvement>,
    /// A stop signal ended the rounds before `most` were done or `enough` said so.
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
    /// `options.fix`) fixed, and every other candidate free.
    pub fn neighborhood(&self, best: &Bits<W>, rng: &mut Rng) -> TaxUnit<W> {
        let Options { fix, .. } = self.options;
        let mut members = best.to_vec();
        rng.shuffle(&mut members);
        let share = fix.0 + rng.unit() * (fix.1 - fix.0);
        let fixed: Bits<W> = members[..(members.len() as f64 * share).round() as usize].iter().copied().collect();
        TaxUnit { promoted: fixed, cands: self.p.candidates.difference(&fixed) }
    }

    /// A neighborhood of the incumbent `best` ([`neighborhood`](Self::neighborhood), drawn with `rng`) searched from it
    /// as a unit, for at most `unit_nodes` steps, steered by `control` ([`run_unit`]).
    pub fn unit(&self, best: &Bits<W>, rng: &mut Rng, control: &Arc<Control>) -> Ran<W> {
        run_unit(self.space, control, self.neighborhood(best, rng), best, Some(self.options.unit_nodes))
    }

    /// Searches from the incumbent `start` until `over` says to stop (asked before each unit).  Each unit fixes a
    /// random share of the incumbent's promotions and searches the rest exactly, with every other candidate, for at
    /// most `unit_nodes` steps; a better set becomes the incumbent.  `control` steers the units (stopping it ends the
    /// one in progress); `clock` is the caller's seconds so far, read when a better set is found.
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
            let ran = run(self.neighborhood(&best, rng), &best);
            (units, nodes, finished) = (units + 1, nodes + ran.nodes, finished + u64::from(ran.completed));
            if ran.set.sum() > best.sum() {
                (best, improved, found) = (ran.set, improved + 1, clock());
            }
        }
        Outcome { set: best, nodes, units, finished, improved, found }
    }

    /// Round `round` (counted from 1) of the rounds on many threads (see the module documentation, "On many threads,
    /// alone"), from the incumbent `best`, on `threads` threads: the [`NEIGHBORHOODS`] neighborhoods of `best`, the kth
    /// drawn from [`Options::rng_of`] and searched as a unit ([`unit`](Self::unit)) with a control of its own, since a
    /// control passes the incumbent of one run on to the next.  The answer is the same on any number of threads.
    pub fn round(&self, best: &Bits<W>, round: u64, threads: usize) -> Round<W> {
        let n = self.p.n;
        let ran = in_parallel(NEIGHBORHOODS as usize, threads, |k| {
            self.unit(best, &mut self.options.rng_of(n, round, k as u64), &Control::new())
        });
        // The heaviest, the first of equals: a later set replaces it only if it is strictly heavier.
        let mut out = Round { set: *best, nodes: 0, finished: 0 };
        for ran in ran {
            (out.nodes, out.finished) = (out.nodes + ran.nodes, out.finished + u64::from(ran.completed));
            if ran.set.sum() > out.set.sum() {
                out.set = ran.set;
            }
        }
        out
    }

    /// Rounds 1, 2, ... of the rounds on many threads ([`round`](Self::round)), from the incumbent `start`, on
    /// `threads` threads: at most `most` of them, and none once `enough` says that the incumbent's promotion sum is
    /// enough (asked of the start, and after each round).  The set a round finds becomes the incumbent if it is
    /// heavier, as an [`Improvement`], whose time is the caller's `clock`.  A stop signal ([`engine::stop_signal`])
    /// ends the rounds before the next, or in the middle of one, whose units it stops: then the rounds are
    /// `interrupted`, and the round it stopped is not counted.
    pub fn rounds(
        &self,
        start: &Bits<W>,
        most: u64,
        threads: usize,
        clock: &dyn Fn() -> f64,
        enough: &dyn Fn(u64) -> bool,
    ) -> Rounds<W> {
        let mut out = Rounds { set: *start, rounds: 0, nodes: 0, improvements: Vec::new(), interrupted: false };
        let signaled = || engine::stop_signal().is_some();
        while out.rounds < most && !enough(out.set.sum()) {
            if signaled() {
                out.interrupted = true;
                break;
            }
            let round = out.rounds + 1;
            let ran = self.round(&out.set, round, threads);
            // A signal in the middle of the round stopped some of its units.
            if signaled() {
                out.interrupted = true;
                break;
            }
            (out.rounds, out.nodes) = (round, out.nodes + ran.nodes);
            if ran.set.sum() > out.set.sum() {
                out.set = ran.set;
                out.improvements.push(Improvement { round, sum: ran.set.sum(), seconds: clock() });
            }
        }
        out
    }
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
            let ran = lns.unit(&best, &mut rng, &units);
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

    /// The rounds on many threads on the prepared game `p` from its greedy set: at most `most`, on `threads` threads,
    /// ending once the incumbent reaches `target` if there is one.
    fn rounds_of(p: &Prepared<W>, most: u64, threads: usize, target: Option<u64>) -> Rounds<W> {
        let space = SearchSpace::new(p);
        let lns = Lns { p, space: &space, options: Options::default() };
        lns.rounds(&p.greedy, most, threads, &|| 0.0, &|sum| target.is_some_and(|target| sum >= target))
    }

    /// The game of the tests of the rounds on many threads, whose greedy set its rounds 1 and 2 improve.
    const ROUNDS_GAME: u32 = 400;

    #[test]
    fn the_rounds_on_many_threads_are_the_same_on_any_number_of_threads() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let one = rounds_of(&p, 2, 1, None);
        let rounds: Vec<u64> = one.improvements.iter().map(|improvement| improvement.round).collect();
        assert_eq!(rounds, [1, 2], "each round improves the greedy set: {:?}", one.improvements);
        assert!(p.can_promote(&one.set) && one.set.sum() == one.improvements[1].sum);
        assert_eq!((one.rounds, one.interrupted), (2, false));
        for threads in [2, 4] {
            assert_eq!(rounds_of(&p, 2, threads, None), one, "{threads} threads");
        }
    }

    #[test]
    fn more_rounds_do_the_rounds_of_fewer_first_and_stop_once_they_are_enough() {
        let p = board(ROUNDS_GAME).prepare(SearchTactics::default()).unwrap();
        let (short, long) = (rounds_of(&p, 1, 4, None), rounds_of(&p, 2, 4, None));
        assert!(
            long.improvements.starts_with(&short.improvements),
            "{:?}, {:?}",
            short.improvements,
            long.improvements
        );
        assert!(long.set.sum() > short.set.sum() && !short.improvements.is_empty());
        // Enough at the first improvement: the rounds end with it.
        let first = long.improvements[0];
        let stopped = rounds_of(&p, 2, 4, Some(first.sum));
        assert_eq!((stopped.rounds, stopped.set.sum(), stopped.improvements), (first.round, first.sum, vec![first]));
        // A start that is enough runs no round, and neither do no rounds.
        for none in [rounds_of(&p, 2, 4, Some(0)), rounds_of(&p, 0, 4, None)] {
            assert_eq!((none.set, none.rounds, none.nodes, none.improvements.len()), (p.greedy, 0, 0, 0));
        }
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
