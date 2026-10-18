//! The bounds store: `results/bounds.jsonl`, the bounds on opt(n) that the ledger finds, kept for good.  One JSON line
//! a bound, appended and never changed; a line is added only when it improves the file's best bound of its board and
//! side.  A board is proven when its best lower bound is its best upper bound.  The format is described in
//! results/README.md, "bounds.jsonl"; this module is its one copy in code: the line (`Line`), the file (`Store`), and
//! the checks of a line against the lines before it (`Chain`), which `ledger export` makes before it appends,
//! `ledger verify-bounds` makes of every line, and `tools/check_bounds.py` makes again independently.

use crate::book::{carries, upper_by, Rule, Side};
use std::collections::{BTreeMap, HashMap};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::solver::MAX_N;

/// The schema number of a line.
pub const SCHEMA: u64 = 1;

/// The rules a lower bound can have, and an upper bound.
const LOWER_RULES: [Rule; 8] =
    [Rule::Start, Rule::Search, Rule::Lns, Rule::Given, Rule::Monotone, Rule::Ceiling, Rule::Prime, Rule::Repeat];
const UPPER_RULES: [Rule; 10] = [
    Rule::Root,
    Rule::Dual,
    Rule::Exhausted,
    Rule::Floor,
    Rule::Nogoods,
    Rule::Given,
    Rule::Monotone,
    Rule::Ceiling,
    Rule::Prime,
    Rule::Repeat,
];

/// The names of a line's fields, in the order it writes them.
const KEYS: [&str; 15] = [
    "schema",
    "n",
    "side",
    "value",
    "rule",
    "from",
    "from_value",
    "bag",
    "from_bag",
    "rests_on",
    "checkpoint",
    "source",
    "date",
    "promotions",
    "moves",
];

/// A lower bound's game: the promotions it was made from, which `solution_for_promotions` turns into moves (the
/// compact form, for every game that was made from promotions), or its moves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Game {
    Promotions(Vec<u32>),
    Moves(Vec<u32>),
}

/// A line of the store: a bound on opt(`n`), `side` of it, worth `value`, by `rule`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub n: u32,
    pub side: Side,
    pub value: u64,
    pub rule: Rule,
    /// The board a relation's bound came from.
    pub from: Option<u32>,
    /// A lower bound's game, which the referee replays at `value`.
    pub game: Option<Game>,
    /// An upper bound by a relation: the value of the upper bound of `from` that it was derived from.
    pub from_value: Option<u64>,
    /// A repeat's upper bound: `in_the_bag` of `n` and of `from`.
    pub bags: Option<(u64, u64)>,
    /// An upper bound: the boards at which statement U is assumed, ascending (THEORY.md, "In the ledger").
    pub rests_on: Vec<u32>,
    /// A dual bound: where the checkpoint of the search that gave it is, if the export was given the place.
    pub checkpoint: Option<String>,
    /// The run that found the bound, and the day it was exported.
    pub source: String,
    pub date: String,
}

impl Line {
    /// The line as it is written.
    pub fn to_json(&self) -> Json {
        let mut pairs: Vec<(&str, Json)> = vec![
            ("schema", Json::from(SCHEMA)),
            ("n", Json::from(self.n)),
            ("side", Json::from(self.side.name())),
            ("value", Json::from(self.value)),
            ("rule", Json::from(self.rule.name())),
        ];
        let mut put = |key, value: Option<Json>| pairs.extend(value.map(|value| (key, value)));
        put("from", self.from.map(Json::from));
        put("from_value", self.from_value.map(Json::from));
        put("bag", self.bags.map(|(bag, _)| Json::from(bag)));
        put("from_bag", self.bags.map(|(_, from_bag)| Json::from(from_bag)));
        put("rests_on", (!self.rests_on.is_empty()).then(|| Json::array(self.rests_on.iter().copied())));
        put("checkpoint", self.checkpoint.as_deref().map(Json::from));
        put("source", Some(Json::from(self.source.as_str())));
        put("date", Some(Json::from(self.date.as_str())));
        match &self.game {
            Some(Game::Promotions(set)) => put("promotions", Some(Json::array(set.iter().copied()))),
            Some(Game::Moves(moves)) => put("moves", Some(Json::array(moves.iter().copied()))),
            None => {}
        }
        Json::object(pairs)
    }

