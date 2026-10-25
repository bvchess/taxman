//! The arrival of n in the frame model: for each game n of a range, whether game n poses the root problem of game
//! n - 1 when every frame that n joins is closed in game n - 1.  It searches nothing: it builds the root problem Π of
//! each board from the definitions of the theory alone ([`taxman::root_problem::build_with_frames`], docs/theory/
//! frame-model.md) and compares the boards n - 1 and n.
//!
//! To run it, copy this file to solver/frame/examples/arrival.rs, and from solver/frame:
//!
//! ```text
//! cargo run --release --example arrival -- FROM TO [--threads K] [--out FILE] > summary.txt
//! ```
//!
//! then delete the copy.  `FROM` is at least 3, `TO` at most the largest board the solver supports.  For every board
//! `n` in `FROM..=TO` it writes one JSON object on one line.  With `--out FILE` the lines are appended to FILE, and a
//! run that finds FILE already holding lines of the range skips those boards (a last line cut short by an
//! interruption is cut off first), so an interrupted run resumes; without it the lines go to stdout.  `--threads K`
//! (default 1) runs K threads on runs of consecutive boards; the lines are written in increasing `n` however the
//! threads finish, each flushed as it is written.  Progress goes to stderr about every 100 boards.  At the end it
//! reads the lines of the range again and prints the summary (to stdout, or to stderr when the lines went to stdout).
//!
//! **The hypothesis.**  Game n differs from game n - 1 by the arrival of n.  Let a *maximal factor* of n be n/p for a
//! prime p dividing n, and `cond` say that the frames n joins are closed in game n - 1:
//!
//! * for an odd n, every number up to n/2 is the same on both boards, and every maximal factor of n is a factor of a
//!   frame of game n - 1; `cond` holds when none of those frames has a freedom;
//! * for an even n, n/2 turns from a source of game n - 1 into a factor that only n uses; `cond` holds when n/2 is not
//!   in U\* of game n - 1 and every maximal factor n/p with p odd is a factor of a frame of game n - 1 with no
//!   freedom.
//!
//! The hypothesis is that when `cond` holds, game n has the root problem of game n - 1 (`repeat`), and that for an odd
//! n exactly one source drops out of U\*, the frame n joins is closed, and the source is the shadow of n; for an even
//! n nothing drops out, and U\*(n) = U\*(n - 1) + n.
//!
//! **A line.**
//!
//! * `n`, `fac` (the factorization as `[prime, exponent]` pairs), `parity` (`"odd"` or `"even"`), `omega` (Ω(n)),
//!   `prev_prime` (whether n - 1 is prime), and `secs` (the wall seconds, the build of the boards included).
//! * `max_factors`: for each prime p dividing n, in ascending order, `{"p", "m"}` with `m` = n/p, and the frame of game
//!   n - 1 that holds `m`: `held` (whether `m` is among its factors), `layer`, and `free` (its freedoms, its factors
//!   less its moves).  A maximal factor that is no factor of game n - 1 is a source there, which happens only for
//!   n/2 of an even n; it has `held` false and `in_ustar`, whether U\*(n - 1) holds it.  `half_in_ustar` repeats that
//!   for an even n (`null` for an odd one).  `held_all`: every maximal factor but n/2 is held.
//! * `cond` as above, and `joined`, the number of different frames of game n - 1 that hold a maximal factor.
//! * `repeat`: the candidates and the root frames (moves and factors) of game n equal those of game n - 1.  U\* is not
//!   compared: it always differs, since n is in U\*(n).  `repeat_prev2` is the same comparison with game n - 2,
//!   present when n - 1 is prime and n >= 4 (`null` otherwise).  `candidates` and `root_frames` count them in game n.
//! * `added` and `dropped`: U\*(n) less U\*(n - 1), and U\*(n - 1) less U\*(n).  They are set differences, so n/2 of an
//!   even n is in `dropped` when it was in U\*(n - 1) (it becomes a factor), and never when it was not.
//!   `dropped_in_joined`: every dropped source is a source of a frame of game n - 1 that holds a maximal factor of n.
//! * `merged_layer`, `merged_kept`, `merged_factors` and `merged_free`: the frame of game n that holds n, its moves and
//!   factors, and its freedoms.  For an odd n, `merged_is_union`: that frame is n and the frames of game n - 1 that
//!   hold its maximal factors, put together; `shadow` is the shadow of n (each prime factor replaced by the prime
//!   just below it, with multiplicity), and `dropped_is_shadow` whether `dropped` is exactly `[shadow]`.  All three
//!   are `null` for an even n.
//!
//! **The summary** counts, for the odd and the even n and for n from FROM and from 188, the four cases of `cond` and
//! `repeat`, once with `repeat` as defined and once with `repeat_prev2` where n - 1 is prime (`partner repeat`); lists
//! every n where `cond` holds and `repeat` does not and the first 30 where `repeat` holds and `cond` does not; counts,
//! of the odd n with `cond`, those with exactly one dropped source, a merged frame with no freedom and the shadow
//! dropped, and, of the even n with `cond`, those that dropped nothing and those whose `added` is n alone; and checks
//! `repeat` against the committed chain (results/strict.jsonl, or `--chain FILE`; the first completed line of each
//! n): for each n of the range up to the chain's end whose method is not `known` and whose line has a setup digest,
//! `repeat` (`repeat_prev2` where n - 1 is prime) must equal whether the digest is that of game n - 1 (n - 2 where
//! n - 1 is prime).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Instant;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::root_problem::{build_with_frames, Frame, RootProblem};
use taxman::solver::MAX_N;

