//! The sacrifice-bound study: the command-line harness behind the measurements in `taxman::sacrifice`, which
//! has the bound, its proof, and the (negative) finding that computing it does not pay.  Study code: nothing in
//! `taxman solve` uses any of this.
//!
//! The brute-force and validation modes check the frame model on small boards, with one number taken off,
//! against exhaustive search (`taxman::brute`); the range mode computes the bound for real games with the frame
//! model and, if asked, times the direct solve of the same game for comparison.
//!
//! ```text
//! cargo run --release --example sacrifice -- brute [MAX] [--out FILE]
//!     n = 2..=MAX (default 60) by exhaustive search, no frame model: opt(n), and for every maximal factor f of n
//!     opt_M and opt (the maximal-factor rule and the rules) on {1..n-1} without f; checks the bound and the frame
//!     model's punctured games against them.
//! cargo run --release --example sacrifice -- validate [MAX]
//!     every board {1..N} without one number h, N <= MAX (default 40): the frame model against the brute force.
//! cargo run --release --example sacrifice -- range LO HI --out FILE [--threads T] [--games FILE]... [--direct]
//!     [--target] [--time-limit S] [--select all|needs|reuse|search] [--tactic NAME=VALUE]...
//!     the bound by the frame model for n = LO..=HI, one JSON line per game appended to FILE (games already in FILE
//!     are skipped, so a run can be resumed).  Previous games and optima come from optimal.json and the --games
//!     files (results files of `taxman solve`).  --direct also times solving n itself (force_search).  --target
//!     only decides whether the bound proves the known optimum (each punctured game stops once it beats it).
//!     --time-limit applies to each punctured game and to the direct solve.  --select picks the games that are
//!     neither prime nor exact (needs), and of those the ones reuse_prev derives (reuse) or not (search).  Search
//!     tactics default to the sound ones (revise_freedoms=off).
//! cargo run --release --example sacrifice -- summary FILE...
//!     statistics of range files.
//! ```

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use taxman::accelerate::reuse_prev;
use taxman::bits::words_for;
use taxman::board::Board;
use taxman::brute::{self, Rule};
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::results::{Taint, OPTIMAL_JSON};
use taxman::sacrifice::{bound_value, punctured, sacrifice_bound, PuncturedOptions};
use taxman::solver::{solve_game_with, OptimalSource, Prev, PrevSource, SolveOptions};
use taxman::tactics::Tactics;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("brute") => brute_study(&args[1..]),
        Some("validate") => validate(&args[1..]),
        Some("range") => range(&args[1..]),
        Some("summary") => summary(&args[1..]),
        _ => Err("usage: sacrifice brute|validate|range|summary ... (see the source)".into()),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}

// ---------------------------------------------------------------------------------------------------------
// Games

/// The known optimal games by n: optimal.json plus the completed games of the results files `extra`.
fn load_games(extra: &[String]) -> Result<Optimal, String> {
    let paths: Vec<PathBuf> = extra.iter().map(PathBuf::from).collect();
    SolveOptions::default().known_games(&paths)
}

fn secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

// ---------------------------------------------------------------------------------------------------------
// The brute-force study

