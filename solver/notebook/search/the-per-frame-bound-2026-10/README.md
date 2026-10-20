# The per-frame bound (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-14.  Concluded 2026-10-14.

**Question.**  [The anatomy](../the-anatomy-of-the-gap-2026-10/README.md) found that what the GF(2) bound misses is
winnability in odd cores of frames 1-3.  Does a bound exact frame by frame close a share of that residual that holds or
grows with n, at the roots and at nodes; what does it cost; and what does it do in the plain branch and bound?
**Answer.**  It is valid and the strongest bound measured where it can be computed: at the roots it closes 15-100% of
the GF(2) gap (59% on average, within 200 steps), and at the probe's nodes with the node's own multipliers it cuts 29
and 27 of 30 where the GF(2) bound cuts 20 and 14.  But each frame's exact problem holds the same odd cores, so the
share closed falls at the frontier (15-39% at 1254-1300), and in the branch and bound it is a constant factor at 28
times the time, behind the GF(2) bound with learned cuts.  Not kept.

## Method

**The bound.**  A candidate `x` of home `h` is a mover of frame `h − 1` and a factor frame `h` loses, and a set can be
promoted exactly when every frame is winnable with its part of the set applied (the frame-by-frame criterion of
`model::promote`).  A frame's *configurations* are the sets of the candidates touching it (those of home `i + 1`, which
become moves of frame `i`, and those of home `i`, which leave its factors) that keep it winnable.  Give `x` a copy in
each of its two frames and a multiplier `μ_x` on their equality, weighing it `μ_x` in frame `h − 1` and `x − μ_x` in
frame `h`.  For every `μ`, `L(μ) = Σ_i max { w_i(C) : C a configuration of frame i }` bounds every promotable set `S`
(weak duality: a promotable `S` gives each frame the winnable configuration `S ∩ (candidates touching it)`, and the
weights add to `sum(S)`).  With every frame's configurations relaxed to the sets independent in its GF(2) matroid (the
[matroid study's](../matroid-intersection-bounds-2026-10/README.md) frame matroids: the even frames' make one matroid
`A`, the odd frames' another, `B`), `min_μ L` is the GF(2) bound by Frank's weight splitting for the common independent
sets of two matroids; so with winnability where it is exact, and GF(2) elsewhere, the least `L` is at most the GF(2)
bound.

**A frame's problem** (the heaviest winnable configuration for weights `w`, only members of positive weight) is solved
by its own branch and bound.  A node fixes some members in and some out.  Its bound is the heaviest configuration
independent in the frame's GF(2) matroid with those fixed (the fixed members first, then greedy by weight), which every
winnable one is; a node whose fixed members are dependent, or whose bound is not above the incumbent, is dropped.  If
that configuration is winnable it is the node's best; if not, a minimal conflict of the frame is branched on, "`k₁ …
k_{j−1}` in, `k_j` out" for each `j`, depth first.  The incumbent starts from the heaviest of the frame's winnable
configurations found for earlier weights (a pool of up to 400 a frame) and from a dive.  Past a budget of nodes the
answer is the largest bound left open, still an upper bound.  Frames 1-3 are exact and the others use their GF(2)
matroids: frame 0's exact problem hits 20,000 nodes at most multipliers past 1000, and the GF(2) bound's heaviest set
has no core there.  The budget is 100,000 nodes a frame at the roots and the probe's nodes, 20,000 in the branch and
bound.

**The multipliers** start at a weight split of the GF(2) bound for the weights `64x` (with GF(2) in every frame `L` is
then the GF(2) bound, which every run checks) and move by subgradient steps: the subgradient of `L` at `μ` for candidate
`x` is whether the frame below its home takes it, less whether its home frame does; Polyak's rule towards `P*` (or the
node's target), step `θ (L − target)/‖g‖²` with `θ` starting at 1, halved and the multipliers reset to the best after 20
steps without a lower value, ending when `θ < 10⁻³`.  At the probe's nodes the starting point is the root's multipliers
from such steps at the root (200 for the table's roots, 150 in the branch and bound), and a node gets 20 steps of its
own (ending when `L` is at most the target).  *Column generation* was tried in place of the steps: the master is the LP
over the frames' configurations tied by the copies, each step evaluating `L` at the master's multipliers and adding
every frame's best configuration as a row, with the multipliers kept within a *box* of ±8 around the best point found
so far, and ending when the best value is within 0.25 of the master's optimum.

**Validity.**  Weak duality gives a bound for any multipliers, given each frame's answer is an upper bound on its
heaviest winnable configuration.  On 141 boards of 2-200, with 20 random multiplier vectors each (between −x/2 and
3x/2), the frame search agrees with trying every subset on all 9,401 frame problems of at most 20 positive-weight
members, and `L` is never below `P*` (2,820 values); at the 19 roots it is at least `P*`, and at the 39 probe nodes
whose best completion the matroid study found, at least that.

## Results

**1. At the root it closes 59% of the GF(2) gap within 200 steps, a share falling with n.**  The 19 games of the slope
study's margin scans, frames 1-3 exact:

| n | GF(2) gap | per-frame gap | closed | GF(2) with cuts | s |
|---|---|---|---|---|---|
| 900 | 119 | 0 | 1.00 | 5 | 13 |
| 936 | 132 | 13 | 0.90 | 31 | 142 |
| 950 | 176 | 98 | 0.44 | 58 | 265 |
| 992 | 318 | 257 | 0.19 | 143 | 351 |
| 1000 | 214 | 128 | 0.40 | 101 | 282 |
| 1025 | 147 | 84 | 0.43 | 90 | 210 |
| 1050 | 106 | 23 | 0.78 | 26 | 20 |
| 1088 | 81 | 8 | 0.90 | 16 | 35 |
| 1100 | 178 | 68 | 0.62 | 79 | 138 |
| 1150 | 77 | 46 | 0.41 | 2 | 164 |
| 1184 | 139 | 46 | 0.67 | 6 | 34 |
| 1196 | 174 | 34 | 0.81 | 58 | 72 |
| 1215 | 213 | 60 | 0.72 | 123 | 123 |
| 1225 | 105 | 45 | 0.58 | 63 | 348 |
| 1240 | 86 | 17 | 0.80 | 41 | 119 |
| 1250 | 126 | 46 | 0.63 | 40 | 168 |
| 1254 | 204 | 125 | 0.39 | 105 | 172 |
| 1288 | 209 | 141 | 0.32 | 153 | 292 |
| 1300 | 151 | 129 | 0.15 | 112 | 235 |

("GF(2) with cuts" is the anatomy's root gap with all its cuts; 900 stopped at 136 steps, having reached `P*`.)  It
closes 15-100% of the GF(2) gap, 59% on average, with a trend of −5 ± 4 points per 100 in n.  At the GF(2) split's
multipliers alone it is the GF(2) bound or up to 11 below it (the frames find winnable configurations as heavy as the
split's ties allow); all the gain is the multipliers'.  The steps are the cost: an evaluation solves frames 1-3
exactly, 0.05-2 s (frame 2 of 1150 takes 100,000 nodes at the split, and a frame's search hits the budget at the split
at 992, 1025 and 1150, the answer still a bound), and the subgradient stalls short of convergence at the frontier (1254:
37959 after 200 steps and 37958 when the step had been halved ten times, after 272, at a gap of 125).  Column
generation does no better: without the box its master's multipliers swing to extremes (`L` of 10⁵ to 3·10⁶ at 936) and
nothing improves in 100 steps; with it, 936 reaches 20528 in 400 steps (1,036 columns, 410 s), against 20520 in 200
subgradient steps (142 s).  How much the converged bound would close at the frontier is not known; what can be computed
in minutes a root closes less there.

**2. At the probe's nodes it is the strongest bound measured, at hundreds to thousands of times the cost.**  Every 10th
node; cuts of 30, and the mean of `max(0, bound − target)`:

| n | GF(2) | per-frame, root's multipliers | per-frame, 20 steps of the node's own | s a node |
|---|---|---|---|---|
| 936 | 20, 25.7 | 25, 5.9 | 29, 0.8 | 0.10, 0.66 |
| 1215 | 14, 53.5 | 23, 24.9 | 27, 5.9 | 0.46, 2.14 |

The root's cuts of the anatomy at the node's multipliers cut 26 and 22; the GF(2) bound costs 0.3-0.7 ms, so the two
per-frame columns cost 300 to 3,000 times as much.

**3. In the branch and bound it is a constant factor at many times the time.**  The anatomy's skeleton with the
per-frame bound at the root's multipliers (150 steps at the root) where the GF(2) bound does not cut, the frames' pools
seeded with the root's winnable configurations: 13 of the 19 games finish within 240 s, with 2.8 times fewer nodes than
the GF(2) bound alone (geometric mean; 740 times at 900, whose root it cuts, otherwise 1.0-7.3) in 28 times the time;
the root's cuts at the node's multipliers need fewer nodes than it at 11 of those 13 (not at 900 and 1215).  The six
that do not finish are the hard ones (992, 1025, 1225, 1254, 1288, 1300), so no slope can be fitted; at 1215 it needs
5,120 nodes against the GF(2) bound's 24,516 and the cuts' 7,104, in 216 s against 17 s and 31 s.

## Decision

Not recommended, and not kept.  The hypothesis holds where the bound can be computed: exact frames see the odd cores,
closing most of the GF(2) residual at 900, 936, 1088 and at the probe's nodes when the multipliers are the node's own.
But computing it means solving each frame's own promotion problem exactly, and those problems hold the same odd cores:
a frame's search takes up to 100,000 nodes, the multipliers need hundreds of evaluations, and within budgets the share
closed falls at the frontier.  The residual's difficulty moves into the pricing problem rather than going away.

A frame-local relaxation that sees odd cores cheaply (a bound for one frame stronger than its GF(2) matroid and
computable without search) would make it practical, and would reopen the question; none is known (Lorea's matroid
([matroid study](../matroid-intersection-bounds-2026-10/README.md), section 5), cover cuts and rank cuts of small unions
do not see the cores).

## Files

The folder holds no data.  The code (the frame search, the subgradient and the column generation) was not kept in the
tree; it was a module of the example `matroid`, which went with that study's, removed from master on 2026-10-17 and
building at commit 8753189 (the [matroid study's Decision](../matroid-intersection-bounds-2026-10/README.md#decision)).

## Commits

* 1a9ce54: the record of this study and of the anatomy's, in the lab notebook of the day; the code is not kept.
