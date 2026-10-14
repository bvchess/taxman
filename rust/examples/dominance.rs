//! Dominance and symmetry among promotion sets: how much of a proof's tree could be pruned because another promotion
//! set does at least as well with every completion.  Research code: nothing in the solver calls it.  The study is in
//! docs/EXPERIMENTS.md, "Dominance and symmetry (2026-10)".
//!
//! ```text
//! cargo run --release --example dominance -- sample [--known FILE] [--count K] [--delta D] [--out DIR] N...
//! cargo run --release --example dominance -- nodes [--pairs] [--samples K] [--delta D] FIXTURE...
//! cargo run --release --example dominance -- symmetries N|A-B...
//! cargo run --release --example dominance -- symcheck [--known FILE] [--samples K] N|A-B...
//! ```
//!
//! Games are set up as `taxman solve` sets them up (the default tactics).  `P*` is the optimum promotion sum, the
//! known optimal game's promotions; `--delta D` sets the incumbent's value to `P* + D` (every bound lowered by `D`,
//! as `slope prove --deltas` does).  Each command prints one JSON object per line.
//!
//! * **sample**: nodes of the solver's own proof, uniformly over its steps (see [`sample_one`]), as a `.nodes`
//!   fixture `nNNNN.nodes` in `--out` (default `.`).
//! * **nodes**: at each node `(P, C)` of a fixture (`testdata/perf/*.nodes`, or `sample`'s), whether it is
//!   dominated: some other promotion set `P'`, worth more, with which every completion of the node that could
//!   beat the incumbent can still be promoted (then `P' ∪ S` beats `P ∪ S` for every such completion `S`, and the
//!   node can be pruned; see "Soundness" below).  The candidates such a completion can use are the node's
//!   *relevant* ones ([`relevant`]).  Tried: `P' = P + a` and `P' = P − b + a` with `a > b` (`a` a candidate the
//!   node has excluded, `b ∈ P`), with `--pairs` two for one and two for two, each judged by the state cover of
//!   [`covers`] (literally for all of `C` and for the relevant candidates, and with moves relabelled) and by
//!   `--samples` random maximal completions (default 64; a cover must take them all, so this is an upper estimate of
//!   what any cover proves); `P'` the image of `P` under a prime exchange ([`covers_mapped`]); classes of twins
//!   among the candidates ([`twins`]); and the rule of the root's prime symmetries ([`symmetry_cut`]).
//! * **symmetries**: the root's prime symmetries of each game (`common/symmetry.rs`).
//! * **symcheck**: their checks ([`symcheck_one`]).
//!
//! **Soundness** of pruning a dominated node: the pruned completions `S` are those with `P' ∪ S` feasible and
//! worth strictly more (or, for a two-for-two exchange of equal sums, larger in the order of sets as binary
//! numbers), so a heaviest feasible set (the largest of them in that order) is never pruned.

#[path = "common/fixtures.rs"]
mod fixtures;
#[path = "common/symmetry.rs"]
mod symmetry;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use symmetry::{cofactors, map_set, primes_to, root_pairs, swap_primes};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Ctx, EngineConfig, Searcher, Start};
use taxman::game::Optimal;
use taxman::matchbound::{MatchBound, MatchGraph};
use taxman::model::{FactorProvider, Frame, FrameSet, Model, Scratch};
use taxman::search::{Prepared, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};

fn setup<const W: usize>(n: u32) -> Prepared<W> {
    Board::<W>::for_n(n).prepare(SearchTactics::default()).expect("prepare")
}

/// The command's options: `--known`, `--count`, `--samples`, `--delta`, `--out`, `--pairs`, and the games or files.
struct Options {
    known: Option<PathBuf>,
    count: u64,
    samples: usize,
    delta: i64,
    out: PathBuf,
    pairs: bool,
    rest: Vec<String>,
}

