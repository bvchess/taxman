# Ideas from other fields: the pilots (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  What else could the solver borrow from SAT solving, MaxSAT, MIP, constraint programming, #SAT and
game-tree search, beyond the conflict analysis, nogoods, relaxation bound and strong branching it has?  **Answer.**  Two
ideas paid: reusing the result of a repeated search (29.7% of the search time of the chain 1232-1300 went to games
whose search space an earlier game had solved) and a Lagrangian bound over the nogoods (proofs 1184-1300 take 2-6 times
fewer cycles).  The other pilots each failed on a number, and clearing each thread's nogood table every 50,000 steps
stays right at 192 threads.

## Method

Every idea that looked promising got a cheap pilot: a counter, a fixture, a node-limited or proof-only run.  The pilots
ran on the 10-CPU M1 Max laptop, shared with other jobs (load average 4 to 8), so each used at most 4 CPUs and reports
counts (nodes, lookups, cuts) and the process's cycles before wall time.  All searches used only proven tactics
(`revise_freedoms=off`, a tactic since removed) and, unless stated otherwise, were *proof-only*: seeded with the game's
own optimum (`ladder --seed-opt`), which makes the node count of a one-thread search exact and leaves the tree to the
bound and the branching, the parts these ideas change.  The *fast set* is 880, 936, 1050, 1100, 1170 and 1184, each
seeded from n − 1.  The [cost curve](../benchmarks-the-probe-pgo-and-scaling/cost-curve.md) rises ×10 per 55-70 in n, so
each result says whether its effect grows with n where that could be measured.  The probe's tools are in
[performance.md](../../../docs/design/performance.md).

The pilots' code hooked the searcher's inner loop (0.5% more cycles on the probe with every pilot off) and was research
scaffolding, so it was discarded after the study.  Each pilot below says what it computed, on which games and with which
settings, so that it can be rebuilt.

To study many threads on four CPUs, `EngineConfig::cores` (`ladder --cores C`) lets at most C of the T worker threads
search at once, taking turns every 1024 polls in ticket order.  Then 64 or 128 workers split, steal and learn as they
would on that many CPUs, in counts if not in time.

## Results

### 1. The recommended order

| rank | idea | borrowed from | pilot result | outcome |
|---|---|---|---|---|
| 1 | Reuse the answer of a game whose search space an earlier game already searched | memoization, #SAT's component cache | 29.7% of the chain 1232-1300's search time re-searches a solved space (4.1) | kept as `same_search` |
| 2 | A Lagrangian bound over the nogoods | Lagrangian relaxation (MIP), core-guided bounds (MaxSAT) | proof nodes −19% to −67% (4.2) | kept as `lagrange` |
| 3 | Knuth's estimator of a tree's size | backtrack-search estimation | not piloted | open |
| 4 | Reliability branching (pseudocosts) for strong branching | MIP | strong branching is 28-34% of the time in the hardest games (2) | open |
| 5 | Less fragmentation at many workers | parallel B&B | 32-128 emulated workers take 10-20% more nodes than 1-8 (3) | open |
| — | Periodic clearing of each thread's nogoods | CDCL clause-database reduction | still right at 192 threads (3) | kept |

The study ranked 1 and 2 first, 3 before renting the 192-CPU machine, and 4 and 5 after 2, which changes the time split
they would act on.  Sections 3 and 4.3 to 4.7 measured and rejected the rest.

### 2. Where the time goes in the hardest games

