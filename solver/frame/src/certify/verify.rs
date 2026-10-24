//! Checking the files of certificates `taxman certify` wrote, without changing them (`taxman verify-certificates`), and
//! how far the proofs of the recorded scores reach.
//!
//! **The lines.**  [`verify_certificates`] reads each file whole (a torn last line is ignored, as
//! [`CertificateFile::open`](super::CertificateFile::open) cuts it) and refuses a complete line that is no certificate
//! ([`Certificate::from_json`]) or whose `build` lacks the commit of 40 hexadecimal digits or the `dirty` flag.  A line
//! with a final verdict (`certified` or `beaten`; the `incomplete` ones are counted and skipped) is then checked
//! against the data it was made from: the prover's barred bounds (the board needs a certificate, with the line's open
//! members and the line's score), the results' game (its score, the checks `certify` makes of the recorded game, at the
//! set width [`certificate_width`] gives, [`check_recorded_game`]), and Π⁺(O) set up again, which must reproduce the
//! line's digest and sizes ([`Posed::reproduces`](super::Posed::reproduces)).  Each line is set up in its own
//! formulation, the one it records (`claims`: with the claims of `unzip` that E4 (docs/theory/extended-problem.md)
//! keeps, or without), for the width and for the problem.  A `certified` line has no `beaten_by` and no line for its
//! board in the counterexample file beside the certificates ([`counterexample_path`]); a `certified` line with
//! `same_problem` rests on the last final line of the game it names before it in the same file, which must be
//! `certified` and among the last `SAME_SEARCH_WINDOW` games, and the rule of the same problem ([`apply_rule`], the one
//! `certify` applies) must give what the line records, with the two problems equal whole, in the formulation of the
//! line that rests on the other (the game it rests on is set up again in that formulation if its own line is of the
//! other); a `beaten` line has a finished legal game scoring more than the recorded score, and an identical line in the
//! counterexample file.  A board with final lines in several places must have only `certified` ones.  The first line to
//! fail ends the check.
//!
//! What it cannot check is that a search explored its whole tree: a `certified` line made by a search is taken on
//! trust, as an exhausted proof is.
//!
//! **The proven chain.**  [`frontier`] says how far the proofs of the recorded scores reach, from plain data: the
//! results' scores, the prover's proofs and the boards certified.

use super::{
    apply_rule, certificate_width, check_recorded_game, counterexample_path, extended_problem, Certificate, Recorded,
    Rule, Upper, Verdict,
};
use crate::game::{replay_finished, Optimal};
use crate::json::Json;
use crate::results::{in_parallel, CeilingProof};
use crate::search::Problem;
use crate::solver::{digest_text, MAX_N, SAME_SEARCH_WINDOW};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------------------------------------
// Reading the files

/// A complete line of a file of certificates, read.
struct Line {
    /// Its line in the file, counting from 1.
    number: usize,
    /// The line as written, without its newline.
    text: String,
    certificate: Certificate,
    /// The build that wrote it: its commit and whether the sources differed from it.
    build: (String, bool),
}

/// What the file of counterexamples beside a file of certificates holds.
#[derive(Default)]
struct Counterexamples {
    /// Its lines, as written.
    lines: HashSet<String>,
    /// The boards it has a line for.
    boards: BTreeSet<u32>,
}

/// A file of certificates, read.
struct Source {
    path: PathBuf,
    lines: Vec<Line>,
    /// The file ends in a line that has no newline, which is ignored.
    torn: bool,
    counterexamples: Counterexamples,
}

/// The commit and the `dirty` flag of the `build` of a line: the commit is a string of 40 hexadecimal digits.
fn read_build(json: &Json) -> Result<(String, bool), String> {
    let build = json.get("build").ok_or("no \"build\"")?;
    let commit = build.get("commit").and_then(Json::as_str);
    let commit = commit.filter(|commit| commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()));
    let dirty = build.get("dirty").and_then(Json::as_bool);
    Ok((
        commit.ok_or("the \"build\" has no \"commit\" of 40 hexadecimal digits")?.to_owned(),
        dirty.ok_or("the \"build\" has no \"dirty\" flag, true or false")?,
    ))
}

