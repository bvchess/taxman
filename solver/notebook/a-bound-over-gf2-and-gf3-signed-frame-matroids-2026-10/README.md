# A bound over GF(2) and GF(3) signed frame matroids (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  Avenue A1 of the
[agenda](../../docs/AGENDA.md#a1-a-bound-over-several-fields-and-signings-a-change-of-slope-if-its-share-holds) and its conjecture
C1: does the LP over the two GF(2) frame matroids and a family of GF(3) *signed* frame matroids (the plain signing,
random signings, and signings built from the cores the LP's own columns run into) close a share of the root's GF(2) gap
that holds with n?  This is the advisor's experiment E1 (with E0, its validation, E1-lite, a quick look, and E1n, the
probe's nodes), whose kill criteria were fixed before any run: on the main arm (`f1cores`), counting only the roots
whose bracket is within 1% of their GF(2) gap, **K1** the mean share over n ≥ 1700 below 0.25, **K2** a slope of the
share in n of −2 points per 100 n or worse with t ≤ −2, **K3** fewer than half of the 1215 probe's nodes that GF(2)
leaves open cut with node-fitted multipliers; inconclusive if more than a fifth of the roots do not converge; "passes
strongly" at a mean share of 0.5 or more for n ≥ 1700 and a slope above −1 point per 100 n.

*What was kept.*  The study's code, as an instrument (as the matroid study's was): `examples/fields.rs` (commands
`roots`, `nodes`, `check`, and the tests), `examples/common/gf3.rs` (GF(3) vectors and bases, signings, the signed frame
matroids and their greedy), `common/cores.rs` (a frame's cores with a set applied, minimal cores, column-sum signings
and their merging), `common/dw.rs` (the bound and its multipliers), `common/lp.rs` (the master's simplex), and
`common/frames.rs` (the GF(2) frame matroids and the game setup, moved out of `examples/matroid.rs`, which prints the
same lines as before); the data and the tables in [solver/notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md](README.md).  Theorems 14 and 15 of
[THEORY.md](../../../docs/THEORY.md#signed-frame-matroids-the-frame-bound-over-other-fields) prove what the bound
rests on.  No solver change.

*The bound.*  A *signing* σ of frame h over a field gives each incidence (mover, maximal factor) a nonzero value, each
mover the vector of its values, and the frame the linear matroid N_h^σ on the candidates of homes h and h + 1 with the
moves contracted (a candidate of home h + 1 its vector, one of home h its unit vector).  Every promotable set is
independent in every such matroid, over every field (Theorem 14: a winnable frame's movers are triangular with a nonzero
diagonal under any values), so for a family N_1, …, N_K and any multipliers λ_k, `L(λ) = MI_cap(w − Σ λ_k) + Σ_k
G_k(λ_k)` bounds every promotion set (Theorem 15), MI the GF(2) intersection of at most `cap` elements and G_k the
greedy maximum in N_k.  The bound reported is ⌊L⌋ evaluated in integers (weights x·2^16, multipliers rounded to the
grid; the crate's `gf2::meet::Meet`, checked against `common/meet.rs`'s `intersect` at every 50th evaluation), so its
validity never rests on the floating-point LP that proposes λ.  Its least value is the LP over the matroids' polytopes.
Arms: `gf2` (no extra matroid), `f1` (the plain signing, all values 1, of frames 1, 2, 3), `f1r1`, `f1r3` (that and one
or three random signings a frame, a value per incidence), `cores` (core-derived signings alone), `f1cores` (the main
arm: plain and core-derived).

*Core-derived signings.*  After the master's LP of a family is bracketed, every column of value above 10⁻⁶ is applied to
the root's frames and peeled; each frame left with a residual gives up to 4 minimal self-covering subsets (cores), the
next one found after un-applying the lightest of the previous core's *breaking* candidates (its promoted members, or a
promoted-out factor that only one member lists).  A core is satisfied by a signing whose values sum to 0 over GF(3) on
each of its pool factors, which makes its set dependent (THEORY.md, after Theorem 14); it is merged first fit into the
frame's core-derived signings (per factor, the values of every assigned core listing it: kept if they satisfy all the
sums, else the column-sum construction where no other core lists the factor, else a search over at most 2^12
assignments), or opens a new signing (seeded random values elsewhere), at most 8 a frame.  The loop stops when a round
assigns no core, after 40 rounds, or at the budget.

*The multipliers, and what the design's master became.*  The design specified a Dantzig-Wolfe master with a column per
GF(2) set and per independent set of each N_k, tied by a *link row* per (k, x) (`Σ α [x ∈ Q] − Σ β_k [x ∈ I] ≤ ε`),
solved by a dense revised simplex.  It was built (commit 77c989f) and passed the tests of the time, but its simplex
spent its pivots in steps of the perturbation's size: at the root of 360 (arm `cores`) 55,754 pivots for 274 pricings,
71% of them steps of order ε = 10⁻⁷ and 4% zero; at 1000 in 60 s 232,000 pivots for 150 pricings, the master at 18,893
against L = 24,382 (larger ε: 10⁻⁵ 292,000 pivots, 10⁻³ 16,000 but a looser master; ε = 0 a singular basis).  The master
now describes each P(N_k) from outside, by its rank inequalities `z(S) ≤ r_k(S)` (Edmonds), added as the master's point
`z = Σ α_j 1_{Q_j}` violates them: in each matroid the most violated level set of z and each dependent support set,
closed within z's support (a closed set is at least as violated), and when none is found, an *exact check*: for each
matroid the largest `y ≤ z` in its polytope, by a small column generation over its independent sets (a packing LP whose
value is `min_S r(S) + z(T − S)`; less than z(T) exactly when z is outside, and then a level set of its final duals is
violated).  When every check passes, `ẑ = min(z, y_1, …)` lies in every polytope (all are down-closed): its weight is a
*certified* lower bound LB, and with the lowest L it brackets the LP.  An inequality's dual μ_{k,S} gives `λ_{k,x} =
Σ_{S ∋ x} μ_{k,S}` (and `G_k(λ_k) ≤ Σ μ r(S)`), so the pricing is the same L.  Before the master, 100 Polyak steps from
λ = 0 towards P*; then Wentges-stabilized pricing at ½ λ_best + ½ λ_master, or at λ_master when that prices nothing out;
the exact checks only once a separation round has found no new core (intermediate families stop when no column prices
out and no cheap inequality is violated); a family's generation stops when the lowest L is within `max(0.25, 0.005 ×
gap)` of the master's value (half the convergence criterion).  The simplex (`lp.rs`, a partitioned basis inverse: only
the block of the rows whose slack is not basic is kept, dense) gained partial Dantzig pricing, *elastic* rows (a
violated inequality enters with an artificial column at penalty 10⁵ in place of its slack, so the basis stays feasible),
a return to the empty set's basis when a refactorization finds the basis singular (108 times in E1) or a solve exceeds
50,000 pivots (12 times), purging of nonbasic columns beyond 300, and right-hand sides perturbed by up to 10⁻⁶ (10⁻⁸
left rounding ties to decide pivots); the exact checks drop the coloops of the matroid restricted to z's support, start
from the support's independent parts and a pool, and stop at 300 columns (then repeat from the pool while the
certificate is short).  A dual simplex for the violated rows was tried (kept in no commit): it looped (10 million
pivots in a 73-row LP), and the elastic rows replaced it.

*Validity (E0).*  `cargo t --example fields`, 35 tests, all pass: GF(3) arithmetic and bases against scalar and dense
elimination, the greedy against brute force on 3,000 random matroids, `det3` against integer determinants; the peel's
residual against `winnable_using` on 10,650 frames; Lemmas 5-6 on 2,000 unpromotable sets of boards 60-400 (14,602
cores, each one's column-sum signing makes its set dependent) and merging (11,217 cores into first-fit signings, every
assigned core's sums still 0); the study's **test 3**, every promotable set of every board with ≤ 22 candidates and the
optimal and 100 random maximal promotion sets of every non-prime board of 137-400 independent in every frame's matroid
for the plain and five random signings (334 boards, 6.4 million pairs); **test 6**, 20 random multipliers at each of 309
non-prime roots of 2-400 never below P* and L at zero multipliers the GF(2) bound; **test 7**, the exact LP (every
common independent set and every independent set of each extra matroid a column) against the column generation on 124
instances (boards with 4-16 candidates, and boards of 300-700 restricted to ≤ 12 candidates around the GF(2) set's
cores, 65 of them with the LP below the GF(2) bound): the certified bound and the lowest L bracket it, within 10⁻⁵
relative; **test 8**, the reweighted intersection equal to the reference's at 60 evaluations on 20 boards of 300-2175;
the simplex against an enumeration reference on 3,000 random LPs and in stages, kills, degenerate and large LPs.
`fields check` at every 25th board of 400-2175 (`check.jsonl`, 54 s): 14,472 promotion sets (200 random maximal ones and
the optimal one a board) independent in 4,320 signed matroids (all frames, plain, five random and 20 column-sum signings
a board), 302,623 cores of 14,400 random unpromotable sets each making its set dependent, 1,440 random multiplier
vectors never below P*.  Every run asserted that the bound is at least P* (330 extra-matroid runs at 66 roots and the 13
of E1-lite), that the optimal promotions are independent in every final matroid, and at the probe's nodes that the bound
is at least the best completion recorded in `solver/notebook/matroid-intersection-bounds-2026-10/nodes_exact.jsonl`.

*E1-lite* (`lite.jsonl`, the code of b8d8256, 11 minutes): the main arm at every 100th game of 900-2100; 8 of 13
converged (not 1400, 1500, 1700, 1800, 2000), the 3 converged with n ≥ 1500 at a mean share of 0.64: no early kill.  The
stalled master solves and short certificates it showed were fixed before E1 (1051560).

**E1: the main arm passes strongly.**  66 roots (every 25th game of 900-2175, the 9 others of the per-frame and slope
studies, and 1430, 1472, 1521, 2170, 2172: the design's 67 count 1475 twice), six arms, 120 s a root for the core arms
and 30 s for the others (`roots.jsonl`; 1.26 hours of one thread in all).  Share of the GF(2) gap closed, mean over the
converged roots (converged of roots):

| n | roots | f1 | f1r1 | f1r3 | cores | f1cores |
|---|---|---|---|---|---|---|
| 900-1099 | 11 | 0.05 (11/11) | 0.12 (11/11) | 0.19 (11/11) | 0.68 (11/11) | 0.68 (11/11) |
| 1100-1299 | 14 | 0.08 (14/14) | 0.10 (14/14) | 0.16 (14/14) | 0.61 (14/14) | 0.66 (14/14) |
| 1300-1499 | 10 | 0.02 (10/10) | 0.05 (10/10) | 0.12 (10/10) | 0.45 (6/10) | 0.47 (4/10) |
| 1500-1699 | 9 | 0.09 (9/9) | 0.13 (9/9) | 0.17 (9/9) | 0.51 (7/9) | 0.57 (7/9) |
| 1700-1899 | 8 | 0.02 (8/8) | 0.04 (8/8) | 0.10 (8/8) | 0.59 (8/8) | 0.59 (8/8) |
| 1900-2175 | 14 | 0.02 (14/14) | 0.05 (14/14) | 0.08 (14/14) | 0.64 (10/14) | 0.65 (10/14) |

The kill criteria on `f1cores`: 12 of 66 roots unconverged (18%, under the 20% that makes the run inconclusive; 1350,
1400, 1425, 1430, 1472, 1475, 1500, 1521, 2000, 2050, 2125, 2170, each stopped at 120 s with its bracket wider than 1%
of its gap); **K1**: mean share 0.619 over the 18 converged roots with n ≥ 1700 (killed below 0.25); **K2**: slope +1.00
± 2.61 points per 100 n, t = +0.4 (killed at ≤ −2 with t ≤ −2); so **passes strongly**.  Over all 66 roots, converged or
not (the bound is valid either way, and an unconverged root's share is at most its LP's): 0.620 for n ≥ 1700, slope
+0.52 ± 2.30.  The `cores` arm alone: 0.618 and +0.78 ± 2.45 (10 unconverged).  The fixed arms close little and less at
large n: the plain signing 2% for n ≥ 1700, with three random signings a frame 9%, 0.16 of the main arm's share (the
design asked whether F1 + R3 reaches 80% of it: no).  So the search cannot do without tracking cores: what sees the
residual is the signings made for the cores it meets.

Against the per-frame study's 19 roots (200 subgradient steps, mean 0.59): the converged main arm closes more at 950,
992, 1000, 1150, 1184, 1225, 1250, 1254 and 1300 (where that study's subgradient had stalled: its LP is at least as
tight as A1's by the design's Lemma 8, so there the stall was its method), less at the other ten; mean 0.62.

*Signings, the support, the cores.*  Every root filled the cap of 8 core-derived signings on some frame (6.4 a frame
used, 3.2 cores a signing, rising from 2.7 at 900-999 to 5.4 at 2100-2175), and 59 of the 123 distinct cores a root
found could not be placed: the family is fragmented, which predicts that deep nodes meet cores the root's signings do
not see.  The LP's support has 5-30 columns (13 on average), and 95% of its mass is on sets that cannot be promoted: far
from integral where it binds.  No GF(3) loop occurred.  The census (`census.jsonl`, 8,129 cores of the main arm): 34
members on average (25 at 900-999, 40 at 2100-2175), hardly ever square (0.3%) and never of three primes, so C2's
hole-free three-prime case is not where these cores live; the plain signing sees 0.7% of them and the three random
signings of `f1r3` 1.3%.

*Cost.*  The main arm averages 36 s a root (1-120 s): 0.3 s in 563 GF(2) intersections (0.15 ms each at 900, 0.96 at
2175), 10.6 s in the master's simplex, most of the rest in the exact checks (1.25 million pivots a root on average). The
fixed arms converge in 0.01-1.4 s.

**E1n: the probe's nodes** (`nodes.jsonl`, 3.5 minutes): the main arm at the fixture's root (120 s; 936 closed at 20555,
1215 at 35168), then at each of the 300 nodes of each fixture either one evaluation at the root's final signings and
multipliers (`root`) or the node's own master and separation from them (`node`, 10 s a node; the root's signings fixed,
new ones the node's), each the lower of it and the node's GF(2) bound:

| n | GF(2) | `root` mode | `node` mode | all conflicts (recorded) | per-frame, every 10th (recorded) | ms a node (`root`, `node`) |
|---|---|---|---|---|---|---|
| 936 | 163 of 300 | 212 (49 of the 137 open) | 297 (134 of 137) | 278 | 25-29 of 30 | 2.8, 106 |
| 1215 | 167 of 300 | 209 (42 of 133) | 281 (114 of 133) | 249 | 23-27 of 30 | 5.2, 577 |

**K3** passes: 86% of the nodes GF(2) leaves open at 1215 are cut with node-fitted multipliers (98% at 936), more than
the LP over all conflicts cut.  The root's multipliers alone cut 0.37 of what the node's do, under the design's half: E3
should start with form (c), the node's own master warm-started from its parent's columns.

*Conclusion.*  C1 holds at the roots measured: with signings made for the cores its own columns meet, the LP closes
about 60% of the root's GF(2) gap from 900 to 2175, with no trend in n (+1.0 ± 2.6 points per 100 n), and at the probe's
nodes it cuts nearly every node GF(2) leaves open when the node fits its own multipliers.  Fixed signings do not do it.
The cost is the multipliers: tens of seconds a root, 0.1-0.6 s a node, two to three orders of magnitude over the GF(2)
bound; the design's next steps are E2 (past 2175) and E3 (a branch and bound with the bound in its three forms, form (c)
first), and THEORY.md now proves the bound (Theorems 14 and 15).

*Questions for the advisor and deviations from the design* (each resolved as stated, all sound):

1. The master's link rows stalled (above); the master by rank inequalities with an exact certificate replaced it.  The
   LP, the bound and its validity are the design's; the certified lower bound is the weight of a point of the LP's
   polytope instead of the master's value.
2. The closing tolerance is `max(0.25, 0.005 × gap)`, and the exact checks run only once a separation round finds no new
   core (intermediate families stop at the cheap inequalities): the design's 0.25 at every family did not fit the budget
   past 1300.
3. A GF(3) loop of any extra matroid is left out of the intersection (L at λ_{k,x} = w_x; Theorem 14 (3)); none
   occurred.
4. `Scratch::stuck` is private to the crate and the study may not change it: `cores.rs` has its own peel (the same rule,
   checked against `winnable_using`).
5. "Delete from R the lightest candidate of the cores found" was read as: after each core, un-apply the lightest of its
   breaking candidates (cumulatively), so each next core is a different one; a core with no promoted member is broken by
   a promoted-out factor listed once.
6. `merge` uses the column-sum construction for a factor no assigned core lists (no limit on its listers), the 2^12
   search only where assigned cores share the factor.
7. At nodes (`node` mode) the root's signings are kept fixed (their cores are the root's) and the multipliers start at
   the root's; each node bound is the lower of L and the node's GF(2) bound.
8. Test 7 runs on boards of 300-700 restricted to ≤ 12 candidates (the boards with ≤ 16 candidates have GF(2) gap 0, so
   no extra matroid can cut there), with a relative tolerance of 10⁻⁵ (the perturbations and the grid).  The design's
   test 4 lives in `cores.rs`'s tests.
9. 66 roots, not 67 (1475 is a multiple of 25).  The census reports `det0` for square cores and the rank deficiency over
   GF(3) for all, and adds whether the plain signing sees each core.

## E2: past 2175 (2026-10)

*Question.*  The advisor's second consultation (S4): does the main arm's share hold on the games past 2175, where the
chain stops and the root gaps are 440 to 1,440?  The kill criteria were fixed before any run, on `f1cores` certified at
600 s a root: **K1** the mean share over the roots from 2200 on that converge (a bracket within 1% of the gap) below
0.25; **K2** a slope of the share against n over 900 to the last root of −2 points per 100 n or worse with t ≤ −2; and
if more than 30% of the roots from 2200 on fail to converge in 600 s, the mean over all of them decides (their shares
are lower bounds on their LPs').  The same runs decide the limit on core-derived signings a frame (16 against 8, adopted
at a gain of 0.05 of the gap or more at the dear roots), whether *search mode* (a fit with no exact checks, 60 s)
reaches E1's certified bound within 5% of the gap at 80% of E1's 54 converged roots, and they write the root fits that
E3 starts from.

*Roots and data.*  The optima past 2175 are those of the EC2 chain, from a snapshot of its results to board 3423 (not
yet in `results/`: `--known` takes the snapshot after `results/strict.jsonl`, and [the files section below](#files) names it by its hash).  The roots: every 100th game of 2200-3400 (13) and the
*dear* games, the searched games (proof `exhausted`) past 2175 with the largest root GF(2) gaps: 3300 (1443), 3333, 3335
and 3336 (1427 each) and 3176 (1388): 17 distinct roots, 3300 being both.  3333, 3335 and 3336 are neighbours of one
search with nearly one root, three roots of one family and not three independent ones.  Two games of proof `cap` have
gaps as large (3303: 1443, 3339: 1427); they were not searched, and are not among the dear roots.

*Instrument* (32cccb3; the bound, its validity and E1's results are as they were, see the reproduction below).
`--fit-mode search` skips the exact checks: a family's generation ends when no column prices out at the master's duals
and no cheap inequality is violated, the run when the separation then finds no new core (`Stop::Fitted`); `lb` is 0, so
the root does not count as converged.  `--save-fit DIR` writes the fit of each `f1cores` run as `DIR/n{N}.json`:
`common/fit.rs` (its module comment is the format) holds the signings, the multipliers, the final master's columns and
its rank inequalities, reads a file back, and gives its bound again at the root, which every run asserts before it
writes.  The fit is that of the *lowest L the run evaluated*, not of the final family: merging cores into a signing
changes its matroid, which is not a restriction of the old one, and then the final family's LP can be the higher; a
certified lower bound above the run's bound proves it, and happened at 3335 (the final family's lower bound 254732.7,
the bound 254722) and in 17 of E1's 132 core-arm runs.  An arm `f1cores16` is the main arm with 16 core-derived signings
a frame.  `common/dw.rs` already dispatched the intersection to `Meet<3>` for blocks of 129 to 192 coordinates (it
asserts at most 192; the largest frame matroid has rank 137 at 3423, 128 at 3200 and 132 at 3300); a test compares it
with the reference at 3200, 3300 and 3400, and every run checks it every 50th pricing as before.  A test reads
`solver/frame/testdata/fits` back and checks each file's bound at its root.

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

Twelve of the 17 roots converged (5 did not, 29.4%: under the 30% at which the rule changes), so the criteria are
computed on the converged roots.  **K1**: the mean share over them is **0.464** (0.466 over all 17; killed below 0.25).
**K2**: over the converged roots of 900-3333, E1's 54 and E2's 12, the slope is **−1.00 ± 0.34 points per 100 n, t =
−2.9** (killed at −2 or worse with t ≤ −2); over all 83 roots −0.84 ± 0.27, t = −3.1.  So E2 is **not killed**; it does
not pass strongly either (E1's measure: a mean of 0.5 and a slope above −1 point).  Past 2200 alone the share is flat:
by 200 n from 2200 the means are 0.43, 0.45, 0.47, 0.56, 0.53, 0.41, 0.49 (−0.02 ± 0.69 points per 100 n over the 12
converged roots), at a mean of 0.464, 0.155 ± 0.046 below E1's 0.619 at 1700-2175 (t = 3.4): the share fell as the games
went from E1's to E2's, and then stopped falling.

*The fall is the gap's.*  After the fact (not one of the criteria), over E1's and E2's 83 roots of the main arm,
converged or not: the share against the root's GF(2) gap is 0.725 at gaps of 0-199, 0.571 at 200-399, 0.544 at 400-599,
0.467 at 600-799, 0.540 at 1000-1249 (3 roots) and 0.413 at 1250-1499 (6 roots), and a least-squares fit of the share on
n and the gap gives +0.9 ± 0.6 points per 100 n and **−3.8 ± 1.1 points per 100 of gap**.  The chain's gaps grow with n
(+37 per 100 n over 1301-3312), so the share seems a function of the gap, not of n, and at the gaps of the dearest
games, 1,300 to 1,450, it is 0.4 (0.37-0.40 at 3300 and 3333-3336).  E1's draw of roots had gaps of 70-770; E2's are
440-1,443.  (The standard errors take the roots as independent, which they are not quite.)

*The unconverged roots are near their LPs.*  At the five roots that did not converge, the lowest L is within 6.2-9.9 of
the master's value, an uncertified estimate of the LP (1% of their gaps is 12-14): what is missing is the certificate,
whose exact checks and separation take three quarters of the time (3300: 535 s of 600 outside the intersections and the
master's simplex, 15.7 million pivots in the checks), not a bound that is still falling.  Their shares are probably
within about 0.01 of the LPs'.  At 3100 the share (0.633) is the highest of the 17.

*The limit of 16.*  With 16 core-derived signings a frame the shares at the six roots of gap 1,013 or less are higher by
0.086, 0.041, 0.027, 0.050, 0.026 and 0.051 of the gap (mean 0.047; 2200 converged, in 471 s against 27 s); at the five
dear roots the gains are −0.024, −0.021, +0.013, +0.008 and +0.056 (mean **0.006**; the design asks for 0.05), and at
3400 −0.128.  Over all 12 roots the mean gain is 0.015 and three are worse.  At the large gaps the 16-signing fit does
not finish in 600 s (all five dear roots end at the budget, with certified lower bounds far below), because its rounds
end in an exact check of about twice as many matroids (49-63 against 28-36): at 3333, 3335, 3336 and 3400 it completes 5
or 6 rounds where the main arm completes 12 to 14, and finds fewer cores (3400: 128 against 648).  By the rule **the
limit stays 8**; whether 16 would gain at the dear roots with more time is not measured (its gain at the roots that
converge, 0.03-0.09, suggests it would, and that the limit of 8 binds: every root has 8 core-derived signings on at
least one of frames 1-4, and 57% of the cores found cannot be placed).

*Search mode does not serve; a short certified budget does.*  Search mode as specified ends in under 4 s (the 60 s
are not used: the separation finds no new core after 23 to 480 pricings, where E1's runs at the same roots took 133 to
1,082) with a bound above E1's certified one by **0.274 of the gap** on average (median 0.252, worst 0.640; 0.26 to 0.33
in every band of 500 n), within 5% of the gap at **0 of E1's 54 converged roots** (80% needed).  Its master has only the
cheap inequalities (the most violated level set of its point, and the support sets, closed), which describe a relaxation
of the LP: the exact checks find rank inequalities that those miss, so what they add is bound, not only certificate.  By
the design's rule the fit is then *certified mode with a longer budget*, and a budget alone does it: the main arm
certified with **60 s**, cut off wherever it is, is within 5% of the gap of E1's bound at **54 of 54** of E1's roots
(mean difference 0.002 of the gap, worst 0.044, at 1950 and 1975, which need 88 and 114 s to converge) and, at the 17
roots past 2175, above the 600 s run's by 0.038 of the gap on average (median 0.025), within 5% at 12 of the 17 (the
other five, 2500, 3176, 3200, 3300 and 3400, are above it by 0.088, 0.058, 0.117, 0.051 and 0.151).  A fit of 60 s thus
costs under 0.05 of the gap where the gap is under 800 and up to 0.15 at the frontier's gaps, where the design's budget
that grows with the gap is the right shape.  The runs of the 54 roots with the new code reproduce E1's lines exactly
where both converged within 60 s (48 of 48 roots: bound, lowest L, lower bound, pricings, rounds, cores, columns and
cuts all equal): the changes to `dw.rs` did not touch the fit.

*Fits, signings and cost.*  The 17 fits are in `solver/frame/testdata/fits` (14 MB; the README there says what each holds).  The
main arm took 81 minutes of one thread for the 17 roots (the design's 5 hours were for 600 s at every root), 27 to 600 s
a root; the 12 roots of the limit of 16, 96 minutes.  A pricing (one intersection and the greedies) costs 1.2 ms at 2200
and 2.9 ms at 3400 (0.15-0.96 ms at 900-2175), 640 to 2,660 of them a root; over the 17 roots the intersections are 1%
of the time, the master's simplex 24% and the rest, 75%, the exact checks (millions of pivots a root) and the
separation, converged roots and others alike.  The cores come from more frames than E1's: frame 4 holds core-derived
signings at every root from 2700 on, with 8 at 6 of the 12 (E1 had them on frames 1-3), and frames 5 and 6 at four
roots; a root finds 150 to 663 distinct cores, 57% of them unplaced (E1: 48%).  No GF(3) loop occurred.

*Conclusion.*  E2 does not kill A1 at the frontier, and does not repeat E1's strong pass.  The main arm closes 0.46 of
the root GF(2) gap at 17 roots of 2200-3400 (12 certified; the master's value puts the other five within about 0.01 of
their LPs): flat past 2200 (−0.02 ± 0.69 points per 100 n) but 0.15 below E1's 0.62, a fall that goes with the gap (−3.8
± 1.1 points per 100 of gap, +0.9 ± 0.6 per 100 n beside it) and leaves 0.37-0.40 at the dearest games, gaps of 1,300 to
1,450.  The advisor's table S2 (second consultation) puts a share of 0.3 at +210 of affordable gap and 0.5 at +930 for a
node 20 times as dear as GF(2)'s; 0.4 lies between.  The limit of 8 core-derived signings a frame stays (16 gains
0.03-0.09 where it converges, nothing at the dear roots in 600 s), and the family stays fragmented: every root fills the
limit on some frame, and 57% of the cores found go unplaced.  For E3 and E4 the root fit should be the certified arm
with a budget that grows with the gap, not search mode (60 s costs under 0.05 of the gap below a gap of 800 and up to
0.15 above it, and the 17 certified fits are in `solver/frame/testdata/fits`).

*Questions for the advisor and deviations from the design* (each resolved as stated, all sound):

1. 17 distinct roots, not 18: 3300 is a multiple of 100 and a dear root.  The dear roots are the literal five largest
   gaps among the chain's exhausted searches; 3333, 3335 and 3336 are neighbours (one root family), so the verdict on
   the limit at "the dear roots" rests on three families, 3300, 3176 and 3333-3336.  3303 and 3339 (proof `cap`) were
   left out.
2. The saved fit is the lowest L the run evaluated, not the final family's, so that the file's `bound` is the run's
   bound and reproducible; the cuts of the final master are kept for the signings the fit has and their ranks are the
   final family's (a reader takes the ranks again).  `support` (how many leading columns are the master's support) is an
   addition to the design's format.
3. Runs beyond the design: the main arm certified with 60 s at E1's 54 roots and at the 17 roots (the design's fallback
   "certified mode with a longer budget" needed to know what a budget buys), and the roots reproduction above.  The
   search-mode criterion is measured on the bound each run reached (E1's `bound` against the search run's), the lowest L
   over all families, not E1's `l_best` of the final family.
4. The kill criteria were applied as written: K2 over the converged roots of 900-3333 (E1's 54 and E2's 12); the slope
   over all roots is also given.  Its t of −2.9 with a slope of −1.0 is a significant fall that the criterion, set at
   −2, does not count; the gap fit above is an after-the-fact reading of it and not a test.
5. The limit-of-16 fits (12 MB) were not kept in the repository; the budgets are wall-clock, and the runs shared the
   laptop with other jobs (the 1-minute load was 4 to 10), so the five unconverged roots might converge on an idle one.
   No run overlapped the owner's pause of the machine's low-priority jobs (14:00-14:40), which would have eaten its
   budget.
6. `--known` already took several files (E1's code: `Known::load` gets all of them); no change was needed.

## Later (2026-10): the program removed from master

The example `fields`, with the files of `examples/common/` it included, was removed from master on 2026-10-17.  It
builds at commit 8753189: `git checkout 8753189`, then `cargo build --release --example fields` in `rust/`.

## Files

The data and the tables of this entry, with E2 as its last section; for the bound in a branch and bound (E3's pilot) see
[its entry](../the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md), and for R3 and the slope test (U4)
[theirs](../r3-and-the-slope-test-u4-of-a1-2026-10/README.md).
This directory holds the data that `examples/fields.rs` wrote (its module comment and `common/dw.rs`'s define the
bound and every field), `fields.py`, which makes the first entry's tables from them (`./fields.py >
fields_output.md`; `./fields.py --roots lite.jsonl --lite` for the quick look), `e2.py` (E2's), `e3.py` (the
pilot's) and `u4.py` (U4's, from `u4/`, in [its entry](../r3-and-the-slope-test-u4-of-a1-2026-10/README.md)).  Everything ran on the M1 Max, one process at a time under
`nice -n 10` (two at a time for U4), one thread, with other jobs on the machine; times are wall seconds of one
thread.

### Files and the commands that made them

`fields` is `target/release/examples/fields` (`nice -n 10 cargo build --release -j 4 --example fields`), run from
`rust/`; `K` is `--known results/strict.jsonl`.  The example `fields` and the files of `examples/common/` it included
(`dw`, `gf3`, `cores`, `lp`, `fit`, `bnb`, `frames`, `meet`; `fit.rs` specifies the format of `fits/`) were removed from
master (2026-10-17); they build at commit 8753189 (`git checkout 8753189`, then the build command above), and `git
show 8753189:rust/examples/common/fit.rs` shows the specification.

| file | command | what it is |
|---|---|---|
| `lite.jsonl` | `fields roots K --arms f1cores --seconds 120 --out lite.jsonl 900-2100/100`, the code of b8d8256 | E1-lite: the main arm at 13 roots (before the master's stall restart and the exact checks' column limit) |
| `roots.jsonl`, `census.jsonl` | `fields roots K --arms gf2,f1,f1r1,f1r3,cores,f1cores --seconds 120 --fixed-seconds 30 --out roots.jsonl --census census.jsonl 900-2175/25 936 992 1088 1184 1196 1215 1240 1254 1288 1430 1472 1521 2170 2172`, the code of 1051560 | E1: six arms at 66 roots (the design's 67 count 1475 twice), and every core the main arm found |
| `nodes.jsonl` | `fields nodes K --modes root,node --seconds 10 --root-seconds 120 --exact solver/notebook/matroid-intersection-bounds-2026-10/nodes_exact.jsonl --out nodes.jsonl testdata/perf/n0936.nodes testdata/perf/n1215.nodes`, the code of 1e12c77 | E1n: the probe's 600 nodes, in modes `root` and `node` |
| `check.jsonl` | `fields check K --samples 200 400-2175/25` | Theorems 14 and 15 and the column-sum signings at scale: per board, the promotion sets and signed matroids checked |
| `fields_output.md`, `lite_output.md` | `./fields.py > fields_output.md`, `./fields.py --roots lite.jsonl --lite > lite_output.md` | the tables |
| `roots_e2.jsonl`, `fits/` | `fields roots K S --arms gf2,f1cores --seconds 600 --save-fit solver/frame/testdata/fits --out roots_e2.jsonl 2200-3400/100 3333 3335 3336 3176`, then the same with `--arms f1cores16` at 2200-3200/200, 3300, 3333, 3335, 3336, 3176 and 3400, the code of 32cccb3 (run in pieces of a few roots, each resuming where the last ended) | E2: the main arm certified (600 s a root) at 17 roots past 2175, the limit of 16 core-derived signings a frame (`f1cores16`) at 12 of them, and the fit of each main-arm run (below) |
| `search_e1roots.jsonl` | `fields roots K --arms f1cores --seconds 60 --fit-mode search --out search_e1roots.jsonl 900 925 936 ... 2175` (the 54 games of `roots.jsonl` whose `f1cores` line has `converged` true), the code of 32cccb3 | E2: search mode (no exact checks) against E1's certified bounds |
| `budget60_e1roots.jsonl`, `budget60_e2roots.jsonl` | `fields roots K S --arms f1cores --seconds 60 --out budget60_e1roots.jsonl` at the same 54 roots, and `... budget60_e2roots.jsonl 2200-3400/100 3333 3335 3336 3176`, the code of 32cccb3 | E2: the main arm certified with a budget of 60 s (not run to convergence): the fit a search can afford, against E1's and the 600 s runs |
| `e2_output.md` | `./e2.py > e2_output.md` | E2's tables |

`S` is `--known` of a snapshot of the EC2 chain's results to board 3423 (`chain_snapshot2.jsonl`, 3,423 games, sha256
`a24709934602002d79c9caf29f4b39e3c3aad17c9650115fcb8d4ccd18e008a9`; the chain's games past 2175 were not yet in
`results/` then; they are now), after `K`; the optimum of each game used is `opt` of its line.  The dear roots are the searched games
(`method` search, proof `exhausted`) past 2175 with the largest root GF(2) gaps in that snapshot: 3300 (1443), 3333,
3335 and 3336 (1427 each; neighbours of one search, with the same root structure) and 3176 (1388, the longest search
of the three games of that gap).  Games with a proof of `cap` (3303: 1443, 3339: 1427) were not searched.

The root fits that these runs saved (`fits/`) are in [solver/frame/testdata/fits/](../../frame/testdata/fits/README.md), where the tests read them.
