# Matroid intersection bounds (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  The slope of the cost curve is the growth of the root's gap
([slope](../what-drives-the-slope-2026-10/README.md)), so a bound whose gap grows more slowly changes it.  Playable sets
are linearly independent over GF(2) ([THEORY.md](../../../../docs/THEORY.md), Theorem 7), which made the upper half
exact.  Does that structure give a bound on the promotions, how much of the gap does it close at the root and at search
nodes, does the share hold with n, and what does it cost?  **Answer.**  The heaviest common independent set of two GF(2)
frame matroids is a valid bound, computed in about a millisecond.  At the root its gap averages 136 over 880-1300
against 269 for the Lagrangian bound and 760 for the matching bound, and it grows 4.6 ± 2.4 per 100 in n against 56
and 142.  A plain branch and bound with it proves 146 games with 12 to 613 times fewer nodes than the solver.  It became
the search's node bound ([GF(2) bound in the search](../the-gf2-bound-in-the-search-2026-10/README.md)).

## Method

**The bound.**  Give each frame the space over GF(2) whose coordinates are its factors.  Each mover paying in the frame
(a move of it, or a candidate of the next home) gets the vector of its maximal factors in the frame, and each candidate
of the frame's home the unit vector of its own number.  If a set Q of candidates can be promoted, then in every frame
the vectors of the moves, of Q's members promoted into the frame and the unit vectors of Q's members promoted out of it
are independent, because a winnable frame peels completely.  So each frame is a linear matroid on the candidates that
touch it, the moves contracted.  A candidate touches its home frame and the one below, one of even index and one of odd,
so the even frames' matroids make one matroid A on the candidates and the odd frames' another, B.  Every promotable set
is independent in both, and the heaviest common independent set (weighted matroid intersection, polynomial) bounds the
best completion.  It is never above the matching bound, and it misses only the *odd* cores of movers with three or more
factors: self-covering sets no subset of which lists every factor an even number of times.  The proof is the module
comment of `examples/common/meet.rs`; the bound is the GF(2) bound of the
[glossary](../../../../docs/GLOSSARY.md#bounds) ([THEORY.md](../../../../docs/THEORY.md), Theorem 11).

The same construction bounds the whole game with no frame model (`matroid whole`): every number s of 2..n gets a vector
v(s) and, if s ≤ n/2, a unit vector e_s, and the two parities of Ω make two matroids on 2..n whose heaviest common
independent set bounds opt(n).  That is the parity bound ([THEORY.md](../../../../docs/THEORY.md), Theorem 10); the
Franklín–Moniot matching bound is the same construction with Hall's condition in place of linear independence.

**The code.**  No solver code changed.  The data were made on the build of 4372b36, before the tactics cleanup
(271c817), with `revise_freedoms` off; on the cleaned-up build the examples reproduce `roots.jsonl` and every command's
values that were checked.  `examples/common/meet.rs` holds the weighted intersection by shortest augmenting paths
(Lawler; Schrijver, Theorem 41.7; the reference `intersect`), a fast form for blocks of rank at most 64
(`intersect_fast`), warm starts checked extreme (`intersect_warm`) and the weight split (`split`); `examples/matroid.rs`
builds the frame matroids of any state and runs the measurements.  Everything ran on one thread, at most two processes,
on the laptop shared with other agents, so the times are indicative.

**Validity**, beyond the proof.  The intersection algorithms agree with brute force on 3,000 random pairs of block
matroids.  At every non-prime root 2..1300 the bound is at least P* and the fast algorithm's value is the reference's;
the optimal game's promotions and 100 random maximal promotable sets a game are independent in both matroids; at the 90
roots with at most 22 candidates (to n = 136) every promotable set is enumerated, all independent.  At the probe's 600
nodes, 20 random completions each are independent, and the best completion found by the solver's own search from the
node (all 300 nodes of 936, 98 of 100 sampled nodes of 1215, the others over 30 s) is never above the bound.  The
model-free bound is at least opt(n), with the optimal game's moves independent, for every non-prime n ≤ 1300, and equals
`in_the_bag` plus the frame bound at all 361 roots of 880-1300.

## Results

### 1. At the root it closes 82% of the matching gap and 42% of the Lagrangian gap, more as n grows

| n | games | matching gap | Lagrangian gap | GF(2) gap | GF(2) / matching | GF(2) / Lagrangian | ms |
|---|---|---|---|---|---|---|---|
| 851-900 | 18 | 432 | 112 | 88 | 0.20 | 0.79 | 0.42 |
| 901-950 | 43 | 619 | 185 | 147 | 0.24 | 0.80 | 0.43 |
| 951-1000 | 43 | 749 | 261 | 172 | 0.23 | 0.66 | 0.52 |
| 1001-1050 | 42 | 795 | 362 | 156 | 0.20 | 0.43 | 0.64 |
| 1051-1100 | 42 | 546 | 206 | 81 | 0.15 | 0.39 | 0.75 |
| 1101-1150 | 45 | 522 | 162 | 107 | 0.21 | 0.66 | 0.72 |
| 1151-1200 | 43 | 629 | 154 | 108 | 0.17 | 0.70 | 0.73 |
| 1201-1250 | 42 | 1091 | 426 | 148 | 0.14 | 0.35 | 1.02 |
| 1251-1300 | 43 | 1277 | 471 | 185 | 0.14 | 0.39 | 1.03 |
| 1301-1310 | 7 | 1445 | 554 | 154 | 0.11 | 0.28 | 1.15 |
| 1350-1352 | 3 | 1255 | 465 | 272 | 0.22 | 0.59 | 1.26 |

Over the 361 games of 880-1300 the matching gap grows by 142 ± 9 per 100 in n, the Lagrangian gap by 56 ± 5 and the
GF(2) gap by 4.6 ± 2.4 (R² 0.01): it still grows, at a quarter of the Lagrangian gap's rate or less.  The model-free
form of section 6, equal to it there, grows 13.8 ± 1.0 over 600-1300 and 14.3 ± 0.3 over 2-1300.  The share of the
matching gap it closes rises by 2.3 ± 0.2 points per 100 in n, of the Lagrangian gap by 9.2 ± 0.9.  The GF(2) bound is
below the root's Lagrangian bound at 346 of the 371 roots (at most 24 above it at the others, mostly 897-918), and the
cap `max_total` never binds.  It costs 0.4-1.3 ms a root, a few of the Lagrangian fit's steps.

### 2. A plain branch and bound with it proves the games with 12 to 800 times fewer nodes than the solver

The skeleton (`bnb`) has one bound and no refinement, forced members, nogoods or strong branching: a node is cut when
the bound (capped at `max_add`) falls below what it needs; otherwise its heaviest set has a conflict k₁ < … < k_r and
the node branches into "k₁ … k_{j−1} promoted, k_j not" for each j.  With the incumbent at P* (the slope study's δ = 0
proofs), against the solver's one-thread proofs:

| n | games | log10 nodes, solver | log10 nodes, GF(2) B&B | ratio (geometric) | s, solver | s, GF(2) B&B |
|---|---|---|---|---|---|---|
| 851-900 | 6 | 3.78 | 2.59 | 15 | 0.5 | 1.0 |
| 901-950 | 16 | 4.56 | 3.48 | 12 | 12.8 | 22.8 |
| 951-1000 | 16 | 5.21 | 3.74 | 30 | 107.0 | 120.1 |
| 1001-1050 | 19 | 5.62 | 3.99 | 43 | 163.8 | 160.1 |
| 1051-1100 | 18 | 4.46 | 2.77 | 49 | 12.6 | 8.0 |
| 1101-1150 | 17 | 3.82 | 2.72 | 13 | 1.5 | 6.4 |
| 1151-1200 | 16 | 4.52 | 2.90 | 42 | 13.2 | 32.8 |
| 1201-1250 | 19 | 6.13 | 3.90 | 171 | 872.2 | 168.6 |
| 1251-1300 | 16 | 6.90 | 4.11 | 613 | 1750.5 | 240.5 |
| 1309 | 1 | 7.14 | 4.24 | 794 | 157.5 | 16.3 |
| 1350, 1352 | 2 | 6.88 | 4.79 | 124 | 315.9 | 128.3 |

The log10 of the solver's nodes grows 0.476 ± 0.066 per 100 in n over these games, the GF(2) branch and bound's 0.138 ±
0.048 (R² 0.05): that is +0.48 ± 0.07 against +0.14 ± 0.05.  Its node count follows the root's GF(2) gap (0.0111 ±
0.0005 decades per unit, R² 0.78: ×10 per 90 of gap) as the solver's follows the Lagrangian gap, and 0.011 times the
gap's growth of 5-14 per 100 in n predicts 0.05-0.15 decades per 100 in n, the measured slope at its upper end.  Its
largest trees are 1287 (87,841 nodes; the solver 20,972,332), 992 (77,567; 2,099,730) and 1350 (77,283; 17,504,520).
With the matching bound in the same skeleton, 880 and 936 are unfinished after 200,000 nodes (the solver proves them in
25,398 and 41,653).  In time, computing the bound from scratch at every node (0.68 ms a node): 905 s for the 146 proofs
against the solver's 3,408 s, 409 s against 2,622 s over 1201-1300; slower in the five bands whose proofs the solver
makes in 10^5.2 nodes or fewer on average, faster in the others.

