//! Dominance among promotion sets: how much of a proof's tree a node could be pruned for, because another
//! promotion set does at least as well with every completion.  The study is in docs/EXPERIMENTS.md, "Dominance and
//! symmetry (2026-10)".
//!
//! ```text
//! cargo run --release --example dominance -- nodes [--pairs] FIXTURE...
//! ```
//!
//! **nodes**: at each node `(P, C)` of the probe's fixtures (`testdata/perf/*.nodes`), the exchanges `P → P'` =
//! `P − B + A` (`A` among the candidates the node has excluded, `B ⊆ P`) whose state *covers* the node's for its
//! candidates (see [`covers`]): then every completion of the node is a completion of `P'`, worth more if
//! `sum(A) > sum(B)`, and the node can be pruned.  Kinds: `add` (`B` empty, one `a`), `swap` (one for one, `a >
//! b`), and with `--pairs` two for one, one for two and two for two.

#[path = "common/fixtures.rs"]
mod fixtures;
#[path = "common/symmetry.rs"]
mod symmetry;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Ctx, EngineConfig, Searcher, Start};
use taxman::game::Optimal;
use taxman::matchbound::{MatchBound, MatchGraph};
use taxman::model::{FactorProvider, Frame, FrameSet, Model, Scratch};
use symmetry::{cofactors, map_set, primes_to, root_pairs, swap_primes};
use taxman::search::{Prepared, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};

/// The search tactics of the frontier's results (the defaults, all of them proven).
fn tactics() -> SearchTactics {
    SearchTactics::default()
}

fn setup<const W: usize>(n: u32) -> Prepared<W> {
    Board::<W>::for_n(n).prepare(tactics()).expect("prepare")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match command.as_str() {
        "nodes" => nodes(&rest),
        "sample" => sample(&rest),
        "symmetries" => symmetries(&rest),
        "symcheck" => symcheck(&rest),
        _ => {
            eprintln!("usage: dominance nodes [--pairs] FIXTURE... | sample --known FILE --every K --out DIR N...");
            std::process::exit(2);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// symmetries

/// For each game: the pairs of primes whose exchange is a symmetry of the search problem at its root (see
/// [`symmetry::root_pairs`]), and how many candidates they move.
fn symmetries(args: &[String]) {
    let mut games = Vec::new();
    for arg in args {
        match arg.split_once('-') {
            Some((a, b)) => games.extend(a.parse::<u32>().expect("A")..=b.parse::<u32>().expect("B")),
            None => games.push(arg.parse::<u32>().expect("N")),
        }
    }
    for n in games {
        taxman::with_width!(n, W => symmetries_of::<W>(n));
    }
}

fn symmetries_of<const W: usize>(n: u32) {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return;
    }
    let p = board.prepare(tactics()).expect("prepare");
    let pairs = root_pairs(&p);
    let mut moved = Bits::<W>::EMPTY;
    for &(a, b) in &pairs {
        moved |= p.candidates.iter().filter(|&x| swap_primes(x, a, b) != x).collect::<Bits<W>>();
    }
    println!(
        "{{\"n\":{n},\"candidates\":{},\"symmetric_pairs\":{},\"moved\":{},\"pairs\":\"{pairs:?}\"}}",
        p.candidates.len(),
        pairs.len(),
        moved.len()
    );
}

// ---------------------------------------------------------------------------------------------------------
// symcheck

/// Checks of the root's prime symmetries (`symmetry::root_pairs`), per game: for every pair `(a, b)`, the image of
/// the optimal set and of `samples` random maximal promotable sets (and, on boards of at most 22 candidates, of every
/// promotable set) can be promoted, and is worth `(b − a)(c_a − c_b)` more; on those small boards, the best set that
/// keeps every pair's constraint `c_a ≤ c_b` is worth the optimum; and up to `n = 40`, the brute-force oracle
/// (`taxman::brute`, no frame model) scores the known optimum.  Prints a line per game with pairs, and panics on a
/// failure.
fn symcheck(args: &[String]) {
    let (mut known, mut samples_per_pair, mut games) = (None, 50usize, Vec::new());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--known" => known = it.next().map(PathBuf::from),
            "--samples" => samples_per_pair = it.next().and_then(|v| v.parse().ok()).expect("--samples K"),
            g => match g.split_once('-') {
                Some((a, b)) => games.extend(a.parse::<u32>().expect("A")..=b.parse::<u32>().expect("B")),
                None => games.push(g.parse::<u32>().expect("N")),
            },
        }
    }
    let optimal = Optimal::built_in().with_results_files(known.as_slice()).expect("known games");
    for n in games {
        taxman::with_width!(n, W => symcheck_one::<W>(n, &optimal, samples_per_pair));
    }
}

