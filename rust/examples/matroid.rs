//! Matroid bounds for the promotions: the heaviest common independent set of the two GF(2) frame matroids,
//! measured against the matching bound at the roots and at search nodes, and the search's proofs with it.  The
//! search's bound is the crate's `taxman::gf2` (tactic `gf2`), which `prove` runs; the other commands are the study
//! that led to it and use its research implementation, `common/meet.rs` (the matroids built afresh at each node, a
//! reference algorithm and a fast one), which stays as an independent check of the crate's (`nodes` asserts that the
//! crate's contraction gives the matroids built at the node).  The studies are in docs/EXPERIMENTS.md, "Matroid
//! intersection bounds (2026-10)" and "The GF(2) bound in the search (2026-10)".
//!
//! ```text
//! cargo run --release --example matroid -- roots [--known FILE] [--opts FILE] N|A-B...
//! cargo run --release --example matroid -- nodes [--every K] [--samples K] [--lagrange STEPS] [--exact SECONDS]
//!     FIXTURE...
//! cargo run --release --example matroid -- check [--known FILE] [--samples K] N|A-B...
//! cargo run --release --example matroid -- bnb [--known FILE] [--opts FILE] [--bounds gf2,gf2warm,matching]
//!     [--limit NODES] N|A-B...
//! cargo run --release --example matroid -- lagrange [--known FILE] [--limit STEPS] N|A-B...
//! cargo run --release --example matroid -- forced [--every K] FIXTURE...
//! cargo run --release --example matroid -- cores N|A-B...
//! cargo run --release --example matroid -- whole [--known FILE] N|A-B...
//! cargo run --release --example matroid -- prove [--known FILE] [--tactic NAME=VALUE]... [--branching B]
//!     [--prefilter] [--strong K] [--first-forced] [--threads T] [--seconds S] N|A-B...
//! ```
//!
//! Games are set up as `taxman solve` sets them up (the default tactics, which are strict).  `P*` is the
//! optimum promotion sum (the known score minus `in_the_bag`, or the `opt` of the lines of `--opts`, as
//! `bench/curve/slope_roots.jsonl` has them).  A range `A-B` stands for its games that are not prime.  Each command
//! prints one JSON object per line.  `bench/matroid/matroid.py` makes the study's tables from them.
//!
//! * **roots**: per game, the root's matching bound and the GF(2) bound, capped at `max_total` promotions and not,
//!   the size of the heaviest common independent set, and the time it took.
//! * **nodes**: per node of a `.nodes` fixture of `testdata/perf` (the proof of the fixture's game, seeded with
//!   the optimum; every `--every`-th), the node's matching bound and GF(2) bound (capped at the node's `max_add`)
//!   next to what a completion must beat, `P* − sum(promoted)`; `--samples` random completions checked
//!   independent; with `--lagrange`, the Lagrangian bound over nogoods of the node's own fitted at the node
//!   (`common/root_lagrange.rs`); with `--exact`, the node's best completion by the solver's search from it,
//!   within that many seconds, which the GF(2) bound must not be below.
//! * **check**: validity.  At each root, the GF(2) bound is at least `P*` (and the fast algorithm's equals the
//!   reference's); the optimal game's promotions and every maximal promotable set found by promoting the
//!   candidates in `--samples` random orders (default 200) are independent in both matroids; and on boards with at
//!   most 22 candidates, the bound is at least the best promotable set found by enumerating every one.
//! * **bnb**: a plain branch and bound with one bound, proving that nothing beats `P*` ([`bnb`]).
//! * **lagrange**: the Lagrangian bound over nogoods with the intersection as its inner problem, at the root.
//! * **forced**: forced members of the GF(2) bound at the nodes of a fixture, exactly and from a weight split.
//! * **cores**: the cores that the GF(2) bound's heaviest set runs into at the root, and whether Lorea's matroid
//!   sees them.
//! * **whole**: the same bound for the whole game, with no frame model: an upper bound on `opt(n)` ([`whole`]).
//! * **prove**: the search's proof that nothing beats `P*`, with the search tactics of `--tactic` (named as for
//!   `taxman solve`) and the GF(2) bound's knobs, which are not tactics: `--branching` (`exact`, the default;
//!   `split`, the member whose split bound is lowest; `largest`; `forced`, exact solves only to find forced
//!   members), `--prefilter` (the matching bound as a first check), `--strong K` (solve at most K members of a
//!   conflict) and `--first-forced` (stop solving at the first forced member) ([`prove`]).

#[path = "common/meet.rs"]
mod meet;
#[path = "common/root_lagrange.rs"]
mod root_lagrange;

use meet::{intersect, intersect_fast, intersect_warm, split, split_bound, BlockMatroid, Meet, Reduced, Span};
use root_lagrange::{root_lagrange, CONFLICTS_PER_STEP, STALL};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, EngineConfig, Start};
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::matchbound::lagrange::SCALE;
use taxman::matchbound::{MatchBound, MatchGraph};
use taxman::model::{FrameSet, Model, Scratch};
use taxman::search::{stat, Gf2Branching, Prepared, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};
use taxman::tactics::Tactics;

#[path = "common/counters.rs"]
mod counters;
#[path = "common/fixtures.rs"]
#[allow(dead_code)] // the hex writer is the probe's
mod fixtures;

// ---------------------------------------------------------------------------------------------------------
// The frame matroids of a state

/// The two frame matroids of a state and its candidates (see `common/meet.rs`): element `i` is the candidate
/// `elems[i]`; `a` is the direct sum of the even frames' matroids, `b` of the odd frames'.
struct FrameMatroids {
    elems: Vec<u32>,
    a: BlockMatroid,
    b: BlockMatroid,
    /// Candidates left out because they are loops of a frame matroid: no promotable set holds them.
    loops: Vec<u32>,
    /// Per frame, the rank of its matroid (its factors minus its moves).
    ranks: Vec<usize>,
}

/// One frame's coordinates and the span of its moves, reduced to echelon form.
struct FrameSpace {
    /// Coordinate of each number that is a factor of the frame (`u32::MAX` for the others).
    coord: Vec<u32>,
    words: usize,
    /// Rows of the moves' span, by pivot (lowest set bit).
    pivot_row: Vec<Option<Vec<u64>>>,
    /// The quotient coordinate of each non-pivot coordinate (`u32::MAX` for pivots).
    quotient: Vec<u32>,
    rank: usize,
}

