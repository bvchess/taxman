//! The book of the ledger: for every board size n, a lower bound L(n), the score of a game it holds, and an upper
//! bound U(n) on opt(n), each with its derivation (the rule and the board it came from), and the relations between
//! board sizes, applied to a fixpoint.  The relations are proven in heuristics/THEORY.md, "The ledger: bounds between
//! boards"; each lemma there gives an L direction (a game of one board made from a game of another, which the
//! referee replays here before it counts) and a U direction (an inequality between the optima):
//!
//! * `monotone` (Lemma L1): opt(n − 1) ≤ opt(n).  L(n) from the game of n − 1, followed by n when it is still legal
//!   (n composite); U(n − 1) ≤ U(n).
//! * `ceiling` (Lemma L2): opt(n) ≤ opt(n − 1) + n.  U(n) ≤ U(n − 1) + n; L(n − 1) from the game of n without the
//!   move n, completed (n composite).
//! * `prime` (Lemma L3): for a prime p and q the largest prime below it, opt(p) = opt(p − 1) + p − q, both ways, the
//!   games by exchanging the opening q and p.
//! * `repeat` (Lemma L4): boards with the same search problem have the same best promotion sum, so U(n) − bag(n)
//!   ≤ U(m) − bag(m); L(n) from the promotions of m's game played on n's board.
//!
//! The sources (a board's own bounds) are the standalone start and root bound (`start`, `root`: the map, as `wall
//! landscape` computes it), searches (`search`, `dual`, `exhausted`, Lemma D for the dual bound), proven games given
//! (`given`) and boards with nothing to promote (`trivial`).  Every bound only improves: L only rises and U only
//! falls, and a board is proven when they meet.  L > U anywhere is a violation, which [`Book::propagate`] reports as an
//! error: a bug, or, since the U of the frame model rests on statement U, a counterexample to it.

use std::collections::{BTreeMap, HashMap, HashSet};
use taxman::accelerate::solution_for_promotions;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::game::{largest_prime_at_most, legal_moves_left, replay_finished};
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::search::Problem;

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
    /// A proven game given to the ledger: both bounds.
    Given,
    /// The relations between boards.
    Monotone,
    Ceiling,
    Prime,
    Repeat,
}

impl Rule {
    pub const ALL: [Rule; 11] = [
        Rule::Trivial,
        Rule::Start,
        Rule::Root,
        Rule::Search,
        Rule::Dual,
        Rule::Exhausted,
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

/// A lower bound: a finished game of the board (the referee's `replay_finished` accepts it) and its score.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lower {
    pub value: u64,
    pub moves: Vec<u32>,
    /// The promotions that make the game in the frame model (`solution_for_promotions`), when it was made so.
    pub promotions: Option<Vec<u32>>,
    pub rule: Rule,
    pub from: Option<u32>,
}

/// An upper bound on opt(n).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Upper {
    pub value: u64,
    pub rule: Rule,
    pub from: Option<u32>,
}

/// What the ledger keeps of a board size.
#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub n: u32,
    pub bag: u64,
    pub prime: bool,
    pub lower: Option<Lower>,
    pub upper: Option<Upper>,
    /// The searches made of this board, the level of the next one (its limit doubles with each), and their seconds
    /// and steps in all.
    pub attempts: u32,
    pub level: u32,
    pub search_s: f64,
    pub nodes: u64,
    /// The board's own map is made (`start` and `root`), which every board searched has first.
    pub mapped: bool,
    /// The bounds changed through a proven neighbour since the last search: the board goes before the others.
    pub boosted: bool,
    /// The CPU seconds of the run (all its threads, all its runs) when the board was proven.
    pub proven_cpu: Option<f64>,
}

impl Entry {
    pub fn gap(&self) -> Option<i64> {
        Some(self.upper?.value as i64 - self.lower.as_ref()?.value as i64)
    }

    pub fn proven(&self) -> bool {
        self.gap() == Some(0)
    }