fn symcheck_one<const W: usize>(n: u32, optimal: &Optimal, samples_per_pair: usize) {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return;
    }
    let p = board.prepare(tactics()).expect("prepare");
    let pairs = root_pairs(&p);
    if n <= 40 {
        let exact = taxman::brute::solve(n, &[], taxman::brute::Rule::Divisor).score;
        assert_eq!(Some(exact), optimal.score(n), "n={n}: the oracle disagrees with the known optimum");
    }
    if pairs.is_empty() {
        return;
    }
    let (model, root) = (&*p.model, &p.root);
    let mut scratch = Scratch::new();
    let opt = optimal.score(n).expect("a known game") - board.in_the_bag;
    let best: Bits<W> = optimal.moves(n).expect("moves").iter().copied().filter(|&x| p.candidates.contains(x)).collect();
    assert_eq!(best.sum(), opt);
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
    for _ in 0..samples_per_pair {
        let s = random_maximal(model, &mut scratch, root, &p.candidates, &mut rng);
        check(&s, &mut scratch);
    }
    // Small boards: every promotable set, and the best that keeps the constraints.
    let mut enumerated = 0u64;
    let mut kept_best = None;
    if p.candidates.len() <= 22 {
        let cands = p.candidates.to_vec();
        let mut stack = vec![(0usize, Bits::<W>::EMPTY, *root)];
        while let Some((i, set, state)) = stack.pop() {
            if i == cands.len() {
                enumerated += 1;
                check(&set, &mut scratch);
                let keeps = pairs.iter().all(|&(a, b)| cofactors(&set, a) <= cofactors(&set, b));
                if keeps {
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

/// Nodes of a proof sampled uniformly over its steps: the proof (one thread, strict tactics, the incumbent the
/// optimum) in runs of `every` steps, each resuming the checkpoint of the last; the run stops in the middle of a
/// node's loop, and the deepest unit the next run is handed is that node.  Written as a `.nodes` fixture.  (Each
/// run starts a new searcher, which forgets its nogoods and multipliers, so the tree is that of a search that
/// forgets every `every` steps: larger than the proof's for small `every`.)
fn sample(args: &[String]) {
    let (mut known, mut every, mut out, mut games, mut fresh) = (None, 2000u64, PathBuf::from("."), Vec::new(), 0);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--known" => known = it.next().map(PathBuf::from),
            "--every" => every = it.next().and_then(|v| v.parse().ok()).expect("--every K"),
            "--fresh" => fresh = it.next().and_then(|v| v.parse().ok()).expect("--fresh K"),
            "--out" => out = it.next().map(PathBuf::from).expect("--out DIR"),
            n => games.push(n.parse::<u32>().expect("a board size")),
        }
    }
    let optimal = Optimal::built_in().with_results_files(known.as_slice()).expect("known games");
    for n in games {
        if fresh > 0 {
            taxman::with_width!(n, W => sample_fresh::<W>(n, &optimal, fresh, &out));
        } else {
            taxman::with_width!(n, W => sample_one::<W>(n, &optimal, every, &out));
        }
    }
}

/// The proof's own tree, sampled: the proof (one thread, strict tactics, the incumbent the optimum, so the tree is
/// the same every time) run once to count its steps `T`, then `count` times from the start, stopped after `k·T /
/// (count + 1)` steps for `k = 1..=count` (at the engine's next poll); the deepest unit each stopped run leaves is
/// the node it stopped in.  Costs about `count / 2` proofs.
fn sample_fresh<const W: usize>(n: u32, optimal: &Optimal, count: u64, out: &std::path::Path) {
    let p = setup::<W>(n);
    let moves = optimal.moves(n).unwrap_or_else(|| panic!("no known game for n={n}"));
    let incumbent: Bits<W> = moves.iter().copied().filter(|&x| p.candidates.contains(x)).collect();
    assert!(p.model.promote_all_using(&mut Scratch::new(), &p.root, &incumbent).is_some(), "the optimum");
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace::new(&p, &graph, None);
    let payload: Vec<u8> = incumbent.as_words().iter().flat_map(|w| w.to_le_bytes()).collect();
    let start = || Start::Fresh {
        units: vec![TaxUnit { promoted: Bits::<W>::EMPTY, cands: p.candidates }],
        best: incumbent.sum() + delta(),
        payload: payload.clone(),
    };
    let tmp = std::env::temp_dir().join(format!("taxman-dominance-{n}-{}.ckpt", std::process::id()));
    let t0 = Instant::now();
    let cfg = EngineConfig { threads: 1, progress: None, ..EngineConfig::default() };
    let total = engine::run(&cfg, start(), b"dominance", |_| TaxSearcher::new(&space)).expect("search").stats.nodes;
    let log = Mutex::new(Vec::new());
    let mut samples = Vec::new();
    for k in 1..=count {
        let cfg = EngineConfig {
            threads: 1,
            checkpoint: Some((tmp.clone(), Duration::from_secs(1 << 20))),
            node_limit: Some(k * total / (count + 1)),
            progress: None,
            ..EngineConfig::default()
        };
        let o = engine::run(&cfg, start(), b"dominance", |_| Recorder { inner: TaxSearcher::new(&space), log: &log })
            .expect("search");
        log.lock().expect("log").clear();
        if o.completed {
            continue;
        }
        // The stopped run's units are in its checkpoint; a resume would hand them over, deepest included.
        let resume = EngineConfig { node_limit: Some(o.stats.nodes), ..cfg.clone() };
        let _ = engine::run(&resume, Start::Resume(tmp.clone()), b"dominance", |_| Recorder {
            inner: TaxSearcher::new(&space),
            log: &log,
        });
        let handed = std::mem::take(&mut *log.lock().expect("log"));
        if let Some(deepest) = handed.iter().max_by_key(|u| u.promoted.len()) {
            samples.push(*deepest);
        }
    }
    let _ = std::fs::remove_file(&tmp);
    let path = out.join(format!("n{n:04}.nodes"));
    let mut text = format!(
        "# Search nodes of game {n} sampled by `dominance sample --fresh {count}`: the node the proof ({total} steps,\n\
         # one thread) is in after k·{total}/{} steps, k = 1..{count} ({:.1} s).\n",
        count + 1,
        t0.elapsed().as_secs_f64()
    );
    text.push_str(&format!("n {n}\nincumbent {}\n", fixtures::to_hex(&incumbent)));
    for u in &samples {
        text.push_str(&format!("node {} {}\n", fixtures::to_hex(&u.promoted), fixtures::to_hex(&u.cands)));
    }
    std::fs::write(&path, text).expect("write the samples");
    eprintln!("n={n}: {} samples of {total} steps -> {} ({:.1} s)", samples.len(), path.display(), t0.elapsed().as_secs_f64());
}

fn sample_one<const W: usize>(n: u32, optimal: &Optimal, every: u64, out: &std::path::Path) {
    let p = setup::<W>(n);
    let moves = optimal.moves(n).unwrap_or_else(|| panic!("no known game for n={n}"));
    let incumbent: Bits<W> = moves.iter().copied().filter(|&x| p.candidates.contains(x)).collect();
    assert!(p.model.promote_all_using(&mut Scratch::new(), &p.root, &incumbent).is_some(), "the optimum");
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace::new(&p, &graph, None);
    let log = Mutex::new(Vec::new());
    let tmp = std::env::temp_dir().join(format!("taxman-dominance-{n}-{}.ckpt", std::process::id()));
    let payload = incumbent.as_words().iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut start =
        Start::Fresh { units: vec![TaxUnit { promoted: Bits::EMPTY, cands: p.candidates }], best: incumbent.sum(), payload };
    let cfg = EngineConfig {
        threads: 1,
        checkpoint: Some((tmp.clone(), Duration::from_secs(1 << 20))),
        node_limit: Some(every),
        progress: None,
        ..EngineConfig::default()
    };
    let mut samples = Vec::new();
    let t0 = Instant::now();
    let total = loop {
        let o = engine::run(&cfg, start, b"dominance", |_| Recorder { inner: TaxSearcher::new(&space), log: &log })
            .expect("search");
        let handed = std::mem::take(&mut *log.lock().expect("log"));
        if !samples.is_empty() || handed.len() > 1 {
            // The first unit a resumed run is handed is not necessarily the deepest; the deepest is the node the
            // last run stopped in.
            if let Some(deepest) = handed.iter().max_by_key(|u| u.promoted.len()) {
                samples.push(*deepest);
            }
        }
        if o.completed {
            break o.stats.nodes;
        }
        start = Start::Resume(tmp.clone());
    };
    let _ = std::fs::remove_file(&tmp);
    let path = out.join(format!("n{n:04}.nodes"));
    let mut text = format!(
        "# Search nodes of game {n} sampled by `dominance sample --every {every}`: the node each run of {every} steps\n\
         # stopped in ({} runs, {total} steps in all, {:.1} s).\n",
        samples.len(),
        t0.elapsed().as_secs_f64()
    );
    text.push_str(&format!("n {n}\nincumbent {}\n", fixtures::to_hex(&incumbent)));
    for u in &samples {
        text.push_str(&format!("node {} {}\n", fixtures::to_hex(&u.promoted), fixtures::to_hex(&u.cands)));
    }
    std::fs::write(&path, text).expect("write the samples");
    eprintln!("n={n}: {} samples of {total} steps -> {}", samples.len(), path.display());
}

// ---------------------------------------------------------------------------------------------------------
// Covering states

/// The numbers adjacent to some member of `xs` as their maximal factors (restricted to the frames, as `g` is).
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
/// member of `cin` has), and then factors that no remaining move nor member of `cin` has, outside `cout`, until
/// nothing changes.  For every `G ⊆ cin`, `L ⊆ cout`, the frame with `G` added and `L` removed is winnable exactly
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

/// Why [`covers`] said no, for the tables (`None`: it covers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fails {
    /// A candidate of the node is no longer a factor of its home frame in the other state.
    Candidate,
    /// The other state's core has a move the node's has not.
    Move,
    /// The other state's core lacks a factor the node's core needs.
    Factor,
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
) -> Result<(), Fails> {
    let (mine, theirs) = (state.frames(), other.frames());
    for h in 0..mine.len() {
        let cin = mine.get(h + 1).map_or(Bits::EMPTY, |above| above.factors & *cands);
        let cout = mine[h].factors & *cands;
        if !cout.is_subset(&theirs[h].factors) {
            return Err(Fails::Candidate);
        }
        if mine[h] == theirs[h] {
            continue;
        }
        let (m1, f1) = core(g, &mine[h], &cin, &cout);
        let (m2, f2) = core(g, &theirs[h], &cin, &cout);
        if relabel {
            // The factors only `state`'s core has must be of no use to the moves that stand in for `other`'s, nor
            // to `cin`; and each move of `other`'s core needs its own move of `state`'s core with the same live
            // factors among the common ones.
            if !(f1 & factors_of(g, &cin)).is_subset(&f2) {
                return Err(Fails::Factor);
            }
            let common = f1 & f2;
            let only_mine = f1 - f2;
            let key = |x: u32| -> Vec<u16> {
                g.factors(x).iter().copied().filter(|&q| common.contains(u32::from(q))).collect()
            };
            let mut spare: std::collections::HashMap<Vec<u16>, i32> = Default::default();
            for x in m1.iter() {
                if !g.factors(x).iter().any(|&q| only_mine.contains(u32::from(q))) {
                    *spare.entry(key(x)).or_default() += 1;
                }
            }
            for x in m2.iter() {
                let slot = spare.entry(key(x)).or_default();
                *slot -= 1;
                if *slot < 0 {
                    return Err(Fails::Move);
                }
            }
            continue;
        }
        if !m2.is_subset(&m1) {
            return Err(Fails::Move);
        }
        let usable = factors_of(g, &(m2 | cin));
        if !(f1 & usable).is_subset(&f2) {
            return Err(Fails::Factor);
        }
    }
    Ok(())
}

/// [`covers`] up to the relabelling `pi` (an involution of the numbers): whether `other` covers `state` with
/// every number of `state`'s cores renamed by `pi`, for the candidates `cands`, which `pi` must fix.  Frame by frame,
/// `pi` must carry the factor graph between the moves of `state`'s core (and the members of `cands` that could be
/// promoted into the frame) and the factors of its core onto the graph between their images: then the renamed core
/// is the same game for every `G ⊆ cin`, `L ⊆ cout` (which `pi` fixes), and the literal test of [`covers`] applies to
/// it.
fn covers_mapped<const W: usize>(
    g: &FactorProvider,
    other: &FrameSet<W>,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    pi: &impl Fn(u32) -> u32,
) -> Result<(), Fails> {
    if cands.iter().any(|x| pi(x) != x) {
        return Err(Fails::Candidate);
    }
    let (mine, theirs) = (state.frames(), other.frames());
    for h in 0..mine.len() {
        let cin = mine.get(h + 1).map_or(Bits::EMPTY, |above| above.factors & *cands);
        let cout = mine[h].factors & *cands;
        if !cout.is_subset(&theirs[h].factors) {
            return Err(Fails::Candidate);
        }
        let (m1, f1) = core(g, &mine[h], &cin, &cout);
        let (m2, f2) = core(g, &theirs[h], &cin, &cout);
        let (Some(pm1), Some(pf1)) = (map_set(&m1, pi), map_set(&f1, pi)) else {
            return Err(Fails::Move);
        };
        // The renamed core's edges must be the images of the core's.
        for x in (m1 | cin).iter() {
            let live: Bits<W> = g.factors(x).iter().map(|&q| u32::from(q)).filter(|&q| f1.contains(q)).collect();
            let image: Bits<W> = g.factors(pi(x)).iter().map(|&q| u32::from(q)).filter(|&q| pf1.contains(q)).collect();
            if map_set(&live, pi) != Some(image) {
                return Err(Fails::Move);
            }
        }
        if !m2.is_subset(&pm1) {
            return Err(Fails::Move);
        }
        let usable = factors_of(g, &(m2 | cin));
        if !(pf1 & usable).is_subset(&f2) {
            return Err(Fails::Factor);
        }
    }
    Ok(())
}


/// The classes of *twins* among `cands` in `state`: candidates of the same home frame `h` with the same live
/// maximal factors in frame `h − 1` and the same users in frame `h` (the moves of the core of frame `h` for
/// completions within `cands`, and the members of `cands` that could be promoted into it).  Swapping two twins is
/// an automorphism of the node's problem restricted to `cands`, so a completion that promotes the smaller and not
/// the larger can be traded for one that promotes the larger instead.  Classes of two or more, largest first.
fn twins<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>) -> Vec<Vec<u32>> {
    let g = model.graph();
    let frames = state.frames();
    let mut cores = Vec::with_capacity(frames.len());
    for h in 0..frames.len() {
        let cin = frames.get(h + 1).map_or(Bits::EMPTY, |above| above.factors & *cands);
        let cout = frames[h].factors & *cands;
        cores.push((core(g, &frames[h], &cin, &cout), cin));
    }
    let mut classes: std::collections::HashMap<(usize, Vec<u32>, Vec<u32>), Vec<u32>> = Default::default();
    for x in cands.iter() {
        let h = model.home(x);
        if h == 0 || !frames[h].factors.contains(x) {
            continue;
        }
        let below = frames[h - 1].factors;
        let down: Vec<u32> = g.factors(x).iter().map(|&q| u32::from(q)).filter(|&q| below.contains(q)).collect();
        let ((moves, _), cin) = &cores[h];
        let users = *moves | *cin;
        let up: Vec<u32> = g.composites(x).iter().map(|&c| u32::from(c)).filter(|&c| users.contains(c)).collect();
        classes.entry((h, down, up)).or_default().push(x);
    }
    let mut out: Vec<Vec<u32>> = classes.into_values().filter(|c| c.len() > 1).collect();
    for c in &mut out {
        c.sort_unstable_by(|a, b| b.cmp(a));
    }
    out.sort_unstable();
    out
}

