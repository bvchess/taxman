//! The dual bound of a stopped search: what a search that a limit stopped proves about P* from above.
//!
//! A search that a limit (or a stop) ends early has an incumbent, its best promotion set so far, and the open units it
//! has not explored: the engine's deque and the units its workers held, which its final checkpoint holds, exactly what
//! a resume needs ([`crate::engine`], "Checkpoint").  The search is exact (solver/docs/design/search.md: every pruning
//! drops only sets no heavier than the incumbent of its time, sets that cannot be promoted, or, with the tactic
//! `symmetry`, sets that break an inequality every heaviest set keeps, the prime-exchange theorem (R2,
//! docs/theory/search-rules.md)), so every promotable set that a resume would have to find, one heavier than the
//! incumbent (keeping the inequalities, under `symmetry`), is a completion of an open unit's node: its promotions, plus
//! some of its candidates.  So the best promotion sum of the game is at most the larger of the incumbent and, over the
//! open units, the node's promotion sum plus a bound on what its completions add.  The bound of a node is the one the
//! search computes when it starts the unit ([`TaxSearcher`](super::TaxSearcher)`::start`, then its first check): the
//! GF(2) bound of the node's candidates given its promotions, with at most [`max_additional`] more promotions (the
//! frame bound (B2), docs/theory/frame-bounds.md), which holds for every completion whatever the tactics.  This is the
//! *dual bound* of the stopped branch and bound (the dual-bound lemma (R4), docs/theory/search-rules.md), and a stopped
//! search usually throws it away; [`dual_bound`] reads it from the checkpoint, without the engine.  Like every bound of
//! the frame model it bounds the frame model's optimum, which is opt(n) given statement U at n (solver/frame/README.md,
//! “What "optimal" means”).

use super::header::Header;
use super::setup::Prepared;
use super::TaxUnit;
use crate::bounds::max_additional;
use crate::engine::{self, Unit};
use crate::gf2::{Gf2Bound, Gf2Graph};
use crate::model::Scratch;
use std::path::Path;

/// The bound a search computes at the node of a unit when it starts it: [`NodeBound::completions`].
pub struct NodeBound<'a, const W: usize> {
    prepared: &'a Prepared<W>,
    gf2: Gf2Bound<'a, W>,
    scratch: Scratch<W>,
}

impl<'a, const W: usize> NodeBound<'a, W> {
    /// The bounds of the nodes of the search of `prepared`, whose root's GF(2) graph is `graph`
    /// (`Gf2Graph::new(&prepared.model, &prepared.root, &prepared.candidates)`, as the search builds it).
    pub fn new(prepared: &'a Prepared<W>, graph: &'a Gf2Graph) -> Self {
        NodeBound { prepared, gf2: Gf2Bound::new(graph), scratch: Scratch::new() }
    }

    /// The GF(2) bound on what a completion of `unit`'s node adds to its promotions: the heaviest common independent
    /// set of the node's candidates, given its promotions, of at most [`max_additional`] members.  `None` if the
    /// unit's promotions cannot be promoted from the root (no unit of a search of this game is such).
    pub fn completions(&mut self, unit: &TaxUnit<W>) -> Option<u64> {
        let p = self.prepared;
        let state = p.model.promote_all_using(&mut self.scratch, &p.root, &unit.promoted)?;
        let max_add = max_additional(&p.model, p.max_total, &state, &unit.promoted, &unit.cands);
        self.gf2.reset(&unit.promoted, &unit.cands);
        Some(self.gf2.evaluate(max_add, None).value)
    }
}

/// The dual bound of a stopped search, as promotion sums ([`dual_bound`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DualBound {
    /// The incumbent's promotion sum when the search stopped.
    pub incumbent: u64,
    /// The open units.
    pub units: u64,
    /// The largest bound of an open unit's node (its promotion sum plus [`NodeBound::completions`]), if any is open.
    pub units_bound: Option<u64>,
}

