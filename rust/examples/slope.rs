//! What drives the slope of the cost curve: the root of each game measured against what its search cost, and
//! proofs searched with every bound tightened by a known margin.  The study, its data and its conclusion are in
//! docs/EXPERIMENTS.md, "What drives the slope (2026-10)".
//!
//! ```text
//! cargo run --release --example slope -- roots [--known FILE] [--sums FILE] [--tactic NAME=VALUE]... [--depth D]
//!     [--lagrange K] N|A-B...
//! cargo run --release --example slope -- prove [--known FILE] [--sums FILE] [--tactic NAME=VALUE]...
//!     [--deltas D,D...] [--threads T] [--seconds S] [--diag] N|A-B...
//! ```
//!
//! Games are set up as `taxman solve` sets them up (`taxman::board::Board`, the search tactics given with
//! `--tactic`, named as for `taxman solve`), and the optimum of each, `P*` (its score minus `in_the_bag`, the
//! promotion sum), is read from the known games: optimal.json, and the completed games of `--known`; or from
//! `--sums`, lines of the ladder as `bench/curve` records them (`n`, `status` `DONE`, `best`).  A range
//! `A-B` stands for its games that are not prime and have candidates.  Each command prints one JSON object per
//! line, per game (`roots`) or per search (`prove`).
//!
//! **roots**: what the root of each game looks like, with no search.
//!
//! * `matching`: the matching bound of the root, the size of its heaviest matching `Q*`, and whether `Q*` can be
//!   promoted (then the bound is the optimum).
//! * `refined`: the matching bound refined by the branches of the conflicts of its heaviest matchings, 1 to
//!   `--depth` levels deep (default 3): the search's refinement (`search::refine`, `match_refine`), computed to a
//!   value instead of compared with what a node needs.  Every level is a bound, and each is at most the one
//!   before.
//! * `forced_in`, `forced_out`, `undecided`: with the incumbent at the optimum (a completion must reach
//!   `P* + 1`), the members of `Q*` without which the matching bound falls below it (every better set would
//!   promote them), the candidates that cannot be promoted from the root or with which it falls below it (no
//!   better set promotes them), and the candidates left, which is what the search has to decide.
//! * `lagrange`: a Lagrangian bound over nogoods of the root (`matchbound::lagrange`), fitted by at most
//!   `--lagrange` (default 8000; 0 skips it) subgradient steps towards `P* + 1` (see [`root_lagrange`]), whose
//!   nogoods are generated as it goes: the conflicts of the refinement above, and at each step conflicts of the
//!   weighted matching's candidates (`FrameGraph::conflict_using`, minimal).  The search learns its nogoods at its
//!   nodes; these are the root's own, so this is about the best bound the root has with nogoods and no search.
//!   `refit` is what `matchbound::lagrange::fit` makes of the same nogoods in the steps of one of the search's
//!   refits (which, unlike this, starts from the multipliers of the refit before).  With the incumbent at the
//!   optimum, `forced_in`, `forced_out` and `undecided` as above, by this bound.
//! * `start`: the incumbent the chain's search of the game starts from (the best of the greedy set, the previous
//!   game's promotions that survive, the matching dive, improved by local search: `search::search` stopped
//!   before its first node), seeded from the known game of `n − 1` if there is one (`seeded`); `cap`: the
//!   ceiling on the promotion sum from that game.
//!
//! **prove**: for each `--deltas` margin `δ` (default 0), a search of the game whose incumbent starts at the value
//! `P* + δ`, with no set behind it (the engine's incumbent is a value), no seed and no ceiling.  With `δ ≥ 0`
//! nothing beats it, so the search proves that nothing does, and every cut it makes is the cut it would make with
//! every bound (matching, refined, Lagrangian, forced members) lowered by `δ`: the tree of a uniformly tighter
//! bound, and `δ = 0` is the proof of the optimum (`ladder --seed-opt`).  With `δ < 0` the search must also find a
//! set better than `P* + δ`, as a search seeded below the optimum does.  One thread (the default) gives exact,
//! repeatable node counts.  `--diag` collects the search's diagnostics (slower) and reports its steps by the
//! number of promotions of the node taking them.

#[path = "common/root_lagrange.rs"]
mod root_lagrange;

use root_lagrange::{root_lagrange, RootLagrange};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, EngineConfig, Start};
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::matchbound::lagrange::{LagrangeBound, Relaxation, SCALE};
use taxman::matchbound::{MatchBound, MatchGraph};
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::search::{search, Diagnostics, Prepared, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};
use taxman::tactics::Tactics;

