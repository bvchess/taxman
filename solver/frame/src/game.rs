//! The rules of Taxman as an independent referee, and the table of optimal games.
//!
//! # The rules
//!
//! The pot holds the numbers `1..=n`.  The player takes a number `m` that still has at least one proper divisor in
//! the pot and scores `m`; the taxman takes `m` and every one of its divisors still in the pot (the *tax*).  The
//! game is over when no number in the pot has a divisor left in the pot, and the taxman sweeps what remains.  A
//! game is its list of moves, its score is their sum, and an *optimal game* of `n` scores the most (`opt(n)`, the
//! Taxman sequence).  For `n = 4` the optimal game is `3, 4` (the taxman takes 1, then 2) and scores 7.
//!
//! # What is here
//!
//! * The **referee**: [`replay`] plays a list of moves and returns the score or says which move is illegal;
//!   [`replay_finished`] also demands what every optimal game shows, that the game is over and opens with the
//!   largest prime.  It is the last word on any game the crate produces: every solved game is replayed before it
//!   counts ([`crate::solver`]), and `taxman verify-optimal` and `verify-results` replay whole files.  It
//!   deliberately shares no code with the solver's number-theory tables ([`crate::numtheory`]): it uses only
//!   divisibility and a plain "is it still in the pot" array, so a bug in a table cannot make a wrong game look
//!   legal.  [`replay_punctured`] is the same referee for a board with holes (study code, [`crate::sacrifice`]).
//! * [`Optimal`]: the table of optimal games `optimal.json` (an optimal game for every `n` it reaches: the Java
//!   solver's for 1 to 1000, then the games of the chain `results/strict.jsonl`, which `taxman extend-optimal`
//!   appends), built into the binary ([`Optimal::built_in`]).  The solver and the verifiers confirm every score
//!   they meet against it, and the solver can take the previous game from it when it solves the board just past a
//!   game the table has.  It is a table, not a proof: the Java made its part with the conjectures `reuse_prev` and
//!   `revise_freedoms` (since removed here), and the file does not say which games are whose ([`crate::results`]
//!   records what a game taken from it is assumed to be).  The file is tens of megabytes, so the binary reads a game
//!   of it when the game is asked for and no sooner (see [`Optimal::built_in`]).

use crate::json::Json;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Plays `moves` on the board `1..=n` and returns the player's score (the sum of the moves).
///
/// A move is legal if it is a number on the board that is still in the pot and at least one of its proper
/// divisors is still in the pot.  Making it removes the number and all of its divisors from the pot (the
/// taxman takes the divisors).  The sequence need not be a complete game: legal moves may remain at the end.
pub fn replay(n: u32, moves: &[u32]) -> Result<u64, String> {
    play(n, moves).map(|(score, _)| score)
}

/// The moves still legal after `moves` (which must be legal): the numbers in the pot with a proper divisor in
/// the pot, ascending.
pub fn legal_moves_left(n: u32, moves: &[u32]) -> Result<Vec<u32>, String> {
    play(n, moves).map(|(_, in_pot)| legal_in(n, &in_pot))
}

/// `moves` followed by legal moves for as long as there are any, the largest each time: the game over.  One descending
/// pass finds them all, for a move only takes numbers out of the pot, so a number that is not legal now never
/// becomes legal.  (After a game of n − 1 followed by n, or a game of n + 1 less its move n + 1, these are the
/// completions of monotonicity (N3) and the ceiling lemma (N4), docs/theory/neighbors.md.)
pub fn complete(n: u32, moves: &[u32]) -> Result<Vec<u32>, String> {
    let (_, mut in_pot) = play(n, moves)?;
    let mut game = moves.to_vec();
    for m in (2..=n).rev() {
        if in_pot[m as usize] && proper_divisors(m).any(|d| in_pot[d as usize]) {
            proper_divisors(m).for_each(|d| in_pot[d as usize] = false);
            in_pot[m as usize] = false;
            game.push(m);
        }
    }
    Ok(game)
}

/// The numbers of the pot `in_pot` that have a proper divisor in it, ascending.
fn legal_in(n: u32, in_pot: &[bool]) -> Vec<u32> {
    let mut legal = vec![false; in_pot.len()];
    for d in (1..=n).filter(|&d| in_pot[d as usize]) {
        for m in (2 * d..=n).step_by(d as usize) {
            legal[m as usize] |= in_pot[m as usize];
        }
    }
    (1..=n).filter(|&m| legal[m as usize]).collect()
}

/// Referees `moves` as a finished game of `n` and returns its score.  Besides being legal, the game must have
/// the two properties every optimal game has, which it can show by itself:
///
/// * it is over: no legal move is left (one more move would only add to the score);
/// * for `n ≥ 2`, it opens with the largest prime `≤ n`.  A prime can be played only while 1 is in the pot, so
///   only as the first move, and an optimal game plays the largest one (a short exchange argument, proved with
///   the prime shortcut it supports in [`crate::accelerate::prime_shortcut`]).
pub fn replay_finished(n: u32, moves: &[u32]) -> Result<u64, String> {
    let (score, in_pot) = play(n, moves)?;
    let left = legal_in(n, &in_pot);
    if !left.is_empty() {
        let shown: Vec<String> = left.iter().take(8).map(u32::to_string).collect();
        let more = if left.len() > shown.len() { ", ..." } else { "" };
        return Err(format!("the game is not over: {} legal moves are left ({}{more})", left.len(), shown.join(", ")));
    }
    if let Some(prime) = largest_prime_at_most(n) {
        if moves.first() != Some(&prime) {
            let first = moves.first().map_or_else(|| "nothing".to_owned(), u32::to_string);
            return Err(format!("the game opens with {first}, not with {prime}, the largest prime up to {n}"));
        }
    }
    Ok(score)
}

/// The largest prime `≤ n` (`None` for `n < 2`).
pub fn largest_prime_at_most(n: u32) -> Option<u32> {
    (2..=n).rev().find(|&p| is_prime(p))
}