impl Options {
    fn parse(args: &[String]) -> Options {
        let mut o = Options {
            known: None,
            count: 60,
            samples: 64,
            delta: 0,
            out: PathBuf::from("."),
            pairs: false,
            rest: Vec::new(),
        };
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            let mut value = |name: &str| it.next().unwrap_or_else(|| panic!("{name} needs a value")).clone();
            match arg.as_str() {
                "--known" => o.known = Some(PathBuf::from(value("--known"))),
                "--count" => o.count = value("--count").parse().expect("--count K"),
                "--samples" => o.samples = value("--samples").parse().expect("--samples K"),
                "--delta" => o.delta = value("--delta").parse().expect("--delta D"),
                "--out" => o.out = PathBuf::from(value("--out")),
                "--pairs" => o.pairs = true,
                _ => o.rest.push(arg.clone()),
            }
        }
        o
    }

    /// The games named by `rest`: board sizes and ranges `A-B`.
    fn games(&self) -> Vec<u32> {
        let mut games = Vec::new();
        for arg in &self.rest {
            match arg.split_once('-') {
                Some((a, b)) => games.extend(a.parse::<u32>().expect("A")..=b.parse::<u32>().expect("B")),
                None => games.push(arg.parse::<u32>().expect("a board size")),
            }
        }
        games
    }

    fn optimal(&self) -> Optimal {
        Optimal::built_in().with_results_files(self.known.as_slice()).expect("known games")
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let o = Options::parse(&args.collect::<Vec<_>>());
    match command.as_str() {
        "sample" => {
            let optimal = o.optimal();
            for n in o.games() {
                taxman::with_width!(n, W => sample_one::<W>(n, &optimal, &o));
            }
        }
        "nodes" => {
            for path in &o.rest {
                let path = PathBuf::from(path);
                let text = std::fs::read_to_string(&path).expect("fixture");
                let n: u32 =
                    text.lines().find_map(|l| l.strip_prefix("n ")).and_then(|v| v.trim().parse().ok()).expect("n");
                taxman::with_width!(n, W => nodes_of::<W>(&path, &o));
            }
        }
        "symmetries" => {
            for n in o.games() {
                taxman::with_width!(n, W => symmetries_of::<W>(n));
            }
        }
        "symcheck" => {
            let optimal = o.optimal();
            for n in o.games() {
                taxman::with_width!(n, W => symcheck_one::<W>(n, &optimal, o.samples));
            }
        }
        _ => {
            eprintln!("usage: dominance sample|nodes|symmetries|symcheck ... (see the module docs)");
            std::process::exit(2);
        }
    }
}

/// The optimal game's promotions, which must be a promotable set of the board's candidates.
fn optimal_set<const W: usize>(p: &Prepared<W>, optimal: &Optimal) -> Bits<W> {
    let moves = optimal.moves(p.n).unwrap_or_else(|| panic!("no known game for n={} (--known FILE?)", p.n));
    let set: Bits<W> = moves.iter().copied().filter(|&x| p.candidates.contains(x)).collect();
    assert!(p.model.promote_all_using(&mut Scratch::new(), &p.root, &set).is_some(), "the optimum");
    set
}

// ---------------------------------------------------------------------------------------------------------
// sample

/// Records the units a run of the searcher is handed.
struct Recorder<'a, const W: usize> {
    inner: TaxSearcher<'a, W>,
    log: &'a Mutex<Vec<TaxUnit<W>>>,
}

impl<const W: usize> Searcher<TaxUnit<W>> for Recorder<'_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.log.lock().expect("log").push(unit);
        self.inner.run(unit, ctx);
    }
}

