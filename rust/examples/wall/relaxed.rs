//! `wall relaxed`: would nogood multipliers fitted once at the root, used unchanged at every node, cut the nodes that
//! today's bound leaves open?  The study, its data and its conclusion are in
//! docs/experiments/root-fitted-nogood-multipliers-at-the-nodes-2026-10.md.
//!
//! A *nogood* `N` is a set of candidates that cannot all be promoted from the root, so at a node with promotions `P`
//! every completion `F` has `|F ∩ N| ≤ |N| - 1 - |P ∩ N|`.  For multipliers `λ_N ≥ 0`, the heaviest common independent
//! set of at most `max_additional` of the node's candidates in the node's GF(2) matroids, each candidate `x` weighing
//! `x - Σ_{N ∋ x} λ_N` (those of no positive weight left out), plus the constant `Σ λ_N (|N| - 1 - |P ∩ N|)`, bounds
//! every completion (weak duality, for any multipliers; at the root it is the bound of `wall nogoods`).  A tighter
//! constant holds too: `|F ∩ N| ≤ |N ∩ C|` for the node's candidates `C`, so the term of `N` can be `λ_N (|N - P| -
//! max(1, |N - P - C|))`, which counts a nogood with a member gone from the candidates (not live) by its members still
//! candidates.  Both are measured, with the weights the root's either way.
//!
//! For each game with a known optimum (`--known FILE`; a game with none is an error), on one thread:
//!
//! 1. the pool: the nogoods that the finder (large neighbourhood search) and the sound search learn from the standalone
//!    start incumbent in `--cpu` CPU seconds each (default 5), harvested as `wall nogoods` harvests them;
//! 2. the fit: `--steps` subgradient steps at the root (default 200) towards the optimum plus one, as `wall nogoods`
//!    fits; the multipliers of the lowest bound are the root's;
//! 3. the nodes: those of the fixture `--fixtures DIR` has for the game (`nNNNN.nodes`, as the probe's, made by `perf
//!    make-nodes`: units that the proof seeded with the optimum handed its searcher), the units of the game in the
//!    tuning suite `--suite FILE` (`bench/tuning/suite.jsonl`), and `--paths K` nodes on the way to the optimum
//!    (default 50): the optimum's promotions less a random 30 to 70% of them, its other candidates narrowed, whose best
//!    completion is at least the optimum less what they promote, a check of the bound's validity.
//!
//! At each node: today's bound (the GF(2) bound of the node as the search computes it for a unit,
//! [`NodeBound::completions`]), and the root-λ bound with each constant, all as promotion sums of the completions
//! (rounded down), with their CPU seconds.  The bound of a node is the matroids of its state built afresh
//! (`Gf2Graph::new`, timed apart), reweighted and solved; today's is solved from the root's matroids contracted by the
//! promotions, as a search does at a unit (along its path a search keeps its levels, which costs less than either).
//!
//! A line a game: `prime`, `in_the_bag`, `candidates`, `root` (the root's GF(2) bound), `opt`, `gap` (both promotion
//! sums), `finder_pool`, `search_pool` and `pool` (distinct nogoods), `harvest_cpu_s`, `fit_steps`, `fit_cpu_s`,
//! `active` (nogoods with a positive multiplier), `relaxed_root` and `share` (the root-λ bound at the root, and the share
//! of the gap it closes), and `nodes`, a list with an object for each node: `node` (its index), `source`
//! (`fixture`, `suite` or `path`), `depth` (its promotions), `cands`, `max_add`, `need` (what a completion needs to beat
//! the optimum: `opt + 1` less the promotions' sum), `today`, `relaxed` (the constant as above), `tight` (the tighter
//! constant), `lower` (on a path node, the optimum less the promotions' sum: no bound may be below it; else null),
//! `subtree` (a suite unit's nodes when the default solved it; else null), and the CPU seconds `today_s`, `graph_s`
//! (the node's matroids built afresh), `plain_s` (the GF(2) bound solved on them, which must equal `today`) and
//! `relaxed_s` (reweighted and solved, with both constants).  A bound below `lower`, a fresh bound that is not today's,
//! or a tighter constant above the other is an error after the game's lines.

use super::finders::{Case, Clock, Played};
use super::nogoods::{control, finder, fit, reweighted_bound, root_rooms, shifts, union, Fitting};
use super::{root_bound, Line};
use crate::fixtures::{from_hex, read_nodes};
use crate::known::known_incumbent;
use crate::rng::Rng;
use std::path::PathBuf;
use taxman::bits::Bits;
use taxman::bounds::max_additional;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::json::Json;
use taxman::matchbound::lagrange::SCALE;
use taxman::model::Scratch;
use taxman::search::{NodeBound, TaxUnit};

/// Where the nodes of a game come from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    Fixture,
    Suite,
    Path,
}

impl Source {
    pub(super) fn name(self) -> &'static str {
        match self {
            Source::Fixture => "fixture",
            Source::Suite => "suite",
            Source::Path => "path",
        }
    }
}

