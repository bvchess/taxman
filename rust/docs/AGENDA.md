# Research agenda (2026-10)

What to pursue to push the wall (the largest n for which an optimal game is proven) much further than 2175: where the
project stands, what limits it, the avenues ranked, the conjectures worth proving, the ideas worth borrowing, and what
the coming cloud runs should measure.  Written on 2026-10-14, after the rule of symmetric pairs was merged.  It links
to the measurements instead of repeating them; the ones made for it are in section 7.  On 2026-10-15 section 2 and the
results of A4, C5 and C6 were brought up to date with the measurement of the chain past 2175 (section 7.4).  The
register of [EXPERIMENTS.md](EXPERIMENTS.md#register-of-everything-tried) stays the record of what was tried; an
avenue started from here gets its row there when it ends.

* [1. Where we are](#1-where-we-are)
* [2. The diagnosis of the wall](#2-the-diagnosis-of-the-wall)
* [3. Ranked avenues](#3-ranked-avenues)
* [4. Conjectures](#4-conjectures)
* [5. Ideas from other domains](#5-ideas-from-other-domains)
* [6. What the cloud runs should measure](#6-what-the-cloud-runs-should-measure)
* [7. Measurements made for this agenda](#7-measurements-made-for-this-agenda)

## 1. Where we are

**Known.**  Every n ≤ 2175 has a game proven optimal within the frame model, a chain from n = 1
([results/README.md](../results/README.md)); the prover, which knows nothing of frames, proves every n ≤ 2175 on the
whole board, 1301-2175 in a cloud run of 226 thread-hours and $3.84
([prover 3](../prover/README.md#to-2175-on-a-cloud-machine)).
A 64-thread chain reached 2276 in the Graviton calibration (`bench/cloud/c8g.chain-2176.jsonl`, benchmark data, not
a committed result), and the EC2 chain, 64 threads on a `c8g.16xlarge`, 3314 in 6.4 hours (a snapshot of
2026-10-15, not committed; its root gaps are section 7.4).  The frame model's assumptions are validated, not proven
([README](../README.md#what-optimal-means)).  Outside the project the record stops at n = 1000 (OEIS A019312's table is
this project's), and the literature has bounds and one asymptotic result, not optima (section 5).

**The theory the search stands on.**  A set of moves is playable, independent over GF(2) in the parity matroids, or
matchable exactly when some payment set has one, an odd number, or at least one payment scheme
([THEORY.md, Theorem 12](../../heuristics/THEORY.md#one-count-three-conditions)).  The GF(2) bound of the search is the
frame model's form of that middle condition (Theorem 11), equal to the parity bound at every root measured, exact on
movers with at most two live factors, and blind only to *odd cores*: self-covering sets with an odd number of schemes,
which need movers with three or more distinct primes.  At the roots, what the bound gets wrong is which candidates,
never how many, through odd cores of frames 1-3 whose size grows with n
([anatomy](EXPERIMENTS.md#the-anatomy-of-the-gap-2026-10)).

**What drives the cost.**  A proof's nodes follow the root's gap (the bound less the optimum) and nothing else measured:
×10 per 236-266 of GF(2) gap past 1300 (section 2), as ×10 per 130 of Lagrangian gap did on the matching path
([slope](EXPERIMENTS.md#what-drives-the-slope-2026-10)); at a given gap a larger board has the smaller tree, the gap
counting in units of √n ([the gap past 2175](EXPERIMENTS.md#the-gap-past-2175-2026-10)).  A node costs +0.042
decades per 100 in n at the root of a tree and +0.02 averaged over a whole one, 98% of it the GF(2) intersection; the
fixed costs of a game are gone since round A; a proof
scales 58 times on 64 threads ([wall regime](EXPERIMENTS.md#the-wall-regime-2026-10),
[round A](EXPERIMENTS.md#round-a-the-fixed-costs-of-a-game-2026-10),
[Graviton calibration](EXPERIMENTS.md#graviton-calibration-2026-10)).

**What has been tried** (the register has every row; by family):

* *Bounds on top of GF(2)*: cover cuts and the Lagrangian over nogoods (46% of the root gap by exact separation),
  rank cuts (54%), the per-frame exact bound (59% within 200 steps, at 300 to 3,000 times the cost), Lorea's matroid
  (nothing), the Lagrangian of the symmetric pairs (nothing).  Every share closed falls with n, and in a search each
  was a constant factor ([anatomy](EXPERIMENTS.md#the-anatomy-of-the-gap-2026-10),
  [per-frame](EXPERIMENTS.md#the-per-frame-bound-2026-10), [nogoods](EXPERIMENTS.md#how-far-can-nogoods-go-2026-10)).
* *Branching and learning*: strong branching, forced members, nogoods, the rule of symmetric pairs (×0.8 of the
  cycles); dominance by exchange and transpositions (none to speak of).  Constant factors, all taken.
* *Reuse along the chain*: the ceiling, the prime formula and same search are in; carrying nogoods, a chain that
  comes back down, ceilings from neighbouring problems and equivalent problems gave nothing more
  ([going higher](EXPERIMENTS.md#going-higher-and-coming-back-down-2026-10)).
* *The node and the engine*: the inner-loop round (×4.4 cycles), set widths, PGO, round A.  Constant factors.

The one change of slope in the project's history is the GF(2) bound itself (the plain branch and bound's log10 nodes
+0.14 per 100 in n against the matching path's +0.48).  Everything since has moved the curve down, not bent it.

## 2. The diagnosis of the wall

**The cost model.**  Over the 314 exhausted searches of 1301-2175 in `results/strict.jsonl` (8 threads, the build of
feba6d0), against the root GF(2) gaps of `bench/gf2/roots.jsonl` (section 7.1):

    log10 nodes ≈ 3.24 + G / 255        (G the root's GF(2) gap; R² 0.50, residual sd 0.46 decades)

The build that now runs the chain makes trees 0.29 times as large (the same 391 games solved again on one thread: 0.27
at 1301-1500, 0.41 near 2175; 0.44 for the 39 searches of 2176-2276 against the Graviton calibration's chain; section
7.4), so the model to project with is refitted on that build, over 1301-3312 (714 exhausted searches: the games of
1301-2175 solved again, and the EC2 chain's 400 of 2176-3312 with their nodes less 0.06 decades for the window and the
threads):

    log10 nodes ≈ 2.67 + G / 247                (R² 0.85, residual sd 0.47)
    log10 nodes ≈ 2.30 + 0.0479 G / n^0.3       (R² 0.875, residual sd 0.44)

The slope is the old one, a decade of nodes per 250 of gap (221 ± 10 over 1301-2175, 266 ± 7 over 2176-3312).  What the
new data add is a dependence on n that the old model could not see: at a given gap a larger board has the smaller tree
(the second model: a decade per 230 of gap at n = 3000, 251 at 4000, 284 at 6000), which G alone reads as an intercept
that falls about 0.4 to 0.5 decades per 1,000 n.  The first edition of this section kept G alone because n and G/n
extrapolated worse past 2175, against the committed chain's trees; on one build that is reversed: G alone overpredicts
every later stretch by 0.2 to 0.6 decades, and the gap scaled by n^p with p = 0.3 predicts best (worst bias 0.27 over
five origins).  But the two chains disagree about p (0 to 0.2 across 2175, 0.4 to 0.5 within 2176-3312), so G alone
is the conservative end, p = 0.5 the optimistic one and 0.3 the centre of every projection.  The gap spans 125 to
1,443, four decades of tree; the node's cost moves 0.02 decades per 100 n averaged over a whole tree (0.042 at its
root).  The gap is the cost.

**Past 2175 the trees still follow their gaps, and the gaps rise.**  The wall regime found the dear games of the c8g
chain 4 to 18 times larger than their gaps predict; those games were chosen for their trees.  With the root gaps of all
509 searched games of 2176-3312 (section 7.4) the dear games are the games of high gap: the 47 searches of gap 1,100 or
more hold 77% of the chain's nodes, and against the scaled gap the 18 dearest are within +0.6 decades of the model (the
most in excess 2346, 3.9 times, and 3300, 3.3 times; the residuals' sd is 0.48).  The level of the gap is new: about 450
at 2100, 650-1,000 by 2400, then waves of a hundred games and more (mean 530 at 2601-2700, 1,130 at 3101-3200).  The
residuals run (lag-1 autocorrelation +0.84, consecutive boards being nearly one problem) and nothing at the root
explains them (A4, section 3): conjecture C6 is refuted as stated.

**How fast the gap grows decides the wall.**  Over 1301-2175 the root gap's trend was +6.9 ± 1.7 per 100 n
([GF(2) search](EXPERIMENTS.md#the-gf2-bound-in-the-search-2026-10)), a flat stretch below 2150 (+3.5 ± 2.4); the gap
over the unseeded start rises +57 per 100 n to 4095, an upper bound (that start is about 770 below the optimum at 3100,
against 0 to 403 where it had been measured).  The true gaps of 2176-3312 grow +37 per 100 n over the whole 1301-3312
(95%: 22 to 50 by blocks of 100 games, 6 to 63 by blocks of 200) and +25 over 2176-3312 alone (−43 to +83: the waves
are longer than a window of 1,100 n); G/n stays at 300.  The regimes, with the node's +0.042 of the first edition, and
the measured one with the node's +0.02 of a whole tree and the cost model above:

| gap trend | decades per 100 n | cost ×10 per | what 100 times the compute buys |
|---|---|---|---|
| +7 per 100 n (1301-2175) | 0.07 | about 1,450 n | about 2,900 n |
| +57 per 100 n (unseeded starts to 4095) | 0.27 | about 380 n | about 750 n |
| +25 to +37 per 100 n (measured, 2176-3312 and 1301-3312), gap scaled by n^0.3 | 0.08 to 0.095 | about 1,050-1,250 n (830 to 1,500 for p = 0 to 0.5) | about 2,100-2,500 n |

The steeper regime is refuted: the gap does not grow +57 per 100 n (4% of the draws by blocks of 200 games), and the
cost of a game grows ×10 per 1,050-1,250 n, not per 380.  (That the chain reached 3314 in 6.4 hours is not by itself
the evidence: its cost sits in the crests of the gap, an hour or two of the 64 threads for a hundred games, and the
valleys cost seconds.)  The wall of the present search on one `c8g.16xlarge`: a day reaches about 4,100 (3,700 to 4,850
across the models, −400 to +700 for a single run's luck), 4000 takes 13 to 20 hours past 3313, 5000 four to ten days,
6000 a month or more; a `c8g.48xlarge` (1.9 to 2.7 times as fast) adds 300 to 500 n to a day.  The numbers and their
derivation are in the entry [The gap past 2175](EXPERIMENTS.md#the-gap-past-2175-2026-10).

**Where an order of magnitude can come from.**

* *The bound.*  The cost is 10^(G/250), so a bound that closes a share s of the gap at every node multiplies the slope's
  gap term by 1 − s: a change of slope, the only one in reach.  Closing a fixed amount is a constant factor (×10 per
  250).  Every relaxation tried on top of GF(2) closed a share that falls with n (section 1); the residual is odd cores.
* *The cost per unit of gap* (1/250 decades).  Set by branching and learning; every change measured moved it by a
  constant, and it does not rise with n: it falls (a decade per 230 of gap at 3000, 284 at 6000).  Its runs past 2175
  are not explained by the root (A4).
* *The node, the engine, the fixed costs.*  Constant factors: the intersection is 98% of a node, parallel scaling is
  58 times at 64 threads.  Needed, but they shift the wall by a few hundred, not by thousands.
* *The frame model, for the proven wall.*  The solver's results rest on the model; the prover's check does not, but
  its nodes per solver node grow threefold per 1000 in n (2.5 at 1301-1499, 6.1 at 2060-2169) and its node costs 9 to
  17 times the solver's (17 at 990-1300; 4.1 ms against 0.46 near 2170, both on Graviton4;
  [the prover's proofs of 1301-2175](EXPERIMENTS.md#the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10)),
  against the committed chain's nodes; the present build's trees are 0.29 times as large, so against them the ratios
  are 3.4 times higher.  At that trend the check costs about 15 times the solver's nodes near 3000 (50 against the
  present build's), the proofs of 1301-2175 having taken 226 thread-hours: the wall of *verified* results will be the
  prover's, not the solver's, unless the model is proven or the prover gets the model's reductions as proven facts
  (avenue A2).

## 3. Ranked avenues

Ranked by expected gain to the proven wall over cost.  Each says what must be proven before it can prune: every rule
the search relies on is proven and tested, as the project requires.

### A1. A bound over several fields and signings (a change of slope, if its share holds)

*Idea.*  A playable set's payment matrix is unit triangular after its play order, so its rows are independent over
*every* field and for *every* choice of nonzero entries
([THEORY.md](../../heuristics/THEORY.md#what-the-parity-bound-keeps-and-what-it-drops), the end of Theorem 12's
discussion).  Each choice (a field, and signs or values on the edges) gives a pair of frame
matroids, even and odd, in which every promotion set is independent.  GF(2) with 0/1 entries is one pair; GF(3) with 0/1
entries and GF(3) with signs are others.  Alone, each is weaker than GF(2): with 0/1 entries GF(3) misses the triangle
{6, 10, 15} (determinant ±2), and with signs it sees only the orientable closed surfaces, where GF(2) sees all, which is
why [the matroid study](EXPERIMENTS.md#matroid-intersection-bounds-2026-10) called GF(2) the field.  *Together* they are
stronger than either: the LP over the intersection of the four matroid polytopes is a valid
bound at least as tight as GF(2)'s, computable in polynomial time by Lagrangian decomposition into two weighted matroid
intersections, one per field, with one multiplier per candidate.  The study's objection, that three matroids cannot be
intersected in polynomial time, holds for the integer problem, not for this bound.

*Why it might work.*  It sees odd cores, the whole of what GF(2) misses: a core is dependent over GF(p) exactly when p
divides every maximal minor of its matrix, and the smallest odd core, {12, 18, 20, 30, 45}, has determinant 3.  And the
signed family over GF(3) is complete: by Hershkowitz and Schneider (section 5), over any field of three or more
elements a pattern has full row rank for *every* choice of nonzero entries exactly when it has a uniquely restricted
matching of its rows, which for a set of moves is playability (Theorem 12).  The proof is constructive and fits a
search: a self-covering set (every factor listed twice or more) becomes dependent, with all coefficients 1, under any
signing whose sums vanish column by column over GF(3), which is possible for every column listed at least twice
(1 + 2, 1 + 1 + 1, ...).  So every core the search meets yields a signing whose matroids see it, and all its other
rank inequalities come with it (GF(2) is the exception: a column listed three times cannot vanish there, which is
exactly how odd cores escape).  Measured
(section 7.2): at the roots of the 1,055 games of 78-1300 whose parity bound exceeds opt(n), the bound's heaviest set is
dependent over GF(3), GF(5) or GF(7), with 0/1 entries or Koszul signs, in 566 (54%), and the share *rises* with n: 0%
below 300, 49% at 300-599, 57%, 55%, 73% at 1100-1300, as the residuals grow.  Every cut family tried so far closed a
share that fell with n; this is the first relaxation whose reach grows with the cores.  It is also a whole family, all
rank inequalities of the other field's matroids at once, which the anatomy's small unions of cuts could not reach.

*Cheapest experiment.*  At the roots of the frame model, 880-2175 every 25th game (`bench/gf2/roots.jsonl` has the
GF(2) bounds and optima), the decomposition bound min over μ of [heaviest GF(2) common independent set for w − μ] +
[heaviest GF(3) common independent set for μ], μ by a bundle or volume method from the GF(2) weight split (the
subgradient stalled in the per-frame study), for GF(3) plain, GF(3) Koszul, a few random signings, and the signings
built from the cores of the GF(2) heaviest sets met on the way (a matroid added per core, as a cut would be).  Code: the
frame matroids of `examples/matroid.rs` and a GF(3) twin of `examples/common/meet.rs` (two bits an entry); research code
in an example, as the matroid study was.  *Metric:* the share of the root GF(2) gap closed, by band, and its trend in n;
then at the 600 probe nodes (`testdata/perf`), the share of the open nodes it cuts.  *Kill:* a mean share under 20%, or
one that falls with n as the cuts' did (−4 points per 100 n).  A pilot on the whole board first is cheaper still: the
prover's `parity.rs` with a GF(3) twin, on 1000-1300, where section 7.2's sets come from.

*Payoff.*  A share s that holds with n lowers the gap term of the slope by s.  At s = 0.5 and the steeper gap trend, the
cost goes from ×10 per 380 n to ×10 per about 600 n.  Below that, a constant factor (×10 per 255 of gap closed).

*Risk.*  The bound may move little where it bites: excluding one heaviest set leaves others nearly as heavy with other
cores (the anatomy's lesson), and 54% of the square residuals have determinant ±1 under both entries tried (section
7.2), invisible to every prime field with those entries.  At nodes the multipliers must be refitted to pay (the cuts
study: ×4.2 in nodes with the node's multipliers, ×1.26 with the root's), and a GF(3) level kept along the path doubles
the node's work.  Random signings see small cores often and large ones rarely (the chance that a column listed k times
sums to zero is 1/2 for k = 2 and 1/4 for k = 3), so the signings built from the cores met are what scales, at one
matroid pair each.  The LP over three or more matroid polytopes is not integral (its gap can reach 2 for three, by
Linhares, Olver, Swamy and Zenklusen), so the bound lies between GF(2)'s and the integer optimum over all the
matroids, not at the latter.

*Proven first.*  The lemma over any field and any nonzero entries, frame by frame (Theorem 11's proof with "nonzero" for
"1"; the unit-triangular argument is already in THEORY.md), and weak duality for the decomposition (as for the weight
split).  Both short.  For THEORY.md, with proof in this setting: Hershkowitz and Schneider's theorem (playable exactly
when independent under every GF(3) signing) and the column-sum signing of a self-covering set.

### A2. Certify the frame model, then prove with its facts (a change of slope for the proven wall)

*Idea.*  Turn the model's assumptions into proven facts, in general or game by game, so that the solver's chain is a
proof on the whole board and the prover need not repeat the model's work by search.  The first assumption is the upper
part: every optimal game plays U\*.  Barring a member u of U\* in the parity bound (Theorem 10 allows barred numbers)
proves that every optimal set holds u when the bound falls below opt(n); with U\* maximal, every optimal set then has
upper part U\*.

*Why it might work.*  Measured (section 7.3): one barred parity bound each certifies 207 of the 224 members of U\* at
600, 339 of 377 at 1000 and 472 of 490 at 1300 (92-96%), in about ten seconds a game.  The rest are open by margins up
to the root's gap, and some are settled by theorems already (the largest prime is in every optimal set, THEORY.md's
lemma, yet its barred bound is open at 1000 and 1300).  The prover then searches with the certified members forced,
and its tree should lose the part spent refuting sacrifices of U\*, which grows with n.

*Cheapest experiment.*  (1) The barred bounds for every game of 1301-2175 at the roots (seconds to a minute a game on
one thread; the prover's `Meet::solve` with `Role::Barred`): the share of U\* certified and its trend.  (2) The prover's
search of five games it has proven (1302-2100, from the cloud run's data) with the certified members forced
(`Role::Pick`) and the open ones free: nodes against the unforced proof.  *Kill:* fewer than 2 times fewer nodes on the
dear games.  (3) The remaining assumptions one at a time (the prover README's open list says they are tested only end
to end): for each, a statement and either a proof or a counterexample on boards to 2000.

*Payoff.*  The verified wall moves with the solver's instead of falling behind tenfold per 1000 n; with a general proof
of the model the prover becomes a cross-check that can lag.

*Risk.*  Low for (1) and (2), which are certificates.  A general proof of "every optimal game plays U\*" is the open
problem of [prover 8](../prover/README.md#8-what-remains-open) and may be hard; the game-by-game certificate does not
need it.

*Proven first.*  Nothing new for barring (Theorem 10 with barred numbers); the forced members in the prover are sound
because they are certified.

### A3. Split by the number of promotions (a sound tightening; constant factor, maybe more)

*Idea.*  Every promotion set has some size k.  Search each k that can matter apart, bounding every node by the
heaviest common independent set of *exactly* the size its completion needs: the intersection computes the heaviest set
of every size on its way (the dominant sets the GF(2) levels already keep).  The sizes are a partition, so this is
sound without any conjecture, and the root's bounds of each size prune most k at once.

*Why it might work.*  The GF(2) heaviest set at the root has the optimum's size at 1,119 of the 1,121 roots measured
(C4), and the optimal sets of every game enumerated have one size
([prover 5](../prover/README.md#5-is-the-optimal-set-unique)).  So only one or two k survive at the root, and at a node
whose heaviest completion has the wrong size the exact-size bound is lower.  Unlike `revise_freedoms`, it assumes
nothing about which size the optimum has.

*Cheapest experiment.*  At the root and the probe's 600 nodes: the heaviest common independent set of each size, the
k that survive against the optimum, and at the nodes the drop from the unconstrained bound to the exact-size bound of
the surviving k.  *Kill:* a drop that cuts under 10% of the open probe nodes.

*Payoff.*  A constant factor if the drop is a fixed amount; more if wrong-size completions are what deep nodes are
made of.  *Risk:* low; nodes may already have the right size.  *Proven first:* nothing beyond the partition.

### A4. What the cost per unit of gap is made of (a diagnostic, then branching on cores)

*Idea.*  The residuals of the cost model run in blocks (2250-2275 four times dearer per unit of gap than 2196-2248).
Find what predicts them at the root: the number of disjoint odd cores, the size of the heaviest set's conflict (12.9
members, +1.1 per 100 n), the frames holding cores, the forced members at the root.  If the conflict's size drives the
cost per unit of gap, branch on whole cores: the *victim law* (a single sacrifice repairs a core exactly when it lies in
every minimal core, THEORY.md's frontier argument) gives a branching on which minimal core breaks where, with fewer
children than the members of a large conflict.

*Cheapest experiment.*  From existing results: the root's anatomy (cores per frame, conflict size) for the 39 searches
of 2176-2276 and 100 of 1301-2175, against their residuals from section 2.  An hour of one thread.  *Kill:* nothing
explains more than 30% of the residual variance.

*Payoff.*  A model of c that extrapolates, and possibly a branching that lowers it.  *Risk:* the runs may be noise of
the incumbent's history.  *Proven first:* the branching's partition (as for conflicts).

*Result* (the 400 exhausted searches of 2176-3312, section 7.4): killed.  Nineteen features of the root were tried
against the cost model's residual: the heaviest set's conflicts (their number, the first one's size, the mean, the
largest, the members in all), the cores it runs into (the frames holding one, their movers, the movers with three or
more live factors, the promoted members and factors inside), the forced members at the root, the start incumbent's
distance below the optimum, and the sizes (candidates, `max_total`, the heaviest set's size, the freedoms, n).  The best
explains 13% of the residual's variance when the gap is scaled by n^0.3 (19% of the residual of the gap alone; the
largest conflict, correlated negatively: the bigger it is, the fewer nodes), none adds to the cost per unit of gap (the
feature times G), and all of them together explain 38% in sample (47%) and nothing held out (R² −0.7 and −0.5, a
stretch of 40 searches predicted from the rest).  The root's cores grow with the gap they make (movers in the cores of
frames 1-3: 230 on average, +5 per 100 n, r = 0.68 with G), the first conflict has 13.3 members and falls 0.7 per 100 n
(the anatomy's +1.1 per 100 n at 880-1300 does not continue), 13.5 conflicts are met before the heaviest set is
promotable, and 161 of the 509 roots force members (3.3 on average, in the games of low gap).  The runs of residuals
are real (lag-1 autocorrelation +0.84, 60 runs of signs where independence gives 201, the means of 20 consecutive
searches varying 3.4 times as much as independent games would) and nothing at the root explains them: the dear games
are the games of high gap, and what the gap leaves is not in the root.

### A5. Decompose across the frames (AND/OR search)

*Idea.*  The frames form a path (a candidate of home h touches frames h and h − 1 only), so fixing the decisions on the
candidates of one home separates the frames in front from those behind, and the best completion is the sum of the two
sides' bests.  An AND/OR branch and bound (branch on a separator home, then solve the two sides independently, caching
nothing across) turns a product of the two sides' trees into a sum.  The component decomposition of IDEAS.md looked for
disconnected parts and found one component; this conditions on a separator instead.  `max_total` couples all frames, but
it never binds and can be dualized.

*Cheapest experiment.*  At the probe's nodes and along the proofs of 1215 and 2170: how the GF(2) gap of a node splits
by frame (the anatomy did it at roots: homes 1-3), and whether the subtrees under a node resolve the cores of frame 1
and of frame 3 independently (the tree's size against the product of the per-frame resolutions).  *Kill:* the cores
sit in adjacent frames (homes 1 and 2), as at the roots of 1251-1300, so that no separator splits them.

*Payoff.*  A change of slope where the gap is spread over separable frames; nothing where it is concentrated.
*Risk:* high, given the anatomy's homes.  *Proven first:* the decomposition (the frame-by-frame promotion criterion).

### A6. The prover's node (a constant factor that keeps verification in pace)

A node of the prover costs 9 to 17 times the solver's (section 2) because it solves the parity matroids at each node
warm-started from its parent, where the solver keeps contracted matroids by pivots along the path
([GF(2) search](EXPERIMENTS.md#the-gf2-bound-in-the-search-2026-10), item 5).  Reimplementing the technique in the
prover (not sharing code, which is the point of the prover) would give it most of that factor, and the bound without a
member read off the exchange graph would cut its strong branching.  *Experiment:* the prover's node profile on 1300 and
2100.  *Payoff:* about ten times, a constant, which is what A2's remaining searches will need near 3000.

### A7. The engineering the wall will need (constant factors, in this order)

* 192 threads: the tree's growth with workers (10-20% at 32-128 emulated), the nogood period at 192 (IDEAS.md,
  section 3), checkpoints at 300 s.  Measured on the cloud machine, not designed further here.
* The intersection's shortest paths (69% of a node at 4000): the GF(2) inner loop has left the solves of strong
  branching mostly certified; what remains is the augmentations of a level, a few dozen a node.
* Boards past 4095: a width of 128, `MAX_FRAMES` 9, the GF(2) vectors to 5 words
  ([wall regime, 4](EXPERIMENTS.md#the-wall-regime-2026-10)).  Built: the boards go to 8191
  ([boards past 4095](EXPERIMENTS.md#boards-past-4095-2026-10)).
* Proving runs of neighbours that share a gap in one search (19.8% of the proof nodes of 880-1300; the register's
  open row): a constant, and the run structure of section 2 says where it would pay.

## 4. Conjectures

**C1 (fields).**  The LP over the frame matroids of GF(2) and of a few GF(3) signings (0/1 entries, Koszul signs, and
the signings built from the cores met) closes a share of the root GF(2) gap that does not fall with n.  *Evidence:*
the heaviest GF(2) set is dependent in another field or signing at 49-73% of the roots of 300-1300, rising (section
7.2); the signed GF(3) family is complete (Hershkowitz and Schneider, section 5).  *Test:* A1's root experiment.

**C2 (sign coherence of three-prime cores).**  On numbers p^a q^b r^c of three primes, movers of degree d + 1 over
factors of degree d form the honeycomb graph of a triangular region, and a set's schemes onto one payment set are
its lozenge tilings.  Conjecture: in a region without holes all of a core's schemes have the same sign, so its
determinant is ± its number of schemes, and GF(p) sees it exactly when p divides that number.  The smallest odd core,
{12, 18, 20, 30, 45} on the primes 2, 3, 5, has three schemes, all of one sign: determinant 3.  This would be the
combinatorics of the weak Lefschetz property of monomial ideals in characteristic p, where the multiplication map's
determinant counts lozenge tilings (section 5; to check).  *Why it matters:* it would say which fields to add for
which cores, and why GF(3) sees so many.  *Test:* enumerate the minimal odd cores of three primes to 8 members
(`heuristics/evaluation/fm_relation.py`'s `odd_cores`, restricted) and compare |determinant| with the count of
schemes; then regions with holes.

**C3 (the upper part).**  Every optimal game plays U\*, and game by game this is certified by barred parity bounds and
small searches.  *Evidence:* every game enumerated ([prover 5](../prover/README.md#5-is-the-optimal-set-unique)); 92-96%
of U\* certified at the root by one bound each (section 7.3).  *Attack:* the frontier argument of
[prover 8](../prover/README.md#8-what-remains-open) for a proof; A2 for certificates.

**C4 (one size, near the relaxation's).**  All optimal sets of a game have one size, and it is within one of the size
of the GF(2) bound's heaviest set at the root.  *Evidence:* every enumerated game's optimal sets have one size
([prover 5](../prover/README.md#5-is-the-optimal-set-unique)); the heaviest set has the optimum's size at all 361 roots
of 880-1300 ([anatomy](EXPERIMENTS.md#the-anatomy-of-the-gap-2026-10), 1) and at 758 of the 760 searched and repeated
games of 1301-2175 (`bench/gf2/roots.jsonl`'s `size` against `strict.jsonl`'s promotions); at 1320 and 1322 it has 77
members and the optimum 78, which is `max_total` (a tie of weights, or a heavier set of the wrong size: not known).  So
the strict form, the same size, is false or rests on ties.  *Why it matters:* with it, A3 searches two values of k at
most.  Not needed for A3's soundness.  *Test:* the heaviest set of each size at the roots of 1320 and 1322.

**C5 (the gap law).**  The true root gap of the searched games grows linearly in n past 2175, at a rate between the
two regimes of section 2; or, in the form the fit of 1301-2175 prefers, G/n stays bounded.  *Test:* section 6's root
gaps of the chain past 2175.  Everything about where the wall is rests on it.  *Result* (the EC2 chain's 509
searched games of 2176-3312, section 7.4): both forms hold, and neither regime.  The gap is 341 to 1,443 (mean 818,
against 391 at 1301-2175), risen from about 450 at 2100 to 650 at 2160-2200 and then in waves of a hundred games and
more (mean 530 at 2601-2700, 1,130 at 3101-3200).  Its slope is +37 per 100 n over 1301-3312 (95%: 22 to 50 with blocks
of 100 games, 6 to 63 with blocks of 200) and +25 over 2176-3312 alone (−43 to +83: a window of 1,100 n holds too few
waves to fix a slope); G/n stays at 300 (224 at 1301-2175), drifting down 3 per 100 n.  So +7 was a flat stretch below
2150, and +57, the unseeded start's upper bound, is not the gap's growth (4% of the draws by blocks of 200).  The waves
are partly arithmetic (counts of numbers of three or more distinct primes by tier predict a third of their variance out
of sample).

**C6 (runs).**  The cost per unit of gap is a function of the root's core structure (the number and size of the odd
cores, the frames holding them), which changes in blocks of n.  *Test:* A4.  *Result:* refuted as stated.  The cost's
residuals do run (lag-1 autocorrelation +0.84: consecutive boards pose nearly one problem), but what runs is the gap, in
waves, and the root's cores, conflicts and forced members add nothing to it (A4's result).  The runs of dear games are
runs of high gaps: the 47 searches of gap 1,100 or more (12% of the exhausted ones) hold 77% of the chain's nodes,
the 11 of 1,300 or more 53%, and all of the 18 dearest have a gap of 1,094 or more.  What stays unexplained is a
factor of 3 (sd 0.48 decades) in a game's tree at its gap, with 2346 the largest excess among the dear games (3.9
times the model, 3300 3.3 times).

**C7 (♦).**  Every rejection of descending greedy on an upper-half instance is a GF(2) dependency of what it kept
above; it would prove the upper-half conjecture in general.  Certified to 100,000 game by game; open
([THEORY.md](../../heuristics/THEORY.md#the-open-problem)).  Not on the critical path of the wall; C3 is.

## 5. Ideas from other domains

Each with the piece that transfers.  The sources were surveyed for this agenda; "read" marks those read in full or in
the relevant pages, the others were seen in abstracts.

* **Matrix theory: zero patterns over fields of three or more elements.**  D. Hershkowitz and H. Schneider, "Ranks of
  zero patterns and sign patterns", Linear and Multilinear Algebra 34 (1993) 3-19 (read): over a field of at least three
  elements, an r × n zero-nonzero pattern has independent rows for every choice of nonzero entries exactly when its
  bipartite graph has a *constrained* r-matching (the only one between its vertex sets: a uniquely restricted
  matching), and their Example 3.6 shows it fails over GF(2).  With Theorem 12, a set of moves is playable exactly when
  it is independent under every GF(3) signing.  *Transfers:* the completeness of the signed GF(3) family, and its
  constructive proof (column sums made to vanish), which turns each core the search meets into a matroid that sees it
  (A1).  The minimum-rank literature (zero forcing bounds the maximum nullity over every field; minimum rank depends on
  the field for some graphs) is the general setting.
* **Integer linear algebra.**  A set's rows are independent over GF(p) exactly when p does not divide the gcd of its
  maximal minors, the last determinantal divisor of its Smith normal form (R. P. Stanley, "Smith normal form in
  combinatorics", JCTA 144 (2016), arXiv:1602.00166).  *Transfers:* which fixed fields to add (section 7.2's census of
  determinants), and the fact that cores whose divisor is 1 escape every fixed-entry field, so signings are needed.
* **Coding theory, the erasure channel.**  The cores are *stopping sets* of a parity-check matrix whose variable nodes
  are movers and check nodes factors; the even cores are supports of codewords; peeling is the erasure decoder.
  Stopping sets that contain no codeword support, the odd cores, have a name: *coverable* stopping sets (Yakimenka,
  Skachek, Bocharova, Kudryashov, "Stopping redundancy hierarchy beyond the minimum distance", arXiv:1804.06770,
  read in part), the ones redundant checks are added to remove (Schwartz and Vardy, IEEE Trans. Inf. Theory 52 (2006)).
  Small stopping sets are enumerated by tree search (Rosnes and Ytrehus, IEEE Trans. Inf. Theory 55 (2009); Wang,
  Kulkarni and Poor, IEEE Trans. Inf. Theory 55 (2009), all sets to size 13 at about 500 variables) and by integer
  programming (Velasquez, Subramani and Drager, ISCO 2018, to size 48), and LP decoding on the erasure channel succeeds
  exactly when peeling does (Feldman, Wainwright, Karger, IEEE Trans. Inf. Theory 51 (2005)).  *Transfers:* the
  enumerators, to list a frame's odd cores for A4's census and for the signings of A1; and the mirror image of A1:
  there the code is fixed and the decoder gains checks, here the game is fixed and the relaxation gains fields.
* **Random XORSAT and hypergraph 2-cores.**  For random 3-XORSAT the 2-core appears at density about 0.818 and full
  rank lasts to about 0.918 (Dubois and Mandler, FOCS 2002; Pittel and Sorkin, CPC 25 (2016)): in between, a non-empty
  core of full GF(2) rank, an odd core, is the typical case.  *Transfers:* a density-evolution model of the frames
  (movers per factor, the share with three live factors) to predict how the cores, and so the gap, grow with n (C5);
  and the lesson that the odd-core gap is generic, not an accident of a few boards.
* **Uniquely restricted matchings.**  The maximum uniquely restricted matching is NP-hard on bipartite graphs
  (Golumbic, Hirst, Lewenstein, Algorithmica 31 (2001); Müller, Order 7 (1990)), not approximable within n^(1/3 − ε)
  unless NP = ZPP (Mishra, 2011), and fixed-parameter tractable in treewidth in 2^O(tw²) n (Chaudhary, Sau, Zehavi,
  arXiv:2508.12004, 2025).  No integer-programming formulation or polyhedral study of it was found.  *Transfers:* a
  good polynomial bound must use the arithmetic, as GF(2) does; the core inequalities of A1 and the anatomy are new
  ground; the treewidth recursion could close small residual components exactly after branching.
* **Algebraic combinatorics.**  The weak Lefschetz property of monomial ideals in three variables: the multiplication
  map between degrees has a determinant that counts lozenge tilings, and fails in characteristic p when p divides the
  count.  *Transfers:* product formulas for those counts, to predict which primes see the three-prime cores (C2).
  (Not surveyed; the statement to check before relying on it.)
* **Matroid optimization.**  Lagrangian decomposition (Guignard and Kim, Math. Programming 39 (1987)) reaches the LP
  over the intersection of the matroid polytopes with two-matroid subproblems; for three matroids that LP's integrality
  gap is 2 (Linhares, Olver, Swamy, Zenklusen, Math. Programming 2020), so it is a bound, not the optimum.  The k best
  common independent sets can be ranked (Camerini and Hamacher, SIAM J. Discrete Math. 2 (1989)), and intersection
  under small changes of the matroids has fast algorithms (Blikstad, Mukhopadhyay, Nanongkai, Tu, STOC 2023).
  *Transfers:* the decomposition for A1, with a volume or bundle method for the multipliers (the plain subgradient
  stalled in the per-frame study); ranking for A3's sizes and for incumbents.
* **AND/OR search** (graphical models).  Branch on a separator, solve the sides independently.  *Transfers:* the frame
  path's homes as separators (A5).
* **Proof logging.**  Gaussian elimination certified in pseudo-Boolean proofs (Gocht and Nordström, AAAI 2021), branch
  and bound certified by aggregation and disjunction steps (VIPR: Cheung, Gleixner, Steffy, IPCO 2017), dominance and
  symmetry breaking certified (Bogaerts, Gocht, McCreesh, Nordström).  No certificate format for matroid-intersection
  duality exists; Frank's weight split with greedy-optimality certificates per matroid is the natural one, checkable
  with XORs and comparisons.  *Transfers:* a log of the solver's tree with each leaf's certificate (a weight split, a
  conflict, a symmetric pair), checked by a small program, would make the code's correctness checkable without the
  prover.  It does not touch the frame model's assumptions, which is why A2 ranks above it.
* **Number theory: the asymptotic constant.**  D. Hensley, "A winning strategy at Taxman", Fibonacci Quarterly 26
  (1988) 262-270 (read): opt(n) / (n²/2) tends to a constant C with 1/2 < C ≤ 35/48, so the taxman keeps a fixed share
  of the pot; and for every prime p, L_p ≤ C ≤ L_p + 2/p, where L_p is a sum over the p-smooth numbers j of the optimal
  scores of the games on the p-smooth numbers up to j.  Nothing later improves it.  Franklín and Moniot (read) compute
  no optimum themselves (their data are this project's, to 1000), prove that their born-free matching takes at least
  1724/3375 of the pot asymptotically, and pose no open problems; nothing cites them yet.  *Transfers:* a new use for
  exact solvers, proven bounds on C from games on smooth boards (about a thousand 7-smooth numbers below 10^6), a
  result in its own right and a check of the asymptotic regime; and Hensley's blocks {kd : d p-smooth} (play in block k
  never disturbs a larger block), a decomposition of the board by the rough part of each number.  For the gap's growth,
  Landau's count of numbers with k prime factors, x (log log x)^(k−1) / ((k − 1)! log x), says how fast the movers with
  three distinct primes, where all odd cores live, thicken in frames 1-3.

## 6. What the cloud runs should measure

The chain past 2175 and the prover's proofs of 1301-2175 decide C5 and A2's premise.  Most of what follows needs only
their output files, analysed afterwards on one thread; what needs the machine is marked.

* **The true root gap of every searched game** past 2175, from the chain's own optima (`examples/wall.rs nodes --nodes
  1` over the results file, as in section 7.1): the gap's trend (C5, the two regimes of section 2) and the residuals
  of the cost model, with the thread count in the fit.  Done for a snapshot of the EC2 chain at game 3314 (509
  searched games, section 7.4, [EXPERIMENTS](EXPERIMENTS.md#the-gap-past-2175-2026-10)); it can be repeated on a later
  snapshot with `bench/wall/gap.py`, to see whether the waves keep their level.
* **The tree's growth with threads** (on the machine): the same three dear games at 1, 16, 64 and 192 threads, for
  converting 192-thread trees to one-thread equivalents in that fit.  Measured for 2170 and 2172 (1 to 192 threads)
  and 2184 (16 to 192): the trees differ by up to 18% with no trend in the threads
  ([EXPERIMENTS](EXPERIMENTS.md#scaling-to-192-threads-2026-10)); proofs of a second or ten, not dear games.  The
  window's speculation in the chain's own nodes, game by game against the scaling run's, is −0.04 ± 0.03 decades for a
  window of 1 or 4 (−0.11 over 2226-2276, where the dear games lay), and the threads' −0.02: the fit takes them off.
* **The prover's run**: per game, its nodes against the solver's (the ratio's trend past 2000: done, threefold per 1000
  in n, [EXPERIMENTS](EXPERIMENTS.md#the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10)), and where its
  tree branches (members above n/2 against below), which is A2's premise: the share of its nodes spent on U\*.  The
  run's lines hold totals only, so the second needs a rerun that counts them.
* **The scaling at 192** (on the machine): effective parallelism, idle and lock time on a dear game, the nogood
  period (IDEAS.md, section 3), the checkpoint pause at 300 s
  ([RUNBOOK 8](../tools/cloud/RUNBOOK.md#8-what-has-not-been-measured)).  Measured but for the nogood period, on
  searches of up to 85 s ([EXPERIMENTS](EXPERIMENTS.md#scaling-to-192-threads-2026-10)).
* **The fixed costs on Graviton** after round A
  ([bench/wall, "For a cloud machine"](../bench/wall/README.md#for-a-cloud-machine)): measured, the same entry.
* **Keep every checkpoint of a dear game that does not finish**: a resumable tree past 2175 is the test bed for A1, A3
  and A4 at the size that matters.

## 7. Measurements made for this agenda

Scratch code, not kept; what follows is enough to redo them.

### 7.1 The root gaps of the c8g chain

`examples/wall.rs nodes --known KNOWN --nodes 1 N...` (one thread, 29 s), `KNOWN` the committed chain followed by
`bench/cloud/c8g.chain-2176.jsonl`, for the 33 searched games of 2176-2276 that `gaps_c8g.jsonl` lacks:
[bench/wall/gaps_c8g_rest.jsonl](../bench/wall/gaps_c8g_rest.jsonl).  With `gaps_c8g.jsonl` that is every exhausted
search of the c8g chain (39 of at least 50 nodes).  Fit: log10 of `results/strict.jsonl`'s nodes for its 314 exhausted
searches of 1301-2175 (at least 50 nodes) against `gf2 − opt` of `bench/gf2/roots.jsonl`, least squares; then the
c8g chain's nodes (64 threads, the build before the inner-loop round, as `strict.jsonl`'s) against it.

| fit over 1301-2175 | coefficients | residual sd | R² |
|---|---|---|---|
| gap | 0.00392 ± 0.00022 per unit (×10 per 255) | 0.46 | 0.50 |
| gap, n | 0.00423 ± 0.00019, −0.00093 ± 0.00009 | 0.39 | 0.63 |
| gap / n | 6.95 ± 0.31 per unit of 1000 G/n | 0.40 | 0.61 |

The c8g chain's 39 searches: mean residual +0.08 ± 0.08 decades against the gap fit (sd 0.50), +0.45 against the fit
with n, +0.67 against the fit on G/n; their own fit ×10 per 193 ± 40 of gap.  Largest residuals 2268 (+1.09), 2260
(+0.95), 2272 (+0.77); smallest 2208 (−0.69), 2205 (−0.65).

### 7.2 Other fields at the roots of the whole game

For every n of 78-1300, the parity bound's heaviest common independent set I at the root (the prover's
`parity::Meet::solve` with 0 and 1 barred, the weights the numbers; 17 s for all), the 1,055 games where it exceeds
opt(n).  I is peeled (front moves: a member with one live factor takes it; back moves: a factor with one live lister
goes to it; each keeps the rank over every field), and the residual R's matrix (its members against their maximal
factors outside I) is tested for rank over GF(3), GF(5), GF(7) and a large prime (for the rationals), with 0/1 entries
and with Koszul signs (the entry of s at s/p is (−1)^i, i the index of p among the distinct primes of s, increasing).

| n | games | mean gap | mean residual | dependent over GF(3), 0/1 | in some other field or signing |
|---|---|---|---|---|---|
| 78-299 | 73 | 6 | 8 | 0% | 0% |
| 300-599 | 281 | 28 | 17 | 43% | 49% |
| 600-899 | 300 | 77 | 33 | 47% | 57% |
| 900-1099 | 200 | 139 | 58 | 28% | 55% |
| 1100-1300 | 201 | 138 | 65 | 48% | 73% |

In all, 412 over GF(3) with 0/1 entries, 30 over GF(5), 12 over GF(7); with signs 209, 39 and 27; none over the
rationals; the rank deficiency is 1 but in six residuals (2, over GF(3)).  Of the 658 square residuals, the integer
determinant with 0/1 entries is ±1 in 408, divisible by 3 in 219, by 5 in 21, by 7 in 10; with signs ±1 in 572; ±1
under both in 358 (54%).

### 7.3 Barred bounds on the upper part

For n = 600, 1000 and 1300 (30 s for the three), the parity bound with each member u of the recorded game above n/2
barred, against opt(n): certified (below opt(n)) for 207 of 224, 339 of 377 and 472 of 490 members; the open ones are
above opt(n) by at most 102, 208 and 147, at or below the root's gap (106, 214, 151).  The open members at 1300 include
the largest prime, 1297, which THEORY.md's lemma puts in every optimal set.

### 7.4 The root gaps of the EC2 chain

`examples/wall.rs nodes --known SNAPSHOT --nodes 1 N...` (one thread, 9 minutes) over the 509 searched games of
2176-3312 of a copy of the EC2 chain's results file (games 1..3314, taken on 2026-10-15: 64 threads on a
`c8g.16xlarge`, the code of 55287ff, a window of 16 to about game 3310 and 4 after it, 6.4 hours for 2176..3314), and
`examples/matroid.rs lagrange --limit 0` and `cores` for the conflicts and cores of the root's heaviest set (3 minutes);
then the committed chain's 391 searched games of 1301-2175 solved again by that build (`taxman solve 1301-2175
--threads 1`, in order), to put the two chains' trees on one build.  The data are in
[bench/wall/](../bench/wall/README.md) (`gaps_chain.jsonl`, `anatomy_chain.jsonl`, `resolved_1301_2175.jsonl`), the
tables in `gap_output.md` (`gap.py` makes them), and the study is the entry
[The gap past 2175](EXPERIMENTS.md#the-gap-past-2175-2026-10).  What it found, for sections 2, 3 and 4: the gaps (341
to 1,443, +37 per 100 n over 1301-3312, G/n steady at 300), neither regime of section 2; the cost model on one build
(a decade per 250 of gap, trees 0.29 times the committed chain's, the gap counting in units of n^0.3 with 0 and 0.5 as
brackets); the dear games as the games of high gap; A4 killed; and the projection (a day on a `c8g.16xlarge` reaches
about 4,100).
