//! The known games: what `taxman solve --use-prev` draws on besides what the run itself has made.
//!
//! Three sources hold games the run has not solved, each with what its game rests on:
//!
//! * **the table** ([`Origin::Optimal`]), the table of optimal games built into the binary: the games of
//!   `results/optimal.jsonl.gz` as of the build.  It records no proofs, so a game of it is *assumed* optimal (`the game
//!   of n in results/optimal.jsonl.gz is optimal`).
//! * **the committed chain** ([`Origin::Strict`]), `results/optimal.jsonl.gz` in the checkout: a chain from n = 1, each
//!   game with its proof and what it assumes.  A game of it is as proven as its line is.
//! * **the bounds store** ([`Origin::Bounds`]), `results/bounds.jsonl` ([`crate::store`]): bounds on opt(n) that the
//!   ledger finds.  A board whose best lower bound is its best upper bound is *proven*: the lower bound's game scores
//!   what the upper bound allows, and the game is optimal wherever statement U holds at the boards its upper bound rests
//!   on (`rests_on`; the statements are the game's assumptions, [`assumed_u`]).  A board with only one of the bounds, or
//!   with a gap, is *open*; the store still helps its search ([`Known::open`]): its lower bound's game is a seed, and
//!   its upper bound a ceiling the search may stop at.
//!
//! [`Known::game`] is the game of n from the best source (the committed chain, then the store, then the table); the
//! solver records it without a search (method `known`, proof `known`) or builds the next game on it.  A result line says
//! which source it took the game from ([`Cite`]), and what the game assumes follows from the source; the referees check
//! the citation against the same sources ([`Known::check_cite`]).
//!
//! The sources are read as they are given: the committed chain and the store are checked by their own referees
//! (`taxman verify-results`, `solver/tools/check_results.py`; `ledger verify-bounds`, `solver/tools/check_bounds.py`),
//! not again here.  The committed chain is a gzip file that decodes to tens of megabytes, so it is decoded and parsed only
//! as far as the games asked for ([`Known::load`]): a run or a check that takes few games from it reads a member or two.
//! What the solver does check of a game it takes is what it checks of every game: the referee's replay
//! (legal, over, opening with the largest prime) at the score the source gives, and the chain bounds against the game
//! before it.

use crate::game::{is_gzip, Optimal, Text};
use crate::json::Json;
use crate::results::{assumed_optimal, assumed_u, committed_games, parse_committed, Assumptions, CommittedGame, Proof};
use crate::store::{carried, play, Line, Rule, Side, Store};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Where a known game comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    /// The table built into the binary.
    Optimal,
    /// The committed chain, `results/optimal.jsonl.gz`.
    Strict,
    /// The bounds store, `results/bounds.jsonl`.
    Bounds,
}

impl Origin {
    /// The name in result lines.
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Optimal => "optimal",
            Origin::Strict => "strict",
            Origin::Bounds => "bounds",
        }
    }

    pub fn parse(name: &str) -> Option<Origin> {
        [Origin::Optimal, Origin::Strict, Origin::Bounds].into_iter().find(|origin| origin.as_str() == name)
    }

    /// Where it came from, for a message.
    pub fn whence(self) -> &'static str {
        match self {
            Origin::Optimal => "the built-in table",
            Origin::Strict => "the committed chain",
            Origin::Bounds => "the bounds store (bounds.jsonl)",
        }
    }
}

/// A line of the bounds store as a result line cites it: the rule that derived the bound and the run that found it.
/// With the board and the value (the line's own board and score) it names the line: a store has one line of a board,
/// side and value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineRef {
    pub rule: Rule,
    pub source: String,
}

impl LineRef {
    fn of(line: &Line) -> LineRef {
        LineRef { rule: line.rule, source: line.source.clone() }
    }

    fn to_json(&self) -> Json {
        Json::object([("rule", Json::from(self.rule.name())), ("source", Json::from(self.source.as_str()))])
    }

