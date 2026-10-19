//! The ledger's tests: the relations on the boards of optimal.json (every bound they derive brackets the optimum, and
//! every game they make is legal), a violation, the choice of the next search (the climbs), the finder, and runs of the
//! scheduler checked against optimal.json.  The runs of the first tests leave the finder off (`lns_seconds` 0), whose
//! default second a board would make them slow; the finder has tests of its own at the end.

use super::book::{Book, Entry, Lower, Rule, Upper};
use super::report;
use super::schedule::{
    self, read_ledger, Attempt, Done, Job, Kind, LnsPlan, LnsRun, Running, Scheduler, Seed, SeedFrom, Settings, Way,
};
use super::support::{TempDir, QUICK};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::{largest_prime_at_most, replay_finished, Optimal};
use taxman::json::Json;
use taxman::store::{ceiling_game, monotone_game, prime_down, prime_up};

pub(crate) fn opt(n: u32) -> u64 {
    Optimal::built_in().score(n).expect("a board of optimal.json")
}

pub(crate) fn opt_game(n: u32) -> Vec<u32> {
    Optimal::built_in().moves(n).expect("a board of optimal.json").to_vec()
}

/// The last board of the stretch 901.. that the runs of the tests prove: 960, and 912 in a quick run (`QUICK`), a
/// fifth of the work with the same climbs (up to 909 and down to 903, either side of the class 904-908).
pub(crate) const LAST: u32 = if QUICK { 912 } else { 960 };

/// A book of the boards `ns`, with no bounds but those of the trivial boards.
pub(crate) fn book_of(ns: impl Iterator<Item = u32>) -> Book {
    let ns: Vec<u32> = ns.collect();
    Book::new(schedule::all_facts(&ns, 8).expect("the boards' facts"))
}

/// Gives the book the optimal game of `n` as proven.
pub(crate) fn give(book: &mut Book, n: u32) {
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
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_relations_bracket_the_optimum_of_every_board_to_520"
)]
fn the_relations_bracket_the_optimum_of_every_board_to_1300() {
    the_relations_bracket_the_optimum_of_every_board_to(1300, 2000);
}

/// [`the_relations_bracket_the_optimum_of_every_board_to_1300`] for the boards to 520, which pass the width of 512.
#[test]
fn the_relations_bracket_the_optimum_of_every_board_to_520() {
    the_relations_bracket_the_optimum_of_every_board_to(520, 750);
}

/// The test of the boards 2..=`top`, which the relations must reach with more than `min_changes` changes.
fn the_relations_bracket_the_optimum_of_every_board_to(top: u32, min_changes: usize) {
    let mut book = book_of(2..=top);
    let given: Vec<u32> = (10..=top).step_by(10).collect();
    for &n in &given {
        give(&mut book, n);
    }
    let changes = book.propagate(given.iter().copied()).unwrap();
    assert!(changes.len() > min_changes, "{} changes", changes.len());
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
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is loose_bounds_and_the_relations_bracket_the_optimum_of_1001_to_1100"
)]
fn loose_bounds_and_the_relations_bracket_the_optimum() {
    loose_bounds_and_the_relations_bracket_the_optimum_of(1001..=1300, (100, 30));
}

/// [`loose_bounds_and_the_relations_bracket_the_optimum`] for the boards 1001..=1100.
#[test]
fn loose_bounds_and_the_relations_bracket_the_optimum_of_1001_to_1100() {
    loose_bounds_and_the_relations_bracket_the_optimum_of(1001..=1100, (30, 10));
}

