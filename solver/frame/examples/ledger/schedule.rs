//! The scheduler: limited one-thread searches of many boards at once, where a proof looks cheapest, with the
//! relations applied after each.  The module documentation of `main.rs` says what it does and writes.

use crate::book::{facts, Book, Change, Entry, Facts, Lower, Rule, Side, Stalled, Step, Upper};
use crate::cost::{self, Plan};
use crate::counters;
use crate::known::survivors;
use crate::lns::{self, Lns};
use crate::pool;
use crate::store::{Line, Store};
use crate::verify::root_upper;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Control, EngineConfig};
use taxman::game::{replay_finished, Optimal};
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::search::{self, dual_bound, Prepared, SearchSpace, SearchTactics};
pub use taxman::store::{play, refereed, Played};

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
    /// The time limit of a climb, in seconds: the search of a board next to a proven one, which that game seeds (0: no
    /// climbs).  60 s: on the cloud run's boards a climb of 60 s on each would have cost 142 core-seconds a proof, and
    /// the proofs a longer limit adds cost 310 (60 to 90 s), 420 (to 120 s), 870 (to 180 s) against the 1,500 to 2,600 of
    /// the run's late ones (solver/notebook/search/the-ledger-bounds-from-both-sides-2026-10/climb_limit.txt).
    pub climb_limit: f64,
    /// The ways climbs go from a proven board: to the board above (up) and to the board below (down).
    pub climb_up: bool,
    pub climb_down: bool,
    /// The seconds of one thread that the finder (large neighborhood search, `search/finder.rs`) is given after a board's
    /// map and before a search whose best seed has changed since it last ran on the board (0: none).  Its game is
    /// offered as the board's lower bound and seeds the search if it is the better.  1 s: from the finders'
    /// measurements (solver/notebook/search/finders-for-good-games-that-are-not-sound-2026-10/README.md) it has most of
    /// its gain in the first second, and a search seeded with it costs 0.48 of the CPU of the unseeded, the finder
    /// included.
    pub lns_seconds: f64,
    /// The points a floor step goes down below the board's upper bound, at the median slope of the cost of a search
    /// under a floor (a tenfold in cost; a steeper board's line takes fewer points).  A board whose gap is more than
    /// two steps is stepped down, each step a search under a floor (`cost.rs`), before its proof, and the boards are
    /// ordered by the forecast cost of their proofs.  150: the first step of the measurements (10,000 nodes prove down
    /// 514 points).  0: no floor steps, the ordinary searches whose limit doubles, ordered by `level + gap / gap_unit`.
    pub floor_step: u64,
    /// A floor step's limit is this many times its forecast time (at least `base`), and doubles each time the step is
    /// stopped by it; the line fitted from a board's steps is within a factor of 2 of the next step's cost from the
    /// second step on (the 14 games' measurements), so 4 leaves room.
    pub floor_slack: f64,
    /// The gap that counts as one doubling of the expected cost, in the order of the searches with no floor steps.
    pub gap_unit: f64,
    /// Seconds between writes of the ledger (and at the end).
    pub save_every: f64,
    /// Seconds between a search's checkpoints (a stopped search writes its final one).
    pub checkpoint_every: f64,
    /// Stop on SIGINT or SIGTERM (`engine::stop_on_signals`), as the command does; tests leave the handlers alone.
    pub signals: bool,
    /// A bounds store (results/bounds.jsonl) whose best bounds the run starts from, for the boards of its stretch.
    pub bounds: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            workers: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
            cpu_budget: 3600.0,
            base: 2.0,
            max_limit: 3600.0,
            climb_limit: 60.0,
            climb_up: true,
            climb_down: true,
            lns_seconds: 1.0,
            floor_step: 150,
            floor_slack: 4.0,
            gap_unit: 175.0,
            save_every: 30.0,
            checkpoint_every: 60.0,
            signals: false,
            bounds: None,
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

/// The checkpoint of the board's ordinary searches and climbs.
fn checkpoint_path(dir: &Path, n: u32) -> PathBuf {
    checkpoint_dir(dir).join(format!("n{n}.ckpt"))
}

/// The checkpoint of the board's floor step that its limit stopped: good only under that floor, so it is kept beside
/// the other, whose search it does not continue.
fn floor_checkpoint_path(dir: &Path, n: u32) -> PathBuf {
    checkpoint_dir(dir).join(format!("n{n}.floor.ckpt"))
}

/// Deletes both checkpoints of the board `n` (the first error, if any, after trying both).
fn remove_checkpoints(dir: &Path, n: u32) -> std::io::Result<()> {
    engine::remove_checkpoint(&checkpoint_path(dir, n)).and(engine::remove_checkpoint(&floor_checkpoint_path(dir, n)))
}

/// Which way a climb goes from a proven board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    /// To the board above: the proven game seeds the search, and the ceiling opt(n) <= opt(n − 1) + n, which the
    /// relations have derived and is exact in about a third of the boards, is its ceiling.
    Up,
    /// To the board below: the proven game seeds the search (the game below it that Lemma L2 makes, its moves less n
    /// and then completed, is exactly optimal for n − 1 in about 35% of the composite pairs), and the ceiling is the
    /// ledger's, which is loose (opt(n − 1) <= opt(n)).
    Down,
}

impl Way {
    fn name(self) -> &'static str {
        match self {
            Way::Up => "up",
            Way::Down => "down",
        }
    }

    /// The board just outside the repeat class `class` (ascending) that a climb to it in this way goes from, whose
    /// proof starts it: the board below the class's smallest for a climb up, the board above its largest for a climb
    /// down.
    fn source(self, class: &[u32]) -> u32 {
        match self {
            Way::Up => outside(class)[0],
            Way::Down => outside(class)[1],
        }
    }
}

