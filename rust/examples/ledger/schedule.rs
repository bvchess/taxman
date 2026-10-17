//! The scheduler: limited one-thread searches of many boards at once, where a proof looks cheapest, with the
//! relations applied after each.  The module documentation of `main.rs` says what it does and writes.

use crate::book::{facts, Book, Change, Entry, Facts, Lower, Rule, Upper};
use crate::counters;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use taxman::accelerate::solution_for_promotions;
use taxman::board::Board;
use taxman::engine::{self, Control, EngineConfig};
use taxman::game::{replay_finished, Optimal};
use taxman::json::Json;
use taxman::search::{self, dual_bound, SearchTactics};

/// How a run goes.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Searches at once, each on one thread.
    pub workers: usize,
    /// The CPU seconds of the process, over all its runs, after which no search is started.
    pub cpu_budget: f64,
    /// The time limit of a board's first search, in seconds; each search of a board doubles it.
    pub base: f64,
    /// The most any search is given, in seconds.
    pub max_limit: f64,
    /// The time limit of a climb, in seconds: the search of a board whose board below has just been proven, which a
    /// proven game seeds and an exact ceiling caps (0: no climbs).  60 s is where the cloud run's searches that began
    /// with the board below proven cost about 260 core-seconds for each proof added by a longer limit, against the
    /// 1,500 to 2,600 its late proofs cost (the notebook entry has the table).
    pub climb_limit: f64,
    /// The gap that counts as one doubling of the expected cost, in the order of the searches.
    pub gap_unit: f64,
    /// Seconds between writes of the ledger (and at the end).
    pub save_every: f64,
    /// Seconds between a search's checkpoints (a stopped search writes its final one).
    pub checkpoint_every: f64,
    /// Stop on SIGINT or SIGTERM (`engine::stop_on_signals`), as the command does; tests leave the handlers alone.
    pub signals: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            workers: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
            cpu_budget: 3600.0,
            base: 2.0,
            max_limit: 3600.0,
            climb_limit: 60.0,
            gap_unit: 175.0,
            save_every: 30.0,
            checkpoint_every: 60.0,
            signals: false,
        }
    }
}

/// The files of a run's directory.
pub fn ledger_path(dir: &Path) -> PathBuf {
    dir.join("ledger.jsonl")
}

pub fn events_path(dir: &Path) -> PathBuf {
    dir.join("events.jsonl")
}

fn checkpoint_dir(dir: &Path) -> PathBuf {
    dir.join("ckpt")
}

fn checkpoint_path(dir: &Path, n: u32) -> PathBuf {
    checkpoint_dir(dir).join(format!("n{n}.ckpt"))
}

/// What a worker is asked to do with a board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The map's bounds: the search stopped before its first node (the start incumbent, standalone, and the root's
    /// bound).
    Init,
    /// A limited search, resumed from the board's checkpoint if it has one, seeded and capped by the ledger.
    Search,
    /// A climb: a search of a board whose board below is proven, made as a `Search` is but ahead of every other job,
    /// for `climb_limit` seconds.  If it ends unproven the board is parked: it goes by the ordinary order from then on.
    Climb,
}

impl Kind {
    /// The action of the event that ends a job of this kind.
    fn name(self) -> &'static str {
        match self {
            Kind::Init => "init",
            Kind::Search | Kind::Climb => "search",
        }
    }

    /// The `job` of a search event: which kind of search it was.
    fn job(self) -> Option<&'static str> {
        match self {
            Kind::Init => None,
            Kind::Search => Some("ordinary"),
            Kind::Climb => Some("climb"),
        }
    }
}

/// A search to make.
pub struct Job {
    pub n: u32,
    pub kind: Kind,
    pub limit: Duration,
    /// The games whose candidates seed the search: the one that keeps the most is taken.
    pub seeds: Vec<Vec<u32>>,
    /// The ceiling on the score.
    pub cap: Option<u64>,
}

/// What a search gave.
pub struct Attempt {
    pub completed: bool,
    pub reached_cap: bool,
    pub value: u64,
    pub moves: Vec<u32>,
    pub promotions: Vec<u32>,
    /// The dual bound as a score, if the search stopped.
    pub upper: Option<u64>,
    /// The search's steps over all its runs.
    pub nodes: u64,
    pub seconds: f64,
}

