//! Searches of games on a fixed number of threads, several threads to a game when fewer games are left than threads:
//! the campaign of `exact`.
//!
//! A game's search is a set of *units*, the roles of its open subtrees, each searched depth first
//! ([`Prover::run_units`]).  A game in progress has a *pool*, the units that no thread runs, and its *workers*, the
//! threads that run its units, each with a prover of its own.  Every thread does the first of these that it can:
//!
//! * **Start** the next game not yet started: set it up and become its first worker.  Its pool holds the roles
//!   searched from, or the units its checkpoint saved, and the thread takes the last of them.
//! * **Help**: take the first unit of a pool (the shallowest of a checkpoint's units), from the game with the fewest
//!   workers.
//! * **Ask** the worker whose offer is the largest for a unit, and wait.  A worker asked gives away the largest open
//!   child of the shallowest node it runs that has one ([`Prover::team`]): the largest piece of work it holds, by the
//!   estimated tree of its gap.  A unit that moves starts cold, its first node solving its bounds from scratch, so work
//!   moves only to a thread that would be idle, and in the largest pieces.
//!
//! A worker that ends a unit takes the last unit of its game's pool, and leaves the game when there is none.
//!
//! **No unit is lost.**  At every moment each unit of a game is in one place: its pool, or the run of one worker (the
//! child the worker runs and the open children of the nodes above it), which a pause returns to the pool before the
//! worker leaves.  A worker gives a child away by putting it in the pool, under the lock, and marking it taken in its
//! node, which then never runs it.  The game ends, under the lock, only when no worker runs: proven if its pool is
//! empty, and otherwise paused at the deadline (the pool, with the game's nogoods, saved in its one checkpoint, which
//! a later run resumes on any number of threads) or stopped by a worker (a set beating the target, or the budget).
//!
//! **Nogoods** are facts of the board (minimal unplayable sets), valid in every unit of a game whatever its roles.
//! Every 256 nodes each worker adds those it learned to the game's table, which the checkpoint holds and the game's
//! first worker in a run starts from.  A thread that joins later starts with no nogoods, since a node scans every
//! nogood its prover holds (a thread starting from the table of four threads ran at half the speed); with `share`, it
//! starts from the table, and takes the others' from it every `share`.

use std::cell::Cell;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::game::{Board, Prover, Role, Team, Verdict};
use crate::jsonl::Ledger;

/// A game's search, as its setup gives it.
pub struct Search {
    pub board: Board,
    pub target: u64,
    /// The most nodes of the search, over its threads and runs (u64::MAX: no limit).  A thread joining the game gets
    /// what is left of it after the nodes of the threads that left.
    pub budget: u64,
    /// The roles searched from, when no checkpoint holds open units.
    pub root: Vec<Role>,
    /// Where a pause saves the search, and where a later run resumes it from.
    pub checkpoint: PathBuf,
    /// Whether the provers use the parity bound (the checkpoint counts the nodes searched without it).
    pub parity: bool,
    /// With the Lagrangian bound: how long the search runs before it fits the bound to the nogoods learned
    /// ([`Prover::enable_lagrangian`]); a resumed search fits it at once.  Such a search keeps to one thread.
    pub warmup: Option<Duration>,
    pub prover: MakeProver,
}

/// A thread's prover, given the board and its budget, with its bounds and builders (the campaign adds the deadline,
/// the team and the nogoods).
pub type MakeProver = Box<dyn for<'b> Fn(&'b Board, u64) -> Prover<'b> + Send + Sync>;

/// The game's line from what its search came to (None, with a note, for a paused search).
pub type Finish = Box<dyn FnOnce(&Tally) -> (Option<String>, Option<String>) + Send>;

/// What a game's setup gives: its line at once (a game that needs no search), or its search.
pub enum Plan {
    Line(String),
    Search(Search, Finish),
}