/// The test of the boards `ns`, where the relations must derive more than `(lower, upper)` bounds of the two sides.
fn loose_bounds_and_the_relations_bracket_the_optimum_of(
    ns: std::ops::RangeInclusive<u32>,
    (lower, upper): (usize, usize),
) {
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
    assert!(derived("lower") > lower && derived("upper") > upper, "{rules:?}");
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
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_run_is_exported_and_both_referees_accept_the_file")]
fn a_run_proves_a_stretch_at_the_optimum_and_goes_on_from_its_ledger() {
    let dir = TempDir::new("ledger-run");
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.004,
        lns_seconds: 0.0,
        save_every: 0.5,
        ..Settings::default()
    };
    let reason = schedule::run(901, LAST, &dir, &settings, None).unwrap();
    assert_eq!(reason, "all proven");
    let entries = read_ledger(&dir).unwrap();
    assert_eq!(entries.len(), (LAST - 900) as usize);
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

    let again = schedule::run(901, LAST, &dir, &settings, None).unwrap();
    assert_eq!(again, "all proven");
    let kept = read_ledger(&dir).unwrap();
    for (a, b) in entries.iter().zip(&kept) {
        assert_eq!((a.n, &a.lower, a.upper, a.attempts), (b.n, &b.lower, b.upper, b.attempts));
    }
    let (text, ok) = report::report(&dir, &[]).unwrap();
    assert!(ok, "{text}");
    assert!(text.contains(&format!("{0} in the ledger, {0} proven", LAST - 900)), "{text}");
}

