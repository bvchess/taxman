//! `ledger export`: the bounds of a run's ledger that improve on the store's best, as new lines of the store.

use crate::book::{carries, carry_game, upper_by, Entry, Rule, Side, Upper};
use crate::schedule::read_ledger;
use crate::store::{self, Best, Chain, Game, Line, Store};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;
use taxman::game::Optimal;

/// What an export is told: the name of the run, the day, and where the run's checkpoints are kept (a prefix, with
/// `nN.ckpt` under it) if they are kept anywhere.
pub struct Options<'a> {
    pub source: &'a str,
    pub date: &'a str,
    pub checkpoints: Option<&'a str>,
}

/// What an export appended.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Lines appended, boards that got some, and for each side the lines that are the first of their board and the
    /// lines that improved on a bound the store had.
    pub lines: usize,
    pub boards: usize,
    pub lower_first: usize,
    pub lower_better: usize,
    pub upper_first: usize,
    pub upper_better: usize,
    /// Boards whose best bounds meet now and did not before, and all that do.
    pub newly_proven: Vec<u32>,
    pub proven: usize,
    /// Boards of the ledger left out because the committed chain has proven them.
    pub chain: usize,
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} lines for {} boards: lower bounds {} ({} first for their board, {} improved), upper bounds {} ({} first, \
             {} improved); {} boards newly proven{}, {} proven in the file; {} boards left out (the chain has proven them)",
            self.lines,
            self.boards,
            self.lower_first + self.lower_better,
            self.lower_first,
            self.lower_better,
            self.upper_first + self.upper_better,
            self.upper_first,
            self.upper_better,
            self.newly_proven.len(),
            if self.newly_proven.is_empty() { String::new() } else { format!(" ({})", ranges(&self.newly_proven)) },
            self.proven,
            self.chain,
        )
    }
}

/// Boards written as ranges: `4400-4410, 4412`.
pub fn ranges(boards: &[u32]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < boards.len() {
        let mut j = i;
        while j + 1 < boards.len() && boards[j + 1] == boards[j] + 1 {
            j += 1;
        }
        out.push(if j > i + 1 {
            format!("{}-{}", boards[i], boards[j])
        } else {
            boards[i..=j].iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
        });
        i = j + 1;
    }
    out.join(", ")
}

/// An upper bound of a board once the export is done: its value, and what it rests on.
#[derive(Clone)]
struct Bound {
    value: u64,
    rests_on: Vec<u32>,
}

/// The lower bounds to append: each after the line it cites, if it cites one.
struct Lowers<'a> {
    ledger: &'a HashMap<u32, &'a Entry>,
    best: &'a Best<'a>,
    /// The lower lines there are, by board and value: the store's, then the export's.
    known: HashSet<(u32, u64)>,
    chain: &'a Optimal,
    options: &'a Options<'a>,
    /// The lines checked so far: the store's, then the export's.
    checks: Chain<'a>,
    /// The boards whose line is settled (a board whose line is being made counts: a circle of citations is not made).
    settled: HashSet<u32>,
    lines: Vec<Line>,
}

impl Lowers<'_> {
    /// Appends the line of the ledger's lower bound of board `n` if it improves on the store's and the chain has not
    /// proven the board.  A game that a relation made of the game of another board (monotone, ceiling or prime) is not
    /// written but cited, if the ledger's lower bound of that board is that game and the file has it by then, after
    /// the line of that board if the export has to append it (a game of the chain is never cited, as a run need not
    /// have the chain).
    fn settle(&mut self, n: u32) -> Result<(), String> {
        if !self.settled.insert(n) || self.chain.score(n).is_some() {
            return Ok(());
        }
        let Some(lower) = self.ledger.get(&n).and_then(|entry| entry.lower.as_ref()) else { return Ok(()) };
        if lower.rule == Rule::Trivial || self.best.lower.get(&n).is_some_and(|have| have.value >= lower.value) {
            return Ok(());
        }
        let game = lower.promotions.clone().map_or_else(|| Game::Moves(lower.moves.clone()), Game::Promotions);
        let mut line = Line {
            n,
            side: Side::Lower,
            value: lower.value,
            rule: lower.rule,
            from: lower.from,
            game: Some(game),
            from_value: None,
            bags: None,
            rests_on: Vec::new(),
            checkpoint: None,
            source: self.options.source.to_owned(),
            date: self.options.date.to_owned(),
        };
        let source = lower.from.and_then(|from| Some((from, self.ledger.get(&from)?.lower.as_ref()?)));
        if let Some((from, source)) = source.filter(|&(from, _)| carries(lower.rule, from, n)) {
            if carry_game(lower.rule, from, n, &source.moves).is_ok_and(|moves| moves == lower.moves) {
                self.settle(from)?;
                if self.known.contains(&(from, source.value)) {
                    (line.game, line.from_value) = (None, Some(source.value));
                }
            }
        }
        self.checks.add(&line).map_err(|e| format!("n={n}: its lower line: {e}"))?;
        self.known.insert((n, line.value));
        self.lines.push(line);
        Ok(())
    }
}

