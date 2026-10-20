# Measurements made for the research agenda (2026-10)

Part of the [lab notebook](../README.md).

These were section 7 of the [research agenda](../../docs/AGENDA.md), "Measurements made for this agenda", until
2026-10-18, when they moved here unchanged.  The agenda's sections that cite them are its own.

Scratch code, not kept; what follows is enough to redo them.

## 7.1 The root gaps of the c8g chain

`examples/wall.rs nodes --known KNOWN --nodes 1 N...` (one thread, 29 s), `KNOWN` the committed chain followed by
`solver/frame/testdata/c8g.chain-2176.jsonl`, for the 33 searched games of 2176-2276 that `gaps_c8g.jsonl` lacks:
[solver/notebook/the-wall-regime-2026-10/gaps_c8g_rest.jsonl](../the-wall-regime-2026-10/gaps_c8g_rest.jsonl).  With `gaps_c8g.jsonl` that is every exhausted
search of the c8g chain (39 of at least 50 nodes).  Fit: log10 of `results/strict.jsonl`'s nodes for its 314 exhausted
searches of 1301-2175 (at least 50 nodes) against `gf2 − opt` of `solver/notebook/the-gf2-bound-in-the-search-2026-10/roots.jsonl`, least squares; then the
c8g chain's nodes (64 threads, the build before the inner-loop round, as `strict.jsonl`'s) against it.

| fit over 1301-2175 | coefficients | residual sd | R² |
|---|---|---|---|
| gap | 0.00392 ± 0.00022 per unit (×10 per 255) | 0.46 | 0.50 |
| gap, n | 0.00423 ± 0.00019, −0.00093 ± 0.00009 | 0.39 | 0.63 |
| gap / n | 6.95 ± 0.31 per unit of 1000 G/n | 0.40 | 0.61 |

The c8g chain's 39 searches: mean residual +0.08 ± 0.08 decades against the gap fit (sd 0.50), +0.45 against the fit
with n, +0.67 against the fit on G/n; their own fit ×10 per 193 ± 40 of gap.  Largest residuals 2268 (+1.09), 2260
(+0.95), 2272 (+0.77); smallest 2208 (−0.69), 2205 (−0.65).

## 7.2 Other fields at the roots of the whole game

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

## 7.3 Barred bounds on the upper part

For n = 600, 1000 and 1300 (30 s for the three), the parity bound with each member u of the recorded game above n/2
barred, against opt(n): certified (below opt(n)) for 207 of 224, 339 of 377 and 472 of 490 members; the open ones are
above opt(n) by at most 102, 208 and 147, at or below the root's gap (106, 214, 151).  The open members at 1300 include
the largest prime, 1297, which THEORY.md's lemma puts in every optimal set.

## 7.4 The root gaps of the EC2 chain

`examples/wall.rs nodes --known SNAPSHOT --nodes 1 N...` (one thread, 9 minutes) over the 509 searched games of
2176-3312 of a copy of the EC2 chain's results file (games 1..3314, taken on 2026-10-15: 64 threads on a
`c8g.16xlarge`, the code of 55287ff, a window of 16 to about game 3310 and 4 after it, 6.4 hours for 2176..3314), and
`examples/matroid.rs lagrange --limit 0` and `cores` (removed from master since; it builds at commit 8753189) for the
conflicts and cores of the root's heaviest set (3 minutes); then the committed chain's 391 searched games of 1301-2175
solved again by that build (`taxman solve 1301-2175 --threads 1`, in order), to put the two chains' trees on one
build.  The data are in [solver/notebook/the-wall-regime-2026-10/README.md](../the-wall-regime-2026-10/README.md) (`gaps_chain.jsonl`, `anatomy_chain.jsonl`,
`resolved_1301_2175.jsonl`), the tables in `gap_output.md` (`gap.py` makes them), and the study is the entry
[The gap past 2175](../the-gap-past-2175-2026-10/README.md).  What it found, for sections 2, 3 and 4: the gaps (341
to 1,443, +37 per 100 n over 1301-3312, G/n steady at 300), neither regime of section 2; the cost model on one build
(a decade per 250 of gap, trees 0.29 times the committed chain's, the gap counting in units of n^0.3 with 0 and 0.5 as
brackets); the dear games as the games of high gap; A4 killed; and the projection (a day on a `c8g.16xlarge` reaches
about 4,100).  The day itself reached 3440, below the projection's 10th percentile for the centre model
([the cloud run](../the-frontier-chain-to-3440-2026-10/README.md)).