The three slowest games of the chain 1232-1300 of commit bd9bb5d (1232 439 s, 1287 273 s, 1299 265 s on 6 threads) were
each run as the chain runs it (`taxman solve N`, from a copy of the chain's results truncated before N) for 40-70 s on 4
threads, and sampled every millisecond by macOS `sample`.  Each sample's return addresses were symbolicated with their
inlined frames (`atos -i`).  The *kind* of work is the innermost function of the crate, the *purpose* the innermost
search function above it.  Shares of busy samples (the idle coordinator thread excluded):

| work | 1232 | 1287 | 1299 |
|---|---|---|---|
| kind: matching repairs (Hungarian searches, the radix heap) | 66.8% | 57.0% | 56.4% |
| kind: peels and promotions (frame checks of children) | 16.9% | 21.3% | 21.5% |
| kind: nogood table (scans for lookups, rule-outs) | 2.2% | 5.1% | 5.0% |
| kind: engine | < 0.1% | < 0.1% | < 0.1% |
| purpose: strong branching (exact bound without each member of the conflict) | 33.6% | 27.8% | 28.8% |
| purpose: making the child (promotion, candidate list, rule-outs) | 19.7% | 26.0% | 26.2% |
| purpose: refinement (bounds of the branches of the conflict) | 20.4% | 18.4% | 18.9% |
| purpose: the matching bound of a node or child (`check`) | 15.8% | 15.9% | 14.2% |
| purpose: conflict lookup, or computation when the table has none | 3.9% | 5.0% | 4.7% |

The other kinds are the node's own work (6.6-7.5%), bounds from the potentials (4.3-4.8%), copies (3.0-4.2%) and
conflict minimization (under 0.5%).  A fourth game, 1295 (25 s), splits within two points of these (repairs 58.1%,
peels 20.8%; strong branching 29.0%, making children 25.5%, refinement 19.8%).  The probe agrees on the hot kernels: a
strong-branching exclusion costs 9-12 k cycles and a step 16-17 k.  This profile is of the solver before the Lagrangian
bound; the [profile with it](../faster-nodes-with-the-lagrangian-bound-same-trees-2026-10/README.md) has the Lagrangian
levels at a quarter of the time and strong branching at 11-20%.  What follows:

* Three quarters of the matching work is *lookahead* (strong branching and refinement), not the node's own bound.
  Estimates in place of strong branching's exact solves act on 28-34% of the time; a tighter node bound can remove
  lookahead where it cuts.
* Conflict computation is under 1%, because the nogood table answers 85-93% of lookups (3).  Cheaper conflicts would not
  pay.  The table itself takes 2-5%, so its policy matters through the tree it causes, not its own cost.
* The engine is invisible.  Its scaling is a question of tree size and fragmentation at many workers, not overhead.
* Finding the incumbent is not where the time goes: in 8 of the 11 slowest games of 1232-1300 the seed (the previous
  game's promotions, improved by the local search) is already optimal.  1290 and 1300 reach their optimum after 14 and
  19 s on 4 threads, of runs that take 142 and 179 s on 6; only 1232 and 1287 had not reached theirs after 45 s (1232 at
  37005 of 37113, 1287 at 38936 of 38977).  The cost of the hardest games is mostly the proof.

### 3. Nogoods with very many threads

**Question.**  Each thread keeps its own table and clears it every 50,000 of its steps.  At 192 threads, is that still
right, or should threads share nogoods, or keep the good ones?

A tracking mode of the nogood table, plus a registry shared by the searchers of one search and set up by the ladder for
these runs only, counted: lookups, rule-outs, and the nogoods learned, imported, dropped and dropped unused, in total
and by the nogood's number of members; the uses of a nogood by its *age* (the steps its own searcher has taken since it
entered the table), in buckets `[2^k, 2^(k+1))`, against the steps spent at each age; and, for each nogood learned,
whether it was new, held by another worker now, learned by another worker and forgotten since, or forgotten and learned
again by the same worker.  Policies were set per run: how often a searcher forgets (never, or every K steps), keeping
the nogoods of at most K members at a clear, and sharing those of at most K members through a common pool that each
searcher reads at every step.  Every run is a proof with `revise_freedoms=off` and the default tactics otherwise.

**How useful an old nogood is.**  Uses per 1000 steps a nogood spends in the table, by age, on the proof of 1215 never
forgetting (one thread):

| age in steps | 1 | 16 | 256 | 1k | 4k | 16k | 64k | 256k | 1M | 4M |
|---|---|---|---|---|---|---|---|---|---|---|
| uses per 1000 nogood-steps | 170 | 49 | 22 | 14 | 8.7 | 7.3 | 6.8 | 3.6 | 3.4 | 3.3 |

Usefulness drops 20-fold in the first ten thousand steps and then flattens, so the question is what it costs to keep.

**What a nogood costs.**  Lookups and rule-outs scan the whole table.  The measurement ran on the 300 search nodes of
the proof of 1215 in the probe's fixture (`testdata/perf/n1215.nodes`: each node's promotions `P`, candidates, and `Q*`,
the promotions of its heaviest matching), with tables made of the first k of the 3,032 distinct nogoods that a study run
of that proof had learned (k doubling from 50; 20 repetitions of the 300 nodes).  A *lookup* asks `Nogoods::conflict`
for the smallest conflict within `P ∪ Q*`; a *rule-out* asks `Nogoods::ruled_out` for each of the 8 largest members of
`Q*` in turn as the child's one newly promoted number.  Cycles per call:

