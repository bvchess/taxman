//! Results files: what a result line records about where its game came from, and the checks a results file
//! passes before anything builds on it.
//!
//! # Provenance
//!
//! A result line ([`GameResult::to_json`], schema [`SCHEMA`], the only one this build reads) says how the game's
//! optimality is established (its [`Proof`]), what the proof takes as given beyond the frame model (its
//! *assumptions*), which game of `n − 1` it was built from (`prev`), and which build made it (`build`, see
//! [`build_json`]).
//!
//! * `trivial`: the board has no promotion candidates, so the game with no promotions is the only one.
//! * `exhausted`: the search explored its whole tree.
//! * `cap`: the search reached `n + score(n − 1)`.  That is a ceiling if the game of `n − 1` is optimal:
//!   `opt(n) ≤ n + opt(n − 1)`, since deleting the move `n` from an optimal game of `n` leaves a game of `n − 1`
//!   (`n` divides nothing else on the board, so no other move changes).
//! * `prime_formula`: a prime `n` plays the game of `n − 1` with its first move replaced by `n`.  That is optimal
//!   if the game of `n − 1` is (proved at `accelerate::prime_shortcut`).
//! * `same_search`: the game's search problem is the one an earlier game `m` of the file posed, and `m`'s proof
//!   settles that problem's best promotion sum (`exhausted`, `cap`, or `same_search` again); so this game's best
//!   promotion sum is the same, and its game is `m`'s promotions played on this board (tactic `same_search`;
//!   DESIGN.md, "Same search", has the argument).
//!
//! `cap` and `prime_formula` rest on the game of `n − 1` ([`Proof::rests_on_prev`]), and `same_search` on the game
//! it repeats.  A proof that rests on another game assumes what that game's proof assumes ([`assumptions_of`]), and
//! so on down the chain; a game's own search assumes nothing, since every tactic of the search is proven or sound.
//! The one assumption a run can bring in is a game of `optimal.json` taken as the game of `n − 1` (`solve
//! --use-prev`): the file records no proofs, so the game is assumed optimal, and the line says so, as
//! [`assumed_optimal`] words it ("the game of 1300 in optimal.json is optimal").  The researcher knows where that
//! comes from: the table's games 1 to 1000 are the Java's, made with two conjectures (`reuse_prev` and
//! `revise_freedoms`, removed from this solver), and its later ones were copied from a chain from n = 1.  A line
//! that assumes nothing is proven within the frame model, given that the solver is right; so is every game of a
//! chain from n = 1.
//!
//! # Checks
//!
//! [`read_results`] checks every completed line, in order of `n`, and fails naming the line:
//!
//! * the game replays legally to its recorded score, is over (no legal move left) and opens with the largest
//!   prime ([`replay_finished`]); its promotions are among its moves;
//! * its score is `optimal.json`'s, where the table ([`Optimal::built_in`], or another a test gives) has the game;
//! * the fields the proof and the assumptions are derived from agree with each other, as the solver writes them: a
//!   searched or repeat game, and only such a game, has a `setup`; every game but a prime one records
//!   `in_the_bag`; only a search reaches its ceiling (`reached_cap`); a prime game's `n` is prime; a trivial game
//!   promotes nothing; a repeat, and only a repeat, records the game it repeats, and its `setup` has no `cap` (it
//!   ran no search to stop);
//! * against the game of `n − 1` (from the file, else from `optimal.json`):
//!   `score(n − 1) ≤ score(n) ≤ n + score(n − 1)`.  Both are theorems for optimal games, so a violation proves one
//!   of the two games is not optimal;
//! * the arithmetic of the line: a searched game scores `in_the_bag` plus its promotions; the ceiling on the
//!   promotion sum (`setup`'s `cap`) is `n + score(n − 1) − in_the_bag` and the promotions do not exceed it; a
//!   game records reaching it exactly when its promotions sum to it, where the search stops (so a proof by `cap`
//!   cannot pass for an exhausted search, which does not rest on the game of `n − 1`); a prime game scores
//!   `n + score(n − 1) − P`, `P` the largest prime below `n`;
//! * the predecessor the line names is the game the file (or `optimal.json`) holds for `n − 1`: same score,
//!   same moves (by digest, [`moves_digest`]), same proof and assumptions;
//! * a repeat: the game `m` it names is a completed game of the file, on an earlier line, whose proof settles its
//!   search ([`Proof::settles_search`]) and whose proof and assumptions the record gives; the search problems of
//!   both games, set up again from their board sizes ([`search_problem`]), are as large as their lines record
//!   (frames, candidates, `max_total`), are equal, and have the digest both lines record; the promotions are `m`'s,
//!   and both games score their own `in_the_bag` (from the boards set up again) plus the recorded promotion sum;
//! * the recorded proof and assumptions are the ones derived from the line and the game its proof rests on;
//! * no board size has two completed lines.
//!
//! The digest a line that is not a repeat records is not checked against its problem: it is how the solver finds
//! an earlier game to repeat, which it then sets up again and compares problem for problem, as the check of the
//! repeat's line does.  Those setups are independent of each other and of the order of the checks, so given several
//! threads ([`read_results`]) they are done first, side by side, and the checks then run in order as on one thread.
//!
//! # Torn lines
//!
//! Lines are appended one at a time and synced before the next, so a crash can damage only the last one, and
//! power loss can leave garbage in it before its newline.  The last non-blank line is therefore dropped as torn
//! if it does not parse as JSON, whether or not it ends in a newline.  A last line that parses but is not a
//! valid result, and any bad line before the last, is an error.
//!
//! # The file of a run, and checking files on their own
//!
//! [`ResultsFile`] is the file a run appends to.  Opening it locks it against other processes (two runs appending to
//! one file would leave two lines for a game) and checks it ([`read_results`]); the completed games in it count as
//! done and are the previous games of the ones after them; each new result is appended and synced as soon as it is
//! done (so a crash loses at most the line being written); and a game that failed a check after it was found is
//! written to [`rejected_path`] instead, with the reason.  [`verify_files`] runs the same checks on files without
//! changing them, and that the files agree on the games they share (`taxman verify-results`).  The Python referee
//! `tools/check_results.py` derives the same proofs and assumptions from the same rules with no code in common, so a
//! mistake in either shows as a disagreement.
//!
//! # Extending the table of optimal games
//!
//! [`extend_optimal`] appends to the table `optimal.json` the games of a checked results file that continue it:
//! from the board size after the table's last, every game that is completed, up to the first board size that is not
//! (`taxman extend-optimal`).  A game that assumes a game of the table optimal assumes nothing the table does not
//! already stand for, so it is appended like any other.  Its scores must agree with the table's wherever the two
//! overlap, so a results file of another game cannot be mistaken for the table's continuation.
//!
//! The fields of a line are described in README.md ("A results line"); [`SCHEMA`] is the version of the format.

use crate::board::search_problem;
use crate::game::{largest_prime_at_most, replay_finished, Optimal};
use crate::json::Json;
use crate::search::{Problem, ProblemSize};
use crate::solver::{parse_digest, GameResult, Method, PrevSource, SameSearch, MAX_N};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// The version of the result-line format, the only one this build reads.
pub const SCHEMA: u32 = 5;

/// What a game's proof takes as given beyond the frame model, each a plain statement ([`assumed_optimal`]); empty
/// for a proof that rests on nothing else.
pub type Assumptions = BTreeSet<String>;

/// What a proof that rests on `optimal.json`'s game of `n` assumes: "the game of `n` in optimal.json is optimal".
pub fn assumed_optimal(n: u32) -> String {
    format!("the game of {n} in optimal.json is optimal")
}

