//! A searcher's hook for studies ([`StepHook`]): told what every step saw and did, and asked which member of a
//! conflict to branch on.
//!
//! A search runs without one ([`SearchSpace::hooks`](super::SearchSpace::hooks) is `None`, as
//! [`SearchSpace::new`](super::SearchSpace::new) makes it), and then each site of the hook in a step is a test of an
//! `Option` that is always `None`, as the sites of the [`Diagnostics`](super::Diagnostics) are.  With one, each
//! searcher makes its own hook when it is made, and the hook sees the steps of that searcher in their order.  What
//! the hook is told is a copy: it cannot change the step.  What it is asked ([`StepHook::branch`]) can change the
//! tree, never the answer: every completion of a node leaves out a member of the conflict of its heaviest set, so
//! branching on any member, and going on without it, explores every completion (DESIGN.md, section 4, "The default
//! step", the second of the facts its exactness rests on).  The searcher checks that the member is one.
//!
//! The hook is the GF(2) path's (the tactic `gf2`, the default): on the matching path it is told of the steps, without
//! their bound checks, and asked nothing.

use crate::bits::Bits;
use crate::model::FrameSet;

/// Makes a searcher's hook: [`SearchSpace::hooks`](super::SearchSpace::hooks).
pub type MakeHook<const W: usize> = dyn Fn() -> Box<dyn StepHook<W>> + Sync;

/// What a study is told of a search's steps, and asked by it (see the module docs).
pub trait StepHook<const W: usize> {
    /// After each step: what it saw and did, the state of its node, and the state of the child it made (`None` if
    /// it made none, or the promotion failed).
    fn step(&mut self, record: &StepRecord<W>, state: &FrameSet<W>, child: Option<&FrameSet<W>>);

    /// The member of `conflict` (a node's conflict of two or more candidates, none of them forced) that the node
    /// branches on, where strong branching chose `strong`.  It must be a member of `conflict`.  Asked only with
    /// the default branching, [`Gf2Branching::Exact`](super::Gf2Branching::Exact), when strong branching found no
    /// member forced.
    fn branch(&mut self, conflict: &Bits<W>, strong: u32) -> u32 {
        let _ = conflict;
        strong
    }
}

/// What one step of a node's loop saw and did.
#[derive(Clone, Copy, Debug, Default)]
pub struct StepRecord<const W: usize> {
    /// The node: its promotions, the candidates its loop had when the step began, and what a completion must add to
    /// beat the incumbent.
    pub promoted: Bits<W>,
    pub cands: Bits<W>,
    pub need: u64,
    /// The node's bound check in this step: `None` if the step ended before it (the cheap checks, or the rule of
    /// symmetric pairs), or if the verdict was the one the node's check as a child left (`inherited`).
    pub node: Option<BoundCheck<W>>,
    pub inherited: bool,
    /// The member the node branched on, and what its child promotes (that member, or all the forced members);
    /// `forced` is the node's forced members when it chose (the weight split's and strong branching's).
    pub branch: Option<u32>,
    pub added: Bits<W>,
    pub forced: Bits<W>,
    /// The child's bound check (`None` if it had none: the promotion failed, the cheap checks cut it, or it had no
    /// candidates).
    pub child: Option<BoundCheck<W>>,
    /// How the step ended.
    pub end: StepEnd,
}

/// A GF(2) bound check: of a node in its step, or of a child as it is made.
#[derive(Clone, Copy, Debug, Default)]
pub struct BoundCheck<const W: usize> {
    /// The bound, its heaviest set, and the members of that set forced by the weight split.
    pub value: u64,
    pub chosen: Bits<W>,
    pub forced: Bits<W>,
    /// The conflict of the heaviest set, if the check got as far as one (the bound reached the need and the set
    /// cannot be promoted), and whether it was computed (and learned as a nogood) rather than known.
    pub conflict: Option<Bits<W>>,
    pub computed: bool,
}

/// How a step ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepEnd {
    /// The node's loop ended, for the reason ([`cut`](super::cut)).
    LoopEnd(usize),
    /// The promotion that makes the child failed.
    ChildFailed,
    /// The child was made and left unexplored, for the reason.
    ChildCut(usize),
    /// The child had no candidates: offered, not explored.
    Leaf,
    /// The child was pushed: the next step is its first.
    #[default]
    Child,
}
