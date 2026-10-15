//! The fields study: a bound on the promotions that adds GF(3) signed frame matroids to the two GF(2) frame matroids
//! and couples them by Lagrangian decomposition (avenue A1 of `docs/AGENDA.md`, experiment E1; the record is
//! `docs/EXPERIMENTS.md`, "A bound over GF(2) and GF(3) signed frame matroids").  `common/dw.rs` has the bound and
//! why it is valid (`heuristics/THEORY.md`, Theorems 14 and 15), `common/gf3.rs` the signed
//! matroids, `common/cores.rs` the cores and their signings, `common/lp.rs` the master's simplex.
//!
//! ```text
//! cargo run --release --example fields -- roots --known results/strict.jsonl [--arms gf2,f1,f1r1,f1r3,cores,f1cores]
//!     [--rounds 40] [--per-frame 8] [--cores-per-frame 4] [--pricings 3000] [--seconds 120] [--fixed-seconds 30]
//!     [--frames 1,2,3] [--defaults random|ones] [--fit-mode certified|search] [--out FILE] [--census FILE]
//!     N|A-B|A-B/STEP...
//! cargo run --release --example fields -- nodes --known results/strict.jsonl [--modes root,node] [--seconds 10]
//!     [--root-seconds 120] [--fit-mode certified|search] [--exact bench/matroid/nodes_exact.jsonl] [--out FILE]
//!     FIXTURE...
//! cargo run --release --example fields -- check [--known FILE] [--samples 100] N|A-B...
//! ```
//!
//! Games are set up as `taxman solve` sets them up; `P*`, the optimum promotion sum, is the known score less
//! `in_the_bag` (`--known` adds results files to the built-in table, which ends at 1300).  `A-B` stands for its games
//! that are not prime, `A-B/STEP` for A, A + STEP, ... up to B.  Every command writes one JSON object per line, to
//! `--out` (appended, and a game or node already there is skipped, so a run resumes) or to standard output.
//!
//! * **roots**: per game and *arm* (the family of extra matroids), the bound and its bracket (see `common/dw.rs`):
//!   `gf2` none (the GF(2) bound); `f1` the plain signing (all values 1) of each frame of `--frames`; `f1r1`, `f1r3`
//!   that and one or three random signings per frame; `cores` the core-derived signings that the separation adds
//!   alone; `f1cores` (the main arm) the plain signings and the core-derived ones.  `--seconds` budgets the two core
//!   arms, `--fixed-seconds` the others.  With `--census`, every core the main arm finds, with its census data.
//! * **nodes**: per node of a `.nodes` fixture of `testdata/perf` (target `P* − sum(promoted)`, cut when the bound is
//!   at most it), the bound in mode `root` (the main arm's final signings and multipliers at the root, one evaluation
//!   at the node's matroids) and `node` (the node's own master and separation from those signings, `--seconds` a
//!   node), each the lower of it and the node's GF(2) bound.  `--exact` gives best completions to check against.
//! * **check**: Theorems 14 and 15 and the column-sum signings at scale (the tests 3, 4 and 6 of the study on any
//!   boards).
//!
//! `--fit-mode` (of `roots` and `nodes`, every computation of the run): `certified` (the default) brackets each LP
//! with a certified lower bound (the exact checks); `search` fits the multipliers as a search would, without the
//! exact checks (a family's generation ends when nothing prices out and no cheap inequality is violated, a
//! separation round that places no core ends the computation), and reports `lb` 0, so `converged` false.
//!
//! Every run asserts that the bound is at least `P*` (at nodes, at least the best completion where known), that the
//! optimal game's promotions are independent in every final matroid, and checks the crate's intersection against
//! the reference algorithm every 50th evaluation.

#[allow(dead_code)]
#[path = "common/cores.rs"]
mod cores;
#[allow(dead_code)]
#[path = "common/dw.rs"]
mod dw;
#[allow(dead_code)]
#[path = "common/fixtures.rs"]
mod fixtures;
#[allow(dead_code)]
#[path = "common/frames.rs"]
mod frames;
#[allow(dead_code)]
#[path = "common/gf3.rs"]
mod gf3;
#[allow(dead_code)]
#[path = "common/lp.rs"]
mod lp;
#[allow(dead_code)]
#[path = "common/meet.rs"]
mod meet;

use cores::{core_of, cores_of, frame_with, merge, primes, residual, sees, zero_one, Core};
use dw::{bound, evaluate, Ctx, Family, FitMode, Outcome, Settings, Stats};
use frames::{frame_matroids, is_prime, setup, Known, Rng};
use gf3::{seed, Default3, Matroid3, Signing};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use taxman::bits::Bits;
use taxman::json::Json;
use taxman::model::{FrameGraph, FrameSet, Scratch};

/// The salt of the random arms' seeds.
const SALT_RANDOM: u64 = 1;

// ---------------------------------------------------------------------------------------------------------
// Options

struct Options {
    command: String,
    known: Vec<PathBuf>,
    arms: Vec<String>,
    modes: Vec<String>,
    settings: Settings,
    fixed_seconds: f64,
    root_seconds: f64,
    frames: Vec<usize>,
    out: Option<PathBuf>,
    census: Option<PathBuf>,
    exact: Option<PathBuf>,
    samples: usize,
    games: Vec<u32>,
    files: Vec<PathBuf>,
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("expected a command: roots, nodes or check")?;
    let mut o = Options {
        command,
        known: Vec::new(),
        arms: ["gf2", "f1", "f1r1", "f1r3", "cores", "f1cores"].map(String::from).to_vec(),
        modes: vec!["root".into(), "node".into()],
        settings: Settings::default(),
        fixed_seconds: 30.0,
        root_seconds: 120.0,
        frames: vec![1, 2, 3],
        out: None,
        census: None,
        exact: None,
        samples: 100,
        games: Vec::new(),
        files: Vec::new(),
    };
    let mut seconds = None;
    let list = |s: String| s.split(',').map(str::to_string).collect::<Vec<_>>();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        let num = |v: String| v.parse::<f64>().map_err(|e| format!("{v}: {e}"));
        match arg.as_str() {
            "--known" => o.known.push(value("--known")?.into()),
            "--arms" => o.arms = list(value("--arms")?),
            "--modes" => o.modes = list(value("--modes")?),
            "--rounds" => o.settings.rounds = num(value("--rounds")?)? as usize,
            "--per-frame" => o.settings.per_frame = num(value("--per-frame")?)? as usize,
            "--cores-per-frame" => o.settings.cores_per_frame = num(value("--cores-per-frame")?)? as usize,
            "--pricings" => o.settings.pricings = num(value("--pricings")?)? as usize,
            "--seconds" => seconds = Some(num(value("--seconds")?)?),
            "--fixed-seconds" => o.fixed_seconds = num(value("--fixed-seconds")?)?,
            "--root-seconds" => o.root_seconds = num(value("--root-seconds")?)?,
            "--samples" => o.samples = num(value("--samples")?)? as usize,
            "--close" => o.settings.close = num(value("--close")?)?,
            "--frames" => {
                o.frames = list(value("--frames")?)
                    .iter()
                    .map(|f| f.parse().map_err(|e| format!("{f}: {e}")))
                    .collect::<Result<_, _>>()?
            }
            "--defaults" => {
                o.settings.random_defaults = match value("--defaults")?.as_str() {
                    "random" => true,
                    "ones" => false,
                    other => return Err(format!("unknown defaults {other:?}")),
                }
            }
            "--fit-mode" => {
                o.settings.mode = match value("--fit-mode")?.as_str() {
                    "certified" => FitMode::Certified,
                    "search" => FitMode::Search,
                    other => return Err(format!("unknown fit mode {other:?}")),
                }
            }
            "--out" => o.out = Some(value("--out")?.into()),
            "--census" => o.census = Some(value("--census")?.into()),
            "--exact" => o.exact = Some(value("--exact")?.into()),
            file if o.command == "nodes" => o.files.push(file.into()),
            range => o.games.extend(games(range)?),
        }
    }
    let default_seconds = if o.command == "nodes" { 10.0 } else { 120.0 };
    o.settings.seconds = seconds.unwrap_or(default_seconds);
    Ok(o)
}

