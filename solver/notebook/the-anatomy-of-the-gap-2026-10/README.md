# The anatomy of the gap (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  Where does the root's gap live (by frame, by kind of candidate), which rule of the game do the
relaxations' heaviest sets break, which part grows with n, and which valid inequalities cut it?  Begun against the
matching and Lagrangian bounds; when the GF(2) bound of the [matroid study](../matroid-intersection-bounds-2026-10/README.md)
arrived, re-aimed at the gap that remains above it.  Run in parallel with that study, on the same laptop.

*What was kept.*  Nothing but this entry and the next ([the per-frame bound](../the-per-frame-bound-2026-10/README.md)); the solver
is unchanged.  The code of both lived on a branch that was never pushed and has been deleted, as the project does with
research branches once they are harvested.  It was one example program (the measurements, a small simplex solver of its
own for the LPs, and the per-frame bound's module), a table script and the data, and the move of the matroid study's
frame matroids out of `examples/matroid.rs` into a file the two examples shared.  The methods are written out here to
be redone.  What the studies read is on master: the built-in optimal games, the probe's fixtures
(`testdata/perf/n0936.nodes`, `n1215.nodes`), the matroid study's frame matroids and weighted intersection
(`examples/matroid.rs`, `examples/common/meet.rs`) and its best completions at the fixtures' nodes
(`solver/notebook/matroid-intersection-bounds-2026-10/nodes_exact.jsonl`), and the slope study's root Lagrangian bounds and 19 margin-scan games
(`solver/notebook/what-drives-the-slope-2026-10/slope_roots.jsonl`, `slope_proofs.jsonl`).

*Method.*  No solver code changed.  Games are set up as `taxman solve` sets them up (default tactics); `S*` is the known
game's moves that are candidates and `P* = sum(S*)`, from the built-in optimal games.  One thread a process, at most two
processes; the cut separations stop at time limits, so a loaded machine finds slightly fewer cuts.  The program
computed:

* *Heaviest sets against the optimum.*  The heaviest set of the matching bound (a min-cost flow of its own over the
  matching bound's graph: every move matched to a distinct live factor of its frame, a candidate promoted when matched
  as a mover of the frame below its home, weighing its number) or of the GF(2) bound (the matroid study's
  intersection), with its ties broken towards `S*` (weights `1024·x + [x ∈ S*]`, so the errors shown are the least the
  bound's own ties allow), against `S*`: the members it has that `S*` lacks (*swaps*), by home frame.  Applying the set
  to the root's frames (a frame gains the set's members of the frame above as moves and loses its own members as
  factors), the *core* of each frame it leaves unwinnable (the moves a peel cannot drop), the core's GF(2) rank over its
  movers' live-factor vectors (an *odd* core has rank equal to its size: independent), and the movers' live-factor
  counts.
* *The matching bound as an LP.*  A variable per edge of the matching graph (a mover to a live factor of its frame, a
  candidate weighing its number, the moves weighing a constant larger than every sum so that all are matched), a row
  per vertex (at most one edge each), solved by a dense-tableau simplex in floating point (primal, and dual to
  re-optimize after cuts).  A candidate's promotion variable `z_x` is the sum of its edges as a mover.  *Cover cuts*
  `Σ_K z ≤ |K| − 1`: for the LP's solution, the candidates with `z ≥ t` at `t` = 0.999, 0.9, 0.75, 0.5, 0.3, 0.1, 0.01
  form a set; up to 12 times, a conflict of it (`Model::conflict_using`: a minimal subset that cannot be promoted from
  the node's state) is added if violated, and its largest member removed from the set; rounds until none is violated.
  *Rank cuts* `Σ_T z ≤ r` for unions `T` of the connected groups of fractional candidates linked by tight cuts, their
  supports, and the unions of two or three tight cuts that share a fractional member (at most 26 members; 3 s of
  separation a round, 30 s a root, the 60 most violated added).  With `l = Σ_T z` the cut `Σ_T z ≤ r` for the largest
  integer `r < l` is valid when no promotable set holds more than `r` members of `T`, and that is proved by exhaustion:
  a subset of a promotable set is promotable, so such a set would be `T` with at most `|T| − r − 1` members left out,
  and the check tries every way: if the set at hand cannot be promoted, take a conflict of it (one of its members must
  be out), and branch on which, each branch keeping the members tried before it so that no choice is explored twice,
  down to the allowed number of leave-outs (a budget of 3,000 conflict computations; past it the cut is not made).  A
  greedy promotable subset of `T` as large as `l` rules the cut out at once.
* *The GF(2) bound with cuts.*  The LP over the convex hull of the two frame matroids' common independent sets (of at
  most `max_total` members), with cuts `Σ_T z ≤ r`, by column generation.  The master is the dual: variables `θ` and
  `π_c ≥ 0` for each cut `c`, minimize `θ + Σ_c r_c π_c` subject to `θ + Σ_c π_c |S ∩ T_c| ≥ sum(S)` for each generated
  set `S` (a column; the empty set to start from, costs perturbed by about 10⁻⁶ against degeneracy).  Each step prices
  with one weighted intersection for the weights `64x − Σ_{c ∋ x} π_c`, the multipliers rounded down to 64ths, so that
  it is exact in integers; that set's weight plus `Σ r_c π_c` is a Lagrangian bound, an upper bound on every promotable
  set's sum for any `π ≥ 0` (weak duality), and every step's value is kept, the least reported.  The set becomes a row
  of the master (so a column of the primal) unless the master's multipliers already satisfy it (within 10⁻⁷) or it was
  generated before, which ends the generation, as do 2,000 steps.  The master's solution, `z = Σ μ_S 1_S` for the duals
  `μ` of its rows, is where cuts are separated, in two phases (cover cuts to exhaustion, then cover and rank cuts
  together), and each call starts from the sets generated so far.  Also with the matching LP's cuts added.
* *Nodes.*  At every 10th node of the fixtures (30 each, the units that the solver's proofs of 936 and 1215 handed to
  its searcher), the same bounds next to the target `P* − sum(promoted)`: a node is cut when a bound is at most it, and
  "open" is the mean of `max(0, bound − target)`.  The root's cuts are those of the matching LP and of the GF(2) hull's
  separations together; projected to the node they are `Σ_{T ∩ cands} z ≤ r − |T ∩ promoted|` (kept when they can bind;
  a node with more than `r` of `T` promoted has no completion), valid at every node; the *root's multipliers* are the
  `π` the root's column generation ended with, giving one intersection a node (weights `64x − Σ π`, plus `Σ π (r − |T ∩
  promoted|)`); the *node's multipliers* are those of column generation on the node's own hull with the projected cuts;
  and cuts separated at the node itself, as at a root.
* *A plain branch and bound* with these bounds: the matroid study's skeleton (section 2 there: one bound, no refinement,
  forced members or nogoods; a node is cut when its largest candidates or its bound fall below what it needs; else the
  node branches on the members of a conflict of the GF(2) bound's heaviest set, "`k₁ … k_{j−1}` promoted, `k_j` not"),
  the branching the same in every kind so that the trees differ only by what the bound prunes.  A node is cut when the
  GF(2) bound does; where it does not, the Lagrangian bound over the root's cuts at the root's multipliers (one
  intersection), or the LP over the node's hull and the projected cuts, is tried.  The incumbent is `P*`, proving the
  optimum, on the 19 games of the slope study's margin scans (900, 936, 950, 992, 1000, 1025, 1050, 1088, 1100, 1150,
  1184, 1196, 1215, 1225, 1240, 1250, 1254, 1288, 1300).

The tables below are means by band of 50 over the 361 non-prime games of 880-1300 (the first band is 880-900), the
trends are least-squares slopes per 100 in n with their standard errors.  *Validity*: every bound is an upper bound by
weak duality over valid cuts (a cover cut is a conflict of its node, a rank cut's rank is proved); at all 361 roots of
880-1300 and 309 boards of 2-400 none is below `P*` (the GF(2) bound with all the cuts is within 1 of it at 305 of the
309, the GF(2) bound alone at 177), and at the 39 probe nodes for which the matroid study found the best completion,
none is below it.

**1. Both relaxations promote as many as the optimum, but the wrong ones.**  The heaviest set of the matching bound, and
that of the GF(2) bound, has exactly as many members as `S*` at all 361 roots; they differ by swaps (the matching
bound's 8-18 a game, the GF(2) bound's 2-14; band means 9-15 and 3.5-8).  The freedoms and `max_total` never bind; the
error is entirely which candidates, never how many.

| n | matching gap | its excess by home frame 1, 2, 3 | its cores in frames 0-3 (movers), odd ones | GF(2) gap | its excess by home frame 1, 2, 3 | its cores in frames 0-3, odd ones |
|---|---|---|---|---|---|---|
| 851-900 | 432 | 288, 105, 22 | 41, 53, 6, 15; 21 of 58 | 88 | −228, 278, 22 | 0, 49, 15, 15; all |
| 951-1000 | 749 | 338, 346, 176 | 53, 51, 37, 17; 0 of 172 | 172 | −115, 254, −65 | 0, 54, 32, 6; all |
| 1101-1150 | 522 | 345, −216, 362 | 57, 39, 44, 10; 7 of 242 | 107 | 12, 214, −118 | 0, 34, 43, 0; all |
| 1251-1300 | 1277 | 794, 1169, −769 | 62, 66, 53, 16; 28 of 207 | 185 | 45, 434, −294 | 0, 60, 40, 12; all |

(Band means; "home frame h" are the candidates promoted out of frame h into frame h − 1.)  The matching bound's excess
grows in homes 1 and 2, into frames 0 and 1, whose cores are mostly *even* (GF(2)-dependent; 87 of its 1,602 cores are
odd): exactly what the GF(2) bound removes.  What the GF(2) heaviest set breaks is winnability alone, through cores that
are all odd (all 841 of them), in frames 1-3 and never frame 0: frame 1's holds 34-60 movers, frame 2's 15-43, over as
many factors as movers or a few more; 36-42% of their movers have two live factors, 42-48% three, 5-9% four or more.
Its excess is heavier promotions from frame 2 into frame 1, partly paid back by lighter ones from frame 3.  *What grows
with n* is the size of these odd cores and what they hold: the GF(2) heaviest set's conflict has 12.9 members on
average, growing 1.1 ± 0.1 per 100 in n, and its cores hold 20.6 of its promotions, growing 1.8 ± 0.2 (16 at 851-900,
26 at 1251-1300).

**2. Cover cuts are the Lagrangian bound; their LP is half-integral, and rank cuts close most of the rest of the
matching gap.**  The matching LP with separated cover cuts (24-66 of them, mean 41, tens of milliseconds) gives at every
band what the slope study's root Lagrangian reached in up to 8,000 steps, within 27 of it (band means 114-472 against
112-471; 1 to 27 above, once 2 below), as the exact separation of [How far can nogoods
go](../how-far-can-nogoods-go-2026-10/README.md) found too (its LP over every conflict is 0-12 below that Lagrangian): the root's
conflicts are worth that much and no more.  Its solutions are half-integral over clusters of overlapping conflicts: at
936, the pairwise conflicts {370, 410}, {410, 430} and {370, 430} each hold 1 with all three at ½, though at most one of
the three can be promoted.  Rank cuts on the unions of tight cuts (`{370, 410, 430} ≤ 1`, and unions of 20-26 members
with ranks 2-7, mostly 3-4, below their size, in homes 1 and 2) take the gap to 3-15% of the matching gap:

| n | matching gap | cover-cut LP gap | rank-cut LP gap | GF(2) gap |
|---|---|---|---|---|
| 851-900 | 432 | 114 | 17 | 88 |
| 951-1000 | 749 | 288 | 84 | 172 |
| 1101-1150 | 522 | 165 | 25 | 107 |
| 1151-1200 | 629 | 159 | 19 | 108 |
| 1201-1250 | 1091 | 430 | 148 | 148 |
| 1251-1300 | 1277 | 472 | 182 | 185 |

Over the 361 games the rank-cut LP's gap averages 81 and grows 26 ± 3 per 100 in n, the GF(2) bound's 136 and 4.6 ±
2.4: the rank cuts beat the GF(2) bound in every band below 1200 and match it above, and at the probe's nodes they cut
28 of 30 at both 936 and 1215 (the cover-cut LP 23 and 17, the matching bound none).  The GF(2) bound supersedes this as
a bound to build (it is exact on two-factor movers and grows a fifth as fast); the cuts' value is on top of it.

**3. On top of the GF(2) bound, cuts close two thirds of its root gap on average, a falling share.**

| n | GF(2) gap | + cover cuts | + rank cuts | + the matching LP's cuts too | that over the GF(2) gap |
|---|---|---|---|---|---|
| 851-900 | 88 | 28 | 11 | 7 | 0.08 |
| 901-950 | 147 | 78 | 36 | 14 | 0.09 |
| 951-1000 | 172 | 115 | 88 | 59 | 0.34 |
| 1001-1050 | 156 | 111 | 89 | 78 | 0.50 |
| 1051-1100 | 81 | 41 | 32 | 25 | 0.30 |
| 1101-1150 | 107 | 58 | 31 | 21 | 0.20 |
| 1151-1200 | 108 | 51 | 33 | 15 | 0.14 |
| 1201-1250 | 148 | 111 | 101 | 79 | 0.54 |
| 1251-1300 | 185 | 136 | 112 | 101 | 0.54 |

The last column's gap averages 47 and grows 13.9 ± 1.7 per 100 in n (the cover cuts alone: mean 84, +9.1 ± 2.0; with
rank cuts, 62, +13.2 ± 1.9), three times as fast as the GF(2) gap: the cuts remove most of the residual where the odd
cores are small and less than half where they are large.  The cover cuts alone, separated here by rounding, close 38%
of the GF(2) gap; separated exactly (every conflict), 46%, a share falling with n, with a residual growing 8.4 ± 2.0
([How far can nogoods go](../how-far-can-nogoods-go-2026-10/README.md)); with the rank cuts, 54%, and with the matching LP's cuts
too, 66%.  Unions of up to 48 members with 20,000 conflict computations a check move 1215 by 6 and 1300 not at all (the
time limit then allows fewer rounds).  The cover cut of a 13-member conflict is weak (the LP can keep 12/13 of each
member), the alternatives to one odd core are many other odd cores of nearly the same candidates, and the unions of two
or three cuts that a rank check can afford do not reach the family.  A counting cut was derived and not built (a set
`U` of factors and the movers whose live factors all lie in it: at most `|U| − 1` of them when each has two live
factors or more, at most `|U| − d₁ − d₂ + 3` with `d₁ ≤ d₂` the two smallest live-factor counts, since the first mover
played takes all its factors and the second shares at most one with it): it cuts a core that is square, and a rank cut
on the same candidates is at least as strong (its right-hand side is the exact rank), so the rank cuts measured bound
what such counting families do on the sets separated.

**4. At search nodes the root's cuts help when the multipliers are the node's own.**  Every 10th node of the fixtures
(30 each; cuts of 30, and "open"):

| n | GF(2): cuts, open | root's cuts, root's multipliers | root's cuts, node's multipliers | cuts separated at the node |
|---|---|---|---|---|
| 936 | 20, 25.7 | 24, 4.2 | 26, 2.0 | 27, 4.1 |
| 1215 | 14, 53.5 | 17, 31.6 | 22, 11.5 | 27, 7.3 |

(The last column is cover and rank cuts; cover cuts alone: 26, 6.9 and 22, 16.0.)  In the plain branch and bound,
against the GF(2) bound alone (nodes; the per-frame column is the [next entry](../the-per-frame-bound-2026-10/README.md)'s):

| n | GF(2) gap | with cuts | GF(2) | cuts, root's multipliers | cuts, node's multipliers | per-frame | s: GF(2) | per-frame |
|---|---|---|---|---|---|---|---|---|
| 900 | 119 | 5 | 740 | 254 | 78 | 1 | 0.2 | 13.3 |
| 936 | 132 | 31 | 3,392 | 2,236 | 425 | 467 | 1.0 | 33.0 |
| 950 | 176 | 58 | 8,584 | 7,823 | 1,326 | 5,606 | 2.5 | 175.7 |
| 992 | 318 | 143 | 77,567 | 75,371 | 32,790 | 23,011+ | 29.8 | 240.4 |
| 1000 | 214 | 101 | 10,120 | 9,467 | 3,663 | 9,339 | 4.0 | 158.0 |
| 1025 | 147 | 90 | 28,033 | 27,083 | 15,363 | 5,241+ | 11.3 | 240.4 |
| 1050 | 106 | 26 | 1,171 | 1,108 | 247 | 613 | 0.7 | 27.7 |
| 1088 | 81 | 16 | 289 | 271 | 140 | 253 | 0.1 | 15.5 |
| 1100 | 178 | 79 | 1,874 | 1,759 | 1,038 | 1,847 | 1.0 | 29.9 |
| 1150 | 77 | 2 | 251 | 167 | 29 | 230 | 0.1 | 30.3 |
| 1184 | 139 | 6 | 1,093 | 393 | 33 | 876 | 0.6 | 28.6 |
| 1196 | 174 | 58 | 7,825 | 6,278 | 1,174 | 2,994 | 4.6 | 81.2 |
| 1215 | 213 | 123 | 24,516 | 23,412 | 7,104 | 5,120 | 16.6 | 216.1 |
| 1225 | 105 | 63 | 3,306 | 3,172 | 1,123 | 1,922+ | 2.5 | 240.3 |
| 1240 | 86 | 41 | 1,486 | 1,184 | 394 | 719 | 1.1 | 58.9 |
| 1250 | 126 | 40 | 2,543 | 1,691 | 273 | 2,284 | 1.8 | 87.3 |
| 1254 | 204 | 105 | 18,956 | 17,093 | 7,551 | 5,407+ | 13.6 | 240.3 |
| 1288 | 209 | 153 | 13,831 | 13,377 | 5,609 | 9,345+ | 9.7 | 240.1 |
| 1300 | 151 | 112 | 17,136 | 16,901 | 10,128 | 7,817+ | 11.5 | 240.3 |

("With cuts" is the root's gap with all the cuts, section 3; + marks a run stopped at 240 s.)  The root's cuts at the
node's multipliers need 4.2 times fewer nodes (geometric mean over the 19 games; 33 times at 1184, 9.3 at 1250, 1.7 at
1300, 2.4 at 992) in 1.9 times the time (a column generation a node); at the root's multipliers, one intersection more a
node, 1.26 times fewer nodes (1.0-2.9) in 1.3 times the time.  Neither changes the slope of log10 nodes in n (0.046 ±
0.131 per 100 with the GF(2) bound alone over these 19 games, 0.080 ± 0.169 with the cuts at the node's multipliers,
0.069 ± 0.145 at the root's).  The root's gap reduction overpredicts: at 0.011 decades per unit of GF(2) gap (the
matroid study's rate) the root's 90 units on average predict ten times fewer nodes, and 992's 175 a factor of 90 against
the 2.4 measured: the root's cuts are about the root's cores, and deep nodes meet others.  The same holds for the cover
LP over every conflict solved at every node (nogoods: 6.2 times fewer nodes over the 146 proofs, less as n grows, a
quarter slower), and for the Lagrangian over nogoods beside the parity bound in the model-free proofs, whose multipliers
are fitted at the root and which cut nothing at 990 ([parity](../the-parity-bound-in-the-model-free-proofs-2026-10/README.md),
section 5).

*Conclusion.*  Both relaxations get the number of promotions right and the choice wrong.  The matching bound's error
is mostly even cores in frames 0-2 (the GF(2) bound's territory); what is left above the GF(2) bound is odd cores of
two- and three-factor movers in frames 1-3, never frame 0, and what grows with n is their size and the promotions
inside them.  Cover and rank cuts on top of the GF(2) bound close 91-92% of its root gap at 851-950 but 46% at 1201-1300
(both separations), and in a branch and bound give a constant factor of about 4 in nodes with multipliers refitted at
every node (1.26 with the root's), not a lower slope.

*What it implies.*

* *Superseded*: the matching-side anatomy and the rank-cut LP over the matching bound, as a bound to build, by the
  GF(2) bound.  They stand as measurements: cover cuts are worth what the converged Lagrangian is worth (as the
  nogoods study found by exact separation), and rank cuts on their unions are what is beyond them.
* *For the GF(2) search (phase 2 of the matroid study)*: carrying the nogoods and the Lagrangian over onto the
  intersection is worth up to a factor of about 4 in nodes if its multipliers are refitted near the nodes, and
  little with multipliers fitted at the root; whether a cheap refit (a few column-generation steps along the path)
  pays in time is the measurement to make there.  Rank cuts add to cover cuts mostly below 1200.
* *For the slope*: the lever left is a relaxation that sees odd cores of a frame, for frames 1 and 2, which neither
  GF(2), Lorea's matroid, cover cuts nor rank cuts of small unions do.  An exact per-frame bound (each frame's
  winnable sets' convex hull, the frames coupled through their shared candidates by a Lagrangian decomposition, with
  an exact search per frame as the pricing problem) is the next candidate: measured in
  [the next entry](../the-per-frame-bound-2026-10/README.md).

## Later (2026-10): the program removed from master

The example `matroid`, with the files of `examples/common/` it included, was removed from master on 2026-10-17.  It
builds at commit 8753189: `git checkout 8753189`, then `cargo build --release --example matroid` in `rust/`.