/// The two boards just outside the repeat class `class` (ascending): below its smallest and above its largest.  A
/// proven one starts a climb into the class, and its game seeds the class's searches; the other waits while a climb
/// into the class runs.
fn outside(class: &[u32]) -> [u32; 2] {
    [class[0] - 1, class[class.len() - 1] + 1]
}

/// What a worker is asked to do with a board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The map's bounds: the search stopped before its first node (the start incumbent, standalone, and the root's
    /// bound).
    Init,
    /// A limited search, resumed from the board's checkpoint if it has one, seeded and capped by the ledger.
    Search,
    /// A climb: a search of a board next to a proven one, made as a `Search` is but ahead of every other job, for
    /// `climb_limit` seconds.  If it ends unproven the board is parked: it goes by the ordinary order from then on.
    Climb(Way),
    /// A floor step: a search under a floor below the board's upper bound, with no seed (`search::search_above`),
    /// resumed from its own checkpoint if the step was stopped before.  If it completes having found nothing, the floor
    /// is the new upper bound.
    Floor,
}

impl Kind {
    /// The action of the event that ends a job of this kind.
    fn name(self) -> &'static str {
        match self {
            Kind::Init => "init",
            Kind::Search | Kind::Climb(_) | Kind::Floor => "search",
        }
    }

    /// The `job` of a search event: which kind of search it was.
    fn job(self) -> Option<&'static str> {
        match self {
            Kind::Init => None,
            Kind::Search => Some("ordinary"),
            Kind::Climb(_) => Some("climb"),
            Kind::Floor => Some("floor"),
        }
    }

    fn way(self) -> Option<Way> {
        match self {
            Kind::Climb(way) => Some(way),
            Kind::Init | Kind::Search | Kind::Floor => None,
        }
    }
}

/// The floor a step runs under (`Kind::Floor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Floor {
    /// The floor, as a score (the promotion sum is that less the board's bag).
    pub score: u64,
    /// Whether the step goes on from the checkpoint of a stalled one under this floor (else it starts, and an old
    /// checkpoint is deleted).
    pub resume: bool,
}

/// A game that can seed a search: the moves of the lower bound of the board `from`, which is n − 1, n or n + 1.
pub struct Seed {
    pub from: u32,
    pub moves: Vec<u32>,
}

/// Which game seeded a search, as its event says: the lower bound of the board below it (`n-1`), its own (`n`), the
/// board above's (`n+1`), or the game the finder found (`lns`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedFrom {
    Below,
    Own,
    Above,
    Lns,
}

impl SeedFrom {
    /// The board `from`, as seen from the board `n`.
    fn of(from: u32, n: u32) -> SeedFrom {
        match from.cmp(&n) {
            std::cmp::Ordering::Less => SeedFrom::Below,
            std::cmp::Ordering::Equal => SeedFrom::Own,
            std::cmp::Ordering::Greater => SeedFrom::Above,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SeedFrom::Below => "n-1",
            SeedFrom::Own => "n",
            SeedFrom::Above => "n+1",
            SeedFrom::Lns => "lns",
        }
    }
}

/// What the finder (large neighborhood search) is asked to do in a job.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LnsPlan {
    /// Seconds of one thread (0: it does not run).
    pub seconds: f64,
    /// The score of the best seed the board had when the finder last ran on it, its own game included: it runs before
    /// a search only if the best seed is another score (or the job is a climb).  None: it has not run on the board.
    pub last: Option<u64>,
    /// The number of searches the board has had, so that the finder's random choices differ from run to run.
    pub round: u32,
}

/// A search to make.
pub struct Job {
    pub n: u32,
    pub kind: Kind,
    pub limit: Duration,
    /// The games whose candidates seed the search: the one that keeps the most is taken.
    pub seeds: Vec<Seed>,
    /// The ceiling on the score.
    pub cap: Option<u64>,
    /// Whether the boards n − 1 and n + 1 were proven when the job started, which its event says.
    pub below_proven: bool,
    pub above_proven: bool,
    pub lns: LnsPlan,
    /// The floor of a floor step.
    pub floor: Option<Floor>,
}

/// What the finder did: the scores of the game it began from and of the game it found (the same if it found nothing
/// better), the seconds it took, and the game, which the referee has replayed at `found`.
pub struct LnsRun {
    pub from: u64,
    pub found: u64,
    pub seconds: f64,
    pub moves: Vec<u32>,
    pub promotions: Vec<u32>,
}

impl LnsRun {
    pub fn improved(&self) -> bool {
        self.found > self.from
    }
}

/// What a search gave.
pub struct Attempt {
    pub completed: bool,
    pub reached_cap: bool,
    /// A floor step completed having found no set: no promotion set weighs more than its floor.  Then the game is the
    /// one with no promotions.
    pub held: bool,
    pub value: u64,
    pub moves: Vec<u32>,
    pub promotions: Vec<u32>,
    /// The dual bound as a score, if an ordinary search or a climb stopped.
    pub upper: Option<u64>,
    /// The search's steps over all its runs (those of its checkpoint's, for a floor step the stalled step's).
    pub nodes: u64,
    /// The seconds of the search (of the map, for a map), the finder's not among them.
    pub seconds: f64,
    /// What the finder did before the search (after the map, for a map), if it ran.
    pub lns: Option<LnsRun>,
    /// Which game seeded the search (none if nothing did).
    pub seed_from: Option<SeedFrom>,
}

