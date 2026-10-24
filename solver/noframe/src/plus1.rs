//! The incremental exact check at a rejection threshold of (★): does the new jam need one more exclusion?
//!
//! At a rejection x, P is x's component of the residual R_x of the members down to x.  The components of the
//! residual just above x that lie in P, C₁ … C_k, were settled at earlier thresholds: the fewest exclusions
//! making C_i playable is g(C_i), greedy's own count there.  P's other members N (x among them) are new; greedy
//! kept all of them but x.  So g(P) = Σ g(C_i) + 1, and (★) at x says P needs g(P) exclusions.  It needs at
//! least Σ g(C_i) (each C_i needs its own), so the question is whether exactly Σ g(C_i) can do:
//!
//! > is there an E with |E| ≤ B = Σ g(C_i) such that P ∖ E is playable?
//!
//! Such an E avoids N.  Every earlier threshold's jam D ⊆ P (a component of some residual above x, settled
//! before) also needs |E ∩ D| ≥ g(D), and these sets form a laminar family (residuals only grow downwards, so
//! their components only merge).  With the total capped at B the family propagates hard: `need(D)` =
//! max(g(D), exclusions among D's own members + Σ need(children)) must sum to at most B over the top sets, which
//! leaves each set D room for at most `room(D)` more exclusions; a member whose exclusion would push the total
//! over B is kept, and a set that can only just reach its g excludes all its open members.  Every core of P
//! (a self-covering subset) must lose a member: the cores found are kept as clauses and propagated (a core with
//! one member left open loses it; one with none left is a dead end), they bound the search (cores with
//! disjoint open members inside D need that many more exclusions in D), and they stay valid at later
//! thresholds, whose P contain this one.
//!
//! The search branches on the clause with the fewest open members, or, when every known core is hit, on a new
//! minimal core of the members not excluded (if there is none, those members are playable: a counterexample).
//! It is exact given the earlier thresholds: every pruning follows from valid constraints, and the branching
//! (child j excludes the j-th open member of the core and keeps the ones before it) covers every solution.

use crate::instance::Instance;
use crate::maxcard::Outcome;
use crate::peel::{components, flagged, minimal_core, Peeler};

const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Open,
    Kept,
    Excluded,
}

pub struct Plus1<'a> {
    inst: &'a Instance,
    /// Innermost laminar set of each member, or NONE for the new members N (never excluded).
    node_of: Vec<u32>,
    parent: Vec<u32>,
    depth: Vec<u32>,
    g: Vec<usize>,
    /// Laminar sets, children before parents.
    order: Vec<u32>,
    budget_total: usize,
    /// Known cores of P (member indices), each a clause: some member must be excluded.
    pub cores: Vec<Vec<u32>>,
    max_cores: usize,
    peeler: Peeler,
    pub nodes: u64,
    node_budget: u64,
}

/// What the counting constraints leave: the room for more exclusions in each set, and in all (B minus the
/// exclusions made).
struct Room {
    room: Vec<usize>,
    total: usize,
}

impl<'a> Plus1<'a> {
    /// `sets` are the laminar family's sets (member indices of `inst`, each ascending) with their g; members in
    /// none of them are the new members N.  `budget_total` is B.  `cores` are known cores of `inst`.
    pub fn new(
        inst: &'a Instance,
        sets: &[(Vec<u32>, usize)],
        budget_total: usize,
        node_budget: u64,
        cores: Vec<Vec<u32>>,
    ) -> Self {
        let mut idx: Vec<usize> = (0..sets.len()).collect();
        idx.sort_by_key(|&i| sets[i].0.len());
        let order: Vec<u32> = (0..sets.len() as u32).collect();
        let sorted: Vec<&(Vec<u32>, usize)> = idx.iter().map(|&i| &sets[i]).collect();
        // Innermost set of each member: assign from the largest set down to the smallest.
        let mut node_of = vec![NONE; inst.len()];
        for (j, (members, _)) in sorted.iter().enumerate().rev() {
            for &m in members {
                node_of[m as usize] = j as u32;
            }
        }
        // Parent: the smallest later set containing the set (laminarity makes one member enough).
        let mut parent = vec![NONE; sorted.len()];
        let mut depth = vec![0u32; sorted.len()];
        let mut owner = vec![NONE; inst.len()];
        for (j, (members, _)) in sorted.iter().enumerate().rev() {
            parent[j] = owner[members[0] as usize];
            if parent[j] != NONE {
                depth[j] = depth[parent[j] as usize] + 1;
            }
            for &m in members {
                debug_assert_eq!(owner[m as usize], parent[j], "the family must be laminar");
                owner[m as usize] = j as u32;
            }
        }
        let g = sorted.iter().map(|(_, g)| *g).collect();
        Plus1 {
            inst,
            node_of,
            parent,
            depth,
            g,
            order,
            budget_total,
            cores,
            max_cores: 50_000,
            peeler: Peeler::new(),
            nodes: 0,
            node_budget,
        }
    }