/// A node to measure: where it comes from, the unit, and for a suite unit the nodes of its subtree.
pub(super) struct Sampled<const W: usize> {
    pub(super) source: Source,
    pub(super) unit: TaxUnit<W>,
    pub(super) subtree: Option<u64>,
}

/// The nodes of game `n` from the fixture in `dir` (none if it has no file for the game), checking that its
/// incumbent is worth `opt`.
pub(super) fn fixture_nodes<const W: usize>(
    dir: &Option<PathBuf>,
    n: u32,
    opt: u64,
) -> Result<Vec<Sampled<W>>, String> {
    let Some(path) = dir.as_ref().map(|dir| dir.join(format!("n{n:04}.nodes"))).filter(|path| path.exists()) else {
        return Ok(Vec::new());
    };
    let (m, incumbent, units) = read_nodes::<W>(&path)?;
    if m != n || incumbent.sum() != opt {
        return Err(format!(
            "{}: game {m} from an incumbent worth {}, not {n} from {opt}",
            path.display(),
            incumbent.sum()
        ));
    }
    Ok(units.into_iter().map(|unit| Sampled { source: Source::Fixture, unit, subtree: None }).collect())
}

/// The units of game `n` in the tuning suite `suite` (none without one).
pub(super) fn suite_nodes<const W: usize>(suite: &Option<PathBuf>, n: u32) -> Result<Vec<Sampled<W>>, String> {
    let Some(path) = suite else { return Ok(Vec::new()) };
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let json = Json::parse(line).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let field = |key: &str| json.get(key).and_then(Json::as_str).unwrap_or_default();
        if field("kind") != "unit" || json.get("game").and_then(Json::as_u32) != Some(n) {
            continue;
        }
        let unit = TaxUnit { promoted: from_hex(field("promoted"))?, cands: from_hex(field("cands"))? };
        let default = json.get("default");
        let completed = default.and_then(|d| d.get("completed")).and_then(Json::as_bool) == Some(true);
        let subtree = default.and_then(|d| d.get("nodes")).and_then(Json::as_u64).filter(|_| completed);
        out.push(Sampled { source: Source::Suite, unit, subtree });
    }
    Ok(out)
}

/// `count` nodes on the way to the optimum `known` of `case`: `known` less a random 30 to 70% of its members promoted,
/// the root's other candidates narrowed in the state they reach (which keeps the rest of `known`, promotable there).
pub(super) fn path_nodes<const W: usize>(case: &Case<'_, W>, known: &Bits<W>, count: usize) -> Vec<Sampled<W>> {
    let p = case.p;
    let mut rng = Rng(0x9A7E_0000 + u64::from(case.n()));
    let mut scratch = Scratch::new();
    let members = known.to_vec();
    (0..count)
        .filter_map(|_| {
            let keep = 0.3 + 0.4 * rng.unit();
            let promoted: Bits<W> = members.iter().copied().filter(|_| rng.unit() < keep).collect();
            let state = p.model.promote_all_using(&mut scratch, &p.root, &promoted)?;
            let cands = p.model.narrow(&state, &(p.candidates - promoted));
            let unit = TaxUnit { promoted, cands };
            (*known - promoted).is_subset(&cands).then_some(Sampled { source: Source::Path, unit, subtree: None })
        })
        .collect()
}

/// The root's multipliers: the nogoods with a positive one, each with its multiplier, and the weight they take off
/// each number.
struct Relaxation<const W: usize> {
    nogoods: Vec<(Bits<W>, i64)>,
    shift: Vec<i64>,
}

impl<const W: usize> Relaxation<W> {
    fn new(nogoods: &[Bits<W>], lambda: &[i64]) -> Self {
        let top = nogoods.iter().filter_map(Bits::max).max().unwrap_or(0) as usize;
        let mut shift = vec![0; top + 1];
        shifts(nogoods, &root_rooms(nogoods), lambda, &mut shift);
        let kept = nogoods.iter().zip(lambda).filter(|&(_, &l)| l > 0).map(|(&n, &l)| (n, l)).collect();
        Relaxation { nogoods: kept, shift }
    }

    /// The constants of the node with the promotions `promoted` and the candidates `cands`: `Σ λ (|N| - 1 - |P ∩ N|)`,
    /// and the tighter `Σ λ (|N - P| - max(1, |N - P - C|))`, both scaled.
    fn constants(&self, promoted: &Bits<W>, cands: &Bits<W>) -> (i64, i64) {
        let (mut plain, mut tight) = (0, 0);
        for &(n, l) in &self.nogoods {
            let rest = i64::from((n - *promoted).len());
            let gone = i64::from((n - *promoted - *cands).len());
            plain += l * (rest - 1);
            tight += l * (rest - gone.max(1));
        }
        (plain, tight)
    }
}

