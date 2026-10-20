# Research agenda (2026-10)

What to pursue to push the wall much further: what limits the search, the avenues ranked, the conjectures worth proving,
the ideas worth borrowing, and what the cloud runs should measure.  It is for Brian, and for an agent choosing the next
experiment.  The wall is the first game whose own search takes longer than the limit that [STATUS](STATUS.md) gives
([the glossary](../../docs/GLOSSARY.md)).  Each avenue says where it stands.  The
[register](../notebook/README.md#register-of-everything-tried) is the record of what was tried, and an avenue started
from here gets its row there when it ends.  The measurements made for this agenda are the entry [Measurements made for
the research agenda](../notebook/measurements-made-for-the-research-agenda-2026-10/README.md), cited below as
"measurements 7.1" to "7.4".

* [1. Where we are](#1-where-we-are)
* [2. The diagnosis of the wall](#2-the-diagnosis-of-the-wall)
* [3. Ranked avenues](#3-ranked-avenues)
* [4. Conjectures](#4-conjectures)
* [5. Ideas from other domains](#5-ideas-from-other-domains)
* [6. What the cloud runs should measure](#6-what-the-cloud-runs-should-measure)

## 1. Where we are

[STATUS](STATUS.md) gives the state of the work, and [results/README.md](../../results/README.md) how far the chain
reaches and what each score rests on.  Outside the project the record stops at n = 1000 (OEIS A019312's table is this
project's), and the literature has bounds and one asymptotic result, not values of opt(n) (section 5).

**The theory the search stands on.**  One count decides three conditions on a set of moves ([THEORY.md, Theorem
12](../../docs/THEORY.md#one-count-three-conditions)): the set is playable exactly when some payment set has one payment
scheme, independent over GF(2) in the parity matroids when some has an odd number of them, and matchable when some has
at least one.  The search's GF(2) bound is the frame model's form of the middle condition (Theorem 11).  It equals the
parity bound at every root measured, and it is exact on movers with at most two live factors.  It is blind only to *odd
cores*: self-covering sets with an odd number of schemes, which need movers with three or more distinct primes.  At the
roots, what the bound gets wrong is which candidates, never how many, through odd cores of frames 1-3 whose size grows
with n ([the anatomy of the gap](../notebook/the-anatomy-of-the-gap-2026-10/README.md)).

**What has been tried,** by family (the register has every row):

* *Bounds on top of GF(2)*: cover cuts and the Lagrangian bound over the nogoods, rank cuts, the per-frame exact bound,
  Lorea's matroid, the Lagrangian bound of the symmetric pairs, and the multi-field bound (avenue A1, section 3).  Every
  share of the gap that one closed falls with n, and in a search each was a constant factor ([the anatomy of the
  gap](../notebook/the-anatomy-of-the-gap-2026-10/README.md), [the per-frame
  bound](../notebook/the-per-frame-bound-2026-10/README.md), [how far nogoods can
  go](../notebook/how-far-can-nogoods-go-2026-10/README.md)).  Flat-cycle cuts, measured at roots only, add little to
  the matching LP.
* *Branching and learning*: strong branching, forced members, nogoods, the rule of symmetric pairs, dominance by
  exchange and transpositions.  Constant factors, all taken.
* *Reuse along the chain*: the ceiling, the prime formula and same search are in.  Carrying nogoods or a search's tree
  to the next game, a chain that comes back down, ceilings from neighbouring problems and equivalent problems gave
  nothing more ([going higher](../notebook/going-higher-and-coming-back-down-2026-10/README.md),
  [carrying](../notebook/carrying-a-searchs-learning-to-the-next-game-2026-10/README.md)).
* *The node and the engine*: the inner-loop round, set widths, PGO, round A.  Constant factors.

The GF(2) bound itself is the one change of slope in the project's history: the plain branch and bound's log10 nodes
grow +0.14 per 100 in n, against the matching path's +0.48.  Everything since has moved the curve down, not bent it.
This section's text of 2026-10-18, with its numbers and links, is [in the measurements
entry](../notebook/measurements-made-for-the-research-agenda-2026-10/README.md#later-2026-10-the-agendas-text-of-2026-10-18).

## 2. The diagnosis of the wall

**The gap is the cost.**  A proof's nodes grow tenfold for about every 250 points of the root's gap G, the root's GF(2)
bound less P\* ([the glossary](../../docs/GLOSSARY.md)), and nothing else measured explains them.  The gap spans 125 to
1,443 over the searches of 1301-3312, four decades of tree, while the cost of a node moves only 0.02 decades per 100 in
n averaged over a whole tree (0.042 at its root).  So the wall moves with the gap: with a bound that closes more of it,
or with gaps that grow more slowly.

**The cost model.**  Fitted on one build over 1301-3312, to 714 exhausted searches (the games of 1301-2175 solved again,
and the EC2 chain's 400 of 2176-3312 with their nodes less 0.06 decades for the window and the threads; [the gap past
2175](../notebook/the-gap-past-2175-2026-10/README.md)):

    log10 nodes ≈ 2.67 + G / 247                (R² 0.85, residual sd 0.47)
    log10 nodes ≈ 2.30 + 0.0479 G / n^0.3       (R² 0.875, residual sd 0.44)

The slope is a decade of nodes per about 250 of gap (221 ± 10 over 1301-2175, 266 ± 7 over 2176-3312).  At a given gap a
larger board has the smaller tree: the second model gives a decade per 230 of gap at n = 3000, 251 at 4000 and 284 at
6000, which G alone reads as an intercept that falls about 0.4 to 0.5 decades per 1,000 n.  On one build, G alone
overpredicts every later stretch by 0.2 to 0.6 decades, and the gap scaled by n^p with p = 0.3 predicts best (worst bias
0.27 over five origins).  The two chains disagree about p (0 to 0.2 across 2175, 0.4 to 0.5 within 2176-3312), so G
alone is the conservative end of every projection, p = 0.5 the optimistic one, and 0.3 the centre.

*Details.*  The first model was fitted to the 314 exhausted searches of 1301-2175 in `results/strict.jsonl` (8 threads,
the build of feba6d0), against the root GF(2) gaps of `solver/notebook/the-gf2-bound-in-the-search-2026-10/roots.jsonl`
(measurements 7.1): log10 nodes ≈ 3.24 + G / 255 (R² 0.50, residual sd 0.46).  It kept G alone, because n and G/n
extrapolated worse past 2175 against the committed chain's trees; on one build that is reversed.  The later builds make
trees 0.29 times as large: 0.27 at 1301-1500 and 0.41 near 2175 for the same 391 games solved again on one thread, and
0.44 for the 39 searches of 2176-2276 against the Graviton calibration's chain (measurements 7.4).

**Past 2175 the trees still follow their gaps, and the gaps rise.**  With the root gaps of all 509 searched games of
2176-3312 (measurements 7.4), the dear games are the games of high gap: the 47 searches of gap 1,100 or more hold 77% of
the chain's nodes, and against the scaled gap the 18 dearest are within +0.6 decades of the model (the most in excess
2346, 3.9 times, and 3300, 3.3 times; the residuals' sd is 0.48).  The wall regime had found the dear games of the c8g
chain 4 to 18 times larger than their gaps predict, but those games were chosen for their trees.  The level of the gap
is new: about 450 at 2100, 650-1,000 by 2400, then waves of a hundred games and more (mean 530 at 2601-2700, 1,130 at
3101-3200).  The residuals run in blocks (lag-1 autocorrelation +0.84, consecutive boards being nearly one problem), and
nothing at the root explains them (A4, section 3): conjecture C6 is refuted as stated.

**Why the trees are big** (2026-10-18).  The trees are the bound's error, not near-optimal completions: four of five
units, hard and easy, hold no completion within 100 points of their best ([completions near the
optimum](../notebook/completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md)).  The gap is paid in
exclusions, each priced at a tenth of the member left out, since the bound substitutes a near-equal number: 5 to 8 a
path ([the anatomy of a hard subtree](../notebook/the-anatomy-of-a-hard-subtree-2026-10/README.md)).  The prime swaps
behind them are no symmetries of their nodes, only the root's pairs, so they prune nothing ([local prime
exchanges](../notebook/local-prime-exchanges-at-the-nodes-2026-10/README.md)).

**How fast the gap grows decides the wall.**  Over 1301-2175 the root gap's trend was +6.9 ± 1.7 per 100 n ([the GF(2)
bound in the search](../notebook/the-gf2-bound-in-the-search-2026-10/README.md)), with a flat stretch below 2150 (+3.5 ±
2.4).  The gap over the unseeded start rises +57 per 100 n to 4095, an upper bound: that start is about 770 below opt(n)
at 3100, against 0 to 403 where it had been measured.  The true gaps of 2176-3312 grow +37 per 100 n over the whole
1301-3312 (95%: 22 to 50 by blocks of 100 games, 6 to 63 by blocks of 200), and +25 over 2176-3312 alone (−43 to +83:
the waves are longer than a window of 1,100 n); G/n stays at 300.  The regimes, the first two with the node's +0.042 at
the root, the measured one with the +0.02 of a whole tree and the cost model above:

| gap trend | decades per 100 n | cost ×10 per | what 100 times the compute buys |
|---|---|---|---|
| +7 per 100 n (1301-2175) | 0.07 | about 1,450 n | about 2,900 n |
| +57 per 100 n (unseeded starts to 4095) | 0.27 | about 380 n | about 750 n |
| +25 to +37 per 100 n (measured, 2176-3312 and 1301-3312), gap scaled by n^0.3 | 0.08 to 0.095 | about 1,050-1,250 n (830 to 1,500 for p = 0 to 0.5) | about 2,100-2,500 n |

The steeper regime is refuted: the gap does not grow +57 per 100 n (4% of the draws by blocks of 200 games), and the
cost of a game grows tenfold per 1,050-1,250 n, not per 380.  That the chain reached 3314 in 6.4 hours is not by itself
the evidence: its cost sits in the crests of the gap, an hour or two of the 64 threads for a hundred games, and the
valleys cost seconds.

**What the trend projects.**  The models put a day of one `c8g.16xlarge` at about 4,100 (3,700 to 4,850 across the
models, −400 to +700 for a single run's luck), 4000 at 13 to 20 hours past 3313, 5000 at four to ten days and 6000 at a
month or more; a `c8g.48xlarge`, 1.9 to 2.7 times as fast, adds 300 to 500 n to a day ([the gap past
2175](../notebook/the-gap-past-2175-2026-10/README.md) has the derivation).  The day that ran reached 3440, under the
10th percentile of the centre model ([the cloud run to 3440](../notebook/the-frontier-chain-to-3440-2026-10/README.md)).
At that run's own trend, tenfold for about every 1,040 in n, one `c8g.16xlarge` at spot ($0.68 an hour) reaches about
(projected on 2026-10-16, from the run to 3440):

| more machine time | n reached | cost |
|---|---|---|
| 1 day | 3,740 | $16 |
| 3 days | 4,050 | $49 |
| 10 days | 4,500 | $160 |
| a month | 5,000 | $530 |

The models' projections came out high, so this table is the better guide.  The run to the wall then reached 3914 in 35.7
hours, and its dearest game took 2.77 hours, so the wall itself, the first game of 24 hours, lies further on ([its
entry](../notebook/the-run-to-the-wall-2026-10/README.md)).  Every doubling of speed or of spending is worth about 310
in n; only a tighter bound in the search changes the tenfold per 1,040.

**Where an order of magnitude can come from.**

* *The bound.*  The cost is 10^(G/250), so a bound that closes a share s of the gap at every node multiplies the slope's
  gap term by 1 − s: a change of slope, the only one in reach.  Closing a fixed amount is a constant factor, tenfold for
  each 250 closed.  Every relaxation tried on top of GF(2) closed a share that falls with n (section 1); what is left is
  odd cores.
* *The cost per unit of gap* (1/250 decades).  Branching and learning set it.  Every change measured moved it by a
  constant, and it does not rise with n: it falls (a decade per 230 of gap at 3000, 284 at 6000).  Nothing at the root
  explains its runs past 2175 (A4).
* *The node, the engine, the fixed costs.*  Constant factors: the intersection is 98% of a node, and a proof scales 58
  times on 64 threads.  They are needed, but they shift the wall by a few hundred n, not by thousands.
* *The frame model, for the proofs on the whole board.*  The solver's results rest on the model; the prover's check does
  not, but its nodes per solver node grow threefold per 1000 in n (2.5 at 1301-1499, 6.1 at 2060-2169), and its node
  costs 9 to 17 times the solver's (17 at 990-1300; 4.1 ms against 0.46 near 2170, both on Graviton4; [the prover's
  proofs of 1301-2175](../notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)).  Those
  ratios are against the committed chain's nodes; against the later builds' trees, 0.29 times as large, they are 3.4
  times higher.  At that trend the check costs about 15 times the solver's nodes near 3000 (50 against the later
  builds'), and the proofs of 1301-2175 took 226 thread-hours.  So the proofs on the whole board will fall behind the
  chain, unless the model is proven or the prover gets the model's reductions as proven facts (avenue A2).

## 3. Ranked avenues

Ranked by expected gain to the wall over cost.  Each opens with where it stands.  An open avenue then says what must be
proven before it can prune, since every rule the search relies on is proven and tested, as the project requires.  A
closed avenue keeps its finding in a line; its text as it stood is [in the measurements
entry](../notebook/measurements-made-for-the-research-agenda-2026-10/README.md#later-2026-10-the-agendas-text-of-2026-10-18).

### A1. A bound over several fields and signings (a change of slope, if its share holds)

*Closed* (2026-10-18; the slope test killed it in the search on 2026-10-16).  The multi-field bound, the GF(2) frame
matroids and the signed GF(3) ones coupled by Lagrangian multipliers ([the glossary](../../docs/GLOSSARY.md); THEORY.md,
Theorems 14 and 15), closes a large share of the root's gap ([the fields
study](../notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md)).  But in a branch and bound its
trees shrink by a factor that falls as the gap grows, and at a node it cuts none of gap 500 or more, where the cost is,
so every policy of computing it loses ([the E3
pilot](../notebook/the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md), [R3 and the slope
test](../notebook/r3-and-the-slope-test-u4-of-a1-2026-10/README.md), [the bound at the
nodes](../notebook/the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/README.md)).

### A2. Certify the frame model, then prove with its facts (a change of slope for the proven wall)

*Paused.*  Its first steps are done, and the campaign of certificates of 2176..3440 is ready and paused; the games past
3440 have none yet.  The proven wall of the heading is the end of the games proven on the whole board, 2175 (the
prover's proofs, and certificates of 600..1300) ([STATUS](STATUS.md); [RUNBOOK section
9](RUNBOOK.md#9-certifying-u-stage-4)).

*Idea.*  Turn the model's assumptions into proven facts, in general or game by game, so that the solver's chain is a
proof on the whole board and the prover need not repeat the model's work by search.  The first assumption is the upper
part: every optimal game plays U\*.  Barring a member u of U\* in the parity bound (Theorem 10 allows barred numbers)
proves that every optimal set holds u when the bound falls below opt(n); with U\* maximal, every optimal set then has
upper part U\*.

*Why it might work.*  One barred parity bound each certifies 207 of the 224 members of U\* at 600, 339 of 377 at 1000
and 472 of 490 at 1300 (92-96%), in about ten seconds a game (measurements 7.3).  The rest are open by margins up to the
root's gap, and theorems settle some of them already: the largest prime is in every optimal set (THEORY.md's lemma), yet
its barred bound is open at 1000 and 1300.  The prover then searches with the certified members forced, and its tree
should lose the part spent refuting sacrifices of U\*, which grows with n: A2's premise.

*What is done.*

* **Every assumption is a theorem** (2026-10-15): THEORY.md's Theorems B and M, with the numbering by layer clean by
  construction, and the upper part reduced to U, its members of three or more prime factors ([the frame model on a
  proven footing](../notebook/the-frame-model-on-a-proven-footing-2026-10/README.md)).
* **The barred bounds**, with layers 1 and 2 fixed by Theorem B, for every game of 1301-3314 that rests on the model:
  7.5% of U\*_{≥3} open to 2175 and 14.9% past it, no game closed by the bounds alone, and the open share following the
  root's gap ([the upper part game by
  game](../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md)).
* **The search of the open members** is the integrated search over the extended problem Π⁺(O), built and proven exact
  (THEORY.md, Theorem M⁺; [the frame model by layer, and the extended
  problem](../notebook/the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md)).  `taxman certify` runs it
  game by game, and the certificates of 600..1300 are committed
  ([results/README.md](../../results/README.md#certificates)).


*What remains.*  The campaign of certificates of 2176..3440, and the claim-by-claim rule (Lemma M5⁺), which would settle
its last phase without a search; both are paused (STATUS).  The prover's search with the certified members forced, on
five games of 1302-2100 (killed if the dear games take fewer than half the nodes), has not run; E6, the measurement of
the sources left free against the upper part fixed to U\*, was prepared and then superseded by the certificates.

*Payoff.*  The proofs on the whole board move with the solver's chain instead of falling behind tenfold per 1000 n.
With a general proof of the model, the prover becomes a cross-check that can lag.

*Risk.*  Low for the certificates.  A general proof of "every optimal game plays U\*" is the open problem of [the
prover's README, section 8](../noframe/README.md#8-what-remains-open), and may be hard; the certificate game by game
does not need it.

*Proven first.*  Nothing new for barring (Theorem 10 with barred numbers); the forced members in the prover are sound
because they are certified.

### A3. Split by the number of promotions (a sound tightening; constant factor, maybe more)

*Closed* (2026-10-16), at the diagnostic: at the nodes of 936, 1215 and 2112 every node's heaviest completion already
has the root's size, so the bound of each size equals the plain bound ([A3's
test](../notebook/split-by-the-number-of-promotions-a3s-test-2026-10/README.md)).

### A4. What the cost per unit of gap is made of (a diagnostic, then branching on cores)

*Closed* (2026-10-15): nothing at the root explains the cost model's residuals, neither the heaviest set's conflicts nor
the cores it runs into, the forced members or the sizes; the dear games are the games of high gap (measurements 7.4).

### A5. Decompose across the frames (AND/OR search)

*Closed* (2026-10-18), at the measurement: the conflicts sit in adjacent homes of frames 1-3, and the cost is the
separator's enumeration, which the AND/OR search must do too ([A5's
test](../notebook/decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md)).

