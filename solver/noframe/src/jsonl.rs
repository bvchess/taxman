//! Resumable result files: one JSON object per line, appended and synced as each item finishes, so that a run
//! stopped at any moment loses at most the items in flight, and a rerun skips what is done.

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// A JSON object written field by field.
pub struct Obj(String);

impl Default for Obj {
    fn default() -> Self {
        Self::new()
    }
}

impl Obj {
    pub fn new() -> Self {
        Obj(String::from("{"))
    }

    fn key(&mut self, k: &str) {
        if self.0.len() > 1 {
            self.0.push(',');
        }
        self.0.push('"');
        self.0.push_str(k);
        self.0.push_str("\":");
    }

    pub fn num(mut self, k: &str, v: impl std::fmt::Display) -> Self {
        self.key(k);
        self.0.push_str(&v.to_string());
        self
    }

    pub fn float(mut self, k: &str, v: f64) -> Self {
        self.key(k);
        self.0.push_str(&format!("{v:.3}"));
        self
    }

    pub fn str(mut self, k: &str, v: &str) -> Self {
        self.key(k);
        self.0.push('"');
        self.0.push_str(&v.replace('\\', "\\\\").replace('"', "\\\""));
        self.0.push('"');
        self
    }

    pub fn nums<T: std::fmt::Display>(mut self, k: &str, vs: &[T]) -> Self {
        self.key(k);
        self.0.push('[');
        self.0.push_str(&vs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));
        self.0.push(']');
        self
    }

    /// A field whose value is already JSON.
    pub fn raw(mut self, k: &str, json: &str) -> Self {
        self.key(k);
        self.0.push_str(json);
        self
    }

    pub fn objs(self, k: &str, objs: Vec<Obj>) -> Self {
        let inner = objs.into_iter().map(Obj::finish).collect::<Vec<_>>().join(",");
        self.raw(k, &format!("[{inner}]"))
    }

    pub fn finish(mut self) -> String {
        self.0.push('}');
        self.0
    }
}

/// The value of an integer field `"key":v` in a line written by [`Obj`], if any.
pub fn field_u64(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let at = line.find(&pat)? + pat.len();
    let digits: String = line[at..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// The value of a string field `"key":"v"` (without escapes) in a line written by [`Obj`], if any.
pub fn field_str<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":\"");
    let at = line.find(&pat)? + pat.len();
    let end = line[at..].find('"')?;
    Some(&line[at..at + end])
}

/// The values of an integer-array field `"key":[a,b,...]` in a line written by [`Obj`], if any.
pub fn field_nums(line: &str, key: &str) -> Option<Vec<u64>> {
    let pat = format!("\"{key}\":[");
    let at = line.find(&pat)? + pat.len();
    let end = line[at..].find(']')?;
    line[at..at + end].split(',').filter(|s| !s.is_empty()).map(|s| s.parse().ok()).collect()
}

/// An append-only results file.  Opening drops a torn last line (one without its newline).
pub struct Appender {
    file: File,
}

impl Appender {
    pub fn open(path: &Path) -> std::io::Result<(Appender, Vec<String>)> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut lines = Vec::new();
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            let keep = text.rfind('\n').map_or(0, |i| i + 1);
            if keep < text.len() {
                OpenOptions::new().write(true).open(path)?.set_len(keep as u64)?;
            }
            lines = BufReader::new(&text.as_bytes()[..keep]).lines().collect::<Result<_, _>>()?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok((Appender { file }, lines))
    }

    pub fn append(&mut self, line: &str) -> std::io::Result<()> {
        self.file.write_all(line.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.sync_data()
    }
}

/// The integer values of `key` over `lines`.
pub fn done(lines: &[String], key: &str) -> HashSet<u64> {
    lines.iter().filter_map(|l| field_u64(l, key)).collect()
}

