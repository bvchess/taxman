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
//!   counts ([`crate::solver`]), and `taxman verify-results` replays whole files.  It
//!   deliberately shares no code with the solver's number-theory tables ([`crate::numtheory`]): it uses only
//!   divisibility and a plain "is it still in the pot" array, so a bug in a table cannot make a wrong game look
//!   legal.
//! * [`Optimal`]: the table of optimal games, an optimal game for every `n` it reaches: the games of the committed
//!   chain, the file `results/optimal.jsonl.gz`, built into the binary ([`Optimal::built_in`]).  The solver and the
//!   verifiers confirm every score they meet against it, and the solver can take the previous game from it when it
//!   solves the board just past a game the table has.  It is a table, not a proof: it holds each line's score and moves
//!   and not its proof, so a game taken from it is assumed optimal ([`crate::results`] records what such a game is
//!   assumed to be), where the file read by path ([`read_results_text`]) brings the proofs.  The file is megabytes
//!   compressed, so the binary decodes a member of it when a game of that member is asked for and no sooner (see
//!   [`Optimal::built_in`]).

use crate::json::Json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

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

/// Trial division.
fn is_prime(p: u32) -> bool {
    p >= 2 && (2..).take_while(|q| q * q <= p).all(|q| !p.is_multiple_of(q))
}

/// Sanity bound on `n` when reading a file of games, so a corrupt file cannot request a huge allocation.
const MAX_PLAUSIBLE_N: u32 = 1 << 20;

/// The bytes of `results/optimal.jsonl.gz` when the crate was compiled: the file [`Optimal::built_in`] reads.
pub const BUILT_IN: &[u8] = include_bytes!("../../../results/optimal.jsonl.gz");

/// The score of a game and its moves.
type Recorded = (u64, Vec<u32>);

/// Whether `path` names a gzip file: the committed results, `results/optimal.jsonl.gz`, are one.
pub fn is_gzip(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "gz")
}

/// The text of the results file at `path`: a file whose name ends in `.gz` decoded by [`taxman_gzip`] (every member, a
/// failed check of one an error), any other file as it is.  Every reader of a results file given by path reads it
/// through here; what a run appends to (`--out`) is plain.
///
/// # Errors
///
/// If the file cannot be read, is not what it should be (gzip, text), or fails a check of the decoder; the message
/// names the file.
pub fn read_results_text(path: &Path) -> Result<String, String> {
    let named = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    if !is_gzip(path) {
        return fs::read_to_string(path).map_err(|e| named(&e));
    }
    let bytes = fs::read(path).map_err(|e| named(&e))?;
    let text = taxman_gzip::decode(&bytes).map_err(|e| named(&e))?;
    String::from_utf8(text).map_err(|e| named(&e))
}

/// The known optimal games, by n: the table built into the program ([`built_in`](Self::built_in): the games of
/// `results/optimal.jsonl.gz`), possibly overlaid with the completed games of results files
/// ([`with_results`](Self::with_results)).  For each n, the best score and a game that achieves it.
#[derive(Clone, Default)]
pub struct Optimal {
    /// The games of the gzip file the table was built on (the built-in one, or a results file given by a name that ends
    /// in `.gz`), each decoded and parsed when it is asked for.
    text: Option<Text<Recorded>>,
    /// The games set over those of the text, by n; all the games of a table made of other parts.
    overlay: BTreeMap<u32, Recorded>,
    /// The table that this one is laid over: its games stand for the n that the text and the overlay have none of.
    under: Option<Arc<Optimal>>,
    /// Whether a game of the text is compared with the game of the same n in `under` when it is looked up, and a
    /// different score stops the lookup ([`with_results_files`](Self::with_results_files)).
    compared: bool,
}

/// How a [`Text`] reads one line of its file, given the line's number (from 1) and its text: the n of the game it
/// records and the game, `None` for a line that records no completed game, or what is wrong with the line, which starts
/// with ": " or " (" so that it follows "the line of game N".
pub(crate) type ParseLine<G> = fn(usize, &str) -> Result<Option<(u32, G)>, String>;

/// The games of a gzip file of result lines, one game a line, the line of the game of n the nth line of the file.
/// The file is decoded member by member, in order, only as far as a game that is asked for, and what is decoded is kept:
/// a process that needs a few games of the file decodes the members that hold them, and one that needs none decodes
/// nothing.  A clone shares what is decoded with the table it was cloned from.
///
/// The decoded members are a chain, each holding the one after it once that is decoded, so that a game is handed out as
/// a reference that stays valid and any number of threads may ask for games at once.  What a game is, and so what its
/// line is parsed into, is the reader's (`G`): the table's games are scores and moves, the committed chain's carry their
/// proofs ([`crate::known`]).
pub(crate) struct Text<G: 'static> {
    file: Arc<GzFile<G>>,
}

impl<G> Clone for Text<G> {
    fn clone(&self) -> Self {
        Text { file: Arc::clone(&self.file) }
    }
}