/// The complete lines of the text of a file (all but a last line with no newline), and whether there was such a line.
fn complete_lines(text: &str) -> (impl Iterator<Item = (usize, &str)>, bool) {
    let complete = text.rfind('\n').map_or(0, |end| end + 1);
    let lines = text[..complete].lines().enumerate().filter(|(_, line)| !line.trim().is_empty());
    (lines.map(|(i, line)| (i + 1, line)), complete < text.len())
}

/// The counterexample file at `path`: nothing if there is none.
fn read_counterexamples(path: &Path) -> Result<Counterexamples, String> {
    let mut found = Counterexamples::default();
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(found),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    for (number, line) in complete_lines(&text).0 {
        let at = |e: &str| format!("{}: line {number}: {e}", path.display());
        let json = Json::parse(line).map_err(|e| at(&e.to_string()))?;
        found.boards.insert(Certificate::from_json(&json).map_err(|e| at(&e))?.n);
        found.lines.insert(line.to_owned());
    }
    Ok(found)
}

fn read_source(path: &Path) -> Result<Source, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (complete, torn) = complete_lines(&text);
    let mut lines = Vec::new();
    for (number, text) in complete {
        let at = |e: &str| format!("{}: line {number}: {e}", path.display());
        let json = Json::parse(text).map_err(|e| at(&e.to_string()))?;
        let certificate = Certificate::from_json(&json).map_err(|e| at(&e))?;
        let build = read_build(&json).map_err(|e| at(&format!("n={}: {e}", certificate.n)))?;
        lines.push(Line { number, text: text.to_owned(), certificate, build });
    }
    let counterexamples = read_counterexamples(&counterexample_path(path))?;
    Ok(Source { path: path.to_owned(), lines, torn, counterexamples })
}

// ---------------------------------------------------------------------------------------------------------
// Checking the lines

/// A line with a final verdict, in its file.
struct Final<'a> {
    source: &'a Source,
    line: &'a Line,
    /// For a line with `same_problem`: the index in the list of final lines of the last final line of the game it names
    /// before this one in the same file, if there is one.
    rests_on: Option<usize>,
}

impl Final<'_> {
    fn verdict(&self) -> Verdict {
        self.line.certificate.verdict
    }

    /// `path`, the line and what is wrong with it: how every refusal of a line begins.
    fn refusal(&self, why: impl fmt::Display) -> String {
        format!("{}: line {}: {why}", self.source.path.display(), self.line.number)
    }

    /// [`refusal`](Self::refusal) of a check of the line's game.
    fn fail(&self, why: impl fmt::Display) -> String {
        self.refusal(format!("n={}: {why}", self.line.certificate.n))
    }
}

/// What [`verify_certificates`] found in a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileReport {
    pub path: PathBuf,
    /// The complete lines that are not blank.
    pub lines: usize,
    /// The ones with a final verdict, and those that are `incomplete`.
    pub finals: usize,
    pub incomplete: usize,
    /// The `certified` ones made by a search, and those that rest on an earlier game's (`same_problem`).
    pub searched: usize,
    pub same_problem: usize,
    pub beaten: usize,
    /// The file ends in a line with no newline, which was ignored.
    pub torn: bool,
}

/// A game that beats a recorded score, from a `beaten` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counterexample {
    pub n: u32,
    /// The recorded score, and the score of the game that beats it.
    pub score: u64,
    pub found: u64,
    pub path: PathBuf,
    pub line: usize,
}

/// What [`verify_certificates`] found: every line passed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    pub files: Vec<FileReport>,
    /// The boards with a `certified` line (and no other final one).
    pub certified: BTreeSet<u32>,
    pub counterexamples: Vec<Counterexample>,
    /// The builds that wrote the lines (commit, dirty) and how many lines each wrote.
    pub builds: BTreeMap<(String, bool), usize>,
}

