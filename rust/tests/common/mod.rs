//! Helpers shared by the integration tests: temporary directories and a random number generator (`support.rs`, which
//! the unit tests share), the Java's golden data and the committed results files as oracles beyond `optimal.json`,
//! one-thread searches at a chosen set width, and a few small utilities.
//!
//! Each test binary compiles this module for itself and uses only part of it.
#![allow(dead_code)]

mod support;

#[allow(unused_imports)]
pub use support::{Rng, TempDir};

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, EngineConfig, Start};
use taxman::gf2::Gf2Graph;
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::numtheory::Tables;
use taxman::results::Assumptions;
use taxman::search::lagrange::Refits;
use taxman::search::symmetry::{cofactors, Symmetries};
use taxman::search::{search, Gf2Branching, SearchSpace, SearchTactics, TaxSearcher, TaxUnit, NOGOOD_PERIOD};
use taxman::solver::{Prev, PrevSource, SolveOptions};

/// A committed file under `testdata/`.
pub fn testdata(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join(path)
}

/// The Java's golden data for game `n`: `testdata/golden/n<NNNN>.json` (made by `tools/golden/export.sh`).
pub fn golden(n: u32) -> Json {
    Json::from_file(testdata(&format!("golden/n{n:04}.json"))).unwrap_or_else(|e| panic!("n={n}: {e}"))
}

/// Every `n*.json` under `testdata/<dir>` (`golden`, or `golden_wide`, which holds the structure only), as `(n,
/// data)` by n.  A file in which the Java export reported an error fails the test.
pub fn golden_files(dir: &str) -> Vec<(u32, Json)> {
    let dir = testdata(dir);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let json = Json::from_file(path).unwrap();
            let n = json.get("n").and_then(Json::as_u32).expect("n");
            assert!(json.get("error").is_none(), "n={n}: the Java export reported {:?}", json.get("error"));
            (n, json)
        })
        .collect()
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

/// The game `n − 1` of a committed results file as the previous game of `n`.  The games of `results/strict.jsonl`
/// assume nothing (`results/README.md`); the proof only matters to the results lines.
pub fn prev_of(games: &StoredGames, n: u32) -> Prev {
    let (score, moves) = &games[&(n - 1)];
    Prev { moves: moves.clone(), score: *score, source: PrevSource::Results, proof: None, assumes: Assumptions::new() }
}

/// The default options on `threads` threads.
pub fn options(threads: usize) -> SolveOptions {
    SolveOptions { threads, ..SolveOptions::default() }
}

/// [`options`] with the search tactics of the matching path ([`SearchTactics::matching`], which
/// `results/strict.jsonl` was made with): for the tests that stop a search of a small game by a node limit.
pub fn matching_options(threads: usize) -> SolveOptions {
    let mut opts = options(threads);
    opts.tactics.search = SearchTactics::matching();
    opts
}

