//! The finders: searches for good games that are not sound, and the sound search they are weighed against.  A lower
//! bound on a game's optimum needs only a legal game, so any method may find one; only an upper bound has to be proven.
//! Nothing here proves anything: a finder may miss the optimum, nothing it finds is recorded as a result, and no
//! `results/` file is written.  Every game a finder returns is replayed by the referee ([`replay_finished`]) at the
//! score claimed, and one that does not replay at it, or scores above the known optimum, is an error.  `target`, below,
//! measures what proving an upper bound costs, part of the way and all of it.
//!
//! The commands take the games as `N` or `A-B`, play them one after the other on one thread of one process, and print
//! a JSON line a game as it ends.  A board with nothing to promote has the line `{"n":N,"trivial":true}`.  Every line
//! of `sound` and `finders` begins with the board (`prime`, `in_the_bag`, `candidates`), `upper` (the root's GF(2)
//! bound, as a score), `opt` (the known optimum with `--known FILE`, as for `landscape`, else null), `start` (the score
//! of the game a standalone `solve` starts from, the `lower` of `landscape`) with `start_cpu_s`, and `meets`, whether
//! `start` already reaches `upper`, when there is nothing to find.  The budgets are CPU seconds of the process (the
//! laptop is shared, and the wall time would not be the work), which is why one process plays one game at a time.
//!
//! * **sound**: `wall sound --from start|opt|lns|restricted` runs the sound search of each game standalone (no game of
//!   `n - 1`, so no ceiling and no prime formula), on one thread, from the incumbent `--from` names: `start`; `opt`, the
//!   known optimum, which is what the best possible finder would hand it; or `lns` or `restricted`, the game that finder
//!   returns after `--cpu` seconds from `start`.  `--cap S` stops the search after S CPU seconds (default: it goes on
//!   to the end).  The line adds `from`, the finder's fields if there is one (as below), `search_from` (the score the
//!   search began with) and the search's: `search` (its best score), `search_cpu_s`, `search_nodes` (the engine's
//!   steps), `search_completed` and `search_replay`.
//! * **finders**: for each game, three methods from the same start, each for `--cpu` CPU seconds (default 5).
//!   **lns**, large neighbourhood search: keep a random share of the incumbent's promotions fixed (`--fix LO-HI`, the
//!   share drawn from the range for each unit, default 0.7-0.9), search the rest exactly as a unit (the fixed
//!   promotions and every other candidate), stopping after `--unit-nodes` steps (default 1,024, the engine's block),
//!   take any better game, and repeat (`common/lns.rs`, which the ledger uses too).  **restricted**, the sound search
//!   on a smaller set of candidates, those in the start incumbents of `n - 1`, `n` and `n + 1` and the `--extra`
//!   (default 32) first others by `--pool` (`chosen`, the default: the members of the heaviest set of the root's GF(2)
//!   bound first, then the larger numbers; or `value`: the larger numbers); when it finishes with time left, it takes
//!   twice the others and goes on from its best game.
//!   **control**, the sound search itself from the start for the same CPU time.  The line adds `neighbours_cpu_s` (the
//!   start incumbents of `n - 1` and `n + 1`, which `restricted` needs beside the budget), and for each method, under
//!   its name: its score, `_cpu_s`, `_nodes`, `_completed` (the search finished, so its game is the best one), `_replay`
//!   (the referee's verdict), and its counters: `lns_units` (units run), `lns_finished` (searched to the end within the
//!   limit), `lns_improved` and `lns_found_ms` (the CPU milliseconds at which the last of them was found); `restricted_candidates` (the last set's size) and `restricted_rounds`.
//!
//! `--seed` seeds the random choices (mixed with the board size).
//!
//! * **target**: how cheaply a search proves an upper bound on a game part of the way down from the root's bound, and
//!   what a proof of the whole way costs.  For a game with a known optimum (`--known FILE`; a board with none is an
//!   error), with `root` the root's GF(2) bound and `opt` the optimum, both as promotion sums, and the gap `root - opt`,
//!   it runs on one thread, each for at most `--cap` CPU seconds (default: to the end): a search under the floor
//!   `root - round(k * gap)` ([`search_above`]: an incumbent worth the floor and no set, no start, no seed) for each
//!   share `k` in `--fractions` (default 0.25,0.5,0.75; shares strictly between 0 and 1, so every floor is above the
//!   optimum), in the order of the shares, and then the full proof, the sound search from the known optimum as its seed
//!   (`sound --from opt`).  A floor search that completes has proven that no promotion set is heavier than its floor
//!   (heuristics/THEORY.md, Lemma F).  The line has `prime`, `in_the_bag`, `candidates`, `root`, `opt`, `gap`,
//!   `setup_cpu_s` (the CPU seconds a floor search spends on the matching graph and the search's space before its
//!   engine starts, built once here to time it; the proof's search is run in a space built before) and `runs`, a list
//!   with an object each, floors first and the proof last: `fraction` (1 for the proof), `floor` (the optimum for the
//!   proof), `proof`, `completed`, `held` (a floor search completed and found no heavier set; for the proof, it
//!   completed), `nodes` (the engine's steps) and `cpu_s`.  A floor search that finds a set heavier than the floor, or
//!   a root bound below the optimum, is an error after the line.
//!
//! The command `nogoods` (`nogoods.rs`) is played the same way, on the same [`Case`]; it takes the budgets, the start
//! and the referee from here.