/// The `brute` mode: for every `n` up to `MAX`, the sacrifice theorem checked on exhaustive-search values (so with
/// no frame model in the bound), one line per `n`, and the frame model's punctured games compared with the brute
/// force.  A counterexample to the theorem, or a disagreement of the frame model, is reported.
fn brute_study(args: &[String]) -> Result<(), String> {
    let mut max = 60;
    let mut out = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(it.next().ok_or("--out needs a file")?.clone()),
            v => max = v.parse().map_err(|e| format!("{v}: {e}"))?,
        }
    }
    let games = load_games(&[])?;
    let popts = PuncturedOptions::default();
    let mut lines = Vec::new();
    let (mut tight_m, mut tight_d, mut model_ok, mut holes_seen, mut composite, mut composite_tight) =
        (0, 0, 0, 0, 0, 0);
    let mut opt = vec![0u64; max as usize + 1];
    for n in 1..=max {
        opt[n as usize] = brute::solve(n, &[], Rule::Divisor).score;
        if Some(opt[n as usize]) != games.score(n) {
            return Err(format!(
                "n={n}: the brute force finds {}, optimal.json says {:?}",
                opt[n as usize],
                games.score(n)
            ));
        }
    }
    println!("    n   opt  prev sigma | bound_M bound_D tight | f: opt_M opt_D frame c(f)");
    for n in 2..=max {
        let t0 = Instant::now();
        let (o, prev) = (opt[n as usize], opt[n as usize - 1]);
        let prev_moves = games.moves(n - 1).ok_or("no previous game")?;
        let factors = Tables::new(n).max_factors[n as usize].clone();
        let mut parts = Vec::new();
        let mut holes = Vec::new();
        let (mut best_m, mut best_d) = (0u64, 0u64);
        for &f in &factors {
            let m = brute::solve(n - 1, &[f], Rule::MaximalFactor).score;
            let d = brute::solve(n - 1, &[f], Rule::Divisor).score;
            let frame = punctured(n - 1, f, prev_moves, prev, None, &popts)?;
            holes_seen += 1;
            model_ok += usize::from(frame.score == m);
            if frame.score != m {
                println!("  FRAME MODEL MISMATCH n-1={} hole={f}: frame {} brute {m}", n - 1, frame.score);
            }
            best_m = best_m.max(m);
            best_d = best_d.max(d);
            parts.push(format!("{f}: {m} {d} {} {}", frame.score, prev - m));
            holes.push(Json::object([
                ("f", Json::from(f)),
                ("opt_m", Json::from(m)),
                ("opt_d", Json::from(d)),
                ("frame", Json::from(frame.score)),
                ("cost", Json::from(prev - m)),
            ]));
        }
        let bound_m = bound_value(n, prev, [best_m]);
        let bound_d = bound_value(n, prev, [best_d]);
        if bound_m < o || bound_d < bound_m {
            return Err(format!("n={n}: COUNTEREXAMPLE opt {o} bound_M {bound_m} bound_D {bound_d}"));
        }
        tight_m += usize::from(bound_m == o);
        tight_d += usize::from(bound_d == o);
        let prime = factors == [1];
        let exact = o == prev + u64::from(n);
        if !prime && !exact {
            composite += 1;
            composite_tight += usize::from(bound_m == o);
        }
        let sigma = prev + u64::from(n) - o;
        let flag = if bound_m == o { "yes" } else { "no" };
        println!(
            "{n:5} {o:5} {prev:5} {sigma:5} | {bound_m:7} {bound_d:7} {flag:>5} | {}   ({:.2} s)",
            parts.join("; "),
            secs(t0.elapsed())
        );
        lines.push(Json::object([
            ("n", Json::from(n)),
            ("opt", Json::from(o)),
            ("prev_opt", Json::from(prev)),
            ("prime", Json::Bool(prime)),
            ("exact", Json::Bool(exact)),
            ("bound_m", Json::from(bound_m)),
            ("bound_d", Json::from(bound_d)),
            ("holes", Json::Arr(holes)),
        ]));
    }
    let games_count = max as usize - 1;
    println!(
        "n=2..={max}: bound holds everywhere; tight with opt_M in {tight_m}/{games_count}, with the rules' opt in \
         {tight_d}/{games_count}; neither prime nor exact: tight in {composite_tight}/{composite}; frame model = \
         brute force on {model_ok}/{holes_seen} punctured boards"
    );
    if let Some(path) = out {
        let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
        fs::write(&path, text).map_err(|e| format!("{path}: {e}"))?;
    }
    Ok(())
}