/// A search that has ended, with what it was asked, and when it ended (how long its result waited for the main thread
/// is how long its worker did).
pub struct Done {
    pub n: u32,
    pub kind: Kind,
    pub limit: f64,
    pub below_proven: bool,
    pub above_proven: bool,
    /// The floor a floor step ran under, as a score.
    pub floor: Option<u64>,
    pub result: Result<Attempt, String>,
    pub sent: Instant,
}

/// The finder on `board` from the promotions `start`, for the seconds of the job's plan (less if `stop` is stopped or
/// the process is told to stop), with the referee's replay of its game: the promotions of the game it found, and what
/// it did.
fn find<const W: usize>(
    job: &Job,
    board: &Board<W>,
    p: &Prepared<W>,
    start: &Bits<W>,
    stop: &Control,
) -> Result<(Bits<W>, LnsRun), String> {
    let all = Instant::now();
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace::new(p, &graph);
    let finder =
        Lns { p, space: &space, options: lns::Options { seed: u64::from(job.lns.round), ..lns::Options::default() } };
    let began = Instant::now();
    let seconds = || began.elapsed().as_secs_f64();
    let over = || seconds() >= job.lns.seconds || stop.stopped() || engine::stop_signal().is_some();
    let out = finder.search(start, &mut finder.options.rng(job.n), &Control::new(), &seconds, &over);
    let (found, moves) = refereed(board, &out.set).map_err(|e| format!("n={}: the finder's game: {e}", job.n))?;
    let run = LnsRun {
        from: board.in_the_bag + start.sum(),
        found,
        seconds: all.elapsed().as_secs_f64(),
        moves,
        promotions: out.set.to_vec(),
    };
    Ok((out.set, run))
}

/// Makes the search of `job` on its board, at the set width the board needs.
pub fn attempt(job: &Job, control: &Arc<Control>, dir: &Path, every: Duration) -> Result<Attempt, String> {
    let n = job.n;
    taxman::with_width!(n, W => attempt_at::<W>(job, control, dir, every), else Err(format!("n={n} is too large")))
}

/// The search of `job`, with the finder before it or, for a map, after it (`LnsPlan`): the finder starts from the best
/// seed (or the map's start incumbent), and its game seeds the search if it beats the seed.  A floor step has neither
/// (`step_at`).
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
    if job.kind == Kind::Init {
        remove_checkpoints(dir, n).map_err(|e| at("an old checkpoint", e.to_string()))?;
    }
    let cap = job.cap.map(|score| score.saturating_sub(board.in_the_bag));
    let engine_config = |ckpt: &Path| EngineConfig {
        threads: 1,
        progress: None,
        time_limit: Some(job.limit),
        checkpoint: Some((ckpt.to_path_buf(), every)),
        control: Some(Arc::clone(control)),
        ..EngineConfig::default()
    };
    if let Some(floor) = job.floor {
        return step_at(job, floor, &board, &p, &engine_config(&floor_checkpoint_path(dir, n)), cap);
    }
    let ckpt = checkpoint_path(dir, n);
    let resume = job.kind != Kind::Init && ckpt.exists();
    // The best seed: the game that keeps the most promotions (the last of equals), and what survives of it.
    let best = job
        .seeds
        .iter()
        .map(|seed| (SeedFrom::of(seed.from, n), board.seed(&seed.moves)))
        .map(|(from, seed)| (from, search::seed_survivors_sum(&p, &seed), seed))
        .max_by_key(|&(_, kept, _)| kept);
    let (mut seed, mut lns) = (best.as_ref().map(|(from, _, seed)| (*from, *seed)), None);
    if job.lns.seconds > 0.0 && job.kind != Kind::Init {
        let start = match &best {
            Some((_, _, seed)) => survivors(&p, seed).map_err(|e| at("the seed", e))?,
            None => p.greedy,
        };
        let score = board.in_the_bag + start.sum();
        // Nothing to find if the seed reaches the ceiling; a climb always has a new seed (a proven neighbor's game).
        let changed = matches!(job.kind, Kind::Climb(_)) || job.lns.last != Some(score);
        if changed && !cap.is_some_and(|cap| start.sum() >= cap) {
            let (found, run) = find(job, &board, &p, &start, control)?;
            if run.improved() {
                seed = Some((SeedFrom::Lns, found));
            }
            lns = Some(run);
        }
    }
    let t0 = Instant::now();
    let seed_set = seed.as_ref().map(|(_, set)| set);
    let r = search::search(&p, &engine_config(&ckpt), seed_set, cap, resume)
        .map_err(|e| at("the search", e.to_string()))?;
    let (value, moves) = refereed(&board, &r.promotions).map_err(|e| at("the search's game", e))?;
    let upper = match r.completed {
        true => None,
        false => Some(board.in_the_bag + dual_bound(&p, &ckpt).map_err(|e| at("the dual bound", e))?.bound()),
    };
    let seconds = t0.elapsed().as_secs_f64();
    // A map is followed by the finder, from the start incumbent, unless that reaches the root's bound.
    if job.lns.seconds > 0.0 && job.kind == Kind::Init && upper.is_some_and(|upper| value < upper) {
        lns = Some(find(job, &board, &p, &r.promotions, control)?.1);
    }
    Ok(Attempt {
        completed: r.completed,
        reached_cap: r.reached_cap,
        held: false,
        value,
        moves,
        promotions: r.promotions.to_vec(),
        upper,
        nodes: r.stats.nodes,
        seconds,
        lns,
        seed_from: seed.map(|(from, _)| from),
    })
}

