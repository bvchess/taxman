//! `wall choice-prepare` and `wall choice`: at which search nodes is a slow, strong bound worth computing instead of
//! today's fast GF(2) bound alone?  The question and the data it asks for are in
//! solver/notebook/the-node-choice-where-the-strong-bound-pays-2026-10/README.md; the analysis is
//! `solver/notebook/the-node-choice-where-the-strong-bound-pays-2026-10/analyze.py`.
//!
//! The *strong bound* of a node (promotions `P`, candidates `C`) is the Lagrangian bound over nogoods that
//! [`nogoods`](super::nogoods) fits at the root, with the GF(2) bound as its inner problem, but with the multipliers
//! fitted at the node: for the nogoods `N` live there (those whose members outside `P` are all in `C`; the others are
//! vacuous and only loosen the bound), every completion `F` has `|F ∩ N| ≤ |N - P| - 1`, so for any multipliers
//! `λ_N ≥ 0` the heaviest common independent set of the node's GF(2) matroids, each candidate `x` weighing
//! `x - Σ_{N ∋ x} λ_N`, plus `Σ λ_N (|N - P| - 1)` bounds every completion (weak duality).  The fit is `--node-steps`
//! subgradient steps (the root's, `nogoods::fit`), warm-started from the multipliers fitted at the root, and it stops
//! at the first step whose bound is below what the node needs (the node is then *cut*).
//!
//! Two more strong bounds can be measured at the same nodes, the *A1 bounds* ([`fields`](super::fields): GF(2) and
//! signed GF(3) frame matroids coupled by multipliers that a column generation fits, with a certified bracket;
//! solver/notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md): **r2**, the study's warm refit
//! (the multipliers of the game's root fit, then a column generation of at most `--a1-pricings` evaluations from the
//! root's master columns and rank inequalities that still hold at the node, with no exact check), and **full**, the
//! node's own master with the exact checks and the separation of the node's own cores, from the root fit's multipliers
//! and signings, stopping at the first bound that cuts and within `--a1-seconds` (then *capped*).  Each is the lowest
//! `⌊L⌋` it evaluated, a bound on the node's completions at any multipliers and signings, and cuts the node when it
//! is below `need`.
//!
//! The work is in two commands, so that the cloud can share the nodes among processes:
//!
//! * **choice-prepare** (`--prep DIR --known FILE N...`), once a game: the nogood pool (the nogoods that the finder
//!   and the sound search learn from the standalone start in `--cpu` CPU seconds each, as `nogoods::finder` and
//!   `nogoods::control` harvest them), the multipliers fitted at the root (`--steps` steps towards the incumbent plus
//!   one), with `--fit-seconds S` the A1 root fit (the file `DIR/nNNNN.fit.json`, which `choice --a1` reads: the fit
//!   of `--fits DIR`'s `nN.json` if that has one, else the main arm of the study fitted for S seconds), and the nodes
//!   to measure, written to `DIR/nNNNN.choice`.  The incumbent a node needs to beat is the game's
//!   known optimum (`--known`), else the incumbent of the game's units in the tuning suite (`--suite`: a game of the
//!   suite that the chain has not reached).  The nodes ([`sample`](super::sample) has the sources) are those of the
//!   fixture `--fixtures DIR` has for the game and the game's units in the suite, each source thinned to `--keep`
//!   nodes spread evenly over their gaps among those today's bound leaves open (a node it cuts has no subtree to speak
//!   of), and `--paths` nodes on the way to the optimum, whose best completion is known (a check of the bound's
//!   validity, not measured further).
//!   The line printed has the game's `n`, `incumbent`, `known`, `root` and `gap` (the root's GF(2) bound over the
//!   incumbent), `finder_pool`, `search_pool` and `pool`, `harvest_cpu_s`, `fit_steps`, `fit_cpu_s`, `active` (nogoods
//!   with a positive multiplier), `relaxed_root` and `share` (the root-λ bound and the share of the gap it closes),
//!   and for each source the nodes there were, the open ones and the units kept: `fixture`, `fixture_open`,
//!   `fixture_kept`, `suite`, `suite_open`, `suite_kept`, `path_kept`; with `--fit-seconds` also `a1_source` (`file` or
//!   `fitted`), `a1_fit_s` (the CPU seconds it took to read the fit back, checking that it gives its bound again, or
//!   to fit it), `a1_bound` (the fit's bound at the root), `a1_share` (the share of the root's gap over the
//!   incumbent that it closes), and for a fitted one `a1_stop` (how it ended: `Converged`, or the budget that ran
//!   out) and `a1_lb` (the certified lower bound on its LP, 0 if none).
//! * **choice** (`--prep DIR N...`), for each unit of the games' files, the heaviest first: its subtree, today's
//!   bound, the strong bound and the times, as a line of JSON on stdout.  With `--results DIR --worker K` the lines
//!   are also appended to `DIR/worker-K.jsonl`, and a unit that any file of `DIR` already has is skipped, so a run
//!   that stops goes on where it was; with `--claims DIR` a unit is taken by creating a file of its name there, so
//!   that workers share the units (clear the directory when a run begins again).  `--cap S` stops a unit's solve
//!   after S CPU seconds (the unit is then `capped`), `--node-steps K` is the steps of the fit (default 50), and
//!   `--sub-nogoods` repeats the fit with the nogoods that the solve's searcher held at its end besides the pool's.
//!
//! A line of `choice` is a unit: `n`, `id` (`N-SOURCE-K`), `source` (`fixture`, `suite` or `path`), `depth` (its
//! promotions), `cands`, `max_add`, `incumbent` (its promotion sum) and `known`, `need` (`incumbent + 1` less the
//! promotions' sum: what a completion must reach to beat it), `today` (today's bound on the completions, as the
//! search computes it for a unit, `NodeBound::completions`), `gap` (`today - need`, at least 0 for a node the bound
//! leaves open), `lower` (path nodes: what the best completion reaches, which no bound may be below; else null),
//! `hint` (a suite unit's nodes when the suite's default solved it; else null), the CPU seconds `today_s`, `graph_s`
//! (the node's matroids built afresh) and `plain_s` (the GF(2) bound solved on them, which must give `today`), `pool`
//! (nogoods in the file) and then
//!
//! * `strong`: the fit over the pool's nogoods: `live` (nogoods live at the node), `steps` (taken), `cut` (the bound
//!   fell below `need`), `cut_at` (the step it did, else null), `lows` (the lowest bound so far after each step, as a
//!   promotion sum of the completions, rounded down; the first is the root's multipliers kept at the node), `ts` (the
//!   wall seconds into the fit at the end of each step) and `s` (the fit's CPU seconds, the live filter included, the
//!   matroids not); and `sub`: the same with the nogoods of the solve besides (`extra`: those not in the pool), or
//!   null without `--sub-nogoods`.  That is a bound of hindsight, for what a searcher's own table could give;
//! * `a1`, with `--a1` (else null): the A1 bounds.  `build_s` is the CPU seconds of the node's GF(2) frame matroids
//!   as vectors, and `r2` and `full` are the two bounds, each with `bound` (the lowest `⌊L⌋` it evaluated), `cut` (it is
//!   below `need`), `stop` (how it ended: `Target` when it cut, `Converged` or `Fitted` when it settled, `Seconds`,
//!   `Pricings`, `Rounds` or `Breakdown` when a budget or the master ended it), `capped` (it ended before settling
//!   and did not cut: more time might have cut), `pricings` (evaluations of `L`; `r2`'s are those after the first),
//!   and `s`, the CPU seconds it took besides `build_s` (`r2`'s include its first evaluation, at the root fit's
//!   multipliers, which is `tier1`, a field of `r2` alone; `full` has `rounds`, its separation rounds).  A path node's
//!   `full` gets at most 2 CPU seconds: it cannot cut, and only its bound is checked;
//! * the unit's solve by the engine on one thread, fresh nogood tables and the forgetting of the default (as the
//!   example `tune` solves a unit) from the incumbent: `subtree` (the engine's steps), `completed` (false when `--cap`
//!   stopped it),
//!   `best` (the best promotion sum it found, the incumbent's if none better), `solve_cpu_s`, and `learned` (the
//!   nogoods its searcher held at the end).  Path nodes are not solved (null).
//!
//! A bound below the best completion's known end, a fresh bound that is not today's, a cut of a node whose subtree
//! holds a better set, or a set better than the known optimum is a fault, reported after the game's lines.