/// Writes a results file of the optimal games of `ns` to `path`, to give a run as proven.
pub(crate) fn write_results(path: &Path, ns: &[u32]) {
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
    let settings = Settings { workers: 2, cpu_budget: 1e9, base: 0.01, lns_seconds: 0.0, ..Settings::default() };
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
pub(crate) fn climbing(dir: &Path, proven: &[u32], settings: &Settings) -> Scheduler {
    let results = dir.join("given.jsonl");
    write_results(&results, proven);
    let (mut scheduler, _) = Scheduler::open(901, 960, dir, settings, Some(&results)).unwrap();
    for n in 901..=960 {
        let (lower, upper) = if n == 930 { (opt(n), opt(n) + 1) } else { (opt(n) - 10, opt(n) + 400) };
        let lower = Lower { value: lower, moves: opt_game(n), promotions: None, rule: Rule::Start, from: None };
        scheduler.book.offer_lower(n, lower, false).unwrap();
        scheduler.book.offer_upper(n, Upper { value: upper, rule: Rule::Root, from: None }, false).unwrap();
        let entry = scheduler.book.entries.get_mut(&n).unwrap();
        (entry.mapped, entry.root) = (true, Some(upper + 1000));
    }
    scheduler
}

/// Settings for the tests of the order of the jobs: no floor steps, so that an ordinary board is searched as it was.
pub(crate) fn quiet() -> Settings {
    Settings { workers: 4, cpu_budget: 1e9, floor_step: 0, ..Settings::default() }
}

/// The same with floor steps of 150 points.
pub(crate) fn stepping() -> Settings {
    Settings { floor_step: 150, ..quiet() }
}

/// Puts `job` among the searches on their way, as `launch` does (without the search).
pub(crate) fn start(scheduler: &mut Scheduler, job: &Job) {
    scheduler.running.insert(job.n, Running { kind: job.kind, control: Control::new() });
}

/// A search that ended: the optimal game of `n`, proven by the search if `proven`; else a search that stopped, with a
/// game scoring 1 less than the optimum (the moves are the optimal game's: nothing here replays them) and a dual bound
/// at the optimum, so that the gap is 1.
pub(crate) fn ended(n: u32, kind: Kind, proven: bool) -> Done {
    let attempt = Attempt {
        completed: proven,
        reached_cap: false,
        held: false,
        value: if proven { opt(n) } else { opt(n) - 1 },
        moves: opt_game(n),
        promotions: Vec::new(),
        upper: (!proven).then(|| opt(n)),
        nodes: 1000,
        seconds: 3.0,
        lns: None,
        seed_from: None,
    };
    Done {
        n,
        kind,
        limit: 60.0,
        below_proven: false,
        above_proven: false,
        floor: None,
        result: Ok(attempt),
        sent: Instant::now(),
    }
}

/// Whether `job` is seeded with the game `moves`.
pub(crate) fn seeded_with(job: &Job, moves: &[u32]) -> bool {
    job.seeds.iter().any(|seed| seed.moves == moves)
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
        assert_eq!(job.kind, Kind::Climb(Way::Up), "n={}", job.n);
        assert_eq!(job.limit, Duration::from_secs(60));
        let (below, entry) = (&scheduler.book.entries[&(job.n - 1)], &scheduler.book.entries[&job.n]);
        assert!(below.proven() && !entry.proven());
        assert!(seeded_with(&job, &below.lower.as_ref().unwrap().moves), "n={}: seeded with the game below", job.n);
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
    scheduler.finish(ended(909, Kind::Climb(Way::Up), true)).unwrap();
    assert!(scheduler.book.entries[&909].proven());
    let next = scheduler.next_job(0.0).unwrap();
    assert_eq!((next.kind, next.n), (Kind::Climb(Way::Up), 910));
    assert!(seeded_with(&next, &scheduler.book.entries[&909].lower.as_ref().unwrap().moves));
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    let climb = events.lines().find(|line| line.contains("\"job\":\"climb\"")).expect("an event of the climb");
    assert!(climb.contains("\"action\":\"search\"") && climb.contains("\"outcome\":\"proved\""), "{climb}");
}

/// A climb that stops at its limit parks its board: the board is not climbed to again, goes by the ordinary order
/// with the level it had, and the ledger remembers it.  With the climbs off there are none.
#[test]
fn a_climb_that_stops_parks_its_board_in_the_ordinary_queue() {
    let dir = TempDir::new("ledger-climb-park");
    let mut scheduler = climbing(&dir, &[906], &Settings { climb_down: false, ..quiet() });
    let job = scheduler.next_job(0.0).unwrap();
    assert_eq!((job.kind, job.n), (Kind::Climb(Way::Up), 909));
    start(&mut scheduler, &job);
    scheduler.finish(ended(909, Kind::Climb(Way::Up), false)).unwrap();

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

    for off in [Settings { climb_limit: 0.0, ..quiet() }, Settings { climb_up: false, climb_down: false, ..quiet() }] {
        let dir = TempDir::new("ledger-climb-off");
        let job = climbing(&dir, &[906], &off).next_job(0.0).unwrap();
        assert_eq!(job.kind, Kind::Search, "n={}", job.n);
    }
}

/// With the game of 906 given, 909 is climbed to up and 903 down (904-908 are proven; 902 is not searched, 901 being
/// the board of its class): the climb up goes first, the climb down is seeded with the proven game above, and a
/// climb down that stops parks its board like one up, with the way in its event.  The ways are settable: the climb down
/// alone goes first, the climb up alone has no climb down after it, and with neither there are none.  A board next to
/// one that is climbed to, either way, has no ordinary search meanwhile.
#[test]
fn a_proven_board_is_climbed_down_from_after_the_climbs_up() {
    let dir = TempDir::new("ledger-climb-down");
    let mut scheduler = climbing(&dir, &[906], &quiet());
    let up = scheduler.next_job(0.0).unwrap();
    assert_eq!((up.kind, up.n), (Kind::Climb(Way::Up), 909), "the climbs up have priority");
    start(&mut scheduler, &up);
    let down = scheduler.next_job(0.0).unwrap();
    assert_eq!((down.kind, down.n, down.limit), (Kind::Climb(Way::Down), 903, Duration::from_secs(60)));
    let above = &scheduler.book.entries[&904];
    assert!(above.proven() && seeded_with(&down, &above.lower.as_ref().unwrap().moves), "seeded with the game above");
    assert_eq!(down.cap, scheduler.book.entries[&903].upper.map(|u| u.value), "capped by the ledger's bound");
    start(&mut scheduler, &down);
    scheduler.finish(ended(903, Kind::Climb(Way::Down), false)).unwrap();

    let entry = &scheduler.book.entries[&903];
    assert!(!entry.proven() && entry.parked && (entry.attempts, entry.level) == (1, 0), "{entry:?}");
    let next = scheduler.next_job(0.0).unwrap();
    assert_eq!((next.kind, next.n), (Kind::Search, 903), "the parked board is the cheapest ordinary search");
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    let climb = events.lines().find(|line| line.contains("\"job\":\"climb\"")).expect("an event of the climb");
    assert!(climb.contains("\"way\":\"down\"") && climb.contains("\"outcome\":\"parked\""), "{climb}");

    // The ways alone: the climb down first when it is the only way, no climb down after the climb up when only up.
    let only = |climb_up, climb_down| Settings { climb_up, climb_down, ..quiet() };
    let dir = TempDir::new("ledger-climb-down-only");
    let first = climbing(&dir, &[906], &only(false, true)).next_job(0.0).unwrap();
    assert_eq!((first.kind, first.n), (Kind::Climb(Way::Down), 903));
    let dir = TempDir::new("ledger-climb-up-only");
    let mut scheduler = climbing(&dir, &[906], &only(true, false));
    let first = scheduler.next_job(0.0).unwrap();
    assert_eq!((first.kind, first.n), (Kind::Climb(Way::Up), 909));
    start(&mut scheduler, &first);
    assert_eq!(scheduler.next_job(0.0).unwrap().kind, Kind::Search, "no climb down");
}

/// The board below one that is being climbed to down has no ordinary search meanwhile, as the board above one climbed
/// to up has none: 924 and 925 are both searched boards (the smallest of their repeat classes), 924 one of the
/// cheapest of the ordinary order.
#[test]
fn a_board_next_to_a_climb_down_waits_as_one_next_to_a_climb_up_does() {
    let dir = TempDir::new("ledger-climb-hold-down");
    let mut scheduler = climbing(&dir, &[], &Settings { climb_limit: 0.0, ..quiet() });
    let lower = Lower { value: opt(924), moves: opt_game(924), promotions: None, rule: Rule::Start, from: None };
    scheduler.book.offer_lower(924, lower, false).unwrap();
    scheduler.book.offer_upper(924, Upper { value: opt(924) + 1, rule: Rule::Root, from: None }, false).unwrap();
    assert_eq!(scheduler.next_job(0.0).unwrap().n, 924, "the cheapest board, tied with 930, the lower");
    let running = Running { kind: Kind::Climb(Way::Down), control: Control::new() };
    scheduler.running.insert(925, running);
    assert_eq!(scheduler.next_job(0.0).unwrap().n, 930, "924 waits for the climb down to 925");
    scheduler.running.remove(&925);
    assert_eq!(scheduler.next_job(0.0).unwrap().n, 924);
}

/// A run given the proven game of 906 climbs from it both ways, with climbs of a few milliseconds some of which stop:
/// every board is proven at its optimum, each board is climbed to at most once, a climb that ended unproven left its
/// board parked in the ledger, and the report counts the climbs of each way.  With one way only or with `climb_limit`
/// 0, only the climbs that are asked for are made.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_run_is_exported_and_both_referees_accept_the_file")]
fn a_run_climbs_and_parks_and_turns_the_climbs_off() {
    let dir = TempDir::new("ledger-climb-run");
    let results = dir.join("given.jsonl");
    write_results(&results, &[906]);
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.01,
        lns_seconds: 0.0,
        save_every: 0.5,
        ..Settings::default()
    };
    assert_eq!(schedule::run(901, LAST, &dir, &settings, Some(&results)).unwrap(), "all proven");
    let entries = read_ledger(&dir).unwrap();
    for e in &entries {
        assert!(e.proven() && e.upper.unwrap().value == opt(e.n), "n={}: {e:?}", e.n);
    }
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    let events: Vec<Json> = events.lines().map(|line| Json::parse(line).unwrap()).collect();
    let text = |e: &Json, field: &str| e.get(field).and_then(Json::as_str).map(str::to_owned);
    let number = |e: &Json| e.get("n").and_then(Json::as_u32).unwrap();
    let climbs: Vec<&Json> = events.iter().filter(|e| text(e, "job").as_deref() == Some("climb")).collect();
    let by_way = |way: &str| climbs.iter().filter(|e| text(e, "way").as_deref() == Some(way)).count();
    // 904-908 are proven by the relations with 906 (one repeat class): the climb down is to 903.
    let climbed = |n: u32, way: &str| climbs.iter().any(|e| number(e) == n && text(e, "way").as_deref() == Some(way));
    let summary: Vec<String> = climbs
        .iter()
        .map(|e| format!("{} {} {}", number(e), text(e, "way").unwrap(), text(e, "outcome").unwrap()))
        .collect();
    assert!(climbed(903, "down") && by_way("up") > 0, "{summary:?}");
    let mut boards: Vec<u32> = climbs.iter().map(|e| number(e)).collect();
    boards.sort_unstable();
    boards.dedup();
    assert_eq!(boards.len(), climbs.len(), "a board is climbed to once");
    for e in &climbs {
        let outcome = text(e, "outcome").expect("a climb has its outcome");
        assert!(["proved", "parked", "moot"].contains(&outcome.as_str()), "{outcome}");
        let parked = entries.iter().find(|x| x.n == number(e)).unwrap().parked;
        assert_eq!(parked, outcome == "parked", "n={}: {outcome}", number(e));
    }
    assert!(events.iter().any(|e| text(e, "job").as_deref() == Some("ordinary")), "ordinary searches too");
    let (report, ok) = report::report(&dir, &[]).unwrap();
    assert!(ok, "{report}");
    for way in ["up", "down"] {
        assert!(report.contains(&format!("climbs {way}: {} among the searches", by_way(way))), "{report}");
    }

    // One way, or none: only the climbs asked for are made (a quick run takes 903..910, the boards that have them).
    let (first, last) = if QUICK { (903, 910) } else { (901, LAST) };
    let climbs_of = |settings: &Settings| {
        let dir = TempDir::new("ledger-climb-run-ways");
        assert_eq!(schedule::run(first, last, &dir, settings, Some(&results)).unwrap(), "all proven");
        let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
        (events.contains("\"way\":\"up\""), events.contains("\"way\":\"down\""), events.contains("\"job\":\"climb\""))
    };
    assert_eq!(climbs_of(&Settings { climb_down: false, ..settings.clone() }), (true, false, true));
    assert_eq!(climbs_of(&Settings { climb_up: false, ..settings.clone() }), (false, true, true));
    // (A quick run leaves out the runs with the climbs off: the jobs with them off are another test's.)
    if !QUICK {
        assert!(!climbs_of(&Settings { climb_up: false, climb_down: false, ..settings.clone() }).2);
        assert!(!climbs_of(&Settings { climb_limit: 0.0, ..settings }).2, "climbs are off");
    }
}

