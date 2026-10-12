//! Helpers shared by the integration tests: temporary directories, the committed results files as oracles beyond
//! `optimal.json`, one-thread searches at a chosen set width, and a few small utilities.
//!
//! Each test binary compiles this module for itself and uses only part of it.
#![allow(dead_code)]

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use taxman::bits::Bits;
use taxman::bounds::{Preclusions, RunnerUp};
use taxman::engine::{self, EngineConfig, Start};
use taxman::frames::build_frames;
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameSet, Model, Scratch};
use taxman::numtheory::Tables;
use taxman::search::{prepare, search, Branching, SearchSpace, SearchTactics, TaxSearcher, TaxUnit, NOGOOD_PERIOD};
use taxman::solver::{Prev, PrevSource, SolveOptions};

/// A fresh directory under the system temp dir, removed on drop.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("taxman-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs `$body` with the const generic `$w` set to the width the board `$n` needs (as `solver::solve_game` picks it):
/// `W = 8` up to n = 511, 16 up to 1023, 32 up to 2047, 64 up to 4095.
#[macro_export]
macro_rules! with_width {
    ($n:expr, $w:ident => $body:expr) => {
        match taxman::bits::words_for($n) {
            1..=8 => {
                const $w: usize = 8;
                $body
            }
            9..=16 => {
                const $w: usize = 16;
                $body
            }
            17..=32 => {
                const $w: usize = 32;
                $body
            }
            33..=64 => {
                const $w: usize = 64;
                $body
            }
            _ => panic!("n={} is beyond the widest set", $n),
        }
    };
}

/// A committed file under `testdata/`.
pub fn testdata(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join(path)
}

/// A committed results file under `results/`.
pub fn results_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("results").join(name)
}

/// One game of a results file: its score and moves.
pub type StoredGames = BTreeMap<u32, (u64, Vec<u32>)>;

/// The completed games of a committed results file, by n.  Read-only (unlike `ResultsFile::open`, which repairs the
/// file), so a test never touches a tracked file.
pub fn stored_games(name: &str) -> StoredGames {
    let text = fs::read_to_string(results_path(name)).expect("results file");
    let mut games = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let json = Json::parse(line).expect("a results line");
        if json.get("completed").and_then(Json::as_bool) != Some(true) {
            continue;
        }
        let n = json.get("n").and_then(Json::as_u32).expect("n");
        let score = json.get("score").and_then(Json::as_u64).expect("score");
        let moves = json.get("moves").and_then(Json::as_u32_vec).expect("moves");
        assert!(games.insert(n, (score, moves)).is_none(), "{name}: n={n} twice");
    }
    games
}

/// The game `n − 1` of a results file as the previous game of `n`.
pub fn prev_of(games: &StoredGames, n: u32) -> Prev {
    let (score, moves) = &games[&(n - 1)];
    Prev { moves: moves.clone(), score: *score, source: PrevSource::Results }
}

/// The options `results/strict.jsonl` was made with (neither unproven tactic that existed then): `reuse_prev` and
/// `revise_freedoms` off.
pub fn strict_options(threads: usize) -> SolveOptions {
    let mut opts = SolveOptions { threads, ..SolveOptions::default() };
    opts.tactics.set("reuse_prev=off").unwrap();
    opts.tactics.set("revise_freedoms=off").unwrap();
    opts
}

/// The default search tactics but for `revise_freedoms`, which the committed strict results were made without.
pub fn strict_tactics() -> SearchTactics {
    SearchTactics { revise_freedoms: false, ..SearchTactics::default() }
}

