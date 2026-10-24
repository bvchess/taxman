# A bound over GF(2) and GF(3) signed frame matroids (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-15.  Concluded 2026-10-15.

**Question.**  Avenue A1 of the
[agenda](../../../docs/AGENDA.md#a1-a-bound-over-several-fields-and-signings-a-change-of-slope-if-its-share-holds) and
its conjecture C1: does the LP over the two GF(2) frame matroids and a family of GF(3) *signed* frame matroids (the
plain signing, random signings, and signings built from the cores the LP's own columns run into) close a share of the
root's GF(2) gap that holds with n?  **Answer.**  Yes at the games of the chain, less at the frontier.  With signings
made for the cores its own columns meet, the LP closes 0.62 of the root's GF(2) gap at n ≥ 1700 (E1, which passes its
kill criteria strongly), and it cuts 86% (1215) and 98% (936) of the nodes of the probe's fixtures that the GF(2) bound
leaves open when the node fits its own multipliers.  At 17 roots of 2200-3400 it closes 0.46 (E2, which is not killed
and does not pass strongly): the share falls with the root's gap, not with n.  Fixed signings close little, and the cost
of the multipliers is tens of seconds a root and 0.1 to 0.6 s a node, two to three orders over the GF(2) bound.

The study is the advisor's experiment E1 (with E0, its validation, E1-lite, a quick look, and E1n, the probe's nodes)
and then E2 (past 2175).  The solver is unchanged.  Theorems 14 and 15 of
[THEORY.md](../../../../docs/theory/frame-bounds.md#the-signed-frame-theorem) prove what the
bound rests on.

## Method

**The kill criteria**, fixed before any run, on the main arm (`f1cores`), counting only the roots whose bracket is
within 1% of their GF(2) gap: **K1**, the mean share over n ≥ 1700 below 0.25; **K2**, a slope of the share in n of −2
points per 100 n or worse with t ≤ −2; **K3**, fewer than half of the 1215 probe's nodes that GF(2) leaves open cut with
node-fitted multipliers.  The experiment is inconclusive if more than a fifth of the roots do not converge, and "passes
strongly" at a mean share of 0.5 or more for n ≥ 1700 and a slope above −1 point per 100 n.

**The bound.**  A *signing* σ of frame h over a field gives each incidence (mover, maximal factor) a nonzero value, each
mover the vector of its values, and the frame the linear matroid N_h^σ on the candidates of homes h and h + 1 with the
moves contracted.  Every promotable set is independent in every such matroid, over every field (Theorem 14: a winnable
frame's movers are triangular with a nonzero diagonal under any values), so for a family N_1, …, N_K and any multipliers
λ_k, `L(λ) = MI_cap(w − Σ λ_k) + Σ_k G_k(λ_k)` bounds every promotion set (Theorem 15), MI the GF(2) intersection of at
most `cap` elements and G_k the greedy maximum in N_k.  The bound reported is ⌊L⌋ evaluated in integers (weights x·2^16,
multipliers rounded to the grid), so its validity never rests on the floating-point LP that proposes λ.  Its least value
is the LP over the matroids' polytopes.  The arms are `gf2` (no extra matroid), `f1` (the plain signing, all values 1,
of frames 1, 2, 3), `f1r1` and `f1r3` (that and one or three random signings a frame), `cores` (core-derived signings
alone) and `f1cores` (the main arm: plain and core-derived).

**Core-derived signings.**  After the master's LP of a family is bracketed, every column of value above 10⁻⁶ is applied
to the root's frames and peeled; each frame left with a residual gives up to 4 minimal self-covering subsets (cores),
the next one found after un-applying the lightest of the previous core's *breaking* candidates.  A core is satisfied by
a signing whose values sum to 0 over GF(3) on each of its pool factors, which makes its set dependent.  It is merged
first fit into the frame's core-derived signings, or opens a new signing, at most 8 a frame.  The loop stops when a
round assigns no core, after 40 rounds, or at the budget.

**The multipliers.**  The design specified a Dantzig-Wolfe master with a column per GF(2) set and per independent set of
each N_k, tied by a *link row* per (k, x), solved by a dense revised simplex (commit 77c989f).  It passed the tests but
stalled: its pivots went in steps of the perturbation's size (at the root of 360, arm `cores`, 55,754 pivots for 274
pricings, 71% of them steps of order 10⁻⁷), and at 1000 in 60 s the master stood at 18,893 against L = 24,382.  The
master of the study describes each polytope P(N_k) from outside, by its rank inequalities `z(S) ≤ r_k(S)` (Edmonds),
added as the master's point `z` violates them; when no cheap one is found, an *exact check* finds the largest `y ≤ z` in
each polytope by a small column generation over its independent sets.  When every check passes, `ẑ = min(z, y_1, …)`
lies in every polytope, and its weight is a *certified* lower bound LB that, with the lowest L, brackets the LP.  It
converges at 1000 in 4-8 s.  Around it: 100 Polyak steps from λ = 0 towards P*, Wentges-stabilized pricing, exact checks
only once a separation round has found no new core, and a family's generation stopping when the lowest L is within
`max(0.25, 0.005 × gap)` of the master's value.  The simplex gained elastic rows (a violated inequality enters with an
artificial column at penalty 10⁵), partial Dantzig pricing, and a return to the empty set's basis when a refactorization
finds the basis singular (108 times in E1) or a solve exceeds 50,000 pivots (12 times).  A dual simplex for the violated
rows was tried and not kept: it looped, 10 million pivots in a 73-row LP within one 120 s root.