impl FrameSpace {
    fn new<const W: usize>(model: &Model, factors: &Bits<W>, moves: &Bits<W>, max: u32) -> Result<Self, String> {
        let mut coord = vec![u32::MAX; max as usize + 1];
        for (k, y) in factors.iter().enumerate() {
            coord[y as usize] = k as u32;
        }
        let dim = factors.len() as usize;
        let words = dim.div_ceil(64).max(1);
        let mut s = FrameSpace { coord, words, pivot_row: vec![None; 64 * words], quotient: Vec::new(), rank: 0 };
        for m in moves.iter() {
            let v = s.vector(model, m);
            let r = s.reduce(v);
            let p = meet::lowest(&r).ok_or_else(|| format!("the moves of a frame are dependent at {m}"))?;
            s.pivot_row[p] = Some(r);
        }
        let mut q = 0u32;
        s.quotient = (0..64 * words)
            .map(|c| {
                if c < dim && s.pivot_row[c].is_none() {
                    q += 1;
                    q - 1
                } else {
                    u32::MAX
                }
            })
            .collect();
        s.rank = q as usize;
        Ok(s)
    }

    /// The vector of the mover `u` (its maximal factors among the frame's factors).
    fn vector(&self, model: &Model, u: u32) -> Vec<u64> {
        let mut v = vec![0u64; self.words];
        for &q in model.graph().factors(u) {
            if let Some(&c) = self.coord.get(usize::from(q)) {
                if c != u32::MAX {
                    v[c as usize / 64] |= 1 << (c % 64);
                }
            }
        }
        v
    }

    /// The unit vector of the factor `y`.
    fn unit(&self, y: u32) -> Vec<u64> {
        let mut v = vec![0u64; self.words];
        let c = self.coord[y as usize];
        assert_ne!(c, u32::MAX, "{y} is not a factor of the frame");
        v[c as usize / 64] |= 1 << (c % 64);
        v
    }

    /// `v` fully reduced by the moves' rows: no pivot coordinate left.
    fn reduce(&self, mut v: Vec<u64>) -> Vec<u64> {
        let mut from = 0;
        loop {
            let next = v[from / 64..].iter().enumerate().find_map(|(i, &w)| {
                let w = if i == 0 { w & (!0u64 << (from % 64)) } else { w };
                (w != 0).then(|| (from / 64 + i) * 64 + w.trailing_zeros() as usize)
            });
            let Some(b) = next else { break };
            if let Some(row) = &self.pivot_row[b] {
                for (a, x) in v.iter_mut().zip(row) {
                    *a ^= x;
                }
            }
            from = b + 1;
            if from >= 64 * self.words {
                break;
            }
        }
        v
    }

    /// `v` in the quotient by the moves' span: reduced, then the non-pivot coordinates.
    fn project(&self, v: Vec<u64>) -> Vec<u64> {
        let r = self.reduce(v);
        let mut out = vec![0u64; self.rank.div_ceil(64).max(1)];
        for c in meet_bits(&r) {
            let q = self.quotient[c];
            debug_assert_ne!(q, u32::MAX, "a pivot survived the reduction");
            out[q as usize / 64] |= 1 << (q % 64);
        }
        out
    }
}

fn meet_bits(v: &[u64]) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, &w) in v.iter().enumerate() {
        let mut w = w;
        while w != 0 {
            out.push(64 * i + w.trailing_zeros() as usize);
            w &= w - 1;
        }
    }
    out
}

fn frame_matroids<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>) -> FrameMatroids {
    let frames = state.frames();
    let max = frames.iter().flat_map(|f| f.factors.max().into_iter().chain(f.moves.max())).max().unwrap_or(0);
    let max = max.max(cands.max().unwrap_or(0));
    let spaces: Vec<FrameSpace> =
        frames.iter().map(|f| FrameSpace::new(model, &f.factors, &f.moves, max).expect("a winnable frame")).collect();
    let words: Vec<usize> = spaces.iter().map(|s| s.rank.div_ceil(64).max(1)).collect();
    let block_words =
        |parity: usize| -> Vec<usize> { (0..frames.len()).filter(|h| h % 2 == parity).map(|h| words[h]).collect() };
    let mut a = BlockMatroid { block: Vec::new(), vecs: Vec::new(), words: block_words(0) };
    let mut b = BlockMatroid { block: Vec::new(), vecs: Vec::new(), words: block_words(1) };
    let (mut elems, mut loops) = (Vec::new(), Vec::new());
    for x in cands.iter() {
        let h = model.home(x);
        if h == 0 || h >= frames.len() || !frames[h].factors.contains(x) {
            loops.push(x);
            continue;
        }
        let out = spaces[h].project(spaces[h].unit(x));
        let into = spaces[h - 1].project(spaces[h - 1].vector(model, x));
        if out.iter().all(|&w| w == 0) || into.iter().all(|&w| w == 0) {
            loops.push(x);
            continue;
        }
        let (even, odd) = if h % 2 == 0 { ((h, out), (h - 1, into)) } else { ((h - 1, into), (h, out)) };
        a.block.push(even.0 / 2);
        a.vecs.push(even.1);
        b.block.push(odd.0 / 2);
        b.vecs.push(odd.1);
        elems.push(x);
    }
    FrameMatroids { elems, a, b, loops, ranks: spaces.iter().map(|s| s.rank).collect() }
}

impl FrameMatroids {
    /// Whether the candidates `set` (all of them elements) are independent in both matroids.
    fn independent<const W: usize>(&self, set: &Bits<W>) -> Result<(), String> {
        for (name, m) in [("A", &self.a), ("B", &self.b)] {
            let mut span = Span::new(m);
            for (i, &x) in self.elems.iter().enumerate() {
                if set.contains(x) {
                    if let Reduced::Circuit(_) = span.reduce(m, i) {
                        return Err(format!("{x} closes a circuit in {name}"));
                    }
                    span.insert(m, i);
                }
            }
        }
        match set.iter().find(|x| !self.elems.contains(x)) {
            Some(x) => Err(format!("{x} is not an element (a loop or not a candidate)")),
            None => Ok(()),
        }
    }

    /// The heaviest common independent set of at most `cap` candidates.
    fn bound(&self, cap: usize) -> Meet {
        let w: Vec<i64> = self.elems.iter().map(|&x| i64::from(x)).collect();
        self.solve(&w, cap)
    }

    /// The heaviest common independent set of at most `cap` candidates, element `i` weighing `w[i]`.
    fn solve(&self, w: &[i64], cap: usize) -> Meet {
        if self.a.words.iter().chain(&self.b.words).all(|&w| w == 1) {
            intersect_fast(&self.a, &self.b, w, cap)
        } else {
            intersect(&self.a, &self.b, w, cap, false)
        }
    }

    /// [`bound`](Self::bound) started from the elements `start`, if they are an extreme common independent set
    /// (see `meet::intersect_warm`).
    fn warm(&self, start: &[bool], cap: usize) -> Option<Meet> {
        let w: Vec<i64> = self.elems.iter().map(|&x| i64::from(x)).collect();
        intersect_warm(&self.a, &self.b, &w, cap, start)
    }

    /// [`bound`](Self::bound) by the reference algorithm.
    fn bound_reference(&self, cap: usize) -> Meet {
        let w: Vec<i64> = self.elems.iter().map(|&x| i64::from(x)).collect();
        intersect(&self.a, &self.b, &w, cap, false)
    }

