//! The tests of the floor steps: a step that completes lowers the upper bound by the floor rule, one that finds a set
//! raises the lower bound with a game the referee replays, one that its limit stops is kept and resumed under its
//! floor, the boards go by the forecast of their proofs with the climbs first, a climb into a repeat class holds the
//! boards beside the whole class, and a ledger written before the steps goes on.

use super::book::{Entry, Lower, Rule, Stalled, Step, Upper};
use super::report;
use super::schedule::{self, read_ledger, Done, Floor, Job, Kind, Running, Scheduler, Settings, Way};
use super::support::TempDir;
use super::tests::{
    attempted, climbing, ended, finder, job_events, job_of, opt, opt_game, quiet, seeded_with, start, stepping, LAST,
};
use std::path::Path;
use std::time::{Duration, Instant};
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::replay_finished;
use taxman::json::Json;

/// A scheduler of the board `n` alone, mapped by hand: its lower bound worth `lower`, at most the optimum (the game is
/// the optimal one, which these tests' jobs do not replay), its upper bound and its root's `upper`.
fn alone(dir: &Path, n: u32, settings: &Settings, lower: u64, upper: u64) -> Scheduler {
    let (mut scheduler, _) = Scheduler::open(n, n, dir, settings, None).unwrap();
    let game = Lower { value: lower, moves: opt_game(n), promotions: None, rule: Rule::Start, from: None };
    scheduler.book.offer_lower(n, game, false).unwrap();
    scheduler.book.offer_upper(n, Upper { value: upper, rule: Rule::Root, from: None }, false).unwrap();
    let entry = scheduler.book.entries.get_mut(&n).unwrap();
    (entry.mapped, entry.root) = (true, Some(upper));
    scheduler
}

/// A floor step on the board `n` by hand: under `floor` (a score), capped by `cap`, for `limit`.
fn step_job(n: u32, floor: Floor, cap: u64, limit: Duration) -> Job {
    let mut job = job_of(n, Kind::Floor, Vec::new(), Some(cap), finder(0.0, None));
    (job.floor, job.limit) = (Some(floor), limit);
    job
}

/// What a worker makes of `job`, as the main thread receives it.
fn ran(dir: &Path, job: &Job) -> Done {
    Done {
        n: job.n,
        kind: job.kind,
        limit: job.limit.as_secs_f64(),
        below_proven: job.below_proven,
        above_proven: job.above_proven,
        floor: job.floor.map(|floor| floor.score),
        result: Ok(attempted(dir, job)),
        sent: Instant::now(),
    }
}

fn text(event: &Json, field: &str) -> Option<String> {
    event.get(field).and_then(Json::as_str).map(str::to_owned)
}

/// A search under a floor above the optimum completes having found nothing and its floor is the board's new upper
/// bound, by the rule `floor`; it is a point of the board's line, the ledger keeps it, the event says what it was, and
/// a gap within two steps is then the proof's.
#[test]
fn a_floor_step_that_completes_lowers_the_upper_bound_with_the_floor_rule() {
    let (dir, n) = (TempDir::new("ledger-floor-held"), 1218);
    let mut scheduler = alone(&dir, n, &Settings { floor_step: 20, lns_seconds: 0.0, ..quiet() }, opt(n), opt(n) + 60);
    let job = scheduler.next_job(0.0).unwrap();
    // The gap, 60, is more than two steps of 20: the floor is a step below the upper bound; nothing seeds the search.
    assert_eq!((job.kind, job.floor), (Kind::Floor, Some(Floor { score: opt(n) + 40, resume: false })));
    assert!(job.seeds.is_empty() && job.cap == Some(opt(n) + 60));
    start(&mut scheduler, &job);
    let done = ran(&dir, &job);
    assert!(done.result.as_ref().unwrap().held);
    scheduler.finish(done).unwrap();

    let entry = &scheduler.book.entries[&n];
    assert_eq!(entry.upper, Some(Upper { value: opt(n) + 40, rule: Rule::Floor, from: None }));
    assert_eq!(entry.lower.as_ref().map(|l| l.value), Some(opt(n)));
    assert_eq!((entry.steps.len(), entry.stalled, entry.attempts, entry.level), (1, None, 1, 0));
    let step = entry.steps[0];
    assert!(step.floor == opt(n) + 40 && step.nodes > 0 && step.seconds > 0.0, "{step:?}");
    let kept = Entry::from_json(&entry.to_json(None)).unwrap();
    assert_eq!((kept.steps, kept.stalled, kept.root), (entry.steps.clone(), None, Some(opt(n) + 60)));

    let events = job_events(&dir);
    let event = &events[0];
    assert_eq!((text(event, "job").as_deref(), text(event, "outcome").as_deref()), (Some("floor"), Some("held")));
    assert_eq!(event.get("floor").and_then(Json::as_u64), Some(opt(n) + 40));
    assert_eq!(event.get("distance").and_then(Json::as_u64), Some(20));
    assert_eq!(event.get("held").and_then(Json::as_bool), Some(true));
    assert!(event.get("forecast").and_then(Json::as_f64).is_some() && event.get("forecast_s").is_some(), "{event}");

    // The gap is 40 now, within two steps of 20: the proof, the ordinary search.
    let next = scheduler.next_job(0.0).unwrap();
    assert_eq!((next.kind, next.floor), (Kind::Search, None));
    start(&mut scheduler, &next);
    scheduler.finish(ran(&dir, &next)).unwrap();
    let entry = &scheduler.book.entries[&n];
    assert!(entry.proven() && entry.upper.unwrap().value == opt(n));
}

