//! The table of nogoods: the minimal unplayable sets that a search has found.  A set of picks is unplayable whatever
//! else is picked, so a nogood holds at every node: no playable set picks all its members.
//!
//! A long search keeps up to tens of thousands of nogoods of about forty members each, and a node asks the table two
//! things, neither of which may read the whole table to answer for what the node changed:
//!
//! * **Propagation** ([`propagate`](Nogoods::propagate)).  A nogood with a barred member holds already; one whose
//!   members are all forced ends the node; one whose members but one are forced bars the last.  Each nogood watches
//!   two of its members that are not forced, as a SAT solver's clause does, and is looked at only when a member it
//!   watches becomes forced.  The watches are never undone.  The roles of a child are those of its parent with some
//!   members decided, and a member that is not forced at a node is not forced at the node's ancestors either, so the
//!   watches that fit a node fit its parent too, and fit it again when the next child is made from it.  A node
//!   therefore looks at the nogoods that watch the numbers it forces, and at the nogoods learned since its parent was
//!   propagated, whose watches were chosen without the parent's roles in mind.
//! * **The nogoods inside a set** of numbers ([`inside`](Nogoods::inside)), the conflicts a node can branch on.  The
//!   set is the heaviest set of the node, and the next node's differs from it in a few numbers, so the table keeps for
//!   each nogood the number of its members outside the set last asked about, and a query changes the counts of the
//!   nogoods that hold the numbers that left or entered.
//!
//! The members of each nogood stay in the order it was learned in (ascending): a node branches on the free members of
//! a nogood in that order.

use std::ops::Range;

use crate::game::Role;

const NONE: u32 = u32::MAX;

/// The sets of numbers that the search asks the nogoods inside of ([`Nogoods::inside`]), each of which changes a
/// little from one query to the next.
#[derive(Clone, Copy)]
pub enum Set {
    /// The set that a node branches on.
    Branching,
    /// The picks of the node's matching.
    Matching,
}

/// The nogoods found so far, in the order they were learned in.
#[derive(Default)]
pub struct Nogoods {
    /// The members of every nogood, one nogood after another, ascending in each.
    members: Vec<u32>,
    /// Where each nogood's members are in `members`.
    spans: Vec<(u32, u32)>,
    /// The two members each nogood watches.
    watch: Vec<[u32; 2]>,
    /// For each number, the nogoods that watch it.
    watchers: Vec<Vec<u32>>,
    /// For each number, the nogoods that hold it, ascending.
    occurrences: Vec<Vec<u32>>,
    /// The state of the questions about each [`Set`].
    questions: [Question; 2],
}

/// What is known of the nogoods inside the last set asked about.
#[derive(Default)]
struct Question {
    /// The set the counts are for.
    set: Vec<bool>,
    /// For the nogoods counted (the first ones), how many members each has outside `set`.
    outside: Vec<u16>,
    /// The nogoods with no member outside `set`, ascending.
    inside: Vec<u32>,
}

impl From<Vec<Vec<u32>>> for Nogoods {
    fn from(nogoods: Vec<Vec<u32>>) -> Self {
        let mut table = Nogoods::default();
        table.extend(nogoods);
        table
    }
}

impl Nogoods {
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// The members of nogood `i`.
    pub fn get(&self, i: usize) -> &[u32] {
        let (start, end) = self.spans[i];
        &self.members[start as usize..end as usize]
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &[u32]> + '_ {
        (0..self.len()).map(|i| self.get(i))
    }

    /// The nogoods as lists of members (for a checkpoint).
    pub fn to_vec(&self) -> Vec<Vec<u32>> {
        self.since(0)
    }

    /// The nogoods from the `from`-th on, as lists of members.
    pub fn since(&self, from: usize) -> Vec<Vec<u32>> {
        (from..self.len()).map(|i| self.get(i).to_vec()).collect()
    }

    /// Adds nogoods that were learned elsewhere (by another search of the same game), after those of the table.  They
    /// are examined whole at the next propagation of every node that was propagated before, like the nogoods learned
    /// by the search itself.
    pub fn extend(&mut self, nogoods: impl IntoIterator<Item = Vec<u32>>) {
        for k in nogoods {
            assert!(k.len() >= 2, "a nogood has two members or more: one number is always playable");
            self.add(&k, [k[0], k[1]]);
        }
    }

