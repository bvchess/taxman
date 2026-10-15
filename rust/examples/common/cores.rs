//! The cores that a set of candidates runs into in the frames of a state, and the GF(3) signings that make them
//! dependent: research code for the fields study `examples/fields.rs`.  Included with
//! `#[path = "common/cores.rs"] mod cores;` beside `gf3` (the signed frame matroids), `frames` and `meet`.
//!
//! # Cores
//!
//! Frame `h` of a state *with `Q` applied* (`Q` a set of candidates) has as movers the frame's moves and `Q`'s members
//! of home `h + 1`, and as pool the frame's factors without `Q`'s members of home `h` ([`frame_with`]).  A mover's
//! live factors are its maximal factors in the pool.  The peel drops a mover that has a live factor no other remaining
//! mover lists; what it cannot drop is the *residual* ([`residual`]; the crate's `Scratch::stuck` is the same peel,
//! private to the crate), empty exactly when the frame is winnable.  The residual is the largest *self-covering* set
//! of movers (every live factor of a member is listed by at least two members): self-covering sets are closed under
//! union, and the peel never drops a member of one.
//!
//! A [`Core`] is a self-covering set of movers, such as a residual or a minimal self-covering subset of one
//! ([`minimal`]).  Its *columns* are the pool factors its members list, each with its listers.  [`cores_of`] finds the
//! cores of `Q` frame by frame, un-applying one of `Q`'s members that the last core depends on ([`breaking`]) so that
//! the next core is another.
//!
//! # Signings
//!
//! A signing `σ` whose values make every column of a core sum to 0 over GF(3) makes the members' vectors sum to 0 on
//! the pool, so `Q`'s part in the frame is dependent in the signed frame matroid `N_h^σ` (Lemma 5 of the study).  The
//! *column-sum signing* (Lemma 4) values a column with `k = 2a + 3b` listers (`b = 1` when `k` is odd) by `(1, 2)` on
//! each of `a` pairs and `(1, 1, 1)` on the triple.  [`merge`] puts a core into a signing that already serves other
//! cores, changing as few of their values as it takes to keep every column of every one of them summing to 0.

use crate::gf3::{det3, rank3, words_for, Signing, V3};
use std::collections::{BTreeMap, HashSet};
use taxman::bits::Bits;
use taxman::model::{FrameGraph, FrameSet};

// ---------------------------------------------------------------------------------------------------------
// Frames with a set applied, and their residuals

/// The live factors of the mover `u` over `pool`: its maximal factors that are in the pool, ascending.
fn live<'a, const W: usize>(model: &'a FrameGraph, u: u32, pool: &'a Bits<W>) -> impl Iterator<Item = u32> + 'a {
    model.graph().factors(u).iter().map(|&f| u32::from(f)).filter(move |&f| pool.contains(f))
}

/// Whether `f` is one of the maximal factors of the mover `u`.
fn lists(model: &FrameGraph, u: u32, f: u32) -> bool {
    model.graph().factors(u).iter().any(|&g| u32::from(g) == f)
}

/// Frame h of `state` with the candidates `q` applied: its movers (ascending) and its pool.  A member of `q` of home
/// `h + 1` is a mover if it is still a factor of frame `h + 1`; the members of `q` of home `h` leave the pool.
pub fn frame_with<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, q: &[u32], h: usize) -> (Vec<u32>, Bits<W>) {
    let frame = &state.frames()[h];
    let into: Bits<W> =
        q.iter().copied().filter(|&x| model.home(x) == h + 1 && model.contains_factor(state, x)).collect();
    let out: Bits<W> = q.iter().copied().filter(|&x| model.home(x) == h).collect();
    ((frame.moves | into).to_vec(), frame.factors - out)
}

/// The residual of the peel of `movers` (ascending) over `pool`: the largest self-covering subset, ascending too.
/// Each pool factor keeps the count of the remaining movers that list it and the XOR of their numbers, so that a
/// factor with count 1 names its mover, as in the crate's peel.
pub fn residual<const W: usize>(model: &FrameGraph, movers: &[u32], pool: &Bits<W>) -> Vec<u32> {
    let mut cells = vec![(0u32, 0u32); 64 * W];
    for &u in movers {
        for f in live(model, u, pool) {
            let (count, xor) = &mut cells[f as usize];
            *count += 1;
            *xor ^= u;
        }
    }
    let mut private: Vec<u32> = pool.iter().filter(|&f| cells[f as usize].0 == 1).collect();
    let mut dropped = Bits::<W>::EMPTY;
    while let Some(f) = private.pop() {
        let (count, u) = cells[f as usize];
        if count != 1 {
            continue;
        }
        dropped.insert(u);
        for g in live(model, u, pool) {
            let (count, xor) = &mut cells[g as usize];
            *count -= 1;
            *xor ^= u;
            if *count == 1 {
                private.push(g);
            }
        }
    }
    movers.iter().copied().filter(|&u| !dropped.contains(u)).collect()
}

