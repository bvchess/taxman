//! Running the `taxman` binary from an integration test, shared by the tests of the command line (`tests/cli.rs`) and
//! of the commands that have a library function behind them (`tests/certify.rs`).

use std::path::Path;
use std::process::Command;

/// What a run of the binary did.
pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Runs the binary with `args` and waits for it.
pub fn taxman(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_taxman")).args(args).output().expect("run taxman");
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A path as the `&str` a command line takes.
pub fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}
