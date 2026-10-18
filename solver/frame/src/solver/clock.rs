//! The wall clock of a game: how long its own search has run, kept between invocations (`--game-hours`; "The wall" in
//! [`window`](super::window)).
//!
//! The clock is a file beside the game's checkpoint, `n<NNNN>.clock`, holding the seconds the game has been the head of
//! the window with its search running, summed over every invocation that searched it.  The checkpoint holds the
//! search's own elapsed time too, but that counts the time a search waited behind an older game, suspended in the
//! window, and the wall is about the time the chain waited for the game; so the clock is the driver's and the
//! checkpoint's format is left as it is.  A clock is valid only beside the checkpoint it was kept with: a game that
//! restarts from its beginning (no checkpoint) starts its clock again, so deleting a checkpoint by hand needs no second
//! deletion.  A finished game's clock goes with its checkpoint ([`forget`], from `discard_checkpoint`).

use super::SolveOptions;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

/// The longest clock a file may claim (a hundred years of seconds); anything else is taken for garbage.
const MAX_SECONDS: f64 = 3.2e9;

/// Game `n`'s clock as its search starts: what the last invocation kept, if the game has a checkpoint to resume, else
/// zero (also for a file that does not say a number of seconds: the clock only bounds the run, and a lost reading
/// makes the wall late, not early).
pub(super) fn resume(opts: &SolveOptions, n: u32) -> Duration {
    let (Some(checkpoint), Some(clock)) = (opts.checkpoint_file(n), opts.clock_file(n)) else { return Duration::ZERO };
    if !checkpoint.exists() {
        return Duration::ZERO;
    }
    read(&clock)
}

/// The seconds in the file `path`, or zero.
fn read(path: &Path) -> Duration {
    let seconds = fs::read_to_string(path).ok().and_then(|text| text.trim().parse::<f64>().ok());
    match seconds {
        Some(seconds) if seconds.is_finite() && (0.0..=MAX_SECONDS).contains(&seconds) => {
            Duration::from_secs_f64(seconds)
        }
        _ => Duration::ZERO,
    }
}

/// Keeps `held` as game `n`'s clock, replacing the file whole (written aside and renamed, so a crash leaves the old
/// reading or the new, never half of one).  Nothing happens without a checkpoint directory.  A failure is reported on
/// stderr and the run goes on: the clock bounds the run and is not its result.
pub(super) fn keep(opts: &SolveOptions, n: u32, held: Duration) {
    let Some(path) = opts.clock_file(n) else { return };
    let aside = path.with_extension("clock.tmp");
    // The directory may not exist yet: the first checkpoint of a run is written after its first interval.
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::File::create(&aside))
        .and_then(|mut file| {
            writeln!(file, "{:.3}", held.as_secs_f64())?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&aside, &path));
    if let Err(e) = written {
        let _ = writeln!(std::io::stderr(), "warning: cannot keep the clock of n={n} in {}: {e}", path.display());
    }
}

/// Deletes game `n`'s clock (and a half-written one), if there is a checkpoint directory.
pub(super) fn forget(opts: &SolveOptions, n: u32) {
    let Some(path) = opts.clock_file(n) else { return };
    for file in [path.with_extension("clock.tmp"), path] {
        // Absent is the usual case (no wall was asked for); a file that stays only keeps a reading that `resume`
        // ignores without a checkpoint.
        let _ = fs::remove_file(file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn options(dir: &TempDir) -> SolveOptions {
        SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), ..SolveOptions::default() }
    }

    #[test]
    fn a_clock_is_kept_beside_its_checkpoint_and_read_back_from_it() {
        let dir = TempDir::new("clock");
        let opts = options(&dir);
        fs::create_dir_all(dir.join("ckpt")).unwrap();
        keep(&opts, 3441, Duration::from_millis(86_400_123));
        assert!(dir.join("ckpt/n3441.clock").exists() && !dir.join("ckpt/n3441.clock.tmp").exists());
        // No checkpoint to resume: the game starts over, and so does its clock.
        assert_eq!(resume(&opts, 3441), Duration::ZERO);
        fs::write(dir.join("ckpt/n3441.ckpt"), b"x").unwrap();
        assert_eq!(resume(&opts, 3441), Duration::from_millis(86_400_123));
        // Another game's clock is its own.
        assert_eq!(resume(&opts, 3442), Duration::ZERO);
        keep(&opts, 3441, Duration::from_secs(5));
        assert_eq!(resume(&opts, 3441), Duration::from_secs(5));
        forget(&opts, 3441);
        assert!(!dir.join("ckpt/n3441.clock").exists());
        assert_eq!(resume(&opts, 3441), Duration::ZERO);
    }

    #[test]
    fn a_clock_that_says_no_number_of_seconds_reads_as_zero() {
        let dir = TempDir::new("clock-garbage");
        let opts = options(&dir);
        fs::create_dir_all(dir.join("ckpt")).unwrap();
        fs::write(dir.join("ckpt/n0007.ckpt"), b"x").unwrap();
        // A reading that is a number, to show that the files are found.
        fs::write(dir.join("ckpt/n0007.clock"), "9\n").unwrap();
        assert_eq!(resume(&opts, 7), Duration::from_secs(9));
        for text in ["", "soon\n", "-5\n", "NaN\n", "inf\n", "1e300\n", "12 seconds"] {
            fs::write(dir.join("ckpt/n0007.clock"), text).unwrap();
            assert_eq!(resume(&opts, 7), Duration::ZERO, "{text:?}");
        }
        fs::write(dir.join("ckpt/n0007.clock"), " 42.5 \n").unwrap();
        assert_eq!(resume(&opts, 7), Duration::from_millis(42_500));
    }

    #[test]
    fn without_a_checkpoint_directory_there_is_nothing_to_keep() {
        let opts = SolveOptions::default();
        keep(&opts, 7, Duration::from_secs(1));
        forget(&opts, 7);
        assert_eq!(resume(&opts, 7), Duration::ZERO);
    }
}