/// A gzip file, and the chain of its members decoded so far.
struct GzFile<G: 'static> {
    /// What the file is, for the errors: the table built into the program, or the path of a file.
    label: String,
    parse: ParseLine<G>,
    /// The members not decoded yet.  Whoever decodes the next member of the chain has the lock, and the members come in
    /// order, for the next can be decoded only once the one before it is (it needs that one's end).
    undecoded: Mutex<taxman_gzip::Members<'static>>,
    /// The first member, decoded when it is first needed.
    first: OnceLock<Link<G>>,
}

/// What follows a place in the chain: the member decoded there (and the chain that follows it), or the end of the file,
/// or what went wrong in decoding it.
type Link<G> = Result<Option<Box<Member<G>>>, String>;

/// One member of the file, decoded: the lines it holds and, once it is needed, the member after it.
struct Member<G: 'static> {
    /// The decoded text: whole lines, each ending in a newline.
    text: String,
    /// Where each line starts in `text`, and then where the text ends, so that line `i` is `text[starts[i]..starts[i +
    /// 1] - 1]` (without its newline).
    starts: Vec<usize>,
    /// The n of the member's first line: the lines before it, plus one.
    first_n: u32,
    /// The games of the member's lines, parsed when asked for (none for a line that is no completed game).
    games: Vec<OnceLock<Result<Option<G>, String>>>,
    next: OnceLock<Link<G>>,
}

impl<G> GzFile<G> {
    /// Decodes the next member of the file, whose first line is the game of `first_n`.
    fn decode_next(&self, first_n: u32) -> Link<G> {
        let mut undecoded = self.undecoded.lock().unwrap_or_else(PoisonError::into_inner);
        match undecoded.next() {
            None => Ok(None),
            Some(Err(e)) => Err(e.to_string()),
            Some(Ok(bytes)) => Member::new(bytes, first_n).map(|member| Some(Box::new(member))),
        }
    }
}

impl<G> Member<G> {
    /// The member of decoded `bytes`, whose first line is the game of `first_n`.
    ///
    /// # Errors
    ///
    /// If the bytes are not text of whole lines, or a line is the game of an n that a table may not hold.
    fn new(bytes: Vec<u8>, first_n: u32) -> Result<Member<G>, String> {
        let text = String::from_utf8(bytes).map_err(|e| format!("a member is not text: {e}"))?;
        if !text.is_empty() && !text.ends_with('\n') {
            return Err("a member does not end with a newline: its last line is cut off".to_owned());
        }
        let starts: Vec<usize> = std::iter::once(0).chain(text.match_indices('\n').map(|(at, _)| at + 1)).collect();
        let lines = starts.len() - 1;
        if lines > (MAX_PLAUSIBLE_N - first_n.min(MAX_PLAUSIBLE_N)) as usize {
            return Err(format!(
                "{lines} lines from the game of {first_n}, past the {MAX_PLAUSIBLE_N} that a table may hold"
            ));
        }
        let games = (0..lines).map(|_| OnceLock::new()).collect();
        Ok(Member { text, starts, first_n, games, next: OnceLock::new() })
    }

    /// The n of the line after the member's last.
    fn end_n(&self) -> u32 {
        self.first_n + self.games.len() as u32
    }

    /// The game of `n`, parsed from its line (the member's `n - first_n`th) by `parse`: none if the line is no completed
    /// game.
    fn read(&self, n: u32, parse: ParseLine<G>) -> Result<Option<G>, String> {
        let i = (n - self.first_n) as usize;
        let line = &self.text[self.starts[i]..self.starts[i + 1] - 1];
        let Some((found, game)) = parse(n as usize, line).map_err(|e| format!("the line of game {n}{e}"))? else {
            return Ok(None);
        };
        if found != n {
            return Err(format!("the line {n} of the file is the game of {found}"));
        }
        Ok(Some(game))
    }
}

/// The members of a [`Text`] in order, each decoded as the walk reaches it.
enum Walk<'a, G: 'static> {
    Start(&'a GzFile<G>),
    After(&'a GzFile<G>, &'a Member<G>),
    Done,
}

impl<'a, G> Iterator for Walk<'a, G> {
    type Item = Result<&'a Member<G>, String>;

    fn next(&mut self) -> Option<Self::Item> {
        let (file, link) = match *self {
            Walk::Start(file) => (file, file.first.get_or_init(|| file.decode_next(1))),
            Walk::After(file, member) => (file, member.next.get_or_init(|| file.decode_next(member.end_n()))),
            Walk::Done => return None,
        };
        match link {
            Ok(Some(member)) => {
                *self = Walk::After(file, member);
                Some(Ok(member))
            }
            Ok(None) => {
                *self = Walk::Done;
                None
            }
            Err(e) => {
                *self = Walk::Done;
                Some(Err(e.clone()))
            }
        }
    }
}

impl<G> Text<G> {
    /// The games of the members `members` of a gzip file called `label` in the errors, whose lines `parse` reads.
    /// Nothing is decoded until a game is asked for.
    pub(crate) fn new(
        label: impl Into<String>,
        members: taxman_gzip::Members<'static>,
        parse: ParseLine<G>,
    ) -> Text<G> {
        let file = GzFile { label: label.into(), parse, undecoded: Mutex::new(members), first: OnceLock::new() };
        Text { file: Arc::new(file) }
    }