/// The upper bounds to append, one board after another, each after the bounds it cites.
struct Uppers<'a> {
    ledger: &'a HashMap<u32, &'a Entry>,
    best: &'a Best<'a>,
    chain: &'a Optimal,
    options: &'a Options<'a>,
    /// The lines checked so far: the store's, then the export's.
    checks: Chain<'a>,
    /// The boards whose bound is settled, and the boards being settled (a circle is an error).
    settled: HashMap<u32, Option<Bound>>,
    working: HashSet<u32>,
    lines: Vec<Line>,
}

impl Uppers<'_> {
    /// The best upper bound of board `n` after the export, if it has one: a game of the chain, which is exact; else the
    /// ledger's if that improves on the store's (appended, after the bounds it cites); else the store's.
    fn settle(&mut self, n: u32) -> Result<Option<Bound>, String> {
        if let Some(done) = self.settled.get(&n) {
            return Ok(done.clone());
        }
        if !self.working.insert(n) {
            return Err(format!("the upper bounds of board {n} come from each other in a circle"));
        }
        let stored = self.best.upper.get(&n).map(|line| Bound { value: line.value, rests_on: line.rests_on.clone() });
        let bound = if let Some(score) = self.chain.score(n) {
            Some(Bound { value: score, rests_on: vec![n] })
        } else {
            match self.ledger.get(&n).and_then(|entry| entry.upper) {
                Some(upper) if stored.as_ref().is_none_or(|have| upper.value < have.value) => {
                    self.append(n, upper, stored)?
                }
                _ => stored,
            }
        };
        self.working.remove(&n);
        self.settled.insert(n, bound.clone());
        Ok(bound)
    }

    /// The line for the ledger's upper bound `upper` of board `n`, appended if it improves on `stored`.
    fn append(&mut self, n: u32, upper: Upper, stored: Option<Bound>) -> Result<Option<Bound>, String> {
        let at = |why: String| format!("n={n}: the {} upper bound: {why}", upper.rule.name());
        let entry = self.ledger[&n];
        let mut line = Line {
            n,
            side: Side::Upper,
            value: upper.value,
            rule: upper.rule,
            from: None,
            game: None,
            from_value: None,
            bags: None,
            rests_on: vec![n],
            checkpoint: None,
            source: self.options.source.to_owned(),
            date: self.options.date.to_owned(),
        };
        match upper.rule {
            // A board with nothing to promote: its bounds follow from its setup, and are not kept.
            Rule::Trivial => return Ok(stored),
            Rule::Dual if !entry.proven() => {
                line.checkpoint =
                    self.options.checkpoints.map(|prefix| format!("{}/n{n}.ckpt", prefix.trim_end_matches('/')));
            }
            rule if rule.is_relation() => {
                let from = upper.from.ok_or_else(|| at("no board it came from".into()))?;
                let source = self.settle(from)?.ok_or_else(|| at(format!("board {from} has no bound to come from")))?;
                // The bags of a repeat (the source's, the target's): the board it repeats is of its class, so the ledger
                // has it.
                let bags = if rule == Rule::Repeat {
                    let from_bag = self.ledger.get(&from).map(|e| e.bag);
                    Some((
                        from_bag.ok_or_else(|| at(format!("it repeats board {from}, not in the ledger")))?,
                        entry.bag,
                    ))
                } else {
                    None
                };
                // The bound of `from` may have fallen since this one was derived from it (or the store has a better):
                // the line cites what there is.
                line.value = upper_by(rule, from, n, source.value, bags).map_err(at)?;
                if line.value > upper.value {
                    return Err(at(format!(
                        "it is {} but board {from}, {}, gives {}",
                        upper.value, source.value, line.value
                    )));
                }
                line.from = Some(from);
                line.from_value = Some(source.value);
                line.bags = bags.map(|(from_bag, bag)| (bag, from_bag));
                line.rests_on = source.rests_on;
                if rule == Rule::Repeat && !line.rests_on.contains(&n) {
                    line.rests_on.push(n);
                    line.rests_on.sort_unstable();
                }
            }
            _ => {}
        }
        if stored.as_ref().is_some_and(|have| line.value >= have.value) {
            return Ok(stored);
        }
        self.checks.add(&line).map_err(|e| format!("n={n}: its upper line: {e}"))?;
        let bound = Bound { value: line.value, rests_on: line.rests_on.clone() };
        self.lines.push(line);
        Ok(Some(bound))
    }
}