**Validity (E0).**  `cargo t --example fields` passes its 35 tests (the greedy against brute force on 3,000 random
matroids, the peel's residual on 10,650 frames, Lemmas 5-6 on 14,602 cores, the simplex on 3,000 random LPs).  Every
promotable set of every board with ≤ 22 candidates, and the optimal and 100 random maximal promotion sets of every
non-prime board of 137-400, are independent in every frame's matroid for the plain and five random signings (334 boards,
6.4 million pairs).  20 random multipliers at each of 309 non-prime roots of 2-400 never gave a bound below P*, and L at
zero multipliers is the GF(2) bound.  The exact LP against the column generation on 124 instances (65 with the LP below
the GF(2) bound) is bracketed within 10⁻⁵ relative.  `fields check` at every 25th board of 400-2175 (`check.jsonl`, 54
s) found 14,472 promotion sets independent in 4,320 signed matroids, 302,623 cores each making its set dependent, and
1,440 random multiplier vectors never below P*.  Every run asserted that the bound is at least P*, and at the probe's
nodes that it is at least the best completion in
[nodes_exact.jsonl](../matroid-intersection-bounds-2026-10/nodes_exact.jsonl).

## Results

**E1-lite** (`lite.jsonl`, the code of b8d8256, 11 minutes): the main arm at every 100th game of 900-2100; 8 of 13
converged, the 3 converged with n ≥ 1500 at a mean share of 0.64, so no early kill.  The stalled master solves and short
certificates it showed were fixed before E1 (1051560).

**E1: the main arm passes strongly.**  66 roots (every 25th game of 900-2175, the 9 others of the per-frame and slope
studies, and 1430, 1472, 1521, 2170, 2172), six arms, 120 s a root for the core arms and 30 s for the others
(`roots.jsonl`; 1.26 hours of one thread in all).  Share of the GF(2) gap closed, mean over the converged roots
(converged of roots):

| n | roots | f1 | f1r1 | f1r3 | cores | f1cores |
|---|---|---|---|---|---|---|
| 900-1099 | 11 | 0.05 (11/11) | 0.12 (11/11) | 0.19 (11/11) | 0.68 (11/11) | 0.68 (11/11) |
| 1100-1299 | 14 | 0.08 (14/14) | 0.10 (14/14) | 0.16 (14/14) | 0.61 (14/14) | 0.66 (14/14) |
| 1300-1499 | 10 | 0.02 (10/10) | 0.05 (10/10) | 0.12 (10/10) | 0.45 (6/10) | 0.47 (4/10) |
| 1500-1699 | 9 | 0.09 (9/9) | 0.13 (9/9) | 0.17 (9/9) | 0.51 (7/9) | 0.57 (7/9) |
| 1700-1899 | 8 | 0.02 (8/8) | 0.04 (8/8) | 0.10 (8/8) | 0.59 (8/8) | 0.59 (8/8) |
| 1900-2175 | 14 | 0.02 (14/14) | 0.05 (14/14) | 0.08 (14/14) | 0.64 (10/14) | 0.65 (10/14) |