    fn chosen<const W: usize>(&self, m: &Meet) -> Bits<W> {
        self.elems.iter().zip(&m.chosen).filter(|(_, &c)| c).map(|(&x, _)| x).collect()
    }
}

// ---------------------------------------------------------------------------------------------------------
// Setup

/// The board of `n` set up for a search as `taxman solve` sets it up (the default tactics); `None` for a board with
/// no candidates.
fn setup<const W: usize>(n: u32) -> Option<(Board<W>, Prepared<W>)> {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return None;
    }
    let p = board.prepare(SearchTactics::default()).expect("prepare");
    Some((board, p))
}

fn is_prime(n: u32) -> bool {
    n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| !n.is_multiple_of(d))
}

struct Options {
    command: String,
    known: Vec<PathBuf>,
    opts: Option<PathBuf>,
    samples: usize,
    games: Vec<u32>,
    files: Vec<PathBuf>,
    bounds: Vec<BoundKind>,
    limit: u64,
    every: usize,
    exact: u64,
    lagrange: usize,
    tactics: SearchTactics,
    prefilter: bool,
    branching: Gf2Branching,
    strong: usize,
    more_forced: bool,
    threads: usize,
    seconds: Option<u64>,
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("expected a command: roots, nodes, check or bnb")?;
    let mut o = Options {
        command,
        known: Vec::new(),
        opts: None,
        samples: 200,
        games: Vec::new(),
        files: Vec::new(),
        bounds: vec![BoundKind::Gf2],
        limit: 1_000_000,
        every: 1,
        exact: 0,
        lagrange: 0,
        tactics: SearchTactics::default(),
        prefilter: false,
        branching: Gf2Branching::Exact,
        strong: usize::MAX,
        more_forced: true,
        threads: 1,
        seconds: None,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--known" => o.known.push(args.next().ok_or("--known needs a file")?.into()),
            "--opts" => o.opts = Some(args.next().ok_or("--opts needs a file")?.into()),
            "--limit" => o.limit = args.next().ok_or("--limit needs a value")?.parse().map_err(|e| format!("{e}"))?,
            "--every" => o.every = args.next().ok_or("--every needs a value")?.parse().map_err(|e| format!("{e}"))?,
            "--exact" => o.exact = args.next().ok_or("--exact needs seconds")?.parse().map_err(|e| format!("{e}"))?,
            "--lagrange" => {
                o.lagrange = args.next().ok_or("--lagrange needs a value")?.parse().map_err(|e| format!("{e}"))?
            }
            "--tactic" => {
                let spec = args.next().ok_or("--tactic needs NAME=VALUE")?;
                let mut t = Tactics { search: o.tactics, ..Tactics::default() };
                t.set(&spec)?;
                o.tactics = t.search;
            }
            "--prefilter" => o.prefilter = true,
            "--first-forced" => o.more_forced = false,
            "--strong" => {
                o.strong = args.next().ok_or("--strong needs a value")?.parse().map_err(|e| format!("{e}"))?
            }
            "--branching" => {
                o.branching = match args.next().ok_or("--branching needs a value")?.as_str() {
                    "largest" => Gf2Branching::Largest,
                    "split" => Gf2Branching::Split,
                    "exact" => Gf2Branching::Exact,
                    "forced" => Gf2Branching::Forced,
                    other => return Err(format!("unknown branching {other:?}")),
                }
            }
            "--threads" => {
                o.threads = args.next().ok_or("--threads needs a value")?.parse().map_err(|e| format!("{e}"))?
            }
            "--seconds" => {
                o.seconds = Some(args.next().ok_or("--seconds needs a value")?.parse().map_err(|e| format!("{e}"))?)
            }
            "--bounds" => {
                let list = args.next().ok_or("--bounds needs a list")?;
                o.bounds = list
                    .split(',')
                    .map(|b| match b {
                        "gf2" => Ok(BoundKind::Gf2),
                        "gf2warm" => Ok(BoundKind::Gf2Warm),
                        "matching" => Ok(BoundKind::Matching),
                        other => Err(format!("unknown bound {other:?}")),
                    })
                    .collect::<Result<_, _>>()?;
            }
            "--samples" => {
                o.samples = args.next().ok_or("--samples needs a value")?.parse().map_err(|e| format!("{e}"))?
            }
            file if ["nodes", "forced"].contains(&o.command.as_str()) => o.files.push(file.into()),
            range => {
                let parse_n = |t: &str| t.parse::<u32>().map_err(|e| format!("{range}: {e}"));
                match range.split_once('-') {
                    Some((a, b)) => o.games.extend((parse_n(a)?..=parse_n(b)?).filter(|&n| !is_prime(n))),
                    None => o.games.push(parse_n(range)?),
                }
            }
        }
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
    match o.command.as_str() {
        "roots" => {
            for &n in &o.games {
                taxman::with_width!(n, W => roots::<W>(n, &known));
            }
        }
        "check" => {
            for &n in &o.games {
                taxman::with_width!(n, W => check::<W>(n, &known, o.samples));
            }
        }
        "nodes" => {
            for path in &o.files {
                let n = first_n(path);
                taxman::with_width!(n, W => nodes::<W>(path, o.every, o.samples, o.lagrange, o.exact));
            }
        }
        "prove" => {
            for &n in &o.games {
                taxman::with_width!(n, W => prove::<W>(n, &known, &o));
            }
        }
        "bnb" => {
            for &n in &o.games {
                for &bound in &o.bounds {
                    taxman::with_width!(n, W => bnb::<W>(n, &known, bound, o.limit));
                }
            }
        }
        "lagrange" => {
            for &n in &o.games {
                taxman::with_width!(n, W => lagrange::<W>(n, &known, o.limit as usize));
            }
        }
        "whole" => {
            for &n in &o.games {
                whole(n, &known);
            }
        }
        "cores" => {
            for &n in &o.games {
                taxman::with_width!(n, W => cores::<W>(n));
            }
        }
        "forced" => {
            for path in &o.files {
                let n = first_n(path);
                taxman::with_width!(n, W => forced::<W>(path, o.every));
            }
        }
        other => {
            eprintln!("error: unknown command {other:?}");
            std::process::exit(2);
        }
    }
}

/// What is known of the optima: the known games, and the promotion sums `P*` of `--opts` (lines with `n` and
/// `opt`, as `bench/curve/slope_roots.jsonl` has them).
struct Known {
    optimal: Optimal,
    sums: HashMap<u32, u64>,
}

impl Known {
    fn load(o: &Options) -> Result<Known, String> {
        let optimal = Optimal::built_in().with_results_files(&o.known)?;
        let mut sums = HashMap::new();
        if let Some(path) = &o.opts {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let line = Json::parse(line).map_err(|e| format!("{}: {e}", path.display()))?;
                let field = |name| line.get(name).and_then(Json::as_u64);
                if let (Some(n), Some(opt)) = (field("n"), field("opt")) {
                    sums.insert(n as u32, opt);
                }
            }
        }
        Ok(Known { optimal, sums })
    }

    /// `P*` of the board `n` whose game with no promotions scores `in_the_bag`, if known.
    fn opt(&self, n: u32, in_the_bag: u64) -> Option<u64> {
        self.sums.get(&n).copied().or_else(|| self.optimal.score(n).map(|s| s - in_the_bag))
    }
}