use super::{beyond, chained_of, counters, root_bound, set_up, Line, Options};
use crate::known::known_incumbent;
use crate::lns::{self, run_unit, Lns};
use crate::nogoods;
use std::cmp::Reverse;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use taxman::accelerate::solution_for_promotions;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{Control, EngineConfig};
use taxman::game::{replay_finished, Optimal};
use taxman::matchbound::MatchGraph;
use taxman::search::{search_above, start_incumbent, Prepared, SearchResult, SearchSpace, TaxUnit};

// ---------------------------------------------------------------------------------------------------------
// Options

/// The value of `text` in a table of names (the names of an option's values).
fn named<T: Copy>(text: &str, table: &[(&str, T)]) -> Result<T, String> {
    let names: Vec<&str> = table.iter().map(|&(name, _)| name).collect();
    table
        .iter()
        .find(|&&(name, _)| name == text)
        .map(|&(_, value)| value)
        .ok_or_else(|| format!("{text:?}: {}", names.join(", ")))
}

/// Where the sound search of `wall sound` gets the incumbent it starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The game a standalone `solve` starts from.
    Start,
    /// The known optimum.
    Opt,
    /// What [`Case::lns`] returns from the start.
    Lns,
    /// What [`Case::restricted`] returns from the start.
    Restricted,
}

impl Source {
    const NAMES: [(&'static str, Source); 4] =
        [("start", Source::Start), ("opt", Source::Opt), ("lns", Source::Lns), ("restricted", Source::Restricted)];

    fn name(self) -> &'static str {
        Self::NAMES.iter().find(|&&(_, source)| source == self).map_or("", |&(name, _)| name)
    }
}

impl FromStr for Source {
    type Err = String;

    fn from_str(text: &str) -> Result<Source, String> {
        named(text, &Source::NAMES)
    }
}

/// How [`Case::restricted`] orders the candidates beyond those in the start incumbents of `n - 1`, `n` and `n + 1`
/// (the pool), of which it takes the first `--extra`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolScore {
    /// The larger number first: a promotion is worth its number.
    Value,
    /// The members of the heaviest set of the root's GF(2) bound first (the relaxation's best), then by value.
    Chosen,
}

impl PoolScore {
    const NAMES: [(&'static str, PoolScore); 2] = [("value", PoolScore::Value), ("chosen", PoolScore::Chosen)];
}

impl FromStr for PoolScore {
    type Err = String;

    fn from_str(text: &str) -> Result<PoolScore, String> {
        named(text, &PoolScore::NAMES)
    }
}

/// The options of `sound`, `finders`, `nogoods` and `target`, with their defaults.
#[derive(Clone)]
pub struct Params {
    pub from: Source,
    /// CPU seconds a finder runs for.
    pub cpu: f64,
    /// CPU seconds after which `sound` stops its search, and `target` each of its (none: they go on to the end).
    pub cap: Option<f64>,
    /// The shares of the gap `target` proves down: its floors are this far below the root's bound.
    pub fractions: Vec<f64>,
    /// How [`Case::lns`] searches, and the seed of the finders' random choices (`--seed`, `--fix`, `--unit-nodes`).
    pub lns: lns::Options,
    /// How many candidates beyond the starts' [`Case::restricted`] begins with, and in which order it takes them.
    pub extra: usize,
    pub pool: PoolScore,
    /// The subgradient steps of each bound of `nogoods`, and the steps of the longer fit that shows how far the
    /// bound goes (at least `steps`).
    pub steps: usize,
    pub long: usize,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            from: Source::Start,
            cpu: 5.0,
            cap: None,
            fractions: vec![0.25, 0.5, 0.75],
            lns: lns::Options::default(),
            extra: 32,
            pool: PoolScore::Chosen,
            steps: 200,
            long: 2000,
        }
    }
}