### A6. The prover's node (a constant factor that keeps verification in pace)

*Open.*  A node of the prover costs 9 to 17 times the solver's (section 2), because it solves the parity matroids at
each node, warm-started from its parent, where the solver keeps contracted matroids by pivots along the path ([the GF(2)
bound in the search](../notebook/the-gf2-bound-in-the-search-2026-10/README.md), item 5).  Reimplementing the technique
in the prover, without sharing code, which is the point of the prover, would give it most of that factor.  The bound
without a member, read off the exchange graph, would cut its strong branching.  *Experiment:* the prover's node profile
on 1300 and 2100.  *Payoff:* about ten times, a constant, which is what A2's remaining searches will need near 3000.

### A7. The engineering the wall will need (constant factors, in this order)

* **192 threads**: the tree's growth with workers (10-20% at 32-128 emulated), the nogood period at 192 ([ideas from
  other fields, section
  3](../notebook/ideas-from-other-fields-the-pilots-2026-10/README.md#3-nogoods-with-very-many-threads)), checkpoints at
  300 s.  The cloud machine measures them, and they are not designed further here; [scaling to 192
  threads](../notebook/scaling-to-192-threads-2026-10/README.md) has what it measured.
* **The intersection's shortest paths** (69% of a node at 4000), open: the GF(2) inner loop has left the solves of
  strong branching mostly certified; what remains is the augmentations of a level, a few dozen a node.
* **Boards past 4095**, built: a width of 128, `MAX_FRAMES` 9 and the GF(2) vectors to 5 words ([the wall regime,
  4](../notebook/the-wall-regime-2026-10/README.md)) take the boards to 8191 ([boards past
  4095](../notebook/boards-past-4095-2026-10/README.md)).
* **Proving runs of neighbours that share a gap in one search**, closed, not worth building: the trees of consecutive
  games share 0.15% of their steps (median of 14 pairs), and a run costs at least its dearest game, which limits the
  saving to 18-29% of the nodes past 2175
  ([carrying](../notebook/carrying-a-searchs-learning-to-the-next-game-2026-10/README.md)).
* **Strong branching's solves**, untried: 70% of the intersection's work, they restart far below their answer, and
  cancelling the negative cycles of the optimal set less the member would start them next to it ([the GF(2) bound in the
  search](../notebook/the-gf2-bound-in-the-search-2026-10/README.md), the item "Next on the inner loop").
* **What only the matching path uses**, open: the GF(2) bound has made it idle, and it can be removed once a production
  run has been made with the GF(2) search: the Lagrangian bound in the search and refinement ([the same
  entry](../notebook/the-gf2-bound-in-the-search-2026-10/README.md), the item "To remove").
* **A unit that carries its GF(2) level**, not built: a rebuilt unit's first node would check the level instead of
  solving it, which wins at most the units' first nodes, 1 to 4% of a thread's time at 64 threads, nothing on long
  searches ([scaling to 192 threads](../notebook/scaling-to-192-threads-2026-10/README.md), the paragraph "What to build
  next").

**Closed beside these avenues** (2026-10-18):

* **The Lagrangian bound over the nogoods, refitted at a node** (the entries' "strong bound"): it cuts most open nodes
  of small gap and none from a gap of 300, where the cost is, and every policy of computing it loses ([the node
  choice](../notebook/the-node-choice-where-the-strong-bound-pays-2026-10/README.md)).
* **Flat-cycle cuts** on the matching LP: they add a little of the gap, do not beat A1, and the rest is not cycle-shaped
  ([the entry](../notebook/flat-cycle-cuts-on-the-matching-lp-2026-10/README.md)).

## 4. Conjectures

Each opens with where it stands; a settled one keeps its result in a few lines.

**C1 (fields).**  *Tested at the roots: not killed, but the share falls with the gap; A1, the avenue it served, is
closed.*  The LP over the frame matroids of GF(2) and of a few GF(3) signings (0/1 entries, Koszul signs, and the
signings built from the cores met) closes a share of the root GF(2) gap that does not fall with n.  At the roots of
2200-3400 it closes 0.46 of the gap, flat in n there, against 0.62 at 1700-2175, and over 900-3333 the share falls 1.0 ±
0.34 points per 100 n, and 3.8 ± 1.1 per 100 points of gap ([E2 of the fields
study](../notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md#e2-past-2175-2026-10)).  *Evidence
before the test:* the heaviest GF(2) set is dependent in another field or signing at 49-73% of the roots of 300-1300,
rising (measurements 7.2), and the signed GF(3) family is complete (Hershkowitz and Schneider, section 5).

**C2 (sign coherence of three-prime cores).**  *Open; not tested.*  On numbers p^a q^b r^c of three primes, movers of
degree d + 1 over factors of degree d form the honeycomb graph of a triangular region, and a set's schemes onto one
payment set are its lozenge tilings.  Conjecture: in a region without holes all of a core's schemes have the same sign,
so its determinant is ± its number of schemes, and GF(p) sees it exactly when p divides that number.  The smallest odd
core, {12, 18, 20, 30, 45} on the primes 2, 3, 5, has three schemes, all of one sign: determinant 3.  This would be the
combinatorics of the weak Lefschetz property of monomial ideals in characteristic p, where the multiplication map's
determinant counts lozenge tilings (section 5; to check).  *Why it matters:* it would say which fields to add for which
cores, and why GF(3) sees so many.  *Test:* enumerate the minimal odd cores of three primes to 8 members
(`heuristics/evaluation/fm_relation.py`'s `odd_cores`, restricted) and compare |determinant| with the count of schemes;
then regions with holes.

**C3 (the upper part).**  *Open.*  Every optimal game plays U\*, and game by game barred parity bounds and small
searches could certify it.  This strong form holds in every game whose optimal sets were enumerated (954 games of
2..989).  The weak form, U, that some optimal set contains U\*_{≥3}, holds wherever the prover has proven opt(n), every
n up to 2175, and certificates prove it game by game for 600..1300 ([THEORY.md, "The upper
part"](../../docs/THEORY.md#the-upper-part); [results/README.md](../../results/README.md#what-supports-the-scores)).
*Evidence:* every game enumerated ([the prover's README, section 5](../noframe/README.md#5-is-the-optimal-set-unique));
92-96% of U\* certified at the root by one bound each (measurements 7.3).  *Attack:* the frontier argument of [the
prover's README, section 8](../noframe/README.md#8-what-remains-open) for a proof; A2 for certificates.

**C4 (one size, near the relaxation's).**  *Open.*  All optimal sets of a game have one size, and it is within one of
the size of the GF(2) bound's heaviest set at the root.  *Evidence:* every enumerated game's optimal sets have one size
([the prover's README, section 5](../noframe/README.md#5-is-the-optimal-set-unique)).  The heaviest set has the size of
an optimal set at all 361 roots of 880-1300 ([the anatomy of the
gap](../notebook/the-anatomy-of-the-gap-2026-10/README.md), 1), and at 758 of the 760 searched and repeated games of
1301-2175 (`solver/notebook/the-gf2-bound-in-the-search-2026-10/roots.jsonl`'s `size` against `strict.jsonl`'s
promotions).  At 1320 and 1322 it has 77 members and the optimal set 78, which is `max_total`: a tie of weights, or a
heavier set of the wrong size, which is not known.  So the strict form, the same size, is false or rests on ties.  *Why
it mattered:* with it, A3 would have searched two values of k at most (it was not needed for A3's soundness); A3 is
closed.  *Test:* the heaviest set of each size at the roots of 1320 and 1322.

**C5 (the gap law).**  *Measured (2026-10-15): both forms hold, and neither regime.*  The true root gap of the searched
games grows linearly in n past 2175, at a rate between the two regimes of section 2; or, in the form the fit of
1301-2175 prefers, G/n stays bounded.  Everything about where the wall is rests on it.  Over the EC2 chain's 509
searched games of 2176-3312 (measurements 7.4), the gap's growth is section 2's.  Beside it: the gap is 341 to 1,443
with a mean of 818, against 391 at 1301-2175, risen from about 450 at 2100 to 650 at 2160-2200; G/n was 224 at
1301-2175, and drifts down 3 per 100 n.  So +7 was a flat stretch below 2150, and +57, the unseeded start's upper bound,
is not the gap's growth.  The waves are partly arithmetic: counts of numbers of three or more distinct primes by tier
predict a third of their variance out of sample.

**C6 (runs).**  *Refuted as stated* (2026-10-15, by A4).  The cost per unit of gap is a function of the root's core
structure (the number and size of the odd cores, the frames holding them), which changes in blocks of n.  The cost's
residuals do run (lag-1 autocorrelation +0.84: consecutive boards pose nearly one problem), but what runs is the gap, in
waves, and the root's cores, conflicts and forced members add nothing to it.  The runs of dear games are runs of high
gaps (section 2): the 47 searches of gap 1,100 or more are 12% of the exhausted ones, the 11 of 1,300 or more hold 53%
of the chain's nodes, and all of the 18 dearest have a gap of 1,094 or more.  What stays unexplained is a factor of 3
(sd 0.48 decades) in a game's tree at its gap.

**C7 (♦).**  *Open; certified in every game to 100,000, and in 902 games to 2,000,000* ([THEORY.md, "The span
certificate (♦)"](../../docs/THEORY.md#the-span-certificate-)).  Every source that descending greedy rejects on the
sources' instance is a GF(2) combination of the sources it kept above it; it would prove the upper-half conjecture in
general.  It is not on the critical path of the wall; C3 is.

## 5. Ideas from other domains

Each with the piece that transfers.  The sources were surveyed for this agenda; "read" marks those read in full or in
the relevant pages, the others were seen in abstracts.

* **Matrix theory: zero patterns over fields of three or more elements.**  D. Hershkowitz and H. Schneider, "Ranks of
  zero patterns and sign patterns", Linear and Multilinear Algebra 34 (1993) 3-19 (read): over a field of at least three
  elements, an r × n zero-nonzero pattern has independent rows for every choice of nonzero entries exactly when its
  bipartite graph has a *constrained* r-matching (the only one between its vertex sets: a uniquely restricted matching),
  and their Example 3.6 shows it fails over GF(2).  With Theorem 12, a set of moves is playable exactly when it is
  independent under every GF(3) signing.  *Transfers:* the completeness of the signed GF(3) family, and its constructive
  proof (column sums made to vanish), which turns each core the search meets into a matroid that sees it (A1, closed).
  The minimum-rank literature (zero forcing bounds the maximum nullity over every field; minimum rank depends on the
  field for some graphs) is the general setting.
* **Integer linear algebra.**  A set's rows are independent over GF(p) exactly when p does not divide the gcd of its
  maximal minors, the last determinantal divisor of its Smith normal form (R. P. Stanley, "Smith normal form in
  combinatorics", JCTA 144 (2016), arXiv:1602.00166).  *Transfers:* which fixed fields to add (measurements 7.2's census
  of determinants), and the fact that cores whose divisor is 1 escape every fixed-entry field, so signings are needed.
* **Coding theory, the erasure channel.**  The cores are *stopping sets* of a parity-check matrix whose variable nodes
  are movers and check nodes factors; the even cores are supports of codewords; peeling is the erasure decoder.
  Stopping sets that contain no codeword support, the odd cores, have a name: *coverable* stopping sets (Yakimenka,
  Skachek, Bocharova, Kudryashov, "Stopping redundancy hierarchy beyond the minimum distance", arXiv:1804.06770, read in
  part), the ones redundant checks are added to remove (Schwartz and Vardy, IEEE Trans. Inf. Theory 52 (2006)).  Small
  stopping sets are enumerated by tree search (Rosnes and Ytrehus, IEEE Trans. Inf. Theory 55 (2009); Wang, Kulkarni and
  Poor, IEEE Trans. Inf. Theory 55 (2009), all sets to size 13 at about 500 variables) and by integer programming
  (Velasquez, Subramani and Drager, ISCO 2018, to size 48), and LP decoding on the erasure channel succeeds exactly when
  peeling does (Feldman, Wainwright, Karger, IEEE Trans. Inf. Theory 51 (2005)).  *Transfers:* the enumerators, to list
  a frame's odd cores, for a census of them and for the signings of A1; and the mirror image of A1: there the code is
  fixed and the decoder gains checks, here the game is fixed and the relaxation gains fields.
* **Random XORSAT and hypergraph 2-cores.**  For random 3-XORSAT the 2-core appears at density about 0.818 and full rank
  lasts to about 0.918 (Dubois and Mandler, FOCS 2002; Pittel and Sorkin, CPC 25 (2016)): in between, a non-empty core
  of full GF(2) rank, an odd core, is the typical case.  *Transfers:* a density-evolution model of the frames (movers
  per factor, the share with three live factors) to predict how the cores, and so the gap, grow with n (C5); and the
  lesson that the odd-core gap is generic, not an accident of a few boards.
* **Uniquely restricted matchings.**  The maximum uniquely restricted matching is NP-hard on bipartite graphs (Golumbic,
  Hirst, Lewenstein, Algorithmica 31 (2001); Müller, Order 7 (1990)), not approximable within n^(1/3 − ε) unless NP =
  ZPP (Mishra, 2011), and fixed-parameter tractable in treewidth in 2^O(tw²) n (Chaudhary, Sau, Zehavi,
  arXiv:2508.12004, 2025).  No integer-programming formulation or polyhedral study of it was found.  *Transfers:* a good
  polynomial bound must use the arithmetic, as GF(2) does; the core inequalities of A1 and the anatomy are new ground;
  the treewidth recursion could close small residual components exactly after branching.
* **Algebraic combinatorics.**  The weak Lefschetz property of monomial ideals in three variables: the multiplication
  map between degrees has a determinant that counts lozenge tilings, and fails in characteristic p when p divides the
  count.  *Transfers:* product formulas for those counts, to predict which primes see the three-prime cores (C2).  Not
  surveyed: the statement is to check before relying on it.
* **Matroid optimization.**  Lagrangian decomposition (Guignard and Kim, Math. Programming 39 (1987)) reaches the LP
  over the intersection of the matroid polytopes with two-matroid subproblems; for three matroids that LP's integrality
  gap is 2 (Linhares, Olver, Swamy, Zenklusen, Math. Programming 2020), so it is a bound, not the integer optimum.  The
  k best common independent sets can be ranked (Camerini and Hamacher, SIAM J. Discrete Math. 2 (1989)), and
  intersection under small changes of the matroids has fast algorithms (Blikstad, Mukhopadhyay, Nanongkai, Tu, STOC
  2023).  *Transfers:* the decomposition for A1, with a volume or bundle method for the multipliers (the plain
  subgradient stalled in the per-frame study); ranking for incumbents, and for A3's sizes (A3, closed).
* **AND/OR search** (graphical models).  Branch on a separator, solve the sides independently.  *Transfers:* the frame
  path's homes as separators (A5, closed).
* **Proof logging.**  Gaussian elimination certified in pseudo-Boolean proofs (Gocht and Nordström, AAAI 2021), branch
  and bound certified by aggregation and disjunction steps (VIPR: Cheung, Gleixner, Steffy, IPCO 2017), dominance and
  symmetry breaking certified (Bogaerts, Gocht, McCreesh, Nordström).  No certificate format for matroid-intersection
  duality exists; Frank's weight split with greedy-optimality certificates per matroid is the natural one, checkable
  with XORs and comparisons.  *Transfers:* a log of the solver's tree with each leaf's certificate (a weight split, a
  conflict, a symmetric pair), checked by a small program, would make the code's correctness checkable without the
  prover.  It does not touch the frame model's assumptions, which is why A2 ranks above it.
* **Number theory: the asymptotic constant.**  D. Hensley, "A winning strategy at Taxman", Fibonacci Quarterly 26 (1988)
  262-270 (read): opt(n) / (n²/2) tends to a constant C with 1/2 < C ≤ 35/48, so the taxman keeps a fixed share of the
  pot; and for every prime p, L_p ≤ C ≤ L_p + 2/p, where L_p is a sum over the p-smooth numbers j of the optimal scores
  of the games on the p-smooth numbers up to j.  Nothing later improves it.  Franklín and Moniot (read) compute no value
  of opt(n) themselves (their data are this project's, to 1000), prove that their born-free matching takes at least
  1724/3375 of the pot asymptotically, and pose no open problems; nothing cites them yet.  *Transfers:* a new use for
  exact solvers, proven bounds on C from games on smooth boards (about a thousand 7-smooth numbers below 10^6), a result
  in its own right and a check of the asymptotic regime; and Hensley's blocks {kd : d p-smooth} (play in block k never
  disturbs a larger block), a decomposition of the board by the rough part of each number.  For the gap's growth,
  Landau's count of numbers with k prime factors, x (log log x)^(k−1) / ((k − 1)! log x), says how fast the movers with
  three distinct primes, where all odd cores live, thicken in frames 1-3.

Still open from [the pilots of ideas from other
fields](../notebook/ideas-from-other-fields-the-pilots-2026-10/README.md) (2026-10), measured on the matching path:

* **Knuth's estimate of a tree's size** (rank 3, not piloted): a game's cost before running it, and which units to split
  first ([its section 5](../notebook/ideas-from-other-fields-the-pilots-2026-10/README.md#5-every-idea-considered),
  "Tree-size estimation").
* **Reliability branching** (rank 4): pseudocosts in place of strong branching's exact solves, built only if they
  predict the chosen member and the forced set ([its section
  5](../notebook/ideas-from-other-fields-the-pilots-2026-10/README.md#5-every-idea-considered)).
* **The tree's growth at many workers, and the nogood period at 192 threads** (rank 5): A7.
* **The ideas considered and not piloted**, each with its cheapest experiment: [its section
  5](../notebook/ideas-from-other-fields-the-pilots-2026-10/README.md#5-every-idea-considered).

## 6. What the cloud runs should measure

The chain past 2175 and the prover's proofs of 1301-2175 were to decide C5 and A2's premise: C5 is decided, and A2's
premise needs a rerun (below).  Most of what follows needs only the runs' output files, analysed afterwards on one
thread; what needs the machine is marked.

* **The true root gap of every searched game** past 2175, from the chain's own scores
  (`examples/wall.rs nodes --nodes 1` over the results file, as in measurements 7.1): the gap's trend (C5, the two
  regimes of section 2) and the residuals of the cost model, with the thread count in the fit.  *Done* for a snapshot of
  the EC2 chain at game 3314 (509 searched games: measurements 7.4, [the gap past
  2175](../notebook/the-gap-past-2175-2026-10/README.md)).  It can be repeated on the committed chain with
  `solver/notebook/the-gap-past-2175-2026-10/gap.py`, to see whether the waves keep their level.
* **The tree's growth with threads** (on the machine): the same three dear games at 1, 16, 64 and 192 threads, to
  convert 192-thread trees to one-thread equivalents in that fit.  *Measured* for 2170 and 2172 (1 to 192 threads) and
  2184 (16 to 192): the trees differ by up to 18% with no trend in the threads, on proofs of a second or ten, not dear
  games ([scaling to 192 threads](../notebook/scaling-to-192-threads-2026-10/README.md)).  The window's speculation in
  the chain's own nodes, game by game against the scaling run's, is −0.04 ± 0.03 decades for a window of 1 or 4 (−0.11
  over 2226-2276, where the dear games lay), and the threads' −0.02: the fit takes them off.
* **The prover's run**: per game, its nodes against the solver's, and where its tree branches (members above n/2 against
  below), which is A2's premise: the share of its nodes spent on U\*.  The ratio's trend past 2000 is *measured*:
  threefold per 1000 in n ([the prover's proofs of
  1301-2175](../notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)).  The run's lines
  hold totals only, so where the tree branches needs a rerun that counts it.
* **The scaling at 192** (on the machine): effective parallelism, idle and lock time on a dear game, the nogood period
  ([ideas from other fields, section
  3](../notebook/ideas-from-other-fields-the-pilots-2026-10/README.md#3-nogoods-with-very-many-threads)), and the
  checkpoint pause at 300 s (below, "What has not been measured").  *Measured* but for the nogood period, on searches of
  up to 85 s ([scaling to 192 threads](../notebook/scaling-to-192-threads-2026-10/README.md)).
* **The fixed costs on Graviton** after round A: *measured* ([scaling to 192 threads, "For a cloud
  machine"](../notebook/scaling-to-192-threads-2026-10/README.md#for-a-cloud-machine), item 1).
* **Keep every checkpoint of a dear game that does not finish**: a resumable tree past 2175 is the test bed for a bound
  or a branching at the size that matters.

### What has not been measured

The Graviton calibration measured the speed of a Graviton3 and a Graviton4 core against the M1 Max's, the contention
among 64 one-thread copies (none), a proof's scaling to 64 threads, and the chain's games an hour on 64 threads ([the
calibration](../notebook/graviton-calibration-2026-10/README.md), with its scripts and data).  The scaling run of
2026-10-15 on a `c8g.48xlarge` (two sockets, two NUMA nodes) measured, after the inner loop's round, round A and round
B, where a thread's time goes at 16, 64, 96 and 192 threads, the window at 64 and 192 threads, the pause of a checkpoint
and the memory at 192, the fixed costs of a game on a Graviton4 core, and the chain's speed and cost a game on 64 and
192 threads ([scaling to 192 threads](../notebook/scaling-to-192-threads-2026-10/README.md)).  What neither measured:

* **Why a 192-thread search after one-thread work in the same process steps 1.6 to 1.7 times slower.**  The ladder
  showed it twice, `solve` never; the suspect is the kernel's automatic NUMA balancing.  The check (the threads' CPUs
  during such a run, and the same under `numactl --interleave=all`) takes minutes on the next 192-thread machine; it
  matters for a long `chain.sh` run on 192 threads.
* **Where the second socket's cost goes.**  At 192 threads a step is 7 to 10% dearer than on one socket; remote memory
  and the other chip's caches were not separated.
* **Large trees on many threads.**  The longest search at 192 threads was 85 s (2288).  Hours of one tree (item 2 of
  [scaling to 192 threads, "For a cloud
  machine"](../notebook/scaling-to-192-threads-2026-10/README.md#for-a-cloud-machine)) were not run, and nor was the
  size of a checkpoint at 4000 on 192 threads (its standalone search ended in 43 s, before the first).
* **The window's policy on many threads.**  Its constants (a tick of 5 ms, busy below 5% idle, giving back at 25% idle,
  10 ms to settle) were not tuned.  A window of 4 was the best of 1, 4, 16 and 32 on the games past 2225 at 64 and 192
  threads, in one run each; 2, 3, 6 and 8 were not tried there.
* **Graviton5.**  A `c9g.16xlarge` was launched for the calibration, and the spot service took it back five minutes in,
  before it ran anything.  The speed of its CPUs against Graviton4's (AWS says up to 25 %) is unmeasured.
* **PGO and `target-cpu=native` on Graviton.**  `solver/tools/pgo.sh` exists and was run on the M1 only; no gain from
  profile-guided builds is claimed for Graviton, and the calibration built with `native` without comparing generic code.
* **A spot interruption, and the scripts on Ubuntu.**  `spot-watch.sh` has not seen an interruption, so neither has the
  restore after one (the cloud run's restore followed a stop on purpose): no notice came in the cloud runs, and the
  scaling run's instance had its notice two minutes before the end, with no `spot-watch.sh` running.  `scaling.sh` has
  not run on EC2, and its first run, on a small instance, is its test; nothing has run on Ubuntu ([the RUNBOOK, "What
  has run where"](RUNBOOK.md#what-has-run-where)).