/// A minimal self-covering subset of the self-covering `set` (ascending) over `pool`: C := set; for each member u of
/// the original set in increasing order, if u is still in C and the residual of C − u is nonempty, C := that residual.
/// No member of the result can then be removed so as to leave a nonempty residual: when u was tried, the residual of
/// C − u was empty, and what C became is a subset of that C.
pub fn minimal<const W: usize>(model: &FrameGraph, set: &[u32], pool: &Bits<W>) -> Vec<u32> {
    let mut core = set.to_vec();
    for &u in set {
        if core.binary_search(&u).is_ok() {
            let rest: Vec<u32> = core.iter().copied().filter(|&x| x != u).collect();
            let inner = residual(model, &rest, pool);
            if !inner.is_empty() {
                core = inner;
            }
        }
    }
    core
}

// ---------------------------------------------------------------------------------------------------------
// Cores

/// A self-covering set of movers of one frame with a set Q applied, and its columns.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Core {
    /// The index of the frame.
    pub frame: usize,
    /// The movers, ascending: moves of the frame and members of Q of home frame + 1.
    pub members: Vec<u32>,
    /// Each pool factor listed by a member (ascending), with its listers among the members (ascending, ≥ 2).
    pub columns: Vec<(u32, Vec<u32>)>,
    /// The members that are Q's members promoted into the frame (home frame + 1), ascending.
    pub promoted_in: Vec<u32>,
    /// Q's members promoted out of the frame (home frame, so not in the pool) that some member lists, ascending.
    pub promoted_out: Vec<u32>,
}

impl Core {
    /// The listers of the column `f`, if the core has one.
    pub fn listers(&self, f: u32) -> Option<&[u32]> {
        let i = self.columns.binary_search_by_key(&f, |column| column.0).ok()?;
        Some(&self.columns[i].1)
    }
}

/// The core made of the self-covering set `members` (ascending) of frame h over `pool`, with `q` (the set applied)
/// giving its promoted members.  Panics if `members` is not self-covering.
pub fn core_of<const W: usize>(
    model: &FrameGraph,
    state: &FrameSet<W>,
    h: usize,
    members: Vec<u32>,
    pool: &Bits<W>,
    q: &[u32],
) -> Core {
    debug_assert!(members.windows(2).all(|w| w[0] < w[1]), "the members of a core ascend");
    let mut listers: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &u in &members {
        for f in live(model, u, pool) {
            listers.entry(f).or_default().push(u);
        }
    }
    assert!(listers.values().all(|l| l.len() >= 2), "every live factor of a core is listed by two members");
    let frame = &state.frames()[h];
    let promoted_in = members.iter().copied().filter(|&u| !frame.moves.contains(u) && q.contains(&u)).collect();
    let listed = |x: u32| members.iter().any(|&u| lists(model, u, x));
    let mut promoted_out: Vec<u32> =
        q.iter().copied().filter(|&x| frame.factors.contains(x) && !pool.contains(x) && listed(x)).collect();
    promoted_out.sort_unstable();
    promoted_out.dedup();
    Core { frame: h, members, columns: listers.into_iter().collect(), promoted_in, promoted_out }
}

/// The breaking candidates of `core`, ascending: the members of Q that it depends on.  Un-applying one destroys the
/// core: either it is a mover of the core (`promoted_in`), or it is a factor that exactly one member lists
/// (`promoted_out`), which that member would then have for itself.
pub fn breaking(model: &FrameGraph, core: &Core) -> Vec<u32> {
    let listed_once = |&x: &u32| core.members.iter().filter(|&&u| lists(model, u, x)).count() == 1;
    let mut candidates: Vec<u32> =
        core.promoted_in.iter().copied().chain(core.promoted_out.iter().copied().filter(listed_once)).collect();
    candidates.sort_unstable();
    candidates
}

/// The cores of `q` in `state`: for each frame of the state (all frames) whose frame-with-q has a nonempty residual,
/// up to `per_frame` minimal cores, found as follows.  R := the residual; repeat: C := minimal(R); record core_of(C);
/// then *un-apply* the lightest of C's *breaking candidates* ([`breaking`]) and recompute R as the residual of the
/// frame with all the un-applications so far (they accumulate); stop when R is empty or when `per_frame` cores are
/// recorded.  C has a breaking candidate, since the state's frames are winnable, so a core depends on Q (asserted).
/// Un-applying a breaking candidate destroys C, so the cores recorded for a frame are distinct.  The cores found after
/// an un-application are cores of a subset of q, so of q's sets too.
pub fn cores_of<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, q: &[u32], per_frame: usize) -> Vec<Core> {
    let mut cores = Vec::new();
    for h in 0..state.len() {
        let mut applied = q.to_vec();
        for _ in 0..per_frame {
            let (movers, pool) = frame_with(model, state, &applied, h);
            let stuck = residual(model, &movers, &pool);
            if stuck.is_empty() {
                break;
            }
            let core = core_of(model, state, h, minimal(model, &stuck, &pool), &pool, &applied);
            let lightest = *breaking(model, &core).first().expect("a core of a winnable frame depends on q");
            applied.retain(|&x| x != lightest);
            cores.push(core);
        }
    }
    cores
}

