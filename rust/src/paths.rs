//! Where the files the solver reads and writes by default are, found when the program runs.
//!
//! Nothing here is a path fixed at compile time, so a binary works wherever it is as long as it can find the
//! repository: the nearest directory above the working directory, and failing that above the directory of the
//! executable, that holds the table of optimal games, `src/main/resources/optimal.json`.  The
//! defaults are relative to that directory:
//!
//! | file | default | flag, then variable |
//! |---|---|---|
//! | the table of optimal games | `src/main/resources/optimal.json` | `--optimal FILE`, then `TAXMAN_OPTIMAL` |
//! | the benchmark history (`taxman bench`) | `rust/bench/history.jsonl` | `--history FILE` |
//! | the results `taxman bench` takes games past `optimal.json` from | `rust/results/strict.jsonl` | `--known FILE` |
//!
//! A flag wins over a variable, and a variable over the repository.  When the repository cannot be found the
//! error says where it looked and what to give instead.

use std::path::{Path, PathBuf};

/// The table of optimal games, relative to the repository; its presence is what makes a directory the repository.
pub const OPTIMAL_JSON_IN_REPOSITORY: &str = "src/main/resources/optimal.json";

/// The environment variable that names the table of optimal games, for programs that have no `--optimal` flag.
pub const OPTIMAL_VARIABLE: &str = "TAXMAN_OPTIMAL";

/// The benchmark history, relative to the repository.
pub const BENCH_HISTORY_IN_REPOSITORY: &str = "rust/bench/history.jsonl";

/// The committed strict results, relative to the repository.
pub const STRICT_RESULTS_IN_REPOSITORY: &str = "rust/results/strict.jsonl";

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

/// The repository checkout (the directory with `src/main/resources/optimal.json` in it) that the working directory
/// or the executable is in.
///
/// # Errors
///
/// Says where it looked, if it is in none.
pub fn repository_root() -> Result<PathBuf, String> {
    let starts = starting_points();
    find_above(&starts, OPTIMAL_JSON_IN_REPOSITORY).ok_or_else(|| {
        let looked: Vec<String> = starts.iter().map(|start| start.display().to_string()).collect();
        format!(
            "cannot find the repository: there is no {OPTIMAL_JSON_IN_REPOSITORY} in {}, or in any directory above",
            if looked.is_empty() { "the working directory".to_owned() } else { looked.join(" or in ") }
        )
    })
}

/// The table of optimal games: the file `$TAXMAN_OPTIMAL` names, else the one in the repository.
///
/// # Errors
///
/// If the variable names a file that is not there, or there is no repository (the message says to give
/// `--optimal FILE`, or `--optimal none` to compare with nothing, or to set the variable).
pub fn optimal_json() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os(OPTIMAL_VARIABLE).filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(format!("{OPTIMAL_VARIABLE} names {}, which is not a file", path.display()))
        };
    }
    let root = repository_root().map_err(|e| {
        format!(
            "{e}.  Run taxman in a checkout of the repository, or give --optimal FILE (--optimal none to compare \
             with nothing), or set {OPTIMAL_VARIABLE}"
        )
    })?;
    Ok(root.join(OPTIMAL_JSON_IN_REPOSITORY))
}

/// `relative` in the repository, for the defaults of `taxman bench`; `flag` is what to give instead of finding it.
///
/// # Errors
///
/// If there is no repository.
pub fn in_repository(relative: &str, flag: &str) -> Result<PathBuf, String> {
    repository_root().map(|root| root.join(relative)).map_err(|e| format!("{e}.  Give {flag} instead"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::fs;

    #[test]
    fn the_repository_is_the_nearest_directory_above_with_the_table() {
        let dir = TempDir::new("paths");
        let inner = dir.join("repo/rust/target/release");
        fs::create_dir_all(&inner).unwrap();
        fs::create_dir_all(dir.join("repo/src/main/resources")).unwrap();
        fs::write(dir.join("repo").join(OPTIMAL_JSON_IN_REPOSITORY), "[]").unwrap();
        fs::create_dir_all(dir.join("elsewhere")).unwrap();
        let elsewhere = dir.join("elsewhere");

        assert_eq!(find_above(std::slice::from_ref(&inner), OPTIMAL_JSON_IN_REPOSITORY), Some(dir.join("repo")));
        // The starting points are tried in order: the first that is in a repository decides.
        assert_eq!(find_above(&[elsewhere.clone(), inner], OPTIMAL_JSON_IN_REPOSITORY), Some(dir.join("repo")));
        assert_eq!(find_above(&[elsewhere], OPTIMAL_JSON_IN_REPOSITORY), None);
        assert_eq!(find_above(&[], OPTIMAL_JSON_IN_REPOSITORY), None);
    }

    #[test]
    fn this_checkout_is_found_from_the_working_directory() {
        // `cargo test` runs in the crate's directory, which is in the repository.
        let root = repository_root().expect("tests run in the repository");
        assert!(root.join(OPTIMAL_JSON_IN_REPOSITORY).is_file());
        assert!(in_repository(BENCH_HISTORY_IN_REPOSITORY, "--history FILE").unwrap().starts_with(&root));
    }
}