/// A search that has ended, with what it was asked, and when it ended (how long its result waited for the main thread
/// is how long its worker did).
pub struct Done {
    pub n: u32,
    pub kind: Kind,
    pub limit: f64,
    pub result: Result<Attempt, String>,
    pub sent: Instant,
}

/// Makes the search of `job` on its board, at the set width the board needs.
fn attempt(job: &Job, control: &Arc<Control>, dir: &Path, every: Duration) -> Result<Attempt, String> {
    let n = job.n;
    taxman::with_width!(n, W => attempt_at::<W>(job, control, dir, every), else Err(format!("n={n} is too large")))
}

fn attempt_at<const W: usize>(
    job: &Job,
    control: &Arc<Control>,
    dir: &Path,
    every: Duration,
) -> Result<Attempt, String> {
    let n = job.n;
    let at = |what: &str, e: String| format!("n={n}: {what}: {e}");
    let board = Board::<W>::for_n(n);
    let p = board.prepare(SearchTactics::default()).map_err(|e| at("setup", e))?;
    let ckpt = checkpoint_path(dir, n);
    if job.kind == Kind::Init {
        engine::remove_checkpoint(&ckpt).map_err(|e| at("an old checkpoint", e.to_string()))?;
    }
    let resume = job.kind != Kind::Init && ckpt.exists();
    let seed = job.seeds.iter().map(|moves| board.seed(moves)).max_by_key(|seed| search::seed_survivors_sum(&p, seed));
    let cap = job.cap.map(|score| score.saturating_sub(board.in_the_bag));
    let cfg = EngineConfig {
        threads: 1,
        progress: None,
        time_limit: Some(job.limit),
        checkpoint: Some((ckpt.clone(), every)),
        control: Some(Arc::clone(control)),
        ..EngineConfig::default()
    };
    let t0 = Instant::now();
    let r = search::search(&p, &cfg, seed.as_ref(), cap, resume).map_err(|e| at("the search", e.to_string()))?;
    let moves = solution_for_promotions(&board.tables, r.promotions);
    let value = replay_finished(n, &moves).map_err(|e| at("the search's game", e))?;
    if value != board.in_the_bag + r.sum {
        return Err(at("the search's game", format!("scores {value}, not {} and {}", board.in_the_bag, r.sum)));
    }
    let upper = match r.completed {
        true => None,
        false => Some(board.in_the_bag + dual_bound(&p, &ckpt).map_err(|e| at("the dual bound", e))?.bound()),
    };
    Ok(Attempt {
        completed: r.completed,
        reached_cap: r.reached_cap,
        value,
        moves,
        promotions: r.promotions.to_vec(),
        upper,
        nodes: r.stats.nodes,
        seconds: t0.elapsed().as_secs_f64(),
    })
}

/// The facts of the boards `ns`, on `workers` threads.
pub fn all_facts(ns: &[u32], workers: usize) -> Result<Vec<Facts>, String> {
    let next = AtomicUsize::new(0);
    let out = Mutex::new(Vec::with_capacity(ns.len()));
    std::thread::scope(|scope| {
        for _ in 0..workers.clamp(1, ns.len().max(1)) {
            scope.spawn(|| {
                while let Some(&n) = ns.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let f = facts(n);
                    out.lock().unwrap_or_else(PoisonError::into_inner).push(f);
                }
            });
        }
    });
    let mut all =
        out.into_inner().unwrap_or_else(PoisonError::into_inner).into_iter().collect::<Result<Vec<_>, _>>()?;
    all.sort_by_key(|f| f.n);
    Ok(all)
}

/// The proven games of the results file `path` in `first - 1..=last`, each checked by the referee.
fn given_games(path: &Path, first: u32, last: u32) -> Result<Vec<(u32, u64, Vec<u32>)>, String> {
    let known = Optimal::from_json(&Json::Arr(Vec::new()))?.with_results(path)?;
    let mut games = Vec::new();
    for (n, score, moves) in known.games().filter(|&(n, _, _)| n + 1 >= first && n <= last) {
        let replayed = replay_finished(n, moves).map_err(|e| format!("{}: n={n}: {e}", path.display()))?;
        if replayed != score {
            return Err(format!("{}: n={n} scores {replayed}, not {score}", path.display()));
        }
        games.push((n, score, moves.to_vec()));
    }
    Ok(games)
}