/// The search tactics a differential test compares: the strict default and every sound variant that is cheap at
/// n ~ 1100.
pub fn variants() -> Vec<(&'static str, SearchTactics)> {
    let d = strict_tactics();
    vec![
        ("default", d),
        ("nogoods=off", SearchTactics { nogoods: false, ..d }),
        ("fix_forced=off", SearchTactics { fix_forced: false, ..d }),
        ("forget_nogoods=off", SearchTactics { forget_nogoods: false, ..d }),
        ("match_refine=0", SearchTactics { match_refine: 0, ..d }),
        ("match_refine=3", SearchTactics { match_refine: 3, ..d }),
        ("conflict-smallest", SearchTactics { branching: Branching::ConflictSmallest, ..d }),
        ("conflict-largest", SearchTactics { branching: Branching::ConflictLargest, ..d }),
    ]
}

pub fn config(threads: usize) -> EngineConfig {
    EngineConfig { threads, progress: None, ..EngineConfig::default() }
}

/// What a search of a game from the frame builder looks like at set width `W`: everything that must not depend on
/// `W`, from the frames to the nodes of the tree.  One thread, so the tree is deterministic.
#[derive(Debug, PartialEq, Eq)]
pub struct Profile {
    pub columns: Vec<Vec<u32>>,
    pub candidates: Vec<u32>,
    pub greedy: Vec<u32>,
    pub max_total: u32,
    pub frame_zero_freedoms: i32,
    pub revise: String,
    pub completed: bool,
    pub sum: u64,
    pub promotions: Vec<u32>,
    pub nodes: u64,
}

/// The game `n`'s [`Profile`] instantiated at width `W`, which may be wider than `n` needs.  With `node_limit` the
/// search may stop early (and the profile then says `completed: false`).
pub fn profile_at_width<const W: usize>(n: u32, tactics: SearchTactics, node_limit: Option<u64>) -> Profile {
    let tables = Tables::new(n);
    let frames = build_frames::<W>(&tables);
    let p = prepare::<W>(&tables, &frames.columns(), &frames.candidates, tactics, 1).unwrap();
    let cfg = EngineConfig { node_limit, ..config(1) };
    let r = search(&p, &cfg, None, None, false).unwrap();
    Profile {
        columns: frames.columns(),
        candidates: frames.candidates.to_vec(),
        greedy: p.greedy.to_vec(),
        max_total: p.max_total,
        frame_zero_freedoms: p.root.frames().first().map_or(0, |f| f.freedoms),
        revise: format!("{:?}", p.revise),
        completed: r.completed,
        sum: r.sum,
        promotions: r.promotions.to_vec(),
        nodes: r.stats.nodes,
    }
}

/// (promotion sum, nodes) of the whole search of game `n` from the frame builder at width `W`, on one thread.
pub fn search_at_width<const W: usize>(n: u32, tactics: SearchTactics) -> (u64, u64) {
    let tables = Tables::new(n);
    let frames = build_frames::<W>(&tables);
    let p = prepare::<W>(&tables, &frames.columns(), &frames.candidates, tactics, 1).unwrap();
    let r = search(&p, &config(1), None, None, false).unwrap();
    assert!(r.completed);
    (r.sum, r.stats.nodes)
}

/// Tiny xorshift64 generator, so the tests need no crates and are reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A number in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// True with probability `percent`/100.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len() as u64) as usize]
    }

    /// A random subset of `set`, each member kept with probability `percent`/100.
    pub fn subset<const W: usize>(&mut self, set: &Bits<W>, percent: u64) -> Bits<W> {
        set.iter().filter(|_| self.chance(percent)).collect()
    }
}

/// FNV-1a over the decimal text of the moves, each followed by a comma: the fingerprint of a game that the Java
/// fixtures (`testdata/accel_java.txt`, `testdata/games_java.txt`) record.
pub fn fnv1a64(moves: &[u32]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for m in moves {
        for byte in format!("{m},").bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01B3);
        }
    }
    hash
}

// ---------------------------------------------------------------------------------------------------------
// Searching a game, or a part of one, with the searcher's own parts

/// A search configuration beyond what [`SearchTactics`] and the solver offer: how often the searchers forget their
/// nogoods, and how many threads there are.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub matching: bool,
    pub refine: u32,
    pub branching: Branching,
    pub fix_forced: bool,
    pub nogoods: bool,
    /// After how many of its steps a searcher forgets its nogoods; `None` for never.
    pub period: Option<u64>,
    pub preclusions: bool,
    pub threads: usize,
}