### 3. At search nodes it cuts more than half of the probe's open nodes, where the matching bound cuts none

The fixtures' nodes are units the solver's proof of 936 or 1215 handed to its searcher; the target is what a completion
must beat, P* − sum(promoted); a *true gap* is a bound less the node's best completion.

| n | nodes | matching cuts | GF(2) cuts | GF(2) ms | every 3rd: node-fitted Lagrangian cuts (GF(2)) | either | true gap, matching | true gap, GF(2) | forced exactly | by the split |
|---|---|---|---|---|---|---|---|---|---|---|
| 936 | 300 | 0 | 163 | 0.33 | 66 of 100 (55) | 69 | 317 | 154 (300 nodes) | 26.1 a node | 66% |
| 1215 | 300 | 0 | 167 | 0.68 | 50 of 100 (59) | 67 | 548 | 272 (98 nodes) | 28.0 a node | 60% |

The GF(2) bound halves the true gaps.  A Lagrangian over the matching with nogoods of the node's own, fitted at the node
(`root_lagrange` of the slope study, moved to `examples/common/root_lagrange.rs`, 2,000 steps: about a second a node,
which no search can afford), cuts about as many nodes; the two cut different ones (either cuts 69 and 67 of 100).  At
the nodes the GF(2) bound leaves open, 26-28 members of its heaviest set (of 40-55) are forced: the bound without each
falls below the target.

