//! Hives: the units the frame model is built from.
//!
//! The frame model looks at the board `1..=n` through *maximal* factors (see [`crate::numtheory`]): a move
//! `m` is paid for by one of its maximal factors, and a factor can pay for only one move.  A number with no
//! maximal composite (every number above `n/2`) is never anybody's tax, so it is always a move: a *source*.
//! The search may also *promote* further numbers, which are then treated as sources too
//! (`treat_as_sources`).  Everything that is not a source is a *factor*.
//!
//! A **hive** is a connected component of the graph that joins every source to its factors: a set of sources
//! together with all the factors they compete for.  All the sources of a hive can be played exactly when
//! they can be matched to distinct factors, so hives are independent of each other except that a factor of
//! one hive can be *promoted* into a source of the hive below it.  An [`Apiary`] is the set of all hives of
//! one board, in an order (downstream hives first) in which their games can simply be concatenated.
//!
//! After the hives are formed, [`Apiary::new`] simplifies each one until nothing changes:
//!
//! * **unzip** finds forced moves.  A source with a single usable factor must take it; a factor wanted by a
//!   single source, which cannot be put to better use outside the hive, is given to that source.  Forced
//!   pairs leave the hive's *remaining* sources and factors.
//! * **removeImpossible** drops, greedily from the largest source down, every source that cannot be matched
//!   together with the sources already kept.  They are the hive's *impossible* moves.
//! * The *free factor count* is `|factors| − |sources|`: how many factors the hive can spare, which is how
//!   many numbers the hive above it may promote into it.
//!
//! The matching test (`count_freedoms`) is a peeling heuristic, not a full matching, and its exact behaviour
//! defines which sources count as impossible.  The results of the Java solver are what this module has to
//! reproduce, so the order of every loop below is deliberate.

use crate::bits::Bits;
use crate::numtheory::Tables;
use std::collections::VecDeque;

/// One connected group of sources and the factors they compete for, plus what the simplification found.
#[derive(Clone, Debug)]
pub struct Hive<const W: usize> {
    name: String,
    /// The moves of the hive, without those found impossible.
    sources: Bits<W>,
    /// Every number the sources can use as tax.
    factors: Bits<W>,
    /// Sources whose move is forced.
    mandatory: Bits<W>,
    /// The factors those forced moves use up.
    claimed: Bits<W>,
    /// Sources that cannot be played together with the others.
    impossible: Bits<W>,
    free_factor_count: i32,
}

impl<const W: usize> Hive<W> {
    /// A hive that has only just been formed: nothing is forced or impossible, every factor is free.
    fn new(sources: Bits<W>, factors: Bits<W>) -> Self {
        Hive {
            name: String::new(),
            sources,
            factors,
            mandatory: Bits::EMPTY,
            claimed: Bits::EMPTY,
            impossible: Bits::EMPTY,
            free_factor_count: factors.len() as i32,
        }
    }

    /// One hive made of several, as when the hives of a frame are treated as a single unit.  It starts from
    /// scratch: the sources and factors are the unions of the current ones, and whatever the parts had found
    /// forced or impossible is forgotten (and found again on the next [`Apiary::finish_setup`]).
    fn merged(parts: &[&Hive<W>]) -> Self {
        let sources = parts.iter().fold(Bits::EMPTY, |acc, h| acc | h.sources);
        let factors = parts.iter().fold(Bits::EMPTY, |acc, h| acc | h.factors);
        let mut hive = Hive::new(sources, factors);
        hive.name = parts.iter().map(|h| h.name.as_str()).collect::<Vec<_>>().join("+");
        hive
    }

    /// A spreadsheet-style label (A, B, ... Z, AA, ...), by position in the apiary; merged hives join the
    /// labels of their parts with `+`.  Only for diagnostics.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The hive's moves, forced ones included, impossible ones excluded.
    pub fn sources(&self) -> Bits<W> {
        self.sources
    }

    /// Every factor of the hive, including those used up by forced moves.
    pub fn factors(&self) -> Bits<W> {
        self.factors
    }

    /// Sources that are neither forced nor impossible: the moves the search still has to place.
    pub fn remaining_sources(&self) -> Bits<W> {
        self.sources - (self.mandatory | self.impossible)
    }

    /// Factors not used up by forced moves.
    pub fn remaining_factors(&self) -> Bits<W> {
        self.factors - self.claimed
    }

    /// Sources found impossible.
    pub fn impossible(&self) -> Bits<W> {
        self.impossible
    }

    /// `|factors| − |sources|`: how many factors are left over once every source has one.
    pub fn free_factor_count(&self) -> i32 {
        self.free_factor_count
    }

    /// Whether the hive can spare a factor.
    pub fn has_free_factors(&self) -> bool {
        self.free_factor_count > 0
    }
}

/// Which hive a number belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Home {
    /// The hive at this position in [`Apiary::hives`].
    Live(u32),
    /// A hive that has since been merged into another one, which this number did not follow: it had already
    /// been removed from the hive as an impossible source, and a merge only re-homes the numbers the merged
    /// hive still holds.  Such a number keeps pointing at the old hive, which is no longer part of the
    /// apiary but still remembers whether it had free factors when it was merged.  This is how the Java
    /// solver behaves; it is reproduced for parity (see `Apiary::has_outside_move_using`).
    Merged { had_free_factors: bool },
    /// None: the number is a hole of a punctured board ([`Tables::punctured`]).
    OffBoard,
}

