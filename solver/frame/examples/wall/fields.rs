//! The multi-field bound (A1) at a search node: the GF(2) frame matroids and a family of GF(3) signed frame matroids,
//! coupled by Lagrangian multipliers that a column generation fits, for the node-choice measurement (`choice.rs`).  The
//! bound, its proof (the signed frame theorem (B1) and the decomposition bound (B4), docs/theory/frame-bounds.md) and
//! the study that measured it at the roots and in a branch and bound are in
//! solver/notebook/search/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md and
//! solver/notebook/search/the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md; its code is that study's,
//! from commit 8753189 (`dw` the bound and its fit, `gf3` the signed matroids, `cores` the signings the fit derives
//! from the cores of its columns, `lp` the master's simplex, `frames` the GF(2) matroids as vectors, `fit` a root fit
//! as a file), adapted to today's library and cut to what a node needs.
//!
//! Three things are made here, and nothing else is public:
//!
//! * [`fit_root`]: the *root fit* of a game, the main arm of the study (the plain signing of frames 1 to 3 and the
//!   signings derived from the cores the master's columns run into, certified: the exact checks and the separation),
//!   within a budget in seconds.  It is the one fit a game has, `choice-prepare` writes it beside the game's prepared
//!   file (`nNNNN.fit.json`, [`SavedFit`]'s format, which `testdata/fits` has for the games the study fitted) and
//!   `choice` reads it back ([`Root`]).
//! * [`Node::warm_refit`], the study's *R2* at a node: the root fit's multipliers (Tier 1: one evaluation of `L`), then
//!   a column generation of at most `pricings` evaluations from the root's master columns and rank inequalities that
//!   still hold at the node (`Warm::child`), with no exact check and no separation: what a branch and bound can
//!   afford at every node it leaves open.
//! * [`Node::full`], the study's *reference* (`nodes` mode of E1, the audit of E3): the node's own master from the root
//!   fit's multipliers and frozen signings, with the exact checks and the separation of the node's own cores, within
//!   a budget in seconds.
//!
//! Every bound returned is `⌊L⌋` at some multipliers and signings, valid whatever the master proposes (B1 and B4, at a
//! node by B1 (2)): the node's completions weigh at most that.  A node is *cut* when it is at most the `target` the
//! node was made with (`need - 1`: the most a completion may reach for the incumbent to stand).

#[path = "fields/cores.rs"]
pub mod cores;
#[path = "fields/dw.rs"]
mod dw;
#[path = "fields/fit.rs"]
mod fit;
#[path = "fields/frames.rs"]
mod frames;
#[path = "fields/gf3.rs"]
mod gf3;
/// The simplex, which `cycles.rs` uses as well.
#[path = "fields/lp.rs"]
pub(crate) mod lp;

use dw::{bound, refit_r2, tier1, Ctx, Family, Settings, Stats, Warm};
use frames::{frame_matroids, FrameMatroids};
use gf3::Signing;
use std::path::Path;
use taxman::bits::Bits;
use taxman::model::{FrameGraph, FrameSet};
use taxman::search::Prepared;

pub use dw::Stop;
pub use fit::SavedFit;

/// The frames whose plain signing (all values 1) starts a root fit: the study's main arm, `f1cores`.
const PLAIN_FRAMES: [usize; 3] = [1, 2, 3];

/// A root fit made by [`fit_root`], with how its computation ended.
pub struct Fitted {
    pub saved: SavedFit,
    /// Why it ended: [`Stop::Converged`] when the bracket `[lb, l_best]` closed, else the budget that ran out.
    pub stop: Stop,
    /// The certified lower bound on the LP over the final family of signings (0 if none was certified).
    pub lb: f64,
}