    /// The line of a JSON object; the error says what is wrong with it.
    pub fn from_json(json: &Json) -> Result<Line, String> {
        let fields = json.as_obj().ok_or("not an object")?;
        if let Some((key, _)) = fields.iter().find(|(key, _)| !KEYS.contains(&key.as_str())) {
            return Err(format!("the field \"{key}\" is not in the format"));
        }
        if json.get("schema").and_then(Json::as_u64) != Some(SCHEMA) {
            return Err(format!("the schema is not {SCHEMA}"));
        }
        let get = |key: &str| json.get(key).ok_or_else(|| format!("no \"{key}\""));
        let number = |key: &str| get(key)?.as_u64().ok_or_else(|| format!("bad \"{key}\""));
        let text = |key: &str| get(key)?.as_str().map(str::to_owned).ok_or_else(|| format!("bad \"{key}\""));
        let optional = |key: &str| json.get(key).map(|value| value.as_u64().ok_or_else(|| format!("bad \"{key}\"")));
        let list = |key: &str| json.get(key).map(|v| v.as_u32_vec().ok_or_else(|| format!("bad \"{key}\"")));
        let side = Side::of(&text("side")?).ok_or("bad \"side\"")?;
        let rule = Rule::of(&text("rule")?).ok_or("bad \"rule\"")?;
        let bags = match (optional("bag").transpose()?, optional("from_bag").transpose()?) {
            (Some(bag), Some(from_bag)) => Some((bag, from_bag)),
            (None, None) => None,
            _ => return Err("\"bag\" and \"from_bag\" go together".into()),
        };
        let game = match (list("promotions").transpose()?, list("moves").transpose()?) {
            (Some(set), None) => Some(Game::Promotions(set)),
            (None, Some(moves)) => Some(Game::Moves(moves)),
            (None, None) => None,
            _ => return Err("both \"promotions\" and \"moves\"".into()),
        };
        let line = Line {
            n: u32::try_from(number("n")?).map_err(|_| "bad \"n\"")?,
            side,
            value: number("value")?,
            rule,
            from: optional("from")
                .transpose()?
                .map(|from| u32::try_from(from).map_err(|_| "bad \"from\""))
                .transpose()?,
            game,
            from_value: optional("from_value").transpose()?,
            bags,
            rests_on: list("rests_on").transpose()?.unwrap_or_default(),
            checkpoint: json.get("checkpoint").map(|_| text("checkpoint")).transpose()?,
            source: text("source")?,
            date: text("date")?,
        };
        line.check_shape()?;
        Ok(line)
    }

