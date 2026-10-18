# Root-fitted nogood multipliers at the nodes (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

**Question.**  [Nogoods from the finder's work](nogoods-from-the-finders-work-2026-10.md) found that a Lagrangian bound
over the nogoods of a few seconds' search, with the GF(2) bound as its inner problem and multipliers fitted at the root,
closes about 0.38 of the root's GF(2) gap at 3016-3396.  The multipliers cost seconds to fit, so a search could fit them
once and use them unchanged at every node, at about the cost of a second GF(2) bound.  How much of the root's gain
survives at the nodes of a proof: what share of the nodes that today's bound leaves open would the root's multipliers
cut, and by how much do they lower the bound there?  [How far nogoods can go](how-far-can-nogoods-go-2026-10.md) found
the matching-based Lagrangian's root multipliers weak at the nodes ("fitted at the root, for the root"); this asks the
same of the GF(2) form, at the end of the chain.

**The bound.**  For multipliers `λ_N ≥ 0` over nogoods `N` of the root and a node with promotions `P`, every completion
`F` has `|F ∩ N| ≤ |N| − 1 − |P ∩ N|`, so the heaviest common independent set of at most `max_additional` of the node's
candidates in its GF(2) matroids, each candidate `x` weighing `x − Σ_{N ∋ x} λ_N` (those of no positive weight left
out), plus `Σ λ_N (|N| − 1 − |P ∩ N|)` bounds every completion (weak duality, Theorem 11, any multipliers).  Since also
`|F ∩ N| ≤ |N ∩ C|` for the node's candidates `C`, the *tight* constant `Σ λ_N (|N − P| − max(1, |N − P − C|))` holds
too: a nogood with a member gone from the candidates counts only its members still candidates.  The weights stay the
root's either way, so along a path only the matroids change, as for the GF(2) bound itself.

**What was run** (laptop, 2026-10-18, test-profile builds, at most four processes under `nice -n 10`, about 12 minutes
of CPU in all; data, commands and tables in [bench/nogood_bound/](../../bench/nogood_bound/README.md)).  The command
`wall relaxed` (`examples/wall/relaxed.rs`), for each game with a known optimum: the pool, the nogoods that the finder
and the sound search learn from the standalone start in 10 CPU s each (as `wall nogoods` harvests them: 263-6,008
distinct); the fit, 200 subgradient steps at the root towards the optimum plus one (`wall nogoods`'s fit, 0.03-0.66 s),
leaving 36-630 nogoods with a positive multiplier; then at each node today's bound (the GF(2) bound of a unit's node, as
the search computes it, `NodeBound`) and the root-λ bound with each constant.  The nodes: the probe's fixtures of 936
and 1215 (300 units each of the proof seeded with the optimum), fixtures made the same way for six games of 2700-3267
(`perf make-nodes`, 300 units each, not kept: the README has the command), the tuning suite's units of 3300, 3425 and
3440 (10 each, with the nodes of their subtrees), and 50 nodes a game on the way to the optimum (the optimum's
promotions less 30-70% of them), whose best completion is known to reach the optimum less what they promote: the
validity check.  A node is *open* when today's bound reaches what a completion needs to beat the optimum.

**1. At the open nodes the root's multipliers cut little** (the open nodes of the fixtures and the suite):

| n | gap | root share | open nodes | cut | cut, tight constant | median drop, tight | median excess |
|---|---|---|---|---|---|---|---|
| 936 | 132 | 0.27 | 137 | 0.04 | 0.07 | 3 | 38 |
| 1215 | 213 | 0.23 | 133 | 0.09 | 0.12 | 21 | 77 |
| 2700 | 731 | 0.44 | 300 | 0.01 | 0.03 | 0 | 134 |
| 2976 | 776 | 0.45 | 300 | 0.02 | 0.07 | 13 | 112 |
| 3129 | 924 | 0.36 | 300 | 0.00 | 0.01 | −11 | 188 |
| 3213 | 934 | 0.40 | 300 | 0.01 | 0.06 | 30 | 175 |
| 3260 | 943 | 0.46 | 300 | 0.01 | 0.06 | 22 | 165 |
| 3267 | 795 | 0.52 | 300 | 0.05 | 0.13 | 61 | 174 |
| 3300 | 1,443 | 0.33 | 10 | 0 | 0 | 121 | 860 |
| 3425 | 1,407 | 0.35 | 10 | 0 | 0 | 196 | 880 |
| 3440 | 1,555 | 0.32 | 10 | 0 | 0 | 285 | 863 |

(*Root share*: the share of the root's GF(2) gap the fit closes there; *drop*: today's bound less the root-λ bound;
*excess*: how far today's bound is above what the node needs, the drop that would cut it.)  Pooled over 2700-3440, the
root-λ bound cuts 2% of the 1,830 open nodes with the plain constant and 6% with the tight one; with the plain constant
it is above today's at most nodes (median drop −56), with the tight one below it at 60% of them (median drop 18 against a
median excess of 160).  At 936 and 1215 it cuts 7% and 10%.  The 30 suite units, whose subtrees hold 10.8 million of the
default's nodes, are cut by neither (the drop there is 85-285, the excess 860-880).

**2. The gain is at the top of the tree, where it is not enough, and gone below** (2700-3440, open nodes, tight
constant): at depth 0-9 the drop is a median 235 (excess 794), 10-19 141 (421), 20-29 88 (312), 30-39 62 (260), 40-49
53 (254), 50-59 34 (214), 60 and deeper −2 (112), where 1,092 of the 1,830 nodes are; the share cut is 0-8% at every
depth.  The nogoods with multipliers lose their pull as the path promotes or drops their members, as the matching-based
form's did.  The saving does not grow with the gap: the games of gap 924-943 are cut least (1-6%) beside 3267 (gap 795,
the best root share, 13%), and the suite's games of gap 1,400-1,555 not at all.