/// The proof's own tree, sampled: the search of game `n` (one thread, the incumbent worth `P* + delta`, so the tree
/// is the same every time) run once to count its steps `T`, then `count` times from the start, stopped after `k·T
/// / (count + 1)` steps for `k = 1..=count` (at the engine's next poll).  The stopped run's checkpoint holds the
/// path it stopped on, and a run resumed from it is handed the deepest unit first: the node it stopped in.  Costs
/// about `count / 2` proofs.
fn sample_one<const W: usize>(n: u32, optimal: &Optimal, o: &Options) {
    let p = setup::<W>(n);
    let incumbent = optimal_set(&p, optimal);
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace::new(&p, &graph, None);
    let payload: Vec<u8> = incumbent.as_words().iter().flat_map(|w| w.to_le_bytes()).collect();
    let best = incumbent.sum().checked_add_signed(o.delta).expect("a margin above zero");
    let start = || Start::Fresh {
        units: vec![TaxUnit { promoted: Bits::<W>::EMPTY, cands: p.candidates }],
        best,
        payload: payload.clone(),
    };
    let tmp = std::env::temp_dir().join(format!("taxman-dominance-{n}-{}.ckpt", std::process::id()));
    let t0 = Instant::now();
    let cfg = EngineConfig { threads: 1, progress: None, ..EngineConfig::default() };
    let total = engine::run(&cfg, start(), b"dominance", |_| TaxSearcher::new(&space)).expect("search").stats.nodes;
    let log = Mutex::new(Vec::new());
    let mut samples = Vec::new();
    for k in 1..=o.count {
        let cfg = EngineConfig {
            threads: 1,
            checkpoint: Some((tmp.clone(), Duration::from_secs(1 << 20))),
            node_limit: Some(k * total / (o.count + 1)),
            progress: None,
            ..EngineConfig::default()
        };
        let stopped = engine::run(&cfg, start(), b"dominance", |_| TaxSearcher::new(&space)).expect("search");
        if stopped.completed {
            continue;
        }
        let resume = EngineConfig { node_limit: Some(stopped.stats.nodes), ..cfg.clone() };
        let make = |_| Recorder { inner: TaxSearcher::new(&space), log: &log };
        engine::run(&resume, Start::Resume(tmp.clone()), b"dominance", make).expect("resume");
        let handed = std::mem::take(&mut *log.lock().expect("log"));
        if let Some(deepest) = handed.iter().max_by_key(|u| u.promoted.len()) {
            samples.push(*deepest);
        }
    }
    let _ = std::fs::remove_file(&tmp);
    let mut text = format!(
        "# Search nodes of game {n}, by `dominance sample --count {} --delta {}`: the node the proof ({total} steps,\n\
         # one thread) is in after k·{total}/{} steps, k = 1..{}.\nn {n}\nincumbent {}\n",
        o.count,
        o.delta,
        o.count + 1,
        o.count,
        fixtures::to_hex(&incumbent)
    );
    for u in &samples {
        text.push_str(&format!("node {} {}\n", fixtures::to_hex(&u.promoted), fixtures::to_hex(&u.cands)));
    }
    let path = o.out.join(format!("n{n:04}.nodes"));
    std::fs::write(&path, text).expect("write the samples");
    eprintln!(
        "n={n}: {} nodes of {total} steps -> {} ({:.1} s)",
        samples.len(),
        path.display(),
        t0.elapsed().as_secs_f64()
    );
}

// ---------------------------------------------------------------------------------------------------------
// Covering states

/// The numbers that are maximal factors of some member of `xs` (within the frames, as `g` has them).
fn factors_of<const W: usize>(g: &FactorProvider, xs: &Bits<W>) -> Bits<W> {
    let mut out = Bits::EMPTY;
    for x in xs.iter() {
        for &q in g.factors(x) {
            out.insert(u32::from(q));
        }
    }
    out
}

/// Whether some member of `xs` other than `except` has `f` as a maximal factor.
fn used_by<const W: usize>(g: &FactorProvider, f: u32, xs: &Bits<W>, except: u32) -> bool {
    g.composites(f).iter().any(|&c| u32::from(c) != except && xs.contains(u32::from(c)))
}

/// The *core* of the frame `moves -> factors` for edits that add moves from `cin` and remove factors from `cout`:
/// moves that can always be taken last are dropped (a move with a factor outside `cout` that no other move and no
/// member of `cin` has), and then the factors outside `cout` that no remaining move nor member of `cin` has, until
/// nothing changes.  For every `G ⊆ cin` and `L ⊆ cout`, the frame with `G` added and `L` removed is winnable exactly
/// when its core is: a dropped move goes last (its private factor is never taken, nor removed), and a dropped
/// factor is never used.
fn core<const W: usize>(g: &FactorProvider, frame: &Frame<W>, cin: &Bits<W>, cout: &Bits<W>) -> (Bits<W>, Bits<W>) {
    let (mut moves, mut factors) = (frame.moves, frame.factors);
    loop {
        let mut changed = false;
        for a in moves.iter().collect::<Vec<_>>() {
            let last = g.factors(a).iter().map(|&q| u32::from(q)).any(|f| {
                factors.contains(f) && !cout.contains(f) && !used_by(g, f, &moves, a) && !used_by(g, f, cin, a)
            });
            if last {
                moves.remove(a);
                changed = true;
            }
        }
        let users = moves | *cin;
        for f in factors.iter().collect::<Vec<_>>() {
            if !cout.contains(f) && !used_by(g, f, &users, u32::MAX) {
                factors.remove(f);
                changed = true;
            }
        }
        if !changed {
            return (moves, factors);
        }
    }
}