/// A floor step: the search under `floor` (`search::search_above`), from the checkpoint in `cfg` if the step was
/// stopped before and is resumed, else from scratch (an old checkpoint is deleted).  There is no seed and no finder:
/// the floor is the start, and a set found above it is a game like any other, which the referee replays.  A stopped
/// step gives no bound (Lemma D would give its dual bound, but the open units of a depth-first search are the root's
/// and near it).
fn step_at<const W: usize>(
    job: &Job,
    floor: Floor,
    board: &Board<W>,
    p: &Prepared<W>,
    cfg: &EngineConfig,
    cap: Option<u64>,
) -> Result<Attempt, String> {
    let at = |what: &str, e: String| format!("n={}: {what}: {e}", job.n);
    let ckpt = &cfg.checkpoint.as_ref().expect("a floor step checkpoints").0;
    let resume = floor.resume && ckpt.exists();
    if !resume {
        engine::remove_checkpoint(ckpt).map_err(|e| at("an old checkpoint", e.to_string()))?;
    }
    let t0 = Instant::now();
    let r = search::search_above(p, cfg, floor.score.saturating_sub(board.in_the_bag), cap, resume)
        .map_err(|e| at("the search under the floor", e.to_string()))?;
    let (value, moves) = refereed(board, &r.promotions).map_err(|e| at("the step's game", e))?;
    Ok(Attempt {
        completed: r.completed,
        reached_cap: r.reached_cap,
        // At the ceiling the step ended without exploring its tree.
        held: r.floor_held && !r.reached_cap,
        value,
        moves,
        promotions: r.promotions.to_vec(),
        upper: None,
        nodes: r.stats.nodes,
        seconds: t0.elapsed().as_secs_f64(),
        lns: None,
        seed_from: None,
    })
}

/// The facts of the boards `ns`, on `workers` threads.
pub fn all_facts(ns: &[u32], workers: usize) -> Result<Vec<Facts>, String> {
    let mut all = pool::map(ns, workers, |&n| facts(n)).into_iter().collect::<Result<Vec<_>, _>>()?;
    all.sort_by_key(|f| f.n);
    Ok(all)
}

/// The games of the lower bounds `lines` of a store, in the order of the file, each replayed by the referee at its
/// value: the promotion sets on `workers` threads, then, in order, the lines that cite the game they are carried from
/// (by a relation: `carry_game`), each of the game of the earlier line it cites.
pub fn play_all(lines: &[&Line], workers: usize) -> Vec<Result<Played, String>> {
    let direct = pool::map(lines, workers, |&line| line.game.is_some().then(|| play(line)));
    let mut index: HashMap<(u32, u64), usize> = HashMap::new();
    let mut done: Vec<Result<Played, String>> = Vec::with_capacity(lines.len());
    for (i, (line, direct)) in lines.iter().zip(direct).enumerate() {
        let played = direct.unwrap_or_else(|| carried(line, &index, &done));
        index.insert((line.n, line.value), i);
        done.push(played);
    }
    done
}

/// The game of `line`, which cites its source, carried from it and replayed at the line's value.
fn carried(line: &Line, index: &HashMap<(u32, u64), usize>, done: &[Result<Played, String>]) -> Result<Played, String> {
    let (from, value) = line.from.zip(line.from_value).ok_or("it has no game and cites none")?;
    let at = index
        .get(&(from, value))
        .ok_or_else(|| format!("it cites a game of board {from} worth {value}, which no line before it has"))?;
    let source = done[*at].as_ref().map_err(|e| format!("the game it cites (board {from}, {value}): {e}"))?;
    taxman::store::carried(line, &source.moves)
}

