# How far can nogoods go (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  The Lagrangian bound over the nogoods (tactic `lagrange`) fits its multipliers at the root, to the
nogoods each searcher has learned, and uses them at every node.  The [slope
study](../what-drives-the-slope-2026-10/README.md)'s pilots found that the root's best-fitted bound given to every
searcher saved little, and that a refit from zero multipliers ends 50-540 above the best bound the same nogoods allow.
(1) What is the best bound the nogoods a searcher holds could give, at the root and at its nodes, and how far is the
search's own bound from it?  (2) What does the bound become with more conflicts: every minimal conflict of a few
members, every conflict of the node?  (3) Does the gap left by the best possible use of all conflicts still grow with n?
(4) Is something cheap worth most of it?  And once the [matroid study](../matroid-intersection-bounds-2026-10/README.md)
made the GF(2) bound the baseline: (5) what share of its residual gap do conflicts close, at the root and at nodes, by
n, and at what cost?

**Answer.**  A Lagrangian bound over nogoods can be no better than the cover LP over all conflicts, and that is a
constant factor, not a slope.  Over the matching bound the LP closes 67% of the root gap and leaves a gap that grows
with n as the Lagrangian bound's does; used at every node it makes proofs 5 to 31 times smaller in steps (a log-log
slope of 0.81 ± 0.14) at 5-40 ms a check, and every cheap form loses time by a factor of 11 to 100.  Over the GF(2)
intersection it closes 46% of the root gap, a share falling with n, and makes a branch and bound 6.2 times smaller in
nodes but 24% slower.  Nothing of it was kept.

## Method

The study's code lived on a branch that was never pushed and has been deleted: a library module solving the LP below,
an example program that ran the measurements, a patch that instrumented the search, a script that made the tables, and
their data.  The method is written out here to be redone.  What it read is on master: the games' optima
(`results/strict.jsonl`, `solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/bands.jsonl`), the probe's
fixtures (`testdata/perf/n0936.nodes`, `n1215.nodes`), and the slope and matroid studies' data
(`solver/notebook/search/what-drives-the-slope-2026-10/slope_roots.jsonl` and `slope_proofs.jsonl`,
`solver/notebook/search/matroid-intersection-bounds-2026-10/roots.jsonl` and `bnb.jsonl`).

**The cover LP.**  A conflict `K` of a node gives every completion `F` the *cover inequality* `|F ∩ K| ≤ |K| − 1`.  The
cover LP of a family of conflicts is `max Σ x·y_x` over the polytope of an *inner relaxation* (`y` has a number in
[0, 1] for each candidate `x`) with those inequalities.  The inner relaxation's sets are the candidate sets that a
matching can promote, or the common independent sets of the two GF(2) frame matroids.  Both polytopes are integral, so
the LP is the Lagrangian dual of the Lagrangian bound over the same conflicts with that inner problem (Geoffrion): the
lowest that bound can be, whatever its multipliers, and the LP's optimal duals are the best multipliers.

**Solving it.**  Dantzig-Wolfe column generation with a dense revised simplex: a column for each set of the inner
problem, a row for each conflict, each column priced by the inner problem's heaviest set at the master's duals smoothed
towards the best found so far (Wentges, weight 0.5).  Every pricing is a Lagrangian bound, an upper bound on the LP, and
the master's value is a lower bound; the solve stops when they are within 1e-3.  *Separation* adds the cover
inequalities of all conflicts of the node that the master's solution violates.  A minimal conflict fails in one frame
`i`, with all its members promoted into or out of it, so a violated one lies among the candidates with `y_x = 1` and
the fractional candidates of homes `i` and `i + 1`; checking the maximal such sets frame by frame, with the model's
conflict analysis (`Model::conflict_using`) finding a conflict inside each that cannot be promoted, is exact.  The LP
over all conflicts (`ideal`) is *certified* when the master was optimal and the last separation complete.
*Relax-and-cut* is the cheap relative: at most `s` Polyak subgradient steps on the Lagrangian bound towards a node's
`need`, each first adding up to `p` conflicts of the inner problem's heaviest set.