impl Config {
    /// The default tactics on one thread.
    pub fn standard() -> Config {
        Config::of(&SearchTactics::default())
    }

    pub fn of(t: &SearchTactics) -> Config {
        Config {
            matching: t.match_bound,
            refine: u32::from(t.match_refine),
            branching: t.branching,
            fix_forced: t.fix_forced,
            nogoods: t.nogoods,
            period: t.forget_nogoods.then_some(NOGOOD_PERIOD),
            preclusions: t.preclusions,
            threads: 1,
        }
    }
}

/// How a search from some units ended.
#[derive(Debug)]
pub struct Outcome<const W: usize> {
    pub best: u64,
    /// The incumbent's payload: the promotion set that reaches `best`.
    pub set: Bits<W>,
    /// Whether `set` really is a promotable set of candidates worth `best` (promoted at once, and one by one).
    pub real: bool,
    pub completed: bool,
    pub nodes: u64,
}

/// The parts of a search of one game: the model, the starting state, the candidates and what the bounds need.
/// Searching from a unit with an incumbent worth less than the best completion (and a payload that is not a
/// completion) tests every pruning at once: if one loses every best completion, the fake incumbent survives.
pub struct Game<const W: usize> {
    pub model: Model,
    pub root: FrameSet<W>,
    pub candidates: Bits<W>,
    pub max_total: u32,
    graph: MatchGraph,
    preclusions: OnceCell<Preclusions<W>>,
}

impl<const W: usize> Game<W> {
    /// The game that starts at `root` and may promote `candidates`.
    pub fn new(model: Model, root: FrameSet<W>, candidates: Bits<W>) -> Self {
        let max_total = root.max_promotions_among(&candidates).max(0) as u32;
        let graph = MatchGraph::new(&model, &root);
        Game { model, root, candidates, max_total, graph, preclusions: OnceCell::new() }
    }

    /// The game with the frames `columns` (factors and moves per frame, bottom first), if every frame of the root is
    /// winnable.
    pub fn from_columns(tables: &Tables, columns: &[Vec<u32>], candidates: Bits<W>) -> Option<Self> {
        let (model, root) = Model::build::<W>(tables, columns).ok()?;
        let winnable = root.frames().iter().all(|f| model.winnable(&f.moves, &f.factors));
        winnable.then(|| Game::new(model, root, candidates))
    }

    /// Game `n` from the frame builder, with the frame builder's candidates (none: `None`).
    pub fn of_board(n: u32) -> Option<Self> {
        let tables = Tables::new(n);
        let frames = build_frames::<W>(&tables);
        if frames.candidates.is_empty() {
            return None;
        }
        Game::from_columns(&tables, &frames.columns(), frames.candidates)
    }

    /// The same game with frame 0's freedoms set to `freedoms`, as `revise_freedoms` does (and the promotion cap
    /// and the matching graph redone).
    pub fn with_frame_zero_freedoms(&self, freedoms: i32) -> Self {
        Game::new(self.model.clone(), self.root.with_frame_zero_freedoms(freedoms), self.candidates)
    }

    /// The unit of the root: nothing promoted, all candidates to try.
    pub fn root_unit(&self) -> TaxUnit<W> {
        TaxUnit { promoted: Bits::EMPTY, cands: self.candidates }
    }