/// What a search came to, over its threads and runs.
#[derive(Clone, Debug)]
pub struct Tally {
    /// Proven, Beaten (a set beating the target), Budget, or Paused (saved in the checkpoint).
    pub verdict: Verdict,
    pub nodes: u64,
    /// Those searched without the parity bound.
    pub matching_nodes: u64,
    /// The wall time, setups included.
    pub secs: f64,
    /// The time of its threads, added up.
    pub thread_secs: f64,
    /// The most threads at once.
    pub threads: usize,
    /// The counters of [`Prover::watch`].
    pub branching: u64,
    pub barring: u64,
    /// This run: the units its workers gave away, and the open units a pause saved.
    pub given: u64,
    pub open: usize,
    /// This run: the sets beating the target that its provers found ([`Prover::found`]; all of them with
    /// [`Prover::enumerate`]).
    pub found: Vec<Vec<u32>>,
}

/// A paused search: its target, its counters so far, its nogoods and its open units (roles as the letters F, P, B).
struct Checkpoint {
    target: u64,
    nodes: u64,
    /// None in a checkpoint written before the parity bound existed: all its nodes were searched without it.
    matching_nodes: Option<u64>,
    secs: f64,
    thread_secs: f64,
    threads: usize,
    /// The counters of [`Prover::watch`] so far (0 in a checkpoint written before they existed).
    branching: u64,
    barring: u64,
    nogoods: Vec<Vec<u32>>,
    units: Vec<Vec<Role>>,
}

impl Checkpoint {
    fn new(target: u64) -> Checkpoint {
        Checkpoint {
            target,
            nodes: 0,
            matching_nodes: Some(0),
            secs: 0.0,
            thread_secs: 0.0,
            threads: 0,
            branching: 0,
            barring: 0,
            nogoods: Vec::new(),
            units: Vec::new(),
        }
    }

