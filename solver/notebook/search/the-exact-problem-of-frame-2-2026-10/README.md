# The exact problem of frame 2 (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-20.  Concluded 2026-10-20.

**Question.**  [A bound exact in a few frames](../a-bound-exact-in-a-few-frames-2026-10/README.md) found that in the
hardest games the search is, in effect, a branch and bound through frame 2's exact problem.  What is that problem, and
does it have an exact algorithm, specific to its structure, at least ten times faster than the node's own proof at nodes
of gap 400 to 1,000?

**Answer.**  No.  Frame 2's problem is a heaviest *uniquely restricted matching* that must cover the frame's moves,
tied to frames 1 and 3 by their GF(2) matroids.  It is NP-hard in general, and our instances lie in no class where it
is known to be easy.  At the nodes that hold the tree, frame 2 is one connected bipartite graph of 520 to 770
vertices with no 4-cycle, and at gaps of 400 to 1,000, after every fix the GF(2) bound allows, it still has treewidth
at least 9 to 12.  The best
known exact algorithms then need more than 10⁹ states for each bag of a tree decomposition, against the proof's
median 1.6 s.  Nothing was prototyped.

* **The shape.**  The graph is a smooth core (the numbers with no prime above 22 or 23) of treewidth at least 9, and 22
  to 52 fibers of treewidth at most 3, one for each larger prime, joined to the core at 14 to 21 hubs: the factors 16,
  24, 36, 40, ... whose multiples m·p form the twin families, which hold 65-88% of the movers.
* **The heaviest set fills frame 2.**  At every node of the searches the GF(2) bound's heaviest set uses every freedom
  of frame 2 and is independent there, yet the peel leaves 126 to 167 movers in its residual.  So near the optimum the
  exact problem is to choose which of frame 2's candidates go in and out so that a square matrix becomes triangular.

## Words

