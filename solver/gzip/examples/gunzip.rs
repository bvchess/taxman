//! Decodes a gzip file of any number of members to standard output, and says on standard error how long the decoding
//! took.  Usage: `cargo run --release --example gunzip FILE.gz > FILE`.

use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: gunzip FILE.gz > FILE");
        return ExitCode::from(2);
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let started = Instant::now();
    let decoded = match taxman_gzip::decode(&bytes) {
        Ok(decoded) => decoded,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("{path}: {} bytes decoded to {} in {:.3} s", bytes.len(), decoded.len(), started.elapsed().as_secs_f64());
    match std::io::stdout().lock().write_all(&decoded) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("standard output: {e}");
            ExitCode::FAILURE
        }
    }
}
