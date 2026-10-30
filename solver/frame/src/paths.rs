//! Where the files the commands read by default are, found when the program runs.
//!
//! Nothing here is a path fixed at compile time, so a binary works wherever it is as long as it can find the
//! repository: the nearest directory above the working directory, and failing that above the directory of the
//! executable, that holds the committed results, `results/optimal.jsonl.gz`.  (The solver itself needs no
//! file: the table of optimal games is built into the binary, [`crate::game::Optimal::built_in`].)  The defaults are
//! relative to that directory:
//!
//! | file | default | flag |
//! |---|---|---|
//! | the prover's proofs (`verify-certificates`) | `solver/noframe/data/exact_*.jsonl` | `--exact FILE...` |
//!
//! A flag wins over the repository.  When the repository cannot be found the error says where it looked and what to
//! give instead.

use std::fs;
use std::path::{Path, PathBuf};

/// The committed results, the chain of games from n = 1 (gzip), relative to the repository; its presence is what makes a
/// directory the repository.
pub const RESULTS_IN_REPOSITORY: &str = "results/optimal.jsonl.gz";

/// The bounds store, the bounds on opt(n) that the ledger finds, relative to the repository.
pub const BOUNDS_STORE_IN_REPOSITORY: &str = "results/bounds.jsonl";

/// The prover's data (the barred bounds, the proofs of the scores), relative to the repository.
pub const PROVER_DATA_IN_REPOSITORY: &str = "solver/noframe/data";

/// The nearest directory at or above one of `starts`, tried in order, that contains `marker`.
fn find_above(starts: &[PathBuf], marker: &str) -> Option<PathBuf> {
    starts.iter().find_map(|start| start.ancestors().find(|dir| dir.join(marker).is_file())).map(Path::to_path_buf)
}

/// Where to look for the repository: the working directory, then the directory of the executable.
fn starting_points() -> Vec<PathBuf> {
    let exe = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf));
    let mut starts: Vec<PathBuf> = std::env::current_dir().ok().into_iter().chain(exe).collect();
    starts.dedup();
    starts
}

/// The repository checkout (the directory with `results/optimal.jsonl.gz` in it) that the working directory
/// or the executable is in.
///
/// # Errors
///
/// Says where it looked, if it is in none.
pub fn repository_root() -> Result<PathBuf, String> {
    let starts = starting_points();
    find_above(&starts, RESULTS_IN_REPOSITORY).ok_or_else(|| {
        let looked: Vec<String> = starts.iter().map(|start| start.display().to_string()).collect();
        format!(
            "cannot find the repository: there is no {RESULTS_IN_REPOSITORY} in {}, or in any directory above",
            if looked.is_empty() { "the working directory".to_owned() } else { looked.join(" or in ") }
        )
    })
}

/// `relative` in the repository, for a default file; `flag` is what to give instead of finding it.
///
/// # Errors
///
/// If there is no repository.
pub fn in_repository(relative: &str, flag: &str) -> Result<PathBuf, String> {
    repository_root().map(|root| root.join(relative)).map_err(|e| format!("{e}.  Give {flag} instead"))
}

/// The prover's proofs of the scores, `exact_*.jsonl` in its data directory in the repository, ordered by name (a later
/// file's line for a game replaces an earlier file's: `certify::verify::read_exact`).
///
/// # Errors
///
/// If there is no repository, or its data directory cannot be read.
pub fn exact_files_in_repository() -> Result<Vec<PathBuf>, String> {
    let dir = in_repository(PROVER_DATA_IN_REPOSITORY, "--exact FILE")?;
    exact_files_in(&dir)
}

/// The files `exact_*.jsonl` of the directory `dir`, ordered by name.
fn exact_files_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let read = |e: std::io::Error| format!("{}: {e}", dir.display());
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(read)? {
        let path = entry.map_err(read)?.path();
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        if path.is_file() && name.starts_with("exact_") && name.ends_with(".jsonl") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn the_repository_is_the_nearest_directory_above_with_the_results() {
        let dir = TempDir::new("paths");
        let inner = dir.join("repo/solver/frame/target/release");
        fs::create_dir_all(&inner).unwrap();
        fs::create_dir_all(dir.join("repo/results")).unwrap();
        fs::write(dir.join("repo").join(RESULTS_IN_REPOSITORY), "").unwrap();
        fs::create_dir_all(dir.join("elsewhere")).unwrap();
        let elsewhere = dir.join("elsewhere");

        assert_eq!(find_above(std::slice::from_ref(&inner), RESULTS_IN_REPOSITORY), Some(dir.join("repo")));
        // The starting points are tried in order: the first that is in a repository decides.
        assert_eq!(find_above(&[elsewhere.clone(), inner], RESULTS_IN_REPOSITORY), Some(dir.join("repo")));
        assert_eq!(find_above(&[elsewhere], RESULTS_IN_REPOSITORY), None);
        assert_eq!(find_above(&[], RESULTS_IN_REPOSITORY), None);
    }

    #[test]
    fn the_exact_files_are_those_named_exact_ending_in_jsonl_by_name() {
        let dir = TempDir::new("paths-exact");
        for name in
            ["exact_parity_2_1300.jsonl", "exact_2_1231.jsonl", "upper_2_1300.jsonl", "exact_x.txt", "exact.jsonl"]
        {
            fs::write(dir.join(name), "").unwrap();
        }
        fs::create_dir(dir.join("exact_dir.jsonl")).unwrap();
        let names: Vec<_> = exact_files_in(&dir).unwrap().iter().map(|p| p.file_name().unwrap().to_owned()).collect();
        assert_eq!(names, ["exact_2_1231.jsonl", "exact_parity_2_1300.jsonl"]);
        assert!(exact_files_in(&dir.join("missing")).unwrap_err().contains("missing"));
    }

    #[test]
    fn this_checkout_is_found_from_the_working_directory() {
        // `cargo test` runs in the crate's directory, which is in the repository.
        let root = repository_root().expect("tests run in the repository");
        assert!(root.join(RESULTS_IN_REPOSITORY).is_file());
        assert!(in_repository(PROVER_DATA_IN_REPOSITORY, "--exact FILE").unwrap().starts_with(&root));
    }
}
