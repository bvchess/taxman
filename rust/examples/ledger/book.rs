//! The book of the ledger: for every board size n, a lower bound L(n), the score of a game it holds, and an upper
//! bound U(n) on opt(n), each with its derivation (the rule and the board it came from), and the relations between
//! board sizes, applied to a fixpoint.  The relations are proven in heuristics/THEORY.md, "The ledger: bounds between
//! boards"; each lemma there gives an L direction (a game of one board made from a game of another, which the
//! referee replays here before it counts, and which is made and replayed only if it can beat the board's own) and a
//! U direction (an inequality between the optima):
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
//! landscape` computes it), searches (`search`, `dual`, `exhausted`, Lemma D for the dual bound; `floor`, Lemma F, a
//! search under a floor that completed having found nothing: U is the floor), the finder's games (`lns`: L only),
//! proven games given (`given`), boards with nothing to promote (`trivial`), and the Lagrangian bound over checked
//! nogoods (`nogoods`: U only, which nothing makes now: the removed `wall nogoods` did).  Every bound only
//! improves: L only rises and U only falls, and a board is proven when they meet.  L > U anywhere is a violation, which
//! [`Book::propagate`] reports as an error: a bug, or, since the U of the frame model rests on statement U, a
//! counterexample to it.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use taxman::board::Board;
use taxman::game::replay_finished;
use taxman::json::Json;
use taxman::search::Problem;

/// The vocabulary of a bound, and the relations between boards, are the bounds store's (`taxman::store`), which
/// `taxman solve` reads too.
pub use taxman::store::{carries, carry_game, repeat_game, upper_by, Rule, Side};

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

/// A floor step that completed having found no set: the search under the floor `floor` (a score) explored its whole
/// tree, which took `nodes` steps and `seconds` (over all its tries).  Its cost, against how far below the root's
/// bound the floor is, is a point of the board's line (`cost.rs`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub floor: u64,
    pub nodes: u64,
    pub seconds: f64,
}

/// A floor step that its limit stopped with nothing found: its checkpoint is kept, good only under the floor `floor`
/// (a score), and the step is tried again under it with the limit doubled, `tries` times so far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stalled {
    pub floor: u64,
    pub nodes: u64,
    pub seconds: f64,
    pub tries: u32,
}

impl Step {
    fn to_json(self) -> Json {
        Json::object([
            ("floor", Json::from(self.floor)),
            ("nodes", Json::from(self.nodes)),
            ("seconds", Json::from(self.seconds)),
        ])
    }

    fn from_json(json: &Json) -> Result<Step, String> {
        let get = |name: &str| json.get(name).ok_or_else(|| format!("a step with no \"{name}\""));
        Ok(Step {
            floor: get("floor")?.as_u64().ok_or("bad step \"floor\"")?,
            nodes: get("nodes")?.as_u64().ok_or("bad step \"nodes\"")?,
            seconds: get("seconds")?.as_f64().ok_or("bad step \"seconds\"")?,
        })
    }
}

impl Stalled {
    fn to_json(self) -> Json {
        Json::object([
            ("floor", Json::from(self.floor)),
            ("nodes", Json::from(self.nodes)),
            ("seconds", Json::from(self.seconds)),
            ("tries", Json::from(self.tries)),
        ])
    }

    fn from_json(json: &Json) -> Result<Stalled, String> {
        let step = Step::from_json(json)?;
        let tries = json.get("tries").and_then(Json::as_u32).ok_or("a stalled step with no \"tries\"")?;
        Ok(Stalled { floor: step.floor, nodes: step.nodes, seconds: step.seconds, tries })
    }
}