/// Whether `other` *covers* `state` for the candidates `cands`: every `S ⊆ cands` that can be promoted from `state`
/// can be promoted from `other`.  Frame by frame, with `cin` the candidates that `S` could promote into the frame
/// and `cout` those it could promote out of it (both as `state` has them): every member of `cout` must be a factor
/// of `other`'s frame, the core of `other`'s frame (see [`core`]) can have no move that the core of `state`'s has
/// not, and must have every factor of `state`'s core that its own moves or the members of `cin` could use.  Then
/// for `G ⊆ cin`, `L ⊆ cout`: `state`'s core with `G` and `L` winnable ⇒ the same with `other`'s moves (fewer) ⇒
/// with only the factors those moves and `G` can use ⇒ with `other`'s factors (more, minus `L`).
///
/// With `relabel`, a move of `other`'s core may also stand in for a different move of `state`'s with the same live
/// factors among those both cores have (a frame's game sees a move only through its factors), each move of
/// `state`'s core standing in for one at most, provided it uses no factor that `other`'s core lacks: then
/// `state`'s core with `G` and `L` winnable ⇒ the same with only the stand-ins and the common factors (the others
/// unused) ⇒ the same graph with `other`'s moves ⇒ with `other`'s factors (more).
fn covers<const W: usize>(
    g: &FactorProvider,
    other: &FrameSet<W>,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    relabel: bool,
) -> bool {
    let (mine, theirs) = (state.frames(), other.frames());
    for h in 0..mine.len() {
        let cin = mine.get(h + 1).map_or(Bits::EMPTY, |above| above.factors & *cands);
        let cout = mine[h].factors & *cands;
        if !cout.is_subset(&theirs[h].factors) {
            return false;
        }
        if mine[h] == theirs[h] {
            continue;
        }
        let (m1, f1) = core(g, &mine[h], &cin, &cout);
        let (m2, f2) = core(g, &theirs[h], &cin, &cout);
        if !relabel {
            if !m2.is_subset(&m1) || !(f1 & factors_of(g, &(m2 | cin))).is_subset(&f2) {
                return false;
            }
            continue;
        }
        if !(f1 & factors_of(g, &cin)).is_subset(&f2) {
            return false;
        }
        let (common, only_mine) = (f1 & f2, f1 - f2);
        let key =
            |x: u32| -> Vec<u16> { g.factors(x).iter().copied().filter(|&q| common.contains(u32::from(q))).collect() };
        let mut spare: std::collections::HashMap<Vec<u16>, i32> = Default::default();
        for x in m1.iter().filter(|&x| !g.factors(x).iter().any(|&q| only_mine.contains(u32::from(q)))) {
            *spare.entry(key(x)).or_default() += 1;
        }
        for x in m2.iter() {
            let slot = spare.entry(key(x)).or_default();
            *slot -= 1;
            if *slot < 0 {
                return false;
            }
        }
    }
    true
}

