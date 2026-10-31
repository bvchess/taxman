//! The `exact` binary's modes: every line records whether the search started from the roles of F4
//! (docs/theory/frame-model.md) (`fix_low`, on by default), and the roles change what is searched from, not what is
//! proven or which games are searched.  Boards 187..=300 (their frames of layer 2 are closed, and the games that need a
//! search take a second).  And its relations: a game that meets the semiprime formula (J3, docs/theory/arrival.md) is
//! named and not searched.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use taxman_prover::jsonl::{field_str, field_u64};

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
/// `fixed_bound` (the parity bound at the roles) at least the score.
#[test]
fn every_line_records_whether_the_roles_of_theorem_b_were_used() {
    let by_default = lines_of("default", &[]);
    let plain = lines_of("plain", &["--fix-low", "0"]);
    let mut searched = 0;
    for (n, line) in &by_default {
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
