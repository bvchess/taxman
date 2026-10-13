//! Results files: what a result line records about where its game came from, and the checks a results file
//! passes before anything builds on it.
//!
//! # Provenance
//!
//! A result line ([`GameResult::to_json`], schema [`SCHEMA`]) says how the game's optimality is established (its
//! [`Proof`]), what the proof rests on that is not proven (its *taint*), which game of `n − 1` it was built from
//! (`prev`), and which build made it (`build`, see [`build_json`]).
//!
//! * `trivial`: the board has no promotion candidates, so the game with no promotions is the only one.
//! * `exhausted`: the search explored its whole tree.
//! * `cap`: the search reached `n + score(n − 1)`.  That is a ceiling if the game of `n − 1` is optimal:
//!   `opt(n) ≤ n + opt(n − 1)`, since deleting the move `n` from an optimal game of `n` leaves a game of `n − 1`
//!   (`n` divides nothing else on the board, so no other move changes).
//! * `prime_formula`: a prime `n` plays the game of `n − 1` with its first move replaced by `n`.  That is optimal
//!   if the game of `n − 1` is (proved in `sacrifice.rs`, "Prime `n`").
//! * `reuse_prev`: the tactic of that name accepted the game; a conjecture.
//!
//! `cap`, `prime_formula` and `reuse_prev` rest on the game of `n − 1` ([`Proof::rests_on_prev`]).  The *taint*
//! is the set of unproven things a proof rests on: the unproven tactics the game used itself ([`own_taint`]),
//! and, when the proof rests on the game of `n − 1`, that game's taint, and so on down the chain.  A game taken
//! from `optimal.json` has the taint [`OPTIMAL_JSON`]: the Java made that file with `reuse_prev` and
//! `revise_freedoms` on and records nothing about which games used them.  An empty taint means proven within the
//! frame model, given that the solver is right.
//!
//! Lines without a `schema` field were written before there was provenance (schema 1).  Their proof and taint
//! are derived from the fields they have, by the same rules: `reached_cap` is "the promotions sum to `setup.cap`"
//! and the predecessor is the one `prev_source` names.
//!
//! # Checks
//!
//! [`read_results`] checks every completed line, in order of `n`, and fails naming the line:
//!
//! * the game replays legally to its recorded score, is over (no legal move left) and opens with the largest
//!   prime ([`replay_finished`]); its promotions are among its moves;
//! * against the game of `n − 1` (from the file, else from `optimal.json`):
//!   `score(n − 1) ≤ score(n) ≤ n + score(n − 1)`.  Both are theorems for optimal games, so a violation proves one
//!   of the two games is not optimal;
//! * the arithmetic of the line: a searched game scores `in_the_bag` plus its promotions; the cap is
//!   `n + score(n − 1) − in_the_bag` and the promotions do not exceed it; a game proven by the cap reached it; a
//!   prime game scores `n + score(n − 1) − P`, `P` the largest prime below `n`;
//! * the predecessor the line names is the game the file (or `optimal.json`) holds for `n − 1`: same score,
//!   same moves (by digest, [`moves_digest`]), same proof and taint;
//! * the recorded proof and taint are the ones derived from the line and its predecessor;
//! * no board size has two completed lines.
//!
//! # Torn lines
//!
//! Lines are appended one at a time and synced before the next, so a crash can damage only the last one, and
//! power loss can leave garbage in it before its newline.  The last non-blank line is therefore dropped as torn
//! if it does not parse as JSON, whether or not it ends in a newline.  A last line that parses but is not a
//! valid result, and any bad line before the last, is an error.

use crate::game::{largest_prime_at_most, replay_finished, Optimal};
use crate::json::Json;
use crate::solver::{GameResult, Method, PrevSource, MAX_N};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The version of the result-line format.  1 (no `schema` field) is the format before provenance.
pub const SCHEMA: u32 = 2;

/// The taint of a game taken from `optimal.json`.
pub const OPTIMAL_JSON: &str = "optimal.json";

/// Unproven things a game's optimality rests on: names of unproven tactics, and [`OPTIMAL_JSON`].
pub type Taint = BTreeSet<String>;

/// How a completed game's optimality is established (see the module documentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Proof {
    Trivial,
    PrimeFormula,
    Cap,
    Exhausted,
    ReusePrev,
}

impl Proof {
    pub const ALL: [Proof; 5] = [Proof::Trivial, Proof::PrimeFormula, Proof::Cap, Proof::Exhausted, Proof::ReusePrev];