/// The `validate` mode: the frame model against the brute force on every board `1..=N` with one number `h` taken
/// off, under the maximal-factor rule, for every `N` up to `MAX`.  Prints any mismatch; the count at the end
/// should be 0.
fn validate(args: &[String]) -> Result<(), String> {
    let max: u32 = args.first().map_or(Ok(40), |a| a.parse().map_err(|e| format!("{a}: {e}")))?;
    let games = load_games(&[])?;
    let popts = PuncturedOptions::default();
    let (mut boards, mut mismatches) = (0, 0);
    for n in 1..=max {
        let t0 = Instant::now();
        let (full, full_score) = (games.moves(n).ok_or("no game")?, games.score(n).ok_or("no game")?);
        for hole in 1..=n {
            let exact = brute::solve(n, &[hole], Rule::MaximalFactor);
            let frame = punctured(n, hole, full, full_score, None, &popts)?;
            boards += 1;
            if frame.score != exact.score {
                mismatches += 1;
                println!(
                    "MISMATCH n={n} hole={hole}: frame {} {:?} brute {} {:?}",
                    frame.score, frame.moves, exact.score, exact.moves
                );
            }
        }
        println!("n={n}: {:.2} s", secs(t0.elapsed()));
    }
    println!("{boards} punctured boards up to n={max}: {mismatches} mismatches");
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// The frame-model range

/// Which games a range run looks at.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Select {
    All,
    /// Neither prime nor exact: the games that need more than the prime shortcut or the cap.
    Needs,
    /// Of those, the ones `reuse_prev` derives.
    Reuse,
    /// Of those, the ones it does not derive (which need a search).
    Search,
}

struct RangeOptions {
    lo: u32,
    hi: u32,
    out: String,
    threads: usize,
    games: Vec<String>,
    direct: bool,
    /// Decide whether the bound proves opt(n) (from the known games) instead of computing it exactly.
    target: bool,
    time_limit: Option<Duration>,
    select: Select,
    tactics: Tactics,
}

/// The `range` mode: parses the options, then computes one line per game `LO..=HI` ([`one_game`]) and appends it to
/// the output file as soon as it is done, skipping the games the file already has.
fn range(args: &[String]) -> Result<(), String> {
    let mut tactics = Tactics::default();
    tactics.set("reuse_prev=off")?;
    tactics.set("revise_freedoms=off")?;
    let mut o = RangeOptions {
        lo: 0,
        hi: 0,
        out: String::new(),
        threads: 2,
        games: Vec::new(),
        direct: false,
        target: false,
        time_limit: None,
        select: Select::All,
        tactics,
    };
    let mut bounds = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().cloned().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--out" => o.out = value()?,
            "--threads" => o.threads = value()?.parse().map_err(|e| format!("--threads: {e}"))?,
            "--games" => o.games.push(value()?),
            "--direct" => o.direct = true,
            "--target" => o.target = true,
            "--time-limit" => {
                let s: f64 = value()?.parse().map_err(|e| format!("--time-limit: {e}"))?;
                o.time_limit = Some(Duration::from_secs_f64(s));
            }
            "--select" => {
                o.select = match value()?.as_str() {
                    "all" => Select::All,
                    "needs" => Select::Needs,
                    "reuse" => Select::Reuse,
                    "search" => Select::Search,
                    other => return Err(format!("--select: expected all, needs, reuse or search, got {other}")),
                }
            }
            "--tactic" => o.tactics.set(&value()?)?,
            v => bounds.push(v.parse::<u32>().map_err(|e| format!("{v}: {e}"))?),
        }
    }
    let [lo, hi] = bounds[..] else { return Err("range needs LO and HI".into()) };
    (o.lo, o.hi) = (lo.max(2), hi);
    if o.out.is_empty() {
        return Err("range needs --out FILE".into());
    }
    let games = load_games(&o.games)?;
    let done: Vec<u32> = match fs::read_to_string(&o.out) {
        Ok(text) => text.lines().filter_map(|l| Json::parse(l).ok()?.get("n")?.as_u32()).collect(),
        Err(_) => Vec::new(),
    };
    let mut file = OpenOptions::new().create(true).append(true).open(&o.out).map_err(|e| format!("{}: {e}", o.out))?;
    for n in o.lo..=o.hi {
        if done.contains(&n) {
            continue;
        }
        let line = match words_for(n) {
            1..=16 => one_game::<16>(n, &o, &games)?,
            17..=32 => one_game::<32>(n, &o, &games)?,
            _ => return Err(format!("n={n} is beyond this study")),
        };
        if let Some(line) = line {
            writeln!(file, "{line}").and_then(|()| file.sync_data()).map_err(|e| format!("{}: {e}", o.out))?;
        }
    }
    Ok(())
}