use super::case::{budgeted, Case, Clock, Played};
use super::fields::{self, Found, Limits, Node, Root, SavedFit};
use super::nogoods::{control, finder, fit, harvesting, promotable, sorted, union, Fit, Fitting, Sink};
use super::sample::{fixture_nodes, path_nodes, suite_nodes, Source};
use super::{root_bound, Line};
use crate::fixtures::{from_hex, to_hex};
use crate::known::{known_incumbent, survivors};
use crate::lns::{run_unit_with, Ran};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use taxman::bits::Bits;
use taxman::bounds::max_additional;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::json::Json;
use taxman::matchbound::lagrange::SCALE;
use taxman::model::{FrameSet, Scratch};
use taxman::search::{NodeBound, TaxUnit};

/// The version of the prepared file.
const VERSION: u64 = 1;
/// A thread's steps a second at the end of the chain, to turn a forecast of nodes into the seconds that order the units.
const NODES_PER_SECOND: f64 = 3000.0;
/// The gap it takes to multiply a subtree's size by ten, for that forecast (the floor entry's, 147 points).
const GAP_PER_DECADE: f64 = 147.0;
/// The most nogoods of a solve's table that are checked to fail to promote from the root (they are checked in the
/// searches that learned them, so this guards against a bug in the harvest only).
const CHECKED: usize = 100;

// ---------------------------------------------------------------------------------------------------------
// The incumbent and the prepared file

/// The game a node must beat: the known optimum, or for a game with none the suite's checkpoint incumbent.
struct Incumbent<const W: usize> {
    set: Bits<W>,
    sum: u64,
    known: bool,
}

/// The incumbent of the game of `case`: the known optimum's promotions, else the incumbent of its units in the suite.
fn incumbent_of<const W: usize>(case: &Case<'_, W>) -> Result<Incumbent<W>, String> {
    let n = case.n();
    if let Some((set, sum)) = known_incumbent(case.board, case.p, case.optimal)? {
        return Ok(Incumbent { set, sum, known: true });
    }
    let missing =
        || format!("n={n}: needs the game's known optimum (--known FILE) or its units in the suite (--suite)");
    let path = case.params.suite.as_ref().ok_or_else(missing)?;
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let json = Json::parse(line).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let is_game = json.get("kind").and_then(Json::as_str) == Some("game");
        if !is_game || json.get("game").and_then(Json::as_u32) != Some(n) {
            continue;
        }
        let (hex, sum) =
            (json.get("incumbent").and_then(Json::as_str), json.get("incumbent_sum").and_then(Json::as_u64));
        let (Some(hex), Some(sum)) = (hex, sum) else {
            return Err(format!("{}: game {n} has no incumbent", path.display()));
        };
        let set = survivors(case.p, &from_hex(hex)?)?;
        if set.sum() != sum {
            return Err(format!(
                "n={n}: the suite's incumbent is worth {sum}, but {} of it can be promoted",
                set.sum()
            ));
        }
        return Ok(Incumbent { set, sum, known: false });
    }
    Err(missing())
}

/// What a completion of a node with promotions `promoted` must reach to beat `incumbent`.
fn need_of<const W: usize>(incumbent: u64, promoted: &Bits<W>) -> u64 {
    (incumbent + 1).saturating_sub(promoted.sum())
}

/// A unit of a prepared file.
struct PrepUnit<const W: usize> {
    id: String,
    source: String,
    /// The seconds to measure it, forecast, to order the units (the heaviest first).
    weight: f64,
    unit: TaxUnit<W>,
    /// The nodes of the suite's solve, if it has the unit.
    hint: Option<u64>,
    /// For a node on the way to the optimum, what its best completion reaches.
    lower: Option<u64>,
    /// The gap when it was prepared.
    gap: i64,
}

/// A game's prepared file: what is measured about every node of it alike, and the nodes.
struct Prep<const W: usize> {
    n: u32,
    incumbent: Incumbent<W>,
    root: u64,
    /// The nogood pool and the multipliers fitted for it at the root.
    pool: Vec<Bits<W>>,
    lambda: Vec<i64>,
    units: Vec<PrepUnit<W>>,
}

/// The name of the file of game `n` in a prepared directory.
fn file_name(n: u32) -> String {
    format!("n{n:04}.choice")
}

/// The name of the file of game `n`'s A1 root fit in a prepared directory.
fn fit_name(n: u32) -> String {
    format!("n{n:04}.fit.json")
}