    pub fn as_str(self) -> &'static str {
        match self {
            Proof::Trivial => "trivial",
            Proof::PrimeFormula => "prime_formula",
            Proof::Cap => "cap",
            Proof::Exhausted => "exhausted",
            Proof::ReusePrev => "reuse_prev",
        }
    }

    pub fn parse(name: &str) -> Option<Proof> {
        Proof::ALL.into_iter().find(|proof| proof.as_str() == name)
    }

    /// The proof of a game solved by `method`; `None` for a search that a limit stopped.
    pub fn of(method: Method, completed: bool, reached_cap: bool) -> Option<Proof> {
        match method {
            Method::Trivial => Some(Proof::Trivial),
            Method::Prime => Some(Proof::PrimeFormula),
            Method::Reuse => Some(Proof::ReusePrev),
            Method::Search if !completed => None,
            Method::Search if reached_cap => Some(Proof::Cap),
            Method::Search => Some(Proof::Exhausted),
        }
    }

    /// Whether the proof holds only if the game of `n − 1` it was built from is optimal.
    pub fn rests_on_prev(self) -> bool {
        matches!(self, Proof::PrimeFormula | Proof::Cap | Proof::ReusePrev)
    }
}

/// The unproven tactics a game used itself: `reuse_prev` if it derived the game; for a searched game,
/// `revise_freedoms` if frame 0's freedoms came from it (`revise` is `setup.revise`: `GreedyIsMax`, `Searched`,
/// or `FromCheckpoint`, which does not say which; counted even when the revision left the number unchanged), and
/// `legacy_bound` if it was on.
pub fn own_taint(method: Method, revise: Option<&str>, legacy_bound: bool) -> Taint {
    let mut taint = Taint::new();
    if method == Method::Reuse {
        taint.insert("reuse_prev".to_owned());
    }
    if method == Method::Search {
        let revised = ["GreedyIsMax", "Searched", "FromCheckpoint"];
        if revise.is_some_and(|revise| revised.iter().any(|kind| revise.starts_with(kind))) {
            taint.insert("revise_freedoms".to_owned());
        }
        if legacy_bound {
            taint.insert("legacy_bound".to_owned());
        }
    }
    taint
}

/// A game's taint: its own, and that of the game of `n − 1` if its proof rests on it.
pub fn taint_of(own: Taint, proof: Proof, prev: Option<&Taint>) -> Taint {
    let mut taint = own;
    if proof.rests_on_prev() {
        taint.extend(prev.into_iter().flatten().cloned());
    }
    taint
}

pub fn taint_json(taint: &Taint) -> Json {
    Json::array(taint.iter().map(String::as_str))
}

/// A digest of a game's moves: the 64-bit FNV-1a hash of the moves written in decimal and separated by commas
/// (`"997,961,713"`), as 16 hexadecimal digits.
pub fn moves_digest(moves: &[u32]) -> String {
    let text = moves.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    format!("{hash:016x}")
}

/// The build this binary comes from (set by `build.rs`): `{"commit": hash, "dirty": bool}`, with `null` for
/// what is not known (no git checkout).  `dirty` means the sources differed from the commit.
pub fn build_json() -> Json {
    let commit = env!("TAXMAN_GIT_COMMIT");
    let dirty = match env!("TAXMAN_GIT_DIRTY") {
        "true" => Json::Bool(true),
        "false" => Json::Bool(false),
        _ => Json::Null,
    };
    Json::object([("commit", if commit.is_empty() { Json::Null } else { Json::from(commit) }), ("dirty", dirty)])
}

// ---------------------------------------------------------------------------------------------------------
// Reading and checking

/// A completed game of a results file, checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredGame {
    pub n: u32,
    /// Its line in the file, counting from 1.
    pub line: usize,
    pub score: u64,
    pub moves: Vec<u32>,
    pub proof: Proof,
    pub taint: Taint,
    /// Which theorem the score reaches, given the game of `n − 1` (if the file or `optimal.json` has it).
    pub certificate: Option<Certificate>,
}

/// A score that equals a proven ceiling, given that the game of `n − 1` is optimal: this certifies the game
/// whatever found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Certificate {
    /// `score(n) = n + score(n − 1)`, the ceiling `opt(n) ≤ n + opt(n − 1)`.
    NPlusPrev,
    /// A prime `n` with `score(n) = n + score(n − 1) − P`, `P` the largest prime below `n` (`sacrifice.rs`).
    PrimeFormula,
}

impl Certificate {
    pub fn as_str(self) -> &'static str {
        match self {
            Certificate::NPlusPrev => "n + score(n-1)",
            Certificate::PrimeFormula => "prime formula",
        }
    }

    /// The ceiling that a game of `n` scoring `score` reaches, given that the game of `n − 1` scores `before`.
    pub fn of(n: u32, score: u64, before: u64) -> Option<Certificate> {
        let ceiling = before + u64::from(n);
        if score == ceiling {
            Some(Certificate::NPlusPrev)
        } else if largest_prime_at_most(n) == Some(n) && score + replaced_by_prime(n) == ceiling {
            Some(Certificate::PrimeFormula)
        } else {
            None
        }
    }
}

