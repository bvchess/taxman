//! The ledger's tests: the relations on the boards of optimal.json (every bound they derive brackets the optimum, and
//! every game they make is legal), a violation, the choice of the next search (the climbs) and runs of the scheduler
//! checked against optimal.json.

use super::book::{ceiling_game, monotone_game, prime_down, prime_up, Book, Entry, Lower, Rule, Upper};
use super::report;
use super::schedule::{self, read_ledger, Attempt, Done, Job, Kind, Running, Scheduler, Settings};
use super::support::TempDir;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::{largest_prime_at_most, replay_finished, Optimal};
use taxman::json::Json;

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

/// The games the relations make from the optimal game of a neighbour are finished and score what the lemmas say: the
/// exchange of the opening primes by exactly p - q, the game of n - 1 followed by n by its sum or n more, the game
/// below that of n + 1 by at least its sum less n + 1 and at most opt(n).  The ledger relies on the first and the
/// second to leave a game unmade or unrefereed when it cannot beat the target's.
#[test]
fn the_games_the_relations_make_score_as_the_lemmas_say() {
    let q_of = |p: u32| u64::from(largest_prime_at_most(p - 1).expect("a prime below"));
    for n in 3..=1299 {
        let prime = largest_prime_at_most(n) == Some(n);
        if prime {
            let q = q_of(n);
            let up = prime_up(n, &opt_game(n - 1));
            assert_eq!(replay_finished(n, &up), Ok(opt(n - 1) + u64::from(n) - q), "n={n}: the exchange up");
            let down = prime_down(n - 1, &opt_game(n));
            assert_eq!(replay_finished(n - 1, &down), Ok(opt(n) + q - u64::from(n)), "n={n}: the exchange down");
        } else {
            let up = monotone_game(n, &opt_game(n - 1)).unwrap();
            let score = replay_finished(n, &up).unwrap_or_else(|e| panic!("n={n}: the game of n - 1 and n: {e}"));
            assert!(score == opt(n - 1) || score == opt(n - 1) + u64::from(n), "n={n}: scores {score}");
        }
        // The game of n below the game of n + 1, when n + 1 is composite.
        if largest_prime_at_most(n + 1) != Some(n + 1) {
            let below = ceiling_game(n, &opt_game(n + 1)).unwrap();
            let score = replay_finished(n, &below).unwrap_or_else(|e| panic!("n={n}: the game below n + 1: {e}"));
            assert!(opt(n + 1) <= score + u64::from(n + 1) && score <= opt(n), "n={n}: scores {score}");
        }
    }
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
    // one below it: found at 1000, or at a board the wrong bound reaches first (the sweep goes up from 1000).
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
    assert!((1000..=1002).any(|n| err.contains(&format!("violation at n={n}"))), "{err}");
}

/// A run of the scheduler on 901..960 with searches of a few milliseconds at first (the climbs too): every board is
/// proven at its optimum, by searches (stopped ones give dual bounds) and by the relations, and a second run on the
/// same directory finds nothing to do and keeps every bound; the report's check passes.
#[test]
fn a_run_proves_a_stretch_at_the_optimum_and_goes_on_from_its_ledger() {
    let dir = TempDir::new("ledger-run");
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.004,
        save_every: 0.5,
        ..Settings::default()
    };
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
    // The maps come first, all of them; the relations carry their bounds once (`settle`); then the searches.
    let actions: Vec<String> = events
        .lines()
        .map(|line| Json::parse(line).unwrap().get("action").and_then(Json::as_str).unwrap().to_owned())
        .collect();
    let first = |action: &str| actions.iter().position(|a| a == action);
    let last_init = actions.iter().rposition(|a| a == "init");
    assert!(last_init.is_some() && last_init < first("settle") && first("settle") < first("search"), "{actions:?}");
    assert_eq!(actions.iter().filter(|a| *a == "settle").count(), 1);
    assert_eq!(actions.last().map(String::as_str), Some("end"));
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

/// Writes a results file of the optimal games of `ns` to `path`, to give a run as proven.
fn write_results(path: &Path, ns: &[u32]) {
    let line = |n: u32| {
        let moves: Vec<String> = opt_game(n).iter().map(u32::to_string).collect();
        format!("{{\"n\":{n},\"score\":{},\"moves\":[{}],\"completed\":true}}\n", opt(n), moves.join(","))
    };
    std::fs::write(path, ns.iter().map(|&n| line(n)).collect::<String>()).unwrap();
}

