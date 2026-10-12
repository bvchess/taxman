//! Bounds and candidate filters for the branch-and-bound search over promotions: the preclusion table, the
//! `blameGame` upper bound, the greedy incumbent, and the per-node helpers of the search (the Java's
//! `UpperBoundEstimator`, `LowerBoundEstimator.greedy`, `BranchExplorer.makeNewCandidateList` and
//! `SearchManager.maxAdditionalPromotions`).
//!
//! Everything here works on a [`FrameSet`] and the [`Model`] it belongs to, and allocates nothing per call
//! except [`Preclusions::build`].

use crate::bits::Bits;
use crate::model::{FrameSet, Model, Scratch, MAX_FRAMES};
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------------------------------------
// Per-node helpers

/// The Java's `SearchManager.maxAdditionalPromotions`: how many more promotions a node can make, given the
/// overall limit `max_total`, the promotions made so far, and the candidates left; never negative.
#[inline]
pub fn max_additional<const W: usize>(max_total: u32, state: &FrameSet<W>, promoted: &Bits<W>, cands: &Bits<W>) -> u32 {
    let by_state = state.max_promotions_among(cands).max(0) as u32;
    max_total.saturating_sub(promoted.len()).min(by_state)
}

/// The Java's `BranchExplorer.makeNewCandidateList`: the candidates for the child made by promoting `n`.
///
/// `new_state` is the state after promoting `n`, `cands` the parent's remaining candidates (all below `n`),
/// `promoted` the parent's promotions (without `n`), and `max_next` the child's [`max_additional`].  Steps:
/// narrow to the factors of frames that can still promote; drop what `preclusions` (if given) rules out after
/// `promoted` and `n`; then, only when `n`'s home frame is above 1, drop the candidates of `n`'s home frame and
/// the one below that cannot be promoted, checking from the top and stopping once `max_next` have passed (as
/// in the Java, `max_next = 0` never stops the checks).
pub fn new_candidate_list<const W: usize>(
    model: &Model,
    preclusions: Option<&Preclusions<W>>,
    n: u32,
    max_next: u32,
    new_state: &FrameSet<W>,
    cands: &Bits<W>,
    promoted: &Bits<W>,
) -> Bits<W> {
    let mut filtered = new_state.narrow_candidates(cands);
    if let Some(p) = preclusions {
        p.remove_precluded(&mut filtered, n, promoted);
    }
    let home_n = model.home(n);
    if home_n > 1 {
        let mut hits = 0;
        let mut scratch = None;
        let unchecked = filtered;
        for x in unchecked.iter_desc() {
            let home_x = model.home(x);
            if (home_x != home_n && home_x + 1 != home_n)
                || model.can_promote_using(scratch.get_or_insert_with(Scratch::new), new_state, x)
            {
                hits += 1;
                if hits == max_next {
                    break;
                }
            } else {
                filtered.remove(x);
            }
        }
    }
    filtered
}

// ---------------------------------------------------------------------------------------------------------
// Upper bound

/// How [`upper_bound`] replaces the first member of its bid that fails.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunnerUp {
    /// The largest candidate not in the bid (0 if none).  Sound: sets within the caps of [`largest_that_fit`] form
    /// a matroid whose heaviest set is the bid, and the heaviest such set avoiding one bid member is the bid with
    /// that member swapped for at most one candidate outside it.  A feasible set must avoid some member at or
    /// above the first failure, and all of those are at least as large as the failing one.  (This assumes the
    /// caps and `max_size` hold for every feasible set, as they do for freedoms that have not been revised.)
    #[default]
    Sound,
    /// The Java's `getRunnerUp`: the largest candidate below the bid's smallest member (0 if none).  When
    /// [`largest_that_fit`] skips candidates because frame 1 or the last frame is at its cap, a skipped
    /// candidate can be larger than that and be exactly what replaces a failing number of its own frame, so this
    /// bound can be too low.  Kept to reproduce the Java.
    Legacy,
}

