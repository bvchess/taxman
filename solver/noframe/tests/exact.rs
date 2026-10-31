//! The `exact` binary's modes: every line records whether the search started from the roles of F4
//! (docs/theory/frame-model.md) (`fix_low`, on by default), and the roles change what is searched from, not what is
//! proven or which games are searched.  Boards 187..=300 (their frames of layer 2 are closed, and the games that need a
//! search take a second).  And its relations: a game that meets the semiprime formula (J3, docs/theory/arrival.md) is
//! named and not searched.  And its stop: a SIGTERM pauses every search into its checkpoint, and a rerun finishes them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use taxman_prover::jsonl::{committed_chain, field_str, field_u64, recorded_games_to};

extern "C" {
    fn kill(pid: i32, signum: i32) -> i32;
}

const SIGTERM: i32 = 15;

/// `exact` over the games `games` (flags that name them, such as `--from 187 --to 300`) with `args` more, its lines by
/// n.
fn lines_of_games(tag: &str, games: &[&str], args: &[&str]) -> BTreeMap<u64, String> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("exact_{tag}_{}", std::process::id()));
    let out = dir.join("out.jsonl");
    let status = Command::new(env!("CARGO_BIN_EXE_exact"))
        .args(games)
        .args(["--threads", "2", "--checkpoints"])
        .arg(dir.join("checkpoints"))
        .arg("--out")
        .arg(&out)
        .args(args)
        .output()
        .expect("run exact");
    assert!(status.status.success(), "exact {args:?}: {}", String::from_utf8_lossy(&status.stderr));
    let text = std::fs::read_to_string(&out).expect("the output file");
    let lines: BTreeMap<u64, String> =
        text.lines().map(|line| (field_u64(line, "n").expect("a line has its n"), line.to_owned())).collect();
    std::fs::remove_dir_all(&dir).expect("remove the output directory");
    lines
}

/// `exact` over 187..=300 with `args` more, its lines by n.
fn lines_of(tag: &str, args: &[&str]) -> BTreeMap<u64, String> {
    let lines = lines_of_games(tag, &["--from", "187", "--to", "300"], args);
    assert_eq!(lines.len(), 300 - 187 + 1, "a line for every game");
    lines
}

/// The default starts from the roles of F4 (docs/theory/frame-model.md) and says so on every line; `--fix-low 0` is the
/// plain root and says so; the games searched and their verdicts are the same, and a game that was searched has its
/// `fixed_bound` (the parity bound at the roles) at least the score.  Every line counts its threads.
#[test]
fn every_line_records_whether_the_roles_of_theorem_b_were_used() {
    let by_default = lines_of("default", &[]);
    let plain = lines_of("plain", &["--fix-low", "0"]);
    let mut searched = 0;
    for (n, line) in &by_default {
        assert!(matches!(field_u64(line, "threads"), Some(1 | 2)), "n={n}: {line}");
        assert!(field_u64(line, "thread_secs").is_some(), "n={n}: {line}");
        assert_eq!(field_u64(line, "fix_low"), Some(1), "n={n}: {line}");
        let bound = field_u64(line, "fixed_bound").unwrap_or_else(|| panic!("n={n}: no fixed_bound: {line}"));
        assert!(bound >= field_u64(line, "score").unwrap(), "n={n}: the bound at the roles is below the score");
        let plain_line = &plain[n];
        assert_eq!(field_u64(plain_line, "fix_low"), Some(0), "n={n}: {plain_line}");
        assert_eq!(field_u64(plain_line, "fixed_bound"), None, "n={n}: {plain_line}");
        assert_eq!(field_str(line, "search"), field_str(plain_line, "search"), "n={n}");
        for key in ["score", "fm_bound", "parity_bound"] {
            assert_eq!(field_u64(line, key), field_u64(plain_line, key), "n={n}: {key}");
        }
        searched += usize::from(field_str(line, "search") == Some("proven"));
    }
    assert!(searched > 0, "no game of 187..=300 needs a search");
}

