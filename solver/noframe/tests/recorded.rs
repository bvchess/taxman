//! The recorded games read from result files, plain or gzip (`recorded_games`, `recorded_games_to`), on files of twelve
//! small games made here.

use std::path::{Path, PathBuf};
use taxman_gzip::crc32;
use taxman_prover::jsonl::{recorded_games, recorded_games_to};

/// The result lines of the games `first..first + 4`, each with a score and moves of its own.
fn lines(first: u64) -> String {
    (first..first + 4)
        .map(|n| {
            format!("{{\"schema\":5,\"n\":{n},\"score\":{},\"moves\":[{n},{}],\"completed\":true}}\n", 10 * n, n + 1)
        })
        .collect()
}

/// A gzip member that holds `text` in one stored (uncompressed) block of deflate.
fn member(text: &str) -> Vec<u8> {
    let (bytes, len) = (text.as_bytes(), text.len() as u16);
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3, 1];
    out.extend(len.to_le_bytes());
    out.extend((!len).to_le_bytes());
    out.extend(bytes);
    out.extend(crc32(bytes).to_le_bytes());
    out.extend((bytes.len() as u32).to_le_bytes());
    out
}

/// A new directory for the files of the test `tag`.
fn directory(tag: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("recorded_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make the directory");
    dir
}

/// The file `games.jsonl.gz` of the games 1..=12 in three members, the third with a byte of its text changed if
/// `damaged`.
fn gzip_file(dir: &Path, damaged: bool) -> PathBuf {
    let mut third = member(&lines(9));
    if damaged {
        third[20] ^= 1;
    }
    let path = dir.join("games.jsonl.gz");
    std::fs::write(&path, [member(&lines(1)), member(&lines(5)), third].concat()).expect("write the file");
    path
}

/// A gzip file gives the games of the plain file of the same lines, and an earlier file wins over a later one.
#[test]
fn a_gzip_file_and_a_plain_file_give_the_same_games() {
    let dir = directory("same");
    let gz = gzip_file(&dir, false);
    let plain = dir.join("games.jsonl");
    std::fs::write(&plain, [lines(1), lines(5), lines(9)].concat()).expect("write the file");
    let other = dir.join("other.jsonl");
    std::fs::write(&other, lines(5).replace("\"score\":70", "\"score\":71")).expect("write the file");
    let games = recorded_games(&[&gz]);
    assert_eq!(games.len(), 12);
    assert_eq!(games[&7], (70, vec![7, 8]));
    assert_eq!(games, recorded_games(&[&plain]));
    assert_eq!(recorded_games(&[&gz, &other])[&7].0, 70);
    assert_eq!(recorded_games(&[&other, &gz])[&7].0, 71);
    std::fs::remove_dir_all(&dir).expect("remove the directory");
}

/// The games up to `last` are read from the members that hold them and no more: a damaged member after them is not
/// decoded.  A damaged member is an error, never wrong games, when the games wanted reach it.
#[test]
fn a_gzip_file_is_decoded_only_as_far_as_the_last_game_wanted() {
    let dir = directory("damaged");
    let gz = gzip_file(&dir, true);
    assert_eq!(recorded_games_to(&[&gz], 6).keys().copied().collect::<Vec<_>>(), (1..=6).collect::<Vec<_>>());
    let broken = std::panic::catch_unwind(|| recorded_games_to(&[&gz], 9)).expect_err("a damaged member is read");
    let message = broken.downcast_ref::<String>().expect("a message");
    assert!(message.contains("games.jsonl.gz") && message.contains("member 2"), "{message}");
    std::fs::remove_dir_all(&dir).expect("remove the directory");
}