/// Offers the book the best bounds of the store `path` for the boards `first..=last`: each lower bound with its game,
/// played by the referee (with the games it cites), and each upper bound as the store has it (`ledger verify-bounds`
/// checks them: this reads the store as given, as the run reads a results file it is given).  A board with both
/// bounds there needs no map.
fn start_from(book: &mut Book, path: &Path, first: u32, last: u32, workers: usize) -> Result<Vec<Change>, String> {
    let store = Store::read(path)?;
    let best = store.best();
    let lower_lines = || store.lines.iter().enumerate().filter(|(_, line)| line.side == Side::Lower);
    let index: HashMap<(u32, u64), usize> = lower_lines().map(|(i, line)| ((line.n, line.value), i)).collect();
    // The lines to play: the best of the boards, and the lines they cite, and so on.
    let mut needed = BTreeSet::new();
    let mut pending: Vec<usize> =
        best.lower.range(first..=last).map(|(_, line)| index[&(line.n, line.value)]).collect();
    while let Some(i) = pending.pop() {
        let line = &store.lines[i];
        if needed.insert(i) && line.game.is_none() {
            pending.extend(line.from.zip(line.from_value).and_then(|cited| index.get(&cited)));
        }
    }
    let positions: Vec<usize> = needed.into_iter().collect();
    let lines: Vec<&Line> = positions.iter().map(|&i| &store.lines[i]).collect();
    let mut games: Vec<Option<Result<Played, String>>> = play_all(&lines, workers).into_iter().map(Some).collect();
    let mut changes = Vec::new();
    for (_, line) in best.lower.range(first..=last) {
        let at = positions.binary_search(&index[&(line.n, line.value)]).expect("a line to play");
        let played = games[at].take().expect("a game played once");
        let Played { moves, promotions } =
            played.map_err(|e| format!("{}: n={}: the game: {e}", path.display(), line.n))?;
        let lower = Lower { value: line.value, moves, promotions, rule: line.rule, from: line.from };
        changes.extend(book.offer_lower(line.n, lower, false)?);
    }
    for (&n, line) in best.upper.range(first..=last) {
        changes.extend(book.offer_upper(n, Upper { value: line.value, rule: line.rule, from: line.from }, false)?);
        if best.lower.contains_key(&n) {
            book.entries.get_mut(&n).expect("a board of the stretch").mapped = true;
        }
    }
    Ok(changes)
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

/// The book of a run: set up from the boards, with what an earlier run's ledger holds, the games given and the best
/// bounds of the store `bounds`, if the run has one.
pub fn open_book(
    dir: &Path,
    first: u32,
    last: u32,
    proven: Option<&Path>,
    bounds: Option<&Path>,
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
    if let Some(path) = bounds {
        changes.extend(start_from(&mut book, path, first, last, workers)?);
    }
    // The root's bound of every board that will be searched and is mapped without one: a ledger from before the floor
    // steps does not have it, nor does a board the store mapped.  (A fresh board gets it from its map.)
    let rootless: Vec<u32> = book
        .entries
        .values()
        .filter(|e| e.mapped && e.root.is_none() && !e.proven() && book.searched(e.n))
        .map(|e| e.n)
        .collect();
    for (n, root) in rootless.iter().zip(pool::map(&rootless, workers, |&n| root_upper(n))) {
        book.entries.get_mut(n).expect("a board of the ledger").root = Some(root?);
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
struct Tally {
    ended: u32,
    proved: u32,
    parked: u32,
}

impl std::fmt::Display for Tally {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ended ({} proved their board, {} parked)", self.ended, self.proved, self.parked)
    }
}

/// The tallies of the climbs in each way.
#[derive(Default)]
struct Climbs {
    up: Tally,
    down: Tally,
}

impl Climbs {
    fn of(&mut self, way: Way) -> &mut Tally {
        match way {
            Way::Up => &mut self.up,
            Way::Down => &mut self.down,
        }
    }
}

/// The floor steps of this run that have ended: all of them, those that completed having found nothing (the floor is
/// the new upper bound), those that completed having found a set (the board is proven), those that stopped with a set
/// (the floor is below the lower bound now: the step is dropped), and those the limit stopped with nothing (the step
/// is kept).  (A step whose board a relation proved meanwhile, or whose floor the upper bound fell to, is none of them.)
#[derive(Default)]
struct StepTally {
    ended: u32,
    held: u32,
    proved: u32,
    found: u32,
    stalled: u32,
}

impl std::fmt::Display for StepTally {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ended ({} lowered U, {} proved their board, {} found a set, {} stalled)",
            self.ended, self.held, self.proved, self.found, self.stalled
        )
    }
}

/// The steps and the seconds of the searches of this run, from which a board with no measure of its own takes its rate.
#[derive(Default)]
struct Pooled {
    nodes: u64,
    seconds: f64,
}

impl Pooled {
    /// The floor steps that the ledger records, from which a run that goes on from it starts.
    fn of(book: &Book) -> Pooled {
        book.entries.values().map(cost::spent).fold(Pooled::default(), |pooled, (nodes, seconds)| Pooled {
            nodes: pooled.nodes + nodes,
            seconds: pooled.seconds + seconds,
        })
    }

    /// The steps a thread makes in a second, if the searches took a while to measure it (else 0).
    fn rate(&self) -> f64 {
        if self.seconds >= 10.0 {
            self.nodes as f64 / self.seconds
        } else {
            0.0
        }
    }
}

/// How a floor step ended, for `Scheduler::end_step`: its floor (a score), whether it completed having found nothing
/// (`held`), whether it ended at the ceiling, whether it found a set, and its steps and seconds over this run of it.
struct StepEnd {
    floor: u64,
    held: bool,
    capped: bool,
    found: bool,
    nodes: u64,
    seconds: f64,
}

/// The runs of the finder in this run, and how many of them found a better game than the one they began from.
#[derive(Default)]
struct LnsTally {
    runs: u32,
    improved: u32,
}

impl LnsTally {
    fn add(&mut self, run: &LnsRun) {
        self.runs += 1;
        self.improved += u32::from(run.improved());
    }
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
    steps: StepTally,
    pooled: Pooled,
    lns: LnsTally,
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
        let (book, changes) = open_book(dir, first, last, proven, settings.bounds.as_deref(), settings.workers)?;
        let events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(events_path(dir))
            .map_err(|e| format!("{}: {e}", events_path(dir).display()))?;
        let now = Instant::now();
        let pooled = Pooled::of(&book);
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
            steps: StepTally::default(),
            pooled,
            lns: LnsTally::default(),
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

    /// The next climb, if any, and its way: to the board of smallest gap among those not proven, not on their way,
    /// searched, not parked (a climb to them ended unproven) and next to a proven board, the board just below their
    /// repeat class (up) or just above it (down); the climbs up first.  The first unproven, searched board above (or
    /// below) a proven one is such a board, whatever boards between were settled by the relations (a board unproven but
    /// not searched stops the climb until its own board is proven).  None if climbs are off.
    fn next_climb(&self) -> Option<(Way, &Entry)> {
        let s = &self.settings;
        if s.climb_limit <= 0.0 {
            return None;
        }
        [(s.climb_up, Way::Up), (s.climb_down, Way::Down)].into_iter().filter(|&(on, _)| on).find_map(|(_, way)| {
            let next_to_proven = |e: &&Entry| {
                !e.parked && self.book.entries.get(&way.source(&self.book.class(e.n))).is_some_and(Entry::proven)
            };
            self.eligible().filter(next_to_proven).min_by_key(|e| (e.gap(), e.n)).map(|e| (way, e))
        })
    }

