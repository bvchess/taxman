//! A clean stop on SIGINT and SIGTERM: the first signal asks `exact`'s campaign ([`crate::team`]) to pause every search
//! as the end of a pass does, and a second signal ends the process at once.
//!
//! The handler only sets a flag, [`requested`], which the campaign polls; once every search is saved, the program
//! exits with [`status`], 128 plus the signal's number (130 for SIGINT, 143 for SIGTERM), which
//! `solver/tools/cloud/prover.sh` reads as a stop by a signal.  A second signal ends the process with that status
//! without saving anything more: the checkpoints and the lines already written stay valid (a checkpoint is written to a
//! file of its own and renamed, and a torn last line of the output is dropped by the next run).
//!
//! The binding is the C library's `signal`, which has BSD semantics on Linux and macOS (the handler stays installed and
//! interrupted system calls restart).  The crate has no dependency but the gzip decoder, and a handler of one atomic
//! flag needs no more than this.

use std::ffi::c_int;
use std::io;
use std::sync::atomic::{AtomicI32, Ordering};

/// The same numbers on Linux and macOS.
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
/// What `signal` returns when it fails (`SIG_ERR`, that is `(void (*)(int)) -1`).
const SIG_ERR: usize = usize::MAX;

extern "C" {
    /// The previous handler is returned as an address, since it can be `SIG_DFL` (0) or `SIG_IGN` (1).
    fn signal(signum: c_int, handler: extern "C" fn(c_int)) -> usize;
    fn _exit(status: c_int) -> !;
}

/// The first signal that asked for a stop (0: none yet).
static REQUESTED: AtomicI32 = AtomicI32::new(0);

/// Records the first signal; a second ends the process.  Async-signal-safe: a lock-free atomic and `_exit`.
extern "C" fn on_signal(signum: c_int) {
    if REQUESTED.swap(signum, Ordering::SeqCst) != 0 {
        // SAFETY: `_exit` has no preconditions.
        unsafe { _exit(128 + signum) }
    }
}

/// Makes SIGINT and SIGTERM ask for a stop ([`requested`]) instead of ending the process.
///
/// # Errors
///
/// The operating system's, if it refuses a handler.
pub fn on_signals() -> io::Result<()> {
    for signum in [SIGINT, SIGTERM] {
        // SAFETY: `signal` has no preconditions, and the handler is async-signal-safe.
        if unsafe { signal(signum, on_signal) } == SIG_ERR {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// The signal that asked for a stop, if one has.
pub fn requested() -> Option<i32> {
    match REQUESTED.load(Ordering::Relaxed) {
        0 => None,
        signum => Some(signum),
    }
}

/// The exit status of a program stopped by a signal, 128 plus its number, if one has stopped it.
pub fn status() -> Option<i32> {
    requested().map(|signum| 128 + signum)
}