/// The book of a run: set up from the boards, with what an earlier run's ledger holds and the games given.
pub fn open_book(
    dir: &Path,
    first: u32,
    last: u32,
    proven: Option<&Path>,
    workers: usize,
) -> Result<(Book, Vec<Change>), String> {
    let given = proven.map(|path| given_games(path, first, last)).transpose()?.unwrap_or_default();
    let mut ns: Vec<u32> = (first..=last).collect();
    ns.extend(given.iter().map(|&(n, _, _)| n).filter(|&n| n < first));
    ns.sort_unstable();
    let mut book = Book::new(all_facts(&ns, workers)?);
    let path = ledger_path(dir);
    if path.exists() {
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
            let kept = Entry::from_json(&Json::parse(line).map_err(|e| at(e.to_string()))?).map_err(at)?;
            let entry = book.entries.get_mut(&kept.n).ok_or_else(|| at(format!("n={} is not in this run", kept.n)))?;
            if entry.bag != kept.bag {
                return Err(at(format!("n={} has the bag {}, not {}", kept.n, kept.bag, entry.bag)));
            }
            *entry = kept;
        }
    }
    let mut changes = Vec::new();
    for (n, score, moves) in given {
        let lower = Lower { value: score, moves, promotions: None, rule: Rule::Given, from: None };
        changes.extend(book.offer_lower(n, lower, true)?);
        changes.extend(book.offer_upper(n, Upper { value: score, rule: Rule::Given, from: None }, true)?);
    }
    let bounded: Vec<u32> =
        book.entries.values().filter(|e| e.lower.is_some() || e.upper.is_some()).map(|e| e.n).collect();
    changes.extend(book.propagate(bounded)?);
    Ok((book, changes))
}

/// A search on its way, as the main thread steers it.
pub struct Running {
    pub kind: Kind,
    pub control: Arc<Control>,
}

/// The climbs of this run that have ended: all of them, those whose search proved the board, and those that stopped
/// with it unproven (the board parked).  (A climb whose board a relation proved meanwhile is neither.)
#[derive(Default)]
struct Climbs {
    ended: u32,
    proved: u32,
    parked: u32,
}

/// What the main thread did with its time: it is the one thread that applies results, and while it is busy the workers
/// whose searches ended wait for it.
#[derive(Default)]
struct MainThread {
    /// Seconds applying results, carrying the maps' bounds, and saving.
    busy: f64,
    /// The longest single piece of that.
    longest: f64,
    /// Seconds that results waited for it, in all.
    waited: f64,
}

/// A run of the scheduler.
pub struct Scheduler {
    pub book: Book,
    dir: PathBuf,
    settings: Settings,
    events: BufWriter<File>,
    t0: Instant,
    /// The CPU seconds of the earlier runs, and of this process when the run began.
    cpu_before: f64,
    cpu_at_start: f64,
    pub running: HashMap<u32, Running>,
    /// The boards whose maps came in, whose bounds the relations have not carried to the other boards yet: that is done
    /// once, when the last map is in (`settle`).
    pending: Vec<u32>,
    last_save: Instant,
    last_status: Instant,
    main: MainThread,
    climbs: Climbs,
}

/// The CPU seconds of the earlier runs: the last `cpu` of the events file.
fn earlier_cpu(path: &Path) -> f64 {
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines().rev().find_map(|line| Json::parse(line).ok()?.get("cpu")?.as_f64()).unwrap_or(0.0)
}

fn process_cpu() -> f64 {
    counters::cpu_seconds().unwrap_or(0.0)
}