On `f1cores`, 12 of 66 roots did not converge (18%, under the 20% that makes the run inconclusive: 1350, 1400, 1425,
1430, 1472, 1475, 1500, 1521, 2000, 2050, 2125, 2170, each stopped at 120 s with its bracket wider than 1% of its gap).
**K1**: the mean share over the 18 converged roots with n ≥ 1700 is 0.619 (killed below 0.25).  **K2**: the slope is
+1.00 ± 2.61 points per 100 n, t = +0.4 (killed at ≤ −2 with t ≤ −2).  So it **passes strongly**.  Over all 66 roots,
converged or not (the bound is valid either way, and an unconverged root's share is at most its LP's), the share is
0.620 for n ≥ 1700, slope +0.52 ± 2.30; the `cores` arm alone gives 0.618 and +0.78 ± 2.45 (10 unconverged).  The fixed
arms close little and less at large n: the plain signing 2% for n ≥ 1700, with three random signings a frame 9%, 0.16 of
the main arm's share (the design asked whether F1 + R3 reaches 80% of it: no).  So the search cannot do without tracking
cores: what sees the residual is the signings made for the cores it meets.

Against the per-frame study's 19 roots (200 subgradient steps, mean 0.59), the converged main arm closes more at 950,
992, 1000, 1150, 1184, 1225, 1250, 1254 and 1300, where that study's subgradient had stalled (its LP is at least as
tight as A1's by the design's Lemma 8, so there the stall was its method), and less at the other ten; its mean is 0.62.

*Signings and cores.*  Every root filled the cap of 8 core-derived signings on some frame (3.2 cores a signing, rising
from 2.7 at 900-999 to 5.4 at 2100-2175), and 59 of the 123 distinct cores a root found could not be placed: the family
is fragmented, which predicts that deep nodes meet cores the root's signings do not see.  The LP's support has 5-30
columns, and 95% of its mass is on sets that cannot be promoted.  The census (`census.jsonl`, 8,129 cores of the main
arm) finds 34 members on average, hardly ever square (0.3%) and never of three primes; the plain signing sees 0.7% of
them and the three random signings of `f1r3` 1.3%.  The main arm averages 36 s a root (1-120 s), most of it in the
master's simplex (10.6 s) and the exact checks, 0.3 s in 563 GF(2) intersections; the fixed arms converge in 0.01-1.4 s.

**E1n: the probe's nodes** (`nodes.jsonl`, 3.5 minutes): the main arm at the fixture's root (120 s; 936 closed at 20555,
1215 at 35168), then at each of the 300 nodes of each fixture either one evaluation at the root's final signings and
multipliers (`root`) or the node's own master and separation from them (`node`, 10 s a node; the root's signings fixed,
new ones the node's), each the lower of it and the node's GF(2) bound:

| n | GF(2) | `root` mode | `node` mode | all conflicts | per-frame, every 10th | ms a node (`root`, `node`) |
|---|---|---|---|---|---|---|
| 936 | 163 of 300 | 212 (49 of the 137 open) | 297 (134 of 137) | 278 | 25-29 of 30 | 2.8, 106 |
| 1215 | 167 of 300 | 209 (42 of 133) | 281 (114 of 133) | 249 | 23-27 of 30 | 5.2, 577 |

**K3** passes: 86% of the nodes GF(2) leaves open at 1215 are cut with node-fitted multipliers (98% at 936), more than
the LP over all conflicts cut.  The root's multipliers alone cut 0.37 of what the node's do, under the design's half.

## E2: past 2175 (2026-10)

**Question.**  Does the main arm's share hold on the games past 2175, where the chain stops and the root gaps are 440 to
1,440?  The kill criteria were fixed before any run, on `f1cores` certified at 600 s a root: **K1**, the mean share over
the roots from 2200 on that converge below 0.25; **K2**, a slope of the share against n over 900 to the last root of −2
points per 100 n or worse with t ≤ −2; and if more than 30% of the roots from 2200 on fail to converge in 600 s, the
mean over all of them decides.  The same runs decide the limit on core-derived signings a frame (16 against 8, adopted
at a gain of 0.05 of the gap or more at the dear roots) and whether *search mode* (a fit with no exact checks, 60 s)
reaches E1's certified bound within 5% of the gap at 80% of E1's 54 converged roots, and they write the root fits that
E3 starts from.