/// The games of `N`, `A-B` (those that are not prime) or `A-B/STEP`.
fn games(range: &str) -> Result<Vec<u32>, String> {
    let parse_n = |t: &str| t.parse::<u32>().map_err(|e| format!("{range}: {e}"));
    let (span, step) = match range.split_once('/') {
        Some((span, step)) => (span, Some(parse_n(step)?)),
        None => (range, None),
    };
    Ok(match (span.split_once('-'), step) {
        (Some((a, b)), Some(step)) => (parse_n(a)?..=parse_n(b)?).step_by(step.max(1) as usize).collect(),
        (Some((a, b)), None) => (parse_n(a)?..=parse_n(b)?).filter(|&n| !is_prime(n)).collect(),
        (None, _) => vec![parse_n(span)?],
    })
}

/// Where lines go: a file, appended (with the keys of the lines already there), or standard output.
struct Output {
    file: Option<std::fs::File>,
    done: HashSet<String>,
}

impl Output {
    /// `key` names the fields that identify a line (a line is skipped when its key is done).
    fn open(path: Option<&Path>, key: &[&str]) -> Output {
        let Some(path) = path else { return Output { file: None, done: HashSet::new() } };
        let mut done = HashSet::new();
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let json = Json::parse(line).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                done.insert(Self::key_of(&json, key));
            }
        }
        let file = std::fs::OpenOptions::new().create(true).append(true).open(path);
        Output { file: Some(file.unwrap_or_else(|e| panic!("{}: {e}", path.display()))), done }
    }

    fn key_of(json: &Json, key: &[&str]) -> String {
        key.iter()
            .map(|k| match json.get(k) {
                Some(Json::Str(s)) => s.clone(),
                Some(v) => v.as_u64().map_or_else(|| format!("{v:?}"), |x| x.to_string()),
                None => String::new(),
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn done(&self, key: &str) -> bool {
        self.done.contains(key)
    }

    fn write(&mut self, line: &str) {
        match &mut self.file {
            Some(f) => {
                writeln!(f, "{line}").expect("write");
                f.flush().expect("flush");
            }
            None => println!("{line}"),
        }
    }
}

fn main() {
    let o = parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    let known = Known::load(&o.known, None).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    match o.command.as_str() {
        "roots" => {
            let mut out = Output::open(o.out.as_deref(), &["n", "arm"]);
            let mut census = o.census.as_deref().map(|p| Output::open(Some(p), &["n", "index"]));
            for &n in &o.games {
                taxman::with_width!(n, W => roots::<W>(n, &known, &o, &mut out, census.as_mut()));
            }
        }
        "nodes" => {
            let mut out = Output::open(o.out.as_deref(), &["n", "node", "mode"]);
            let exact = o.exact.as_deref().map(load_exact).unwrap_or_default();
            for path in &o.files {
                let n = fixture_n(path);
                taxman::with_width!(n, W => nodes::<W>(path, &known, &o, &exact, &mut out));
            }
        }
        "check" => {
            for &n in &o.games {
                taxman::with_width!(n, W => check::<W>(n, &known, o.samples));
            }
        }
        other => {
            eprintln!("error: unknown command {other:?}");
            std::process::exit(2);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Arms

/// The plain and random signings of an arm: the plain signing of each frame of `frames` (that the state has),
/// then `random` random signings of each.
fn fixed_signings(n: u32, len: usize, frames: &[usize], plain: bool, random: usize) -> Vec<Signing> {
    let frames: Vec<usize> = frames.iter().copied().filter(|&h| h < len).collect();
    let mut out: Vec<Signing> = Vec::new();
    if plain {
        out.extend(frames.iter().map(|&h| Signing::ones(h)));
    }
    for &h in &frames {
        out.extend((0..random).map(|i| Signing::random(h, seed(n, h, i, SALT_RANDOM))));
    }
    out
}

/// The initial signings of `arm` and whether it separates.
fn arm(name: &str, n: u32, len: usize, frames: &[usize]) -> (Vec<Signing>, bool) {
    match name {
        "gf2" => (Vec::new(), false),
        "f1" => (fixed_signings(n, len, frames, true, 0), false),
        "f1r1" => (fixed_signings(n, len, frames, true, 1), false),
        "f1r3" => (fixed_signings(n, len, frames, true, 3), false),
        "cores" => (Vec::new(), true),
        "f1cores" => (fixed_signings(n, len, frames, true, 0), true),
        other => panic!("unknown arm {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------------------
// roots

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

/// The optimal game's promotions, if the known game reaches `P*`.
fn optimal_promotions<const W: usize>(
    n: u32,
    known: &Known,
    board: &taxman::board::Board<W>,
    opt: u64,
) -> Option<Bits<W>> {
    let seed = board.seed(known.optimal.moves(n)?);
    (seed.sum() == opt).then_some(seed)
}

/// Asserts what every run asserts of an outcome: the optimal promotions independent in every final matroid.
fn assert_independent<const W: usize>(n: u32, fam: &Family, set: &Bits<W>, what: &str) {
    let members: Vec<u32> = set.iter().collect();
    for ex in &fam.extras {
        assert!(
            ex.m.independent(&members),
            "n={n}: {what} is dependent in the signed matroid of frame {} ({:?})",
            ex.m.frame,
            ex.signing.default
        );
    }
}

fn roots<const W: usize>(n: u32, known: &Known, o: &Options, out: &mut Output, census: Option<&mut Output>) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known optimum, skipped");
        return;
    };
    let model = &p.model;
    let fm = frame_matroids(model, &p.root, &p.candidates);
    let cap = p.max_total as usize;
    let gf2 = fm.bound(cap).value;
    let mut ctx = Ctx::new(n, model, &p.root, &fm, cap, opt as i64, false);
    ctx.gap = Some(gf2 - opt as i64);
    let optimal = optimal_promotions(n, known, &board, opt);
    if let Some(set) = &optimal {
        fm.independent(set).unwrap_or_else(|e| panic!("n={n}: the optimal promotions: {e}"));
    }
    let mut census = census;
    for name in &o.arms {
        if out.done(&format!("{n}/{name}")) {
            continue;
        }
        let (signings, separate) = arm(name, n, p.root.len(), &o.frames);
        let seconds = if separate { o.settings.seconds } else { o.fixed_seconds };
        let settings = Settings { seconds, ..o.settings.clone() };
        let r = bound(&ctx, signings, None, separate, &settings);
        assert!(r.bound >= opt as i64, "n={n}, arm {name}: the bound {} is below P* = {opt}", r.bound);
        if name == "gf2" {
            assert_eq!(r.bound, gf2, "n={n}: the gf2 arm differs from the GF(2) bound");
        }
        if let Some(set) = &optimal {
            assert_independent(n, &r.family, set, "the optimal promotions");
        }
        out.write(&root_line(n, name, opt, gf2, &r, model, &p.root, p.root.len()));
        if name == "f1cores" {
            if let Some(c) = census.as_deref_mut() {
                for (i, f) in r.cores.iter().enumerate() {
                    if !c.done(&format!("{n}/{i}")) {
                        c.write(&census_line(n, i, f.core.clone(), f.assigned));
                    }
                }
            }
        }
        eprintln!(
            "n={n} {name}: bound {} (gf2 {gf2}, P* {opt}), bracket [{:.2}, {:.2}], {:?}, {:.1} s",
            r.bound, r.lb, r.l_best, r.stop, r.seconds
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn root_line<const W: usize>(
    n: u32,
    arm: &str,
    opt: u64,
    gf2: i64,
    r: &Outcome,
    model: &FrameGraph,
    root: &FrameSet<W>,
    frames: usize,
) -> String {
    let gap = gf2 - opt as i64;
    let bracket = r.l_best - r.lb;
    let converged = gap > 0 && bracket <= 0.01 * gap as f64;
    let share = if gap > 0 { format!("{:.5}", (gf2 - r.bound) as f64 / gap as f64) } else { "null".into() };
    let mut per_frame = vec![0usize; frames];
    let mut core_signings = Vec::new();
    for ex in &r.family.extras {
        per_frame[ex.m.frame] += 1;
        if ex.signing.mergeable {
            core_signings.push(format!("[{},{}]", ex.m.frame, ex.signing.cores.len()));
        }
    }
    let mut scratch = Scratch::<W>::new();
    let (mut promotable, mut unpromotable_mass) = (0, 0.0);
    for (a, set) in &r.support {
        let bits: Bits<W> = Bits::from_slice(set);
        if model.can_promote_all_using(&mut scratch, root, &bits) {
            promotable += 1;
        } else {
            unpromotable_mass += a;
        }
    }
    let s: &Stats = &r.stats;
    format!(
        "{{\"n\":{n},\"arm\":\"{arm}\",\"opt\":{opt},\"gf2\":{gf2},\"bound\":{},\"lb\":{:.4},\"l_best\":{:.4},\
         \"bracket\":{bracket:.4},\"converged\":{converged},\"share\":{share},\"stop\":\"{:?}\",\"matroids\":[{}],\
         \"core_signings\":[{}],\"cores_found\":{},\"cores_assigned\":{},\"cores_skipped\":{},\
         \"cores_repeated\":{},\"gf3_loops\":{},\"rounds\":{},\"pricings_mi\":{},\"pricings_greedy\":{},\
         \"pivots\":{},\"degenerate\":{},\"repairs\":{},\"solves\":{},\"masters\":{},\"columns\":{},\"cuts\":{},\
         \"master\":{:.4},\"cuts_cheap\":{},\"cuts_exact\":{},\"checks_exact\":{},\"pivots_exact\":{},\"max_s\":{},\
         \"purged\":{},\"restarts\":{},\"checks_capped\":{},\"support\":{},\
         \"support_promotable\":{promotable},\"mass_unpromotable\":{unpromotable_mass:.5},\"checks\":{},\
         \"ms\":{:.0},\"ms_mi\":{:.0},\"ms_simplex\":{:.0}}}",
        r.bound,
        r.lb,
        r.l_best,
        r.stop,
        list(&per_frame),
        core_signings.join(","),
        r.cores.len(),
        r.cores_assigned,
        r.cores_unplaced,
        r.cores_repeated,
        r.family.loops(),
        r.rounds,
        s.pricings_mi,
        s.pricings_greedy,
        s.pivots,
        s.degenerate,
        s.repairs,
        s.solves,
        s.masters,
        r.columns,
        r.cuts,
        r.master,
        s.cuts_cheap,
        s.cuts_exact,
        s.checks_exact,
        s.pivots_exact,
        s.max_s,
        s.purged,
        s.restarts,
        s.checks_capped,
        r.support.len(),
        s.checks,
        r.seconds * 1e3,
        s.ms_mi,
        s.ms_simplex,
    )
}

/// The census of a core the main arm found (for avenue A4 and conjecture C2): its size, its 0/1 matrix's
/// determinant mod 3 when square (else its rank deficiency over GF(3)), its primes, whether the plain signing and
/// the three random signings of its frame (those of arm `f1r3`) see it, and the signing it went to.
fn census_line(n: u32, index: usize, core: Core, assigned: Option<usize>) -> String {
    let (det0, deficiency) = zero_one(&core);
    let r3 = (0..3).any(|i| sees(&Signing::random(core.frame, seed(n, core.frame, i, SALT_RANDOM)), &core));
    format!(
        "{{\"n\":{n},\"index\":{index},\"frame\":{},\"size\":{},\"members\":[{}],\"factors\":{},\"square\":{},\
         \"primes\":{},\"det0\":{},\"deficiency3\":{deficiency},\"promoted_in\":{},\"promoted_out\":{},\
         \"seen_f1\":{},\"seen_r3\":{r3},\"assigned_to\":{}}}",
        core.frame,
        core.members.len(),
        list(&core.members),
        core.columns.len(),
        core.columns.len() == core.members.len(),
        primes(&core),
        det0.map_or("null".into(), |d| d.to_string()),
        core.promoted_in.len(),
        core.promoted_out.len(),
        sees(&Signing::ones(core.frame), &core),
        assigned.map_or("null".into(), |a| a.to_string()),
    )
}

// ---------------------------------------------------------------------------------------------------------
// nodes

fn fixture_n(path: &Path) -> u32 {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let line = text.lines().find(|l| l.starts_with("n ")).expect("an n line");
    line[2..].trim().parse().expect("a board size")
}

/// The best completions of `bench/matroid/nodes_exact.jsonl`, by (n, depth, promoted sum, candidates).
type Exact = HashMap<(u64, u64, u64, u64), u64>;

fn load_exact(path: &Path) -> Exact {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = HashMap::new();
    let mut clash = HashSet::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let json = Json::parse(line).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let field = |k| json.get(k).and_then(Json::as_u64);
        let (Some(n), Some(d), Some(s), Some(c)) =
            (field("n"), field("depth"), field("promoted_sum"), field("candidates"))
        else {
            continue;
        };
        if let Some(e) = field("exact") {
            if out.insert((n, d, s, c), e).is_some_and(|old| old != e) {
                clash.insert((n, d, s, c));
            }
        }
    }
    for k in clash {
        out.remove(&k);
    }
    out
}

fn nodes<const W: usize>(path: &Path, known: &Known, o: &Options, exact: &Exact, out: &mut Output) {
    let (n, incumbent, units) = fixtures::read_nodes::<W>(path).expect("a fixture");
    let (board, p) = setup::<W>(n).expect("a game with candidates");
    let opt = known.opt(n, board.in_the_bag).expect("a known optimum");
    let model = &p.model;
    let fm = frame_matroids(model, &p.root, &p.candidates);
    let mut ctx = Ctx::new(n, model, &p.root, &fm, p.max_total as usize, opt as i64, false);
    ctx.gap = Some(fm.bound(p.max_total as usize).value - opt as i64);
    let (signings, _) = arm("f1cores", n, p.root.len(), &o.frames);
    let root_settings = Settings { seconds: o.root_seconds, ..o.settings.clone() };
    let root = bound(&ctx, signings, None, true, &root_settings);
    assert!(root.bound >= opt as i64, "n={n}: the root's bound {} is below P* = {opt}", root.bound);
    let final_signings: Vec<Signing> = root.family.extras.iter().map(|ex| ex.signing.clone()).collect();
    eprintln!(
        "n={n}: P* {opt} (fixture incumbent {}), root bound {} in {:.1} s, {} signings",
        incumbent.sum(),
        root.bound,
        root.seconds,
        final_signings.len()
    );
    let mut scratch = Scratch::<W>::new();
    for (i, u) in units.iter().filter(|u| !u.cands.is_empty()).enumerate() {
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).expect("a node of the game");
        let max_add = (p.max_total as i32 - u.promoted.len() as i32).min(state.max_promotions_among(&u.cands)).max(0);
        let fmn = frame_matroids(model, &state, &u.cands);
        let gf2 = fmn.bound(max_add as usize).value;
        let target = opt as i64 - u.promoted.sum() as i64;
        let key = (u64::from(n), u64::from(u.promoted.len()), u.promoted.sum(), u64::from(u.cands.len()));
        let best = exact.get(&key).copied();
        for mode in &o.modes {
            if out.done(&format!("{n}/{i}/{mode}")) {
                continue;
            }
            let t = Instant::now();
            let ctxn = Ctx::new(n, model, &state, &fmn, max_add as usize, target, true);
            let (l_bound, pricings) = match mode.as_str() {
                "root" => {
                    let fam = Family::new(&ctxn, final_signings.clone());
                    let lam: Vec<Vec<f64>> = fam
                        .extras
                        .iter()
                        .zip(&root.lambda)
                        .map(|(ex, l)| ex.m.elems.iter().map(|x| l.get(x).copied().unwrap_or(0.0)).collect())
                        .collect();
                    let mut stats = Stats::default();
                    let ev = evaluate(&ctxn, &fam, &lam, &mut stats, 1);
                    (ev.value.div_euclid(dw::SCALE), 1)
                }
                "node" => {
                    // The root's signings stay as they are (their cores are the root's); the node's separation adds
                    // signings of its own.
                    let frozen = final_signings.iter().map(dw::frozen).collect();
                    let r = bound(&ctxn, frozen, Some(&root.lambda), true, &o.settings);
                    (r.bound, r.stats.pricings_mi)
                }
                other => panic!("unknown mode {other:?}"),
            };
            let value = l_bound.min(gf2);
            if let Some(best) = best {
                assert!(value >= best as i64, "n={n}, node {i}: the bound {value} is below the best completion {best}");
            }
            out.write(&format!(
                "{{\"n\":{n},\"node\":{i},\"mode\":\"{mode}\",\"depth\":{},\"target\":{target},\"gf2\":{gf2},\
                 \"l\":{l_bound},\"bound\":{value},\"cut\":{},\"excess\":{},\"exact\":{},\"pricings\":{pricings},\
                 \"ms\":{:.1}}}",
                u.promoted.len(),
                value <= target,
                value - target,
                best.map_or("null".into(), |b| b.to_string()),
                t.elapsed().as_secs_f64() * 1e3,
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// check

/// A random maximal promotable set: the candidates promoted in a random order, those that can be.
fn random_promotable<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, cands: &[u32], rng: &mut Rng) -> Bits<W> {
    let mut order = cands.to_vec();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i as u64 + 1) as usize);
    }
    let mut state = *root;
    let mut set = Bits::<W>::EMPTY;
    for &x in &order {
        if model.promote_in_place(&mut state, x) {
            set.insert(x);
        }
    }
    set
}

/// The column-sum signing of a core: merged into a fresh signing of its frame.
fn column_sum_signing(core: &Core, seed: u64) -> Signing {
    let mut s = Signing::for_cores(core.frame, Default3::Random(seed));
    assert!(merge(&mut s, core, &[]), "a core merges into a fresh signing");
    s
}

/// The checks of Theorem 14, the column-sum signings and Theorem 15 at the root of `n` (see the module docs), with
/// `samples` random sets of each kind; prints what was checked.
fn check<const W: usize>(n: u32, known: &Known, samples: usize) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let model = &p.model;
    let root = &p.root;
    let cands: Vec<u32> = p.candidates.iter().collect();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
    let mut scratch = Scratch::<W>::new();
    // Theorem 14: the optimal promotions and random maximal promotable sets under the plain and random signings.
    let mut promotable: Vec<Bits<W>> = (0..samples).map(|_| random_promotable(model, root, &cands, &mut rng)).collect();
    let opt = known.opt(n, board.in_the_bag);
    if let Some(set) = opt.and_then(|o| optimal_promotions(n, known, &board, o)) {
        promotable.push(set);
    }
    let mut signings: Vec<Signing> = Vec::new();
    for h in 0..root.len() {
        signings.push(Signing::ones(h));
        signings.extend((0..5).map(|i| Signing::random(h, seed(n, h, i, SALT_RANDOM))));
    }
    // The converse: random unpromotable sets, made dependent by the column-sum signings of their residuals and cores.
    let (mut unpromotable, mut cores_checked) = (0, 0);
    let mut generated = 0;
    let mut tries = 0;
    while unpromotable < samples && tries < 100 * samples {
        tries += 1;
        let q: Vec<u32> = cands.iter().copied().filter(|_| rng.below(4) == 0).collect();
        if model.can_promote_all_using(&mut scratch, root, &Bits::from_slice(&q)) {
            continue;
        }
        unpromotable += 1;
        for h in 0..root.len() {
            let (movers, pool) = frame_with(model, root, &q, h);
            let stuck = residual(model, &movers, &pool);
            if stuck.is_empty() {
                continue;
            }
            let mut found = vec![core_of(model, root, h, stuck, &pool, &q)];
            found.extend(cores_of(model, root, &q, 4).into_iter().filter(|c| c.frame == h));
            for core in found {
                let s = column_sum_signing(&core, seed(n, h, cores_checked, 3));
                let m = Matroid3::build(model, root, &cands, &s);
                assert!(!m.independent(&q), "n={n}: a core's column-sum signing leaves its set independent");
                cores_checked += 1;
                if generated < 20 {
                    signings.push(s);
                    generated += 1;
                }
            }
        }
    }
    for s in &signings {
        let m = Matroid3::build(model, root, &cands, s);
        for set in &promotable {
            let members: Vec<u32> = set.iter().collect();
            assert!(m.independent(&members), "n={n}: a promotable set is dependent in frame {}", s.frame);
        }
    }
    // Theorem 15: random multipliers never bound below P*; zero multipliers give the GF(2) bound.
    let fm = frame_matroids(model, root, &p.candidates);
    let cap = p.max_total as usize;
    let gf2 = fm.bound(cap).value;
    let mut duality = 0;
    if let Some(opt) = opt {
        let ctx = Ctx::new(n, model, root, &fm, cap, opt as i64, false);
        let fam = Family::new(&ctx, signings.clone());
        let mut stats = Stats::default();
        let empty = Family::new(&ctx, Vec::new());
        let zero = evaluate(&ctx, &empty, &[], &mut stats, 1);
        assert_eq!(zero.value, gf2 * dw::SCALE, "n={n}: L at zero multipliers is not the GF(2) bound");
        for _ in 0..20 {
            let lam: Vec<Vec<f64>> = fam
                .extras
                .iter()
                .map(|ex| {
                    let r = &mut rng;
                    ex.m.elems
                        .iter()
                        .map(|&x| if r.below(10) < 3 { (r.below(2001) as f64 / 1000.0) * f64::from(x) } else { 0.0 })
                        .collect()
                })
                .collect();
            let ev = evaluate(&ctx, &fam, &lam, &mut stats, 1);
            assert!(ev.value.div_euclid(dw::SCALE) >= opt as i64, "n={n}: L below P*");
            duality += 1;
        }
    }
    println!(
        "{{\"n\":{n},\"candidates\":{},\"signings\":{},\"promotable\":{},\"unpromotable\":{unpromotable},\
         \"cores\":{cores_checked},\"duality\":{duality},\"gf2\":{gf2}}}",
        cands.len(),
        signings.len(),
        promotable.len(),
    );
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dw::{cut_rank, fit_from_json, refit_r1, refit_r2, tier1, Fit, Trace, Warm};
    use crate::lp::{Lp, RowKind, Status};
    use crate::meet::{Reduced, Span};

    /// Every promotable set of a board with few candidates, by exhaustion.
    fn all_promotable<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, cands: &[u32]) -> Vec<Bits<W>> {
        let mut out = Vec::new();
        let mut scratch = Scratch::<W>::new();
        let mut stack = vec![(*root, Bits::<W>::EMPTY, 0usize)];
        while let Some((state, set, from)) = stack.pop() {
            out.push(set);
            for (i, &x) in cands.iter().enumerate().skip(from) {
                if let Some(next) = model.promote_all_using(&mut scratch, &state, &Bits::EMPTY.with(x)) {
                    stack.push((next, set.with(x), i + 1));
                }
            }
        }
        out
    }

    fn signings_of(n: u32, len: usize) -> Vec<Signing> {
        let mut out = Vec::new();
        for h in 0..len {
            out.push(Signing::ones(h));
            out.extend((0..5).map(|i| Signing::random(h, seed(n, h, i, SALT_RANDOM))));
        }
        out
    }

    /// The study's test 3: Theorem 14 at every root with at most 22 candidates, by exhaustion, and at
    /// the roots of 137-400 for the optimal promotions and 100 random maximal promotable sets.
    #[test]
    fn promotable_sets_are_independent_in_every_signed_matroid() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let (mut boards, mut sets) = (0, 0);
        for n in 2..=400u32 {
            let Some((board, p)) = setup::<8>(n) else { continue };
            let cands: Vec<u32> = p.candidates.iter().collect();
            let promotable: Vec<Bits<8>> = if cands.len() <= 22 {
                all_promotable(&p.model, &p.root, &cands)
            } else if !is_prime(n) && n >= 137 {
                let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ u64::from(n));
                let mut v: Vec<Bits<8>> =
                    (0..100).map(|_| random_promotable(&p.model, &p.root, &cands, &mut rng)).collect();
                let opt = known.opt(n, board.in_the_bag).expect("a built-in optimum");
                v.push(optimal_promotions(n, &known, &board, opt).expect("the optimal promotions"));
                v
            } else {
                continue;
            };
            boards += 1;
            for s in signings_of(n, p.root.len()) {
                let m = Matroid3::build(&p.model, &p.root, &cands, &s);
                for set in &promotable {
                    let members: Vec<u32> = set.iter().collect();
                    assert!(m.independent(&members), "n={n}: {members:?} dependent in frame {}", s.frame);
                    sets += 1;
                }
            }
        }
        eprintln!("test 3: {boards} boards, {sets} (set, matroid) pairs");
        assert!(boards > 200);
    }

    /// The study's test 6: at every non-prime root of 2-400, 20 random multipliers (30% of the coordinates nonzero,
    /// uniform in [0, 2x]) never give a bound below `P*`; at zero multipliers `L` is the GF(2) bound (no family, or a
    /// family without loops), and never above it.
    #[test]
    fn weak_duality() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let (mut roots, mut evals) = (0, 0);
        for n in (2..=400u32).filter(|&n| !is_prime(n)) {
            let Some((board, p)) = setup::<8>(n) else { continue };
            let Some(opt) = known.opt(n, board.in_the_bag) else { continue };
            let fm = frame_matroids(&p.model, &p.root, &p.candidates);
            let cap = p.max_total as usize;
            let gf2 = fm.bound(cap).value;
            let ctx = Ctx::new(n, &p.model, &p.root, &fm, cap, opt as i64, false);
            let mut stats = Stats::default();
            let empty = Family::new(&ctx, Vec::new());
            assert_eq!(evaluate(&ctx, &empty, &[], &mut stats, 1).value, gf2 * dw::SCALE, "n={n}");
            let fam = Family::new(&ctx, signings_of(n, p.root.len()));
            let zero = evaluate(&ctx, &fam, &fam_zeros(&fam), &mut stats, 1).value;
            assert!(zero <= gf2 * dw::SCALE, "n={n}");
            if fam.loops() == 0 {
                assert_eq!(zero, gf2 * dw::SCALE, "n={n}");
            }
            let mut rng = Rng(0xD1B5_4A32_D192_ED03 ^ u64::from(n));
            for _ in 0..20 {
                let lam: Vec<Vec<f64>> = fam
                    .extras
                    .iter()
                    .map(|ex| {
                        ex.m.elems
                            .iter()
                            .map(|&x| {
                                if rng.below(10) < 3 {
                                    (rng.below(1 << 20) as f64 / (1 << 20) as f64) * 2.0 * f64::from(x)
                                } else {
                                    0.0
                                }
                            })
                            .collect()
                    })
                    .collect();
                let v = evaluate(&ctx, &fam, &lam, &mut stats, 1).value;
                assert!(v.div_euclid(dw::SCALE) >= opt as i64, "n={n}: L = {v} below P* = {opt}");
                evals += 1;
            }
            roots += 1;
        }
        eprintln!("test 6: {roots} roots, {evals} evaluations");
        assert!(roots > 250);
    }

    fn fam_zeros(fam: &Family) -> Vec<Vec<f64>> {
        fam.extras.iter().map(|ex| vec![0.0; ex.elem.len()]).collect()
    }

    /// Whether `set` (element indices) is independent in a GF(2) block matroid.
    fn independent_gf2(m: &crate::meet::BlockMatroid, set: &[usize]) -> bool {
        let mut span = Span::new(m);
        set.iter().all(|&e| {
            let free = matches!(span.reduce(m, e), Reduced::Free);
            if free {
                span.insert(m, e);
            }
            free
        })
    }

    /// The study's test 7: on boards with at most 16 candidates, the exact LP (every common independent set of `A`
    /// and `B` of at most `cap` elements and every independent set of each extra matroid as a column) against the
    /// column generation: the certified lower bound and the lowest `L` bracket the LP, and both are within 10⁻⁵ of it,
    /// relatively (run until nothing prices out; the pricing is at multipliers rounded to the grid 1/S, so a column is
    /// missed only by the rounding, and the perturbations of the rows move the point by 10⁻⁶ a row).
    #[test]
    fn column_generation_reaches_the_exact_lp() {
        let (mut instances, mut tighter) = (0, 0);
        // Every board with 4-16 candidates.
        for n in 2..=200u32 {
            let Some((_, p)) = setup::<8>(n) else { continue };
            if (4..=16).contains(&p.candidates.len()) {
                tighter += usize::from(exact_against_generated(n, &p, &p.candidates));
                instances += 1;
            }
        }
        // Their GF(2) bounds are their optima, so the extra matroids cannot cut: also boards of 300-700 restricted to
        // at most 12 candidates, those of the cores of the GF(2) bound's heaviest set first (its members, then the
        // other candidates in its frames).
        for n in (300..=700u32).filter(|&n| !is_prime(n)).step_by(5) {
            let Some((_, p)) = setup::<16>(n) else { continue };
            let fm = frame_matroids(&p.model, &p.root, &p.candidates);
            let heaviest: Vec<u32> = fm.chosen::<16>(&fm.bound(p.max_total as usize)).iter().collect();
            let cores = cores_of(&p.model, &p.root, &heaviest, 1);
            let Some(core) = cores.iter().min_by_key(|c| c.promoted_in.len() + c.promoted_out.len()) else { continue };
            let mut chosen: Vec<u32> = core.promoted_in.iter().chain(&core.promoted_out).copied().collect();
            let frames = [core.frame, core.frame + 1];
            let all: Vec<u32> = p.candidates.iter().collect();
            let more = heaviest.iter().chain(&all).copied();
            for x in more.filter(|&x| frames.contains(&p.model.home(x))) {
                if chosen.len() < 12 && !chosen.contains(&x) {
                    chosen.push(x);
                }
            }
            tighter += usize::from(exact_against_generated(n, &p, &Bits::from_slice(&chosen)));
            instances += 1;
        }
        eprintln!("test 7: {instances} instances, {tighter} with the LP below the GF(2) bound");
        assert!(instances >= 80 && tighter >= 10);
    }

    /// Test 7 on one instance: the board of `n` with the candidates `cands`; the family is the plain and a random
    /// signing of each frame and the column-sum signings of the cores of the GF(2) bound's heaviest set and of three
    /// random sets.  Whether the LP is below the GF(2) bound.
    fn exact_against_generated<const W: usize>(n: u32, p: &taxman::search::Prepared<W>, cands: &Bits<W>) -> bool {
        let fm = frame_matroids(&p.model, &p.root, cands);
        let cap = p.max_total as usize;
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, cap, 0, false);
        let mut signings: Vec<Signing> = (1..p.root.len())
            .flat_map(|h| [Signing::ones(h), Signing::random(h, seed(n, h, 0, SALT_RANDOM))])
            .collect();
        let gf2 = fm.bound(cap);
        let mut rng = Rng(0x5851_F42D_4C95_7F2D ^ u64::from(n));
        let mut sets: Vec<Vec<u32>> = vec![fm.chosen::<W>(&gf2).iter().collect()];
        sets.extend((0..3).map(|_| fm.elems.iter().copied().filter(|_| rng.below(2) == 0).collect::<Vec<u32>>()));
        for q in &sets {
            for core in cores_of(&p.model, &p.root, q, 2) {
                signings.push(column_sum_signing(&core, seed(n, core.frame, signings.len(), 3)));
            }
        }
        let fam = Family::new(&ctx, signings.clone());
        let lp = exact_lp(&ctx, &fam, cap);
        let settings = Settings {
            seconds: 60.0,
            pricings: 100_000,
            polyak: 5,
            check_every: 1,
            close: 1e-9,
            ..Settings::default()
        };
        let r = bound(&ctx, signings, None, false, &settings);
        assert!(r.lb <= lp + 1e-6, "n={n}: the certified bound {} above the LP {lp}", r.lb);
        // The certified point loses what the perturbations (10⁻⁶ a row) and the checks' tolerance leave out.
        let near = 1e-5 * lp.max(1.0);
        assert!(r.lb >= lp - near, "n={n}: the certified bound {} below the LP {lp}", r.lb);
        assert!(r.l_best >= lp - 1e-6, "n={n}: L {} below the LP {lp}", r.l_best);
        assert!(r.l_best <= lp + near, "n={n}: L {} far above the LP {lp}", r.l_best);
        lp < gf2.value as f64 - 1e-6
    }

    /// The LP over `P(A) ∩ P(B) ∩ ⋂ P(N_k)` with `Σ z ≤ cap`, every column enumerated.
    fn exact_lp<const W: usize>(ctx: &Ctx<'_, W>, fam: &Family, cap: usize) -> f64 {
        let e = ctx.len();
        let mut lp = Lp::new();
        let conv0 = lp.add_row(RowKind::Eq, 1.0, &[]);
        let conv: Vec<usize> = fam.extras.iter().map(|_| lp.add_row(RowKind::Eq, 1.0, &[])).collect();
        let mut link = vec![vec![usize::MAX; e]; fam.extras.len()];
        for (k, ex) in fam.extras.iter().enumerate() {
            for &x in &ex.elem {
                link[k][x] = lp.add_row(RowKind::Le, 0.0, &[]);
            }
        }
        let mut empties = Vec::new();
        for mask in 0u32..(1 << e) {
            let set: Vec<usize> = (0..e).filter(|&i| mask >> i & 1 == 1).collect();
            if set.len() > cap || set.iter().any(|&i| fam.banned[i]) {
                continue;
            }
            if !independent_gf2(&ctx.fm.a, &set) || !independent_gf2(&ctx.fm.b, &set) {
                continue;
            }
            let mut entries = vec![(conv0, 1.0)];
            for (k, row) in link.iter().enumerate() {
                let _ = k;
                entries.extend(set.iter().filter(|&&i| row[i] != usize::MAX).map(|&i| (row[i], 1.0)));
            }
            let cost = set.iter().map(|&i| f64::from(ctx.fm.elems[i])).sum();
            let j = lp.add_column(cost, &entries);
            if set.is_empty() {
                empties.push(j);
            }
        }
        for (k, ex) in fam.extras.iter().enumerate() {
            let m = ex.elem.len();
            assert!(m <= 16);
            for mask in 0u32..(1 << m) {
                let numbers: Vec<u32> = (0..m).filter(|&i| mask >> i & 1 == 1).map(|i| ex.m.elems[i]).collect();
                if !ex.m.independent(&numbers) {
                    continue;
                }
                let mut entries = vec![(conv[k], 1.0)];
                entries.extend((0..m).filter(|&i| mask >> i & 1 == 1).map(|i| (link[k][ex.elem[i]], -1.0)));
                let j = lp.add_column(0.0, &entries);
                if mask == 0 {
                    empties.push(j);
                }
            }
        }
        lp.reset(&empties);
        assert_eq!(lp.solve(u64::MAX), Status::Optimal);
        lp.value()
    }

    /// The study's test 8: the crate's intersection against the reference on random reweightings at the roots of
    /// 20 boards of 300-2175 (every evaluation checked).
    #[test]
    fn the_intersection_matches_the_reference() {
        let mut checks = 0;
        for i in 0..20u32 {
            let n = 300 + i * (2175 - 300) / 19;
            let n = if is_prime(n) { n + 1 } else { n };
            checks += taxman::with_width!(n, W => reweighted::<W>(n));
        }
        eprintln!("test 8: {checks} checked intersections");
    }

    fn reweighted<const W: usize>(n: u32) -> u64 {
        let Some((_, p)) = setup::<W>(n) else { return 0 };
        let fm = frame_matroids(&p.model, &p.root, &p.candidates);
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, p.max_total as usize, 0, false);
        let fam = Family::new(&ctx, (1..4.min(p.root.len())).map(Signing::ones).collect());
        let mut stats = Stats::default();
        let mut rng = Rng(0xA076_1D64_78BD_642F ^ u64::from(n));
        for _ in 0..3 {
            let lam: Vec<Vec<f64>> = fam
                .extras
                .iter()
                .map(|ex| ex.m.elems.iter().map(|&x| (rng.below(1000) as f64 / 1000.0) * f64::from(x)).collect())
                .collect();
            evaluate(&ctx, &fam, &lam, &mut stats, 1);
        }
        stats.checks
    }

    /// The bound on small boards with the main arm: valid, converged, the optimal promotions independent in every
    /// final matroid (the asserts of `roots`).
    #[test]
    fn the_main_arm_on_small_boards() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let mut out = Output { file: None, done: HashSet::new() };
        let o = Options {
            command: "roots".into(),
            known: Vec::new(),
            arms: ["gf2", "f1", "f1r1", "cores", "f1cores"].map(String::from).to_vec(),
            modes: Vec::new(),
            settings: Settings { seconds: 20.0, check_every: 1, ..Settings::default() },
            fixed_seconds: 20.0,
            root_seconds: 20.0,
            frames: vec![1, 2, 3],
            out: None,
            census: None,
            exact: None,
            samples: 0,
            games: Vec::new(),
            files: Vec::new(),
        };
        for n in [300u32, 360, 420, 480] {
            taxman::with_width!(n, W => roots::<W>(n, &known, &o, &mut out, None));
        }
    }

    /// Runs `f` at the root of `n`, set up as `roots` sets it up (target `P*`, not stopping there), with the main
    /// arm's fit in search mode within `seconds` and the GF(2) bound.
    fn with_root_fit<const W: usize>(
        n: u32,
        seconds: f64,
        f: impl FnOnce(&taxman::search::Prepared<W>, &Ctx<'_, W>, &Outcome, i64),
    ) {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let (board, p) = setup::<W>(n).expect("a board with candidates");
        let opt = known.opt(n, board.in_the_bag).expect("a built-in optimum") as i64;
        let fm = frame_matroids(&p.model, &p.root, &p.candidates);
        let cap = p.max_total as usize;
        let gf2 = fm.bound(cap).value;
        let mut ctx = Ctx::new(n, &p.model, &p.root, &fm, cap, opt, false);
        ctx.gap = Some(gf2 - opt);
        let (signings, separate) = arm("f1cores", n, p.root.len(), &[1, 2, 3]);
        let settings = Settings { mode: FitMode::Search, seconds, ..Settings::default() };
        let r = bound(&ctx, signings, None, separate, &settings);
        assert!(r.bound >= opt, "n={n}: the bound {} below P* = {opt}", r.bound);
        assert_eq!(r.lb, 0.0, "n={n}: search mode certifies nothing");
        f(&p, &ctx, &r, gf2);
    }

    /// The design's S5.2 test 6: at the roots of 600 and 900, the main arm's fit in search mode, written as a fit
    /// file's line and read back, is the same fit, and Tier 1 at the root from the fit read (a family of its
    /// signings) gives exactly its `l_best`, which is at least the bound.
    #[test]
    fn fit_round_trip() {
        for n in [600u32, 900] {
            taxman::with_width!(n, W => with_root_fit::<W>(n, 10.0, |_, ctx, r, gf2| {
                let fit = Fit::from_outcome(n, ctx.target, gf2, r);
                let line = fit.to_json();
                assert!(!line.contains('\n'), "n={n}: a fit is one line");
                let read = fit_from_json(&line).unwrap_or_else(|e| panic!("n={n}: {e}"));
                assert_eq!(read, fit, "n={n}: the fit read differs from the fit written");
                let fam = Family::new(ctx, read.warm.signings.clone());
                let (value, ev) = tier1(ctx, &fam, &read.warm, &mut Stats::default());
                assert_eq!(ev.value, (read.l_best * dw::SCALE as f64).round() as i64, "n={n}: L at the fit");
                assert_eq!(value, read.l_best.floor() as i64, "n={n}");
                assert!(value >= read.bound, "n={n}: {value} below the bound {}", read.bound);
                eprintln!(
                    "n={n}: bound {}, l_best {}, {} signings, {} columns, {} cuts, {} bytes",
                    read.bound,
                    read.l_best,
                    read.warm.signings.len(),
                    read.warm.columns.len(),
                    read.warm.cuts.len(),
                    line.len()
                );
            }));
        }
    }

    /// Asserts what a refit's results must satisfy: its trace starts at Tier 1's bound `tier`, never falls below
    /// `P*` and never rises, has an entry per pricing after Tier 1's, and ends at the bound returned, which the
    /// returned state's multipliers give again exactly.
    fn assert_refit<const W: usize>(ctx: &Ctx<'_, W>, what: &str, tier: i64, r: &(i64, Warm, Trace)) {
        let (value, state, trace) = r;
        let after = &trace.after;
        assert_eq!(after.first(), Some(&tier), "{what}: the trace starts at Tier 1");
        assert!(after.iter().all(|&a| a >= ctx.target), "{what}: below P* = {}: {after:?}", ctx.target);
        assert!(after.windows(2).all(|w| w[1] <= w[0]), "{what}: the trace rises: {after:?}");
        assert_eq!(after.len(), trace.pricings + 1, "{what}: a trace entry per pricing");
        assert_eq!(after.last(), Some(value), "{what}: the bound returned is the trace's last");
        let fam = Family::new(ctx, state.signings.clone());
        let (again, _) = tier1(ctx, &fam, state, &mut Stats::default());
        assert_eq!(again, *value, "{what}: the state returned gives another bound");
    }

    /// R1 and R2 (local 0 and 2) with `k = 20` at the roots of the non-prime boards 300-700 every 40th, from the main
    /// arm's search-mode fit (3 s) with every multiplier halved: valid traces and states (`assert_refit`).
    #[test]
    fn refits_are_valid() {
        let (mut lowered, mut pricings, mut opened) = (0, 0, 0);
        for n in (300..=700u32).step_by(40).filter(|&n| !is_prime(n)) {
            taxman::with_width!(n, W => with_root_fit::<W>(n, 3.0, |_, ctx, r, _| {
                let mut warm = Warm::from_outcome(r);
                for m in &mut warm.lambda {
                    m.values_mut().for_each(|v| *v /= 2.0);
                }
                let mut stats = Stats::default();
                let fam = || Family::new(ctx, warm.signings.clone());
                let settings = Settings { seconds: 10.0, ..Settings::default() };
                let f = fam();
                let (tier, first) = tier1(ctx, &f, &warm, &mut stats);
                let r1 = refit_r1(ctx, &f, &warm, &first, 20, &mut stats);
                assert_refit(ctx, &format!("n={n} r1"), tier, &r1);
                assert_eq!((r1.2.new_signings, &r1.1.columns, &r1.1.cuts), (0, &warm.columns, &warm.cuts));
                let mut line = format!("n={n}: tier 1 {tier}, r1 {} ({} pricings)", r1.0, r1.2.pricings);
                for local in [0, 2] {
                    let f = fam();
                    let (_, first) = tier1(ctx, &f, &warm, &mut stats);
                    let r2 = refit_r2(ctx, f, &warm, first, 20, local, &settings, &mut stats);
                    let what = format!("n={n} r2 local {local}");
                    assert_refit(ctx, &what, tier, &r2);
                    let (state, trace) = (&r2.1, &r2.2);
                    assert!(trace.new_signings <= 2 * ctx.state.len() * local, "{what}");
                    assert_eq!(state.local, warm.local + trace.new_signings, "{what}");
                    assert_eq!(state.signings.len(), warm.signings.len() + trace.new_signings, "{what}");
                    assert!(state.signings.iter().all(|s| !s.mergeable && s.cores.is_empty()), "{what}: frozen");
                    line += &format!(
                        ", r2/{local} {} ({} pricings, {} signings, {:?})",
                        r2.0, trace.pricings, trace.new_signings, trace.stop
                    );
                    lowered += usize::from(r2.0 < tier);
                    pricings += trace.pricings;
                    opened += trace.new_signings;
                }
                lowered += usize::from(r1.0 < tier);
                pricings += r1.2.pricings;
                eprintln!("{line}");
            }));
        }
        eprintln!("refits: {lowered} lowered the bound, {pricings} pricings, {opened} local signings");
        assert!(lowered > 0 && opened > 0, "the refits had nothing to do");
    }

    /// At the roots of 300-700 every 50th, each child of the root as `examples/matroid.rs`'s `bnb` makes them (the
    /// GF(2) heaviest set's conflict `k₁ < … < k_r`: child `j` promotes `k₁ … k_{j−1}` and leaves out `k_j`) takes
    /// over the root fit's state (`Warm::child`): every column is common independent in the child's GF(2) matroids
    /// and within its `max_add`; every cut is within its elements and, with its rank taken in the child's signed
    /// matroid, holds for 20 random maximal promotable sets of the child.
    #[test]
    fn child_keeps_valid_columns() {
        let (mut children, mut columns, mut cuts) = (0, 0, 0);
        for n in (300..=700u32).step_by(50) {
            taxman::with_width!(n, W => with_root_fit::<W>(n, 3.0, |p, _, r, _| {
                let warm = Warm::from_outcome(r);
                let model = &p.model;
                let mut scratch = Scratch::<W>::new();
                let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
                let max_add = |state: &FrameSet<W>, promoted: u32, cands: &Bits<W>| {
                    (p.max_total as i32 - promoted as i32).min(state.max_promotions_among(cands)).max(0) as usize
                };
                let cands = p.root.narrow_candidates(&p.candidates);
                let fm = frame_matroids(model, &p.root, &cands);
                let chosen = fm.chosen::<W>(&fm.bound(max_add(&p.root, 0, &cands)));
                let Some(k) = model.conflict_using(&mut scratch, &p.root, &chosen, true) else { return };
                let (mut required, mut rest, mut at) = (Vec::new(), cands, p.root);
                for x in k.iter() {
                    rest.remove(x);
                    let live_cands = at.narrow_candidates(&rest);
                    let limit = max_add(&at, required.len() as u32, &live_cands);
                    let fmc = frame_matroids(model, &at, &live_cands);
                    let child = warm.child(&required, &[x], &fmc.elems, limit);
                    for c in &child.columns {
                        fmc.independent(&Bits::<W>::from_slice(c))
                            .unwrap_or_else(|e| panic!("n={n}, child {}: {c:?}: {e}", required.len()));
                        assert!(c.len() <= limit, "n={n}: a column of {} members, max_add {limit}", c.len());
                    }
                    let ctxc = Ctx::new(n, model, &at, &fmc, limit, 0, false);
                    let famc = Family::new(&ctxc, child.signings.clone());
                    let live: Vec<u32> = live_cands.iter().collect();
                    let completions: Vec<Vec<u32>> =
                        (0..20).map(|_| random_promotable(model, &at, &live, &mut rng).iter().collect()).collect();
                    for (j, set) in &child.cuts {
                        assert!(set.iter().all(|x| fmc.elems.binary_search(x).is_ok()), "n={n}: a cut's member");
                        let rank = cut_rank(&famc, *j, set);
                        for c in &completions {
                            let common = set.iter().filter(|x| c.binary_search(x).is_ok()).count();
                            assert!(common <= rank, "n={n}: {common} of a cut of rank {rank} in a promotable set");
                        }
                    }
                    children += 1;
                    columns += child.columns.len();
                    cuts += child.cuts.len();
                    match model.promote_all_using(&mut scratch, &at, &Bits::EMPTY.with(x)) {
                        Some(next) => {
                            at = next;
                            required.push(x);
                        }
                        None => break,
                    }
                }
            }));
        }
        eprintln!("child_keeps_valid_columns: {children} children, {columns} columns and {cuts} cuts checked");
        assert!(children > 0 && columns > 0 && cuts > 0);
    }
}