impl Scheduler {
    /// The scheduler of the boards `first..=last` in `dir`, going on from what the directory holds, with the proven
    /// games of `proven` if given, and the changes of bounds that setting up the book made.  The run's time and CPU
    /// count from here.
    pub fn open(
        first: u32,
        last: u32,
        dir: &Path,
        settings: &Settings,
        proven: Option<&Path>,
    ) -> Result<(Scheduler, Vec<Change>), String> {
        let cpu_before = earlier_cpu(&events_path(dir));
        let (cpu_at_start, t0) = (process_cpu(), Instant::now());
        let (book, changes) = open_book(dir, first, last, proven, settings.workers)?;
        let events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(events_path(dir))
            .map_err(|e| format!("{}: {e}", events_path(dir).display()))?;
        let now = Instant::now();
        let scheduler = Scheduler {
            book,
            dir: dir.to_path_buf(),
            settings: settings.clone(),
            events: BufWriter::new(events),
            t0,
            cpu_before,
            cpu_at_start,
            running: HashMap::new(),
            pending: Vec::new(),
            last_save: now,
            last_status: now,
            main: MainThread::default(),
            climbs: Climbs::default(),
        };
        Ok((scheduler, changes))
    }

    /// The CPU seconds of the run so far, its earlier runs included.
    fn cpu(&self) -> f64 {
        self.cpu_before + (process_cpu() - self.cpu_at_start)
    }

    /// The seconds since the run began and the CPU seconds so far, as an event is stamped.
    fn stamp(&self) -> (f64, f64) {
        (self.t0.elapsed().as_secs_f64(), self.cpu())
    }

    /// Appends an event, with the time and the CPU seconds so far (buffered: `flush_events` writes them out).
    fn event(&mut self, action: &str, n: Option<u32>, fields: Vec<(&str, Json)>) -> Result<(), String> {
        let stamp = self.stamp();
        self.event_at(stamp, action, n, fields)
    }

    fn event_at(
        &mut self,
        (t, cpu): (f64, f64),
        action: &str,
        n: Option<u32>,
        mut fields: Vec<(&str, Json)>,
    ) -> Result<(), String> {
        let mut pairs = vec![
            ("t", Json::from((t * 1000.0).round() / 1000.0)),
            ("cpu", Json::from((cpu * 1000.0).round() / 1000.0)),
            ("action", Json::from(action)),
        ];
        if let Some(n) = n {
            pairs.push(("n", Json::from(n)));
        }
        pairs.append(&mut fields);
        writeln!(self.events, "{}", Json::object(pairs)).map_err(|e| format!("the events file: {e}"))
    }

    fn flush_events(&mut self) -> Result<(), String> {
        self.events.flush().map_err(|e| format!("the events file: {e}"))
    }

    /// Records the changes of bounds that the relations made (all with the one stamp: the thousands of a map's
    /// cascade are made in one go).
    fn log_changes(&mut self, changes: &[Change]) -> Result<(), String> {
        let stamp = self.stamp();
        for c in changes {
            let fields = vec![
                ("side", Json::from(c.side.name())),
                ("rule", Json::from(c.rule.name())),
                ("from", c.from.map_or(Json::Null, Json::from)),
                ("before", c.before.map_or(Json::Null, Json::from)),
                ("after", Json::from(c.after)),
            ];
            self.event_at(stamp, "derive", Some(c.n), fields)?;
        }
        Ok(())
    }

    /// Writes the ledger, one line a board in increasing n, through a temporary file.
    fn save(&mut self) -> Result<(), String> {
        let path = ledger_path(&self.dir);
        let tmp = path.with_extension("jsonl.tmp");
        let mut text = String::new();
        for entry in self.book.entries.values() {
            text.push_str(&entry.to_json(self.book.class_of(entry.n)).to_string());
            text.push('\n');
        }
        fs::write(&tmp, text).and_then(|()| fs::rename(&tmp, &path)).map_err(|e| format!("{}: {e}", path.display()))?;
        self.flush_events()?;
        self.last_save = Instant::now();
        Ok(())
    }

    fn proven(&self) -> usize {
        self.book.entries.values().filter(|e| e.proven()).count()
    }

    /// The boards that may be searched: not proven, not on their way, and the board of their repeat class that is
    /// searched (no prime whose board before it the ledger has).
    fn eligible(&self) -> impl Iterator<Item = &Entry> {
        self.book
            .entries
            .values()
            .filter(|e| !e.proven() && !self.running.contains_key(&e.n) && self.book.searched(e.n))
    }

