//! The unsound mode, `taxman solve LO-HI --unsound --effort E --out FILE`: each game played as well as a level of
//! effort finds, with no proof that its score is opt(n).
//!
//! The proof of a hard game past 5060 takes hours, and nearly all of it goes to proving a set that the search found
//! early.  The unsound mode keeps the finding and drops the proof.  Its line holds legal moves and their score, a lower
//! bound on opt(n), and is marked unsound ([`UNSOUND`]), so that no reader of proven lines takes one.  The user names a
//! level of effort: more effort costs more time and gives a better chance that each score is opt(n).
//!
//! # One game
//!
//! * The setup is the proven mode's ([`set_up`]): the board and its frames.  A board with no promotion candidates has
//!   the moves with no promotions (method `trivial`).
//! * A prime n plays the prime shortcut on the moves for n − 1 (method `prime`).  A game whose search problem a line of
//!   the run or of the file posed, within [`SAME_SEARCH_WINDOW`](super::SAME_SEARCH_WINDOW) games before it, plays that
//!   line's promotions on its own board (method `repeat`, the proven mode's lookup).  Neither searches, and both copy
//!   what the line they start from falls short by, if it does.
//! * Any other game (method `search`) makes the start that a proven search makes
//!   ([`start_incumbent`](crate::search::start_incumbent)), seeded with the moves for n − 1, and then the rounds of its
//!   level ([`Plan::rounds`]) of LNS on every thread, whose answer is the same on any number of threads
//!   ([`Lns::rounds`](crate::search::finder::Lns::rounds)).
//! * There is no ceiling.  The proven mode's, n + score(n − 1) − in_the_bag, holds only when the moves for n − 1 are
//!   optimal, which nothing here says, so nothing stops at it, is held below it or is skipped by it.  A score above
//!   n + score(n − 1) proves the line of n − 1 short: the line records by how much at least (`prev.short_by`), and the
//!   run goes on.
//! * The moves are replayed before they count ([`replay`]), and a score above the built-in table's, which is proven, is
//!   a hard error.
//!
//! # The level of effort
//!
//! Effort 0 is the start alone, and effort E ≥ 1 is 2^(E−1) rounds of [`NEIGHBORHOODS`] neighborhoods each.  The rounds
//! do not depend on the level: a run at E + 1 does the rounds of E first, and then more.  So the `improvements` of a
//! line at one level show what every lower level would have scored.
//!
//! # A range
//!
//! [`solve_range`] plays the games lo..=hi in order, one at a time, on all the threads given.  The moves for n − 1 that
//! a game starts from are this run's line of n − 1, else the file's, else the built-in table's (as a seed only, never
//! a line).  Each line is appended to the file and synced as soon as its game ends, and a restarted run skips the games
//! the file has.  A stop signal (SIGINT, SIGTERM) ends the run between rounds, and the game in play is not recorded.
//! The file holds unsound lines only: [`UnsoundFile::open`] refuses a line of a proven run, as every reader of proven
//! lines refuses an unsound one ([`refuse_unsound`](crate::game::refuse_unsound)).
//!
//! # Measuring
//!
//! Two options measure the mode on the games whose opt(n) is proven, and are refused for a game the built-in table
//! lacks.  `--stop-at-known` ends a game's rounds as soon as its score is the table's, and the line records the round
//! at which it was, or that it never was.  `--from-known` seeds every game with the table's moves for n − 1, not with
//! this run's line.  solver/docs/REFERENCE.md has the keys of the line ("An unsound line").

use super::{
    prime_game, set_up, Before, Derived, Earlier, Failure, Method, RangeSummary, SearchSetup, SetUp, SolveOptions,
};
use crate::engine;
use crate::game::{replay, Optimal, UNSOUND};
use crate::json::Json;
use crate::results::{append_line, build_json, each_line, mend, moves_digest, open_locked, Fields};
use crate::search::finder::{self, Improvement, NEIGHBORHOODS};
use crate::search::ProblemSize;
use crate::tactics::Tactics;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The most effort a run may ask for: 2^31 rounds, far more than a game needs.
pub const MAX_EFFORT: u32 = 32;

/// How the unsound mode plays: the level of effort, the seed of the rounds' random choices, and the measuring options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    /// From 0 to [`MAX_EFFORT`].
    pub effort: u32,
    pub seed: u64,
    /// `--stop-at-known`: a game's rounds end once its score is the built-in table's.
    pub stop_at_known: bool,
    /// `--from-known`: every game starts from the built-in table's moves for n − 1.
    pub from_known: bool,
}