### 4. Cost, and the incremental forms

* *From scratch*, the fast algorithm takes 0.33 and 0.68 ms a node at 936 and 1215: about 0.2 ms to build the frame
  matroids and about 45 augmentations of about 10 µs.  The solver's step costs about 5 µs.
* *A warm start from the parent's heaviest set* (`bnb --bounds gf2warm`): not kept.  A child also deletes the member
  branched on, and the rest was extreme (no negative cycle in its exchange graph, Schrijver Theorem 41.5, and no heavier
  smaller set) at 2 of the 2,543 nodes of 1250 and none of the 1,874 of 1100.
* *Starting from the heaviest uncontended elements* (the longest prefix of the candidates by weight that is common
  independent): 20% fewer augmentations (1250: 146,565 to 117,863), the same values.  Kept in the research code.
* *A weight split* (Frank's weight-splitting theorem) `w = w1 + w2`, with the heaviest set the heaviest independent set
  of A for `w1` and of B for `w2`, found from the final exchange graph as difference constraints.  The two greedy optima
  then bound the problem with any members left out, the analogue of the matching bound's `bound_excluding`.  It finds
  66% and 60% of the exactly forced members (looser by 26 and 43 a member on average) at two greedy runs a member.
* *The frame matroids of a child* are the parent's contracted by the members it promotes (at all 600 probe nodes this
  gives the node's bound exactly): one pivot per promoted member in each of its two frames instead of the 0.2 ms build.

### 5. What it misses

The conflicts of the root's heaviest set are large, 7-18 members (most 13-18): the odd cores.  The cores themselves
(`cores`) span frames 1-3: 18-65 movers each, a third to two thirds with three or more live factors.

* *Lorea's hypergraphic matroid* (every subset lists at least one more factor than it has movers) is valid per frame
  too.  It sees none of the 14 cores of 936-1300: every subset of each lists more factors than movers.  Three matroids
  cannot be intersected in polynomial time.
* *A Lagrangian over nogoods with the intersection as the inner problem* (`lagrange`: the conflicts of each step's
  heaviest set, 1,500 damped Polyak steps at the root) takes the GF(2) gap of 936 from 132 to 118, 1000 from 214 to 137,
  1215 from 213 to 198, 1254 from 204 to 166 and 1300 from 151 to 129 (7-36%).  The cover inequalities of 15-member
  conflicts are weak.
* *Other fields*: over GF(3) or the rationals the unsigned vectors of a triangle of movers are independent, so odd
  cycles are lost; signed vectors keep every cycle but see only orientable closed surfaces, where GF(2) sees all.

### 6. The whole game, model-free

| n | games | gap mean | median | max | exact | ms |
|---|---|---|---|---|---|---|
| 2-100 | 74 | 0.5 | 0 | 18 | 70 | 0 |
| 101-200 | 79 | 0.9 | 0 | 24 | 64 | 2 |
| 201-300 | 84 | 2.8 | 0 | 19 | 44 | 8 |
| 301-400 | 84 | 15.0 | 12 | 35 | 11 | 18 |
| 401-500 | 83 | 20.7 | 19 | 51 | 0 | 34 |
| 501-600 | 86 | 43.6 | 46 | 106 | 5 | 57 |
| 601-700 | 84 | 66.4 | 59 | 139 | 0 | 90 |
| 701-800 | 86 | 78.1 | 90 | 178 | 0 | 132 |
| 801-900 | 85 | 86.0 | 92 | 178 | 0 | 186 |
| 901-1000 | 86 | 159.4 | 146 | 323 | 0 | 252 |
| 1001-1100 | 84 | 118.4 | 101 | 223 | 0 | 334 |
| 1101-1200 | 88 | 107.7 | 95 | 201 | 0 | 429 |
| 1201-1300 | 85 | 167.0 | 176 | 282 | 0 | 545 |

The model-free matching bound's gap is 140-430 around 500-700 and 650-1,200 from 800 on ([prover
3](../../../noframe/docs/proofs.md)); this one's is a tenth of that, from the reference algorithm in half a second at
1300.  That the two constructions agree at every root of 880-1300 checks each against the other.