/// A run given proven games (a results file) takes them as fixed points, and the boards past them follow.
#[test]
fn a_run_takes_the_proven_games_it_is_given() {
    let dir = TempDir::new("ledger-given");
    let results = dir.join("given.jsonl");
    write_results(&results, &[700, 705]);
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

/// A scheduler of the boards 901..960, given the proven games of `proven`, with every board mapped and bounds that a
/// map might leave (the optimum less 10 and more 400), except board 930, the cheapest board of the ordinary order (a
/// gap of 1).  The tests set the rest of what they need by hand.
fn climbing(dir: &Path, proven: &[u32], settings: &Settings) -> Scheduler {
    let results = dir.join("given.jsonl");
    write_results(&results, proven);
    let (mut scheduler, _) = Scheduler::open(901, 960, dir, settings, Some(&results)).unwrap();
    for n in 901..=960 {
        let (lower, upper) = if n == 930 { (opt(n), opt(n) + 1) } else { (opt(n) - 10, opt(n) + 400) };
        let lower = Lower { value: lower, moves: opt_game(n), promotions: None, rule: Rule::Start, from: None };
        scheduler.book.offer_lower(n, lower, false).unwrap();
        scheduler.book.offer_upper(n, Upper { value: upper, rule: Rule::Root, from: None }, false).unwrap();
        scheduler.book.entries.get_mut(&n).unwrap().mapped = true;
    }
    scheduler
}

fn quiet() -> Settings {
    Settings { workers: 4, cpu_budget: 1e9, ..Settings::default() }
}

/// Puts `job` among the searches on their way, as `launch` does (without the search).
fn start(scheduler: &mut Scheduler, job: &Job) {
    scheduler.running.insert(job.n, Running { kind: job.kind, control: Control::new() });
}

/// A search that ended: the optimal game of `n`, proven by the search if `proven`; else a search that stopped, with a
/// game scoring 1 less than the optimum (the moves are the optimal game's: nothing here replays them) and a dual bound
/// 1 above it.
fn ended(n: u32, kind: Kind, proven: bool) -> Done {
    let attempt = Attempt {
        completed: proven,
        reached_cap: false,
        value: if proven { opt(n) } else { opt(n) - 1 },
        moves: opt_game(n),
        promotions: Vec::new(),
        upper: (!proven).then(|| opt(n) + 1),
        nodes: 1000,
        seconds: 3.0,
    };
    Done { n, kind, limit: 60.0, result: Ok(attempt), sent: Instant::now() }
}

/// With the games of 900 and 906 given, the relations prove the boards of their repeat classes (900-902, 904-908, 907
/// a prime among them), so that the boards to climb to are 903 and 909, the first unproven, searched boards above
/// them, not 901 or 908: each is searched ahead of the ordinary order (here board 940, boosted, and board 930, of a gap
/// of 1), seeded with the proven game below it and capped by the ceiling the relations derive.  The board above a
/// board climbed to waits (910 would be the cheapest of the ordinary order), and when a climb proves its board the
/// board above that is climbed to next.
#[test]
fn a_proven_board_is_climbed_to_with_its_game_and_the_board_above_waits() {
    let dir = TempDir::new("ledger-climb-order");
    let mut scheduler = climbing(&dir, &[900, 906], &quiet());
    for n in 900..=908 {
        assert_eq!(scheduler.book.entries[&n].proven(), n != 903, "n={n}");
    }
    let lower = Lower { value: opt(910), moves: opt_game(910), promotions: None, rule: Rule::Start, from: None };
    scheduler.book.offer_lower(910, lower, false).unwrap();
    scheduler.book.offer_upper(910, Upper { value: opt(910) + 1, rule: Rule::Root, from: None }, false).unwrap();
    scheduler.book.entries.get_mut(&940).unwrap().boosted = true;

    let mut climbed = Vec::new();
    for _ in 0..2 {
        let job = scheduler.next_job(0.0).unwrap();
        assert_eq!(job.kind, Kind::Climb, "n={}", job.n);
        assert_eq!(job.limit, Duration::from_secs(60));
        let (below, entry) = (&scheduler.book.entries[&(job.n - 1)], &scheduler.book.entries[&job.n]);
        assert!(below.proven() && !entry.proven());
        assert!(job.seeds.contains(&below.lower.as_ref().unwrap().moves), "n={}: seeded with the game below", job.n);
        assert_eq!(job.cap, entry.upper.map(|u| u.value), "n={}: capped by the ledger's bound", job.n);
        assert!(job.cap.unwrap() <= opt(job.n - 1) + u64::from(job.n), "n={}: at most the ceiling", job.n);
        start(&mut scheduler, &job);
        climbed.push(job.n);
    }
    climbed.sort_unstable();
    assert_eq!(climbed, [903, 909], "the climbs of the two wedges run side by side");

    // With the climbs on their way the ordinary order goes on: the boosted board first, and then the cheapest, 930, not
    // 910, whose board below is climbed to (it waits).
    let ordinary = scheduler.next_job(0.0).unwrap();
    assert_eq!((ordinary.kind, ordinary.n), (Kind::Search, 940));
    scheduler.book.entries.get_mut(&940).unwrap().boosted = false;
    let ordinary = scheduler.next_job(0.0).unwrap();
    assert_eq!((ordinary.kind, ordinary.n), (Kind::Search, 930));

    // The climb to 909 proves it: 910 is climbed to next, with the game of 909.
    scheduler.finish(ended(909, Kind::Climb, true)).unwrap();
    assert!(scheduler.book.entries[&909].proven());
    let next = scheduler.next_job(0.0).unwrap();
    assert_eq!((next.kind, next.n), (Kind::Climb, 910));
    assert!(next.seeds.contains(&scheduler.book.entries[&909].lower.as_ref().unwrap().moves));
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    let climb = events.lines().find(|line| line.contains("\"job\":\"climb\"")).expect("an event of the climb");
    assert!(climb.contains("\"action\":\"search\"") && climb.contains("\"outcome\":\"proved\""), "{climb}");
}

/// A climb that stops at its limit parks its board: the board is not climbed to again, goes by the ordinary order
/// with the level it had, and the ledger remembers it.  With the climbs off there are none.
#[test]
fn a_climb_that_stops_parks_its_board_in_the_ordinary_queue() {
    let dir = TempDir::new("ledger-climb-park");
    let mut scheduler = climbing(&dir, &[906], &quiet());
    let job = scheduler.next_job(0.0).unwrap();
    assert_eq!((job.kind, job.n), (Kind::Climb, 909));
    start(&mut scheduler, &job);
    scheduler.finish(ended(909, Kind::Climb, false)).unwrap();

    let entry = &scheduler.book.entries[&909];
    assert!(!entry.proven() && entry.parked, "{entry:?}");
    assert_eq!((entry.attempts, entry.level), (1, 0), "a climb is a search of the board, not a step of its doubling");
    let kept = Entry::from_json(&entry.to_json(None)).unwrap();
    assert!(kept.parked, "the ledger keeps it");
    // Its gap of 1 ties with the cheapest board's and it is the lower: it is the next ordinary search, with the base
    // limit.
    let next = scheduler.next_job(0.0).unwrap();
    assert_eq!((next.kind, next.n), (Kind::Search, 909));
    assert_eq!(next.limit, Duration::from_secs_f64(Settings::default().base));
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    assert!(events.contains("\"job\":\"climb\"") && events.contains("\"outcome\":\"parked\""), "{events}");

    let off = Settings { climb_limit: 0.0, ..quiet() };
    let dir = TempDir::new("ledger-climb-off");
    let scheduler = climbing(&dir, &[906], &off);
    let job = scheduler.next_job(0.0).unwrap();
    assert_eq!(job.kind, Kind::Search, "n={}", job.n);
}

/// A run given the proven game of 900 climbs from it, with climbs of a few milliseconds some of which stop: every
/// board is proven at its optimum, each board is climbed to at most once, a climb that ended unproven left its board
/// parked in the ledger, and the report counts the climbs.  With `climb_limit` 0 no climb is made.
#[test]
fn a_run_climbs_and_parks_and_turns_the_climbs_off() {
    let dir = TempDir::new("ledger-climb-run");
    let results = dir.join("given.jsonl");
    write_results(&results, &[900]);
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.01,
        save_every: 0.5,
        ..Settings::default()
    };
    assert_eq!(schedule::run(901, 960, &dir, &settings, Some(&results)).unwrap(), "all proven");
    let entries = read_ledger(&dir).unwrap();
    for e in &entries {
        assert!(e.proven() && e.upper.unwrap().value == opt(e.n), "n={}: {e:?}", e.n);
    }
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    let events: Vec<Json> = events.lines().map(|line| Json::parse(line).unwrap()).collect();
    let text = |e: &Json, field: &str| e.get(field).and_then(Json::as_str).map(str::to_owned);
    let climbs: Vec<&Json> = events.iter().filter(|e| text(e, "job").as_deref() == Some("climb")).collect();
    // 901 and 902 are proven by the relations with 900 (one repeat class): the first board to climb to is 903.
    assert!(climbs.iter().any(|e| e.get("n").and_then(Json::as_u32) == Some(903)), "903 is climbed to");
    let mut boards: Vec<u32> = climbs.iter().map(|e| e.get("n").and_then(Json::as_u32).unwrap()).collect();
    boards.sort_unstable();
    boards.dedup();
    assert_eq!(boards.len(), climbs.len(), "a board is climbed to once");
    for e in &climbs {
        let n = e.get("n").and_then(Json::as_u32).unwrap();
        let outcome = text(e, "outcome").expect("a climb has its outcome");
        assert!(["proved", "parked", "moot"].contains(&outcome.as_str()), "{outcome}");
        assert_eq!(entries.iter().find(|x| x.n == n).unwrap().parked, outcome == "parked", "n={n}: {outcome}");
    }
    assert!(events.iter().any(|e| text(e, "job").as_deref() == Some("ordinary")), "ordinary searches too");
    let (report, ok) = report::report(&dir, &[]).unwrap();
    assert!(ok && report.contains(&format!("climbs: {} among the searches", climbs.len())), "{report}");

    let dir = TempDir::new("ledger-climb-run-off");
    let settings = Settings { climb_limit: 0.0, ..settings };
    assert_eq!(schedule::run(901, 960, &dir, &settings, Some(&results)).unwrap(), "all proven");
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    assert!(events.contains("\"job\":\"ordinary\"") && !events.contains("\"job\":\"climb\""), "climbs are off");
}
