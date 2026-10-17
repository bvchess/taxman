//! The ledger's tests: the relations on the boards of optimal.json (every bound they derive brackets the optimum, and
//! every game they make is legal), a violation, and runs of the scheduler checked against optimal.json.

use super::book::{Book, Lower, Rule, Upper};
use super::report;
use super::schedule::{self, read_ledger, Settings};
use super::support::TempDir;
use std::collections::BTreeMap;
use taxman::board::Board;
use taxman::game::{replay_finished, Optimal};

fn opt(n: u32) -> u64 {
    Optimal::built_in().score(n).expect("a board of optimal.json")
}

fn opt_game(n: u32) -> Vec<u32> {
    Optimal::built_in().moves(n).expect("a board of optimal.json").to_vec()
}

/// A book of the boards `ns`, with no bounds but those of the trivial boards.
fn book_of(ns: impl Iterator<Item = u32>) -> Book {
    let ns: Vec<u32> = ns.collect();
    Book::new(schedule::all_facts(&ns, 8).expect("the boards' facts"))
}

/// Gives the book the optimal game of `n` as proven.
fn give(book: &mut Book, n: u32) {
    let lower = Lower { value: opt(n), moves: opt_game(n), promotions: None, rule: Rule::Given, from: None };
    book.offer_lower(n, lower, true).unwrap();
    book.offer_upper(n, Upper { value: opt(n), rule: Rule::Given, from: None }, true).unwrap();
}

/// Every bound of the book brackets the optimum, and every game replays to its lower bound.  Returns how many
/// lower and upper bounds each rule has.
fn brackets(book: &Book) -> BTreeMap<(&'static str, Rule), usize> {
    let mut rules = BTreeMap::new();
    for e in book.entries.values() {
        let n = e.n;
        if let Some(l) = &e.lower {
            assert_eq!(
                replay_finished(n, &l.moves),
                Ok(l.value),
                "n={n}: the game of L ({:?} from {:?})",
                l.rule,
                l.from
            );
            assert!(l.value <= opt(n), "n={n}: L = {} ({:?} from {:?}) > opt {}", l.value, l.rule, l.from, opt(n));
            *rules.entry(("lower", l.rule)).or_default() += 1;
        }
        if let Some(u) = e.upper {
            assert!(u.value >= opt(n), "n={n}: U = {} ({:?} from {:?}) < opt {}", u.value, u.rule, u.from, opt(n));
            *rules.entry(("upper", u.rule)).or_default() += 1;
        }
    }
    rules
}

/// Proven games every tenth board of 2..1300 and nothing else: the relations give every board both bounds, all of
/// them on the right side of the optimum and every game legal, with each relation used for both bounds; and they are
/// exact where the lemmas are equalities (a prime after a proven board, a repeat of one).
#[test]
fn the_relations_bracket_the_optimum_of_every_board_to_1300() {
    let mut book = book_of(2..=1300);
    let given: Vec<u32> = (10..=1300).step_by(10).collect();
    for &n in &given {
        give(&mut book, n);
    }
    let changes = book.propagate(given.iter().copied()).unwrap();
    assert!(changes.len() > 2000, "{} changes", changes.len());
    let rules = brackets(&book);
    for e in book.entries.values() {
        assert!(e.lower.is_some() && e.upper.is_some(), "n={}: no bound", e.n);
    }
    for rule in [Rule::Monotone, Rule::Ceiling, Rule::Prime, Rule::Repeat] {
        for side in ["lower", "upper"] {
            assert!(rules.get(&(side, rule)).copied().unwrap_or(0) > 0, "no {side} bound by {rule:?}: {rules:?}");
        }
    }
    for e in book.entries.values() {
        let n = e.n;
        let exact = (e.prime && given.contains(&(n - 1)))
            || book.class(n).iter().any(|m| given.contains(m))
            || e.lower.as_ref().is_some_and(|l| l.rule == Rule::Given);
        if exact {
            assert!(e.proven() && e.upper.unwrap().value == opt(n), "n={n}: not proven at the optimum: {e:?}");
        }
    }
}

/// Loose bounds everywhere (U above the optimum by up to 48, L the game with no promotions) and a proven game every
/// fiftieth board: what the relations derive still brackets the optimum, and tightens many of the loose bounds.
#[test]
fn loose_bounds_and_the_relations_bracket_the_optimum() {
    let ns = 1001..=1300;
    let mut book = book_of(ns.clone());
    for n in ns.clone() {
        let plain = taxman::with_width!(n, W => Board::<W>::for_n(n).plain);
        let value = replay_finished(n, &plain).unwrap();
        book.offer_lower(n, Lower { value, moves: plain, promotions: None, rule: Rule::Start, from: None }, false)
            .unwrap();
        let loose = Upper { value: opt(n) + 3 * u64::from(n % 17), rule: Rule::Root, from: None };
        book.offer_upper(n, loose, false).unwrap();
    }
    let given: Vec<u32> = ns.clone().filter(|n| n % 50 == 0).collect();
    for &n in &given {
        give(&mut book, n);
    }
    book.propagate(ns).unwrap();
    let rules = brackets(&book);
    let derived =
        |side| rules.iter().filter(|((s, r), _)| *s == side && r.is_relation()).map(|(_, c)| c).sum::<usize>();
    assert!(derived("lower") > 100 && derived("upper") > 30, "{rules:?}");
}