impl<const W: usize> Prep<W> {
    /// The file's lines: a header, a line a nogood and a line a unit.
    fn lines(&self) -> Vec<Json> {
        let header = Json::object([
            ("kind", Json::from("choice")),
            ("version", Json::from(VERSION)),
            ("n", Json::from(self.n)),
            ("incumbent", Json::from(self.incumbent.sum)),
            ("set", Json::from(to_hex(&self.incumbent.set))),
            ("known", Json::from(self.incumbent.known)),
            ("root", Json::from(self.root)),
            ("pool", Json::from(self.pool.len())),
            ("units", Json::from(self.units.len())),
        ]);
        let nogoods = self.pool.iter().zip(&self.lambda).map(|(set, &lambda)| {
            Json::object([
                ("kind", Json::from("nogood")),
                ("set", Json::from(to_hex(set))),
                ("lambda", Json::from(lambda)),
            ])
        });
        let units = self.units.iter().map(|u| {
            Json::object([
                ("kind", Json::from("unit")),
                ("id", Json::from(u.id.as_str())),
                ("source", Json::from(u.source.as_str())),
                ("weight", Json::from(u.weight)),
                ("promoted", Json::from(to_hex(&u.unit.promoted))),
                ("cands", Json::from(to_hex(&u.unit.cands))),
                ("hint", u.hint.map_or(Json::Null, Json::from)),
                ("lower", u.lower.map_or(Json::Null, Json::from)),
                ("gap", Json::from(u.gap)),
            ])
        });
        std::iter::once(header).chain(nogoods).chain(units).collect()
    }

    /// Writes the file to `path`, whole or not at all.
    fn write(&self, path: &Path) -> Result<(), String> {
        let text: String = self.lines().iter().map(|line| format!("{line}\n")).collect();
        let at = |e: std::io::Error| format!("{}: {e}", path.display());
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(at)?;
        }
        let partial = path.with_extension("choice.partial");
        fs::write(&partial, text).map_err(at)?;
        fs::rename(&partial, path).map_err(at)
    }

    fn read(path: &Path) -> Result<Prep<W>, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (mut header, mut pool, mut lambda, mut units) = (None, Vec::new(), Vec::new(), Vec::new());
        for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
            let json = Json::parse(line).map_err(|e| at(e.to_string()))?;
            let string = |name: &str| json.get(name).and_then(Json::as_str).ok_or_else(|| at(format!("no \"{name}\"")));
            let number = |name: &str| json.get(name).and_then(Json::as_u64).ok_or_else(|| at(format!("no \"{name}\"")));
            let optional = |name: &str| json.get(name).filter(|value| !value.is_null()).and_then(Json::as_u64);
            match string("kind")? {
                "choice" => {
                    if number("version")? != VERSION {
                        return Err(at(format!("version {} is not {VERSION}", number("version")?)));
                    }
                    let set = from_hex(string("set")?).map_err(at)?;
                    let known = json.get("known").and_then(Json::as_bool).ok_or_else(|| at("no \"known\"".into()))?;
                    header = Some((
                        number("n")? as u32,
                        Incumbent { set, sum: number("incumbent")?, known },
                        number("root")?,
                    ));
                }
                "nogood" => {
                    pool.push(from_hex(string("set")?).map_err(at)?);
                    lambda.push(json.get("lambda").and_then(Json::as_i64).ok_or_else(|| at("no \"lambda\"".into()))?);
                }
                "unit" => {
                    let sets = |name: &str| from_hex(string(name)?).map_err(at);
                    units.push(PrepUnit {
                        id: string("id")?.to_owned(),
                        source: string("source")?.to_owned(),
                        weight: json.get("weight").and_then(Json::as_f64).ok_or_else(|| at("no \"weight\"".into()))?,
                        unit: TaxUnit { promoted: sets("promoted")?, cands: sets("cands")? },
                        hint: optional("hint"),
                        lower: optional("lower"),
                        gap: json.get("gap").and_then(Json::as_i64).ok_or_else(|| at("no \"gap\"".into()))?,
                    });
                }
                other => return Err(at(format!("kind {other:?}"))),
            }
        }
        let (n, incumbent, root) = header.ok_or_else(|| format!("{}: no header", path.display()))?;
        Ok(Prep { n, incumbent, root, pool, lambda, units })
    }
}

// ---------------------------------------------------------------------------------------------------------
// choice-prepare

/// `keep` of `items`, spread evenly over their order by the key (all of them if there are no more than `keep`).
fn spread<T>(mut items: Vec<(i64, T)>, keep: usize) -> Vec<T> {
    items.sort_by_key(|&(key, _)| key);
    if items.len() <= keep {
        return items.into_iter().map(|(_, item)| item).collect();
    }
    let last = items.len() - 1;
    let picks: Vec<usize> = (0..keep).map(|i| if keep == 1 { last / 2 } else { i * last / (keep - 1) }).collect();
    let mut items: Vec<Option<T>> = items.into_iter().map(|(_, item)| Some(item)).collect();
    picks.into_iter().filter_map(|i| items[i].take()).collect()
}

/// The seconds a unit of that gap is forecast to take (a subtree of `hint` nodes when the suite has it).
fn forecast(gap: i64, hint: Option<u64>) -> f64 {
    let nodes = hint.map_or_else(|| 10f64.powf((gap as f64 - 300.0) / GAP_PER_DECADE + 1.0), |nodes| nodes as f64);
    1.0 + nodes / NODES_PER_SECOND
}

