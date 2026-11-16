//! The best-known file of the unsound mode, `taxman solve --unsound --best-known FILE`: the best-known line of each
//! game past the built-in table, the best line any run has found, at any level.  The module documentation of
//! [`super`], "The best-known file", says how a run uses it.
//!
//! The file holds unsound lines, as a run's file does, and each line carries the key [`BEST_KNOWN`], `{"file": ...,
//! "was": ...}`: the name of the file of the run that offered the line, and the score of the game's line in the
//! best-known file before this one (`null` for the first).  A game's lines score more, line after line, and the last is
//! the game's.  The first line of a game may record a repair (`repaired`), since the line it replaced is in the file of
//! the run, not here.  A line of a game that the built-in table has is refused (the table a run with `--known-to N`
//! reads the file with has the games up to N only), and so is a score above the line's `bound`.
//!
//! Several runs share one file.  A holder takes an exclusive lock (`flock`) only to read what the others appended and
//! to append, and lets it go at once, so that a run never waits for another's life.  A holder that stopped in the
//! middle of a line leaves a torn last line, which the next holder cuts off.  The holders append whole lines, and that
//! cut changes nothing before the last newline, so a run that opens the file reads its lines up to the last newline
//! without the lock and takes the lock only to read the rest: runs that open a large file at once do not wait in turn
//! for each other's read of it.  Nothing here is proven: every reader of proven lines refuses a best-known file, as it
//! refuses any file of unsound lines.

use super::{read_line, Stored};
use crate::game::Optimal;
use crate::json::Json;
use crate::results::{each_line, mend, Fields, Kept};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// The key that a line of the best-known file has besides a run's: `{"file": the name of the run's file the line came
/// from, "was": the best-known file's score of the game before this line, or null}`.
pub const BEST_KNOWN: &str = "best_known";

/// A best-known file as one run sees it: the last line of each game, as far as the run has read the file.
pub struct BestKnownFile<'a> {
    path: PathBuf,
    /// The built-in table, cut at the run's `--known-to` if it has one, whose games the file never holds.
    table: &'a Optimal,
    /// The last line of each game.
    games: BTreeMap<u32, Stored>,
    /// The bytes of the file read so far: up to the end of the last line kept.
    read_to: u64,
}