/// [`covers`] up to the renaming `pi` (an involution of the numbers that fixes every member of `cands`): whether
/// `other` covers `state` with every number of `state`'s cores renamed by `pi`.  Frame by frame, `pi` must carry the
/// factor graph between the moves of `state`'s core (and the members of `cands` that could be promoted into the
/// frame) and the factors of its core onto the graph between their images: then the renamed core is the same game
/// for every `G ⊆ cin`, `L ⊆ cout` (which `pi` fixes), and the literal test of [`covers`] applies to it.
fn covers_mapped<const W: usize>(
    g: &FactorProvider,
    other: &FrameSet<W>,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    pi: &impl Fn(u32) -> u32,
) -> bool {
    if cands.iter().any(|x| pi(x) != x) {
        return false;
    }
    let (mine, theirs) = (state.frames(), other.frames());
    for h in 0..mine.len() {
        let cin = mine.get(h + 1).map_or(Bits::EMPTY, |above| above.factors & *cands);
        let cout = mine[h].factors & *cands;
        if !cout.is_subset(&theirs[h].factors) {
            return false;
        }
        let (m1, f1) = core(g, &mine[h], &cin, &cout);
        let (m2, f2) = core(g, &theirs[h], &cin, &cout);
        let (Some(pm1), Some(pf1)) = (map_set(&m1, pi), map_set(&f1, pi)) else { return false };
        for x in (m1 | cin).iter() {
            let live: Bits<W> = g.factors(x).iter().map(|&q| u32::from(q)).filter(|&q| f1.contains(q)).collect();
            let image: Bits<W> = g.factors(pi(x)).iter().map(|&q| u32::from(q)).filter(|&q| pf1.contains(q)).collect();
            if map_set(&live, pi) != Some(image) {
                return false;
            }
        }
        if !m2.is_subset(&pm1) || !(pf1 & factors_of(g, &(m2 | cin))).is_subset(&f2) {
            return false;
        }
    }
    true
}

/// The classes of *twins* among `cands` in `state`: candidates of the same home frame `h` with the same live
/// maximal factors in frame `h − 1` and the same users in frame `h` (the moves of the core of frame `h` for
/// completions within `cands`, and the members of `cands` that could be promoted into it).  Exchanging two twins is
/// an automorphism of the node's problem restricted to `cands`.  The members of the classes of two or more.
fn twins<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>) -> usize {
    let g = model.graph();
    let frames = state.frames();
    let users: Vec<Bits<W>> = (0..frames.len())
        .map(|h| {
            let cin = frames.get(h + 1).map_or(Bits::EMPTY, |above| above.factors & *cands);
            core(g, &frames[h], &cin, &(frames[h].factors & *cands)).0 | cin
        })
        .collect();
    let mut classes: std::collections::HashMap<(usize, Vec<u32>, Vec<u32>), usize> = Default::default();
    for x in cands.iter() {
        let h = model.home(x);
        if h == 0 || !frames[h].factors.contains(x) {
            continue;
        }
        let below = frames[h - 1].factors;
        let down: Vec<u32> = g.factors(x).iter().map(|&q| u32::from(q)).filter(|&q| below.contains(q)).collect();
        let up: Vec<u32> = g.composites(x).iter().map(|&c| u32::from(c)).filter(|&c| users[h].contains(c)).collect();
        *classes.entry((h, down, up)).or_default() += 1;
    }
    classes.into_values().filter(|&c| c > 1).sum()
}

// ---------------------------------------------------------------------------------------------------------
// nodes

/// The candidates of the node `(promoted, cands)` that a completion adding `need` could promote, by the matching
/// bound: `x` is left out when it cannot be promoted, or the bound with `x` promoted, plus `x`, is below `need`.
/// `None` if the node's own bound is below `need`.
fn relevant<const W: usize>(
    m: &mut MatchBound<'_, W>,
    model: &Model,
    scratch: &mut Scratch<W>,
    state: &FrameSet<W>,
    promoted: &Bits<W>,
    cands: &Bits<W>,
    need: u64,
) -> Option<Bits<W>> {
    m.reset(promoted, cands);
    if m.value() < need {
        return None;
    }
    let chosen = m.chosen();
    let mut out = chosen;
    for x in (*cands - chosen).iter().filter(|&x| model.can_promote_using(scratch, state, x)) {
        m.push(x, &cands.without(x));
        if m.value() + u64::from(x) >= need {
            out.insert(x);
        }
        m.pop();
    }
    Some(out)
}