/// A range `LO-HI` of shares of one.
pub fn fix_range(text: &str) -> Result<(f64, f64), String> {
    let bad = |e: &dyn std::fmt::Display| format!("--fix {text}: {e}");
    let (lo, hi) = text.split_once('-').ok_or_else(|| bad(&"LO-HI"))?;
    let (lo, hi): (f64, f64) = (lo.parse().map_err(|e| bad(&e))?, hi.parse().map_err(|e| bad(&e))?);
    if (0.0..=hi).contains(&lo) && hi <= 1.0 {
        Ok((lo, hi))
    } else {
        Err(bad(&"0 <= LO <= HI <= 1"))
    }
}

/// The shares of a list `F,F,...`, strictly between 0 and 1, in increasing order.
pub fn fractions(text: &str) -> Result<Vec<f64>, String> {
    let bad = |e: &dyn std::fmt::Display| format!("--fractions {text}: {e}");
    let mut shares = Vec::new();
    for part in text.split(',') {
        let share: f64 = part.parse().map_err(|e| bad(&e))?;
        if !(share > 0.0 && share < 1.0) {
            return Err(bad(&"each share is above 0 and below 1 (the full proof is always run)"));
        }
        shares.push(share);
    }
    shares.sort_by(f64::total_cmp);
    shares.dedup();
    Ok(shares)
}

// ---------------------------------------------------------------------------------------------------------
// Budgets and runs

/// Elapsed CPU seconds of this process (wall seconds where the system does not say).
pub struct Clock {
    cpu: Option<f64>,
    wall: Instant,
}

impl Clock {
    pub fn start() -> Clock {
        Clock { cpu: counters::cpu_seconds(), wall: Instant::now() }
    }

    pub fn elapsed(&self) -> f64 {
        match (self.cpu, counters::cpu_seconds()) {
            (Some(before), Some(after)) => after - before,
            _ => self.wall.elapsed().as_secs_f64(),
        }
    }
}

/// What a method works under: the control that stops its searches when the CPU budget is up, whether it is, and the
/// CPU clock of the budget.
pub struct Budget {
    pub control: Arc<Control>,
    expired: AtomicBool,
    pub clock: Clock,
}

impl Budget {
    pub fn expired(&self) -> bool {
        self.expired.load(Ordering::Acquire)
    }
}