/// The cheap checks of a line that rests on another game (`same_problem`): the game is among the last
/// `SAME_SEARCH_WINDOW` games before it, the file has a final line of it before this one, and that line is
/// certified.
fn check_rests_on(f: &Final, rests_on: Option<&Final>) -> Result<(), String> {
    let c = &f.line.certificate;
    let (n, m) = (c.n, c.same_problem.as_ref().expect("a line that rests on a game").n);
    if c.verdict != Verdict::Certified {
        return Err(f.fail(format!("a {} line has no business resting on the game of n={m}", c.verdict.as_str())));
    }
    if !(n.saturating_sub(SAME_SEARCH_WINDOW)..n).contains(&m) {
        return Err(f.fail(format!("it rests on n={m}, which is none of the {SAME_SEARCH_WINDOW} games before it")));
    }
    let Some(game) = rests_on else {
        return Err(f.fail(format!("it rests on n={m}, but the file has no final line of n={m} before it")));
    };
    if game.verdict() != Verdict::Certified {
        let (number, verdict) = (game.line.number, game.verdict().as_str());
        return Err(f.fail(format!("it rests on n={m}, whose last final line before it (line {number}) is {verdict}")));
    }
    Ok(())
}

/// The checks of a line with a final verdict that need the data it was made from, in the order of the module docs.
/// Returns Π⁺(O) as the line was set up again, if `keep` asks for it.
fn check_line(f: &Final, known: &Optimal, upper: &BTreeMap<u32, Upper>, keep: bool) -> Result<Option<Problem>, String> {
    let c = &f.line.certificate;
    let n = c.n;

    let Some(bounds) = upper.get(&n) else { return Err(f.fail("the --upper files have no line for it")) };
    if !bounds.needs {
        return Err(f.fail("the --upper files say it needs no certificate"));
    }
    if bounds.open != c.open {
        return Err(f.fail(format!("its open members are {:?}, those of the --upper files {:?}", c.open, bounds.open)));
    }
    if bounds.score != c.score {
        return Err(f.fail(format!("its score is {}, that of the --upper files {}", c.score, bounds.score)));
    }

    let (Some(score), Some(moves)) = (known.score(n), known.moves(n)) else {
        return Err(f.fail("the results have no game of it"));
    };
    if score != c.score {
        return Err(f.fail(format!("its score is {}, that of the results' game {score}", c.score)));
    }
    let (words, width) = certificate_width(n, &c.open, c.claims).map_err(|e| f.refusal(e))?;
    if width != Some(c.width) {
        let gives = width.map_or_else(|| "none".to_owned(), |width| width.to_string());
        return Err(
            f.fail(format!("it records the width {}, but its {words} block words give the width {gives}", c.width))
        );
    }
    let beyond = || format!("n={n} is beyond the largest supported board, n={MAX_N}");
    let posed = crate::with_words!(c.width, W => {
        check_recorded_game::<W>(n, score, moves, &c.open, c.claims, None).map(|recorded: Recorded<W>| recorded.posed)
    }, else Err(beyond()))
    .map_err(|e| f.refusal(e))?;
    if !posed.reproduces(c) {
        let sizes = |frames, two_sided, one_sided, rejected, in_the_bag| {
            format!("{frames} frames, candidates {two_sided}+{one_sided}, {rejected} rejected, in_the_bag {in_the_bag}")
        };
        return Err(f.fail(format!(
            "Π⁺(O) set up again is not the line's: the line has the digest {} ({}), the problem set up again {} ({})",
            digest_text(c.digest),
            sizes(c.frames, c.two_sided, c.one_sided, c.rejected, c.in_the_bag),
            digest_text(posed.problem.digest()),
            sizes(posed.frames, posed.two_sided, posed.one_sided, posed.rejected, posed.in_the_bag),
        )));
    }

    let counterexamples = counterexample_path(&f.source.path);
    match c.verdict {
        Verdict::Certified => {
            if c.beaten_by.is_some() {
                return Err(f.fail("it is certified, but records a game that beats the score"));
            }
            if f.source.counterexamples.boards.contains(&n) {
                return Err(
                    f.fail(format!("it is certified, but {} has a counterexample for it", counterexamples.display()))
                );
            }
        }
        Verdict::Beaten => {
            if c.same_problem.is_some() {
                return Err(f.fail("it is beaten, which only a search says, but records the game it rests on"));
            }
            let Some((found, game)) = &c.beaten_by else { return Err(f.fail("it is beaten, but records no game")) };
            let scored =
                replay_finished(n, game).map_err(|e| f.fail(format!("the game that beats it is refused: {e}")))?;
            if scored != *found {
                return Err(f.fail(format!("the game that beats it scores {scored}, not the {found} recorded")));
            }
            if *found <= c.score {
                let why =
                    format!("the game that beats it scores {found}, not more than the recorded score {}", c.score);
                return Err(f.fail(why));
            }
            if !f.source.counterexamples.lines.contains(&f.line.text) {
                return Err(f.fail(format!("{} has no line identical to it", counterexamples.display())));
            }
        }
        Verdict::Incomplete => unreachable!("only lines with a final verdict are checked"),
    }
    Ok(keep.then_some(posed.problem))
}

