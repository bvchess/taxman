//! `ledger verify-bounds`, the Rust referee of a bounds store, and `ledger games`, the moves of its promotion sets for
//! the Python referee (tools/check_bounds.py).

use crate::book::{facts, Rule, Side};
use crate::export::ranges;
use crate::pool;
use crate::schedule::{play, play_all, Played};
use crate::store::{self, Chain, Game, Line, Store};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::json::Json;
use taxman::search::SearchTactics;

/// A check that takes a board's setup, made on a thread of the pool.
enum Task<'a> {
    /// A root bound is the root's GF(2) bound.
    Root(usize, &'a Line),
    /// The repeats of one board: the bags they record, and the search problems they claim to share with it.
    Repeats(u32, Vec<(usize, &'a Line)>),
}

/// The root bound of board `n` as the map has it: `in_the_bag` and the GF(2) bound of the root with all the
/// candidates, which is what the dual bound of a search stopped before its first node is (the tests of
/// `src/search/dual.rs` check that, and the largest of it and the start incumbent, which is never above it).
pub fn root_upper(n: u32) -> Result<u64, String> {
    taxman::with_width!(n, W => root_upper_at::<W>(n), else Err(format!("n={n} is beyond the widest set")))
}

fn root_upper_at<const W: usize>(n: u32) -> Result<u64, String> {
    let board = Board::<W>::for_n(n);
    let p = board.prepare(SearchTactics::default())?;
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let mut bound = Gf2Bound::<W>::new(&graph);
    bound.reset(&Bits::EMPTY, &p.candidates);
    Ok(board.in_the_bag + bound.evaluate(p.max_total, None).value)
}

/// What is wrong with the lines of a task, each with its line number.
fn check(task: &Task) -> Vec<(usize, String)> {
    let at = |no: usize, line: &Line, why: String| (no, format!("line {no} (n={}): {why}", line.n));
    match task {
        Task::Root(no, line) => match root_upper(line.n) {
            Ok(value) if value == line.value => Vec::new(),
            Ok(value) => vec![at(*no, line, format!("the root bound is {value}, not {}", line.value))],
            Err(e) => vec![at(*no, line, e)],
        },
        Task::Repeats(from, lines) => {
            let source = match facts(*from) {
                Ok(source) => source,
                Err(e) => return lines.iter().map(|&(no, line)| at(no, line, e.clone())).collect(),
            };
            let mut found = Vec::new();
            for &(no, line) in lines {
                match facts(line.n) {
                    Ok(target) => {
                        if line.bags != Some((target.bag, source.bag)) {
                            let (bag, from_bag) = (target.bag, source.bag);
                            found.push(at(no, line, format!("the bags are {bag} and {from_bag}, not {:?}", line.bags)));
                        }
                        if target.problem.is_none() || target.problem != source.problem {
                            found.push(at(no, line, format!("board {from} does not pose the same search problem")));
                        }
                    }
                    Err(e) => found.push(at(no, line, e)),
                }
            }
            found
        }
    }
}

/// The report on the store at `path` and whether it passed.  Every line is parsed; every lower bound's game replayed by
/// the referee (`schedule::play_all`) at its value; every root bound recomputed; every repeat's bags and search problem
/// compared with its source's; every relation's arithmetic checked against the line it cites (a line before it, or a
/// game of the chain `strict`), with what it rests on; and every board of the chain against the file's bounds.  The
/// bounds of searches (`exhausted`, `dual`, `floor`), the Lagrangian bounds over nogoods (`nogoods`) and given games
/// are taken on their record, and listed.
pub fn verify(path: &Path, strict: &Path, workers: usize) -> Result<(String, bool), String> {
    let (numbered, parse_problems) = Store::parse(path)?;
    let games = store::chain(strict)?;
    let mut problems: Vec<(usize, String)> = parse_problems.into_iter().map(|p| (0, p)).collect();
    let mut chain = Chain::new(&games);
    for (no, line) in &numbered {
        if let Err(e) = chain.add(line) {
            problems.push((*no, format!("line {no} (n={}): {e}", line.n)));
        }
    }
    problems.extend(chain.against_chain().into_iter().map(|p| (0, p)));

    let mut tasks = Vec::new();
    let mut repeats: BTreeMap<u32, Vec<(usize, &Line)>> = BTreeMap::new();
    for (no, line) in &numbered {
        match (line.side, line.rule, line.from) {
            (Side::Upper, Rule::Root, _) => tasks.push(Task::Root(*no, line)),
            (Side::Upper, Rule::Repeat, Some(from)) => repeats.entry(from).or_default().push((*no, line)),
            _ => {}
        }
    }
    tasks.extend(repeats.into_iter().map(|(from, lines)| Task::Repeats(from, lines)));
    problems.extend(pool::map(&tasks, workers, check).into_iter().flatten());
    let lower: Vec<(usize, &Line)> =
        numbered.iter().filter(|(_, line)| line.side == Side::Lower).map(|(no, line)| (*no, line)).collect();
    let played = play_all(&lower.iter().map(|&(_, line)| line).collect::<Vec<_>>(), workers);
    for (&(no, line), played) in lower.iter().zip(played) {
        if let Err(e) = played {
            problems.push((no, format!("line {no} (n={}): the game: {e}", line.n)));
        }
    }
    problems.sort();

    let lines: Vec<&Line> = numbered.iter().map(|(_, line)| line).collect();
    let best = store::Best::of(lines.iter().copied());
    let count = |side: Side, rules: &[Rule]| lines.iter().filter(|l| l.side == side && rules.contains(&l.rule)).count();
    let by_rule = |side: Side| -> String {
        let mut counts: BTreeMap<Rule, usize> = BTreeMap::new();
        lines.iter().filter(|l| l.side == side).for_each(|l| *counts.entry(l.rule).or_default() += 1);
        counts.iter().map(|(rule, n)| format!("{} {n}", rule.name())).collect::<Vec<_>>().join(", ")
    };
    let promoted = lines.iter().filter(|l| matches!(l.game, Some(Game::Promotions(_)))).count();
    let cited = lines.iter().filter(|l| l.side == Side::Lower && l.game.is_none()).count();
    let boards = best.boards();
    let mut out = String::new();
    let w = &mut out;
    let _ = writeln!(
        w,
        "{}: {} lines for {} boards{}",
        path.display(),
        lines.len(),
        boards.len(),
        boards.first().zip(boards.last()).map_or_else(String::new, |(a, b)| format!(", {a}..{b}")),
    );
    let _ = writeln!(w, "lower bounds {}: {}", count(Side::Lower, &Rule::ALL), by_rule(Side::Lower));
    let _ = writeln!(
        w,
        "  {} games replayed by the referee ({promoted} from promotion sets, {} from moves, {cited} carried from the game \
         of another line), each legal, finished and at its value",
        count(Side::Lower, &Rule::ALL),
        count(Side::Lower, &Rule::ALL) - promoted - cited,
    );
    let _ = writeln!(w, "upper bounds {}: {}", count(Side::Upper, &Rule::ALL), by_rule(Side::Upper));
    let relations = lines.iter().filter(|l| l.side == Side::Upper && l.rule.is_relation()).count();
    let _ = writeln!(
        w,
        "  {} root bounds recomputed; {relations} by relations, each checked against the line it cites (or the chain's \
         game) and what it rests on; the {} repeats' bags and search problems recomputed",
        count(Side::Upper, &[Rule::Root]),
        count(Side::Upper, &[Rule::Repeat]),
    );
    for rule in [Rule::Exhausted, Rule::Dual, Rule::Floor, Rule::Nogoods, Rule::Given] {
        let ns: Vec<u32> = best.upper.iter().filter(|(_, l)| l.rule == rule).map(|(&n, _)| n).collect();
        let _ = writeln!(
            w,
            "  on record, not recomputed: {} {} lines; best upper bound of {} boards{}",
            count(Side::Upper, &[rule]),
            rule.name(),
            ns.len(),
            if ns.is_empty() { String::new() } else { format!(": {}", ranges(&ns)) },
        );
    }
    let proven = best.proven();
    let _ = writeln!(w, "proven (best lower bound = best upper bound): {} boards{}", proven.len(), {
        if proven.is_empty() {
            String::new()
        } else {
            format!(": {}", ranges(&proven))
        }
    });
    let in_chain = boards.iter().filter(|&&n| games.score(n).is_some()).count();
    let _ = if in_chain == 0 {
        writeln!(w, "no board of the file is a game of the chain ({})", strict.display())
    } else {
        writeln!(
            w,
            "{in_chain} boards of the file are also games of the chain ({}), whose scores its bounds enclose",
            strict.display()
        )
    };
    let ok = problems.is_empty();
    if ok {
        let _ = writeln!(w, "all checks passed");
    } else {
        let _ = writeln!(w, "{} PROBLEMS:", problems.len());
        for (_, problem) in problems.iter().take(50) {
            let _ = writeln!(w, "  {problem}");
        }
        if problems.len() > 50 {
            let _ = writeln!(w, "  ... and {} more", problems.len() - 50);
        }
    }
    Ok((out, ok))
}

/// Writes to `out` a line `{"n", "value", "moves"}` for every lower bound of the store at `path` that is a promotion
/// set: the game that `solution_for_promotions` makes of it, replayed by the referee first.  The Python referee
/// (tools/check_bounds.py) reads them, as it cannot rebuild a game from promotions.  Returns how many.
pub fn games(path: &Path, out: &mut impl std::io::Write, workers: usize) -> Result<usize, String> {
    let store = Store::read(path)?;
    let promoted: Vec<&Line> = store.lines.iter().filter(|l| matches!(l.game, Some(Game::Promotions(_)))).collect();
    for chunk in promoted.chunks((workers * 8).max(1)) {
        for (line, game) in chunk.iter().zip(pool::map(chunk, workers, |&line| play(line))) {
            let Played { moves, .. } = game.map_err(|e| format!("n={}, {}: the game: {e}", line.n, line.value))?;
            let json = Json::object([
                ("n", Json::from(line.n)),
                ("value", Json::from(line.value)),
                ("moves", Json::array(moves)),
            ]);
            writeln!(out, "{json}").map_err(|e| format!("the output: {e}"))?;
        }
    }
    out.flush().map_err(|e| format!("the output: {e}"))?;
    Ok(promoted.len())
}