**Validity.**  Over the matching, the LP over all conflicts was tested at 5,495 nodes of the matching bound's walk
harness (separation certified; the LP between the best completion and the matching bound; every forced member read off
the potentials, 13,642 of them, promoted by every best completion).  Over the GF(2) intersection, a branch and bound
with the incumbent one below the optimum finds a set worth P\* in all 709 non-prime games 30-879, and the 146 proofs of
result 6 met no promotable set above P\*.

**The measurements,** with the default (strict) tactics, one thread a process and at most two processes, on the laptop
shared with four other agents (times are indicative).  A node's *excess* under a bound is the bound's integer part less
P\* − sum(promoted); the node is cut when that is at most 0.

* *Roots:* the 370 non-prime games of 880-1310 and 1350-1352 with a known optimum (371 over GF(2)), with either inner
  problem; and, at the roots of 880, 1000, 1215 and 1300, the LP over every minimal conflict of one, two and three
  members.
* *Nodes:* the 300 nodes of each of the probe's fixtures of 936 and 1215 (the incumbent is the optimum): the LP over all
  conflicts, and relax-and-cut from no conflicts (3, 10 and 30 steps of 4 conflicts each).
* *Traces of the search,* from an instrumented build of the solver of 4372b36 (`revise_freedoms` off) running one-thread
  proofs seeded with the optimum of 936 (41,653 steps) and 1215 (1,111,451).  It wrote each nogood learned, each
  refit's nogoods and multipliers, and, at every K-th Lagrangian check (K = 100 at 936, 2,000 at 1215), the node's
  state and the search's bound; replaying measured at each sampled check the LP over the relaxation's live nogoods, over
  those and the table's, relax-and-cut from the search's multipliers, and the LP over all conflicts.
* *Extra checks in the search,* the same build with a bound added where the search's own checks leave a node open: if
  it is below `need` the node is cut, and the members it shows forced join the node's.  The bounds were the LP over all
  conflicts, stopped at the first bound below `need`; the LP over the live nogoods the searcher holds; relax-and-cut
  from the search's multipliers (5 or 20 steps, adding 0 or 4 conflicts a step); per-subtree multipliers (the LP at
  every node whose promotions number a multiple of 4, 8 or 16, its covers made nogoods of the root and used below); and
  the LP over all conflicts only at nodes of at most 8, 16 or 24 promotions.  The searches are the slope study's proofs
  (an incumbent `P* + δ` with no set behind it).
* *Branch and bound with the LP at every node:* a node is cut when the LP's bound is below `need`, branching on the most
  fractional candidate over the matching and, over GF(2), on a conflict of the heaviest set as the matroid study does.

## Results

**1. At the root, all conflicts are worth what the root's own conflicts are.**  The LP over every conflict of the root,
over the matching, leaves a gap of 263.9 on average over 880-1300 against the slope study's 269.0 for its Lagrangian
bound (8,000 damped steps over conflicts of its own Lagrangian matchings), 0.2 to 11.6 below it band by band.  It grows
54.7 ± 5.1 per 100 in n against 56.0 ± 5.2.  The root's conflicts were already all it has: the LP closes 67% of the
matching gap, a share that does not grow (−0.8 ± 0.4 points per 100 in n), so over the matching the best a Lagrangian
over nogoods can do leaves a gap growing with n as fast as the Lagrangian bound's.  By band the matching gap is 432 at
851-900 and 1277 at 1251-1300, the Lagrangian 112 and 471, all conflicts 110 and 459, and the GF(2) gap 88 and 185.
About 30 covers bind at the optimum, a third of them pairs and 36% of eight members or more.

**2. Small conflicts are not enough.**  A root has no minimal conflict of one member, 245-413 pairs and 892-1,723
triples.  Over the matching the conflicts of at most 2 members close 24-37% of the gap, against 57-66% for all
conflicts, and the triples add almost nothing to the pairs (at 880 the gaps are 487 for the matching, 371 for up to 2
and for up to 3 members, and 167 for all; at 1215, 966, 739, 702 and 366).  Over GF(2) they add nothing, since the
intersection already implies them.  The conflicts that bind are large, and generating them proactively by size cannot
reach them.

**3. The search's bound: near the best its nogoods allow at the root, far from it at the nodes.**  At the root of 1215
the final relaxation's bound (35,445) is 29 above the LP over all conflicts (35,416; P\* is 35,050), and at each of its
23 refits (88-329 nogoods each) the 50 warm-started steps end 7.8 above the LP over the nogoods fitted, 14.3 at most.
The one refit of 936 (from zero) ends 39 above.  The slope study's "50-540 above" was a refit from zero; the search
refits from its last multipliers.  At the nodes, of the checks the search leaves open (242 of 251 sampled at 936, 345 of
370 at 1215), each bound's mean excess and the share it would cut:

| | the search's | its relaxation, refitted at the node | all the nogoods it holds | all conflicts | 20 refit steps | 50 | 20 with conflicts added | 50 |
|---|---|---|---|---|---|---|---|---|
| 936: excess | 54 | 47 | −22 | −139 | 13 | 9 | 7 | 4 |
| 936: cut | 0 | 5% | 74% | 97% | 38% | 54% | 62% | 79% |
| 1215: excess | 73 | 38 | −18 | −191 | 24 | 18 | 12 | 6 |
| 1215: cut | 0 | 24% | 67% | 95% | 30% | 41% | 46% | 66% |

(The refits start from the search's multipliers, the table's nogoods at 0; "conflicts added" is 4 conflicts of each
step.)  Refitting the relaxation's own nogoods at the node gains little: a typical node has 1 of 936's 16 live and 9 of
1215's 52-65.  The nogoods in the searcher's table, 37 and 49 of them live at a typical node, would cut two thirds to
three quarters of the open checks with multipliers fitted there, and all conflicts of the node 95-97%.  The weakness is
not the nogoods but the multipliers: fitted at the root, for the root.

**4. All conflicts at every node: a constant factor, not a slope.**  Cutting with the LP over all conflicts wherever the
search's own bounds leave a node open (seeded with the optimum), the steps fall by 14.6 at 880, 10.6 at 900, 31.4 at 936
and 950, 11.1 at 1050, 13.7 at 1088, 4.8 at 1100, 8.8 at 1150 and 8.1 at 1184 (11.7 with its forced members), at 5-40
ms a check; 1000 and 1196, stopped at 590 s, lie above the line.  The log-log slope against the search's steps is
0.81 ± 0.14 over the nine finished games.  Margin scans (incumbent `P* + δ`) show the advantage shrinking as the
remaining gap grows: at 1215 the ratio is 1,614 at δ = 400 and 24 at 100, at 936 604 at δ = 200 and 30 at 0.  The LP
over all conflicts is no tighter than the Lagrangian bound at the top of the tree and far tighter deep down, so it
removes the deep nodes of a tree whose top it leaves as it was.  A plain branch and bound with the LP at every node
needs 10-63 times fewer nodes than the solver's steps at 880, 900, 936, 1000, 1088, 1100 and 1184, with a log-log slope
of 1.15 ± 0.17.  The two slopes lie within 1.4 standard errors of 1: about ten times fewer nodes, no visible change of
the tree's growth, and a check costing 1,000-5,000 of the search's steps.

**5. Nothing cheap gets enough of it.**  The same build with cheaper extra checks, their forced members joining the
node's, in steps (the search takes 25,398 at 880, 41,653 at 936 and 52,259 at 1184):

| extra check at the open checks | 880 | 936 | 1184 | ms a check |
|---|---|---|---|---|
| the LP over the nogoods the searcher holds | 8,856 | 10,004 | 12,471 | 1.3-4.0 |
| 5 refit steps from its multipliers, 4 conflicts added a step | 11,267 | 17,185 | 18,970 | 0.5-1.2 |
| 20 such steps | 4,383 | 5,929 | 9,707 | 1.6-3.6 |
| 20 refit steps, no conflicts added | | 15,741 | | 1.0 |
| the LP over all conflicts at nodes of at most 8, 16 or 24 promotions only | | 39,858; 39,579; 27,507 | | 33-40 |
| per-subtree multipliers at every node of 4, 8 or 16k promotions | | 13,291; 17,841; 26,432 | | 0.2-0.5 |