    fn maps_running(&self) -> bool {
        self.running.values().any(|r| r.kind == Kind::Init)
    }

    /// Whether the maps are still to come: one on its way, or a board to set up (`stopping`: none is set up any
    /// more).  No search starts before they are all in.
    fn mapping(&self, stopping: bool) -> bool {
        self.maps_running() || (!stopping && self.eligible().any(|e| !e.mapped))
    }

    /// The boards to climb to: not proven, not on their way, searched, with the board below them proven (so that a
    /// proven game seeds the search and the ceiling the relations derive caps it) and not parked (a climb to them ended
    /// unproven).  The first unproven, searched board above a proven one is such a board, whatever boards between were
    /// settled by the relations; none if climbs are off.
    fn climbable(&self) -> impl Iterator<Item = &Entry> {
        let on = self.settings.climb_limit > 0.0;
        self.eligible().filter(move |e| on && !e.parked && self.book.entries.get(&(e.n - 1)).is_some_and(Entry::proven))
    }

    /// Whether a climb is on its way to the board below `n`: the climb may prove it within its limit, and then a search
    /// of `n` starts with its game and its exact ceiling, so `n` waits.
    fn held(&self, n: u32) -> bool {
        self.running.get(&(n - 1)).is_some_and(|r| r.kind == Kind::Climb)
    }

    /// The next search to start, if any: a board to set up first (the map's bounds), and only when every map is in and
    /// the relations have carried them (`settle`) a climb, to the board of smallest gap among those to climb to, else
    /// the boards whose bounds a proven neighbour has just changed, smallest gap first, else the board of least `level
    /// + gap / gap_unit`.  Only one board of a repeat class is searched, and no prime whose board before it the ledger
    /// has.  A board whose board below is being climbed to waits (`held`).
    pub fn next_job(&self, cpu: f64) -> Option<Job> {
        let s = &self.settings;
        let key = |e: &Entry| -> (u8, f64, u32) {
            match e.gap() {
                _ if !e.mapped => (0, 0.0, e.n),
                Some(gap) if e.boosted => (1, gap as f64, e.n),
                Some(gap) => (2, f64::from(e.level) + gap as f64 / s.gap_unit, e.n),
                None => unreachable!("a board mapped has both bounds"),
            }
        };
        let ordinary =
            self.eligible().filter(|e| !self.held(e.n)).min_by(|a, b| key(a).partial_cmp(&key(b)).expect("no NaN"))?;
        if !ordinary.mapped {
            let n = ordinary.n;
            return Some(Job { n, kind: Kind::Init, limit: Duration::ZERO, seeds: Vec::new(), cap: None });
        }
        if !self.pending.is_empty() || self.maps_running() {
            return None;
        }
        // A board to climb to is not held (the board below it is proven, so not on its way).
        let (kind, entry, limit) = match self.climbable().min_by_key(|e| (e.gap(), e.n)) {
            Some(entry) => (Kind::Climb, entry, s.climb_limit),
            None => (Kind::Search, ordinary, s.base * 2f64.powi(ordinary.level.min(60) as i32)),
        };
        let n = entry.n;
        let remaining = (s.cpu_budget - cpu) / s.workers as f64;
        let limit = limit.min(s.max_limit).min(remaining.max(1.0));
        let seeds = [n.wrapping_sub(1), n, n + 1]
            .iter()
            .filter_map(|m| self.book.entries.get(m)?.lower.as_ref().map(|l| l.moves.clone()))
            .collect();
        let cap = entry.upper.map(|u| u.value);
        Some(Job { n, kind, limit: Duration::from_secs_f64(limit), seeds, cap })
    }