// ---------------------------------------------------------------------------------------------------------
// The finder

/// A job on board `n` for a worker: a map is a search of no time, any other a search of 20 ms.
pub(crate) fn job_of(n: u32, kind: Kind, seeds: Vec<Seed>, cap: Option<u64>, lns: LnsPlan) -> Job {
    let limit = if kind == Kind::Init { Duration::ZERO } else { Duration::from_millis(20) };
    Job { n, kind, limit, seeds, cap, below_proven: false, above_proven: false, lns, floor: None }
}

pub(crate) fn finder(seconds: f64, last: Option<u64>) -> LnsPlan {
    LnsPlan { seconds, last, round: 0 }
}

/// What a worker makes of `job`, with its checkpoint in `dir`.
pub(crate) fn attempted(dir: &Path, job: &Job) -> Attempt {
    std::fs::create_dir_all(dir.join("ckpt")).unwrap();
    schedule::attempt(job, &Control::new(), dir, Duration::from_secs(60)).unwrap()
}

/// The finder follows a board's map, from the start incumbent, and precedes a search whose best seed is another than
/// the one it last ran from (a climb's always counts as another), from that seed; it does not run for an ordinary search
/// that goes on from the same seed, nor when the seed reaches the ceiling, nor with no seconds.  Its game seeds the
/// search when it beats the seed, and every game it returns plays at the score it says, by the referee.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_run_is_exported_and_both_referees_accept_the_file")]
fn the_finder_follows_a_map_and_precedes_a_search_whose_seed_it_has_not_seen() {
    let (dir, n) = (TempDir::new("ledger-finder"), 1218);
    // The seconds of each run of the finder, which has most of its gain in the first fraction of a second.
    let seconds = if QUICK { 0.2 } else { 0.5 };
    let referee = |run: &LnsRun| assert_eq!(replay_finished(n, &run.moves), Ok(run.found), "the finder's game");
    // 1218's start incumbent is 204 short of the optimum, which the finder reaches in a fraction of a second.
    let map = attempted(&dir, &job_of(n, Kind::Init, Vec::new(), None, finder(seconds, None)));
    let run = map.lns.as_ref().expect("the finder follows the map");
    assert_eq!(run.from, map.value, "it begins from the start incumbent");
    assert!(run.improved() && run.found <= opt(n), "from {} to {}, the optimum {}", run.from, run.found, opt(n));
    assert!(run.seconds >= seconds && map.seed_from.is_none());
    referee(run);

    // A search whose best seed is the start game, not the finder's game that the board has since.
    let seeded = |moves: &[u32]| vec![Seed { from: n, moves: moves.to_vec() }];
    let before = job_of(n, Kind::Search, seeded(&map.moves), None, finder(seconds, Some(run.found)));
    let first = attempted(&dir, &before);
    let again = first.lns.as_ref().expect("a seed it has not seen: it runs");
    assert_eq!((again.from, first.seed_from), (map.value, Some(SeedFrom::Lns)));
    assert!(again.improved() && first.value >= again.found);
    referee(again);

    // A search of the board with the game the finder left it with has nothing new.
    let same =
        attempted(&dir, &job_of(n, Kind::Search, seeded(&again.moves), None, finder(seconds, Some(again.found))));
    assert!(same.lns.is_none() && same.seed_from == Some(SeedFrom::Own));
    // A climb runs the finder all the same.
    let climb = job_of(n, Kind::Climb(Way::Up), seeded(&again.moves), None, finder(0.1, Some(again.found)));
    assert!(attempted(&dir, &climb).lns.is_some());
    // A seed that reaches the ceiling is settled by the search at once (in a directory of its own: the checkpoint of
    // the searches above may hold a game that beats the finder's).
    let capped = job_of(n, Kind::Search, seeded(&again.moves), Some(again.found), finder(seconds, None));
    let capped = attempted(&TempDir::new("ledger-finder-capped"), &capped);
    assert!(capped.lns.is_none() && capped.reached_cap && capped.value == again.found);
    // No seconds, no finder.
    assert!(attempted(&TempDir::new("ledger-no-finder"), &job_of(n, Kind::Init, Vec::new(), None, finder(0.0, None)))
        .lns
        .is_none());
}