/// How a completed game's optimality is established (see the module documentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Proof {
    Trivial,
    PrimeFormula,
    Cap,
    Exhausted,
    SameSearch,
}

impl Proof {
    pub const ALL: [Proof; 5] = [Proof::Trivial, Proof::PrimeFormula, Proof::Cap, Proof::Exhausted, Proof::SameSearch];

    pub fn as_str(self) -> &'static str {
        match self {
            Proof::Trivial => "trivial",
            Proof::PrimeFormula => "prime_formula",
            Proof::Cap => "cap",
            Proof::Exhausted => "exhausted",
            Proof::SameSearch => "same_search",
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
            Method::Repeat => Some(Proof::SameSearch),
            Method::Search if !completed => None,
            Method::Search if reached_cap => Some(Proof::Cap),
            Method::Search => Some(Proof::Exhausted),
        }
    }

    /// Whether the proof holds only if the game of `n − 1` it was built from is optimal.
    pub fn rests_on_prev(self) -> bool {
        matches!(self, Proof::PrimeFormula | Proof::Cap)
    }

    /// Whether the proof holds only if another game's proof does: the game of `n − 1` ([`rests_on_prev`]), or for
    /// `same_search` the earlier game it repeats.
    ///
    /// [`rests_on_prev`]: Self::rests_on_prev
    pub fn rests_on_another_game(self) -> bool {
        self.rests_on_prev() || self == Proof::SameSearch
    }

    /// Whether the proof settles the best promotion sum of the game's search problem, so that a later game that
    /// poses the same problem can repeat it (tactic `same_search`): an exhausted search directly; a search that
    /// reached its ceiling because every promotion set of the model is a game, which scores at most the ceiling; a
    /// repeat by induction (DESIGN.md, "Same search").  A trivial or prime game settles no search.
    pub fn settles_search(self) -> bool {
        matches!(self, Proof::Exhausted | Proof::Cap | Proof::SameSearch)
    }
}

/// A game's assumptions: those of the game its proof rests on ([`Proof::rests_on_another_game`]: the game of
/// `n − 1`, or the game a repeat repeats), which are `basis`, if the proof rests on one; else none.
pub fn assumptions_of(proof: Proof, basis: Option<&Assumptions>) -> Assumptions {
    match basis {
        Some(basis) if proof.rests_on_another_game() => basis.clone(),
        _ => Assumptions::new(),
    }
}

/// The `assumes` field of a line: an array of the statements.
pub fn assumptions_json(assumes: &Assumptions) -> Json {
    Json::array(assumes.iter().map(String::as_str))
}

/// A digest of a game's moves: the 64-bit FNV-1a hash of the moves written in decimal and separated by commas
/// (`"997,961,713"`), as 16 hexadecimal digits.
pub fn moves_digest(moves: &[u32]) -> String {
    let text = moves.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    format!("{hash:016x}")
}

/// The build this binary comes from, as `build.rs` recorded it when the crate was compiled; `None` for what was
/// not known then (not a git checkout, or no `git`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Build {
    /// The hash of `HEAD`.
    pub commit: Option<&'static str>,
    /// Whether the sources differed from that commit.
    pub dirty: Option<bool>,
}

/// The build this binary comes from.
pub fn build() -> Build {
    let commit = env!("TAXMAN_GIT_COMMIT");
    let dirty = match env!("TAXMAN_GIT_DIRTY") {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };
    Build { commit: (!commit.is_empty()).then_some(commit), dirty }
}

/// [`build`] as the `build` field of a result line: `{"commit": hash, "dirty": bool}`, with `null` for what is not
/// known.
pub fn build_json() -> Json {
    let Build { commit, dirty } = build();
    Json::object([("commit", commit.map_or(Json::Null, Json::from)), ("dirty", dirty.map_or(Json::Null, Json::Bool))])
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
    /// The numbers promoted, ascending.
    pub promotions: Vec<u32>,
    pub proof: Proof,
    pub assumes: Assumptions,
    /// Which theorem the score reaches, given the game of `n − 1` (if the file or `optimal.json` has it).
    pub ceiling_proof: Option<CeilingProof>,
    /// For a game that set a search up (searched or repeat), the size of its search problem as the line records it
    /// (`setup.frames`, `setup.candidates`, `setup.max_total`), which the problem set up again from its board size
    /// ([`search_problem`]) must have.
    pub size: Option<ProblemSize>,
    /// For such a game, the digest of its search problem (`setup.digest`).
    pub digest: Option<u64>,
}

/// A score that equals a proven ceiling, given that the game of `n − 1` is optimal: this proves the game optimal
/// whatever found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CeilingProof {
    /// `score(n) = n + score(n − 1)`, the ceiling `opt(n) ≤ n + opt(n − 1)`.
    NPlusPrev,
    /// A prime `n` with `score(n) = n + score(n − 1) − P`, `P` the largest prime below `n` (proof at
    /// `accelerate::prime_shortcut`).
    PrimeFormula,
}

impl CeilingProof {
    pub fn as_str(self) -> &'static str {
        match self {
            CeilingProof::NPlusPrev => "n + score(n-1)",
            CeilingProof::PrimeFormula => "prime formula",
        }
    }

    /// The ceiling that a game of `n` scoring `score` reaches, given that the game of `n − 1` scores `before`.
    pub fn of(n: u32, score: u64, before: u64) -> Option<CeilingProof> {
        let ceiling = before + u64::from(n);
        if score == ceiling {
            Some(CeilingProof::NPlusPrev)
        } else if largest_prime_at_most(n) == Some(n) && score + replaced_by_prime(n) == ceiling {
            Some(CeilingProof::PrimeFormula)
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

/// The predecessor a line names, and what the line records about it.
#[derive(Clone, Debug)]
struct Named {
    source: PrevSource,
    record: PrevRecord,
}

#[derive(Clone, Debug)]
struct PrevRecord {
    n: u32,
    score: u64,
    digest: String,
    proof: Option<Proof>,
    assumes: Assumptions,
}

/// What a searched or repeat game's line says of its setup, as far as the checks need it.
#[derive(Clone, Copy, Debug)]
struct Setup {
    cap: Option<u64>,
    /// Its frames, candidates and `max_total`.
    size: ProblemSize,
    digest: u64,
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
    setup: Option<Setup>,
    reached_cap: bool,
    in_the_bag: Option<u64>,
    named: Option<Named>,
    /// The proof and the assumptions the line records.
    recorded: (Proof, Assumptions),
    /// A repeat's record of the game it repeats.
    same_search: Option<SameSearch>,
}

/// A line of a results file.
enum Line {
    Completed(Box<Entry>),
    /// A search a limit stopped, with the number of invocations it records.
    Incomplete {
        n: u32,
        invocations: u32,
    },
}

/// Field access with errors that name the field (also the reader of `taxman certify`'s certificates uses it).
pub(crate) struct Fields<'a>(pub(crate) &'a Json);

impl<'a> Fields<'a> {
    pub(crate) fn get(&self, name: &str) -> Result<&'a Json, String> {
        self.0.get(name).ok_or_else(|| format!("missing \"{name}\""))
    }

    pub(crate) fn bad(name: &str) -> String {
        format!("bad \"{name}\"")
    }

    pub(crate) fn u32(&self, name: &str) -> Result<u32, String> {
        self.get(name)?.as_u32().ok_or_else(|| Self::bad(name))
    }

    pub(crate) fn u64(&self, name: &str) -> Result<u64, String> {
        self.get(name)?.as_u64().ok_or_else(|| Self::bad(name))
    }

    pub(crate) fn usize(&self, name: &str) -> Result<usize, String> {
        self.get(name)?.as_u64().and_then(|value| usize::try_from(value).ok()).ok_or_else(|| Self::bad(name))
    }

    pub(crate) fn f64(&self, name: &str) -> Result<f64, String> {
        self.get(name)?.as_f64().ok_or_else(|| Self::bad(name))
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

    /// A flag that older lines do not have: `default` where the field is absent, an error if it is there and is no
    /// boolean.
    pub(crate) fn bool_or(&self, name: &str, default: bool) -> Result<bool, String> {
        self.0.get(name).map_or(Ok(default), |_| self.bool(name))
    }

    fn str(&self, name: &str) -> Result<&'a str, String> {
        self.get(name)?.as_str().ok_or_else(|| Self::bad(name))
    }

    pub(crate) fn u32s(&self, name: &str) -> Result<Vec<u32>, String> {
        self.get(name)?.as_u32_vec().ok_or_else(|| Self::bad(name))
    }

    fn assumptions(&self, name: &str) -> Result<Assumptions, String> {
        let items = self.get(name)?.as_arr().ok_or_else(|| Self::bad(name))?;
        items.iter().map(|item| item.as_str().map(str::to_owned).ok_or_else(|| Self::bad(name))).collect()
    }

    fn proof(&self, name: &str) -> Result<Option<Proof>, String> {
        match self.get(name)? {
            Json::Null => Ok(None),
            value => value.as_str().and_then(Proof::parse).map(Some).ok_or_else(|| Self::bad(name)),
        }
    }

    /// A search problem's digest, as [`crate::solver::digest_text`] writes it.
    pub(crate) fn digest(&self, name: &str) -> Result<u64, String> {
        parse_digest(self.str(name)?).ok_or_else(|| Self::bad(name))
    }
}

