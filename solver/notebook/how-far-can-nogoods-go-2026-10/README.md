# How far can nogoods go (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The Lagrangian bound over the nogoods (tactic `lagrange`) fits its multipliers at the root, to the
nogoods each searcher has learned, and uses them at every node.  The [slope study](what-drives-the-slope-2026-10.md)'s
two pilots found that the root's best-fitted bound given to every searcher saved little, and that a refit from zero
multipliers ends 50-540 above the best bound the same nogoods allow.  (1) What is the best bound the nogoods a
searcher holds could give, at the root and at its nodes, and how far is the search's own bound from it?  (2) What does
the bound become with more conflicts: every minimal conflict of a few members, every conflict of the node?  (3) Does
the gap left by the best possible use of all conflicts still grow with n?  If it does, that bounds what this direction
can do.  (4) Is something cheap worth most of it?  And once the
[matroid study](matroid-intersection-bounds-2026-10.md) made the GF(2) bound the baseline: (5) what share of its
residual gap do conflicts close, at the root and at nodes, by n, and at what cost?

*What was kept.*  Nothing but this entry; the solver is unchanged.  The study's code lived on a branch that was never
pushed and has been deleted, as the project does with research branches once they are harvested (the pilots of
[IDEAS.md](../IDEAS.md#6-the-tools-and-what-became-of-the-pilot-code) went the same way).  It was a library module that
solves the LP below (Dantzig-Wolfe column generation with a dense revised simplex, and exact separation of conflicts),
an example program that ran the measurements, a patch that instrumented the search, and a script that made the tables
from the data; the data went with it.  So the method is written out here to be redone: which LP, with which inner
problems, how it is solved, what was measured where, on which games and fixtures, and how it was checked.  What it
read is on master: the games' optima (`results/strict.jsonl`, `bench/curve/bands.jsonl`), the probe's fixtures
(`testdata/perf/n0936.nodes`, `n1215.nodes`), and the slope and matroid studies' data (`bench/curve/slope_roots.jsonl`
and `slope_proofs.jsonl`, `bench/matroid/roots.jsonl` and `bnb.jsonl`), against which its tables were made.

*The tool: the cover LP.*  A conflict `K` of a node (a set of its candidates that cannot all be promoted) gives every
completion `F` the *cover inequality* `|F ∩ K| ≤ |K| − 1`.  The cover LP of a family of conflicts is `max Σ x·y_x` over
the polytope of an *inner relaxation* (`y` has a number in [0, 1] for each candidate `x`) with those inequalities.  The
inner relaxation's sets are the candidate sets that a matching can promote (the matching bound's relaxation), or the
common independent sets of the two GF(2) frame matroids of the matroid study.  Both polytopes are integral (bipartite
matching; Edmonds' matroid intersection), so the LP is the Lagrangian dual of the Lagrangian bound over the same
conflicts with that inner problem (Geoffrion): the lowest that bound can be, whatever its multipliers, and the LP's
optimal duals are the best multipliers.  The bound itself, for multipliers `λ_K ≥ 0`, is the inner problem's heaviest
set with each candidate `x` weighed `x − Σ_{K ∋ x} λ_K`, plus the constant `Σ λ_K (|K| − 1)`; with the matching inner
it is the bound of the `lagrange` tactic.

*Solving it.*  The *master* is the LP over convex combinations `μ` of the inner problem's sets: a column for each set
`Q` (its sum, and how many members of each conflict it holds), a row `Σ_j μ_j |Q_j ∩ K| ≤ |K| − 1` for each conflict,
and `Σ μ = 1`; it starts from the empty set as its one column, which satisfies every row.  It is solved by a dense
revised simplex with an explicit basis inverse (refactorized every 64 pivots, tolerances 1e-9, the right-hand sides
perturbed by tiny amounts against degeneracy): a new column enters by the primal simplex, a new row with its slack
basic and is repaired by the dual simplex.  A column is priced by the inner problem's heaviest set at multipliers that
are the master's duals smoothed towards the best multipliers found so far (Wentges, weight 0.5; if no column of
positive reduced cost comes out, the plain duals; if none still, the master is optimal).  The matching inner problem
was the solver's own weighted matching solved from scratch, the intersection the matroid study's.  Every pricing is a
Lagrangian bound, an upper bound on the LP whatever its multipliers (weak duality), computed in fixed point with
multipliers rounded to 2⁻²⁰, so exact for the multipliers it used; the master's value is a lower bound on the LP over
the rows it has.  The solve stops when they are within 1e-3 (or after 50,000 pricings).