    pub fn run(&mut self) -> Outcome {
        let state: Vec<State> =
            self.node_of.iter().map(|&d| if d == NONE { State::Kept } else { State::Open }).collect();
        self.node(state)
    }

    fn learn(&mut self, core: Vec<u32>) {
        if self.cores.len() < self.max_cores {
            self.cores.push(core);
        }
    }

    /// The counting constraints of the laminar family and the total.  None if they cannot be met; otherwise
    /// keeps every open member they forbid excluding, excludes every open member they require (setting
    /// `changed`), and returns the room left.
    fn count(&self, state: &mut [State], changed: &mut bool) -> Option<Room> {
        let sets = self.g.len();
        let mut own_x = vec![0usize; sets];
        let mut poss = vec![0usize; sets];
        for (m, &s) in state.iter().enumerate() {
            let d = self.node_of[m];
            if d != NONE && s != State::Kept {
                poss[d as usize] += 1;
                if s == State::Excluded {
                    own_x[d as usize] += 1;
                }
            }
        }
        let mut need = vec![0usize; sets];
        let mut x_sub = own_x.clone();
        let mut sum_term = own_x;
        for &d in &self.order {
            let d = d as usize;
            need[d] = self.g[d].max(sum_term[d]);
            if poss[d] < self.g[d] {
                return None;
            }
            let p = self.parent[d];
            if p != NONE {
                sum_term[p as usize] += need[d];
                poss[p as usize] += poss[d];
                x_sub[p as usize] += x_sub[d];
            }
        }
        let total: usize = (0..sets).filter(|&d| self.parent[d] == NONE).map(|d| need[d]).sum();
        if total > self.budget_total {
            return None;
        }
        // Top-down: the most exclusions D can hold (ub), whether it can only just reach its g (tight: all its
        // open members must go), and whether one more exclusion inside it is absorbed somewhere on its chain.
        let mut ub = vec![0usize; sets];
        let mut tight = vec![false; sets];
        let mut absorbs = vec![true; sets];
        for &d in self.order.iter().rev() {
            let d = d as usize;
            let p = self.parent[d];
            ub[d] = if p == NONE {
                self.budget_total - (total - need[d])
            } else {
                ub[p as usize] - (sum_term[p as usize] - need[d])
            };
            tight[d] = poss[d] == self.g[d] || (p != NONE && tight[p as usize]);
            if total == self.budget_total {
                let own = self.g[d] > sum_term[d];
                absorbs[d] = own || (p != NONE && absorbs[p as usize]);
            }
        }
        for (m, s) in state.iter_mut().enumerate() {
            if *s != State::Open {
                continue;
            }
            let d = self.node_of[m] as usize;
            match (tight[d], absorbs[d]) {
                (true, false) => return None,
                (true, true) => {
                    *s = State::Excluded;
                    *changed = true;
                }
                (false, false) => {
                    *s = State::Kept;
                    *changed = true;
                }
                (false, true) => {}
            }
        }
        let room = (0..sets).map(|d| ub[d] - x_sub[d]).collect();
        let excluded = state.iter().filter(|&&s| s == State::Excluded).count();
        Some(Room { room, total: self.budget_total - excluded })
    }

    /// Unit propagation over the known cores: false on a core with every member kept.
    fn clauses(&self, state: &mut [State], changed: &mut bool) -> bool {
        for core in &self.cores {
            let mut open = NONE;
            let mut open_count = 0;
            let mut hit = false;
            for &m in core {
                match state[m as usize] {
                    State::Excluded => {
                        hit = true;
                        break;
                    }
                    State::Open => {
                        open = m;
                        open_count += 1;
                    }
                    State::Kept => {}
                }
            }
            if hit {
                continue;
            }
            match open_count {
                0 => return false,
                1 => {
                    state[open as usize] = State::Excluded;
                    *changed = true;
                }
                _ => {}
            }
        }
        true
    }

    /// Failed keeps: an open member whose keeping would make the kept members unplayable must be excluded.  (A
    /// member with a factor no kept member lists can always be kept: the extension lemma, P6.)  False if the kept
    /// members are unplayable already.
    fn keeps(&mut self, state: &mut [State], changed: &mut bool) -> bool {
        let inst = self.inst;
        let mut kept: Vec<bool> = state.iter().map(|&s| s == State::Kept).collect();
        if !self.peeler.playable_mask(inst, &kept) {
            return false;
        }
        let mut listed = vec![0u32; inst.num_factors()];
        for (m, &k) in kept.iter().enumerate() {
            if k {
                inst.factors[m].iter().for_each(|&f| listed[f as usize] += 1);
            }
        }
        for u in 0..inst.len() {
            if state[u] != State::Open || inst.factors[u].iter().any(|&f| listed[f as usize] == 0) {
                continue;
            }
            kept[u] = true;
            if !self.peeler.playable_mask(inst, &kept) {
                state[u] = State::Excluded;
                *changed = true;
            }
            kept[u] = false;
        }
        true
    }