/// A repeat's record of the game it repeats (`same_search`).
fn parse_same_search(json: &Json) -> Result<SameSearch, String> {
    let f = Fields(json);
    let bad = |name: &str| format!("bad \"same_search.{name}\"");
    Ok(SameSearch {
        n: f.u32("n").map_err(|_| bad("n"))?,
        digest: f.digest("digest").map_err(|_| bad("digest"))?,
        promotion_sum: f.u64("promotion_sum").map_err(|_| bad("promotion_sum"))?,
        proof: f.proof("proof").ok().flatten().ok_or_else(|| bad("proof"))?,
        assumes: f.assumptions("assumes").map_err(|_| bad("assumes"))?,
    })
}

fn parse_source(text: &str) -> Option<PrevSource> {
    [PrevSource::Run, PrevSource::Results, PrevSource::Optimal].into_iter().find(|s| s.as_str() == text)
}

/// Parses a line that is valid JSON.
fn parse_line(json: &Json, line: usize) -> Result<Line, String> {
    let f = Fields(json);
    let schema = f.u32("schema")?;
    if schema != SCHEMA {
        let which = if schema > SCHEMA { "newer" } else { "older" };
        return Err(format!("schema {schema} is {which} than the schema {SCHEMA} that this build reads"));
    }
    let n = f.u32("n")?;
    if !(1..=MAX_N).contains(&n) {
        return Err(format!("n={n} is not a board size from 1 to {MAX_N}"));
    }
    if !f.bool("completed")? {
        return Ok(Line::Incomplete { n, invocations: f.u32("invocations")? });
    }
    let method = Method::parse(f.str("method")?).ok_or("bad \"method\"")?;
    let promotions = f.u32s("promotions")?;
    let setup = match f.get("setup")? {
        Json::Null => None,
        setup => {
            let s = Fields(setup);
            let digest = s.digest("digest").map_err(|e| format!("{e} in \"setup\""))?;
            let size = ProblemSize {
                frames: s.u32("frames")?,
                candidates: s.u32("candidates")?,
                max_total: s.u32("max_total")?,
            };
            Some(Setup { cap: s.opt_u64("cap")?, size, digest })
        }
    };
    let same_search = match f.get("same_search")? {
        Json::Null => None,
        record => Some(parse_same_search(record)?),
    };
    let named = match f.get("prev")? {
        Json::Null => None,
        prev => {
            let p = Fields(prev);
            let source = parse_source(p.str("source")?).ok_or("bad \"prev.source\"")?;
            let record = PrevRecord {
                n: p.u32("n")?,
                score: p.u64("score")?,
                digest: p.str("moves_digest")?.to_owned(),
                proof: p.proof("proof")?,
                assumes: p.assumptions("assumes")?,
            };
            Some(Named { source, record })
        }
    };
    let proof = f.proof("proof")?.ok_or("a completed game needs a \"proof\"")?;
    Ok(Line::Completed(Box::new(Entry {
        line,
        n,
        score: f.u64("score")?,
        moves: f.u32s("moves")?,
        promotions,
        method,
        setup,
        reached_cap: f.bool("reached_cap")?,
        in_the_bag: f.opt_u64("in_the_bag")?,
        named,
        recorded: (proof, f.assumptions("assumes")?),
        same_search,
    })))
}

/// Reads and checks the text of a results file (see the module documentation).  Errors name the line.
///
/// The checks of the repeats set up search problems, which is most of what a check costs at a large `n`, and those
/// setups depend neither on each other nor on the order of the checks.  With `threads` above 1 they are done first,
/// on that many threads ([`set_up_boards`]); the checks themselves run in order on the calling thread, so the result
/// and the first error are the same whatever `threads` is.  With 1 no thread is spawned and each setup waits for
/// the check that needs it.
pub fn read_results(text: &str, optimal: &Optimal, threads: usize) -> Result<Contents, String> {
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
                    contents.runs.insert(n, invocations);
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
    let mut problems = if threads > 1 { set_up_boards(&repeat_boards(&entries), threads) } else { Problems::new() };
    for entry in entries {
        contents.runs.remove(&entry.n);
        let game = check_entry(entry, &contents.games, optimal, &mut problems)?;
        contents.games.insert(game.n, game);
    }
    Ok(contents)
}

/// The search problems of the games of a file that the checks of its repeats set up again, and their boards'
/// `in_the_bag`, by board size.  A board whose setup failed keeps the error, to be reported where a check reaches
/// the board, whether the board was set up beforehand ([`set_up_boards`]) or when that check asked for it
/// ([`problem_of`]).
type Problems = BTreeMap<u32, Result<(Problem, u64), String>>;

/// The search problem of game `k` set up again from its board size, and the board's `in_the_bag` ([`search_problem`]).
fn set_up(k: u32) -> Result<(Problem, u64), String> {
    search_problem(k).map_err(|e| format!("the search of game {k} cannot be set up: {e}"))
}

/// The boards whose problems the checks of the repeats among the completed lines `entries` set up: a repeat's own and
/// the one it repeats.  A repeat of a game that is not a completed game of the file below it fails ([`check_repeat`])
/// before any setup, and asks for none.
fn repeat_boards(entries: &[Entry]) -> BTreeSet<u32> {
    // Every board size of a line is from 1 to `MAX_N` (`parse_line`), so a board size in this set is at least 1.
    let completed: BTreeSet<u32> = entries.iter().map(|e| e.n).collect();
    let mut boards = BTreeSet::new();
    for e in entries {
        if let Some(record) = &e.same_search {
            if record.n < e.n && completed.contains(&record.n) {
                boards.extend([e.n, record.n]);
            }
        }
    }
    boards
}