    pub fn to_json(&self, class: Option<u32>) -> Json {
        let lower = self.lower.as_ref().map_or(Json::Null, |l| {
            Json::object([
                ("value", Json::from(l.value)),
                ("rule", Json::from(l.rule.name())),
                ("from", l.from.map_or(Json::Null, Json::from)),
                ("moves", Json::array(l.moves.iter().copied())),
                ("promotions", l.promotions.as_ref().map_or(Json::Null, |p| Json::array(p.iter().copied()))),
            ])
        });
        let upper = self.upper.map_or(Json::Null, |u| {
            Json::object([
                ("value", Json::from(u.value)),
                ("rule", Json::from(u.rule.name())),
                ("from", u.from.map_or(Json::Null, Json::from)),
            ])
        });
        Json::object([
            ("n", Json::from(self.n)),
            ("in_the_bag", Json::from(self.bag)),
            ("prime", Json::from(self.prime)),
            ("class", class.map_or(Json::Null, Json::from)),
            ("lower", lower),
            ("upper", upper),
            ("gap", self.gap().map_or(Json::Null, Json::from)),
            ("proven", Json::from(self.proven())),
            ("proven_cpu_s", self.proven_cpu.map_or(Json::Null, Json::from)),
            ("attempts", Json::from(self.attempts)),
            ("level", Json::from(self.level)),
            ("search_s", Json::from(self.search_s)),
            ("nodes", Json::from(self.nodes)),
            ("mapped", Json::from(self.mapped)),
            ("boosted", Json::from(self.boosted)),
        ])
    }

    /// The entry of a ledger line (as [`to_json`](Self::to_json) writes it).
    pub fn from_json(line: &Json) -> Result<Entry, String> {
        let field = |name: &str| line.get(name).ok_or_else(|| format!("no \"{name}\""));
        let number = |name: &str| field(name)?.as_u64().ok_or_else(|| format!("bad \"{name}\""));
        let rule = |bound: &Json| {
            let name = bound.get("rule").and_then(Json::as_str).ok_or("a bound with no rule")?;
            Rule::of(name).ok_or_else(|| format!("no rule \"{name}\""))
        };
        let from = |bound: &Json| bound.get("from").and_then(Json::as_u32);
        let lower = match field("lower")? {
            Json::Null => None,
            l => Some(Lower {
                value: l.get("value").and_then(Json::as_u64).ok_or("a lower bound with no value")?,
                moves: l.get("moves").and_then(Json::as_u32_vec).ok_or("a lower bound with no moves")?,
                promotions: l.get("promotions").and_then(Json::as_u32_vec),
                rule: rule(l)?,
                from: from(l),
            }),
        };
        let upper = match field("upper")? {
            Json::Null => None,
            u => Some(Upper {
                value: u.get("value").and_then(Json::as_u64).ok_or("an upper bound with no value")?,
                rule: rule(u)?,
                from: from(u),
            }),
        };
        Ok(Entry {
            n: field("n")?.as_u32().ok_or("bad \"n\"")?,
            bag: number("in_the_bag")?,
            prime: field("prime")?.as_bool().ok_or("bad \"prime\"")?,
            lower,
            upper,
            attempts: number("attempts")? as u32,
            level: number("level")? as u32,
            search_s: field("search_s")?.as_f64().ok_or("bad \"search_s\"")?,
            nodes: number("nodes")?,
            mapped: field("mapped")?.as_bool().ok_or("bad \"mapped\"")?,
            boosted: field("boosted")?.as_bool().ok_or("bad \"boosted\"")?,
            proven_cpu: line.get("proven_cpu_s").and_then(Json::as_f64),
        })
    }
}

/// What a board is, from its setup: its bag, whether n is prime, whether it has nothing to promote (and then its
/// game, the one with no promotions), its candidates and its search problem.
pub struct Facts {
    pub n: u32,
    pub bag: u64,
    pub prime: bool,
    pub trivial: bool,
    pub plain: Vec<u32>,
    pub candidates: Vec<u32>,
    pub problem: Option<Problem>,
}

/// The facts of board `n`, set up as `taxman solve` sets it up.
pub fn facts(n: u32) -> Result<Facts, String> {
    taxman::with_width!(n, W => facts_at::<W>(n), else Err(format!("n={n} is beyond the widest set")))
}

fn facts_at<const W: usize>(n: u32) -> Result<Facts, String> {
    let board = Board::<W>::for_n(n);
    let trivial = board.is_trivial();
    Ok(Facts {
        n,
        bag: board.in_the_bag,
        prime: board.tables.is_prime(n),
        trivial,
        plain: board.plain.clone(),
        candidates: board.candidates().to_vec(),
        problem: if trivial { None } else { Some(board.problem()?) },
    })
}

/// Which side of a board's bounds changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// A bound that improved.
#[derive(Clone, Debug)]
pub struct Change {
    pub n: u32,
    pub side: Side,
    pub before: Option<u64>,
    pub after: u64,
    pub rule: Rule,
    pub from: Option<u32>,
    /// The board it came from was proven at the time.
    pub from_proven: bool,
}