const USAGE: &str = "usage: arrival FROM TO [--threads K] [--out FILE] [--chain FILE]   (3 <= FROM <= TO)";

/// The first n of the second row of every count in the summary.
const LARGE: u32 = 188;

/// How many boards a thread takes at a time: the board n - 1 is built once for a run of boards, not twice.
const RUN: usize = 16;

/// How many of the games where `repeat` holds and `cond` does not the summary lists.
const LISTED: usize = 30;

/// What the command line asks for.
struct Args {
    from: u32,
    to: u32,
    threads: usize,
    out: Option<PathBuf>,
    chain: PathBuf,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let (range, flags) = args.split_at(2.min(args.len()));
    let [from, to] = range else { return Err(USAGE.to_owned()) };
    let board = |text: &String| text.parse::<u32>().map_err(|_| USAGE.to_owned());
    let (from, to) = (board(from)?, board(to)?);
    let (mut threads, mut out) = (1, None);
    let mut chain = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/strict.jsonl");
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        match (flag.as_str(), flags.next()) {
            ("--threads", Some(k)) => threads = k.parse().ok().filter(|&k| k >= 1).ok_or(USAGE)?,
            ("--out", Some(path)) => out = Some(PathBuf::from(path)),
            ("--chain", Some(path)) => chain = PathBuf::from(path),
            _ => return Err(USAGE.to_owned()),
        }
    }
    if from < 3 || from > to || to > MAX_N {
        return Err(format!("{USAGE}\nthe boards must be from 3 to {MAX_N}"));
    }
    Ok(Args { from, to, threads, out, chain })
}

// ---------------------------------------------------------------------------------------------------------
// Numbers

/// The prime factorization of `n` as `(prime, exponent)` pairs, ascending.
fn factorize(mut n: u32) -> Vec<(u32, u32)> {
    let mut factors = Vec::new();
    let mut p = 2;
    while p * p <= n {
        let mut exponent = 0;
        while n % p == 0 {
            n /= p;
            exponent += 1;
        }
        if exponent > 0 {
            factors.push((p, exponent));
        }
        p += 1;
    }
    if n > 1 {
        factors.push((n, 1));
    }
    factors
}

fn is_prime(n: u32) -> bool {
    n >= 2 && factorize(n) == [(n, 1)]
}

/// The largest prime below `p`, for a prime `p` above 2.
fn prime_below(p: u32) -> u32 {
    (2..p).rev().find(|&q| is_prime(q)).expect("a prime above 2 has a prime below it")
}

/// The shadow of an odd `n` with the factorization `fac`: each prime factor replaced by the prime just below it, with
/// multiplicity.
fn shadow(fac: &[(u32, u32)]) -> u32 {
    fac.iter().map(|&(p, e)| prime_below(p).pow(e)).product()
}

/// The members of the ascending list `a` that are not in the ascending list `b`.
fn minus(a: &[u32], b: &[u32]) -> Vec<u32> {
    a.iter().copied().filter(|x| b.binary_search(x).is_err()).collect()
}

