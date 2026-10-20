//! The bounds store: `results/bounds.jsonl`, the bounds on opt(n) that the ledger (`examples/ledger/`) finds, kept for
//! good.  One JSON line a bound, appended and never changed; a line is added only when it improves the file's best
//! bound of its board and side.  A board is proven when its best lower bound is its best upper bound.  The format is
//! described in `results/README.md`, "bounds.jsonl"; this module is its one copy in code, shared by the ledger (which
//! writes, checks and extends the store) and by `taxman solve --use-prev` ([`crate::known`], which reads it):
//!
//! * the vocabulary of a bound: [`Rule`], how it was derived, and [`Side`];
//! * the relations between boards, with the arithmetic of the upper bounds ([`upper_by`]) and the games that carry a
//!   lower bound across ([`carry_game`]), which docs/THEORY.md, "The ledger: bounds between boards", proves;
//! * the line ([`Line`], [`Game`]) and the file ([`Store`], [`Best`]);
//! * a lower bound's game as the referee has it ([`play`], [`expand`], [`carried`]).
//!
//! The checks of a line against the lines before it (`Chain`) are the ledger's, in `examples/ledger/store.rs`.

use crate::accelerate::{prime_shortcut, solution_for_promotions};
use crate::bits::Bits;
use crate::board::Board;
use crate::game::{complete, largest_prime_at_most, replay_finished};
use crate::json::Json;
use crate::numtheory::Tables;
use crate::solver::MAX_N;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::path::Path;

/// The schema number of a line.
pub const SCHEMA: u64 = 1;

// ---------------------------------------------------------------------------------------------------------
// The vocabulary

/// How a bound was derived.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rule {
    /// The game with no promotions of a board that has nothing to promote: both bounds.
    Trivial,
    /// The standalone start incumbent (L) and the root's GF(2) bound (U), as the map has them.
    Start,
    Root,
    /// A search's best game (L); the dual bound of a search that a limit stopped (U); the game of a search that
    /// completed (U: it explored its tree).
    Search,
    Dual,
    Exhausted,
    /// A search under a floor F that completed having found no set (U): P* <= F (Lemma F), so U is in_the_bag + F.
    Floor,
    /// The Lagrangian bound over the nogoods of searches, each checked to fail to promote, with the GF(2) bound inside
    /// (U): the removed `wall nogoods` made it; the store takes it on record.
    Nogoods,
    /// The game the finder found (L): large neighbourhood search, which is not sound and proves nothing; its game is a
    /// lower bound because the referee replays it.
    Lns,
    /// A proven game given to the ledger: both bounds.
    Given,
    /// The relations between boards.
    Monotone,
    Ceiling,
    Prime,
    Repeat,
}

impl Rule {
    pub const ALL: [Rule; 14] = [
        Rule::Trivial,
        Rule::Start,
        Rule::Root,
        Rule::Search,
        Rule::Dual,
        Rule::Exhausted,
        Rule::Floor,
        Rule::Nogoods,
        Rule::Lns,
        Rule::Given,
        Rule::Monotone,
        Rule::Ceiling,
        Rule::Prime,
        Rule::Repeat,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Rule::Trivial => "trivial",
            Rule::Start => "start",
            Rule::Root => "root",
            Rule::Search => "search",
            Rule::Dual => "dual",
            Rule::Exhausted => "exhausted",
            Rule::Floor => "floor",
            Rule::Nogoods => "nogoods",
            Rule::Lns => "lns",
            Rule::Given => "given",
            Rule::Monotone => "monotone",
            Rule::Ceiling => "ceiling",
            Rule::Prime => "prime",
            Rule::Repeat => "repeat",
        }
    }

    pub fn of(name: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|rule| rule.name() == name)
    }

    /// Whether the rule relates two boards (its bound has a `from`).
    pub fn is_relation(self) -> bool {
        matches!(self, Rule::Monotone | Rule::Ceiling | Rule::Prime | Rule::Repeat)
    }
}