struct Options {
    command: String,
    known: Option<PathBuf>,
    sums: Option<PathBuf>,
    tactics: SearchTactics,
    depth: u32,
    lagrange: usize,
    deltas: Vec<i64>,
    threads: usize,
    seconds: Option<u64>,
    diag: bool,
    games: Vec<u32>,
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("expected a command: roots or prove")?;
    if command != "roots" && command != "prove" {
        return Err(format!("unknown command {command:?}: roots or prove"));
    }
    let mut o = Options {
        command,
        known: None,
        sums: None,
        tactics: SearchTactics::default(),
        depth: 3,
        lagrange: 8000,
        deltas: vec![0],
        threads: 1,
        seconds: None,
        diag: false,
        games: Vec::new(),
    };
    let mut ranges = Vec::new();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        let number = |name: &str, text: String| text.parse::<u64>().map_err(|e| format!("{name} {text}: {e}"));
        match arg.as_str() {
            "--known" => o.known = Some(value("--known")?.into()),
            "--sums" => o.sums = Some(value("--sums")?.into()),
            "--tactic" => {
                // The solver's parser, so that the names and values are those of `taxman solve --tactic`.
                let spec = value("--tactic")?;
                let mut tactics = Tactics { search: o.tactics, ..Tactics::default() };
                tactics.set(&spec).map_err(|e| format!("--tactic: {e}"))?;
                if tactics != (Tactics { search: tactics.search, ..Tactics::default() }) {
                    return Err(format!("--tactic {spec}: only search tactics apply"));
                }
                o.tactics = tactics.search;
            }
            "--depth" => o.depth = number("--depth", value("--depth")?)? as u32,
            "--lagrange" => o.lagrange = number("--lagrange", value("--lagrange")?)? as usize,
            "--deltas" => {
                let list = value("--deltas")?;
                let deltas: Result<Vec<i64>, _> = list.split(',').map(str::parse).collect();
                o.deltas = deltas.map_err(|e| format!("--deltas {list}: {e}"))?;
            }
            "--threads" => o.threads = number("--threads", value("--threads")?)?.max(1) as usize,
            "--seconds" => o.seconds = Some(number("--seconds", value("--seconds")?)?),
            "--diag" => o.diag = true,
            game => ranges.push(game.to_owned()),
        }
    }
    for range in ranges {
        let parse_n = |text: &str| text.parse::<u32>().map_err(|e| format!("{range}: {e}"));
        match range.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (parse_n(a)?, parse_n(b)?);
                o.games.extend((a..=b).filter(|&n| !is_prime(n)));
            }
            None => o.games.push(parse_n(&range)?),
        }
    }
    if o.games.is_empty() {
        return Err("no games given".into());
    }
    Ok(o)
}

fn main() {
    let o = parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    let known = Known::load(&o).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    for &n in &o.games {
        if known.optimal.score(n).is_none() && !known.sums.contains_key(&n) {
            eprintln!("n={n}: no known game, skipped");
            continue;
        }
        taxman::with_width!(n, W => match o.command.as_str() {
            "roots" => roots::<W>(n, &o, &known),
            _ => prove::<W>(n, &o, &known),
        });
    }
}

/// What is known of the optima: the known games, and the promotion sums of `--sums`.
struct Known {
    optimal: Optimal,
    sums: HashMap<u32, u64>,
}

impl Known {
    fn load(o: &Options) -> Result<Known, String> {
        let optimal = Optimal::built_in().with_results_files(o.known.as_slice())?;
        let mut sums = HashMap::new();
        if let Some(path) = &o.sums {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let line = Json::parse(line).map_err(|e| format!("{}: {e}", path.display()))?;
                if line.get("status").and_then(Json::as_str) == Some("DONE") {
                    let field = |name| line.get(name).and_then(Json::as_u64);
                    if let (Some(n), Some(best)) = (field("n"), field("best")) {
                        sums.insert(n as u32, best);
                    }
                }
            }
        }
        Ok(Known { optimal, sums })
    }

    /// The optimum promotion sum of the board `n` whose game with no promotions scores `in_the_bag`.
    fn opt(&self, n: u32, in_the_bag: u64) -> u64 {
        match self.sums.get(&n) {
            Some(&sum) => sum,
            None => self.optimal.score(n).expect("a known game") - in_the_bag,
        }
    }
}

/// The board of `n` set up with `o`'s tactics, and its optimum promotion sum `P*`; `None` for a board with no
/// candidates.
fn setup<const W: usize>(n: u32, o: &Options, known: &Known) -> Option<(Board<W>, Prepared<W>, u64)> {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return None;
    }
    let p = board.prepare(o.tactics).expect("prepare");
    let opt = known.opt(n, board.in_the_bag);
    Some((board, p, opt))
}

fn secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

fn is_prime(n: u32) -> bool {
    n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| !n.is_multiple_of(d))
}