/// `n` written as its factorization: `3^2*5`.
fn product(n: u32) -> String {
    let parts: Vec<String> =
        factorize(n).iter().map(|&(p, e)| if e > 1 { format!("{p}^{e}") } else { p.to_string() }).collect();
    parts.join("*")
}

// ---------------------------------------------------------------------------------------------------------
// The boards

/// A board of the game: its root problem, its frames, and the frame that holds each number.
struct Board {
    root: RootProblem,
    frames: Vec<Frame>,
    /// The index in `frames` of the frame that holds each number `1..=n`.
    holder: Vec<usize>,
}

impl Board {
    fn new(n: u32) -> Board {
        let (root, frames) = build_with_frames(&Tables::new(n));
        let mut holder = vec![usize::MAX; n as usize + 1];
        for (i, frame) in frames.iter().enumerate() {
            for &x in frame.sources.iter().chain(&frame.factors) {
                holder[x as usize] = i;
            }
        }
        assert!(holder[1..].iter().all(|&i| i != usize::MAX), "board {n}: a number is in no frame");
        Board { root, frames, holder }
    }

    fn frame_of(&self, x: u32) -> &Frame {
        &self.frames[self.holder[x as usize]]
    }

    fn in_ustar(&self, x: u32) -> bool {
        self.root.ustar.binary_search(&x).is_ok()
    }

    /// Whether the root problem of `self` is that of `other`: the same candidates and root frames.
    fn repeats(&self, other: &Board) -> bool {
        self.root.candidates == other.root.candidates && self.root.frames == other.root.frames
    }
}

fn freedoms(frame: &Frame) -> usize {
    frame.factors.len() - frame.kept.len()
}

/// The boards a thread built lately, so that a run of boards builds each once.
#[derive(Default)]
struct Cache(HashMap<u32, Rc<Board>>);

impl Cache {
    fn get(&mut self, n: u32) -> Rc<Board> {
        Rc::clone(self.0.entry(n).or_insert_with(|| Rc::new(Board::new(n))))
    }
}

// ---------------------------------------------------------------------------------------------------------
// The line of one board

/// What game n - 1 says about one maximal factor of n.
struct MaximalFactor {
    p: u32,
    m: u32,
    held: bool,
    layer: u32,
    free: usize,
    in_ustar: bool,
}

impl MaximalFactor {
    fn json(&self) -> Json {
        let mut pairs = vec![
            ("p", Json::from(self.p)),
            ("m", Json::from(self.m)),
            ("held", Json::from(self.held)),
            ("layer", Json::from(self.layer)),
            ("free", Json::from(self.free)),
        ];
        if !self.held {
            pairs.push(("in_ustar", Json::from(self.in_ustar)));
        }
        Json::object(pairs)
    }

    /// Whether the frame is a closed one that holds the number among its factors.
    fn closed_factor(&self) -> bool {
        self.held && self.free == 0
    }
}

fn numbers(list: &[u32]) -> Json {
    Json::array(list.iter().copied())
}