**Roots and instrument.**  The optima past 2175 are those of the EC2 chain, from a snapshot of its results to board 3423
(`--known` takes the snapshot after `results/strict.jsonl`; [the files section](#files) names it by its hash).  The
roots are every 100th game of 2200-3400 (13) and the *dear* games, the searched games past 2175 with the largest root
GF(2) gaps: 3300 (1443), 3333, 3335 and 3336 (1427 each, neighbors of one search with nearly one root) and 3176 (1388),
17 distinct roots.  Two games of proof `cap` have gaps as large (3303, 3339) and were not searched.  The instrument
(32cccb3) adds `--fit-mode search`, which skips the exact checks; `--save-fit DIR`, which writes the fit of each
`f1cores` run as `DIR/n{N}.json`; and an arm `f1cores16` with 16 core-derived signings a frame.  The fit is that of the
*lowest L the run evaluated*, not of the final family: merging cores into a signing changes its matroid, so the final
family's LP can be the higher (3335: lower bound 254732.7, bound 254722).

**The main arm certified, 600 s a root.**  Share of the GF(2) gap closed (`*`: not converged in 600 s, the share a lower
bound on the LP's), seconds, and the share with a limit of 16 signings a frame where run:

| n | gap | share | s | limit 16 | | n | gap | share | s | limit 16 |
|---|---|---|---|---|---|---|---|---|---|---|
| 2200 | 573 | 0.529 | 27 | 0.614 | | 3000 | 734 | 0.496 | 96 | 0.522 |
| 2300 | 721 | 0.320 | 40 | | | 3100 | 1181 | 0.633* | 600 | |
| 2400 | 740 | 0.428 | 152 | 0.469* | | 3176 | 1388 | 0.465* | 600 | 0.441* |
| 2500 | 1048 | 0.467 | 248 | | | 3200 | 1013 | 0.519 | 277 | 0.571* |
| 2600 | 439 | 0.392 | 32 | 0.419 | | 3300 | 1443 | 0.398* | 600 | 0.377* |
| 2700 | 731 | 0.539 | 43 | | | 3333 | 1427 | 0.391 | 396 | 0.404* |
| 2800 | 774 | 0.616 | 122 | 0.667 | | 3335 | 1427 | 0.371 | 345 | 0.379* |
| 2900 | 662 | 0.495 | 71 | | | 3336 | 1427 | 0.367* | 600 | 0.423* |
| | | | | | | 3400 | 1320 | 0.488* | 600 | 0.360* |

Twelve of the 17 roots converged (5 did not, 29.4%: under the 30% at which the rule changes).  **K1**: the mean share
over them is **0.464** (0.466 over all 17; killed below 0.25).  **K2**: over the converged roots of 900-3333, E1's 54
and E2's 12, the slope is **−1.00 ± 0.34 points per 100 n, t = −2.9** (killed at −2 or worse with t ≤ −2); over all 83
roots −0.84 ± 0.27, t = −3.1.  So E2 is **not killed**; it does not pass strongly either.  Past 2200 alone the share is
flat: by 200 n from 2200 the means are 0.43, 0.45, 0.47, 0.56, 0.53, 0.41, 0.49 (−0.02 ± 0.69 points per 100 n over the
12 converged roots), at a mean of 0.464, 0.155 ± 0.046 below E1's 0.619 at 1700-2175 (t = 3.4): the share fell as the
games went from E1's to E2's, and then stopped falling.

**The fall is the gap's** (after the fact, not one of the criteria).  Over E1's and E2's 83 roots of the main arm, the
share is 0.725 at root gaps of 0-199, 0.571 at 200-399, 0.544 at 400-599, 0.467 at 600-799, 0.540 at 1000-1249 (3 roots)
and 0.413 at 1250-1499 (6 roots), and a least-squares fit of the share on n and the gap gives +0.9 ± 0.6 points per 100
n and **−3.8 ± 1.1 points per 100 of gap**.  The chain's gaps grow with n (+37 per 100 n over 1301-3312), so the share
seems a function of the gap, not of n, and at the gaps of the dearest games, 1,300 to 1,450, it is 0.4 (0.37-0.40 at
3300 and 3333-3336).  E1's roots had gaps of 70-770; E2's are 440-1,443.

**The unconverged roots are near their LPs.**  At the five roots that did not converge, the lowest L is within 6.2-9.9
of the master's value, an uncertified estimate of the LP (1% of their gaps is 12-14).  What is missing is the
certificate, whose exact checks and separation take three quarters of the time (3300: 535 s of 600 outside the
intersections and the master's simplex), not a bound that is still falling, so their shares are probably within about
0.01 of the LPs'.

**The limit of 16.**  With 16 core-derived signings a frame the shares at the six roots of gap 1,013 or less are higher
by 0.086, 0.041, 0.027, 0.050, 0.026 and 0.051 of the gap (mean 0.047; 2200 converged, in 471 s against 27 s); at the
five dear roots the gains are −0.024, −0.021, +0.013, +0.008 and +0.056 (mean **0.006**; the design asks for 0.05), and
at 3400 −0.128.  At the large gaps the 16-signing fit does not finish in 600 s, because its rounds end in an exact check
of about twice as many matroids (49-63 against 28-36): at 3333, 3335, 3336 and 3400 it completes 5 or 6 rounds where the
main arm completes 12 to 14, and finds fewer cores (3400: 128 against 648).  By the rule **the limit stays 8**.  Whether
16 would gain at the dear roots with more time is not measured; its gain at the roots that converge, 0.03-0.09, suggests
it would, and that the limit of 8 binds (every root has 8 core-derived signings on at least one of frames 1-4, and 57%
of the cores found cannot be placed).

**Search mode does not serve; a short certified budget does.**  Search mode ends in under 4 s with a bound above E1's
certified one by **0.274 of the gap** on average (median 0.252, worst 0.640), within 5% of the gap at **0 of E1's 54
converged roots** (80% needed).  Its master has only the cheap inequalities, which describe a relaxation of the LP: the
exact checks find rank inequalities that those miss, so what they add is bound, not only certificate.  By the design's
rule the fit is then *certified mode with a longer budget*, and a budget alone does it: the main arm certified with
**60 s**, cut off wherever it is, is within 5% of the gap of E1's bound at **54 of 54** of E1's roots (mean difference
0.002 of the gap, worst 0.044, at 1950 and 1975, which need 88 and 114 s to converge) and, at the 17 roots past 2175,
above the 600 s run's by 0.038 of the gap on average (median 0.025), within 5% at 12 of the 17 (the other five, 2500,
3176, 3200, 3300 and 3400, are above it by 0.088, 0.058, 0.117, 0.051 and 0.151).  So a fit of 60 s costs under 0.05 of
the gap where the gap is under 800 and up to 0.15 at the frontier's gaps.  The runs of the 54 roots with the new code
reproduce E1's lines exactly where both converged within 60 s (48 of 48 roots).

**Fits and cost.**  The 17 fits are in [solver/frame/testdata/fits](../../../frame/testdata/fits/README.md) (14 MB),
where a test reads each back and checks its bound at its root.  The main arm took 81 minutes of one thread for the 17
roots, 27 to 600 s a root; the 12 roots of the limit of 16, 96 minutes.  Over the 17 roots the intersections are 1% of
the time, the master's simplex 24% and the exact checks and the separation 75%.  The cores come from more frames than
E1's (frame 4 at every root from 2700 on, frames 5 and 6 at four roots), and a root finds 150 to 663 distinct cores, 57%
of them unplaced (E1: 48%).

## Decision

**A1 is not killed at the frontier, and its strong pass at the chain's games stands.**  With signings made for the cores
its own columns meet, the LP closes about 60% of the root's GF(2) gap from 900 to 2175 with no trend in n (+1.0 ± 2.6
points per 100 n), and 0.46 of it at 17 roots of 2200-3400, flat past 2200 (−0.02 ± 0.69 points per 100 n) but 0.15
below E1's 0.62, a fall that goes with the gap (−3.8 ± 1.1 points per 100 of gap) and leaves 0.37-0.40 at the dearest
games.  The advisor's table S2 puts a share of 0.3 at +210 of affordable gap and 0.5 at +930 for a node 20 times as dear
as GF(2)'s; 0.4 lies between.  At the probe's nodes it cuts nearly every node GF(2) leaves open when the node fits its
own multipliers.

* **Kept:** the instrument, and the 17 root fits for E3.  E3 should start with form (c), the node's own master
  warm-started from its parent's columns, since the root's multipliers alone cut 0.37 of what the node's do.  Its root
  fit should be the certified arm with a budget that grows with the gap, not search mode: 60 s costs under 0.05 of the
  gap below a gap of 800 and up to 0.15 above it.
* **Kept as it is:** the limit of 8 core-derived signings a frame (16 gains 0.03-0.09 where it converges, nothing at the
  dear roots in 600 s), though the family stays fragmented.
* **Not kept:** the fixed signings, the design's link-row master, the dual simplex and search mode.
* **What would reopen it:** E3 and its pilot, [the A1 bound in a branch and
  bound](../the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md), and [R3 and the slope test
  (U4)](../r3-and-the-slope-test-u4-of-a1-2026-10/README.md).  Whether the limit of 16 gains at the dear roots with more
  time is not measured.

**Deviations from the design**, each resolved as stated and sound:

1. The master's link rows stalled; the master by rank inequalities with an exact certificate replaced it.  The LP, the
   bound and its validity are the design's; the certified lower bound is the weight of a point of the LP's polytope.
2. The closing tolerance is `max(0.25, 0.005 × gap)`, and the exact checks run only once a separation round finds no new
   core: the design's 0.25 at every family did not fit the budget past 1300.
3. A GF(3) loop of any extra matroid is left out of the intersection (Theorem 14 (3)); none occurred.
4. E1 has 66 roots, not 67 (1475 is a multiple of 25), and E2 17 distinct roots (3300 is both a hundredth game and a
   dear root); the verdict on the limit at "the dear roots" rests on three families, 3300, 3176 and 3333-3336.
5. K2 was applied as written, over the converged roots of 900-3333.  Its t of −2.9 with a slope of −1.0 is a significant
   fall that the criterion, set at −2, does not count; the gap fit is an after-the-fact reading.
6. The limit-of-16 fits (12 MB) were not kept; the budgets are wall-clock and the runs shared the laptop with other jobs
   (load 4 to 10), so the five unconverged roots might converge on an idle one.

## Files

This directory holds the data that `examples/fields.rs` wrote (its module comment and `common/dw.rs`'s define the bound
and every field), `fields.py`, which makes E1's tables from them (`./fields.py > fields_output.md`;
`./fields.py --roots lite.jsonl --lite` for the quick look), and `e2.py`, which makes E2's.  Everything ran on the M1
Max, one process at a time under `nice -n 10`, one thread, with other jobs on the machine; times are wall seconds of one
thread.  For the bound in a branch and bound (E3's pilot) see [its
entry](../the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md), and for R3 and the slope test (U4)
[theirs](../r3-and-the-slope-test-u4-of-a1-2026-10/README.md).

### Files and the commands that made them

`fields` is `target/release/examples/fields` (`nice -n 10 cargo build --release -j 4 --example fields`), run from
`rust/`; `K` is `--known results/strict.jsonl`.  The example `fields` and the files of `examples/common/` it included
(`dw`, `gf3`, `cores`, `lp`, `fit`, `bnb`, `frames`, `meet`; `fit.rs` specifies the format of `fits/`) were removed from
master on 2026-10-17; they build at commit 8753189 (`git checkout 8753189`, then the build command above), and
`git show 8753189:rust/examples/common/fit.rs` shows the specification.  Of them, `gf3.rs` holds GF(3) vectors and
bases, signings, the signed frame matroids and their greedy; `cores.rs` a frame's cores with a set applied, minimal
cores, column-sum signings and their merging; `dw.rs` the bound and its multipliers; `lp.rs` the master's simplex; and
`frames.rs` the GF(2) frame matroids and the game setup, moved out of `examples/matroid.rs`.

* `lite.jsonl`: `fields roots K --arms f1cores --seconds 120 --out lite.jsonl 900-2100/100`, the code of b8d8256;
  E1-lite, the main arm at 13 roots (before the master's stall restart and the exact checks' column limit).
* `roots.jsonl`, `census.jsonl`: `fields roots K --arms gf2,f1,f1r1,f1r3,cores,f1cores --seconds 120 --fixed-seconds 30
  --out roots.jsonl --census census.jsonl 900-2175/25 936 992 1088 1184 1196 1215 1240 1254 1288 1430 1472 1521 2170
  2172`, the code of 1051560; E1, six arms at 66 roots, and every core the main arm found.
* `nodes.jsonl`: `fields nodes K --modes root,node --seconds 10 --root-seconds 120 --exact
  solver/notebook/search/matroid-intersection-bounds-2026-10/nodes_exact.jsonl --out nodes.jsonl
  testdata/perf/n0936.nodes testdata/perf/n1215.nodes`, the code of 1e12c77; E1n, the probe's 600 nodes, in modes `root`
  and `node`.
* `check.jsonl`: `fields check K --samples 200 400-2175/25`; Theorems 14 and 15 and the column-sum signings at scale:
  per board, the promotion sets and signed matroids checked.
* `fields_output.md`, `lite_output.md`: `./fields.py > fields_output.md` and `./fields.py --roots lite.jsonl --lite >
  lite_output.md`; the tables.
* `roots_e2.jsonl`, `fits/`: `fields roots K S --arms gf2,f1cores --seconds 600 --save-fit
  solver/frame/testdata/fits --out roots_e2.jsonl 2200-3400/100 3333 3335 3336 3176`, then
  the same with `--arms f1cores16` at 2200-3200/200, 3300, 3333, 3335, 3336, 3176 and 3400, the code of 32cccb3, run in
  pieces of a few roots, each resuming where the last ended; E2, the main arm certified (600 s a root) at 17
  roots past 2175, the limit of 16 core-derived signings a frame at 12 of them, and the fit of each main-arm run.  The
  fits are in [solver/frame/testdata/fits](../../../frame/testdata/fits/README.md), where the tests read them.
* `search_e1roots.jsonl`: `fields roots K --arms f1cores --seconds 60 --fit-mode search --out search_e1roots.jsonl 900
  925 936 ... 2175` (the 54 games of `roots.jsonl` whose `f1cores` line has `converged` true), the code of 32cccb3; E2,
  search mode against E1's certified bounds.
* `budget60_e1roots.jsonl`, `budget60_e2roots.jsonl`: `fields roots K S --arms f1cores --seconds 60 --out
  budget60_e1roots.jsonl` at the same 54 roots, and the same with `budget60_e2roots.jsonl 2200-3400/100 3333 3335 3336
  3176`, the code of 32cccb3; E2, the main arm certified with a budget of 60 s, against E1's and the 600 s runs.
* `e2_output.md`: `./e2.py > e2_output.md`; E2's tables.

`S` is `--known` of a snapshot of the EC2 chain's results to board 3423 (`chain_snapshot2.jsonl`, 3,423 games, sha256
`a24709934602002d79c9caf29f4b39e3c3aad17c9650115fcb8d4ccd18e008a9`; the chain's games past 2175 were not yet in
`results/` then), after `K`; the optimum of each game used is `opt` of its line.  The dear roots are the searched games
(`method` search, proof `exhausted`) past 2175 with the largest root GF(2) gaps in that snapshot.

## Commits

* 77c989f (2026-10-15): the driver `examples/fields.rs` and the bound `examples/common/dw.rs`, with the design's
  link-row master.
* b8d8256 (2026-10-15): the master by rank inequalities in place of link rows; the code of E1-lite.
* 1051560 (2026-10-15): a stalled master solve starts again from the empty set, a smaller artificial penalty, exact
  checks capped at 300 columns; the code of E1.
* 1e12c77 (2026-10-15): `fields nodes`: the root's signings stay fixed at a node; the code of E1n.
* 32cccb3 (2026-10-15): search fit mode, saved root fits, a cap per arm, the intersection checked past rank 128; the
  code of E2.
* 8753189 (2026-10-17): the last commit that builds the example `fields`.