/// What a prime `n` replaces in the game of `n − 1`: its first move, the largest prime below `n` (none for 2).
fn replaced_by_prime(n: u32) -> u64 {
    u64::from(n.checked_sub(1).and_then(largest_prime_at_most).unwrap_or(0))
}

/// What [`read_results`] found in a results file.
#[derive(Clone, Debug, Default)]
pub struct Contents {
    pub games: BTreeMap<u32, StoredGame>,
    /// For each board size whose search was stopped by a limit and not yet completed: how many invocations its
    /// incomplete lines record.
    pub runs: BTreeMap<u32, u32>,
    pub incomplete_lines: usize,
    /// Lines up to the end of the last one kept (blank ones included).
    pub lines: usize,
    /// Bytes up to the end of the last line kept.
    pub valid_len: usize,
    /// The last line was torn and is not counted.
    pub torn: bool,
}

/// The predecessor a line names, and what a schema-2 line records about it.
#[derive(Clone, Debug)]
struct Named {
    source: PrevSource,
    record: Option<PrevRecord>,
}

#[derive(Clone, Debug)]
struct PrevRecord {
    n: u32,
    score: u64,
    digest: String,
    proof: Option<Proof>,
    taint: Taint,
}

/// A completed line, parsed.
#[derive(Clone, Debug)]
struct Entry {
    line: usize,
    n: u32,
    score: u64,
    moves: Vec<u32>,
    promotions: Vec<u32>,
    method: Method,
    cap: Option<u64>,
    reached_cap: bool,
    in_the_bag: Option<u64>,
    own: Taint,
    named: Option<Named>,
    /// What a schema-2 line records; `None` for schema 1.
    recorded: Option<(Proof, Taint)>,
}

/// A line of a results file.
enum Line {
    Completed(Box<Entry>),
    /// A search a limit stopped, with the number of invocations it records (schema 2).
    Incomplete {
        n: u32,
        invocations: Option<u32>,
    },
}

/// Field access with errors that name the field.
struct Fields<'a>(&'a Json);

impl<'a> Fields<'a> {
    fn get(&self, name: &str) -> Result<&'a Json, String> {
        self.0.get(name).ok_or_else(|| format!("missing \"{name}\""))
    }

    fn bad(name: &str) -> String {
        format!("bad \"{name}\"")
    }

    fn u32(&self, name: &str) -> Result<u32, String> {
        self.get(name)?.as_u32().ok_or_else(|| Self::bad(name))
    }

    fn u64(&self, name: &str) -> Result<u64, String> {
        self.get(name)?.as_u64().ok_or_else(|| Self::bad(name))
    }

    fn opt_u64(&self, name: &str) -> Result<Option<u64>, String> {
        match self.get(name)? {
            Json::Null => Ok(None),
            value => value.as_u64().map(Some).ok_or_else(|| Self::bad(name)),
        }
    }

    fn bool(&self, name: &str) -> Result<bool, String> {
        self.get(name)?.as_bool().ok_or_else(|| Self::bad(name))
    }

    fn str(&self, name: &str) -> Result<&'a str, String> {
        self.get(name)?.as_str().ok_or_else(|| Self::bad(name))
    }

    fn u32s(&self, name: &str) -> Result<Vec<u32>, String> {
        self.get(name)?.as_u32_vec().ok_or_else(|| Self::bad(name))
    }

    fn taint(&self, name: &str) -> Result<Taint, String> {
        let items = self.get(name)?.as_arr().ok_or_else(|| Self::bad(name))?;
        items.iter().map(|item| item.as_str().map(str::to_owned).ok_or_else(|| Self::bad(name))).collect()
    }

    fn proof(&self, name: &str) -> Result<Option<Proof>, String> {
        match self.get(name)? {
            Json::Null => Ok(None),
            value => value.as_str().and_then(Proof::parse).map(Some).ok_or_else(|| Self::bad(name)),
        }
    }
}

fn parse_source(text: &str) -> Option<PrevSource> {
    [PrevSource::Run, PrevSource::Results, PrevSource::Optimal].into_iter().find(|s| s.as_str() == text)
}