    /// Starts `job` on a thread of its own.
    fn launch(&mut self, job: Job, tx: Sender<Done>) {
        let control = Control::new();
        self.running.insert(job.n, Running { kind: job.kind, control: Arc::clone(&control) });
        let (dir, every) = (self.dir.clone(), Duration::from_secs_f64(self.settings.checkpoint_every));
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| attempt(&job, &control, &dir, every)))
                .unwrap_or_else(|_| Err(format!("n={}: the search panicked", job.n)));
            let _ = tx.send(Done {
                n: job.n,
                kind: job.kind,
                limit: job.limit.as_secs_f64(),
                result,
                sent: Instant::now(),
            });
        });
    }

    /// Applies what a search gave, and the relations from it (a map's are left for `settle`, when the last map is in).
    pub fn finish(&mut self, done: Done) -> Result<(), String> {
        self.running.remove(&done.n);
        let a = done.result?;
        let n = done.n;
        let bounds = |e: &Entry| (e.lower.as_ref().map(|l| l.value), e.upper.map(|u| u.value));
        let (before, nodes_before) = (bounds(&self.book.entries[&n]), self.book.entries[&n].nodes);
        let was_proven = self.book.entries[&n].proven();
        let (lower_rule, upper_rule) = match done.kind {
            Kind::Init => (Rule::Start, Rule::Root),
            _ if a.completed => (Rule::Search, Rule::Exhausted),
            _ => (Rule::Search, Rule::Dual),
        };
        let lower =
            Lower { value: a.value, moves: a.moves, promotions: Some(a.promotions), rule: lower_rule, from: None };
        let mut changes: Vec<Change> = self.book.offer_lower(n, lower, false)?.into_iter().collect();
        // A search that reached its ceiling proves its game by the bound that was its ceiling, which stands.
        let upper = if a.reached_cap { None } else { Some(a.upper.unwrap_or(a.value)) };
        if let Some(value) = upper {
            changes.extend(self.book.offer_upper(n, Upper { value, rule: upper_rule, from: None }, false)?);
        }
        let entry = self.book.entries.get_mut(&n).expect("a board of the ledger");
        entry.mapped = true;
        if done.kind != Kind::Init {
            entry.attempts += 1;
            entry.search_s += a.seconds;
            entry.nodes = a.nodes;
            entry.boosted = false;
        }
        // A climb leaves the board's level, and so the limit of its next ordinary search, as it was.
        if done.kind == Kind::Search {
            entry.level += 1;
        }
        let after = bounds(&self.book.entries[&n]);
        let mut fields = vec![
            ("limit_s", Json::from(done.limit)),
            ("seconds", Json::from((a.seconds * 1000.0).round() / 1000.0)),
            ("nodes", Json::from(a.nodes.saturating_sub(nodes_before))),
            ("completed", Json::from(a.completed)),
            ("reached_cap", Json::from(a.reached_cap)),
            ("found", Json::from(a.value)),
            ("dual", a.upper.map_or(Json::Null, Json::from)),
        ];
        let pair = |(l, u): (Option<u64>, Option<u64>)| {
            Json::array([l.map_or(Json::Null, Json::from), u.map_or(Json::Null, Json::from)])
        };
        fields.extend([("before", pair(before)), ("after", pair(after))]);
        if let Some(job) = done.kind.job() {
            fields.push(("job", Json::from(job)));
        }
        if done.kind == Kind::Climb {
            fields.push(("outcome", Json::from(self.end_climb(n, was_proven))));
        }
        self.event(done.kind.name(), Some(n), fields)?;
        if done.kind == Kind::Init {
            self.pending.push(n);
        } else {
            let derived = self.book.propagate([n])?;
            self.log_changes(&derived)?;
            changes.extend(derived);
        }
        self.after_changes(&changes)?;
        // A board proven while its search went on (by a relation, which stopped the search) leaves the checkpoint
        // that the stopped search wrote.
        if self.book.entries[&n].proven() {
            let _ = engine::remove_checkpoint(&checkpoint_path(&self.dir, n));
        }
        self.flush_events()
    }

    /// Counts the climb to `n` that has ended, by how the board stands: proved by the climb's search, proven by a
    /// relation while it went on (`was_proven`: the climb was moot), or neither, when the board is parked.  Returns the
    /// outcome's name, as the event has it.
    fn end_climb(&mut self, n: u32, was_proven: bool) -> &'static str {
        let entry = self.book.entries.get_mut(&n).expect("a board of the ledger");
        self.climbs.ended += 1;
        if was_proven {
            "moot"
        } else if entry.proven() {
            self.climbs.proved += 1;
            "proved"
        } else {
            entry.parked = true;
            self.climbs.parked += 1;
            "parked"
        }
    }

    /// Carries the bounds of the maps that came in to the other boards, once for all of them: a map applied to the
    /// boards one at a time, each followed by its cascade through the boards that have no map yet, makes every board
    /// many times over what once does.
    fn settle(&mut self) -> Result<(), String> {
        let t = Instant::now();
        let pending = std::mem::take(&mut self.pending);
        let changes = self.book.propagate(pending.iter().copied())?;
        self.log_changes(&changes)?;
        self.after_changes(&changes)?;
        let fields = vec![
            ("boards", Json::from(pending.len())),
            ("changes", Json::from(changes.len())),
            ("seconds", Json::from((t.elapsed().as_secs_f64() * 1000.0).round() / 1000.0)),
        ];
        self.event("settle", None, fields)?;
        self.flush_events()
    }

    /// What follows changes of bounds: boards changed through a proven neighbour go first; a board that is proven is
    /// recorded, its search stopped and its checkpoint deleted; a search whose ceiling fell is told.
    fn after_changes(&mut self, changes: &[Change]) -> Result<(), String> {
        let cpu = self.cpu();
        let mut seen = Vec::new();
        for c in changes {
            if c.from.is_some() && c.from_proven {
                self.book.entries.get_mut(&c.n).expect("a board of the ledger").boosted = true;
            }
            if !seen.contains(&c.n) {
                seen.push(c.n);
            }
        }
        for n in seen {
            let entry = &self.book.entries[&n];
            let (proven, bag, upper) = (entry.proven(), entry.bag, entry.upper.map(|u| u.value));
            if proven && entry.proven_cpu.is_none() {
                let entry = self.book.entries.get_mut(&n).expect("a board of the ledger");
                entry.proven_cpu = Some(cpu);
                entry.boosted = false;
                let (l, u) = (entry.lower.clone().expect("proven"), entry.upper.expect("proven"));
                let fields = vec![
                    ("value", Json::from(l.value)),
                    ("lower_rule", Json::from(l.rule.name())),
                    ("lower_from", l.from.map_or(Json::Null, Json::from)),
                    ("upper_rule", Json::from(u.rule.name())),
                    ("upper_from", u.from.map_or(Json::Null, Json::from)),
                ];
                self.event("proven", Some(n), fields)?;
            }
            match self.running.get(&n) {
                Some(run) if proven => run.control.stop(),
                Some(run) => {
                    if let Some(cap) = upper.and_then(|u| u.checked_sub(bag)) {
                        run.control.set_ceiling(cap);
                    }
                }
                None if proven => {
                    let _ = engine::remove_checkpoint(&checkpoint_path(&self.dir, n));
                }
                None => {}
            }
        }
        Ok(())
    }

    fn status(&mut self) {
        let (proven, total) = (self.proven(), self.book.entries.len());
        eprintln!(
            "ledger: {:.0} s, {:.0} CPU s of {:.0}: {proven} of {total} proven, {} searching; {} climbs ended, {} proved \
             their board, {} parked; main thread busy {:.1} s, results waited {:.1} s",
            self.t0.elapsed().as_secs_f64(),
            self.cpu(),
            self.settings.cpu_budget,
            self.running.len(),
            self.climbs.ended,
            self.climbs.proved,
            self.climbs.parked,
            self.main.busy,
            self.main.waited,
        );
        self.last_status = Instant::now();
    }

    /// Runs `step`, whose time is the main thread's busy time.
    fn timed<T>(&mut self, step: impl FnOnce(&mut Self) -> T) -> T {
        let t = Instant::now();
        let out = step(self);
        let seconds = t.elapsed().as_secs_f64();
        self.main.busy += seconds;
        self.main.longest = self.main.longest.max(seconds);
        out
    }

    /// Stops the other searches (each writes its checkpoint) and notes the first error, which the run ends with.
    fn fail(&self, failure: &mut Option<String>, error: String) {
        for run in self.running.values() {
            run.control.stop();
        }
        failure.get_or_insert(error);
    }

    /// The loop: start searches while there are workers free, work to do and budget left; apply each result; stop
    /// when every board is proven, the budget is spent, a signal came, or a search failed.  The maps go first, all
    /// of them at once; the relations carry their bounds when the last is in (`settle`), and then the searches start.
    fn run(&mut self) -> Result<String, String> {
        let (tx, rx) = mpsc::channel::<Done>();
        let mut failure: Option<String> = None;
        loop {
            let cpu = self.cpu();
            let mut stopping = failure.is_some() || engine::stop_signal().is_some() || cpu >= self.settings.cpu_budget;
            if failure.is_none() && !self.pending.is_empty() && !self.mapping(stopping) {
                if let Err(e) = self.timed(Scheduler::settle) {
                    self.fail(&mut failure, e);
                    stopping = true;
                }
            }
            while !stopping && self.running.len() < self.settings.workers {
                match self.next_job(cpu) {
                    Some(job) => self.launch(job, tx.clone()),
                    None => break,
                }
            }
            if self.running.is_empty() {
                break;
            }
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(done) => {
                    self.main.waited += done.sent.elapsed().as_secs_f64();
                    if let Err(e) = self.timed(|s| s.finish(done)) {
                        self.fail(&mut failure, e);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Err("the searches' channel closed".into()),
            }
            if self.last_save.elapsed().as_secs_f64() >= self.settings.save_every {
                self.timed(Scheduler::save)?;
            }
            if self.last_status.elapsed().as_secs_f64() >= 60.0 {
                self.status();
            }
        }
        let reason = if let Some(e) = &failure {
            format!("error: {e}")
        } else if self.proven() == self.book.entries.len() {
            "all proven".into()
        } else if engine::stop_signal().is_some() {
            "stopped by a signal".into()
        } else if self.cpu() >= self.settings.cpu_budget {
            "budget spent".into()
        } else {
            "nothing left to search".into()
        };
        let fields = vec![
            ("reason", Json::from(reason.as_str())),
            ("proven", Json::from(self.proven())),
            ("games", Json::from(self.book.entries.len())),
            ("main_s", Json::from((self.main.busy * 1000.0).round() / 1000.0)),
            ("main_longest_s", Json::from((self.main.longest * 1000.0).round() / 1000.0)),
            ("waited_s", Json::from((self.main.waited * 1000.0).round() / 1000.0)),
        ];
        self.event("end", None, fields)?;
        self.save()?;
        self.status();
        match failure {
            Some(e) => Err(e),
            None => Ok(reason),
        }
    }
}