*Separation, exact.*  The rows to add are the cover inequalities of *all* conflicts of the node that the master's
solution `y = Σ μ_j 1_{Q_j}` violates.  A conflict `K` is violated when its *deficit* `Σ_{x ∈ K} (1 − y_x)` is below 1,
so it lies in `I ∪ F`, where `I` is the set of the candidates with `y_x = 1` and `F` a set of fractional ones whose
deficits sum below 1.  A minimal conflict fails in one frame `i` (a set that cannot be promoted has a frame that is
not winnable with it), and all its members are promoted into or out of frame `i` (one that is not could be left out,
and frame `i` would still fail), so `F` lies among the fractional candidates of homes `i` and `i + 1`.  A subset of a
set that can be promoted can be promoted, so it is enough to check the *maximal* such sets `I ∪ F`, frame by frame
(enumerated by branching on the fractional candidates in ascending order of deficit, pruned when the room left cannot
be filled): if every one can be promoted, no cover is violated, and each that cannot holds a violated conflict, which
the model's conflict analysis finds inside it (its deficit is at most that of `F`; `Model::conflict_using`).  If `I`
alone cannot be promoted, the conflicts are the one found and, for each of its members, the one found without it.
Solving and separating alternate, at most 400 rounds of at most 200,000 feasibility checks, until separation adds
nothing.  The LP over *all* conflicts of the node (`ideal`) is then between the master's value and the lowest
Lagrangian bound found, and is *certified* (exact) when the master was optimal and the last separation complete within
its budget.  Every LP reported below is, except where said.

*Relax-and-cut*, the cheap relative: from some conflicts and multipliers, at most `s` subgradient steps on the
Lagrangian bound (Polyak's rule towards the target, a node's `need`: step `damping × (bound − need + 1) / ‖g‖²` with
`g_K = |Q ∩ K| − (|K| − 1)` for the inner problem's heaviest set `Q`, the damping halved after 3 steps without a lower
bound, multipliers kept at least 0), each step first adding up to `p` conflicts of `Q` with multiplier 0 (the conflict
of `Q`, then of `Q` without that conflict's largest member, and so on), stopping as soon as the bound is below `need`.

*Validity.*  The LP over all conflicts, over the matching, was tested on the matching bound's walk harness (random walks
of the search from the root, 2 walks of 25 loop iterations for each game 40-200 with a golden file in `testdata/golden`,
seeded by n) at 5,495 nodes: separation certified; the LP at least the best completion of the node (exhaustive) and at
most the matching bound; the columns of one node's LP carried to the next node of the walk (a matching of a node is a
matching of the next if every promotion the next has and the first does not is promoted in it, every promotion the first
has and the next does not is a candidate of the next, and it promotes no candidate the next does not have) are matchings
there, and starting from them gives the same LP; and every forced member read off the potentials (13,642 of them) is
promoted by every best completion.  Over the GF(2) intersection, a branch and bound with the incumbent one below the
optimum (which must find a set worth `P*`) finds it in all 709 non-prime games 30-879 (largest tree 1,222 nodes), and
the 146 proofs of section 6 met no promotable set above `P*`.

*Method.*  No solver code changed.  Games are set up with the default (strict) tactics, the optimum promotion sum `P*`
read as the slope study reads it (from `results/strict.jsonl`, and `bench/curve/bands.jsonl` past 1300), and everything
is one thread a process and at most two processes, on the laptop shared with four other agents: times are indicative.  A
node's *excess* under a bound is the bound's integer part less `P* − sum(promoted)`, the most a completion can be worth
without beating the incumbent; the node is cut when that is at most 0 (its `need` is `P* + 1 − sum(promoted)`).  The `±`
of a trend is the standard error of its least-squares slope.  The measurements:

* *Roots*: for each of the 370 non-prime games of 880-1310 and 1350-1352 with a known optimum (371 over GF(2), which
  has 1351 too), the matching bound, the inner problem's own bound, and the LP over all conflicts of the root, with the
  matching or the GF(2) intersection inside; the number and sizes of the covers with a positive dual at the end, the
  columns, pricings and simplex pivots.  The slope study's root Lagrangian bound (8,000 damped steps over conflicts of
  its own Lagrangian matchings) and the matroid study's GF(2) bound are those of `slope_roots.jsonl` and
  `roots.jsonl`.