/// The line of the board `n >= 3`, from the boards `n - 1` and (where `n - 1` is prime and `n >= 4`) `n - 2`.
fn line_of(n: u32, prev: &Board, cur: &Board, prev2: Option<&Board>) -> Json {
    let odd = n % 2 == 1;
    let fac = factorize(n);

    let maximal: Vec<MaximalFactor> = (fac.iter())
        .map(|&(p, _)| {
            let m = n / p;
            let frame = prev.frame_of(m);
            let held = frame.factors.binary_search(&m).is_ok();
            MaximalFactor { p, m, held, layer: frame.layer, free: freedoms(frame), in_ustar: prev.in_ustar(m) }
        })
        .collect();
    let joined: BTreeSet<usize> = maximal.iter().map(|mf| prev.holder[mf.m as usize]).collect();
    let half_in_ustar = (!odd).then(|| prev.in_ustar(n / 2));
    let others = maximal.iter().filter(|mf| odd || mf.p != 2);
    let held_all = others.clone().all(|mf| mf.held);
    let cond = half_in_ustar != Some(true) && others.clone().all(MaximalFactor::closed_factor);

    let added = minus(&cur.root.ustar, &prev.root.ustar);
    let dropped = minus(&prev.root.ustar, &cur.root.ustar);
    let dropped_in_joined = dropped.iter().all(|&x| joined.contains(&prev.holder[x as usize]));

    let merged = cur.frame_of(n);
    let (merged_is_union, shadow_of_n, dropped_is_shadow) = if odd {
        let (mut sources, mut factors) = (vec![n], Vec::new());
        for &i in &joined {
            sources.extend(&prev.frames[i].sources);
            factors.extend(&prev.frames[i].factors);
        }
        sources.sort_unstable();
        factors.sort_unstable();
        let shadow_of_n = shadow(&fac);
        let union = merged.sources == sources && merged.factors == factors;
        (Json::from(union), Json::from(shadow_of_n), Json::from(dropped == [shadow_of_n]))
    } else {
        (Json::Null, Json::Null, Json::Null)
    };

    Json::object([
        ("n", Json::from(n)),
        ("fac", Json::Arr(fac.iter().map(|&(p, e)| Json::array([p, e])).collect())),
        ("parity", Json::from(if odd { "odd" } else { "even" })),
        ("omega", Json::from(fac.iter().map(|&(_, e)| e).sum::<u32>())),
        ("prev_prime", Json::from(is_prime(n - 1))),
        ("max_factors", Json::Arr(maximal.iter().map(MaximalFactor::json).collect())),
        ("half_in_ustar", half_in_ustar.map_or(Json::Null, Json::from)),
        ("held_all", Json::from(held_all)),
        ("cond", Json::from(cond)),
        ("joined", Json::from(joined.len())),
        ("repeat", Json::from(cur.repeats(prev))),
        ("repeat_prev2", prev2.map_or(Json::Null, |board| Json::from(cur.repeats(board)))),
        ("candidates", Json::from(cur.root.candidates.len())),
        ("root_frames", Json::from(cur.root.frames.len())),
        ("added", numbers(&added)),
        ("dropped", numbers(&dropped)),
        ("dropped_in_joined", Json::from(dropped_in_joined)),
        ("merged_layer", Json::from(merged.layer)),
        ("merged_kept", Json::from(merged.kept.len())),
        ("merged_factors", Json::from(merged.factors.len())),
        ("merged_free", Json::from(freedoms(merged))),
        ("merged_is_union", merged_is_union),
        ("shadow", shadow_of_n),
        ("dropped_is_shadow", dropped_is_shadow),
    ])
}

/// The line of the board `n`, with the seconds it took added.
fn timed_line(n: u32, cache: &mut Cache) -> Json {
    let start = Instant::now();
    let (prev, cur) = (cache.get(n - 1), cache.get(n));
    let prev2 = (is_prime(n - 1) && n >= 4).then(|| cache.get(n - 2));
    let mut line = line_of(n, &prev, &cur, prev2.as_deref());
    // Boards before n - 1 are not needed again by the run.
    cache.0.retain(|&k, _| k + 1 >= n);
    if let Json::Obj(pairs) = &mut line {
        let secs = (start.elapsed().as_secs_f64() * 1000.0).round() / 1000.0;
        pairs.insert(5, ("secs".to_owned(), Json::from(secs)));
    }
    line
}

// ---------------------------------------------------------------------------------------------------------
// The summary

/// A line of the file, read for the summary.
struct Row {
    n: u32,
    odd: bool,
    prev_prime: bool,
    cond: bool,
    repeat: bool,
    repeat_prev2: Option<bool>,
    added: Vec<u32>,
    dropped: Vec<u32>,
    merged_free: u64,
    dropped_is_shadow: bool,
    held_all: bool,
    secs: f64,
}

impl Row {
    fn of(line: &Json) -> Option<Row> {
        let flag = |key: &str| line.get(key).and_then(Json::as_bool);
        let list = |key: &str| line.get(key).and_then(Json::as_u32_vec);
        Some(Row {
            n: line.get("n").and_then(Json::as_u32)?,
            odd: line.get("parity").and_then(Json::as_str)? == "odd",
            prev_prime: flag("prev_prime")?,
            cond: flag("cond")?,
            repeat: flag("repeat")?,
            repeat_prev2: flag("repeat_prev2"),
            added: list("added")?,
            dropped: list("dropped")?,
            merged_free: line.get("merged_free").and_then(Json::as_u64)?,
            dropped_is_shadow: flag("dropped_is_shadow").unwrap_or(false),
            held_all: flag("held_all")?,
            secs: line.get("secs").and_then(Json::as_f64).unwrap_or(0.0),
        })
    }