/// The Java's `largestThatFitInFrames`: the largest candidates, at most `max_size` of them, taking from frame 1
/// at most `min(freedoms of frames 0, 1)` and from the last frame at most `min(freedoms of the last two
/// frames)`.  With fewer than 4 frames, simply the `max_size` largest.
pub fn largest_that_fit<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>, max_size: u32) -> Bits<W> {
    let frames = state.frames();
    let len = frames.len();
    if len < 4 {
        return cands.largest(max_size);
    }
    let mut low = frames[0].freedoms.min(frames[1].freedoms);
    let mut high = frames[len - 2].freedoms.min(frames[len - 1].freedoms);
    let mut out = Bits::EMPTY;
    let mut taken = 0;
    for x in cands.iter_desc() {
        if taken >= max_size {
            break;
        }
        let home = model.home(x);
        let cap = if home == 1 {
            &mut low
        } else if home == len - 1 {
            &mut high
        } else {
            out.insert(x);
            taken += 1;
            continue;
        };
        if *cap > 0 {
            *cap -= 1;
            out.insert(x);
            taken += 1;
        }
    }
    out
}

/// The Java's `blameGame`: an upper bound on the sum of the promotions still possible from `state` among
/// `cands`, at most `max_size` of them.  Bid the [`largest_that_fit`]; find the first member, from the top, that
/// cannot be promoted after the ones above it (or is no longer a factor); assume it is the only failure and swap
/// it for the [`RunnerUp`].
pub fn upper_bound<const W: usize>(
    model: &Model,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    max_size: u32,
    rule: RunnerUp,
) -> u64 {
    let bid = largest_that_fit(model, state, cands, max_size);
    let best = bid.sum();
    match first_failure(model, state, &bid) {
        None => best,
        Some(fail) => best - u64::from(fail) + u64::from(runner_up(cands, &bid, rule)),
    }
}