/// Prints, to stderr, an exchange from `promoted` to `set` that no sampled completion refutes and [`covers`] does
/// not prove: per frame where the two states differ, the moves and factors of each core.
fn explain_exchange<const W: usize>(
    model: &Model,
    state: &FrameSet<W>,
    other: &FrameSet<W>,
    promoted: &Bits<W>,
    set: &Bits<W>,
    cands: &Bits<W>,
) {
    let g = model.graph();
    let (out, inn) = (*promoted - *set, *set - *promoted);
    let homes = |s: &Bits<W>| s.iter().map(|x| format!("{x}@{}", model.home(x))).collect::<Vec<_>>().join(" ");
    eprintln!("exchange: out [{}] in [{}]; reason {:?}", homes(&out), homes(&inn), covers(g, other, state, cands, true));
    let (mine, theirs) = (state.frames(), other.frames());
    for h in 0..mine.len() {
        if mine[h] == theirs[h] {
            continue;
        }
        let cin = mine.get(h + 1).map_or(Bits::EMPTY, |above| above.factors & *cands);
        let cout = mine[h].factors & *cands;
        let (m1, f1) = core(g, &mine[h], &cin, &cout);
        let (m2, f2) = core(g, &theirs[h], &cin, &cout);
        let show = |x: u32, f: &Bits<W>| {
            let live: Vec<String> = g.factors(x).iter().map(|&q| u32::from(q)).filter(|&q| f.contains(q)).map(|q| {
                let users = g.composites(q).iter().filter(|&&c| cin.contains(u32::from(c))).count();
                format!("{q}{}{}", if cout.contains(q) { "*" } else { "" }, if users > 0 { format!("+{users}") } else { String::new() })
            }).collect();
            format!("{x}({})", live.join(","))
        };
        let only1: Vec<String> = (m1 - m2).iter().map(|x| show(x, &f1)).collect();
        let only2: Vec<String> = (m2 - m1).iter().map(|x| show(x, &f2)).collect();
        eprintln!(
            "  frame {h}: core moves {} / {}, only the node's: {:?}; only the exchange's: {:?}; factors only the node's: {:?}; only the exchange's: {:?}",
            m1.len(),
            m2.len(),
            only1,
            only2,
            (f1 - f2).to_vec(),
            (f2 - f1).to_vec()
        );
    }
}