    /// The boards that wait while a climb runs: those just outside the repeat class of a board being climbed to.  One
    /// of them is the proven board the climb goes from; the other may be proven within the climb's limit, and then it
    /// is climbed to with its game (and, going up, its exact ceiling), so it has no ordinary search meanwhile.
    fn held(&self) -> HashSet<u32> {
        let climbing = self.running.iter().filter(|(_, run)| matches!(run.kind, Kind::Climb(_)));
        climbing.flat_map(|(&n, _)| outside(&self.book.class(n))).collect()
    }

    /// The cost the order of the ordinary searches goes by, the least first: the log of the forecast seconds of the
    /// board's proof (`cost.rs`), with the boards that have yet to give their line two points ahead of all (they cost
    /// a second or so, `cost::exploring`), or, with no floor steps, `level + gap / gap_unit`.
    fn cost(&self, e: &Entry, gap: i64) -> f64 {
        let s = &self.settings;
        if s.floor_step == 0 {
            return f64::from(e.level) + gap as f64 / s.gap_unit;
        }
        if cost::exploring(e, s.floor_step) {
            return f64::NEG_INFINITY;
        }
        cost::forecast(e, self.pooled.rate()).map_or(f64::INFINITY, |forecast| forecast.log_seconds)
    }

    /// The next search to start, if any: a board to set up first (the map's bounds), and only when every map is in and
    /// the relations have carried them (`settle`) a climb (`next_climb`), else the boards whose bounds a proven
    /// neighbor has just changed, smallest gap first, else the board whose proof is forecast to cost the least
    /// (`cost`).  Only one board of a repeat class is searched, and no prime whose board before it the ledger has.  A
    /// board next to the class of one that is being climbed to waits (`held`).  The job of an ordinary board is a step
    /// down to a floor (`cost::plan`) unless its gap is within two steps, and then it is the proof, the ordinary search
    /// that doubles its limit.
    pub fn next_job(&self, cpu: f64) -> Option<Job> {
        let s = &self.settings;
        let key = |e: &Entry| -> (u8, f64, u32) {
            match e.gap() {
                _ if !e.mapped => (0, 0.0, e.n),
                Some(gap) if e.boosted => (1, gap as f64, e.n),
                Some(gap) => (2, self.cost(e, gap), e.n),
                None => unreachable!("a board mapped has both bounds"),
            }
        };
        let held = self.held();
        let ordinary = self
            .eligible()
            .filter(|e| !held.contains(&e.n))
            .map(|e| (key(e), e))
            .min_by(|(a, _), (b, _)| a.partial_cmp(b).expect("no NaN"))?
            .1;
        if !ordinary.mapped {
            return Some(self.job(ordinary, Kind::Init, Duration::ZERO, Vec::new(), None, None));
        }
        if !self.pending.is_empty() || self.maps_running() {
            return None;
        }
        // The hold keeps ordinary searches off a board; a climb to it has its proven neighbor.
        let (kind, entry, limit, floor) = match self.next_climb() {
            Some((way, entry)) => (Kind::Climb(way), entry, s.climb_limit, None),
            None => match cost::plan(ordinary, s.floor_step, self.pooled.rate()) {
                Plan::Proof => (Kind::Search, ordinary, s.base * 2f64.powi(ordinary.level.min(60) as i32), None),
                Plan::Step { score, resume, seconds, tries } => {
                    let limit = (s.floor_slack * seconds).max(s.base) * 2f64.powi(tries.min(60) as i32);
                    (Kind::Floor, ordinary, limit, Some(Floor { score, resume }))
                }
            },
        };
        let remaining = (s.cpu_budget - cpu) / s.workers as f64;
        let limit = Duration::from_secs_f64(limit.min(s.max_limit).min(remaining.max(1.0)));
        // The games that seed a search: those of the boards next to it and, if it is the board of a repeat class, of
        // the boards just outside the class (the proven one a climb into the class goes from).
        let seeds = match floor {
            Some(_) => Vec::new(),
            None => {
                let n = entry.n;
                let [below, above] = outside(&self.book.class(n));
                let boards: BTreeSet<u32> = [n - 1, n, n + 1, below, above].into_iter().collect();
                boards
                    .into_iter()
                    .filter_map(|m| {
                        self.book.entries.get(&m)?.lower.as_ref().map(|l| Seed { from: m, moves: l.moves.clone() })
                    })
                    .collect()
            }
        };
        Some(self.job(entry, kind, limit, seeds, entry.upper.map(|u| u.value), floor))
    }