impl Plan {
    /// The rounds of the level: none at effort 0, 2^(E−1) at effort E.
    pub fn rounds(&self) -> u64 {
        self.effort.checked_sub(1).map_or(0, |e| 1u64 << e)
    }
}

/// Where the moves for n − 1 that a game started from came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// This run's line of n − 1.
    Run,
    /// The file's line of n − 1.
    Results,
    /// The built-in table's moves for n − 1, which are proven.
    Table,
}

impl Source {
    /// The name in a line.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Run => "run",
            Source::Results => "results",
            Source::Table => "table",
        }
    }
}

/// The moves for n − 1 that a game starts from.
#[derive(Clone, Debug)]
struct Moves {
    moves: Vec<u32>,
    score: u64,
    source: Source,
}

/// What a line records of the moves for n − 1 it started from (`prev`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Previous {
    pub n: u32,
    pub score: u64,
    pub source: Source,
    /// [`moves_digest`] of the moves.
    pub moves_digest: String,
    /// How many points the moves for n − 1 fall short of opt(n − 1) at least, as this game's score proves: its score
    /// less n + score(n − 1), where that is above 0 (no moves for n score more than n + opt(n − 1)); else 0.
    pub short_by: u64,
}

/// What `--stop-at-known` recorded of a game: the built-in table's score, and the round after which the game's score
/// was that (0 for the start, or for a game with no rounds to run), or `None` if it never was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Known {
    pub score: u64,
    pub round: Option<u64>,
}

/// A round whose better set raised the game's score: the round, the time on the game's clock when it ended, and the new
/// score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Raised {
    pub round: u64,
    pub seconds: f64,
    pub score: u64,
}

/// An unsound line: a game as the unsound mode played it (the module documentation; solver/docs/REFERENCE.md, "An
/// unsound line", has its keys).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub n: u32,
    pub score: u64,
    pub moves: Vec<u32>,
    /// The numbers promoted, ascending.
    pub promotions: Vec<u32>,
    /// `trivial`, `prime`, `repeat` or `search`.
    pub method: Method,
    /// What the moves with no promotions score; `None` for a prime game, which sets no board up.
    pub in_the_bag: Option<u64>,
    pub prev: Option<Previous>,
    /// For a repeat, the n of the line whose promotions it plays.
    pub repeats: Option<u32>,
    /// What a searched or repeat game was set up with (its `cap` is always `None`: there is no ceiling).
    pub setup: Option<SearchSetup>,
    pub plan: Plan,
    /// The score of a searched game's start.
    pub start: Option<u64>,
    /// The rounds done, and the engine's steps in them.
    pub rounds: u64,
    pub steps: u64,
    /// Each round that raised the score, in order.
    pub improvements: Vec<Raised>,
    /// The game's wall time, from the start of its setup to its line, and the part of it before the rounds.
    pub seconds: f64,
    pub start_seconds: f64,
    pub threads: usize,
    /// What `--stop-at-known` recorded, if it was given.
    pub known: Option<Known>,
    pub tactics: Tactics,
}