// ---------------------------------------------------------------------------------------------------------
// What a signing makes of a core

/// The sum mod 3 of the values `s` gives the incidences (u, f) of the `listers` of the column `f`.
fn column_sum(s: &Signing, f: u32, listers: &[u32]) -> u32 {
    listers.iter().map(|&u| u32::from(s.value(u, f))).sum::<u32>() % 3
}

/// Whether every column of `core` sums to 0 under `s`.
pub fn column_sums_zero(s: &Signing, core: &Core) -> bool {
    core.columns.iter().all(|(f, listers)| column_sum(s, *f, listers) == 0)
}

/// The members × columns matrix of `core`, a row per member: `value(u, f)` where member u lists column f, else 0.
fn matrix(core: &Core, value: impl Fn(u32, u32) -> u8) -> Vec<V3> {
    let mut rows = vec![V3::zero(words_for(core.columns.len())); core.members.len()];
    for (j, (f, listers)) in core.columns.iter().enumerate() {
        for &u in listers {
            let i = core.members.binary_search(&u).expect("a lister is a member");
            rows[i].set(j, value(u, *f));
        }
    }
    rows
}

/// Whether `s` *sees* `core`: its members' vectors restricted to its columns (member u has value s.value(u, f) at
/// each column f it lists) are dependent over GF(3) (rank below the number of members).
pub fn sees(s: &Signing, core: &Core) -> bool {
    rank3(&matrix(core, |u, f| s.value(u, f)), core.columns.len()) < core.members.len()
}

/// The 0/1 matrix of `core` (members × columns) over GF(3): its determinant mod 3 if square (as many columns as
/// members), and its rank deficiency (members − rank).
pub fn zero_one(core: &Core) -> (Option<u8>, usize) {
    let rows = matrix(core, |_, _| 1);
    let (members, columns) = (core.members.len(), core.columns.len());
    ((members == columns).then(|| det3(&rows)), members - rank3(&rows, columns))
}

/// The distinct primes dividing `n`, ascending.
fn prime_divisors(mut n: u32) -> Vec<u32> {
    let mut primes = Vec::new();
    let mut p = 2;
    while p * p <= n {
        if n.is_multiple_of(p) {
            primes.push(p);
            while n.is_multiple_of(p) {
                n /= p;
            }
        }
        p += 1;
    }
    if n > 1 {
        primes.push(n);
    }
    primes
}

/// The number of distinct primes dividing the members of `core`.
pub fn primes(core: &Core) -> usize {
    core.members.iter().flat_map(|&u| prime_divisors(u)).collect::<HashSet<_>>().len()
}

// ---------------------------------------------------------------------------------------------------------
// Merging cores into a signing

/// The most incidence variables `merge` searches over for one factor: 2^12 assignments.
pub const MAX_VARS: usize = 12;

/// The column-sum values of a column with `k ≥ 2` listers, in their increasing order: `k = 2a + 3b` with `b = 1` when
/// `k` is odd, `(1, 2)` on each of `a` pairs and then `(1, 1, 1)` on the `b` triple.
fn column_sum_values(k: usize) -> Vec<u8> {
    assert!(k >= 2, "a column is listed at least twice");
    let triples = k % 2;
    let mut values = [1, 2].repeat((k - 3 * triples) / 2);
    values.extend([1, 1, 1].repeat(triples));
    values
}

/// The values in {1, 2} of the variables `vars` (ascending) that make every equation, the listers of one column,
/// sum to 0, with the fewest changes from the `current` values and then the lexicographically least; none if no
/// assignment does.  An assignment is the bits of its values minus 1, the first variable the most significant, so that
/// the numeric order of assignments is their lexicographic order.
fn nearest_values(vars: &[u32], current: &[u8], equations: &[&[u32]]) -> Option<Vec<u8>> {
    let n = vars.len();
    let bit = |i: usize| 1u32 << (n - 1 - i);
    let masks: Vec<u32> = equations
        .iter()
        .map(|listers| listers.iter().map(|u| bit(vars.binary_search(u).expect("a lister is a variable"))).sum())
        .collect();
    let now: u32 = current.iter().enumerate().filter(|&(_, &v)| v == 2).map(|(i, _)| bit(i)).sum();
    // An equation of `m.count_ones()` listers sums to that plus the number of its listers valued 2.
    let sums_to_zero = |a: &u32| masks.iter().all(|&m| (m.count_ones() + (a & m).count_ones()).is_multiple_of(3));
    let nearest = (0..1u32 << n).filter(sums_to_zero).min_by_key(|&a| ((a ^ now).count_ones(), a))?;
    Some((0..n).map(|i| 1 + u8::from(nearest & bit(i) != 0)).collect())
}