/// The lines that an export of `entries` (a ledger's boards) to `store` appends, checked as the checker will check
/// them, and what they are.  The lower bounds come first, in order of n, then the upper bounds, each after the ones it
/// cites.  A board of the committed chain (`chain`) gets none.
pub fn plan(
    entries: &[Entry],
    store: &Store,
    chain: &Optimal,
    options: &Options,
) -> Result<(Vec<Line>, Summary), String> {
    let best = store.best();
    let mut checks = Chain::new(chain);
    for (i, line) in store.lines.iter().enumerate() {
        checks
            .add(line)
            .map_err(|e| format!("the store's line {}, board {}: {e}; run verify-bounds", i + 1, line.n))?;
    }
    let mut summary =
        Summary { chain: entries.iter().filter(|e| chain.score(e.n).is_some()).count(), ..Summary::default() };
    let ledger: HashMap<u32, &Entry> = entries.iter().map(|e| (e.n, e)).collect();
    let mut lowers = Lowers {
        ledger: &ledger,
        best: &best,
        known: store.lines.iter().filter(|l| l.side == Side::Lower).map(|l| (l.n, l.value)).collect(),
        chain,
        options,
        checks,
        settled: HashSet::new(),
        lines: Vec::new(),
    };
    for entry in entries {
        lowers.settle(entry.n)?;
    }
    let (mut lines, checks) = (lowers.lines, lowers.checks);
    let mut uppers = Uppers {
        ledger: &ledger,
        best: &best,
        chain,
        options,
        checks,
        settled: HashMap::new(),
        working: HashSet::new(),
        lines: Vec::new(),
    };
    for entry in entries {
        uppers.settle(entry.n)?;
    }
    lines.append(&mut uppers.lines);

    summary.lines = lines.len();
    summary.boards = lines.iter().map(|l| l.n).collect::<HashSet<_>>().len();
    for line in &lines {
        let counter = match line.side {
            Side::Lower if best.lower.contains_key(&line.n) => &mut summary.lower_better,
            Side::Lower => &mut summary.lower_first,
            Side::Upper if best.upper.contains_key(&line.n) => &mut summary.upper_better,
            Side::Upper => &mut summary.upper_first,
        };
        *counter += 1;
    }
    let before: HashSet<u32> = best.proven().into_iter().collect();
    let after = Best::of(store.lines.iter().chain(&lines)).proven();
    summary.newly_proven = after.iter().copied().filter(|n| !before.contains(n)).collect();
    summary.proven = after.len();
    Ok((lines, summary))
}

/// Appends to the store `bounds` the bounds of the ledger in `dir` that improve on its best, for the boards the
/// committed chain `strict` has not proven.
pub fn export(dir: &Path, bounds: &Path, strict: &Path, options: &Options) -> Result<Summary, String> {
    let entries = read_ledger(dir)?;
    let store = Store::read_or_new(bounds)?;
    let (lines, summary) = plan(&entries, &store, &store::chain(strict)?, options)?;
    Store::append(bounds, &lines)?;
    Ok(summary)
}

/// Today's date in UTC, YYYY-MM-DD.
pub fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    civil_date((secs / 86_400) as i64)
}

/// The date `days` days after 1970-01-01, YYYY-MM-DD (Howard Hinnant's algorithm).
pub fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let (era, day_of_era) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