/// Parses a line that is valid JSON.
fn parse_line(json: &Json, line: usize) -> Result<Line, String> {
    let f = Fields(json);
    let schema = match json.get("schema") {
        None => 1,
        Some(value) => value.as_u32().ok_or("bad \"schema\"")?,
    };
    if schema > SCHEMA {
        return Err(format!("schema {schema} is newer than this build reads ({SCHEMA})"));
    }
    let n = f.u32("n")?;
    if !(1..=MAX_N).contains(&n) {
        return Err(format!("n={n} is not a board size from 1 to {MAX_N}"));
    }
    if !f.bool("completed")? {
        let invocations = if schema >= 2 { Some(f.u32("invocations")?) } else { None };
        return Ok(Line::Incomplete { n, invocations });
    }
    let method = Method::parse(f.str("method")?).ok_or("bad \"method\"")?;
    let promotions = f.u32s("promotions")?;
    let setup = f.get("setup")?;
    let (cap, revise) = match setup {
        Json::Null => (None, None),
        setup => {
            let s = Fields(setup);
            (s.opt_u64("cap")?, Some(s.str("revise")?))
        }
    };
    let switches = Fields(f.get("tactic_switches")?);
    let legacy_bound = switches.bool("legacy_bound")?;
    let source = match f.get("prev_source")? {
        Json::Null => None,
        value => Some(value.as_str().and_then(parse_source).ok_or("bad \"prev_source\"")?),
    };
    let sum: u64 = promotions.iter().map(|&p| u64::from(p)).sum();
    let mut entry = Entry {
        line,
        n,
        score: f.u64("score")?,
        moves: f.u32s("moves")?,
        promotions,
        method,
        cap,
        reached_cap: cap.is_some_and(|cap| sum >= cap),
        in_the_bag: None,
        own: own_taint(method, revise, legacy_bound),
        named: source.map(|source| Named { source, record: None }),
        recorded: None,
    };
    if schema >= 2 {
        entry.reached_cap = f.bool("reached_cap")?;
        entry.in_the_bag = f.opt_u64("in_the_bag")?;
        let proof = f.proof("proof")?.ok_or("a completed game needs a \"proof\"")?;
        entry.recorded = Some((proof, f.taint("taint")?));
        match (f.get("prev")?, &mut entry.named) {
            (Json::Null, None) => {}
            (prev @ Json::Obj(_), Some(named)) => {
                let p = Fields(prev);
                if parse_source(p.str("source")?) != Some(named.source) {
                    return Err("\"prev.source\" differs from \"prev_source\"".into());
                }
                named.record = Some(PrevRecord {
                    n: p.u32("n")?,
                    score: p.u64("score")?,
                    digest: p.str("moves_digest")?.to_owned(),
                    proof: p.proof("proof")?,
                    taint: p.taint("taint")?,
                });
            }
            _ => return Err("\"prev\" and \"prev_source\" disagree".into()),
        }
    }
    Ok(Line::Completed(Box::new(entry)))
}

/// Reads and checks the text of a results file (see the module documentation).  Errors name the line.
pub fn read_results(text: &str, optimal: Option<&Optimal>) -> Result<Contents, String> {
    let mut contents = Contents::default();
    let mut entries: Vec<Entry> = Vec::new();
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let last = lines.iter().rposition(|line| !line.trim().is_empty());
    let mut offset = 0;
    for (i, raw) in lines.iter().enumerate() {
        let number = i + 1;
        if !raw.trim().is_empty() {
            let json = match Json::parse(raw) {
                Ok(json) => json,
                Err(_) if Some(i) == last => {
                    contents.torn = true;
                    break;
                }
                Err(e) => return Err(format!("line {number}: {e}")),
            };
            match parse_line(&json, number).map_err(|e| format!("line {number}: {e}"))? {
                Line::Completed(entry) => entries.push(*entry),
                Line::Incomplete { n, invocations } => {
                    contents.incomplete_lines += 1;
                    let runs = contents.runs.entry(n).or_insert(0);
                    *runs = invocations.unwrap_or(*runs + 1);
                }
            }
        }
        offset += raw.len();
        contents.lines = number;
        contents.valid_len = offset;
    }

    entries.sort_by_key(|e| (e.n, e.line));
    for pair in entries.windows(2) {
        if pair[0].n == pair[1].n {
            return Err(format!(
                "lines {} and {} both hold a completed game of n={}",
                pair[0].line, pair[1].line, pair[0].n
            ));
        }
    }
    for entry in entries {
        contents.runs.remove(&entry.n);
        let game = check_entry(entry, &contents.games, optimal)?;
        contents.games.insert(game.n, game);
    }
    Ok(contents)
}

/// The game of `n − 1` that a line was built from, as far as it is known.  For a game of `optimal.json` when that
/// file is not at hand, the line's own record (if any) stands in for its score, and its moves are not known.
struct Pred<'a> {
    score: Option<u64>,
    moves: Option<&'a [u32]>,
    /// The file's game; `None` for `optimal.json`'s.
    stored: Option<&'a StoredGame>,
    taint: Taint,
    whence: String,
}

