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
//!   records what a game taken from it is assumed to be).

use crate::json::Json;
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
    let (_, in_pot) = play(n, moves)?;
    let mut legal = vec![false; in_pot.len()];
    for d in (1..=n).filter(|&d| in_pot[d as usize]) {
        for m in (2 * d..=n).step_by(d as usize) {
            legal[m as usize] |= in_pot[m as usize];
        }
    }
    Ok((1..=n).filter(|&m| legal[m as usize]).collect())
}

/// Referees `moves` as a finished game of `n` and returns its score.  Besides being legal, the game must have
/// the two properties every optimal game has, which it can show by itself:
///
/// * it is over: no legal move is left (one more move would only add to the score);
/// * for `n ≥ 2`, it opens with the largest prime `≤ n`.  A prime can be played only while 1 is in the pot, so
///   only as the first move, and an optimal game plays the largest one (a short exchange argument, proved with
///   the prime shortcut it supports in [`crate::accelerate::prime_shortcut`]).
pub fn replay_finished(n: u32, moves: &[u32]) -> Result<u64, String> {
    let score = replay(n, moves)?;
    let left = legal_moves_left(n, moves)?;
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
        for d in (1..m).filter(|d| m % d == 0) {
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

/// The text of `src/main/resources/optimal.json` when the crate was compiled: the table
/// [`Optimal::built_in`] reads.
pub const BUILT_IN: &str = include_str!("../../src/main/resources/optimal.json");

/// The known optimal games, by board size: the table `src/main/resources/optimal.json` ([`built_in`](Self::built_in)),
/// possibly overlaid with the completed games of results files ([`with_results`](Self::with_results)).  For each
/// board size, the best score and a game that achieves it.
#[derive(Clone)]
pub struct Optimal {
    /// Indexed by `n`; `None` where the file has no entry.
    by_n: Vec<Option<(u64, Vec<u32>)>>,
}

impl Optimal {
    /// The table of optimal games built into this binary: `src/main/resources/optimal.json` as it was when the crate
    /// was compiled, read once.  The solver and the verifiers confirm every score against it, wherever the binary
    /// runs; a changed file takes a rebuild (`build.rs` marks the build dirty until it is committed).
    pub fn built_in() -> &'static Optimal {
        static TABLE: OnceLock<Optimal> = OnceLock::new();
        TABLE.get_or_init(|| {
            let json = Json::parse(BUILT_IN).expect("the built-in optimal.json is JSON");
            Optimal::from_json(&json).expect("the built-in optimal.json is a table of games")
        })
    }

    /// The table with the completed games of the results files `results` added (they replace the table's moves for
    /// the same board sizes; an unfinished search's line is skipped): how a measurement (the ladder, `taxman
    /// bench`) gets the previous game of a board past the table's end, to seed its search and give it a ceiling.
    ///
    /// # Errors
    ///
    /// If a results file cannot be read, or a game of one scores differently from one the games before it have.
    pub fn with_results_files(&self, results: &[PathBuf]) -> Result<Optimal, String> {
        let mut known = self.clone();
        for path in results {
            let before: Vec<(u32, u64)> = known.games().map(|(n, score, _)| (n, score)).collect();
            known = known.with_results(path)?;
            if let Some(&(n, score)) = before.iter().find(|&&(n, score)| known.score(n) != Some(score)) {
                let now = known.score(n).map_or_else(|| "none".to_owned(), |s| s.to_string());
                return Err(format!("{}: n={n} scores {now}, but the known games have {score}", path.display()));
            }
        }
        Ok(known)
    }

    /// Loads a file holding an array of `{"n": .., "score": .., "moves": [..]}` objects.
    pub fn load(path: impl AsRef<Path>) -> Result<Optimal, String> {
        Optimal::from_json(&Json::from_file(path)?)
    }

    /// Reads a parsed JSON array of game objects (see [`load`](Self::load)); the error names the entry that is
    /// malformed, or a board size that appears twice.
    pub fn from_json(json: &Json) -> Result<Optimal, String> {
        let entries = json.as_arr().ok_or("expected an array of games")?;
        let mut out = Optimal { by_n: Vec::new() };
        for (i, entry) in entries.iter().enumerate() {
            let (n, score, moves) = Self::parse_entry(entry).map_err(|e| format!("entry {i}{e}"))?;
            if out.set(n, score, moves) {
                return Err(format!("entry {i}: duplicate n={n}"));
            }
        }
        Ok(out)
    }

    /// These games with the completed games of the results file `path` (one JSON object per line, as `taxman
    /// solve --out` writes them), which replace the games of the same board sizes.  Lines of searches that did
    /// not complete are skipped.
    pub fn with_results(mut self, path: &Path) -> Result<Optimal, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let at = |e: String| format!("{}: line {}{e}", path.display(), i + 1);
            let game = Json::parse(line).map_err(|e| at(format!(": {e}")))?;
            if game.get("completed").and_then(Json::as_bool) == Some(true) {
                let (n, score, moves) = Self::parse_entry(&game).map_err(at)?;
                self.set(n, score, moves);
            }
        }
        Ok(self)
    }

    /// The board size, score and moves of one game object; the error says what is wrong after a ": " (or after
    /// " (n=…): " once the board size is known).
    fn parse_entry(entry: &Json) -> Result<(u32, u64, Vec<u32>), String> {
        let field = |name: &str| entry.get(name).ok_or_else(|| format!(": missing \"{name}\""));
        let n = field("n")?.as_u32().filter(|&n| n <= MAX_PLAUSIBLE_N).ok_or(": bad \"n\"")?;
        let score = field("score")?.as_u64().ok_or_else(|| format!(" (n={n}): bad \"score\""))?;
        let moves = field("moves")?.as_u32_vec().ok_or_else(|| format!(" (n={n}): bad \"moves\""))?;
        Ok((n, score, moves))
    }

    /// Records the game of board size `n`; returns whether it replaced one.
    fn set(&mut self, n: u32, score: u64, moves: Vec<u32>) -> bool {
        if self.by_n.len() <= n as usize {
            self.by_n.resize(n as usize + 1, None);
        }
        self.by_n[n as usize].replace((score, moves)).is_some()
    }

    /// Appends the game of board size `n`, which must be past every board size the table has: a table is extended
    /// past its end, and its games are never replaced.
    pub fn push(&mut self, n: u32, score: u64, moves: Vec<u32>) -> Result<(), String> {
        if n <= self.max_n() {
            return Err(format!("n={n} is not past the largest board size of the table, {}", self.max_n()));
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
                format!("  {{ \"n\": {n}, \"score\": {score}, \"moves\": [{}] }}", moves.join(", "))
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

    /// The `(score, moves)` recorded for `n`, if any.
    fn entry(&self, n: u32) -> Option<&(u64, Vec<u32>)> {
        self.by_n.get(n as usize)?.as_ref()
    }

    /// The optimal score for board size `n`, if known.
    pub fn score(&self, n: u32) -> Option<u64> {
        self.entry(n).map(|(score, _)| *score)
    }

    /// An optimal game for board size `n`, if known.
    pub fn moves(&self, n: u32) -> Option<&[u32]> {
        self.entry(n).map(|(_, moves)| moves.as_slice())
    }

    /// Every entry as `(n, score, moves)`, ascending in `n`.
    pub fn games(&self) -> impl Iterator<Item = (u32, u64, &[u32])> + '_ {
        let present = self.by_n.iter().enumerate();
        present.filter_map(|(n, e)| e.as_ref().map(|(score, moves)| (n as u32, *score, moves.as_slice())))
    }

    /// The board sizes that have an entry, ascending.
    pub fn sizes(&self) -> impl Iterator<Item = u32> + '_ {
        self.games().map(|(n, _, _)| n)
    }

    /// The largest board size with an entry (0 if there are none).
    pub fn max_n(&self) -> u32 {
        self.sizes().last().unwrap_or(0)
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

    #[test]
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
        // The parse is ~10 ms when optimized; the generous bound only guards against something quadratic.
        assert!(start.elapsed().as_secs_f64() < 1.0, "loading took {:?}", start.elapsed());
        // Every board size from 1 to the end of the table (the Java's 1000, and what has been appended since).
        let top = opt.max_n();
        assert!(top >= 1000);
        assert_eq!(opt.sizes().count(), top as usize);
        assert!(opt.sizes().eq(1..=top));
        // n = 0 and anything past the file are unknown.
        assert_eq!((opt.score(0), opt.moves(0), opt.score(top + 1), opt.moves(top + 1)), (None, None, None, None));
        // n = 1 has score 0 and no moves.
        assert_eq!((opt.score(1), opt.moves(1)), (Some(0), Some(&[][..])));
        assert_eq!((opt.score(2), opt.moves(2)), (Some(2), Some(&[2][..])));
        assert_eq!((opt.score(4), opt.moves(4)), (Some(7), Some(&[3, 4][..])));
        assert_eq!((opt.score(6), opt.moves(6)), (Some(15), Some(&[5, 4, 6][..])));
    }

    #[test]
    fn every_optimal_game_replays_to_its_recorded_score() {
        let opt = Optimal::built_in();
        assert_eq!(opt.verify_all(), Ok(opt.max_n() as usize));
        // Also spot-check through replay() directly.
        for n in [1, 2, 10, 100, 500, 1000, opt.max_n()] {
            assert_eq!(replay(n, opt.moves(n).unwrap()), Ok(opt.score(n).unwrap()), "n={n}");
        }
    }

    #[test]
    fn the_known_games_are_the_table_with_the_results_files_added() {
        let dir = crate::test_support::TempDir::new("known");
        let table = Optimal::from_json(&Json::parse(r#"[{"n":4,"score":7,"moves":[3,4]}]"#).unwrap()).unwrap();
        let line = |n: u32, score: u64, moves: &str| {
            format!("{{\"n\":{n},\"score\":{score},\"moves\":{moves},\"completed\":true}}\n")
        };
        let results = dir.join("results.jsonl");
        fs::write(&results, line(4, 7, "[2,3,4]") + &line(5, 12, "[5,4,3]")).unwrap();

        let known = table.with_results_files(std::slice::from_ref(&results)).unwrap();
        // The results file's game replaces the table's, and adds a board size the table lacks.
        assert_eq!((known.score(4), known.moves(4)), (Some(7), Some(&[2, 3, 4][..])));
        assert_eq!(known.score(5), Some(12));
        assert_eq!(table.with_results_files(&[]).unwrap().score(5), None);

        // A score that disagrees with the table's is an error, which names the file.
        fs::write(&results, line(4, 9, "[2,3,4]")).unwrap();
        let err = table.with_results_files(std::slice::from_ref(&results)).err().expect("a disagreement");
        assert!(err.contains("results.jsonl") && err.contains("n=4 scores 9"), "{err}");
    }

    /// The text of a table is its file: `optimal.json` is read and written back byte for byte, and the shapes the
    /// format has to get right (no games, one game, a game with no moves) read back.
    #[test]
    fn a_table_writes_the_file_it_was_read_from() {
        let text = BUILT_IN;
        assert_eq!(Optimal::built_in().to_file_text(), text);
        assert!(text.starts_with("[\n  { \"n\": 1, \"score\": 0, \"moves\": [] },\n  { \"n\": 2, \"score\": 2,"));
        assert!(text.ends_with("] }\n]"), "the last game has no comma, and the closing bracket no newline");

        let load = |text: &str| Optimal::from_json(&Json::parse(text).unwrap()).unwrap();
        assert_eq!(load("[]").to_file_text(), "[\n]");
        let one = "[\n  { \"n\": 4, \"score\": 7, \"moves\": [3, 4] }\n]";
        assert_eq!(load(one).to_file_text(), one);
        let sparse =
            "[\n  { \"n\": 3, \"score\": 3, \"moves\": [3] },\n  { \"n\": 7, \"score\": 17, \"moves\": [7, 4, 6] }\n]";
        assert_eq!(load(sparse).to_file_text(), sparse);
        // The order of the text is that of the board sizes, whatever the order of the file it was read from.
        let shuffled = r#"[{"n":7,"score":17,"moves":[7,4,6]},{"n":3,"score":3,"moves":[3]}]"#;
        assert_eq!(load(shuffled).to_file_text(), sparse);
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
        let opt = Optimal::built_in();
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