/// A margin over the optimum for the incumbent's value (`DOMINANCE_DELTA`, default 0): every bound lowered by it,
/// as `slope prove --deltas` does.
fn delta() -> u64 {
    std::env::var("DOMINANCE_DELTA").ok().and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// Random maximal sets drawn per question by [`random_maximal`].
fn samples() -> usize {
    std::env::var("DOMINANCE_SAMPLES").ok().and_then(|v| v.parse().ok()).unwrap_or(64)
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
    let mut kept = Bits::EMPTY;
    for x in order {
        if model.promote_in_place_using(scratch, &mut now, x) {
            kept.insert(x);
        }
    }
    kept
}

// ---------------------------------------------------------------------------------------------------------
// nodes

/// Counts of exchanges tried at a node, by outcome.
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    tried: u64,
    infeasible: u64,
    candidate: u64,
    moves: u64,
    factors: u64,
    covers: u64,
}

impl Tally {
    fn add(&mut self, outcome: Option<Result<(), Fails>>) {
        self.tried += 1;
        match outcome {
            None => self.infeasible += 1,
            Some(Err(Fails::Candidate)) => self.candidate += 1,
            Some(Err(Fails::Move)) => self.moves += 1,
            Some(Err(Fails::Factor)) => self.factors += 1,
            Some(Ok(())) => self.covers += 1,
        }
    }
}