/// All the hives of the board `1..=n`, for a given set of promoted numbers.
///
/// On a punctured board ([`Tables::punctured`]) the holes belong to no hive and to no factor or composite list,
/// so they are never moves and never tax; a number whose maximal factor is a hole simply has one factor fewer.
/// Everything else is as on the full board: in particular the sources are still the numbers above `n/2` (a number
/// whose only maximal composites are holes is a factor nobody needs, not a source, so the search decides whether
/// to promote it), and the hives are put in an order in which every number comes after its divisors' hives
/// even when the chain of maximal factors between them passes through a hole (playing a number sweeps all of its
/// divisors, holes or not in between).
pub struct Apiary<'t, const W: usize> {
    t: &'t Tables,
    /// In apiary order: a hive is listed after every hive downstream of it.
    hives: Vec<Hive<W>>,
    /// The hive of every number, indexed by number (entry 0 is unused).
    home: Vec<Home>,
    /// For each number, the factors it can use as tax: its maximal factors that are not sources (or holes);
    /// nothing for a *sink* (a number all of whose maximal factors are promoted or holes).
    factors_of: Vec<Bits<W>>,
    /// For each number, the moves that could use it as tax: its maximal composites other than sinks (and holes);
    /// nothing for a source.  The mirror image of `factors_of`.
    composites_of: Vec<Bits<W>>,
}

impl<'t, const W: usize> Apiary<'t, W> {
    /// Builds the apiary of the board `1..=t.n`, treating the numbers in `treat_as_sources` (the promoted
    /// numbers) as moves in addition to the numbers that are always moves.
    pub fn new(t: &'t Tables, treat_as_sources: Bits<W>) -> Self {
        let n = t.n;
        assert!(n <= Bits::<W>::MAX_VALUE, "the board 1..={n} does not fit in Bits<{W}>");
        assert!(
            !treat_as_sources.contains(0)
                && treat_as_sources.max().is_none_or(|m| m <= n)
                && t.holes.iter().all(|&h| !treat_as_sources.contains(h)),
            "promotions {treat_as_sources:?} are not all on the board 1..={n} without {:?}",
            t.holes
        );

        let (sources, sinks) = Self::sources_and_sinks(t, treat_as_sources);
        let size = n as usize + 1;
        let mut factors_of = vec![Bits::EMPTY; size];
        let mut composites_of = vec![Bits::EMPTY; size];
        for x in (1..=n).filter(|&x| !t.is_hole(x)) {
            let xi = x as usize;
            if !sinks.contains(x) {
                let usable = |f: &u32| !sources.contains(*f) && !t.is_hole(*f);
                factors_of[xi] = t.max_factors[xi].iter().copied().filter(usable).collect();
            }
            if !sources.contains(x) {
                let usable = |c: &u32| !sinks.contains(*c) && !t.is_hole(*c);
                composites_of[xi] = t.max_composites[xi].iter().copied().filter(usable).collect();
            }
        }

        let (hives, home) = Self::form_hives(t, sources, &factors_of);
        let mut apiary = Apiary { t, hives, home, factors_of, composites_of };
        let order = apiary.order_hives();
        apiary.reorder(&order);
        apiary.finish_setup();
        apiary
    }

    /// The numbers that are moves (promoted ones, and those with no maximal composite) and the sinks (numbers
    /// whose maximal factors are all promoted or holes, which therefore have no factor to pay tax with).  Holes
    /// are neither.
    fn sources_and_sinks(t: &Tables, promoted: Bits<W>) -> (Bits<W>, Bits<W>) {
        let (mut sources, mut sinks) = (promoted, Bits::EMPTY);
        for x in (1..=t.n).filter(|&x| !t.is_hole(x)) {
            let xi = x as usize;
            if t.max_composites[xi].is_empty() {
                sources.insert(x);
            }
            if t.max_factors[xi].iter().all(|&f| promoted.contains(f) || t.is_hole(f)) {
                sinks.insert(x);
            }
        }
        (sources, sinks)
    }

    /// Joins every source to its factors and returns the connected components, in creation order, with the
    /// hive of each number; holes are in none.  (A number that is neither a source nor a factor of one forms a
    /// hive of its own, without sources.  On the board `1..=n` that never happens, because every `x ≤ n/2` has a
    /// prime `p` with `n/2 < x·p ≤ n`, and `x·p` is a source of which `x` is a maximal factor.  On a punctured
    /// board it can, when every such `x·p` is a hole: then `x` is a factor nobody needs, a free hive of one,
    /// which the search may promote.)
    fn form_hives(t: &Tables, sources: Bits<W>, factors_of: &[Bits<W>]) -> (Vec<Hive<W>>, Vec<Home>) {
        let n = t.n;
        let size = n as usize + 1;
        let mut parent: Vec<u32> = (0..=n).collect();
        for s in sources.iter() {
            for f in factors_of[s as usize].iter() {
                let (a, b) = (find(&mut parent, s), find(&mut parent, f));
                parent[a as usize] = b;
            }
        }

        let mut hive_of_root = vec![u32::MAX; size];
        let mut parts: Vec<(Bits<W>, Bits<W>)> = Vec::new();
        let mut home = vec![Home::OffBoard; size];
        for x in (1..=n).filter(|&x| !t.is_hole(x)) {
            let root = find(&mut parent, x) as usize;
            if hive_of_root[root] == u32::MAX {
                hive_of_root[root] = parts.len() as u32;
                parts.push((Bits::EMPTY, Bits::EMPTY));
            }
            let h = hive_of_root[root];
            let (hive_sources, hive_factors) = &mut parts[h as usize];
            if sources.contains(x) {
                hive_sources.insert(x)
            } else {
                hive_factors.insert(x)
            }
            home[x as usize] = Home::Live(h);
        }
        let hives = parts.into_iter().map(|(s, f)| Hive::new(s, f)).collect();
        (hives, home)
    }