/// One game's line, or `None` if the game is not selected.
fn one_game<const W: usize>(n: u32, o: &RangeOptions, games: &Optimal) -> Result<Option<Json>, String> {
    let (prev_moves, prev_score) = match (games.moves(n - 1), games.score(n - 1)) {
        (Some(m), Some(s)) => (m, s),
        _ => return Err(format!("no game for n={}", n - 1)),
    };
    let opt = games.score(n);
    let prime = Tables::new(n).is_prime(n);
    let exact = opt == Some(prev_score + u64::from(n));
    if o.select != Select::All && (prime || exact) {
        return Ok(None);
    }
    let board = Board::<W>::for_n(n);
    let trivial = board.is_trivial();
    let reuse = if trivial {
        None
    } else {
        reuse_prev(&board.tables, &board.frames, prev_moves).map(|a| a.moves.iter().map(|&m| u64::from(m)).sum::<u64>())
    };
    if (o.select == Select::Reuse && reuse.is_none()) || (o.select == Select::Search && (reuse.is_some() || trivial)) {
        return Ok(None);
    }

    let popts = PuncturedOptions { tactics: o.tactics.search, threads: o.threads, time_limit: o.time_limit };
    let target = if o.target { Some(opt.ok_or(format!("n={n}: --target needs the optimum"))?) } else { None };
    let bound = sacrifice_bound(n, prev_moves, prev_score, target, &popts).map_err(|e| format!("n={n}: {e}"))?;
    let holes: Vec<Json> = bound
        .holes
        .iter()
        .map(|h| {
            Json::object([
                ("f", Json::from(h.hole)),
                ("score", Json::from(h.score)),
                ("cost", Json::from(prev_score - h.score)),
                ("how", Json::from(h.how.as_str())),
                ("exact", Json::Bool(h.exact)),
                ("nodes", Json::from(h.nodes)),
                ("secs", Json::Num(secs(h.time))),
            ])
        })
        .collect();

    let direct = if o.direct && !prime && !trivial {
        let opts = SolveOptions {
            tactics: o.tactics,
            threads: o.threads,
            force_search: true,
            optimal: OptimalSource::None,
            time_limit: o.time_limit,
            // The previous game is optimal.json's, and the answer is compared with the known optimum.
            allow_tainted_prev: true,
            ..SolveOptions::default()
        };
        let taint = Taint::from([OPTIMAL_JSON.to_owned()]);
        let prev =
            Prev { moves: prev_moves.to_vec(), score: prev_score, source: PrevSource::Optimal, proof: None, taint };
        let r = solve_game_with(n, Some(&prev), &opts, None)?;
        if r.completed && opt.is_some_and(|s| s != r.score) {
            return Err(format!("n={n}: the direct search scores {}, not {opt:?}", r.score));
        }
        Json::object([
            ("score", Json::from(r.score)),
            ("completed", Json::Bool(r.completed)),
            ("secs", Json::Num(secs(r.phases.total))),
            ("search_secs", Json::Num(secs(r.phases.search))),
            ("nodes", Json::from(r.nodes)),
        ])
    } else {
        Json::Null
    };

    // In a decision, tight means that the bound proves the optimum.
    let tight = opt.map(|s| if o.target { bound.proves(s) } else { bound.value == s });
    if let Some(s) = opt {
        if bound.exact() && bound.value < s {
            return Err(format!("n={n}: COUNTEREXAMPLE: bound {} below the optimum {s}", bound.value));
        }
    }
    // A cost marked `?` is only an upper bound (its punctured game stopped early).
    let costs: Vec<String> = (bound.holes.iter().zip(bound.costs()))
        .map(|(h, (f, c))| format!("{f}:{c}{}", if h.exact { "" } else { "?" }))
        .collect();
    println!(
        "n={n:4} opt {} bound {} tight {} {}{}{} costs [{}] bound {:.2} s{}",
        opt.map_or("?".into(), |s| s.to_string()),
        bound.value,
        tight.map_or("?", |t| if t { "yes" } else { "NO" }),
        if prime { "prime " } else { "" },
        if opt == Some(prev_score + u64::from(n)) { "exact " } else { "" },
        reuse.map_or(String::new(), |r| format!("reuse={r} ")),
        costs.join(" "),
        secs(bound.time()),
        match &direct {
            Json::Null => String::new(),
            d => format!(", direct {:.2} s", d.get("secs").and_then(Json::as_f64).unwrap_or(0.0)),
        }
    );
    Ok(Some(Json::object([
        ("n", Json::from(n)),
        ("opt", opt.map_or(Json::Null, Json::from)),
        ("prev_opt", Json::from(prev_score)),
        ("prime", Json::Bool(prime)),
        ("trivial", Json::Bool(trivial)),
        ("reuse", reuse.map_or(Json::Null, Json::from)),
        ("target", target.map_or(Json::Null, Json::from)),
        ("bound", Json::from(bound.value)),
        ("proven", Json::Bool(bound.exact())),
        ("tight", tight.map_or(Json::Null, Json::Bool)),
        ("holes", Json::Arr(holes)),
        ("bound_secs", Json::Num(secs(bound.time()))),
        ("bound_nodes", Json::from(bound.nodes())),
        ("direct", direct),
        ("threads", Json::from(o.threads)),
        ("tactics", Json::from(o.tactics.description())),
    ])))
}