/// A floor below the optimum finds sets, the last of them the optimum, and the search that completes has proven it
/// (Lemma F): the lower bound is the game it found, which the referee replays, and the upper bound is exhausted.  One
/// that its limit stops with a set found leaves the lower bound at that game; its floor, now below it, is dropped, and
/// its checkpoint, a search from that set, becomes the board's own for the proof if the board has none.
#[test]
fn a_floor_step_that_finds_a_set_raises_the_lower_bound_with_a_game_the_referee_replays() {
    let (dir, n) = (TempDir::new("ledger-floor-found"), 1218);
    let mut scheduler = alone(&dir, n, &quiet(), opt(n) - 204, opt(n) + 100);
    let job = step_job(n, Floor { score: opt(n) - 100, resume: false }, opt(n) + 100, Duration::from_secs(120));
    start(&mut scheduler, &job);
    let done = ran(&dir, &job);
    let attempt = done.result.as_ref().unwrap();
    assert!(attempt.completed && !attempt.held && !attempt.promotions.is_empty());
    scheduler.finish(done).unwrap();
    let entry = &scheduler.book.entries[&n];
    let (lower, upper) = (entry.lower.as_ref().unwrap(), entry.upper.unwrap());
    assert!(entry.proven() && lower.value == opt(n) && upper.value == opt(n));
    assert_eq!((lower.rule, upper.rule), (Rule::Search, Rule::Exhausted));
    assert_eq!(replay_finished(n, &lower.moves), Ok(opt(n)), "the game of the lower bound");
    assert!(entry.steps.is_empty() && entry.stalled.is_none());
    let events = job_events(&dir);
    assert_eq!(text(&events[0], "outcome").as_deref(), Some("proved"));
    assert!(!dir.join("ckpt").read_dir().unwrap().any(|_| true), "no checkpoint is left of a proven board");

    // A step stopped with a set found, 30 under the optimum: the lower bound is that game; the floor, below it, is
    // dropped with the stalled step that was there; its checkpoint becomes the board's own, unless the board has one,
    // which stays and the step's is deleted.
    for ordinary in [false, true] {
        let dir = TempDir::new("ledger-floor-found-stopped");
        let mut scheduler = alone(&dir, n, &quiet(), opt(n) - 204, opt(n) + 100);
        let floor = opt(n) - 100;
        let stalled = Stalled { floor, nodes: 100, seconds: 1.0, tries: 1 };
        scheduler.book.entries.get_mut(&n).unwrap().stalled = Some(stalled);
        std::fs::create_dir_all(dir.join("ckpt")).unwrap();
        let (step_checkpoint, board_checkpoint) =
            (dir.join("ckpt").join(format!("n{n}.floor.ckpt")), dir.join("ckpt").join(format!("n{n}.ckpt")));
        std::fs::write(&step_checkpoint, b"the step's checkpoint").unwrap();
        if ordinary {
            std::fs::write(&board_checkpoint, b"the board's checkpoint").unwrap();
        }
        let mut done = ended(n, Kind::Floor, false);
        done.floor = Some(floor);
        let attempt = done.result.as_mut().unwrap();
        (attempt.value, attempt.promotions, attempt.nodes) = (opt(n) - 30, vec![7], 5000);
        scheduler.finish(done).unwrap();
        let entry = &scheduler.book.entries[&n];
        assert_eq!(entry.lower.as_ref().map(|l| (l.value, l.rule)), Some((opt(n) - 30, Rule::Search)));
        assert!(entry.stalled.is_none() && entry.steps.is_empty() && entry.upper.unwrap().value == opt(n) + 100);
        assert!(!step_checkpoint.exists(), "the step's checkpoint is gone");
        let expected: &[u8] = if ordinary { b"the board's checkpoint" } else { b"the step's checkpoint" };
        assert_eq!(std::fs::read(&board_checkpoint).unwrap(), expected, "ordinary: {ordinary}");
        assert_eq!(entry.nodes, if ordinary { 0 } else { 5000 }, "the steps of the search the checkpoint holds");
        assert_eq!(text(&job_events(&dir)[0], "outcome").as_deref(), Some("found"));
    }
}