/// The file of game `n`: see the module comment.
pub fn prepare<const W: usize>(case: &Case<'_, W>) -> Result<Played, String> {
    let (n, p, params) = (case.n(), case.p, case.params);
    let dir = params.prep.as_ref().ok_or_else(|| format!("n={n}: choice-prepare needs --prep DIR"))?;
    let graph = case.space.gf2.as_ref().ok_or_else(|| format!("n={n}: choice-prepare needs the GF(2) bound"))?;
    let root = root_bound(p, graph).0.value;
    let incumbent = incumbent_of(case)?;
    let mut faults = Vec::new();
    if root < incumbent.sum {
        faults.push(format!("n={n}: the root's bound {root} is below the incumbent {}", incumbent.sum));
    }

    let start = case.start().0;
    let (found, searched) = (finder(case, &start), control(case, &start));
    let pool = union(&found.nogoods, &searched.nogoods);
    let wrong = promotable(case, &pool);
    if wrong > 0 {
        faults.push(format!("n={n}: {wrong} harvested nogoods can be promoted from the root"));
    }
    let clock = Clock::start();
    let steps = params.steps;
    let fitted = fit(&Fitting::at_root(graph, p.max_total, &pool, incumbent.sum), steps, &[steps]);
    let fit_cpu_s = clock.elapsed();
    let relaxed_root = fitted.marks[0].scaled.div_euclid(SCALE);
    if relaxed_root < incumbent.sum as i64 {
        faults.push(format!("n={n}: the root-λ bound {relaxed_root} is below the incumbent {}", incumbent.sum));
    }

    let mut bound = NodeBound::new(p, graph);
    let mut gap_of = |unit: &TaxUnit<W>| -> Result<i64, String> {
        let today = bound.completions(unit).ok_or_else(|| format!("n={n}: a node cannot be promoted"))?;
        Ok(today as i64 - need_of(incumbent.sum, &unit.promoted) as i64)
    };
    let mut units = Vec::new();
    let mut line = Line::of(n);
    line.field("incumbent", incumbent.sum)
        .field("known", incumbent.known)
        .field("root", root)
        .field("gap", root as i64 - incumbent.sum as i64)
        .field("finder_pool", found.nogoods.len())
        .field("search_pool", searched.nogoods.len())
        .field("pool", pool.len())
        .seconds("harvest_cpu_s", found.cpu_s + searched.cpu_s)
        .field("fit_steps", fitted.steps)
        .seconds("fit_cpu_s", fit_cpu_s)
        .field("active", fitted.marks[0].active)
        .field("relaxed_root", relaxed_root)
        .maybe(
            "share",
            (root > incumbent.sum)
                .then(|| format!("{:.4}", (root as i64 - relaxed_root) as f64 / (root - incumbent.sum) as f64)),
        );
    if params.fit_seconds > 0.0 {
        a1_fit(case, dir, (incumbent.sum, root), &mut line, &mut faults)?;
    }
    for source in [Source::Fixture, Source::Suite] {
        let sampled = match source {
            Source::Fixture => fixture_nodes::<W>(&params.fixtures, n, incumbent.sum)?,
            _ => suite_nodes::<W>(&params.suite, n)?,
        };
        let total = sampled.len();
        let mut open = Vec::new();
        for (k, node) in sampled.into_iter().enumerate() {
            let gap = gap_of(&node.unit)?;
            if gap >= 0 {
                open.push((gap, (k, gap, node)));
            }
        }
        let open_count = open.len();
        let kept = spread(open, params.keep);
        line.field(source.name(), total).field(&format!("{}_open", source.name()), open_count);
        line.field(&format!("{}_kept", source.name()), kept.len());
        for (k, gap, node) in kept {
            units.push(PrepUnit {
                id: format!("{n}-{}-{k}", source.name()),
                source: source.name().to_owned(),
                weight: forecast(gap, node.subtree),
                unit: node.unit,
                hint: node.subtree,
                lower: None,
                gap,
            });
        }
    }
    let mut on_the_way = 0;
    if incumbent.known {
        for (k, node) in path_nodes(case, &incumbent.set, params.paths).into_iter().enumerate() {
            let lower = incumbent.sum - node.unit.promoted.sum();
            let gap = gap_of(&node.unit)?;
            units.push(PrepUnit {
                id: format!("{n}-{}-{k}", Source::Path.name()),
                source: Source::Path.name().to_owned(),
                weight: 1.0,
                unit: node.unit,
                hint: None,
                lower: Some(lower),
                gap,
            });
            on_the_way += 1;
        }
    }
    line.field("path_kept", on_the_way);

    let prep = Prep { n, incumbent, root, pool, lambda: fitted.lambda, units };
    prep.write(&dir.join(file_name(n)))?;
    Ok(Played { line, faults })
}

/// The A1 root fit of the game, made for `choice --a1`: `--fits`' `nN.json` if it has one, else the study's main arm
/// fitted within `--fit-seconds`; checked to give its bound again at the root, written to the prepared directory
/// (`fit_name`), and described on `line`.  `(incumbent, root)`: the incumbent's promotion sum and the root's GF(2)
/// bound.
fn a1_fit<const W: usize>(
    case: &Case<'_, W>,
    dir: &Path,
    (incumbent, root): (u64, u64),
    line: &mut Line,
    faults: &mut Vec<String>,
) -> Result<(), String> {
    let (n, p, params) = (case.n(), case.p, case.params);
    let on_file = params.fits.as_ref().map(|fits| fits.join(format!("n{n}.json"))).filter(|path| path.exists());
    let clock = Clock::start();
    let (saved, how) = match &on_file {
        Some(path) => (SavedFit::load(path)?, None),
        None => {
            let fitted = fields::fit_root(p, n, incumbent, root, params.fit_seconds);
            (fitted.saved, Some((fitted.stop, fitted.lb)))
        }
    };
    let fit = Root::new(saved, p, n).map_err(|e| format!("n={n}: {e}"))?;
    let seconds = clock.elapsed();
    let bound = fit.saved().bound;
    if bound < incumbent as i64 {
        faults.push(format!("n={n}: the A1 root bound {bound} is below the incumbent {incumbent}"));
    }
    fit.saved().save(&dir.join(fit_name(n)))?;
    let share = (root > incumbent).then(|| format!("{:.4}", (root as i64 - bound) as f64 / (root - incumbent) as f64));
    line.text("a1_source", if on_file.is_some() { "file" } else { "fitted" })
        .seconds("a1_fit_s", seconds)
        .field("a1_bound", bound)
        .maybe("a1_share", share);
    match how {
        Some((stop, lb)) => line.text("a1_stop", &format!("{stop:?}")).field("a1_lb", format!("{lb:.2}")),
        None => line.field("a1_stop", "null").field("a1_lb", "null"),
    };
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// choice

/// The seconds of the A1 warm refit, which settles in a fraction of one.
const R2_SECONDS: f64 = 10.0;
/// The seconds of the A1 bound of a node on the way to the optimum, which cannot cut it: only its validity is checked.
const PATH_SECONDS: f64 = 2.0;

/// The nogood pool of a game and the multipliers fitted for it at the root.
struct Pool<'a, const W: usize> {
    sets: &'a [Bits<W>],
    lambda: &'a [i64],
}

/// What a worker keeps from node to node.
struct Tools<'a, const W: usize> {
    bound: NodeBound<'a, W>,
    scratch: Scratch<W>,
    a1: Option<A1>,
}

/// The A1 bounds of a game (`--a1`): its root fit and the budgets.
struct A1 {
    root: Root,
    limits: Limits,
    /// The seconds the node's own master has.
    full_seconds: f64,
}

/// An A1 bound at a node with the CPU seconds it took (besides the node's frame matroids) and whether it cut.
struct Timed {
    refit: Found,
    seconds: f64,
    cut: bool,
}