    fn propagate(&mut self, state: &mut [State]) -> Option<Room> {
        loop {
            let mut changed = false;
            let room = self.count(state, &mut changed)?;
            if !self.clauses(state, &mut changed) {
                return None;
            }
            if !changed && !self.keeps(state, &mut changed) {
                return None;
            }
            if !changed {
                return Some(room);
            }
        }
    }

    /// The smallest laminar set holding all of `members` (NONE: none does).
    fn container(&self, members: impl Iterator<Item = u32>) -> u32 {
        let mut c = NONE;
        let mut first = true;
        for m in members {
            let mut d = self.node_of[m as usize];
            if first {
                c = d;
                first = false;
                continue;
            }
            let mut a = c;
            while a != d {
                if a == NONE || d == NONE {
                    return NONE;
                }
                if self.depth[a as usize] >= self.depth[d as usize] {
                    a = self.parent[a as usize];
                } else {
                    d = self.parent[d as usize];
                }
            }
            c = a;
        }
        c
    }

    /// Packing bound: unhit cores with pairwise disjoint open members each need their own exclusion, inside
    /// the smallest set holding their open members.  False if some set (or the total) has too little room.
    fn packing_fits(&self, state: &[State], room: &Room, unhit: &[(usize, usize)]) -> bool {
        let mut used = vec![false; self.inst.len()];
        let mut count = vec![0usize; self.g.len()];
        let mut packed = 0usize;
        for &(i, _) in unhit {
            let core = &self.cores[i];
            let open = || core.iter().copied().filter(|&m| state[m as usize] == State::Open);
            if open().any(|m| used[m as usize]) {
                continue;
            }
            open().for_each(|m| used[m as usize] = true);
            packed += 1;
            let c = self.container(open());
            if c != NONE {
                count[c as usize] += 1;
            }
        }
        if packed > room.total {
            return false;
        }
        for &d in &self.order {
            let d = d as usize;
            if count[d] > room.room[d] {
                return false;
            }
            let p = self.parent[d];
            if p != NONE {
                count[p as usize] += count[d];
            }
        }
        true
    }

    fn node(&mut self, mut state: Vec<State>) -> Outcome {
        self.nodes += 1;
        if self.nodes > self.node_budget {
            return Outcome::Budget;
        }
        let Some(room) = self.propagate(&mut state) else { return Outcome::Below };
        let inst = self.inst;
        let mut kept: Vec<bool> = state.iter().map(|&s| s == State::Kept).collect();
        if self.peeler.residual(inst, &mut kept) > 0 {
            // A core of kept members: a dead end here, and a clause everywhere.
            let core = minimal_core(inst, &mut self.peeler, &flagged(&kept));
            self.learn(core);
            return Outcome::Below;
        }
        let mut unhit: Vec<(usize, usize)> = Vec::new();
        for (i, core) in self.cores.iter().enumerate() {
            if core.iter().any(|&m| state[m as usize] == State::Excluded) {
                continue;
            }
            unhit.push((i, core.iter().filter(|&&m| state[m as usize] == State::Open).count()));
        }
        if unhit.is_empty() {
            // Every known core is hit: are the members left playable?  If not, learn a minimal core in each
            // component of their residual (deleting open members first, so that few of them remain).
            let mut rest: Vec<bool> = state.iter().map(|&s| s != State::Excluded).collect();
            if self.peeler.residual(inst, &mut rest) == 0 {
                return Outcome::Found(flagged(&state.iter().map(|&s| s != State::Excluded).collect::<Vec<_>>()));
            }
            for comp in components(inst, &rest) {
                let mut ordered: Vec<u32> =
                    comp.iter().copied().filter(|&m| state[m as usize] == State::Open).collect();
                ordered.extend(comp.iter().copied().filter(|&m| state[m as usize] != State::Open));
                let core = minimal_core(inst, &mut self.peeler, &ordered);
                let open = core.iter().filter(|&&m| state[m as usize] == State::Open).count();
                unhit.push((self.cores.len(), open));
                self.learn(core);
            }
            unhit.retain(|&(i, _)| i < self.cores.len());
            if unhit.is_empty() {
                return Outcome::Budget;
            }
        }
        unhit.sort_by_key(|&(_, open)| open);
        if !self.packing_fits(&state, &room, &unhit) {
            return Outcome::Below;
        }
        let core = self.cores[unhit[0].0].clone();
        let branch: Vec<u32> = core.iter().copied().filter(|&c| state[c as usize] == State::Open).collect();
        assert!(!branch.is_empty(), "a core inside the kept set, which is playable");
        for (j, &c) in branch.iter().enumerate() {
            if j > 0 {
                state[branch[j - 1] as usize] = State::Kept;
            }
            let mut child = state.clone();
            child[c as usize] = State::Excluded;
            match self.node(child) {
                Outcome::Below => {}
                other => return other,
            }
        }
        Outcome::Below
    }
}