/// `--fix-upper 1` fixes the upper part too: the games that were proven are `upper_fixed` and no proof, and its lines
/// say so.
#[test]
fn fixing_the_upper_part_is_a_measurement() {
    let proofs = lines_of("proofs", &[]);
    let fixed = lines_of("fixed", &["--fix-upper", "1"]);
    for (n, line) in &fixed {
        assert_eq!(field_u64(line, "fix_low"), Some(1), "n={n}: {line}");
        assert_eq!(field_u64(line, "fix_upper"), Some(1), "n={n}: {line}");
        let expected = match field_str(&proofs[n], "search") {
            Some("proven") => "upper_fixed",
            other => other.expect("a search"),
        };
        assert_eq!(field_str(line, "search"), Some(expected), "n={n}: {line}");
    }
}

/// A game that meets the semiprime formula (J3, docs/theory/arrival.md) says so (`"relation":"semiprime"`) and is not
/// searched; with `--all 1` it is searched when its bounds at the root leave a gap, and the line still names the
/// relation.  Games 39 = 3·13, 51 = 3·17, 203 = 7·29, 247 = 13·19 and 253 = 11·23 close a cycle with U\*_2 of the game
/// before; 21 = 3·7 closes none, and 33 = 3·11 is at its ceiling.  The parity bound at the root of 247 and of 253 is
/// above the score, so `--all 1` searches them.
#[test]
fn a_semiprime_game_is_named_and_not_searched() {
    let games = ["--list", "21,33,39,51,203,247,253"];
    let by_default = lines_of_games("semiprime", &games, &[]);
    let searched = lines_of_games("semiprime_all", &games, &["--all", "1"]);
    assert_eq!(by_default.len(), 7);
    for n in [39, 51, 203, 247, 253] {
        let line = &by_default[&n];
        assert_eq!(field_str(line, "relation"), Some("semiprime"), "n={n}: {line}");
        assert_eq!(field_str(line, "search"), Some("skipped"), "n={n}: {line}");
        assert_eq!(field_u64(line, "nodes"), Some(0), "n={n}: {line}");
    }
    for n in [247, 253] {
        let line = &searched[&n];
        assert_eq!(field_str(line, "relation"), Some("semiprime"), "n={n}: {line}");
        assert_eq!(field_str(line, "search"), Some("proven"), "n={n}: {line}");
    }
    assert_eq!(field_str(&by_default[&21], "relation"), Some("none"));
    assert_eq!(field_str(&by_default[&33], "relation"), Some("chain"));
}

/// The complete lines of the output file (the last line of a file being appended may be torn), by n.
fn written(out: &Path) -> BTreeMap<u64, String> {
    let text = std::fs::read_to_string(out).unwrap_or_default();
    let complete = &text[..text.rfind('\n').map_or(0, |i| i + 1)];
    complete.lines().map(|line| (field_u64(line, "n").expect("a line has its n"), line.to_owned())).collect()
}

/// The checkpoint of game n's search from the roles of F4, if the run left one.
fn checkpoint_text(checkpoints: &Path, n: u64) -> Option<String> {
    std::fs::read_to_string(checkpoints.join(format!("exact_{n}_low.ckpt"))).ok()
}

