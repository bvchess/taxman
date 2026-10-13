# Ideas from other fields: a ranked proposal, with pilot measurements

The solver already borrows from SAT solving (conflict analysis, nogoods, forgetting them) and from MIP (a
relaxation bound, strong branching), and those borrowings paid a great deal ([EXPERIMENTS.md](EXPERIMENTS.md)).
This page asks what else to borrow, and answers with measurements rather than opinions: every idea that looked
promising got a cheap pilot (a counter, a fixture, a node-limited or proof-only run), and the ones that did not
survive their pilot are recorded with the number that killed them, as in EXPERIMENTS' "Tried and not kept".

* [1. The recommended order](#1-the-recommended-order)
* [2. Where the time goes in the hardest games](#2-where-the-time-goes-in-the-hardest-games)
* [3. Nogoods with very many threads](#3-nogoods-with-very-many-threads)
* [4. The pilots](#4-the-pilots)
* [5. Every idea considered](#5-every-idea-considered)
* [6. The tools, and which commit holds what](#6-the-tools-and-which-commit-holds-what)

How it was measured.  The machine is the 10-core M1 Max laptop, shared with other jobs the whole time (its load
average stayed between 4 and 8), so every pilot uses at most 4 cores at a time and reports counts (nodes, lookups,
nogoods learned, cuts) and cycles of the process before wall time; wall times are quoted only where nothing else
says what something costs, and only as ratios within one run.  All searches are strict (`revise_freedoms=off`) and,
unless they say otherwise, *proof-only*: seeded with the game's own optimum (`ladder --seed-opt`), which makes the
node count of a one-thread search exact and leaves the tree to the bound and the branching, the parts these ideas
change.  "Fast set" is EXPERIMENTS' 880, 936, 1050, 1100, 1170, 1184 seeded from n − 1.  What matters at the
frontier is the slope of the cost curve ([bench/curve/README.md](../bench/curve/README.md): ×10 per 55-70 in n), so
each result says whether its effect grows with n where that could be measured.

## 1. The recommended order

| rank | idea | borrowed from | sound? | pilot result | expected payoff | cost |
|---|---|---|---|---|---|---|
| 1 | Reuse the answer of a game whose search space an earlier game already searched | memoization: dynamic programming, the component cache of #SAT, transposition tables | proven, given the earlier game's proof (a new proof kind) | 29.7% of the search time of the strict chain 1232-1300 (851 of 2866 s), 16.6% of 1201-1231, is spent re-searching a space already solved | the same fraction of the chain, at once; constant factor, not slope | small: a digest per game, a proof kind, the verifiers |
| 2 | A Lagrangian bound over the nogoods | Lagrangian relaxation (MIP), core-guided bounds (MaxSAT); already in `evidence/` | sound (weak duality) | proof nodes −19% to −67% with multipliers fitted once at the root to the nogoods of a short warm-up (1184: 168k → 55k; 1215: 5.1-5.7M → 3.9M; 1225: 9.5-10.1M → 7.6M; 1250: 3.1M → 2.1M) | up to 1.3-3× fewer nodes, if a second matching can be kept incrementally for less than that | large: weighted incremental matching with weight raises, multiplier fitting, a nogood source |
| 3 | Knuth's estimator of a tree's size | backtrack-search estimation (Knuth 1975), cube-and-conquer's lookahead | not a pruning: nothing to prove | not piloted | a game's cost before running it (budgets for the 192-core machine); split work by estimated size | small to medium |
| 4 | Reliability branching (pseudocosts) for strong branching | MIP branching | sound (any order explores every set) | strong branching is 28-34% of the time at the frontier (section 2) | up to a quarter of the time, if the tree does not grow | medium |
| 5 | Less fragmentation at many workers | parallel B&B, work stealing | sound (units are caches) | 32-128 emulated workers take 10-20% more nodes than 1-8 on the proofs of 1215 and 1225, and sharing nogoods does not bring it back (section 3) | 10-20% at 192 threads | small to medium |
| — | Periodic clearing of each thread's nogoods | CDCL clause-database reduction | (keeps) | still right at 192 threads (section 3) | — | — |

Measured and not kept (section 4 has the numbers): reduced-cost fixing (it cannot change a tree: the matching
relaxation is integral), component decomposition (the residual problem never splits), disjoint conflicts from the
dual (MaxSAT's disjoint cores; 1% fewer nodes for 12% more cycles), the most recent conflict instead of the one
with the largest member (VSIDS-like activity; no gain), sharing nogoods between threads, keeping the short ones
across a clearing, and a portfolio of branchings for finding incumbents (8 of the 11 hardest games 1232-1300 start
at their optimum).

The order of experiments: do (1) now, it is engineering with a known payoff.  Then (2), in two steps: first the
incremental weighted matching and a fixed warm-up fit, measured on the proofs of 1184, 1215, 1225 and 1250 against
the numbers here (the pilot already gives the node counts; the step decides the cost per node); then multipliers
refitted as the search moves (the per-node fits of the pilot cut 1.6-2.4 times as many nodes as root-fitted ones,
section 4.2).  (3) is cheap and pays on the cloud regardless of the rest; do it before renting the machine.  (4) and (5)
are second-order and can wait for (2), which changes the time split they would act on.

## 2. Where the time goes in the hardest games

The three slowest games of the strict chain 1232-1300 (`results/strict.jsonl`: 1232 439 s, 1287 273 s, 1299 265 s
on 6 threads), each run as the chain runs it (`taxman solve N --strict`, its predecessor from a copy of the strict
results truncated before N) for 40-70 s on 4 threads and sampled every millisecond by macOS `sample`.  Each sample's
return addresses were symbolicated with their inlined frames (`atos -i`), so that code the compiler inlined into the
step is charged to the function it came from: the *kind* of work is the innermost function of the crate, the
*purpose* the innermost search function above it.  Shares of busy samples (the idle coordinator thread excluded):

| kind of work | 1232 | 1287 | 1299 |
|---|---|---|---|
| matching repairs (Hungarian searches: `Search::run`, `add_inner`, the radix heap) | 66.8% | 57.0% | 56.4% |
| peels and promotions (frame checks of children) | 16.9% | 21.3% | 21.5% |
| the node's own work (candidate lists, bit sets, the step) | 6.6% | 7.3% | 7.5% |
| bounds from the potentials (`bound_excluding`) | 4.3% | 4.8% | 4.8% |
| copies (matching levels, frame sets) | 3.0% | 3.8% | 4.2% |
| nogood table (scans for lookups, rule-outs) | 2.2% | 5.1% | 5.0% |
| conflict minimization (peels of a core) | 0.2% | < 0.5% | < 0.5% |
| engine | < 0.1% | < 0.1% | < 0.1% |

| purpose | 1232 | 1287 | 1299 |
|---|---|---|---|
| strong branching (exact bound without each member of the conflict) | 33.6% | 27.8% | 28.8% |
| making the child (its promotion, candidate list, rule-outs) | 19.7% | 26.0% | 26.2% |
| refinement (bounds of the branches of the conflict) | 20.4% | 18.4% | 18.9% |
| the matching bound of a node or child (`check`) | 15.8% | 15.9% | 14.2% |
| forced members and the conflict (`Refiner::top`) | 5.8% | 6.3% | 6.0% |
| conflict lookup, or computation when the table has none | 3.9% | 5.0% | 4.7% |

What follows for the ideas:

* Three quarters of the matching work is *lookahead* (strong branching and refinement), not the node's own bound.
  Replacing strong branching's exact solves by estimates (reliability branching, 4) acts on 28-34% of the
  time; anything that makes the node's bound tighter (a Lagrangian, 2) can remove lookahead altogether where it cuts.
* Conflict computation is under 1%: the nogood table answers 85-93% of lookups (section 3), so cheaper conflicts
  would not pay, and neither would anything that only saves re-deriving nogoods.
* The nogood table is 2-5%, so its policy matters only through the tree it causes, not its own cost (section 3).
* The engine is invisible.  Its scaling is a question of tree size and fragmentation at many workers, not overhead.
* Finding the incumbent is not where most of the time goes: in 8 of the 11 slowest games 1232-1300 the seed (the
  previous game's promotions, improved by the local search) is already optimal; 1290 and 1300 reach their optimum
  after 14 and 19 s on 4 threads, of runs that take 142 and 179 s on 6; only 1232 and 1287 had not reached theirs
  after 45 s (1232 at 37005 of 37113, 1287 at 38936 of 38977).  The frontier's cost is mostly the proof.

A fourth game, 1295 (25 s), gives the same split to within two points (repairs 58.1%, peels 20.8%; strong branching
29.0%, making children 25.5%, refinement 19.8%).  `tools/profile/profile.sh N SECONDS THREADS OUTDIR` reproduces any
of them; the probe (`examples/perf.rs`) agrees on the hot kernels (a strong-branching exclusion costs 9-12 k cycles,
a step 16-17 k).

## 3. Nogoods with very many threads

The question: each thread keeps its own table and clears it every 50,000 of its steps; at 192 threads is that still
right, or should threads share nogoods, or keep the good ones?  It was measured on this laptop by *emulating* many
workers on 4 cores: `EngineConfig::cores` (new, `ladder --cores C`) lets at most C of the T worker threads search at
once, taking turns every 1024 polls in ticket order, so that 64 or 128 workers split, steal and learn exactly as
they would on 64 or 128 cores, in counts if not in time.  A study of the tables (`ladder --nogood-study`, off the
solver's path; the probe's fingerprint is unchanged) counts lookups, scans, hits, rule-outs, the size and age of
every nogood used, and, through a registry of all nogoods, how often a worker learns one that another worker holds
at that moment.

**How useful an old nogood is.**  Uses (a lookup answered or a candidate ruled out) per 1000 steps a nogood spends in
the table, by its age, on the proof of 1215 never forgetting (one thread):

| age in steps | 1 | 16 | 256 | 1k | 4k | 16k | 64k | 256k | 1M | 4M |
|---|---|---|---|---|---|---|---|---|---|---|
| uses per 1000 nogood-steps | 170 | 49 | 22 | 14 | 8.7 | 7.3 | 6.8 | 3.6 | 3.4 | 3.3 |

Usefulness drops by 20× in the first ten thousand steps and then flattens: an old nogood is still used now and then,
so the question is what it costs to keep.

**What a nogood costs.**  Lookups and rule-outs scan the whole table (`ideas lookup-cost`, the probe's nodes of 1215,
cycles per call):

| nogoods in the table | 100 | 200 | 400 | 800 | 1600 | 3032 |
|---|---|---|---|---|---|---|
| lookup | 757 | 1040 | 1495 | 2433 | 4198 | 8631 |
| rule-out | 292 | 392 | 566 | 892 | 1559 | 3127 |

About 2.6 cycles per nogood per lookup and 1 per rule-out, with 0.7 lookups and 0.5 rule-outs per step: a table
of S nogoods costs about 450 + 2.3 S cycles of a 16-20 k-cycle step.  The proof of 1215 on one thread: forgetting
every 50,000 steps the tables average 125 nogoods at a lookup and the search takes 5,485,294 nodes and 112 G
cycles; never forgetting they average 547 and it takes 5,446,460 nodes (−0.7%) and 134 G cycles (+19%).  What
forgetting costs instead is re-deriving: 17,556 nogoods learned against 621, each a conflict computed (50-90 k
cycles), about 1.7 G cycles, a twelfth of what the bigger table costs.

**Duplication across threads.**  The share of the nogoods learned that another worker held at that moment:

| proof | 4 workers | 16 | 32 | 64 | 128 |
|---|---|---|---|---|---|
| 1215 | 42.5% | 75.0% | | 88.7% | |
| 1225 | 32.5% | | 78.3% | | 88.4% |

Nearly all of it at many workers; but learning is cheap (4-7 nogoods per 1000 steps, rising from 4.05 at 4 workers
to 6.81 at 128 on 1225 because of the re-derivations, about 1.4% of the cycles), and the tables grow only a little
with it (an average of 153 nogoods at a lookup at 4 workers, 228 at 128).

**Policies, in nodes** (proofs; many-worker runs vary by ±10% from run to run, so repeats are given):

| proof, workers | clear every 50k (as now) | share every nogood | share those of ≤ 3 members | keep those of ≤ 3 members at a clear |
|---|---|---|---|---|
| 1215, 16 | 5.94M, 5.92M, 5.54M | 6.10M (tables 434 at a lookup) | | 5.50M |
| 1215, 64 | 5.88M, 5.76M, 6.05M | 5.99M (tables 601) | | 5.71M |
| 1225, 32 | 13.65M, 11.05M, 10.30M | 13.35M (tables 773) | 11.56M | |
| 1225, 128 | 11.46M, 10.92M | 14.38M (tables 1214) | 12.89M | |

Sharing does not bring the node count down, and sharing everything makes the tables 3-8 times as large and the
trees larger: most imported nogoods are dropped at the next clear without ever having been used, and a large table
picks its conflict (the smallest, then the one with the largest member) from more candidates, which by
EXPERIMENTS' "Carrying nogoods over" and these runs gives no better branching.  Keeping the short nogoods changes
nothing measurable; it could not anyway: the nogoods of the search are *large* (on 1215, 11,483 of 17,556 learned
have 8 or more members, 1,945 have 2), and the large ones answer most lookups (1.6 M of 3.3 M) and rule out most
candidates (2.7 M of 4.7 M).  "Short" or "low glue" is not where the value is here; size is the natural stand-in for
glue (the members of a nogood come from at most two adjacent frames), and recency is what the period keeps.

**Answer.**  Periodic clearing of per-thread tables is still right at 192 threads, for three reasons measured here:
an old nogood earns little (a few uses per thousand steps) against a scan cost on every lookup; more nogoods do not
make smaller trees (never forgetting: same nodes; sharing: more); and re-deriving what another thread knows costs
about 1% of the cycles.  The regime does not change at 192 threads: a frontier game of 10^8-10^9 nodes gives each
of 192 threads 0.5-5 M steps, which is the 1215 proof on 1-16 workers, where clearing behaves as on one thread.  Two
small adjustments are worth a test on the big machine, not more: a period of about 30,000 steps keeps the tables at
the size they have at 4 workers (they learn ~1.7× faster at 128), and the time to full occupancy and the unit count
should be watched, because what does grow with the number of workers is the tree, by 10-20% from 4-8 to 32-128
workers on 1225 (9.3-10.1 M to 10.3-13.6 M nodes; the committed `bench/curve/scaling.jsonl` has 9.45 M at 4 and
9.30 M at 8), with ten to twenty-five times as many unit starts (385 at 4 workers, 4,000-10,000 at 32-128), each a
rebuild that forgets the conflicts known along its path.  That is idea 5.

## 4. The pilots

### 4.1 Memoizing searches that repeat (rank 1)

Many consecutive games have exactly the same search space: the same frames (moves, factors, freedoms), the same
candidates and the same `max_total`, so the same best promotion set; only the cap `n + score(n − 1) − in_the_bag`
differs, and when it is above the optimum the search runs a full proof again (EXPERIMENTS noticed this in 1201-1231).
`ideas repeats A B` hashes what a strict search depends on for every searched game; joined with the committed
`results/strict.jsonl`:

* 1232-1300: 29 of the 60 searched games repeat an earlier space; their searches took **851 s of the 2866 s** of the
  chain (29.7%): 1299 repeats 1296 (265 s), 1293 repeats 1292 (207 s), 1271 repeats 1269 (75 s), 1267 repeats 1265
  (64 s), 1263 and 1261 repeat 1260 (51 s and 48 s), ...; the others reached the cap at once.
* 1201-1231: 46 s of 276 s (16.6%).

Soundness: the search space is a function of the root frames, the candidates and `max_total` (the model's graph is
the maximal-factor relation restricted to those numbers), so its best promotion sum is the same for both games; the
earlier game's proof (exhausted, or the cap) establishes that sum, and the later game's score is its own
`in_the_bag` plus it, replayed by the referee as always.  It needs a proof kind ("same search as game m", resting
on m's proof and taint), the digest in the result line or recomputed when the range starts, and the two verifiers.
The payoff is a constant fraction of the chain, at no risk, and it multiplies with everything else here.
Implemented as the tactic and proof kind `same_search`
([EXPERIMENTS, "Same search"](EXPERIMENTS.md#same-search-2026-10)).

### 4.2 A Lagrangian bound over the nogoods (rank 2)

*Mechanism.*  The matching bound knows no nogood.  Every completion `F` of a node with promotions `P` promotes at most
`|N − P| − 1` members of each nogood `N`; dualizing these inequalities with multipliers `λ_N ≥ 0` gives, for every
`λ`, the bound `max_M Σ_{x ∈ Q(M)} (x − Σ_{N ∋ x} λ_N) + Σ_N λ_N (|N − P| − 1)`: a matching with each
candidate's weight lowered by the multipliers of its nogoods, plus a constant
(`src/search/lagrange.rs` on the branch `ideas-pilots`).  A nogood with a member that has left the candidates cannot
be broken any more and is left out (multiplier 0 at that node).  It is the evidence crate's second bound
([evidence/README.md](../evidence/README.md), section 5: 3.3× fewer nodes at n = 793 for its model-free search),
brought to the frame solver, and MaxSAT's core-guided bounds in spirit: each nogood is a core, and the multipliers
say how much each costs.

*Soundness.*  Weak duality, for any `λ ≥ 0` and any set of valid inequalities; it needs no proof beyond the
validity of the nogoods, which the search already rests on.

*Pilot.*  Multipliers fitted once at the root (200 subgradient steps, Polyak's step towards `need`, in a second)
to the nogoods of a short warm-up (a node-limited proof of the same game, `--dump-nogoods`), then the bound tried
at every node and child where the matching bound and refinement did not cut, solved from scratch by the incremental
solver on a reweighed copy of the graph (`MatchGraph::set_weights`, `ladder --lagrange FILE --lagrange-depth 100`):

| proof (seed-opt) | threads | nodes without | nodes with | change | nogoods (warm-up) | bounds computed | cuts |
|---|---|---|---|---|---|---|---|
| 880 | 1 | 31,416 | 30,167 | −4% | 340 (whole proof) | 18,508 | 772 |
| 936 | 1 | 59,055 | 51,222 | −13% | 469 (whole proof) | 33,305 | 3,134 |
| 1100 | 1 | 55,543 | 40,960 | −26% | 227 (whole proof) | 26,581 | 3,219 |
| 1170 | 1 | 27,842 | 12,646 | −55% | 84 (whole proof) | 8,545 | 1,347 |
| 1184 | 1 | 167,960 | 54,795 | −67% | 140 (first 20k nodes) | 37,827 | 7,084 |
| 1215 | 4 | 5.13M, 5.68M | 3.92M | −24 to −31% | 639 (first 300k) | 2.66M | 371k |
| 1225 | 4 | 9.45M, 9.85M (8 threads: 9.30M, 10.11M) | 7.64M | −19 to −24% | 1,255 (first 500k) | 5.20M | 804k |
| 1250 | 4 | 3.17M, 3.11M | 2.10M | −33% | 582 (first 300k) | 1.50M | 289k |

(1184 with all 169 nogoods of its whole proof: 55,303, the same as with the warm-up's 140: the warm-up is enough.)
The nodes of the warm-up are not counted: 20 k to 500 k, 5-12% of each proof.

What the pilot also showed:

* *The multipliers of dead nogoods must go.*  Keeping every nogood's multiplier whatever happened to its members
  (`--lagrange-static`, a bound that stays valid and whose weights never change) gives 1184 129,095 nodes (−23%)
  instead of 54,795: most of the effect comes from dropping the nogoods a node has already broken.
* *Refitting pays.*  On the probe's search nodes (`ideas lagrange`; units of the proofs, shallow, with a mean gap of
  145 at 936 and 260 at 1215 between the matching bound and what they need), the root's multipliers lower the bound
  by 66 and 69 on average and cut 34 and 21 of 300 nodes (refinement cuts 27 and 10); refitted at each node (20
  steps from the root's) they lower it by 110 and 130 and cut 81 and 33.
* Across 1100-1250 the effect is −19% to −67% with no clear trend in n: it varies more from game to game than with
  n.  The frontier needs its own measurement (step (a) below gives it at no extra cost).

*Cost, and the experiment.*  The pilot's evaluation is a matching solved from scratch at each check (0.7 per node),
which makes it 7× slower per node at 1184; useless as it is, and not meant to be used.  The real cost is that of a
second incremental solver kept along the path next to the first: about what the node and child checks cost today
(14-16% of the time, section 2), plus repairs when a dead nogood's multiplier is dropped (a weight *rise* on its
other members: raise the potential of the vertex by the same amount, and unmatch it if it was paying for a mover
or free, so that the existing Hungarian repair restores optimality).  So: (a) build the weighted incremental solver
(weights per vertex are in place: `MatchGraph::set_weights`; weight rises are the new part), measure cycles per
node on the proofs above, and accept if nodes × cycles drops; (b) choose the nogood source for production: the
nogoods of game n − 1 (they survive into game n almost all, EXPERIMENTS "Carrying nogoods over") or a warm-up;
(c) refit: at every clearing of the table (fold the nogoods about to be forgotten into the multipliers instead), or
at unit starts.  A tactic only once (a) pays; it is sound, so it would not taint.

### 4.3 Reduced-cost fixing (killed)

From MIP: a candidate `x` that the heaviest matching does not promote has a reduced cost read off the potentials
(`MatchBound::bound_including`: `value − min over its factors f of (y_x + y_f − x)` bounds every matching that
promotes it); if that is below `need`, no completion that beats the incumbent promotes `x`, here or below, and it
can leave the candidates.  It fires a lot: 70.9 of 159 candidates outside `Q*` per node at 936 and 61.6 of 212 at
1215 (`ideas fixing-exact`, every one checked against the matching solved with `x` promoted).  And it changes no
tree: the fast set, the proofs of 1215 (5,485,294 nodes) and 1025 (5,570,602) came out node for node the same
(tactic `rc_fixing` in commit 5dff240, taken out in 6967c6c).  The reason is general: the matching relaxation is
integral, so a fixed candidate is never in the heaviest matching of a node that stays open below (if it were, that
matching would be a matching of the node promoting `x` worth at least `need`), and removing a candidate the optimum
does not use changes neither the bound nor the conflicts.  In MIP reduced-cost fixing helps because LP solutions are
fractional; here the forced members (`fix_forced`) are the useful half of the same dual information.

### 4.4 Component decomposition (killed)

From #SAT model counting and AND/OR search: if the problem left at a node splits into parts that share no frame
component, the best completion is the sum of the parts' best, and searching them apart turns a product into a sum.
A frame is winnable exactly when each connected component of its move-factor graph is, and an undecided candidate
links the component where it is a factor to the one it would pay in, so the parts are the connected components of
that graph (`ideas components`).  At the root of 936, 1000, 1100, 1215, 1232, 1287 and 1300, and at all 600 search
nodes of the probe's fixtures (936 and 1215), there is exactly one component holding candidates.  Nothing to split.

### 4.5 Disjoint conflicts from the dual (killed)

From MaxSAT's disjoint-core bounds (and the evidence crate's "conflict-packing cut"): two disjoint conflicts within
`Q*` must each lose a member, and lowering the potentials of both excluded members (and their partners) one after
the other keeps the dual feasible, so their drops add up.  `MatchBound::bound_excluding_all` does that; checked
never below the exact value on 16,800 pairs (26-33 above it on average), and tried where refinement did not cut:
the fast set took between 2.7% fewer and 0.3% more nodes, and 12% more cycles (commit e248b18, taken out in
bc6c716).  The frame solver's bound is already tight where it matters (forced members and one level of refinement
catch nearly all of what pairs of conflicts would), unlike the model-free search's, whose gap is about 1,170 at
n = 1000.

### 4.6 Recency in the choice of conflict (killed)

From CDCL's activity heuristics (VSIDS, CHB): the nogood table returns the smallest conflict within `P ∪ Q*`, then
the one with the largest member, then the first learned.  Preferring the most recently learned among the smallest
(`--prefer 2`, commit 80f448c, taken out in bc6c716): 880 −1.2%, 936 +3.6%, 1050 +20%, 1100 +5%, 1170 −0.3%, 1184
+7% nodes, the proofs of 1215 +0.7% and 1025 +6.7%; keeping the largest member first and only then the most recent
(`--prefer 1`): −2.8% to +9.4%, no direction.  Which equally small conflict to branch on moves a game's node count by
±10% either way, and recency does not choose better than size and the largest member.

### 4.7 A portfolio for finding incumbents (weak)

From parallel SAT portfolios: run some threads with another branching that finds good sets early
(`conflict-largest` follows the dive).  Section 2 shows the frontier's cost is the proof, so there is little to win;
on the games that do improve their start (4 threads, 45 s, time to the best set found): 1290 at 9.3 s against
13.9 s, 1287 at 13.7 s against 26.6 s (the same set, 41 below the optimum), 1300 not found in 45 s against
19.2 s, 1232 found by neither.  Not worth a mechanism; worth remembering for games like 992 and 1025 whose optimum
the search reaches late, if they come back at the frontier.

## 5. Every idea considered

Each with what it would mean in this solver, why it might help here, whether it is sound, its cost, and the cheapest
experiment.  "Pilot" refers to section 4.

**From CDCL SAT**

* *LBD / glue-based clause-database reduction* instead of periodic clearing: keep nogoods by quality.  Here a
  nogood's members come from two adjacent frames, so size stands in for glue, and the large nogoods are the useful
  ones (section 3).  Sound (a cache).  Measured: keeping those of ≤ 3 members changes nothing.  Not worth more.
* *Clause minimization*: already done; a nogood is a minimal subset keeping a frame of the root unwinnable
  (`Model::nogood`).
* *Restarts with phase saving*: in a branch-and-bound proof with the optimum known the tree is fixed by the bound,
  so a restart only loses work; their use in SAT (escaping heavy tails while looking for a solution) is the
  incumbent-finding phase, which is not where the time goes (section 2).  Not tried.
* *Activity-based branching (VSIDS, CHB)*: pilot 4.6, no gain.  Strong branching on a conflict of 2-3 members is a
  better-informed choice than activity, and EXPERIMENTS records that cheaper estimates traded nodes for time at
  about even.
* *Chronological backtracking / backjumping*: in CDCL, a conflict tells how far back the reason lies.  Here a node
  is closed by a bound, and the reason for a bound failure involves the incumbent and the whole matching, not a few
  decisions; learning "bound nogoods" (MIP's conflict analysis of infeasible LPs) is possible but costly, with modest
  gains in MIP solvers.  Not tried; low expected payoff.

**From parallel SAT**

* *Clause sharing filtered by size or LBD*: section 3; sharing every nogood makes trees larger and tables 3-8×
  larger; sharing the short ones does nothing.
* *Portfolio vs divide and conquer*: the engine divides; a portfolio for incumbents is pilot 4.7.
* *Cube-and-conquer with lookahead splitting*: pre-split the root into many cubes by lookahead so that 192 threads
  start at once with balanced work.  The engine's splits already fill all workers within 3-8 ms (`ladder` "full"),
  and its tail is short; what cubes could add is balance by *estimated* size, which is idea 3.  Not tried.

**From MaxSAT**

* *Core-guided bounds*: the conflicts are cores; refinement by one conflict is a one-core disjunctive bound;
  disjoint cores are pilot 4.5 (killed); the Lagrangian over all cores is pilot 4.2 (rank 2).

**From MIP branch and bound**

* *Reduced-cost fixing*: pilot 4.3, killed by integrality.
* *Pseudocost / reliability branching* (rank 4): estimate each candidate's exclusion drop from the drops measured
  before, and solve exactly only for members whose estimate is unreliable (few observations).  Strong branching is
  28-34% of the time at the frontier.  Sound.  The risk is the forced members: strong branching's exact solves also
  find members whose exclusion drops the bound below `need`, which `fix_forced` turns into single-child nodes, and
  EXPERIMENTS found that deciding by the potentials alone traded nodes for time at about even.  Experiment: record,
  per candidate, the exact drops strong branching computes (a counter, no change), and measure how well a pseudocost
  predicts the chosen member and the forced set on the proofs of 1215 and 1225; build only if it predicts > 90%.
* *Best-bound vs depth-first node selection*: with the optimum in hand from the start (section 2), the proof tree is
  the same in any order (bar the nogoods learned), and depth-first keeps memory bounded.  No gain to find.
* *Cutting planes*: the matching LP is integral, so the only useful cuts are inequalities the matching does not
  imply: the nogoods themselves, which the Lagrangian dualizes.  Within rank 2.
* *Lagrangian bounds*: rank 2.

**From constraint programming**

* *Dominance rules*: a candidate `y` dominated by a larger `x` (every frame-level role `y` can play, `x` can play,
  and promoting `x` instead of `y` keeps every frame winnable) would allow "if `x` is excluded, exclude `y`".  In
  Taxman Mini a move takes *all* its factors still in the frame, so having more maximal factors is not monotone
  (more ways to be paid, but more taken), and a sound dominance needs a proof per pair; the symmetric case (two
  candidates with the same neighbourhoods) is rare, since a number's maximal factors determine it (up to those
  missing from the frame).  High cost, unknown payoff; not tried.  A first, cheap look would count pairs with
  nested neighbourhoods in a frame.
* *Symmetry breaking*: no symmetries found (as above).
* *Nogood recording on restarts*: subsumed by the existing nogoods.

**From #SAT and model counting**

* *Component decomposition*: pilot 4.4, killed.
* *Component caching*: within one search, the space never splits; across games it is rank 1, where the whole search
  repeats.

**From game-tree search**

* *Transposition tables*: the conflict branching partitions the promotion sets, so one search never meets the same
  node twice; across games, rank 1.  Caching frame checks (14-17% of peels repeat one exactly, 5% one of the last
  eight; EXPERIMENTS) bounds the gain to a few percent of the 17-22% the peels take.
* *Move ordering*: strong branching is the ordering; see pseudocosts.

**Tree-size estimation (Knuth 1975)** (rank 3).  Walk from the root to a leaf choosing a child at random; the product
of the numbers of children along the way, summed over the depths, is an unbiased estimate of the tree's size, and a
few thousand walks give its order of magnitude.  Here a "child" is a node's surviving child (a node's loop is run to
the end, its children checked and not explored), and steps rather than tree nodes are counted (each node adds its
loop's steps).  Uses: the cost of a game before running it (the cloud budget is ×10 per 55-70 in n, with games of a
band varying ×3-×18, bench/curve), and, inside a run, which units to split first.  Not a pruning, so nothing to prove.
Experiment: a probe searcher that expands a node fully and descends into one surviving child at random (units can
be rebuilt from `(promoted, cands)`, as the engine does), run on the proofs of 1215, 1225 and 1250 whose sizes are
known; report the median and spread of the estimate against the truth for 100, 1,000 and 10,000 walks.  The nogood
tables make the real tree history-dependent, so the walks should run with empty tables and be compared with the
real search, not with a tree the walks define.

**Engine-side, for many cores** (rank 5).  The tree grows by 10-20% from 4-8 to 32-128 emulated workers on 1225, with
ten to twenty-five times as many unit starts, and sharing nogoods does not bring it back, so the suspect is what a
rebuilt unit forgets: the conflicts known along its path (`known`), the kept strong-branching level and the
verdict of its first check.  Experiment (with `--cores`): count, per unit start, how often the first steps recompute a
conflict the path had known; then try handing a unit its path's known conflicts (they are facts of the state: valid
for any node below), or answering split requests with deeper units.  Soundness is unaffected either way (units are
caches).

## 6. The tools, and which commit holds what

The pilots' code is on the branch `ideas-pilots`, not on `master`: it hooks the searcher's inner loop (the probe
measured 0.5% more cycles with every pilot off) and is research scaffolding, not a tactic.  `master` has this page
and `tools/profile/`, which needs only the solver.  The commits below are on that branch; a pilot that graduates
(rank 2 first, once its incremental matching pays) comes to `master` as a tactic, rewritten for it.

| commit | what |
|---|---|
| 5dff240 | `examples/ideas.rs` (the pilots' harness: `components`, `fixing`, `fixing-exact`), `examples/common/fixtures.rs` (shared with the probe), `MatchBound::bound_including` and `fixed_out`, and the tactic `rc_fixing` (pilot 4.3) |
| 6967c6c | the tactic `rc_fixing` taken out again (it changes no tree); the measurement and the bound stay |
| e248b18 | `EngineConfig::cores` (many workers on few cores; a test in `tests/engine.rs`), the nogood study (`search::NogoodStudy`, tracking in `model::Nogoods`, `ladder --nogood-study --forget --keep --share --cores --node-limit`), and the disjoint-conflicts pilot (4.5) |
| 80f448c | the Lagrangian pilot (`search::lagrange`, `MatchGraph::set_weights`, `matchbound::match_bound_weighted`, `ladder --lagrange --lagrange-depth --lagrange-steps --lagrange-static --dump-nogoods`, `ideas lagrange`), the recency pilot (4.6), `ideas repeats`, `ideas lookup-cost`, and the cycle counters shared by the probe and the pilots (`examples/common/counters.rs`) |
| bc6c716 | the pilots that did not pay taken out (4.5, 4.6) |
| 7c1ff92 | `tools/profile/` (the profile of section 2), and this page |

None of this is on the solver's default path or changes a tree: `taxman solve` and its tactics are as they were, the
probe's fingerprint is unchanged (6197a47eae6cf8c8, and its cycles within noise), and `cargo test --release` passes.
The pilots are search-space fields set by the ladder (`SearchSpace::nogood_keep`, `nogood_share`, `nogood_study`,
`lagrange`), not tactics, because none of them is ready to be one; rank 2 becomes a tactic once its incremental
solver pays (4.2).

To reproduce (from `rust/`, after `cargo build --release --examples`; `K=results/strict.jsonl`):

```
ladder --threads 1 --seed-opt --known $K --tactic revise_freedoms=off --nogood-study [--forget 0] 1215
ladder --threads 4,16,64 --cores 4 --seed-opt --known $K --tactic revise_freedoms=off --nogood-study [--share 64] 1215
ladder --threads 1 --seed-opt --known $K --tactic revise_freedoms=off --nogood-study --node-limit 20000 \
    --dump-nogoods /tmp/n1184.nogoods 1184
ladder --threads 1 --seed-opt --known $K --tactic revise_freedoms=off --lagrange /tmp/n1184.nogoods \
    --lagrange-depth 100 1184
ideas repeats 1201 1300
ideas components 936 1215 1232
ideas fixing-exact 936 1215
ideas lagrange --nogoods /tmp/n1215.nogoods --iterations 20 1215
ideas lookup-cost --nogoods /tmp/n1215.nogoods 1215
```

The profile of section 2: `tools/profile/profile.sh 1232 40 4 /tmp/prof` (macOS only: `sample` and `atos -i`; the
attribution is `tools/profile/attribute.py`, standard library only).
