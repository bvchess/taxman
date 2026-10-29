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
/// must last) uses a smaller one, and `cargo test --release` uses the full one (solver/docs/design/testing.md).
pub const QUICK: bool = cfg!(debug_assertions);

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

/// The first `count` lines of the committed results, `results/optimal.jsonl.gz`, as text: the members that hold them are
/// decoded and no others (the file is read member by member, as the built-in table is).
pub fn committed_text(count: usize) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/optimal.jsonl.gz");
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let (mut text, mut lines) = (String::new(), 0);
    for member in taxman_gzip::Members::new(&bytes) {
        let member = member.unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let member = String::from_utf8(member).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        lines += member.matches('\n').count();
        text.push_str(&member);
        if lines >= count {
            break;
        }
    }
    text.split_inclusive('\n').take(count).collect()
}

/// A gzip file whose members are the texts `members`, in order, each stored in deflate blocks that are not compressed
/// (at most 65,535 bytes each): a file made by hand, so that a test of the reading of gzip files needs no compressor.
pub fn gzip_stored(members: &[&str]) -> Vec<u8> {
    let mut file = Vec::new();
    for text in members {
        // The header: the magic, deflate, no flags, no time, no extra flags, Unix.
        file.extend([0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3]);
        let blocks: Vec<&[u8]> = if text.is_empty() { vec![&[]] } else { text.as_bytes().chunks(0xFFFF).collect() };
        for (i, block) in blocks.iter().enumerate() {
            let length = block.len() as u16;
            // The block's header byte: the last block's bit and the type 0, stored.
            file.push(u8::from(i + 1 == blocks.len()));
            file.extend(length.to_le_bytes());
            file.extend((!length).to_le_bytes());
            file.extend(*block);
        }
        file.extend(taxman_gzip::crc32(text.as_bytes()).to_le_bytes());
        file.extend((text.len() as u32).to_le_bytes());
    }
    file
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