    /// The job of `kind` on the board of `entry`: what it is asked, and what the events say of its neighbors and the
    /// finder (the neighbors are proven or not as the job starts).
    fn job(
        &self,
        entry: &Entry,
        kind: Kind,
        limit: Duration,
        seeds: Vec<Seed>,
        cap: Option<u64>,
        floor: Option<Floor>,
    ) -> Job {
        let proven = |n: Option<u32>| n.is_some_and(|n| self.book.entries.get(&n).is_some_and(Entry::proven));
        let n = entry.n;
        Job {
            n,
            kind,
            limit,
            seeds,
            cap,
            below_proven: proven(n.checked_sub(1)),
            above_proven: proven(n.checked_add(1)),
            lns: LnsPlan { seconds: self.settings.lns_seconds, last: entry.lns_score, round: entry.attempts },
            floor,
        }
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
                below_proven: job.below_proven,
                above_proven: job.above_proven,
                floor: job.floor.map(|floor| floor.score),
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
        let was_proven = self.book.entries[&n].proven();
        // The floor step this one goes on from, if it does (a step whose floor became moot starts again), and the steps
        // of the search over its earlier runs: the stalled step's, or the board's ordinary searches' and climbs'.
        let resumed = self.book.entries[&n].stalled.filter(|s| done.kind == Kind::Floor && Some(s.floor) == done.floor);
        let before = bounds(&self.book.entries[&n]);
        let nodes_before = match done.kind {
            Kind::Floor => resumed.map_or(0, |s| s.nodes),
            _ => self.book.entries[&n].nodes,
        };
        let (lower_rule, upper_rule) = match done.kind {
            Kind::Init => (Rule::Start, Rule::Root),
            Kind::Floor if a.held => (Rule::Search, Rule::Floor),
            _ if a.completed => (Rule::Search, Rule::Exhausted),
            _ => (Rule::Search, Rule::Dual),
        };
        let found_set = !a.promotions.is_empty();
        // The games offered as lower bounds, the lesser first (the finder's before an equal search's, which began from
        // it and so has found nothing better): a game replaces the board's only if it is better.  The finder's counts
        // only if it improved on the game it began from.  Every one has passed the referee on its worker.
        let mut lowers = vec![Lower {
            value: a.value,
            moves: a.moves,
            promotions: Some(a.promotions),
            rule: lower_rule,
            from: None,
        }];
        let found = a.lns.as_ref().filter(|run| run.improved());
        lowers.extend(found.map(|run| Lower {
            value: run.found,
            moves: run.moves.clone(),
            promotions: Some(run.promotions.clone()),
            rule: Rule::Lns,
            from: None,
        }));
        lowers.sort_by_key(|l| (l.value, l.rule != Rule::Lns));
        let mut changes = Vec::new();
        for lower in lowers {
            changes.extend(self.book.offer_lower(n, lower, false)?);
        }
        // The upper bound the job proved: a search that reached its ceiling proves its game by the bound that was its
        // ceiling, which stands; a floor step that completed having found nothing proves its floor, and one that
        // completed having found a set proves that set (Lemma F); a floor step that stopped proves nothing.
        let upper = match done.kind {
            _ if a.reached_cap => None,
            Kind::Floor if a.held => done.floor,
            Kind::Floor => a.completed.then_some(a.value),
            _ => Some(a.upper.unwrap_or(a.value)),
        };
        if let Some(value) = upper {
            changes.extend(self.book.offer_upper(n, Upper { value, rule: upper_rule, from: None }, false)?);
        }
        let lns_s = a.lns.as_ref().map_or(0.0, |run| run.seconds);
        if let Some(run) = &a.lns {
            self.lns.add(run);
        }
        let entry = self.book.entries.get_mut(&n).expect("a board of the ledger");
        entry.mapped = true;
        entry.search_s += lns_s;
        entry.lns_score = a.lns.as_ref().map_or(entry.lns_score, |run| Some(run.found));
        match done.kind {
            // The map's dual bound is the root's bound.
            Kind::Init => entry.root = a.upper.or(entry.root),
            Kind::Floor => {}
            Kind::Search | Kind::Climb(_) => entry.nodes = a.nodes,
        }
        if done.kind != Kind::Init {
            entry.attempts += 1;
            entry.search_s += a.seconds;
            entry.boosted = false;
            self.pooled.nodes += a.nodes.saturating_sub(nodes_before);
            self.pooled.seconds += a.seconds;
        }
        // A climb leaves the board's level, and so the limit of its next ordinary search, as it was.
        if done.kind == Kind::Search {
            self.book.entries.get_mut(&n).expect("a board of the ledger").level += 1;
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
        // The finder (its seconds, 0 if it did not run; the scores of its input and of its game), which game seeded the
        // search, and whether the boards n − 1 and n + 1 were proven when the job started.
        let run = a.lns.as_ref();
        fields.extend([
            ("lns_s", Json::from((lns_s * 1000.0).round() / 1000.0)),
            ("lns_from", run.map_or(Json::Null, |run| Json::from(run.from))),
            ("lns_found", run.map_or(Json::Null, |run| Json::from(run.found))),
            ("seed_from", a.seed_from.map_or(Json::Null, |seed| Json::from(seed.name()))),
            ("below_proven", Json::from(done.below_proven)),
            ("above_proven", Json::from(done.above_proven)),
        ]);
        if let Some(job) = done.kind.job() {
            fields.push(("job", Json::from(job)));
        }
        if let Some(way) = done.kind.way() {
            fields.push(("way", Json::from(way.name())));
            fields.push(("outcome", Json::from(self.end_climb(n, way, was_proven))));
        }
        if let Some(floor) = done.floor {
            let distance = self.book.entries[&n].root.map(|root| root.saturating_sub(floor));
            fields.push(("floor", Json::from(floor)));
            fields.push(("distance", distance.map_or(Json::Null, Json::from)));
            fields.push(("held", Json::from(a.held)));
            let step = StepEnd {
                floor,
                held: a.held,
                capped: a.reached_cap,
                found: found_set,
                nodes: a.nodes,
                seconds: a.seconds,
            };
            fields.push(("outcome", Json::from(self.end_step(n, step, resumed, was_proven))));
        }
        // What the rest of the board's proof is forecast to cost, now (not for a board proven).
        let forecast =
            cost::forecast(&self.book.entries[&n], self.pooled.rate()).filter(|_| !self.book.entries[&n].proven());
        fields.push(("forecast", forecast.map_or(Json::Null, |f| Json::from((f.log_nodes * 100.0).round() / 100.0))));
        fields.push(("forecast_s", forecast.map_or(Json::Null, |f| Json::from(f.seconds().round()))));
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
            let _ = remove_checkpoints(&self.dir, n);
        }
        self.flush_events()
    }