/// A step of xorshift64*.
fn next(rng: &mut u64) -> u64 {
    *rng ^= *rng >> 12;
    *rng ^= *rng << 25;
    *rng ^= *rng >> 27;
    rng.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

/// A maximal subset of `cands` that can be promoted from `state`: the candidates in a random order, each kept if
/// it can be promoted after those kept.
fn random_maximal<const W: usize>(
    model: &Model,
    scratch: &mut Scratch<W>,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    rng: &mut u64,
) -> Bits<W> {
    let mut order: Vec<u32> = cands.iter().collect();
    for i in (1..order.len()).rev() {
        order.swap(i, (next(rng) % (i as u64 + 1)) as usize);
    }
    let mut now = *state;
    order.into_iter().filter(|&x| model.promote_in_place_using(scratch, &mut now, x)).collect()
}

/// The rule of the root's prime symmetries at the node `(promoted, cands)` that needs `need`, with the matching
/// bound: for a pair `(a, b)` (see `common/symmetry.rs`), a set worth beating the incumbent with `c_a > c_b` has
/// an image worth more, so only completions `S` with `c_a(P ∪ S) ≤ c_b(P ∪ S)` count.  `Some(true)` if for some
/// pair no completion can lift `c_b` to `c_a` (`hopeless`), `Some(false)` if the matching bound of the completions
/// that keep some pair's constraint is below `need` (each subset `U` of the pair's multiples among `cands`, at most
/// 12 of them, promoted if it can be and the others left out, the rest relaxed), `None` if neither.
#[allow(clippy::too_many_arguments)]
fn symmetry_cut<const W: usize>(
    m: &mut MatchBound<'_, W>,
    model: &Model,
    scratch: &mut Scratch<W>,
    state: &FrameSet<W>,
    pairs: &[(u32, u32)],
    promoted: &Bits<W>,
    cands: &Bits<W>,
    need: u64,
) -> Option<bool> {
    for &(a, b) in pairs {
        let (ca, cb) = (cofactors(promoted, a) as i64, cofactors(promoted, b) as i64);
        if ca > cb + cofactors(cands, b) as i64 {
            return Some(true);
        }
    }
    for &(a, b) in pairs {
        let (ca, cb) = (cofactors(promoted, a) as i64, cofactors(promoted, b) as i64);
        let family: Vec<u32> = cands.iter().filter(|x| x.is_multiple_of(a) || x.is_multiple_of(b)).collect();
        if family.len() > 12 {
            continue;
        }
        let rest = *cands - family.iter().copied().collect::<Bits<W>>();
        let reaches = (0u32..1 << family.len()).any(|mask| {
            let u: Bits<W> = family.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).map(|(_, &x)| x).collect();
            if ca + cofactors(&u, a) as i64 > cb + cofactors(&u, b) as i64 {
                return false;
            }
            if model.promote_all_using(scratch, state, &u).is_none() {
                return false; // no completion promotes U
            }
            m.reset(&(*promoted | u), &rest);
            m.value() + u.sum() >= need
        });
        if !reaches {
            return Some(false);
        }
    }
    None
}

/// What [`nodes_of`] found at one node, per kind of exchange (add, swap, pairs): proved by a literal cover for all
/// of `C`, for the relevant candidates, with moves relabelled; and not refuted by the sampled completions.
#[derive(Default)]
struct Found {
    cover_all: [bool; 3],
    cover_relevant: [bool; 3],
    cover_relabelled: [bool; 3],
    sampled: [bool; 3],
    prime_cover: bool,
    prime_sampled: bool,
}