// ---------------------------------------------------------------------------------------------------------
// roots

fn roots<const W: usize>(n: u32, known: &Known) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known game, skipped");
        return;
    };
    let graph = MatchGraph::new(&p.model, &p.root);
    let mut mb = MatchBound::new(&graph);
    mb.reset(&Bits::EMPTY, &p.candidates);
    let matching = mb.value();
    let t0 = Instant::now();
    let fm = frame_matroids(&p.model, &p.root, &p.candidates);
    let build = t0.elapsed();
    let t1 = Instant::now();
    let capped = fm.bound(p.max_total as usize);
    let solve = t1.elapsed();
    let free = fm.bound(usize::MAX);
    let chosen: Bits<W> = fm.chosen(&capped);
    let mut scratch = Scratch::new();
    let promotable = p.model.conflict_using(&mut scratch, &p.root, &chosen, true).is_none();
    let list = |v: &[usize]| v.iter().map(usize::to_string).collect::<Vec<_>>().join(",");
    println!(
        "{{\"n\":{n},\"opt\":{opt},\"candidates\":{},\"elements\":{},\"loops\":{},\"max_total\":{},\"ranks\":[{}],\
         \"matching\":{matching},\"gf2\":{},\"gf2_uncapped\":{},\"size\":{},\"size_uncapped\":{},\
         \"promotable\":{promotable},\"augmentations\":{},\"rounds\":{},\"build_s\":{:.5},\"solve_s\":{:.5}}}",
        p.candidates.len(),
        fm.elems.len(),
        fm.loops.len(),
        p.max_total,
        list(&fm.ranks),
        capped.value,
        free.value,
        chosen.len(),
        free.chosen.iter().filter(|&&c| c).count(),
        capped.augmentations,
        capped.rounds,
        build.as_secs_f64(),
        solve.as_secs_f64(),
    );
}

// ---------------------------------------------------------------------------------------------------------
// nodes

fn first_n(path: &Path) -> u32 {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let line = text.lines().find(|l| l.starts_with("n ")).expect("an n line");
    line[2..].trim().parse().expect("a board size")
}

/// The nodes of the fixture `path` (every `every`-th): the bounds, and the validity checks of `check` at the node
/// (`samples` completions in random orders, each independent in the node's matroids), and that the node's bound is
/// that of the root's matroids contracted by the node's promotions.  With `lagrange` steps, also
/// the Lagrangian bound over the matching with nogoods of the node's own (`root_lagrange` with the node as the
/// root): what the matching bound's Lagrangian makes of the node without a search.
fn nodes<const W: usize>(path: &Path, every: usize, samples: usize, lagrange: usize, exact: u64) {
    let (n, incumbent, units) = fixtures::read_nodes::<W>(path).expect("a fixture");
    let (_, p) = setup::<W>(n).expect("a game with candidates");
    let model = &p.model;
    let graph = MatchGraph::new(model, &p.root);
    let mut mb = MatchBound::new(&graph);
    let mut scratch = Scratch::<W>::new();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
    let best = incumbent.sum();
    let root_fm = frame_matroids(model, &p.root, &p.candidates);
    for u in units.iter().filter(|u| !u.cands.is_empty()).step_by(every.max(1)) {
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).expect("a node of the game");
        mb.reset(&u.promoted, &u.cands);
        let matching = mb.value();
        let max_add = (p.max_total as i32 - u.promoted.len() as i32).min(state.max_promotions_among(&u.cands));
        let t0 = Instant::now();
        let fm = frame_matroids(model, &state, &u.cands);
        let capped = fm.bound(max_add.max(0) as usize);
        let time = t0.elapsed();
        let target = best as i64 - u.promoted.sum() as i64;
        // The node's matroids are the root's contracted by its promotions: weighing the promotions above everything
        // else in the root's matroids, the heaviest common independent set holds them all and is otherwise the
        // node's (the node's candidates that its frames no longer hold are loops of the contraction).
        let heavy = 1i64 << 40;
        let w: Vec<i64> = root_fm
            .elems
            .iter()
            .map(|&x| match () {
                () if u.promoted.contains(x) => heavy,
                () if u.cands.contains(x) => i64::from(x),
                () => 0,
            })
            .collect();
        let size = u.promoted.len() as usize + max_add.max(0) as usize;
        let contracted = root_fm.solve(&w, size).value - heavy * i64::from(u.promoted.len());
        assert_eq!(contracted, capped.value, "n={n}: the node's matroids are not the root's contracted");
        // Validity at the node: maximal completions in random orders are independent.
        let cands: Vec<u32> = u.cands.iter().collect();
        for _ in 0..samples {
            let mut order = cands.clone();
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i as u64 + 1) as usize);
            }
            let mut at = state;
            let mut set = Bits::<W>::EMPTY;
            for &x in &order {
                if model.promote_in_place(&mut at, x) {
                    set.insert(x);
                }
            }
            fm.independent(&set).unwrap_or_else(|e| panic!("n={n}: a completion {set:?} of a node: {e}"));
        }
        let lagrange = (lagrange > 0).then(|| {
            let node_graph = MatchGraph::new(model, &state);
            let need = (target + 1).max(0) as u64;
            let l = root_lagrange(&node_graph, model, &mut scratch, &state, &u.cands, Vec::new(), need, lagrange);
            l.scaled.div_euclid(SCALE)
        });
        // The best completion of the node, by the solver's search from it (incumbent: no completion at all).
        let exact = (exact > 0).then(|| {
            let space = SearchSpace::new(&p, &graph, None);
            let cfg = EngineConfig {
                threads: 1,
                time_limit: Some(std::time::Duration::from_secs(exact)),
                ..EngineConfig::default()
            };
            let payload = vec![0u8; 8 * W];
            let start = Start::Fresh {
                units: vec![TaxUnit { promoted: u.promoted, cands: u.cands }],
                best: u.promoted.sum(),
                payload,
            };
            let out = engine::run(&cfg, start, b"matroid", |_| TaxSearcher::new(&space)).expect("search");
            let completion = out.best - u.promoted.sum();
            if out.completed {
                assert!(
                    capped.value as u64 >= completion,
                    "n={n}: the GF(2) bound {} is below the node's best completion {completion}",
                    capped.value
                );
            }
            (completion, out.completed)
        });
        println!(
            "{{\"n\":{n},\"depth\":{},\"promoted_sum\":{},\"candidates\":{},\"elements\":{},\"max_add\":{max_add},\
             \"target\":{target},\"matching\":{matching},\"gf2\":{},\"lagrange\":{},\"exact\":{},\"size\":{},\
             \"augmentations\":{},\"rounds\":{},\"seconds\":{:.5}}}",
            u.promoted.len(),
            u.promoted.sum(),
            u.cands.len(),
            fm.elems.len(),
            capped.value,
            lagrange.map_or("null".into(), |l| l.to_string()),
            match exact {
                Some((c, true)) => c.to_string(),
                _ => "null".into(),
            },
            capped.chosen.iter().filter(|&&c| c).count(),
            capped.augmentations,
            capped.rounds,
            time.as_secs_f64(),
        );
    }
}