/// Checks one completed line against the games of the file below it (see the module documentation).
fn check_entry(e: Entry, games: &BTreeMap<u32, StoredGame>, optimal: Option<&Optimal>) -> Result<StoredGame, String> {
    let n = e.n;
    let fail = |message: String| format!("line {} (n={n}): {message}", e.line);
    let replayed = replay_finished(n, &e.moves).map_err(fail)?;
    if replayed != e.score {
        return Err(fail(format!("the moves score {replayed}, not the recorded {}", e.score)));
    }
    if !e.promotions.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(fail("the promotions are not in ascending order".into()));
    }
    let moves: BTreeSet<u32> = e.moves.iter().copied().collect();
    if let Some(p) = e.promotions.iter().find(|p| !moves.contains(p)) {
        return Err(fail(format!("promotion {p} is not a move")));
    }

    let in_file = n.checked_sub(1).and_then(|before| games.get(&before)).map(|g| Pred {
        score: Some(g.score),
        moves: Some(&g.moves),
        stored: Some(g),
        taint: g.taint.clone(),
        whence: format!("line {}", g.line),
    });
    let in_optimal = n.checked_sub(1).and_then(|before| {
        let optimal = optimal?;
        Some(Pred {
            score: Some(optimal.score(before)?),
            moves: Some(optimal.moves(before)?),
            stored: None,
            taint: Taint::from([OPTIMAL_JSON.to_owned()]),
            whence: OPTIMAL_JSON.to_owned(),
        })
    });

    // The chain, against whichever game of n − 1 there is: both bounds are theorems about optimal games.
    if let Some((before, whence)) = in_file.as_ref().or(in_optimal.as_ref()).and_then(|p| Some((p.score?, &p.whence))) {
        let ceiling = before + u64::from(n);
        if e.score < before {
            return Err(fail(format!(
                "scores {}, less than the {before} of n − 1 ({whence}); a game of n − 1 is a game of n, so one of \
                 the two is not optimal",
                e.score
            )));
        }
        if e.score > ceiling {
            return Err(fail(format!(
                "scores {}, more than n + score(n − 1) = {ceiling} (n − 1 from {whence}), which proves that the game \
                 of n − 1 is not optimal",
                e.score
            )));
        }
    }

    // The game the line was built from, and the line's record of it.
    let pred = match &e.named {
        None => None,
        Some(named) => {
            let pred = match named.source {
                PrevSource::Run | PrevSource::Results => in_file.ok_or_else(|| {
                    fail("it was built from the game of n − 1 in this file, which the file does not hold".to_owned())
                })?,
                PrevSource::Optimal => in_optimal.unwrap_or_else(|| Pred {
                    score: named.record.as_ref().map(|r| r.score),
                    moves: None,
                    stored: None,
                    taint: Taint::from([OPTIMAL_JSON.to_owned()]),
                    whence: OPTIMAL_JSON.to_owned(),
                }),
            };
            if let Some(record) = &named.record {
                let same = record.n + 1 == n
                    && pred.score == Some(record.score)
                    && pred.moves.is_none_or(|moves| moves_digest(moves) == record.digest)
                    && record.proof == pred.stored.map(|g| g.proof)
                    && record.taint == pred.taint;
                if !same {
                    return Err(fail(format!("its record of the game of n − 1 does not match {}", pred.whence)));
                }
            }
            Some(pred)
        }
    };

    // The line's own arithmetic.
    let sum: u64 = e.promotions.iter().map(|&p| u64::from(p)).sum();
    let prev_score = pred.as_ref().and_then(|p| p.score);
    if let Some(bag) = e.in_the_bag {
        if e.score != bag + sum {
            return Err(fail(format!("scores {}, not in_the_bag {bag} + the promotions' {sum}", e.score)));
        }
    }
    if let Some(cap) = e.cap {
        if sum > cap {
            return Err(fail(format!(
                "the promotions' {sum} exceed the cap {cap}, so the game of n − 1 is not optimal"
            )));
        }
        // in_the_bag is the score less the promotions, and the cap is n + score(n − 1) − in_the_bag.
        if prev_score.is_some_and(|before| before + u64::from(n) + sum != cap + e.score) {
            return Err(fail(format!("the cap {cap} is not n + score(n − 1) − in_the_bag")));
        }
    }
    if e.reached_cap && e.cap != Some(sum) {
        return Err(fail(format!(
            "it records reaching the cap, but its promotions' {sum} are not the cap {:?}",
            e.cap
        )));
    }
    let replaced = replaced_by_prime(n);
    if e.method == Method::Prime && prev_score.is_some_and(|before| e.score + replaced != before + u64::from(n)) {
        return Err(fail(format!("a prime game scores n + score(n − 1) − {replaced}, and this one does not")));
    }

    let proof = Proof::of(e.method, true, e.reached_cap).expect("a completed game has a proof");
    if proof.rests_on_prev() && pred.is_none() {
        return Err(fail(format!("its proof ({}) rests on a game of n − 1 that it does not name", proof.as_str())));
    }
    let taint = taint_of(e.own, proof, pred.as_ref().map(|p| &p.taint));
    if let Some((recorded_proof, recorded_taint)) = &e.recorded {
        if *recorded_proof != proof {
            return Err(fail(format!(
                "it records the proof {}, but it is {}",
                recorded_proof.as_str(),
                proof.as_str()
            )));
        }
        if *recorded_taint != taint {
            return Err(fail(format!("it records the taint {recorded_taint:?}, but it is {taint:?}")));
        }
    }
    let certificate = prev_score.and_then(|before| Certificate::of(n, e.score, before));
    Ok(StoredGame { n, line: e.line, score: e.score, moves: e.moves, proof, taint, certificate })
}