* *Small conflicts*, at the roots of 880, 1000, 1215 and 1300: every minimal conflict of one, two and three members,
  by the same frame argument as the separation (all of a minimal conflict's members are candidates of home `i` or
  `i + 1` for the frame `i` it fails in): a pair is a conflict if promoting both fails, a triple a minimal one if no
  pair of it is a conflict and promoting all three fails; then the LP, by solving only (no separation), over the
  conflicts of at most one, two and three members, against the LP over all.
* *Nodes*: the 300 nodes of each of the probe's fixtures `n0936.nodes` and `n1215.nodes` (units that the solver's
  proof handed to its searcher; the incumbent is the optimum): the LP over all conflicts of each node, with either
  inner problem, and relax-and-cut from no conflicts and zero multipliers, 3, 10 and 30 steps of 4 conflicts each.
* *Traces of the search*, from an instrumented build of the solver of 4372b36 (before the tactics cleanup, with
  `revise_freedoms` off: the trees the default build makes now) running a one-thread proof seeded with the optimum.
  The instrumentation wrote, to one file: each nogood the table learns (its members in hexadecimal, one digit for four
  numbers, lowest first) and each time the table is cleared; at each refit of the Lagrangian relaxation, its nogoods
  and their integer multipliers (64ths); and, at every K-th Lagrangian check (K = 100 at 936, 2,000 at 1215), the
  node's promotions and candidates, its `need` and the value of the search's bound.  Replaying the file rebuilds the
  table and the relaxation at each sampled check, and measures there: the LP over the relaxation's nogoods that are
  live at the node (the nogood less the promotions, nonempty and among the candidates), the LP over those and the
  table's, relax-and-cut from the search's multipliers (5, 20 and 50 steps, adding 0 or 4 conflicts a step; the table's
  nogoods start at 0), and the LP over all conflicts; and at each refit, the bound its multipliers give at the root
  against the LP over the nogoods it fitted (the relaxation's and the table's).
* *Extra checks in the search*, the same build with a bound added where the search's matching check and Lagrangian bound
  leave a node or child open: if the extra bound is below `need` the node is cut; if it is not, and forced members are
  asked for, the members of the heaviest set at its best multipliers without which its bound (read from the inner
  problem's potentials, as the Lagrangian bound reads its forced members) falls below `need` join the node's forced
  members.  The bound was one of: the LP over all conflicts of the node (`ideal`), stopped at the first Lagrangian bound
  below `need`, starting from the nogoods the searcher holds that are live there, from every conflict found at earlier
  nodes (kept in a pool, as nogoods of the root, and added where live) and from the columns of the last LP carried over;
  the LP over the relaxation's live nogoods (`relax`) or over those and the table's (`known`), solved without
  separation; relax-and-cut from the search's multipliers over the relaxation's and the table's nogoods, 5 or 20 steps,
  with no conflicts added (`rf`) or 4 a step (`rc`); *per-subtree multipliers* (`anchor`): at each node whose promotions
  number a multiple of 4, 8 or 16 and which has no anchor of its own, the LP over the nogoods live there (and those of
  the enclosing anchor), after which the covers with a positive dual are made nogoods of the root and, with their duals
  as multipliers, give the bound of every node below it, by one pricing; and the LP over all conflicts used only at
  nodes of at most 8, 16 or 24 promotions.  (A solve stopped at the first bound below `need` is not certified; any
  Lagrangian bound cuts soundly.)  The searches are the slope study's proofs (one thread; the incumbent a value `P* + δ`
  with no set behind it, so δ = 0 is the proof of the optimum), and the "search's" steps those of `slope_proofs.jsonl`
  (the traced proofs of 936 and 1215 took its 41,653 and 1,111,451 steps).
* *Branch and bound with the LP at every node*: a node is cut when the LP over all conflicts of the node has a
  Lagrangian bound below `need` (checked first with no conflicts); an integral LP solution is a promotable set, the
  node's best completion; the children inherit the conflicts that still hold in them.  Branching, with the matching
  inside, on the most fractional candidate of the LP's solution (promoted first, then left out); with GF(2) inside, as
  the matroid study's branch and bound branches, on a conflict `k₁ < … < k_r` of the heaviest set at the best
  multipliers ("`k₁ … k_{j−1}` promoted, `k_j` not", for each `j`).  The incumbent is `P*`, or `P* − 1` for the
  soundness check.