// ---------------------------------------------------------------------------------------------------------
// check

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

fn check<const W: usize>(n: u32, known: &Known, samples: usize) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let model = &p.model;
    let fm = frame_matroids(model, &p.root, &p.candidates);
    let bound = fm.bound(p.max_total as usize).value;
    assert_eq!(bound, fm.bound_reference(p.max_total as usize).value, "n={n}: the two algorithms differ");
    let mut scratch = Scratch::<W>::new();
    if let Some(opt) = known.opt(n, board.in_the_bag) {
        assert!(bound >= opt as i64, "n={n}: the GF(2) bound {bound} is below the optimum {opt}");
    }
    // The optimal game's promotions.
    if let Some(moves) = known.optimal.moves(n) {
        let seed = board.seed(moves);
        if Some(seed.sum()) == known.opt(n, board.in_the_bag) {
            fm.independent(&seed).unwrap_or_else(|e| panic!("n={n}: the optimal promotions: {e}"));
        }
    }
    // Maximal promotable sets in random orders.
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ u64::from(n));
    let cands: Vec<u32> = p.candidates.iter().collect();
    let mut best_sampled = 0;
    for _ in 0..samples {
        let mut order = cands.clone();
        for i in (1..order.len()).rev() {
            order.swap(i, rng.below(i as u64 + 1) as usize);
        }
        let mut state = p.root;
        let mut set = Bits::<W>::EMPTY;
        for &x in &order {
            if model.promote_in_place(&mut state, x) {
                set.insert(x);
            }
        }
        if let Err(e) = fm.independent(&set) {
            panic!("n={n}: the promotable set {set:?} is not independent: {e}");
        }
        best_sampled = best_sampled.max(set.sum());
    }
    // Every promotable set, on small boards.
    let mut exhaustive = None;
    if cands.len() <= 22 {
        let mut best = 0u64;
        let mut stack = vec![(p.root, Bits::<W>::EMPTY, 0usize)];
        while let Some((state, set, from)) = stack.pop() {
            best = best.max(set.sum());
            fm.independent(&set).unwrap_or_else(|e| panic!("n={n}: {set:?}: {e}"));
            for (i, &x) in cands.iter().enumerate().skip(from) {
                if let Some(next) = model.promote_all_using(&mut scratch, &state, &Bits::EMPTY.with(x)) {
                    stack.push((next, set.with(x), i + 1));
                }
            }
        }
        assert!(bound as u64 >= best, "n={n}: the GF(2) bound {bound} is below the best promotable set {best}");
        exhaustive = Some(best);
    }
    println!(
        "{{\"n\":{n},\"candidates\":{},\"elements\":{},\"bound\":{bound},\"best_sampled\":{best_sampled},\
         \"exhaustive\":{}}}",
        cands.len(),
        fm.elems.len(),
        exhaustive.map_or("null".into(), |b| b.to_string())
    );
}

// ---------------------------------------------------------------------------------------------------------
// bnb

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoundKind {
    Gf2,
    /// The GF(2) bound, started from the parent's heaviest set where that is extreme.
    Gf2Warm,
    Matching,
}

/// A plain branch and bound that proves that no promotion set beats `P*`, with one bound at every node: the
/// node is cut when the `max_add` largest candidates, or the bound, fall below what it needs; otherwise the
/// bound's heaviest set is promotable (a completion better than `P*`: counted, and impossible if the bound and
/// `P*` are right) or has a conflict `k₁ < … < k_r`, and the node branches into "`k₁ … k_{j−1}` promoted, `k_j`
/// not" for each `j` (every completion leaves out a member of the conflict).  No refinement, forced members,
/// nogoods or strong branching: the same skeleton for either bound, so that their trees compare.
fn bnb<const W: usize>(n: u32, known: &Known, kind: BoundKind, limit: u64) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known optimum, skipped");
        return;
    };
    let model = &p.model;
    let graph = MatchGraph::new(model, &p.root);
    let mut mb = MatchBound::new(&graph);
    let mut scratch = Scratch::<W>::new();
    // Each node with the heaviest set of its parent's bound (a warm start for Gf2Warm).
    let mut stack: Vec<(Bits<W>, FrameSet<W>, Bits<W>, Bits<W>)> =
        vec![(Bits::EMPTY, p.root, p.candidates, Bits::EMPTY)];
    let (mut nodes, mut cut_count, mut cut_bound, mut better, mut branches) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut warm, mut cold, mut augmentations) = (0u64, 0u64, 0u64);
    let t0 = Instant::now();
    let mut bound_time = std::time::Duration::ZERO;
    let mut build_time = std::time::Duration::ZERO;
    while let Some((promoted, state, cands, parent)) = stack.pop() {
        if nodes >= limit {
            stack.push((promoted, state, cands, parent));
            break;
        }
        nodes += 1;
        let cands = state.narrow_candidates(&cands);
        let need = (opt + 1).saturating_sub(promoted.sum());
        let max_add = (p.max_total as i32 - promoted.len() as i32).min(state.max_promotions_among(&cands)).max(0);
        if cands.sum_largest(max_add as u32) < need {
            cut_count += 1;
            continue;
        }
        let tb = Instant::now();
        let (value, chosen) = match kind {
            BoundKind::Gf2 | BoundKind::Gf2Warm => {
                let fm = frame_matroids(model, &state, &cands);
                build_time += tb.elapsed();
                let start: Vec<bool> = fm.elems.iter().map(|&x| parent.contains(x)).collect();
                let size = start.iter().filter(|&&s| s).count();
                let warmed = (kind == BoundKind::Gf2Warm && size > 0 && size <= max_add as usize)
                    .then(|| fm.warm(&start, max_add as usize))
                    .flatten();
                let m = match warmed {
                    Some(m) => {
                        warm += 1;
                        m
                    }
                    None => {
                        cold += 1;
                        fm.bound(max_add as usize)
                    }
                };
                augmentations += m.augmentations as u64;
                (m.value as u64, fm.chosen::<W>(&m))
            }
            BoundKind::Matching => {
                mb.reset(&promoted, &cands);
                (mb.value(), mb.chosen())
            }
        };
        bound_time += tb.elapsed();
        if value < need {
            cut_bound += 1;
            continue;
        }
        let Some(k) = model.conflict_using(&mut scratch, &state, &chosen, true) else {
            better += 1;
            continue;
        };
        let (mut required, mut rest, mut at) = (Bits::<W>::EMPTY, cands, state);
        for x in k.iter() {
            rest.remove(x);
            stack.push((promoted | required, at, rest, chosen - required));
            branches += 1;
            match model.promote_all_using(&mut scratch, &at, &Bits::EMPTY.with(x)) {
                Some(next) => {
                    at = next;
                    required.insert(x);
                }
                None => break,
            }
        }
    }
    println!(
        "{{\"n\":{n},\"bound\":\"{kind:?}\",\"opt\":{opt},\"completed\":{},\"nodes\":{nodes},\"cut_by_count\":{cut_count},\
         \"cut_by_bound\":{cut_bound},\"branches\":{branches},\"better\":{better},\"warm\":{warm},\"cold\":{cold},\
         \"augmentations\":{augmentations},\"seconds\":{:.3},\"bound_seconds\":{:.3},\"build_seconds\":{:.3}}}",
        stack.is_empty(),
        t0.elapsed().as_secs_f64(),
        bound_time.as_secs_f64(),
        build_time.as_secs_f64(),
    );
}