// ---------------------------------------------------------------------------------------------------------
// The results file of a run

/// A results file being appended to: one JSON object per line, as games finish.  Only completed games count as
/// done.  Opening it checks it ([`read_results`]).
pub struct ResultsFile {
    path: PathBuf,
    file: File,
    contents: Contents,
}

impl ResultsFile {
    /// Opens (creating if need be) the file and checks it, with `optimal` for the games of `n − 1` that came
    /// from it.  A torn last line is cut off.
    pub fn open(path: &Path, optimal: Option<&Optimal>) -> Result<ResultsFile, String> {
        let io_error = |e: std::io::Error| format!("{}: {e}", path.display());
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(io_error(e)),
        };
        let contents = read_results(&text, optimal).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut file = OpenOptions::new().create(true).append(true).open(path).map_err(io_error)?;
        if contents.torn {
            file.set_len(contents.valid_len as u64).map_err(io_error)?;
        }
        if contents.valid_len > 0 && !text[..contents.valid_len].ends_with('\n') {
            file.write_all(b"\n").map_err(io_error)?;
        }
        Ok(ResultsFile { path: path.to_owned(), file, contents })
    }

    /// The completed game for board size `n`, if the file has one.
    pub fn completed(&self, n: u32) -> Option<&StoredGame> {
        self.contents.games.get(&n)
    }

    /// The invocations the file records for an unfinished search of `n` (0 if none).
    pub fn earlier_runs(&self, n: u32) -> u32 {
        self.contents.runs.get(&n).copied().unwrap_or(0)
    }

    /// Appends the result and makes sure it reaches the disk.
    pub fn append(&mut self, result: &GameResult) -> Result<(), String> {
        append_line(&mut self.file, &result.to_json()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.contents.lines += 1;
        match result.proof {
            Some(proof) => {
                let game = StoredGame {
                    n: result.n,
                    line: self.contents.lines,
                    score: result.score,
                    moves: result.moves.clone(),
                    proof,
                    taint: result.taint.clone(),
                    certificate: result.prev.as_ref().and_then(|p| Certificate::of(result.n, result.score, p.score)),
                };
                self.contents.games.insert(result.n, game);
                self.contents.runs.remove(&result.n);
            }
            None => {
                self.contents.runs.insert(result.n, result.invocations);
            }
        }
        Ok(())
    }

    /// Records a game that failed a check after it was found, with the reason, in [`rejected_path`].  Returns
    /// that path.
    pub fn reject(&self, result: &GameResult, reason: &str) -> Result<PathBuf, String> {
        let path = rejected_path(&self.path);
        let mut line = result.to_json();
        if let Json::Obj(pairs) = &mut line {
            pairs.push(("rejected".to_owned(), Json::from(reason)));
        }
        let written =
            OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| append_line(&mut f, &line));
        written.map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }
}

/// Where the games that failed a check go: `<results>.rejected.jsonl`, with a `.jsonl` ending of the results
/// file dropped (`default.jsonl` → `default.rejected.jsonl`).
pub fn rejected_path(results: &Path) -> PathBuf {
    let name = results.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = name.strip_suffix(".jsonl").unwrap_or(&name);
    results.with_file_name(format!("{stem}.rejected.jsonl"))
}

/// Writes one line and syncs it.
fn append_line(file: &mut File, line: &Json) -> std::io::Result<()> {
    file.write_all(format!("{line}\n").as_bytes())?;
    file.sync_data()
}

// ---------------------------------------------------------------------------------------------------------
// Checking files on their own

/// What [`verify_files`] found in one file.
#[derive(Clone, Debug)]
pub struct FileReport {
    pub path: PathBuf,
    pub contents: Contents,
}

/// What [`verify_files`] found.
#[derive(Clone, Debug)]
pub struct Report {
    pub files: Vec<FileReport>,
    /// Board sizes completed in more than one file (all of them with equal scores).
    pub shared: usize,
}