impl Timed {
    fn json(&self) -> Json {
        let r = &self.refit;
        let mut fields = vec![
            ("bound", Json::from(r.bound)),
            ("cut", Json::from(self.cut)),
            ("stop", Json::from(format!("{:?}", r.stop))),
            ("capped", Json::from(r.capped())),
            ("pricings", Json::from(r.pricings)),
            ("s", Json::from(self.seconds)),
        ];
        match r.tier1 {
            Some(tier1) => fields.push(("tier1", Json::from(tier1))),
            None => fields.push(("rounds", Json::from(r.rounds))),
        }
        Json::object(fields)
    }
}

/// The A1 bounds at a node whose bound must fall to `target` to cut it: the cost of its frame matroids, and the warm
/// refit and the node's own master (the latter for at most `PATH_SECONDS` at a node on the way to the optimum).
struct A1Bounds {
    build_s: f64,
    r2: Timed,
    full: Timed,
}

fn a1_bounds<const W: usize>(
    case: &Case<'_, W>,
    a1: &A1,
    state: &FrameSet<W>,
    pu: &PrepUnit<W>,
    max_add: u32,
    target: i64,
) -> A1Bounds {
    let clock = Clock::start();
    let node = Node::new(case.p, case.n(), state, &pu.unit.promoted, &pu.unit.cands, max_add, target);
    let build_s = clock.elapsed();
    let timed = |bound: &dyn Fn() -> Found| {
        let clock = Clock::start();
        let refit = bound();
        let seconds = clock.elapsed();
        Timed { cut: refit.bound <= target, refit, seconds }
    };
    let r2 = timed(&|| node.warm_refit(&a1.root, &a1.limits));
    let seconds = if pu.lower.is_some() { a1.full_seconds.min(PATH_SECONDS) } else { a1.full_seconds };
    let full = timed(&|| node.full(&a1.root, seconds));
    A1Bounds { build_s, r2, full }
}

/// A fit at a node: the nogoods live there, the fit, and the CPU seconds it took.
struct Refit {
    live: usize,
    fitted: Fit,
    seconds: f64,
}

/// The strong bound of the node with promotions `promoted` and candidates `cands`, whose matroids are `graph`: a fit
/// of `steps` steps towards a bound below `need`, over the `nogoods` live at the node (each with the multiplier to
/// begin from).
fn refit<'a, const W: usize>(
    graph: &Gf2Graph,
    cap: u32,
    nogoods: impl Iterator<Item = (&'a Bits<W>, i64)>,
    promoted: &Bits<W>,
    cands: &Bits<W>,
    need: u64,
    steps: usize,
) -> Result<Refit, String> {
    let clock = Clock::start();
    let (mut sets, mut rooms, mut start) = (Vec::new(), Vec::new(), Vec::new());
    for (set, lambda) in nogoods {
        let rest = *set - *promoted;
        if rest.is_empty() {
            return Err("a nogood is inside the promotions of a node".into());
        }
        if rest.is_subset(cands) {
            sets.push(*set);
            rooms.push(i64::from(rest.len()) - 1);
            start.push(lambda);
        }
    }
    let fitting = Fitting { graph, cap, nogoods: &sets, rooms, start, target: SCALE * need as i64 };
    let fitted = fit(&fitting, steps, &[]);
    Ok(Refit { live: sets.len(), fitted, seconds: clock.elapsed() })
}

impl Refit {
    /// The lowest bound after each step, as a promotion sum of the completions (rounded down).
    fn lows(&self) -> Vec<i64> {
        self.fitted.trace.iter().map(|&scaled| scaled.div_euclid(SCALE)).collect()
    }

    fn cut(&self) -> bool {
        self.fitted.closed_at.is_some()
    }

    fn json(&self) -> Json {
        Json::object([
            ("live", Json::from(self.live)),
            ("steps", Json::from(self.fitted.steps)),
            ("cut", Json::from(self.cut())),
            ("cut_at", self.fitted.closed_at.map_or(Json::Null, Json::from)),
            ("lows", Json::array(self.lows())),
            ("ts", Json::array(self.fitted.at.clone())),
            ("s", Json::from(self.seconds)),
        ])
    }
}

/// A unit solved as `tune solve` solves it, and what its searcher held at the end.
struct Solved<const W: usize> {
    ran: Ran<W>,
    cpu_s: f64,
    learned: Vec<Bits<W>>,
}

/// The subtree of `unit` solved on one thread from the incumbent, with fresh tables and the space's forgetting, for at
/// most `--cap` CPU seconds.
fn solve<const W: usize>(case: &Case<'_, W>, unit: TaxUnit<W>, incumbent: &Incumbent<W>) -> Solved<W> {
    let sink = Sink::<W>::default();
    let (ran, cpu_s) = budgeted(case.params.cap, |budget| {
        run_unit_with(&budget.control, unit, &incumbent.set, None, |_| harvesting(case.space, &sink))
    });
    Solved { ran, cpu_s, learned: sorted(sink) }
}

/// Every `k`th of `items`, at most `count` of them.
fn sample<T: Copy>(items: &[T], count: usize) -> Vec<T> {
    items.iter().step_by(items.len().div_ceil(count.max(1)).max(1)).copied().collect()
}