| nogoods in the table | 100 | 200 | 400 | 800 | 1600 | 3032 |
|---|---|---|---|---|---|---|
| lookup | 757 | 1040 | 1495 | 2433 | 4198 | 8631 |
| rule-out | 292 | 392 | 566 | 892 | 1559 | 3127 |

That is about 2.6 cycles per nogood per lookup and 1 per rule-out, at 0.7 lookups and 0.5 rule-outs per step: a table of
S nogoods costs about 450 + 2.3 S cycles of a 16-20 k-cycle step.  On the proof of 1215 on one thread, forgetting every
50,000 steps leaves tables averaging 125 nogoods at a lookup, and the search takes 5,485,294 nodes and 112 G cycles.
Never forgetting, the tables average 547 and it takes 5,446,460 nodes (−0.7%) and 134 G cycles (+19%).  Forgetting costs
re-deriving instead: 17,556 nogoods learned against 621, about 1.7 G cycles, a twelfth of what the bigger table costs.

**Duplication across threads.**  The share of the nogoods learned that another worker held at that moment:

| proof | 4 workers | 16 | 32 | 64 | 128 |
|---|---|---|---|---|---|
| 1215 | 42.5% | 75.0% | | 88.7% | |
| 1225 | 32.5% | | 78.3% | | 88.4% |

Nearly all of it at many workers, but learning is cheap: 4-7 nogoods per 1000 steps (4.05 at 4 workers to 6.81 at 128 on
1225), about 1.4% of the cycles.

**Policies, in nodes** (proofs, T workers taking turns on 4 CPUs; many-worker runs vary by ±10%, so repeats are given):

| proof, workers | clear every 50k | share every nogood | share those of ≤ 3 members | keep those of ≤ 3 members at a clear |
|---|---|---|---|---|
| 1215, 16 | 5.94M, 5.92M, 5.54M | 6.10M (tables 434 at a lookup) | | 5.50M |
| 1215, 64 | 5.88M, 5.76M, 6.05M | 5.99M (tables 601) | | 5.71M |
| 1225, 32 | 13.65M, 11.05M, 10.30M | 13.35M (tables 773) | 11.56M | |
| 1225, 128 | 11.46M, 10.92M | 14.38M (tables 1214) | 12.89M | |

Sharing does not bring the node count down, and sharing everything makes the tables 3-8 times as large and the trees
larger: most imported nogoods are dropped at the next clear unused, and a large table picks its conflict from more
candidates, which gives no better branching.  Keeping the short nogoods changes nothing measurable, and could not: the
search's nogoods are large (on 1215, 11,483 of 17,556 learned have 8 or more members, 1,945 have 2), and the large ones
answer most lookups (1.6 M of 3.3 M) and rule out most candidates (2.7 M of 4.7 M).