// ---------------------------------------------------------------------------------------------------------
// lagrange

/// A Lagrangian bound over nogoods with the GF(2) intersection as the inner problem, at the root: for multipliers
/// `λ_N ≥ 0` on nogoods `N` (sets that cannot all be promoted, so every completion `F` has `|F ∩ N| ≤ |N| − 1`),
/// the heaviest common independent set with each candidate weighing its number less the multipliers of the nogoods
/// it is in, plus `Σ λ_N (|N| − 1)`, bounds every completion (weak duality, as the matching bound's Lagrangian in
/// `matchbound::lagrange`).  The nogoods are generated as the multipliers are fitted, from the conflicts of each
/// step's optimum (as `examples/slope.rs`'s `root_lagrange` does for the matching bound), and the multipliers by
/// at most `steps` damped Polyak steps towards `P* + 1`.  Also: the conflicts of the GF(2) optimum itself.
fn lagrange<const W: usize>(n: u32, known: &Known, steps: usize) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known optimum, skipped");
        return;
    };
    let model = &p.model;
    let t0 = Instant::now();
    let fm = frame_matroids(model, &p.root, &p.candidates);
    let cap = p.max_total as usize;
    let plain = fm.bound(cap);
    let target = SCALE * (opt as i64 + 1);
    let index: HashMap<u32, usize> = fm.elems.iter().enumerate().map(|(i, &x)| (x, i)).collect();
    let mut scratch = Scratch::<W>::new();

    // The conflicts of the GF(2) optimum: the odd cores it runs into.
    let q0: Bits<W> = fm.chosen(&plain);
    let mut first = Vec::new();
    let mut rest = q0;
    while let Some(k) = model.conflict_using(&mut scratch, &p.root, &rest, true) {
        first.push(k.len());
        rest.remove(k.max().expect("a conflict has members"));
    }

    let mut nogoods: Vec<Bits<W>> = Vec::new();
    let mut members: Vec<Vec<usize>> = Vec::new();
    let mut lambda: Vec<i64> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let (mut lowest, mut best) = (i64::MAX, Vec::new());
    let (mut damping, mut stalled) = (1.0f64, 0usize);
    let (mut done, mut closed_at) = (0, None);
    for step in 0..steps {
        done = step + 1;
        let mut w: Vec<i64> = fm.elems.iter().map(|&x| SCALE * i64::from(x)).collect();
        let mut constant = 0i64;
        for ((k, mem), &l) in nogoods.iter().zip(&members).zip(&lambda) {
            if l > 0 {
                mem.iter().for_each(|&i| w[i] -= l);
                constant += l * (i64::from(k.len()) - 1);
            }
        }
        let m = fm.solve(&w, cap);
        let value = m.value + constant;
        if value < lowest {
            (lowest, stalled) = (value, 0);
            best.clone_from(&lambda);
        } else {
            stalled += 1;
            if stalled == STALL {
                (damping, stalled) = (damping / 2.0, 0);
                if damping < 1e-3 {
                    break;
                }
                lambda.iter_mut().zip(best.iter().chain(std::iter::repeat(&0))).for_each(|(l, b)| *l = *b);
                continue;
            }
        }
        if lowest < target {
            closed_at = Some(step + 1);
            break;
        }
        let q: Bits<W> = fm.chosen(&m);
        let mut rest = q;
        for _ in 0..CONFLICTS_PER_STEP {
            let Some(k) = model.conflict_using(&mut scratch, &p.root, &rest, true) else { break };
            if seen.insert(k) {
                nogoods.push(k);
                members.push(k.iter().filter_map(|x| index.get(&x).copied()).collect());
                lambda.push(0);
            }
            rest.remove(k.max().expect("a conflict has members"));
        }
        let g: Vec<i64> = nogoods.iter().map(|k| i64::from((q & *k).len()) - (i64::from(k.len()) - 1)).collect();
        let norm: i64 = g.iter().map(|x| x * x).sum();
        if norm == 0 {
            break;
        }
        let size = damping * (value - target + SCALE) as f64 / norm as f64;
        for (l, g) in lambda.iter_mut().zip(&g) {
            *l = (*l + (size * *g as f64).round() as i64).max(0);
        }
    }
    let positive = best.iter().filter(|&&l| l > 0).count();
    let list = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    println!(
        "{{\"n\":{n},\"opt\":{opt},\"gf2\":{},\"lagrange_gf2\":{},\"scaled\":{lowest},\"nogoods\":{},\"positive\":{positive},\
         \"steps\":{done},\"closed_at\":{},\"first_conflicts\":[{}],\"seconds\":{:.3}}}",
        plain.value,
        lowest.div_euclid(SCALE),
        nogoods.len(),
        closed_at.map_or("null".into(), |s: usize| s.to_string()),
        list(&first),
        t0.elapsed().as_secs_f64(),
    );
}

// ---------------------------------------------------------------------------------------------------------
// whole