impl Line {
    /// The line as one JSON object.
    pub fn to_json(&self) -> Json {
        let prev = self.prev.as_ref().map_or(Json::Null, |p| {
            Json::object([
                ("n", Json::from(p.n)),
                ("score", Json::from(p.score)),
                ("source", Json::from(p.source.as_str())),
                ("moves_digest", Json::from(p.moves_digest.as_str())),
                ("short_by", Json::from(p.short_by)),
            ])
        });
        let setup = self.setup.map_or(Json::Null, |s| {
            Json::object([
                ("frames", Json::from(s.frames)),
                ("candidates", Json::from(s.candidates)),
                ("max_total", Json::from(s.max_total)),
                ("greedy_sum", Json::from(s.greedy_sum)),
                ("digest", Json::from(super::digest_text(s.digest))),
            ])
        });
        let improvements = self.improvements.iter().map(|raised| {
            Json::object([
                ("round", Json::from(raised.round)),
                ("neighborhoods", Json::from(raised.round * NEIGHBORHOODS)),
                ("seconds", Json::Num(raised.seconds)),
                ("score", Json::from(raised.score)),
            ])
        });
        let known = self.known.map_or(Json::Null, |known| {
            Json::object([("known", Json::from(known.score)), ("round", known.round.map_or(Json::Null, Json::from))])
        });
        Json::object([
            (UNSOUND, Json::Bool(true)),
            ("n", Json::from(self.n)),
            ("score", Json::from(self.score)),
            ("moves", Json::array(self.moves.iter().copied())),
            ("promotions", Json::array(self.promotions.iter().copied())),
            ("method", Json::from(self.method.as_str())),
            ("in_the_bag", self.in_the_bag.map_or(Json::Null, Json::from)),
            ("prev", prev),
            ("repeats", self.repeats.map_or(Json::Null, Json::from)),
            ("setup", setup),
            ("effort", Json::from(self.plan.effort)),
            ("seed", Json::from(self.plan.seed)),
            ("start", self.start.map_or(Json::Null, Json::from)),
            ("rounds", Json::from(self.rounds)),
            ("neighborhoods", Json::from(self.rounds * NEIGHBORHOODS)),
            ("steps", Json::from(self.steps)),
            ("improvements", Json::array(improvements)),
            ("seconds", Json::Num(self.seconds)),
            ("start_seconds", Json::Num(self.start_seconds)),
            ("threads", Json::from(self.threads)),
            ("from_known", Json::Bool(self.plan.from_known)),
            ("stop_at_known", known),
            ("tactics", Json::from(self.tactics.description())),
            ("build", build_json()),
        ])
    }

    /// One line for a person: n, score, method, the rounds and how many raised the score, the time, `unsound`, and what
    /// `--stop-at-known` recorded and a line of n − 1 that this one proves short, if anything.
    pub fn summary_line(&self) -> String {
        let known = match self.known {
            None => String::new(),
            Some(Known { round: Some(round), .. }) => format!("  known reached in round {round}"),
            Some(Known { round: None, .. }) => "  known not reached".to_owned(),
        };
        let short = self
            .prev
            .as_ref()
            .filter(|prev| prev.short_by > 0)
            .map_or_else(String::new, |prev| format!("  n-1 short by {} at least", prev.short_by));
        format!(
            "n={:<5} score={:<9} {:<7} rounds={:<6} improved={:<3} {:>9.3} s  unsound{known}{short}",
            self.n,
            self.score,
            self.method.as_str(),
            self.rounds,
            self.improvements.len(),
            self.seconds
        )
    }
}

/// What the unsound mode's search of a game came to ([`Job::improve`](super::Job::improve)): the start's promotion sum,
/// when it was known on the game's clock, and the rounds after it.
pub(super) struct Improved {
    pub(super) start: u64,
    pub(super) start_seconds: f64,
    /// The best promotions found, ascending, and the moves they make.
    pub(super) promotions: Vec<u32>,
    pub(super) moves: Vec<u32>,
    pub(super) rounds: u64,
    pub(super) nodes: u64,
    pub(super) improvements: Vec<Improvement>,
    /// A stop signal ended the rounds: the game is not recorded.
    pub(super) interrupted: bool,
}

// ---------------------------------------------------------------------------------------------------------
// The file

/// A line of an unsound file as the run needs it: the moves for the next game, and a repeat's.
#[derive(Clone, Debug)]
struct Stored {
    score: u64,
    moves: Vec<u32>,
    promotions: Vec<u32>,
    /// The size and the digest of the search problem of a searched or repeat game.
    problem: Option<(ProblemSize, u64)>,
}

/// The file of unsound lines that a run appends to: one JSON object a line, an unsound line each.  Opening it locks it
/// against other processes and checks it; each line is appended and synced as its game ends.  It has the torn last line
/// of a results file, and is mended as one is.
pub struct UnsoundFile {
    path: PathBuf,
    /// Locked (`flock`) for as long as it is open.
    file: File,
    games: BTreeMap<u32, Stored>,
}

impl UnsoundFile {
    /// Opens (creating if need be) the file at `path`, locks it, and checks every line: an unsound line (a line of a
    /// proven run is refused, as is a line that is not a game), whose moves replay to its score, whose promotions are
    /// moves in ascending order, which scores `in_the_bag` plus their sum where it records `in_the_bag`, no more than
    /// the score of `optimal` where that has the game, and the only line of its game.  A torn last line is cut off.
    pub fn open(path: &Path, optimal: &Optimal) -> Result<UnsoundFile, String> {
        let at = |e: String| format!("{}: {e}", path.display());
        let (mut file, text) = open_locked(path)?;
        let mut games = BTreeMap::new();
        let kept = each_line(&text, |number, json| {
            let (n, stored) = read_line(json, optimal).map_err(|e| format!("line {number}: {e}"))?;
            match games.insert(n, stored) {
                Some(_) => Err(format!("line {number}: a second line for n={n}")),
                None => Ok(()),
            }
        })
        .map_err(at)?;
        mend(&mut file, &text, kept).map_err(|e| at(e.to_string()))?;
        Ok(UnsoundFile { path: path.to_owned(), file, games })
    }