/// A step that its limit stops with nothing found is kept: the board remembers its floor, its steps and its seconds,
/// and the next job of the board is that step again, resumed under its floor with the limit doubled; it completes
/// where it left off.  The checkpoint of a step is not the checkpoint of the board's ordinary searches.
#[test]
fn a_floor_step_that_its_limit_stops_is_kept_and_resumed_under_its_floor() {
    let (dir, n) = (TempDir::new("ledger-floor-stalled"), 1218);
    let mut scheduler =
        alone(&dir, n, &Settings { floor_step: 20, lns_seconds: 0.0, ..quiet() }, opt(n) - 30, opt(n) + 100);
    // The floor at the optimum is the proof's tree, thousands of steps, and the limit of a millisecond stops it at an
    // early check of the engine.
    let floor = Floor { score: opt(n), resume: false };
    let job = step_job(n, floor, opt(n) + 100, Duration::from_millis(1));
    start(&mut scheduler, &job);
    scheduler.finish(ran(&dir, &job)).unwrap();
    let entry = &scheduler.book.entries[&n];
    let stalled = entry.stalled.expect("the step is kept");
    assert_eq!((stalled.floor, stalled.tries), (opt(n), 1));
    assert!(stalled.seconds > 0.0, "{stalled:?}");
    assert!(entry.steps.is_empty() && entry.upper.unwrap().value == opt(n) + 100, "{entry:?}");
    assert!(dir.join("ckpt").join(format!("n{n}.floor.ckpt")).exists());
    assert!(!dir.join("ckpt").join(format!("n{n}.ckpt")).exists(), "not the checkpoint of the ordinary searches");
    assert_eq!(text(&job_events(&dir)[0], "outcome").as_deref(), Some("stalled"));
    let kept = Entry::from_json(&entry.to_json(None)).unwrap();
    assert_eq!(kept.stalled, Some(stalled), "the ledger keeps it");

    // The next job is that step, resumed, with a limit of at least twice the base.
    let next = scheduler.next_job(0.0).unwrap();
    assert_eq!((next.kind, next.floor), (Kind::Floor, Some(Floor { score: opt(n), resume: true })));
    assert!(next.limit >= Duration::from_secs_f64(2.0 * Settings::default().base), "{:?}", next.limit);
    start(&mut scheduler, &next);
    scheduler.finish(ran(&dir, &next)).unwrap();
    let entry = &scheduler.book.entries[&n];
    assert!(entry.stalled.is_none());
    let step = entry.steps.last().expect("the step completed");
    assert_eq!(step.floor, opt(n));
    assert!(step.nodes >= stalled.nodes && step.seconds > stalled.seconds, "{step:?} from {stalled:?}");
    assert_eq!(entry.upper, Some(Upper { value: opt(n), rule: Rule::Floor, from: None }));
    assert!(!dir.join("ckpt").join(format!("n{n}.floor.ckpt")).exists(), "a completed step leaves no checkpoint");
    let outcomes: Vec<Option<String>> = job_events(&dir).iter().map(|e| text(e, "outcome")).collect();
    assert_eq!(outcomes, [Some("stalled".to_owned()), Some("held".to_owned())]);
}