/// The measurement of one unit against `incumbent`: its line, and the faults found.
fn measure_unit<const W: usize>(
    case: &Case<'_, W>,
    pool: &Pool<'_, W>,
    incumbent: &Incumbent<W>,
    pu: &PrepUnit<W>,
    tools: &mut Tools<'_, W>,
) -> Result<(Json, Vec<String>), String> {
    let (n, p, params) = (case.n(), case.p, case.params);
    let (promoted, cands) = (pu.unit.promoted, pu.unit.cands);
    let at = format!("n={n}: {}", pu.id);
    let mut faults = Vec::new();

    let clock = Clock::start();
    let today = tools.bound.completions(&pu.unit).ok_or_else(|| format!("{at}: cannot be promoted"))?;
    let today_s = clock.elapsed();
    let need = need_of(incumbent.sum, &promoted);
    let state = p
        .model
        .promote_all_using(&mut tools.scratch, &p.root, &promoted)
        .ok_or_else(|| format!("{at}: cannot be promoted"))?;
    let max_add = max_additional(&p.model, p.max_total, &state, &promoted, &cands);
    let clock = Clock::start();
    let fresh = Gf2Graph::new(&p.model, &state, &cands);
    let graph_s = clock.elapsed();
    let clock = Clock::start();
    let mut plain = Gf2Bound::<W>::new(&fresh);
    plain.reset(&Bits::EMPTY, &cands);
    let plain_value = plain.evaluate(max_add, None).value;
    let plain_s = clock.elapsed();
    if plain_value != today {
        faults.push(format!("{at}: the bound of its matroids built afresh is {plain_value}, not {today}"));
    }

    // A node on the way to the optimum is fitted towards what its best completion reaches, which no bound goes below.
    let aim = pu.lower.unwrap_or(need);
    let pooled = pool.sets.iter().zip(pool.lambda).map(|(set, &lambda)| (set, lambda));
    let strong =
        refit(&fresh, max_add, pooled, &promoted, &cands, aim, params.node_steps).map_err(|e| format!("{at}: {e}"))?;
    let lows = strong.lows();
    if let Some(lower) = pu.lower.filter(|&lower| lows.iter().any(|&low| low < lower as i64) || today < lower) {
        faults.push(format!("{at}: a bound below {lower}: today {today}, strong {lows:?}"));
    }
    let a1 = tools.a1.as_ref().map(|a1| a1_bounds(case, a1, &state, pu, max_add, aim as i64 - 1));
    if let (Some(a1), Some(lower)) = (&a1, pu.lower) {
        for (name, bound) in [("r2", &a1.r2), ("full", &a1.full)] {
            if bound.refit.bound < lower as i64 {
                faults.push(format!("{at}: the A1 {name} bound {} is below {lower}", bound.refit.bound));
            }
        }
    }

    let solved = pu.lower.is_none().then(|| solve(case, pu.unit, incumbent));
    let mut sub = None;
    if let Some(solved) = solved.as_ref().filter(|_| params.sub) {
        let held: HashSet<&Bits<W>> = pool.sets.iter().collect();
        let extra: Vec<Bits<W>> = solved.learned.iter().filter(|set| !held.contains(set)).copied().collect();
        let wrong = promotable(case, &sample(&extra, CHECKED));
        if wrong > 0 {
            faults.push(format!("{at}: {wrong} nogoods of the solve can be promoted from the root"));
        }
        let all = pooled_and(pool, &extra);
        let again = refit(&fresh, max_add, all, &promoted, &cands, need, params.node_steps)
            .map_err(|e| format!("{at}: {e}"))?;
        sub = Some((extra.len(), again));
    }

    if let Some(solved) = &solved {
        let best = solved.ran.set.sum();
        if solved.ran.completed && best > incumbent.sum {
            if incumbent.known {
                faults.push(format!(
                    "{at}: the solve found a set worth {best}, above the known optimum {}",
                    incumbent.sum
                ));
            }
            let cuts = [
                ("strong", strong.cut()),
                ("sub", sub.as_ref().is_some_and(|(_, r)| r.cut())),
                ("A1 r2", a1.as_ref().is_some_and(|a1| a1.r2.cut)),
                ("A1 full", a1.as_ref().is_some_and(|a1| a1.full.cut)),
            ];
            for (name, cut) in cuts {
                if cut {
                    faults.push(format!("{at}: the {name} bound cut a node whose subtree holds a set worth {best}"));
                }
            }
        }
    }

    let mut line = Json::object([
        ("n", Json::from(n)),
        ("id", Json::from(pu.id.as_str())),
        ("source", Json::from(pu.source.as_str())),
        ("depth", Json::from(promoted.len())),
        ("cands", Json::from(cands.len())),
        ("max_add", Json::from(max_add)),
        ("incumbent", Json::from(incumbent.sum)),
        ("known", Json::from(incumbent.known)),
        ("need", Json::from(need)),
        ("today", Json::from(today)),
        ("gap", Json::from(today as i64 - need as i64)),
        ("lower", pu.lower.map_or(Json::Null, Json::from)),
        ("hint", pu.hint.map_or(Json::Null, Json::from)),
        ("today_s", Json::from(today_s)),
        ("graph_s", Json::from(graph_s)),
        ("plain_s", Json::from(plain_s)),
        ("pool", Json::from(pool.sets.len())),
        ("strong", strong.json()),
    ]);
    let Json::Obj(fields) = &mut line else { unreachable!("an object") };
    let mut push = |name: &str, value: Json| fields.push((name.to_owned(), value));
    push(
        "sub",
        sub.as_ref().map_or(Json::Null, |(extra, again)| {
            let Json::Obj(mut object) = again.json() else { unreachable!("an object") };
            object.insert(0, ("extra".to_owned(), Json::from(*extra)));
            Json::Obj(object)
        }),
    );
    push(
        "a1",
        a1.as_ref().map_or(Json::Null, |a1| {
            Json::object([("build_s", Json::from(a1.build_s)), ("r2", a1.r2.json()), ("full", a1.full.json())])
        }),
    );
    let solved_field = |value: Option<Json>| value.unwrap_or(Json::Null);
    push("subtree", solved_field(solved.as_ref().map(|s| Json::from(s.ran.nodes))));
    push("completed", solved_field(solved.as_ref().map(|s| Json::from(s.ran.completed))));
    push("best", solved_field(solved.as_ref().map(|s| Json::from(s.ran.set.sum()))));
    push("solve_cpu_s", solved_field(solved.as_ref().map(|s| Json::from(s.cpu_s))));
    push("learned", solved_field(solved.as_ref().map(|s| Json::from(s.learned.len()))));
    Ok((line, faults))
}

/// The pool's nogoods with their multipliers, then `extra` with none.
fn pooled_and<'a, const W: usize>(
    pool: &'a Pool<'a, W>,
    extra: &'a [Bits<W>],
) -> impl Iterator<Item = (&'a Bits<W>, i64)> {
    pool.sets.iter().zip(pool.lambda).map(|(set, &lambda)| (set, lambda)).chain(extra.iter().map(|set| (set, 0)))
}

/// Takes the unit `id` for this worker, if `dir` is given and nobody has: a file of that name is made there.
fn claim(dir: &Option<PathBuf>, id: &str) -> Result<bool, String> {
    let Some(dir) = dir else { return Ok(true) };
    let at = |e: std::io::Error| format!("{}: {e}", dir.display());
    fs::create_dir_all(dir).map_err(at)?;
    match OpenOptions::new().write(true).create_new(true).open(dir.join(id)) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(at(e)),
    }
}

/// The lines `choice` has measured: the ids any file of the results directory has, and this worker's file.
struct Results {
    done: HashSet<String>,
    file: Option<File>,
}