impl<'a> BestKnownFile<'a> {
    /// Opens the file at `path` (creating it if it is missing) and reads it, checking every line as a run's file is
    /// checked ([`read_line`]) and that it is a line of the best-known file: it carries [`BEST_KNOWN`], is not a game
    /// of `table`, does not score above its `bound`, scores more than the line of its game before it, and says that
    /// score in `best_known.was`.  Reads the lines up to the last newline without the lock (the module documentation
    /// says why that is safe), and holds the lock only while it reads the rest.
    ///
    /// A read without the lock that passes a torn last line as another holder cuts it may go on into the lines
    /// appended after the cut, and so see a line that the file never held.  So a line that this read refuses is not
    /// taken as the file's: the run forgets what it read and reads the whole file again under the lock, which refuses
    /// the line if the file holds it.
    ///
    /// # Errors
    ///
    /// A file that cannot be read, or holds a line that fails a check; the message names the line.
    pub fn open(path: &Path, table: &'a Optimal) -> Result<BestKnownFile<'a>, String> {
        let mut best_known = Self::unread(path, table);
        if best_known.read_unlocked().is_err() {
            best_known = Self::unread(path, table);
        }
        best_known.locked()?;
        Ok(best_known)
    }

    /// The file at `path` before the run has read any of it.
    fn unread(path: &Path, table: &'a Optimal) -> BestKnownFile<'a> {
        BestKnownFile { path: path.to_owned(), table, games: BTreeMap::new(), read_to: 0 }
    }

    /// The moves of the game `n`'s best-known line and its score, as the run last read them.
    pub fn moves(&self, n: u32) -> Option<(&[u32], u64)> {
        self.games.get(&n).map(|game| (game.moves.as_slice(), game.score))
    }

    /// The score of the game `n`'s best-known line, as the run last read it.
    pub fn score(&self, n: u32) -> Option<u64> {
        self.games.get(&n).map(|game| game.score)
    }

    /// The game `n`'s best-known line, as the run last read it.
    pub(super) fn line(&self, n: u32) -> Option<&Stored> {
        self.games.get(&n)
    }

    /// Reads what other holders have appended since the last read, if anything: the lock is taken only if the file has
    /// changed in size.
    ///
    /// # Errors
    ///
    /// As [`open`](Self::open), and a file that has become shorter, since another process replaced it.
    pub fn refresh(&mut self) -> Result<(), String> {
        let size = fs::metadata(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?.len();
        if size != self.read_to {
            self.locked()?;
        }
        Ok(())
    }

    /// Whether a line of the game `n` that scores `score` would be added: the table lacks the game, and the
    /// best-known file's line of it, as last read, scores less.
    fn wants(&self, n: u32, score: u64) -> bool {
        self.table.score(n).is_none() && self.games.get(&n).is_none_or(|known| known.score < score)
    }

    /// Offers the lines `lines` (a game and its line each), which a run wrote to its file `file`, and appends those
    /// that score more than the best-known file's line of their game, in the order given, each with `best_known.was`
    /// the score it beats.  Reads what the others appended first, under the lock, and syncs the file once, after the
    /// last append.  Returns the number of lines appended.
    ///
    /// # Errors
    ///
    /// A file that cannot be read or written, or a line that scores above its `bound`.
    pub(super) fn offer<'b>(
        &mut self,
        file: Option<&str>,
        lines: impl IntoIterator<Item = (u32, &'b Stored)>,
    ) -> Result<usize, String> {
        let wanted: Vec<(u32, &Stored)> = lines.into_iter().filter(|&(n, line)| self.wants(n, line.score)).collect();
        if wanted.is_empty() {
            return Ok(0);
        }
        let mut handle = self.locked()?;
        let at = |e: &dyn Display| format!("{}: {e}", self.path.display());
        let mut added = 0;
        for (n, line) in wanted {
            // Another holder may have added a line as good since the run last read the file.
            if !self.wants(n, line.score) {
                continue;
            }
            let json = Json::parse(&line.text).map_err(|e| format!("n={n}: the line to offer: {e}"))?;
            if let Some(bound) = json.get("bound").and_then(Json::as_u64).filter(|&bound| line.score > bound) {
                return Err(format!("n={n}: the line scores {}, above the game's bound {bound}", line.score));
            }
            let was = self.games.get(&n).map(|known| known.score);
            let text = with_best_known(json, file, was).map_err(|e| format!("n={n}: {e}"))?.to_string();
            handle.write_all(format!("{text}\n").as_bytes()).map_err(|e| at(&e))?;
            self.games.insert(n, Stored { text, ..line.clone() });
            added += 1;
        }
        handle.sync_data().map_err(|e| at(&e))?;
        self.read_to = handle.metadata().map_err(|e| at(&e))?.len();
        Ok(added)
    }

    /// Opens the file for reading and appending (creating it if need be), takes the exclusive lock on it, and reads
    /// what was appended since the last read.  The lock goes when the file returned is dropped.
    fn locked(&mut self) -> Result<File, String> {
        let path = self.path.clone();
        let at = |e: &dyn Display| format!("{}: {e}", path.display());
        let mut file = OpenOptions::new().create(true).read(true).append(true).open(&path).map_err(|e| at(&e))?;
        file.lock().map_err(|e| at(&e))?;
        self.catch_up(&mut file).map_err(|e| at(&e))?;
        Ok(file)
    }

    /// Reads, with no lock, the lines of the unread file up to its last newline, and checks them as
    /// [`catch_up`](Self::catch_up) does; a missing file has none.  Writes nothing: what lies past the last newline, a
    /// torn last line or a last line with no newline, is left to `catch_up`, under the lock.  An error goes no further
    /// than [`open`](Self::open), which then reads the whole file under the lock.
    fn read_unlocked(&mut self) -> Result<(), String> {
        let mut bytes = match fs::read(&self.path) {
            Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
            read => read.map_err(|e| e.to_string())?,
        };
        bytes.truncate(bytes.iter().rposition(|&byte| byte == b'\n').map_or(0, |last| last + 1));
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let kept = self.check(&text)?;
        self.read_to = kept.valid_len as u64;
        Ok(())
    }

    /// Reads the lines `file` gained after the byte `read_to`, checks them (the documentation of [`open`](Self::open))
    /// and cuts a torn last line off.  The caller holds the lock, so no holder is in the middle of a line.
    fn catch_up(&mut self, file: &mut File) -> Result<(), String> {
        let io = |e: std::io::Error| e.to_string();
        let size = file.metadata().map_err(io)?.len();
        if size < self.read_to {
            return Err(format!(
                "the file is shorter than the {} bytes already read: another process replaced or cut it",
                self.read_to
            ));
        }
        if size == self.read_to {
            return Ok(());
        }
        file.seek(SeekFrom::Start(self.read_to)).map_err(io)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(io)?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let kept = self.check(&text)?;
        mend(file, &text, kept, self.read_to).map_err(io)?;
        self.read_to = file.metadata().map_err(io)?.len();
        Ok(())
    }

    /// Checks the lines of `text`, the file's bytes from the byte `read_to` on (the documentation of
    /// [`open`](Self::open)), and keeps the last line of each game.  Returns how far the lines are kept.
    fn check(&mut self, text: &str) -> Result<Kept, String> {
        let (table, games, start) = (self.table, &mut self.games, self.read_to);
        let place = |number: usize| match start {
            0 => format!("line {number}"),
            _ => format!("line {number} after byte {start}"),
        };
        each_line(text, |number, json| {
            let (n, stored, was) = read_best_known_line(json, table).map_err(|e| format!("{}: {e}", place(number)))?;
            let before = games.get(&n).map(|game| game.score);
            if let Some(before) = before.filter(|&before| stored.score <= before) {
                return Err(format!(
                    "{}: (n={n}) it scores {}, no more than the {before} of the line before it",
                    place(number),
                    stored.score
                ));
            }
            if was != before {
                return Err(format!(
                    "{}: (n={n}) {BEST_KNOWN}.was is {was:?}, not the score {before:?} of the line before it",
                    place(number)
                ));
            }
            games.insert(n, stored);
            Ok(())
        })
    }
}

/// The game, line and `best_known.was` of a line of a best-known file, checked as a run's line is ([`read_line`]) and
/// as a best-known file's ([`BestKnownFile::open`]), but for the order of the lines of a game.
fn read_best_known_line(json: &Json, table: &Optimal) -> Result<(u32, Stored, Option<u64>), String> {
    let (n, stored, _) = read_line(json, table)?;
    let fail = |message: String| format!("(n={n}) {message}");
    if table.score(n).is_some() {
        return Err(fail("the built-in table has this game, which a best-known file never holds".to_owned()));
    }
    if let Some(bound) = json.get("bound").and_then(Json::as_u64).filter(|&bound| stored.score > bound) {
        return Err(fail(format!("it scores {}, above the game's bound {bound}", stored.score)));
    }
    let key = Fields(json)
        .get(BEST_KNOWN)
        .map_err(|e| fail(format!("a best-known file's line needs a {BEST_KNOWN}: {e}")))?;
    let (was, file) = (Fields(key).opt_u64("was"), Fields(key).get("file"));
    let was = was.map_err(|e| fail(format!("{BEST_KNOWN}: {e}")))?;
    if !matches!(file, Ok(Json::Null | Json::Str(_))) {
        return Err(fail(format!("{BEST_KNOWN}: {}", Fields::bad("file"))));
    }
    Ok((n, stored, was))
}

/// `json`, a line of a run, with the key [`BEST_KNOWN`] set (added at the end, or replaced where the line has it) to
/// the name `file` of the run's file and the score `was` of the game's line in the best-known file before it.
fn with_best_known(json: Json, file: Option<&str>, was: Option<u64>) -> Result<Json, String> {
    let Json::Obj(mut pairs) = json else { return Err("the line is not an object".to_owned()) };
    let value =
        Json::object([("file", file.map_or(Json::Null, Json::from)), ("was", was.map_or(Json::Null, Json::from))]);
    match pairs.iter_mut().find(|(key, _)| key == BEST_KNOWN) {
        Some((_, existing)) => *existing = value,
        None => pairs.push((BEST_KNOWN.to_owned(), value)),
    }
    Ok(Json::Obj(pairs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::UNSOUND;
    use crate::test_support::TempDir;

    /// A best-known file's first line of the game `n`, with the built-in table's moves: a line for a table cut below n.
    fn line(n: u32) -> String {
        let table = Optimal::built_in();
        let run = Json::object([
            (UNSOUND, Json::from(true)),
            ("n", Json::from(n)),
            ("score", Json::from(table.score(n).unwrap())),
            ("moves", Json::array(table.moves(n).unwrap().iter().copied())),
            ("promotions", Json::array::<u32>([])),
            ("in_the_bag", Json::Null),
            ("method", Json::from("trivial")),
            ("setup", Json::Null),
            ("repaired", Json::Null),
        ]);
        with_best_known(run, None, None).unwrap().to_string()
    }

    /// The last line of each game, as `best_known` has read them.
    fn games(best_known: &BestKnownFile) -> Vec<(u32, String)> {
        best_known.games.iter().map(|(&n, game)| (n, game.text.clone())).collect()
    }

    #[test]
    fn a_run_opens_the_file_by_reading_its_complete_lines_without_the_lock_and_the_rest_under_it() {
        let (dir, table) = (TempDir::new("best-known-unlocked"), Optimal::built_in().up_to(9));
        let complete: String = (10..=12).map(|n| line(n) + "\n").collect();
        let last = line(13);
        // The tail past the last newline is a line with no newline, which the read under the lock takes and ends with
        // one, or a torn line, which it cuts off.
        let tails = [(last.as_str(), format!("{complete}{last}\n"), 13), (&last[..20], complete.clone(), 12)];
        for (tail, mended, top) in tails {
            let (path, once) = (dir.join("open.jsonl"), dir.join("once.jsonl"));
            let text = format!("{complete}{tail}");
            fs::write(&path, &text).unwrap();
            fs::write(&once, &text).unwrap();
            // The read without the lock stops at the last newline, and writes nothing.
            let mut unlocked = BestKnownFile::unread(&path, &table);
            unlocked.read_unlocked().unwrap();
            assert_eq!(unlocked.read_to, complete.len() as u64);
            assert_eq!(unlocked.games.keys().copied().collect::<Vec<_>>(), [10, 11, 12]);
            assert_eq!(fs::read_to_string(&path).unwrap(), text);
            // Opened, the file is read as one read under the lock reads it, and mended as that read mends it.
            let opened = BestKnownFile::open(&path, &table).unwrap();
            let mut locked = BestKnownFile::unread(&once, &table);
            drop(locked.locked().unwrap());
            assert_eq!(games(&opened), games(&locked));
            assert_eq!(opened.games.keys().copied().collect::<Vec<_>>(), (10..=top).collect::<Vec<_>>());
            assert_eq!((opened.read_to, locked.read_to), (mended.len() as u64, mended.len() as u64));
            assert_eq!(
                (fs::read_to_string(&path).unwrap(), fs::read_to_string(&once).unwrap()),
                (mended.clone(), mended)
            );
        }
    }
}