    /// The order the hives are kept in: downstream before upstream, so that playing the hives one after the
    /// other is always a legal game.  Start from the hives with nothing downstream, smallest source first;
    /// after taking a hive, queue the hives directly upstream of it whose every downstream hive is already
    /// taken, again smallest source first.  (Hives without a source sort first; among those the order is
    /// that of formation.)  Downstream here passes through holes ([`Self::neighbours_through_holes`]).
    fn order_hives(&self) -> Vec<usize> {
        let count = self.hives.len();
        let t = self.t;
        let down: Vec<Vec<usize>> = (0..count).map(|i| self.neighbours_through_holes(i, &t.max_factors)).collect();
        let up: Vec<Vec<usize>> = (0..count).map(|i| self.neighbours_through_holes(i, &t.max_composites)).collect();
        let min_source = |i: usize| self.hives[i].sources.min();

        let mut roots: Vec<usize> = (0..count).filter(|&i| down[i].is_empty()).collect();
        roots.sort_by_key(|&i| min_source(i));
        let mut queue: VecDeque<usize> = roots.into();
        let mut done = vec![false; count];
        let mut order = Vec::with_capacity(count);
        while let Some(h) = queue.pop_front() {
            if done[h] {
                continue;
            }
            done[h] = true;
            order.push(h);
            let mut ready: Vec<usize> =
                up[h].iter().copied().filter(|&u| !done[u] && down[u].iter().all(|&d| done[d])).collect();
            ready.sort_by_key(|&i| min_source(i));
            queue.extend(ready);
        }
        assert_eq!(order.len(), count, "the hives are not a DAG");
        order
    }

    /// Puts the hives into the given order (a permutation of positions) and gives them their names.
    fn reorder(&mut self, order: &[usize]) {
        let mut new_pos = vec![0u32; order.len()];
        for (pos, &old) in order.iter().enumerate() {
            new_pos[old] = pos as u32;
        }
        let mut old: Vec<Option<Hive<W>>> = std::mem::take(&mut self.hives).into_iter().map(Some).collect();
        self.hives = order.iter().map(|&i| old[i].take().expect("a permutation")).collect();
        for (pos, hive) in self.hives.iter_mut().enumerate() {
            hive.name = column_name(pos);
        }
        for home in &mut self.home {
            if let Home::Live(p) = home {
                *p = new_pos[*p as usize];
            }
        }
    }

    /// The hives, in apiary order.
    pub fn hives(&self) -> &[Hive<W>] {
        &self.hives
    }

    /// The position in [`hives`](Self::hives) of the hive that holds `x`, unless `x` is an impossible source
    /// of a hive that has since been merged away, or a hole.
    pub(crate) fn hive_index_of(&self, x: u32) -> Option<usize> {
        match self.home.get(x as usize)? {
            Home::Live(p) => Some(*p as usize),
            Home::Merged { .. } | Home::OffBoard => None,
        }
    }

    /// The hives (other than hive `i`) that hold a maximal factor of a number in hive `i`, ascending.
    /// Note that this looks at the board's maximal factors, not at the hives' factor tables.
    pub(crate) fn downstream_hives(&self, i: usize) -> Vec<usize> {
        self.neighbours(i, &self.t.max_factors)
    }

    /// The hives (other than hive `i`) that hold a maximal composite of a number in hive `i`, ascending.
    pub(crate) fn upstream_hives(&self, i: usize) -> Vec<usize> {
        self.neighbours(i, &self.t.max_composites)
    }