    fn from_json(json: &Json) -> Result<LineRef, String> {
        let fields = json.as_obj().ok_or("not an object")?;
        if fields.iter().any(|(key, _)| key != "rule" && key != "source") {
            return Err("only \"rule\" and \"source\"".into());
        }
        let rule = json.get("rule").and_then(Json::as_str).and_then(Rule::of).ok_or("bad \"rule\"")?;
        let source = json.get("source").and_then(Json::as_str).filter(|s| !s.is_empty()).ok_or("bad \"source\"")?;
        Ok(LineRef { rule, source: source.to_owned() })
    }
}

/// What a result line records of the known source its game, or its proof, comes from (its `known` field).  The board
/// and the value of the cited lines are the line's own `n` and `score`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cite {
    /// The game is the table's.
    Optimal,
    /// The game is the committed chain's.
    Strict,
    /// The store's lines: the lower bound whose game the line plays, if it is a game of the store (not if the search
    /// found it), and the upper bound the score meets.
    Bounds { lower: Option<LineRef>, upper: LineRef },
}

impl Cite {
    pub fn origin(&self) -> Origin {
        match self {
            Cite::Optimal => Origin::Optimal,
            Cite::Strict => Origin::Strict,
            Cite::Bounds { .. } => Origin::Bounds,
        }
    }

    pub fn to_json(&self) -> Json {
        let source = ("source", Json::from(self.origin().as_str()));
        match self {
            Cite::Optimal | Cite::Strict => Json::object([source]),
            Cite::Bounds { lower, upper } => Json::object([
                source,
                ("lower", lower.as_ref().map_or(Json::Null, LineRef::to_json)),
                ("upper", upper.to_json()),
            ]),
        }
    }

    pub fn from_json(json: &Json) -> Result<Cite, String> {
        let fields = json.as_obj().ok_or("not an object")?;
        let origin = json.get("source").and_then(Json::as_str).and_then(Origin::parse).ok_or("bad \"source\"")?;
        let allowed: &[&str] = if origin == Origin::Bounds { &["source", "lower", "upper"] } else { &["source"] };
        if let Some((key, _)) = fields.iter().find(|(key, _)| !allowed.contains(&key.as_str())) {
            return Err(format!("the field \"{key}\" is not in a citation of {}", origin.as_str()));
        }
        Ok(match origin {
            Origin::Optimal => Cite::Optimal,
            Origin::Strict => Cite::Strict,
            Origin::Bounds => {
                let lower = match json.get("lower") {
                    None => return Err("no \"lower\"".into()),
                    Some(Json::Null) => None,
                    Some(lower) => Some(LineRef::from_json(lower).map_err(|e| format!("\"lower\": {e}"))?),
                };
                let upper = json.get("upper").ok_or("no \"upper\"")?;
                Cite::Bounds { lower, upper: LineRef::from_json(upper).map_err(|e| format!("\"upper\": {e}"))? }
            }
        })
    }
}

/// A game one of the sources holds, and what it rests on.
#[derive(Clone, Debug)]
pub struct KnownGame {
    pub n: u32,
    pub score: u64,
    pub moves: Vec<u32>,
    pub origin: Origin,
    /// How the source establishes it, as the game of `n − 1` of a result line records it (`prev.proof`): the committed
    /// line's proof; [`Proof::Known`] for a board of the store; none for the table, which records none.
    pub proof: Option<Proof>,
    /// What the game rests on: [`assumed_optimal`] for the table's, the committed line's for the committed chain's, and
    /// [`assumed_u`] at the boards the store's upper bound rests on.
    pub assumes: Assumptions,
    pub cite: Cite,
}

/// An upper bound of the store on a board that it has not proven: a ceiling a search may stop at.
#[derive(Clone, Debug)]
pub struct Bound {
    pub value: u64,
    pub line: LineRef,
    /// [`assumed_u`] at the boards the bound rests on.
    pub assumes: Assumptions,
}

