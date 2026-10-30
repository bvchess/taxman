//! Cycles and instructions retired of this process (other load on the machine barely moves them, unlike wall
//! time), for the examples that measure (`#[path = "common/counters.rs"] mod counters;`; the probe,
//! `examples/perf.rs`, is the first).  On Linux there is no per-process cycle counter without a crate, so [`now`]
//! says nothing there.

/// `struct rusage` as `getrusage` fills it in on Linux and macOS: two `timeval`s (16 bytes each on both, `[i64; 2]`
/// has their size and alignment) and fourteen `long`s, of which `ru_maxrss` is the first.
#[repr(C)]
struct Rusage {
    times: [[i64; 2]; 2],
    longs: [i64; 14],
}

unsafe extern "C" {
    fn getrusage(who: i32, usage: *mut Rusage) -> i32;
}

/// The CPU time this process has used so far, user plus system, in seconds (`getrusage`), if the system says.  Wall time
/// changes with the load on the machine, and CPU time less, though it still moves with it (shared caches, other
/// cores' heat); cycles ([`now`]) move least.
#[allow(dead_code)] // the probe does not read it
pub fn cpu_seconds() -> Option<f64> {
    const RUSAGE_SELF: i32 = 0;
    let mut usage = Rusage { times: [[0; 2]; 2], longs: [0; 14] };
    // SAFETY: `usage` is writable and has the layout of `struct rusage`, which `getrusage` fills in.
    let rc = unsafe { getrusage(RUSAGE_SELF, &mut usage) };
    // A `timeval` is seconds and microseconds: on macOS the microseconds are an `i32` in the low half of the second
    // word, on Linux an `i64`; they are under a million, so the low 32 bits hold them on both.
    let seconds = |[sec, usec]: [i64; 2]| sec as f64 + (usec & 0xFFFF_FFFF) as f64 * 1e-6;
    (rc == 0).then(|| seconds(usage.times[0]) + seconds(usage.times[1]))
}

#[cfg(target_os = "macos")]
mod imp {
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
    }

    /// (instructions retired, cycles) of this process so far: `rusage_info_v4` from `proc_pid_rusage`, whose
    /// fields after the 16-byte uuid are u64s, `ri_instructions` and `ri_cycles` the 30th and 31st.
    pub fn now() -> Option<(u64, u64)> {
        let mut info = [0u64; 64];
        // SAFETY: `info` is writable and larger than `rusage_info_v4` (37 words), which flavor 4 fills in.
        let rc = unsafe { proc_pid_rusage(std::process::id() as i32, 4, info.as_mut_ptr()) };
        (rc == 0).then_some((info[2 + 29], info[2 + 30]))
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn now() -> Option<(u64, u64)> {
        None
    }
}

pub use imp::now;