## Later (2026-10): the agenda's text of 2026-10-18

AGENDA.md held this text, its section 1 and the avenues A1 to A5, until 2026-10-19, when it kept a line for each closed
avenue and a shorter section 1, and the text moved here unchanged but for its links and the level of its headings.  Its
"section 2", "section 5", "(A1)" and "(C4)" are the agenda's, and its "measurements 7.1" to "7.4" are this entry's
sections.

### 1. Where we are

**Known.**  The results and what each rests on are in the [README](../../frame/README.md#what-optimal-means), and the
current state of the work in [STATUS.md](../../docs/STATUS.md); measurements 7.4 has the root gaps of the cloud chain.
Outside the project the record stops at n = 1000 (OEIS A019312's table is this project's), and the literature has bounds
and one asymptotic result, not optima (section 5).

**The theory the search stands on.**  A set of moves is playable, independent over GF(2) in the parity matroids, or
matchable exactly when some payment set has one, an odd number, or at least one payment scheme
([THEORY.md, Theorem 12](../../../docs/THEORY.md#one-count-three-conditions)).  The GF(2) bound of the search is the
frame model's form of that middle condition (Theorem 11), equal to the parity bound at every root measured, exact on
movers with at most two live factors, and blind only to *odd cores*: self-covering sets with an odd number of schemes,
which need movers with three or more distinct primes.  At the roots, what the bound gets wrong is which candidates,
never how many, through odd cores of frames 1-3 whose size grows with n
([anatomy](../the-anatomy-of-the-gap-2026-10/README.md)).

**What drives the cost.**  A proof's nodes follow the root's gap (the bound less the optimum) and nothing else measured:
×10 per 236-266 of GF(2) gap past 1300 (section 2), as ×10 per 130 of Lagrangian gap did on the matching path
([slope](../what-drives-the-slope-2026-10/README.md)); at a given gap a larger board has the smaller tree, the gap
counting in units of √n ([the gap past 2175](../the-gap-past-2175-2026-10/README.md)).  A node costs +0.042
decades per 100 in n at the root of a tree and +0.02 averaged over a whole one, 98% of it the GF(2) intersection; the
fixed costs of a game are gone since round A; a proof
scales 58 times on 64 threads ([wall regime](../the-wall-regime-2026-10/README.md),
[round A](../round-a-the-fixed-costs-of-a-game-2026-10/README.md),
[Graviton calibration](../graviton-calibration-2026-10/README.md)).

**What has been tried** (the register has every row; by family):

* *Bounds on top of GF(2)*: cover cuts and the Lagrangian over nogoods (46% of the root gap by exact separation),
  rank cuts (54%), the per-frame exact bound (59% within 200 steps, at 300 to 3,000 times the cost), Lorea's matroid
  (nothing), the Lagrangian of the symmetric pairs (nothing).  Every share closed falls with n, and in a search each
  was a constant factor ([anatomy](../the-anatomy-of-the-gap-2026-10/README.md),
  [per-frame](../the-per-frame-bound-2026-10/README.md), [nogoods](../how-far-can-nogoods-go-2026-10/README.md)).
* *Branching and learning*: strong branching, forced members, nogoods, the rule of symmetric pairs (×0.8 of the
  cycles); dominance by exchange and transpositions (none to speak of).  Constant factors, all taken.
* *Reuse along the chain*: the ceiling, the prime formula and same search are in; carrying nogoods, a chain that
  comes back down, ceilings from neighbouring problems and equivalent problems gave nothing more
  ([going higher](../going-higher-and-coming-back-down-2026-10/README.md)), nor did reusing the previous game's tree,
  carrying its nogoods on the GF(2) path or proving a run in one search
  ([carrying](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md)); what carries is the fields bound's
  signings and multipliers, once that bound is in the search (A1).
* *The node and the engine*: the inner-loop round (×4.4 cycles), set widths, PGO, round A.  Constant factors.

The one change of slope in the project's history is the GF(2) bound itself (the plain branch and bound's log10 nodes
+0.14 per 100 in n against the matching path's +0.48).  Everything since has moved the curve down, not bent it.

### 3. Ranked avenues, A1 to A5

#### A1. A bound over several fields and signings (a change of slope, if its share holds)

*Idea.*  A playable set's payment matrix is unit triangular after its play order, so its rows are independent over
*every* field and for *every* choice of nonzero entries
([THEORY.md](../../../docs/THEORY.md#what-the-parity-bound-keeps-and-what-it-drops), the end of Theorem 12's
discussion).  Each choice (a field, and signs or values on the edges) gives a pair of frame
matroids, even and odd, in which every promotion set is independent.  GF(2) with 0/1 entries is one pair; GF(3) with 0/1
entries and GF(3) with signs are others.  Alone, each is weaker than GF(2): with 0/1 entries GF(3) misses the triangle
{6, 10, 15} (determinant ±2), and with signs it sees only the orientable closed surfaces, where GF(2) sees all, which is
why [the matroid study](../matroid-intersection-bounds-2026-10/README.md) called GF(2) the field.  *Together* they are
stronger than either: the LP over the intersection of the four matroid polytopes is a valid
bound at least as tight as GF(2)'s, computable in polynomial time by Lagrangian decomposition into two weighted matroid
intersections, one per field, with one multiplier per candidate.  The study's objection, that three matroids cannot be
intersected in polynomial time, holds for the integer problem, not for this bound.

*Why it might work.*  It sees odd cores, the whole of what GF(2) misses: a core is dependent over GF(p) exactly when p
divides every maximal minor of its matrix, and the smallest odd core, {12, 18, 20, 30, 45}, has determinant 3.  And the
signed family over GF(3) is complete: by Hershkowitz and Schneider (section 5), over any field of three or more elements
a pattern has full row rank for *every* choice of nonzero entries exactly when it has a uniquely restricted matching of
its rows, which for a set of moves is playability (Theorem 12).  The proof is constructive and fits a search: a
self-covering set (every factor listed twice or more) becomes dependent, with all coefficients 1, under any signing
whose sums vanish column by column over GF(3), which is possible for every column listed at least twice (1 + 2, 1 + 1 +
1, ...).  So every core the search meets yields a signing whose matroids see it, and all its other rank inequalities
come with it (GF(2) is the exception: a column listed three times cannot vanish there, which is exactly how odd cores
escape).  Measured (measurements 7.2): at the roots of the 1,055 games of 78-1300 whose parity bound exceeds opt(n), the
bound's heaviest set is dependent over GF(3), GF(5) or GF(7), with 0/1 entries or Koszul signs, in 566 (54%), and the
share *rises* with n: 0% below 300, 49% at 300-599, 57%, 55%, 73% at 1100-1300, as the residuals grow.  Every cut family
tried so far closed a share that fell with n; this is the first relaxation whose reach grows with the cores.  It is also
a whole family, all rank inequalities of the other field's matroids at once, which the anatomy's small unions of cuts
could not reach.

*Cheapest experiment.*  At the roots of the frame model, 880-2175 every 25th game
(`solver/notebook/the-gf2-bound-in-the-search-2026-10/roots.jsonl` has the GF(2) bounds and optima), the decomposition
bound min over μ of [heaviest GF(2) common independent set for w − μ] + [heaviest GF(3) common independent set for μ], μ
by a bundle or volume method from the GF(2) weight split (the subgradient stalled in the per-frame study), for GF(3)
plain, GF(3) Koszul, a few random signings, and the signings built from the cores of the GF(2) heaviest sets met on the
way (a matroid added per core, as a cut would be).  Code: the frame matroids of `examples/matroid.rs` and a GF(3) twin
of `examples/common/meet.rs` (two bits an entry); research code in an example, as the matroid study was (both files are
removed from master now; they build at commit 8753189).  *Metric:* the share of the root GF(2) gap closed, by band, and
its trend in n; then at the 600 probe nodes (`testdata/perf`), the share of the open nodes it cuts.  *Kill:* a mean
share under 20%, or one that falls with n as the cuts' did (−4 points per 100 n).  A pilot on the whole board first is
cheaper still: the prover's `parity.rs` with a GF(3) twin, on 1000-1300, where measurements 7.2's sets come from.

*Payoff.*  A share s that holds with n lowers the gap term of the slope by s.  At s = 0.5 and the steeper gap trend, the
cost goes from ×10 per 380 n to ×10 per about 600 n.  Below that, a constant factor (×10 per 255 of gap closed).

*Risk.*  The bound may move little where it bites: excluding one heaviest set leaves others nearly as heavy with other
cores (the anatomy's lesson), and 54% of the square residuals have determinant ±1 under both entries tried (measurements
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

#### A2. Certify the frame model, then prove with its facts (a change of slope for the proven wall)

*Idea.*  Turn the model's assumptions into proven facts, in general or game by game, so that the solver's chain is a
proof on the whole board and the prover need not repeat the model's work by search.  The first assumption is the upper
part: every optimal game plays U\*.  Barring a member u of U\* in the parity bound (Theorem 10 allows barred numbers)
proves that every optimal set holds u when the bound falls below opt(n); with U\* maximal, every optimal set then has
upper part U\*.

*Why it might work.*  Measured (measurements 7.3): one barred parity bound each certifies 207 of the 224 members of U\*
at 600, 339 of 377 at 1000 and 472 of 490 at 1300 (92-96%), in about ten seconds a game.  The rest are open by margins
up to the root's gap, and some are settled by theorems already (the largest prime is in every optimal set, THEORY.md's
lemma, yet its barred bound is open at 1000 and 1300).  The prover then searches with the certified members forced, and
its tree should lose the part spent refuting sacrifices of U\*, which grows with n.

*Cheapest experiment.*  (1) The barred bounds for every game of 1301-2175 at the roots (seconds to a minute a game on
one thread; the prover's `Meet::solve` with `Role::Barred`): the share of U\* certified and its trend.  (2) The prover's
search of five games it has proven (1302-2100, from the cloud run's data) with the certified members forced
(`Role::Pick`) and the open ones free: nodes against the unforced proof.  *Kill:* fewer than 2 times fewer nodes on the
dear games.  (3) The remaining assumptions one at a time (the prover README's open list says they are tested only end
to end): for each, a statement and either a proof or a counterexample on boards to 2000.  *Done (2026-10-15):* all
of them are theorems (THEORY.md, Theorems B and M; the numbering by layer makes the numbering clean by construction),
and the upper part is reduced to U, its members of three or more prime factors ([The frame
model on a proven footing](../the-frame-model-on-a-proven-footing-2026-10/README.md)).  *Done too:* (1), with layers 1
and 2 fixed by Theorem B, for every game of 1301-3314 that rests on the model: 7.5% of U\*_{≥3} open to 2175 and 14.9%
past it, no game closed by the bounds alone, the open share following the root's gap
([EXPERIMENTS.md](../the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md)); the search of the
open members is the integrated search, whose premise E6 measures (prepared), and whose problem, the extended problem
Π⁺(O), is built and proven exact (THEORY.md, Theorem M⁺; [The frame model by layer, and the extended
problem](../the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md)).

*Payoff.*  The verified wall moves with the solver's instead of falling behind tenfold per 1000 n; with a general proof
of the model the prover becomes a cross-check that can lag.

*Risk.*  Low for (1) and (2), which are certificates.  A general proof of "every optimal game plays U\*" is the open
problem of [prover 8](../../noframe/README.md#8-what-remains-open) and may be hard; the game-by-game certificate does
not need it.

*Proven first.*  Nothing new for barring (Theorem 10 with barred numbers); the forced members in the prover are sound
because they are certified.

#### A3. Split by the number of promotions (a sound tightening; constant factor, maybe more)

*Idea.*  Every promotion set has some size k.  Search each k that can matter apart, bounding every node by the
heaviest common independent set of *exactly* the size its completion needs: the intersection computes the heaviest set
of every size on its way (the dominant sets the GF(2) levels already keep).  The sizes are a partition, so this is
sound without any conjecture, and the root's bounds of each size prune most k at once.

*Why it might work.*  The GF(2) heaviest set at the root has the optimum's size at 1,119 of the 1,121 roots measured
(C4), and the optimal sets of every game enumerated have one size ([prover
5](../../noframe/README.md#5-is-the-optimal-set-unique)).  So only one or two k survive at the root, and at a node whose
heaviest completion has the wrong size the exact-size bound is lower.  Unlike `revise_freedoms`, it assumes nothing
about which size the optimum has.

*Cheapest experiment.*  At the root and the probe's 600 nodes: the heaviest common independent set of each size, the
k that survive against the optimum, and at the nodes the drop from the unconstrained bound to the exact-size bound of
the surviving k.  *Kill:* a drop that cuts under 10% of the open probe nodes.

*Payoff.*  A constant factor if the drop is a fixed amount; more if wrong-size completions are what deep nodes are
made of.  *Risk:* low; nodes may already have the right size.  *Proven first:* nothing beyond the partition.

*Result* (the nodes of 936, 1215 and 2112,
[A3's test](../split-by-the-number-of-promotions-a3s-test-2026-10/README.md)): killed at the diagnostic.  The root's
window of winning sizes is one or two (52; 69; 116-117) and all 812 nodes, every open one included, have a heaviest
completion of the root's total size, the root's heaviest set being a largest common independent set, so no node lies
outside the window and the exact-size bound equals the plain bound.

#### A4. What the cost per unit of gap is made of (a diagnostic, then branching on cores)

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

*Result* (the 400 exhausted searches of 2176-3312, measurements 7.4): killed.  Nineteen features of the root were tried
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

#### A5. Decompose across the frames (AND/OR search)

*Idea.*  The frames form a path (a candidate of home h touches frames h and h − 1 only), so fixing the decisions on the
candidates of one home separates the frames in front from those behind, and the best completion is the sum of the two
sides' bests.  An AND/OR branch and bound (branch on a separator home, then solve the two sides independently, caching
nothing across) turns a product of the two sides' trees into a sum.  The component decomposition of [the pilots of ideas
from other fields](../ideas-from-other-fields-the-pilots-2026-10/README.md#44-component-decomposition-killed)
looked for disconnected parts and found one component; this conditions on a separator instead.  `max_total` couples all
frames, but it never binds and can be dualized.

*Cheapest experiment.*  At the probe's nodes and along the proofs of 1215 and 2170: how the GF(2) gap of a node splits
by frame (the anatomy did it at roots: homes 1-3), and whether the subtrees under a node resolve the cores of frame 1
and of frame 3 independently (the tree's size against the product of the per-frame resolutions).  *Kill:* the cores
sit in adjacent frames (homes 1 and 2), as at the roots of 1251-1300, so that no separator splits them.

*Payoff.*  A change of slope where the gap is spread over separable frames; nothing where it is concentrated.
*Risk:* high, given the anatomy's homes.  *Proven first:* the decomposition (the frame-by-frame promotion criterion).

*Result* (412 open nodes of the proofs of 936, 1215, 2112 and 2170,
[A5's test](../decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md)): killed at the
measurement.  Every conflict lies in two adjacent homes (none of 11,197 crosses a separator) and all of them in frames
1-3, so a home always splits them, but it touches 8-10 of the heaviest set's 9-16 cores and 22-40 candidates (a quarter
to a half of the home's), and the bound's error sits at that home.  With the separator fixed as the best completion has
it, the two sides cost 0.01-2% of the subtree; the cost is the separator's enumeration, which the AND/OR search has to
do too: its OR part alone took 5.7 to 154 times the plain tree's steps at 936, 1215 and 2112, mostly without finishing.