/// What the store has of an open board: the game of its best lower bound, and its best upper bound.
#[derive(Clone, Debug, Default)]
pub struct Open {
    pub game: Option<Vec<u32>>,
    pub upper: Option<Bound>,
}

/// The files `--use-prev` draws on besides the table, and how far it goes.
#[derive(Clone, Debug, Default)]
pub struct KnownSources {
    /// The committed chain.
    pub committed: Option<PathBuf>,
    /// The bounds store.
    pub bounds: Option<PathBuf>,
    /// Whether an n that a source holds a proven game of is recorded without a search (`solve --use-prev`).
    /// Without it the sources give the game before the range, and the store helps the searches of its open boards.
    pub boards: bool,
}

impl KnownSources {
    /// What `solve --use-prev` draws on: every game of the table, the committed chain and the store in the repository's
    /// `results`, for the games of the range as well as the game before it.  The files that are not there
    /// are left out.
    ///
    /// # Errors
    ///
    /// If there is no repository (see [`crate::paths::repository_root`]); the table is built in, and the caller may go
    /// on with it alone.
    pub fn in_repository() -> Result<KnownSources, String> {
        let root = crate::paths::repository_root()?;
        let file = |relative: &str| Some(root.join(relative)).filter(|path| path.is_file());
        Ok(KnownSources {
            committed: file(crate::paths::RESULTS_IN_REPOSITORY),
            bounds: file(crate::paths::BOUNDS_STORE_IN_REPOSITORY),
            boards: true,
        })
    }
}

/// The store, indexed: its best lower and upper line of each board, and each line by board, side and value.
struct Held {
    store: Store,
    lower: HashMap<u32, usize>,
    upper: HashMap<u32, usize>,
    at: HashMap<(u32, Side, u64), usize>,
}

impl Held {
    fn new(store: Store) -> Held {
        let (mut lower, mut upper, mut at) = (HashMap::new(), HashMap::new(), HashMap::new());
        for (i, line) in store.lines.iter().enumerate() {
            // The best line of a board and side is the one `Best::of` picks.
            let best: &mut HashMap<u32, usize> = if line.side == Side::Lower { &mut lower } else { &mut upper };
            if best.get(&line.n).is_none_or(|&j| line.side.replaces(line.value, store.lines[j].value)) {
                best.insert(line.n, i);
            }
            at.entry((line.n, line.side, line.value)).or_insert(i);
        }
        Held { store, lower, upper, at }
    }

    fn line(&self, i: usize) -> &Line {
        &self.store.lines[i]
    }

    /// The game of the lower line `i`, replayed by the referee at its value: its own, or, for a line that cites the game
    /// it is carried from, that of the line it cites (a line before it), carried by the relation.
    fn moves_of(&self, i: usize) -> Result<Vec<u32>, String> {
        let mut chain = vec![i];
        loop {
            let line = self.line(*chain.last().expect("a line"));
            if line.game.is_some() {
                break;
            }
            let (from, value) = line.from.zip(line.from_value).ok_or("a lower bound with no game that cites none")?;
            let cited =
                *self.at.get(&(from, Side::Lower, value)).filter(|&&j| j < *chain.last().expect("a line")).ok_or_else(
                    || format!("it cites a game of board {from} worth {value}, which no line before it has"),
                )?;
            chain.push(cited);
        }
        let first = chain.pop().expect("a line");
        let mut played = play(self.line(first))?;
        while let Some(next) = chain.pop() {
            played = carried(self.line(next), &played.moves)?;
        }
        Ok(played.moves)
    }

    /// The assumptions of an upper line: statement U at the boards it rests on.
    fn assumes(line: &Line) -> Assumptions {
        line.rests_on.iter().map(|&board| assumed_u(board)).collect()
    }
}