    /// `repeat` against the partner game: n - 1, or n - 2 where n - 1 is prime (`None` where that is not recorded).
    fn partner_repeat(&self) -> Option<bool> {
        if self.prev_prime {
            self.repeat_prev2
        } else {
            Some(self.repeat)
        }
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "-".to_owned()
    } else {
        format!("{:.1}%", 100.0 * part as f64 / whole as f64)
    }
}

/// The four cases of `cond` and `repeat` among `rows`, as a line.
fn four_cases(rows: &[&Row], repeat: impl Fn(&Row) -> Option<bool>) -> String {
    let mut counts = [[0usize; 2]; 2];
    let mut unknown = 0;
    for row in rows {
        match repeat(row) {
            Some(r) => counts[usize::from(row.cond)][usize::from(r)] += 1,
            None => unknown += 1,
        }
    }
    let note = if unknown > 0 { format!("; no comparison for {unknown}") } else { String::new() };
    format!(
        "cond and repeat {}; cond and not repeat {}; not cond and repeat {}; neither {}{note}",
        counts[1][1], counts[1][0], counts[0][1], counts[0][0]
    )
}

/// The first completed line of each n of the chain: its method and its setup digest, if it has one.
fn load_chain(path: &Path) -> Result<BTreeMap<u32, (String, Option<String>)>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut games = BTreeMap::new();
    for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let row = Json::parse(line).map_err(|e| format!("{} line {}: {e}", path.display(), i + 1))?;
        if row.get("completed").and_then(Json::as_bool) != Some(true) {
            continue;
        }
        let n =
            row.get("n").and_then(Json::as_u32).ok_or_else(|| format!("{} line {}: no n", path.display(), i + 1))?;
        let method = row.get("method").and_then(Json::as_str).unwrap_or("").to_owned();
        let digest = row.get("setup").and_then(|setup| setup.get("digest")).and_then(Json::as_str);
        games.entry(n).or_insert((method, digest.map(str::to_owned)));
    }
    Ok(games)
}

/// The rows of the summary about the chain: `repeat` against the equality of setup digests.
fn chain_rows(rows: &[Row], chain: &BTreeMap<u32, (String, Option<String>)>, out: &mut Vec<String>) {
    let (mut agree, mut known, mut no_digest, mut no_partner, mut no_flag) = (0, 0, 0, 0, 0);
    let mut disagree = Vec::new();
    let mut last = 0;
    for row in rows.iter().filter(|row| chain.contains_key(&row.n)) {
        last = row.n;
        let (method, digest) = &chain[&row.n];
        let partner = if row.prev_prime { row.n - 2 } else { row.n - 1 };
        if method == "known" {
            known += 1;
        } else if digest.is_none() {
            no_digest += 1;
        } else if row.partner_repeat().is_none() {
            no_flag += 1;
        } else if let Some(partner_digest) = chain.get(&partner).and_then(|(_, digest)| digest.as_ref()) {
            let (mine, theirs) = (row.partner_repeat() == Some(true), digest.as_ref() == Some(partner_digest));
            if mine == theirs {
                agree += 1;
            } else {
                disagree.push(format!(
                    "n={} ({}): repeat {mine}, digest equal to game {partner}'s {theirs}, method {method}",
                    row.n,
                    product(row.n)
                ));
            }
        } else {
            no_partner += 1;
        }
    }
    out.push(format!(
        "== Against the committed chain, {} boards of the range up to n = {last}",
        rows.iter().filter(|row| chain.contains_key(&row.n)).count()
    ));
    out.push(format!(
        "compared {}: agree {agree}, disagree {}; left out: method known {known}, no setup digest {no_digest}, partner \
         game without a digest {no_partner}, no repeat_prev2 {no_flag}",
        agree + disagree.len(),
        disagree.len()
    ));
    out.extend(disagree.iter().map(|line| format!("  {line}")));
}