/// The first member of `bid`, from the top, that cannot be promoted after the ones above it.
///
/// Rather than promoting the members one by one (two frame checks each), this works frame by frame (see
/// [`Model::winnable`]): promoting the members down to `x` succeeds exactly when every member down to `x` is a
/// factor of its home frame `h ≥ 1` and every frame is winnable with all of them promoted.  A frame that is
/// winnable with all its members promoted cannot fail; otherwise a binary search over its members finds where
/// it starts to fail, since a frame only gets harder as members are added.
fn first_failure<const W: usize>(model: &Model, state: &FrameSet<W>, bid: &Bits<W>) -> Option<u32> {
    let frames = state.frames();
    // Per frame, the members promoted into it and out of it.
    let mut gained = [Bits::<W>::EMPTY; MAX_FRAMES];
    let mut lost = [Bits::<W>::EMPTY; MAX_FRAMES];
    // The failure found so far (0 for none); only members above it still matter.
    let mut fail = 0;
    for x in bid.iter_desc() {
        let h = model.home(x);
        if h == 0 || !model.contains_factor(state, x) {
            fail = x;
            break;
        }
        gained[h - 1].insert(x);
        lost[h].insert(x);
    }
    let mut scratch = Scratch::new();
    for (i, frame) in frames.iter().enumerate() {
        let edits = (gained[i] | lost[i]).intersect(&Bits::from_range(fail + 1, Bits::<W>::MAX_VALUE));
        let Some(lowest) = edits.min() else {
            continue;
        };
        // Frame i with the members down to `t` promoted.
        let mut winnable_down_to = |t: u32| {
            let above = Bits::from_range(t, Bits::<W>::MAX_VALUE);
            let moves = frame.moves | gained[i].intersect(&above);
            model.winnable_using(&mut scratch, &moves, &(frame.factors - lost[i].intersect(&above)))
        };
        if winnable_down_to(lowest) {
            continue;
        }
        // The frame fails at its lowest edit; find the first (largest) edit where it fails.
        let (mut lo, mut hi) = (0, edits.len() - 1); // edits in descending order: winnable before lo, not at hi
        while lo < hi {
            let mid = (lo + hi) / 2;
            if winnable_down_to(edits.nth_largest(mid).unwrap()) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        fail = edits.nth_largest(hi).unwrap();
    }
    (fail != 0).then_some(fail)
}

fn runner_up<const W: usize>(cands: &Bits<W>, bid: &Bits<W>, rule: RunnerUp) -> u32 {
    match rule {
        RunnerUp::Sound => cands.difference(bid).max(),
        RunnerUp::Legacy => match bid.min() {
            None => cands.max(),
            Some(low) => cands.max_below(low),
        },
    }
    .unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------------------
// Greedy lower bound

/// The Java's `LowerBoundEstimator.greedy` (`findGreedySolutionWithSkips` with no skips): from the top, promote
/// every candidate that can be; after each success, narrow the candidates in the new state and go on with the
/// next one below in the narrowed set.  Returns the promotions made.
pub fn greedy<const W: usize>(model: &Model, root: &FrameSet<W>, cands: &Bits<W>) -> Bits<W> {
    let mut scratch = Scratch::new();
    let mut state = *root;
    let mut cands = *cands;
    let mut promoted = Bits::EMPTY;
    let mut next = cands.max();
    while let Some(n) = next {
        if model.promote_in_place_using(&mut scratch, &mut state, n) {
            cands = state.narrow_candidates(&cands);
            promoted.insert(n);
        }
        next = cands.max_below(n);
    }
    promoted
}

// ---------------------------------------------------------------------------------------------------------
// Preclusions

/// Candidates known to fail after certain others: the Java's `UpperBoundEstimator.findPreclusions` with
/// `CONTEXT_DEPTH = 2`.  For each number `n` it holds entries (context, precluded):
///
/// * context ∅: the candidates `m < n`, still factors after promoting `n` from the start, that then cannot be
///   promoted;
/// * context `{p}` (for `p > n`, promoting `p` then `n` from the start succeeds): the candidates `z < n`, still
///   factors after both, that then cannot be promoted.
///
/// [`precluded`](Self::precluded) unions the entries whose context lies within the promotions made.  Ruling
/// those candidates out is sound because a promotion that fails keeps failing after further promotions: they
/// only add moves to frames and take factors away, which never makes a frame winnable (see [`Model::winnable`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preclusions<const W: usize> {
    /// The entry with the empty context, by number.
    alone: Vec<Bits<W>>,
    /// `after[start[n]..start[n + 1]]`: the entries of `n` with a one-number context, ascending by context.
    start: Vec<u32>,
    after: Vec<(u32, Bits<W>)>,
}

impl<const W: usize> Preclusions<W> {
    /// Builds the table for the search that starts at `root` with `candidates`, on `threads` threads (at least
    /// one).  The result does not depend on the number of threads or their scheduling.  A candidate that cannot
    /// be promoted at the start contributes nothing (the Java would fail on it).
    pub fn build(model: &Model, root: &FrameSet<W>, candidates: &Bits<W>, threads: usize) -> Self {
        // Largest first: those have the most candidates below them, so they are the longest tasks.
        let tops: Vec<u32> = candidates.iter_desc().collect();
        let next = AtomicUsize::new(0);
        let work = || {
            let mut found = Vec::new();
            let mut scratch = Scratch::new();
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(&n) = tops.get(i) else { break };
                explore_preclusions(model, &mut scratch, root, candidates, n, &mut found);
            }
            found
        };
        let mut found: Vec<(u32, u32, Bits<W>)> = if threads <= 1 {
            work()
        } else {
            std::thread::scope(|scope| {
                let workers: Vec<_> = (0..threads).map(|_| scope.spawn(work)).collect();
                workers.into_iter().flat_map(|w| w.join().expect("preclusion worker panicked")).collect()
            })
        };
        found.sort_unstable_by_key(|&(n, context, _)| (n, context));

        let size = candidates.max().map_or(0, |m| m as usize + 1);
        let mut out = Preclusions { alone: vec![Bits::EMPTY; size], start: vec![0; size + 1], after: Vec::new() };
        for (n, context, precluded) in found {
            if context == 0 {
                out.alone[n as usize] = precluded;
            } else {
                out.after.push((context, precluded));
                out.start[n as usize + 1] += 1;
            }
        }
        for i in 0..size {
            out.start[i + 1] += out.start[i];
        }
        out
    }

    /// The candidates ruled out after promoting `promoted` and then `n`.
    #[inline]
    pub fn precluded(&self, n: u32, promoted: &Bits<W>) -> Bits<W> {
        let i = n as usize;
        let mut out = self.alone.get(i).copied().unwrap_or_default();
        if i < self.alone.len() {
            for (p, set) in &self.after[self.start[i] as usize..self.start[i + 1] as usize] {
                if promoted.contains(*p) {
                    out |= *set;
                }
            }
        }
        out
    }

    /// Removes from `set` the candidates ruled out after promoting `promoted` and then `n`.
    #[inline]
    pub fn remove_precluded(&self, set: &mut Bits<W>, n: u32, promoted: &Bits<W>) {
        *set -= self.precluded(n, promoted);
    }

    /// Number of entries (with an empty or a one-number context).
    pub fn entries(&self) -> usize {
        self.alone.iter().filter(|s| !s.is_empty()).count() + self.after.len()
    }

    /// Total size of all the precluded sets.
    pub fn precluded_total(&self) -> u64 {
        self.alone.iter().chain(self.after.iter().map(|(_, s)| s)).map(|s| u64::from(s.len())).sum()
    }
}