/// The rule of the same problem applied again to the line `f`, which rests on `game`, as `certify` applied it: in the
/// formulation of `f`, whatever formulation `game`'s line was made in.  The problem `ours` of `f`, and `theirs` of
/// `game`, as the checks of the two lines set them up (each in its line's formulation), are equal whole in that
/// formulation (`game` set up again in it where its line is of the other), and the rule gives the `same_problem` the
/// line records.
fn check_rule(f: &Final, game: &Final, ours: &Problem, theirs: &Problem) -> Result<(), String> {
    let (c, m) = (&f.line.certificate, &game.line.certificate);
    let recorded = c.same_problem.as_ref().expect("a line that rests on a game");
    let (Some(worth), Some(best)) = (c.worth(), m.worth()) else {
        return Err(f.fail("a score below its in_the_bag cannot be a certificate"));
    };
    let theirs = if m.claims == c.claims {
        Cow::Borrowed(theirs)
    } else {
        let posed = extended_problem(m.n, &m.open, c.claims).map_err(|e| f.refusal(e))?;
        Cow::Owned(posed.problem)
    };
    match apply_rule(m, best, &theirs, ours, worth).map_err(|e| f.fail(e))? {
        Rule::Other => Err(f.fail(format!("its problem is not that of n={}, which it rests on", m.n))),
        Rule::Searched => Err(f.fail(format!(
            "its recorded game is worth {worth}, less than the {best} that the certificate of n={} proved the most: it \
             should have been searched",
            m.n
        ))),
        Rule::Settled(expected) if expected == *recorded => Ok(()),
        Rule::Settled(expected) => {
            let mut wrong = Vec::new();
            if recorded.digest != expected.digest {
                wrong.push(format!(
                    "digest {} (the problem's is {})",
                    digest_text(recorded.digest),
                    digest_text(expected.digest)
                ));
            }
            if recorded.best != expected.best {
                wrong.push(format!("best {} (the certificate of n={} proved {})", recorded.best, m.n, expected.best));
            }
            if recorded.worth != expected.worth {
                wrong.push(format!(
                    "worth {} (the line's score less its in_the_bag is {})",
                    recorded.worth, expected.worth
                ));
            }
            Err(f.fail(format!("its same_problem is wrong: {}", wrong.join(", "))))
        }
    }
}

