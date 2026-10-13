//! Cycles and instructions retired of this process (other load on the machine barely moves them, unlike wall
//! time), for the examples that measure (`#[path = "common/counters.rs"] mod counters;`; the probe,
//! `examples/perf.rs`, is the first).  On Linux there is no per-process cycle counter without a crate, so [`now`]
//! says nothing there.

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