## Decision

The heaviest common independent set of the two GF(2) frame matroids is a valid bound on the promotions, computable in
about a millisecond from scratch, tighter than the matching bound by construction and than the root's Lagrangian bound
at 346 of 371 roots, with a gap that grows a quarter as fast or less.  In a plain branch and bound it cuts the slope of
a proof's node count from 0.48 to 0.14 decades per 100 in n and the time of the 146 proofs by 3.8 times (6.4 over
1201-1300).

* **Kept.**  The bound is the search's node bound, with branching on the conflicts of its heaviest set, forced members
  from the weight split, and the frame matroids carried down the path by contraction.  The cost to beat was about
  0.3-0.7 ms a node from scratch against a step of about 5 µs; at the frontier the trees shrink by two to three decades,
  so even from scratch it wins there, and an incremental exact intersection is the engineering that wins everywhere
  ([the GF(2) bound in the search](../the-gf2-bound-in-the-search-2026-10/README.md)).  The node-fitted Lagrangian cuts
  different nodes from it, so a Lagrangian over the intersection may pay at nodes though at the root it closes little.
* **For the prover**: the model-free bound sees more of the game than the matching bound, with a tenth of its gap, and
  should take the prover's model-free cross-check of opt(n) well past 989.
* **For THEORY.md**: the lemma that a playable set stays independent with the unit vectors of its members below n/2, and
  the split by the parity of Ω into two matroids, extend Theorem 7's lemma from the upper half to the whole game.
* **Not kept**: the warm start from the parent's heaviest set, Lorea's matroid, GF(3) and signed vectors.
* **Reopens** if a third relaxation sees the odd cores.

The example `matroid`, with the files of `examples/common/` it included, was removed from master on 2026-10-17.  It
builds at commit 8753189: `git checkout 8753189`, then `cargo build --release --example matroid` in `rust/`.

## Files

`matroid.py` joins the data files with the slope study's `slope_roots.jsonl` (the matching and Lagrangian bounds of
every root) and `slope_proofs.jsonl` (the solver's one-thread proofs) in [the slope
entry](../what-drives-the-slope-2026-10/README.md), and prints the tables above: `./matroid.py > matroid_output.md`.
The commands are `cargo run --release --example matroid --` followed by:

| file | command |
|---|---|
| `roots.jsonl` | `roots --opts slope_roots.jsonl 880-1310 1350-1352`: the GF(2) bound at every root (the Lagrangian is the slope study's, fitted with up to 8,000 steps over the root's own conflicts) |
| `bnb.jsonl` | `bnb --opts slope_roots.jsonl --bounds gf2 --limit 400000` on the 146 games that `slope_proofs.jsonl` proves at δ = 0, run before the start from the heaviest elements was added (the same trees, about 20% more time than now) |
| `bnb_matching.jsonl` | `bnb --bounds matching --limit 200000 880 936` |
| `nodes.jsonl` | `nodes --samples 20` on `testdata/perf/n0936.nodes` and `n1215.nodes` (a later `nodes --samples 0` on both checked the contraction of section 4) |
| `nodes_lagrange.jsonl` | `nodes` as above, with `--every 3 --lagrange 2000` |
| `nodes_exact.jsonl` | `nodes` as above, with `--exact 20` (936, every node) and `--every 3 --exact 30` (1215) |
| `forced.jsonl` | `forced --every 3` on both fixtures |
| `lagrange.jsonl` | `lagrange --limit 1500 936 1000 1215 1254 1300 --known results/strict.jsonl` |
| `cores.jsonl` | `cores 936 1000 1100 1215 1254 1300 --known results/strict.jsonl` |
| `whole.jsonl` | `whole 2-1300 --known results/strict.jsonl` |
| `matroid.py`, `matroid_output.md` | the script and its output, the tables of the results |

## Commits

* 67bb492, 8ee0199, 12a0f60 (2026-10-13): `examples/matroid.rs` and `examples/common/meet.rs`: the frame matroids and
  their weighted intersection, a faster intersection with warm starts and the Lagrangian over the intersection, and the
  node, forced-member, core and model-free measurements with their data and tables.
* 4372b36 (2026-10-13): the build of the data; 271c817 (2026-10-13): the tactics cleanup after it.
* 8753189 (2026-10-17): the last commit with the example, which master removed afterwards.