fn nodes_of<const W: usize>(path: &Path, o: &Options) {
    let (n, incumbent, units) = fixtures::read_nodes::<W>(path).expect("fixture");
    let p = setup::<W>(n);
    let (model, root) = (&*p.model, &p.root);
    let g = model.graph();
    let graph = MatchGraph::new(model, root);
    let mut m = MatchBound::new(&graph);
    let mut scratch = Scratch::new();
    let best = incumbent.sum().checked_add_signed(o.delta).expect("a margin above zero");
    let primes = primes_to(n / 2);
    let pairs = root_pairs(&p);
    let t0 = Instant::now();
    for unit in &units {
        let (promoted, cands) = (unit.promoted, unit.cands);
        let state = model.promote_all_using(&mut scratch, root, &promoted).expect("a node of the proof");
        let need = (best + 1).saturating_sub(promoted.sum());
        let Some(rel) = relevant(&mut m, model, &mut scratch, &state, &promoted, &cands, need) else {
            println!("{{\"n\":{n},\"promoted\":{},\"closed\":true}}", promoted.len());
            continue;
        };
        let excluded = p.candidates - promoted - cands;
        let mut rng = 0x2545_f491_4f6c_dd1du64 ^ promoted.sum();
        let maximal: Vec<Bits<W>> =
            (0..o.samples).map(|_| random_maximal(model, &mut scratch, &state, &rel, &mut rng)).collect();
        let mut f = Found::default();
        let mut check = |set: Bits<W>, kind: usize, scratch: &mut Scratch<W>| {
            let Some(other) = model.promote_all_using(scratch, root, &set) else { return };
            let takes_all = maximal.iter().all(|s| model.promote_all_using(scratch, &other, s).is_some());
            let proved = [
                covers(g, &other, &state, &cands, false),
                covers(g, &other, &state, &rel, false),
                covers(g, &other, &state, &rel, true),
            ];
            assert!(takes_all || !proved.iter().any(|&c| c), "n={n}: a cover refuted by a completion: {set:?}");
            f.sampled[kind] |= takes_all;
            f.cover_all[kind] |= proved[0];
            f.cover_relevant[kind] |= proved[1];
            f.cover_relabelled[kind] |= proved[2];
        };
        for a in excluded.iter() {
            check(promoted.with(a), 0, &mut scratch);
        }
        for b in promoted.iter() {
            for a in excluded.iter().filter(|&a| a > b) {
                check(promoted.without(b).with(a), 1, &mut scratch);
            }
        }
        if o.pairs {
            let (ex, pr): (Vec<u32>, Vec<u32>) = (excluded.to_vec(), promoted.to_vec());
            for (i, &a1) in ex.iter().enumerate() {
                for &a2 in &ex[i + 1..] {
                    let added = Bits::EMPTY.with(a1).with(a2);
                    for &b in pr.iter().filter(|&&b| u64::from(b) < added.sum()) {
                        check((promoted | added).without(b), 2, &mut scratch);
                    }
                    for (j, &b1) in pr.iter().enumerate() {
                        for &b2 in &pr[j + 1..] {
                            let taken = Bits::EMPTY.with(b1).with(b2);
                            let more =
                                added.sum() > taken.sum() || (added.sum() == taken.sum() && added.max() > taken.max());
                            if more {
                                check((promoted - taken) | added, 2, &mut scratch);
                            }
                        }
                    }
                }
            }
        }
        // Prime exchanges of P (any two primes, not only the root's symmetries), worth more.
        for (i, &a) in primes.iter().enumerate().filter(|&(_, &a)| promoted.iter().any(|x| x.is_multiple_of(a))) {
            for &b in &primes[i + 1..] {
                let pi = |x: u32| swap_primes(x, a, b);
                let Some(set) = map_set(&promoted, &pi) else { continue };
                if set.sum() <= promoted.sum() || !set.is_subset(&p.candidates) || set.intersects(&rel) {
                    continue;
                }
                let Some(other) = model.promote_all_using(&mut scratch, root, &set) else { continue };
                let takes_all = maximal.iter().all(|s| model.promote_all_using(&mut scratch, &other, s).is_some());
                let proved = covers_mapped(g, &other, &state, &rel, &pi);
                assert!(takes_all || !proved, "n={n}: a mapped cover refuted by a completion");
                f.prime_sampled |= takes_all;
                f.prime_cover |= proved;
            }
        }
        let symmetry = symmetry_cut(&mut m, model, &mut scratch, &state, &pairs, &promoted, &rel, need);
        println!(
            "{{\"n\":{n},\"promoted\":{},\"cands\":{},\"relevant\":{},\"excluded\":{},\"cover_all\":{:?},\
             \"cover_relevant\":{:?},\"cover_relabelled\":{:?},\"sampled\":{:?},\"prime_cover\":{},\"prime_sampled\":{},\
             \"twins\":{},\"twins_relevant\":{},\"symmetry_hopeless\":{},\"symmetry_bound\":{}}}",
            promoted.len(),
            cands.len(),
            rel.len(),
            excluded.len(),
            f.cover_all,
            f.cover_relevant,
            f.cover_relabelled,
            f.sampled,
            f.prime_cover,
            f.prime_sampled,
            twins(model, &state, &cands),
            twins(model, &state, &rel),
            symmetry == Some(true),
            symmetry.is_some(),
        );
    }
    eprintln!("n={n}: {} nodes in {:.1} s", units.len(), t0.elapsed().as_secs_f64());
}