    /// The moves for `n` that the file has, and their score.
    fn moves(&self, n: u32) -> Option<(&[u32], u64)> {
        self.games.get(&n).map(|game| (game.moves.as_slice(), game.score))
    }

    /// Appends `line` and makes sure it reaches the disk.
    fn append(&mut self, line: &Line) -> Result<(), String> {
        append_line(&mut self.file, &line.to_json()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.games.insert(line.n, stored_of(line));
        Ok(())
    }
}

/// Dropping releases the lock at once, as a results file does (`results::ResultsFile`).
impl Drop for UnsoundFile {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// What the run keeps of `line`.
fn stored_of(line: &Line) -> Stored {
    Stored {
        score: line.score,
        moves: line.moves.clone(),
        promotions: line.promotions.clone(),
        problem: line.setup.map(|setup| (setup.size(), setup.digest)),
    }
}

/// The game of an unsound line of a file, checked (see [`UnsoundFile::open`]).
fn read_line(json: &Json, optimal: &Optimal) -> Result<(u32, Stored), String> {
    const NOT_UNSOUND: &str =
        "not an unsound line (a line of a proven run, or no game): a file of unsound lines holds no other kind";
    if json.get(UNSOUND) != Some(&Json::Bool(true)) {
        return Err(NOT_UNSOUND.to_owned());
    }
    let f = Fields(json);
    let n = f.u32("n")?;
    if !(1..=super::MAX_N).contains(&n) {
        return Err(format!("n={n} is not a game from 1 to {}", super::MAX_N));
    }
    let (score, moves, promotions) = (f.u64("score")?, f.u32s("moves")?, f.u32s("promotions")?);
    let fail = |message: String| format!("(n={n}) {message}");
    let replayed = replay(n, &moves).map_err(fail)?;
    if replayed != score {
        return Err(fail(format!("the moves score {replayed}, not the recorded {score}")));
    }
    let played: BTreeSet<u32> = moves.iter().copied().collect();
    if !promotions.windows(2).all(|pair| pair[0] < pair[1]) || !promotions.iter().all(|p| played.contains(p)) {
        return Err(fail("the promotions are not moves in ascending order".into()));
    }
    let sum: u64 = promotions.iter().map(|&p| u64::from(p)).sum();
    if f.opt_u64("in_the_bag")?.is_some_and(|bag| bag + sum != score) {
        return Err(fail(format!("it scores {score}, not in_the_bag plus the promotions' {sum}")));
    }
    if let Some(best) = optimal.score(n).filter(|&best| score > best) {
        return Err(fail(format!("it scores {score}, above the built-in table's {best}, which is proven")));
    }
    let problem = match f.get("setup")? {
        Json::Null => None,
        setup => {
            let s = Fields(setup);
            let size = ProblemSize {
                frames: s.u32("frames")?,
                candidates: s.u32("candidates")?,
                max_total: s.u32("max_total")?,
            };
            Some((size, s.digest("digest")?))
        }
    };
    Ok((n, Stored { score, moves, promotions, problem }))
}

// ---------------------------------------------------------------------------------------------------------
// A range

/// Plays the games `lo..=hi` in the unsound mode with `plan` (see the module documentation), on `opts.threads` threads
/// with `opts.tactics`, appending each line to the file `out`, if there is one, then handing it to `on_game`.  The
/// games the file has are skipped.  A stop signal ends the range with the game in play unrecorded (`stopped_at`).
///
/// # Errors
///
/// A range that is empty or starts below 1; `--stop-at-known` or `--from-known` for a game the built-in table lacks; a
/// file that cannot be opened or written, or holds a line it may not ([`UnsoundFile::open`]); and a game whose moves
/// fail a check (the module documentation, "One game").
pub fn solve_range(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    plan: &Plan,
    out: Option<&Path>,
    mut on_game: impl FnMut(&Line),
) -> Result<RangeSummary, String> {
    if lo < 1 || lo > hi {
        return Err(format!("the range {lo}..={hi} is empty or starts below 1"));
    }
    if plan.effort > MAX_EFFORT {
        return Err(format!("the effort {} is more than the most, {MAX_EFFORT}", plan.effort));
    }
    let optimal = Optimal::built_in();
    if plan.stop_at_known || plan.from_known {
        if let Some(n) = (lo..=hi).find(|&n| optimal.score(n).is_none()) {
            return Err(format!(
                "--stop-at-known and --from-known measure the games whose opt(n) is proven, and the built-in table \
                 lacks game {n}"
            ));
        }
    }
    let mut file = out.map(|path| UnsoundFile::open(path, optimal)).transpose()?;
    // The lines of the file that set a search up, whose problems a game may repeat.
    let mut earlier = Earlier::<()>::default();
    for (&n, game) in file.iter().flat_map(|file| &file.games) {
        if let Some((size, digest)) = game.problem {
            earlier.add(n, size, digest, game.promotions.clone(), ());
        }
    }
    let mut summary = RangeSummary::default();
    // This run's line of the game before, once there is one.
    let mut last: Option<Moves> = None;
    for n in lo..=hi {
        if file.as_ref().is_some_and(|file| file.games.contains_key(&n)) {
            summary.skipped += 1;
            last = None;
            continue;
        }
        if engine::stop_signal().is_some() {
            summary.stopped_at = Some(n);
            break;
        }
        let before = moves_before(n, last.take(), file.as_ref(), plan, optimal);
        let Some(line) = play(n, before.as_ref(), opts, plan, &mut earlier, optimal)? else {
            summary.stopped_at = Some(n);
            break;
        };
        if let Some(file) = file.as_mut() {
            file.append(&line)?;
        }
        on_game(&line);
        summary.solved += 1;
        if let Some(setup) = line.setup {
            earlier.add(n, setup.size(), setup.digest, line.promotions.clone(), ());
        }
        last = Some(Moves { moves: line.moves, score: line.score, source: Source::Run });
    }
    Ok(summary)
}

/// The moves for `n − 1` that game `n` starts from: with `--from-known` the built-in table's, else `last` (this run's
/// line of `n − 1`, if it has one), else the file's line of `n − 1`, else the table's.  None for game 1, or where
/// nothing has the moves for `n − 1`, which a note on stderr says.
fn moves_before(
    n: u32,
    last: Option<Moves>,
    file: Option<&UnsoundFile>,
    plan: &Plan,
    optimal: &Optimal,
) -> Option<Moves> {
    let before = n.checked_sub(1).filter(|&before| before >= 1)?;
    let table = || {
        let (moves, score) = (optimal.moves(before)?, optimal.score(before)?);
        Some(Moves { moves: moves.to_vec(), score, source: Source::Table })
    };
    if plan.from_known {
        return table();
    }
    let file = || {
        let (moves, score) = file?.moves(before)?;
        Some(Moves { moves: moves.to_vec(), score, source: Source::Results })
    };
    let found = last.or_else(file).or_else(table);
    if found.is_none() {
        let _ = writeln!(
            std::io::stderr(),
            "note: no moves for {before} (in this run, the file or the built-in table): game {n} starts with no seed"
        );
    }
    found
}

/// Game `n` played in the unsound mode, from the moves for `n − 1` `before` (see the module documentation, "One game"),
/// and its line, checked; `None` if a stop signal ended its rounds.  `earlier` has the lines a repeat may repeat, and
/// is told of the problem of this game's board.  An error names the game.
fn play(
    n: u32,
    before: Option<&Moves>,
    opts: &SolveOptions,
    plan: &Plan,
    earlier: &mut Earlier<()>,
    optimal: &Optimal,
) -> Result<Option<Line>, String> {
    let began = Instant::now();
    let clock = || began.elapsed().as_secs_f64();
    // The moves for n − 1 are a seed and the prime shortcut's moves, which nothing checks again: the run's and the
    // file's lines were replayed before they were kept, and the table's are proven.
    let given = if before.is_some() { Before::Run } else { Before::None };
    let mut line = Line {
        n,
        score: 0,
        moves: Vec::new(),
        promotions: Vec::new(),
        method: Method::Search,
        in_the_bag: None,
        prev: None,
        repeats: None,
        setup: None,
        plan: *plan,
        start: None,
        rounds: 0,
        steps: 0,
        improvements: Vec::new(),
        seconds: 0.0,
        start_seconds: 0.0,
        threads: opts.threads,
        known: None,
        tactics: opts.tactics,
    };
    let known = plan.stop_at_known.then(|| optimal.score(n).expect("the range was checked against the table"));
    match set_up(n, &given, opts).map_err(Failure::message)? {
        SetUp::Derived(Derived::Trivial { plain, in_the_bag, .. }) => {
            (line.method, line.moves, line.in_the_bag) = (Method::Trivial, plain, Some(in_the_bag));
        }
        SetUp::Derived(Derived::Prime { candidates }) => {
            let before = before.expect("the prime shortcut is taken only with the moves for n − 1");
            line.method = Method::Prime;
            (line.moves, line.promotions) = prime_game(n, &candidates, &before.moves);
        }
        SetUp::Search(set, problem) => {
            let repeated = opts.tactics.same_search.then(|| earlier.find(n, &problem)).flatten();
            earlier.remember(n, problem);
            line.setup = Some(set.setup);
            line.in_the_bag = Some(set.in_the_bag);
            if let Some(m) = repeated {
                let promotions = earlier.promotions(m).expect("a game that find found").to_vec();
                (line.method, line.repeats) = (Method::Repeat, Some(m));
                line.moves = set.job.moves_for(&promotions);
                line.promotions = promotions;
            } else {
                let bag = set.in_the_bag;
                let enough = |sum: u64| known.is_some_and(|known| bag + sum >= known);
                let lns = finder::Options { seed: plan.seed, ..finder::Options::default() };
                let seed = before.map(|before| before.moves.as_slice());
                let improved = set.job.improve(seed, plan.rounds(), opts.threads, lns, &clock, &enough);
                if improved.interrupted {
                    return Ok(None);
                }
                line.start = Some(bag + improved.start);
                line.start_seconds = improved.start_seconds;
                (line.rounds, line.steps) = (improved.rounds, improved.nodes);
                line.improvements = improved
                    .improvements
                    .iter()
                    .map(|i| Raised { round: i.round, seconds: i.seconds, score: bag + i.sum })
                    .collect();
                (line.moves, line.promotions) = (improved.moves, improved.promotions);
            }
        }
    }
    conclude(&mut line, before, known, optimal).map_err(|e| format!("n={n}: {e}"))?;
    line.seconds = clock();
    Ok(Some(line))
}

/// The checks of the moves of `line` before they count (the module documentation, "One game"), and what they give the
/// line: its score, its record of the moves for n − 1 it started from, `before`, and what `--stop-at-known` recorded,
/// if it was given (`known`, the table's score).
fn conclude(line: &mut Line, before: Option<&Moves>, known: Option<u64>, optimal: &Optimal) -> Result<(), String> {
    let n = line.n;
    let method = line.method.as_str();
    line.score = replay(n, &line.moves).map_err(|e| format!("the {method} moves do not stand up: {e}"))?;
    let (score, sum): (u64, u64) = (line.score, line.promotions.iter().map(|&p| u64::from(p)).sum());
    if let Some(bag) = line.in_the_bag.filter(|&bag| bag + sum != score) {
        return Err(format!(
            "the {method} moves score {score}, not inTheBag {bag} + their promotions' {sum}: the moves and the frame \
             model disagree (promotions {:?})",
            line.promotions
        ));
    }
    if let Some(best) = optimal.score(n).filter(|&best| score > best) {
        return Err(format!(
            "the {method} moves score {score}, above the built-in table's {best}, which is proven (promotions {:?})",
            line.promotions
        ));
    }
    line.prev = before.map(|before| Previous {
        n: n - 1,
        score: before.score,
        source: before.source,
        moves_digest: moves_digest(&before.moves),
        short_by: score.saturating_sub(before.score + u64::from(n)),
    });
    line.known = known.map(|known| {
        // The round after which the score was the table's: the first improvement's that reached it, else the start's.
        let reached = line.improvements.iter().find(|raised| raised.score >= known).map(|raised| raised.round);
        Known { score: known, round: reached.or_else(|| (line.start.unwrap_or(score) >= known).then_some(0)) }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rounds_of_a_level_double_with_each_level() {
        let plan = |effort| Plan { effort, seed: 1, stop_at_known: false, from_known: false };
        let rounds: Vec<u64> = (0..=4).map(|effort| plan(effort).rounds()).collect();
        assert_eq!(rounds, [0, 1, 2, 4, 8]);
        assert_eq!(plan(MAX_EFFORT).rounds(), 1 << 31);
    }
}