/// The committed chain, as the sources give it.
enum Committed {
    /// There is none.
    None,
    /// A plain file, read whole: its completed games by n.
    Games(BTreeMap<u32, CommittedGame>),
    /// A gzip file, `results/optimal.jsonl.gz`, which is one game a line, the line of the game of n the nth, read as its
    /// games are asked for: a check of results that cites nothing of it, or a run that takes a few games from it,
    /// decodes a few members of tens of megabytes.
    Chain(Text<CommittedGame>),
}

impl Committed {
    /// The committed game of `n`, if the chain has one.
    ///
    /// # Errors
    ///
    /// If a gzip chain is damaged where the game is read from, or is not one game a line; the message names the file.
    fn game(&self, n: u32) -> Result<Option<&CommittedGame>, String> {
        match self {
            Committed::None => Ok(None),
            Committed::Games(games) => Ok(games.get(&n)),
            Committed::Chain(text) => text.get(n),
        }
    }

    /// Whether a committed chain with a game in it was given to the check.
    fn is_given(&self) -> bool {
        match self {
            Committed::None => false,
            Committed::Games(games) => !games.is_empty(),
            Committed::Chain(_) => true,
        }
    }
}

/// The known sources, loaded.
pub struct Known<'a> {
    optimal: &'a Optimal,
    committed: Committed,
    held: Option<Held>,
}

impl<'a> Known<'a> {
    /// The table `optimal` alone.
    pub fn table(optimal: &'a Optimal) -> Known<'a> {
        Known { optimal, committed: Committed::None, held: None }
    }

    /// The table of optimal games the sources include.
    pub fn optimal(&self) -> &'a Optimal {
        self.optimal
    }