**Answer.**  Periodic clearing of per-thread tables is still right at 192 threads, for three reasons: an old nogood
earns little (a few uses per thousand steps) against a scan cost on every lookup; more nogoods do not make smaller trees
(never forgetting: the same nodes; sharing: more); and re-deriving what another thread knows costs about 1% of the
cycles.  The regime does not change at 192 threads: a game at the end of the chain, of 10^8-10^9 nodes, gives each of
192 threads 0.5-5 M steps, which is the 1215 proof on 1-16 workers.  What does grow with the number of workers is the
tree: by 10-20% from 4-8 to 32-128 workers on 1225 (9.3-10.1 M to 10.3-13.6 M nodes; the committed
[scaling.jsonl](../benchmarks-the-probe-pgo-and-scaling/scaling.jsonl) has 9.45 M at 4 and 9.30 M at 8), with ten to
twenty-five times as many unit starts (385 at 4 workers, 4,000-10,000 at 32-128), each a rebuild that forgets the
conflicts known along its path.  A period of about 30,000 steps would keep the tables at the size they have at 4 workers
(they learn about 1.7 times faster at 128).

### 4.1 Memoizing searches that repeat (rank 1)

Many consecutive games have exactly the same search space: the same frames (moves, factors, freedoms), candidates and
`max_total`, so the same best promotion set.  Only the ceiling on the promotion sum, `n + score(n − 1) − in_the_bag`,
differs, and when it is above the optimum the search runs a full proof again.  The pilot computed, for every game of a
range that is neither trivial nor prime, a 64-bit digest of what a search with only proven tactics depends on: the
root's frames, the candidates and `max_total`, as `Board::prepare` gives them with `revise_freedoms=off`.  Games with
the same digest search the same space; the first is the original and the later ones repeat it.  Joined with the search
seconds that the chain of commit bd9bb5d recorded for each game:

* 1232-1300: 29 of the 60 searched games repeat an earlier space, and their searches took **851 s of the chain's 2866
  s** (29.7%): 1299 repeats 1296 (265 s), 1293 repeats 1292 (207 s), 1271 repeats 1269 (75 s), 1267 repeats 1265 (64 s),
  1263 and 1261 repeat 1260 (51 s and 48 s), and so on; the other games reached the ceiling at once.
* 1201-1231: 46 s of 276 s (16.6%).

The method is sound: both games have the same best promotion sum, established by the earlier game's proof (exhausted, or
the ceiling), and the later game's score is its own `in_the_bag` plus it, replayed by the referee as always.  It needs a
proof kind (a repeat of game m, resting on m's proof and what it assumes), the digest in the result line, and the two
verifiers.  The payoff is a constant fraction of the chain, at no risk.  Built as the tactic and proof kind
`same_search` ([same search](../same-search-2026-10/README.md)).

### 4.2 A Lagrangian bound over the nogoods (rank 2)

The matching bound knows no nogood.  Every completion of a node with promotions `P` promotes at most `|N − P| − 1`
members of each nogood `N`.  Dualizing these inequalities with multipliers `λ_N ≥ 0` gives, for every `λ`, the bound
`max_M Σ_{x ∈ Q(M)} (x − Σ_{N ∋ x} λ_N) + Σ_N λ_N (|N − P| − 1)`: a matching with each candidate's weight lowered by the
multipliers of its nogoods, plus a constant.  A nogood with a member that has left the candidates cannot be broken any
more and is left out (multiplier 0 at that node).  The bound is sound by weak duality, for any `λ ≥ 0`.  It is the
prover's second bound ([the prover's README](../../../noframe/README.md), section 3: 3.3× fewer nodes at n = 793 in the
prover) brought to the frame solver, and MaxSAT's core-guided bounds in spirit: each nogood is a core.

