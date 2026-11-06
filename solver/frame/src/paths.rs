//! Where the files the commands read by default are, found when the program runs.
//!
//! Nothing here is a path fixed at compile time, so a binary works wherever it is as long as it can find the
//! repository: the nearest directory above the working directory, and failing that above the directory of the
//! executable, that holds the committed results, `results/optimal.jsonl.gz`.  (The solver itself needs no
//! file: the table of optimal moves is built into the binary, [`crate::game::Optimal::built_in`].)  When the repository
//! cannot be found the error says where it looked.

use std::path::{Path, PathBuf};

/// The committed results, the chain of lines from n = 1 (gzip), relative to the repository; its presence is what makes
/// a directory the repository.
pub const RESULTS_IN_REPOSITORY: &str = "results/optimal.jsonl.gz";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::fs;

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
    fn this_checkout_is_found_from_the_working_directory() {
        // `cargo test` runs in the crate's directory, which is in the repository.
        let root = repository_root().expect("tests run in the repository");
        assert!(root.join(RESULTS_IN_REPOSITORY).is_file());
    }
}