/// Which side of a board's bounds a bound is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Lower,
    Upper,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::Lower => "lower",
            Side::Upper => "upper",
        }
    }

    pub fn of(name: &str) -> Option<Side> {
        [Side::Lower, Side::Upper].into_iter().find(|side| side.name() == name)
    }

    /// Whether a bound of this side worth `new` takes the place of the best so far, worth `have`: a higher lower bound,
    /// a lower upper bound, and of equal values the later line (the rule of [`Best`]).
    pub fn replaces(self, new: u64, have: u64) -> bool {
        match self {
            Side::Lower => new >= have,
            Side::Upper => new <= have,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The relations between boards (docs/THEORY.md, "The ledger: bounds between boards")

/// Lemma L1: the game of `n` (composite) that the finished game `moves` of n − 1 gives: the same moves, then n if it is
/// still legal.  (n is the only move that can be left: the legal moves that `complete` adds are n, or none.)
pub fn monotone_game(n: u32, moves: &[u32]) -> Result<Vec<u32>, String> {
    complete(n, moves)
}

/// Lemma L2: the game of `n` that the finished game `moves` of n + 1 (composite) gives: its moves without n + 1, which
/// stay legal, then the largest legal move as long as there is one.
pub fn ceiling_game(n: u32, moves: &[u32]) -> Result<Vec<u32>, String> {
    let without: Vec<u32> = moves.iter().copied().filter(|&m| m != n + 1).collect();
    complete(n, &without)
}

/// Lemma L3: the game of the prime `p` that the finished game `moves` of p − 1 gives: its opening move, the largest
/// prime below p, exchanged for p (`accelerate::prime_shortcut`).
pub fn prime_up(p: u32, moves: &[u32]) -> Vec<u32> {
    prime_shortcut(p, moves)
}

/// Lemma L3: the game of `n` = p − 1 that the finished game `moves` of the prime p gives: its opening move p exchanged
/// for the largest prime up to n (none for n = 1, whose game has no move).
pub fn prime_down(n: u32, moves: &[u32]) -> Vec<u32> {
    match largest_prime_at_most(n) {
        Some(q) => {
            let mut game = moves.to_vec();
            if let Some(first) = game.first_mut() {
                *first = q;
            }
            game
        }
        None => Vec::new(),
    }
}

/// Whether the relation `rule` (named for the lemma that carries a game) carries a game from the board `from` to the
/// board `n`: `Monotone` up a board to a composite, `Ceiling` down a board from a composite, `Prime` across a prime, in
/// either direction (Lemmas L1 to L3).  The game is made by [`carry_game`].
pub fn carries(rule: Rule, from: u32, n: u32) -> bool {
    let prime = |m: u32| largest_prime_at_most(m) == Some(m);
    match rule {
        Rule::Monotone => from + 1 == n && !prime(n),
        Rule::Ceiling => from == n + 1 && !prime(from),
        Rule::Prime => (from + 1 == n && prime(n)) || (from == n + 1 && prime(from)),
        _ => false,
    }
}

/// The game of `n` that the relation `rule` makes of the finished game `moves` of the board `from` (see [`carries`]).
///
/// # Errors
///
/// `rule` does not carry a game from `from` to `n`, or the game is not a game of `from`.
pub fn carry_game(rule: Rule, from: u32, n: u32, moves: &[u32]) -> Result<Vec<u32>, String> {
    if !carries(rule, from, n) {
        return Err(format!("the {} relation does not carry a game from {from} to {n}", rule.name()));
    }
    match rule {
        Rule::Monotone => monotone_game(n, moves),
        Rule::Ceiling => ceiling_game(n, moves),
        _ if from < n => Ok(prime_up(n, moves)),
        _ => Ok(prime_down(n, moves)),
    }
}

/// Lemma L4: `promotions` played on the board of `n` (`solution_for_promotions`).
pub fn repeat_game(n: u32, promotions: &[u32]) -> Vec<u32> {
    crate::with_width!(n, W => solution_for_promotions::<W>(&Tables::new(n), Bits::<W>::from_slice(promotions)))
}

/// The upper bound that the relation `rule` gives opt(`target`) from the upper bound `value` on opt(`source`): for the
/// boards next to each other, `Ceiling` (the target is the board after the source: opt(n) <= opt(n − 1) + n),
/// `Monotone` (the board before: opt(n − 1) <= opt(n)) and `Prime` (either way, across a prime p, q the largest prime
/// below it: opt(p) = opt(p − 1) + p − q); and for boards with the same search problem, `Repeat`, with their bags
/// (`bags`: the source's, the target's): U(n) − bag(n) <= U(m) − bag(m).  Lemmas L1 to L4 (docs/THEORY.md, "The
/// ledger: bounds between boards"); the book and the store's verifiers use this one copy of the arithmetic.
///
/// # Errors
///
/// `rule` is not a relation, the boards are not so related, or a repeat has no bags.
pub fn upper_by(rule: Rule, source: u32, target: u32, value: u64, bags: Option<(u64, u64)>) -> Result<u64, String> {
    let prime_below = |n: u32| u64::from(largest_prime_at_most(n).unwrap_or(0));
    let is_prime = |n: u32| largest_prime_at_most(n) == Some(n);
    let (s, t) = (u64::from(source), u64::from(target));
    match rule {
        Rule::Ceiling if target == source + 1 => Ok(value + t),
        Rule::Monotone if source == target + 1 => Ok(value),
        Rule::Prime if target == source + 1 && is_prime(target) => Ok((value + t).saturating_sub(prime_below(source))),
        Rule::Prime if source == target + 1 && is_prime(source) => Ok((value + prime_below(target)).saturating_sub(s)),
        Rule::Repeat if source != target => {
            let (s_bag, t_bag) = bags.ok_or("a repeat needs the bags of its two boards")?;
            Ok((value + t_bag).saturating_sub(s_bag))
        }
        _ => Err(format!("the {} bound of {target} cannot come from {source}", rule.name())),
    }
}

// ---------------------------------------------------------------------------------------------------------
// The line and the file

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
            let side = match line.side {
                Side::Lower => &mut best.lower,
                Side::Upper => &mut best.upper,
            };
            if side.get(&line.n).is_none_or(|have| line.side.replaces(line.value, have.value)) {
                side.insert(line.n, line);
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

// ---------------------------------------------------------------------------------------------------------
// A lower bound's game, as the referee has it

/// The game of the promotions `set` on `board`, as the referee has it: its score and moves.  The referee replays the
/// game (legal, over, opening with the largest prime), and its score must be the board's `in_the_bag` and the sum of
/// the promotions, as it is for every promotion set of the frame model.
pub fn refereed<const W: usize>(board: &Board<W>, set: &Bits<W>) -> Result<(u64, Vec<u32>), String> {
    let moves = solution_for_promotions(&board.tables, *set);
    let value = replay_finished(board.n(), &moves)?;
    if value == board.in_the_bag + set.sum() {
        Ok((value, moves))
    } else {
        Err(format!("scores {value}, not {} and {}", board.in_the_bag, set.sum()))
    }
}

/// The game of the promotions `set` on the board of `n`, as the referee has it ([`refereed`]): its score and moves.
pub fn expand(n: u32, set: &[u32]) -> Result<(u64, Vec<u32>), String> {
    if set.iter().any(|&member| member < 2 || member > n) {
        return Err(format!("the promotions are not numbers of the board 1..={n}"));
    }
    crate::with_width!(
        n,
        W => refereed(&Board::<W>::for_n(n), &Bits::<W>::from_slice(set)),
        else Err(format!("n={n} is beyond the widest set"))
    )
}

/// A lower bound's game as the referee has it: its moves, and the promotions it was made from if it was.
pub struct Played {
    pub moves: Vec<u32>,
    pub promotions: Option<Vec<u32>>,
}

/// The game that a line of the store carries (its promotions or its moves), replayed by the referee: it must score the
/// line's value.  (A line is data from a file: promotions that no board has made would stop the frames' setup, and
/// that is an error of the line.)  A line that cites the game it is carried from has none: see [`carried`].
pub fn play(line: &Line) -> Result<Played, String> {
    std::panic::catch_unwind(AssertUnwindSafe(|| play_checked(line)))
        .unwrap_or_else(|_| Err("the frame model's setup stops on these promotions".into()))
}

fn play_checked(line: &Line) -> Result<Played, String> {
    let n = line.n;
    let (score, moves, promotions) = match line.game.as_ref().ok_or("it has no game")? {
        Game::Promotions(set) => {
            let (score, moves) = expand(n, set)?;
            (score, moves, Some(set.clone()))
        }
        Game::Moves(moves) => (replay_finished(n, moves)?, moves.clone(), None),
    };
    if score == line.value {
        Ok(Played { moves, promotions })
    } else {
        Err(format!("the game scores {score}, not the {} of the line", line.value))
    }
}

/// The game of `line`, which cites the game of its source, as the relation carries it from `source` (the moves of the
/// game the citation names), replayed by the referee at the line's value.
pub fn carried(line: &Line, source: &[u32]) -> Result<Played, String> {
    let from = line.from.ok_or("it has no game and cites none")?;
    let moves = carry_game(line.rule, from, line.n, source)?;
    let score = replay_finished(line.n, &moves)?;
    if score == line.value {
        Ok(Played { moves, promotions: None })
    } else {
        Err(format!("the game scores {score}, not the {} of the line", line.value))
    }
}