The pilot fitted multipliers once at the root (200 subgradient steps, Polyak's step towards `need`, in a second) to the
nogoods of a short warm-up (a node-limited proof of the same game, whose distinct nogoods were written out).  It then
tried the bound at every node and child where the matching bound and refinement did not cut, solved from scratch by the
incremental solver on a reweighed copy of the graph:

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

The warm-up is enough: 1184 with all 169 nogoods of its whole proof took 55,303 nodes, the same as with the warm-up's
140.  The warm-up's own nodes (20 k to 500 k, 5-12% of each proof) are not counted.  The effect varies more from game to
game than with n, with no clear trend across 1100-1250.  Two further findings:

* *The multipliers of dead nogoods must go.*  Keeping every nogood's multiplier whatever happened to its members gives
  1184 129,095 nodes (−23%) instead of 54,795.
* *Refitting pays.*  On the probe's 300 shallow nodes of each proof's fixture (the incumbent the optimum, a mean gap of
  145 at 936 and 260 at 1215 between the matching bound and what the nodes need), the root's multipliers lower the bound
  by 66 and 69 on average and cut 34 and 21 nodes, where refinement cuts 27 and 10.  Refitted at each node (20 steps
  from the root's), they lower it by 110 and 130 and cut 81 and 33, which is 2.4 and 1.6 times as many.

The pilot's from-scratch evaluation made it 7× slower per node at 1184: a measurement of nodes, not a usable bound.  The
follow-up built a second incremental matching along the path as the tactic `lagrange`, on by default, with the forced
members its potentials give: about 3 times fewer cycles on the proofs of 1184-1300
([a Lagrangian bound over the nogoods](../a-lagrangian-bound-over-the-nogoods-2026-10/README.md)).

### 4.3 Reduced-cost fixing (killed)

From MIP: a candidate `x` that the heaviest matching does not promote has a reduced cost read off the potentials
(`MatchBound::bound_including`): `value − max(0, min over its factors f not required of (y_x + y_f − x))`, clamped to
`[0, value]`, bounds every matching that promotes `x`.  If it is below `need`, no completion that beats the incumbent
promotes `x`, here or below, and `x` can leave the candidates.

*Measurement.*  On the 300 search nodes of the probe's fixtures of 936 and 1215 (`testdata/perf/`), with the incumbent
set to the game's optimum, each node still open was taken.  Of its candidates outside `Q*`, those whose reduced-cost
bound is below `need` were counted, and each was checked exactly by promoting it in the matching bound and solving (no
check failed).  It fires a lot: 70.9 of 159 candidates outside `Q*` per open node at 936 and 61.6 of 212 at 1215.

*Pilot.*  A search tactic (off by default) removed those candidates from the node's candidates, and so from its
children's.  It changes no tree: the fast set (one thread) and the proofs of 1215 (5,485,294 nodes) and 1025 (5,570,602)
came out node for node the same, and the tactic was taken out again.  The reason is general: the matching relaxation is
integral, so a fixed candidate is never in the heaviest matching of a node that stays open below, and removing a
candidate the optimum does not use changes neither the bound nor the conflicts.  In MIP the idea helps because LP
solutions are fractional; here the forced members (`fix_forced`) are the useful half of the same dual information.

### 4.4 Component decomposition (killed)

From #SAT and AND/OR search: if the problem left at a node splits into parts that share no frame component, the best
completion is the sum of the parts' best, and searching them apart turns a product into a sum.  The parts are the
connected components of a graph over the numbers of the state, computed with a union-find: for each frame, each mover (a
move of the frame, or a candidate that is a factor of the frame behind, which would be promoted into this one) is joined
to each of its maximal factors that is a factor of the frame; a candidate belongs to the component of its own number.
At the root of 936, 1000, 1100, 1215, 1232, 1287 and 1300, and at all 600 search nodes of the probe's fixtures (936 and
1215), exactly one component holds candidates: nothing to split.  The version that conditions on a separator home
instead (agenda avenue A5) was killed later too ([A5's
test](../decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md)).

### 4.5 Disjoint conflicts from the dual (killed)

From MaxSAT's disjoint-core bounds: two disjoint conflicts within `Q*` must each lose a member, and lowering the
potentials of both excluded members (and their partners) one after the other keeps the dual feasible, so their drops add
up.  The pilot's bound generalizes `bound_excluding` to several excluded candidates; it was never below the exact value
on 16,800 pairs, and 26-33 above it on average.  The cut was tried at a node where refinement did not cut and no member
was forced.  Each conflict (the one the search chose and every other nogood of the table within `P ∪ Q*`) is costed by
the least single drop `value − bound_excluding(x)` over its members.  For each pair of disjoint conflicts whose costs
sum to more than the gap `value − need`, the node is cut if the bound with an `x` of the first and a `y` of the second
excluded is below `need` for every such pair.  The fast set took between 2.7% fewer and 0.3% more nodes, and 12% more
cycles.  The frame solver's bound is already tight where it matters (forced members and a refinement one deep catch
nearly all of what pairs of conflicts would), unlike the prover's, whose gap is about 1,170 at n = 1000.