/// [`replay`], also returning which numbers are still in the pot (indexed by number; 0 is never in it).
fn play(n: u32, moves: &[u32]) -> Result<(u64, Vec<bool>), String> {
    let mut in_pot = vec![true; n as usize + 1];
    in_pot[0] = false;
    let mut score = 0u64;
    for (i, &m) in moves.iter().enumerate() {
        let turn = i + 1;
        if m < 1 || m > n {
            return Err(format!("move {turn} ({m}): not on the board 1..={n}"));
        }
        if !in_pot[m as usize] {
            return Err(format!("move {turn} ({m}): already taken"));
        }
        // The taxman takes every proper divisor still in the pot; the move is legal only if there was one.
        // (Clearing before the check is harmless: an illegal move ends the replay.)
        let mut pays_tax = false;
        for d in proper_divisors(m) {
            pays_tax |= in_pot[d as usize];
            in_pot[d as usize] = false;
        }
        if !pays_tax {
            return Err(format!("move {turn} ({m}): no proper divisor left in the pot, so it pays no tax"));
        }
        in_pot[m as usize] = false;
        score += u64::from(m);
    }
    Ok((score, in_pot))
}

/// The proper divisors of `m` (those below it that divide it), in no particular order: by trial division up to √m, each
/// divisor found with its partner `m / d`.  Plain divisibility, as the rest of the referee, but a move costs √m
/// divisions rather than m, which a board of thousands of numbers feels (the ledger replays a game per board).
fn proper_divisors(m: u32) -> impl Iterator<Item = u32> {
    (1..)
        .take_while(move |&d| u64::from(d) * u64::from(d) <= u64::from(m))
        .filter(move |&d| m.is_multiple_of(d))
        .flat_map(move |d| [Some(d), Some(m / d).filter(|&partner| partner != d)].into_iter().flatten())
        .filter(move |&d| d < m)
}

/// Plays `moves` on the board `1..=n` without `holes` under the *maximal-factor rule*, and returns the score.
/// Study code: only [`crate::sacrifice`] and its tests use it.
///
/// A hole is never in the pot.  A move is legal if it is still in the pot and one of its maximal factors (`m/p`
/// for a prime `p | m`) is still in the pot; it removes the number and all of its divisors.  On the board `1..=n`
/// this is the same game as [`replay`]'s (a divisor in the pot lies below a maximal factor in the pot: the
/// lifting argument of [`crate::numtheory`]).  On a punctured board it is stricter: a move whose maximal factor
/// is a hole may not pay with a divisor of the hole.  It is the game the frame model plays on a punctured board
/// ([`crate::sacrifice`]).  Like [`replay`], it uses only divisibility.
pub fn replay_punctured(n: u32, holes: &[u32], moves: &[u32]) -> Result<u64, String> {
    let mut in_pot = vec![true; n as usize + 1];
    in_pot[0] = false;
    for &h in holes {
        if (1..=n).contains(&h) {
            in_pot[h as usize] = false;
        }
    }
    let mut score = 0u64;
    for (i, &m) in moves.iter().enumerate() {
        let turn = i + 1;
        if m < 1 || m > n {
            return Err(format!("move {turn} ({m}): not on the board 1..={n}"));
        }
        if !in_pot[m as usize] {
            let why = if holes.contains(&m) { "a hole" } else { "already taken" };
            return Err(format!("move {turn} ({m}): {why}"));
        }
        let pays_tax = (2..=m).filter(|&p| m % p == 0 && is_prime(p)).any(|p| in_pot[(m / p) as usize]);
        if !pays_tax {
            return Err(format!("move {turn} ({m}): no maximal factor left in the pot, so it pays no tax"));
        }
        for d in (1..m).filter(|d| m % d == 0) {
            in_pot[d as usize] = false;
        }
        in_pot[m as usize] = false;
        score += u64::from(m);
    }
    Ok(score)
}

/// Trial division.
fn is_prime(p: u32) -> bool {
    p >= 2 && (2..).take_while(|q| q * q <= p).all(|q| !p.is_multiple_of(q))
}

/// Sanity bound on `n` when reading a file of games, so a corrupt file cannot request a huge allocation.
const MAX_PLAUSIBLE_N: u32 = 1 << 20;

/// The text of `results/optimal.json` when the crate was compiled: the table
/// [`Optimal::built_in`] reads.
pub const BUILT_IN: &str = include_str!("../../../results/optimal.json");

/// The score of a game and its moves.
type Recorded = (u64, Vec<u32>);

/// How the file `optimal.json` starts a line of a game, up to its n: `  { "n": 4, "score": 7, "moves": [3, 4] }`.
const GAME_LINE_START: &str = "  { \"n\": ";

/// The known optimal games, by n: the table `results/optimal.json` ([`built_in`](Self::built_in)), possibly
/// overlaid with the completed games of results files ([`with_results`](Self::with_results)).  For each n, the
/// best score and a game that achieves it.
#[derive(Clone, Default)]
pub struct Optimal {
    /// The games of the text the table was built on (the built-in file's), each found and parsed when it is asked for.
    text: Text,
    /// The games set over those of the text, by n; all the games of a table read from a file.
    overlay: BTreeMap<u32, Recorded>,
}

/// The games of a file in the format [`Optimal::to_file_text`] writes, one game a line in ascending order of n, which
/// are found by bisection and parsed when they are asked for.  A process that needs a few games of a file of tens of
/// megabytes then reads a few lines of it, and one that needs none reads nothing.
#[derive(Clone, Default)]
struct Text {
    /// The lines of the games, joined by newlines, without the brackets of the array around them.
    lines: &'static str,
    /// What was found for each n from 0 to the largest n of the text: `None` where the text has no game of n.
    found: Vec<OnceLock<Option<Recorded>>>,
}

impl Text {
    /// The games of a file as `to_file_text` writes it.
    ///
    /// # Errors
    ///
    /// If the file is not in that format, or its last line is not a game of a plausible n.
    fn new(file: &'static str) -> Result<Text, String> {
        let body = file.strip_prefix("[\n").ok_or("the file does not start with a bracket and a newline")?;
        let lines = match body {
            "]" => "",
            _ => body.strip_suffix("\n]").ok_or("the file does not end with a newline and a bracket")?,
        };
        let largest = match lines.rsplit('\n').next().filter(|line| !line.is_empty()) {
            Some(last) => line_n(last)?,
            None => return Ok(Text::default()),
        };
        if largest > MAX_PLAUSIBLE_N {
            return Err(format!(
                "the last game is of n={largest}, more than the {MAX_PLAUSIBLE_N} that a table may hold"
            ));
        }
        Ok(Text { lines, found: vec![OnceLock::new(); largest as usize + 1] })
    }

