//! Records the commit the binary is built from, for the provenance of every result line (`results::build_json`).
//!
//! * `TAXMAN_GIT_COMMIT`: the hash of `HEAD`; empty outside a git checkout or without `git`.
//! * `TAXMAN_GIT_DIRTY`: `true` if the sources the binary is built from ([`SOURCES`]) differ from that commit
//!   (changed, staged or untracked files), `false` if not, empty when unknown.
//!
//! The script runs again when a source changes and when `HEAD` moves (a commit or a checkout changes the commit
//! without touching the sources).

use std::path::{Path, PathBuf};
use std::process::Command;

/// The files the binary is compiled from, relative to the crate.
const SOURCES: [&str; 4] = ["src", "build.rs", "Cargo.toml", "Cargo.lock"];

/// The output of `git -C dir args...`, if it runs and succeeds.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    let crate_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    for source in SOURCES {
        println!("cargo:rerun-if-changed={source}");
    }
    let commit = git(&crate_dir, &["rev-parse", "HEAD"]);
    let dirty = commit.as_ref().and_then(|_| {
        // `--no-optional-locks`: do not refresh the index, which would itself look like a change to watch.
        let mut args = vec!["--no-optional-locks", "status", "--porcelain", "--"];
        args.extend(SOURCES);
        git(&crate_dir, &args).map(|status| !status.is_empty())
    });

    // What `HEAD` depends on: the HEAD file itself, and the branch it names (loose or packed).
    let absolute = |text: String| {
        let path = PathBuf::from(text);
        if path.is_absolute() {
            path
        } else {
            crate_dir.join(path)
        }
    };
    let git_dir = git(&crate_dir, &["rev-parse", "--git-dir"]).map(absolute);
    let common_dir = git(&crate_dir, &["rev-parse", "--git-common-dir"]).map(absolute);
    let mut watched: Vec<PathBuf> = Vec::new();
    if let Some(dir) = &git_dir {
        watched.push(dir.join("HEAD"));
    }
    if let Some(dir) = &common_dir {
        watched.push(dir.join("packed-refs"));
        if let Some(branch) = git(&crate_dir, &["symbolic-ref", "-q", "HEAD"]) {
            watched.push(dir.join(branch));
        }
    }
    // A path that does not exist would make cargo run the script on every build.
    for path in watched.iter().filter(|path| path.exists()) {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    println!("cargo:rustc-env=TAXMAN_GIT_COMMIT={}", commit.unwrap_or_default());
    println!("cargo:rustc-env=TAXMAN_GIT_DIRTY={}", dirty.map_or_else(String::new, |dirty| dirty.to_string()));
}