    /// What a line of this side and rule has and has not.
    fn check_shape(&self) -> Result<(), String> {
        let relation = self.rule.is_relation();
        let allowed: &[Rule] = match self.side {
            Side::Lower => &LOWER_RULES,
            Side::Upper => &UPPER_RULES,
        };
        let fail = |why: &str| Err(format!("a {} bound by {}: {why}", self.side.name(), self.rule.name()));
        if self.n < 2 || self.n > MAX_N || self.from.is_some_and(|from| from > MAX_N) {
            return Err(format!("a board is a number from 2 to {MAX_N}, the widest board of the solver"));
        }
        if !allowed.contains(&self.rule) {
            return fail("there is no such rule");
        }
        if self.from.is_some() != relation {
            return fail(if relation { "it needs the board it came from" } else { "it has a board it came from" });
        }
        if self.from == Some(self.n) {
            return fail("it came from its own board");
        }
        if self.source.is_empty() || !is_date(&self.date) {
            return Err("the source is empty or the date is not YYYY-MM-DD".into());
        }
        match self.side {
            Side::Lower => {
                // A game, or the game it is carried from: the lower bound `from_value` of the board `from`.
                match (&self.game, self.from_value, self.from) {
                    (None, None, _) => return fail("it needs its game, or the game it is carried from"),
                    (Some(_), Some(_), _) => return fail("it has a game and cites one"),
                    (None, Some(_), from) if !from.is_some_and(|from| carries(self.rule, from, self.n)) => {
                        return fail("that relation does not carry a game from its board to this one");
                    }
                    (Some(Game::Promotions(set)), ..) if !set.windows(2).all(|pair| pair[0] < pair[1]) => {
                        return fail("the promotions are not ascending numbers");
                    }
                    _ => {}
                }
                if self.bags.is_some() || !self.rests_on.is_empty() || self.checkpoint.is_some() {
                    return fail("it has fields of an upper bound");
                }
            }
            Side::Upper => {
                if self.game.is_some() {
                    return fail("it has a game");
                }
                if self.from_value.is_some() != relation || self.bags.is_some() != (self.rule == Rule::Repeat) {
                    return fail(
                        "it needs the value it came from (and a repeat the two bags), and only a relation has them",
                    );
                }
                if self.checkpoint.is_some() && self.rule != Rule::Dual {
                    return fail("only a dual bound has a checkpoint");
                }
                if self.rests_on.is_empty() || !self.rests_on.windows(2).all(|pair| pair[0] < pair[1]) {
                    return fail("rests_on is not a list of boards, ascending, with one at least");
                }
                if !relation && self.rests_on != [self.n] {
                    return fail("it is a bound of the frame model at its own board, so it rests on that board alone");
                }
                if let Some(from) = self.from {
                    let bags = self.bags.map(|(bag, from_bag)| (from_bag, bag));
                    upper_by(self.rule, from, self.n, 0, bags)?;
                }
            }
        }
        Ok(())
    }
}

/// Whether `text` is a date, YYYY-MM-DD.
pub fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else { return false };
    let digits = |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
    digits(year, 4)
        && digits(month, 2)
        && digits(day, 2)
        && matches!(month.parse::<u32>(), Ok(1..=12))
        && matches!(day.parse::<u32>(), Ok(1..=31))
}

/// Lines with their numbers in the file, counting from 1.
pub type Numbered = Vec<(usize, Line)>;

/// The lines of a store file, in order.
#[derive(Clone, Debug, Default)]
pub struct Store {
    pub lines: Vec<Line>,
}

/// The best bounds of a store: for each board the line of its highest lower bound and the line of its lowest upper.
#[derive(Default)]
pub struct Best<'a> {
    pub lower: BTreeMap<u32, &'a Line>,
    pub upper: BTreeMap<u32, &'a Line>,
}

impl Store {
    /// The store in the file `path`.  The error names the first line that is not a line of the format.
    pub fn read(path: &Path) -> Result<Store, String> {
        let (lines, mut bad) = Store::parse(path)?;
        match bad.is_empty() {
            true => Ok(Store { lines: lines.into_iter().map(|(_, line)| line).collect() }),
            false => Err(bad.swap_remove(0)),
        }
    }

    /// The store in the file `path`, empty if there is no such file (the file an export creates).
    pub fn read_or_new(path: &Path) -> Result<Store, String> {
        if path.exists() {
            Store::read(path)
        } else {
            Ok(Store::default())
        }
    }