/// The ledger's bounds of every board it has, with the repeat classes and the candidates the relations need.
pub struct Book {
    pub entries: BTreeMap<u32, Entry>,
    /// The repeat class of each board in one: the boards with the same search problem, ascending (classes of one
    /// board are left out).
    classes: HashMap<u32, std::rc::Rc<Vec<u32>>>,
    candidates: HashMap<u32, Vec<u32>>,
    /// For each pair (source, target) of an L transfer, the source's L when the transfer was last made, so that a
    /// transfer is made once for each L of its source.
    transferred: HashMap<(u32, u32, Link), u64>,
}

impl Book {
    /// A book of the boards of `facts`, with no bounds but those of the boards with nothing to promote, and the repeat
    /// classes of their search problems (equal problems, found by digest and compared whole).
    pub fn new(facts: Vec<Facts>) -> Book {
        let mut book = Book {
            entries: BTreeMap::new(),
            classes: HashMap::new(),
            candidates: HashMap::new(),
            transferred: HashMap::new(),
        };
        let mut by_digest: HashMap<u64, Vec<(u32, &Problem)>> = HashMap::new();
        for f in &facts {
            if let Some(problem) = &f.problem {
                by_digest.entry(problem.digest()).or_default().push((f.n, problem));
            }
        }
        for group in by_digest.values() {
            let mut left: Vec<(u32, &Problem)> = group.clone();
            while let Some((n, problem)) = left.first().copied() {
                let (same, rest): (Vec<_>, Vec<_>) = left.into_iter().partition(|(_, other)| *other == problem);
                let mut members: Vec<u32> = same.iter().map(|(m, _)| *m).collect();
                members.sort_unstable();
                debug_assert!(members.contains(&n));
                if members.len() > 1 {
                    let class = std::rc::Rc::new(members);
                    for &m in class.iter() {
                        book.classes.insert(m, class.clone());
                    }
                }
                left = rest;
            }
        }
        for f in facts {
            let mut entry = Entry { n: f.n, bag: f.bag, prime: f.prime, ..Entry::default() };
            if f.trivial {
                let value = f.plain.iter().map(|&m| u64::from(m)).sum();
                entry.lower =
                    Some(Lower { value, moves: f.plain, promotions: Some(vec![]), rule: Rule::Trivial, from: None });
                entry.upper = Some(Upper { value, rule: Rule::Trivial, from: None });
            }
            book.candidates.insert(f.n, f.candidates);
            book.entries.insert(f.n, entry);
        }
        book
    }

    /// The boards of `n`'s repeat class, ascending, with `n` (just `n` if it has none).
    pub fn class(&self, n: u32) -> Vec<u32> {
        self.classes.get(&n).map_or_else(|| vec![n], |class| class.to_vec())
    }

    /// The smallest board of `n`'s repeat class, if it has one.
    pub fn class_of(&self, n: u32) -> Option<u32> {
        self.classes.get(&n).map(|class| class[0])
    }

    /// Whether `n` is a prime whose board before it is in the book: its bounds are those of n − 1 by the prime formula
    /// (both ways), so it is never searched.
    pub fn slaved(&self, n: u32) -> bool {
        self.entries.get(&n).is_some_and(|e| e.prime) && self.entries.contains_key(&(n - 1))
    }

    /// The board that is searched for `n`'s repeat class: its smallest board that is not slaved, if any.
    pub fn representative(&self, n: u32) -> Option<u32> {
        self.class(n).into_iter().find(|&m| !self.slaved(m))
    }

    /// Whether `n` is searched: it is the representative of its class.
    pub fn searched(&self, n: u32) -> bool {
        self.representative(n) == Some(n)
    }

    /// Offers `n` a lower bound; returns the change if it is better than the one it has.
    pub fn offer_lower(&mut self, n: u32, lower: Lower, from_proven: bool) -> Result<Option<Change>, String> {
        let entry = self.entries.get_mut(&n).ok_or_else(|| format!("n={n} is not in the ledger"))?;
        let before = entry.lower.as_ref().map(|l| l.value);
        if before.is_some_and(|b| b >= lower.value) {
            return Ok(None);
        }
        let change = Change {
            n,
            side: Side::Lower,
            before,
            after: lower.value,
            rule: lower.rule,
            from: lower.from,
            from_proven,
        };
        entry.lower = Some(lower);
        check(entry)?;
        Ok(Some(change))
    }