/// The lines of game `n` (see the module comment).
pub fn of<const W: usize>(case: &Case<'_, W>) -> Result<Played, String> {
    let (n, p) = (case.n(), case.p);
    let (known, opt) = known_incumbent(case.board, p, case.optimal)?
        .ok_or_else(|| format!("n={n}: relaxed needs the known optimum (--known FILE)"))?;
    let graph = case.space.gf2.as_ref().ok_or_else(|| format!("n={n}: relaxed needs the GF(2) bound"))?;
    let root = root_bound(p, graph).0.value;
    let mut faults = Vec::new();

    let start = case.start().0;
    let (found, searched) = (finder(case, &start), control(case, &start));
    let pool = union(&found.nogoods, &searched.nogoods);
    let clock = Clock::start();
    let steps = case.params.steps;
    let fitted = fit(&Fitting::at_root(graph, p.max_total, &pool, opt), steps, &[steps]);
    let relaxation = Relaxation::new(&pool, &fitted.lambda);
    let fit_cpu_s = clock.elapsed();
    let relaxed_root = fitted.marks[0].scaled.div_euclid(SCALE);
    if relaxed_root < opt as i64 {
        faults.push(format!("n={n}: the root-λ bound {relaxed_root} is below the optimum {opt}"));
    }

    let mut nodes = fixture_nodes::<W>(&case.params.fixtures, n, opt)?;
    nodes.extend(suite_nodes::<W>(&case.params.suite, n)?);
    nodes.extend(path_nodes(case, &known, case.params.paths));
    let mut node_lines = Vec::new();
    let mut today_bound = NodeBound::new(p, graph);
    let mut scratch = Scratch::new();
    for (i, node) in nodes.iter().enumerate() {
        let (promoted, cands) = (node.unit.promoted, node.unit.cands);
        let clock = Clock::start();
        let today = today_bound.completions(&node.unit).ok_or_else(|| format!("n={n}: node {i} cannot be promoted"))?;
        let today_s = clock.elapsed();
        let state = p.model.promote_all_using(&mut scratch, &p.root, &promoted).expect("promoted above");
        let max_add = max_additional(&p.model, p.max_total, &state, &promoted, &cands);
        let clock = Clock::start();
        let fresh = Gf2Graph::new(&p.model, &state, &cands);
        let graph_s = clock.elapsed();
        let clock = Clock::start();
        let mut plain = Gf2Bound::<W>::new(&fresh);
        plain.reset(&Bits::EMPTY, &cands);
        let plain_value = plain.evaluate(max_add, None).value;
        let plain_s = clock.elapsed();
        let clock = Clock::start();
        let (value, _) = reweighted_bound::<W>(&fresh, max_add, &relaxation.shift);
        let (given, tight) = relaxation.constants(&promoted, &cands);
        let relaxed_s = clock.elapsed();
        let (relaxed, tightened) = ((value + given).div_euclid(SCALE), (value + tight).div_euclid(SCALE));
        let lower = (node.source == Source::Path).then(|| opt - promoted.sum());
        let at = format!("n={n}: node {i} ({}, depth {})", node.source.name(), promoted.len());
        if plain_value != today {
            faults.push(format!("{at}: the bound of its matroids built afresh is {plain_value}, not {today}"));
        }
        if tight > given {
            faults.push(format!("{at}: the tighter constant {tight} is above the other, {given}"));
        }
        if let Some(lower) = lower.filter(|&lower| tightened < lower as i64 || today < lower) {
            faults.push(format!("{at}: a bound below {lower}: today {today}, root-λ {relaxed} and {tightened}"));
        }
        let mut object = Line::new();
        object
            .field("node", i)
            .text("source", node.source.name())
            .field("depth", promoted.len())
            .field("cands", cands.len())
            .field("max_add", max_add)
            .field("need", (opt + 1).saturating_sub(promoted.sum()))
            .field("today", today)
            .field("relaxed", relaxed)
            .field("tight", tightened)
            .maybe("lower", lower)
            .maybe("subtree", node.subtree)
            .seconds("today_s", today_s)
            .seconds("graph_s", graph_s)
            .seconds("plain_s", plain_s)
            .seconds("relaxed_s", relaxed_s);
        node_lines.push(object);
    }

    let gap = root - opt;
    let mut line = Line::of(n);
    line.field("prime", case.board.tables.is_prime(n))
        .field("in_the_bag", case.board.in_the_bag)
        .field("candidates", p.candidates.len())
        .field("root", root)
        .field("opt", opt)
        .field("gap", gap)
        .field("finder_pool", found.nogoods.len())
        .field("search_pool", searched.nogoods.len())
        .field("pool", pool.len())
        .seconds("harvest_cpu_s", found.cpu_s + searched.cpu_s)
        .field("fit_steps", fitted.steps)
        .seconds("fit_cpu_s", fit_cpu_s)
        .field("active", relaxation.nogoods.len())
        .field("relaxed_root", relaxed_root)
        .maybe("share", (gap > 0).then(|| format!("{:.4}", (root as i64 - relaxed_root) as f64 / gap as f64)))
        .field("nodes", format!("[{}]", node_lines.iter().map(Line::render).collect::<Vec<_>>().join(",")));
    Ok(Played { line, faults })
}
