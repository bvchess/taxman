//! The ledger's checks of the bounds store: `results/bounds.jsonl`, the bounds on opt(n) that the ledger finds, kept
//! for good.  One JSON line a bound, appended and never changed; a line is added only when it improves the file's best
//! bound of its board and side.  A board is proven when its best lower bound is its best upper bound.  The format is
//! described in results/README.md, "bounds.jsonl"; its one copy in code is the library's (`taxman::store`: the line,
//! the file, the best bounds), which `taxman solve --use-prev` reads too.  What is here is what only the ledger does:
//! the checks of a line against the lines before it (`Chain`), which `ledger export` makes before it appends,
//! `ledger verify-bounds` makes of every line, and `solver/tools/check_bounds.py` makes again independently.

use crate::book::{upper_by, Rule, Side};
use std::collections::HashMap;
use std::path::Path;
use taxman::game::Optimal;
use taxman::json::Json;

pub use taxman::store::{is_date, Best, Game, Line, Store};

/// The proven games of the committed chain, or of any results file, at `path`: the games a store's line may cite.
pub fn chain(path: &Path) -> Result<Optimal, String> {
    Optimal::from_json(&Json::Arr(Vec::new()))?.with_results(path)
}

/// The lines of a store checked one by one against the lines before them, and the chain's proven games
/// (`strict`, results/optimal.jsonl.gz: a line may cite a board of the committed chain, whose score is exact).
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