    /// The lines of the file `path` with their numbers (counting from 1), and what is wrong with each that is not a
    /// line of the format.
    pub fn parse(path: &Path) -> Result<(Numbered, Vec<String>), String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (mut lines, mut bad) = (Vec::new(), Vec::new());
        for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let parsed = Json::parse(line).map_err(|e| e.to_string()).and_then(|json| Line::from_json(&json));
            match parsed {
                Ok(line) => lines.push((i + 1, line)),
                Err(e) => bad.push(format!("{}: line {}: {e}", path.display(), i + 1)),
            }
        }
        Ok((lines, bad))
    }

    /// Appends `lines` to the file `path`, creating it if need be.  A file whose last line is torn (no newline at its
    /// end) is refused: the new lines would join it.
    pub fn append(path: &Path, lines: &[Line]) -> Result<(), String> {
        let at = |e: std::io::Error| format!("{}: {e}", path.display());
        if let Ok(text) = std::fs::read(path) {
            if text.last().is_some_and(|&last| last != b'\n') {
                return Err(format!("{}: the last line is torn (no newline at its end)", path.display()));
            }
        }
        let text: String = lines.iter().map(|line| format!("{}\n", line.to_json())).collect();
        let mut file = OpenOptions::new().create(true).append(true).open(path).map_err(at)?;
        file.write_all(text.as_bytes()).map_err(at)
    }

    pub fn best(&self) -> Best<'_> {
        Best::of(&self.lines)
    }
}

impl<'a> Best<'a> {
    /// The best bounds of `lines`.
    pub fn of(lines: impl IntoIterator<Item = &'a Line>) -> Best<'a> {
        let mut best = Best::default();
        for line in lines {
            match line.side {
                Side::Lower => {
                    if best.lower.get(&line.n).is_none_or(|have| have.value <= line.value) {
                        best.lower.insert(line.n, line);
                    }
                }
                Side::Upper => {
                    if best.upper.get(&line.n).is_none_or(|have| have.value >= line.value) {
                        best.upper.insert(line.n, line);
                    }
                }
            }
        }
        best
    }

    /// The boards with a bound, ascending.
    pub fn boards(&self) -> Vec<u32> {
        let mut boards: Vec<u32> = self.lower.keys().chain(self.upper.keys()).copied().collect();
        boards.sort_unstable();
        boards.dedup();
        boards
    }

    /// The boards whose best lower bound is their best upper bound, ascending.
    pub fn proven(&self) -> Vec<u32> {
        let same = |(n, lower): (&u32, &&Line)| self.upper.get(n).is_some_and(|upper| upper.value == lower.value);
        self.lower.iter().filter(|&entry| same(entry)).map(|(&n, _)| n).collect()
    }
}

/// The proven games of the committed chain, or of any results file, at `path`: the games a store's line may cite.
pub fn chain(path: &Path) -> Result<Optimal, String> {
    Optimal::from_json(&Json::Arr(Vec::new()))?.with_results(path)
}

/// The lines of a store checked one by one against the lines before them, and the chain's proven games
/// (`strict`, results/strict.jsonl: a line may cite a board of the committed chain, whose score is exact).
pub struct Chain<'a> {
    strict: &'a Optimal,
    /// For each board, its lower bounds so far, ascending (each improves on the one before).
    lower: HashMap<u32, Vec<u64>>,
    /// For each board, its upper bounds so far: the value and what it rests on.
    upper: HashMap<u32, Vec<(u64, Vec<u32>)>>,
}