/// Sets up the search problems of `boards` on `threads` threads, each taking the next board from one counter.  The
/// setup of a board costs more the larger its size, so the largest come first, and the threads end together.
fn set_up_boards(boards: &BTreeSet<u32>, threads: usize) -> Problems {
    let order: Vec<u32> = boards.iter().rev().copied().collect();
    in_parallel(order.len(), threads, |i| (order[i], set_up(order[i]))).into_iter().collect()
}

/// `task(0)`, `task(1)`, ..., `task(count - 1)` in that order, computed on `threads` threads (at least one) that each
/// take the next index from one counter, so that a slow task delays no other; with one thread, on the calling thread.
/// Each task is a function of its index only, so the values are the same whatever `threads` is.
pub(crate) fn in_parallel<T: Send>(count: usize, threads: usize, task: impl Fn(usize) -> T + Sync) -> Vec<T> {
    if threads <= 1 {
        return (0..count).map(task).collect();
    }
    let next = AtomicUsize::new(0);
    let mut done: Vec<(usize, T)> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads.min(count))
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= count {
                            break done;
                        }
                        done.push((i, task(i)));
                    }
                })
            })
            .collect();
        // A worker that panicked panics here, as the same task would have on the calling thread.
        workers.into_iter().flat_map(|w| w.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))).collect()
    });
    done.sort_unstable_by_key(|&(i, _)| i);
    done.into_iter().map(|(_, value)| value).collect()
}

/// The search problem of game `k` and its board's `in_the_bag`: the one in `problems`, or else set up now and kept.
fn problem_of(problems: &mut Problems, k: u32) -> Result<&(Problem, u64), String> {
    problems.entry(k).or_insert_with(|| set_up(k)).as_ref().map_err(String::clone)
}

/// [`read_results`] on the file at `path`, without changing it.  Errors name the file.
pub fn read_results_file(path: &Path, optimal: &Optimal, threads: usize) -> Result<Contents, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    read_results(&text, optimal, threads).map_err(|e| format!("{}: {e}", path.display()))
}

/// The game of `n − 1` that a line was built from, as far as it is known.  For a game of `optimal.json` that the
/// table at hand does not have (a line made with a longer table), the line's own record stands in for its score,
/// and its moves are not known.
struct Pred<'a> {
    score: u64,
    moves: Option<&'a [u32]>,
    /// The file's game; `None` for `optimal.json`'s.
    stored: Option<&'a StoredGame>,
    assumes: Assumptions,
    whence: String,
}

/// What the fields of a line say against each other, if anything: combinations the solver never writes, through
/// which a hand-edited line could change the proof or the assumptions derived from it.
fn contradiction(e: &Entry) -> Option<String> {
    let method = e.method.as_str();
    let searched = e.method == Method::Search;
    if e.method.sets_up() != e.setup.is_some() {
        return Some(if e.method.sets_up() {
            format!("a {method} game without a \"setup\"")
        } else {
            format!("a {method} game with a \"setup\", which only a searched or repeat game has")
        });
    }
    let repeat = e.method == Method::Repeat;
    if repeat != e.same_search.is_some() {
        return Some(if repeat {
            "a repeat game without a \"same_search\" record of the game it repeats".to_owned()
        } else {
            format!("a {method} game with a \"same_search\" record, which only a repeat has")
        });
    }
    if repeat && e.setup.is_some_and(|setup| setup.cap.is_some()) {
        return Some("a repeat game whose \"setup\" has a cap, which only a search has".to_owned());
    }
    if e.in_the_bag.is_some() == (e.method == Method::Prime) {
        let with = if e.in_the_bag.is_some() { "with" } else { "without" };
        return Some(format!("a {method} game {with} \"in_the_bag\", which every game but a prime one records"));
    }
    if e.reached_cap && !searched {
        return Some(format!("a {method} game that reached a cap, which only a search does"));
    }
    if e.method == Method::Prime && largest_prime_at_most(e.n) != Some(e.n) {
        return Some(format!("a prime game, but {} is not prime", e.n));
    }
    if e.method == Method::Trivial && !e.promotions.is_empty() {
        return Some("a trivial game with promotions".to_owned());
    }
    None
}

/// The checks of a repeat's line `e` against the game it repeats, which its record `record` names (see the module
/// documentation).  Returns that game.
fn check_repeat<'a>(
    e: &Entry,
    record: &SameSearch,
    games: &'a BTreeMap<u32, StoredGame>,
    problems: &mut Problems,
) -> Result<&'a StoredGame, String> {
    let m = record.n;
    let Some(earlier) = games.get(&m).filter(|_| m < e.n) else {
        return Err(format!("it repeats the search of game {m}, which is not a completed game of the file below it"));
    };
    if earlier.line > e.line {
        return Err(format!("it repeats the search of game {m}, whose line ({}) comes after it", earlier.line));
    }
    if !earlier.proof.settles_search() {
        let proof = earlier.proof.as_str();
        return Err(format!("it repeats the search of game {m}, whose proof ({proof}) settles no search"));
    }
    if record.proof != earlier.proof || record.assumes != earlier.assumes {
        return Err(format!("its record of game {m} does not match line {}", earlier.line));
    }
    let digest = e.setup.map(|s| s.digest);
    if digest != Some(record.digest) || earlier.digest != Some(record.digest) {
        return Err(format!("it and game {m} (line {}) record different search digests", earlier.line));
    }
    if e.promotions != earlier.promotions {
        return Err(format!("its promotions are not those of game {m} (line {})", earlier.line));
    }
    let sum: u64 = e.promotions.iter().map(|&p| u64::from(p)).sum();
    if sum != record.promotion_sum {
        return Err(format!("its promotions sum to {sum}, not the promotion_sum {} it records", record.promotion_sum));
    }

    // The two search problems, set up again from the board sizes, and as large as the lines say (the frame builder of
    // the build that wrote a line posed the same problem as this build's, as far as the line tells).
    let Some((setup, size)) = e.setup.zip(earlier.size) else {
        return Err(format!("it or game {m} has no setup to set its search up from"));
    };
    for (k, size) in [(e.n, setup.size), (m, size)] {
        let set_up_size = problem_of(problems, k)?.0.size();
        if set_up_size != size {
            return Err(format!(
                "the search problem of game {k} set up again is {set_up_size:?}, not the {size:?} its line records"
            ));
        }
    }
    let (Some(Ok((mine, bag))), Some(Ok((their_problem, their_bag)))) = (problems.get(&e.n), problems.get(&m)) else {
        unreachable!("the loop above found both boards set up, without error");
    };
    if mine.digest() != record.digest {
        return Err(format!(
            "its search problem's digest is {}, not the {} it records",
            crate::solver::digest_text(mine.digest()),
            crate::solver::digest_text(record.digest)
        ));
    }
    if mine != their_problem {
        return Err(format!("its search problem is not the one game {m} (line {}) posed", earlier.line));
    }
    if e.in_the_bag != Some(*bag) {
        return Err(format!("its in_the_bag is not the board's {bag}"));
    }
    if earlier.score != their_bag + sum {
        return Err(format!("game {m} does not score its board's in_the_bag {their_bag} plus the promotion sum {sum}"));
    }
    Ok(earlier)
}