    /// Searches from `units` with the incumbent `best` (and an empty payload), or resumes the checkpoint `resume`.
    /// `engine` replaces the default engine configuration for `cfg.threads`.
    pub fn search(
        &self,
        cfg: Config,
        units: Vec<TaxUnit<W>>,
        best: u64,
        engine: Option<EngineConfig>,
        resume: Option<PathBuf>,
    ) -> Outcome<W> {
        let preclusions = cfg
            .preclusions
            .then(|| self.preclusions.get_or_init(|| Preclusions::build(&self.model, &self.root, &self.candidates, 1)));
        let space = SearchSpace {
            model: &self.model,
            root: &self.root,
            preclusions,
            match_graph: cfg.matching.then_some(&self.graph),
            match_refine: cfg.refine,
            conflict_branching: cfg.branching,
            fix_forced: cfg.fix_forced,
            nogoods: cfg.nogoods && cfg.matching,
            nogood_period: cfg.period,
            root_max: self.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(0),
            max_total: self.max_total,
            rule: RunnerUp::Sound,
            cap: None,
            diagnostics: None,
            began: Instant::now(),
            found: Mutex::new(None),
        };
        let engine = engine.unwrap_or_else(|| config(cfg.threads));
        let start = match resume {
            Some(path) => Start::Resume(path),
            None => Start::Fresh { units, best, payload: encode(&Bits::<W>::EMPTY) },
        };
        let out = engine::run(&engine, start, b"test", |_| TaxSearcher::new(&space)).unwrap();
        let set = decode::<W>(&out.payload);
        Outcome {
            best: out.best,
            set,
            real: self.is_real(&set, out.best),
            completed: out.completed,
            nodes: out.stats.nodes,
        }
    }

    /// Whether `set` is made of candidates, worth `value`, and can be promoted all at once and one by one, leaving
    /// no frame with negative freedoms (frame 0's may have been lowered, which the model's promotions do not
    /// check: the search does, by narrowing its candidates).
    pub fn is_real(&self, set: &Bits<W>, value: u64) -> bool {
        let mut scratch = Scratch::new();
        let mut state = self.root;
        set.sum() == value
            && set.is_subset(&self.candidates)
            && self.model.promote_all_using(&mut scratch, &self.root, set).is_some()
            && set.iter().all(|x| self.model.promote_in_place(&mut state, x))
            && state.frames().iter().all(|f| f.freedoms >= 0)
    }

    /// The best completion of `state` among `cands`, found exhaustively (pruned by the sum and the freedoms'
    /// count; a number is tried only where both its frames have freedoms left, as the search narrows its
    /// candidates), or `None` past `budget` nodes.
    pub fn exhaustive(&self, state: &FrameSet<W>, cands: &Bits<W>, budget: u64) -> Option<u64> {
        fn dfs<const W: usize>(
            model: &Model,
            state: &FrameSet<W>,
            list: &[u32],
            sum: u64,
            best: &mut u64,
            nodes: &mut u64,
            budget: u64,
        ) -> bool {
            *nodes += 1;
            if *nodes > budget {
                return false;
            }
            *best = (*best).max(sum);
            let left: Bits<W> = list.iter().copied().collect();
            let cap = state.max_promotions_among(&left).max(0) as usize;
            let open = state.narrow_candidates(&left);
            for i in 0..list.len() {
                let optimistic: u64 = sum + list[i..].iter().take(cap).map(|&x| u64::from(x)).sum::<u64>();
                if optimistic <= *best {
                    return true;
                }
                let mut next = *state;
                if open.contains(list[i])
                    && model.promote_in_place(&mut next, list[i])
                    && !dfs(model, &next, &list[i + 1..], sum + u64::from(list[i]), best, nodes, budget)
                {
                    return false;
                }
            }
            true
        }
        let list: Vec<u32> = cands.iter_desc().collect();
        let (mut best, mut nodes) = (0, 0);
        dfs(&self.model, state, &list, 0, &mut best, &mut nodes, budget).then_some(best)
    }
}

fn encode<const W: usize>(set: &Bits<W>) -> Vec<u8> {
    set.as_words().iter().flat_map(|w| w.to_le_bytes()).collect()
}

fn decode<const W: usize>(bytes: &[u8]) -> Bits<W> {
    let mut words = [0u64; W];
    for (w, chunk) in words.iter_mut().zip(bytes.chunks(8)) {
        *w = u64::from_le_bytes(chunk.try_into().unwrap());
    }
    Bits::from_words(words)
}