impl Results {
    fn open(dir: &Option<PathBuf>, worker: usize) -> Result<Results, String> {
        let Some(dir) = dir else { return Ok(Results { done: HashSet::new(), file: None }) };
        let at = |e: std::io::Error| format!("{}: {e}", dir.display());
        fs::create_dir_all(dir).map_err(at)?;
        let mut done = HashSet::new();
        for entry in fs::read_dir(dir).map_err(at)? {
            let path = entry.map_err(at)?.path();
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                // A line cut short by a kill does not parse, and its unit is measured again.
                let ids =
                    text.lines().filter_map(|line| Json::parse(line).ok()?.get("id")?.as_str().map(str::to_owned));
                done.extend(ids);
            }
        }
        let path = dir.join(format!("worker-{worker}.jsonl"));
        let cut_short = fs::read_to_string(&path).is_ok_and(|text| !text.is_empty() && !text.ends_with('\n'));
        let mut file = OpenOptions::new().append(true).create(true).open(&path).map_err(at)?;
        if cut_short {
            writeln!(file).map_err(at)?;
        }
        Ok(Results { done, file: Some(file) })
    }

    /// Prints the line, and appends it to this worker's file.
    fn write(&mut self, line: &Json) -> Result<(), String> {
        println!("{line}");
        match &mut self.file {
            Some(file) => writeln!(file, "{line}").map_err(|e| format!("--results: {e}")),
            None => Ok(()),
        }
    }
}

/// The units of game `n` that this worker measures: see the module comment.
pub fn measure<const W: usize>(case: &Case<'_, W>) -> Result<Played, String> {
    let (n, p, params) = (case.n(), case.p, case.params);
    let dir = params.prep.as_ref().ok_or_else(|| format!("n={n}: choice needs --prep DIR"))?;
    let prep = Prep::<W>::read(&dir.join(file_name(n)))?;
    let graph = case.space.gf2.as_ref().ok_or_else(|| format!("n={n}: choice needs the GF(2) bound"))?;
    let root = root_bound(p, graph).0.value;
    if prep.n != n || root != prep.root {
        return Err(format!(
            "n={n}: the file is of game {} with a root bound of {}, not {root}: another build?",
            prep.n, prep.root
        ));
    }
    let incumbent = &prep.incumbent;
    if survivors(p, &incumbent.set)?.sum() != incumbent.sum {
        return Err(format!("n={n}: the file's incumbent is not worth {} here", incumbent.sum));
    }
    let mut results = Results::open(&params.results, params.worker)?;
    let pool = Pool { sets: &prep.pool, lambda: &prep.lambda };
    let a1 = params
        .a1
        .then(|| {
            let root = Root::load(&dir.join(fit_name(n)), p, n)
                .map_err(|e| format!("n={n}: --a1 needs the fit that choice-prepare --fit-seconds makes: {e}"))?;
            let limits = Limits { pricings: params.a1_pricings, seconds: R2_SECONDS };
            Ok::<_, String>(A1 { root, limits, full_seconds: params.a1_seconds })
        })
        .transpose()?;
    let mut tools = Tools { bound: NodeBound::new(p, graph), scratch: Scratch::new(), a1 };
    let mut order: Vec<&PrepUnit<W>> = prep.units.iter().collect();
    order.sort_by(|a, b| b.weight.total_cmp(&a.weight));

    let (mut measured, mut skipped, mut elsewhere, mut faults) = (0, 0, 0, Vec::new());
    let clock = Clock::start();
    for pu in order {
        if results.done.contains(&pu.id) {
            skipped += 1;
        } else if !claim(&params.claims, &pu.id)? {
            elsewhere += 1;
        } else {
            let (line, found) = measure_unit(case, &pool, incumbent, pu, &mut tools)?;
            results.write(&line)?;
            faults.extend(found);
            measured += 1;
        }
    }
    let mut line = Line::of(n);
    line.field("units", prep.units.len())
        .field("measured", measured)
        .field("skipped", skipped)
        .field("elsewhere", elsewhere)
        .seconds("cpu_s", clock.elapsed());
    Ok(Played { line, faults })
}