/// The search tactics a differential test compares: the default and every sound variant that is cheap at n ~ 1100,
/// then the matching path ([`SearchTactics::matching`], which is without the rule of `symmetry`) and its variants.
pub fn variants() -> Vec<(&'static str, SearchTactics)> {
    let (d, m) = (SearchTactics::default(), SearchTactics::matching());
    vec![
        ("default", d),
        ("nogoods=off", SearchTactics { nogoods: false, ..d }),
        ("fix_forced=off", SearchTactics { fix_forced: false, ..d }),
        ("forget_nogoods=off", SearchTactics { forget_nogoods: false, ..d }),
        ("lagrange=on", SearchTactics { lagrange: true, ..d }),
        ("symmetry=off", SearchTactics { symmetry: false, ..d }),
        ("matching", m),
        ("matching,nogoods=off", SearchTactics { nogoods: false, ..m }),
        ("matching,fix_forced=off", SearchTactics { fix_forced: false, ..m }),
        ("matching,match_refine=0", SearchTactics { match_refine: 0, ..m }),
        ("matching,match_refine=3", SearchTactics { match_refine: 3, ..m }),
        ("matching,symmetry=on", SearchTactics { symmetry: true, ..m }),
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
    pub completed: bool,
    pub sum: u64,
    pub promotions: Vec<u32>,
    pub nodes: u64,
}

/// The game `n`'s [`Profile`] instantiated at width `W`, which may be wider than `n` needs.  With `node_limit` the
/// search may stop early (and the profile then says `completed: false`).
pub fn profile_at_width<const W: usize>(n: u32, tactics: SearchTactics, node_limit: Option<u64>) -> Profile {
    let board = Board::<W>::for_n(n);
    let p = board.prepare(tactics).unwrap();
    let cfg = EngineConfig { node_limit, ..config(1) };
    let r = search(&p, &cfg, None, None, false).unwrap();
    Profile {
        columns: board.frames.columns(),
        candidates: board.candidates().to_vec(),
        greedy: p.greedy.to_vec(),
        max_total: p.max_total,
        frame_zero_freedoms: p.root.frames().first().map_or(0, |f| f.freedoms),
        completed: r.completed,
        sum: r.sum,
        promotions: r.promotions.to_vec(),
        nodes: r.stats.nodes,
    }
}

/// (promotion sum, nodes) of the whole search of game `n` from the frame builder at width `W`, on one thread.
pub fn search_at_width<const W: usize>(n: u32, tactics: SearchTactics) -> (u64, u64) {
    let p = Board::<W>::for_n(n).prepare(tactics).unwrap();
    let r = search(&p, &config(1), None, None, false).unwrap();
    assert!(r.completed);
    (r.sum, r.stats.nodes)
}

/// A random subset of `set`, each member kept with probability `percent`/100.
pub fn random_subset<const W: usize>(rng: &mut Rng, set: &Bits<W>, percent: u64) -> Bits<W> {
    set.iter().filter(|_| rng.chance(percent)).collect()
}

/// FNV-1a over the decimal text of the moves, each followed by a comma: the fingerprint of a game that the Java
/// fixture `testdata/games_java.txt` records.
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
    pub refine: u32,
    pub fix_forced: bool,
    pub nogoods: bool,
    /// After how many of its steps a searcher forgets its nogoods; `None` for never.
    pub period: Option<u64>,
    /// When a searcher refits the multipliers of its Lagrangian bound; `None` without that bound.
    pub refits: Option<Refits>,
    pub threads: usize,
    /// The GF(2) bound (tactic `gf2`), and with it the matching bound as a first check and the branching.
    pub gf2: bool,
    pub gf2_prefilter: bool,
    pub gf2_branching: Gf2Branching,
    /// The rule of the root's symmetric pairs (tactic `symmetry`).
    pub symmetry: bool,
    /// The GF(2) graph padded to that many block words ([`Gf2Graph::padded`]), to run the engines of more words than
    /// the board needs; `None` for the words it needs.
    pub gf2_words: Option<usize>,
}

impl Config {
    /// The default tactics on one thread.
    pub fn standard() -> Config {
        Config::of(&SearchTactics::default())
    }

    /// The tactics of the matching path ([`SearchTactics::matching`]) on one thread.
    pub fn matching() -> Config {
        Config::of(&SearchTactics::matching())
    }