### 4.6 Recency in the choice of conflict (killed)

From CDCL's activity heuristics (VSIDS, CHB).  The nogood table returns the smallest conflict within `P ∪ Q*`, then the
one with the largest member, then the first learned.  The pilot preferred the most recently learned among the smallest
and ignored the largest member.  Nodes changed by −1.2% at 880, +3.6% at 936, +20% at 1050, +5% at 1100, −0.3% at 1170
and +7% at 1184 (the fast set, one thread), and by +0.7% on the proof of 1215 and +6.7% on 1025.  Keeping the largest
member first and only then the most recent gave −2.8% to +9.4%, with no direction.  Which equally small conflict to
branch on moves a game's node count by ±10% either way, and recency does not choose better than size and the largest
member.

### 4.7 A portfolio for finding incumbents (weak)

From parallel SAT portfolios: run some threads with another branching that finds good sets early (`conflict-largest`
follows the dive).  Section 2 shows that the cost of the hardest games is the proof, so there is little to win.  On the
games that do improve their start (4 threads, 45 s, time to the best set found with `branching=conflict-largest` against
the default `conflict-strong`): 1290 at 9.3 s against 13.9 s, 1287 at 13.7 s against 26.6 s (the same set, 41 below the
optimum), 1300 not found in 45 s against 19.2 s, and 1232 found by neither.  It is worth remembering for games
like 992 and 1025, whose optimum the search reaches late, if they come back among the hardest games.

### 5. Every idea considered

The ideas that no section above piloted, each with the reason it was not:

* *Restarts with phase saving.*  With the optimum known the bound fixes the tree, so a restart only loses work; SAT
  uses restarts to escape heavy tails while looking for a solution, and that phase is not where the time goes (2).
* *Backjumping and bound nogoods.*  A node is closed by a bound, and the reason for a bound failure involves the
  incumbent and the whole matching, not a few decisions; MIP solvers gain modestly from such nogoods.
* *Cube-and-conquer with lookahead splitting.*  The engine's splits already fill all workers within 3-8 ms (`ladder`
  "full") and its tail is short; cubes could add balance by *estimated* size, which is Knuth's estimator.
* *Best-bound selection.*  With the optimum in hand the proof tree is the same in any order (bar the nogoods learned),
  and depth-first keeps memory bounded.
* *Cutting planes.*  The matching LP is integral, so the only useful cuts are the nogoods, which the Lagrangian
  dualizes (4.2).
* *Dominance rules and symmetry breaking.*  A move takes *all* its factors still in the frame, so having more maximal
  factors is not monotone, and a sound dominance needs a proof per pair.  Piloted later:
  [dominance and symmetry](../dominance-and-symmetry-2026-10/README.md).
* *Transposition tables and component caching.*  The conflict branching partitions the promotion sets, so one search
  never meets a node twice and its space never splits; across games they are rank 1.  Caching frame checks (14-17% of
  peels repeat one exactly, 5% one of the last eight) bounds the gain to a few percent of the 17-22% the peels take.