    /// The table `optimal` and the files of `sources`.
    ///
    /// The committed chain is read whole if it is a plain file.  A gzip file (its name ends in `.gz`, as the committed
    /// results' does) is read into memory and decoded as far as the games asked for (its first member now, so that a
    /// file that is no gzip file is refused), and parsed when asked; it must be one game a line, as the committed
    /// results are, and a fault in a member decoded later is an error of the call that asks for the game, so that a
    /// member that nothing asks for is not read.
    ///
    /// # Errors
    ///
    /// If a file cannot be read or is not what it should be.
    pub fn load(optimal: &'a Optimal, sources: &KnownSources) -> Result<Known<'a>, String> {
        let committed = match sources.committed.as_deref() {
            None => Committed::None,
            Some(path) if is_gzip(path) => Committed::Chain(Text::of_file(path, parse_committed)?),
            Some(path) => Committed::Games(committed_games(path)?),
        };
        let held = sources.bounds.as_deref().map(Store::read).transpose()?.map(Held::new);
        Ok(Known { optimal, committed, held })
    }

    /// The game of `n` that `origin` holds, if it holds a proven one: the table's, the committed chain's, or, for the
    /// store, the game of the best lower bound of a board whose best bounds meet.
    ///
    /// # Errors
    ///
    /// If the store's game does not replay under the referee at its value.
    pub fn game_from(&self, origin: Origin, n: u32) -> Result<Option<KnownGame>, String> {
        let known = |score, moves: Vec<u32>, proof, assumes, cite| {
            Some(KnownGame { n, score, moves, origin, proof, assumes, cite })
        };
        Ok(match origin {
            Origin::Optimal => self.optimal.score(n).zip(self.optimal.moves(n)).and_then(|(score, moves)| {
                known(score, moves.to_vec(), None, Assumptions::from([assumed_optimal(n)]), Cite::Optimal)
            }),
            Origin::Strict => self.committed.game(n)?.and_then(|game| {
                known(game.score, game.moves.clone(), Some(game.proof), game.assumes.clone(), Cite::Strict)
            }),
            Origin::Bounds => {
                let Some(held) = &self.held else { return Ok(None) };
                let (Some(&lower), Some(&upper)) = (held.lower.get(&n), held.upper.get(&n)) else { return Ok(None) };
                let (low, high) = (held.line(lower), held.line(upper));
                if low.value != high.value {
                    return Ok(None);
                }
                let moves = held.moves_of(lower).map_err(|e| format!("the bounds store, n={n}: the game: {e}"))?;
                let cite = Cite::Bounds { lower: Some(LineRef::of(low)), upper: LineRef::of(high) };
                known(high.value, moves, Some(Proof::Known), Held::assumes(high), cite)
            }
        })
    }

    /// The game of `n` from the best source that holds one: the committed chain, the store, then the
    /// table.
    ///
    /// # Errors
    ///
    /// As [`game_from`](Self::game_from).
    pub fn game(&self, n: u32) -> Result<Option<KnownGame>, String> {
        for origin in [Origin::Strict, Origin::Bounds, Origin::Optimal] {
            if let Some(game) = self.game_from(origin, n)? {
                return Ok(Some(game));
            }
        }
        Ok(None)
    }

    /// What the store has of board `n` to help its search: the game of its best lower bound (replayed by the referee),
    /// and its best upper bound.  A board the store has proven is a known game ([`game`](Self::game)); this is for the
    /// others, and gives the same for the proven.
    ///
    /// # Errors
    ///
    /// If the game of the lower bound does not replay under the referee at its value.
    pub fn open(&self, n: u32) -> Result<Open, String> {
        let Some(held) = &self.held else { return Ok(Open::default()) };
        let game = held
            .lower
            .get(&n)
            .map(|&i| held.moves_of(i).map_err(|e| format!("the bounds store, n={n}: the game: {e}")))
            .transpose()?;
        let upper = held.upper.get(&n).map(|&i| {
            let line = held.line(i);
            Bound { value: line.value, line: LineRef::of(line), assumes: Held::assumes(line) }
        });
        Ok(Open { game, upper })
    }

    /// Checks a citation of a result line against the sources: board `n`'s game scores `score` there, the cited lines
    /// exist with those values, rules and sources.  Returns what the citation makes the game assume.  The sources are not
    /// checked themselves (their own referees do), and which game the line plays is not compared: a finished game of
    /// the score the source gives for `n` is optimal as the source's game is.
    ///
    /// # Errors
    ///
    /// If a source the citation needs was not given, or has no such line or game, or a different one.
    pub fn check_cite(&self, n: u32, score: u64, cite: &Cite) -> Result<Assumptions, String> {
        match cite {
            Cite::Optimal => match self.optimal.score(n) {
                Some(table) if table == score => Ok(Assumptions::from([assumed_optimal(n)])),
                Some(table) => {
                    Err(format!("it cites the built-in table, whose game of {n} scores {table}, not {score}"))
                }
                None => Err(format!("it cites the built-in table, which has no game of {n}")),
            },
            Cite::Strict => match self.committed.game(n)? {
                Some(game) if game.score == score => Ok(game.assumes.clone()),
                Some(game) => {
                    Err(format!("it cites the committed chain, whose game of {n} scores {}, not {score}", game.score))
                }
                None if !self.committed.is_given() => {
                    Err("it cites the committed chain, which was not given to the check".into())
                }
                None => Err(format!("it cites the committed chain, which has no game of {n}")),
            },
            Cite::Bounds { lower, upper } => {
                let held = self.held.as_ref().ok_or("it cites the bounds store, which was not given to the check")?;
                let find = |side: Side, wanted: &LineRef| -> Result<&Line, String> {
                    let line = held.at.get(&(n, side, score)).map(|&i| held.line(i)).ok_or_else(|| {
                        format!("the bounds store has no {} bound of board {n} worth {score}", side.name())
                    })?;
                    match LineRef::of(line) == *wanted {
                        true => Ok(line),
                        false => Err(format!(
                            "the bounds store's {} bound of board {n} worth {score} is by {} from {}, not by {} from {} as cited",
                            side.name(),
                            line.rule.name(),
                            line.source,
                            wanted.rule.name(),
                            wanted.source
                        )),
                    }
                };
                if let Some(lower) = lower {
                    find(Side::Lower, lower)?;
                }
                Ok(Held::assumes(find(Side::Upper, upper)?))
            }
        }
    }
}