    /// Offers `n` an upper bound; returns the change if it is better than the one it has.
    pub fn offer_upper(&mut self, n: u32, upper: Upper, from_proven: bool) -> Result<Option<Change>, String> {
        let entry = self.entries.get_mut(&n).ok_or_else(|| format!("n={n} is not in the ledger"))?;
        let before = entry.upper.map(|u| u.value);
        if before.is_some_and(|b| b <= upper.value) {
            return Ok(None);
        }
        let change = Change {
            n,
            side: Side::Upper,
            before,
            after: upper.value,
            rule: upper.rule,
            from: upper.from,
            from_proven,
        };
        entry.upper = Some(upper);
        check(entry)?;
        Ok(Some(change))
    }

    /// Applies the relations from the boards `changed` (whose bounds changed) to a fixpoint: every board a relation
    /// reaches from a changed board gets the bounds it derives, if they are better, and is a changed board in turn.
    /// Returns the changes, in order.  An error is a violation (L > U), or a game a relation made that the referee
    /// refuses: either is a bug, or a counterexample to statement U (THEORY.md, "The ledger").
    pub fn propagate(&mut self, changed: impl IntoIterator<Item = u32>) -> Result<Vec<Change>, String> {
        let mut queue: Vec<u32> = changed.into_iter().collect();
        let mut queued: HashSet<u32> = queue.iter().copied().collect();
        let mut changes = Vec::new();
        while let Some(source) = queue.pop() {
            queued.remove(&source);
            let mut links = vec![(source + 1, Link::Up)];
            links.extend(source.checked_sub(1).map(|below| (below, Link::Down)));
            links.extend(self.class(source).into_iter().filter(|&m| m != source).map(|m| (m, Link::Repeat)));
            links.retain(|(t, _)| self.entries.contains_key(t));
            for (target, link) in links {
                let before = changes.len();
                self.derive(source, target, link, &mut changes)?;
                if changes.len() > before && queued.insert(target) {
                    queue.push(target);
                }
            }
        }
        Ok(changes)
    }

    /// The bounds of `target` that the relation `link` derives from `source`'s.
    fn derive(&mut self, source: u32, target: u32, link: Link, changes: &mut Vec<Change>) -> Result<(), String> {
        let s = &self.entries[&source];
        let (s_lower, s_upper, s_bag, s_prime, s_proven) = (s.lower.clone(), s.upper, s.bag, s.prime, s.proven());
        let (t_bag, t_prime) = (self.entries[&target].bag, self.entries[&target].prime);
        // The relation, and the upper bound it gives the target.
        let (relation, upper) = match link {
            // Lemma L3 (prime): U(p) <= U(p - 1) + p - q; Lemma L2 (the ceiling): U(n) <= U(n - 1) + n.
            Link::Up if t_prime => {
                let q = largest_prime_at_most(source).map_or(0, u64::from);
                (Rule::Prime, s_upper.map(|u| (u.value + u64::from(target)).saturating_sub(q)))
            }
            Link::Up => (Rule::Monotone, s_upper.map(|u| u.value + u64::from(target))),
            // Lemma L3 (prime): U(p - 1) <= U(p) - p + q; Lemma L1 (monotone): U(n - 1) <= U(n).
            Link::Down if s_prime => {
                let q = largest_prime_at_most(target).map_or(0, u64::from);
                (Rule::Prime, s_upper.map(|u| (u.value + q).saturating_sub(u64::from(source))))
            }
            Link::Down => (Rule::Ceiling, s_upper.map(|u| u.value)),
            // Lemma L4: U(n) - bag(n) <= U(m) - bag(m).
            Link::Repeat => (Rule::Repeat, s_upper.map(|u| (u.value + t_bag).saturating_sub(s_bag))),
        };
        // The upper bound's rule names the lemma's inequality: up a board it is the ceiling, down a board monotonicity.
        let upper_rule = match (link, relation) {
            (Link::Up, Rule::Monotone) => Rule::Ceiling,
            (Link::Down, Rule::Ceiling) => Rule::Monotone,
            _ => relation,
        };
        if let Some(value) = upper {
            if let Some(change) =
                self.offer_upper(target, Upper { value, rule: upper_rule, from: Some(source) }, s_proven)?
            {
                changes.push(change);
            }
        }
        let Some(lower) = s_lower else { return Ok(()) };
        if self.transferred.insert((source, target, link), lower.value) == Some(lower.value) {
            return Ok(());
        }
        let t_value = self.entries[&target].lower.as_ref().map_or(0, |l| l.value);
        let made = match relation {
            Rule::Prime if link == Link::Up => Some((prime_up(target, &lower.moves), None)),
            Rule::Prime => Some((prime_down(target, &lower.moves), None)),
            Rule::Monotone => Some((monotone_game(target, &lower.moves)?, None)),
            Rule::Ceiling => Some((ceiling_game(target, &lower.moves)?, None)),
            _ => {
                // A repeat is worth making only if the source's game has more promotions to give than the target's.
                let more = lower.value as i64 - s_bag as i64 > t_value as i64 - t_bag as i64;
                more.then(|| {
                    let promotions = lower.promotions.clone().unwrap_or_else(|| {
                        let cands: HashSet<u32> = self.candidates[&source].iter().copied().collect();
                        lower.moves.iter().copied().filter(|m| cands.contains(m)).collect()
                    });
                    (repeat_game(target, &promotions), Some(promotions))
                })
            }
        };
        let Some((moves, promotions)) = made else { return Ok(()) };
        let what = || format!("the {} game of {target} from the game of {source}", relation.name());
        let value = match replay_finished(target, &moves) {
            Ok(value) => value,
            // Moves of a game that was not made from promotions need not be a promotion set of the frame model.
            Err(_) if relation == Rule::Repeat && lower.promotions.is_none() => return Ok(()),
            Err(e) => return Err(format!("{}: {e}", what())),
        };
        if let (Rule::Repeat, Some(_), Some(promotions)) = (relation, &lower.promotions, &promotions) {
            let sum: u64 = promotions.iter().map(|&x| u64::from(x)).sum();
            if value != t_bag + sum {
                return Err(format!("{}: scores {value}, not the bag {t_bag} and the promotions' {sum}", what()));
            }
        }
        if value > t_value {
            let promotions = promotions.filter(|_| lower.promotions.is_some());
            let lower = Lower { value, moves, promotions, rule: relation, from: Some(source) };
            if let Some(change) = self.offer_lower(target, lower, s_proven)? {
                changes.push(change);
            }
        }
        Ok(())
    }
}