/// The referee accepts the promotions of a game that plays at the bag and their sum, and refuses a set that does not
/// play as the game it claims.
#[test]
fn the_referee_refuses_promotions_that_do_not_play_as_the_game_they_claim() {
    let n = 250;
    let map = attempted(&TempDir::new("ledger-referee"), &job_of(n, Kind::Init, Vec::new(), None, finder(0.0, None)));
    taxman::with_width!(n, W => {
        let board = Board::<W>::for_n(n);
        let set = taxman::bits::Bits::<W>::from_slice(&map.promotions);
        assert_eq!(schedule::refereed(&board, &set), Ok((map.value, map.moves.clone())));
        // Not every candidate can be promoted at once: what plays is not a game of all their sum.
        let all = board.candidates();
        assert!(all.sum() > set.sum());
        assert!(schedule::refereed(&board, &all).is_err(), "all the candidates are not a game");
    });
}

/// A search that ended after the finder, for `finish`: it stopped with a game of `value`, after the finder's run (if
/// any), seeded by `seed_from`, with the board below proven.
fn ended_after_finder(n: u32, value: u64, lns: Option<LnsRun>, seed_from: Option<SeedFrom>) -> Done {
    let mut done = ended(n, Kind::Search, false);
    let attempt = done.result.as_mut().unwrap();
    (attempt.value, attempt.lns, attempt.seed_from) = (value, lns, seed_from);
    done.below_proven = true;
    done
}