/// A step whose upper bound fell to its floor while it went on ended at the ceiling with nothing explored: it is moot,
/// not stalled, and the board plans again from its bounds.
#[test]
fn a_floor_step_ended_by_the_ceiling_at_its_floor_is_moot() {
    let (dir, n) = (TempDir::new("ledger-floor-moot"), 940);
    let mut scheduler =
        alone(&dir, n, &Settings { floor_step: 20, lns_seconds: 0.0, ..quiet() }, opt(n) - 10, opt(n) + 100);
    let mut done = ended(n, Kind::Floor, true);
    done.floor = Some(opt(n) + 80);
    let attempt = done.result.as_mut().unwrap();
    (attempt.value, attempt.reached_cap, attempt.promotions) = (opt(n) - 10, true, Vec::new());
    scheduler.finish(done).unwrap();
    let entry = &scheduler.book.entries[&n];
    assert!(entry.stalled.is_none() && entry.steps.is_empty() && !entry.proven());
    assert_eq!(text(&job_events(&dir)[0], "outcome").as_deref(), Some("moot"));
}

/// The climbs go first; then the boards whose bounds a proven neighbour changed; then the boards that have yet to give
/// their line two points, each a step; then the board whose proof is forecast to cost the least, by the line of its own
/// steps; the ordinary job of a board with a wide gap is a step to a floor a step below its upper bound.
#[test]
fn floor_steps_go_by_the_forecast_of_the_proofs_and_the_climbs_go_first() {
    let dir = TempDir::new("ledger-floor-order");
    let mut scheduler = climbing(&dir, &[906], &stepping());
    // Two boards with steps: 950 on a line well under the median, 954 on one well over it.  The root of each is 1,400
    // above the optimum, and the lower bound 10 under: the proof is a floor 1,410 below the root.
    let steps_of = |intercept: f64, slope: f64, n: u32| -> Vec<Step> {
        let root = opt(n) + 1400;
        [700u64, 900, 1100]
            .iter()
            .map(|&d| {
                let nodes = 10f64.powf(intercept + slope * d as f64);
                Step { floor: root - d, nodes: nodes as u64, seconds: nodes / 1000.0 }
            })
            .collect()
    };
    scheduler.book.entries.get_mut(&950).unwrap().steps = steps_of(-1.0, 0.007, 950);
    scheduler.book.entries.get_mut(&954).unwrap().steps = steps_of(1.0, 0.007, 954);
    scheduler.book.entries.get_mut(&940).unwrap().boosted = true;
    // Board 948 has the cheap line too, but its ordinary searches have spent 10^13 steps without finishing: it is
    // forecast dearer than any line.
    let entry = scheduler.book.entries.get_mut(&948).unwrap();
    (entry.steps, entry.nodes) = (steps_of(-1.0, 0.007, 948), 10u64.pow(13));
    // Board 930, a gap of 1, goes to its proof, not a step; its root is as far above as the others'.
    scheduler.book.entries.get_mut(&930).unwrap().root = Some(opt(930) + 1400);

    let mut jobs = Vec::new();
    while let Some(job) = scheduler.next_job(0.0) {
        start(&mut scheduler, &job);
        jobs.push(job);
    }
    let order: Vec<(Kind, u32)> = jobs.iter().map(|job| (job.kind, job.n)).collect();
    // The climbs of the two wedges, the board that was boosted, the boards with no line yet by n, and then the boards
    // with one by the forecast: 950 on the cheap line, 930 (its proof, by the median line), 954 on the dear line, and
    // 948, whose searches spent the most.
    assert_eq!(order[..2], [(Kind::Climb(Way::Up), 909), (Kind::Climb(Way::Down), 903)], "{order:?}");
    let ordinary = &jobs[2..];
    assert_eq!((ordinary[0].kind, ordinary[0].n), (Kind::Floor, 940), "the boosted board first");
    let (explored, tail) = ordinary[1..].split_at(ordinary.len() - 1 - 4);
    let tail: Vec<u32> = tail.iter().map(|job| job.n).collect();
    assert_eq!(tail, [950, 930, 954, 948], "{order:?}");
    assert!(explored.len() > 10 && explored.windows(2).all(|pair| pair[0].n < pair[1].n), "{order:?}");
    for job in ordinary {
        let kind = if job.n == 930 { Kind::Search } else { Kind::Floor };
        assert_eq!(job.kind, kind, "n={}: a gap of {:?}", job.n, scheduler.book.entries[&job.n].gap());
    }
    // 950's line has the slope 0.007 and the step is 150 points at the median slope, 146 at 0.007: a floor that far
    // below its upper bound; a board with none has the median slope and the step of 150.
    let floor_of = |n: u32| jobs.iter().find(|job| job.n == n).and_then(|job| job.floor).unwrap();
    assert_eq!(floor_of(950).score, opt(950) + 400 - 146);
    assert_eq!(floor_of(940).score, opt(940) + 400 - 150);
    assert!(floor_of(954).score > 0 && !floor_of(954).resume);
}