// ---------------------------------------------------------------------------------------------------------
// roots

fn roots<const W: usize>(n: u32, o: &Options, known: &Known) {
    let t0 = Instant::now();
    let Some((board, p, opt)) = setup::<W>(n, o, known) else { return };
    let optimal = &known.optimal;
    let setup_time = t0.elapsed();
    let (model, root, cands) = (&p.model, &p.root, p.candidates);
    let graph = MatchGraph::new(model, root);
    let mut m = MatchBound::new(&graph);
    m.reset(&Bits::EMPTY, &cands);
    let mut scratch = Scratch::new();

    // The matching bound and its heaviest matching.
    let bound = m.value();
    let chosen = m.chosen();
    let promotable = model.conflict_using(&mut scratch, root, &chosen, true).is_none();

    // Refined, 1 to `depth` levels.
    let t1 = Instant::now();
    let mut conflicts = Vec::new();
    let refined: Vec<u64> = (1..=o.depth)
        .map(|d| refine(&mut m, model, &mut scratch, root, Bits::EMPTY, d, &mut conflicts).unwrap_or(0))
        .collect();
    let refine_time = t1.elapsed();

    // What the matching bound decides at the root when a completion must reach P* + 1.
    let need = opt + 1;
    let mut forced_in = Bits::<W>::EMPTY;
    for x in chosen.iter() {
        if m.value_excluding(x) < need {
            forced_in.insert(x);
        }
        m.pop();
    }
    let mut forced_out = Bits::<W>::EMPTY;
    for x in (cands - chosen).iter() {
        let out = !model.can_promote(root, x) || {
            m.push(x, &cands.without(x));
            let with_x = m.value() + u64::from(x);
            m.pop();
            with_x < need
        };
        if out {
            forced_out.insert(x);
        }
    }
    let undecided = cands - forced_in - forced_out;

    // The Lagrangian bound with nogoods generated at the root, and what it decides.
    let t2 = Instant::now();
    let lagrange = (o.lagrange > 0).then(|| {
        let l = root_lagrange(&graph, model, &mut scratch, root, &cands, conflicts, need, o.lagrange);
        let (forced_in, forced_out) = decided(&graph, model, root, &cands, &l, need);
        (l, forced_in, forced_out)
    });
    let lagrange_time = t2.elapsed();

    // The chain's starting incumbent: the search stopped before its first node.
    let prev = optimal.moves(n - 1).map(|moves| board.seed(moves));
    let cap = optimal.score(n - 1).and_then(|s| board.promotion_cap(s));
    let cfg = EngineConfig { threads: 1, time_limit: Some(Duration::ZERO), ..EngineConfig::default() };
    let start = search(&p, &cfg, prev.as_ref(), cap, false).expect("search").start.sum();

    let list = |v: &[u64]| v.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    let lagrange = match lagrange {
        Some((l, forced_in, forced_out)) => format!(
            "{{\"bound\":{},\"scaled\":{},\"refit\":{},\"nogoods\":{},\"steps\":{},\"closed_at\":{},\"forced_in\":{},\
             \"forced_out\":{},\"undecided\":{}}}",
            l.scaled.div_euclid(SCALE),
            l.scaled,
            l.refit.div_euclid(SCALE),
            l.nogoods.len(),
            l.steps,
            l.closed_at.map_or("null".into(), |s| s.to_string()),
            forced_in.len(),
            forced_out.len(),
            (cands - forced_in - forced_out).len(),
        ),
        None => "null".into(),
    };
    println!(
        "{{\"n\":{n},\"in_the_bag\":{},\"opt\":{opt},\"candidates\":{},\"frames\":{},\"max_total\":{},\"greedy\":{},\
         \"start\":{start},\"seeded\":{},\"cap\":{},\
         \"matching\":{{\"bound\":{bound},\"chosen\":{},\"promotable\":{promotable}}},\"refined\":[{}],\
         \"forced_in\":{},\"forced_out\":{},\"undecided\":{},\"lagrange\":{lagrange},\
         \"seconds\":{{\"setup\":{:.4},\"refine\":{:.4},\"lagrange\":{:.4},\"total\":{:.4}}}}}",
        board.in_the_bag,
        cands.len(),
        root.len(),
        p.max_total,
        p.greedy.sum(),
        prev.is_some(),
        cap.map_or("null".into(), |c| c.to_string()),
        chosen.len(),
        list(&refined),
        forced_in.len(),
        forced_out.len(),
        undecided.len(),
        secs(setup_time),
        secs(refine_time),
        secs(lagrange_time),
        secs(t0.elapsed()),
    );
}

