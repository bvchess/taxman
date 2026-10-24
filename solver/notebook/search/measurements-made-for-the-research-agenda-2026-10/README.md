# Measurements made for the research agenda (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-14.  Concluded 2026-10-18.

**Question.**  What do four measurements on the roots of the whole game say about where a proof's cost comes from and
which avenues of the [research agenda](../../../docs/AGENDA.md) can pay?  **Answer.**  The trees follow the gaps: the
39 searches of the cloud chain of 2176-2276 run +0.08 ± 0.08 decades above the fit of 1301-2175, so the dear games are
the games of high gap.  At the roots, 566 of 1,055 parity-bound sets are dependent in some other field or signing,
barred parity bounds certify 92-96% of the upper part, and the gaps of 2176-3312 rise by 37 per 100 in n.  Of the
agenda's five avenues, A3, A4 and A5 then died at their cheapest experiment, and A1 died later in the search.

These were section 7 of the agenda, "Measurements made for this agenda": its "measurements 7.1" to "7.4" are the four
sections below.  The code was scratch and is not kept; each section says enough to redo it.

## 7.1 The root gaps of the c8g chain

The command was `examples/wall.rs nodes --known KNOWN --nodes 1 N...` (one thread, 29 s), with `KNOWN` the committed
chain followed by `solver/frame/testdata/c8g.chain-2176.jsonl`, for the 33 searched games of 2176-2276 that
`gaps_c8g.jsonl` lacks: [gaps_c8g_rest.jsonl](../the-wall-regime-2026-10/gaps_c8g_rest.jsonl).  Together they are every
exhausted search of the c8g chain (39 of at least 50 nodes).  The fit is the log10 of the nodes of
`results/strict.jsonl`'s 314 exhausted searches of 1301-2175 (at least 50 nodes) against `gf2 − opt` of
[roots.jsonl](../the-gf2-bound-in-the-search-2026-10/roots.jsonl), by least squares; the c8g chain's nodes (64 threads,
the build before the inner-loop round, as `strict.jsonl`'s) were then set against it.

| fit over 1301-2175 | coefficients | residual sd | R² |
|---|---|---|---|
| gap | 0.00392 ± 0.00022 per unit (×10 per 255) | 0.46 | 0.50 |
| gap, n | 0.00423 ± 0.00019, −0.00093 ± 0.00009 | 0.39 | 0.63 |
| gap / n | 6.95 ± 0.31 per unit of 1000 G/n | 0.40 | 0.61 |

The c8g chain's 39 searches run a mean residual of +0.08 ± 0.08 decades against the gap fit (sd 0.50), +0.45 against
the fit with n and +0.67 against the fit on G/n; their own fit is ×10 per 193 ± 40 of gap.  The largest residuals are
2268 (+1.09), 2260 (+0.95) and 2272 (+0.77), the smallest 2208 (−0.69) and 2205 (−0.65).

## 7.2 Other fields at the roots of the whole game

For every n of 78-1300 the parity bound's heaviest common independent set I at the root was computed (the prover's
`parity::Meet::solve` with 0 and 1 barred, the weights the numbers; 17 s for all), and the 1,055 games where it exceeds
opt(n) were kept.  I is peeled (front moves: a member with one live factor takes it; back moves: a factor with one live
lister goes to it; each keeps the rank over every field), and the residual R's matrix (its members against their
maximal factors outside I) is tested for rank over GF(3), GF(5), GF(7) and a large prime (for the rationals), with 0/1
entries and with Koszul signs (the entry of s at s/p is (−1)^i, i the index of p among the distinct primes of s,
increasing).

| n | games | mean gap | mean residual | dependent over GF(3), 0/1 | in some other field or signing |
|---|---|---|---|---|---|
| 78-299 | 73 | 6 | 8 | 0% | 0% |
| 300-599 | 281 | 28 | 17 | 43% | 49% |
| 600-899 | 300 | 77 | 33 | 47% | 57% |
| 900-1099 | 200 | 139 | 58 | 28% | 55% |
| 1100-1300 | 201 | 138 | 65 | 48% | 73% |

In all, 412 are dependent over GF(3) with 0/1 entries, 30 over GF(5) and 12 over GF(7); with signs 209, 39 and 27; none
over the rationals; the rank deficiency is 1 but in six residuals (2, over GF(3)).  Of the 658 square residuals, the
integer determinant with 0/1 entries is ±1 in 408, divisible by 3 in 219, by 5 in 21 and by 7 in 10; with signs it is
±1 in 572; it is ±1 under both in 358 (54%).

## 7.3 Barred bounds on the upper part