/// The finder's game is a board's lower bound where it beats the board's and the search's (and credited to it when the
/// search, which began from it, found nothing better); the search's where that is better, and nobody's but the
/// search's where the finder did not improve on its seed.  The board keeps the score the finder left it with, and the
/// event says what the finder and the seed were.
#[test]
fn the_finders_game_is_the_lower_bound_where_it_beats_the_search_and_the_event_says_what_it_did() {
    // A board alone in its ledger (so that no relation reaches it from a fabricated neighbour), its bounds 10 under
    // the optimum and 400 over, that a search ended on after a finder whose run began 10 under the optimum and found
    // `found` under it (the same if it found nothing better): the board after, and the event.
    let finished = |n: u32, value: u64, found: Option<u64>, seed_from: Option<SeedFrom>| {
        let dir = TempDir::new("ledger-finder-finish");
        let (mut scheduler, _) = Scheduler::open(n, n, &dir, &quiet(), None).unwrap();
        let lower = Lower { value: opt(n) - 10, moves: opt_game(n), promotions: None, rule: Rule::Start, from: None };
        scheduler.book.offer_lower(n, lower, false).unwrap();
        scheduler.book.offer_upper(n, Upper { value: opt(n) + 400, rule: Rule::Root, from: None }, false).unwrap();
        scheduler.book.entries.get_mut(&n).unwrap().mapped = true;
        let run = found.map(|found| LnsRun {
            from: opt(n) - 10,
            found: opt(n) - found,
            seconds: 0.5,
            moves: opt_game(n),
            promotions: Vec::new(),
        });
        scheduler.finish(ended_after_finder(n, opt(n) - value, run, seed_from)).unwrap();
        let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
        let event = events.lines().map(|line| Json::parse(line).unwrap()).find(|e| e.get("n").is_some()).unwrap();
        (scheduler.book.entries[&n].clone(), event)
    };
    let lower = |entry: &Entry, n: u32| entry.lower.as_ref().map(|l| (l.value, l.rule)).map(|(v, r)| (opt(n) - v, r));
    let lns = Some(SeedFrom::Lns);

    // The finder improved on its seed (4 under the optimum), and the search that began from its game found nothing
    // better: the lower bound is the finder's.
    let (entry, event) = finished(940, 4, Some(4), lns);
    assert_eq!(lower(&entry, 940), Some((4, Rule::Lns)));
    assert_eq!(entry.lns_score, Some(opt(940) - 4), "the score the finder left the board with");
    assert_eq!(entry.search_s, 3.5, "the search's seconds and the finder's");
    assert_eq!(Entry::from_json(&entry.to_json(None)).unwrap().lns_score, entry.lns_score, "the ledger keeps it");
    assert_eq!(event.get("lns_s").and_then(Json::as_f64), Some(0.5));
    assert_eq!(event.get("lns_from").and_then(Json::as_u64), Some(opt(940) - 10));
    assert_eq!(event.get("lns_found").and_then(Json::as_u64), Some(opt(940) - 4));
    assert_eq!(event.get("seed_from").and_then(Json::as_str), Some("lns"));
    assert_eq!(event.get("below_proven").and_then(Json::as_bool), Some(true));
    assert_eq!(event.get("above_proven").and_then(Json::as_bool), Some(false));

    // The search found more: the search's game.
    let (entry, _) = finished(944, 2, Some(4), lns);
    assert_eq!((lower(&entry, 944), entry.lns_score), (Some((2, Rule::Search)), Some(opt(944) - 4)));
    // The finder found nothing better than its seed: the search's game, and the board has the seed's score for the
    // finder; the seed was the board's own.
    let (entry, event) = finished(948, 8, Some(10), Some(SeedFrom::Own));
    assert_eq!((lower(&entry, 948), entry.lns_score), (Some((8, Rule::Search)), Some(opt(948) - 10)));
    assert_eq!(event.get("seed_from").and_then(Json::as_str), Some("n"));
    // No finder: nothing of it on the board or in the event.
    let (entry, event) = finished(950, 8, None, Some(SeedFrom::Below));
    assert_eq!((lower(&entry, 950), entry.lns_score), (Some((8, Rule::Search)), None));
    assert_eq!(event.get("seed_from").and_then(Json::as_str), Some("n-1"));
    assert_eq!(event.get("lns_s").and_then(Json::as_f64), Some(0.0));
    assert!(event.get("lns_from").unwrap().is_null() && event.get("lns_found").unwrap().is_null());
}

