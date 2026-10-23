//! The lab channel (`SolveOptions::lab`: `--lab-dir`, `--hook`; solver/docs/design/lab.md): a hostile lab changes no
//! result.  A range with a lab folder and a hook is fed offers of sets that are off the board, given twice, not
//! promotable or claimed above their worth, commands that are not commands, restarts with tactics a restart does not
//! take, threads it cannot have, `conflict_branching`, `conflict_look` and the gate switched at every tick, a restart
//! of a game in the middle of its search, and a redeploy at a commit that is not on master (the solver does not judge
//! commits: it stops, and the chain's script, which does, runs the build it has again, as the second run here does).
//! Every bad command is refused with its reason, every game ends with the score of `optimal.json`, and the file passes
//! the verifier.  The game restarted records it, keeps its first checkpoint aside, and the hook hears of it twice, once
//! when it runs long and once with its line.  Small games of the default search, two threads.
//!
//! The policy for the games to come (`policy`) has its tests below the hostile lab's: it reaches the games in flight
//! and those set up later, yields to a game's own `branching`, survives a restart of the run, and reaches a search that
//! resumes from a checkpoint made under other settings.  The gate, the gap from which conflict branching chooses, is
//! one of the settings of both commands, and a policy's gate changes the tree of a game and not its answer.

mod common;

use common::{options, TempDir};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::results::verify_files;
use taxman::solver::{self, LabOptions, SolveOptions};

/// The range, and its game whose search the lab restarts: 950 searches about 1,600 steps from the game before it, a
/// tenth of a second on one worker, which the lab holds and then stops in the middle.
const LO: u32 = 948;
const G: u32 = 950;
const HI: u32 = 952;

/// How long the lab waits for the run before the test fails.
const PATIENCE: Duration = Duration::from_secs(30);

