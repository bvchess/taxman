//! The clock of a game: what its search has spent, kept between invocations (a line records it in `seconds`, as
//! `cpu_all_runs`, `active_all_runs` and `waiting_all_runs`; "The clock" in [`window`](super::window)).
//!
//! The clock is a file beside the game's checkpoint, `n<NNNN>.clock`, holding the CPU time of the game's workers, the
//! time it was active (with a worker) and the time it waited (in flight with none), summed over every invocation that
//! searched it: one line, `cpu=112.300 active=80.100 waiting=15.000`, in seconds.  The checkpoint holds the search's
//! own elapsed time too, but that counts the time a search waited behind an older game, suspended in the window, as
//! active; so the clock is the driver's, which tells the two apart, and the checkpoint's format is left as it is.  A
//! clock is valid only beside the checkpoint it was kept with: a game that restarts from its beginning (no checkpoint)
//! starts its clock again, so deleting a checkpoint by hand needs no second deletion.  A finished game's clock goes
//! with its checkpoint ([`forget`], from `discard_checkpoint`).

use super::SolveOptions;
use crate::engine::Spent;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

/// The longest time a file may claim (a hundred years of seconds); anything else is taken for garbage.
const MAX_SECONDS: f64 = 3.2e9;

/// Game `n`'s clock as its search starts: what the last invocation kept, if the game has a checkpoint to resume, else
/// nothing spent (also for a file that does not say the three times: the clock only reports, and a lost reading makes
/// the times in the line too small).
pub(super) fn resume(opts: &SolveOptions, n: u32) -> Spent {
    let (Some(checkpoint), Some(clock)) = (opts.checkpoint_file(n), opts.clock_file(n)) else {
        return Spent::default();
    };
    if !checkpoint.exists() {
        return Spent::default();
    }
    read(&clock)
}

/// The times in the file `path`, or none.
fn read(path: &Path) -> Spent {
    fs::read_to_string(path).ok().and_then(|text| parse(&text)).unwrap_or_default()
}

/// The times in `text`, `cpu=S active=S waiting=S` in that order, each a number of seconds that is neither negative nor
/// absurd.
fn parse(text: &str) -> Option<Spent> {
    let mut words = text.split_whitespace();
    let mut time = |key: &str| -> Option<Duration> {
        let seconds: f64 = words.next()?.strip_prefix(key)?.strip_prefix('=')?.parse().ok()?;
        (seconds.is_finite() && (0.0..=MAX_SECONDS).contains(&seconds)).then(|| Duration::from_secs_f64(seconds))
    };
    let spent = Spent { cpu: time("cpu")?, active: time("active")?, waiting: time("waiting")? };
    words.next().is_none().then_some(spent)
}

/// Keeps `spent` as game `n`'s clock, replacing the file whole (written aside and renamed, so a crash leaves the old
/// reading or the new, never half of one).  Nothing happens without a checkpoint directory.  A failure is reported on
/// stderr and the run goes on: the clock is a report and not a result.
pub(super) fn keep(opts: &SolveOptions, n: u32, spent: Spent) {
    let Some(path) = opts.clock_file(n) else { return };
    let aside = path.with_extension("clock.tmp");
    // The directory may not exist yet: the first checkpoint of a run is written after its first interval.
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::File::create(&aside))
        .and_then(|mut file| {
            let seconds = |time: Duration| time.as_secs_f64();
            writeln!(
                file,
                "cpu={:.3} active={:.3} waiting={:.3}",
                seconds(spent.cpu),
                seconds(spent.active),
                seconds(spent.waiting)
            )?;
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
        // Absent is the usual case (the game's search ended before its first interval); a file that stays only keeps a
        // reading that `resume` ignores without a checkpoint.
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

    fn ms(cpu: u64, active: u64, waiting: u64) -> Spent {
        let ms = Duration::from_millis;
        Spent { cpu: ms(cpu), active: ms(active), waiting: ms(waiting) }
    }

    #[test]
    fn a_clock_is_kept_beside_its_checkpoint_and_read_back_from_it() {
        let dir = TempDir::new("clock");
        let opts = options(&dir);
        fs::create_dir_all(dir.join("ckpt")).unwrap();
        keep(&opts, 3441, ms(9_000_456, 86_400_123, 7));
        assert!(dir.join("ckpt/n3441.clock").exists() && !dir.join("ckpt/n3441.clock.tmp").exists());
        assert_eq!(
            fs::read_to_string(dir.join("ckpt/n3441.clock")).unwrap(),
            "cpu=9000.456 active=86400.123 waiting=0.007\n"
        );
        // No checkpoint to resume: the game starts over, and so does its clock.
        assert_eq!(resume(&opts, 3441), Spent::default());
        fs::write(dir.join("ckpt/n3441.ckpt"), b"x").unwrap();
        assert_eq!(resume(&opts, 3441), ms(9_000_456, 86_400_123, 7));
        // Another game's clock is its own.
        assert_eq!(resume(&opts, 3442), Spent::default());
        keep(&opts, 3441, ms(5_000, 5_000, 0));
        assert_eq!(resume(&opts, 3441), ms(5_000, 5_000, 0));
        forget(&opts, 3441);
        assert!(!dir.join("ckpt/n3441.clock").exists());
        assert_eq!(resume(&opts, 3441), Spent::default());
    }

    #[test]
    fn a_clock_that_says_no_three_times_reads_as_nothing_spent() {
        let dir = TempDir::new("clock-garbage");
        let opts = options(&dir);
        fs::create_dir_all(dir.join("ckpt")).unwrap();
        fs::write(dir.join("ckpt/n0007.ckpt"), b"x").unwrap();
        // A reading, to show that the files are found.
        fs::write(dir.join("ckpt/n0007.clock"), "cpu=9 active=3 waiting=1\n").unwrap();
        assert_eq!(resume(&opts, 7), ms(9_000, 3_000, 1_000));
        for text in [
            "",
            "soon\n",
            "9\n",
            "cpu=9 active=3\n",
            "active=3 cpu=9 waiting=1\n",
            "cpu=9 active=3 waiting=1 extra=1\n",
            "cpu=-5 active=3 waiting=1\n",
            "cpu=NaN active=3 waiting=1\n",
            "cpu=inf active=3 waiting=1\n",
            "cpu=9 active=1e300 waiting=1\n",
            "cpu=9 active=3 waiting=12 seconds",
        ] {
            fs::write(dir.join("ckpt/n0007.clock"), text).unwrap();
            assert_eq!(resume(&opts, 7), Spent::default(), "{text:?}");
        }
        fs::write(dir.join("ckpt/n0007.clock"), " cpu=42.5   active=40 \n waiting=2.25 \n").unwrap();
        assert_eq!(resume(&opts, 7), ms(42_500, 40_000, 2_250));
    }

    #[test]
    fn without_a_checkpoint_directory_there_is_nothing_to_keep() {
        let opts = SolveOptions::default();
        keep(&opts, 7, ms(1_000, 1_000, 0));
        forget(&opts, 7);
        assert_eq!(resume(&opts, 7), Spent::default());
    }
}