/// Checks one completed line against the games of the file below it (see the module documentation).  `problems`
/// keeps the search problems that the checks of repeats set up.
fn check_entry(
    e: Entry,
    games: &BTreeMap<u32, StoredGame>,
    optimal: &Optimal,
    problems: &mut Problems,
) -> Result<StoredGame, String> {
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
    if let Some(expected) = optimal.score(n).filter(|&expected| expected != e.score) {
        return Err(fail(format!("scores {}, but optimal.json has {expected}", e.score)));
    }
    if let Some(contradiction) = contradiction(&e) {
        return Err(fail(contradiction));
    }

    let in_file = n.checked_sub(1).and_then(|before| games.get(&before)).map(|g| Pred {
        score: g.score,
        moves: Some(&g.moves),
        stored: Some(g),
        assumes: g.assumes.clone(),
        whence: format!("line {}", g.line),
    });
    let in_optimal = n.checked_sub(1).and_then(|before| {
        Some(Pred {
            score: optimal.score(before)?,
            moves: Some(optimal.moves(before)?),
            stored: None,
            assumes: Assumptions::from([assumed_optimal(before)]),
            whence: "optimal.json".to_owned(),
        })
    });

    // The chain, against whichever game of n − 1 there is: both bounds are theorems about optimal games.
    if let Some((before, whence)) = in_file.as_ref().or(in_optimal.as_ref()).map(|p| (p.score, &p.whence)) {
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
                    score: named.record.score,
                    moves: None,
                    stored: None,
                    assumes: Assumptions::from([assumed_optimal(n - 1)]),
                    whence: "optimal.json".to_owned(),
                }),
            };
            let record = &named.record;
            let same = record.n + 1 == n
                && pred.score == record.score
                && pred.moves.is_none_or(|moves| moves_digest(moves) == record.digest)
                && record.proof == pred.stored.map(|g| g.proof)
                && record.assumes == pred.assumes;
            if !same {
                return Err(fail(format!("its record of the game of n − 1 does not match {}", pred.whence)));
            }
            Some(pred)
        }
    };

    // The line's own arithmetic.
    let sum: u64 = e.promotions.iter().map(|&p| u64::from(p)).sum();
    let prev_score = pred.as_ref().map(|p| p.score);
    if let Some(bag) = e.in_the_bag {
        if e.score != bag + sum {
            return Err(fail(format!("scores {}, not in_the_bag {bag} + the promotions' {sum}", e.score)));
        }
    }
    let cap = e.setup.and_then(|s| s.cap);
    if let Some(cap) = cap {
        if sum > cap {
            return Err(fail(format!(
                "the promotions' {sum} exceed the cap {cap}, so the game of n − 1 is not optimal"
            )));
        }
        // in_the_bag is the score less the promotions, and the ceiling `cap` is n + score(n − 1) − in_the_bag.
        if prev_score.is_some_and(|before| before + u64::from(n) + sum != cap + e.score) {
            return Err(fail(format!("the cap {cap} is not n + score(n − 1) − in_the_bag")));
        }
    }
    // The search stops as soon as its incumbent reaches the ceiling, and only then.
    if e.reached_cap && cap != Some(sum) {
        return Err(fail(format!("it records reaching the cap, but its promotions' {sum} are not the cap {cap:?}")));
    }
    if !e.reached_cap && cap == Some(sum) {
        return Err(fail(format!("its promotions' {sum} reach the cap, but it records not reaching it")));
    }
    let replaced = replaced_by_prime(n);
    if e.method == Method::Prime && prev_score.is_some_and(|before| e.score + replaced != before + u64::from(n)) {
        return Err(fail(format!("a prime game scores n + score(n − 1) − {replaced}, and this one does not")));
    }

    let proof = Proof::of(e.method, true, e.reached_cap).expect("a completed game has a proof");
    if proof.rests_on_prev() && pred.is_none() {
        return Err(fail(format!("its proof ({}) rests on a game of n − 1 that it does not name", proof.as_str())));
    }
    let repeated = match &e.same_search {
        Some(record) => Some(check_repeat(&e, record, games, problems).map_err(fail)?),
        None => None,
    };
    let basis = match repeated {
        Some(earlier) => Some(&earlier.assumes),
        None => pred.as_ref().map(|p| &p.assumes),
    };
    let assumes = assumptions_of(proof, basis);
    let (recorded_proof, recorded_assumes) = &e.recorded;
    if *recorded_proof != proof {
        return Err(fail(format!("it records the proof {}, but it is {}", recorded_proof.as_str(), proof.as_str())));
    }
    if *recorded_assumes != assumes {
        return Err(fail(format!("it records the assumptions {recorded_assumes:?}, but they are {assumes:?}")));
    }
    let ceiling_proof = prev_score.and_then(|before| CeilingProof::of(n, e.score, before));
    Ok(StoredGame {
        n,
        line: e.line,
        score: e.score,
        moves: e.moves,
        promotions: e.promotions,
        proof,
        assumes,
        ceiling_proof,
        size: e.setup.map(|s| s.size),
        digest: e.setup.map(|s| s.digest),
    })
}

// ---------------------------------------------------------------------------------------------------------
// The results file of a run

/// A results file being appended to: one JSON object per line, as games finish.  Only completed games count as
/// done.  Opening it locks it and checks it ([`read_results`]).
pub struct ResultsFile {
    path: PathBuf,
    /// Locked (`flock`) for as long as it is open.
    file: File,
    contents: Contents,
}