/// Checks results files without changing them: each as [`read_results`] does, and that files agree on the
/// score of every board size they share.
pub fn verify_files(paths: &[PathBuf], optimal: Option<&Optimal>) -> Result<Report, String> {
    let mut files = Vec::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let contents = read_results(&text, optimal).map_err(|e| format!("{}: {e}", path.display()))?;
        files.push(FileReport { path: path.clone(), contents });
    }
    let mut seen: BTreeMap<u32, (&Path, &StoredGame)> = BTreeMap::new();
    let mut shared = BTreeSet::new();
    for file in &files {
        for (&n, game) in &file.contents.games {
            if let Some((other, theirs)) = seen.get(&n) {
                shared.insert(n);
                if theirs.score != game.score {
                    return Err(format!(
                        "n={n}: {} line {} scores {}, but {} line {} scores {}",
                        other.display(),
                        theirs.line,
                        theirs.score,
                        file.path.display(),
                        game.line,
                        game.score
                    ));
                }
            } else {
                seen.insert(n, (&file.path, game));
            }
        }
    }
    Ok(Report { files, shared: shared.len() })
}

/// `name count, name count` for the counts that are not zero.
fn counts<K: Ord>(items: impl IntoIterator<Item = K>, name: impl Fn(&K) -> String) -> String {
    let mut by: BTreeMap<K, usize> = BTreeMap::new();
    for item in items {
        *by.entry(item).or_default() += 1;
    }
    let parts: Vec<String> = by.iter().map(|(k, count)| format!("{} {count}", name(k))).collect();
    if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join(", ")
    }
}