Already in the solver, or subsumed: clause minimization (`FrameGraph::nogood` is a minimal subset keeping a frame of the
root unwinnable), nogood recording on restarts, move ordering (strong branching is the ordering), and core-guided bounds
(the conflicts are cores).  LBD-based reduction needs no trial: size stands in for glue, because a nogood's members come
from two adjacent frames (3).

## Decision

**Kept.**  `same_search` (4.1), `lagrange` (4.2), clearing of the per-thread tables every 50,000 steps (3), and the
emulation of many workers by few CPUs (`EngineConfig::cores`).

**Not kept**, on the numbers above: reduced-cost fixing, component decomposition, disjoint conflicts, recency and the
portfolio of branchings (4.3 to 4.7), and, in 3, never forgetting, sharing nogoods between threads and keeping the short
ones at a clearing.

**Open, and what would reopen it.**

* *A clearing period of about 30,000 steps at 192 threads*: a test on the big machine; the [end-of-chain study of
  keeping and forgetting](../keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/README.md) takes it up.
* *Reliability branching* (rank 4): record, per candidate, the exact drops strong branching computes (a counter, no
  change), and measure how well a pseudocost predicts the chosen member and the forced set on the proofs of 1215 and
  1225.  Build it only if it predicts more than 90%.  The risk is the forced members: strong branching's exact solves
  also find the members whose exclusion drops the bound below `need`, and an earlier measurement found that deciding by
  the potentials alone traded nodes for time at about even.
* *Knuth's estimate of a tree's size* (rank 3): the product of the numbers of surviving children along a random walk
  from the root to a leaf, summed over the depths, is an unbiased estimate of the tree's size.  It would give a game's
  cost before running it (the cloud budget is ×10 per 55-70 in n, with games of a band varying ×3 to ×18) and, in a run,
  which units to split first.  The experiment is a probe searcher that expands a node fully and descends into one
  surviving child at random, run with empty nogood tables on the proofs of 1215, 1225 and 1250, whose sizes are known;
  report the median and spread of the estimate against the truth for 100, 1,000 and 10,000 walks.
* *Fragmentation at many workers* (rank 5): the suspect is what a rebuilt unit forgets: the conflicts known along its
  path (`known`), the kept strong-branching level and the verdict of its first check.  Count, per unit start (with
  `--cores`), how often the first steps recompute a conflict the path had known; then try handing a unit its path's
  known conflicts, or answering split requests with deeper units.  Soundness is unaffected either way (units are
  caches).

## Files

The folder holds this README.md only.  The pilots' code was discarded; pieces that were useful on their own are on
`master`:

* `EngineConfig::cores` (with a test in `solver/frame/tests/engine.rs` and `ladder --cores C`).  It costs nothing unless
  set: the check is in the engine's every-1024-polls slow path, and the probe's cycles and fingerprint are unchanged.
* `solver/frame/examples/common/fixtures.rs` (the `.nodes` fixtures and the hex text sets are written in) and
  `solver/frame/examples/common/counters.rs` (cycles and instructions of the process), which the probe used to hold and
  the pilots shared.
* `solver/tools/profile/` (the profile of section 2; needs only the solver; macOS only, as it uses `sample` and
  `atos -i`), attributed by `attribute.py`, standard library only.

From the top of the repository, after `cargo build --release --examples` in `solver/frame`, with
`K=results/strict.jsonl`:

```
solver/frame/target/release/examples/ladder --threads 16 --cores 4 --seed-opt --known $K 1215    # 16 workers on 4 CPUs
solver/tools/profile/profile.sh 1232 40 4 /tmp/prof    # the profile of section 2
```

## Commits

* bd9bb5d (2026-10-12): the commit of the chain 1232-1300 whose search seconds the memoizing pilot (4.1) used and the
  profile (2) ran against.