/// The numbers below `n`.
#[inline]
fn below<const W: usize>(n: u32) -> Bits<W> {
    if n == 0 {
        Bits::EMPTY
    } else {
        Bits::from_range(0, n - 1)
    }
}

/// The preclusions found under the top-level candidate `n`, as `(number, context or 0, precluded)`.
fn explore_preclusions<const W: usize>(
    model: &Model,
    scratch: &mut Scratch<W>,
    root: &FrameSet<W>,
    candidates: &Bits<W>,
    n: u32,
    found: &mut Vec<(u32, u32, Bits<W>)>,
) {
    let mut first = *root;
    if !model.promote_in_place_using(scratch, &mut first, n) {
        return;
    }
    // The candidates below n that are still factors, and those of them that cannot follow n.
    let open: Bits<W> = candidates.intersect(&below(n)).iter().filter(|&m| model.contains_factor(&first, m)).collect();
    let alone: Bits<W> = open.iter().filter(|&m| !model.can_promote_using(scratch, &first, m)).collect();

    // For each m that can follow n: the z below m, still factors after both, that cannot follow them.  Promoting m
    // changed only frames h(m) − 1 and h(m), so for a z that could follow n only those of its two frames need a
    // new check, and a z that could not follow n cannot follow m either (its frame only got harder).
    let mut second = first;
    for m in (open - alone).iter_desc() {
        second.assign(&first);
        let promoted = model.promote_in_place_using(scratch, &mut second, m);
        debug_assert!(promoted);
        let home_m = model.home(m);
        let changed = |frame: usize| frame + 1 == home_m || frame == home_m;
        let mut after = Bits::EMPTY;
        for z in open.intersect(&below(m)).iter() {
            if !model.contains_factor(&second, z) {
                continue;
            }
            let h = model.home(z);
            let fails = alone.contains(z) || {
                let (into, out) = (&second.frames()[h - 1], &second.frames()[h]);
                (changed(h - 1) && !model.winnable_using(scratch, &into.moves.with(z), &into.factors))
                    || (changed(h) && !model.winnable_using(scratch, &out.moves, &out.factors.without(z)))
            };
            if fails {
                after.insert(z);
            }
        }
        if !after.is_empty() {
            found.push((m, n, after));
        }
    }
    if !alone.is_empty() {
        found.push((n, 0, alone));
    }
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;
    use crate::numtheory::Tables;
    use std::hint::black_box;
    use std::time::Instant;

    fn setup<const W: usize>(g: &Golden) -> (Model, FrameSet<W>, Bits<W>) {
        let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        (model, root, Bits::from_slice(&g.candidates))
    }

    // ---- golden data ----

    fn check_golden_greedy<const W: usize>(g: &Golden) {
        let (model, root, cands) = setup::<W>(g);
        let got = greedy(&model, &root, &cands).to_vec();
        assert_eq!(got, u32s(&g.json, "greedy"), "n={}", g.n);
        if let Some(manager) = g.json.get("searchManager").filter(|j| !j.is_null()) {
            assert_eq!(got, u32s(manager, "initialBest"), "n={}", g.n);
        }
    }

    #[test]
    fn golden_greedy() {
        for n in golden_ns() {
            let g = load(n);
            if n < 512 {
                check_golden_greedy::<8>(&g);
            } else {
                check_golden_greedy::<16>(&g);
            }
        }
    }

    // ---- preclusions ----

    #[test]
    fn preclusions_do_not_depend_on_threads() {
        for n in [120, 300, 480] {
            let g = load(n);
            let (model, root, cands) = setup::<8>(&g);
            let one = Preclusions::build(&model, &root, &cands, 1);
            assert!(one.entries() > 0, "n={n} has preclusions");
            for threads in [2, 3, 8] {
                assert_eq!(Preclusions::build(&model, &root, &cands, threads), one, "n={n}, {threads} threads");
            }
        }
    }

    /// The Java's `findPreclusions`, step for step (sequentially, and skipping a candidate the start cannot
    /// promote), as `(number, context or 0, precluded)` sorted.
    fn reference_preclusions<const W: usize>(
        model: &Model,
        root: &FrameSet<W>,
        cands: &Bits<W>,
    ) -> Vec<(u32, u32, Bits<W>)> {
        let mut found = Vec::new();
        for n in cands.iter() {
            let Some(first) = model.promote(root, n) else { continue };
            let mut alone = Bits::EMPTY;
            for m in cands.iter().filter(|&m| m < n && model.contains_factor(&first, m)) {
                match model.promote(&first, m) {
                    None => alone.insert(m),
                    Some(second) => {
                        let after: Bits<W> = cands
                            .iter()
                            .filter(|&z| {
                                z < m && model.contains_factor(&second, z) && model.promote(&second, z).is_none()
                            })
                            .collect();
                        if !after.is_empty() {
                            found.push((m, n, after));
                        }
                    }
                }
            }
            if !alone.is_empty() {
                found.push((n, 0, alone));
            }
        }
        found.sort_unstable_by_key(|&(n, context, _)| (n, context));
        found
    }

    #[test]
    fn preclusions_match_the_java_procedure() {
        for n in [85, 120, 150, 222, 300] {
            let g = load(n);
            let (model, root, cands) = setup::<8>(&g);
            let built = Preclusions::build(&model, &root, &cands, 1);
            let mut entries = Vec::new();
            for x in 0..built.alone.len() {
                if !built.alone[x].is_empty() {
                    entries.push((x as u32, 0, built.alone[x]));
                }
                for &(p, set) in &built.after[built.start[x] as usize..built.start[x + 1] as usize] {
                    entries.push((x as u32, p, set));
                }
            }
            entries.sort_unstable_by_key(|&(n, context, _)| (n, context));
            assert_eq!(entries, reference_preclusions(&model, &root, &cands), "n={n}");
        }
    }

    #[test]
    fn preclusion_lookup() {
        let mut p = Preclusions::<1> { alone: vec![Bits::EMPTY; 11], start: vec![0; 12], after: Vec::new() };
        p.alone[10] = Bits::from_slice(&[3]);
        // Entries of 10 with contexts {20} and {30}; numbers above the table have none.
        p.after = vec![(20, Bits::from_slice(&[4])), (30, Bits::from_slice(&[5, 6]))];
        p.start[11] = 2;
        assert_eq!(p.precluded(10, &Bits::EMPTY).to_vec(), vec![3]);
        assert_eq!(p.precluded(10, &Bits::from_slice(&[30, 40])).to_vec(), vec![3, 5, 6]);
        assert_eq!(p.precluded(10, &Bits::from_slice(&[20, 30])).to_vec(), vec![3, 4, 5, 6]);
        assert!(p.precluded(9, &Bits::from_slice(&[20, 30])).is_empty());
        assert!(p.precluded(63, &Bits::from_slice(&[20])).is_empty());
        let mut set = Bits::from_slice(&[1, 3, 4, 5]);
        p.remove_precluded(&mut set, 10, &Bits::from_slice(&[20]));
        assert_eq!(set.to_vec(), vec![1, 5]);
        assert_eq!((p.entries(), p.precluded_total()), (3, 4));
    }

    // ---- candidate lists ----

    #[test]
    fn candidate_list_check_limit() {
        // n=300: promote a candidate of frame 3 and compare the can_promote filter with and without a limit.
        let g = load(300);
        let (model, root, cands) = setup::<8>(&g);
        let n = cands.iter_desc().find(|&x| model.home(x) == 3 && model.can_promote(&root, x)).expect("candidate");
        let new_state = model.promote(&root, n).unwrap();
        let below = cands.intersect(&Bits::from_range(0, n - 1));
        let narrowed = new_state.narrow_candidates(&below);
        let full = new_candidate_list(&model, None, n, 0, &new_state, &below, &Bits::EMPTY);
        for x in narrowed.iter() {
            let checked = matches!(model.home(x), 2 | 3);
            assert_eq!(full.contains(x), !checked || model.can_promote(&new_state, x), "x={x}");
        }
        // With a limit of one, checking stops at the first candidate that passes.
        let first_pass = narrowed.iter_desc().find(|&x| full.contains(x)).unwrap();
        let limited = new_candidate_list(&model, None, n, 1, &new_state, &below, &Bits::EMPTY);
        assert_eq!(limited, full.union(&narrowed.intersect(&Bits::from_range(0, first_pass - 1))));
    }

    // ---- upper bound ----

    /// The Java's way: promote the bid one by one on a scratch copy.
    fn first_failure_one_by_one<const W: usize>(model: &Model, state: &FrameSet<W>, bid: &Bits<W>) -> Option<u32> {
        let mut scratch = *state;
        bid.iter_desc().find(|&x| !model.promote_in_place(&mut scratch, x))
    }

    fn check_first_failure<const W: usize>(g: &Golden, rng: &mut Rng) -> u64 {
        let (model, root, cands) = setup::<W>(g);
        let mut failures = 0;
        for _ in 0..10 {
            let mut state = root;
            let mut open = cands;
            loop {
                for _ in 0..5 {
                    let size = rng.below(open.len() as usize + 2) as u32;
                    let density = 5 + rng.below(90) as u64;
                    for bid in [largest_that_fit(&model, &state, &open, size), rng.subset(&cands, density)] {
                        let expect = first_failure_one_by_one(&model, &state, &bid);
                        assert_eq!(first_failure(&model, &state, &bid), expect, "n={}, bid {bid:?}", g.n);
                        failures += u64::from(expect.is_some());
                    }
                }
                let members = open.to_vec();
                if members.is_empty() {
                    break;
                }
                let n = rng.pick(&members);
                if model.promote_in_place(&mut state, n) {
                    open = state.narrow_candidates(&open.intersect(&Bits::from_range(0, n - 1)));
                } else {
                    open.remove(n);
                }
            }
        }
        failures
    }

    #[test]
    fn first_failure_matches_promoting_one_by_one() {
        let mut rng = Rng(0x0DDB_A11C_AFE0_0001);
        let mut failures = 0;
        for n in golden_ns().into_iter().filter(|&n| n >= 30 && (n % 11 == 0 || n >= 480)) {
            let g = load(n);
            failures +=
                if n < 512 { check_first_failure::<8>(&g, &mut rng) } else { check_first_failure::<16>(&g, &mut rng) };
        }
        assert!(failures > 1000, "only {failures} bids failed");
    }

    // ---- bound soundness ----

    #[derive(Default)]
    struct Soundness {
        checks: u64,
        children: u64,
        legacy_low: Vec<String>,
    }

    fn check_bound<const W: usize>(
        model: &Model,
        state: &FrameSet<W>,
        cands: &Bits<W>,
        max_size: u32,
        what: &str,
        report: &mut Soundness,
    ) {
        let truth = best_completion(model, state, cands, max_size);
        let sound = upper_bound(model, state, cands, max_size, RunnerUp::Sound);
        let legacy = upper_bound(model, state, cands, max_size, RunnerUp::Legacy);
        assert!(sound >= truth, "{what}: sound bound {sound} < best {truth} (max_size {max_size}, cands {cands:?})");
        assert!(sound >= legacy, "{what}: the sound bound is below the legacy one");
        if legacy < truth {
            report.legacy_low.push(format!("{what}: legacy {legacy} < best {truth} <= sound {sound}"));
        }
        report.checks += 1;
    }

    /// Follows random search paths (as the search would: descending promotions, candidate lists from
    /// `new_candidate_list`), checking the bounds at every node and that the candidate filters lose nothing.
    fn bound_soundness<const W: usize>(g: &Golden, rng: &mut Rng, report: &mut Soundness) {
        let (model, root, all) = setup::<W>(g);
        let max_total = root.max_promotions_among(&all).max(0) as u32;
        let preclusions = Preclusions::build(&model, &root, &all, 2);
        for path in 0..8 {
            let (mut state, mut promoted, mut cands) = (root, Bits::<W>::EMPTY, all);
            let mut depth = 0;
            while !cands.is_empty() {
                let max_add = max_additional(max_total, &state, &promoted, &cands);
                let what = format!("n={} path {path} depth {depth} promoted {promoted:?}", g.n);
                check_bound(&model, &state, &cands, max_add, &what, report);
                // Part way through the node's loop: the remaining candidates shrink, max_add stays.
                let members = cands.to_vec();
                let cut = rng.pick(&members);
                let rest = cands.intersect(&Bits::from_range(0, cut));
                check_bound(&model, &state, &rest, max_add, &format!("{what} below {cut}"), report);

                // Descend into a random child, if a few tries find one.
                let Some((n, new_state)) = (0..4).find_map(|_| {
                    let n = rng.pick(&members);
                    model.promote(&state, n).map(|s| (n, s))
                }) else {
                    break;
                };
                let current = cands.intersect(&Bits::from_range(0, n - 1));
                let new_promoted = promoted.with(n);
                let max_next = max_additional(max_total, &new_state, &new_promoted, &current);
                let child =
                    new_candidate_list(&model, Some(&preclusions), n, max_next, &new_state, &current, &promoted);
                // Narrowing, preclusions and the promotability filter never drop a useful candidate.
                let unfiltered = best_completion(&model, &new_state, &current, max_next);
                assert_eq!(best_completion(&model, &new_state, &child, max_next), unfiltered, "{what}, child {n}");
                for z in preclusions.precluded(n, &promoted).intersect(&current).iter() {
                    assert!(!model.can_promote(&new_state, z), "{what}: {z} precluded after {n} but promotable");
                }
                report.children += 1;
                (state, promoted, cands) = (new_state, new_promoted, child);
                depth += 1;
            }
        }
    }

    #[test]
    fn upper_bound_is_sound() {
        let mut rng = Rng(0xB0B0_5EED_0000_0042);
        let mut report = Soundness::default();
        for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)) {
            bound_soundness::<4>(&load(n), &mut rng, &mut report);
        }
        eprintln!(
            "bound soundness: {} bounds checked, {} children; legacy bound below the best {} times",
            report.checks,
            report.children,
            report.legacy_low.len()
        );
        for line in &report.legacy_low {
            eprintln!("  {line}");
        }
        assert!(report.checks > 1000);
    }

    #[test]
    fn greedy_and_bound_at_the_root_of_85() {
        let g = load(85);
        let (model, root, cands) = setup::<2>(&g);
        let best = greedy(&model, &root, &cands);
        assert_eq!(best.to_vec(), vec![28, 40, 42]);
        let max_total = root.max_promotions_among(&cands) as u32;
        assert_eq!(max_total, 3);
        // Three frames: the bid is simply the three largest, 42 + 40 + 36, and 36 cannot follow 42 and 40.
        let bid = largest_that_fit(&model, &root, &cands, max_total);
        assert_eq!(bid.to_vec(), vec![36, 40, 42]);
        assert_eq!(upper_bound(&model, &root, &cands, max_total, RunnerUp::Sound), 42 + 40 + 30);
        assert_eq!(upper_bound(&model, &root, &cands, max_total, RunnerUp::Legacy), 42 + 40 + 30);
        let none = Bits::EMPTY;
        assert_eq!(max_additional(max_total, &root, &none, &cands), 3);
        assert_eq!(max_additional(2, &root, &Bits::from_slice(&[42, 40, 28]), &cands), 0);
    }

    // ---- micro-benchmarks: cargo test --release bench_ -- --ignored --nocapture --test-threads=1 ----

    fn bench<const W: usize>(g: &Golden) {
        let (model, root, cands) = setup::<W>(g);
        let max_total = root.max_promotions_among(&cands).max(0) as u32;
        let list: Vec<u32> = cands.iter_desc().collect();
        eprintln!(
            "n={} W={W}: {} frames {:?}, {} candidates, max promotions {max_total}",
            g.n,
            root.len(),
            root.frames().iter().map(|f| (f.moves.len(), f.factors.len(), f.freedoms)).collect::<Vec<_>>(),
            list.len()
        );

        // States like the search's: the root, and the states along the greedy solution.
        let best = greedy(&model, &root, &cands);
        let mut states = vec![(root, cands)];
        let (mut s, mut c) = (root, cands);
        for n in best.iter_desc() {
            model.promote_in_place(&mut s, n);
            c = s.narrow_candidates(&c.intersect(&Bits::from_range(0, n - 1)));
            states.push((s, c));
        }
        let pairs: Vec<(usize, u32)> =
            states.iter().enumerate().flat_map(|(i, (_, c))| c.iter().map(move |x| (i, x))).collect();
        let successes = pairs.iter().filter(|&&(i, x)| model.can_promote(&states[i].0, x)).count();
        eprintln!("  {} (state, candidate) pairs along the greedy path, {successes} promotable", pairs.len());

        let mut scratch = Scratch::new();
        for (i, f) in root.frames().iter().enumerate() {
            let ns = time_per_call(1, || {
                black_box(model.winnable_using(&mut scratch, black_box(&f.moves), &f.factors));
            });
            eprintln!("  winnable, root frame {i} ({:2} moves): {ns:6.0} ns", f.moves.len());
        }
        let ns = time_per_call(pairs.len() as u64, || {
            for &(i, x) in &pairs {
                black_box(model.can_promote(black_box(&states[i].0), x));
            }
        });
        eprintln!("  can_promote:                 {ns:8.0} ns");
        let ns = time_per_call(pairs.len() as u64, || {
            for &(i, x) in &pairs {
                black_box(model.can_promote_using(&mut scratch, black_box(&states[i].0), x));
            }
        });
        eprintln!("  can_promote_using:           {ns:8.0} ns");
        let mut state = root;
        let ns = time_per_call(pairs.len() as u64, || {
            for &(i, x) in &pairs {
                state.assign(&states[i].0);
                black_box(model.promote_in_place_using(&mut scratch, black_box(&mut state), x));
            }
        });
        eprintln!("  assign + promote_in_place_using: {ns:4.0} ns");
        let ns = time_per_call(pairs.len() as u64, || {
            for &(i, x) in &pairs {
                black_box(model.promote(black_box(&states[i].0), x));
            }
        });
        eprintln!("  promote (copying):           {ns:8.0} ns");

        let prec = Preclusions::build(&model, &root, &cands, 1);
        let ns = time_per_call(states.len() as u64, || {
            for (i, (s, c)) in states.iter().enumerate() {
                let promoted = best.largest(i as u32);
                let max_add = max_additional(max_total, s, &promoted, c);
                black_box(upper_bound(&model, black_box(s), c, max_add, RunnerUp::Sound));
            }
        });
        eprintln!("  upper_bound (greedy path):   {ns:8.0} ns");
        let ns = time_per_call(1, || {
            black_box(upper_bound(&model, black_box(&root), &cands, max_total, RunnerUp::Sound));
        });
        eprintln!("  upper_bound (root):          {ns:8.0} ns");
        let ns = time_per_call(states.len() as u64, || {
            for (i, (s, c)) in states.iter().enumerate().skip(1) {
                let promoted = best.largest(i as u32 - 1);
                let n = best.nth_largest(i as u32 - 1).unwrap();
                let max_next = max_additional(max_total, s, &promoted.with(n), c);
                black_box(new_candidate_list(&model, Some(&prec), n, max_next, s, c, &promoted));
            }
        });
        eprintln!("  new_candidate_list:          {ns:8.0} ns");
        let ns = time_per_call(1, || {
            black_box(greedy(&model, black_box(&root), &cands));
        });
        eprintln!("  greedy:                      {:8.3} ms", ns / 1e6);
        for threads in [1, 10] {
            let start = Instant::now();
            let p = Preclusions::build(&model, &root, &cands, threads);
            let secs = start.elapsed().as_secs_f64();
            eprintln!(
                "  preclusions, {threads:2} threads:     {:8.3} s ({} entries, {} precluded in total, {:.1} MB)",
                secs,
                p.entries(),
                p.precluded_total(),
                ((p.alone.len() + p.after.len()) * std::mem::size_of::<(u32, Bits<W>)>()) as f64 / 1e6
            );
        }
    }

    #[test]
    #[ignore]
    fn bench_model_760() {
        // Golden data stops at n=760; n=1000 needs the frame builder (module `frames`) for its columns.
        let g = load(760);
        bench::<12>(&g);
        bench::<16>(&g);
    }

    #[test]
    #[ignore]
    fn bench_model_480() {
        bench::<8>(&load(480));
    }
}