**3. Validity and cost.**  At every one of the 550 nodes on the way to the optimum both root-λ bounds were at least the
best completion's known lower end (by 19-544 at the closest), today's at least it too, and at every root the root-λ bound
was above the optimum (by 96-1,051); at each of the 2,980 nodes the matroids built afresh from the node's state gave
today's bound, as the command checks.  One root-λ evaluation from scratch (the node's matroids built, reweighted and
solved) costs a median 1.04-1.14 times a plain GF(2) bound from scratch, and 1.8-3.1 times today's bound at a unit
(contracted from the root's matroids): in a search it is a second GF(2) bound along the path.

**4. A pilot in the search, before the plan changed.**  Before this measurement the bound was built into the search as an
option (a second GF(2) level stack on the root's matroids relabelled by the weights, the tight constant, tried where the
GF(2) bound left a node open; cutting only, or cutting and adding the members it shows forced), and run on three proofs
from the optimum (finder's pool of 2-5 CPU s, 200 fit steps).  It agrees with the nodes above:

| proof | root share | nodes, today | cut only | cut and forced | CPU s, today | cut only | cut and forced |
|---|---|---|---|---|---|---|---|
| 936 | 0.27 | 390 | 394 | 413 | 0.026 | 0.036 | 0.051 |
| 1215 | 0.22 | 3,704 | 3,549 | 3,199 | 0.25 | 0.38 | 0.48 |
| 2700 | 0.42 | 112,464 | 106,190 | 115,992 | 14.9 | 29.5 | 40.3 |

At 2700 the cut-only search cut 1,261 of 73,751 checks: 6% fewer nodes for twice the CPU per node.  The code was not
kept (the owner asked for the node measurement instead).  One thing it showed for a later attempt: the GF(2) levels
(`src/gf2.rs`) read an element's weight as its number in four places, which a bound with other weights must change (a
child's inherited value, `parent.value − added.sum()`, and a certified child's, `value − added.sum()`; what the smaller
sets lose, `Losses::from_history`; and the lightest member in `Losses::reach`); `Gf2Graph::reweighted` also drops the
elements of no positive weight, which a path solver must keep to contract a promotion of one.  `wall nogoods` evaluates
only roots, which need none of this.

**Answer.**  No.  The root-fitted multipliers that close 0.22-0.52 of the root's GF(2) gap cut 2-6% of the nodes today's
bound leaves open at 2700-3440 (7-10% at 936 and 1215), none of the suite's units at the end of the chain, and lower the
bound at the nodes by a median 18 where 160 would be needed; the gain is at the shallow nodes and gone by depth 60, and
it does not grow with the gap.  In the search that is about 6% fewer nodes at twice the cost a node.  A Lagrangian over
nogoods would need multipliers fitted near each node, which [how far nogoods can go](how-far-can-nogoods-go-2026-10.md)
found too dear in every cheap form.

**Commits.**  The command `wall relaxed` and its data are in the commit that has this entry (`wall nogoods`'s fit now
returns its multipliers, and its harvest, fit and pool union are shared with `relaxed`).