Apart from the per-subtree fits (25-70 steps' cost a check, and a fit every 8-34 steps) and the LP at shallow nodes
(4,000-5,000 steps a check, for at most 1.5 times fewer, and 5% at 16 promotions), each costs per check what 80 to 550
of the search's steps cost, for 2.3-7 times fewer steps, and all lose time by factors of 11 (the per-subtree multipliers
at 936) to 100 (the table's LP at 1184): at 936 the search takes 0.34 s and the cheap forms 3.7-13 s, at 1184 0.38 s
against 15-38 s.  Raising multipliers lowers the weights of their members, which the incremental levels cannot repair
without the Hungarian searches that make a solve; keeping the expensive check to the top of the tree saves nothing,
since the gain is at the bottom; and subtrees are short chains (four nodes in five end after their first child), so
per-subtree fits come every few steps.

**6. With the GF(2) intersection inside.**  The GF(2) bound alone beats the best nogood bound over the matching at the
root (its gap is 28-82% of the latter's band by band, and grows 4.6 per 100 in n against 54.7), so nogoods over the
matching are superseded.  With the GF(2) intersection as the cover LP's inner problem:

* *At the roots* (all 371, every LP exact, 10-158 ms), all conflicts close 46% of the GF(2) gap on average (79 left of
  136), from 0.73 at 851-900 to 0.10 at 1301-1310, a share that falls 3.8 ± 0.8 points per 100 in n; what is left
  grows 8.4 ± 2.0 per 100 in n, the GF(2) gap 4.6 ± 2.4.  The matroid study's subgradient over the conflicts of each
  heaviest set closed 7-36% at five roots.  About 10 covers bind at a root, 96% of them of eight members or more.
* *At the probe's nodes,* GF(2) alone cuts 163 and 167 of the 300 nodes of 936 and 1215; with all conflicts 278 and 249,
  that is 84% and 62% of the nodes it leaves open, at 12 and 46 ms a node.  Relax-and-cut from no conflicts cuts 45%,
  55% and 63% of them at 936 with 3, 10 and 30 intersections, and 19%, 39% and 42% at 1215 (over the matching: all
  conflicts 84% and 71%, relax-and-cut 20-55% and 12-45%).
* *In a branch and bound* (the matroid study's skeleton and branching, with the LP over all conflicts in place of the
  bare intersection), on the 146 games the slope study proves, the trees are 6.2 times smaller (geometric mean; 231,592
  nodes against 1,327,966 in all), by 13.1 at 851-900, 4.2 at 1001-1050 and 4.8 at 1251-1300, less as n grows (the
  ratio's logarithm −0.046 ± 0.016 per 100 in n).  The node count follows the root's GF(2) gap at 0.0112 ± 0.0006
  decades per unit, the matroid study's 0.0111 for GF(2) alone: the conflicts move the line down, not its slope.  From
  the root alone, 0.0111 times the gap they close there predicts 0.62 decades on average; the trees shrink by 0.79.  In
  time it is no gain: 1,125 s against 905 s for the 146 proofs (545 against 409 s over 1201-1300), a node costing
  2.4-11 ms against the bare intersection's 0.3-1.

## Decision

Nothing of this is implemented, and the direction is superseded.  To question (3): over the matching the ideal leaves a
gap that grows as the Lagrangian bound's does, and over GF(2) the residual still grows (8.4 ± 2.0 per 100 in n, against
GF(2)'s own 4.6 ± 2.4; the two do not differ by more than their errors).  In both, nogoods give a constant factor.

* The slope's lever is the GF(2) bound, going into the search.  The existing `lagrange` tactic (the matching,
  root-fitted multipliers) should be measured again once GF(2) bounds every node: GF(2) is tighter at the root than the
  best a nogood bound over the matching could ever be, and what the Lagrangian adds at the nodes (the matroid study
  found a node-fitted one cutting different nodes from GF(2)) is at most this study's constant factor, at its cost.
* A Lagrangian over nogoods with the intersection as the inner problem is worth at most about six times fewer nodes at
  the frontier, less as n grows, and only if a node costs little more than one intersection (multipliers carried from
  the parent and a step or two at a node, on an incremental intersection that takes falling weights).  Relax-and-cut
  with three intersections a node already cuts 19-45% of the probe's nodes that GF(2) leaves open.
* What conflicts miss is what GF(2) misses, the large odd cores; their cover inequalities are weak (`|F ∩ K| ≤ 14` for
  15 members).  Stronger families over the same sets (rank inequalities over a frame's candidates, frame-local hulls)
  were not tried.

## Commits

* `4372b36` (2026-10-13): the slope study's merge, whose solver the instrumented build was made from.
* `691d5d8` (2026-10-13): this entry; the study's code is not kept.