// ---------------------------------------------------------------------------------------------------------
// Tests: `cargo test --features studies` runs them (the example is `test = true` in Cargo.toml)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::Params;
    use taxman::board::Board;
    use taxman::engine::Control;
    use taxman::game::Optimal;
    use taxman::matchbound::MatchGraph;
    use taxman::search::SearchTactics;

    #[test]
    fn units_are_spread_evenly_over_their_keys() {
        let items: Vec<(i64, usize)> = (0..10).map(|i| (10 - i as i64, i)).collect();
        assert_eq!(spread(items.clone(), 20).len(), 10);
        assert_eq!(spread(items.clone(), 1), vec![5]);
        // Sorted by key (10 - i), so the order is 9, 8, ..., 0, and the five picks are at 0, 2, 4, 6 and 9 of it.
        assert_eq!(spread(items.clone(), 5), vec![9, 7, 5, 3, 0]);
        assert_eq!(spread(items, 2), vec![9, 0]);
        assert!(forecast(1000, None) > forecast(500, None) && forecast(0, None) < 2.0);
        assert!(forecast(0, Some(30_000)) > 10.0);
    }

    /// A small prepared file, written and read back whole.
    #[test]
    fn a_prepared_file_reads_back_as_written() {
        let set = |members: &[u32]| members.iter().copied().collect::<Bits<16>>();
        let prep = Prep::<16> {
            n: 936,
            incumbent: Incumbent { set: set(&[5, 700]), sum: 705, known: false },
            root: 800,
            pool: vec![set(&[3, 5, 9]), set(&[4, 700])],
            lambda: vec![64, 0],
            units: vec![PrepUnit {
                id: "936-suite-3".into(),
                source: "suite".into(),
                weight: 12.5,
                unit: TaxUnit { promoted: set(&[5]), cands: set(&[7, 9, 900]) },
                hint: Some(40_000),
                lower: None,
                gap: 120,
            }],
        };
        let dir = crate::support::TempDir::new("choice-file");
        let path = dir.0.join(file_name(936));
        prep.write(&path).unwrap();
        let back = Prep::<16>::read(&path).unwrap();
        assert_eq!(
            (back.n, back.root, back.lambda.clone(), back.pool.clone()),
            (936, 800, vec![64, 0], prep.pool.clone())
        );
        assert_eq!((back.incumbent.set, back.incumbent.sum, back.incumbent.known), (set(&[5, 700]), 705, false));
        let (a, b) = (&back.units[0], &prep.units[0]);
        assert_eq!(
            (&a.id, &a.source, a.weight, a.unit, a.hint, a.lower, a.gap),
            (&b.id, &b.source, 12.5, b.unit, Some(40_000), None, 120)
        );
        assert!(!path.with_extension("choice.partial").exists());
    }

    /// At nodes of game 936 (their best completion found exhaustively), against incumbents the node cannot beat and
    /// incumbents it can: no step of the fit goes below the best completion, a cut agrees with the solve (it finds
    /// nothing better after a cut, and a node that holds a better set is never cut), and the matroids built afresh
    /// give today's bound.
    fn valid_and_agreeing<const W: usize>() {
        let n = 936;
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let graph = MatchGraph::new(&p.model, &p.root);
        let mut space = taxman::search::SearchSpace::new(&p, &graph);
        space.forgetting.period = None;
        let params = Params { cpu: 0.2, steps: 30, node_steps: 20, sub: true, ..Params::default() };
        let case = Case { board: &board, p: &p, space: &space, params: &params, optimal: Optimal::built_in() };
        let (optimum, opt) = known_incumbent(&board, &p, case.optimal).unwrap().expect("a known game");
        let gf2 = space.gf2.as_ref().expect("the GF(2) bound");
        let start = case.start().0;
        let pool = union(&finder(&case, &start).nogoods, &control(&case, &start).nogoods);
        assert!(!pool.is_empty());
        let fitted = fit(&Fitting::at_root(gf2, p.max_total, &pool, opt), params.steps, &[params.steps]);
        let pool = Pool { sets: &pool, lambda: &fitted.lambda };
        // The A1 root fit, short: any fit makes valid bounds.
        let root = root_bound(&p, gf2).0.value;
        let a1_fit = fields::fit_root(&p, n, opt, root, 3.0);
        let a1 = A1 {
            root: Root::new(a1_fit.saved, &p, n).expect("a fit that gives its bound again"),
            limits: Limits { pricings: params.a1_pricings, seconds: R2_SECONDS },
            full_seconds: 1.0,
        };
        let mut tools = Tools { bound: NodeBound::new(&p, gf2), scratch: Scratch::new(), a1: Some(a1) };

        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/perf");
        let nodes = fixture_nodes::<W>(&Some(fixtures), n, opt).unwrap();
        let (mut measured, mut cut_at_the_end, mut cut_by_a1, mut open) = (0, 0, 0, 0);
        for (k, node) in nodes.iter().enumerate().step_by(7).take(12) {
            let unit = node.unit;
            // The node's best completion, from an incumbent of nothing: the best set below it, or its promotions alone.
            let ran = crate::lns::run_unit(&space, &Control::new(), unit, &Bits::EMPTY, None);
            assert!(ran.completed);
            let best = if ran.set.sum() > unit.promoted.sum() { ran.set } else { unit.promoted };
            let lower = best.sum() - unit.promoted.sum();
            // Incumbents that no completion beats (the best set itself, and the game's optimum, which is what the
            // fixtures' proofs are seeded with and where the strong bound cuts), and one it beats (the best set without
            // its largest member outside the promotions).
            let beaten = best.difference(&unit.promoted).max().map(|x| {
                let mut set = best;
                set.remove(x);
                set
            });
            for (incumbent, beatable) in [(Some(best), false), (Some(optimum), false), (beaten, true)] {
                let Some(set) = incumbent else { continue };
                let incumbent = Incumbent { set, sum: set.sum(), known: set == optimum };
                let pu = PrepUnit {
                    id: format!("{n}-fixture-{k}"),
                    source: "fixture".into(),
                    weight: 1.0,
                    unit,
                    hint: None,
                    lower: None,
                    gap: 0,
                };
                let (line, faults) = measure_unit(&case, &pool, &incumbent, &pu, &mut tools).unwrap();
                assert!(faults.is_empty(), "node {k}: {faults:?}");
                let number = |value: &Json, name: &str| {
                    value.get(name).and_then(Json::as_i64).unwrap_or_else(|| panic!("{name} in {line}"))
                };
                measured += 1;
                let need = number(&line, "need");
                assert_eq!(need, incumbent.sum as i64 + 1 - unit.promoted.sum() as i64, "{line}");
                open += i32::from(number(&line, "gap") >= 0);
                for variant in ["strong", "sub"] {
                    let Some(fit) = line.get(variant).filter(|fit| !fit.is_null()) else { continue };
                    let lows: Vec<i64> =
                        fit.get("lows").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_i64).collect();
                    assert!(!lows.is_empty() && lows.windows(2).all(|w| w[1] <= w[0]), "{variant}: {lows:?}");
                    // No step of the fit is below what the best completion reaches.
                    assert!(
                        lows.iter().all(|&low| low >= lower as i64),
                        "node {k}, {variant}: {lows:?} against {lower}: {line}"
                    );
                    let cut = fit.get("cut").and_then(Json::as_bool).unwrap();
                    assert_eq!(cut, lows.last().is_some_and(|&low| low < need), "node {k}, {variant}: {line}");
                    if beatable {
                        assert!(!cut, "node {k}: cut although a completion beats the incumbent: {line}");
                    }
                    cut_at_the_end += i32::from(cut);
                }
                // The A1 bounds: neither is below the best completion, each cuts exactly when it is below `need`, and
                // neither cuts a node whose completion beats the incumbent.
                let a1 = line.get("a1").filter(|a1| !a1.is_null()).unwrap_or_else(|| panic!("a1 in {line}"));
                for variant in ["r2", "full"] {
                    let found = a1.get(variant).unwrap_or_else(|| panic!("a1 {variant} in {line}"));
                    let bound = number(found, "bound");
                    assert!(bound >= lower as i64, "node {k}, A1 {variant}: {bound} against {lower}: {line}");
                    let cut = found.get("cut").and_then(Json::as_bool).unwrap();
                    assert_eq!(cut, bound < need, "node {k}, A1 {variant}: {line}");
                    if beatable {
                        assert!(
                            !cut,
                            "node {k}: cut by A1 {variant} although a completion beats the incumbent: {line}"
                        );
                    }
                    cut_by_a1 += i32::from(cut);
                }
                // The solve agrees: from an incumbent the node cannot beat it finds nothing, else the best set.
                let completed = line.get("completed").and_then(Json::as_bool).unwrap();
                let solved_best = number(&line, "best");
                assert!(completed);
                assert_eq!(solved_best, best.sum().max(incumbent.sum) as i64, "node {k}: {line}");
            }
        }
        assert!(measured >= 12, "{measured} measurements");
        assert!(cut_by_a1 > 0, "no node cut by an A1 bound");
        eprintln!("choice: {measured} measurements, {open} open, {cut_at_the_end} cut by a fit, {cut_by_a1} by A1");
    }

    #[test]
    fn the_strong_bound_is_never_below_the_best_completion_and_its_cut_agrees_with_the_solve() {
        taxman::with_width!(936, W => valid_and_agreeing::<W>());
    }
}