// ---------------------------------------------------------------------------------------------------------
// Summary

/// One counter of games, and how many of them the bound is tight on.
#[derive(Default)]
struct Tally {
    games: usize,
    tight: usize,
    bound_secs: f64,
    direct_secs: f64,
    direct_games: usize,
    /// Games where the bound is tight and took less time than the direct search.
    cheaper: usize,
    /// The bound first, then the direct search where the bound does not prove the optimum.
    pipeline_secs: f64,
    /// Bound time over direct time, on the games where the bound is tight.
    ratios: Vec<f64>,
}

impl Tally {
    fn add(&mut self, g: &Json) {
        self.games += 1;
        let tight = g.get("tight").and_then(Json::as_bool) == Some(true);
        self.tight += usize::from(tight);
        let bound = g.get("bound_secs").and_then(Json::as_f64).unwrap_or(0.0);
        self.bound_secs += bound;
        if let Some(d) = g.get("direct").and_then(|d| d.get("secs")).and_then(Json::as_f64) {
            self.direct_secs += d;
            self.direct_games += 1;
            self.pipeline_secs += bound + if tight { 0.0 } else { d };
            if tight {
                self.cheaper += usize::from(bound < d);
                self.ratios.push(bound / d.max(1e-9));
            }
        }
    }

    fn line(&self, label: &str) -> String {
        let direct = if self.direct_games > 0 {
            format!(
                "; bound {:.1} s vs direct {:.1} s on {} games (tight and cheaper on {})",
                self.bound_secs, self.direct_secs, self.direct_games, self.cheaper
            )
        } else {
            format!("; bound {:.1} s", self.bound_secs)
        };
        format!(
            "{label:44} {:4} games, tight {:4} ({:5.1}%){direct}",
            self.games,
            self.tight,
            pct(self.tight, self.games)
        )
    }
}

fn pct(a: usize, b: usize) -> f64 {
    100.0 * a as f64 / b.max(1) as f64
}