    /// The largest n of the text, 0 if it has no game.
    fn max_n(&self) -> u32 {
        self.found.len().saturating_sub(1) as u32
    }

    /// The game of `n`, if the text has one, read the first time it is asked for.
    ///
    /// # Panics
    ///
    /// If the line it finds is not the line of a game.  (A text whose lines are not in ascending order of n loses the
    /// games that the bisection steps over; the tests check that the built-in text is in order.)
    fn get(&self, n: u32) -> Option<&Recorded> {
        let cell = self.found.get(n as usize)?;
        cell.get_or_init(|| self.read(n).unwrap_or_else(|e| panic!("the built-in optimal.json: {e}"))).as_ref()
    }

    /// The game of `n`, parsed from its line.
    fn read(&self, n: u32) -> Result<Option<Recorded>, String> {
        let Some(line) = self.line_of(n)? else { return Ok(None) };
        let at = |e: String| format!("the line of n={n}{e}");
        let json = Json::parse(line.trim_end_matches(',')).map_err(|e| at(format!(": {e}")))?;
        let (found, score, moves) = Optimal::parse_entry(&json).map_err(at)?;
        if found != n {
            return Err(format!("the line found for n={n} is the game of {found}"));
        }
        Ok(Some((score, moves)))
    }

    /// The line of the game of `n`, if the text has one: a bisection of the text, whose lines are in ascending order of
    /// n, so it touches a few of the lines.
    fn line_of(&self, n: u32) -> Result<Option<&str>, String> {
        let bytes = self.lines.as_bytes();
        // Invariant: the game of n, if there is one, is on a line within bytes[lo..hi], and lo and hi are the start of
        // a line (or the end of the text), so that every line in the range is whole.
        let (mut lo, mut hi) = (0, bytes.len());
        while lo < hi {
            let middle = lo + (hi - lo) / 2;
            let start = bytes[..middle].iter().rposition(|&b| b == b'\n').map_or(0, |newline| newline + 1);
            let end = bytes[middle..].iter().position(|&b| b == b'\n').map_or(bytes.len(), |newline| middle + newline);
            let line = &self.lines[start..end];
            match line_n(line)?.cmp(&n) {
                std::cmp::Ordering::Less => lo = end + 1,
                std::cmp::Ordering::Greater => hi = start,
                std::cmp::Ordering::Equal => return Ok(Some(line)),
            }
        }
        Ok(None)
    }
}

/// The n of a line of a game as `to_file_text` writes it.
fn line_n(line: &str) -> Result<u32, String> {
    let digits = line.strip_prefix(GAME_LINE_START).ok_or_else(|| format!("not the line of a game: {line:.60}"))?;
    let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
    digits[..end].parse().map_err(|_| format!("no n at the start of the line of a game: {line:.60}"))
}