/// The summary of the rows of the boards `from..=to` and the chain, if it could be read.
fn summary(
    rows: &[Row],
    from: u32,
    to: u32,
    chain: &Result<BTreeMap<u32, (String, Option<String>)>, String>,
) -> String {
    let mut out = Vec::new();
    let present: BTreeSet<u32> = rows.iter().map(|row| row.n).collect();
    let missing: Vec<u32> = (from..=to).filter(|n| !present.contains(n)).collect();
    out.push(format!("boards {from}..={to}: {} lines; missing {}", rows.len(), missing.len()));
    out.push(format!("seconds, summed over the boards: {:.1}", rows.iter().map(|row| row.secs).sum::<f64>()));
    let not_held: Vec<String> = rows.iter().filter(|row| !row.held_all).map(|row| row.n.to_string()).collect();
    out.push(format!("boards with a maximal factor (n/2 aside) held by no frame as a factor: {}", not_held.len()));

    for (label, odd) in [("odd n", true), ("even n", false)] {
        for first in if from < LARGE { vec![from, LARGE] } else { vec![from] } {
            let part: Vec<&Row> = rows.iter().filter(|row| row.odd == odd && row.n >= first).collect();
            out.push(String::new());
            out.push(format!(
                "== {label} from {first}: {} boards, {} with cond",
                part.len(),
                part.iter().filter(|r| r.cond).count()
            ));
            out.push(format!("repeat:         {}", four_cases(&part, |row| Some(row.repeat))));
            out.push(format!("partner repeat: {}", four_cases(&part, Row::partner_repeat)));
            let with_cond: Vec<&&Row> = part.iter().filter(|row| row.cond).collect();
            let count = |test: &dyn Fn(&Row) -> bool| with_cond.iter().filter(|row| test(row)).count();
            let total = with_cond.len();
            let row = |what: &str, k: usize| format!("with cond, {what}: {k} of {total} ({})", percent(k, total));
            if odd {
                let one = count(&|r| r.dropped.len() == 1);
                let closed = count(&|r| r.merged_free == 0);
                let shadow = count(&|r| r.dropped_is_shadow);
                out.push(row("exactly one source dropped", one));
                out.push(row("a merged frame with no freedom", closed));
                out.push(row("dropped exactly the shadow", shadow));
                out.push(row(
                    "all three",
                    count(&|r| r.dropped.len() == 1 && r.merged_free == 0 && r.dropped_is_shadow),
                ));
            } else {
                out.push(row("dropped nothing", count(&|r| r.dropped.is_empty())));
                out.push(row("added n alone", count(&|r| r.added == [r.n])));
                out.push(row("both", count(&|r| r.dropped.is_empty() && r.added == [r.n])));
            }
        }
    }

    out.push(String::new());
    out.push("== Where cond holds and repeat does not (all n of the range)".to_owned());
    let broken: Vec<&Row> = rows.iter().filter(|row| row.cond && !row.repeat).collect();
    out.push(format!("{} boards", broken.len()));
    for row in &broken {
        let parity = if row.odd { "odd" } else { "even" };
        let prime = if row.prev_prime { ", n - 1 prime" } else { "" };
        let again = match row.partner_repeat() {
            Some(true) if row.prev_prime => ", repeats game n - 2",
            _ => "",
        };
        out.push(format!(
            "  {:>5} {:<14} {parity}{prime}{again}; added {:?}, dropped {:?}",
            row.n,
            product(row.n),
            row.added,
            row.dropped
        ));
    }
    for (label, odd) in [("odd", true), ("even", false)] {
        out.push(String::new());
        out.push(format!("== The first {LISTED} {label} n where repeat holds and cond does not"));
        let list: Vec<String> = (rows.iter())
            .filter(|row| row.odd == odd && row.repeat && !row.cond)
            .take(LISTED)
            .map(|row| format!("{} ({})", row.n, product(row.n)))
            .collect();
        out.push(list.join(", "));
    }

    out.push(String::new());
    match chain {
        Ok(chain) => chain_rows(rows, chain, &mut out),
        Err(e) => out.push(format!("== Against the committed chain: not read ({e})")),
    }
    out.join("\n")
}

// ---------------------------------------------------------------------------------------------------------
// Running

/// Where the lines go.
enum Sink {
    Stdout,
    File(File),
}

impl Sink {
    /// Writes `line` and a newline, and flushes.
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        match self {
            Sink::Stdout => {
                let mut out = io::stdout().lock();
                writeln!(out, "{line}")?;
                out.flush()
            }
            Sink::File(file) => {
                file.write_all(format!("{line}\n").as_bytes())?;
                file.flush()
            }
        }
    }
}

