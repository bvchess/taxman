//! The prices of conflict branching, kept from one choice to the next (the tactic `price_reuse`).
//!
//! Conflict branching ([`conflict_to_branch_on`](super::searcher)) prices the members of a node's heaviest set at every
//! check: what leaving each out costs the bound, read off the heaviest set's exchange graph.  A node's loop checks it
//! again at every step, and a step changes the node little (it goes on without the member it branched on); a child's
//! first check follows its parent's, and its problem is the parent's with some members promoted.  So a price made at one
//! check is nearly the price at the next, and the choice only ranks conflicts by it: any ranking keeps the search exact,
//! since every conflict it can pick is a true conflict of the node.  A [`PriceMemo`] keeps the prices of each node on
//! the searcher's path and lets the choice take them instead of making them again, making only the prices of members
//! it has none for, and all of them again when the schedule ([`Reuse`]) says they are too old.

use crate::bits::Bits;
use std::collections::HashMap;

/// When a [`PriceMemo`] lets a choice reuse the prices it holds.  Any schedule keeps the search exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reuse {
    /// The most choices at one node that use one making of its prices, the one that makes them included.
    pub max_steps: u32,
    /// The most the node's bound may have fallen since its prices were made for the node to take them again.
    pub own_drop: u64,
    /// The most generations of nodes down a line that take their prices from the parent's, one making of them at its
    /// top (0: none does, and every node makes its own at its first check).
    pub max_seeds: u32,
    /// The most the bound may have fallen from the parent's making of the prices for a child to take them.
    pub seed_drop: u64,
}

impl Reuse {
    /// The schedule of the tactic.
    pub const STANDARD: Reuse = Reuse { max_steps: u32::MAX, own_drop: u64::MAX, max_seeds: 6, seed_drop: 40 };
}

/// What the choices at one depth keep of a node.
struct Entry<const W: usize> {
    /// The GF(2) bound's number of the node the prices are for (0: none).
    node: u64,
    /// The node's bound (the sum of its promotions and the GF(2) bound of the rest) when the prices were made.
    value: u64,
    /// The choices of the node that have used them.
    steps: u32,
    /// How many nodes down a line of seeds the prices were made (0: by this node).
    generation: u32,
    /// What leaving each member out costs the bound (`None`: the heaviest set's exchange graph does not certify it).
    losses: HashMap<u32, Option<u64>>,
    /// The ordered conflicts of the last choice that made them, and whether they are held.
    conflicts: Vec<Bits<W>>,
    ordered: bool,
}

impl<const W: usize> Default for Entry<W> {
    fn default() -> Self {
        Entry {
            node: 0,
            value: 0,
            steps: 0,
            generation: 0,
            losses: HashMap::new(),
            conflicts: Vec::new(),
            ordered: false,
        }
    }
}

/// The prices kept for the nodes on a searcher's path, one entry for each depth.
#[derive(Default)]
pub(super) struct PriceMemo<const W: usize> {
    entries: Vec<Entry<W>>,
}

/// What the memo gives a choice.
pub(super) struct Taken<'m, const W: usize> {
    /// The members already priced, with what leaving each out costs the bound.
    pub losses: &'m mut HashMap<u32, Option<u64>>,
    /// The ordered conflicts of the last choice that are within the heaviest set, if any: each is a conflict of the
    /// node's state, since the promotions of the nodes along a line only grow.
    pub conflicts: Option<Vec<Bits<W>>>,
}