**1. At the root, all conflicts are worth what the root's own conflicts are.**  The LP over every conflict of the
root, over the matching (every LP exact; the GF(2) gap is the [matroid study's](matroid-intersection-bounds-2026-10.md),
the Lagrangian gap the [slope study's](what-drives-the-slope-2026-10.md)):

| n | games | matching gap | Lagrangian gap (slope study) | all conflicts | GF(2) gap | ms |
|---|---|---|---|---|---|---|
| 851-900 | 18 | 432 | 112 | 110 | 88 | 30 |
| 901-950 | 43 | 619 | 185 | 180 | 147 | 79 |
| 951-1000 | 43 | 749 | 261 | 256 | 172 | 168 |
| 1001-1050 | 42 | 795 | 362 | 358 | 156 | 203 |
| 1051-1100 | 42 | 546 | 206 | 199 | 81 | 160 |
| 1101-1150 | 45 | 522 | 162 | 162 | 107 | 112 |
| 1151-1200 | 43 | 629 | 154 | 150 | 108 | 182 |
| 1201-1250 | 42 | 1091 | 426 | 419 | 148 | 348 |
| 1251-1300 | 43 | 1277 | 471 | 459 | 185 | 424 |
| 1301-1310 | 7 | 1445 | 554 | 547 | 154 | 685 |
| 1350-1352 | 2 | 1238 | 452 | 430 | 267 | 490 |

Over 880-1300 its gap is 263.9 on average against the slope study's 269.0 (8,000 damped steps over conflicts of its
own Lagrangian matchings), 0.2 to 11.6 below it band by band, and it grows 54.7 ± 5.1 per 100 in n against 56.0 ± 5.2:
those conflicts were already all the root has, and that fit had converged.  It closes 67% of the matching gap, a share
that does not grow (−0.8 ± 0.4 points per 100 in n).  So over the matching the best a Lagrangian over nogoods can do at
the root leaves a gap growing with n as fast as the Lagrangian bound's.  About 30 covers bind at the optimum, a third
of them pairs and 36% of eight members or more; at 880 the root's LP solution is half-integral and its binding covers
span frames 0-2.

**2. Small conflicts are not enough.**  Every minimal conflict of one, two and three members of the root (none of one
member, 245-413 pairs, 892-1,723 triples, 2-11 s to enumerate), against all conflicts (gaps):

| n | matching | ≤ 2 members | ≤ 3 | all | GF(2) | ≤ 3 members | all |
|---|---|---|---|---|---|---|---|
| 880 | 487 | 371 | 371 | 167 | 56 | 56 | 1 |
| 1000 | 911 | 576 | 576 | 392 | 214 | 214 | 130 |
| 1215 | 966 | 739 | 702 | 366 | 213 | 213 | 165 |
| 1300 | 1421 | 1043 | 973 | 520 | 151 | 151 | 130 |

Over the matching they close 24-37% of the gap, against 57-66% for all conflicts, and the triples add almost nothing
to the pairs; over GF(2) they add nothing at all (the intersection already implies them).  The conflicts that bind
are large, and generating them proactively by size cannot reach them.

**3. The search's bound: near the best its nogoods allow at the root, far from it at the nodes.**  Traces of the
one-thread proofs of 936 (41,653 steps, every 100th Lagrangian check) and 1215 (1,111,451 steps, every 2,000th),
replayed.

* *At the root*: at each refit, the bound its multipliers give against the LP over the nogoods it fitted.  At 1215
  (23 refits, 88-329 nogoods each) the 50 warm-started steps end 7.8 above that LP on average, 14.3 at most, and the
  final relaxation's root bound (35,445) is 29 above the LP over all conflicts (35,416; `P*` is 35,050).  The one refit
  of 936 (from zero, after 2,000 steps) ends 39 above.  The slope study's "50-540 above" was a refit from zero
  multipliers; the search refits from its last ones and converges over its refits (the slope
  study did not measure that).
* *At the nodes*, the checks the search leaves open (242 of 251 sampled at 936, 345 of 370 at 1215): the mean excess
  of each bound, and the share of those checks it would cut.

| | the search's | its relaxation, refitted at the node | all the nogoods it holds | all conflicts | 20 refit steps | 50 | 20 with conflicts added | 50 |
|---|---|---|---|---|---|---|---|---|
| 936: excess | 54 | 47 | −22 | −139 | 13 | 9 | 7 | 4 |
| 936: cut | 0 | 5% | 74% | 97% | 38% | 54% | 62% | 79% |
| 1215: excess | 73 | 38 | −18 | −191 | 24 | 18 | 12 | 6 |
| 1215: cut | 0 | 24% | 67% | 95% | 30% | 41% | 46% | 66% |

  (The refits start from the search's multipliers, the table's nogoods at 0; "conflicts added": 4 conflicts of each
  step's Lagrangian matching.)  Refitting the relaxation's own nogoods at the node gains little: a typical node has 1
  of 936's 16 live and 9 of 1215's 52-65, the others' members promoted or dropped on the way down.  The nogoods the
  searcher holds in its table, 37 and 49 of them live at a typical node, would cut two thirds to three quarters of
  the checks it leaves open with multipliers fitted there, and all conflicts of the node 95-97%.  The weakness is not
  the nogoods but the multipliers: fitted at the root, for the root.

**4. All conflicts at every node: a constant factor, not a slope.**  The instrumented build, cutting with the LP over
all conflicts of the node wherever the search's own bounds leave a node or child open (one thread, seeded with the
optimum):

| n | the search's steps | with the extra cut | ratio | ms a check |
|---|---|---|---|---|
| 880 | 25,398 | 1,741 | 14.6 | 9 |
| 900 | 4,194 | 397 | 10.6 | 12 |
| 936 | 41,653 | 1,327 | 31.4 | 9 |
| 950 | 460,813 | 14,658 | 31.4 | 29 |
| 1000 | 366,846 | over 29,080 (stopped at 590 s) | under 12.6 | 22 |
| 1050 | 46,527 | 4,207 | 11.1 | 25 |
| 1088 | 6,205 | 454 | 13.7 | 31 |
| 1100 | 29,614 | 6,157 | 4.8 | 6 |
| 1150 | 4,858 | 550 | 8.8 | 20 |
| 1184 | 52,259 | 6,469 (4,485 with its forced members) | 8.1 (11.7) | 5 |
| 1196 | 216,076 | over 15,675 (stopped at 590 s) | under 13.8 | 39 |

log10 of the steps with the cut against the search's: slope 0.81 ± 0.14 over the nine finished games, and the two
stopped ones, among the largest trees, lie above that line.  Margin scans (incumbent `P* + δ`, the cut and its forced
members): at 1215 the ratio is 1,614 at δ = 400, 209 at 300, 131 at 200, 49 at 150 and 24 at 100; at 936 604, 94, 64
and 30 at δ = 200, 100, 50 and 0.  The advantage shrinks as the remaining gap grows: the LP over all conflicts is no
tighter than the Lagrangian bound at the top of the tree (section 1) and far tighter deep down, so it removes the deep
nodes of a tree whose top it leaves as it was.  A plain branch and bound with the LP at every node (on the most
fractional candidate) needs 10-63 times fewer nodes than the solver's steps at 880, 900, 936, 1000, 1088, 1100 and
1184 (2,439, 127, 1,503, 15,593, 99, 2,531 and 2,135 nodes), with a log-log slope of 1.15 ± 0.17 against them.  The
two log-log slopes, 0.81 and 1.15, lie on either side of 1 and within 1.4 standard errors of it: about ten times fewer
nodes, no visible change of the tree's growth, and a check costs 5-40 ms, 1,000-5,000 of the search's steps.

**5. Nothing cheap gets enough of it.**  The same build with cheaper extra checks, their forced members joining the
node's (steps; the cost of a check):

| extra check at the open checks | 880 | 936 | 1184 | ms a check |
|---|---|---|---|---|
| none (the search) | 25,398 | 41,653 | 52,259 | |
| the LP over the nogoods the searcher holds | 8,856 | 10,004 | 12,471 | 1.3-4.0 |
| 5 refit steps from its multipliers, 4 conflicts added a step | 11,267 | 17,185 | 18,970 | 0.5-1.2 |
| 20 such steps | 4,383 | 5,929 | 9,707 | 1.6-3.6 |
| 20 refit steps, no conflicts added | | 15,741 | | 1.0 |
| the LP over all conflicts at nodes of at most 8, 16 or 24 promotions only | | 39,858; 39,579; 27,507 | | 33-40 |
| per-subtree multipliers: the LP over the nogoods held at every node of 4, 8 or 16k promotions, used below it | | 13,291; 17,841; 26,432 | | 0.2-0.5, and an LP fit every 8-34 steps |

Apart from the per-subtree fits (25-70 steps' cost a check, and a fit every few steps) and the LP at shallow nodes
(4,000-5,000 steps a check, for at most 1.5 times fewer), each costs per check what 80 to 550 of the search's steps
cost, for 2.3-7 times fewer steps, and all lose time: by factors of 11 (the per-subtree multipliers at 936) to 100 (the
table's LP at 1184).  At 936 the search takes 0.34 s and the cheap forms 3.7-13 s; at 880 0.16 s against 4-8 s, at 1184
0.38 s against 15-38 s.  Raising multipliers lowers the weights of their members, which the incremental levels cannot
repair without the Hungarian searches that make a solve, so the refits could not be made much cheaper; keeping the
expensive check to the top of the tree saves nothing, since the gain is at the bottom; and subtrees are short chains
(four nodes in five end after their first child), so per-subtree fits come every few steps.

**6. With the GF(2) intersection inside.**  The GF(2) bound alone beats the best nogood bound over the matching at the
root (section 1's last columns: its gap is 28-82% of the latter's band by band, and grows 4.6 per 100 in n against
54.7), so nogoods over the matching are superseded.  With the heaviest common independent set of the two GF(2)
frame matroids as the cover LP's inner problem:

* *At the root* (all 371 roots of the matroid study, every LP exact, 10-158 ms):

| n | games | GF(2) gap | with all conflicts | share closed |
|---|---|---|---|---|
| 851-900 | 18 | 88 | 26 | 0.73 |
| 901-950 | 43 | 147 | 75 | 0.49 |
| 951-1000 | 43 | 172 | 106 | 0.40 |
| 1001-1050 | 42 | 156 | 107 | 0.32 |
| 1051-1100 | 42 | 81 | 36 | 0.57 |
| 1101-1150 | 45 | 107 | 55 | 0.53 |
| 1151-1200 | 43 | 108 | 44 | 0.65 |
| 1201-1250 | 42 | 148 | 105 | 0.29 |
| 1251-1300 | 43 | 185 | 128 | 0.31 |
| 1301-1310 | 7 | 154 | 138 | 0.10 |
| 1350-1352 | 3 | 272 | 192 | 0.29 |

  Over 880-1300 all conflicts close 46% of the GF(2) gap on average (79 left of 136), a share that falls 3.8 ± 0.8
  points per 100 in n; what is left grows 8.4 ± 2.0 per 100 in n, the GF(2) gap 4.6 ± 2.4.  (The matroid study's
  subgradient over the conflicts of each heaviest set closed 7-36% at five roots; the LP is the best any multipliers
  can do, and its gap at 1300, 129.5, is that study's 129 to within the rounding of its integer bounds.)  About 10
  covers bind at a root, 96% of them of eight members or more.
* *At the probe's nodes*: GF(2) alone cuts 163 and 167 of the 300 nodes of 936 and 1215 (the matroid study's
  figures); with all conflicts 278 and 249, that is 84% and 62% of the nodes it leaves open, at 12 and 46 ms a node.
  Relax-and-cut with the intersection inside, from no conflicts, 4 conflicts of each heaviest set added a step, cuts
  45%, 55% and 63% of them at 936 with 3, 10 and 30 intersections, and 19%, 39% and 42% at 1215.  (Over the
  matching, for comparison: all conflicts cut 84% and 71% of the nodes, relax-and-cut 20-55% and 12-45%.)
* *In a branch and bound* (the matroid study's skeleton and branching, with the LP over all conflicts in place of the
  bare intersection), on the 146 games the slope study proves, against the matroid study's `bnb.jsonl`:

| n | games | log10 nodes, solver | GF(2) B&B | GF(2) and all conflicts | ratio | s, GF(2) B&B | s, with all conflicts |
|---|---|---|---|---|---|---|---|
| 851-900 | 6 | 3.78 | 2.59 | 1.47 | 13.1 | 1.0 | 0.9 |
| 901-950 | 16 | 4.56 | 3.48 | 2.50 | 9.6 | 22.8 | 22.8 |
| 951-1000 | 16 | 5.21 | 3.74 | 2.82 | 8.3 | 120.1 | 136.8 |
| 1001-1050 | 19 | 5.62 | 3.99 | 3.36 | 4.2 | 160.1 | 241.0 |
| 1051-1100 | 18 | 4.46 | 2.77 | 2.09 | 4.8 | 8.0 | 8.9 |
| 1101-1150 | 17 | 3.82 | 2.72 | 2.08 | 4.3 | 6.4 | 9.4 |
| 1151-1200 | 16 | 4.52 | 2.90 | 2.10 | 6.4 | 32.8 | 28.1 |
| 1201-1250 | 19 | 6.13 | 3.90 | 3.04 | 7.3 | 168.6 | 210.2 |
| 1251-1300 | 16 | 6.90 | 4.11 | 3.43 | 4.8 | 240.5 | 334.6 |
| 1309 | 1 | 7.14 | 4.24 | 3.34 | 8.1 | 16.3 | 14.7 |
| 1350, 1352 | 2 | 6.88 | 4.79 | 3.74 | 11.1 | 128.3 | 117.2 |

  All conflicts make the trees 6.2 times smaller (geometric mean; 231,592 nodes against 1,327,966 in all), less as n
  grows (the ratio's logarithm −0.046 ± 0.016 per 100 in n), and the node count grows 0.18 ± 0.05 decades per 100 in n
  against GF(2)'s 0.14 ± 0.05.  It follows the root's GF(2) gap at 0.0112 ± 0.0006 decades per unit, the matroid
  study's 0.0111 for GF(2) alone: the conflicts move the line down, not its slope.  From the root alone, 0.0111 times
  the gap they close there predicts 0.62 decades on average; the trees shrink by 0.79.  In time it is no gain: 1,125 s
  against 905 s for the 146 proofs (545 against 409 s over 1201-1300), a quarter to a third slower, a node costing
  2.4-11 ms against the bare intersection's 0.3-1.

*Conclusion.*  A Lagrangian bound over nogoods can be no better than the cover LP over all conflicts, and that is a
constant factor, not a slope.  Over the matching: at the root it is the slope study's Lagrangian bound (5 below it on
average), whose gap grows 55 per 100 in n; the search's multipliers are near the best its nogoods allow at the root and
far from it at the nodes, where the nogoods it already holds would cut two thirds of the checks it leaves open; but the
best use of every conflict at every node makes the proofs about ten times smaller, with no visible change in how they
grow, at a thousand or more steps' cost a check, and every cheap form tried loses time by a factor of 11 to 100.  Over
the GF(2) intersection, which supersedes it: all conflicts close 46% of the root's gap, a share falling with n, cut
62-84% of the probe's nodes it leaves open, and make its branch and bound about six times smaller (less as n grows)
in a quarter more time.  So the answer to (3) is yes: over the matching the ideal leaves a gap that grows as the
Lagrangian bound's does, and over GF(2) the residual still grows (8.4 ± 2.0 per 100 in n, against GF(2)'s own 4.6 ±
2.4; the two do not differ by more than their errors).  In both, nogoods give a constant factor.

*What it implies.*

* *Nothing of this to implement now; the direction is superseded.*  The slope's lever is the GF(2) bound, going into
  the search.  The existing `lagrange` tactic (the matching, root-fitted multipliers) should be measured again once
  GF(2) bounds every node: GF(2) is tighter at the root than the best a nogood bound over the matching could ever be,
  and what the Lagrangian adds at the nodes (the matroid study found a node-fitted one cutting different nodes from
  GF(2)) is at most this study's constant factor, at its cost.
* *Later, a constant factor over GF(2)*: a Lagrangian over nogoods with the intersection as the inner problem is worth
  at most about six times fewer nodes at the frontier, less as n grows, and only if a node costs little more than one
  intersection (multipliers carried from the parent and a step or two at a node, on an incremental intersection that
  takes falling weights).  Relax-and-cut with three intersections a node already cuts 19-45% of the probe's nodes that
  GF(2) leaves open.
* *What conflicts miss* is what GF(2) misses, the large odd cores; their cover inequalities are weak (`|F ∩ K| ≤ 14`
  for 15 members).  Stronger families over the same sets (rank inequalities over a frame's candidates, frame-local
  hulls) were not tried; the register has no row for them.