* Frames, homes, candidates, freedoms, conflicts and the GF(2) bound are the [glossary's](../../../../docs/GLOSSARY.md).
  A frame's *movers* are its moves and the candidates promoted into it; a mover's *live factors* are its maximal
  factors among the frame's factors.  A node's *gap* and *need* are those of [a bound exact in a few
  frames](../a-bound-exact-in-a-few-frames-2026-10/README.md#words): what its completions must add to beat the optimum
  is the need, and the GF(2) bound less the need, plus one, the gap.
* A matching is *uniquely restricted* when no other matching covers the same vertices, that is when it has no
  alternating cycle (M. C. Golumbic, T. Hirst and M. Lewenstein, "Uniquely restricted matchings", *Algorithmica* 31
  (2001)).  Graph theory's *acyclic* matchings (the matched vertices induce a forest) are another notion.
* A *stopping set* of a frame is a nonempty set of movers whose every live factor is listed by two of them or more: the
  self-covering sets of [decision-procedure.md, "Unplayable sets: cores and the
  residual"](../../../../docs/theory/decision-procedure.md#unplayable-sets-cores-and-the-residual), named as in the
  decoding of erasure codes, where the peel is the same procedure.
* *Treewidth* is measured on the incidence graph, movers and factors as vertices.  Its upper bounds are the widths of
  the elimination orders that min-degree and min-fill choose; its lower bound is the minor-min-width (contract a vertex
  of least degree into its neighbor of least degree, and take the largest least degree met), valid because treewidth
  never grows under contraction and is at least the least degree.
* With T = ⌊√(n/8)⌋ (22 or 23 here), every number of frame 2 has at most one prime above T, to the first power: a
  mover has Ω = 5 and a factor Ω = 4, and two such primes p and q (or p twice) have a product above n/8, which would
  make a mover larger than n and a factor larger than n/2.  The *smooth core* is the
  vertices with no prime above T, and the *fiber* of a prime p > T those with p.  A mover s·p lists s in the core and
  the (s/q)·p of its own fiber, so fibers meet only through the core.  A *hub* is a factor of the core listed by movers
  of fibers, and its *twin family* the movers m·p it heads.

## The problem

**Frame 2's exact problem at a node.**  Let M₀ be frame 2's moves at the node and F₀ its factors.  Let C₃ be the node's
candidates of home 3 (each may be promoted into frame 2 as a mover) and C₂ ⊆ F₀ its candidates of home 2 (each may be
promoted out of frame 2).  Let G be the bipartite graph on M₀ ∪ C₃ and F₀ whose edges join each mover to its maximal
factors.  Let A′ be the GF(2) matroid of the even frames other than 2 and B that of the odd frames, as in
[gf2-bound.md](../../../docs/design/gf2-bound.md), on all the node's candidates.  The bound exact in frame 2 (the
hybrid bound of the bound study) is

> the largest ΣL over sets L of the node's candidates with |L| ≤ `max_add`, L independent in A′ and in B, such that G
> less the factors L ∩ C₂ has a uniquely restricted matching covering M₀ ∪ (L ∩ C₃).

*Proof that this is the hybrid bound.*  The hybrid bound asks L to be independent in both GF(2) matroids and frame 2 to
be winnable with L applied.  Winnable implies independent in frame 2's own matroid (Theorem 11, [bounds.md, "The frame
matroids: a bound on the promotions"](../../../../docs/theory/bounds.md#the-frame-matroids-a-bound-on-the-promotions)),
so dropping that block from A leaves the family unchanged.  A frame is an instance of two ranks: movers above, factors
below, every cycle flat.  By Theorem 1 and the bridge ([franklin-moniot.md, "Their Theorem 1 is Theorem
1"](../../../../docs/theory/franklin-moniot.md#their-theorem-1-is-theorem-1)), the frame is winnable exactly when some
payment scheme has no flat alternating cycle, that is when some matching covering the movers is uniquely restricted.
An alternating cycle passes only through matched vertices, so the factors no mover is paid with play no part: a
factor promoted out is one the matching may not use, and nothing more.  Equivalently, by Theorem 5
([decision-procedure.md, "Unplayable sets: cores and the
residual"](../../../../docs/theory/decision-procedure.md#unplayable-sets-cores-and-the-residual); the frame's instance
is separated), the movers hold no stopping set over F₀ less L ∩ C₂, which the peel decides.  ∎

**Frame 2 alone.**  With the matroids dropped, or priced by Lagrange multipliers as in [the per-frame
bound](../the-per-frame-bound-2026-10/README.md), a member of C₂ that the matching leaves unused can always be promoted
out (removing an unused factor keeps the matching and its uniqueness).  So the problem becomes a heaviest uniquely
restricted matching that covers M₀, with the weight of an edge y–f being y's if y ∈ C₃ less f's if f ∈ C₂, plus ΣC₂.
With M₀ and C₂ empty and unit weights it is the largest uniquely restricted matching of a bipartite graph, NP-complete
(Golumbic, Hirst and Lewenstein).  [franklin-moniot.md, "The
hardness"](../../../../docs/theory/franklin-moniot.md#the-hardness) already places the frames there.

**The guess, corrected.**  The guess to test was a heaviest acyclic matching of a bipartite graph, close to a uniquely
restricted matching.  It is a uniquely restricted matching, with three differences from the textbook problem: one side
is partly mandatory (every move of frame 2 must be covered); both sides carry weights, of opposite effect (a candidate
of home 3 gains its value when matched, one of home 2 loses it); and the problem is not alone: through A′ and B the
choices in C₂ and C₃ are tied to frames 1 and 3, so the bound is a matroid intersection with one frame's block replaced
by this family.

**At the nodes, a square problem.**  At all 49 nodes of the searches the heaviest set fills frame 2's freedoms exactly
(its members promoted into frame 2 and out of it are as many as the freedoms), so the frame it leaves is closed: a
square 0/1 matrix, nonsingular over GF(2).  A closed frame is winnable exactly when its matrix has one perfect matching
(Theorem 12 (3) of [franklin-moniot.md, "One count, three
conditions"](../../../../docs/theory/franklin-moniot.md#one-count-three-conditions), the payment set being all the
factors), that is when its rows and columns can be ordered to make it triangular.  The exact problem near the optimum is
to choose the rows (home 3) and columns (home 2) that make it so.

## Method

`wall local --out FILE` (new; [local.rs](../../../frame/examples/wall/local.rs), studies build) writes each node it
measures whole: its promotions, its frames, its candidates with their homes, the GF(2) bound's heaviest set, and for
each candidate the GF(2) bound with its membership flipped (without it if in the heaviest set, with it if not; a study
accessor, `Gf2Bound::live`, gives the loops).  Run with `--checked none`, it takes the same nodes as the bound study
(its sampling depends only on the default search, which is unchanged): the 49 nodes of the units 3940-104, 4225-1.15
and 4416-95 and the first 400,000 steps of the whole games 4050 and 4176, gaps 185 to 1,211; and, with no known optimum,
4548's root.  Python scripts rebuild frame 2 from the frames (a mover's live factors are its maximal factors among the
frame's factors) and measure it.  Checks: every frame as dumped is winnable by the Python peel; no two movers share
two factors at any node.  The dumps took about 0.3 CPU-hours (345 s of it the default searches), the builds about 0.2
and the scripts a few minutes, on the laptop at a low priority; nothing ran on EC2.

*Fixing by the GF(2) bound* takes out of the problem every candidate whose flipped bound is below the node's need: a
member of the heaviest set without which the bound falls short is in every completion that beats the optimum (forced),
and another candidate with which it falls short is in none.  This is valid for the hybrid bound, whose family lies
inside the GF(2) bound's.  What is left is what any exact method must still decide at the node.  The *kernel* is the
movers that some choice of the free candidates could put in a stopping set (the peel's residual with every free mover in
and every free factor out), with their factors.

The bar comes from the bound study's data: at its 34 nodes of gap 400 to 1,000 whose proofs finished, the proof took a
median 4,017 steps and 1.6 s of CPU (0.009 to 86 s), and the search exact in frame 2 a median 2.1 s.

## Results

### Sizes and degrees

At every node frame 2 is one connected graph (one node of 4050 aside, with 18 components) of 520 to 770 vertices: 157
to 227 moves, 128 to 186 candidates of home 3 that may join them, and 215 to 362 factors, nearly all of them candidates
of home 2.  Half the movers have three live factors and a fifth four or more, so most of the graph lies outside the
regime of two live factors where the GF(2) bound is exact.  By search, the median and range over its nodes:

| search | nodes | moves | candidates of home 3 | factors (candidates of home 2) | freedoms | movers with 1 / 2 / 3 / 4+ live factors | vertices | cycle rank |
|---|---|---|---|---|---|---|---|---|
| 3940-104 | 10 | 206 (202-212) | 137 (120-145) | 276 (274) | 68 (39-84) | 0.02 / 0.31 / 0.51 / 0.18 | 620 (583-633) | 351 (344-378) |
| 4225-1.15 | 10 | 209 (203-219) | 165 (143-173) | 312 (310) | 104 (67-122) | 0.01 / 0.25 / 0.53 / 0.20 | 686 (644-701) | 416 (399-419) |
| 4416-95 | 10 | 216 (211-222) | 154 (139-180) | 300 (296) | 88 (53-105) | 0.04 / 0.29 / 0.48 / 0.19 | 668 (632-721) | 373 (351-446) |
| 4050, the first steps | 10 | 198 (157-211) | 156 (140-166) | 275 (272) | 71 (50-121) | 0.01 / 0.28 / 0.53 / 0.17 | 635 (521-677) | 397 (264-399) |
| 4176, the first steps | 9 | 209 (200-227) | 162 (128-172) | 297 (296) | 93 (59-129) | 0.01 / 0.26 / 0.54 / 0.19 | 669 (624-701) | 411 (368-414) |
| 4548, the root | 1 | 224 | 186 | 362 (362) | 138 | 0.01 / 0.23 / 0.53 / 0.23 | 772 | 459 |

### The arithmetic shape, and the treewidth

The graph is a dense smooth core with tree-like fibers hung on a few hubs, and the core carries the width.

| search | treewidth: lower bound; min-degree; min-fill | smooth core (vertices) | its treewidth: lower; min-fill | fibers; the largest (vertices) | hubs | treewidth without the hubs (min-fill) | twin families of 2+; share of movers in them |
|---|---|---|---|---|---|---|---|
| 3940-104 | 13 (11-13); 24 (24-27); 25 (24-25) | 266 (247-269) | 9; 26 (24-26) | 45; 35 (30-35) | 20 (19-21) | 11 | 28 (27-28); 0.87 (0.80-0.88) |
| 4225-1.15 | 13 (13-14); 27 (26-32); 25 | 280 (274-290) | 9 (9-10); 28 (27-28) | 48; 40 (37-40) | 21 | 13 | 29 (29-30); 0.87 (0.87-0.88) |
| 4416-95 | 12 (12-14); 28 (26-30); 26 (24-27) | 301 (281-329) | 10 (9-10); 26 (24-30) | 49; 30 (28-36) | 17 (17-20) | 15 (13-15) | 28 (26-31); 0.88 (0.87-0.88) |
| 4050, the first steps | 12 (11-13); 25 (22-25); 25 (22-25) | 274 (247-292) | 10 (9-10); 27 (23-28) | 46 (22-46); 36 (26-40) | 21 (14-21) | 12 (12-13) | 28 (22-28); 0.86 (0.65-0.87) |
| 4176, the first steps | 13; 33 (27-35); 25 (25-28) | 279 (264-294) | 9 (9-10); 27 (26-28) | 47; 40 (33-40) | 21 (19-21) | 13 | 30 (28-30); 0.87 (0.87-0.88) |
| 4548, the root | 14; 30; 28 | 351 | 10; 30 | 52; 40 | 21 | 15 | 32; 0.88 |

* **No 4-cycle.**  None of the 46 distinct graphs has two movers sharing two factors.
* **Fibers are thin.**  Every fiber has treewidth at most 3.  A fiber is the layer of the numbers up to n/p, times p,
  which is small for p above 22.
* **The core is not.**  The 22-smooth numbers of the layer have treewidth at least 9 or 10 alone, and the whole graph
  at least 11 to 14 (min-fill 22 to 28).  Taking out the 14 to 21 hubs leaves treewidth at most 11 to 15.
* **Twin families.**  22 to 32 hubs head families of two movers or more, holding 65-88% of the movers.  The largest are
  16·p (40 to 60 members), 24·p (35-40), 36·p (26-28) and 40·p (22-27), families like those whose cheap twin swaps
  separate the near-best completions ([completions near the
  optimum](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md)).

### The heaviest set fills frame 2

At every node of the searches the GF(2) bound's heaviest set uses all of frame 2's freedoms (at 4548's root all but
one) and is independent in its matroid, so the frame it leaves is a square matrix, nonsingular over GF(2).  Yet the peel
strands 126 to 167 movers, about 60% of them:

| search | nodes | the heaviest set fills frame 2 | independent there | promoted in; out | movers left in the peel's residual |
|---|---|---|---|---|---|
| 3940-104 | 10 | 10 | 10 | 18 (14-25); 50 (25-59) | 146 (137-152) |
| 4225-1.15 | 10 | 10 | 10 | 38 (27-45); 64 (37-77) | 159 (155-167) |
| 4416-95 | 10 | 10 | 10 | 34 (18-45); 52 (35-60) | 145 (140-154) |
| 4050, the first steps | 10 | 10 | 10 | 34 (21-43); 38 (28-78) | 148 (126-155) |
| 4176, the first steps | 9 | 9 | 9 | 35 (14-43); 53 (39-86) | 163 (161-165) |
| 4548, the root | 1 | 0 | 1 | 48; 89 | 164 |

So the bound's set is far from winnable in frame 2 as the peel sees it, though the completions that beat the optimum
lie within the gap of it.

### After fixing by the GF(2) bound

Under a gap of 1,000, fixing takes out about half the candidates, nearly all of them fixed out (forced members are rare:
34 at most), and leaves frame 2's problem about as wide as before.  By band of gap:

| gap | nodes | candidates | fixed in; out | free | frame 2: movers that may join; factors that may leave | kernel movers | kernel's 2-core (vertices) | treewidth: lower bound; min-fill |
|---|---|---|---|---|---|---|---|---|
| 185-399 | 8 | 961 (862-1003) | 0; 656 (628-745) | 273 (187-350) | 46 (40-75); 100 (66-128) | 242 (210-260) | 420 (370-447) | 9 (8-10); 21 (19-24) |
| 400-1,000 | 37 | 1042 (890-1117) | 0 (0-34); 545 (287-689) | 458 (319-791) | 89 (58-135); 159 (100-251) | 279 (231-320) | 474 (368-519) | 10 (9-12); 24 (21-27) |
| 1,001-1,211 | 4 | 1058 (978-1152) | 0; 240 (186-295) | 818 (792-857) | 132 (120-141); 254 (244-265) | 326 (314-339) | 522 (504-541) | 12 (11-12); 26 (25-27) |

The kernel keeps most of the frame's moves: with even a hundred of its factors free to leave, nearly every move can sit
in some stopping set.  The gaps here are the GF(2) bound less the need, plus one, at the node's first step.

## The literature

* **Hard in general.**  The largest uniquely restricted matching is NP-complete on bipartite graphs and split graphs
  (Golumbic, Hirst and Lewenstein 2001), APX-complete on bipartite graphs of degree at most 3 and hard to approximate
  within n^(1/3−ε) on bipartite graphs (Mishra, as cited by Chaudhary, Sau and Zehavi, arXiv:2508.12004).  Its
  alternating-cycle-free form is NP-complete on chordal bipartite graphs (H. Müller, "Alternating cycle-free matchings",
  *Order* 7 (1990)).  Franklín and Moniot's Theorem 2 is the same hardness for the game on two ranks.
* **Easy classes.**  Linear time on threshold graphs, cacti and block graphs (Golumbic, Hirst and Lewenstein), on
  proper interval and bipartite permutation graphs, polynomial on interval graphs (Francis, Jacob and Jana), and on
  bipartite distance-hereditary graphs (Müller).  The bipartite graphs of these classes are chordal bipartite (every
  cycle of length 6 or more has a chord), but for the cacti, which have treewidth at most 2.
* **By treewidth.**  O(2^(tw²/2)·n) for uniquely restricted matching (Chaudhary, Sau and Zehavi 2025), and
  2^O(k log k)·n for Morse matchings on a complex of treewidth k, which no 2^o(k log k) algorithm beats unless the
  exponential time hypothesis fails (Philip and Vågset, arXiv:2603.05406).  The state of such a dynamic program
  carries an order of the bag's vertices, or the alternating paths between them.
* **Integer programming.**  The exact solvers of Morse matchings rest on the matching LP with Joswig and Pfetsch's
  per-level cycle inequalities.  On our problem [flat-cycle
  cuts](../flat-cycle-cuts-on-the-matching-lp-2026-10/README.md) found that they close a median 0.12 of the GF(2) gap
  and leave 37-78% of it, so a solver built on that LP would branch about as the search does.  No benchmark of an exact
  solver for these problems at our size was found.
* **Our instances fit none of the easy classes.**  Frame 2's graphs have no 4-cycle (two movers share at most one
  factor: [flat-cycle cuts, "Method"](../flat-cycle-cuts-on-the-matching-lp-2026-10/README.md#method), checked at
  every node here).  A chordal bipartite graph without a 4-cycle is a forest, since a chord of an even cycle of length
  6 or more splits it into two shorter even cycles, down to a 4-cycle; so every bipartite class above meets ours only
  in forests, where the GF(2) bound is already exact (a stopping set in a forest has fewer factors than movers, so it is
  dependent).  Ours have cycle rank 264 to 459 and treewidth at least 11 (8 after fixing).  Whether the problem
  stays NP-hard without 4-cycles is not settled in what was found; the approximation algorithms of Baste, Rautenbach and
  Sau (arXiv:1611.06815) treat that class.

## Decision

Not prototyped; frame 2's exact problem has no fast exact method in view.

* **Treewidth is the only general handle, and it is too wide.**  After fixing, frame 2 at a node of gap 400 to 1,000
  has treewidth at least 9 to 12 (at most 21 to 27).  An order-based program at width 9 holds 10! · 3¹⁰ ≈ 2·10¹¹
  states a bag (orders of the bag's vertices, and each vertex's place in the matching), and the alternating-path
  program 2^(10²/2) ≈ 10¹⁵; the proof it would replace takes a median 1.6 s.  The gap to the bar is many orders of
  magnitude, not a constant to engineer away.
* **The width sits in the arithmetic, not in the node.**  The smooth core alone has treewidth at least 9 at every node,
  and fixing removes candidates but not the core's moves.  Fibers have treewidth at most 3, so the hard part is the
  lattice of the numbers with small primes, and the hubs that join the fibers to it, in every game measured here from
  3940 to 4548.
* **No easy class.**  No 4-cycle and hundreds of independent cycles put the graphs outside every class where the
  problem is known to be polynomial.

What would reopen it: a structural theorem on stopping sets of the divisor lattice's layers (where in the smooth core
an odd core can sit), or a decomposition of the core along its primes with narrow interfaces, neither of which is known.
The square form of the problem at the nodes (choose rows and columns so that the matrix becomes triangular) is the
statement such work would start from.

## Files

* `instances.jsonl.gz`: the 50 nodes, one line each (the fields are in the module comment of
  `solver/frame/examples/wall/local.rs`).  Made, from `solver/frame`, after `cargo build --profile test --example wall
  --features studies`, by `dump.sh` (each command under `renice -n 10`): `wall local --known ../../results/strict.jsonl
  --suite ../notebook/search/the-hardest-searches-of-the-run-to-5000-2026-10/suite-hard.jsonl --ids ID --per-unit 12
  --checked none --out FILE N` for the three units; `wall local --known ../../results/strict.jsonl --root --nodes 400000
  --per-unit 10 --checked none --out FILE N` for 4050 and 4176; and `wall local --known ../../results/strict.jsonl
  --root --out FILE 4548`.
* `f2lib.py`: frame 2 rebuilt from a line, the peel, the kernel, the split by the large prime, treewidth bounds.
* `summary.py`: the tables above, `python3 summary.py instances.jsonl.gz`; `analysis.txt` is its output followed by
  that of the per-node scripts: `structure.py` (sizes, kernel without fixing, treewidth), `fibers.py` (core, fibers,
  hubs), `families.py` (twin families, the 4-cycle check), `fixing.py` (after fixing), `check.py` (each frame winnable
  as dumped; the heaviest set's residual in each frame) and `gf2check.py` (the heaviest set's independence in frame 2).
* `bar.py`: the bar, from the bound study's `local.jsonl.gz`.

## Commits

* 4fb4b03: `wall local --out`, `Gf2Bound::live` in the studies build, and a test of the line.
* The commit of this entry.