/// A climb into the repeat class {925, 926} holds the boards beside the whole class, 924 and 927, not just the
/// neighbours of the board searched; and a climb down into it from 927 is seeded with 927's game, which is not beside
/// the board searched.
#[test]
fn a_climb_into_a_class_holds_the_boards_beside_it_and_is_seeded_from_the_far_side() {
    let dir = TempDir::new("ledger-class-hold");
    let mut scheduler = climbing(&dir, &[], &Settings { climb_limit: 0.0, ..quiet() });
    assert_eq!(scheduler.book.class(925), [925, 926]);
    // 927 is the cheapest board, a gap of 1 as 930 has.
    let lower = Lower { value: opt(927), moves: opt_game(927), promotions: None, rule: Rule::Start, from: None };
    scheduler.book.offer_lower(927, lower, false).unwrap();
    scheduler.book.offer_upper(927, Upper { value: opt(927) + 1, rule: Rule::Root, from: None }, false).unwrap();
    assert_eq!(scheduler.next_job(0.0).unwrap().n, 927);
    scheduler.running.insert(925, Running { kind: Kind::Climb(Way::Down), control: Control::new() });
    assert_eq!(scheduler.next_job(0.0).unwrap().n, 930, "927 waits for the climb into 925 and 926");
    scheduler.running.insert(925, Running { kind: Kind::Search, control: Control::new() });
    assert_eq!(scheduler.next_job(0.0).unwrap().n, 927, "an ordinary search holds nothing");
    scheduler.running.remove(&925);

    let dir = TempDir::new("ledger-class-seed");
    let mut scheduler = climbing(&dir, &[927], &Settings { climb_up: false, ..quiet() });
    let job = scheduler.next_job(0.0).unwrap();
    assert_eq!((job.kind, job.n), (Kind::Climb(Way::Down), 925));
    let game = scheduler.book.entries[&927].lower.as_ref().unwrap().moves.clone();
    assert!(seeded_with(&job, &game), "seeded with the game of 927, past the class");
    assert!(job.seeds.iter().any(|seed| seed.from == 927));
    start(&mut scheduler, &job);
    assert!(scheduler.next_job(0.0).is_some());
}