/// Checks every line of the files of certificates `paths` (see the module docs) against the recorded games `known` and
/// the barred bounds `upper`, and returns what they hold.  The checks that set Π⁺(O) up again, the dear ones, run on
/// `threads` threads; the first line to fail is the same whatever `threads` is.
///
/// # Errors
///
/// If a file cannot be read or holds a complete line that is no certificate, or a line fails a check: the message names
/// the file, the line and the board.
pub fn verify_certificates(
    paths: &[PathBuf],
    known: &Optimal,
    upper: &BTreeMap<u32, Upper>,
    threads: usize,
) -> Result<Verified, String> {
    let sources = paths.iter().map(|path| read_source(path)).collect::<Result<Vec<_>, _>>()?;

    // The lines with a final verdict, in the order of the files, and where each one that rests on a game rests.
    let mut finals: Vec<Final> = Vec::new();
    for source in &sources {
        let mut last: BTreeMap<u32, usize> = BTreeMap::new();
        for line in source.lines.iter().filter(|line| line.certificate.verdict != Verdict::Incomplete) {
            let rests_on = line.certificate.same_problem.as_ref().and_then(|same| last.get(&same.n).copied());
            last.insert(line.certificate.n, finals.len());
            finals.push(Final { source, line, rests_on });
        }
    }

    // The cheap checks first: the games that lines rest on, and the boards with final lines in several places.
    for f in finals.iter().filter(|f| f.line.certificate.same_problem.is_some()) {
        check_rests_on(f, f.rests_on.map(|m| &finals[m]))?;
    }
    let mut by_board: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, f) in finals.iter().enumerate() {
        by_board.entry(f.line.certificate.n).or_default().push(i);
    }
    for places in by_board.values().filter(|places| places.len() > 1) {
        let Some(&bad) = places.iter().find(|&&i| finals[i].verdict() != Verdict::Certified) else { continue };
        let other = *places.iter().find(|&&i| i != bad).expect("more than one place");
        let (bad, other) = (&finals[bad], &finals[other]);
        let (path, number) = (other.source.path.display(), other.line.number);
        return Err(bad.fail(format!(
            "contradictory verdicts: {} here, and {} at {path} line {number}",
            bad.verdict().as_str(),
            other.verdict().as_str()
        )));
    }

    // The dear checks, line by line; the problems of the lines that rest on a game and of those they rest on are kept.
    let mut kept = HashSet::new();
    for (i, m) in finals.iter().enumerate().filter_map(|(i, f)| f.rests_on.map(|m| (i, m))) {
        kept.extend([i, m]);
    }
    let problems = in_parallel(finals.len(), threads, |i| check_line(&finals[i], known, upper, kept.contains(&i)))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    for (i, f) in finals.iter().enumerate().filter(|(_, f)| f.rests_on.is_some()) {
        let m = f.rests_on.expect("a line that rests on a game");
        let (ours, theirs) = (problems[i].as_ref(), problems[m].as_ref());
        let (ours, theirs) = ours.zip(theirs).expect("the problems of the games that rest on one another are kept");
        check_rule(f, &finals[m], ours, theirs)?;
    }
    Ok(report(&sources, &finals, &by_board))
}

/// What the checked files hold.
fn report(sources: &[Source], finals: &[Final], by_board: &BTreeMap<u32, Vec<usize>>) -> Verified {
    let mut verified = Verified {
        files: Vec::new(),
        certified: BTreeSet::new(),
        counterexamples: Vec::new(),
        builds: BTreeMap::new(),
    };
    for source in sources {
        let mut file = FileReport {
            path: source.path.clone(),
            lines: source.lines.len(),
            finals: 0,
            incomplete: 0,
            searched: 0,
            same_problem: 0,
            beaten: 0,
            torn: source.torn,
        };
        for line in &source.lines {
            let c = &line.certificate;
            match (c.verdict, c.same_problem.is_some()) {
                (Verdict::Incomplete, _) => file.incomplete += 1,
                (Verdict::Beaten, _) => file.beaten += 1,
                (Verdict::Certified, true) => file.same_problem += 1,
                (Verdict::Certified, false) => file.searched += 1,
            }
            *verified.builds.entry(line.build.clone()).or_default() += 1;
            if let (Verdict::Beaten, Some((found, _))) = (c.verdict, &c.beaten_by) {
                let (n, score, path, line) = (c.n, c.score, source.path.clone(), line.number);
                verified.counterexamples.push(Counterexample { n, score, found: *found, path, line });
            }
        }
        file.finals = file.searched + file.same_problem + file.beaten;
        verified.files.push(file);
    }
    for (&n, places) in by_board {
        if places.iter().all(|&i| finals[i].verdict() == Verdict::Certified) {
            verified.certified.insert(n);
        }
    }
    verified
}