For n = 600, 1000 and 1300 (30 s for the three), the parity bound was computed with each member u of the recorded game
above n/2 barred, against opt(n).  It is certified (below opt(n)) for 207 of 224, 339 of 377 and 472 of 490 members.
The open ones are above opt(n) by at most 102, 208 and 147, at or below the root's gap (106, 214, 151).  The open
members at 1300 include the largest prime, 1297, which THEORY.md's lemma puts in every optimal set.

## 7.4 The root gaps of the EC2 chain

The command was `examples/wall.rs nodes --known SNAPSHOT --nodes 1 N...` (one thread, 9 minutes) over the 509 searched
games of 2176-3312 of a copy of the EC2 chain's results file (games 1..3314, taken on 2026-10-15: 64 threads on a
`c8g.16xlarge`, the code of 55287ff, a window of 16 to about game 3310 and 4 after it, 6.4 hours for 2176..3314).
`examples/matroid.rs lagrange --limit 0` and `cores` (removed from master since; it builds at commit 8753189) gave the
conflicts and cores of the root's heaviest set (3 minutes).  The committed chain's 391 searched games of 1301-2175 were
then solved again by that build (`taxman solve 1301-2175 --threads 1`, in order), to put the two chains' trees on one
build.  The data are in [the wall regime](../the-wall-regime-2026-10/README.md) (`gaps_chain.jsonl`,
`anatomy_chain.jsonl`, `resolved_1301_2175.jsonl`), the tables in `gap_output.md` (`gap.py` makes them), and the study
is [the gap past 2175](../the-gap-past-2175-2026-10/README.md).  It found:

* the gaps, 341 to 1,443, rising 37 per 100 n over 1301-3312, with G/n steady at 300: neither regime of the agenda's
  section 2;
* the cost model on one build: a decade per 250 of gap, trees 0.29 times the committed chain's, the gap counting in
  units of n^0.3 with 0 and 0.5 as brackets;
* the dear games are the games of high gap, and avenue A4 is killed (below);
* the projection: a day on a `c8g.16xlarge` reaches about 4,100.  The day itself reached 3440, below the projection's
  10th percentile for the center model ([the cloud run](../../the-frontier-chain-to-3440-2026-10/README.md)).

## Later (2026-10): the agenda's text of 2026-10-18

The agenda held the text below, its section 1 and the avenues A1 to A5, until 2026-10-19, when it kept a line for each
closed avenue and a shorter section 1.  This is the text condensed, with the numbers and the links.  Its "section 2",
"section 5", "(A1)" and "(C4)" are the agenda's.

### 1. Where we are