impl fmt::Display for FileReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let games = &self.contents.games;
        let (lo, hi) = (games.keys().next().copied().unwrap_or(0), games.keys().last().copied().unwrap_or(0));
        let gaps = (lo..=hi).filter(|n| !games.contains_key(n)).count();
        writeln!(
            f,
            "{}: {} completed games, n = {lo}..={hi} ({gaps} missing), {} incomplete lines{}",
            self.path.display(),
            games.len(),
            self.contents.incomplete_lines,
            if self.contents.torn { ", a torn last line (ignored)" } else { "" }
        )?;
        writeln!(f, "  proof: {}", counts(games.values().map(|g| g.proof), |p| p.as_str().to_owned()))?;
        let clean = games.values().filter(|g| g.taint.is_empty()).count();
        write!(f, "  taint: {clean} of {} games rest on nothing unproven", games.len())?;
        if clean < games.len() {
            let labels = counts(games.values().flat_map(|g| g.taint.iter().cloned()), String::clone);
            write!(f, "; the others on {labels}")?;
        }
        let certificates = counts(games.values().filter_map(|g| g.certificate), |c| c.as_str().to_owned());
        write!(f, "\n  scores at a proven ceiling, given the game of n-1: {certificates}")
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for file in &self.files {
            writeln!(f, "{file}")?;
        }
        write!(f, "{} board sizes are in more than one file, with equal scores in all", self.shared)
    }
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{solve_range, SolveOptions};

    /// The lines `taxman solve 1-12` writes, as JSON objects.
    fn lines_1_to_12() -> Vec<Json> {
        let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
        solve_range(1, 12, &opts, None).unwrap().iter().map(GameResult::to_json).collect()
    }

    fn text(lines: &[Json]) -> String {
        lines.iter().map(|line| format!("{line}\n")).collect()
    }

    /// `line` with `key` set to `value`.
    fn with(line: &Json, key: &str, value: Json) -> Json {
        let Json::Obj(pairs) = line else { panic!("not an object") };
        let pairs = pairs.iter().map(|(k, v)| (k.clone(), if k == key { value.clone() } else { v.clone() }));
        Json::Obj(pairs.collect())
    }

    #[test]
    fn the_digest_is_fnv1a_of_the_moves_in_decimal() {
        assert_eq!(moves_digest(&[]), "cbf29ce484222325");
        // As computed by tools/check_results.py, which shares no code with this.
        assert_eq!(moves_digest(&[5, 4, 6]), "c27bf6b65c1b8616");
        assert_eq!(moves_digest(&[79]), "07f20f07b4b4be5d");
    }

    #[test]
    fn proofs_and_taints() {
        assert_eq!(Proof::of(Method::Search, false, false), None);
        assert_eq!(Proof::of(Method::Search, true, true), Some(Proof::Cap));
        assert_eq!(Proof::of(Method::Search, true, false), Some(Proof::Exhausted));
        assert_eq!(Proof::of(Method::Reuse, true, false), Some(Proof::ReusePrev));
        assert!(Proof::ALL.iter().all(|&p| Proof::parse(p.as_str()) == Some(p)));
        let names = |taint: Taint| taint.into_iter().collect::<Vec<_>>();
        assert_eq!(names(own_taint(Method::Reuse, None, false)), ["reuse_prev"]);
        assert_eq!(
            names(own_taint(Method::Search, Some("Searched { size: 3, nodes: 9 }"), true)),
            ["legacy_bound", "revise_freedoms"]
        );
        for revise in ["Off", "NotNeeded", "Unchanged"] {
            assert!(own_taint(Method::Search, Some(revise), false).is_empty(), "{revise}");
        }
        assert!(own_taint(Method::Prime, None, true).is_empty());
        // Inherited only by a proof that rests on the game of n − 1.
        let prev = Taint::from([OPTIMAL_JSON.to_owned()]);
        assert_eq!(names(taint_of(Taint::new(), Proof::Cap, Some(&prev))), [OPTIMAL_JSON]);
        assert!(taint_of(Taint::new(), Proof::Exhausted, Some(&prev)).is_empty());
    }

    #[test]
    fn a_file_reads_back_with_its_proofs() {
        let lines = lines_1_to_12();
        let contents = read_results(&text(&lines), None).unwrap();
        assert_eq!(contents.games.keys().copied().collect::<Vec<_>>(), (1..=12).collect::<Vec<_>>());
        assert!(!contents.torn && contents.incomplete_lines == 0 && contents.lines == 12);
        assert_eq!(contents.games[&11].proof, Proof::PrimeFormula);
        assert_eq!(contents.games[&11].certificate, Some(Certificate::PrimeFormula));
        assert_eq!(contents.games[&10].proof, Proof::Trivial);
    }

    #[test]
    fn a_torn_last_line_is_dropped_and_any_other_bad_line_is_an_error() {
        let good = text(&lines_1_to_12());
        for tail in ["{\"n\":13,\"sco", "{\"n\":13,\"sco\n", "\0\0\0\0\0\0\0\0 13,\"score\":5}\n", "garbage\n\n\n"] {
            let contents = read_results(&format!("{good}{tail}"), None).unwrap();
            assert!(contents.torn, "{tail:?}");
            assert_eq!((contents.games.len(), contents.valid_len), (12, good.len()), "{tail:?}");
        }
        // A last line that parses but is not a result, and a bad line before the last, are errors.
        let err = read_results(&format!("{good}{{}}\n"), None).unwrap_err();
        assert!(err.starts_with("line 13: missing \"n\""), "{err}");
        let err = read_results(&format!("garbage\n{good}"), None).unwrap_err();
        assert!(err.starts_with("line 1: JSON syntax error"), "{err}");
        assert!(read_results("", None).unwrap().games.is_empty());
    }

    #[test]
    fn tampered_lines_are_refused_naming_the_line() {
        let lines = lines_1_to_12();
        let refused = |i: usize, line: Json, expected: &str| {
            let mut tampered = lines.clone();
            tampered[i] = line;
            let err = read_results(&text(&tampered), None).unwrap_err();
            assert!(err.starts_with(&format!("line {} ", i + 1)) && err.contains(expected), "{expected}: {err}");
        };
        let score = |i: usize| lines[i].get("score").and_then(Json::as_u64).unwrap();
        refused(9, with(&lines[9], "score", Json::from(score(9) + 5)), "the moves score 40, not the recorded 45");
        // Game 9's moves as game 10's: it is not over on the bigger board.
        let moved =
            with(&with(&lines[9], "moves", lines[8].get("moves").unwrap().clone()), "score", Json::from(score(8)));
        refused(9, moved, "not over");
        let mut prev = lines[10].get("prev").unwrap().clone();
        if let Json::Obj(pairs) = &mut prev {
            pairs.iter_mut().filter(|(k, _)| k == "moves_digest").for_each(|(_, v)| *v = Json::from("0000"));
        }
        refused(10, with(&lines[10], "prev", prev), "record of the game of n − 1 does not match line 10");
        refused(11, with(&lines[11], "taint", Json::array(["reuse_prev"])), "records the taint");
        refused(11, with(&lines[11], "proof", Json::from("cap")), "records the proof cap");
        refused(11, with(&lines[11], "promotions", Json::array([12u32, 8])), "not in ascending order");
        let mut twice = lines.clone();
        twice.push(lines[4].clone());
        assert!(read_results(&text(&twice), None).unwrap_err().contains("lines 5 and 13 both hold"));
    }

    #[test]
    fn the_chain_catches_a_game_of_n_minus_1_that_is_not_optimal() {
        let lines = lines_1_to_12();
        // Game 12 scores 50 (optimal.json's 11 scores 44); an optimal.json that says 11 scores less than 38 is
        // contradicted, and so is one that says 11 scores more than 12.
        let line = text(&lines[11..]);
        for (score, expected) in [(30, "more than n + score(n − 1) = 42"), (51, "less than the 51 of n − 1")] {
            let optimal =
                Optimal::from_json(&Json::parse(&format!(r#"[{{"n":11,"score":{score},"moves":[]}}]"#)).unwrap());
            let err = read_results(&line, Some(&optimal.unwrap())).unwrap_err();
            assert!(err.starts_with("line 1 (n=12)") && err.contains(expected), "{err}");
        }
        // Without a game of 11 the line cannot stand: it names one from this file.
        let err = read_results(&line, None).unwrap_err();
        assert!(err.contains("which the file does not hold"), "{err}");
    }
}
