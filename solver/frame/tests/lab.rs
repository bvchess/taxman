//! The lab channel (`SolveOptions::lab`: `--lab-dir`, `--hook`; solver/docs/design/lab.md): a hostile lab changes no
//! result.  A range with a lab folder and a hook is fed offers of sets that are off the board, given twice, not
//! promotable or claimed above their worth, commands that are not commands, restarts with tactics a restart does not
//! take, threads it cannot have, `conflict_branching` and `conflict_look` switched at every tick, a restart of a game in
//! the middle of its search, and a redeploy at a commit that is not on master (the solver does not judge commits: it stops, and the chain's
//! script, which does, runs the build it has again, as the second run here does).  Every bad command is refused with
//! its reason, every game ends with the score of `optimal.json`, and the file passes the verifier.  The game restarted
//! records it, keeps its first checkpoint aside, and the hook hears of it twice, once when it runs long and once with
//! its line.  Small games of the default search, two threads.

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
/// the restart of `G` in the middle of its search, `conflict_branching` and `conflict_look` switched at every tick, and
/// a redeploy.  Returns the settings of the last switch that was applied while `G` searched, if one was.
fn hostile(lab: &Path, out: &Path) -> Option<(&'static str, &'static str)> {
    let _end = EndIfFailed(lab);
    // `G` waits, set up and held at no worker, until the lab lets it go.
    let since = Instant::now();
    for k in 0.. {
        let probe = send(lab, &format!("01-probe-{k:03}"), &format!(r#"{{"command":"offer","n":{G},"set":[]}}"#));
        if said(&probe).0 == "applied" {
            break;
        }
        assert!(since.elapsed() < PATIENCE, "game {G} is never set up: {probe}");
    }
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
        ("27-no-switch", format!(r#"{{"command":"branching","n":{G}}}"#), "needs conflict_branching or conflict_look"),
        ("28-look-concluded", format!(r#"{{"command":"branching","n":{LO},"conflict_look":"on"}}"#), "concluded"),
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
    // look is switched the other way from the branching.
    let mut last = None;
    for k in 0..6 {
        let (setting, look) = if k % 2 == 0 { ("on", "off") } else { ("off", "on") };
        let name = format!("50-switch-{k}");
        let command =
            format!(r#"{{"command":"branching","n":{G},"conflict_branching":"{setting}","conflict_look":"{look}"}}"#);
        let outcome = send(lab, &name, &command);
        let (status, reason) = said(&outcome);
        let later = ["set up", "ended", "concluded"].iter().any(|why| reason.contains(why));
        assert!(status == "applied" || later, "{name}: {outcome}");
        if status == "applied" {
            last = Some((setting, look));
        }
    }
    let outcome =
        send(lab, "60-redeploy", r#"{"command":"redeploy","commit":"0123456789abcdef0123456789abcdef01234567"}"#);
    assert_eq!(said(&outcome).0, "applied", "{outcome}");
    last
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
    let (summary, last_switch) = summary;
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
