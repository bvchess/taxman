//! A searcher's hook for studies ([`StepHook`]): told what every step saw and did, and asked which member of a
//! conflict to branch on.
//!
//! The hook, and every site of it in the searcher, is built only with the feature `studies`: a default build's steps
//! are what they were before it, with not even a test of an `Option`.  With the feature, a search runs without one
//! ([`SearchSpace::hooks`](super::SearchSpace::hooks) is `None`, as [`SearchSpace::new`](super::SearchSpace::new)
//! makes it), and then each site of the hook in a step is a test of an `Option` that is always `None`.  With one, each
//! searcher makes its own hook when it is made, and the hook sees the steps of that searcher in their order.  What
//! the hook is told is a copy: it cannot change the step.  What it is asked ([`StepHook::branch`]) can change the
//! tree, never the answer: every completion of a node leaves out a member of the conflict of its heaviest set, so
//! branching on any member, and going on without it, explores every completion (DESIGN.md, section 4, "The default
//! step", the second of the facts its exactness rests on).  The searcher checks that the member is one.  A hook may
//! also narrow a node's candidates by a rule of its own ([`StepHook::narrow`]), which prunes as the rule of symmetric
//! pairs does: that can change the answer, and is for a study of a rule whose soundness it has argued.  It may propose
//! sets for the searcher to learn conflicts within, as nogoods ([`StepHook::propose`]): the searcher learns only what
//! a peel shows cannot be promoted, so that changes the tree, never the answer.  And it may make the search count the
//! completions that reach the need instead of beating it ([`StepHook::counting`]).
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

    /// Before a node branches on `member` and goes on without it (no member is forced, so the node's loop goes on
    /// after this child): the node's state, promotions and candidates (`member` among them), and the conflict it
    /// branches on (`member` among its members; `None` for a node with no conflict, which branches on its largest
    /// candidate).  Told only on the GF(2) path.
    fn excluding(
        &mut self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        conflict: Option<&Bits<W>>,
        member: u32,
    ) {
        let _ = (state, promoted, cands, conflict, member);
    }

    /// The candidates `cands` of a node with the promotions `promoted`, at a step after its first (after the rule of
    /// symmetric pairs), or of a child as it is checked, which a study may narrow by a rule of its own.  Returns true
    /// if no completion keeps the rule: then the node's loop ends, or the child is cut, as for the rule of symmetric
    /// pairs ([`cut::SYMMETRY`](super::cut::SYMMETRY)).  The candidates it drops leave the node's bounds as that rule's
    /// do.  This is the one site where a hook can change the answer: the rule's soundness is the study's to prove.
    fn narrow(&mut self, promoted: &Bits<W>, cands: &mut Bits<W>) -> bool {
        let _ = (promoted, cands);
        false
    }

    /// When a bound check has computed, and learned, the conflict `conflict` of its heaviest set `chosen` at a node
    /// with the state `state`, the promotions `promoted` and the candidates `cands`: more sets for the searcher to
    /// learn, put in `more`.  The searcher learns, of each, a conflict that a peel finds within it, as it learned
    /// `conflict` (a nogood, and known for the node and its subtree), unless the nogoods it holds already give one
    /// there, and then tells the hook ([`learned`](Self::learned)).  A set that can be promoted has no conflict within
    /// it and teaches nothing, so this changes the tree, never the answer, whatever the study proposes.  Asked only on
    /// the GF(2) path.
    fn propose(
        &mut self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        chosen: &Bits<W>,
        conflict: &Bits<W>,
        more: &mut Vec<Bits<W>>,
    ) {
        let _ = (state, promoted, cands, chosen, conflict, more);
    }

    /// A conflict the searcher learned within a set that [`propose`](Self::propose) proposed, and the nogood it made of
    /// it (`None` without the tactic `nogoods`, which keeps the conflict for the node and its subtree only).
    fn learned(&mut self, set: &Bits<W>, conflict: &Bits<W>, nogood: Option<&Bits<W>>) {
        let _ = (set, conflict, nogood);
    }

    /// Whether the search *counts* (asked at each offer and at each heaviest set that can be promoted; the answer must
    /// not change).  A counting search makes every completion of its unit that reaches the need as a child, once,
    /// instead of looking for one better than the incumbent.  It offers nothing, so the incumbent and the need stay
    /// where they began; and a node whose heaviest set can be promoted, which a search ends there, branches on that
    /// set's members as on a conflict's (every other completion leaves one of them out), to make that set and every
    /// other.  The promotion sets of the children made are distinct, so the hook counts a completion at the step that
    /// made it, explored or not (the end [`StepEnd::Child`], [`StepEnd::Leaf`] or [`StepEnd::ChildCut`]: a child with
    /// nothing more to promote is cut as exact).  Only with the GF(2) bound, and without the rule of symmetric pairs,
    /// which keeps one side of each pair.
    fn counting(&self) -> bool {
        false
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