    /// The hives (other than hive `i`) that hold a number related to one in hive `i`.  Holes are skipped.
    fn neighbours(&self, i: usize, board_relation: &[Vec<u32>]) -> Vec<usize> {
        let hive = &self.hives[i];
        let mut out = Vec::new();
        for x in (hive.sources | hive.factors).iter() {
            for &y in &board_relation[x as usize] {
                // A number homed in a merged-away hive is skipped: that hive is not in the apiary, so there
                // is nothing to reach.
                if let Home::Live(j) = self.home[y as usize] {
                    if j as usize != i {
                        out.push(j as usize);
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// [`neighbours`](Self::neighbours), except that a related hole is passed through: the hives holding the
    /// numbers related to the hole are reached instead.  This keeps the relation transitive across holes, as
    /// divisibility is (a number sweeps its divisors whether or not the chain of maximal factors between them
    /// is on the board).  Without holes it is `neighbours`.
    fn neighbours_through_holes(&self, i: usize, board_relation: &[Vec<u32>]) -> Vec<usize> {
        if self.t.holes.is_empty() {
            return self.neighbours(i, board_relation);
        }
        let hive = &self.hives[i];
        let mut out = Vec::new();
        let mut pending: Vec<u32> = (hive.sources | hive.factors).iter().collect();
        let mut passed = Vec::new();
        while let Some(x) = pending.pop() {
            for &y in &board_relation[x as usize] {
                match self.home[y as usize] {
                    Home::Live(j) if j as usize != i => out.push(j as usize),
                    Home::OffBoard if !passed.contains(&y) => {
                        passed.push(y);
                        pending.push(y);
                    }
                    _ => {}
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The position of the hive holding the factor `f`.  Factors are never removed from a hive, so they are
    /// always homed in a hive of the apiary.
    fn hive_of_factor(&self, f: u32) -> usize {
        self.hive_index_of(f).expect("a factor always has a live hive")
    }

    /// Whether `pred` holds for any hive that holds one of the factors in `set` (each hive is tried once).
    fn any_hive_of(&self, set: Bits<W>, mut pred: impl FnMut(&Hive<W>) -> bool) -> bool {
        let mut tried = [usize::MAX; 8];
        let mut count = 0;
        for f in set.iter() {
            let h = self.hive_of_factor(f);
            if tried[..count].contains(&h) {
                continue;
            }
            if count < tried.len() {
                tried[count] = h;
                count += 1;
            }
            if pred(&self.hives[h]) {
                return true;
            }
        }
        false
    }

    // -----------------------------------------------------------------------------------------------------
    // Matching

    /// If every move in `moves` can be given a factor of its own from `factors`, how many factors are left;
    /// `None` if not.
    ///
    /// This is a peeling heuristic, applied in a fixed order: while moves remain, take the smallest move that
    /// has just one usable factor and pair them off; failing that, the smallest factor wanted by just one
    /// move, and pair that off; if neither exists (every move has two or more factors and every factor two or
    /// more moves) give up.  A move left with no factor means failure.  Because every step removes one move
    /// and one factor, the answer on success is simply `|factors| − |moves|`.
    ///
    /// It is implemented with degree counters, so a call costs one pass over the moves and factors instead of
    /// a scan per step.  `lone_moves` and `lone_factors` hold the candidates for the two kinds of step;
    /// their smallest members are the ones the plain scan would reach first.
    fn count_freedoms(&self, mut moves: Bits<W>, mut factors: Bits<W>) -> Option<u32> {
        if moves.len() > factors.len() {
            return None;
        }
        let mut move_degree = Degrees::<W>::new();
        let mut factor_degree = Degrees::<W>::new();
        let mut lone_moves = Bits::<W>::EMPTY;
        let mut lone_factors = Bits::<W>::EMPTY;
        for m in moves.iter() {
            let degree = (self.factors_of[m as usize] & factors).len();
            if degree == 0 {
                return None; // can never be paired, so the peeling could never finish
            }
            move_degree.set(m, degree);
            if degree == 1 {
                lone_moves.insert(m);
            }
        }
        for f in factors.iter() {
            let degree = (self.composites_of[f as usize] & moves).len();
            factor_degree.set(f, degree);
            if degree == 1 {
                lone_factors.insert(f);
            }
        }

        while !moves.is_empty() {
            let (m, f) = if let Some(m) = lone_moves.min() {
                (m, (self.factors_of[m as usize] & factors).min()?)
            } else {
                let f = lone_factors.min()?; // none: every move has two factors, and every factor two moves
                ((self.composites_of[f as usize] & moves).min()?, f)
            };
            moves.remove(m);
            factors.remove(f);
            lone_moves.remove(m);
            lone_factors.remove(f);
            // The other factors `m` could have used are now wanted by one move fewer ...
            for g in (self.factors_of[m as usize] & factors).iter() {
                match factor_degree.decrement(g) {
                    0 => lone_factors.remove(g),
                    1 => lone_factors.insert(g),
                    _ => {}
                }
            }
            // ... and the other moves that could have used `f` have one factor fewer.
            for k in (self.composites_of[f as usize] & moves).iter() {
                match move_degree.decrement(k) {
                    0 => return None,
                    1 => lone_moves.insert(k),
                    _ => {}
                }
            }
        }
        Some(factors.len())
    }

    /// Whether `hive` could take the number `n` as an additional source.
    fn can_add_source(&self, hive: &Hive<W>, n: u32) -> bool {
        self.count_freedoms(hive.sources.with(n), hive.factors).is_some()
    }

    // -----------------------------------------------------------------------------------------------------
    // Simplification of the hives

    /// Runs the simplification on every hive, then keeps re-running it on the neighbours of any hive that
    /// changed, until nothing changes.  First every hive drops its impossible sources (so that the checks in
    /// `unzip` see the real hives); then, in rounds, the hives that have been touched are visited in apiary
    /// order.  A hive's findings change what its neighbours may conclude, which is why they are revisited.
    pub(crate) fn finish_setup(&mut self) {
        for i in 0..self.hives.len() {
            self.remove_impossible(i);
        }
        let mut todo = vec![true; self.hives.len()];
        loop {
            let round: Vec<usize> = (0..self.hives.len()).filter(|&i| todo[i]).collect();
            if round.is_empty() {
                break;
            }
            todo.fill(false);
            for i in round {
                if self.finish_hive(i) {
                    for j in self.downstream_hives(i).into_iter().chain(self.upstream_hives(i)) {
                        todo[j] = true;
                    }
                }
            }
        }
    }

    /// Simplifies hive `i` once; whether it found anything new.
    fn finish_hive(&mut self, i: usize) -> bool {
        let found_forced = self.unzip(i);
        let found_impossible = self.remove_impossible(i);
        let hive = &mut self.hives[i];
        hive.free_factor_count = hive.factors.len() as i32 - hive.sources.len() as i32;
        found_forced | found_impossible
    }

    /// Finds the forced moves of hive `i` until there are no more; whether it found any.
    ///
    /// Two kinds of move are forced, as long as the factor involved is of no use outside the hive:
    ///
    /// * a remaining factor with a single remaining composite in the hive, which cannot go downstream (be
    ///   promoted into a hive below) and which no other hive could use as tax, goes to that composite;
    /// * a remaining source with a single unclaimed factor takes it, provided no other hive could use that
    ///   factor.  This is "take the highest prime" for the whole hive: a hive is expected to play all of its
    ///   sources, so a source with one choice makes it.
    ///
    /// The factors are visited in ascending order against a snapshot, with the sources recomputed after each
    /// claim.  The sources are visited in descending order and the pass restarts after the first claim, since
    /// a claim can make a larger source eligible.
    fn unzip(&mut self, i: usize) -> bool {
        let (sources, factors, impossible) = (self.hives[i].sources, self.hives[i].factors, self.hives[i].impossible);
        let (mut mandatory, mut claimed) = (self.hives[i].mandatory, self.hives[i].claimed);
        let mut found = false;
        loop {
            let mut made_claim = false;

            let snapshot = factors - claimed;
            for f in snapshot.iter() {
                let wanting = (sources - (mandatory | impossible)) & self.composites_of[f as usize];
                if wanting.len() == 1 && !self.factor_can_go_downstream(f) && !self.has_outside_move_using(f) {
                    mandatory.insert(wanting.min().expect("one composite"));
                    claimed.insert(f);
                    made_claim = true;
                }
            }

            // Note: the unclaimed factors come from the apiary-wide table, not just this hive's factors.
            let snapshot = sources - (mandatory | impossible);
            for s in snapshot.iter_desc() {
                let open = self.factors_of[s as usize] - claimed;
                if open.len() == 1 {
                    let f = open.min().expect("one factor");
                    if !self.has_outside_move_using(f) {
                        mandatory.insert(s);
                        claimed.insert(f);
                        made_claim = true;
                        break;
                    }
                }
            }

            found |= made_claim;
            if !made_claim {
                break;
            }
        }
        self.hives[i].mandatory = mandatory;
        self.hives[i].claimed = claimed;
        found
    }

    /// Whether factor `f` could be promoted into a source of some hive that holds one of its factors.
    fn factor_can_go_downstream(&self, f: u32) -> bool {
        self.any_hive_of(self.factors_of[f as usize], |hive| self.can_add_source(hive, f))
    }

    /// Whether some move outside the hive of the factor `f` could use `f` as tax: a composite of `f` in a
    /// different hive (that has free factors to spare) which the hive of `f` could take on as a source.
    ///
    /// A composite that is an impossible source of a hive that has since been merged away counts as being in
    /// a different hive (see [`Home::Merged`]), with the free factors that hive had when it was merged.  That
    /// is what the Java solver does; for every board up to n = 1100 the frames come out the same without it.
    fn has_outside_move_using(&self, f: u32) -> bool {
        let f_hive = self.hive_of_factor(f);
        self.composites_of[f as usize].iter().any(|c| {
            let c_hive_has_free_factors = match self.home[c as usize] {
                Home::Live(p) => p as usize != f_hive && self.hives[p as usize].has_free_factors(),
                Home::Merged { had_free_factors } => had_free_factors,
                Home::OffBoard => unreachable!("a hole is nobody's composite"),
            };
            c_hive_has_free_factors && self.can_add_source(&self.hives[f_hive], c)
        })
    }

    /// Marks as impossible the remaining sources of hive `i` that cannot be matched, going from the largest
    /// source down and keeping each one that can be matched together with the ones already kept.  Impossible
    /// sources leave the hive's `sources`.  Whether any new impossible source was found.
    fn remove_impossible(&mut self, i: usize) -> bool {
        let hive = &self.hives[i];
        let factors = hive.remaining_factors();
        let mut impossible = hive.impossible;
        let known = impossible.len();
        let mut kept = Bits::EMPTY;
        for s in hive.remaining_sources().iter_desc() {
            if self.count_freedoms(kept.with(s), factors).is_some() {
                kept.insert(s);
            } else {
                impossible.insert(s);
            }
        }
        let hive = &mut self.hives[i];
        hive.impossible = impossible;
        hive.sources -= impossible;
        impossible.len() > known
    }

    // -----------------------------------------------------------------------------------------------------
    // Merging

    /// Replaces the hives at the given positions (at least two) by a single new hive, which is placed where
    /// the first of them was, and returns its position.  See [`Hive::merged`] for what the new hive holds.
    pub(crate) fn merge(&mut self, group: &[usize]) -> usize {
        assert!(group.len() > 1, "merging fewer than two hives");
        let at = group[0];
        let in_group = |p: usize| group.contains(&p);
        let parts: Vec<&Hive<W>> = group.iter().map(|&p| &self.hives[p]).collect();
        let merged = Hive::merged(&parts);
        let merged_numbers = merged.sources | merged.factors;
        let had_free_factors: Vec<bool> = self.hives.iter().map(Hive::has_free_factors).collect();

        let merged_pos = (0..at).filter(|&p| !in_group(p)).count();
        let mut new_pos = vec![0u32; self.hives.len()];
        let mut hives = Vec::with_capacity(self.hives.len() + 1 - group.len());
        let mut merged = Some(merged);
        for (p, hive) in std::mem::take(&mut self.hives).into_iter().enumerate() {
            if p == at {
                hives.push(merged.take().expect("pushed once"));
            }
            if in_group(p) {
                new_pos[p] = merged_pos as u32;
            } else {
                new_pos[p] = hives.len() as u32;
                hives.push(hive);
            }
        }
        self.hives = hives;

        for x in 1..=self.t.n {
            self.home[x as usize] = match self.home[x as usize] {
                Home::Live(p) if in_group(p as usize) && merged_numbers.contains(x) => Home::Live(merged_pos as u32),
                Home::Live(p) if in_group(p as usize) => {
                    Home::Merged { had_free_factors: had_free_factors[p as usize] }
                }
                Home::Live(p) => Home::Live(new_pos[p as usize]),
                stale @ (Home::Merged { .. } | Home::OffBoard) => stale,
            };
        }
        merged_pos
    }

    // -----------------------------------------------------------------------------------------------------
    // Results

    /// The factors of hive `i` that could be promoted: those that could be dropped from the hive without
    /// stopping its remaining sources from being matched, and that some hive holding one of their factors
    /// could take on as a source.
    pub fn hive_promotion_candidates(&self, i: usize) -> Bits<W> {
        let hive = &self.hives[i];
        let (sources, factors) = (hive.remaining_sources(), hive.remaining_factors());
        factors
            .iter()
            .filter(|&f| {
                self.count_freedoms(sources, factors.without(f)).is_some()
                    && self.any_hive_of(self.factors_of[f as usize], |h| self.can_add_source(h, f))
            })
            .collect()
    }

    /// The promotion candidates of every hive.
    pub fn promotion_candidates(&self) -> Bits<W> {
        (0..self.hives.len()).fold(Bits::EMPTY, |acc, i| acc | self.hive_promotion_candidates(i))
    }

    /// The impossible sources of every hive.
    pub fn impossible(&self) -> Bits<W> {
        self.hives.iter().fold(Bits::EMPTY, |acc, h| acc | h.impossible)
    }

    /// A legal order in which to play the sources of hive `i`.
    ///
    /// The game is built from both ends.  In each round, first every source that has exactly one factor left
    /// (visited in ascending order against a snapshot) is played now, with that factor; then every factor
    /// wanted by exactly one source is paired with that source, which is set aside for the end of the hive's
    /// game (the sources set aside later are played earlier).
    pub fn hive_solution(&self, i: usize) -> Vec<u32> {
        let hive = &self.hives[i];
        let (mut sources, mut factors) = (hive.sources, hive.factors);
        let mut now = Vec::with_capacity(sources.len() as usize);
        let mut at_end = Vec::new();
        while !sources.is_empty() {
            let mut progress = false;
            let snapshot = sources;
            for s in snapshot.iter() {
                let open = self.factors_of[s as usize] & factors;
                if open.len() == 1 {
                    now.push(s);
                    sources.remove(s);
                    factors -= open;
                    progress = true;
                }
            }
            let snapshot = factors;
            for f in snapshot.iter() {
                let wanting = self.composites_of[f as usize] & sources;
                if wanting.len() == 1 {
                    let m = wanting.min().expect("one move");
                    at_end.push(m);
                    sources.remove(m);
                    factors.remove(f);
                    progress = true;
                }
            }
            assert!(progress, "hive {} cannot be played out: {sources:?} / {factors:?}", hive.name);
        }
        now.extend(at_end.into_iter().rev());
        now
    }

    /// The whole game with no promotions beyond those the apiary was built with: the hives' games, one after
    /// the other in apiary order.
    pub fn solution(&self) -> Vec<u32> {
        let mut moves = Vec::with_capacity(self.t.n as usize);
        for i in 0..self.hives.len() {
            moves.extend(self.hive_solution(i));
        }
        moves
    }
}

/// Small counters for the numbers of a board, kept on the stack.
struct Degrees<const W: usize>([[u16; 64]; W]);

impl<const W: usize> Degrees<W> {
    fn new() -> Self {
        Degrees([[0; 64]; W])
    }

    fn set(&mut self, x: u32, value: u32) {
        self.0[(x >> 6) as usize][(x & 63) as usize] = value as u16;
    }

    /// Subtracts one and returns the new value.
    fn decrement(&mut self, x: u32) -> u16 {
        let slot = &mut self.0[(x >> 6) as usize][(x & 63) as usize];
        *slot -= 1;
        *slot
    }
}

/// Root of `x`'s set in a union-find forest, flattening the path on the way.
fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let grandparent = parent[parent[x as usize] as usize];
        parent[x as usize] = grandparent;
        x = grandparent;
    }
    x
}

/// Spreadsheet-style label for position `i`: A, B, ... Z, AA, AB, ...
fn column_name(i: usize) -> String {
    let letter = char::from(b'A' + (i % 26) as u8);
    if i >= 26 {
        format!("{}{letter}", column_name(i / 26 - 1))
    } else {
        letter.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::replay;

    /// The Java `countFreedoms`, verbatim: a scan per step.  The reference for `Apiary::count_freedoms`.
    fn naive_count_freedoms<const W: usize>(a: &Apiary<W>, moves: Bits<W>, factors: Bits<W>) -> Option<u32> {
        if moves.is_empty() {
            return Some(factors.len());
        }
        if moves.len() > factors.len() {
            return None;
        }
        for m in moves.iter() {
            let usable = a.factors_of[m as usize] & factors;
            match usable.len() {
                0 => return None,
                1 => return naive_count_freedoms(a, moves.without(m), factors - usable),
                _ => {}
            }
        }
        for f in factors.iter() {
            let wanting = a.composites_of[f as usize] & moves;
            if wanting.len() == 1 {
                return naive_count_freedoms(a, moves - wanting, factors.without(f));
            }
        }
        None
    }

    /// Tiny xorshift64 generator, so the tests need no crates and are reproducible.
    struct XorShift(u64);

    impl XorShift {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        /// A random percentage in `lo..=hi`.
        fn percent(&mut self, lo: u64, hi: u64) -> u64 {
            lo + self.next() % (hi - lo + 1)
        }

        /// A random subset of `set` in which each member is kept with probability `percent`%.
        fn subset<const W: usize>(&mut self, set: Bits<W>, percent: u64) -> Bits<W> {
            set.iter().filter(|_| self.next() % 100 < percent).collect()
        }
    }

    /// Hives of ascending size, plus a union of several hives' sources against all factors, as the places
    /// where the matching is asked about.
    fn check_count_freedoms<const W: usize>(a: &Apiary<W>, rng: &mut XorShift) {
        let all_factors = a.hives.iter().fold(Bits::<W>::EMPTY, |acc, h| acc | h.factors);
        let all_sources = a.hives.iter().fold(Bits::<W>::EMPTY, |acc, h| acc | h.sources | h.impossible);
        for hive in &a.hives {
            for _ in 0..40 {
                let percent = rng.percent(20, 100);
                let moves = rng.subset(hive.sources | hive.impossible, percent);
                let factors = match rng.next() % 3 {
                    0 => hive.factors,
                    1 => {
                        let percent = rng.percent(70, 100);
                        rng.subset(hive.factors, percent)
                    }
                    _ => rng.subset(all_factors, 30),
                };
                assert_eq!(
                    a.count_freedoms(moves, factors),
                    naive_count_freedoms(a, moves, factors),
                    "moves {moves:?} factors {factors:?}"
                );
            }
        }
        for _ in 0..40 {
            let percent = rng.percent(5, 25);
            let moves = rng.subset(all_sources, percent);
            let percent = rng.percent(50, 100);
            let factors = rng.subset(all_factors, percent);
            assert_eq!(a.count_freedoms(moves, factors), naive_count_freedoms(a, moves, factors));
        }
    }

    #[test]
    fn count_freedoms_matches_the_plain_scan() {
        let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
        for n in [10u32, 30, 85, 120, 200, 301] {
            let t = Tables::new(n);
            let plain = Apiary::<5>::new(&t, Bits::EMPTY);
            check_count_freedoms(&plain, &mut rng);
            // With promotions the sources, sinks and tables change shape.
            for _ in 0..4 {
                let promotable = plain.promotion_candidates();
                let promoted = rng.subset(promotable, 50);
                check_count_freedoms(&Apiary::<5>::new(&t, promoted), &mut rng);
            }
        }
    }

    #[test]
    fn count_freedoms_small_cases() {
        let t = Tables::new(20);
        let a = Apiary::<1>::new(&t, Bits::EMPTY);
        let set = |v: &[u32]| Bits::<1>::from_slice(v);
        // No moves: all factors are free.
        assert_eq!(a.count_freedoms(set(&[]), set(&[2, 3])), Some(2));
        // More moves than factors.
        assert_eq!(a.count_freedoms(set(&[12, 14]), set(&[6])), None);
        // 12 = 2^2*3 has maximal factors 4 and 6; 14 has 2 and 7; both can be paid for.
        assert_eq!(a.count_freedoms(set(&[12, 14]), set(&[4, 6, 7])), Some(1));
        // 14 has no factor in {4, 6}.
        assert_eq!(a.count_freedoms(set(&[12, 14]), set(&[4, 6])), None);
    }

    #[test]
    fn hives_of_ten() {
        let t = Tables::new(10);
        let a = Apiary::<1>::new(&t, Bits::EMPTY);
        let shape: Vec<(&str, Vec<u32>, Vec<u32>)> =
            a.hives().iter().map(|h| (h.name(), h.sources().to_vec(), h.factors().to_vec())).collect();
        assert_eq!(shape, vec![("A", vec![7], vec![1]), ("B", vec![6, 9, 10], vec![2, 3, 5]), ("C", vec![8], vec![4])]);
        assert_eq!(a.solution(), vec![7, 9, 6, 10, 8]);
        assert!(a.impossible().is_empty() && a.promotion_candidates().is_empty());
        assert!(a.hives().iter().all(|h| !h.has_free_factors() && h.remaining_sources().is_empty()));
    }

    #[test]
    fn hives_of_one_and_two() {
        let t1 = Tables::new(1);
        let a = Apiary::<1>::new(&t1, Bits::EMPTY);
        assert_eq!(a.hives().len(), 1);
        assert!(a.hives()[0].sources().is_empty());
        assert_eq!(a.impossible().to_vec(), vec![1]);
        assert!(a.solution().is_empty());
        let t2 = Tables::new(2);
        let a = Apiary::<1>::new(&t2, Bits::EMPTY);
        assert_eq!(a.solution(), vec![2]);
    }

    #[test]
    fn column_names() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(27), "AB");
        assert_eq!(column_name(51), "AZ");
        assert_eq!(column_name(52), "BA");
        assert_eq!(column_name(26 * 27 - 1), "ZZ");
        assert_eq!(column_name(26 * 27), "AAA");
    }

    /// `ApiaryTest.theMysteryOf528`: with these promotions the apiary's solution must be a legal game.
    #[test]
    fn the_mystery_of_528() {
        let t = Tables::new(528);
        let promoted = Bits::<9>::from_slice(&[
            128, 154, 168, 171, 174, 175, 180, 189, 195, 196, 198, 207, 208, 228, 230, 231, 234, 238, 240, 243, 244,
            245, 248, 250, 252, 255, 258, 260, 261, 264,
        ]);
        let moves = Apiary::new(&t, promoted).solution();
        assert!(replay(528, &moves).is_ok());
    }

    #[test]
    fn solutions_are_legal_games() {
        for n in [3u32, 17, 60, 85, 100, 211, 300] {
            let t = Tables::new(n);
            let a = Apiary::<5>::new(&t, Bits::EMPTY);
            let moves = a.solution();
            assert!(replay(n, &moves).is_ok(), "n={n}");
            // Whatever the hives promise as candidates can be promoted without breaking the game.
            for c in a.promotion_candidates().iter() {
                let promoted = Bits::from_slice(&[c]);
                let moves = Apiary::<5>::new(&t, promoted).solution();
                assert!(replay(n, &moves).is_ok(), "n={n} promoting {c}");
            }
        }
    }

    #[test]
    fn merging_places_and_remaps() {
        let t = Tables::new(85);
        let mut a = Apiary::<2>::new(&t, Bits::EMPTY);
        let names = |a: &Apiary<2>| a.hives().iter().map(|h| h.name().to_owned()).collect::<Vec<_>>();
        assert_eq!(names(&a), ["A", "B", "C", "D", "E", "F", "G", "H"]);
        // F and G hold the numbers 8..=42 and 16..=40; merge them, listing G first.
        let g = a.hive_index_of(16).unwrap();
        let f = a.hive_index_of(8).unwrap();
        assert!(g > f);
        let (sources_f, sources_g) = (a.hives()[f].sources(), a.hives()[g].sources());
        let merged = a.merge(&[g, f]);
        // The merged hive sits where the first listed hive (G) was, minus the removed F before it.
        assert_eq!(merged, g - 1);
        assert_eq!(a.hives().len(), 7);
        assert_eq!(a.hives()[merged].name(), "G+F");
        assert_eq!(a.hives()[merged].sources(), sources_f | sources_g);
        assert_eq!(a.hive_index_of(8), Some(merged));
        assert_eq!(a.hive_index_of(16), Some(merged));
        assert_eq!(a.hive_index_of(1), Some(0));
        assert_eq!(a.hive_index_of(4), Some(2));
        assert_eq!(a.hive_index_of(32), Some(merged + 1)); // H, which followed G
        assert_eq!(a.hives()[merged].free_factor_count(), 10);
    }

    #[test]
    fn merging_leaves_impossible_sources_behind() {
        let t = Tables::new(114);
        let mut a = Apiary::<2>::new(&t, Bits::EMPTY);
        let with_impossible = (0..a.hives().len()).find(|&i| !a.hives()[i].impossible().is_empty()).unwrap();
        let other = (0..a.hives().len()).find(|&i| i != with_impossible && !a.hives()[i].sources().is_empty()).unwrap();
        let left_behind = a.hives()[with_impossible].impossible();
        let kept = a.hives()[with_impossible].sources() | a.hives()[with_impossible].factors();
        let merged = a.merge(&[other, with_impossible]);
        for x in left_behind.iter() {
            assert_eq!(a.hive_index_of(x), None, "impossible source {x}");
        }
        for x in kept.iter() {
            assert_eq!(a.hive_index_of(x), Some(merged), "number {x}");
        }
        assert!(a.hives()[merged].impossible().is_empty());
        // The merged hive re-derives what it needs, and the apiary stays consistent.
        a.finish_setup();
        let hive = &a.hives()[merged];
        assert_eq!(hive.free_factor_count(), hive.factors().len() as i32 - hive.sources().len() as i32);
        assert!(a.hive_index_of(left_behind.min().unwrap()).is_none());
    }
}