    /// What a floor step leaves on its board, counted: `held` (it completed having found nothing: the floor is the new
    /// upper bound, and the step is a point of the board's line), `proved` (it completed having found a set, or reached
    /// the ceiling with one: the board is proven), `moot` (a relation proved the board while it went on, or lowered its
    /// upper bound to the floor, which ended the step at the ceiling with nothing explored), `found` (it stopped with a
    /// set, so the floor is below the lower bound now: the step is dropped, and its checkpoint, a search from that set,
    /// is the board's own for the proof if it has none), or `stalled` (the limit stopped it with nothing: it is kept,
    /// to be tried again under the same floor with its limit doubled).  `resumed`
    /// is the stalled step it went on from.  Returns the outcome's name, as the event has it.
    fn end_step(&mut self, n: u32, step: StepEnd, resumed: Option<Stalled>, was_proven: bool) -> &'static str {
        let StepEnd { floor, held, capped, found, nodes, seconds } = step;
        let entry = self.book.entries.get_mut(&n).expect("a board of the ledger");
        let seconds = resumed.map_or(0.0, |s| s.seconds) + seconds;
        entry.stalled = None;
        self.steps.ended += 1;
        if was_proven {
            "moot"
        } else if held {
            entry.steps.push(Step { floor, nodes, seconds });
            self.steps.held += 1;
            "held"
        } else if entry.proven() {
            self.steps.proved += 1;
            "proved"
        } else if capped {
            "moot"
        } else if found {
            // The step has become a search from the set it found, and the proof goes on from its checkpoint if the
            // board has no other; else it is deleted.
            self.steps.found += 1;
            let (floor_checkpoint, checkpoint) = (floor_checkpoint_path(&self.dir, n), checkpoint_path(&self.dir, n));
            if !checkpoint.exists() && fs::rename(&floor_checkpoint, &checkpoint).is_ok() {
                entry.nodes = nodes;
            } else {
                let _ = engine::remove_checkpoint(&floor_checkpoint);
            }
            "found"
        } else {
            let tries = resumed.map_or(0, |s| s.tries) + 1;
            entry.stalled = Some(Stalled { floor, nodes, seconds, tries });
            self.steps.stalled += 1;
            "stalled"
        }
    }

    /// Counts the climb in `way` to `n` that has ended, by how the board stands: proved by the climb's search, proven by
    /// a relation while it went on (`was_proven`: the climb was moot), or neither, when the board is parked.  Returns
    /// the outcome's name, as the event has it.
    fn end_climb(&mut self, n: u32, way: Way, was_proven: bool) -> &'static str {
        let entry = self.book.entries.get_mut(&n).expect("a board of the ledger");
        let tally = self.climbs.of(way);
        tally.ended += 1;
        if was_proven {
            "moot"
        } else if entry.proven() {
            tally.proved += 1;
            "proved"
        } else {
            entry.parked = true;
            tally.parked += 1;
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

    /// What follows changes of bounds: boards changed through a proven neighbor go first; a board that is proven is
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
                    let _ = remove_checkpoints(&self.dir, n);
                }
                None => {}
            }
        }
        Ok(())
    }

    /// The open boards that are searched and mapped, by the forecast time of their proofs on one thread: within an
    /// hour, within a day, longer.
    fn forecasts(&self) -> [usize; 3] {
        let mut counts = [0; 3];
        let open = self.book.entries.values().filter(|e| e.mapped && !e.proven() && self.book.searched(e.n));
        for e in open {
            if let Some(forecast) = cost::forecast(e, self.pooled.rate()) {
                counts[usize::from(forecast.seconds() > 3600.0) + usize::from(forecast.seconds() > 86_400.0)] += 1;
            }
        }
        counts
    }

    fn status(&mut self) {
        let (proven, total) = (self.proven(), self.book.entries.len());
        let [hour, day, longer] = self.forecasts();
        eprintln!(
            "ledger: {:.0} s, {:.0} CPU s of {:.0}: {proven} of {total} proven, {} searching; climbs up {}, down {}; \
             floor steps {}; proofs forecast within an hour of one thread: {hour} boards, within a day: {day}, longer: \
             {longer}; finder {} runs, {} improved; main thread busy {:.1} s, results waited {:.1} s",
            self.t0.elapsed().as_secs_f64(),
            self.cpu(),
            self.settings.cpu_budget,
            self.running.len(),
            self.climbs.up,
            self.climbs.down,
            self.steps,
            self.lns.runs,
            self.lns.improved,
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
        ("climb_up", Json::from(s.climb_up)),
        ("climb_down", Json::from(s.climb_down)),
        ("lns_s", Json::from(s.lns_seconds)),
        ("floor_step", Json::from(s.floor_step)),
        ("floor_slack", Json::from(s.floor_slack)),
        ("gap_unit", Json::from(s.gap_unit)),
        ("proven_file", proven.map_or(Json::Null, |p| Json::from(p.display().to_string()))),
        ("bounds_file", s.bounds.as_ref().map_or(Json::Null, |p| Json::from(p.display().to_string()))),
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
