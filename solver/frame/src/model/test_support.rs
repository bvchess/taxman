use super::{FrameGraph, FrameSet};
use crate::bits::Bits;
use crate::json::Json;
use std::path::PathBuf;
use std::time::Instant;

pub(crate) use crate::test_support::Rng;

/// What the tests of the frame model draw at random besides what [`Rng`] draws.
pub(crate) trait Draws {
    /// Puts `items` in a random order.
    fn shuffle<T>(&mut self, items: &mut [T]);
    /// A random subset of `set`, each member kept with probability `percent`/100.
    fn subset<const W: usize>(&mut self, set: &Bits<W>, percent: u64) -> Bits<W>;
}

impl Draws for Rng {
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i as u64 + 1) as usize);
        }
    }

    fn subset<const W: usize>(&mut self, set: &Bits<W>, percent: u64) -> Bits<W> {
        set.iter().filter(|_| self.chance(percent)).collect()
    }
}

/// One golden file: the Java's intermediate results for game `n`.
pub(crate) struct Golden {
    pub n: u32,
    /// Factors and moves per frame, front first: the columns the Java builds its `FrameSet` from.
    pub columns: Vec<Vec<u32>>,
    pub candidates: Vec<u32>,
    pub json: Json,
}

pub(crate) fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata").join("golden")
}

/// The board sizes with a golden file, ascending.
pub(crate) fn golden_ns() -> Vec<u32> {
    let mut ns: Vec<u32> = std::fs::read_dir(golden_dir())
        .expect("golden directory")
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            name.strip_prefix('n')?.strip_suffix(".json")?.parse().ok()
        })
        .collect();
    ns.sort_unstable();
    ns
}

pub(crate) fn u32s(json: &Json, key: &str) -> Vec<u32> {
    json.get(key).and_then(Json::as_u32_vec).unwrap_or_else(|| panic!("golden field {key}"))
}

pub(crate) fn load(n: u32) -> Golden {
    let json = Json::from_file(golden_dir().join(format!("n{n:04}.json"))).expect("golden file");
    assert!(json.get("error").is_none(), "golden n={n} recorded an error");
    let mut columns = Vec::new();
    for frame in json.get("frames").and_then(Json::as_arr).expect("golden frames") {
        columns.push(u32s(frame, "remainingFactors"));
        columns.push(u32s(frame, "remainingSources"));
    }
    let candidates = u32s(&json, "allCandidates");
    Golden { n, columns, candidates, json }
}

/// The largest sum of at most `max_size` members of `cands` that can be promoted from `state` (exhaustive,
/// pruned only by sums): the truth that upper bounds are checked against.
pub(crate) fn best_completion<const W: usize>(
    model: &FrameGraph,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    max_size: u32,
) -> u64 {
    fn dfs<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, list: &[u32], sum: u64, left: u32, best: &mut u64) {
        *best = (*best).max(sum);
        if left == 0 {
            return;
        }
        for i in 0..list.len() {
            let optimistic: u64 = sum + list[i..].iter().take(left as usize).map(|&x| u64::from(x)).sum::<u64>();
            if optimistic <= *best {
                return;
            }
            let mut next = *state;
            if model.promote_in_place(&mut next, list[i]) {
                dfs(model, &next, &list[i + 1..], sum + u64::from(list[i]), left - 1, best);
            }
        }
    }
    let list: Vec<u32> = cands.iter_desc().collect();
    let mut best = 0;
    dfs(model, state, &list, 0, max_size, &mut best);
    best
}

/// For micro-benchmarks: nanoseconds per call of `f`, which makes `calls` calls: the best of seven trials of
/// about 0.15 s each, so that other load on the machine disturbs the figure less.
pub(crate) fn time_per_call(calls: u64, mut f: impl FnMut()) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..7 {
        let start = Instant::now();
        let mut rounds = 0u64;
        while start.elapsed().as_secs_f64() < 0.15 {
            f();
            rounds += 1;
        }
        best = best.min(start.elapsed().as_secs_f64() * 1e9 / (rounds * calls) as f64);
    }
    best
}