impl<const W: usize> PriceMemo<W> {
    /// The memo for the choice at the node `node` (child of `parent`) at `depth` with the bound `value`, and the heaviest
    /// set `chosen`: the node's entry if it holds prices that may be taken again, else one made from the parent's, if the
    /// schedule allows, else an empty one for the choice to fill.  The choice is counted in the entry's steps.
    pub(super) fn take(
        &mut self,
        depth: usize,
        node: u64,
        parent: u64,
        value: u64,
        chosen: &Bits<W>,
        reuse: &Reuse,
    ) -> Taken<'_, W> {
        if self.entries.len() <= depth {
            self.entries.resize_with(depth + 1, Entry::default);
        }
        let (above, here) = self.entries.split_at_mut(depth);
        let entry = &mut here[0];
        let own = entry.node == node;
        let mine = own && entry.steps < reuse.max_steps && entry.value.saturating_sub(value) <= reuse.own_drop;
        if !mine {
            let seed = above.last().filter(|p| {
                depth > 0
                    && !own
                    && p.node == parent
                    && p.generation < reuse.max_seeds
                    && p.value.saturating_sub(value) <= reuse.seed_drop
            });
            match seed {
                Some(p) => {
                    entry.losses.clone_from(&p.losses);
                    entry.conflicts.clone_from(&p.conflicts);
                    entry.ordered = p.ordered;
                    (entry.node, entry.value, entry.generation) = (node, p.value, p.generation + 1);
                }
                None => {
                    entry.losses.clear();
                    entry.conflicts.clear();
                    entry.ordered = false;
                    (entry.node, entry.value, entry.generation) = (node, value, 0);
                }
            }
            entry.steps = 0;
        }
        entry.steps += 1;
        let conflicts = entry
            .ordered
            .then(|| entry.conflicts.iter().copied().filter(|k| k.is_subset(chosen)).collect::<Vec<Bits<W>>>());
        let conflicts = conflicts.filter(|within| !within.is_empty());
        Taken { losses: &mut entry.losses, conflicts }
    }

    /// Keeps `conflicts` as the ordered conflicts of the choice at `depth`, the entry of which `take` made.
    pub(super) fn keep_conflicts(&mut self, depth: usize, conflicts: &[Bits<W>]) {
        let entry = &mut self.entries[depth];
        entry.conflicts.clear();
        entry.conflicts.extend_from_slice(conflicts);
        entry.ordered = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REUSE: Reuse = Reuse { max_steps: 3, own_drop: 10, max_seeds: 2, seed_drop: 10 };

    fn set(xs: &[u32]) -> Bits<1> {
        xs.iter().copied().collect()
    }

    /// Takes the entry for `node` at `depth`, fills it as a choice does if it is empty, and says how many prices it
    /// held on being taken.
    fn choose(memo: &mut PriceMemo<1>, depth: usize, node: u64, parent: u64, value: u64) -> usize {
        let taken = memo.take(depth, node, parent, value, &set(&[1, 2, 3]), &REUSE);
        let held = taken.losses.len();
        taken.losses.entry(1).or_insert(Some(5));
        taken.losses.entry(2).or_insert(None);
        held
    }

    #[test]
    fn a_node_takes_its_prices_until_they_are_old_or_its_bound_has_fallen() {
        let mut memo = PriceMemo::<1>::default();
        // Made at the first choice, taken at the next two (3 steps), made again at the fourth.
        let held: Vec<usize> = (0..4).map(|_| choose(&mut memo, 0, 7, 0, 100)).collect();
        assert_eq!(held, [0, 2, 2, 0]);
        // A fall of the bound within the limit keeps them; past it they are made again.
        assert_eq!(choose(&mut memo, 0, 7, 0, 95), 2);
        assert_eq!(choose(&mut memo, 0, 7, 0, 89), 0);
        // Another node at the depth starts empty, and so does the first node when it comes back.
        assert_eq!(choose(&mut memo, 0, 8, 0, 89), 0);
        assert_eq!(choose(&mut memo, 0, 7, 0, 89), 0);
    }

    #[test]
    fn a_child_starts_from_its_parents_prices_for_a_few_generations() {
        let mut memo = PriceMemo::<1>::default();
        assert_eq!(choose(&mut memo, 0, 1, 0, 100), 0);
        // A child and a grandchild take the prices made at the top; a great-grandchild, a third generation, makes
        // its own.
        assert_eq!(choose(&mut memo, 1, 2, 1, 100), 2);
        assert_eq!(choose(&mut memo, 2, 3, 2, 100), 2);
        assert_eq!(choose(&mut memo, 3, 4, 3, 100), 0);
        // A second child of the top starts from it again.
        assert_eq!(choose(&mut memo, 1, 5, 1, 100), 2);
        // A child whose parent is another node, or whose bound has fallen past the limit, starts empty.
        assert_eq!(choose(&mut memo, 1, 6, 99, 100), 0);
        assert_eq!(choose(&mut memo, 1, 7, 1, 80), 0);
        // And without seeds allowed.
        let own = Reuse { max_seeds: 0, ..REUSE };
        let taken = memo.take(1, 8, 1, 100, &set(&[1]), &own);
        assert!(taken.losses.is_empty());
    }

    #[test]
    fn ordered_conflicts_are_taken_while_within_the_heaviest_set() {
        let wide = Reuse { max_steps: 10, ..REUSE };
        let mut memo = PriceMemo::<1>::default();
        assert!(memo.take(0, 1, 0, 100, &set(&[1, 2, 3]), &wide).conflicts.is_none());
        memo.keep_conflicts(0, &[set(&[1, 2]), set(&[2, 3])]);
        let taken = memo.take(0, 1, 0, 100, &set(&[1, 2, 3, 4]), &wide);
        assert_eq!(taken.conflicts, Some(vec![set(&[1, 2]), set(&[2, 3])]));
        // One of them has a member the heaviest set has lost: only the other is taken.
        let taken = memo.take(0, 1, 0, 100, &set(&[1, 2, 4]), &wide);
        assert_eq!(taken.conflicts, Some(vec![set(&[1, 2])]));
        // None of them within it: none is taken, and the choice makes them.
        assert!(memo.take(0, 1, 0, 100, &set(&[4, 5]), &wide).conflicts.is_none());
        // A child takes its parent's.
        memo.keep_conflicts(0, &[set(&[1, 2]), set(&[2, 3])]);
        let taken = memo.take(1, 2, 1, 100, &set(&[2, 3, 5]), &wide);
        assert_eq!(taken.conflicts, Some(vec![set(&[2, 3])]));
    }
}