/// The boards, ascending, as a list of ranges: `600-612, 615, 620-621`; `none` for no board.
pub fn compact_ranges(boards: impl IntoIterator<Item = u32>) -> String {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for n in boards {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == n => *last = n,
            _ => runs.push((n, n)),
        }
    }
    let show = |&(first, last): &(u32, u32)| if first == last { first.to_string() } else { format!("{first}-{last}") };
    if runs.is_empty() {
        "none".to_owned()
    } else {
        runs.iter().map(show).collect::<Vec<_>>().join(", ")
    }
}

impl fmt::Display for Verified {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for file in &self.files {
            let torn = if file.torn { "; a torn last line (ignored)" } else { "" };
            writeln!(
                f,
                "{}: {} lines, {} final, {} incomplete; certified by search {}, certified by the same problem {}, \
                 beaten {}{torn}",
                file.path.display(),
                file.lines,
                file.finals,
                file.incomplete,
                file.searched,
                file.same_problem,
                file.beaten
            )?;
        }
        writeln!(f, "boards certified: {} ({})", self.certified.len(), compact_ranges(self.certified.iter().copied()))?;
        if self.counterexamples.is_empty() {
            writeln!(f, "counterexamples: none")?;
        } else {
            writeln!(f, "counterexamples: {}", self.counterexamples.len())?;
            for c in &self.counterexamples {
                let (path, line) = (c.path.display(), c.line);
                writeln!(
                    f,
                    "  n={}: the recorded score {} is beaten by a game of {} ({path} line {line})",
                    c.n, c.score, c.found
                )?;
            }
        }
        writeln!(f, "builds that wrote the lines:")?;
        for ((commit, dirty), lines) in &self.builds {
            writeln!(f, "  {commit}{}: {lines} lines", if *dirty { " (dirty)" } else { "" })?;
        }
        write!(
            f,
            "not checked: that a search explored its whole tree (a certified line made by a search is taken on trust, \
             as an exhausted proof is)"
        )
    }
}

// ---------------------------------------------------------------------------------------------------------
// The proven chain

/// A line of the prover's proofs of the scores (`solver/noframe/data/exact_*.jsonl`), as far as [`frontier`] reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactLine {
    pub score: u64,
    /// The Franklín–Moniot bound on the score, and the parity bound where the line has one.
    pub fm_bound: u64,
    pub parity_bound: Option<u64>,
    /// `proven` where the prover's search finished, and whether it used the segment cuts.
    pub search: String,
    pub segments: bool,
    /// `chain` for a score at the ceiling n + score(n − 1), `prime` for one at the prime formula, else `none`.
    pub relation: String,
}