/// What the ledger keeps of a board size.
#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub n: u32,
    pub bag: u64,
    pub prime: bool,
    pub lower: Option<Lower>,
    pub upper: Option<Upper>,
    /// The searches made of this board (ordinary ones, climbs and floor steps), the level of the next ordinary one
    /// (its limit doubles with each), and their seconds (the finder's included) in all.
    pub attempts: u32,
    pub level: u32,
    pub search_s: f64,
    /// The steps of the board's ordinary searches and climbs over all runs; the floor steps' are in `steps` and
    /// `stalled`.
    pub nodes: u64,
    /// The root's GF(2) bound, as a score: how far below it a floor is measures the cost of the step (`cost.rs`).
    /// Known once the board is mapped; a ledger written before the floor steps has none until a run reads it.
    pub root: Option<u64>,
    /// The floor steps that completed, in order, and the one the limit stopped, if any.
    pub steps: Vec<Step>,
    pub stalled: Option<Stalled>,
    /// The score of the best seed the board had when the finder last ran on it, its own game included (none: it has
    /// not run): the finder runs again before a search only if the best seed is another score.
    pub lns_score: Option<u64>,
    /// The board's own map is made (`start` and `root`), which every board searched has first.
    pub mapped: bool,
    /// The bounds changed through a proven neighbour since the last search: the board goes before the others.
    pub boosted: bool,
    /// A climb to the board stopped at its limit: it goes by the ordinary order from now on, and is not climbed to again.
    pub parked: bool,
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
            ("root", self.root.map_or(Json::Null, Json::from)),
            ("steps", Json::array(self.steps.iter().copied().map(Step::to_json))),
            ("stalled", self.stalled.map_or(Json::Null, |s| s.to_json())),
            ("lns_score", self.lns_score.map_or(Json::Null, Json::from)),
            ("mapped", Json::from(self.mapped)),
            ("boosted", Json::from(self.boosted)),
            ("parked", Json::from(self.parked)),
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
            // Ledgers written before the floor steps have none of the next three.
            root: line.get("root").and_then(Json::as_u64),
            steps: match line.get("steps") {
                None | Some(Json::Null) => Vec::new(),
                Some(steps) => {
                    steps.as_arr().ok_or("bad \"steps\"")?.iter().map(Step::from_json).collect::<Result<_, _>>()?
                }
            },
            stalled: match line.get("stalled") {
                None | Some(Json::Null) => None,
                Some(stalled) => Some(Stalled::from_json(stalled)?),
            },
            // Ledgers written before the finder have no such field.
            lns_score: line.get("lns_score").and_then(Json::as_u64),
            mapped: field("mapped")?.as_bool().ok_or("bad \"mapped\"")?,
            boosted: field("boosted")?.as_bool().ok_or("bad \"boosted\"")?,
            // Ledgers written before the climbs have no such field.
            parked: line.get("parked").and_then(Json::as_bool).unwrap_or(false),
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
    /// The changed boards go in sweeps, up from the lowest and then from the lowest again, so that a chain of changes up
    /// the boards (each one's ceiling from the one before) is passed on in one sweep and a chain down them in the
    /// next, and a board is not improved again by every source that happens to reach it first.
    /// Returns the changes, in order.  An error is a violation (L > U), or a game a relation made that the referee
    /// refuses: either is a bug, or a counterexample to statement U (THEORY.md, "The ledger").
    pub fn propagate(&mut self, changed: impl IntoIterator<Item = u32>) -> Result<Vec<Change>, String> {
        let mut pending: BTreeSet<u32> = changed.into_iter().collect();
        let mut cursor = 0;
        let mut changes = Vec::new();
        while let Some(source) = pending.range(cursor..).next().or_else(|| pending.first()).copied() {
            pending.remove(&source);
            cursor = source + 1;
            let mut links = vec![(source + 1, Link::Up)];
            links.extend(source.checked_sub(1).map(|below| (below, Link::Down)));
            links.extend(self.class(source).into_iter().filter(|&m| m != source).map(|m| (m, Link::Repeat)));
            links.retain(|(t, _)| self.entries.contains_key(t));
            for (target, link) in links {
                let before = changes.len();
                self.derive(source, target, link, &mut changes)?;
                if changes.len() > before {
                    pending.insert(target);
                }
            }
        }
        Ok(changes)
    }

    /// The bounds of `target` that the relation `link` derives from `source`'s.  The upper bound is arithmetic.  The
    /// lower bound is a game: made once for each lower bound its source has (and not at all if the target's cannot be
    /// beaten by it), and shown to the referee only if its score, the sum of its moves, is more than the target's.
    fn derive(&mut self, source: u32, target: u32, link: Link, changes: &mut Vec<Change>) -> Result<(), String> {
        let s = &self.entries[&source];
        let (s_upper, s_bag, s_prime, s_proven) = (s.upper, s.bag, s.prime, s.proven());
        let s_value = s.lower.as_ref().map(|l| l.value);
        let t = &self.entries[&target];
        let (t_bag, t_prime, t_value) = (t.bag, t.prime, t.lower.as_ref().map_or(0, |l| l.value));
        // The relation that carries a game across the link, and the rule that names the upper bound (up a board it is
        // the ceiling, down a board monotonicity).  Lemma L3 (prime): U(p) <= U(p - 1) + p - q, and the game's exchange
        // of the opening q for p scores that much more, and the same down; Lemma L2 (the ceiling): U(n) <= U(n - 1) +
        // n; Lemma L1 (monotone): the game of n - 1 scores as it did, or n more, and U(n - 1) <= U(n); Lemma L4: U(n) -
        // bag(n) <= U(m) - bag(m), and the promotions of the source's game on the target's board score its bag and
        // their sum (a game that was not made from promotions is made only if it has more to give than the target's,
        // on the same terms).  `upper_by` has the arithmetic, which bounds the score of a game too, except down a board.
        let (relation, upper_rule) = match link {
            Link::Up if t_prime => (Rule::Prime, Rule::Prime),
            Link::Up => (Rule::Monotone, Rule::Ceiling),
            Link::Down if s_prime => (Rule::Prime, Rule::Prime),
            // The game below that of n + 1: its moves less n + 1, completed.
            Link::Down => (Rule::Ceiling, Rule::Monotone),
            Link::Repeat => (Rule::Repeat, Rule::Repeat),
        };
        let gain = |value: u64| {
            upper_by(upper_rule, source, target, value, Some((s_bag, t_bag))).expect("a relation between linked boards")
        };
        let upper = s_upper.map(|u| gain(u.value));
        let at_most = if relation == Rule::Ceiling { None } else { s_value.map(gain) };
        if let Some(value) = upper {
            if let Some(change) =
                self.offer_upper(target, Upper { value, rule: upper_rule, from: Some(source) }, s_proven)?
            {
                changes.push(change);
            }
        }
        let Some(s_value) = s_value else { return Ok(()) };
        if self.transferred.insert((source, target, link), s_value) == Some(s_value) {
            return Ok(());
        }
        if at_most.is_some_and(|most| most <= t_value) {
            return Ok(());
        }
        let lower = self.entries[&source].lower.as_ref().expect("a source with a score has its lower bound");
        let from_promotions = lower.promotions.is_some();
        let (moves, promotions) = match relation {
            Rule::Prime | Rule::Monotone | Rule::Ceiling => (carry_game(relation, source, target, &lower.moves)?, None),
            _ => {
                let promotions = lower.promotions.clone().unwrap_or_else(|| {
                    let cands: HashSet<u32> = self.candidates[&source].iter().copied().collect();
                    lower.moves.iter().copied().filter(|m| cands.contains(m)).collect()
                });
                (repeat_game(target, &promotions), Some(promotions))
            }
        };
        let what = || format!("the {} game of {target} from the game of {source}", relation.name());
        let value: u64 = moves.iter().map(|&m| u64::from(m)).sum();
        if let (Rule::Repeat, true, Some(promotions)) = (relation, from_promotions, &promotions) {
            let sum: u64 = promotions.iter().map(|&x| u64::from(x)).sum();
            if value != t_bag + sum {
                return Err(format!("{}: scores {value}, not the bag {t_bag} and the promotions' {sum}", what()));
            }
        }
        if value <= t_value {
            return Ok(());
        }
        match replay_finished(target, &moves) {
            Ok(score) => debug_assert_eq!(score, value, "{}: the score of a game is the sum of its moves", what()),
            // Moves of a game that was not made from promotions need not be a promotion set of the frame model.
            Err(_) if relation == Rule::Repeat && !from_promotions => return Ok(()),
            Err(e) => return Err(format!("{}: {e}", what())),
        }
        let promotions = promotions.filter(|_| from_promotions);
        let lower = Lower { value, moves, promotions, rule: relation, from: Some(source) };
        if let Some(change) = self.offer_lower(target, lower, s_proven)? {
            changes.push(change);
        }
        Ok(())
    }
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