    /// The games of the gzip file at `path`, which is read into memory now and decoded as the games are asked for,
    /// but for its first member, which is decoded now so that a file that is no gzip file is refused at once.
    ///
    /// # Errors
    ///
    /// If the file cannot be read or its first member cannot be decoded; the message names it.
    pub(crate) fn of_file(path: &Path, parse: ParseLine<G>) -> Result<Text<G>, String> {
        let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let text = Text::new(path.display().to_string(), taxman_gzip::Members::owned(bytes), parse);
        text.walk().next().transpose().map_err(|e| text.named(e))?;
        Ok(text)
    }

    fn walk(&self) -> Walk<'_, G> {
        Walk::Start(&self.file)
    }

    /// An error of the file, saying which file it is.
    fn named(&self, e: String) -> String {
        format!("{}: {e}", self.file.label)
    }

    /// The largest n of the text, 0 if it has no game.  It decodes the whole file.
    ///
    /// # Errors
    ///
    /// If a member fails to decode.
    fn max_n(&self) -> Result<u32, String> {
        self.walk().try_fold(0, |_, member| member.map(|member| member.end_n() - 1)).map_err(|e| self.named(e))
    }

    /// The game of `n`, if the file has a completed one there, decoded and parsed the first time it is asked for: only
    /// the members up to the one that holds the line are decoded.
    ///
    /// # Errors
    ///
    /// If a member up to it fails to decode, or the line is not the line of the game of `n`; the message names the file.
    pub(crate) fn get(&self, n: u32) -> Result<Option<&G>, String> {
        if n == 0 {
            return Ok(None);
        }
        for member in self.walk() {
            let member = member.map_err(|e| self.named(e))?;
            if n < member.end_n() {
                let game = member.games[(n - member.first_n) as usize].get_or_init(|| member.read(n, self.file.parse));
                return game.as_ref().map(Option::as_ref).map_err(|e| self.named(e.clone()));
            }
        }
        Ok(None)
    }
}

/// Stops with what is wrong with a file that a table is made on.  Nothing the table is asked can fail unless the file is
/// damaged or is not what the tests of `results/optimal.jsonl.gz` check it is (for the built-in one), so this is loud.
fn broken(e: &str) -> ! {
    panic!("{e}")
}

/// Reads one line of the file of a table: the n, the score and the moves of a completed game, none for any other line.
fn parse_game(_number: usize, line: &str) -> Result<Option<(u32, Recorded)>, String> {
    let json = Json::parse(line).map_err(|e| format!(": {e}"))?;
    if json.get("completed").and_then(Json::as_bool) != Some(true) {
        return Ok(None);
    }
    let (n, score, moves) = Optimal::parse_entry(&json)?;
    Ok(Some((n, (score, moves))))
}