impl ExactLine {
    /// The board and the proof of line `json`, or `None` for a measurement (`fix_upper` set), which proves nothing.
    fn parse(json: &Json) -> Result<Option<(u32, ExactLine)>, String> {
        let get = |name: &str| json.get(name).ok_or_else(|| format!("missing \"{name}\""));
        let number = |name: &str| get(name)?.as_u64().ok_or_else(|| format!("bad \"{name}\""));
        let text =
            |name: &str| Ok::<_, String>(get(name)?.as_str().ok_or_else(|| format!("bad \"{name}\""))?.to_owned());
        // A flag as the prover's tools read it: absent, null, false and 0 are unset.
        let set = |name: &str| match json.get(name) {
            None | Some(Json::Null) => false,
            Some(value) => value.as_bool().unwrap_or_else(|| value.as_u64() != Some(0)),
        };
        if set("fix_upper") {
            return Ok(None);
        }
        let n = get("n")?.as_u32().ok_or("bad \"n\"")?;
        let parity_bound = match json.get("parity_bound") {
            None | Some(Json::Null) => None,
            Some(value) => Some(value.as_u64().ok_or("bad \"parity_bound\"")?),
        };
        let line = ExactLine {
            score: number("score")?,
            fm_bound: number("fm_bound")?,
            parity_bound,
            search: text("search")?,
            segments: set("segments"),
            relation: text("relation")?,
        };
        Ok(Some((n, line)))
    }

    /// Whether the line proves the score of its board: by the Franklín–Moniot bound or the parity bound, by a finished
    /// search (that used the segment cuts only if every smaller game is proven, `below`), or by a ceiling or the prime
    /// formula if the game before it is proven, `before`.
    fn proves(&self, before: bool, below: bool) -> bool {
        self.fm_bound == self.score
            || self.parity_bound == Some(self.score)
            || (self.search == "proven" && (!self.segments || below))
            || (matches!(self.relation.as_str(), "chain" | "prime") && before)
    }
}

/// The prover's proofs in the files `paths`, by board; a later file's line for a board replaces an earlier file's, and
/// so does a later line of one file.  Lines with `fix_upper` set are measurements and are skipped.
///
/// # Errors
///
/// If a file cannot be read or a line is no proof: the message names the file and the line.
pub fn read_exact(paths: &[PathBuf]) -> Result<BTreeMap<u32, ExactLine>, String> {
    let mut lines = BTreeMap::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
            let json = Json::parse(line).map_err(|e| at(e.to_string()))?;
            if let Some((n, proof)) = ExactLine::parse(&json).map_err(at)? {
                lines.insert(n, proof);
            }
        }
    }
    Ok(lines)
}

/// Why a game is not proven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    /// The results have no game of it.
    NoGame,
    /// It has no proof of the prover's, is not certified, and is not at a ceiling given a proven game before it.
    Unproven,
}

/// How far the proofs of the recorded scores reach ([`frontier`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frontier {
    /// Every game `1..=through` is proven; the largest board of the results where none is not.
    pub through: u32,
    /// The first game that is not proven, and why; `None` if every game of the results is.
    pub first_gap: Option<(u32, Gap)>,
    /// The games past the first gap that are proven all the same, ascending.
    pub beyond: Vec<u32>,
}

