//! Running the `taxman` binary, or one of the Python tools of `solver/tools/`, from an integration test, shared by the
//! tests of the command line (`tests/cli.rs`) and of the commands that have a library function behind them
//! (`tests/certify.rs`).

use std::path::Path;
use std::process::Command;

/// What a run of a program did.
pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Runs the binary with `args` and waits for it.
pub fn taxman(args: &[&str]) -> Run {
    run(Command::new(env!("CARGO_BIN_EXE_taxman")).args(args))
}

/// Runs the Python script `script` with `args` and waits for it, as `python3 -I`: isolated mode puts neither the
/// script's directory nor the current one on the module path, so a stray module there cannot be imported by mistake.
pub fn python(script: &Path, args: &[&str]) -> Run {
    run(Command::new("python3").arg("-I").arg(script).args(args))
}

fn run(command: &mut Command) -> Run {
    let out = command.output().unwrap_or_else(|e| panic!("run {:?}: {e}", command.get_program()));
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
