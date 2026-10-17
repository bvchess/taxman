//! Test helpers that need nothing but `std`, so one file serves both kinds of test: the unit tests in `src` use it
//! as `crate::test_support` (see `lib.rs`) and the integration tests as `common::TempDir` and `common::Rng`
//! (`tests/common/mod.rs`).
#![allow(dead_code)]

use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Whether this is a quick run, a non-release `cargo test` (debug assertions are on in the test profile and off in the
/// release profile): a test that draws or walks a count that can be changed (random configurations, an instance that
/// must last) uses a smaller one, and `cargo test --release` uses the full one (DESIGN.md, "Testing").
pub const QUICK: bool = cfg!(debug_assertions);

/// The reason a long test gives for being ignored in a non-release run, where it is skipped (debug assertions are on in
/// the test profile and off in the release profile, so `cargo test` runs the quick suite and `cargo test --release`
/// everything; DESIGN.md, "Testing"): `#[cfg_attr(debug_assertions, ignore = long!(quick_one))]`, with `quick_one`
/// the name of the quick test that checks the same property at a small size.
#[allow(unused_macros)]
macro_rules! long {
    ($quick:ident) => {
        concat!("long: runs under cargo test --release; the quick check is ", stringify!($quick))
    };
}

/// A fresh, empty directory under the system temp dir, removed when dropped.  It derefs to its [`Path`], so
/// `dir.join("file")` is a path inside it.  Each one has a name of its own within the process, so tests that run
/// at the same time never share a directory.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("taxman-test-{name}-{}-{seq}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }
}

impl Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A xorshift64 generator, so that the tests need no crates and their random choices are the same on every run.
/// The state must not be 0.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A number in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// True with probability `percent`/100.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    /// One of `items`.
    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len() as u64) as usize]
    }
}