// ---------------------------------------------------------------------------------------------------------
// symmetries and their checks

/// The pairs of primes whose exchange is a symmetry of the root of game `n` (see `common/symmetry.rs`), and how
/// many candidates they move.
fn symmetries_of<const W: usize>(n: u32) {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return;
    }
    let p = board.prepare(SearchTactics::default()).expect("prepare");
    let pairs = root_pairs(&p);
    let moved: Bits<W> =
        p.candidates.iter().filter(|&x| pairs.iter().any(|&(a, b)| swap_primes(x, a, b) != x)).collect();
    println!(
        "{{\"n\":{n},\"candidates\":{},\"symmetric_pairs\":{},\"moved\":{},\"pairs\":\"{pairs:?}\"}}",
        p.candidates.len(),
        pairs.len(),
        moved.len()
    );
}

/// Checks of the root's prime symmetries of game `n`: for every pair `(a, b)`, the image of the optimal set and of
/// `samples` random maximal promotable sets (and, on boards of at most 22 candidates, of every promotable set) can
/// be promoted and is worth `(b − a)(c_a − c_b)` more; on those small boards, the best set that keeps every pair's
/// constraint `c_a ≤ c_b` is worth the optimum; and up to `n = 40`, the brute-force oracle (`taxman::brute`, no
/// frame model) scores the known optimum.  Panics on a failure; prints a line per game with pairs.
fn symcheck_one<const W: usize>(n: u32, optimal: &Optimal, samples: usize) {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return;
    }
    if n <= 40 {
        let exact = taxman::brute::solve(n, &[], taxman::brute::Rule::Divisor).score;
        assert_eq!(Some(exact), optimal.score(n), "n={n}: the oracle disagrees with the known optimum");
    }
    let p = board.prepare(SearchTactics::default()).expect("prepare");
    let pairs = root_pairs(&p);
    if pairs.is_empty() {
        return;
    }
    let (model, root) = (&*p.model, &p.root);
    let mut scratch = Scratch::new();
    let best = optimal_set(&p, optimal);
    let opt = best.sum();
    let check = |set: &Bits<W>, scratch: &mut Scratch<W>| {
        for &(a, b) in &pairs {
            let image = map_set(set, &|x| swap_primes(x, a, b)).expect("in range");
            assert!(model.promote_all_using(scratch, root, &image).is_some(), "n={n}: ({a}, {b}) of {set:?}");
            let gain = i64::from(b - a) * (cofactors(set, a) as i64 - cofactors(set, b) as i64);
            assert_eq!(image.sum() as i64 - set.sum() as i64, gain, "n={n}: the sum of the image of {set:?}");
        }
    };
    check(&best, &mut scratch);
    let mut rng = 0x5eed_u64 ^ u64::from(n);
    for _ in 0..samples {
        let s = random_maximal(model, &mut scratch, root, &p.candidates, &mut rng);
        check(&s, &mut scratch);
    }
    let (mut enumerated, mut kept_best) = (0u64, None);
    if p.candidates.len() <= 22 {
        let cands = p.candidates.to_vec();
        let mut stack = vec![(0usize, Bits::<W>::EMPTY, *root)];
        while let Some((i, set, state)) = stack.pop() {
            if i == cands.len() {
                enumerated += 1;
                check(&set, &mut scratch);
                if pairs.iter().all(|&(a, b)| cofactors(&set, a) <= cofactors(&set, b)) {
                    kept_best = kept_best.max(Some(set.sum()));
                }
                continue;
            }
            stack.push((i + 1, set, state));
            if let Some(next) = model.promote(&state, cands[i]) {
                stack.push((i + 1, set.with(cands[i]), next));
            }
        }
        assert_eq!(kept_best, Some(opt), "n={n}: the sets that keep the constraints miss the optimum");
    }
    println!(
        "{{\"n\":{n},\"pairs\":{},\"candidates\":{},\"enumerated\":{enumerated},\"kept_best\":{},\"opt\":{opt}}}",
        pairs.len(),
        p.candidates.len(),
        kept_best.map_or("null".into(), |v: u64| v.to_string()),
    );
}