impl<'a> Chain<'a> {
    pub fn new(strict: &'a Optimal) -> Chain<'a> {
        Chain { strict, lower: HashMap::new(), upper: HashMap::new() }
    }

    pub fn best_lower(&self, n: u32) -> Option<u64> {
        self.lower.get(&n)?.last().copied()
    }

    pub fn best_upper(&self, n: u32) -> Option<u64> {
        self.upper.get(&n)?.iter().map(|&(value, _)| value).min()
    }

    /// What the bounds of board `n` that are at most `at_most` rest on: the lines before this one, and the chain's game.
    fn cites(&self, n: u32, at_most: u64) -> Vec<Vec<u32>> {
        let earlier = self.upper.get(&n).into_iter().flatten().filter(|(value, _)| *value <= at_most);
        let chain = self.strict.score(n).filter(|&score| score <= at_most).map(|_| vec![n]);
        earlier.map(|(_, rests)| rests.clone()).chain(chain).collect()
    }

    /// Checks `line` against the lines added before it, and adds it.  Its bound must improve the best of its board and
    /// side; a lower bound may not pass an upper one; a given bound's game is the lower bound at its value; and a
    /// relation's bound must follow from the bound it cites (a line before it, or a game of the chain) by the
    /// arithmetic of the lemma, and rest on at least what that bound rests on (and, for a repeat, on its own board).
    /// A line that fails is not added.
    pub fn add(&mut self, line: &Line) -> Result<(), String> {
        let n = line.n;
        match line.side {
            Side::Lower => {
                if let Some(best) = self.best_lower(n).filter(|&best| best >= line.value) {
                    return Err(format!("it does not improve the lower bound {best}"));
                }
                if let Some(upper) = self.best_upper(n).filter(|&upper| upper < line.value) {
                    return Err(format!("the lower bound {} is above the upper bound {upper}", line.value));
                }
                if let (None, Some(from), Some(value)) = (&line.game, line.from, line.from_value) {
                    if !self.lower.get(&from).is_some_and(|values| values.contains(&value)) {
                        return Err(format!("no line before it gives a game of board {from} worth {value}"));
                    }
                }
                self.lower.entry(n).or_default().push(line.value);
            }
            Side::Upper => {
                if let Some(best) = self.best_upper(n).filter(|&best| best <= line.value) {
                    return Err(format!("it does not improve the upper bound {best}"));
                }
                if let Some(lower) = self.best_lower(n).filter(|&lower| lower > line.value) {
                    return Err(format!("the upper bound {} is below the lower bound {lower}", line.value));
                }
                match (line.rule, line.from, line.from_value) {
                    (Rule::Given, ..) if self.best_lower(n) != Some(line.value) => {
                        return Err("a given bound needs the given game as the lower bound, at the same value".into());
                    }
                    (rule, Some(from), Some(from_value)) if rule.is_relation() => {
                        self.follows(line, from, from_value)?
                    }
                    _ => {}
                }
                self.upper.entry(n).or_default().push((line.value, line.rests_on.clone()));
            }
        }
        Ok(())
    }

    fn follows(&self, line: &Line, from: u32, from_value: u64) -> Result<(), String> {
        let bags = line.bags.map(|(bag, from_bag)| (from_bag, bag));
        let carried = upper_by(line.rule, from, line.n, from_value, bags)?;
        if line.value < carried {
            return Err(format!(
                "U({}) is at most {carried} by {} from U({from}) <= {from_value}, but the line says {}",
                line.n,
                line.rule.name(),
                line.value
            ));
        }
        let cites = self.cites(from, from_value);
        if cites.is_empty() {
            return Err(format!("no line before it, and no game of the chain, bounds board {from} by {from_value}"));
        }
        let own = (line.rule == Rule::Repeat).then_some(line.n);
        let covered = |rests: &Vec<u32>| rests.iter().chain(&own).all(|board| line.rests_on.contains(board));
        if !cites.iter().any(covered) {
            return Err(format!(
                "it rests on {:?}, which leaves out some board that the bound of {from} it comes from rests on, or its own \
                 board (a repeat)",
                line.rests_on
            ));
        }
        Ok(())
    }

    /// What is wrong between the lines and the chain's games: a board of the chain whose score is outside the file's
    /// bounds of it.
    pub fn against_chain(&self) -> Vec<String> {
        let mut found = Vec::new();
        let mut boards: Vec<u32> = self.lower.keys().chain(self.upper.keys()).copied().collect();
        boards.sort_unstable();
        boards.dedup();
        for n in boards {
            let Some(score) = self.strict.score(n) else { continue };
            if let Some(lower) = self.best_lower(n).filter(|&lower| lower > score) {
                found.push(format!("n={n}: the lower bound {lower} is above the chain's score {score}"));
            }
            if let Some(upper) = self.best_upper(n).filter(|&upper| upper < score) {
                found.push(format!("n={n}: the upper bound {upper} is below the chain's score {score}"));
            }
        }
        found
    }
}