/// The matching bound of `m`'s top level with the members of `required` promoted on it (pushed, as the search's
/// refinement pushes them), refined `depth` levels deep: the largest of the refined bounds of the branches of
/// the conflict of its heaviest matching (all members of the conflict before the `j`-th required, the `j`-th
/// excluded), or the bound itself if that matching can be promoted.  The state of the top level is `root` (the
/// root of the search, so its conflicts are nogoods); the conflicts found are added to `found`.  `None` if no
/// branch is left (the required members cannot be promoted together).  Leaves `m` as it found it.
fn refine<const W: usize>(
    m: &mut MatchBound<'_, W>,
    model: &FrameGraph,
    scratch: &mut Scratch<W>,
    root: &FrameSet<W>,
    required: Bits<W>,
    depth: u32,
    found: &mut Vec<Bits<W>>,
) -> Option<u64> {
    let value = m.value() + required.sum();
    if depth == 0 {
        return Some(value);
    }
    let promoted = m.chosen() | required;
    let Some(k) = model.conflict_using(scratch, root, &promoted, true) else { return Some(value) };
    found.push(k);
    let free = k - required;
    let last = free.max();
    let (mut required, mut best, mut pushed) = (required, None, 0);
    for x in free.iter() {
        m.push_excluding(&Bits::EMPTY.with(x));
        best = best.max(refine(m, model, scratch, root, required, depth - 1, found));
        m.pop();
        if Some(x) == last {
            break;
        }
        // The next branches have x.
        let cands = m.candidates();
        m.push(x, &cands);
        required.insert(x);
        pushed += 1;
    }
    for _ in 0..pushed {
        m.pop();
    }
    best
}

/// What the Lagrangian bound `l` decides at the root when a completion must reach `need`: the members of its
/// heaviest matching without which it falls below (from its potentials, `LagrangeBound::forced`), and the
/// candidates that cannot be promoted from the root or with which it falls below.
fn decided<const W: usize>(
    graph: &MatchGraph,
    model: &FrameGraph,
    root: &FrameSet<W>,
    cands: &Bits<W>,
    l: &RootLagrange<W>,
    need: u64,
) -> (Bits<W>, Bits<W>) {
    let target = SCALE * need as i64;
    let mut solver = LagrangeBound::new(graph);
    solver.set_relaxation(Relaxation::new(graph, &l.nogoods, &l.lambda));
    solver.value(&Bits::EMPTY, cands).expect("the root's moves can be matched");
    let (forced_in, chosen) = (solver.forced(target, &Bits::EMPTY), solver.chosen());
    let mut forced_out = Bits::EMPTY;
    for x in (*cands - chosen).iter() {
        let promoted = Bits::EMPTY.with(x);
        let out = !model.can_promote(root, x)
            || solver.value(&promoted, &cands.without(x)).is_none_or(|v| v + SCALE * i64::from(x) < target);
        if out {
            forced_out.insert(x);
        }
    }
    (forced_in, forced_out)
}

// ---------------------------------------------------------------------------------------------------------
// prove

fn prove<const W: usize>(n: u32, o: &Options, known: &Known) {
    let Some((_, p, opt)) = setup::<W>(n, o, known) else { return };
    let graph = MatchGraph::new(&p.model, &p.root);
    for &delta in &o.deltas {
        let best = opt.checked_add_signed(delta).expect("a margin below zero");
        let diag = o.diag.then(|| Mutex::new(Diagnostics::default()));
        let space = SearchSpace { diagnostics: diag.as_ref(), ..SearchSpace::new(&p, &graph) };
        let cfg = EngineConfig {
            threads: o.threads,
            time_limit: o.seconds.map(Duration::from_secs),
            ..EngineConfig::default()
        };
        // The incumbent is a value only: its payload, an empty set, is never read back here.
        let payload = vec![0u8; 8 * W];
        let start =
            Start::Fresh { units: vec![TaxUnit { promoted: Bits::<W>::EMPTY, cands: p.candidates }], best, payload };
        let t0 = Instant::now();
        let out = engine::run(&cfg, start, b"slope", |_| TaxSearcher::new(&space)).expect("search");
        let time = t0.elapsed();
        let depths = diag.map_or("null".into(), |d| {
            let steps = d.into_inner().expect("diagnostics").steps;
            format!("[{}]", steps.iter().map(u64::to_string).collect::<Vec<_>>().join(","))
        });
        println!(
            "{{\"n\":{n},\"opt\":{opt},\"delta\":{delta},\"incumbent\":{best},\"threads\":{},\"completed\":{},\
             \"nodes\":{},\"best\":{},\"seconds\":{:.3},\"steps_by_depth\":{depths}}}",
            o.threads,
            out.completed,
            out.stats.nodes,
            out.best,
            secs(time),
        );
    }
}