    fn save(&self, path: &Path) {
        let mut text = format!("target {}\nnodes {}\nsecs {}\n", self.target, self.nodes, self.secs);
        text += &format!("thread_secs {}\nthreads {}\n", self.thread_secs, self.threads);
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

    /// The checkpoint at `path`, if there is one.  One written before the threads were counted ran on one thread.
    fn load(path: &Path) -> Option<Checkpoint> {
        let text = fs::read_to_string(path).ok()?;
        let mut cp = Checkpoint::new(0);
        cp.matching_nodes = None;
        let (mut thread_secs, mut threads) = (None, 1);
        for line in text.lines() {
            let (key, rest) = line.split_once(' ')?;
            match key {
                "target" => cp.target = rest.parse().ok()?,
                "nodes" => cp.nodes = rest.parse().ok()?,
                "matching_nodes" => cp.matching_nodes = Some(rest.parse().ok()?),
                "secs" => cp.secs = rest.parse().ok()?,
                "thread_secs" => thread_secs = Some(rest.parse().ok()?),
                "threads" => threads = rest.parse().ok()?,
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
        cp.thread_secs = thread_secs.unwrap_or(cp.secs);
        cp.threads = threads;
        Some(cp)
    }
}

/// A game in progress: its search, and what its threads share without the lock.
struct Game {
    n: u64,
    search: Search,
    /// Whether the search resumed from a checkpoint.
    resumed: bool,
    /// When its setup began in this run.
    started: Instant,
    /// Set by the worker that ends the search early (a set beating the target, the budget): the others stop.
    halt: AtomicBool,
}

impl Game {
    fn halted(&self) -> bool {
        self.halt.load(Ordering::Relaxed)
    }

    /// Whether threads other than its first may work on it.
    fn shared(&self) -> bool {
        self.search.warmup.is_none()
    }
}

/// A game's state under the lock.
struct Pool {
    game: Arc<Game>,
    finish: Option<Finish>,
    /// The units no thread runs: a worker takes the last, a thread joining the game the first.
    units: VecDeque<Vec<Role>>,
    /// The threads working on it, and the most at once in this run.
    running: usize,
    peak: usize,
    /// The game's nogoods, each with the thread that added it (None: the checkpoint), each once, at most `cap`.
    nogoods: Vec<(Option<usize>, Vec<u32>)>,
    seen: HashSet<Vec<u32>>,
    cap: usize,
    /// The counters of the earlier runs (the checkpoint's, its nogoods and units taken out).
    earlier: Checkpoint,
    /// This run's, of the threads that left the game.
    nodes: u64,
    thread_secs: f64,
    branching: u64,
    barring: u64,
    given: u64,
    found: Vec<Vec<u32>>,
    /// How a worker ended the search early.
    end: Option<Verdict>,
}

/// A worker, as the idle threads see it.
struct Slot {
    game: Arc<Game>,
    /// Set by an idle thread that waits for a unit from it; cleared when it gives one, or has none.
    asked: AtomicBool,
    /// The estimated tree of the unit it would give, as the bits of an f64.
    offer: AtomicU64,
}

impl Slot {
    fn offer(&self) -> f64 {
        f64::from_bits(self.offer.load(Ordering::Relaxed))
    }
}

struct State {
    /// The next game to start, and the games being set up.
    next: usize,
    starting: usize,
    pools: Vec<Pool>,
    slots: Vec<Arc<Slot>>,
}

impl State {
    fn pool(&mut self, game: &Game) -> &mut Pool {
        self.pools.iter_mut().find(|p| p.game.n == game.n).expect("the game's pool")
    }
}

/// What a thread does next.
enum Job {
    Start(u64),
    Unit(Arc<Game>, Vec<Role>),
    Done,
}

/// The threads' common state: one lock, and the condition idle threads wait on.
struct Hub {
    deadline: Instant,
    share: Option<Duration>,
    state: Mutex<State>,
    wake: Condvar,
    /// Set when a thread panics (where a panic unwinds, as in the tests): the other threads stop instead of waiting
    /// for its game, which can then never end.
    failed: AtomicBool,
}

/// Sets [`Hub::failed`] when its thread unwinds.
struct Alarm<'h>(&'h Hub);

impl Drop for Alarm<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.failed.store(true, Ordering::Relaxed);
            self.0.wake.notify_all();
        }
    }
}

/// Searches the games of `items` not yet recorded in `out` on `threads` threads, appending each game's line as it
/// ends ([`Ledger`]); starts no game and no unit after `seconds`, when every search pauses.  `start(n)` sets game n up.
/// With `share`, a thread joining a game starts from the game's nogoods, and takes those the game's other workers
/// learned every `share`.
pub fn campaign<F>(
    name: &str,
    items: &[u64],
    threads: usize,
    seconds: f64,
    out: &Path,
    share: Option<Duration>,
    start: F,
) where
    F: Fn(u64) -> Plan + Sync,
{
    assert!(threads >= 1, "at least one thread");
    let (ledger, todo) = Ledger::open(name, items, out);
    let state = State { next: 0, starting: 0, pools: Vec::new(), slots: Vec::new() };
    let hub = Hub {
        deadline: ledger.deadline(seconds),
        share,
        state: Mutex::new(state),
        wake: Condvar::new(),
        failed: AtomicBool::new(false),
    };
    std::thread::scope(|scope| {
        for me in 0..threads {
            let (hub, todo, start, ledger) = (&hub, &todo, &start, &ledger);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(scope, move || hub.thread(me, todo, start, ledger))
                .expect("spawn a worker");
        }
    });
    ledger.close();
}