/// A SIGTERM during a run pauses every search in flight into its checkpoint, writes the lines of the games finished, and
/// ends `exact` with status 143 (128 + the signal); a rerun resumes the searches from their checkpoints, and the games
/// come out proven at the recorded scores, one line each.  Game 576 takes 0.2 s of CPU in this profile, and 620 and 624
/// about 0.4 s each, on a thread each: when the line of 576 appears, the others have searched for a tenth of a second
/// and have 10 to 25 open subtrees (the more a pause leaves open, the more a resume costs, since each starts cold).
#[test]
fn a_sigterm_pauses_every_search_into_its_checkpoint_and_a_rerun_finishes_them() {
    let games: [u64; 3] = [576, 620, 624];
    let list = games.map(|n| n.to_string()).join(",");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("exact_stop_{}", std::process::id()));
    let (out, checkpoints) = (dir.join("out.jsonl"), dir.join("checkpoints"));
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_exact"));
        command.args(["--list", &list, "--threads", "3", "--checkpoints"]).arg(&checkpoints);
        command.arg("--out").arg(&out);
        command
    };
    let mut child = command().stderr(Stdio::piped()).spawn().expect("run exact");
    let waiting = Instant::now();
    while written(&out).is_empty() {
        assert!(child.try_wait().unwrap().is_none(), "exact ended before it wrote a line");
        assert!(waiting.elapsed() < Duration::from_secs(60), "exact wrote no line in a minute");
        std::thread::sleep(Duration::from_millis(5));
    }
    // SAFETY: `kill` has no preconditions; the child is still ours to reap.
    assert_eq!(unsafe { kill(child.id() as i32, SIGTERM) }, 0, "send the SIGTERM");
    let stopped = child.wait_with_output().expect("exact stops");
    let stderr = String::from_utf8_lossy(&stopped.stderr);
    assert_eq!(stopped.status.code(), Some(128 + SIGTERM), "exact exits with 128 + the signal: {stderr}");
    assert!(stderr.contains("stopped by signal 15"), "{stderr}");

    // Each game has a line or a checkpoint with open units, and none has both.
    let finished = written(&out);
    let mut nodes_at_stop = BTreeMap::new();
    for n in games {
        let checkpoint = checkpoint_text(&checkpoints, n);
        match (finished.contains_key(&n), checkpoint) {
            (true, None) => {}
            (true, Some(_)) => panic!("n={n}: a line and a checkpoint"),
            (false, None) => panic!("n={n}: neither a line nor a checkpoint: {stderr}"),
            (false, Some(text)) => {
                assert!(text.lines().any(|line| line.starts_with("unit ")), "n={n}: no open unit in\n{text}");
                assert!(stderr.contains(&format!("n={n} not finished: paused")), "n={n}: {stderr}");
                let nodes = text.lines().find_map(|line| line.strip_prefix("nodes ")).expect("the nodes so far");
                nodes_at_stop.insert(n, nodes.parse::<u64>().unwrap());
            }
        }
    }
    assert!(!nodes_at_stop.is_empty(), "every game finished before the SIGTERM took effect: {stderr}");
    assert!(nodes_at_stop.values().any(|&nodes| nodes > 0), "the SIGTERM came before any search began: {stderr}");

    // The rerun finishes the paused games from their checkpoints.
    let resumed = command().output().expect("run exact again");
    assert!(resumed.status.success(), "{}", String::from_utf8_lossy(&resumed.stderr));
    let text = std::fs::read_to_string(&out).unwrap();
    assert_eq!(text.lines().count(), games.len(), "a line for every game, none twice:\n{text}");
    let lines = written(&out);
    let recorded = recorded_games_to(&[committed_chain()], games.into_iter().max().unwrap());
    for n in games {
        let line = &lines[&n];
        assert_eq!(field_str(line, "search"), Some("proven"), "n={n}: {line}");
        assert_eq!(field_u64(line, "score"), Some(recorded[&n].0), "n={n}: the recorded optimum");
        if let Some(at_stop) = nodes_at_stop.get(&n) {
            assert!(field_u64(line, "nodes").unwrap() > *at_stop, "n={n}: the search resumed: {line}");
        } else {
            assert_eq!(Some(line), finished.get(&n), "n={n}: a finished game is not searched again");
        }
        assert!(checkpoint_text(&checkpoints, n).is_none(), "n={n}: a finished game keeps no checkpoint");
    }
    std::fs::remove_dir_all(&dir).expect("remove the output directory");
}