/// The root fit of game `n` (the board of `p`), aimed at `incumbent` (the game's known optimum, or an incumbent):
/// the main arm, certified, within `seconds`.  `gf2` is the root's GF(2) bound (the fit stops closing at a share of its
/// gap to `incumbent`).
pub fn fit_root<const W: usize>(p: &Prepared<W>, n: u32, incumbent: u64, gf2: u64, seconds: f64) -> Fitted {
    let fm = frame_matroids(&p.model, &p.root, &p.candidates);
    let mut ctx = Ctx::new(n, &p.model, &p.root, &fm, p.max_total as usize, incumbent as i64, false);
    ctx.gap = Some(gf2 as i64 - incumbent as i64);
    let signings = PLAIN_FRAMES.iter().filter(|&&h| h < p.root.len()).map(|&h| Signing::ones(h)).collect();
    let settings = Settings { seconds, ..Settings::default() };
    let fitted = bound(&ctx, signings, None, true, &settings);
    Fitted { saved: SavedFit::of(n, incumbent, gf2 as i64, "certified", &fitted), stop: fitted.stop, lb: fitted.lb }
}

/// The root fit of a game, as the nodes of the game start from it.
pub struct Root {
    saved: SavedFit,
    warm: Warm,
}

impl Root {
    /// The fit of the file `path`, checked: it is of game `n` and gives its bound again at the root of `p`.
    pub fn load<const W: usize>(path: &Path, p: &Prepared<W>, n: u32) -> Result<Root, String> {
        Root::new(SavedFit::load(path)?, p, n).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The fit `saved`, checked as [`load`](Self::load) does.
    pub fn new<const W: usize>(saved: SavedFit, p: &Prepared<W>, n: u32) -> Result<Root, String> {
        if saved.n != n {
            return Err(format!("the fit is of game {}, not {n}", saved.n));
        }
        let fm = frame_matroids(&p.model, &p.root, &p.candidates);
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, p.max_total as usize, saved.opt as i64, false);
        let again = saved.bound(&ctx);
        if again != saved.bound {
            return Err(format!(
                "the fit gives a bound of {again} at the root, not the {} it was saved with",
                saved.bound
            ));
        }
        let warm = Warm::from_saved(&saved);
        Ok(Root { saved, warm })
    }

    pub fn saved(&self) -> &SavedFit {
        &self.saved
    }
}

/// The budgets of [`Node::warm_refit`].
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The most evaluations of `L` after the first (Tier 1).
    pub pricings: usize,
    /// The refit's seconds.
    pub seconds: f64,
}

/// What a bound found at a node.
#[derive(Clone, Debug)]
pub struct Found {
    /// The lowest `⌊L⌋` evaluated: a bound on the node's completions.
    pub bound: i64,
    /// `⌊L⌋` at the root fit's multipliers alone, if the method evaluates it (the warm refit does).
    pub tier1: Option<i64>,
    /// The evaluations of `L` (for the warm refit, those after Tier 1).
    pub pricings: usize,
    /// Why it ended: [`Stop::Target`] when the bound reached the target (the node is cut), [`Stop::Seconds`] when the
    /// budget ran out, and so on.
    pub stop: Stop,
    /// The separation rounds (the full bound's).
    pub rounds: usize,
}

impl Found {
    /// Whether it ended before settling: out of seconds, pricings or rounds, or a master that broke down twice.
    pub fn capped(&self) -> bool {
        !matches!(self.stop, Stop::Target | Stop::Converged | Stop::Fitted)
    }
}

/// A search node: its state, promotions and candidates, and its GF(2) frame matroids.
pub struct Node<'a, const W: usize> {
    n: u32,
    model: &'a FrameGraph,
    state: &'a FrameSet<W>,
    /// The node's promotions, and the root's candidates that are neither promoted nor still candidates there.
    promoted: Vec<u32>,
    removed: Vec<u32>,
    /// The most promotions a completion makes.
    cap: usize,
    /// What a bound must reach to cut the node: `need - 1`.
    target: i64,
    fm: FrameMatroids,
}

