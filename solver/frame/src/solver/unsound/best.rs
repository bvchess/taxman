//! The best file of the unsound mode, `taxman solve --unsound --best FILE`: the best line any run found of each game
//! past the built-in table, at any level.  The module documentation of [`super`], "The best file", says how a run uses
//! it.
//!
//! The file holds unsound lines, as a run's file does, and each line carries the key [`BEST`], `{"file": ..., "was":
//! ...}`: the name of the file of the run that offered the line, and the score of the game's line in the best file
//! before this one (`null` for the first).  A game's lines score more, line after line, and the last is the game's.
//! The first line of a game may record a repair (`repaired`), since the line it replaced is in the file of the run, not
//! here.  A line of a game that the built-in table has is refused, and so is a score above the line's `bound`.
//!
//! Several runs share one file.  A holder takes an exclusive lock (`flock`) only to read what the others appended and
//! to append, and lets it go at once, so that a run never waits for another's life.  A holder that stopped in the
//! middle of a line leaves a torn last line, which the next holder cuts off.  Nothing here is proven: every reader of
//! proven lines refuses a best file, as it refuses any file of unsound lines.

use super::{read_line, Stored};
use crate::game::Optimal;
use crate::json::Json;
use crate::results::{each_line, mend, Fields};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// The key that a line of the best file has besides a run's: `{"file": the name of the run's file the line came from,
/// "was": the best file's score of the game before this line, or null}`.
pub const BEST: &str = "best";

/// A best file as one run sees it: the last line of each game, as far as the run has read the file.
pub struct BestFile<'a> {
    path: PathBuf,
    /// The built-in table, whose games the file never holds.
    table: &'a Optimal,
    /// The last line of each game.
    games: BTreeMap<u32, Stored>,
    /// The bytes of the file read so far: up to the end of the last line kept.
    read_to: u64,
}

impl<'a> BestFile<'a> {
    /// Opens the file at `path` (creating it if it is missing) and reads it, checking every line as a run's file is
    /// checked ([`read_line`]) and that it is a line of the best file: it carries [`BEST`], is not a game of `table`,
    /// does not score above its `bound`, scores more than the line of its game before it, and says that score in
    /// `best.was`.  Holds the lock only while it reads.
    ///
    /// # Errors
    ///
    /// A file that cannot be read, or holds a line that fails a check; the message names the line.
    pub fn open(path: &Path, table: &'a Optimal) -> Result<BestFile<'a>, String> {
        let mut best = BestFile { path: path.to_owned(), table, games: BTreeMap::new(), read_to: 0 };
        best.locked()?;
        Ok(best)
    }

    /// The moves of the game `n`'s best line and its score, as the run last read them.
    pub fn moves(&self, n: u32) -> Option<(&[u32], u64)> {
        self.games.get(&n).map(|game| (game.moves.as_slice(), game.score))
    }

    /// The score of the game `n`'s best line, as the run last read it.
    pub fn score(&self, n: u32) -> Option<u64> {
        self.games.get(&n).map(|game| game.score)
    }

    /// The game `n`'s best line, as the run last read it.
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

    /// Whether a line of the game `n` that scores `score` would be added: the table lacks the game, and the best file's
    /// line of it, as last read, scores less.
    fn wants(&self, n: u32, score: u64) -> bool {
        self.table.score(n).is_none() && self.games.get(&n).is_none_or(|best| best.score < score)
    }

    /// Offers the lines `lines` (a game and its line each), which a run wrote to its file `file`, and appends those
    /// that score more than the best file's line of their game, in the order given, each with `best.was` the score it
    /// beats.  Reads what the others appended first, under the lock, and syncs the file once, after the last append.
    /// Returns the number of lines appended.
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
            let was = self.games.get(&n).map(|best| best.score);
            let text = with_best(json, file, was).map_err(|e| format!("n={n}: {e}"))?.to_string();
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
        let (table, games, start) = (self.table, &mut self.games, self.read_to);
        let place = |number: usize| match start {
            0 => format!("line {number}"),
            _ => format!("line {number} after byte {start}"),
        };
        let kept = each_line(&text, |number, json| {
            let (n, stored, was) = read_best_line(json, table).map_err(|e| format!("{}: {e}", place(number)))?;
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
                    "{}: (n={n}) {BEST}.was is {was:?}, not the score {before:?} of the line before it",
                    place(number)
                ));
            }
            games.insert(n, stored);
            Ok(())
        })?;
        mend(file, &text, kept, start).map_err(io)?;
        self.read_to = file.metadata().map_err(io)?.len();
        Ok(())
    }
}

/// The game, line and `best.was` of a line of a best file, checked as a run's line is ([`read_line`]) and as a best
/// file's ([`BestFile::open`]), but for the order of the lines of a game.
fn read_best_line(json: &Json, table: &Optimal) -> Result<(u32, Stored, Option<u64>), String> {
    let (n, stored, _) = read_line(json, table)?;
    let fail = |message: String| format!("(n={n}) {message}");
    if table.score(n).is_some() {
        return Err(fail("the built-in table has this game, which a best file never holds".to_owned()));
    }
    if let Some(bound) = json.get("bound").and_then(Json::as_u64).filter(|&bound| stored.score > bound) {
        return Err(fail(format!("it scores {}, above the game's bound {bound}", stored.score)));
    }
    let best = Fields(json).get(BEST).map_err(|e| fail(format!("a best file's line needs a {BEST}: {e}")))?;
    let (was, file) = (Fields(best).opt_u64("was"), Fields(best).get("file"));
    let was = was.map_err(|e| fail(format!("{BEST}: {e}")))?;
    if !matches!(file, Ok(Json::Null | Json::Str(_))) {
        return Err(fail(format!("{BEST}: {}", Fields::bad("file"))));
    }
    Ok((n, stored, was))
}

/// `json`, a line of a run, with the key [`BEST`] set (added at the end, or replaced where the line has it) to the name
/// `file` of the run's file and the score `was` of the game's line in the best file before it.
fn with_best(json: Json, file: Option<&str>, was: Option<u64>) -> Result<Json, String> {
    let Json::Obj(mut pairs) = json else { return Err("the line is not an object".to_owned()) };
    let best =
        Json::object([("file", file.map_or(Json::Null, Json::from)), ("was", was.map_or(Json::Null, Json::from))]);
    match pairs.iter_mut().find(|(key, _)| key == BEST) {
        Some((_, value)) => *value = best,
        None => pairs.push((BEST.to_owned(), best)),
    }
    Ok(Json::Obj(pairs))
}