**The theory the search stands on.**  A set of moves is playable, independent over GF(2) in the parity matroids, or
matchable exactly when some payment set has one, an odd number, or at least one payment scheme ([THEORY.md, Theorem
12](../../../../docs/theory/franklin-moniot.md#the-counting-theorem)).  The GF(2) bound of the search is the frame
model's form of the middle condition (Theorem 11), equal to the parity bound at every root measured, exact on movers
with at most two live factors, and blind only to *odd cores*: self-covering sets with an odd number of schemes, which
need movers with three or more distinct primes.  At the roots, what the bound gets wrong is which candidates, never how
many, through odd cores of frames 1-3 whose size grows with n ([anatomy](../the-anatomy-of-the-gap-2026-10/README.md)).

**What drives the cost.**  A proof's nodes follow the root's gap and nothing else measured: ×10 per 236-266 of GF(2)
gap past 1300, as ×10 per 130 of Lagrangian gap did on the matching path
([slope](../what-drives-the-slope-2026-10/README.md)).
At a given gap a larger board has the smaller tree, the gap counting in units of √n ([the gap past
2175](../the-gap-past-2175-2026-10/README.md)).  A node costs +0.042 decades per 100 in n at the root of a tree and
+0.02 averaged over a whole one, 98% of it the GF(2) intersection; the fixed costs of a game are gone since round A; a
proof scales 58 times on 64 threads ([wall regime](../the-wall-regime-2026-10/README.md),
[round A](../round-a-the-fixed-costs-of-a-game-2026-10/README.md),
[Graviton calibration](../../graviton-calibration-2026-10/README.md)).

**What had been tried**, by family:

* *Bounds on top of GF(2)*: cover cuts and the Lagrangian over nogoods (46% of the root gap by exact separation), rank
  cuts (54%), the per-frame exact bound (59% within 200 steps, at 300 to 3,000 times the cost), Lorea's matroid
  (nothing), the Lagrangian of the symmetric pairs (nothing).  Every share closed falls with n, and in a search each was
  a constant factor ([anatomy](../the-anatomy-of-the-gap-2026-10/README.md),
  [per-frame](../the-per-frame-bound-2026-10/README.md), [nogoods](../how-far-can-nogoods-go-2026-10/README.md)).
* *Branching and learning*: strong branching, forced members, nogoods, the rule of symmetric pairs (×0.8 of the
  cycles); dominance by exchange and transpositions (none to speak of).  Constant factors, all taken.
* *Reuse along the chain*: the ceiling, the prime formula and same search are in; carrying nogoods, a chain that comes
  back down, ceilings from neighboring problems and equivalent problems gave nothing more
  ([going higher](../going-higher-and-coming-back-down-2026-10/README.md)), nor did reusing the previous game's tree,
  carrying its nogoods on the GF(2) path or proving a run in one search
  ([carrying](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md)).
* *The node and the engine*: the inner-loop round (×4.4 cycles), set widths, PGO, round A.  Constant factors.

The one change of slope in the project's history is the GF(2) bound itself (the plain branch and bound's log10 nodes
+0.14 per 100 in n against the matching path's +0.48).  Everything since has moved the curve down, not bent it.

### 3. The avenues, A1 to A5

**A1. A bound over several fields and signings** (a change of slope, if its share held).  A playable set's payment
matrix is unit triangular after its play order, so its rows are independent over every field and for every choice of
nonzero entries ([THEORY.md](../../../../docs/theory/playability.md#the-peel-order-lemma)).
Each choice (a field, and signs or values on the edges) gives a pair of frame matroids, even and odd, in which every
promotion set is independent.  Alone, each is weaker than GF(2): GF(3) with 0/1 entries misses the triangle {6, 10, 15}
(determinant ±2), and with signs it sees only the orientable closed surfaces.  Together they are stronger: the LP over
the intersection of the matroid polytopes is a valid bound at least as tight as GF(2)'s, computable in polynomial time
by Lagrangian decomposition into two weighted matroid intersections, one per field, with one multiplier per candidate.
It sees odd cores, the whole of what GF(2) misses: a core is dependent over GF(p) exactly when p divides every maximal
minor of its matrix, and the smallest odd core, {12, 18, 20, 30, 45}, has determinant 3.  The signed family over GF(3)
is complete: by Hershkowitz and Schneider, a pattern has full row rank for every choice of nonzero entries exactly when
it has a uniquely restricted matching of its rows, which for a set of moves is playability.

Section 7.2 measured the premise: the share of dependent heaviest sets rises with n (0% below 300, 49% at 300-599, 57%,
55% and 73% at 1100-1300).  The planned experiment was the decomposition bound (min over μ of the GF(2) heaviest set for
w − μ plus the GF(3) heaviest set for μ) at the roots of 880-2175, every 25th game, killed by a mean share of the root
gap closed under 20% or one falling with n as the cuts' did (−4 points per 100 n).  A share of 0.5 would have taken the
cost from ×10 per 380 n to ×10 per about 600 n.  The risks were multipliers that must be refitted at the nodes (×4.2 in
nodes with the node's multipliers, ×1.26 with the root's) and an LP over three or more matroid polytopes that is not
integral (a gap up to 2 for three).  The slope test in the search killed the avenue on 2026-10-16 (the agenda's
[A1](../../../docs/AGENDA.md#a1-a-bound-over-several-fields-and-signings-a-change-of-slope-if-its-share-holds)).

**A2. Certify the frame model, then prove with its facts** (a change of slope for the proven wall).  The assumptions of
the frame model become proven facts, so that the solver's chain is a proof on the whole board.  The first assumption is
the upper part, that every optimal game plays U\*: barring a member u of U\* in the parity bound proves that every
optimal set holds u when the bound falls below opt(n).  Section 7.3 certified 92-96% in about ten seconds a game, and
the open rest is open by margins up to the root's gap, even for the largest prime.  The plan: the barred bounds for
every game of 1301-2175, and the prover's search of five games it has proven (1302-2100) with the certified members
forced, killed if the dear games take fewer than half the nodes.  Done by 2026-10-15:

* all the assumptions are theorems (THEORY.md's Theorems B and M), and the upper part is reduced to U, its members of
  three or more prime factors ([the frame model on a proven
  footing](../../the-frame-model-on-a-proven-footing-2026-10/README.md));
* the barred bounds, with layers 1 and 2 fixed by Theorem B, for every game of 1301-3314 that rests on the model: 7.5%
  of U\*_{≥3} open to 2175 and 14.9% past it, no game closed by the bounds alone, the open share following the root's
  gap ([the upper part game by
  game](../../the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md));
* the search of the open members is the integrated search over the extended problem Π⁺(O), built and proven exact
  (Theorem M⁺; [the frame model by layer, and the extended
  problem](../../the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md)).

The risk is low for the certificates; a general proof that every optimal game plays U\* is the open problem of [the
prover's "What remains open"](../../../noframe/docs/before-and-after.md#what-remains-open) and the certificates do not
need it.

**A3. Split by the number of promotions** (a sound tightening).  Search each size k of a promotion set apart, bounding
every node by the heaviest common independent set of exactly the size its completion needs.  The sizes are a partition,
so the split needs no conjecture, and the premise held: the GF(2) heaviest set at the root has the optimum's size at
1,119 of the 1,121 roots measured (C4).  The kill was a drop that cuts under 10% of the open probe nodes, and it was
killed at the diagnostic, on the nodes of 936, 1215 and 2112 ([A3's
test](../split-by-the-number-of-promotions-a3s-test-2026-10/README.md)).  The root's window of winning sizes is one or
two (52; 69; 116-117) and all 812 nodes, every open one included, have a heaviest completion of the root's total size,
since the root's heaviest set is a largest common independent set, so no node lies outside the window and the exact-size
bound equals the plain bound.

**A4. What the cost per unit of gap is made of** (a diagnostic, then branching on cores).  The residuals of the cost
model run in blocks (2250-2275 four times dearer per unit of gap than 2196-2248).  If the size of the root's conflict
drove the cost per unit of gap, branching on whole cores (the victim law of THEORY.md's frontier argument) would have
fewer children than the members of a large conflict (12.9 members at the roots, +1.1 per 100 n).  The kill was nothing
explaining more than 30% of the residual variance, and it was killed on the 400 exhausted searches of 2176-3312 (7.4).
Nineteen features of the root were tried against the cost model's residual: the heaviest set's conflicts (number and
sizes), the cores it runs into (the frames holding one, their movers, the promoted members and factors inside), the
forced members at the root, the start incumbent's distance below the optimum, and the sizes (candidates, `max_total`,
the heaviest set's size, the freedoms, n).

* The best explains 13% of the residual's variance when the gap is scaled by n^0.3 (19% of the residual of the gap
  alone; the largest conflict, correlated negatively: the bigger it is, the fewer nodes).  None adds to the cost per
  unit of gap, and all of them together explain 38% in sample (47%) and nothing held out (R² −0.7 and −0.5, a stretch of
  40 searches predicted from the rest).
* The root's cores grow with the gap they make (movers in the cores of frames 1-3: 230 on average, +5 per 100 n, r =
  0.68 with G); the first conflict has 13.3 members and falls 0.7 per 100 n (the anatomy's +1.1 per 100 n at 880-1300
  does not continue); 13.5 conflicts are met before the heaviest set is promotable; and 161 of the 509 roots force
  members (3.3 on average, in the games of low gap).
* The runs of residuals are real (lag-1 autocorrelation +0.84, 60 runs of signs where independence gives 201, the means
  of 20 consecutive searches varying 3.4 times as much as independent games would) and nothing at the root explains
  them: the dear games are the games of high gap, and what the gap leaves is not in the root.

**A5. Decompose across the frames** (AND/OR search).  The frames form a path (a candidate of home h touches frames h
and h − 1 only), so fixing the decisions on the candidates of one home separates the frames in front from those behind,
and an AND/OR branch and bound would turn a product of the two sides' trees into a sum.  The pilot of [component
decomposition](../ideas-from-other-fields-the-pilots-2026-10/README.md#44-component-decomposition-killed) found one
component; this conditions on a separator instead.  It was killed at the measurement, on 412 open nodes of the proofs of
936, 1215, 2112 and 2170 ([A5's test](../decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md)).
Every conflict lies in two adjacent homes (none of 11,197 crosses a separator) and all of them in frames 1-3, so a home
always splits them, but it touches 8-10 of the heaviest set's 9-16 cores and 22-40 candidates (a quarter to a half of
the home's), and the bound's error sits at that home.  With the separator fixed as the best completion has it, the two
sides cost 0.01-2% of the subtree; the cost is the separator's enumeration, which the AND/OR search has to do too: its
OR part alone took 5.7 to 154 times the plain tree's steps at 936, 1215 and 2112, mostly without finishing.

## Files

* `README.md`: this record.  The measurements' data are in other entries' folders, named above; the code was scratch.

## Commits

* 55287ff: the code of the EC2 chain whose root gaps 7.4 measured.
* 8753189: the last commit that builds `examples/matroid.rs`, which 7.4 used.
* d2e12f6: the agenda's measurements move into the notebook as this entry.
* cd1ca81: the agenda's text of 2026-10-18 moves into this entry.