/// Lemma L1: the game of `n` (composite) that the finished game `moves` of n − 1 gives: the same moves, then n if it is
/// still legal.
pub fn monotone_game(n: u32, moves: &[u32]) -> Result<Vec<u32>, String> {
    let mut game = moves.to_vec();
    if legal_moves_left(n, moves)?.contains(&n) {
        game.push(n);
    }
    Ok(game)
}

/// Lemma L2: the game of `n` that the finished game `moves` of n + 1 (composite) gives: its moves without n + 1, which
/// stay legal, then the largest legal move as long as there is one.
pub fn ceiling_game(n: u32, moves: &[u32]) -> Result<Vec<u32>, String> {
    let mut game: Vec<u32> = moves.iter().copied().filter(|&m| m != n + 1).collect();
    while let Some(&m) = legal_moves_left(n, &game)?.last() {
        game.push(m);
    }
    Ok(game)
}

/// Lemma L3: the game of the prime `p` that the finished game `moves` of p − 1 gives: its opening move, the largest
/// prime below p, exchanged for p (`accelerate::prime_shortcut`).
pub fn prime_up(p: u32, moves: &[u32]) -> Vec<u32> {
    taxman::accelerate::prime_shortcut(p, moves)
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

/// Lemma L4: `promotions` played on the board of `n` (`solution_for_promotions`).
pub fn repeat_game(n: u32, promotions: &[u32]) -> Vec<u32> {
    taxman::with_width!(n, W => solution_for_promotions::<W>(&Tables::new(n), Bits::<W>::from_slice(promotions)))
}

/// How two boards are related: the target is the board after the source, the one before it, or one of its repeat
/// class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Link {
    Up,
    Down,
    Repeat,
}

/// L > U at a board is a violation.
fn check(entry: &Entry) -> Result<(), String> {
    match (&entry.lower, &entry.upper) {
        (Some(l), Some(u)) if l.value > u.value => Err(format!(
            "violation at n={}: the lower bound {} ({}{}) exceeds the upper bound {} ({}{})",
            entry.n,
            l.value,
            l.rule.name(),
            l.from.map_or_else(String::new, |m| format!(" from {m}")),
            u.value,
            u.rule.name(),
            u.from.map_or_else(String::new, |m| format!(" from {m}")),
        )),
        _ => Ok(()),
    }
}