impl Optimal {
    /// The table of optimal games built into this binary: `results/optimal.json` as it was when the crate
    /// was compiled.  The solver and the verifiers confirm every score against it, wherever the binary
    /// runs; a changed file takes a rebuild (`build.rs` marks the build dirty until it is committed).
    ///
    /// The file is tens of megabytes, so nothing of it is read until a game is asked for, and then that game only: its
    /// line is found by bisection (the lines are one game each, in ascending order of n, as [`to_file_text`] writes
    /// them) and parsed, once.  A process that never uses the table pays nothing for it, and one that uses a few games
    /// pays for those.  The whole table is parsed by what goes over every game ([`games`](Self::games),
    /// [`verify_all`](Self::verify_all)).
    ///
    /// [`to_file_text`]: Self::to_file_text
    pub fn built_in() -> &'static Optimal {
        static TABLE: OnceLock<Optimal> = OnceLock::new();
        TABLE.get_or_init(|| {
            let text = Text::new(BUILT_IN)
                .unwrap_or_else(|e| panic!("the built-in optimal.json is not as `extend-optimal` writes it: {e}"));
            Optimal { text, overlay: BTreeMap::new() }
        })
    }

    /// The table with the completed games of the results files `results` added (they replace the table's moves for
    /// the same values of n; an unfinished search's line is skipped): how a measurement (the ladder, `taxman
    /// bench`) gets the previous game of a board past the table's end, to seed its search and give it a ceiling.
    ///
    /// # Errors
    ///
    /// If a results file cannot be read, or a game of one scores differently from one the games before it have.
    pub fn with_results_files(&self, results: &[PathBuf]) -> Result<Optimal, String> {
        let mut known = self.clone();
        for path in results {
            let games = Self::completed_games(path)?;
            // The first game, in ascending order of n, that the file scores differently from the games known so far.
            let differs = games.iter().find_map(|(&n, &(now, _))| {
                known.score(n).filter(|&before| before != now).map(|before| (n, now, before))
            });
            if let Some((n, now, before)) = differs {
                return Err(format!("{}: n={n} scores {now}, but the known games have {before}", path.display()));
            }
            for (n, (score, moves)) in games {
                known.set(n, score, moves);
            }
        }
        Ok(known)
    }

    /// Loads a file holding an array of `{"n": .., "score": .., "moves": [..]}` objects.
    pub fn load(path: impl AsRef<Path>) -> Result<Optimal, String> {
        Optimal::from_json(&Json::from_file(path)?)
    }

    /// Reads a parsed JSON array of game objects (see [`load`](Self::load)); the error names the entry that is
    /// malformed, or an n that appears twice.
    pub fn from_json(json: &Json) -> Result<Optimal, String> {
        let entries = json.as_arr().ok_or("expected an array of games")?;
        let mut out = Optimal::default();
        for (i, entry) in entries.iter().enumerate() {
            let (n, score, moves) = Self::parse_entry(entry).map_err(|e| format!("entry {i}{e}"))?;
            if out.overlay.insert(n, (score, moves)).is_some() {
                return Err(format!("entry {i}: duplicate n={n}"));
            }
        }
        Ok(out)
    }

    /// These games with the completed games of the results file `path` (one JSON object per line, as `taxman
    /// solve --out` writes them), which replace the games of the same values of n.  Lines of searches that did
    /// not complete are skipped.
    pub fn with_results(mut self, path: &Path) -> Result<Optimal, String> {
        for (n, (score, moves)) in Self::completed_games(path)? {
            self.set(n, score, moves);
        }
        Ok(self)
    }

    /// The completed games of the results file `path` by n, the last of a game that the file has twice.
    fn completed_games(path: &Path) -> Result<BTreeMap<u32, Recorded>, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut games = BTreeMap::new();
        for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let at = |e: String| format!("{}: line {}{e}", path.display(), i + 1);
            let game = Json::parse(line).map_err(|e| at(format!(": {e}")))?;
            if game.get("completed").and_then(Json::as_bool) == Some(true) {
                let (n, score, moves) = Self::parse_entry(&game).map_err(at)?;
                games.insert(n, (score, moves));
            }
        }
        Ok(games)
    }

    /// The n, score and moves of one game object; the error says what is wrong after a ": " (or after " (n=…):
    /// " once n is known).
    fn parse_entry(entry: &Json) -> Result<(u32, u64, Vec<u32>), String> {
        let field = |name: &str| entry.get(name).ok_or_else(|| format!(": missing \"{name}\""));
        let n = field("n")?.as_u32().filter(|&n| n <= MAX_PLAUSIBLE_N).ok_or(": bad \"n\"")?;
        let score = field("score")?.as_u64().ok_or_else(|| format!(" (n={n}): bad \"score\""))?;
        let moves = field("moves")?.as_u32_vec().ok_or_else(|| format!(" (n={n}): bad \"moves\""))?;
        Ok((n, score, moves))
    }

    /// Records game `n`, over the text's if it has one.
    fn set(&mut self, n: u32, score: u64, moves: Vec<u32>) {
        self.overlay.insert(n, (score, moves));
    }

    /// Appends game `n`, which must be past every n the table has: a table is extended past its end, and its
    /// games are never replaced.
    pub fn push(&mut self, n: u32, score: u64, moves: Vec<u32>) -> Result<(), String> {
        if n <= self.max_n() {
            return Err(format!("n={n} is not past the table's largest n, {}", self.max_n()));
        }
        if n > MAX_PLAUSIBLE_N {
            return Err(format!("n={n} is more than the {MAX_PLAUSIBLE_N} that a table of games may hold"));
        }
        self.set(n, score, moves);
        Ok(())
    }

    /// The table as the file `optimal.json` is written: an array with one game per line, `  { "n": 4, "score": 7,
    /// "moves": [3, 4] }`, a comma after every line but the last, and no newline after the closing bracket.
    /// [`Optimal::from_json`] reads the text back, and the text of the loaded `optimal.json` is that file byte
    /// for byte.
    pub fn to_file_text(&self) -> String {
        let lines: Vec<String> = self
            .games()
            .map(|(n, score, moves)| {
                let moves: Vec<String> = moves.iter().map(u32::to_string).collect();
                format!("{GAME_LINE_START}{n}, \"score\": {score}, \"moves\": [{}] }}", moves.join(", "))
            })
            .collect();
        if lines.is_empty() {
            "[\n]".to_owned()
        } else {
            format!("[\n{}\n]", lines.join(",\n"))
        }
    }

    /// Writes [`to_file_text`](Self::to_file_text) to `path` all or nothing: the text goes to `<path>.tmp`, which
    /// is synced and renamed over `path` (as the engine writes a checkpoint), so a write that fails or is
    /// interrupted leaves the old file as it was.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        let written = File::create(&tmp)
            .and_then(|mut file| file.write_all(self.to_file_text().as_bytes()).and_then(|()| file.sync_all()))
            .and_then(|()| fs::rename(&tmp, path));
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(format!("{}: {e}", path.display()));
        }
        // Make the rename durable; not every platform can sync a directory, so this is best effort.
        let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        if let Ok(dir) = File::open(dir) {
            let _ = dir.sync_all();
        }
        Ok(())
    }

    /// The `(score, moves)` recorded for `n`, if any: a game set over the text's, else the text's.
    fn entry(&self, n: u32) -> Option<&Recorded> {
        self.overlay.get(&n).or_else(|| self.text.get(n))
    }

    /// The optimal score for game `n`, if known.
    pub fn score(&self, n: u32) -> Option<u64> {
        self.entry(n).map(|(score, _)| *score)
    }

    /// An optimal game for `n`, if known.
    pub fn moves(&self, n: u32) -> Option<&[u32]> {
        self.entry(n).map(|(_, moves)| moves.as_slice())
    }

    /// Every entry as `(n, score, moves)`, ascending in `n`.  It reads every game of a table built on a text.
    pub fn games(&self) -> impl Iterator<Item = (u32, u64, &[u32])> + '_ {
        (0..=self.max_n()).filter_map(|n| self.entry(n).map(|(score, moves)| (n, *score, moves.as_slice())))
    }

    /// The values of n that have an entry, ascending.  It reads every game of a table built on a text.
    pub fn sizes(&self) -> impl Iterator<Item = u32> + '_ {
        self.games().map(|(n, _, _)| n)
    }

    /// The largest n with an entry (0 if there are none).
    pub fn max_n(&self) -> u32 {
        self.overlay.keys().next_back().copied().unwrap_or(0).max(self.text.max_n())
    }

    /// Replays every game and checks that it is legal and scores what the file says.  Returns the number of
    /// games checked.
    pub fn verify_all(&self) -> Result<usize, String> {
        let mut checked = 0;
        for (n, score, moves) in self.games() {
            match replay(n, moves) {
                Ok(replayed) if replayed == score => checked += 1,
                Ok(replayed) => return Err(format!("n={n}: recorded score {score} but the moves score {replayed}")),
                Err(e) => return Err(format!("n={n}: {e}")),
            }
        }
        Ok(checked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn replay_scores_legal_games() {
        assert_eq!(replay(0, &[]), Ok(0));
        assert_eq!(replay(1, &[]), Ok(0));
        assert_eq!(replay(2, &[2]), Ok(2));
        assert_eq!(replay(4, &[3, 4]), Ok(7));
        assert_eq!(replay(5, &[5, 4]), Ok(9));
        assert_eq!(replay(6, &[5, 4, 6]), Ok(15));
        // Legal, and already a complete game: 4 takes 1 and 2, and 3 can no longer be played.
        assert_eq!(replay(4, &[4]), Ok(4));
        // A sequence need not be complete: 6 alone is legal even though 4 and 5 are left (4 has no tax left).
        assert_eq!(replay(6, &[6]), Ok(6));
    }

    #[test]
    fn replay_rejects_moves_off_the_board() {
        assert!(replay(5, &[0]).unwrap_err().contains("not on the board"));
        assert!(replay(5, &[6]).unwrap_err().contains("not on the board"));
        assert!(replay(0, &[1]).unwrap_err().contains("not on the board"));
        assert!(replay(5, &[5, 4, 100]).unwrap_err().contains("move 3"));
    }

    #[test]
    fn replay_rejects_numbers_not_in_the_pot() {
        assert!(replay(4, &[3, 3]).unwrap_err().contains("already taken"));
        // 6 takes 1, 2 and 3 from the pot, so 3 is gone.
        assert!(replay(6, &[6, 3]).unwrap_err().contains("already taken"));
        // 2 is taxed by the move 4.
        assert!(replay(4, &[4, 2]).unwrap_err().contains("already taken"));
    }

    #[test]
    fn replay_rejects_moves_that_pay_no_tax() {
        // 1 never has a proper divisor.
        assert!(replay(4, &[1]).unwrap_err().contains("pays no tax"));
        // After 2 takes 1, 4 has no divisor left in the pot.
        let err = replay(10, &[2, 4]).unwrap_err();
        assert!(err.contains("move 2") && err.contains("pays no tax"), "{err}");
        // A prime can only be played while 1 is still there.
        assert!(replay(7, &[4, 7]).unwrap_err().contains("pays no tax"));
        assert_eq!(replay(7, &[7, 4]), Ok(11));
    }

    #[test]
    fn the_moves_left_and_a_finished_game() {
        // After 7 (takes 1), 4 (takes 2) and 6 (takes 3), only 5 is in the pot, and it has no divisor left.
        assert_eq!(legal_moves_left(7, &[7, 4, 6]), Ok(vec![]));
        assert_eq!(replay_finished(7, &[7, 4, 6]), Ok(17));
        // After 7 alone, every number from 4 up still has its divisor 2 or 3 in the pot.
        assert_eq!(legal_moves_left(7, &[7]), Ok(vec![4, 6]));
        assert!(replay_finished(7, &[7]).unwrap_err().contains("2 legal moves are left (4, 6)"));
        // Over, but opened with the wrong prime: 5 takes 1, so 7 can never be played.
        assert_eq!(legal_moves_left(7, &[5, 4, 6]), Ok(vec![]));
        assert!(replay_finished(7, &[5, 4, 6]).unwrap_err().contains("opens with 5, not with 7"));
        // The game of 1 has no moves, and an empty game of 2 is not over.
        assert_eq!(replay_finished(1, &[]), Ok(0));
        assert!(replay_finished(2, &[]).unwrap_err().contains("not over"));
        assert!(replay_finished(4, &[1]).unwrap_err().contains("pays no tax"));
        assert_eq!(
            (largest_prime_at_most(1), largest_prime_at_most(2), largest_prime_at_most(90)),
            (None, Some(2), Some(89))
        );
    }

    /// The proper divisors of `m` by a sweep over every number below it: the referee's divisor sweep before it went up
    /// to √m, kept to hold the fast one to.
    fn proper_divisors_naive(m: u32) -> Vec<u32> {
        (1..m).filter(|&d| m.is_multiple_of(d)).collect()
    }

    #[test]
    fn the_divisors_up_to_the_root_are_the_proper_divisors() {
        for m in 0..=3000 {
            let mut fast: Vec<u32> = proper_divisors(m).collect();
            fast.sort_unstable();
            assert_eq!(fast, proper_divisors_naive(m), "m={m}");
        }
        // A square (4 is the partner of itself) and a number near the end of the range.
        assert_eq!(proper_divisors(36).count(), 8);
        assert_eq!(proper_divisors(u32::MAX).count(), 31);
    }

    /// [`play`] with the divisor sweep of every number below the move, as it was before: its scores, pots and errors
    /// are the fast one's, on the games of the table and on those games with their moves swapped, dropped, repeated and
    /// replaced (most of them illegal).
    #[test]
    fn the_fast_referee_plays_as_the_slow_one_did() {
        fn play_slow(n: u32, moves: &[u32]) -> Result<(u64, Vec<bool>), String> {
            let mut in_pot = vec![true; n as usize + 1];
            in_pot[0] = false;
            let mut score = 0u64;
            for (i, &m) in moves.iter().enumerate() {
                let turn = i + 1;
                if m < 1 || m > n {
                    return Err(format!("move {turn} ({m}): not on the board 1..={n}"));
                }
                if !in_pot[m as usize] {
                    return Err(format!("move {turn} ({m}): already taken"));
                }
                let mut pays_tax = false;
                for d in proper_divisors_naive(m) {
                    pays_tax |= in_pot[d as usize];
                    in_pot[d as usize] = false;
                }
                if !pays_tax {
                    return Err(format!("move {turn} ({m}): no proper divisor left in the pot, so it pays no tax"));
                }
                in_pot[m as usize] = false;
                score += u64::from(m);
            }
            Ok((score, in_pot))
        }
        let opt = Optimal::built_in();
        let mut seed = 12345u64;
        let mut random = |bound: usize| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as usize % bound
        };
        let (mut legal, mut illegal) = (0, 0);
        // The games to 1300: the slow referee takes about n² a move.
        for n in (2..=SMALL_GAMES).step_by(37).chain([60, 61, 64, 120]) {
            let game = opt.moves(n).unwrap().to_vec();
            let mut games = vec![game.clone(), game[..game.len() / 2].to_vec()];
            for _ in 0..6 {
                let (i, j) = (random(game.len()), random(game.len()));
                let mut swapped = game.clone();
                swapped.swap(i, j);
                let mut dropped = game.clone();
                dropped.remove(i);
                let mut repeated = game.clone();
                repeated.insert(j, game[i]);
                let mut replaced = game.clone();
                replaced[i] = random(n as usize + 2) as u32;
                games.extend([swapped, dropped, repeated, replaced]);
            }
            for moves in games {
                let (fast, slow) = (play(n, &moves), play_slow(n, &moves));
                assert_eq!(fast, slow, "n={n}, moves {moves:?}");
                if fast.is_ok() {
                    legal += 1;
                } else {
                    illegal += 1;
                }
            }
        }
        assert!(legal > 100 && illegal > 100, "{legal} legal and {illegal} illegal games");
    }

    /// `complete` is the largest legal move, again and again: it finishes every prefix of a table game the way the
    /// slow loop (the moves left, recomputed after each) does, to a game that is over.
    #[test]
    fn a_game_is_completed_by_the_largest_legal_moves() {
        let opt = Optimal::built_in();
        for n in (2..=400).step_by(7) {
            let game = opt.moves(n).unwrap();
            for len in [1, game.len() / 3, game.len() / 2, game.len()] {
                let prefix = &game[..len.max(1)];
                let mut slow = prefix.to_vec();
                while let Some(&m) = legal_moves_left(n, &slow).unwrap().last() {
                    slow.push(m);
                }
                let fast = complete(n, prefix).unwrap();
                assert_eq!(fast, slow, "n={n}, prefix of {len} moves");
                assert!(replay_finished(n, &fast).is_ok(), "n={n}, prefix of {len} moves");
            }
        }
        // After 7 (which takes 1) the legal moves are 4 and 6; the largest, 6, takes 2 and 3 and leaves 4 without a divisor.
        assert_eq!(complete(7, &[7]), Ok(vec![7, 6]));
        assert!(complete(4, &[1]).is_err(), "an illegal prefix is refused");
    }

    /// The games of the table that a test goes over every one of: 1 to 1300.  The table reaches 5000, and parsing and
    /// replaying all of it takes seconds, so a test of every game of it is ignored in a debug build.
    const SMALL_GAMES: u32 = 1300;

    /// The first `count` games of the built-in file, as a file of their own.
    fn file_of_the_first(count: usize) -> String {
        let lines: Vec<&str> = BUILT_IN.lines().skip(1).take(count).collect();
        format!("[\n{}\n]", lines.join("\n").trim_end_matches(','))
    }

    /// The table on `file`, which stays for the rest of the process.
    fn table_on(file: String) -> Optimal {
        let text = Text::new(Box::leak(file.into_boxed_str())).unwrap_or_else(|e| panic!("{e}"));
        Optimal { text, overlay: BTreeMap::new() }
    }

    #[test]
    fn the_first_optimal_games_are_finished() {
        let opt = Optimal::built_in();
        for (n, score, moves) in opt.games().take(SMALL_GAMES as usize) {
            assert_eq!(replay_finished(n, moves), Ok(score), "n={n}");
        }
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is the_first_optimal_games_are_finished")]
    fn every_optimal_game_is_finished() {
        let opt = Optimal::built_in();
        for (n, score, moves) in opt.games() {
            assert_eq!(replay_finished(n, moves), Ok(score), "n={n}");
        }
    }

    #[test]
    fn optimal_json_loads() {
        let start = Instant::now();
        let opt = Optimal::built_in();
        let top = opt.max_n();
        // n = 0 and anything past the file are unknown.
        assert_eq!((opt.score(0), opt.moves(0), opt.score(top + 1), opt.moves(top + 1)), (None, None, None, None));
        // n = 1 has score 0 and no moves.
        assert_eq!((opt.score(1), opt.moves(1)), (Some(0), Some(&[][..])));
        assert_eq!((opt.score(2), opt.moves(2)), (Some(2), Some(&[2][..])));
        assert_eq!((opt.score(4), opt.moves(4)), (Some(7), Some(&[3, 4][..])));
        assert_eq!((opt.score(6), opt.moves(6)), (Some(15), Some(&[5, 4, 6][..])));
        assert!(opt.score(top).is_some() && opt.moves(top).is_some_and(|moves| !moves.is_empty()));
        // The bound is generous: it only guards against a read of the whole file (tens of megabytes) for these.
        assert!(start.elapsed().as_secs_f64() < 1.0, "loading took {:?}", start.elapsed());
    }

    /// The built-in file has one game a line, every n from 1 to the end of the table, as the bisection of its lines
    /// needs.  (The lines are read without parsing the games, which the parse of the whole file would do.)
    #[test]
    fn the_text_of_the_table_has_every_n_from_1_to_its_end() {
        let lines: Vec<&str> = BUILT_IN.lines().skip(1).take_while(|&line| line != "]").collect();
        assert!(lines.len() >= 1000, "{} games", lines.len());
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line_n(line), Ok(i as u32 + 1), "line {}", i + 2);
        }
        assert_eq!(Optimal::built_in().max_n() as usize, lines.len());
    }

    /// The table reads the games it is asked for: none for a table nothing was asked of, and one for each n asked of
    /// it however often, whether the text has a game of it or not.
    #[test]
    fn the_table_reads_the_games_it_is_asked_for_and_no_others() {
        let table = Optimal { text: Text::new(BUILT_IN).unwrap(), overlay: BTreeMap::new() };
        let read = || table.text.found.iter().filter(|cell| cell.get().is_some()).count();
        assert_eq!(read(), 0);
        let top = table.max_n();
        assert!(top >= 1000);
        assert_eq!(read(), 0, "the largest n is that of the last line, which is not read as a game");
        assert!(table.score(1000).is_some() && table.moves(1000).is_some());
        assert_eq!(read(), 1);
        assert!(table.score(top).is_some());
        assert_eq!(read(), 2);
        // Nothing is read for an n past the text, and the miss for 0 is remembered, not repeated.
        assert_eq!(table.score(top + 1), None);
        assert_eq!((table.score(0), table.score(0)), (None, None));
        assert_eq!(read(), 3);
    }

    /// The games found in a text by bisection are those of a parse of the file: for the first 400 games of the
    /// built-in file, and for files with gaps, one game and none, for every n from 0 past the end.
    #[test]
    fn the_games_found_in_the_text_are_those_of_a_parse_of_the_file() {
        let sparse = r#"[{"n":3,"score":3,"moves":[3]},{"n":7,"score":17,"moves":[7,4,6]},{"n":12,"score":50,"moves":[11]},
            {"n":13,"score":52,"moves":[]},{"n":90,"score":1,"moves":[2,3]},{"n":91,"score":9,"moves":[5]}]"#;
        let one = r#"[{"n":4,"score":7,"moves":[3,4]}]"#;
        let parsed = |json: &str| Optimal::from_json(&Json::parse(json).unwrap()).unwrap();
        let files =
            [file_of_the_first(400), parsed(sparse).to_file_text(), parsed(one).to_file_text(), "[\n]".to_owned()];
        for file in files {
            let found = table_on(file.clone());
            let eager = parsed(&file);
            assert_eq!(found.max_n(), eager.max_n());
            for n in 0..=eager.max_n() + 2 {
                assert_eq!((found.score(n), found.moves(n)), (eager.score(n), eager.moves(n)), "n={n} in {file}");
            }
            assert!(found.games().eq(eager.games()));
            assert_eq!(found.to_file_text(), file);
        }
    }

    /// A text that is not as `to_file_text` writes it is refused when the table is built on it, rather than read
    /// wrong: the lines of a game are what the bisection splits.
    #[test]
    fn a_text_that_is_not_one_game_a_line_is_refused() {
        let game = "  { \"n\": 4, \"score\": 7, \"moves\": [3, 4] }";
        for text in [
            "[]".to_owned(),
            "[{\"n\":4,\"score\":7,\"moves\":[3,4]}]".to_owned(),
            format!("[\n{game}\n"),
            format!("{game}\n]"),
            "[\n  {\"n\":4,\"score\":7,\"moves\":[3,4]}\n]".to_owned(),
            format!("[\n  {{ \"n\": {}, \"score\": 7, \"moves\": [] }}\n]", MAX_PLAUSIBLE_N + 1),
        ] {
            let text: &'static str = Box::leak(text.into_boxed_str());
            assert!(Text::new(text).is_err(), "{text}");
        }
        assert!(Text::new(Box::leak(format!("[\n{game}\n]").into_boxed_str())).is_ok());
    }

    #[test]
    fn the_first_optimal_games_replay_to_their_recorded_scores() {
        let opt = table_on(file_of_the_first(SMALL_GAMES as usize));
        assert_eq!(opt.verify_all(), Ok(SMALL_GAMES as usize));
        for n in [1, 2, 10, 100, 500, 1000, SMALL_GAMES] {
            assert_eq!(replay(n, opt.moves(n).unwrap()), Ok(opt.score(n).unwrap()), "n={n}");
        }
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is the_first_optimal_games_replay_to_their_recorded_scores"
    )]
    fn every_optimal_game_replays_to_its_recorded_score() {
        let opt = Optimal::built_in();
        assert_eq!(opt.verify_all(), Ok(opt.max_n() as usize));
        // Also spot-check through replay() directly.
        for n in [1, 2, 10, 100, 500, 1000, opt.max_n()] {
            assert_eq!(replay(n, opt.moves(n).unwrap()), Ok(opt.score(n).unwrap()), "n={n}");
        }
    }

    /// The line of a completed game in a results file.
    fn result_line(n: u32, score: u64, moves: &str) -> String {
        format!("{{\"n\":{n},\"score\":{score},\"moves\":{moves},\"completed\":true}}\n")
    }

    #[test]
    fn the_known_games_are_the_table_with_the_results_files_added() {
        let dir = crate::test_support::TempDir::new("known");
        let table = Optimal::from_json(&Json::parse(r#"[{"n":4,"score":7,"moves":[3,4]}]"#).unwrap()).unwrap();
        let results = dir.join("results.jsonl");
        fs::write(&results, result_line(4, 7, "[2,3,4]") + &result_line(5, 12, "[5,4,3]")).unwrap();

        let known = table.with_results_files(std::slice::from_ref(&results)).unwrap();
        // The results file's game replaces the table's, and adds an n the table lacks.
        assert_eq!((known.score(4), known.moves(4)), (Some(7), Some(&[2, 3, 4][..])));
        assert_eq!(known.score(5), Some(12));
        assert_eq!(table.with_results_files(&[]).unwrap().score(5), None);

        // A score that disagrees with the table's is an error, which names the file.
        fs::write(&results, result_line(4, 9, "[2,3,4]")).unwrap();
        let err = table.with_results_files(std::slice::from_ref(&results)).err().expect("a disagreement");
        assert!(err.contains("results.jsonl") && err.contains("n=4 scores 9"), "{err}");
    }

    /// The same for a table on a text: its games, read when asked for, are replaced and added to, and the table
    /// itself stays as it was.
    #[test]
    fn a_table_on_a_text_takes_the_games_of_the_results_files_over_its_own() {
        let dir = crate::test_support::TempDir::new("known-text");
        let table = table_on(file_of_the_first(12));
        let (score10, score12) = (table.score(10).unwrap(), table.score(12).unwrap());
        let results = dir.join("results.jsonl");
        let lines = result_line(10, score10, "[2,3,4]") + &result_line(13, score12 + 2, "[13]");
        fs::write(&results, lines).unwrap();

        let known = table.with_results_files(std::slice::from_ref(&results)).unwrap();
        assert_eq!(
            (known.moves(10), known.score(12), known.score(13)),
            (Some(&[2, 3, 4][..]), Some(score12), Some(score12 + 2))
        );
        assert_eq!((known.max_n(), known.games().count()), (13, 13));
        assert_ne!(table.moves(10), Some(&[2, 3, 4][..]));
        assert_eq!((table.max_n(), table.score(13)), (12, None));

        // The first game, in order of n, that the results score differently from the table is the one named.
        let lines = result_line(11, 1, "[]") + &result_line(10, score10 + 1, "[]");
        fs::write(&results, lines).unwrap();
        let err = table.with_results_files(std::slice::from_ref(&results)).err().expect("a disagreement");
        assert!(err.contains(&format!("n=10 scores {}, but the known games have {score10}", score10 + 1)), "{err}");
    }

    /// The text of a table is its file: `optimal.json` is read and written back byte for byte, and the shapes the
    /// format has to get right (no games, one game, a game with no moves) read back.
    #[test]
    fn a_table_writes_the_file_it_was_read_from() {
        let text = file_of_the_first(SMALL_GAMES as usize);
        assert_eq!(table_on(text.clone()).to_file_text(), text);
        assert!(BUILT_IN.starts_with("[\n  { \"n\": 1, \"score\": 0, \"moves\": [] },\n  { \"n\": 2, \"score\": 2,"));
        assert!(BUILT_IN.ends_with("] }\n]"), "the last game has no comma, and the closing bracket no newline");

        let load = |text: &str| Optimal::from_json(&Json::parse(text).unwrap()).unwrap();
        assert_eq!(load("[]").to_file_text(), "[\n]");
        let one = "[\n  { \"n\": 4, \"score\": 7, \"moves\": [3, 4] }\n]";
        assert_eq!(load(one).to_file_text(), one);
        let sparse =
            "[\n  { \"n\": 3, \"score\": 3, \"moves\": [3] },\n  { \"n\": 7, \"score\": 17, \"moves\": [7, 4, 6] }\n]";
        assert_eq!(load(sparse).to_file_text(), sparse);
        // The order of the text is that of n, whatever the order of the file it was read from.
        let shuffled = r#"[{"n":7,"score":17,"moves":[7,4,6]},{"n":3,"score":3,"moves":[3]}]"#;
        assert_eq!(load(shuffled).to_file_text(), sparse);
    }

    /// [`a_table_writes_the_file_it_was_read_from`] for the whole built-in table.
    #[test]
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is a_table_writes_the_file_it_was_read_from")]
    fn the_built_in_table_writes_the_file_it_was_read_from() {
        assert_eq!(Optimal::built_in().to_file_text(), BUILT_IN);
    }

    #[test]
    fn a_table_is_extended_past_its_end_only() {
        let mut opt = Optimal::from_json(&Json::parse(r#"[{"n":4,"score":7,"moves":[3,4]}]"#).unwrap()).unwrap();
        for n in [0, 3, 4] {
            let err = opt.push(n, 0, vec![]).unwrap_err();
            assert!(err.contains(&format!("n={n} is not past")) && err.contains("4"), "{err}");
        }
        assert!(opt.push(MAX_PLAUSIBLE_N + 1, 0, vec![]).unwrap_err().contains("more than"));
        assert_eq!(opt.sizes().collect::<Vec<_>>(), [4]);
        // A gap is allowed: a table may be sparse.
        opt.push(6, 15, vec![5, 4, 6]).unwrap();
        assert_eq!((opt.sizes().collect::<Vec<_>>(), opt.max_n(), opt.verify_all()), (vec![4, 6], 6, Ok(2)));
    }

    #[test]
    fn a_table_is_saved_whole_and_leaves_no_temporary_file() {
        let dir = crate::test_support::TempDir::new("save-table");
        let path = dir.join("table.json");
        let opt = table_on(file_of_the_first(40));
        fs::write(&path, "old contents").unwrap();
        opt.save(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), opt.to_file_text());
        let names: Vec<_> = fs::read_dir(&*dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, ["table.json"], "the temporary file is renamed away");
        // A directory that is not there is an error naming the file, and nothing is left behind.
        let err = opt.save(&dir.join("missing").join("table.json")).unwrap_err();
        assert!(err.contains("missing") && err.contains("table.json"), "{err}");
    }

    #[test]
    fn verify_all_detects_bad_entries() {
        let load = |text: &str| Optimal::from_json(&Json::parse(text).unwrap()).unwrap();
        assert_eq!(load(r#"[{"n":4,"score":7,"moves":[3,4]}]"#).verify_all(), Ok(1));
        let wrong_score = load(r#"[{"n":4,"score":8,"moves":[3,4]}]"#).verify_all().unwrap_err();
        assert!(wrong_score.contains("n=4") && wrong_score.contains("recorded score 8"), "{wrong_score}");
        let illegal = load(r#"[{"n":4,"score":4,"moves":[1]}]"#).verify_all().unwrap_err();
        assert!(illegal.contains("n=4") && illegal.contains("pays no tax"), "{illegal}");
    }

    #[test]
    fn from_json_rejects_malformed_input() {
        let err = |text: &str| Optimal::from_json(&Json::parse(text).unwrap()).err().unwrap();
        assert!(err("{}").contains("array"));
        assert!(err(r#"[{"score":1,"moves":[]}]"#).contains("missing \"n\""));
        assert!(err(r#"[{"n":-1,"score":1,"moves":[]}]"#).contains("bad \"n\""));
        assert!(err(r#"[{"n":2,"score":"x","moves":[]}]"#).contains("bad \"score\""));
        assert!(err(r#"[{"n":2,"score":2,"moves":[2,"a"]}]"#).contains("bad \"moves\""));
        let dup = r#"[{"n":2,"score":2,"moves":[2]},{"n":2,"score":2,"moves":[2]}]"#;
        assert!(err(dup).contains("duplicate n=2"));
        assert!(Optimal::load("/nonexistent/optimal.json").is_err());
    }

    #[test]
    fn sparse_files_are_fine() {
        let json = Json::parse(r#"[{"n":7,"score":17,"moves":[7,4,6]},{"n":3,"score":3,"moves":[3]}]"#).unwrap();
        let opt = Optimal::from_json(&json).unwrap();
        assert_eq!(opt.sizes().collect::<Vec<_>>(), vec![3, 7]);
        assert_eq!(opt.max_n(), 7);
        assert_eq!(opt.score(5), None);
        assert_eq!(opt.verify_all(), Ok(2));
    }
}