/// The events of the jobs of a run (a map or a search), parsed.
pub(crate) fn job_events(dir: &Path) -> Vec<Json> {
    let events = std::fs::read_to_string(schedule::events_path(dir)).unwrap();
    let all = events.lines().map(|line| Json::parse(line).unwrap());
    all.filter(|e| matches!(e.get("action").and_then(Json::as_str), Some("init" | "search"))).collect()
}

/// A run with the finder on: every board is proven at its optimum, every job's event says what the finder and the
/// seed were (the finder's scores between its seed's and the optimum), the games of the finder pass the report's
/// referee, and the report counts the finder's runs.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_run_is_exported_and_both_referees_accept_the_file")]
fn a_run_with_the_finder_proves_the_stretch_and_the_events_and_the_report_say_what_it_did() {
    let dir = TempDir::new("ledger-finder-run");
    let results = dir.join("given.jsonl");
    write_results(&results, &[906]);
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.01,
        lns_seconds: 0.05,
        save_every: 0.5,
        ..Settings::default()
    };
    assert_eq!(schedule::run(901, LAST, &dir, &settings, Some(&results)).unwrap(), "all proven");
    for e in read_ledger(&dir).unwrap() {
        assert!(e.proven() && e.upper.unwrap().value == opt(e.n), "n={}: {e:?}", e.n);
    }
    let jobs = job_events(&dir);
    let mut runs = 0;
    for e in &jobs {
        let n = e.get("n").and_then(Json::as_u32).unwrap();
        let seconds = e.get("lns_s").and_then(Json::as_f64).unwrap_or_else(|| panic!("no lns_s: {e}"));
        let (from, found) = (e.get("lns_from").unwrap(), e.get("lns_found").unwrap());
        if from.is_null() {
            assert!(found.is_null() && seconds == 0.0, "{e}");
        } else {
            runs += 1;
            let (from, found) = (from.as_u64().unwrap(), found.as_u64().unwrap());
            // (A board proven by a relation while the job went on stops the finder early.)
            assert!(seconds > 0.0 && from <= found && found <= opt(n), "n={n}: {e}");
        }
        let seed = e.get("seed_from").unwrap();
        assert!(seed.is_null() || ["n-1", "n", "n+1", "lns"].contains(&seed.as_str().unwrap()), "{e}");
        assert!(e.get("below_proven").and_then(Json::as_bool).is_some() && e.get("above_proven").is_some(), "{e}");
    }
    assert!(runs > (LAST - 900) as usize / 6, "the finder ran {runs} times in {} jobs", jobs.len());
    let start = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    assert!(start.lines().next().unwrap().contains("\"lns_s\":0.05"), "the start event has the setting");
    let (report, ok) = report::report(&dir, &[]).unwrap();
    assert!(ok, "{report}");
    assert!(report.contains(&format!("finder: {runs} runs")), "{report}");
}