/// A run with floor steps of 20 points over 901..960, the truth hidden: every board is proven at its optimum, the steps
/// ended as the events say and the report counts them.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_run_is_exported_and_both_referees_accept_the_file")]
fn a_run_with_floor_steps_proves_the_stretch_and_the_events_and_the_report_say_how_they_ended() {
    let dir = TempDir::new("ledger-floor-run");
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.01,
        lns_seconds: 0.0,
        floor_step: 20,
        save_every: 0.5,
        ..Settings::default()
    };
    assert_eq!(schedule::run(901, LAST, &dir, &settings, None).unwrap(), "all proven");
    let entries = read_ledger(&dir).unwrap();
    for e in &entries {
        assert!(e.proven() && e.upper.unwrap().value == opt(e.n), "n={}: {e:?}", e.n);
    }
    let steps: Vec<Json> =
        job_events(&dir).into_iter().filter(|e| text(e, "job").as_deref() == Some("floor")).collect();
    assert!(steps.len() > (LAST - 900) as usize / 6, "{} floor steps", steps.len());
    for e in &steps {
        let outcome = text(e, "outcome").unwrap();
        assert!(["held", "proved", "moot", "found", "stalled"].contains(&outcome.as_str()), "{e}");
        let (floor, after) = (e.get("floor").and_then(Json::as_u64).unwrap(), e.get("after").unwrap());
        if outcome == "held" {
            assert!(
                after.at(1).and_then(Json::as_u64).unwrap() <= floor,
                "a held step leaves U at its floor or lower: {e}"
            );
        }
        assert!(e.get("distance").is_some() && e.get("forecast").is_some() && e.get("forecast_s").is_some(), "{e}");
    }
    assert!(steps.iter().any(|e| text(e, "outcome").as_deref() == Some("held")));
    // Every searched board has its root's bound; the start event has the settings.
    assert!(entries.iter().filter(|e| e.search_s > 0.0).all(|e| e.root.is_some()));
    let events = std::fs::read_to_string(schedule::events_path(&dir)).unwrap();
    assert!(events.lines().next().unwrap().contains("\"floor_step\":20"), "the start event has the setting");
    let (report, ok) = report::report(&dir, &[]).unwrap();
    assert!(ok, "{report}");
    assert!(report.contains(&format!("floor steps: {} among the searches", steps.len())), "{report}");
}

/// A ledger from before the floor steps (its boards have no root, steps or stalled step; the checkpoint of an ordinary
/// search is in its directory) is read, its boards get their roots, and the run goes on to prove the board; the old
/// checkpoint is resumed when the board comes to its proof, and none is left.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_run_is_exported_and_both_referees_accept_the_file")]
fn a_ledger_from_before_the_floor_steps_goes_on() {
    let (dir, n) = (TempDir::new("ledger-old-format"), 1218);
    // What the earlier build left: the board mapped (the start game, the root's bound), and the file as that build
    // wrote it, none of the three new fields.
    let map = attempted(&dir, &job_of(n, Kind::Init, Vec::new(), None, finder(0.0, None)));
    let bag = taxman::with_width!(n, W => Board::<W>::for_n(n).in_the_bag);
    let entry = Entry {
        n,
        bag,
        lower: Some(Lower {
            value: map.value,
            moves: map.moves.clone(),
            promotions: Some(map.promotions.clone()),
            rule: Rule::Start,
            from: None,
        }),
        upper: Some(Upper { value: map.upper.unwrap(), rule: Rule::Root, from: None }),
        mapped: true,
        attempts: 2,
        level: 2,
        search_s: 6.0,
        nodes: 1000,
        ..Entry::default()
    };
    let mut json = entry.to_json(None);
    let Json::Obj(fields) = &mut json else { panic!("{json}") };
    fields.retain(|(key, _)| !["root", "steps", "stalled"].contains(&key.as_str()));
    let path = schedule::ledger_path(&dir);
    std::fs::write(&path, format!("{json}\n")).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("\"root\":") && !text.contains("\"steps\":") && !text.contains("\"stalled\":"));
    let before = read_ledger(&dir).unwrap();
    assert!(before[0].mapped && before[0].root.is_none() && before[0].steps.is_empty() && before[0].stalled.is_none());
    // An ordinary search of the board, stopped, left its checkpoint.
    let stopped = attempted(&dir, &job_of(n, Kind::Search, Vec::new(), None, finder(0.0, None)));
    assert!(!stopped.completed && dir.join("ckpt").join(format!("n{n}.ckpt")).exists());

    let settings =
        Settings { workers: 1, cpu_budget: 1e9, base: 0.05, lns_seconds: 0.0, floor_step: 20, ..Settings::default() };
    assert_eq!(schedule::run(n, n, &dir, &settings, None).unwrap(), "all proven");
    let after = read_ledger(&dir).unwrap();
    assert!(after[0].proven() && after[0].upper.unwrap().value == opt(n));
    assert!(after[0].root.is_some_and(|root| root >= opt(n)), "the root's bound was read");
    assert!(!dir.join("ckpt").read_dir().unwrap().any(|_| true), "no checkpoint is left");
}