impl<'a, const W: usize> Node<'a, W> {
    /// The node of game `n` with promotions `promoted` and candidates `cands`, whose state is `state`, at most `cap`
    /// more promotions, and cut by a bound of at most `target`.
    pub fn new(
        p: &'a Prepared<W>,
        n: u32,
        state: &'a FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        cap: u32,
        target: i64,
    ) -> Self {
        let removed = (p.candidates - *promoted - *cands).to_vec();
        Node {
            n,
            model: &p.model,
            state,
            promoted: promoted.to_vec(),
            removed,
            cap: cap as usize,
            target,
            fm: frame_matroids(&p.model, state, cands),
        }
    }

    fn ctx(&self) -> Ctx<'_, W> {
        Ctx::new(self.n, self.model, self.state, &self.fm, self.cap, self.target, true)
    }

    /// The study's R2: Tier 1 at the root fit's multipliers, and unless that cuts the node, a warm column generation of
    /// at most `limits.pricings` more evaluations.
    pub fn warm_refit(&self, root: &Root, limits: &Limits) -> Found {
        let ctx = self.ctx();
        let warm = root.warm.child(&self.promoted, &self.removed, &self.fm.elems, self.cap);
        let mut stats = Stats::default();
        let fam = Family::new(&ctx, warm.signings.clone());
        let (first_bound, first) = tier1(&ctx, &fam, &warm, &mut stats);
        if first_bound <= self.target {
            return Found { bound: first_bound, tier1: Some(first_bound), pricings: 0, stop: Stop::Target, rounds: 0 };
        }
        let settings = Settings { seconds: limits.seconds, ..Settings::default() };
        let (refitted, trace) = refit_r2(&ctx, fam, &warm, first, limits.pricings, &settings, &mut stats);
        let (bound, tier1) = (first_bound.min(refitted), Some(first_bound));
        Found { bound, tier1, pricings: trace.pricings, stop: trace.stop, rounds: 1 }
    }

    /// The study's reference: the node's own master from the root fit's multipliers and frozen signings, with the
    /// exact checks and the node's separation, within `seconds`, stopping at the target.
    pub fn full(&self, root: &Root, seconds: f64) -> Found {
        let ctx = self.ctx();
        let settings = Settings { seconds, ..Settings::default() };
        let r = bound(&ctx, root.warm.signings.clone(), Some(&root.warm.lambda), true, &settings);
        Found { bound: r.bound, tier1: None, pricings: r.stats.pricings_mi as usize, stop: r.stop, rounds: r.rounds }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::board::Board;
    use taxman::game::Optimal;
    use taxman::gf2::{Gf2Bound, Gf2Graph};
    use taxman::search::SearchTactics;

    /// A root fit makes a bound between P* and the root's GF(2) bound, reads back from its file's object as
    /// the same fit, and is refused for another game or when its bound is not the one it was saved with.
    fn a_root_fit_is_checked_against_its_game<const W: usize>(n: u32) {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let opt = Optimal::built_in().score(n).expect("a built-in optimum") - board.in_the_bag;
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut gf2 = Gf2Bound::<W>::new(&graph);
        gf2.reset(&Bits::EMPTY, &p.candidates);
        let root = gf2.evaluate(p.max_total, None).value;

        let fitted = fit_root(&p, n, opt, root, 3.0);
        let saved = fitted.saved;
        assert!(saved.bound >= opt as i64 && saved.bound <= root as i64, "{} in {opt}..={root}", saved.bound);
        let again = SavedFit::from_json(&saved.to_json()).expect("the fit's object");
        assert!(Root::new(again, &p, n).is_ok(), "a fit that was just made");

        let other = SavedFit::from_json(&saved.to_json()).expect("the fit's object");
        let error = Root::new(SavedFit { n: n + 1, ..other }, &p, n).err().expect("a fit of another game");
        assert!(error.contains("not"), "{error}");
        let other = SavedFit::from_json(&saved.to_json()).expect("the fit's object");
        let error = Root::new(SavedFit { bound: other.bound + 1, ..other }, &p, n).err().expect("a changed bound");
        assert!(error.contains("bound"), "{error}");
    }

    #[test]
    fn a_root_fit_is_checked_against_its_game_at_600() {
        taxman::with_width!(600, W => a_root_fit_is_checked_against_its_game::<W>(600));
    }
}