    /// Adds a nogood learned under `roles`, watching two of its members that are not forced.
    pub fn push(&mut self, k: &[u32], roles: &[Role]) {
        assert!(k.len() >= 2, "a nogood has two members or more: one number is always playable");
        let open = k.iter().filter(|&&c| roles[c as usize] != Role::Pick);
        let mut watched = open.chain(k.iter().filter(|&&c| roles[c as usize] == Role::Pick)).copied();
        let w = [watched.next().unwrap(), watched.next().unwrap()];
        self.add(k, w);
    }

    fn add(&mut self, k: &[u32], w: [u32; 2]) {
        assert!(k.len() <= usize::from(u16::MAX), "a nogood of {} members", k.len());
        let id = self.len() as u32;
        let start = self.members.len() as u32;
        self.members.extend_from_slice(k);
        self.spans.push((start, self.members.len() as u32));
        self.watch.push(w);
        let numbers = k.iter().max().map_or(0, |&c| c as usize + 1);
        if self.watchers.len() < numbers {
            self.watchers.resize_with(numbers, Vec::new);
        }
        if self.occurrences.len() < numbers {
            self.occurrences.resize_with(numbers, Vec::new);
        }
        w.iter().for_each(|&c| self.watchers[c as usize].push(id));
        k.iter().for_each(|&c| self.occurrences[c as usize].push(id));
    }

    /// The ids of the nogoods all of whose members are flagged in `set`, ascending.  The table remembers `set` for
    /// the next question about the same kind of set.
    pub fn inside(&mut self, which: Set, set: &[bool]) -> Vec<usize> {
        let q = &mut self.questions[which as usize];
        if q.set.len() != set.len() {
            *q = Question { set: vec![false; set.len()], ..Question::default() };
        }
        let counted = q.outside.len();
        // The nogoods counted already hold the numbers that entered or left.
        for (c, &entered) in set.iter().enumerate() {
            if q.set[c] == entered {
                continue;
            }
            q.set[c] = entered;
            let Some(held) = self.occurrences.get(c) else { continue };
            for &id in held.iter().take_while(|&&id| (id as usize) < counted) {
                let outside = &mut q.outside[id as usize];
                if entered {
                    *outside -= 1;
                    if *outside == 0 {
                        let at = q.inside.partition_point(|&i| i < id);
                        q.inside.insert(at, id);
                    }
                } else {
                    if *outside == 0 {
                        let at = q.inside.partition_point(|&i| i < id);
                        q.inside.remove(at);
                    }
                    *outside += 1;
                }
            }
        }
        // The nogoods learned since are counted afresh.
        for id in counted..self.spans.len() {
            let (start, end) = self.spans[id];
            let members = &self.members[start as usize..end as usize];
            let outside = members.iter().filter(|&&c| !set[c as usize]).count() as u16;
            q.outside.push(outside);
            if outside == 0 {
                q.inside.push(id as u32);
            }
        }
        let inside: Vec<usize> = q.inside.iter().map(|&id| id as usize).collect();
        debug_assert_eq!(
            inside,
            (0..self.len()).filter(|&i| self.get(i).iter().all(|&c| set[c as usize])).collect::<Vec<_>>(),
            "the counts of the members outside the set drifted"
        );
        inside
    }

    /// Nogood propagation to its fixpoint: false if some nogood has no member that is not forced; otherwise bars the
    /// last free member of every nogood whose other members are all forced.  `trail` are the numbers forced since the
    /// roles were at the fixpoint of the nogoods before `fresh`; every nogood of `fresh` (the nogoods learned since,
    /// or all of them where there is no earlier fixpoint) is examined whole.
    pub fn propagate(&mut self, roles: &mut [Role], trail: &[u32], fresh: Range<usize>) -> bool {
        if self.watchers.len() < roles.len() {
            self.watchers.resize_with(roles.len(), Vec::new);
        }
        for id in fresh {
            if !self.examine(id, roles) {
                return false;
            }
        }
        trail.iter().all(|&x| self.visit(x, roles))
    }