impl DualBound {
    /// The bound on the game's best promotion sum: the larger of the incumbent and every open node's bound.
    #[must_use]
    pub fn bound(&self) -> u64 {
        self.units_bound.map_or(self.incumbent, |units| units.max(self.incumbent))
    }
}

/// The dual bound of the search of `prepared` that a limit stopped, from its final checkpoint `checkpoint`: the
/// incumbent and the bound of every open unit's node (the module documentation has the argument).  The checkpoint
/// must be this game's search's, as for a resume.
///
/// # Errors
///
/// A checkpoint that cannot be read, is damaged or belongs to another search, or a unit whose promotions cannot be
/// promoted.
pub fn dual_bound<const W: usize>(prepared: &Prepared<W>, checkpoint: &Path) -> Result<DualBound, String> {
    let at = |e: String| format!("{}: {e}", checkpoint.display());
    let header = engine::read_checkpoint_header(checkpoint).map_err(|e| at(e.to_string()))?;
    let header = Header::<W>::decode(&header).map_err(at)?;
    if let Some(mismatch) = header.mismatch(&Header::of(prepared)) {
        return Err(at(mismatch));
    }
    let graph = Gf2Graph::new(&prepared.model, &prepared.root, &prepared.candidates);
    let mut nodes = NodeBound::new(prepared, &graph);
    let (mut units, mut units_bound, mut unpromotable) = (0u64, None::<u64>, false);
    let head = engine::scan_checkpoint(checkpoint, |bytes| {
        let unit = TaxUnit::<W>::decode(bytes);
        units += 1;
        match nodes.completions(&unit) {
            Some(add) => units_bound = Some(units_bound.unwrap_or(0).max(unit.promoted.sum() + add)),
            None => unpromotable = true,
        }
        Ok(())
    })
    .map_err(|e| at(e.to_string()))?;
    if unpromotable {
        return Err(at("a unit's promotions cannot be promoted from the root".into()));
    }
    Ok(DualBound { incumbent: head.best, units, units_bound })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::Bits;
    use crate::board::Board;
    use crate::engine::EngineConfig;
    use crate::game::Optimal;
    use crate::search::{search, SearchResult, SearchTactics};
    use crate::test_support::{TempDir, QUICK};
    use std::time::Duration;

    /// The search of game `n` (set up by `board`) on one thread, stopped by `node_limit` or by `time_limit`, with
    /// its checkpoint in `ckpt`, and its dual bound if it stopped (the score's promotion sum if it completed).
    fn stopped<const W: usize>(
        p: &Prepared<W>,
        ckpt: &Path,
        node_limit: Option<u64>,
        time_limit: Option<Duration>,
    ) -> (SearchResult<W>, u64) {
        let _ = std::fs::remove_file(ckpt);
        let cfg = EngineConfig {
            threads: 1,
            progress: None,
            node_limit,
            time_limit,
            checkpoint: Some((ckpt.to_path_buf(), Duration::from_secs(3600))),
            ..EngineConfig::default()
        };
        let result = search(p, &cfg, None, None, false).expect("search");
        let bound = if result.completed {
            assert!(!ckpt.exists(), "a completed search deletes its checkpoint");
            result.sum
        } else {
            let dual = dual_bound(p, ckpt).expect("the dual bound");
            assert_eq!(dual.incumbent, result.sum, "the checkpoint's incumbent is the search's");
            dual.bound()
        };
        (result, bound)
    }

    /// The root's GF(2) bound with no incumbent, as `wall landscape` computes it.
    fn root_bound<const W: usize>(p: &Prepared<W>) -> u64 {
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut bound = Gf2Bound::<W>::new(&graph);
        bound.reset(&Bits::EMPTY, &p.candidates);
        bound.evaluate(p.max_total, None).value
    }

    /// The known optimum of game `n` as a promotion sum.
    fn opt<const W: usize>(board: &Board<W>) -> u64 {
        Optimal::built_in().score(board.n()).expect("a game of optimal.json") - board.in_the_bag
    }

    /// Game `n` stopped before its first node: its checkpoint holds the root unit alone, and the dual bound is the
    /// root's GF(2) bound (the upper end of the landscape), above P*.
    fn stopped_at_the_root<const W: usize>(n: u32, ckpt: &Path) {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let (result, bound) = stopped(&p, ckpt, None, Some(Duration::ZERO));
        assert!(!result.completed, "n={n}");
        let dual = dual_bound(&p, ckpt).unwrap();
        assert_eq!(dual.units, 1, "n={n}: the root unit");
        assert_eq!(dual.units_bound, Some(root_bound(&p)), "n={n}");
        assert_eq!(bound, root_bound(&p).max(result.sum), "n={n}");
        assert!(bound >= opt(&board) && opt(&board) >= result.sum, "n={n}");
    }

    /// Every 89th game of 60..=1300 (every 267th in a quick run).
    #[test]
    fn stopped_before_its_first_node_the_dual_bound_is_the_root_bound() {
        let dir = TempDir::new("dual-root");
        let ckpt = dir.0.join("root.ckpt");
        for n in (60..=1300).step_by(if QUICK { 267 } else { 89 }) {
            crate::with_width!(n, W => stopped_at_the_root::<W>(n, &ckpt));
        }
    }

    /// Game `n` searched on one thread, stopped after more and more steps (1,024, then twice as many each time)
    /// until a search completes: the dual bound is never below P* nor above the root's bound, and it falls
    /// (never rises) as the node limit grows; the search that completes has P*, which its bound is.  Returns
    /// the number of searches its limit stopped.
    fn falls<const W: usize>(n: u32, ckpt: &Path) -> u32 {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let (opt, root) = (opt(&board), root_bound(&p));
        let (mut last, mut limit, mut stops) = (root, 1024, 0);
        loop {
            let (result, bound) = stopped(&p, ckpt, Some(limit), None);
            assert!(opt <= bound && bound <= root, "n={n}, {limit} steps: {opt} <= {bound} <= {root}");
            assert!(bound <= last, "n={n}, {limit} steps: the bound rose from {last} to {bound}");
            last = bound;
            if result.completed {
                assert_eq!((result.sum, bound), (opt, opt), "n={n}: a completed search");
                return stops;
            }
            stops += 1;
            limit *= 2;
        }
    }

    /// [`falls`] on games whose trees take a few thousand to a few tens of thousands of steps on one thread.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is the_dual_bound_brackets_the_optimum_and_falls_as_the_node_limit_grows_at_999"
    )]
    fn the_dual_bound_brackets_the_optimum_and_falls_as_the_node_limit_grows() {
        falls_on(&[996, 1020, 1218], 6);
    }

    /// [`the_dual_bound_brackets_the_optimum_and_falls_as_the_node_limit_grows`] for the game 999, whose tree takes two
    /// thousand steps.
    #[test]
    fn the_dual_bound_brackets_the_optimum_and_falls_as_the_node_limit_grows_at_999() {
        falls_on(&[999], 1);
    }

    /// [`falls`] on the games `ns`, at least `min_stops` of whose searches a limit stops.
    fn falls_on(ns: &[u32], min_stops: u32) {
        let dir = TempDir::new("dual-falls");
        let ckpt = dir.0.join("falls.ckpt");
        let stops: u32 = ns.iter().map(|&n| crate::with_width!(n, W => falls::<W>(n, &ckpt))).sum();
        assert!(stops >= min_stops, "only {stops} searches were stopped by their limit");
    }

    /// A checkpoint of another game's search is refused.
    #[test]
    fn the_dual_bound_of_another_games_checkpoint_is_refused() {
        let dir = TempDir::new("dual-other");
        let ckpt = dir.0.join("other.ckpt");
        let p = Board::<16>::for_n(1000).prepare(SearchTactics::default()).unwrap();
        let q = Board::<16>::for_n(999).prepare(SearchTactics::default()).unwrap();
        stopped(&p, &ckpt, None, Some(Duration::ZERO));
        assert!(dual_bound(&q, &ckpt).unwrap_err().contains(&ckpt.display().to_string()));
    }
}