    pub fn of(t: &SearchTactics) -> Config {
        Config {
            refine: u32::from(t.match_refine),
            fix_forced: t.fix_forced,
            nogoods: t.nogoods,
            period: t.forget_nogoods.then_some(NOGOOD_PERIOD),
            refits: (t.nogoods && t.lagrange).then_some(Refits::SEARCH),
            threads: 1,
            gf2: t.gf2,
            gf2_prefilter: false,
            gf2_branching: Gf2Branching::Exact,
            symmetry: t.symmetry,
            gf2_words: None,
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
    pub model: FrameGraph,
    pub root: FrameSet<W>,
    pub candidates: Bits<W>,
    pub max_total: u32,
    graph: MatchGraph,
}

impl<const W: usize> Game<W> {
    /// The game that starts at `root` and may promote `candidates`.
    pub fn new(model: FrameGraph, root: FrameSet<W>, candidates: Bits<W>) -> Self {
        let max_total = root.max_promotions_among(&candidates).max(0) as u32;
        let graph = MatchGraph::new(&model, &root);
        Game { model, root, candidates, max_total, graph }
    }

    /// The game with the frames `columns` (factors and moves per frame, front first), if every frame of the root is
    /// winnable.
    pub fn from_columns(tables: &Tables, columns: &[Vec<u32>], candidates: Bits<W>) -> Option<Self> {
        let (model, root) = FrameGraph::build::<W>(tables, columns).ok()?;
        let winnable = root.frames().iter().all(|f| model.winnable(&f.moves, &f.factors));
        winnable.then(|| Game::new(model, root, candidates))
    }

    /// Game `n` from the frame builder, with the frame builder's candidates (none: `None`).
    pub fn of_board(n: u32) -> Option<Self> {
        let board = Board::<W>::for_n(n);
        if board.is_trivial() {
            return None;
        }
        Game::from_columns(&board.tables, &board.frames.columns(), board.candidates())
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
        let space = SearchSpace {
            model: &self.model,
            root: &self.root,
            match_graph: &self.graph,
            match_refine: cfg.refine,
            fix_forced: cfg.fix_forced,
            nogoods: cfg.nogoods,
            nogood_period: cfg.period,
            lagrange: cfg.refits.filter(|_| cfg.nogoods),
            candidates: self.candidates,
            root_max: self.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(0),
            max_total: self.max_total,
            diagnostics: None,
            began: Instant::now(),
            found: Mutex::new(None),
            gf2: cfg.gf2.then(|| {
                let graph = Gf2Graph::new(&self.model, &self.root, &self.candidates);
                match cfg.gf2_words {
                    Some(words) => graph.padded(words),
                    None => graph,
                }
            }),
            gf2_prefilter: cfg.gf2_prefilter,
            gf2_branching: cfg.gf2_branching,
            gf2_strong_limit: usize::MAX,
            gf2_more_forced: true,
            symmetry: cfg.symmetry.then(|| self.symmetries()).filter(|pairs| !pairs.is_empty()),
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

    /// Whether `set` is made of candidates, worth `value`, and can be promoted all at once and one by one.
    pub fn is_real(&self, set: &Bits<W>, value: u64) -> bool {
        let mut scratch = Scratch::new();
        let mut state = self.root;
        set.sum() == value
            && set.is_subset(&self.candidates)
            && self.model.promote_all_using(&mut scratch, &self.root, set).is_some()
            && set.iter().all(|x| self.model.promote_in_place(&mut state, x))
    }

    /// The root's symmetric pairs, as the search's rule uses them (empty if there are none).
    pub fn symmetries(&self) -> Symmetries {
        Symmetries::of(&self.model, &self.root, &self.candidates)
    }

    /// The best completion of `state` among `cands`, found exhaustively (pruned by the sum and the freedoms'
    /// count; a number is tried only where both its frames have freedoms left, as the search narrows its
    /// candidates), as its sum and its set (the first of the best ones that the search finds), or `None` past
    /// `budget` nodes.
    pub fn exhaustive(&self, state: &FrameSet<W>, cands: &Bits<W>, budget: u64) -> Option<(u64, Bits<W>)> {
        let best = self.best_completion(state, cands, budget, &|_| true, &|_, _| true);
        best.map(|best| best.expect("the empty completion is accepted"))
    }

    /// [`exhaustive`](Self::exhaustive) over the completions that keep the symmetric pairs' inequalities, for the
    /// node with the promotions `promoted` and the state `state` they reach: those completions `F` for which
    /// `promoted ∪ F` keeps every pair of `sym` ([`Symmetries::keeps`]).  `None` past `budget` nodes, else
    /// `Some(best)`, with `best` the largest sum of such an `F` (not counting `promoted`) and the set `F`, or `None`
    /// if no completion keeps them, the empty one included.
    ///
    /// Until it has found a completion that keeps them the search has no sum to prune by, and where none does it
    /// would take the whole tree, so it also cuts a node where no completion can, by the inequality itself: a pair
    /// `(p, q)` has `c_p(P ∪ F) ≥ c_p(P)` and `c_q(P ∪ F) ≤ c_q(P) + c_q(C)` for every `F` among the candidates `C`
    /// that are left, with `P` the node's promotions (`promoted` and the completion so far; `heuristics/THEORY.md`,
    /// "Pruning with the inequalities"), so `c_p(P) > c_q(P) + c_q(C)` leaves no `F` that keeps `c_p ≤ c_q`.  That is
    /// the cofactor arithmetic alone, not the search's rule ([`Symmetries::hopeless`]).
    pub fn exhaustive_keeping(
        &self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        budget: u64,
        sym: &Symmetries,
    ) -> Option<Option<(u64, Bits<W>)>> {
        let keeps = |completion: &Bits<W>| sym.keeps(&(*promoted | *completion));
        let possible = |completion: &Bits<W>, left: &Bits<W>| {
            let made = *promoted | *completion;
            sym.pairs().iter().all(|&(p, q)| cofactors(&made, p) <= cofactors(&made, q) + cofactors(left, q))
        };
        self.best_completion(state, cands, budget, &keeps, &possible)
    }

    /// The depth-first search of [`exhaustive`](Self::exhaustive): the largest sum of a completion of `state`
    /// among `cands` that `accepts` (the completion as a set), with that completion, `Some(None)` if it accepts
    /// none, `None` past `budget` nodes.  The sum bounds every completion below a node, accepted or not, so it
    /// prunes as before.  A node is cut, with all that it holds, where `possible` says no (given the completion
    /// so far and the candidates left to add to it) that an extension could be accepted.
    fn best_completion(
        &self,
        state: &FrameSet<W>,
        cands: &Bits<W>,
        budget: u64,
        accepts: &dyn Fn(&Bits<W>) -> bool,
        possible: &dyn Fn(&Bits<W>, &Bits<W>) -> bool,
    ) -> Option<Option<(u64, Bits<W>)>> {
        struct Dfs<'a, const W: usize> {
            model: &'a FrameGraph,
            budget: u64,
            accepts: &'a dyn Fn(&Bits<W>) -> bool,
            possible: &'a dyn Fn(&Bits<W>, &Bits<W>) -> bool,
            best: Option<(u64, Bits<W>)>,
            nodes: u64,
        }
        impl<const W: usize> Dfs<'_, W> {
            /// Explores the completions that extend `chosen` (worth `sum`) from `state` with a subset of `list`;
            /// false once the budget is spent.
            fn explore(&mut self, state: &FrameSet<W>, list: &[u32], chosen: Bits<W>, sum: u64) -> bool {
                self.nodes += 1;
                if self.nodes > self.budget {
                    return false;
                }
                let left: Bits<W> = list.iter().copied().collect();
                if !(self.possible)(&chosen, &left) {
                    return true;
                }
                if (self.accepts)(&chosen) && self.best.is_none_or(|(best, _)| sum > best) {
                    self.best = Some((sum, chosen));
                }
                let cap = state.max_promotions_among(&left).max(0) as usize;
                let open = state.narrow_candidates(&left);
                for i in 0..list.len() {
                    let optimistic: u64 = sum + list[i..].iter().take(cap).map(|&x| u64::from(x)).sum::<u64>();
                    if self.best.is_some_and(|(best, _)| optimistic <= best) {
                        return true;
                    }
                    let mut next = *state;
                    if open.contains(list[i])
                        && self.model.promote_in_place(&mut next, list[i])
                        && !self.explore(&next, &list[i + 1..], chosen.with(list[i]), sum + u64::from(list[i]))
                    {
                        return false;
                    }
                }
                true
            }
        }
        let list: Vec<u32> = cands.iter_desc().collect();
        let mut dfs = Dfs { model: &self.model, budget, accepts, possible, best: None, nodes: 0 };
        dfs.explore(state, &list, Bits::EMPTY, 0).then_some(dfs.best)
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