impl Hub {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("the campaign's lock")
    }

    /// Thread `me`: jobs until none is left or the deadline passes.
    fn thread<F: Fn(u64) -> Plan>(&self, me: usize, todo: &[u64], start: &F, ledger: &Ledger) {
        let _alarm = Alarm(self);
        let mut asked = None;
        loop {
            match self.next_job(todo, &mut asked) {
                Job::Done => return,
                Job::Start(n) => {
                    let begun = Instant::now();
                    match start(n) {
                        Plan::Line(line) => {
                            ledger.report(n, Some(line), None);
                            self.lock().starting -= 1;
                            self.wake.notify_all();
                        }
                        Plan::Search(search, finish) => {
                            let (game, unit) = self.open(n, search, finish, begun);
                            self.serve(me, &game, unit, (begun, true), ledger);
                        }
                    }
                }
                Job::Unit(game, unit) => self.serve(me, &game, unit, (Instant::now(), false), ledger),
            }
        }
    }

    /// The next job: a game to start, a unit to help with, or none left.  Otherwise asks a worker for a unit (unless
    /// the last ask is still waiting to be served) and waits.
    fn next_job(&self, todo: &[u64], asked: &mut Option<Arc<Slot>>) -> Job {
        let mut st = self.lock();
        loop {
            let now = Instant::now();
            let job = if now >= self.deadline || self.failed.load(Ordering::Relaxed) {
                Some(Job::Done)
            } else if st.next < todo.len() {
                st.next += 1;
                st.starting += 1;
                Some(Job::Start(todo[st.next - 1]))
            } else if let Some(pool) = st
                .pools
                .iter_mut()
                .filter(|p| !p.units.is_empty() && p.game.shared() && !p.game.halted())
                .min_by_key(|p| p.running)
            {
                let unit = pool.units.pop_front().expect("a unit");
                pool.running += 1;
                pool.peak = pool.peak.max(pool.running);
                Some(Job::Unit(pool.game.clone(), unit))
            } else if st.pools.is_empty() && st.starting == 0 {
                Some(Job::Done)
            } else {
                None
            };
            if let Some(job) = job {
                // A thread that found work withdraws its ask (a unit given for it all the same goes to the next).
                if let Some(slot) = asked.take() {
                    slot.asked.store(false, Ordering::Relaxed);
                }
                return job;
            }
            if asked.as_ref().is_none_or(|slot| !slot.asked.load(Ordering::Relaxed)) {
                *asked = st
                    .slots
                    .iter()
                    .filter(|s| s.game.shared() && !s.asked.load(Ordering::Relaxed) && s.offer() > 0.0)
                    .max_by(|a, b| a.offer().total_cmp(&b.offer()))
                    .cloned();
                if let Some(slot) = asked {
                    slot.asked.store(true, Ordering::Relaxed);
                }
            }
            let wait = (self.deadline - now).min(Duration::from_millis(20));
            st = self.wake.wait_timeout(st, wait).expect("the campaign's lock").0;
        }
    }

    /// A game set up: its pool, with the checkpoint's units or the roles searched from, and the unit its first worker
    /// takes (the last).
    fn open(&self, n: u64, search: Search, finish: Finish, begun: Instant) -> (Arc<Game>, Vec<Role>) {
        let (mut earlier, resumed) = match Checkpoint::load(&search.checkpoint) {
            Some(cp) => {
                assert_eq!(cp.target, search.target, "n={n}: the checkpoint is for another target");
                (cp, true)
            }
            None => {
                let mut cp = Checkpoint::new(search.target);
                cp.units.push(search.root.clone());
                (cp, false)
            }
        };
        let mut units: VecDeque<Vec<Role>> = std::mem::take(&mut earlier.units).into();
        let unit = units.pop_back().unwrap_or_else(|| panic!("n={n}: the checkpoint has no open unit"));
        let nogoods = std::mem::take(&mut earlier.nogoods);
        let game = Arc::new(Game { n, search, resumed, started: begun, halt: AtomicBool::new(false) });
        let pool = Pool {
            game: game.clone(),
            finish: Some(finish),
            units,
            running: 1,
            peak: 1,
            seen: nogoods.iter().cloned().collect(),
            nogoods: nogoods.into_iter().map(|k| (None, k)).collect(),
            cap: usize::MAX,
            earlier,
            nodes: 0,
            thread_secs: 0.0,
            branching: 0,
            barring: 0,
            given: 0,
            found: Vec::new(),
            end: None,
        };
        {
            let mut st = self.lock();
            st.starting -= 1;
            st.pools.push(pool);
        }
        self.wake.notify_all();
        (game, unit)
    }

    /// Thread `me` works on `game`, from `unit` until its pool is empty or the search stops, with a prover of its own;
    /// its time on the game counts from `since`.  The game's first worker in a run starts from the game's nogoods, as
    /// one thread resumes from its own; a thread that joins later starts from them only with `share`, since every
    /// nogood a node holds costs it time, and another thread's are of less use to it.  The last worker to leave ends
    /// the game.
    fn serve(
        &self,
        me: usize,
        game: &Arc<Game>,
        mut unit: Vec<Role>,
        (since, first): (Instant, bool),
        ledger: &Ledger,
    ) {
        let search = &game.search;
        let slot = Arc::new(Slot { game: game.clone(), asked: AtomicBool::new(false), offer: AtomicU64::new(0) });
        let (nogoods, cursor, budget) = {
            let mut st = self.lock();
            let pool = st.pool(game);
            let nogoods: Vec<Vec<u32>> = if first || self.share.is_some() {
                pool.nogoods.iter().map(|(_, k)| k.clone()).collect()
            } else {
                Vec::new()
            };
            (nogoods, pool.nogoods.len(), search.budget.saturating_sub(pool.earlier.nodes + pool.nodes))
        };
        let share = self.share.unwrap_or(Duration::ZERO);
        let link = Link { hub: self, game, slot: &slot, me, cursor: Cell::new(cursor), next: Cell::new(since + share) };
        // A search with the Lagrangian bound runs to the end of its warm-up, then fits the bound (at once if resumed).
        let mut warming = search.warmup.filter(|_| !game.resumed);
        let pause_at = warming.map_or(self.deadline, |w| (Instant::now() + w).min(self.deadline));
        let mut prover = (search.prover)(&search.board, budget).pause_at(pause_at).team(&link, nogoods);
        if search.warmup.is_some() && game.resumed {
            prover.enable_lagrangian(200);
        }
        {
            let mut st = self.lock();
            st.pool(game).cap = prover.max_nogoods();
            st.slots.push(slot.clone());
        }
        self.wake.notify_all();
        loop {
            let verdict = prover.run_units(vec![unit]);
            if verdict == Verdict::Paused && warming.is_some() && Instant::now() < self.deadline {
                warming = None;
                let mut units = prover.take_units(self.deadline);
                prover.enable_lagrangian(200);
                unit = units.pop().expect("a paused search has an open unit");
                self.lock().pool(game).units.extend(units);
                continue;
            }
            let mut st = self.lock();
            let pool = st.pool(game);
            match verdict {
                Verdict::Proven => {
                    if !game.halted() && Instant::now() < self.deadline {
                        if let Some(next) = pool.units.pop_back() {
                            unit = next;
                            continue;
                        }
                    }
                }
                Verdict::Paused => pool.units.extend(prover.take_units(self.deadline)),
                end => {
                    pool.end.get_or_insert(end);
                    game.halt.store(true, Ordering::Relaxed);
                }
            }
            break;
        }
        // The nogoods learned since the last trade go to the game's table, for the checkpoint and later threads.
        prover.trade();
        let (branching, barring) = prover.watch.as_ref().map_or((0, 0), |w| (w.branching, w.barring));
        let ended = {
            let mut st = self.lock();
            st.slots.retain(|s| !Arc::ptr_eq(s, &slot));
            slot.asked.store(false, Ordering::Relaxed);
            let pool = st.pool(game);
            pool.running -= 1;
            pool.nodes += prover.nodes;
            pool.thread_secs += since.elapsed().as_secs_f64();
            pool.branching += branching;
            pool.barring += barring;
            pool.found.append(&mut prover.found);
            // A worker leaves with units in the pool only at the deadline or when the search stops.
            debug_assert!(
                pool.running > 0 || pool.units.is_empty() || game.halted() || Instant::now() >= self.deadline,
                "n={}: the last worker left open units",
                game.n
            );
            let last = pool.running == 0;
            last.then(|| {
                let at = st.pools.iter().position(|p| p.game.n == game.n).expect("the game's pool");
                st.pools.swap_remove(at)
            })
        };
        self.wake.notify_all();
        if let Some(pool) = ended {
            self.end(pool, ledger);
        }
    }

    /// The game is over for this run: its tally, its checkpoint if it paused, and its line or note.
    fn end(&self, mut pool: Pool, ledger: &Ledger) {
        let game = pool.game.clone();
        let paused = if pool.units.is_empty() { Verdict::Proven } else { Verdict::Paused };
        let earlier = &pool.earlier;
        let tally = Tally {
            verdict: pool.end.take().unwrap_or(paused),
            nodes: earlier.nodes + pool.nodes,
            matching_nodes: earlier.matching_nodes.unwrap_or(earlier.nodes)
                + if game.search.parity { 0 } else { pool.nodes },
            secs: earlier.secs + game.started.elapsed().as_secs_f64(),
            thread_secs: earlier.thread_secs + pool.thread_secs,
            threads: earlier.threads.max(pool.peak),
            branching: earlier.branching + pool.branching,
            barring: earlier.barring + pool.barring,
            given: pool.given,
            open: pool.units.len(),
            found: std::mem::take(&mut pool.found),
        };
        if tally.verdict == Verdict::Paused {
            let cp = Checkpoint {
                target: game.search.target,
                nodes: tally.nodes,
                matching_nodes: Some(tally.matching_nodes),
                secs: tally.secs,
                thread_secs: tally.thread_secs,
                threads: tally.threads,
                branching: tally.branching,
                barring: tally.barring,
                nogoods: pool.nogoods.into_iter().map(|(_, k)| k).collect(),
                units: pool.units.into(),
            };
            cp.save(&game.search.checkpoint);
        }
        let (line, note) = (pool.finish.take().expect("a game ends once"))(&tally);
        let recorded = line.is_some();
        ledger.report(game.n, line, note);
        // Only once its line is synced: a game stopped in between is searched again rather than lost.
        if recorded {
            let _ = fs::remove_file(&game.search.checkpoint);
        }
    }
}