    /// What [`propagate`](Self::propagate) comes to, by reading every nogood once: false if some nogood has no member
    /// that is not forced; bars the last free member of every nogood whose other members are all forced, setting
    /// `changed` if it barred any.  The search checks the watches against it in debug builds.
    #[cfg(any(test, debug_assertions))]
    pub fn scan(&self, roles: &mut [Role], changed: &mut bool) -> bool {
        for k in self.iter() {
            if k.iter().any(|&c| roles[c as usize] == Role::Barred) {
                continue;
            }
            let mut free = k.iter().filter(|&&c| roles[c as usize] == Role::Free);
            match (free.next(), free.next()) {
                (None, _) => return false,
                (Some(&c), None) => {
                    roles[c as usize] = Role::Barred;
                    *changed = true;
                }
                _ => {}
            }
        }
        true
    }

    /// Fits nogood `id` to `roles` from scratch: false if all its members are forced; bars its last free member if the
    /// others are forced; otherwise leaves it watching two members that are not forced.
    fn examine(&mut self, id: usize, roles: &mut [Role]) -> bool {
        let [a, b] = self.watch[id];
        if roles[a as usize] != Role::Pick && roles[b as usize] != Role::Pick {
            return true;
        }
        let (start, end) = self.spans[id];
        let (mut open, mut found) = ([NONE; 2], 0);
        for &c in &self.members[start as usize..end as usize] {
            if roles[c as usize] != Role::Pick {
                open[found] = c;
                found += 1;
                if found == 2 {
                    break;
                }
            }
        }
        match found {
            0 => return false,
            1 => {
                if roles[open[0] as usize] == Role::Free {
                    roles[open[0] as usize] = Role::Barred;
                }
                return true;
            }
            _ => {}
        }
        // A watch that is forced moves to the first member found that the nogood does not watch already.
        for slot in 0..2 {
            let from = self.watch[id][slot];
            if roles[from as usize] != Role::Pick {
                continue;
            }
            let other = self.watch[id][1 - slot];
            let to = if open[0] != other { open[0] } else { open[1] };
            let list = &mut self.watchers[from as usize];
            list.swap_remove(list.iter().position(|&i| i as usize == id).expect("a watcher is listed"));
            self.watch[id][slot] = to;
            self.watchers[to as usize].push(id as u32);
        }
        true
    }