/// A lower bound above an upper bound is refused as a violation, whether offered or derived.
#[test]
fn a_lower_bound_above_an_upper_bound_is_a_violation() {
    let mut book = book_of(1000..=1002);
    let low = Upper { value: opt(1001) - 1, rule: Rule::Root, from: None };
    book.offer_upper(1001, low, false).unwrap();
    let lower = Lower { value: opt(1001), moves: opt_game(1001), promotions: None, rule: Rule::Given, from: None };
    assert!(book.offer_lower(1001, lower, true).unwrap_err().contains("violation at n=1001"));

    // The lower bound the relations derive for 1000 from a proven 1002, and the same with a wrong upper bound of 1000
    // one below it.
    let derived = |upper: Option<u64>| {
        let mut book = book_of(1000..=1002);
        give(&mut book, 1002);
        if let Some(value) = upper {
            book.offer_upper(1000, Upper { value, rule: Rule::Root, from: None }, false).unwrap();
        }
        book.propagate([1000, 1002]).map(|_| book.entries[&1000].lower.as_ref().unwrap().value)
    };
    let lower = derived(None).unwrap();
    let err = derived(Some(lower - 1)).unwrap_err();
    assert!(err.contains("violation at n=1000"), "{err}");
}

/// A run of the scheduler on 901..960 with searches of a few milliseconds at first: every board is proven at its
/// optimum, by searches (stopped ones give dual bounds) and by the relations, and a second run on the same directory
/// finds nothing to do and keeps every bound; the report's check passes.
#[test]
fn a_run_proves_a_stretch_at_the_optimum_and_goes_on_from_its_ledger() {
    let dir = TempDir::new("ledger-run");
    let settings = Settings { workers: 3, cpu_budget: 1e9, base: 0.002, save_every: 0.5, ..Settings::default() };
    let reason = schedule::run(901, 960, &dir, &settings, None).unwrap();
    assert_eq!(reason, "all proven");
    let entries = read_ledger(&dir).unwrap();
    assert_eq!(entries.len(), 60);
    for e in &entries {
        assert!(e.proven() && e.upper.unwrap().value == opt(e.n), "n={}: {e:?}", e.n);
    }
    let rule_count = |rule: Rule| entries.iter().filter(|e| e.upper.unwrap().rule == rule).count();
    assert!(rule_count(Rule::Prime) > 0 && rule_count(Rule::Repeat) > 0, "{entries:?}");
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    assert!(events.contains("\"action\":\"search\"") && events.contains("\"completed\":false"), "no search stopped");
    assert!(!dir.join("ckpt").read_dir().unwrap().any(|_| true), "the checkpoints of proven boards are deleted");

    let again = schedule::run(901, 960, &dir, &settings, None).unwrap();
    assert_eq!(again, "all proven");
    let kept = read_ledger(&dir).unwrap();
    for (a, b) in entries.iter().zip(&kept) {
        assert_eq!((a.n, &a.lower, a.upper, a.attempts), (b.n, &b.lower, b.upper, b.attempts));
    }
    let (text, ok) = report::report(&dir, &[]).unwrap();
    assert!(ok, "{text}");
    assert!(text.contains("60 in the ledger, 60 proven"), "{text}");
}

/// A run given proven games (a results file) takes them as fixed points, and the boards past them follow.
#[test]
fn a_run_takes_the_proven_games_it_is_given() {
    let dir = TempDir::new("ledger-given");
    let results = dir.join("given.jsonl");
    let line = |n: u32| {
        let moves: Vec<String> = opt_game(n).iter().map(u32::to_string).collect();
        format!("{{\"n\":{n},\"score\":{},\"moves\":[{}],\"completed\":true}}\n", opt(n), moves.join(","))
    };
    std::fs::write(&results, [line(700), line(705)].concat()).unwrap();
    let settings = Settings { workers: 2, cpu_budget: 1e9, base: 0.01, ..Settings::default() };
    assert_eq!(schedule::run(701, 712, &dir, &settings, Some(&results)).unwrap(), "all proven");
    let entries = read_ledger(&dir).unwrap();
    assert_eq!(entries.first().map(|e| e.n), Some(700), "the given board before the stretch is in the ledger");
    for e in &entries {
        assert!(e.proven() && e.upper.unwrap().value == opt(e.n), "n={}: {e:?}", e.n);
    }
    for n in [700, 705] {
        assert_eq!(entries.iter().find(|e| e.n == n).unwrap().lower.as_ref().unwrap().rule, Rule::Given);
    }
}