/// With no seconds for the finder nothing of it is left in a run (no run, no game of its rule, no score kept, no seed
/// from it), and the choice of the jobs is what it is with the finder on: its setting changes only what a job carries.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_run_is_exported_and_both_referees_accept_the_file")]
fn with_no_seconds_for_the_finder_nothing_of_it_is_left_and_the_order_of_the_jobs_is_the_same() {
    let dir = TempDir::new("ledger-finder-off");
    let results = dir.join("given.jsonl");
    write_results(&results, &[906]);
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.01,
        lns_seconds: 0.0,
        save_every: 0.5,
        ..Settings::default()
    };
    assert_eq!(schedule::run(901, LAST, &dir, &settings, Some(&results)).unwrap(), "all proven");
    for e in read_ledger(&dir).unwrap() {
        assert!(e.lns_score.is_none() && e.lower.as_ref().unwrap().rule != Rule::Lns, "n={}: {e:?}", e.n);
    }
    for e in job_events(&dir) {
        assert_eq!(e.get("lns_s").and_then(Json::as_f64), Some(0.0), "{e}");
        assert!(e.get("lns_from").unwrap().is_null() && e.get("lns_found").unwrap().is_null(), "{e}");
        assert_ne!(e.get("seed_from").and_then(Json::as_str), Some("lns"), "{e}");
    }
    let (report, ok) = report::report(&dir, &[]).unwrap();
    assert!(ok && report.contains("finder: 0 runs"), "{report}");

    // The jobs that a scheduler hands out, with the finder on and off: the same boards, kinds, limits, seeds and
    // ceilings in the same order; only the plan the job carries differs.
    let jobs = |seconds: f64| {
        let dir = TempDir::new("ledger-finder-order");
        let mut scheduler = climbing(&dir, &[906], &Settings { lns_seconds: seconds, ..quiet() });
        let mut out = Vec::new();
        for _ in 0..8 {
            let job = scheduler.next_job(0.0).unwrap();
            assert_eq!(job.lns, LnsPlan { seconds, last: None, round: 0 });
            start(&mut scheduler, &job);
            let seeds: Vec<(u32, Vec<u32>)> = job.seeds.iter().map(|s| (s.from, s.moves.clone())).collect();
            out.push((job.n, job.kind, job.limit, job.cap, seeds, job.below_proven, job.above_proven));
        }
        out
    };
    assert_eq!(jobs(0.0), jobs(1.0));
}