/// A worker's link to its game's other threads ([`Prover::team`]).
struct Link<'h> {
    hub: &'h Hub,
    game: &'h Game,
    slot: &'h Slot,
    me: usize,
    /// How far into the game's table the worker has read, and when it next takes the others' nogoods.
    cursor: Cell<usize>,
    next: Cell<Instant>,
}

impl Team for Link<'_> {
    fn halted(&self) -> bool {
        self.game.halted() || self.hub.failed.load(Ordering::Relaxed)
    }

    fn asked(&self) -> bool {
        self.slot.asked.load(Ordering::Relaxed)
    }

    fn give(&self, unit: Option<Vec<Role>>) {
        {
            let mut st = self.hub.lock();
            match unit {
                Some(unit) => {
                    let pool = st.pool(self.game);
                    pool.units.push_front(unit);
                    pool.given += 1;
                }
                // Before the asking thread wakes, so that it does not ask again.
                None => self.offer(0.0),
            }
            self.slot.asked.store(false, Ordering::Relaxed);
        }
        self.hub.wake.notify_all();
    }

    fn offer(&self, size: f64) {
        self.slot.offer.store(size.to_bits(), Ordering::Relaxed);
    }

    fn trade(&self, learned: &[Vec<u32>]) -> Vec<Vec<u32>> {
        let now = Instant::now();
        let take = self.hub.share.is_some() && now >= self.next.get();
        if learned.is_empty() && !take {
            return Vec::new();
        }
        let mut st = self.hub.lock();
        let pool = st.pool(self.game);
        for k in learned {
            if pool.nogoods.len() >= pool.cap {
                break;
            }
            if pool.seen.insert(k.clone()) {
                pool.nogoods.push((Some(self.me), k.clone()));
            }
        }
        if !take {
            return Vec::new();
        }
        self.next.set(now + self.hub.share.unwrap_or_default());
        let fresh = pool.nogoods[self.cursor.get()..].iter().filter(|(who, _)| *who != Some(self.me));
        let fresh = fresh.map(|(_, k)| k.clone()).collect();
        self.cursor.set(pool.nogoods.len());
        fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::prime_below;
    use crate::jsonl::{committed_chain, recorded_games_to};
    use crate::primes::smallest_prime_factors;

    /// How a test makes a thread's prover: from the board, the target and the budget.
    type Make = for<'b> fn(&'b Board, u64, u64) -> Prover<'b>;

    /// The committed chain's score of game n.
    fn opt(n: u32) -> u64 {
        recorded_games_to(&[committed_chain()], u64::from(n))[&u64::from(n)].0
    }

    /// An empty directory of the test's own.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("taxman_team_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A prover with the parity bound.
    fn with_parity(board: &Board, target: u64, budget: u64) -> Prover<'_> {
        Prover::new(board, target, budget).with_parity()
    }

    /// A prover that lists every playable set beating the target (with the Franklín–Moniot bound alone, whose nodes
    /// are cheap).
    fn listing(board: &Board, target: u64, budget: u64) -> Prover<'_> {
        Prover::new(board, target, budget).enumerate(usize::MAX)
    }

    /// One run of a campaign over game n alone, with the provers `make` makes, on `threads` threads for `seconds`, in
    /// `dir`: its tally.  The search resumes the checkpoint in `dir` if there is one, else starts from `units` (saved
    /// as a checkpoint), else from the root.
    fn run(n: u32, target: u64, units: &[Vec<Role>], threads: usize, seconds: f64, dir: &Path, make: Make) -> Tally {
        let checkpoint = dir.join("game.ckpt");
        if !units.is_empty() && !checkpoint.exists() {
            let mut cp = Checkpoint::new(target);
            cp.units = units.to_vec();
            cp.save(&checkpoint);
        }
        let out = dir.join("out.jsonl");
        let _ = fs::remove_file(&out);
        let spf = smallest_prime_factors(n as usize);
        let tallies = Arc::new(Mutex::new(Vec::new()));
        campaign("test", &[u64::from(n)], threads, seconds, &out, None, |_| {
            let board = Board::new(n, &spf);
            let root = Prover::new(&board, target, 0).root_roles();
            let prover: MakeProver = Box::new(move |board, budget| make(board, target, budget));
            let search = Search {
                board,
                target,
                budget: u64::MAX,
                root,
                checkpoint: checkpoint.clone(),
                parity: true,
                warmup: None,
                prover,
            };
            let tallies = tallies.clone();
            Plan::Search(
                search,
                Box::new(move |tally: &Tally| {
                    tallies.lock().unwrap().push(tally.clone());
                    ((tally.verdict != Verdict::Paused).then(|| format!("{{\"n\":{n}}}")), None)
                }),
            )
        });
        let tally = tallies.lock().unwrap().pop().expect("the game's tally");
        tally
    }

    /// Whether `picks` are playable on board n and worth `worth`.
    fn worth(n: u32, picks: &[u32], worth: u64) -> bool {
        let board = Board::new(n, &smallest_prime_factors(n as usize));
        let mut pick = vec![false; n as usize + 1];
        picks.iter().for_each(|&c| pick[c as usize] = true);
        board.playable(&pick) && picks.iter().map(|&c| u64::from(c)).sum::<u64>() == worth
    }

    /// A game searched on several threads reaches the verdict of one thread: at opt(n) nothing beats it, and one below
    /// it, a playable set worth opt(n) does.  The threads help (units are given away).  Game 600 takes 295 nodes.
    #[test]
    fn several_threads_reach_the_verdict_of_one() {
        let n = 600;
        let v = opt(n);
        let dir = scratch("verdict");
        for threads in [1, 3] {
            let tally = run(n, v, &[], threads, 1e9, &dir, with_parity);
            assert_eq!(tally.verdict, Verdict::Proven, "{threads} threads");
            assert_eq!(tally.given > 0, threads > 1, "{threads} threads: {tally:?}");
            match run(n, v - 1, &[], threads, 1e9, &dir, with_parity).verdict {
                Verdict::Beaten(picks) => assert!(worth(n, &picks, v), "{threads} threads: {picks:?}"),
                other => panic!("{threads} threads: {other:?} below opt(n)"),
            }
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A search paused on several threads, and resumed on another number of threads again and again, loses no unit
    /// and runs none twice: listing every playable set that beats a target below opt(n), it finds each set that one
    /// thread without a pause finds, once.  Game 350 has 20 sets worth opt(n) − 9 or more, in 3,571 nodes.
    #[test]
    fn pauses_and_resumes_on_other_numbers_of_threads_lose_no_unit() {
        let (n, below) = (350, 10);
        let target = opt(n) - below;
        let board = Board::new(n, &smallest_prime_factors(n as usize));
        let mut straight = listing(&board, target, u64::MAX);
        assert_eq!(straight.run(), Verdict::Proven);
        let mut expected = straight.found;
        expected.sort();
        let dir = scratch("resume");
        let (mut found, mut pauses, mut given) = (Vec::new(), 0, 0);
        for threads in [3, 1, 2, 4].into_iter().cycle().take(100) {
            let mut tally = run(n, target, &[], threads, 0.05, &dir, listing);
            found.append(&mut tally.found);
            given += tally.given;
            if tally.verdict != Verdict::Paused {
                assert_eq!(tally.verdict, Verdict::Proven);
                assert!(tally.threads > 1, "{tally:?}");
                break;
            }
            pauses += 1;
        }
        found.sort();
        assert!(expected.len() > 10, "{} sets", expected.len());
        assert_eq!(found.len(), expected.len(), "sets found, against one thread's");
        assert!(found == expected, "the sets found differ from one thread's");
        assert!(pauses >= 3 && given > 0, "{pauses} pauses, {given} units given");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A set beating the target that a thread other than the game's first finds ends the search, and is its verdict.
    /// The first thread takes the last units, where no set beats opt(n) − 1 (every optimal set holds the largest
    /// prime, F4, and they bar it; 231 nodes each at 600), and the second the first unit, which forces it.
    #[test]
    fn a_set_that_another_thread_finds_is_reported() {
        let n = 600;
        let v = opt(n);
        let spf = smallest_prime_factors(n as usize);
        let board = Board::new(n, &spf);
        let p = prime_below(u64::from(n), &spf) as usize;
        let mut units = vec![Prover::new(&board, v, 0).root_roles(); 5];
        units[0][p] = Role::Pick;
        units[1..].iter_mut().for_each(|u| u[p] = Role::Barred);
        let dir = scratch("beaten");
        let tally = run(n, v - 1, &units, 2, 1e9, &dir, with_parity);
        match &tally.verdict {
            Verdict::Beaten(picks) => assert!(worth(n, picks, v), "{picks:?}"),
            other => panic!("{other:?} below opt(n)"),
        }
        assert_eq!(tally.threads, 2);
        fs::remove_dir_all(&dir).unwrap();
    }
}