/// `work` under a budget of `cap` CPU seconds (none: no limit), and the CPU seconds it took.  A watchdog thread stops
/// the budget's control once the budget is spent, which ends the search in progress; it sleeps in between, so it costs
/// nothing.
pub fn budgeted<T>(cap: Option<f64>, work: impl FnOnce(&Budget) -> T) -> (T, f64) {
    let budget = Budget { control: Control::new(), expired: AtomicBool::new(false), clock: Clock::start() };
    let done = AtomicBool::new(false);
    let (budget, done) = (&budget, &done);
    let value = std::thread::scope(|scope| {
        if let Some(cap) = cap {
            scope.spawn(move || {
                while !done.load(Ordering::Acquire) {
                    if budget.clock.elapsed() >= cap {
                        budget.expired.store(true, Ordering::Release);
                        budget.control.stop();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            });
        }
        let value = work(budget);
        done.store(true, Ordering::Release);
        value
    });
    (value, budget.clock.elapsed())
}

// ---------------------------------------------------------------------------------------------------------
// The methods

/// What a method returns, and what it cost.
struct Found<const W: usize> {
    set: Bits<W>,
    cpu_s: f64,
    nodes: u64,
    /// The sound search finished (for a finder, its last search did).
    completed: bool,
    /// The method's own counters.
    notes: Vec<(&'static str, u64)>,
}

/// A game, set up, with what the methods use.
pub struct Case<'a, const W: usize> {
    pub board: &'a Board<W>,
    pub p: &'a Prepared<W>,
    pub space: &'a SearchSpace<'a, W>,
    pub params: &'a Params,
    pub optimal: &'a Optimal,
}

impl<const W: usize> Case<'_, W> {
    pub fn n(&self) -> u32 {
        self.board.n()
    }

    pub fn score(&self, set: &Bits<W>) -> u64 {
        self.board.in_the_bag + set.sum()
    }

    /// The game a standalone `solve` starts from, and the CPU seconds it took.
    pub fn start(&self) -> (Bits<W>, f64) {
        let clock = Clock::start();
        let graph = self.space.match_graph;
        (start_incumbent(self.p, graph, &Control::new(), 1, None), clock.elapsed())
    }

    /// The beginning of the game's line: the board, the root bound (`upper`, as a score), the known optimum and the
    /// start; and whether the start already reaches the root bound (`meets`), when nothing is left to find.
    pub fn line(&self, start: &Bits<W>, start_cpu_s: f64) -> (Line, bool) {
        let bound = self.space.gf2.as_ref().map(|graph| root_bound(self.p, graph).0.value);
        let meets = bound.is_some_and(|bound| bound <= start.sum());
        let mut line = Line::of(self.n());
        line.field("prime", self.board.tables.is_prime(self.n()))
            .field("in_the_bag", self.board.in_the_bag)
            .field("candidates", self.p.candidates.len())
            .maybe("upper", bound.map(|bound| self.board.in_the_bag + bound))
            .maybe("opt", self.optimal.score(self.n()))
            .field("start", self.score(start))
            .seconds("start_cpu_s", start_cpu_s)
            .field("meets", meets);
        (line, meets)
    }

    /// The sound search from `from`, for at most `cap` CPU seconds.
    fn sound(&self, from: &Bits<W>, cap: Option<f64>) -> Found<W> {
        let unit = TaxUnit { promoted: Bits::EMPTY, cands: self.p.candidates };
        let (ran, cpu_s) = budgeted(cap, |budget| run_unit(self.space, &budget.control, unit, from, None));
        Found { set: ran.set, cpu_s, nodes: ran.nodes, completed: ran.completed, notes: Vec::new() }
    }

    /// The search under `floor` ([`search_above`]) on one thread, for at most `cap` CPU seconds, and the CPU seconds it
    /// took, the setup of the search included.
    fn floored(&self, floor: u64, cap: Option<f64>) -> (SearchResult<W>, f64) {
        budgeted(cap, |budget| {
            let cfg =
                EngineConfig { threads: 1, control: Some(Arc::clone(&budget.control)), ..EngineConfig::default() };
            search_above(self.p, &cfg, floor, None, false).expect("the search")
        })
    }

    /// Large neighbourhood search ([`lns`]) from `start`, for `cpu` CPU seconds.
    fn lns(&self, start: &Bits<W>, cpu: f64) -> Found<W> {
        let search = Lns { p: self.p, space: self.space, options: self.params.lns };
        let mut rng = search.options.rng(self.n());
        let (out, cpu_s) = budgeted(Some(cpu), |budget| {
            search.search(start, &mut rng, &budget.control, &|| budget.clock.elapsed(), &|| budget.expired())
        });
        let notes = vec![
            ("units", out.units),
            ("finished", out.finished),
            ("improved", out.improved),
            ("found_ms", (out.found * 1e3) as u64),
        ];
        Found { set: out.set, cpu_s, nodes: out.nodes, completed: false, notes }
    }

    /// The candidates [`restricted`](Self::restricted) begins with: those in the start incumbents of `n - 1`, `n` and
    /// `n + 1`, and the CPU seconds the neighbours' took.
    fn neighbours(&self, start: &Bits<W>) -> (Bits<W>, f64) {
        let clock = Clock::start();
        let mut base = *start;
        for m in [self.n() - 1, self.n() + 1].into_iter().filter(|&m| m >= 2) {
            let game = taxman::with_width!(m, V => chained_of::<V>(m, None), else panic!("{}", beyond(m)));
            // The neighbour's promotions that are numbers of this board (`Board::seed` collects them into a set of this
            // width first, which a number above `n` can overflow).
            let on_board: Vec<u32> = game.set.into_iter().filter(|&x| x <= self.n()).collect();
            base |= self.board.seed(&on_board);
        }
        (base, clock.elapsed())
    }

    /// The candidates outside `base` in the order [`restricted`](Self::restricted) takes them.
    fn pool(&self, base: &Bits<W>) -> Vec<u32> {
        let mut pool = self.p.candidates.difference(base).to_vec();
        match self.params.pool {
            PoolScore::Value => pool.sort_unstable_by_key(|&x| Reverse(x)),
            PoolScore::Chosen => {
                let chosen =
                    self.space.gf2.as_ref().map(|graph| root_bound(self.p, graph).0.chosen).unwrap_or_default();
                pool.sort_unstable_by_key(|&x| (!chosen.contains(x), Reverse(x)));
            }
        }
        pool
    }

    /// The sound search from `start` on a smaller set of candidates, `base` and the `extra` first of the pool, for
    /// `cpu` CPU seconds; when it finishes before then, with twice the extras from its best game.
    fn restricted(&self, start: &Bits<W>, base: &Bits<W>, cpu: f64) -> Found<W> {
        let pool = self.pool(base);
        let ((set, notes, nodes, completed), cpu_s) = budgeted(Some(cpu), |budget| {
            let (mut best, mut nodes, mut rounds, mut extra) = (*start, 0, 0, self.params.extra);
            loop {
                let take = extra.min(pool.len());
                let cands = *base | pool[..take].iter().copied().collect::<Bits<W>>();
                let unit = TaxUnit { promoted: Bits::EMPTY, cands };
                let ran = run_unit(self.space, &budget.control, unit, &best, None);
                (best, nodes, rounds) = (ran.set, nodes + ran.nodes, rounds + 1);
                if budget.expired() || !ran.completed || take == pool.len() {
                    let notes = vec![("candidates", u64::from(cands.len())), ("rounds", rounds)];
                    return (best, notes, nodes, ran.completed && take == pool.len());
                }
                extra *= 2;
            }
        });
        Found { set, cpu_s, nodes, completed, notes }
    }

    /// Whether `set` plays as a legal finished game at the score it claims, by the referee.
    pub fn referee(&self, set: &Bits<W>) -> Result<(), String> {
        let moves = solution_for_promotions::<W>(&self.board.tables, *set);
        let score = replay_finished(self.n(), &moves)?;
        if score == self.score(set) {
            Ok(())
        } else {
            Err(format!("the game replays at {score}, not the {} claimed", self.score(set)))
        }
    }

    /// Adds `found` to `line` under `name` (its score; the other fields are `name_` and their name), and checks it: the
    /// game must replay at its score and not score above the known optimum.  The first fault is returned.
    fn report(&self, line: &mut Line, name: &str, found: &Found<W>, faults: &mut Vec<String>) {
        let referee = self.referee(&found.set);
        let score = self.score(&found.set);
        line.field(name, score)
            .seconds(&format!("{name}_cpu_s"), found.cpu_s)
            .field(&format!("{name}_nodes"), found.nodes)
            .field(&format!("{name}_completed"), found.completed)
            .field(&format!("{name}_replay"), referee.is_ok());
        for (note, value) in &found.notes {
            line.field(&format!("{name}_{note}"), value);
        }
        faults.extend(referee.err().map(|e| format!("n={}: {name}: {e}", self.n())));
        if let Some(opt) = self.optimal.score(self.n()).filter(|&opt| score > opt) {
            faults.push(format!("n={}: {name}: scores {score}, above the known optimum {opt}", self.n()));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The commands

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Sound,
    Finders,
    /// `wall nogoods` (`nogoods.rs`): the nogoods the finder and the search learn, as bounds.
    Nogoods,
    Target,
}

impl Command {
    /// The command of that name, if it is one of these.
    pub fn named(name: &str) -> Option<Command> {
        match name {
            "sound" => Some(Command::Sound),
            "finders" => Some(Command::Finders),
            "nogoods" => Some(Command::Nogoods),
            "target" => Some(Command::Target),
            _ => None,
        }
    }
}

/// What a command made of a game: its line, and the faults found in the games its methods returned.
pub struct Played {
    pub line: Line,
    pub faults: Vec<String>,
}

/// `finders` on a game: the three methods from the same start.
fn finders_of<const W: usize>(case: &Case<'_, W>) -> Played {
    let (start, start_cpu_s) = case.start();
    let (mut line, meets) = case.line(&start, start_cpu_s);
    let mut faults = Vec::new();
    if !meets {
        let cpu = case.params.cpu;
        let (base, neighbours_cpu_s) = case.neighbours(&start);
        line.seconds("neighbours_cpu_s", neighbours_cpu_s);
        case.report(&mut line, "lns", &case.lns(&start, cpu), &mut faults);
        case.report(&mut line, "restricted", &case.restricted(&start, &base, cpu), &mut faults);
        case.report(&mut line, "control", &case.sound(&start, Some(cpu)), &mut faults);
    }
    Played { line, faults }
}

/// `sound` on a game: the sound search from the incumbent `--from` names.
fn sound_of<const W: usize>(case: &Case<'_, W>) -> Result<Played, String> {
    let Params { from, cpu, cap, .. } = *case.params;
    let (start, start_cpu_s) = case.start();
    let mut line = case.line(&start, start_cpu_s).0;
    line.text("from", from.name());
    let mut faults = Vec::new();
    let seed = match from {
        Source::Start => start,
        Source::Opt => {
            known_incumbent(case.board, case.p, case.optimal)?
                .ok_or_else(|| format!("n={}: --from opt needs the known optimum (--known FILE)", case.n()))?
                .0
        }
        Source::Lns | Source::Restricted => {
            let found = match from {
                Source::Lns => case.lns(&start, cpu),
                _ => case.restricted(&start, &case.neighbours(&start).0, cpu),
            };
            case.report(&mut line, from.name(), &found, &mut faults);
            found.set
        }
    };
    line.field("search_from", case.score(&seed));
    case.report(&mut line, "search", &case.sound(&seed, cap), &mut faults);
    Ok(Played { line, faults })
}

/// The floor `share` of the way from `root` down to `opt`: `root` less that share of the gap, rounded.
fn floor_at(root: u64, opt: u64, share: f64) -> u64 {
    let gap = root - opt;
    root - ((share * gap as f64).round() as u64).min(gap)
}

/// `target` on a game: the searches under floors a share of the gap below the root's bound, then the full proof.
fn target_of<const W: usize>(case: &Case<'_, W>) -> Result<Played, String> {
    let n = case.n();
    let (known, opt) = known_incumbent(case.board, case.p, case.optimal)?
        .ok_or_else(|| format!("n={n}: target needs the known optimum (--known FILE)"))?;
    let graph = case.space.gf2.as_ref().ok_or_else(|| format!("n={n}: target needs the GF(2) bound"))?;
    let root = root_bound(case.p, graph).0.value;
    if root < opt {
        return Err(format!("n={n}: the root's bound {root} is below the known optimum {opt}"));
    }
    let cap = case.params.cap;
    let setup = Clock::start();
    let match_graph = MatchGraph::new(&case.p.model, &case.p.root);
    drop(SearchSpace::new(case.p, &match_graph));
    let setup_cpu_s = setup.elapsed();

    let mut faults = Vec::new();
    let mut runs = Vec::new();
    for &fraction in &case.params.fractions {
        let floor = floor_at(root, opt, fraction);
        let (found, cpu_s) = case.floored(floor, cap);
        let (completed, held, nodes) = (found.completed, found.floor_held, found.stats.nodes);
        runs.push(Run { fraction, floor, proof: false, completed, held, nodes, cpu_s });
        if completed && !held {
            faults.push(format!("n={n}: the search under the floor {floor} found a set worth {}", found.sum));
        }
    }
    let proof = case.sound(&known, cap);
    let (completed, nodes, cpu_s) = (proof.completed, proof.nodes, proof.cpu_s);
    runs.push(Run { fraction: 1.0, floor: opt, proof: true, completed, held: completed, nodes, cpu_s });

    let mut line = Line::of(n);
    line.field("prime", case.board.tables.is_prime(n))
        .field("in_the_bag", case.board.in_the_bag)
        .field("candidates", case.p.candidates.len())
        .field("root", root)
        .field("opt", opt)
        .field("gap", root - opt)
        .seconds("setup_cpu_s", setup_cpu_s)
        .field("runs", format!("[{}]", runs.iter().map(Run::render).collect::<Vec<_>>().join(",")));
    Ok(Played { line, faults })
}

/// One search of `target`: under a floor, or the full proof.
struct Run {
    fraction: f64,
    floor: u64,
    proof: bool,
    completed: bool,
    held: bool,
    nodes: u64,
    cpu_s: f64,
}

impl Run {
    /// The run as a JSON object.
    fn render(&self) -> String {
        let mut line = Line::new();
        line.field("fraction", self.fraction)
            .field("floor", self.floor)
            .field("proof", self.proof)
            .field("completed", self.completed)
            .field("held", self.held)
            .field("nodes", self.nodes)
            .seconds("cpu_s", self.cpu_s);
        line.render()
    }
}

/// Game `n` at the set width `W`, played by `command`; a board with nothing to promote has only its line.
fn play<const W: usize>(command: Command, n: u32, o: &Options, optimal: &Optimal) -> Result<Played, String> {
    let (board, prepared) = set_up::<W>(n);
    let Some(p) = prepared else {
        let mut line = Line::of(n);
        line.field("trivial", true);
        return Ok(Played { line, faults: Vec::new() });
    };
    let graph = MatchGraph::new(&p.model, &p.root);
    let mut space = SearchSpace::new(&p, &graph);
    if command == Command::Nogoods {
        // Every nogood a searcher learns is wanted, so none is forgotten (the unit searches of the finder are shorter
        // than the period anyway; the control's table would be cleared every NOGOOD_PERIOD steps).
        space.forgetting.period = None;
    }
    let case = Case { board: &board, p: &p, space: &space, params: &o.finders, optimal };
    match command {
        Command::Sound => sound_of(&case),
        Command::Finders => Ok(finders_of(&case)),
        Command::Nogoods => Ok(nogoods::of(&case)),
        Command::Target => target_of(&case),
    }
}

/// Game `n`, played by `command`, at the set width the solver picks for it.
fn game(command: Command, n: u32, o: &Options, optimal: &Optimal) -> Result<Played, String> {
    taxman::with_width!(n, W => play::<W>(command, n, o, optimal), else Err(beyond(n)))
}

/// `command` on every game of `o`: a line each, printed as its game ends.  A method's game that does not replay at its
/// score, or scores above the known optimum, is an error after its line.
pub fn run(command: Command, o: &Options, optimal: &Optimal) -> Result<(), String> {
    for &n in &o.games {
        let played = game(command, n, o, optimal)?;
        played.line.print();
        if let Some(fault) = played.faults.into_iter().next() {
            return Err(fault);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Tests: `cargo test` runs them (the example is `test = true` in Cargo.toml)

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::json::Json;

    /// The options of a command with finders that run for `cpu` seconds.
    fn options(command: &str, cpu: f64) -> Options {
        let mut o = Options::new(command.to_owned());
        o.finders.cpu = cpu;
        o
    }

    /// The line of game `n` as JSON, played by `command` with the options `o`, with no fault found.
    fn line_of(command: Command, n: u32, o: &Options) -> Json {
        let played = game(command, n, o, Optimal::built_in()).expect("a board the solver supports");
        assert!(played.faults.is_empty(), "{:?}", played.faults);
        Json::parse(&played.line.render()).expect("JSON")
    }

    fn number(line: &Json, name: &str) -> u64 {
        line.get(name).and_then(Json::as_u64).unwrap_or_else(|| panic!("{name} in {line}"))
    }

    fn yes(line: &Json, name: &str) -> bool {
        line.get(name).and_then(Json::as_bool).unwrap_or_else(|| panic!("{name} in {line}"))
    }

    #[test]
    fn the_games_the_finders_return_replay_and_never_score_above_the_optimum() {
        let o = options("finders", 0.3);
        for n in [250, 480, 936] {
            let line = line_of(Command::Finders, n, &o);
            let (opt, start) = (number(&line, "opt"), number(&line, "start"));
            assert!(start <= opt && opt <= number(&line, "upper"), "n={n}: {line}");
            for method in ["lns", "restricted", "control"] {
                let score = number(&line, method);
                assert!(yes(&line, &format!("{method}_replay")), "n={n}: {method}: {line}");
                assert!(start <= score && score <= opt, "n={n}: {method} scores {score}: {line}");
            }
            // The control is the sound search: where it finishes, it has the optimum.
            if yes(&line, "control_completed") {
                assert_eq!(number(&line, "control"), opt, "n={n}: {line}");
            }
        }
    }

    #[test]
    fn the_sound_search_finds_the_optimum_from_every_start_and_the_known_optimum_leaves_nothing_to_find() {
        let mut o = options("sound", 0.3);
        for n in [250, 480] {
            let opt = Optimal::built_in().score(n).expect("a known game");
            for from in [Source::Start, Source::Opt, Source::Lns, Source::Restricted] {
                o.finders.from = from;
                let line = line_of(Command::Sound, n, &o);
                assert!(yes(&line, "search_completed") && yes(&line, "search_replay"), "n={n}: {line}");
                assert_eq!(number(&line, "search"), opt, "n={n}: {from:?}: {line}");
                assert!(number(&line, "search_from") <= opt, "n={n}: {from:?}: {line}");
                if from == Source::Opt {
                    assert_eq!(number(&line, "search_from"), opt, "n={n}: {line}");
                }
            }
        }
    }

    #[test]
    fn the_nogoods_the_searches_learn_fail_to_promote_and_every_bound_over_them_is_at_least_the_optimum() {
        let mut o = options("nogoods", 0.2);
        o.finders.steps = 40;
        o.finders.long = 120;
        for n in [480, 747, 936] {
            let line = line_of(Command::Nogoods, n, &o);
            let (opt, upper) = (number(&line, "opt"), number(&line, "upper"));
            assert_eq!(number(&line, "promotable"), 0, "n={n}: {line}");
            assert!(number(&line, "finder_pool") > 0 && number(&line, "search_pool") > 0, "n={n}: {line}");
            for pool in ["finder", "search", "both"] {
                let (bound, long) = (number(&line, &format!("{pool}_bound")), number(&line, &format!("{pool}_long")));
                assert!(
                    opt <= long && long <= bound && bound <= upper,
                    "n={n}: {pool}: {opt} <= {long} <= {bound} <= {upper}"
                );
            }
            // The pools of the two methods together bound at least as well as either alone, up to the fit.
            assert!(number(&line, "both_pool") >= number(&line, "finder_pool").max(number(&line, "search_pool")));
        }
    }

    #[test]
    fn target_proves_down_the_gap_and_its_full_proof_is_the_search_under_a_floor_at_the_optimum() {
        let o = options("target", 0.0);
        for n in [250, 480] {
            let line = line_of(Command::Target, n, &o);
            let (root, opt, gap) = (number(&line, "root"), number(&line, "opt"), number(&line, "gap"));
            assert_eq!(root - opt, gap, "n={n}: {line}");
            assert!(gap > 0, "n={n}: the root's bound meets the optimum, so there is nothing to prove down: {line}");
            let runs = line.get("runs").and_then(Json::as_arr).expect("runs");
            let shares = [0.25, 0.5, 0.75, 1.0];
            assert_eq!(runs.len(), shares.len(), "n={n}: {line}");
            let mut last = root;
            for (run, share) in runs.iter().zip(shares) {
                let floor = number(run, "floor");
                assert_eq!(run.get("fraction").and_then(Json::as_f64), Some(share), "n={n}: {run}");
                assert_eq!(floor, root - (share * gap as f64).round() as u64, "n={n}: {run}");
                assert!(opt <= floor && floor <= last, "n={n}: the floors fall from the root to the optimum: {run}");
                assert!(yes(run, "completed") && yes(run, "held"), "n={n}: {run}");
                assert_eq!(yes(run, "proof"), share == 1.0, "n={n}: {run}");
                last = floor;
            }
            // The full proof is the search under a floor at the optimum: the same tree, step for step.
            let (_, prepared) = set_up::<16>(n);
            let p = prepared.expect("a board with promotions");
            let cfg = EngineConfig { threads: 1, ..EngineConfig::default() };
            let at_the_optimum = search_above(&p, &cfg, opt, None, false).expect("the search");
            assert_eq!(number(&runs[3], "nodes"), at_the_optimum.stats.nodes, "n={n}: {line}");
        }
    }

    #[test]
    fn target_needs_the_known_optimum() {
        // Game 1301 is not in optimal.json.
        let error = game(Command::Target, 1301, &options("target", 0.0), Optimal::built_in()).err().expect("an error");
        assert!(error.contains("--known"), "{error}");
    }

    #[test]
    fn the_shares_are_read_as_written() {
        assert_eq!(fractions("0.75,0.25,0.5"), Ok(vec![0.25, 0.5, 0.75]));
        assert_eq!(fractions("0.5,0.5"), Ok(vec![0.5]));
        for bad in ["0", "1", "1.5", "-0.1", "", "0.5,", "half", "NaN"] {
            assert!(fractions(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_board_with_nothing_to_promote_has_a_line_of_its_own() {
        let o = options("finders", 0.1);
        let line = line_of(Command::Finders, 2, &o);
        assert!(yes(&line, "trivial"), "{line}");
    }

    #[test]
    fn the_options_are_read_as_written() {
        assert_eq!(fix_range("0.7-0.9"), Ok((0.7, 0.9)));
        assert!(fix_range("0.9-0.7").is_err() && fix_range("0.5").is_err() && fix_range("0.5-1.5").is_err());
        assert_eq!("restricted".parse::<Source>(), Ok(Source::Restricted));
        assert!("opt2".parse::<Source>().is_err());
    }
}