/// Runs `work` on every item of `items` not yet recorded (by its `"n"` field) in `out`, on `threads` threads,
/// appending each line as it finishes; starts no new item after `seconds`.  `work` returns the line (None for
/// an item it could not finish, which is retried by the next run) and a
/// progress note, printed when it is `Some` or every 20 seconds.
pub fn campaign<F>(name: &str, items: &[u64], threads: usize, seconds: f64, out: &Path, work: F)
where
    F: Fn(u64) -> (Option<String>, Option<String>) + Sync,
{
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    let (mut file, lines) = Appender::open(out).expect("open the output file");
    let finished = done(&lines, "n");
    let todo: Vec<u64> = items.iter().copied().filter(|n| !finished.contains(n)).collect();
    eprintln!("{name}: {} to do ({} already in {})", todo.len(), finished.len(), out.display());
    let next = AtomicUsize::new(0);
    let started = Instant::now();
    let deadline = started + Duration::from_secs_f64(seconds.min(1e9));
    let (tx, rx) = mpsc::channel::<(u64, Option<String>, Option<String>)>();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let (tx, todo, next, work) = (tx.clone(), &todo, &next, &work);
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn_scoped(scope, move || loop {
                    if Instant::now() >= deadline {
                        return;
                    }
                    let Some(&n) = todo.get(next.fetch_add(1, Ordering::Relaxed)) else { return };
                    let (line, note) = work(n);
                    if tx.send((n, line, note)).is_err() {
                        return;
                    }
                })
                .expect("spawn a worker");
        }
        drop(tx);
        let mut count = 0usize;
        let mut last = Instant::now();
        for (n, line, note) in rx {
            let Some(line) = line else {
                eprintln!("{name}: n={n} not finished: {}", note.unwrap_or_default());
                continue;
            };
            file.append(&line).expect("append a result");
            count += 1;
            if note.is_some() || last.elapsed() > Duration::from_secs(20) {
                let secs = started.elapsed().as_secs_f64();
                eprintln!("{name}: n={n} {} ({count} done, {secs:.0}s)", note.unwrap_or_default());
                last = Instant::now();
            }
        }
        eprintln!("{name}: {count} written in {:.0}s", started.elapsed().as_secs_f64());
    });
}

/// The recorded game of each n, (score, moves), from files in optimal.json's or the solver's results format
/// (the first "n", "score" and "moves" of each line holding an "n"); an earlier file wins over a later one.
pub fn recorded_games(paths: &[&Path]) -> std::collections::BTreeMap<u64, (u64, Vec<u32>)> {
    recorded_games_to(paths, u64::MAX)
}

/// [`recorded_games`] for the games of n <= `last`; a line of a larger game is skipped before its score and moves are
/// read, so a check of the small boards does not parse the large games of a table that reaches 5000.
pub fn recorded_games_to(paths: &[&Path], last: u64) -> std::collections::BTreeMap<u64, (u64, Vec<u32>)> {
    fn after<'a>(line: &'a str, key: &str) -> Option<&'a str> {
        let at = line.find(&format!("\"{key}\""))? + key.len() + 2;
        Some(line[at..].trim_start().strip_prefix(':')?.trim_start())
    }
    let number = |s: &str| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>().ok();
    let mut known = std::collections::BTreeMap::new();
    for path in paths {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for line in text.lines() {
            let game = || {
                let n = number(after(line, "n")?).filter(|&n| n <= last)?;
                let score = number(after(line, "score")?)?;
                let moves = after(line, "moves")?.strip_prefix('[')?;
                let moves = &moves[..moves.find(']')?];
                let moves: Vec<u32> =
                    moves.split(',').filter(|s| !s.trim().is_empty()).map(|s| s.trim().parse().unwrap()).collect();
                Some((n, score, moves))
            };
            if let Some((n, score, moves)) = game() {
                known.entry(n).or_insert((score, moves));
            }
        }
    }
    known
}

/// Parses `--from A --to B [--step S] [--list a,b,...]` into the items, and returns the other flags as pairs.
pub fn parse_items(args: impl Iterator<Item = String>) -> (Vec<u64>, Vec<(String, String)>) {
    let (mut from, mut to, mut step) = (2u64, 0u64, 1u64);
    let mut list: Option<Vec<u64>> = None;
    let mut rest = Vec::new();
    let mut it = args;
    while let Some(flag) = it.next() {
        let value = it.next().unwrap_or_else(|| panic!("{flag} needs a value"));
        match flag.as_str() {
            "--from" => from = value.parse().unwrap(),
            "--to" => to = value.parse().unwrap(),
            "--step" => step = value.parse().unwrap(),
            "--list" => list = Some(value.split(',').map(|s| s.trim().parse().unwrap()).collect()),
            _ => rest.push((flag, value)),
        }
    }
    (list.unwrap_or_else(|| (from..=to).step_by(step as usize).collect()), rest)
}