impl Optimal {
    /// The table of optimal games built into this binary: the games of `results/optimal.jsonl.gz` as they were when the
    /// crate was compiled.  The solver and the verifiers confirm every score against it, wherever the binary runs; a
    /// changed file takes a rebuild (`build.rs` marks the build dirty until it is committed).
    ///
    /// Nothing of the file is decoded until a game is asked for, and then the members of the file up to the one that holds
    /// the game (the file is one gzip member for each batch of games, in order), and the game's line is parsed, once.  A
    /// process that never uses the table pays nothing for it, and one that uses a few of the first games pays for the
    /// first members.  What goes over every game ([`games`](Self::games) to its end, [`max_n`](Self::max_n)) decodes
    /// it all.
    pub fn built_in() -> &'static Optimal {
        static TABLE: OnceLock<Optimal> = OnceLock::new();
        TABLE.get_or_init(|| {
            let label = "the table built into this program, results/optimal.jsonl.gz";
            let text = Text::new(label, taxman_gzip::Members::new(BUILT_IN), parse_game);
            Optimal { text: Some(text), overlay: BTreeMap::new(), under: None, compared: false }
        })
    }

    /// The table with the completed games of the results files `results` added (they replace the table's moves for
    /// the same values of n; an unfinished search's line is skipped): how a measurement (the probe, the ledger) gets the
    /// previous game of a board past the table's end, to seed its search and give it a ceiling.
    ///
    /// A file whose name ends in `.gz` is taken as [`with_results`](Self::with_results) takes it, decoded as its games are
    /// asked for, so its games are compared with the known games where they are looked up, not all now (that would
    /// read the whole file): a game that scores differently from the known game of the same n stops the lookup, naming
    /// the file, n and both scores, as a fault in a member decoded later does.
    ///
    /// # Errors
    ///
    /// If a results file cannot be read, or a game of a plain one scores differently from one the games before it have.
    pub fn with_results_files(&self, results: &[PathBuf]) -> Result<Optimal, String> {
        let mut known = self.clone();
        for path in results {
            if is_gzip(path) {
                known = known.laid_over_by_file(path, true)?;
                continue;
            }
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

    /// Reads a parsed JSON array of game objects, `{"n": .., "score": .., "moves": [..]}` each (a table of a few games for
    /// a test); the error names the entry that is malformed, or an n that appears twice.
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
    /// solve --out` writes them, or the committed results, `results/optimal.jsonl.gz`), which replace the games of the
    /// same values of n.  Lines of searches that did not complete are skipped.
    ///
    /// A file whose name ends in `.gz` is read as the built-in table's file is ([`built_in`](Self::built_in)): it must be
    /// one game a line, the line of the game of n the nth, as the committed results are, and it is decoded only as far
    /// as a game that is asked for, parsed when asked.  It is read into memory here, and its first member decoded, so a
    /// file that is not there, or is no gzip file, is refused now; a fault in a later member stops the program (naming
    /// the file) at the lookup that meets it, and a member that no lookup reaches is not read.  Any other file is read
    /// whole, and refused if it has a fault.
    pub fn with_results(mut self, path: &Path) -> Result<Optimal, String> {
        if is_gzip(path) {
            return self.laid_over_by_file(path, false);
        }
        for (n, (score, moves)) in Self::completed_games(path)? {
            self.set(n, score, moves);
        }
        Ok(self)
    }

    /// This table with the gzip file `path` laid over it, `compared` with it or not where a game of the file is looked up.
    fn laid_over_by_file(self, path: &Path, compared: bool) -> Result<Optimal, String> {
        let text = Text::of_file(path, parse_game)?;
        Ok(Optimal { text: Some(text), overlay: BTreeMap::new(), under: Some(Arc::new(self)), compared })
    }

    /// The completed games of the results file `path` by n, the last of a game that the file has twice.
    fn completed_games(path: &Path) -> Result<BTreeMap<u32, Recorded>, String> {
        let text = read_results_text(path)?;
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

    /// The `(score, moves)` recorded for `n`, if any: a game set over the text's, else the text's, else that of the table
    /// this one is laid over.  A game of the text that scores differently from the game of the table it is laid over
    /// stops the program if the text is [`compared`](Self::with_results_files) with it.
    fn entry(&self, n: u32) -> Option<&Recorded> {
        self.overlay.get(&n).or_else(|| self.text_entry(n)).or_else(|| self.under.as_ref()?.entry(n))
    }

    /// The game of `n` in the text, if it has one, compared with the table it is laid over where that is asked for.
    fn text_entry(&self, n: u32) -> Option<&Recorded> {
        let text = self.text.as_ref()?;
        let game = text.get(n).unwrap_or_else(|e| broken(&e))?;
        let below = self.under.as_ref().filter(|_| self.compared).and_then(|under| under.entry(n));
        if let Some(&(before, _)) = below.filter(|known| known.0 != game.0) {
            broken(&text.named(format!("n={n} scores {}, but the known games have {before}", game.0)));
        }
        Some(game)
    }

    /// The largest n of a game set by hand in this table or in one it is laid over (0 if there is none): the games of a
    /// file are one for every n from 1 to its end, and those are not counted.
    fn highest_set(&self) -> u32 {
        let own = self.overlay.keys().next_back().copied().unwrap_or(0);
        own.max(self.under.as_ref().map_or(0, |under| under.highest_set()))
    }

    /// The optimal score for game `n`, if known.
    pub fn score(&self, n: u32) -> Option<u64> {
        self.entry(n).map(|(score, _)| *score)
    }

    /// An optimal game for `n`, if known.
    pub fn moves(&self, n: u32) -> Option<&[u32]> {
        self.entry(n).map(|(_, moves)| moves.as_slice())
    }

    /// Every entry as `(n, score, moves)`, ascending in `n`.  A game of a table built on a file is decoded and parsed
    /// when the iterator reaches it, so taking the first games of the table decodes the first members of the file.
    pub fn games(&self) -> impl Iterator<Item = (u32, u64, &[u32])> + '_ {
        // The file has a game for every n from 1 to its end, and the games set over it may reach past that end.
        let set_up_to = self.highest_set();
        (1..)
            .take_while(move |&n| n <= set_up_to || self.entry(n).is_some())
            .filter_map(|n| self.entry(n).map(|(score, moves)| (n, *score, moves.as_slice())))
    }

    /// The values of n that have an entry, ascending, as lazy as [`games`](Self::games).
    pub fn sizes(&self) -> impl Iterator<Item = u32> + '_ {
        self.games().map(|(n, _, _)| n)
    }

    /// The largest n with an entry (0 if there are none).  It decodes the whole file of a table built on one.
    pub fn max_n(&self) -> u32 {
        let in_text = self.text.as_ref().map_or(0, |text| text.max_n().unwrap_or_else(|e| broken(&e)));
        let below = self.under.as_ref().map_or(0, |under| under.max_n());
        self.overlay.keys().next_back().copied().unwrap_or(0).max(in_text).max(below)
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
    use crate::test_support::gzip_stored;

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

    /// The games of the table that a test goes over every one of: 1 to 1300.  The file's members are decoded in order, so
    /// these cost its first members only; a test of every game of the file is ignored in a debug build.
    const SMALL_GAMES: u32 = 1300;

    /// The first `count` games of the built-in table, by n: `(n, score, moves)`, each asked for by itself so that no
    /// more of the file than their members is decoded.
    fn first_games(count: u32) -> impl Iterator<Item = (u32, u64, &'static [u32])> {
        let opt = Optimal::built_in();
        (1..=count).map(move |n| (n, opt.score(n).expect("a game"), opt.moves(n).expect("a game")))
    }

    /// The line of the game of `n` in a table of a test: a result line with the fields a table reads, scoring ten times
    /// `n` with the moves `n` and `n + 1`.
    fn line(n: u32) -> String {
        format!("{{\"n\":{n},\"score\":{},\"moves\":[{n},{}],\"completed\":true}}\n", u64::from(n) * 10, n + 1)
    }

    /// The lines of the games `range`.
    fn lines(range: std::ops::RangeInclusive<u32>) -> String {
        range.map(line).collect()
    }

    /// The games of a gzip file of the bytes `bytes`, as the table reads them.
    fn text_on(bytes: Vec<u8>) -> Text<Recorded> {
        Text::new("a test file", taxman_gzip::Members::owned(bytes), parse_game)
    }

    /// The table on a gzip file whose members are the texts `members`.
    fn table_on(members: &[&str]) -> Optimal {
        Optimal { text: Some(text_on(gzip_stored(members))), overlay: BTreeMap::new(), under: None, compared: false }
    }

    impl<G> Text<G> {
        /// How many members are decoded so far.
        fn decoded(&self) -> usize {
            let (mut count, mut link) = (0, self.file.first.get());
            while let Some(Ok(Some(member))) = link {
                (count, link) = (count + 1, member.next.get());
            }
            count
        }
    }

    #[test]
    fn the_first_optimal_games_are_finished() {
        for (n, score, moves) in first_games(SMALL_GAMES) {
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
    fn the_built_in_table_answers_for_its_first_games() {
        let opt = Optimal::built_in();
        // n = 0 is no game.
        assert_eq!((opt.score(0), opt.moves(0)), (None, None));
        // n = 1 has score 0 and no moves.
        assert_eq!((opt.score(1), opt.moves(1)), (Some(0), Some(&[][..])));
        assert_eq!((opt.score(2), opt.moves(2)), (Some(2), Some(&[2][..])));
        assert_eq!((opt.score(4), opt.moves(4)), (Some(7), Some(&[3, 4][..])));
        assert_eq!((opt.score(6), opt.moves(6)), (Some(15), Some(&[5, 4, 6][..])));
        // The table is as long as the file.
        let top = opt.max_n();
        assert!(top >= SMALL_GAMES, "{top} games");
        assert_eq!((opt.score(top + 1), opt.moves(top + 1)), (None, None));
        assert!(opt.score(top).is_some() && opt.moves(top).is_some_and(|moves| !moves.is_empty()));
    }

    /// The built-in file has one game a line, every n from 1 to the end of the table, as the lookup of a game by its
    /// line needs: the file has as many lines as the table has games, and each line is the game of its number.
    #[test]
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is the_built_in_table_answers_for_its_first_games")]
    fn the_built_in_file_has_every_n_from_1_to_its_end() {
        let lines = taxman_gzip::decode(BUILT_IN).unwrap().iter().filter(|&&byte| byte == b'\n').count();
        let opt = Optimal::built_in();
        assert_eq!(opt.max_n() as usize, lines);
        assert!(opt.sizes().eq(1..=opt.max_n()));
    }

    /// The table decodes the members of the file that hold the games it is asked for, in order, and no others, and
    /// parses a game once however often it is asked for.
    #[test]
    fn the_table_decodes_the_members_it_is_asked_for_and_no_others() {
        let (first, second, third) = (lines(1..=3), lines(4..=4), lines(5..=9));
        let table = table_on(&[&first, &second, &third]);
        let text = table.text.as_ref().unwrap();
        assert_eq!(text.decoded(), 0, "nothing is decoded before a game is asked for");
        assert_eq!(table.score(2), Some(20));
        assert_eq!(text.decoded(), 1);
        assert_eq!(table.moves(3), Some(&[3, 4][..]));
        assert_eq!(text.decoded(), 1, "a game of a member decoded is a lookup");
        // The member of 5 is the third, which the second is read to reach.
        assert_eq!(table.score(5), Some(50));
        assert_eq!(text.decoded(), 3);
        assert_eq!((table.score(0), table.score(0)), (None, None));
        assert_eq!(text.decoded(), 3);
        // The same game is the same value, parsed once.
        assert!(std::ptr::eq(table.moves(8).unwrap(), table.moves(8).unwrap()));
        // A game past the end is found by decoding the rest.
        assert_eq!(table.score(10), None);
        assert_eq!(table.max_n(), 9);
    }

    /// A table decodes nothing before it is asked, and a clone shares what the table has decoded.
    #[test]
    fn a_clone_of_a_table_shares_what_it_decoded() {
        let table = table_on(&[&lines(1..=2), &lines(3..=4)]);
        let copy = table.clone();
        assert_eq!(copy.score(3), Some(30));
        assert_eq!(table.text.as_ref().unwrap().decoded(), 2);
        assert!(std::ptr::eq(table.moves(4).unwrap(), copy.moves(4).unwrap()));
    }

    /// The games found in a file are those of a parse of the games: for members of different sizes, among them an empty
    /// one, and for every n from 0 past the end.
    #[test]
    fn the_games_found_in_a_file_are_those_of_a_parse_of_its_games() {
        let table = table_on(&[&lines(1..=2), "", &lines(3..=3), &lines(4..=7), ""]);
        let games: Vec<String> =
            (1..=7).map(|n| format!("{{\"n\":{n},\"score\":{},\"moves\":[{n},{}]}}", n * 10, n + 1)).collect();
        let eager = Optimal::from_json(&Json::parse(&format!("[{}]", games.join(","))).unwrap()).unwrap();
        assert_eq!(table.max_n(), eager.max_n());
        for n in 0..=eager.max_n() + 2 {
            assert_eq!((table.score(n), table.moves(n)), (eager.score(n), eager.moves(n)), "n={n}");
        }
        assert!(table.games().eq(eager.games()));
        // A file of one empty member has no game; a file of no member is not a gzip file at all.
        assert_eq!(table_on(&[""]).text.as_ref().unwrap().max_n(), Ok(0));
        assert!(table_on(&[]).text.as_ref().unwrap().max_n().unwrap_err().contains("not a gzip file"));
    }

    /// Many threads asking for games at once all get the games, and each member is decoded once.
    #[test]
    fn threads_asking_for_games_at_once_get_the_same_games() {
        let texts: Vec<String> = (0..5).map(|k| lines(k * 20 + 1..=k * 20 + 20)).collect();
        let members: Vec<&str> = texts.iter().map(String::as_str).collect();
        let table = table_on(&members);
        std::thread::scope(|scope| {
            for thread in 0..6u32 {
                let table = &table;
                scope.spawn(move || {
                    for i in 0..100 {
                        let n = if thread % 2 == 0 { i + 1 } else { 100 - i };
                        assert_eq!((table.score(n), table.moves(n)), (Some(u64::from(n) * 10), Some(&[n, n + 1][..])));
                    }
                });
            }
        });
        assert_eq!(table.text.as_ref().unwrap().decoded(), 5);
    }

    /// A file that is not one game a line, in order, is refused when its lines are asked for, rather than read wrong; and
    /// what is damaged is the part of the file from the damage on, not the games before it.
    #[test]
    fn a_file_that_is_not_one_game_a_line_is_refused() {
        let asked =
            |members: &[&str], n: u32| table_on(members).text.as_ref().unwrap().get(n).map(|game| game.cloned());
        let refused = |result: Result<Option<Recorded>, String>, part: &str| {
            let err = result.unwrap_err();
            assert!(err.contains(part), "{err}");
        };
        let (one, three) = (line(1), line(3));
        // A line of another game than its number, a line cut off, a line that is not JSON, and one that is blank.
        refused(asked(&[&format!("{one}{three}")], 2), "the line 2 of the file is the game of 3");
        refused(asked(&[&lines(1..=2), &line(2)], 3), "the line 3 of the file is the game of 2");
        refused(asked(&[&format!("{one}{{\"n\":2")], 1), "does not end with a newline");
        refused(asked(&[&format!("{one}not json\n")], 2), "the line of game 2");
        refused(asked(&[&format!("{one}\n{three}")], 2), "the line of game 2");
        refused(asked(&[&format!("{one}{{\"n\":2,\"score\":1,\"completed\":true}}\n")], 2), "missing \"moves\"");
        // Not a gzip file at all.
        refused(text_on(b"{\"n\":1}\n".to_vec()).get(1).map(|game| game.cloned()), "bad magic");

        // A byte of the second member's text changed: the first member's games are read, the second's are not.
        let first_member = gzip_stored(&[&lines(1..=2)]).len();
        let mut bytes = gzip_stored(&[&lines(1..=2), &lines(3..=4)]);
        bytes[first_member + 15] ^= 1;
        let text = text_on(bytes);
        assert!(text.get(2).is_ok_and(|game| game.is_some()));
        for _ in 0..2 {
            let err = text.get(3).unwrap_err();
            assert!(err.contains("gzip member 1") && err.contains("CRC-32"), "{err}");
        }
        assert!(text.get(1).is_ok_and(|game| game.is_some()));
    }

    #[test]
    fn the_first_optimal_games_replay_to_their_recorded_scores() {
        for (n, score, moves) in first_games(SMALL_GAMES) {
            assert_eq!(replay(n, moves), Ok(score), "n={n}");
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

    /// The same for a table on a file: its games, read when asked for, are replaced and added to, and the table itself
    /// stays as it was.
    #[test]
    fn a_table_on_a_file_takes_the_games_of_the_results_files_over_its_own() {
        let dir = crate::test_support::TempDir::new("known-text");
        let table = table_on(&[&lines(1..=6), &lines(7..=12)]);
        let (score10, score12) = (table.score(10).unwrap(), table.score(12).unwrap());
        let results = dir.join("results.jsonl");
        let text = result_line(10, score10, "[2,3,4]") + &result_line(13, score12 + 2, "[13]");
        fs::write(&results, text).unwrap();

        let known = table.with_results_files(std::slice::from_ref(&results)).unwrap();
        assert_eq!(
            (known.moves(10), known.score(12), known.score(13)),
            (Some(&[2, 3, 4][..]), Some(score12), Some(score12 + 2))
        );
        assert_eq!((known.max_n(), known.games().count()), (13, 13));
        assert_ne!(table.moves(10), Some(&[2, 3, 4][..]));
        assert_eq!((table.max_n(), table.score(13)), (12, None));

        // The first game, in order of n, that the results score differently from the table is the one named.
        let text = result_line(11, 1, "[]") + &result_line(10, score10 + 1, "[]");
        fs::write(&results, text).unwrap();
        let err = table.with_results_files(std::slice::from_ref(&results)).err().expect("a disagreement");
        assert!(err.contains(&format!("n=10 scores {}, but the known games have {score10}", score10 + 1)), "{err}");
    }

    /// A results file whose name ends in `.gz` is read through the decoder, every member of it, and any other as it is;
    /// the error of a file that fails a check names the file and the check.
    #[test]
    fn a_results_file_is_read_by_its_name() {
        let dir = crate::test_support::TempDir::new("results-text");
        let text = lines(1..=5);
        let (plain, zipped) = (dir.join("games.jsonl"), dir.join("games.jsonl.gz"));
        fs::write(&plain, &text).unwrap();
        fs::write(&zipped, gzip_stored(&[&lines(1..=2), &lines(3..=5)])).unwrap();
        assert_eq!(read_results_text(&plain), Ok(text.clone()));
        assert_eq!(read_results_text(&zipped), Ok(text));
        // The games of a gzip file are the table's completed games, as those of a plain one are.
        let known = Optimal::default().with_results(&zipped).unwrap();
        assert_eq!((known.max_n(), known.score(3)), (5, Some(30)));

        // The CRC-32 at the end of the member is wrong.
        let mut damaged = gzip_stored(&[&lines(1..=5)]);
        let at = damaged.len() - 8;
        damaged[at] ^= 1;
        let path = dir.join("damaged.jsonl.gz");
        fs::write(&path, damaged).unwrap();
        let err = read_results_text(&path).unwrap_err();
        assert!(err.contains("damaged.jsonl.gz") && err.contains("CRC-32"), "{err}");
        let err = read_results_text(&dir.join("missing.jsonl.gz")).unwrap_err();
        assert!(err.contains("missing.jsonl.gz"), "{err}");
        // A table on the file is made without decoding it past its first member, but not for a file that is not there
        // or is no gzip file.
        let err = Optimal::default().with_results(&dir.join("missing.jsonl.gz")).err().expect("a refusal");
        assert!(err.contains("missing.jsonl.gz"), "{err}");
        let not_gzip = dir.join("plain.jsonl.gz");
        fs::write(&not_gzip, lines(1..=5)).unwrap();
        let err = Optimal::default().with_results(&not_gzip).err().expect("a refusal");
        assert!(err.contains("plain.jsonl.gz") && err.contains("not a gzip file"), "{err}");
        let mut first_member = gzip_stored(&[&lines(1..=5)]);
        first_member[20] ^= 1;
        fs::write(&path, first_member).unwrap();
        let err = Optimal::default().with_results(&path).err().expect("a refusal");
        assert!(err.contains("damaged.jsonl.gz") && err.contains("CRC-32"), "{err}");
    }

    /// The panic message of a lookup that stops.
    fn stopped_by<T>(lookup: impl FnOnce() -> T) -> String {
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(lookup)).err().expect("the lookup stops");
        panic.downcast_ref::<String>().cloned().unwrap_or_default()
    }

    /// A gzip results file is decoded as its games are asked for, so a fault in it stops the lookup that meets it,
    /// naming the file and the fault, as a fault in the built-in table's does; the games before the fault are read, and
    /// so is a file whose member after the fault nothing asks for.
    #[test]
    fn a_fault_in_a_gzip_results_file_stops_the_lookup_that_meets_it() {
        let dir = crate::test_support::TempDir::new("results-gz-fault");
        let path = dir.join("damaged.jsonl.gz");
        let first_member = gzip_stored(&[&lines(1..=2)]).len();
        let mut bytes = gzip_stored(&[&lines(1..=2), &lines(3..=4)]);
        bytes[first_member + 15] ^= 1;
        fs::write(&path, bytes).unwrap();
        let known = Optimal::default().with_results(&path).unwrap();
        assert_eq!(known.score(2), Some(20));
        let message = stopped_by(|| known.score(3));
        assert!(message.contains("damaged.jsonl.gz") && message.contains("CRC-32"), "{message}");
        // A line that is not the game of its number stops the lookup too.
        let crossed = dir.join("crossed.jsonl.gz");
        fs::write(&crossed, gzip_stored(&[&(line(1) + &line(3))])).unwrap();
        let known = Optimal::default().with_results(&crossed).unwrap();
        let message = stopped_by(|| known.score(2));
        assert!(message.contains("crossed.jsonl.gz") && message.contains("the line 2 of the file is the game of 3"));
    }

    /// A gzip file taken by `with_results_files` is compared with the known games where its games are looked up: a game
    /// that scores differently from the known game of its n stops the lookup of that n, naming the file, n and both
    /// scores, and the games that agree, and those past the known games, are read.  `with_results` makes no comparison.
    #[test]
    fn a_gzip_results_file_is_compared_with_the_known_games_where_its_games_are_looked_up() {
        let dir = crate::test_support::TempDir::new("results-gz-compared");
        let table = table_on(&[&lines(1..=4)]);
        // The games 1 to 6, but the game of 3 scores 31 where the table's scores 30; 5 and 6 are past the table's end.
        let text = line(1) + &line(2) + &result_line(3, 31, "[3,4]") + &line(4) + &line(5) + &line(6);
        let zipped = dir.join("games.jsonl.gz");
        fs::write(&zipped, gzip_stored(&[&text])).unwrap();

        let known = table.with_results_files(std::slice::from_ref(&zipped)).unwrap();
        for n in [1, 2, 4, 5, 6] {
            assert_eq!(known.score(n), Some(u64::from(n) * 10), "n={n}");
        }
        for message in [stopped_by(|| known.score(3)), stopped_by(|| known.moves(3))] {
            assert!(message.contains("games.jsonl.gz"), "{message}");
            assert!(message.contains("n=3 scores 31, but the known games have 30"), "{message}");
        }
        // The file's game stands where nothing compares it.
        assert_eq!(table.clone().with_results(&zipped).unwrap().score(3), Some(31));
    }

    /// A results file whose name ends in `.gz` lies over the games before it as a plain file does, and is decoded only as
    /// far as its games are asked for: its games replace the games before it, a line that is no completed game leaves the
    /// game before it (the file's line of that game is the unfinished search's), and a file read after it lies over it.
    #[test]
    fn a_gzip_results_file_lies_over_the_games_before_it_as_a_plain_file_does() {
        let dir = crate::test_support::TempDir::new("results-gz-over");
        let table = table_on(&[&lines(1..=4)]);
        // The games 1 to 6, but for 3, a stopped search's line, and with another score and moves for 5.
        let unfinished = "{\"n\":3,\"completed\":false}\n";
        let first = line(1) + &line(2);
        let rest = unfinished.to_owned() + &line(4) + &result_line(5, 7, "[5]") + &line(6);
        let (zipped, plain) = (dir.join("games.jsonl.gz"), dir.join("games.jsonl"));
        fs::write(&zipped, gzip_stored(&[&first, &rest])).unwrap();
        fs::write(&plain, first + &rest).unwrap();

        let from_gz = table.clone().with_results(&zipped).unwrap();
        let from_plain = table.clone().with_results(&plain).unwrap();
        assert_eq!(from_gz.text.as_ref().unwrap().decoded(), 1, "the first member is decoded when the file is opened");
        assert_eq!(from_gz.score(2), Some(20));
        assert_eq!(from_gz.text.as_ref().unwrap().decoded(), 1, "and no more for a game of it");
        // The game of 3 is the table's, as it is with the plain file; the file's line of it was read to know that.
        assert_eq!(from_gz.score(3), Some(30));
        assert_eq!(from_gz.text.as_ref().unwrap().decoded(), 2);
        for n in 0..=8 {
            assert_eq!((from_gz.score(n), from_gz.moves(n)), (from_plain.score(n), from_plain.moves(n)), "n={n}");
        }
        assert_eq!((from_gz.score(5), from_gz.moves(5)), (Some(7), Some(&[5][..])));
        assert_eq!((from_gz.max_n(), from_gz.games().count()), (6, 6));
        assert!(from_gz.games().eq(from_plain.games()));
        // The table the file lies over is as it was.
        assert_eq!((table.score(5), table.max_n()), (None, 4));

        // Later files lie over earlier ones, of either kind.
        let other = dir.join("other.jsonl");
        fs::write(&other, result_line(2, 99, "[2]")).unwrap();
        let gz_then_plain = from_gz.clone().with_results(&other).unwrap();
        let plain_then_gz = table.clone().with_results(&other).unwrap().with_results(&zipped).unwrap();
        assert_eq!((gz_then_plain.score(2), plain_then_gz.score(2)), (Some(99), Some(20)));
        assert_eq!((gz_then_plain.score(6), plain_then_gz.score(6)), (Some(60), Some(60)));
        // `with_results_files` takes it the same way.
        let known = table.with_results_files(&[plain.clone(), zipped.clone()]).unwrap();
        assert!(known.games().eq(from_gz.games()));
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