/// The same bound for the whole game, with no frame model: an upper bound on `opt(n)` itself.
///
/// Give each number `s` of the board the vector `v(s)` with a 1 at each of its maximal factors (coordinates
/// `1..=n/2`).  If `S` is playable, the vectors `{v(s) : s ∈ S} ∪ {e_s : s ∈ S, s ≤ n/2}` are independent: in a
/// game's order each move pays with a maximal factor still in the pot (the lifting argument of Theorem 1), which
/// is no maximal factor of an earlier move (that move would have taken it) and not in `S` (it is taken now), so
/// the vectors restricted to the coordinates outside `S` are independent (the last-played member of a vanishing
/// sum has a coordinate no other member has), which is the same as adding the unit vectors of `S`'s coordinates.
/// A maximal factor of `s` has one prime factor fewer, so `v(s)` lies in the coordinates of the other parity of
/// Ω than `s`, and `e_s` in those of `s`'s own: every number has one vector in the even coordinates and one in the
/// odd (a number above `n/2` has none in its own parity, so it is free there).  The two halves make two linear
/// matroids on `2..=n` in which every playable set is independent, and the heaviest common independent set bounds
/// `opt(n)`.  The Franklín–Moniot matching bound is the same construction with Hall's condition for linear
/// independence.  Checked: the bound is at least `opt(n)`, and the optimal game's moves are independent in both.
fn whole(n: u32, known: &Known) {
    let t0 = Instant::now();
    let tables = taxman::numtheory::Tables::new(n);
    let half = n as usize / 2;
    let mut odd = vec![false; n as usize + 1]; // Ω(k) odd
    for k in 2..=n as usize {
        odd[k] = !odd[k / tables.spf[k] as usize];
    }
    let mut coord = vec![usize::MAX; half + 1];
    let mut dims = [0usize; 2];
    for (f, c) in coord.iter_mut().enumerate().skip(1) {
        let q = usize::from(odd[f]);
        *c = dims[q];
        dims[q] += 1;
    }
    let words = [dims[0].div_ceil(64).max(1), dims[1].div_ceil(64).max(1)];
    let mut ms = [0, 1].map(|q| BlockMatroid { block: Vec::new(), vecs: Vec::new(), words: vec![words[q]] });
    let elems: Vec<u32> = (2..=n).collect();
    for &s in &elems {
        let own = usize::from(odd[s as usize]);
        for (q, m) in ms.iter_mut().enumerate() {
            let mut v = vec![0u64; words[q]];
            if q != own {
                for &f in &tables.max_factors[s as usize] {
                    let c = coord[f as usize];
                    v[c / 64] |= 1 << (c % 64);
                }
                m.block.push(0);
                m.vecs.push(v);
            } else if (s as usize) <= half {
                let c = coord[s as usize];
                v[c / 64] |= 1 << (c % 64);
                m.block.push(0);
                m.vecs.push(v);
            } else {
                m.words.push(1);
                m.block.push(m.words.len() - 1);
                m.vecs.push(vec![1]);
            }
        }
    }
    let w: Vec<i64> = elems.iter().map(|&s| i64::from(s)).collect();
    let meet = intersect(&ms[0], &ms[1], &w, usize::MAX, false);
    let opt = known.optimal.score(n);
    if let Some(opt) = opt {
        assert!(meet.value >= opt as i64, "n={n}: the whole-game bound {} is below opt {opt}", meet.value);
    }
    let independent = known.optimal.moves(n).map(|moves| {
        let set: Vec<usize> = moves.iter().map(|&s| s as usize - 2).collect();
        ms.iter().all(|m| {
            let mut span = Span::new(m);
            set.iter().all(|&e| {
                let free = matches!(span.reduce(m, e), Reduced::Free);
                if free {
                    span.insert(m, e);
                }
                free
            })
        })
    });
    assert_ne!(independent, Some(false), "n={n}: the optimal game's moves are dependent");
    println!(
        "{{\"n\":{n},\"opt\":{},\"bound\":{},\"gap\":{},\"size\":{},\"moves\":{},\"seconds\":{:.3}}}",
        opt.map_or("null".into(), |o| o.to_string()),
        meet.value,
        opt.map_or("null".into(), |o| (meet.value - o as i64).to_string()),
        meet.chosen.iter().filter(|&&c| c).count(),
        known.optimal.moves(n).map_or(0, <[u32]>::len),
        t0.elapsed().as_secs_f64(),
    );
}

// ---------------------------------------------------------------------------------------------------------
// cores

/// What the GF(2) bound misses at the root: its heaviest set `Q` applied to the root's frames, the core of each
/// frame that `Q` leaves unwinnable (the movers a peel cannot drop: each live factor of one is listed by
/// another), how many of its movers have three or more live factors, and whether Lorea's hypergraphic matroid
/// sees the core (some subset `Y` of it lists at most `|Y|` live factors, a ground factor added to a mover that
/// lists one).  A core is independent over GF(2) here, since `Q` is.
fn cores<const W: usize>(n: u32) {
    let Some((_, p)) = setup::<W>(n) else { return };
    let model = &p.model;
    let g = model.graph();
    let fm = frame_matroids(model, &p.root, &p.candidates);
    let q: Bits<W> = fm.chosen(&fm.bound(p.max_total as usize));
    let mut found = Vec::new();
    for (h, f) in p.root.frames().iter().enumerate() {
        let out: Bits<W> = q.iter().filter(|&x| model.home(x) == h).collect();
        let live = f.factors - out;
        let movers: Vec<u32> = f.moves.iter().chain(q.iter().filter(|&x| model.home(x) == h + 1)).collect();
        let lists: Vec<Vec<u32>> = movers
            .iter()
            .map(|&u| g.factors(u).iter().map(|&y| u32::from(y)).filter(|&y| live.contains(y)).collect())
            .collect();
        let mut alive = vec![true; movers.len()];
        loop {
            let mut count: HashMap<u32, usize> = HashMap::new();
            for (i, list) in lists.iter().enumerate().filter(|(i, _)| alive[*i]) {
                let _ = i;
                for &y in list {
                    *count.entry(y).or_default() += 1;
                }
            }
            let drop = (0..movers.len()).find(|&i| alive[i] && lists[i].iter().any(|y| count[y] == 1));
            match drop {
                Some(i) => alive[i] = false,
                None => break,
            }
        }
        let core: Vec<usize> = (0..movers.len()).filter(|&i| alive[i]).collect();
        if core.is_empty() {
            continue;
        }
        let wide = core.iter().filter(|&&i| lists[i].len() >= 3).count();
        let promoted = core.iter().filter(|&&i| q.contains(movers[i])).count();
        let core_lists: Vec<Vec<u32>> = core.iter().map(|&i| lists[i].clone()).collect();
        let factors: std::collections::HashSet<u32> = core_lists.iter().flatten().copied().collect();
        found.push(format!(
            "{{\"frame\":{h},\"movers\":{},\"promoted\":{promoted},\"factors\":{},\"wide\":{wide},\"lorea\":{}}}",
            core.len(),
            factors.len(),
            !lorea_independent(&core_lists)
        ));
    }
    let ranks: Vec<String> = fm.ranks.iter().map(usize::to_string).collect();
    println!("{{\"n\":{n},\"ranks\":[{}],\"cores\":[{}]}}", ranks.join(","), found.join(","));
}