    /// The nogoods that watch `x`, which is forced now: each moves its watch to another member that is not forced, or,
    /// if every member but the other watch is forced, bars the other watch (false if that is forced too).
    fn visit(&mut self, x: u32, roles: &mut [Role]) -> bool {
        let mut list = std::mem::take(&mut self.watchers[x as usize]);
        let (mut kept, mut i, mut ok) = (0, 0, true);
        while i < list.len() {
            let id = list[i];
            i += 1;
            let [a, b] = self.watch[id as usize];
            let (slot, other) = if a == x { (0, b) } else { (1, a) };
            if roles[other as usize] != Role::Barred {
                let (start, end) = self.spans[id as usize];
                let members = &self.members[start as usize..end as usize];
                let to = members.iter().copied().find(|&r| r != x && r != other && roles[r as usize] != Role::Pick);
                if let Some(r) = to {
                    self.watch[id as usize][slot] = r;
                    self.watchers[r as usize].push(id);
                    continue;
                }
                if roles[other as usize] == Role::Pick {
                    ok = false;
                    list[kept] = id;
                    kept += 1;
                    break;
                }
                roles[other as usize] = Role::Barred;
            }
            list[kept] = id;
            kept += 1;
        }
        list.copy_within(i.., kept);
        list.truncate(kept + list.len() - i);
        self.watchers[x as usize] = list;
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small generator, so that the tests need no crate for it.
    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: usize) -> usize {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (self.0 >> 33) as usize % n
        }
    }

    const NUMBERS: usize = 14;

    /// The verdict and the roles that propagation comes to, by reading every nogood until nothing changes.
    fn by_scan(table: &Nogoods, roles: &[Role]) -> (bool, Vec<Role>) {
        let mut roles = roles.to_vec();
        loop {
            let mut changed = false;
            if !table.scan(&mut roles, &mut changed) {
                return (false, roles);
            }
            if !changed {
                return (true, roles);
            }
        }
    }

    /// A random nogood of two to four distinct numbers.
    fn random_nogood(rng: &mut Rng) -> Vec<u32> {
        let mut k: Vec<u32> = Vec::new();
        while k.len() < 2 + rng.below(3) {
            let c = 2 + rng.below(NUMBERS - 2) as u32;
            if !k.contains(&c) {
                k.push(c);
            }
        }
        k.sort_unstable();
        k
    }

    /// Visits the nodes of a random tree as the search does: each node's roles extend its parent's, the parent's
    /// roles were propagated before the nogoods learned since, and a node learns nogoods before its children are made
    /// from it.  At every node, propagation by the watches agrees with the scan, and so do the nogoods inside two
    /// sets that change from node to node.  `seen` counts the nodes, those that end at a nogood whose members are all
    /// forced, those where a nogood bars a number, and the nogoods examined whole.
    fn walk(
        table: &mut Nogoods,
        rng: &mut Rng,
        seen: &mut [usize; 4],
        parent: &[Role],
        checked: usize,
        mut roles: Vec<Role>,
        depth: usize,
    ) {
        let trail: Vec<u32> = (0..roles.len() as u32)
            .filter(|&c| roles[c as usize] == Role::Pick && parent[c as usize] != Role::Pick)
            .collect();
        let (verdict, fixpoint) = by_scan(table, &roles);
        let before = roles.clone();
        let ok = table.propagate(&mut roles, &trail, checked..table.len());
        seen[0] += 1;
        seen[3] += table.len() - checked;
        assert_eq!(ok, verdict, "the verdict at depth {depth}");
        if !ok {
            seen[1] += 1;
            return;
        }
        seen[2] += usize::from(roles != before);
        assert_eq!(roles, fixpoint, "the roles at depth {depth}");
        for which in [Set::Branching, Set::Matching] {
            let set: Vec<bool> = (0..roles.len()).map(|_| rng.below(10) < 7).collect();
            let scan: Vec<usize> =
                (0..table.len()).filter(|&i| table.get(i).iter().all(|&c| set[c as usize])).collect();
            assert_eq!(table.inside(which, &set), scan, "the nogoods inside a set at depth {depth}");
        }
        let checked = table.len();
        for _ in 0..rng.below(3) {
            // Learned by the search under the node's roles, or by another search and added as they come.
            let k = random_nogood(rng);
            if rng.below(2) == 0 {
                table.push(&k, &roles);
            } else {
                table.extend([k]);
            }
        }
        let mut free: Vec<u32> = (0..roles.len() as u32).filter(|&c| roles[c as usize] == Role::Free).collect();
        let mut decisions = Vec::new();
        while !free.is_empty() && decisions.len() < 4 {
            decisions.push(free.swap_remove(rng.below(free.len())));
        }
        if depth == 7 {
            return;
        }
        for j in 0..decisions.len() {
            let mut child = roles.clone();
            decisions[..j].iter().for_each(|&c| child[c as usize] = Role::Pick);
            child[decisions[j] as usize] = Role::Barred;
            walk(table, rng, seen, &roles, checked, child, depth + 1);
        }
    }

    #[test]
    fn propagation_by_watches_agrees_with_the_scan_in_a_random_tree_of_roles() {
        let mut seen = [0; 4];
        for seed in 0..40 {
            let mut rng = Rng(seed);
            let mut table = Nogoods::from((0..4).map(|_| random_nogood(&mut rng)).collect::<Vec<_>>());
            let mut roles = vec![Role::Free; NUMBERS];
            roles[0] = Role::Barred;
            roles[1] = Role::Barred;
            // The root has no parent: every nogood is examined.
            let parent = roles.clone();
            walk(&mut table, &mut rng, &mut seen, &parent, 0, roles, 0);
        }
        // The walk must reach the cases: nodes that end, nodes where a nogood bars a number, and learned nogoods.
        assert!(seen.iter().all(|&count| count >= 100), "the walk is too thin: {seen:?}");
    }

    #[test]
    fn nogoods_keep_their_members_in_order_and_come_out_as_lists() {
        let ks = vec![vec![2, 5, 9], vec![3, 4], vec![2, 3, 4, 13]];
        let mut table = Nogoods::from(ks.clone());
        assert_eq!(table.len(), 3);
        assert_eq!(table.to_vec(), ks);
        assert_eq!(table.get(2), &[2, 3, 4, 13]);
        table.extend([vec![6, 7]]);
        assert_eq!(table.since(2), vec![ks[2].clone(), vec![6, 7]]);
    }
}
