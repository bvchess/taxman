//! Whether a set of numbers of a whole board can all be played, decided with nothing from the frame solver.
//!
//! The decision is the peel of heuristics/THEORY.md (Theorems 1 to 3): a set is playable, that is, some order of it is
//! a legal game on the board `1..=n`, exactly when the peel pairs every member with a maximal factor of its own,
//! whatever the order of the peel's moves.  The code uses only the tables of maximal factors and composites of
//! [`taxman::numtheory::Tables`], which are foundation code, and `std`: it shares nothing with `frames` or `model`, so
//! a test that decides playability here checks the frame model against the board and not against itself.
//!
//! [`peel`] gives the pairing, [`playable`] the answer, and [`play_order`] a legal order of the set.

use std::mem;
use taxman::numtheory::Tables;

/// A move of the peel.
enum Move {
    /// The member has just one live factor in its list, and takes it.
    Front(u32),
    /// The factor is listed by just one live member, and goes to it.
    Back(u32),
}

/// The peel of `set` on the board of `t`: the pairs `(member, payment)` in the order the peel made them if it pairs
/// every member, `None` if it stalls (some member is left with no live factor, or no move applies).
///
/// The members are `set` (distinct numbers of `2..=t.n`).  The pool is the maximal factors of the members that are not
/// members themselves, and each member's list is its maximal factors in the pool.  A *front move* pairs a live member
/// that has exactly one live (unpaired) factor in its list with that factor; a *back move* pairs a live factor that
/// exactly one live member lists with that member.  Theorems 2 and 3 say that the pairs are a payment scheme with an
/// acyclic precedence if the peel finishes, and that a playable set is never left stuck, so the set is playable
/// exactly when the peel finishes, in whatever order its moves are made.
///
/// Linear time in the size of the lists: per member the count of its live factors, per factor the count of the live
/// members that list it and the exclusive or of their numbers (so a factor with a count of one names its member), and
/// a stack of the moves that may apply.  The members that list a factor are found through the factor's maximal
/// composites.
///
/// # Panics
///
/// If a member is not on the board `2..=t.n`, or is in `set` twice.
pub fn peel(t: &Tables, set: &[u32]) -> Option<Vec<(u32, u32)>> {
    let size = t.n as usize + 1;
    let mut member = vec![false; size];
    for &m in set {
        assert!((2..=t.n).contains(&m), "{m} is not a number of the board 2..={}", t.n);
        assert!(!mem::replace(&mut member[m as usize], true), "{m} is in the set twice");
    }
    let factors_of = |m: u32| t.max_factors[m as usize].iter().copied().filter(|&f| !member[f as usize]);

    // The pool's side of the lists: how many live members list each factor, and the exclusive or of their numbers.
    let (mut live_factors, mut listers, mut owner) = (vec![0u32; size], vec![0u32; size], vec![0u32; size]);
    for &m in set {
        for f in factors_of(m) {
            live_factors[m as usize] += 1;
            listers[f as usize] += 1;
            owner[f as usize] ^= m;
        }
    }
    let mut moves = Vec::new();
    for &m in set {
        match live_factors[m as usize] {
            0 => return None,
            1 => moves.push(Move::Front(m)),
            _ => {}
        }
        moves.extend(factors_of(m).filter(|&f| listers[f as usize] == 1).map(Move::Back));
    }

    let (mut paired, mut taken) = (vec![false; size], vec![false; size]);
    let mut pairs = Vec::with_capacity(set.len());
    while let Some(next) = moves.pop() {
        // A move made earlier may have used this one up, or changed the counts it was pushed for.
        let (m, f) = match next {
            Move::Front(m) if !paired[m as usize] => {
                let live = factors_of(m).find(|&f| !taken[f as usize]);
                (m, live.expect("a member pushed for a front move has its one live factor"))
            }
            Move::Back(f) if !taken[f as usize] && listers[f as usize] == 1 => (owner[f as usize], f),
            _ => continue,
        };
        pairs.push((m, f));
        (paired[m as usize], taken[f as usize]) = (true, true);
        // The member leaves the lists of its other live factors.
        for g in factors_of(m).filter(|&g| !taken[g as usize]) {
            listers[g as usize] -= 1;
            owner[g as usize] ^= m;
            if listers[g as usize] == 1 {
                moves.push(Move::Back(g));
            }
        }
        // The factor leaves the lists of the other live members that have it.
        for &c in &t.max_composites[f as usize] {
            if member[c as usize] && !paired[c as usize] {
                live_factors[c as usize] -= 1;
                match live_factors[c as usize] {
                    0 => return None,
                    1 => moves.push(Move::Front(c)),
                    _ => {}
                }
            }
        }
    }
    (pairs.len() == set.len()).then_some(pairs)
}

/// Whether `set` can all be played in some order on the board of `t`: [`peel`] pairs every member.
pub fn playable(t: &Tables, set: &[u32]) -> bool {
    peel(t, set).is_some()
}

/// A legal game that plays exactly `set`, if it is playable: the topological order of the precedence of the peel's
/// pairs (Theorems 1 and 2), where `a` comes before `b` whenever the payment of `a` divides `b`, or `a` divides `b`
/// (and `a` is not `b`).  Quadratic in the size of the set.
///
/// # Panics
///
/// If the precedence has a cycle, which would contradict Theorem 2.
pub fn play_order(t: &Tables, set: &[u32]) -> Option<Vec<u32>> {
    let pairs = peel(t, set)?;
    let before = |a: usize, b: usize| {
        let ((first, payment), later) = (pairs[a], pairs[b].0);
        a != b && (later.is_multiple_of(payment) || later.is_multiple_of(first))
    };
    let mut waiting: Vec<usize> =
        (0..pairs.len()).map(|b| (0..pairs.len()).filter(|&a| before(a, b)).count()).collect();
    let mut ready: Vec<usize> = (0..pairs.len()).filter(|&b| waiting[b] == 0).collect();
    let mut order = Vec::with_capacity(pairs.len());
    while let Some(a) = ready.pop() {
        order.push(pairs[a].0);
        for b in (0..pairs.len()).filter(|&b| before(a, b)) {
            waiting[b] -= 1;
            if waiting[b] == 0 {
                ready.push(b);
            }
        }
    }
    assert_eq!(order.len(), pairs.len(), "the precedence of the peel's pairs has a cycle: {pairs:?}");
    Some(order)
}