/// The lines of the file `path` that hold a board of `from..=to`, by n (a missing file has none).  A last line that
/// lacks its newline was cut short by an interruption; with `repair` it is cut off the file, and it is ignored in any
/// case.
fn read_lines(path: &Path, from: u32, to: u32, repair: bool) -> Result<BTreeMap<u32, Json>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let complete = text.rfind('\n').map_or(0, |at| at + 1);
    if repair && complete < text.len() {
        let file = OpenOptions::new().write(true).open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        file.set_len(complete as u64).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let mut lines = BTreeMap::new();
    for (i, line) in text[..complete].lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let line = Json::parse(line).map_err(|e| format!("{} line {}: {e}", path.display(), i + 1))?;
        let n =
            line.get("n").and_then(Json::as_u32).ok_or_else(|| format!("{} line {}: no n", path.display(), i + 1))?;
        if (from..=to).contains(&n) {
            lines.insert(n, line);
        }
    }
    Ok(lines)
}

/// Builds the lines of the boards `todo` (ascending) on `threads` threads, in runs of consecutive boards, and writes
/// them to `sink` in that order, each as soon as it and all before it are done; returns the lines if `keep`.
fn analyze(todo: &[u32], threads: usize, sink: &mut Sink, keep: bool) -> Result<Vec<Json>, String> {
    let (next, stop) = (AtomicUsize::new(0), AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel::<(usize, Json)>();
    let start = Instant::now();
    let mut kept = Vec::new();
    std::thread::scope(|scope| -> Result<(), String> {
        for _ in 0..threads.min(todo.len()) {
            let sender = sender.clone();
            let (next, stop) = (&next, &stop);
            scope.spawn(move || {
                let mut cache = Cache::default();
                while !stop.load(Ordering::Relaxed) {
                    let first = next.fetch_add(RUN, Ordering::Relaxed);
                    for at in first..todo.len().min(first + RUN) {
                        if sender.send((at, timed_line(todo[at], &mut cache))).is_err() {
                            return;
                        }
                    }
                    if first >= todo.len() {
                        break;
                    }
                }
            });
        }
        drop(sender);

        // The lines come back in any order; write the ones that make a prefix of the work.
        let mut waiting: BTreeMap<usize, Json> = BTreeMap::new();
        let mut written = 0;
        for (at, line) in receiver {
            waiting.insert(at, line);
            while let Some(line) = waiting.remove(&written) {
                if let Err(e) = sink.write_line(&line.to_string()) {
                    stop.store(true, Ordering::Relaxed);
                    return Err(format!("writing a line: {e}"));
                }
                written += 1;
                if written % 100 == 0 || written == todo.len() {
                    eprintln!(
                        "{written} of {} boards, last n={}, {:.0} s",
                        todo.len(),
                        todo[written - 1],
                        start.elapsed().as_secs_f64()
                    );
                }
                if keep {
                    kept.push(line);
                }
            }
        }
        Ok(())
    })?;
    Ok(kept)
}

/// Runs the boards of `args`, then prints the summary.
fn run(args: &Args) -> Result<(), String> {
    let to_stdout = args.out.is_none();
    let done: BTreeSet<u32> = match &args.out {
        Some(path) => read_lines(path, args.from, args.to, true)?.into_keys().collect(),
        None => BTreeSet::new(),
    };
    let todo: Vec<u32> = (args.from..=args.to).filter(|n| !done.contains(n)).collect();
    if !done.is_empty() {
        eprintln!("{} boards of {}..={} are in the file already; {} to do", done.len(), args.from, args.to, todo.len());
    }
    let mut sink = match &args.out {
        Some(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            Sink::File(file)
        }
        None => Sink::Stdout,
    };
    let in_memory = analyze(&todo, args.threads, &mut sink, to_stdout)?;

    let lines: Vec<Json> = match &args.out {
        Some(path) => read_lines(path, args.from, args.to, false)?.into_values().collect(),
        None => in_memory,
    };
    let rows: Vec<Row> =
        lines.iter().map(|line| Row::of(line).ok_or("a line lacks a field")).collect::<Result<_, _>>()?;
    let text = summary(&rows, args.from, args.to, &load_chain(&args.chain));
    if to_stdout {
        eprintln!("{text}");
    } else {
        println!("{text}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&args) {
        Ok(args) => args,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