/// Whether the lists (one per mover) are independent in Lorea's hypergraphic matroid, a ground factor added to
/// each list of one: every nonempty subset `Y` lists at least `|Y| + 1` factors, which holds exactly when every
/// mover, taken twice, still leaves a matching of the movers into their factors (Hall's condition with the + 1).
fn lorea_independent(lists: &[Vec<u32>]) -> bool {
    const GROUND: u32 = u32::MAX;
    let lists: Vec<Vec<u32>> =
        lists.iter().map(|l| if l.len() == 1 { vec![l[0], GROUND] } else { l.clone() }).collect();
    (0..lists.len()).all(|twice| {
        let movers: Vec<&Vec<u32>> = lists.iter().chain(std::iter::once(&lists[twice])).collect();
        let mut mate: HashMap<u32, usize> = HashMap::new();
        fn augment(
            i: usize,
            movers: &[&Vec<u32>],
            mate: &mut HashMap<u32, usize>,
            seen: &mut std::collections::HashSet<u32>,
        ) -> bool {
            for &y in movers[i] {
                if seen.insert(y) {
                    let free = match mate.get(&y) {
                        None => true,
                        Some(&j) => augment(j, movers, mate, seen),
                    };
                    if free {
                        mate.insert(y, i);
                        return true;
                    }
                }
            }
            false
        }
        (0..movers.len()).all(|i| augment(i, &movers, &mut mate, &mut std::collections::HashSet::new()))
    })
}

// ---------------------------------------------------------------------------------------------------------
// forced

/// Forced members at the nodes of a fixture (every `every`-th): the members `x` of the GF(2) bound's heaviest
/// set without which the bound falls below what the node needs, found exactly (the intersection without `x`)
/// and from a weight split of the heaviest set (`meet::split`: two greedy runs per member), with the split's
/// looseness and cost.  The heaviest set is of any size here (the split is for that); the node's cap `max_add`
/// is reported when it binds.
fn forced<const W: usize>(path: &Path, every: usize) {
    let (n, incumbent, units) = fixtures::read_nodes::<W>(path).expect("a fixture");
    let (_, p) = setup::<W>(n).expect("a game with candidates");
    let model = &p.model;
    let mut scratch = Scratch::<W>::new();
    let best = incumbent.sum();
    for u in units.iter().filter(|u| !u.cands.is_empty()).step_by(every.max(1)) {
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).expect("a node of the game");
        let max_add = (p.max_total as i32 - u.promoted.len() as i32).min(state.max_promotions_among(&u.cands));
        let fm = frame_matroids(model, &state, &u.cands);
        let w: Vec<i64> = fm.elems.iter().map(|&x| i64::from(x)).collect();
        let m = fm.solve(&w, usize::MAX);
        let size = m.chosen.iter().filter(|&&c| c).count();
        let need = (best + 1) as i64 - u.promoted.sum() as i64;
        let t0 = Instant::now();
        let w1 = split(&fm.a, &fm.b, &w, &m.chosen).expect("a split of the heaviest set");
        let mut out = vec![false; w.len()];
        let mut by_split = Vec::new();
        for i in (0..w.len()).filter(|&i| m.chosen[i]) {
            out[i] = true;
            by_split.push((i, split_bound(&fm.a, &fm.b, &w, &w1, &out)));
            out[i] = false;
        }
        let split_time = t0.elapsed();
        let t1 = Instant::now();
        let mut exact = Vec::new();
        for &(i, _) in &by_split {
            let mut wi = w.clone();
            wi[i] = 0;
            exact.push(fm.solve(&wi, usize::MAX).value);
        }
        let exact_time = t1.elapsed();
        let forced_exact = exact.iter().filter(|&&v| v < need).count();
        let forced_split = by_split.iter().filter(|&&(_, v)| v < need).count();
        let loose: i64 = by_split.iter().zip(&exact).map(|(&(_, s), &e)| s - e).sum();
        assert!(by_split.iter().zip(&exact).all(|(&(_, s), &e)| s >= e), "n={n}: the split's bound is below");
        println!(
            "{{\"n\":{n},\"depth\":{},\"value\":{},\"need\":{need},\"size\":{size},\"max_add\":{max_add},\
             \"forced_exact\":{forced_exact},\"forced_split\":{forced_split},\"mean_looseness\":{:.1},\
             \"split_s\":{:.6},\"exact_s\":{:.6}}}",
            u.promoted.len(),
            m.value,
            loose as f64 / by_split.len().max(1) as f64,
            split_time.as_secs_f64(),
            exact_time.as_secs_f64(),
        );
    }
}

// ---------------------------------------------------------------------------------------------------------
// prove

/// The search's proof that nothing beats `P*` (its incumbent a value, `P*`, with no set behind it; no seed, no cap),
/// with the search tactics of `--tactic` and the GF(2) knobs `--prefilter` and `--branching`: nodes (steps),
/// seconds, the cycles and instructions of the process, and the GF(2) bound's counters.  On one thread (the
/// default) the node counts are exact and repeatable, and compare with `examples/slope.rs`'s `prove` at δ = 0.
fn prove<const W: usize>(n: u32, known: &Known, o: &Options) {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return;
    }
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known optimum, skipped");
        return;
    };
    let p = board.prepare(o.tactics).expect("prepare");
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace {
        gf2_prefilter: o.prefilter,
        gf2_branching: o.branching,
        gf2_strong_limit: o.strong,
        gf2_more_forced: o.more_forced,
        ..SearchSpace::new(&p, &graph, None)
    };
    let cfg = EngineConfig {
        threads: o.threads,
        time_limit: o.seconds.map(std::time::Duration::from_secs),
        ..EngineConfig::default()
    };
    let start = Start::Fresh {
        units: vec![TaxUnit { promoted: Bits::<W>::EMPTY, cands: p.candidates }],
        best: opt,
        payload: vec![0u8; 8 * W],
    };
    let before = counters::now();
    let t0 = Instant::now();
    let out = engine::run(&cfg, start, b"matroid-prove", |_| TaxSearcher::new(&space)).expect("search");
    let time = t0.elapsed();
    let after = counters::now();
    let (instructions, cycles) = match (before, after) {
        (Some((i0, c0)), Some((i1, c1))) => (i1 - i0, c1 - c0),
        _ => (0, 0),
    };
    let x = out.stats.extra;
    println!(
        "{{\"n\":{n},\"opt\":{opt},\"tactics\":\"{}\",\"prefilter\":{},\"branching\":\"{:?}\",\"threads\":{},\
         \"completed\":{},\"best\":{},\"nodes\":{},\"seconds\":{:.3},\"cycles\":{cycles},\"instructions\":{instructions},\
         \"checks\":{},\"gf2_solves\":{},\"gf2_augmentations\":{},\"gf2_strong_solves\":{},\"gf2_strong_augmentations\":{}}}",
        Tactics { search: o.tactics, ..Tactics::default() }.description(),
        o.prefilter,
        o.branching,
        o.threads,
        out.completed,
        out.best,
        out.stats.nodes,
        time.as_secs_f64(),
        x[stat::MATCH_BOUNDS],
        x[stat::GF2_SOLVES],
        x[stat::GF2_AUGMENTATIONS],
        x[stat::GF2_STRONG_SOLVES],
        x[stat::GF2_STRONG_AUGMENTATIONS],
    );
}