/// The `summary` mode: reads range files and prints how often the bound is tight, split by kind of game (prime,
/// exact, neither; of those trivial, derived by `reuse_prev`, or needing a search), with the time comparisons.
fn summary(args: &[String]) -> Result<(), String> {
    let mut rows: BTreeMap<u32, Json> = BTreeMap::new();
    for path in args {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let json = Json::parse(line).map_err(|e| format!("{path}: {e}"))?;
            let n = json.get("n").and_then(Json::as_u32).ok_or("bad n")?;
            rows.insert(n, json);
        }
    }
    let (lo, hi) = (rows.keys().next().copied().unwrap_or(0), rows.keys().last().copied().unwrap_or(0));
    let [mut all, mut prime, mut exact, mut trivial, mut needs, mut reuse, mut reuse_needs, mut searched] =
        std::array::from_fn::<Tally, 8, _>(|_| Tally::default());
    let (mut reuse_wrong, mut not_proven, mut zero_cost_holes, mut holes) = (0, 0, 0, 0);
    let mut gaps: Vec<u64> = Vec::new();
    for g in rows.values() {
        let get = |k: &str| g.get(k).and_then(Json::as_u64);
        let (Some(opt), Some(prev), Some(bound)) = (get("opt"), get("prev_opt"), get("bound")) else { continue };
        let n = u64::from(g.get("n").and_then(Json::as_u32).unwrap_or(0));
        let is = |k: &str| g.get(k).and_then(Json::as_bool) == Some(true);
        all.add(g);
        not_proven += usize::from(!is("proven"));
        if bound > opt {
            gaps.push(bound - opt);
        }
        for h in g.get("holes").and_then(Json::as_arr).unwrap_or(&[]) {
            holes += 1;
            zero_cost_holes += usize::from(h.get("cost").and_then(Json::as_u64) == Some(0));
        }
        let is_exact = opt == prev + n;
        let reused = get("reuse");
        if is("prime") {
            prime.add(g);
        } else if is_exact {
            exact.add(g);
        } else {
            needs.add(g);
            if is("trivial") {
                trivial.add(g);
            } else if reused.is_some() {
                reuse_needs.add(g);
            } else {
                searched.add(g);
            }
        }
        if let Some(r) = reused {
            reuse.add(g);
            reuse_wrong += usize::from(r != opt);
        }
    }
    gaps.sort_unstable();
    println!("n = {lo}..={hi}");
    println!("{}", all.line("all games"));
    println!("{}", prime.line("prime"));
    println!("{}", exact.line("exact (opt = n + opt(n-1)), not prime"));
    println!("{}", needs.line("neither prime nor exact"));
    println!("{}", trivial.line("  of which trivial (no candidates)"));
    println!("{}", reuse_needs.line("  of which reuse_prev derives"));
    println!("{}", searched.line("  of which need a search"));
    println!("{}", reuse.line("every game reuse_prev derives"));
    println!("reuse_prev's score differs from the optimum on {reuse_wrong} games");
    println!("bound not exact (a punctured game stopped: a decision beaten, or a time limit) on {not_proven} games");
    println!("holes with c(f) = 0: {zero_cost_holes} of {holes}");
    let quantiles = |v: &[f64]| {
        let q = |x: f64| v[((v.len() - 1) as f64 * x) as usize];
        format!("min {:.2} p25 {:.2} median {:.2} p75 {:.2} max {:.2}", v[0], q(0.25), q(0.5), q(0.75), q(1.0))
    };
    if !gaps.is_empty() {
        let gaps: Vec<f64> = gaps.iter().map(|&g| g as f64).collect();
        println!(
            "gap bound - opt where not tight ({} games; lower bounds in a decision): {}",
            gaps.len(),
            quantiles(&gaps)
        );
    }
    if !needs.ratios.is_empty() {
        let mut ratios = needs.ratios.clone();
        ratios.sort_by(f64::total_cmp);
        println!("bound / direct on the games the bound proves: {}", quantiles(&ratios));
    }
    if needs.direct_games > 0 {
        // Only meaningful for decisions (--target): an exact bound that fails costs more than a decision would.
        println!(
            "prime -> cap -> bound -> search vs prime -> cap -> search, neither prime nor exact: {:.1} s vs {:.1} s \
             ({:+.0}%) on {} games",
            needs.pipeline_secs,
            needs.direct_secs,
            100.0 * (needs.pipeline_secs / needs.direct_secs.max(1e-9) - 1.0),
            needs.direct_games
        );
    }
    Ok(())
}