/// Runs the ledger over `first..=last` in `dir` (going on from what it holds), with the proven games of `proven` if
/// given.  Returns why it ended.
pub fn run(first: u32, last: u32, dir: &Path, settings: &Settings, proven: Option<&Path>) -> Result<String, String> {
    fs::create_dir_all(checkpoint_dir(dir)).map_err(|e| format!("{}: {e}", dir.display()))?;
    let lock = engine::lock_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if settings.signals {
        engine::stop_on_signals().map_err(|e| format!("signals: {e}"))?;
    }
    let (mut scheduler, changes) = Scheduler::open(first, last, dir, settings, proven)?;
    let s = settings;
    let fields = vec![
        ("first", Json::from(first)),
        ("last", Json::from(last)),
        ("workers", Json::from(s.workers)),
        ("cpu_budget_s", Json::from(s.cpu_budget)),
        ("base_s", Json::from(s.base)),
        ("max_limit_s", Json::from(s.max_limit)),
        ("climb_limit_s", Json::from(s.climb_limit)),
        ("gap_unit", Json::from(s.gap_unit)),
        ("proven_file", proven.map_or(Json::Null, |p| Json::from(p.display().to_string()))),
    ];
    scheduler.event("start", None, fields)?;
    scheduler.log_changes(&changes)?;
    scheduler.after_changes(&changes)?;
    let reason = scheduler.run();
    drop(lock);
    reason
}

/// The bounds of a board as the ledger file has them, for the report and the tests.
pub fn read_ledger(dir: &Path) -> Result<Vec<Entry>, String> {
    let path = ledger_path(dir);
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
            Entry::from_json(&Json::parse(line).map_err(|e| at(e.to_string()))?).map_err(at)
        })
        .collect()
}