impl ResultsFile {
    /// Opens (creating if need be) the file, locks it against every other process (the lock goes when the file is
    /// dropped or the process ends, however it ends), and checks it, with `optimal` for the games of `n − 1` that
    /// came from it.  A torn last line is cut off.
    ///
    /// Two runs appending to one file would both solve the games it lacks and both append them, leaving two lines
    /// for a game, which no later run opens; so a file another process has open is refused, as `taxman solve`
    /// refuses a checkpoint directory in use (`engine::lock_dir`).  Two handles in one process exclude each other
    /// too.
    ///
    /// The check runs on `threads` threads where it can ([`read_results`]).
    pub fn open(path: &Path, optimal: &Optimal, threads: usize) -> Result<ResultsFile, String> {
        let io_error = |e: std::io::Error| format!("{}: {e}", path.display());
        let (mut file, text) = open_locked(path)?;
        let contents = read_results(&text, optimal, threads).map_err(|e| format!("{}: {e}", path.display()))?;
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

    /// The completed games of the file, by board size.
    pub fn games(&self) -> impl Iterator<Item = &StoredGame> {
        self.contents.games.values()
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
                    promotions: result.promotions.clone(),
                    proof,
                    assumes: result.assumes.clone(),
                    ceiling_proof: result.prev.as_ref().and_then(|p| CeilingProof::of(result.n, result.score, p.score)),
                    size: result.setup.map(|s| s.size()),
                    digest: result.setup.map(|s| s.digest),
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

/// Dropping releases the lock at once, by unlocking rather than only closing the file, as `engine::DirLock`
/// does (see there for why).
impl Drop for ResultsFile {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Where the games that failed a check go: `<results>.rejected.jsonl`, with a `.jsonl` ending of the results
/// file dropped (`default.jsonl` → `default.rejected.jsonl`).
pub fn rejected_path(results: &Path) -> PathBuf {
    sibling_path(results, "rejected")
}

/// The file `<file>.<kind>.jsonl` beside `file`, with a `.jsonl` ending of `file` dropped first (`default.jsonl` and
/// `rejected` give `default.rejected.jsonl`).
pub(crate) fn sibling_path(file: &Path, kind: &str) -> PathBuf {
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = name.strip_suffix(".jsonl").unwrap_or(&name);
    file.with_file_name(format!("{stem}.{kind}.jsonl"))
}

/// Opens the file at `path` for appending (creating it if need be), locks it against every other process and every
/// other handle (`flock`; the lock goes when the file is unlocked or dropped, or the process ends), and reads it whole.
/// A file another holds is refused, since two runs appending to one file would both append the same games.
pub(crate) fn open_locked(path: &Path) -> Result<(File, String), String> {
    let io_error = |e: std::io::Error| format!("{}: {e}", path.display());
    let mut file = OpenOptions::new().create(true).read(true).append(true).open(path).map_err(io_error)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            return Err(format!(
                "{} is in use by another process (another run appending to it?): it is locked",
                path.display()
            ))
        }
        Err(TryLockError::Error(e)) => return Err(io_error(e)),
    }
    let mut text = String::new();
    file.read_to_string(&mut text).map_err(io_error)?;
    Ok((file, text))
}

/// Writes one line and syncs it.
pub(crate) fn append_line(file: &mut File, line: &Json) -> std::io::Result<()> {
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

/// Checks results files without changing them: each as [`read_results`] does (on `threads` threads), and that files
/// agree on the score of every board size they share.
pub fn verify_files(paths: &[PathBuf], optimal: &Optimal, threads: usize) -> Result<Report, String> {
    let mut files = Vec::new();
    for path in paths {
        files.push(FileReport { path: path.clone(), contents: read_results_file(path, optimal, threads)? });
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
        let plain = games.values().filter(|g| g.assumes.is_empty()).count();
        write!(f, "  assumptions: none in {plain} of {} games", games.len())?;
        let mut by: BTreeMap<&str, usize> = BTreeMap::new();
        for statement in games.values().flat_map(|g| &g.assumes) {
            *by.entry(statement).or_default() += 1;
        }
        for (statement, count) in by {
            write!(f, "; {count} {} that {statement}", if count == 1 { "assumes" } else { "assume" })?;
        }
        let ceiling_proofs = counts(games.values().filter_map(|g| g.ceiling_proof), |c| c.as_str().to_owned());
        write!(f, "\n  scores at a proven ceiling, given the game of n-1: {ceiling_proofs}")
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
// Extending the table of optimal games

/// Why [`extend_optimal`] stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// The results end before the board size; `last` is the board size of their last completed game (0: none).
    ResultsEnd { last: u32 },
    /// The results hold games past the board size (up to `last`) but none completed for it.
    Gap { last: u32 },
    /// The results hold only searches of the board size that a limit stopped.
    Incomplete,
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StopReason::ResultsEnd { last: 0 } => write!(f, "the results have no completed game"),
            StopReason::ResultsEnd { last } => write!(f, "the results end at n = {last}"),
            StopReason::Gap { last } => write!(f, "the results have no completed game of it, but go on to n = {last}"),
            StopReason::Incomplete => write!(f, "the results have only a search of it that a limit stopped"),
        }
    }
}

/// What [`extend_optimal`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    /// The board sizes of the games it appended to the table (an empty range if none).
    pub added: std::ops::RangeInclusive<u32>,
    /// The first board size it did not append.
    pub stopped_at: u32,
    /// Why it did not.
    pub reason: StopReason,
}

/// Appends to `optimal` the games of a checked results file that continue it: from the board size after the table's
/// last, each game that `contents` ([`read_results`]) holds completed, up to the first board size that has none.  A
/// game whose proof assumes a game of the table optimal is appended too: it assumes nothing the table does not
/// stand for.  A search that a limit stopped and a completed game of the same board size may both be in the file (a
/// run and its resume): the completed game is the game.
///
/// # Errors
///
/// If the results and the table score differently for a board size that both have, naming it: they are not
/// results for the same game, or one of them is wrong.  The scores are compared before anything is appended.
pub fn extend_optimal(optimal: &mut Optimal, contents: &Contents) -> Result<Extension, String> {
    for (&n, game) in &contents.games {
        if let Some(recorded) = optimal.score(n).filter(|&recorded| recorded != game.score) {
            return Err(format!(
                "n={n}: the results (line {}) score {}, but the table has {recorded}",
                game.line, game.score
            ));
        }
    }
    let last = contents.games.keys().next_back().copied().unwrap_or(0);
    let first = optimal.max_n() + 1;
    let mut n = first;
    let reason = loop {
        match contents.games.get(&n) {
            Some(game) => optimal.push(n, game.score, game.moves.clone())?,
            None if contents.runs.contains_key(&n) => break StopReason::Incomplete,
            None if last > n => break StopReason::Gap { last },
            None => break StopReason::ResultsEnd { last },
        }
        n += 1;
    };
    Ok(Extension { added: first..=n - 1, stopped_at: n, reason })
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{solve_game, solve_range, Prev, SolveOptions};

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
    fn tasks_in_parallel_give_their_values_in_the_order_of_their_indices_whatever_the_threads() {
        let square = |i: usize| i * i;
        let expected = |count: usize| (0..count).map(square).collect::<Vec<_>>();
        for (count, threads) in [(0, 1), (0, 3), (1, 4), (7, 1), (7, 2), (7, 3), (7, 16), (100, 4)] {
            assert_eq!(in_parallel(count, threads, square), expected(count), "{count} tasks on {threads} threads");
        }
        // No threads asked for is one.
        assert_eq!(in_parallel(3, 0, square), expected(3));
    }

    #[test]
    fn the_digest_is_fnv1a_of_the_moves_in_decimal() {
        assert_eq!(moves_digest(&[]), "cbf29ce484222325");
        // As computed by tools/check_results.py, which shares no code with this.
        assert_eq!(moves_digest(&[5, 4, 6]), "c27bf6b65c1b8616");
        assert_eq!(moves_digest(&[79]), "07f20f07b4b4be5d");
    }

    #[test]
    fn proofs_and_assumptions() {
        assert_eq!(Proof::of(Method::Search, false, false), None);
        assert_eq!(Proof::of(Method::Search, true, true), Some(Proof::Cap));
        assert_eq!(Proof::of(Method::Search, true, false), Some(Proof::Exhausted));
        assert!(Proof::ALL.iter().all(|&p| Proof::parse(p.as_str()) == Some(p)));
        let names = |assumes: Assumptions| assumes.into_iter().collect::<Vec<_>>();
        // Inherited only by a proof that rests on the game of n − 1.
        assert_eq!(assumed_optimal(1300), "the game of 1300 in optimal.json is optimal");
        let prev = Assumptions::from([assumed_optimal(1300)]);
        assert_eq!(names(assumptions_of(Proof::Cap, Some(&prev))), [assumed_optimal(1300)]);
        assert_eq!(names(assumptions_of(Proof::PrimeFormula, Some(&prev))), [assumed_optimal(1300)]);
        assert!(assumptions_of(Proof::Exhausted, Some(&prev)).is_empty());
        assert!(assumptions_of(Proof::Trivial, Some(&prev)).is_empty());
        assert!(assumptions_of(Proof::Cap, None).is_empty());
        // A repeat rests on the game it repeats; it searched nothing, so no bound of its own.
        assert_eq!(Proof::of(Method::Repeat, true, false), Some(Proof::SameSearch));
        let settled: Vec<Proof> = Proof::ALL.into_iter().filter(|p| p.settles_search()).collect();
        assert_eq!(settled, [Proof::Cap, Proof::Exhausted, Proof::SameSearch]);
        assert!(!Proof::SameSearch.rests_on_prev() && Proof::SameSearch.rests_on_another_game());
        assert_eq!(names(assumptions_of(Proof::SameSearch, Some(&prev))), [assumed_optimal(1300)]);
        assert!(assumptions_of(Proof::SameSearch, Some(&Assumptions::new())).is_empty());
    }

    #[test]
    fn a_file_reads_back_with_its_proofs() {
        let lines = lines_1_to_12();
        let contents = read_results(&text(&lines), Optimal::built_in(), 1).unwrap();
        assert_eq!(contents.games.keys().copied().collect::<Vec<_>>(), (1..=12).collect::<Vec<_>>());
        assert!(!contents.torn && contents.incomplete_lines == 0 && contents.lines == 12);
        assert_eq!(contents.games[&11].proof, Proof::PrimeFormula);
        assert_eq!(contents.games[&11].ceiling_proof, Some(CeilingProof::PrimeFormula));
        assert_eq!(contents.games[&10].proof, Proof::Trivial);
    }

    #[test]
    fn a_torn_last_line_is_dropped_and_any_other_bad_line_is_an_error() {
        let good = text(&lines_1_to_12());
        for tail in ["{\"n\":13,\"sco", "{\"n\":13,\"sco\n", "\0\0\0\0\0\0\0\0 13,\"score\":5}\n", "garbage\n\n\n"] {
            let contents = read_results(&format!("{good}{tail}"), Optimal::built_in(), 1).unwrap();
            assert!(contents.torn, "{tail:?}");
            assert_eq!((contents.games.len(), contents.valid_len), (12, good.len()), "{tail:?}");
        }
        // A last line that parses but is not a result, and a bad line before the last, are errors.
        let err = read_results(&format!("{good}{{\"schema\":5}}\n"), Optimal::built_in(), 1).unwrap_err();
        assert!(err.starts_with("line 13: missing \"n\""), "{err}");
        let err = read_results(&format!("garbage\n{good}"), Optimal::built_in(), 1).unwrap_err();
        assert!(err.starts_with("line 1: JSON syntax error"), "{err}");
        assert!(read_results("", Optimal::built_in(), 1).unwrap().games.is_empty());
    }

    #[test]
    fn tampered_lines_are_refused_naming_the_line() {
        let lines = lines_1_to_12();
        let refused = |i: usize, line: Json, expected: &str| {
            let mut tampered = lines.clone();
            tampered[i] = line;
            let err = read_results(&text(&tampered), Optimal::built_in(), 1).unwrap_err();
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
        refused(11, with(&lines[11], "assumes", Json::array([assumed_optimal(11)])), "records the assumptions");
        // A score that is not the table's: here the table is wrong about game 12, which the check cannot tell.
        let other = Optimal::from_json(&Json::parse(r#"[{"n":12,"score":51,"moves":[]}]"#).unwrap()).unwrap();
        let err = read_results(&text(&lines), &other, 1).unwrap_err();
        assert!(err.starts_with("line 12 (n=12): scores 50, but optimal.json has 51"), "{err}");
        refused(11, with(&lines[11], "proof", Json::from("cap")), "records the proof cap");
        refused(11, with(&lines[11], "promotions", Json::array([12u32, 8])), "not in ascending order");
        // Only schema 5 is read: a line of another schema, or of none, is refused.
        let unreadable = |i: usize, line: Json, expected: &str| {
            let mut tampered = lines.clone();
            tampered[i] = line;
            let err = read_results(&text(&tampered), Optimal::built_in(), 1).unwrap_err();
            assert!(err.starts_with(&format!("line {}: ", i + 1)) && err.contains(expected), "{expected}: {err}");
        };
        for (schema, expected) in [(4u32, "schema 4 is older than the schema 5"), (6, "schema 6 is newer than")] {
            unreadable(11, with(&lines[11], "schema", Json::from(schema)), expected);
        }
        let Json::Obj(pairs) = &lines[11] else { panic!("not an object") };
        let unversioned = Json::Obj(pairs.iter().filter(|(key, _)| key != "schema").cloned().collect());
        unreadable(11, unversioned, "missing \"schema\"");
        let mut twice = lines.clone();
        twice.push(lines[4].clone());
        assert!(read_results(&text(&twice), Optimal::built_in(), 1).unwrap_err().contains("lines 5 and 13 both hold"));
    }

    /// An open results file is locked: opening it again (from this process or another) is refused until it is
    /// closed.
    #[test]
    fn a_results_file_is_locked_while_it_is_open() {
        let dir = crate::test_support::TempDir::new("results-lock");
        let path = dir.join("r.jsonl");
        let held = ResultsFile::open(&path, Optimal::built_in(), 1).unwrap();
        let err = ResultsFile::open(&path, Optimal::built_in(), 1).err().expect("a second handle is refused");
        assert!(err.contains("r.jsonl is in use by another process"), "{err}");
        drop(held);
        assert!(ResultsFile::open(&path, Optimal::built_in(), 1).is_ok());
    }

    /// A closed results file is free at once, though a copy of its descriptor lives on, as in a child process
    /// between `fork` and `exec` (see `engine::DirLock`).
    #[test]
    fn a_closed_results_file_is_free_while_a_copy_of_its_descriptor_lives() {
        let dir = crate::test_support::TempDir::new("results-lock-copy");
        let path = dir.join("r.jsonl");
        let held = ResultsFile::open(&path, Optimal::built_in(), 1).unwrap();
        let copy = held.file.try_clone().unwrap();
        drop(held);
        let again = ResultsFile::open(&path, Optimal::built_in(), 1).err();
        assert!(again.is_none(), "{again:?}");
        drop(copy);
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
            let err = read_results(&line, &optimal.unwrap(), 1).unwrap_err();
            assert!(err.starts_with("line 1 (n=12)") && err.contains(expected), "{err}");
        }
        // Without a game of 11 the line cannot stand: it names one from this file.
        let err = read_results(&line, Optimal::built_in(), 1).unwrap_err();
        assert!(err.contains("which the file does not hold"), "{err}");
    }

    /// The table of the first `count` games of `contents`.
    fn table_of(contents: &Contents, count: u32) -> Optimal {
        let games = (1..=count).map(|n| {
            let game = &contents.games[&n];
            let moves = Json::array(game.moves.iter().copied());
            Json::object([("n", Json::from(n)), ("score", Json::from(game.score)), ("moves", moves)])
        });
        Optimal::from_json(&Json::array(games)).unwrap()
    }

    /// The line a run stopped by a limit writes for a search of `n` that did not finish.
    fn incomplete_line(n: u32) -> Json {
        Json::object([
            ("schema", Json::from(SCHEMA)),
            ("n", Json::from(n)),
            ("completed", Json::Bool(false)),
            ("invocations", Json::from(1u32)),
        ])
    }

    /// A line's board size is checked against `MAX_N`, the largest board the solver supports (8191): the incomplete
    /// line of a search stopped by a limit is read for the first board past 4095 and for the last, and is refused,
    /// naming the range, for board 0 and for the first board past the last.
    #[test]
    fn a_lines_board_size_must_be_a_board_the_solver_supports() {
        assert_eq!(MAX_N, 8191);
        for n in [1, 4095, 4096, 8191] {
            let contents = read_results(&text(&[incomplete_line(n)]), Optimal::built_in(), 1).unwrap();
            assert_eq!((contents.runs.get(&n), contents.incomplete_lines), (Some(&1), 1), "n={n}");
        }
        for n in [0, 8192] {
            let err = read_results(&text(&[incomplete_line(n)]), Optimal::built_in(), 1).unwrap_err();
            assert_eq!(err, format!("line 1: n={n} is not a board size from 1 to 8191"));
        }
    }

    #[test]
    fn the_table_is_extended_by_the_games_that_follow_it() {
        let contents = read_results(&text(&lines_1_to_12()), Optimal::built_in(), 1).unwrap();
        assert!(contents.games.values().all(|game| game.assumes.is_empty()));
        let mut table = table_of(&contents, 5);
        let extension = extend_optimal(&mut table, &contents).unwrap();
        assert_eq!(extension, Extension { added: 6..=12, stopped_at: 13, reason: StopReason::ResultsEnd { last: 12 } });
        assert!(table.sizes().eq(1..=12));
        for (n, score, moves) in table.games() {
            assert_eq!((score, moves), (contents.games[&n].score, &contents.games[&n].moves[..]), "n={n}");
        }

        // Nothing to add is fine, and changes nothing.
        let before = table.to_file_text();
        let extension = extend_optimal(&mut table, &contents).unwrap();
        assert!(extension.added.is_empty() && extension.stopped_at == 13, "{extension:?}");
        assert_eq!(table.to_file_text(), before);
    }

    #[test]
    fn the_extension_takes_a_game_that_assumes_a_game_of_the_table() {
        // Games 51 and 52 are solved standalone (their proofs rest on no other game).  Game 53, a prime, is built
        // on optimal.json's game of 52, and its proof (the prime formula) assumes that game optimal.
        let optimal = Optimal::built_in();
        let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
        let mut lines: Vec<Json> = solve_range(51, 52, &opts, None).unwrap().iter().map(GameResult::to_json).collect();
        let prev = Prev::from_optimal(optimal, 52).unwrap();
        lines.push(solve_game(53, Some(&prev), &opts).unwrap().to_json());
        let contents = read_results(&text(&lines), optimal, 1).unwrap();
        assert!(contents.games[&51].assumes.is_empty() && contents.games[&52].assumes.is_empty());
        assert!(contents.games[&53].assumes.iter().eq([&assumed_optimal(52)]));

        let games = optimal.games().take(50).map(|(n, score, moves)| {
            let moves = Json::array(moves.iter().copied());
            Json::object([("n", Json::from(n)), ("score", Json::from(score)), ("moves", moves)])
        });
        let mut table = Optimal::from_json(&Json::array(games)).unwrap();
        let extension = extend_optimal(&mut table, &contents).unwrap();
        assert_eq!(
            extension,
            Extension { added: 51..=53, stopped_at: 54, reason: StopReason::ResultsEnd { last: 53 } }
        );
        assert!(table.sizes().eq(1..=53));
    }

    #[test]
    fn the_extension_stops_at_an_unfinished_search_or_a_gap() {
        let lines = lines_1_to_12();
        let extend = |lines: &[Json]| {
            let mut table = table_of(&read_results(&text(&lines[..7]), Optimal::built_in(), 1).unwrap(), 7);
            extend_optimal(&mut table, &read_results(&text(lines), Optimal::built_in(), 1).unwrap()).unwrap()
        };

        // The file ends with a search of 10 that a limit stopped.
        let mut unfinished = lines[..9].to_vec();
        unfinished.push(incomplete_line(10));
        let extension = extend(&unfinished);
        assert_eq!((extension.added, extension.stopped_at, extension.reason), (8..=9, 10, StopReason::Incomplete));

        // A run and its resume: the incomplete line comes first and the completed one is the game.
        let mut resumed = lines[..9].to_vec();
        resumed.push(incomplete_line(10));
        resumed.extend_from_slice(&lines[9..]);
        let extension = extend(&resumed);
        assert_eq!((extension.added, extension.stopped_at), (8..=12, 13));

        // A game of 12 solved on its own, with no 10 or 11 in the file.
        let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
        let mut alone = lines[..9].to_vec();
        alone.push(solve_game(12, None, &opts).unwrap().to_json());
        let extension = extend(&alone);
        assert_eq!(
            (extension.added, extension.stopped_at, extension.reason),
            (8..=9, 10, StopReason::Gap { last: 12 })
        );

        // Games that are all in the table already: nothing is added, and the results' end is the reason.
        let extension = extend(&lines[..7]);
        let reason = StopReason::ResultsEnd { last: 7 };
        assert_eq!((extension.added.is_empty(), extension.stopped_at, extension.reason), (true, 8, reason));
    }

    #[test]
    fn the_extension_refuses_results_that_score_differently_from_the_table() {
        let contents = read_results(&text(&lines_1_to_12()), Optimal::built_in(), 1).unwrap();
        let wrong = r#"[{"n":3,"score":4,"moves":[3]}]"#;
        let mut table = Optimal::from_json(&Json::parse(wrong).unwrap()).unwrap();
        let err = extend_optimal(&mut table, &contents).unwrap_err();
        assert!(err.starts_with("n=3: the results (line 3) score 3, but the table has 4"), "{err}");
        assert_eq!(table.max_n(), 3, "nothing was appended");
    }

    /// The first `count` lines of the committed chain, which include hundreds of repeats (107 in the first 300), whose
    /// checks set search problems up.
    fn committed_lines(count: usize) -> Vec<Json> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("results/strict.jsonl");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        text.lines().take(count).map(|line| Json::parse(line).unwrap()).collect()
    }

    /// The error that `lines` give on one thread, which they give on three as well (nothing else is checked of it).
    fn error_on_one_and_on_three_threads(lines: &[Json]) -> String {
        let [one, three] = [1, 3].map(|threads| read_results(&text(lines), Optimal::built_in(), threads).unwrap_err());
        assert_eq!(one, three);
        one
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is the_checks_give_the_same_result_on_one_thread_and_on_several_for_300_games"
    )]
    fn the_checks_give_the_same_result_on_one_thread_and_on_several() {
        the_checks_give_the_same_result_on_one_thread_and_on_several_for(700);
    }

    /// [`the_checks_give_the_same_result_on_one_thread_and_on_several`] for the first 300 games.
    #[test]
    fn the_checks_give_the_same_result_on_one_thread_and_on_several_for_300_games() {
        the_checks_give_the_same_result_on_one_thread_and_on_several_for(300);
    }

    /// The checks of the first `count` games of the committed chain give the same result on one thread and on
    /// several, and its tampered lines the same errors.
    fn the_checks_give_the_same_result_on_one_thread_and_on_several_for(count: usize) {
        let lines = committed_lines(count);
        let repeats: Vec<usize> =
            (0..lines.len()).filter(|&i| lines[i].get("method").and_then(Json::as_str) == Some("repeat")).collect();
        assert!(repeats.len() > 100, "the test needs repeats to set problems up for");
        let one = read_results(&text(&lines), Optimal::built_in(), 1).unwrap();
        assert_eq!(one.games.len(), count);
        for threads in [2, 3] {
            assert_eq!(read_results(&text(&lines), Optimal::built_in(), threads).unwrap(), one, "{threads} threads");
        }

        // A repeat whose problem set up again is not the size its line records: the first such line fails, whichever
        // way the problems were set up.
        let resized = |i: usize| {
            let setup = lines[i].get("setup").unwrap();
            let candidates = setup.get("candidates").unwrap().as_u32().unwrap();
            with(&lines[i], "setup", with(setup, "candidates", Json::from(candidates + 1)))
        };
        let (first, last) = (repeats[100], repeats[repeats.len() - 1]);
        let mut tampered = lines.clone();
        (tampered[first], tampered[last]) = (resized(first), resized(last));
        let err = error_on_one_and_on_three_threads(&tampered);
        assert!(err.starts_with(&format!("line {} (n=", first + 1)) && err.contains("set up again is"), "{err}");

        // A repeat of a game that is not below it fails before any problem is set up for it.
        let n = lines[first].get("n").unwrap().clone();
        let same_search = lines[first].get("same_search").unwrap();
        let mut tampered = lines.clone();
        tampered[first] = with(&lines[first], "same_search", with(same_search, "n", n));
        let err = error_on_one_and_on_three_threads(&tampered);
        assert!(err.contains("which is not a completed game of the file below it"), "{err}");
    }
}