/// How far the proofs of the recorded scores reach, from plain data: the results' score of each board (`scores`), the
/// prover's proofs (`exact`, [`read_exact`]) and the boards that have a certificate (`certified`,
/// [`Verified::certified`]).
///
/// A game `n` of `1..=` the largest board of the results is *proven* when it is the game 1, or the prover proved it (a
/// line of `exact` for it whose `fm_bound` or `parity_bound` is its score, or whose search is `proven` (if it used the
/// segment cuts, only when every smaller game is proven), or whose relation is `chain` or `prime` when the game before
/// it is proven: `exact_proofs` of `solver/noframe/tools/summarize.py`), or it is certified, or its score reaches a
/// proven ceiling ([`CeilingProof::of`]) given the game before it, which is proven.  The result is the largest
/// `through` with every game to it proven, the first game after it that is not, and the games past that one that are
/// proven anyway.
///
/// # Errors
///
/// If a line of `exact` has another score for a board than the results do, naming the board and both scores.
pub fn frontier(
    scores: &BTreeMap<u32, u64>,
    exact: &BTreeMap<u32, ExactLine>,
    certified: &BTreeSet<u32>,
) -> Result<Frontier, String> {
    for (&n, line) in exact {
        if let Some(&score) = scores.get(&n).filter(|&&score| score != line.score) {
            return Err(format!(
                "n={n}: the prover's proof is of the score {}, the results' game scores {score}",
                line.score
            ));
        }
    }
    let last = scores.keys().next_back().copied().unwrap_or(0);
    let mut proven = vec![false; last as usize + 1];
    let mut gap: Option<(u32, Gap)> = None;
    for n in 1..=last {
        let Some(&score) = scores.get(&n) else {
            gap.get_or_insert((n, Gap::NoGame));
            continue;
        };
        let before = n > 1 && proven[n as usize - 1];
        let at_a_ceiling =
            before && scores.get(&(n - 1)).is_some_and(|&prev| CeilingProof::of(n, score, prev).is_some());
        let by_the_prover = exact.get(&n).is_some_and(|line| line.proves(before, gap.is_none()));
        proven[n as usize] = n == 1 || by_the_prover || certified.contains(&n) || at_a_ceiling;
        if !proven[n as usize] {
            gap.get_or_insert((n, Gap::Unproven));
        }
    }
    let through = gap.map_or(last, |(n, _)| n - 1);
    let beyond = (through + 1..=last).filter(|&n| proven[n as usize]).collect();
    Ok(Frontier { through, first_gap: gap, beyond })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boards_are_listed_as_ranges() {
        assert_eq!(compact_ranges([]), "none");
        assert_eq!(compact_ranges([7]), "7");
        assert_eq!(compact_ranges([600, 601, 602, 612, 615, 616, 620]), "600-602, 612, 615-616, 620");
    }

    #[test]
    fn a_build_needs_a_commit_of_forty_hexadecimal_digits_and_a_flag() {
        let commit = "0123456789abcdef0123456789ABCDEF01234567";
        let build = |text: &str| read_build(&Json::parse(&format!("{{\"build\":{text}}}")).unwrap());
        assert_eq!(build(&format!("{{\"commit\":\"{commit}\",\"dirty\":false}}")), Ok((commit.to_owned(), false)));
        for bad in [
            "null",
            "{}",
            "{\"commit\":null,\"dirty\":false}",
            "{\"commit\":\"abc\",\"dirty\":false}",
            "{\"dirty\":true}",
            "{\"commit\":\"0123456789abcdef0123456789abcdef0123456g\",\"dirty\":false}",
            &format!("{{\"commit\":\"{commit}\"}}"),
            &format!("{{\"commit\":\"{commit}\",\"dirty\":\"no\"}}"),
        ] {
            assert!(build(bad).is_err(), "{bad}");
        }
        assert!(read_build(&Json::parse("{}").unwrap()).unwrap_err().contains("no \"build\""));
    }

    #[test]
    fn a_proof_line_is_read_as_the_prover_summary_reads_it() {
        let line = |text: &str| ExactLine::parse(&Json::parse(text).unwrap());
        let plain = "\"n\":9,\"score\":30,\"fm_bound\":31,\"relation\":\"none\",\"search\":\"proven\"";
        let proof = ExactLine {
            score: 30,
            fm_bound: 31,
            parity_bound: None,
            search: "proven".into(),
            segments: false,
            relation: "none".into(),
        };
        assert_eq!(line(&format!("{{{plain}}}")), Ok(Some((9, proof.clone()))));
        let with = format!("{{{plain},\"parity_bound\":30,\"segments\":1,\"fix_upper\":0}}");
        let segmented = ExactLine { parity_bound: Some(30), segments: true, ..proof };
        assert_eq!(line(&with), Ok(Some((9, segmented))));
        // A measurement is skipped, however the flag is written; a bad or missing field is named.
        for flag in ["1", "true"] {
            assert_eq!(line(&format!("{{{plain},\"fix_upper\":{flag}}}")), Ok(None));
        }
        assert_eq!(line(&format!("{{{plain},\"parity_bound\":\"x\"}}")), Err("bad \"parity_bound\"".into()));
        assert_eq!(line("{\"n\":9,\"score\":30}"), Err("missing \"fm_bound\"".into()));
    }
}