/// Values for the incidences (u, f) of column `f`: the variables are the listers in any of the `equations` (the
/// listers of the column in the core and in every assigned core that has it), and each equation must sum to 0.  The
/// current values if they do; else, with the core's column the only equation, the column-sum values; else the nearest
/// satisfying assignment, if there are at most `MAX_VARS` variables.
fn column_values(s: &Signing, f: u32, equations: &[&[u32]]) -> Option<Vec<((u32, u32), u8)>> {
    let mut vars: Vec<u32> = equations.iter().flat_map(|listers| listers.iter().copied()).collect();
    vars.sort_unstable();
    vars.dedup();
    let current: Vec<u8> = vars.iter().map(|&u| s.value(u, f)).collect();
    let values = if equations.iter().all(|listers| column_sum(s, f, listers) == 0) {
        current
    } else if let [listers] = equations {
        column_sum_values(listers.len())
    } else if vars.len() <= MAX_VARS {
        nearest_values(&vars, &current, equations)?
    } else {
        return None;
    };
    Some(vars.into_iter().zip(values).map(|(u, v)| ((u, f), v)).collect())
}

/// Tries to make `core` seen by `s` (a mergeable signing of the core's frame; panic otherwise) without breaking the
/// cores already assigned to it (`assigned`, the cores whose ids are in `s.cores`, given by the caller).  Factor by
/// factor (the variables of different factors are disjoint): for each column f of `core`, the variables are the
/// incidences (u, f) of its listers and of the listers of f in every assigned core that has a column f; the equations
/// are those columns' sums.  If the current values (`s.value`) satisfy them, keep them.  Else, if no assigned core has
/// a column f, use the column-sum construction on the core's column (no limit on its size).  Else search {1, 2}^vars,
/// at most `MAX_VARS` variables (more: incompatible), in order of the number of values changed from the current ones
/// (then lexicographically), and take the first assignment that satisfies all the equations.  If every column of
/// `core` gets values, write all of them into `s.fixed` (every variable of every column of `core`, changed or not)
/// and return true; otherwise leave `s` unchanged and return false.  The caller pushes the core's id into `s.cores`
/// on success.
pub fn merge(s: &mut Signing, core: &Core, assigned: &[&Core]) -> bool {
    assert!(s.mergeable, "cores are merged into mergeable signings only");
    assert_eq!(s.frame, core.frame, "a core of another frame than the signing's");
    let mut values = Vec::new();
    for (f, listers) in &core.columns {
        let equations: Vec<&[u32]> =
            std::iter::once(listers.as_slice()).chain(assigned.iter().filter_map(|c| c.listers(*f))).collect();
        match column_values(s, *f, &equations) {
            Some(column) => values.extend(column),
            None => return false,
        }
    }
    s.fixed.extend(values);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::{setup, Rng};
    use crate::gf3::{seed, Default3, Matroid3};
    use std::collections::{BTreeSet, HashMap};
    use std::ops::Range;
    use taxman::model::Scratch;
    use taxman::search::Prepared;

    /// The set width of the boards of these tests: numbers up to 511.
    const WIDTH: usize = 8;

    /// A board set up for a search, with its candidates ascending.
    struct Case {
        n: u32,
        p: Prepared<WIDTH>,
        cands: Vec<u32>,
    }

    impl Case {
        fn new(n: u32) -> Option<Case> {
            let (_, p) = setup::<WIDTH>(n)?;
            let cands = p.candidates.to_vec();
            Some(Case { n, p, cands })
        }

        fn model(&self) -> &FrameGraph {
            &self.p.model
        }

        fn frames(&self) -> Range<usize> {
            0..self.p.root.len()
        }

        /// A generator seeded from the board and a salt.
        fn rng(&self, salt: u64) -> Rng {
            Rng(0x9E37_79B9_7F4A_7C15 ^ salt.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ u64::from(self.n))
        }

        /// A random subset of the candidates, each in with probability 1 / `one_in`.
        fn subset(&self, rng: &mut Rng, one_in: u64) -> Vec<u32> {
            self.cands.iter().copied().filter(|_| rng.below(one_in) == 0).collect()
        }

        /// A random subset (each candidate in with probability 1/4) that cannot be promoted; none if 200 draws fail.
        fn unpromotable(&self, rng: &mut Rng, scratch: &mut Scratch<WIDTH>) -> Option<Vec<u32>> {
            (0..200)
                .map(|_| self.subset(rng, 4))
                .find(|q| !self.model().can_promote_all_using(scratch, &self.p.root, &bits(q)))
        }

        /// A maximal promotable set: the candidates in a random order, each kept if it can still be promoted.
        fn maximal_promotable(&self, rng: &mut Rng) -> Vec<u32> {
            let mut order = self.cands.clone();
            shuffle(rng, &mut order);
            let mut state = self.p.root;
            let mut set: Vec<u32> =
                order.into_iter().filter(|&x| self.model().promote_in_place(&mut state, x)).collect();
            set.sort_unstable();
            set
        }
    }

    fn bits(set: &[u32]) -> Bits<WIDTH> {
        set.iter().copied().collect()
    }

    fn shuffle<T>(rng: &mut Rng, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, rng.below(i as u64 + 1) as usize);
        }
    }

    /// At most `k` of `items`, evenly spread through them.
    fn spread<T: Clone>(items: Vec<T>, k: usize) -> Vec<T> {
        if items.len() <= k {
            return items;
        }
        (0..k).map(|i| items[i * items.len() / k].clone()).collect()
    }

    /// How many members of `set` list each live factor, by counting afresh.
    fn listings(model: &FrameGraph, set: &[u32], pool: &Bits<WIDTH>) -> HashMap<u32, usize> {
        let mut counts = HashMap::new();
        for &u in set {
            for f in live(model, u, pool) {
                *counts.entry(f).or_default() += 1;
            }
        }
        counts
    }

    fn is_self_covering(model: &FrameGraph, set: &[u32], pool: &Bits<WIDTH>) -> bool {
        listings(model, set, pool).values().all(|&count| count >= 2)
    }

    /// The peel by its definition: drop a mover with a live factor that no other remaining mover lists, until none.
    fn naive_residual(model: &FrameGraph, movers: &[u32], pool: &Bits<WIDTH>) -> Vec<u32> {
        let mut left = movers.to_vec();
        loop {
            let counts = listings(model, &left, pool);
            let private = left.iter().position(|&u| live(model, u, pool).any(|f| counts[&f] == 1));
            match private {
                Some(i) => left.remove(i),
                None => return left,
            };
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // 1. The peel

    #[test]
    fn the_residual_is_empty_exactly_when_the_frame_is_winnable() {
        let (mut frames, mut stuck_frames) = (0, 0);
        for n in (60..=400).step_by(7) {
            let Some(case) = Case::new(n) else { continue };
            let (model, root) = (case.model(), &case.p.root);
            let (mut rng, mut scratch) = (case.rng(1), Scratch::<WIDTH>::new());
            for k in 0..50 {
                let q = case.subset(&mut rng, [2, 4, 8][k % 3]);
                for h in case.frames() {
                    let (movers, pool) = frame_with(model, root, &q, h);
                    let stuck = residual(model, &movers, &pool);
                    frames += 1;
                    let winnable = model.winnable_using(&mut scratch, &bits(&movers), &pool);
                    assert_eq!(stuck.is_empty(), winnable, "n = {n}, frame {h}, q = {q:?}");
                    assert_eq!(stuck, naive_residual(model, &movers, &pool), "n = {n}, frame {h}, q = {q:?}");
                    if stuck.is_empty() {
                        continue;
                    }
                    stuck_frames += 1;
                    assert!(is_self_covering(model, &stuck, &pool), "n = {n}, frame {h}: the residual");
                    let core = minimal(model, &stuck, &pool);
                    assert!(!core.is_empty() && core.iter().all(|u| stuck.contains(u)), "a subset of the residual");
                    assert!(is_self_covering(model, &core, &pool), "n = {n}, frame {h}: the minimal core");
                    for &u in &core {
                        let rest: Vec<u32> = core.iter().copied().filter(|&x| x != u).collect();
                        assert!(residual(model, &rest, &pool).is_empty(), "n = {n}, frame {h}: {u} is not needed");
                    }
                }
            }
        }
        eprintln!("peel: {stuck_frames} of {frames} frames with a random q have a nonempty residual");
        assert!(stuck_frames > 1_000, "only {stuck_frames} unwinnable frames were met");
    }

    // -----------------------------------------------------------------------------------------------------
    // 2. Lemmas 5 and 6

    /// The column-sum signing of `core`: a fresh signing that `merge` puts it into.
    fn column_sum_signing(core: &Core, seed: u64) -> Signing {
        let mut s = Signing::for_cores(core.frame, Default3::Random(seed));
        assert!(merge(&mut s, core, &[]), "a fresh signing takes any core");
        s
    }

    /// What `core`, a core of the candidates `q` on the board, must be, and its column-sum signing, which sees it
    /// and makes `q` dependent in the core's frame.
    fn check_core(case: &Case, q: &[u32], core: &Core, signing_seed: u64) -> Signing {
        let (model, root) = (case.model(), &case.p.root);
        let frame = &root.frames()[core.frame];
        assert!(core.members.windows(2).all(|w| w[0] < w[1]), "ascending members");
        for (f, listers) in &core.columns {
            assert!(listers.len() >= 2, "column {f} is listed once");
            assert!(listers.windows(2).all(|w| w[0] < w[1]), "ascending listers");
            assert!(listers.iter().all(|&u| core.members.contains(&u) && lists(model, u, *f)), "column {f}");
        }
        // Every factor a member lists is a column, or leaves the pool because it is promoted out.
        for &u in &core.members {
            for &g in model.graph().factors(u) {
                let f = u32::from(g);
                let accounted =
                    core.listers(f).is_some() || !frame.factors.contains(f) || core.promoted_out.contains(&f);
                assert!(accounted, "the member {u} lists {f}, which is not live");
            }
        }
        assert!(core.promoted_in.iter().all(|&u| q.contains(&u) && model.home(u) == core.frame + 1));
        assert!(core.members.iter().all(|u| core.promoted_in.contains(u) || frame.moves.contains(*u)));
        assert!(core.promoted_out.iter().all(|&x| q.contains(&x) && frame.factors.contains(x)));
        assert!(!breaking(model, core).is_empty(), "a core of frame {} depends on nothing in q", core.frame);

        let s = column_sum_signing(core, signing_seed);
        assert!(column_sums_zero(&s, core), "column sums, frame {}", core.frame);
        assert!(sees(&s, core), "the signing sees the core of frame {}", core.frame);
        let matroid = Matroid3::build(model, root, &case.cands, &s);
        assert!(!matroid.independent(q), "n = {}: q is independent in frame {}", case.n, core.frame);
        s
    }

    /// What [`check_unpromotable_sets`] met.
    struct Checked {
        sets: usize,
        cores: usize,
        /// At most 20 of the column-sum signings made, spread through them.
        signings: Vec<Signing>,
    }

    /// Lemmas 5 and 6 on `sets` random unpromotable sets of the board: for each, in every frame with a nonempty
    /// residual, the residual and the cores of `cores_of` are cores whose column-sum signings see them and make the
    /// set dependent.
    fn check_unpromotable_sets(case: &Case, sets: usize) -> Checked {
        let (model, root) = (case.model(), &case.p.root);
        let (mut rng, mut scratch) = (case.rng(2), Scratch::<WIDTH>::new());
        let mut checked = Checked { sets: 0, cores: 0, signings: Vec::new() };
        let mut signings = Vec::new();
        for _ in 0..sets {
            let Some(q) = case.unpromotable(&mut rng, &mut scratch) else { break };
            let mut cores = cores_of(model, root, &q, 4);
            let found: BTreeSet<usize> = cores.iter().map(|c| c.frame).collect();
            let mut stuck_frames = BTreeSet::new();
            for h in case.frames() {
                let (movers, pool) = frame_with(model, root, &q, h);
                let stuck = residual(model, &movers, &pool);
                if !stuck.is_empty() {
                    stuck_frames.insert(h);
                    cores.push(core_of(model, root, h, stuck, &pool, &q));
                }
            }
            assert_eq!(found, stuck_frames, "n = {}: the frames with cores", case.n);
            assert!(!found.is_empty(), "n = {}: an unpromotable set with every frame winnable", case.n);
            for core in &cores {
                signings.push(check_core(case, &q, core, seed(case.n, core.frame, checked.cores, 0xC0)));
                checked.cores += 1;
            }
            checked.sets += 1;
        }
        checked.signings = spread(signings, 20);
        checked
    }

    #[test]
    fn column_sum_signings_see_the_cores_and_make_q_dependent() {
        let boards: Vec<Case> = (60..=400).step_by(7).filter_map(Case::new).collect();
        let mut missing = 2_000usize;
        let (mut sets, mut cores) = (0, 0);
        for (i, case) in boards.iter().enumerate() {
            let checked = check_unpromotable_sets(case, missing.div_ceil(boards.len() - i));
            missing -= checked.sets.min(missing);
            sets += checked.sets;
            cores += checked.cores;
        }
        eprintln!("lemmas 5-6: {sets} unpromotable sets on {} boards, {cores} cores", boards.len());
        assert!(sets >= 2_000, "only {sets} unpromotable sets were found");
    }

    // -----------------------------------------------------------------------------------------------------
    // 3. Lemma 1 against those signings

    #[test]
    fn promotable_sets_are_independent_under_every_signing() {
        for n in (60..=400).step_by(7) {
            let Some(case) = Case::new(n) else { continue };
            let (model, root) = (case.model(), &case.p.root);
            let mut signings = check_unpromotable_sets(&case, 10).signings;
            for h in case.frames() {
                signings.push(Signing::ones(h));
                signings.extend((0..5).map(|i| Signing::random(h, seed(n, h, i, 0x51))));
            }
            let matroids: Vec<Matroid3> =
                signings.iter().map(|s| Matroid3::build(model, root, &case.cands, s)).collect();
            let mut rng = case.rng(3);
            for _ in 0..100 {
                let set = case.maximal_promotable(&mut rng);
                for (s, matroid) in signings.iter().zip(&matroids) {
                    assert!(matroid.independent(&set), "n = {n}, frame {}: the promotable set {set:?}", s.frame);
                }
            }
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // 4. Merging

    /// Merges the core `cores[id]` into `s` if `s` takes it, and checks that every core of `s` is then seen with its
    /// columns summing to 0; a failed merge must leave `s` as it was.
    fn try_merge(s: &mut Signing, id: usize, cores: &[Core]) -> bool {
        let before = s.fixed.clone();
        let assigned: Vec<&Core> = s.cores.iter().map(|&c| &cores[c]).collect();
        if !merge(s, &cores[id], &assigned) {
            assert_eq!(s.fixed, before, "a failed merge changed the signing");
            return false;
        }
        s.cores.push(id);
        for &c in &s.cores {
            assert!(column_sums_zero(s, &cores[c]), "core {c} after merging core {id}: column sums");
            assert!(sees(s, &cores[c]), "core {c} after merging core {id}: not seen");
        }
        true
    }

    /// Where first-fit merging put the cores.
    #[derive(Default)]
    struct Merges {
        /// Into the first signing, which was there already.
        first: usize,
        /// Into a later signing that was there already.
        later: usize,
        /// Into a signing opened for the core.
        opened: usize,
        /// Into none, with every signing open.
        failed: usize,
    }

    /// The distinct cores of `cores_of` over `sets` random unpromotable sets, in the frame that has the most.
    fn cores_of_the_busiest_frame(case: &Case, sets: usize) -> Vec<Core> {
        let (mut rng, mut scratch) = (case.rng(4), Scratch::<WIDTH>::new());
        let mut by_frame: Vec<Vec<Core>> = vec![Vec::new(); case.p.root.len()];
        let mut seen = HashSet::new();
        for _ in 0..sets {
            let Some(q) = case.unpromotable(&mut rng, &mut scratch) else { break };
            for core in cores_of(case.model(), &case.p.root, &q, 4) {
                if seen.insert(core.clone()) {
                    by_frame[core.frame].push(core);
                }
            }
        }
        by_frame.into_iter().max_by_key(Vec::len).unwrap_or_default()
    }

    #[test]
    fn cores_merge_into_a_few_signings() {
        const MAX_SIGNINGS: usize = 8;
        let (mut merges, mut boards, mut total) = (Merges::default(), 0, 0);
        for i in 0..40 {
            let Some(case) = Case::new(60 + 340 * i / 39) else { continue };
            let mut cores = cores_of_the_busiest_frame(&case, 400);
            let Some(h) = cores.first().map(|c| c.frame) else { continue };
            shuffle(&mut case.rng(5), &mut cores);
            boards += 1;
            total += cores.len();
            let mut signings: Vec<Signing> = Vec::new();
            for id in 0..cores.len() {
                match signings.iter_mut().position(|s| try_merge(s, id, &cores)) {
                    Some(0) => merges.first += 1,
                    Some(_) => merges.later += 1,
                    None if signings.len() < MAX_SIGNINGS => {
                        let mut s = Signing::for_cores(h, Default3::Random(seed(case.n, h, signings.len(), 0xE5)));
                        assert!(try_merge(&mut s, id, &cores), "n = {}: a fresh signing takes any core", case.n);
                        signings.push(s);
                        merges.opened += 1;
                    }
                    None => merges.failed += 1,
                }
            }
        }
        eprintln!(
            "merges: {total} cores on {boards} boards: {} into the first signing, {} into a later one, {} opened a \
             signing, {} failed everywhere",
            merges.first, merges.later, merges.opened, merges.failed
        );
        assert_eq!(merges.first + merges.later + merges.opened + merges.failed, total);
        assert!(merges.opened >= boards);
    }

    #[test]
    fn column_sum_values_sum_to_zero() {
        for k in 2..=40 {
            let values = column_sum_values(k);
            assert_eq!(values.len(), k);
            assert!(values.iter().all(|&v| v == 1 || v == 2));
            assert_eq!(values.iter().map(|&v| u32::from(v)).sum::<u32>() % 3, 0, "k = {k}");
        }
        assert_eq!(column_sum_values(2), [1, 2]);
        assert_eq!(column_sum_values(3), [1, 1, 1]);
        assert_eq!(column_sum_values(4), [1, 2, 1, 2]);
        assert_eq!(column_sum_values(5), [1, 2, 1, 1, 1]);
    }

    /// A core of frame 0 with nothing promoted, made by hand.
    fn hand_made(members: &[u32], columns: &[(u32, &[u32])]) -> Core {
        let columns = columns.iter().map(|&(f, listers)| (f, listers.to_vec())).collect();
        Core { frame: 0, members: members.to_vec(), columns, promoted_in: Vec::new(), promoted_out: Vec::new() }
    }

    /// A core whose members all list the one column 2.
    fn column_two(listers: impl IntoIterator<Item = u32>) -> Core {
        let listers: Vec<u32> = listers.into_iter().collect();
        hand_made(&listers, &[(2, &listers)])
    }

    #[test]
    fn merge_searches_the_columns_it_shares_with_assigned_cores() {
        let mut s = Signing::for_cores(0, Default3::Ones);
        let a = column_two([1, 2]);
        assert!(merge(&mut s, &a, &[]));
        assert_eq!((s.value(1, 2), s.value(2, 2)), (1, 2), "the column-sum values");

        // Valuing (3, 2) with 2 is the nearest change that gives column {1, 3} the sum 0 and keeps {1, 2}'s.
        let b = column_two([1, 3]);
        assert!(merge(&mut s, &b, &[&a]));
        assert_eq!((s.value(1, 2), s.value(2, 2), s.value(3, 2)), (1, 2, 2));
        assert!(column_sums_zero(&s, &a) && column_sums_zero(&s, &b));

        // The columns {1, 2}, {1, 3} and {2, 3} need three different values.
        let c = column_two([2, 3]);
        let before = s.fixed.clone();
        assert!(!merge(&mut s, &c, &[&a, &b]));
        assert_eq!(s.fixed, before);
    }

    #[test]
    fn merge_changes_the_last_lister_and_gives_up_beyond_max_vars() {
        let mut s = Signing::for_cores(0, Default3::Ones);
        let a = column_two(1..=5);
        assert!(merge(&mut s, &a, &[]));
        let values = |s: &Signing| (1..=12).map(|u| s.value(u, 2)).collect::<Vec<_>>();
        assert_eq!(values(&s), [1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], "the column-sum values, then the default");

        // Column {6..10} sums to 5: the nearest change is one 2 more, the lexicographically least on the last lister.
        let b = column_two(6..=10);
        assert!(merge(&mut s, &b, &[&a]));
        assert_eq!(values(&s), [1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1]);

        // Fourteen variables are beyond MAX_VARS: not searched, and nothing changes.
        let before = s.fixed.clone();
        assert!(!merge(&mut s, &column_two(6..=14), &[&a, &b]));
        assert_eq!(s.fixed, before);

        // Twelve are searched: column {6..12} sums to 8, and only 11 and 12 can change without breaking {6..10}.
        assert_eq!(MAX_VARS, 12);
        assert!(merge(&mut s, &column_two(6..=12), &[&a, &b]));
        assert_eq!(values(&s), [1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2]);
    }

    // -----------------------------------------------------------------------------------------------------
    // 5. Hand-made cores

    /// The integer determinant of a square matrix, by fraction-free elimination.
    fn integer_det(m: &[Vec<i64>]) -> i64 {
        let n = m.len();
        let mut m = m.to_vec();
        let (mut sign, mut previous) = (1, 1);
        for k in 0..n {
            let Some(pivot) = (k..n).find(|&r| m[r][k] != 0) else { return 0 };
            if pivot != k {
                m.swap(pivot, k);
                sign = -sign;
            }
            for i in k + 1..n {
                for j in k + 1..n {
                    m[i][j] = (m[i][j] * m[k][k] - m[i][k] * m[k][j]) / previous;
                }
            }
            previous = m[k][k];
        }
        sign * m[n - 1][n - 1]
    }

    /// The 0/1 matrix of `core` over the integers.
    fn incidence(core: &Core) -> Vec<Vec<i64>> {
        core.members
            .iter()
            .map(|u| core.columns.iter().map(|(_, listers)| i64::from(listers.contains(u))).collect())
            .collect()
    }

    #[test]
    fn the_core_that_gf2_misses_is_seen_over_gf3() {
        let core = hand_made(
            &[12, 18, 20, 30, 45],
            &[(4, &[12, 20]), (6, &[12, 18, 30]), (9, &[18, 45]), (10, &[20, 30]), (15, &[30, 45])],
        );
        assert_eq!(integer_det(&incidence(&core)).abs(), 3);
        assert_eq!(zero_one(&core), (Some(0), 1));
        assert_eq!(primes(&core), 3);
        assert!(sees(&Signing::ones(0), &core), "all ones sees it over GF(3)");
        let s = column_sum_signing(&core, 1);
        assert!(column_sums_zero(&s, &core) && sees(&s, &core));
    }

    #[test]
    fn the_triangle_needs_its_column_sum_signing() {
        let core = hand_made(&[6, 10, 15], &[(2, &[6, 10]), (3, &[6, 15]), (5, &[10, 15])]);
        assert_eq!(integer_det(&incidence(&core)).abs(), 2);
        let (det, deficiency) = zero_one(&core);
        assert_eq!((det, deficiency), (Some(integer_det(&incidence(&core)).rem_euclid(3) as u8), 0));
        assert_ne!(det, Some(0));
        assert_eq!(primes(&core), 3);
        assert!(!sees(&Signing::ones(0), &core), "all ones does not see it");
        let s = column_sum_signing(&core, 1);
        assert!(column_sums_zero(&s, &core) && sees(&s, &core));
    }
}