/// Puts the command `text` in the folder `lab` as `in/name` (written aside and renamed in, as a lab does), and waits
/// for its outcome.
fn send(lab: &Path, name: &str, text: &str) -> Json {
    let aside = lab.join("in").join(format!(".{name}"));
    fs::write(&aside, text).unwrap();
    fs::rename(&aside, lab.join("in").join(name)).unwrap();
    let out = lab.join("out").join(name);
    let since = Instant::now();
    loop {
        if let Ok(json) = Json::from_file(&out) {
            return json;
        }
        assert!(since.elapsed() < PATIENCE, "no outcome for {name}");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The status and the reason of an outcome.
fn said(outcome: &Json) -> (&str, &str) {
    let field = |key| outcome.get(key).and_then(Json::as_str).unwrap_or_default();
    (field("status"), field("reason"))
}

/// Waits until the results file `out` has a line for game `n`.
fn wait_for_line(out: &Path, n: u32) {
    let since = Instant::now();
    let has = || {
        let text = fs::read_to_string(out).unwrap_or_default();
        text.lines()
            .filter_map(|line| Json::parse(line).ok())
            .any(|line| line.get("n").and_then(Json::as_u32) == Some(n))
    };
    while !has() {
        assert!(since.elapsed() < PATIENCE, "no line for n={n}");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Waits until game `n` is set up and its search not ended, which an `offer` of nothing tells (each probe is the command
/// `tag-probe-K`).
fn wait_until_set_up(lab: &Path, n: u32, tag: &str) {
    let since = Instant::now();
    for k in 0.. {
        let probe = send(lab, &format!("{tag}-probe-{k:03}"), &format!(r#"{{"command":"offer","n":{n},"set":[]}}"#));
        if said(&probe).0 == "applied" {
            break;
        }
        assert!(since.elapsed() < PATIENCE, "game {n} is never set up: {probe}");
    }
}

/// The lines of a results file.
fn lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

/// Ends the run if the lab fails, so that the test reports the failure instead of waiting for ever for the games it
/// holds.
struct EndIfFailed<'a>(&'a Path);

impl Drop for EndIfFailed<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _ = fs::write(self.0.join("in/zz-end"), r#"{"command":"redeploy","commit":"0000000"}"#);
        }
    }
}

/// The hostile lab: refused commands, each with the words its reason must hold, then the commands that are applied,
/// the restart of `G` in the middle of its search, `conflict_branching`, `conflict_look` and the gate switched at every
/// tick, and a redeploy.  Returns the settings of the last switch that was applied while `G` searched, if one was, and
/// the last gate.
fn hostile(lab: &Path, out: &Path) -> (Option<(&'static str, &'static str)>, Option<u64>) {
    let _end = EndIfFailed(lab);
    // `G` waits, set up and held at no worker, until the lab lets it go.
    let since = Instant::now();
    wait_until_set_up(lab, G, "01");
    wait_for_line(out, LO);
    let all: Vec<String> = (G / 2 + 1..=G).map(|x| x.to_string()).collect();
    let refused = [
        ("10-not-json", "not json".to_owned(), "JSON syntax error"),
        ("11-unknown", r#"{"command":"close","n":950}"#.to_owned(), "unknown command"),
        ("12-advice", r#"{"command":"advice","n":950}"#.to_owned(), "reserved"),
        ("13-field", r#"{"command":"checkpoint","now":true}"#.to_owned(), "takes no field"),
        ("14-off-board", format!(r#"{{"command":"offer","n":{G},"set":[{}]}}"#, G + 1), "not a number of the board"),
        ("15-twice", format!(r#"{{"command":"offer","n":{G},"set":[600,600]}}"#), "twice"),
        (
            "16-illegal",
            format!(r#"{{"command":"offer","n":{G},"set":[{}]}}"#, all.join(",")),
            "can be promoted in this game",
        ),
        ("17-overstated", format!(r#"{{"command":"offer","n":{G},"set":[],"value":1}}"#), "not the 1 claimed"),
        ("18-not-search", format!(r#"{{"command":"restart","n":{G},"tactics":"prev_cap=off"}}"#), "not a search"),
        ("19-bad-value", format!(r#"{{"command":"restart","n":{G},"tactics":"symmetry=maybe"}}"#), "on or off"),
        ("20-no-threads", r#"{"command":"threads","total":0}"#.to_owned(), "from 1 to"),
        ("21-too-many", r#"{"command":"threads","n":951,"workers":3}"#.to_owned(), "the run has 2 threads"),
        ("22-outside", r#"{"command":"threads","n":960,"workers":1}"#.to_owned(), "not in the range"),
        ("23-concluded", format!(r#"{{"command":"branching","n":{LO},"conflict_branching":"on"}}"#), "concluded"),
        ("24-not-running", format!(r#"{{"command":"finder","n":{G},"seconds":1}}"#), "not running"),
        ("25-commit", r#"{"command":"redeploy","commit":"HEAD"}"#.to_owned(), "hexadecimal"),
        ("26-bad-look", format!(r#"{{"command":"branching","n":{G},"conflict_look":"maybe"}}"#), "conflict_look is"),
        (
            "27-no-switch",
            format!(r#"{{"command":"branching","n":{G}}}"#),
            "needs conflict_branching, conflict_look or gate",
        ),
        ("28-look-concluded", format!(r#"{{"command":"branching","n":{LO},"conflict_look":"on"}}"#), "concluded"),
        (
            "29-gate-high",
            format!(r#"{{"command":"branching","n":{G},"gate":4001}}"#),
            "gate is a whole number from 0 to",
        ),
        ("29-gate-low", format!(r#"{{"command":"branching","n":{G},"gate":-1}}"#), "gate is a whole number from 0 to"),
        (
            "29-gate-text",
            format!(r#"{{"command":"branching","n":{G},"gate":"150"}}"#),
            "gate is a whole number from 0 to",
        ),
        (
            "29-gate-off",
            format!(r#"{{"command":"branching","n":{G},"conflict_branching":"off","gate":150}}"#),
            "turns it off",
        ),
        ("29-gate-concluded", format!(r#"{{"command":"branching","n":{LO},"gate":150}}"#), "concluded"),
    ];
    for (name, text, why) in &refused {
        let outcome = send(lab, name, text);
        let (status, reason) = said(&outcome);
        assert_eq!(status, "refused", "{name}: {outcome}");
        assert!(reason.contains(why), "{name}: {reason}");
    }
    let applied = [
        ("30-nothing-better", format!(r#"{{"command":"offer","n":{G},"set":[],"value":0}}"#)),
        ("31-on", format!(r#"{{"command":"branching","n":{G},"conflict_branching":"on"}}"#)),
        ("32-off", format!(r#"{{"command":"branching","n":{G},"conflict_branching":"off"}}"#)),
        ("32-look-on", format!(r#"{{"command":"branching","n":{G},"conflict_look":"on"}}"#)),
        ("32-both", format!(r#"{{"command":"branching","n":{G},"conflict_branching":"on","conflict_look":"off"}}"#)),
        ("32-gate", format!(r#"{{"command":"branching","n":{G},"gate":150}}"#)),
        (
            "32-all",
            format!(r#"{{"command":"branching","n":{G},"conflict_branching":"on","conflict_look":"on","gate":0}}"#),
        ),
        ("33-more", r#"{"command":"threads","total":3}"#.to_owned()),
        ("34-fewer", r#"{"command":"threads","total":2}"#.to_owned()),
        ("35-checkpoint", r#"{"command":"checkpoint"}"#.to_owned()),
    ];
    for (name, text) in &applied {
        let outcome = send(lab, name, text);
        assert_eq!(said(&outcome).0, "applied", "{name}: {outcome}");
    }
    // `G` starts, as the head, on one worker, and is held at none before it can finish: its search is stopped in the
    // middle, with a checkpoint, and started again from nothing.
    wait_for_line(out, G - 1);
    for (name, workers) in [("40-go", "1"), ("41-hold", "0")] {
        let outcome = send(lab, name, &format!(r#"{{"command":"threads","n":{G},"workers":{workers}}}"#));
        assert_eq!(said(&outcome).0, "applied", "{name}: {outcome}");
    }
    // Once its engine runs (its start, the seed's survivors and the local search, comes first), it writes its
    // checkpoint.
    for k in 0.. {
        let outcome = send(lab, &format!("42-checkpoint-{k:03}"), r#"{"command":"checkpoint"}"#);
        if said(&outcome) == ("applied", "the searches of [950] write their checkpoints") {
            break;
        }
        assert!(since.elapsed() < PATIENCE, "game {G}'s engine never runs: {outcome}");
    }
    let outcome = send(lab, "43-restart", &format!(r#"{{"command":"restart","n":{G},"tactics":"strong_depth=40"}}"#));
    let (status, reason) = said(&outcome);
    assert_eq!(status, "applied", "{outcome}");
    assert!(reason.contains("its checkpoint is n0950.ckpt.restart1"), "{reason}");
    let outcome = send(lab, "44-let-go", &format!(r#"{{"command":"threads","n":{G},"workers":null}}"#));
    assert_eq!(said(&outcome).0, "applied", "{outcome}");
    // At every tick while it runs: applied, or refused while it is set up again or once its search has ended.  The
    // look is switched the other way from the branching, and a gate comes with each switch on of the branching (a gate
    // cannot come with a switch off).
    let (mut last, mut gate) = (None, None);
    for k in 0..6 {
        let (setting, look) = if k % 2 == 0 { ("on", "off") } else { ("off", "on") };
        let given = (setting == "on").then_some(100 * (k + 1));
        let name = format!("50-switch-{k}");
        let with_gate = given.map_or(String::new(), |gate| format!(r#","gate":{gate}"#));
        let command = format!(
            r#"{{"command":"branching","n":{G},"conflict_branching":"{setting}","conflict_look":"{look}"{with_gate}}}"#
        );
        let outcome = send(lab, &name, &command);
        let (status, reason) = said(&outcome);
        let later = ["set up", "ended", "concluded"].iter().any(|why| reason.contains(why));
        assert!(status == "applied" || later, "{name}: {outcome}");
        if status == "applied" {
            last = Some((setting, look));
            gate = given.or(gate);
        }
    }
    let outcome =
        send(lab, "60-redeploy", r#"{"command":"redeploy","commit":"0123456789abcdef0123456789abcdef01234567"}"#);
    assert_eq!(said(&outcome).0, "applied", "{outcome}");
    (last, gate)
}

#[test]
fn a_hostile_lab_changes_no_result() {
    let dir = TempDir::new("lab");
    let (lab, out, hook_log) = (dir.join("lab"), dir.join("r.jsonl"), dir.join("hook.log"));
    let hook = dir.join("hook.sh");
    fs::write(&hook, format!("#!/bin/sh\nprintf '%s\\n' \"$1\" >> '{}'\n", hook_log.display())).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&hook, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    fs::create_dir_all(lab.join("in")).unwrap();
    // Held from the start: `G` is set up, and waits; so does `HI`, so that the range lasts until the lab is done.
    fs::write(lab.join("in/00-hold"), format!(r#"{{"command":"threads","n":{G},"workers":0}}"#)).unwrap();
    fs::write(lab.join("in/00-hold-last"), format!(r#"{{"command":"threads","n":{HI},"workers":0}}"#)).unwrap();
    let opts = SolveOptions {
        use_prev: true,
        checkpoint_dir: Some(dir.join("ckpt")),
        checkpoint_interval: Duration::from_secs(600),
        // The lab's tick.
        progress: Some(Duration::from_millis(5)),
        lab: LabOptions { dir: Some(lab.clone()), hook: Some(hook.clone()), hook_after: Duration::ZERO },
        ..options(2)
    };
    let summary = std::thread::scope(|scope| {
        let lab = scope.spawn(|| hostile(&lab, &out));
        let summary = solver::solve_range_each(LO, HI, &opts, Some(&out), |_| {}).unwrap();
        (summary, lab.join().unwrap())
    });
    let (summary, (last_switch, last_gate)) = summary;
    assert_eq!(summary.redeploy.as_deref(), Some("0123456789abcdef0123456789abcdef01234567"));
    for hold in ["00-hold", "00-hold-last"] {
        let outcome = Json::from_file(lab.join("out").join(hold)).unwrap();
        assert_eq!(said(&outcome).0, "applied", "{outcome}");
    }
    // The chain's script runs the build it has again, with the same command.
    let again = solver::solve_range_each(LO, HI, &opts, Some(&out), |_| {}).unwrap();
    assert_eq!(again.redeploy, None);

    let optimal = Optimal::built_in();
    let lines = lines(&out);
    let ns: Vec<u32> = lines.iter().map(|line| line.get("n").and_then(Json::as_u32).unwrap()).collect();
    assert_eq!(ns, (LO..=HI).collect::<Vec<_>>());
    for line in &lines {
        let n = line.get("n").and_then(Json::as_u32).unwrap();
        assert_eq!(line.get("completed").and_then(Json::as_bool), Some(true), "n={n}");
        assert_eq!(line.get("score").and_then(Json::as_u64), optimal.score(n), "n={n}");
        let restarts = line.get("lab").and_then(|lab| lab.get("restarts")).and_then(Json::as_arr).map(<[_]>::len);
        assert_eq!(restarts, (n == G).then_some(1), "n={n}");
    }
    let game = &lines[(G - LO) as usize];
    let switches = game.get("tactic_switches").unwrap();
    assert_eq!(switches.get("strong_depth").and_then(Json::as_u32), Some(40), "{switches}");
    // The line keeps the lab's last settings of the two switches, and the search ended with them.
    let record = game.get("lab").unwrap();
    let kept = |key: &str| record.get(key).and_then(Json::as_str);
    let ended_with = |key: &str| switches.get(key).and_then(Json::as_bool);
    match last_switch {
        Some((branching, look)) => {
            assert_eq!((kept("conflict_branching"), kept("conflict_look")), (Some(branching), Some(look)));
            assert_eq!(ended_with("conflict_branching"), Some(branching == "on"));
            assert_eq!(ended_with("conflict_look"), Some(look == "on"));
        }
        None => assert_eq!((kept("conflict_branching"), kept("conflict_look")), (None, None)),
    }
    // The gate is the lab's last, or none (a restart returns it to the default): a number in the line, no tactic.
    assert_eq!(record.get("gate").and_then(Json::as_u64), last_gate, "{record}");
    let restart = game.get("lab").and_then(|lab| lab.get("restarts")).and_then(|r| r.at(0)).unwrap();
    assert_eq!(restart.get("aside").and_then(Json::as_str), Some("n0950.ckpt.restart1"));
    assert!(dir.join("ckpt/n0950.ckpt.restart1").exists(), "the abandoned search's checkpoint is kept");
    verify_files(&[PathBuf::from(&out)], optimal, 2).unwrap_or_else(|e| panic!("{e}"));

    // The hook heard that `G` ran long, once, and of its line (the hook runs detached: its last event may come later).
    let events = || -> Vec<Json> {
        let text = fs::read_to_string(&hook_log).unwrap_or_default();
        let events = text.lines().filter_map(|line| Json::parse(line).ok());
        events.filter(|event| event.get("n").and_then(Json::as_u32) == Some(G)).collect()
    };
    let since = Instant::now();
    while events().len() < 2 && since.elapsed() < PATIENCE {
        std::thread::sleep(Duration::from_millis(1));
    }
    let events = events();
    let kinds: Vec<&str> = events.iter().filter_map(|event| event.get("event").and_then(Json::as_str)).collect();
    assert_eq!(kinds, ["long", "done"], "{events:?}");
    // The format the hook's program reads (solver/docs/design/lab.md, "The hook").
    let ckpt = dir.join("ckpt");
    for event in &events {
        let text = |key| event.get(key).and_then(Json::as_str);
        assert_eq!(text("checkpoint_dir"), ckpt.to_str(), "{event}");
        assert_eq!(text("lab_dir"), lab.to_str(), "{event}");
        assert!(event.get("nodes").and_then(Json::as_u64).is_some() && event.get("own_seconds").is_some(), "{event}");
        assert!(event.get("best").and_then(Json::as_u64).is_some() && event.get("commit").is_some(), "{event}");
    }
    assert_eq!(events[0].get("result"), None);
    let result = events[1].get("result").unwrap();
    assert_eq!(result.get("n").and_then(Json::as_u32), Some(G));
    let best = result.get("promotions").and_then(Json::as_u32_vec).unwrap().iter().map(|&p| u64::from(p)).sum();
    assert_eq!(events[1].get("best").and_then(Json::as_u64), Some(best));
    assert_eq!(events[1].get("nodes"), result.get("nodes"));
}

// ---------------------------------------------------------------------------------------------------------
// The policy for the games to come

/// The games of the policy tests, in the range `LO..=HI` as above: `P` searches, and `Q` ends at its ceiling when its
/// search starts; the games between repeat the search of the game before them, and have no search of their own for a
/// policy to steer (the line of a repeat has no `lab`).
const P: u32 = 950;
const Q: u32 = 952;

/// The setting of the switch `key` in a `lab` record of a line, or in the policy in it: `"on"`, `"off"`, or null.
fn setting<'a>(record: &'a Json, key: &str) -> Option<&'a str> {
    record.get(key).and_then(Json::as_str)
}

/// The gate in a `lab` record of a line, or in the policy in it or in `run.lab`: a number, or null.
fn gate(record: &Json) -> Option<u64> {
    record.get("gate").and_then(Json::as_u64)
}

/// The two switches of a `lab` record or a policy: `conflict_branching` and `conflict_look`.
fn setting_pair(record: &Json) -> (Option<&str>, Option<&str>) {
    (setting(record, "conflict_branching"), setting(record, "conflict_look"))
}

/// The two switches a line says its search ended with.
fn ended_with(line: &Json) -> (Option<bool>, Option<bool>) {
    let switches = line.get("tactic_switches").unwrap();
    let switch = |key: &str| switches.get(key).and_then(Json::as_bool);
    (switch("conflict_branching"), switch("conflict_look"))
}

/// The lab of the policy test: commands that are refused, a `branching` that comes before a policy and one that comes
/// after, and the policies themselves (a gate among them), while `P` and `Q` are set up and held.
fn give_policies(lab: &Path, out: &Path) {
    let _end = EndIfFailed(lab);
    wait_until_set_up(lab, P, "01");
    wait_for_line(out, LO);
    let refused = [
        ("10-outside", r#"{"command":"policy","from":960,"conflict_look":"on"}"#, "not in the range"),
        ("11-concluded", r#"{"command":"policy","from":948,"conflict_look":"on"}"#, "concluded"),
        ("12-no-switch", r#"{"command":"policy","from":950}"#, "needs conflict_branching, conflict_look or gate"),
        ("13-bad-value", r#"{"command":"policy","from":950,"conflict_look":"maybe"}"#, "conflict_look is"),
        ("13-bad-gate", r#"{"command":"policy","from":950,"gate":4001}"#, "gate is a whole number from 0 to 4000"),
        (
            "13-gate-off",
            r#"{"command":"policy","from":950,"conflict_branching":"off","gate":150}"#,
            "gate, the gap from which conflict branching chooses, and turns it off",
        ),
        ("14-no-from", r#"{"command":"policy","conflict_look":"on"}"#, "needs from"),
        ("15-no-game", r#"{"command":"policy","n":950,"conflict_look":"on"}"#, "takes no field \"n\""),
    ];
    for (name, text, why) in refused {
        let outcome = send(lab, name, text);
        let (status, reason) = said(&outcome);
        assert_eq!(status, "refused", "{name}: {outcome}");
        assert!(reason.contains(why), "{name}: {reason}");
    }
    // A setting of the game's own comes first, and holds over the policy that follows it.
    let own = send(lab, "20-own", &format!(r#"{{"command":"branching","n":{P},"conflict_branching":"on"}}"#));
    assert_eq!(said(&own).0, "applied", "{own}");
    let both = format!(r#"{{"command":"policy","from":{P},"conflict_branching":"off","conflict_look":"on"}}"#);
    let outcome = send(lab, "21-policy", &both);
    assert_eq!(said(&outcome).0, "applied", "{outcome}");
    // It reaches the game that is set up, at once, and says so.
    assert!(said(&outcome).1.contains("[950"), "{outcome}");
    // A gate comes in a policy of its own, since the policy above turns the branching off: it is kept while the
    // branching is off.
    let gate = format!(r#"{{"command":"policy","from":{P},"gate":150}}"#);
    let outcome = send(lab, "21-policy-gate", &gate);
    assert_eq!(said(&outcome).0, "applied", "{outcome}");
    assert!(said(&outcome).1.contains("gate 150"), "{outcome}");
    // A policy that leaves a setting out keeps what the one before it said; then a setting of the game's own, which
    // comes after the policy and still holds over it, the gate among them.
    wait_until_set_up(lab, Q, "22");
    let look = send(lab, "23-policy-look", &format!(r#"{{"command":"policy","from":{Q},"conflict_look":"off"}}"#));
    assert_eq!(said(&look).0, "applied", "{look}");
    let own = send(lab, "24-own", &format!(r#"{{"command":"branching","n":{Q},"conflict_look":"on","gate":500}}"#));
    assert_eq!(said(&own).0, "applied", "{own}");
    for (name, n) in [("30-let-go-p", P), ("31-let-go-q", Q)] {
        let outcome = send(lab, name, &format!(r#"{{"command":"threads","n":{n},"workers":null}}"#));
        assert_eq!(said(&outcome).0, "applied", "{name}: {outcome}");
    }
}

/// A policy given for the games to come reaches the games in flight at once and the games set up later when they are,
/// holds under a `branching` of a game's own whichever comes first, leaves a setting out to keep the one before it, and
/// is in the line of each game it reached.  `P` and `Q` are held at no worker, set up, until the lab has given its
/// policies; `Q` may be set up before or after the first of them, which its line cannot tell.
#[test]
fn a_policy_reaches_the_games_in_flight_and_those_set_up_later() {
    let dir = TempDir::new("lab-policy");
    let (lab, out) = (dir.join("lab"), dir.join("r.jsonl"));
    fs::create_dir_all(lab.join("in")).unwrap();
    for (name, n) in [("00-hold-p", P), ("00-hold-q", Q)] {
        fs::write(lab.join("in").join(name), format!(r#"{{"command":"threads","n":{n},"workers":0}}"#)).unwrap();
    }
    let opts = SolveOptions {
        use_prev: true,
        checkpoint_dir: Some(dir.join("ckpt")),
        checkpoint_interval: Duration::from_secs(600),
        progress: Some(Duration::from_millis(5)),
        lab: LabOptions { dir: Some(lab.clone()), ..LabOptions::default() },
        ..options(2)
    };
    std::thread::scope(|scope| {
        let lab = scope.spawn(|| give_policies(&lab, &out));
        solver::solve_range_each(LO, HI, &opts, Some(&out), |_| {}).unwrap();
        lab.join().unwrap();
    });
    let optimal = Optimal::built_in();
    let lines = lines(&out);
    for line in &lines {
        let n = line.get("n").and_then(Json::as_u32).unwrap();
        assert_eq!(line.get("score").and_then(Json::as_u64), optimal.score(n), "n={n}");
    }
    let line = |n: u32| &lines[(n - LO) as usize];
    let record = |n: u32| line(n).get("lab").unwrap_or_else(|| panic!("n={n} has no lab"));
    // No policy reaches the game before the first: it ran as the run's flags say, with nothing of a lab.
    assert_eq!(line(LO).get("lab"), None);
    assert_eq!(ended_with(line(LO)), (Some(true), Some(false)));
    // `P`: the policy says branching off and the look on; its own setting of the branching holds over it.
    let policy = record(P).get("policy").unwrap();
    assert_eq!(policy.get("from").and_then(Json::as_u32), Some(P));
    assert_eq!(setting_pair(policy), (Some("off"), Some("on")));
    assert_eq!(setting_pair(record(P)), (Some("on"), None));
    assert_eq!(ended_with(line(P)), (Some(true), Some(true)));
    // The gate is the second policy's, which left the switches out and kept the first policy's; the game set none.
    assert_eq!(gate(policy), Some(150));
    assert_eq!(gate(record(P)), None);
    // `Q`: the second policy left the branching out, so it is off as the first said; the look is off by the second, and
    // on by the game's own setting after it.
    let policy = record(Q).get("policy").unwrap();
    assert_eq!(policy.get("from").and_then(Json::as_u32), Some(Q));
    assert_eq!(setting_pair(policy), (Some("off"), Some("off")));
    assert_eq!(setting_pair(record(Q)), (None, Some("on")));
    assert_eq!(ended_with(line(Q)), (Some(false), Some(true)));
    // The policy gate reaches `Q` through the policy before; the game's own gate, given while its branching was off,
    // was kept.
    assert_eq!(gate(policy), Some(150));
    assert_eq!(gate(record(Q)), Some(500));
    // The policies, as the lab gave them, are in the file beside the checkpoints.
    let file = Json::from_file(dir.join("ckpt/run.lab")).unwrap();
    let entries = file.get("policies").and_then(Json::as_arr).unwrap();
    assert_eq!(entries.len(), 3);
    let given = [
        (P, "21-policy", (Some("off"), Some("on")), None),
        (P, "21-policy-gate", (None, None), Some(150)),
        (Q, "23-policy-look", (None, Some("off")), None),
    ];
    for (entry, (from, name, switches, gate_given)) in entries.iter().zip(given) {
        assert_eq!(entry.get("from").and_then(Json::as_u32), Some(from));
        assert_eq!(setting(entry, "file"), Some(name));
        assert_eq!(setting_pair(entry), switches);
        assert_eq!(gate(entry), gate_given);
        assert!(setting(entry, "at").is_some_and(|at| at.ends_with('Z')), "{entry}");
    }
    verify_files(&[PathBuf::from(&out)], optimal, 2).unwrap_or_else(|e| panic!("{e}"));
}

/// The policies outlive the run: a run that the lab ends to be redeployed, with its policies given and no game
/// concluded, leaves them in `run.lab`, and the run that starts again sets the games up under them, with no lab
/// folder to say so.
#[test]
fn a_policy_survives_a_restart_of_the_run() {
    let dir = TempDir::new("lab-policy-restart");
    let (lab, out) = (dir.join("lab"), dir.join("r.jsonl"));
    fs::create_dir_all(lab.join("in")).unwrap();
    // `LO` is held at no worker, so that no game after it is set up when the policies come.
    let commands = [
        ("00-hold", format!(r#"{{"command":"threads","n":{LO},"workers":0}}"#)),
        ("01-policy", format!(r#"{{"command":"policy","from":{P},"conflict_branching":"off"}}"#)),
        ("02-policy", format!(r#"{{"command":"policy","from":{P},"conflict_look":"on"}}"#)),
        ("02-policy-gate", format!(r#"{{"command":"policy","from":{P},"gate":600}}"#)),
        ("03-redeploy", r#"{"command":"redeploy","commit":"0123456789abcdef0123456789abcdef01234567"}"#.to_owned()),
    ];
    for (name, text) in &commands {
        fs::write(lab.join("in").join(name), text).unwrap();
    }
    let opts = SolveOptions {
        use_prev: true,
        checkpoint_dir: Some(dir.join("ckpt")),
        checkpoint_interval: Duration::from_secs(600),
        progress: Some(Duration::from_millis(5)),
        lab: LabOptions { dir: Some(lab.clone()), ..LabOptions::default() },
        window: 1,
        ..options(2)
    };
    let stopped = solver::solve_range_each(LO, HI, &opts, Some(&out), |_| {}).unwrap();
    assert!(stopped.redeploy.is_some());
    for (name, _) in &commands {
        let outcome = Json::from_file(lab.join("out").join(name)).unwrap();
        assert_eq!(said(&outcome).0, "applied", "{name}: {outcome}");
    }
    let file = Json::from_file(dir.join("ckpt/run.lab")).unwrap();
    assert_eq!(file.get("policies").and_then(Json::as_arr).map(<[_]>::len), Some(3));

    // The run starts again, without a lab: the games set up now are set up under the policies.
    let again = SolveOptions { lab: LabOptions::default(), ..opts };
    let summary = solver::solve_range_each(LO, HI, &again, Some(&out), |_| {}).unwrap();
    assert_eq!(summary.redeploy, None);
    let optimal = Optimal::built_in();
    let lines = lines(&out);
    let ns: Vec<u32> = lines.iter().filter_map(|line| line.get("n").and_then(Json::as_u32)).collect();
    assert_eq!(ns, (LO..=HI).collect::<Vec<_>>());
    for line in &lines {
        let n = line.get("n").and_then(Json::as_u32).unwrap();
        assert_eq!(line.get("score").and_then(Json::as_u64), optimal.score(n), "n={n}");
        if n < P {
            assert_eq!((line.get("lab"), ended_with(line)), (None, (Some(true), Some(false))), "n={n}");
        } else if n == P || n == Q {
            let policy = line.get("lab").and_then(|lab| lab.get("policy")).unwrap();
            assert_eq!(policy.get("from").and_then(Json::as_u32), Some(P), "n={n}");
            assert_eq!(setting_pair(policy), (Some("off"), Some("on")), "n={n}");
            assert_eq!(gate(policy), Some(600), "n={n}");
            assert_eq!(ended_with(line), (Some(false), Some(true)), "n={n}");
        }
    }
    verify_files(&[PathBuf::from(&out)], optimal, 2).unwrap_or_else(|e| panic!("{e}"));
}

/// A checkpoint resumes under either setting of `conflict_branching` and of `conflict_look` (bits 9 and 10 of its
/// tactics bind nothing), so a policy given between the pieces of a search reaches the piece that resumes it.  Game 1000,
/// whose default search takes more than the 1,024 steps after which a node limit stops it, is stopped again and again
/// and resumed: the first piece under the run's flags, each piece after it under a policy that differs from the one
/// before it in both settings.  It finishes with its score in `optimal.json`.
#[test]
fn a_policy_reaches_a_search_resumed_under_the_other_settings() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("lab-policy-resume");
    let ckpt = dir.join("ckpt");
    let opts =
        SolveOptions { checkpoint_dir: Some(ckpt.clone()), node_limit: Some(1), force_search: true, ..options(1) };
    // The first piece is the run's flags, the settings the checkpoint is made under.  The gate is no part of a
    // checkpoint, so it too differs from one piece to the next.
    let settings = [("on", "off"), ("off", "on"), ("on", "on"), ("off", "off")];
    let gates = [300, 150, 4000, 600];
    let mut pieces = 0;
    let result = loop {
        let (branching, look) = settings[pieces % settings.len()];
        let gate_given = gates[pieces % gates.len()];
        if pieces > 0 {
            // The file as the command writes it.
            let entry = format!(
                r#"{{"from":1000,"at":"2026-10-22T00:00:00Z","file":"by-hand","conflict_branching":"{branching}","#
            ) + &format!(r#""conflict_look":"{look}","gate":{gate_given}}}"#);
            fs::create_dir_all(&ckpt).unwrap();
            fs::write(ckpt.join("run.lab"), format!(r#"{{"policies":[{entry}]}}"#)).unwrap();
        }
        let r = solver::solve_game_with(1000, None, &opts, optimal).unwrap_or_else(|e| panic!("piece {pieces}: {e}"));
        let ran = &r.tactics.search;
        assert_eq!((ran.conflict_branching, ran.conflict_look), (branching == "on", look == "on"), "piece {pieces}");
        let policy = r.lab.as_ref().and_then(|lab| lab.get("policy")).filter(|policy| **policy != Json::Null);
        if pieces == 0 {
            assert_eq!(policy, None);
        } else {
            let policy = policy.unwrap_or_else(|| panic!("piece {pieces} has no policy in its line: {:?}", r.lab));
            assert_eq!(policy.get("from").and_then(Json::as_u32), Some(1000), "piece {pieces}");
            assert_eq!(setting_pair(policy), (Some(branching), Some(look)), "piece {pieces}");
            assert_eq!(gate(policy), Some(gate_given), "piece {pieces}");
        }
        pieces += 1;
        if r.completed {
            break r;
        }
        assert!(ckpt.join("n1000.ckpt").exists(), "an unfinished game keeps its checkpoint");
        assert!(pieces < 50, "no progress");
    };
    assert!(pieces >= 2, "the policy never reached a resumed search");
    assert_eq!((result.score, result.optimal_match), (optimal.score(1000).unwrap(), Some(true)));
}

/// The gate of a policy reaches the searchers and changes the tree of a game, and not its answer.  Game 1000, searched
/// from nothing on one thread, is solved with no policy and under a policy whose gate is 0, so that conflict branching
/// chooses at every check, not only where the gap passes 300.  Each ends with its score in `optimal.json`, the trees
/// differ, and the line of the second says which gate the policy gave.
#[test]
fn a_policys_gate_changes_the_tree_of_a_game_and_not_its_answer() {
    let optimal = Optimal::built_in();
    let solved = |tag: &str, policy: Option<&str>| {
        let dir = TempDir::new(tag);
        let ckpt = dir.join("ckpt");
        if let Some(policy) = policy {
            fs::create_dir_all(&ckpt).unwrap();
            let entry = format!(r#"{{"from":1000,"at":"2026-10-22T00:00:00Z","file":"by-hand",{policy}}}"#);
            fs::write(ckpt.join("run.lab"), format!(r#"{{"policies":[{entry}]}}"#)).unwrap();
        }
        let opts = SolveOptions { checkpoint_dir: Some(ckpt), force_search: true, ..options(1) };
        let r = solver::solve_game_with(1000, None, &opts, optimal).unwrap_or_else(|e| panic!("{policy:?}: {e}"));
        let answer = (r.score, r.completed, r.optimal_match);
        assert_eq!(answer, (optimal.score(1000).unwrap(), true, Some(true)), "{policy:?}");
        let given = r.lab.as_ref().and_then(|lab| lab.get("policy")).and_then(|policy| policy.get("gate"));
        (r.nodes, given.and_then(Json::as_u64))
    };
    let (default, given) = solved("lab-gate-none", None);
    assert_eq!(given, None);
    let (gated, given) = solved("lab-gate-0", Some(r#""gate":0"#));
    assert_eq!(given, Some(0));
    assert_ne!(gated, default, "a gate of 0 took the steps of the default gate");
}