fn nodes(args: &[String]) {
    let pairs = args.iter().any(|a| a == "--pairs");
    let explain = args.iter().any(|a| a == "--explain");
    for path in args.iter().filter(|a| !a.starts_with("--")) {
        let path = PathBuf::from(path);
        let text = std::fs::read_to_string(&path).expect("fixture");
        let n: u32 = text
            .lines()
            .find_map(|l| l.strip_prefix("n "))
            .and_then(|v| v.trim().parse().ok())
            .expect("a board size");
        taxman::with_width!(n, W => nodes_of::<W>(&path, pairs, explain));
    }
}

/// The candidates of the node `(promoted, cands)` that some completion adding `need` could promote, by the
/// matching bound: `x` is left out when the bound with `x` promoted, plus `x`, is below `need` (or `x` cannot be
/// promoted).  Every completion that beats the incumbent is a subset of what is left.  `None` if the node's own
/// bound is below `need` (no completion beats the incumbent).
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
    for x in (*cands - chosen).iter() {
        if !model.can_promote_using(scratch, state, x) {
            continue;
        }
        m.push(x, &cands.without(x));
        if m.value() + u64::from(x) >= need {
            out.insert(x);
        }
        m.pop();
    }
    Some(out)
}

fn nodes_of<const W: usize>(path: &PathBuf, pairs: bool, explain: bool) {
    let (n, incumbent, units) = fixtures::read_nodes::<W>(path).expect("fixture");
    let p = setup::<W>(n);
    let (model, root) = (&*p.model, &p.root);
    let g = model.graph();
    let graph = MatchGraph::new(model, root);
    let mut m = MatchBound::new(&graph);
    let mut scratch = Scratch::new();
    let best = incumbent.sum() + delta();
    let primes = primes_to(n / 2);
    let sym_pairs = root_pairs(&p);
    let t0 = Instant::now();
    // Per kind (add, swap, pairs) and candidate set (all, relevant): nodes dominated, and the tallies.
    let mut dominated = [[0u64; 3]; 3];
    let mut kinds = [[Tally::default(); 3]; 3];
    let mut closed = 0;
    for unit in &units {
        let (promoted, cands) = (unit.promoted, unit.cands);
        let state = model.promote_all_using(&mut scratch, root, &promoted).expect("a node of the proof");
        let need = (best + 1).saturating_sub(promoted.sum());
        let Some(rel) = relevant(&mut m, model, &mut scratch, &state, &promoted, &cands, need) else {
            closed += 1;
            continue;
        };
        let excluded = p.candidates - promoted - cands;
        let mut found = [[false; 3]; 3];
        // Random maximal completions within the relevant candidates: an exchange whose state cannot take one of them
        // does not cover the node (infeasibility only grows with the set, so maximal sets are the ones to try).
        let mut rng = 0x2545_f491_4f6c_dd1du64 ^ promoted.sum();
        let maximal: Vec<Bits<W>> =
            (0..samples()).map(|_| random_maximal(model, &mut scratch, &state, &rel, &mut rng)).collect();
        let mut semantic = [false; 3];
        let mut check = |set: Bits<W>, kind: usize, scratch: &mut Scratch<W>| {
            let Some(other) = model.promote_all_using(scratch, root, &set) else {
                for tally in &mut kinds[kind] {
                    tally.add(None);
                }
                return;
            };
            let takes_all = maximal.iter().all(|s| model.promote_all_using(scratch, &other, s).is_some());
            semantic[kind] |= takes_all;
            if takes_all && explain && covers(g, &other, &state, &rel, true).is_err() {
                explain_exchange(model, &state, &other, &promoted, &set, &rel);
            }
            for (j, (within, relabel)) in [(cands, false), (rel, false), (rel, true)].into_iter().enumerate() {
                let outcome = covers(g, &other, &state, &within, relabel);
                kinds[kind][j].add(Some(outcome));
                found[kind][j] |= outcome.is_ok();
                assert!(outcome.is_err() || takes_all, "a cover that does not take a completion: {set:?}");
            }
        };
        for a in excluded.iter() {
            check(promoted.with(a), 0, &mut scratch);
        }
        for b in promoted.iter() {
            for a in excluded.iter().filter(|&a| a > b) {
                check(promoted.without(b).with(a), 1, &mut scratch);
            }
        }
        if pairs {
            let ex: Vec<u32> = excluded.iter().collect();
            let pr: Vec<u32> = promoted.iter().collect();
            for (i, &a1) in ex.iter().enumerate() {
                for &a2 in &ex[i + 1..] {
                    let added = Bits::EMPTY.with(a1).with(a2);
                    // two for one
                    for &b in pr.iter().filter(|&&b| u64::from(b) < added.sum()) {
                        check((promoted | added).without(b), 2, &mut scratch);
                    }
                    // two for two
                    for (j, &b1) in pr.iter().enumerate() {
                        for &b2 in &pr[j + 1..] {
                            let taken = Bits::EMPTY.with(b1).with(b2);
                            let more = added.sum() > taken.sum()
                                || (added.sum() == taken.sum() && added.max() > taken.max());
                            if more {
                                check((promoted - taken) | added, 2, &mut scratch);
                            }
                        }
                    }
                }
            }
        }
        for (d, f) in dominated.iter_mut().zip(found) {
            for j in 0..3 {
                d[j] += u64::from(f[j]);
            }
        }
        // The root's prime symmetries: no completion can bring the cofactors of q up to those of p.
        let violated = |within: &Bits<W>| {
            sym_pairs.iter().filter(|&&(a, b)| cofactors(&promoted, a) > cofactors(&promoted, b) + cofactors(within, b)).count()
        };
        let (sym_all, sym_rel) = (violated(&cands), violated(&rel));
        // The matching bound of the completions that keep a symmetric pair's constraint, with the pair's multiples
        // among the relevant candidates enumerated (each subset `U` promoted, the others left out) and the rest
        // relaxed: below `need` for some pair, and every completion that could beat the incumbent breaks the
        // constraint, so its image under the pair's exchange beats it by more.
        let mut sym_cut = false;
        for &(a, b) in &sym_pairs {
            let delta = cofactors(&promoted, a) as i64 - cofactors(&promoted, b) as i64;
            let family: Vec<u32> = rel.iter().filter(|x| x.is_multiple_of(a) || x.is_multiple_of(b)).collect();
            if family.len() > 12 {
                continue;
            }
            let fam: Bits<W> = family.iter().copied().collect();
            let mut best_u = None;
            for mask in 0u32..(1 << family.len()) {
                let u: Bits<W> = family.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).map(|(_, &x)| x).collect();
                if delta + cofactors(&u, a) as i64 - cofactors(&u, b) as i64 > 0 {
                    continue; // breaks the constraint
                }
                let Some(st) = model.promote_all_using(&mut scratch, &state, &u) else { continue };
                let _ = st;
                m.reset(&(promoted | u), &(rel - fam));
                let value = m.value() + u.sum();
                best_u = best_u.max(Some(value));
            }
            if best_u.is_none_or(|v| v < need) {
                sym_cut = true;
                break;
            }
        }
        // Prime swaps: P' = P with the exponents of p and q exchanged, worth more.
        let (mut prime_sem, mut prime_syn, mut prime_tried) = (false, false, 0u32);
        let mut prime_pairs = Vec::new();
        for (i, &pp) in primes.iter().enumerate() {
            if !promoted.iter().any(|x| x.is_multiple_of(pp)) {
                continue;
            }
            for &qq in &primes[i + 1..] {
                let pi = |x: u32| swap_primes(x, pp, qq);
                let Some(set) = map_set(&promoted, &pi) else { continue };
                if set.sum() <= promoted.sum() || !set.is_subset(&p.candidates) || set.intersects(&rel) {
                    continue;
                }
                prime_tried += 1;
                let Some(other) = model.promote_all_using(&mut scratch, root, &set) else { continue };
                let takes_all = maximal.iter().all(|s| model.promote_all_using(&mut scratch, &other, s).is_some());
                let syn = covers_mapped(g, &other, &state, &rel, &pi).is_ok();
                assert!(!syn || takes_all, "a mapped cover that does not take a completion");
                prime_sem |= takes_all;
                prime_syn |= syn;
                if takes_all {
                    prime_pairs.push((pp, qq, syn));
                }
            }
        }
        // The excluded candidates that can be promoted at the node, and how many of them no single relevant
        // candidate blocks (an upper estimate of the "add" dominance, which needs no blocking set at all).
        let mut addable = 0;
        let mut unblocked = 0;
        let mut unblocked_maximal = 0;
        let mut rng = 0x9e37_79b9_7f4a_7c15u64 ^ u64::from(promoted.len());
        for a in excluded.iter() {
            let Some(with_a) = model.promote(&state, a) else { continue };
            addable += 1;
            let blocked = rel.iter().any(|x| !model.can_promote_using(&mut scratch, &with_a, x));
            unblocked += u32::from(!blocked);
            if !blocked {
                // A set that blocks `a` stays blocking as it grows, so look at random maximal sets.
                let blocked = (0..samples()).any(|_| {
                    let s = random_maximal(model, &mut scratch, &state, &rel, &mut rng);
                    !model.can_promote_all_using(&mut scratch, &with_a, &s)
                });
                unblocked_maximal += u32::from(!blocked);
            }
        }
        let twins_all = twins(model, &state, &cands);
        let twins_rel = twins(model, &state, &rel);
        let members = |t: &Vec<Vec<u32>>| t.iter().map(Vec::len).sum::<usize>();
        println!(
            "{{\"n\":{n},\"promoted\":{},\"cands\":{},\"relevant\":{},\"excluded\":{},\"found\":{:?},\"semantic\":{:?},\"sem_add\":{},\"sem_swap\":{},\"sem_pairs\":{},\
             \"sym_all\":{sym_all},\"sym_rel\":{sym_rel},\"sym_cut\":{sym_cut},\"prime_tried\":{prime_tried},\"prime_sem\":{prime_sem},\"prime_syn\":{prime_syn},\"prime_pairs\":\"{prime_pairs:?}\",\
             \"addable\":{addable},\"unblocked\":{unblocked},\"unblocked_maximal\":{unblocked_maximal},\"twins_all\":{},\"twins_rel\":{},\"classes_rel\":{:?}}}",
            promoted.len(),
            cands.len(),
            rel.len(),
            excluded.len(),
            found, semantic, semantic[0], semantic[1], semantic[2],
            members(&twins_all),
            members(&twins_rel),
            twins_rel,
        );
    }
    eprintln!(
        "n={n}: {} nodes ({closed} closed by the bound), dominated [add, swap, pairs] x [all, relevant, relevant relabelled] {:?} \
         ({:.1} s)",
        units.len(),
        dominated,
        t0.elapsed().as_secs_f64(),
    );
    for (kind, t) in ["add", "swap", "pairs"].iter().zip(kinds) {
        eprintln!("  {kind}: all {:?}\n  {kind}: relevant {:?}\n  {kind}: relabelled {:?}", t[0], t[1], t[2]);
    }
}
