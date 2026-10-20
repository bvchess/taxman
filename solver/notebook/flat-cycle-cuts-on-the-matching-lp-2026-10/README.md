# Flat-cycle cuts on the matching LP (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  Do Taxman's own flat-cycle cuts close the gap that the field-rank bounds (GF(2), A1) leave?  A
measurement of LP bounds only, with no search, at the roots of searched games past 2175 and at the big-subtree nodes
of the tuning suite.  The rule, fixed in advance: a closure clearly above A1's (a median 42% of the excess at the nodes
of gap 700 and up, [the A1 bound at the nodes](../the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/README.md))
means the cuts are worth putting into the branch and bound; otherwise the gap that remains is not cycle-shaped.

## Background: Morse matchings

A playable set is the set of upper ends of a matching of the maximal-factor graph with no flat alternating cycle, a
cycle within two adjacent ranks (Franklín and Moniot's Theorem 1, which is THEORY.md's Theorem 1 read set by set:
[THEORY.md, "Relation to Franklín and Moniot"](../../../docs/theory/franklin-moniot.md)).  That is the
*acyclic* or *Morse* matching of discrete Morse theory, on the divisor poset: Joswig and Pfetsch observe that every
directed cycle of a matching's modified Hasse diagram lies in one level, so "flat" is exactly "acyclic".  They solved
maximum Morse matchings exactly by branch and cut, with the matching inequalities, Betti-number inequalities and, per
level, the cycle inequalities `x(C) ≤ |C|/2 − 1` over a cycle's edges (facet-defining, separable in polynomial time).

The desk study ([literature.md](literature.md), 2026-10-18; primary texts read for Franklín–Moniot arXiv:2211.00461,
Joswig–Pfetsch math/0408331, Benedetti–Lutz 1303.6422, Adiprasito–Benedetti–Lutz 1404.4239, Björner 1101.5704,
Severs–White 1012.3387 and the SCIP 10 paper 2511.18580) found the equivalence right as combinatorics, with two caveats:
the graph theorists' acyclic or uniquely restricted matchings constrain the whole induced subgraph, so their results
transfer only level by level; and the divisor poset is not a regular cell complex, so Betti numbers bound only the
matchings that avoid the edges raising a prime's exponent to an even value.  The field-rank bounds are already
stronger than Joswig and Pfetsch's Betti inequality (a sum of two adjacent per-level rank bounds), and nothing
published gives a sharper bound (Deaett and Grace 2511.02935 show a residual gap is possible for any rank bound).  Their
cycle inequalities are the one computable family that is level-local and that rank bounds miss.  The verdict: worth a
small experiment, the edge variables and the cycle inequalities on the matching and rank rows, against A1's 42%; a
closure well above it justifies the branch and bound, otherwise the gap is not local acyclicity.

## Two readings, checked

**1. Two live factors.**  The reading: a mover with exactly two live factors is an edge between them, a set of such
movers is playable exactly when its edges form a forest, which GF(2) rank captures, so cycle cuts can add something
only through movers with three live factors or more.  *It holds in the frame model, frame by frame, with "live" taken
after the promotions.*  A frame with movers `M` over its pool `P` (its factors less the candidates promoted out of it)
is winnable exactly when the peel empties `M`, that is when no nonempty subset of `M` is self-covering (every live
factor of a member listed by two members or more: Franklín and Moniot's Theorem 6, `wall/fields/cores.rs`).  For
movers with two live factors a self-covering set is a subgraph whose every vertex has degree 2 or more, which exists
exactly when the edges hold a cycle; a mover with one live factor is a half-edge, and a component may hold one.  Both
are exactly the independence of the movers' vectors over `P` in GF(2) (or any field).  The frame model's twist is that
`P` depends on the promotions out of the frame; the GF(2) frame matroid puts a promotion out as the unit vector of the
factor, and `{v₁, …, v_k, e_y}` is independent exactly when the `v_i` restricted to `P ∖ {y}` are, so it is exact
there too.  Hence a GF(2)-independent set that cannot be promoted (an odd core) has a mover with three live factors or
more, and the GF(2) bound's error lives there.  For the edge LP measured here, which is not GF(2), a cycle whose movers
all have two live factors gives `x(C) = Σ_{u ∈ C} z_u`, a circuit inequality of the graphic matroid: implied by the
GF(2) bound but not by the LP, so stage (c) separates those too and counts them.

**2. No flat 4-cycles; the shortest cycles from the arithmetic.**  Two numbers `u ≠ v` with as many prime factors
share at most one maximal factor: if `f = u/p` and `g = u/q` (`p ≠ q`) are both maximal factors of `v` too, then
`lcm(f, g) = u` (it takes `p`'s exponent from `g` and `q`'s from `f`) and likewise `= v`.  So two factors are joined by
at most one mover, and a flat cycle with `k` movers is a `k`-cycle of the *factor graph* (two factors joined by the
mover that lists both) whose `k` edges are distinct movers.  In exponent vectors a cycle's steps remove a prime and add
another, the removed and the added primes being the same multiset; working through the cases, a hexagon is always three
picks over a base and a triangle of primes, `X·pq, X·pr, X·qr` over `X·p, X·q, X·r`, as expected, but the octagons come
in **two** shapes: the squares of primes `X·pq, X·qs, X·sr, X·rp` over `X·p, X·q, X·r, X·s`, and the four triples of
four primes `X·abc, X·abd, X·acd, X·bcd` over four of their six pairs (`30, 105, 42, 70` over `10, 15, 21, 14`).  So
the longer cycles are not all polygons of primes.  `shapes.py` lists them on the whole maximal-factor graph: up to
3441, no pair of movers shares two factors, all 8,182 hexagons have the first shape, and of the 152,077 octagons
149,992 are squares and 2,085 are four triples ([shapes_output.md](shapes_output.md)).  The instrument lists them from
the factor graph's triangles and 4-cycles, which finds every one whatever its shape, and checks every hexagon's shape
at every instance.

## The LP and the cuts, and why each is valid

The *matching graph* of a node (the root is a node) has a vertex per number: the moves and factors of the node's frames,
and its candidates.  A *mover* of frame `h` is a move of `h` or a candidate that a promotion makes a move of `h`; it has
an edge to each of its live factors, its maximal factors among `h`'s factors.  A candidate is one vertex, whether it
pays in frame `h` (promoted) or is paid with in frame `h + 1`.  The LP: `x_e ∈ [0, 1]` per edge; a row per vertex,
`= 1` for a move and `≤ 1` for the others; `Σ z ≤ max_add`, with `z_x` the sum of a candidate's edges as a mover;
maximize `Σ x·z_x`.  Without cuts it is the crate's matching bound (checked at every instance).

* *Integer points.*  Every promotable completion `Q` is a 0/1 point: in each frame with `Q` applied, the peel's order
  pays each mover with a factor that no mover left after it lists, so the payments are a matching with no alternating
  cycle (as in the bridge lemma of THEORY.md's section on Franklín and Moniot: the payment of an alternating cycle's
  first-peeled mover is listed by the next mover on the cycle, still unpeeled, so it was not private), using no member
  of `Q` as a factor.
* *Cover cuts* `Σ_K z ≤ |K| − 1`, `K` a minimal conflict of the node (`FrameGraph::conflict_using`), the anatomy's
  separation by thresholds ([the anatomy of the gap](../the-anatomy-of-the-gap-2026-10/README.md), finding 2).  Valid:
  `K` cannot be promoted from the node, and promotability is hereditary in the frame model (dropping a member removes a
  mover from one frame, whose peel order still serves, and returns a factor to another frame's pool, which stays
  winnable since a self-covering set over the larger pool would be one over the smaller), so no completion holds all
  of `K`.
* *Rank cuts* `Σ_T z ≤ r` on the anatomy's sets of at most 26 candidates (the groups of fractional candidates linked by
  tight cuts, their supports, the unions of two or three tight cuts sharing a fractional member), `r` the largest
  integer below `Σ_T z`, made only when an exhaustion proves that no promotable set holds more than `r` of `T` (by
  heredity such a set is `T` less at most `|T| − r − 1` members, and every unpromotable subset has a conflict one of
  whose members is out; 3,000 conflict computations at most a cut).
* *Flat-cycle cuts* `x(C) ≤ |C|/2 − 1` on the edges of a cycle `C` of a frame's graph (Joswig and Pfetsch's): a matching
  holds at most `|C|/2` of `C`'s edges, and all of them only if `C` alternates.  Every cycle of a frame is flat.
  Stage (b) adds the violated listed hexagons, then octagons, through a mover with three live factors or more; stage
  (c) separates every violated cycle exactly: with `1 − x_{uf} − x_{uf'} ≥ 0` for a mover's two cycle edges (by its
  row), the cut is violated exactly when these sum to less than 1 around the cycle, so for each mover and pair of its
  edges a Dijkstra over the other movers' pairs finds the lightest cycle through it (a path that meets a mover twice is
  shortcut there, which makes it no heavier, so the cycle is simple).
* *No cut on picks alone beyond the anatomy's, and no weighted strengthening, was used.*  The weights (the numbers)
  enter through the objective; a weighted lift of a cycle inequality would need a proof this entry does not have.  The
  separations add the most violated first, and where stage (c) ends uncapped no flat-cycle inequality is violated at
  all, so the order cannot change its value.
* *The bound.*  The LP is solved by the A1 study's simplex (`fields/lp.rs`, floating point, no new dependency), and
  each bound is `⌊B⌋` of the weak-duality bound of the duals it ends with (`≤` rows' duals clamped at 0, every
  `x_e ≤ 1`), computed apart from the simplex, valid whatever state the simplex is in.

The stages run in order on one LP, each keeping the cuts before it: `cover` (cover cuts until none is violated), (a)
(cover and rank cuts), (b) hexagons (`b_hex`), then octagons too, (c) the separated cycles too, and `c all` (every
family together: the cover and rank cuts that the cycles' point opens).  (b) and (c) add cycle cuts alone, so what
they move is the cycles' doing.  A stage runs rounds (separate, add, re-solve) until a round adds nothing or its 45
seconds run out (*capped*).

## What was run

`wall cycles` (`examples/wall/cycles.rs`, research code under the Cargo feature `studies`, commit ca3e13a; no solver
change; the data were made by its code before a method and some comments were renamed and a prime test rewritten for
clippy, which changes nothing it computes), built with `cargo build --profile test --example wall --features
studies`, on the laptop, one thread at `nice -n 10`:

```text
wall cycles --known results/strict.jsonl \
    --suite solver/notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl \
    --units 3 --seconds 45 --out cycles.jsonl 2700 3129 3213 3300 3425 3441
```

(the paths as from the top of the repository; it ran in `solver/frame`.  It was started with 3440 in the list too and
stopped before it, to keep within an hour: 3440's root bound is 3441's.)  The instances are the roots of six searched
games past 2175 with gaps of 731-1,555 (six of the ten whose roots the A1 measurement fitted, so with A1's root share:
[the A1 bound at the nodes](../the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/README.md),
`prepare.jsonl`) and nine units of the tuning suite, three each of 3300, 3425 and 3441 spread over the suite's list
(gaps 753-986, subtrees of 4·10⁴ to 1.1·10⁶ steps; units that the A1 measurement measured too, matched by `need`, with
its node bound `full`, capped at 30 s at all of them).  Their best completions: the optimum at a root; at a unit the
measure is the A1 measurement's, the *excess* over `need` (the optimum plus 1 less the promotions' sum), which every
completion is below (the suite solved each unit's subtree from the optimum and found nothing better), so a share
closed here is what a bound needs to cut the node, and the same quantity as A1's 42%.  The LP has 1,582-2,015 vertices, 3,667-4,763 edges and 2,200-2,900 rows with its cuts; each
instance took 2-4 minutes, 2,811 s in all (and about 10 minutes of trials before).  Checks at every instance: the LP
without cuts equals the crate's matching bound; at the roots every bound is at least the optimum; no pair of movers
shares two factors; every listed hexagon is `X·pq, X·pr, X·qr`; none failed.  `cargo test --features studies
--example wall cycles` checks the same at the root of 600 (two tests, 1.1 s).

## Results

The share of the gap closed, `(GF(2) − bound) / gap` (the gap is GF(2) less the optimum at a root, less `need` at a
unit), by each stage, against A1 on the same instances (`table.py`; the full table with each stage's cuts and seconds
is [table_output.md](table_output.md)):

| instance | gap | (a) | (b) hexagons | (b) | (c) | (c) all | A1 |
|---|---|---|---|---|---|---|---|
| 2700 root | 731 | 0.49 | 0.49 | 0.49 | 0.55 | 0.62 | 0.54 |
| 3129 root | 924 | 0.30 | 0.30 | 0.33 | 0.43 | 0.49 | 0.59 |
| 3213 root | 934 | −0.07 | −0.03 | −0.01 | 0.21 | 0.38 | 0.47 |
| 3300 root | 1,443 | 0.15 | 0.16 | 0.17 | 0.27 | 0.29 | 0.40 |
| 3425 root | 1,407 | 0.01 | 0.02 | 0.05 | 0.20 | 0.22 | 0.34 |
| 3441 root | 1,555 | 0.06 | 0.07 | 0.09 | 0.11 | 0.22 | 0.33 |
| 3300 unit 0 | 753 | 0.20 | 0.25 | 0.25 | 0.40 | 0.42 | 0.42 |
| 3300 unit 4 | 854 | 0.17 | 0.18 | 0.18 | 0.34 | 0.42 | 0.41 |
| 3300 unit 8 | 986 | 0.24 | 0.24 | 0.24 | 0.31 | 0.32 | 0.38 |
| 3425 unit 0 | 879 | 0.12 | 0.13 | 0.18 | 0.25 | 0.33 | 0.38 |
| 3425 unit 4 | 970 | 0.09 | 0.11 | 0.16 | 0.36 | 0.41 | 0.49 |
| 3425 unit 8 | 931 | −0.20 | −0.20 | −0.18 | −0.07 | 0.14 | 0.35 |
| 3441 unit 0 | 960 | 0.62 | 0.62 | 0.63 | 0.63 | 0.73 | 0.42 |
| 3441 unit 4 | 953 | 0.27 | 0.27 | 0.27 | 0.33 | 0.40 | 0.39 |
| 3441 unit 8 | 970 | 0.11 | 0.12 | 0.13 | 0.22 | 0.29 | 0.36 |
| **median, 6 roots** | | 0.10 | 0.11 | 0.13 | 0.24 | 0.34 | 0.43 |
| **median, 9 units** | | 0.17 | 0.18 | 0.18 | 0.33 | 0.40 | 0.39 |

(d) for reference: GF(2) closes 0 by definition and A1 is the last column.  Below (a): the matching bound alone stands
4.3-8.6 times as far above the target as GF(2) (a share of −3.3 to −7.6), and the cover cuts alone leave a share of
−0.5 to −2.5; the rank cuts do the rest of (a).

* **The cycles' own share.**  Stages (b) and (c) add cycle cuts alone, so their increment over (a) is the cycles'
  doing: a median 0.12 of the gap at the roots and 0.13 at the units (0.02 to 0.28).  Listed hexagons add almost
  nothing (median 0.01 at most), the octagons a little more (median (b) − (a) 0.03 and 0.02), and the separated
  cycles the rest: 26-56 hexagons, 57-171 octagons and 13-91 longer cycles were cut an instance, up to 19 movers.  With
  the cover and rank cuts separated again at the cycles' point (`c all`), the increment over (a) is a median 0.17 and
  0.20.
* **Reading 1 in the data.**  Of the movers, 2% have one live factor, 32% two, 55% three and 11% four or more.  Not one
  of the cycle cuts made (listed or separated) had only movers with two live factors or fewer.
* **Totals against A1.**  The LP with all three families (`c all`) closes a median 0.34 of the root gap (A1: 0.43;
  above A1 at one root of six) and 0.40 at the units (A1 on these nine: 0.39, at all 46 nodes of gap 700 or more 0.42;
  above at four of nine, by 0.73 against 0.42 at one).  No unit is cut.
* **What is left is not cycle-shaped.**  Where stage (c) ended before its 45 seconds (7 of the 15: 2700, 3129, the
  units 0 and 4 of 3300 and all three of 3441), its exact separation found no violated flat-cycle inequality at the
  final point, and 0.37-0.78 of the gap remained.  Elsewhere the stages ran out of time, (a) in 13 of the 15, so the
  numbers understate what the LP with these families reaches; but the cycles' increment where (c) finished (median
  0.11) is no larger than where it did not (0.13).

## Verdict

**Not worth putting into the branch and bound.**  The flat-cycle cuts are real (on the matching LP with the anatomy's
cover and rank cuts they close a further 0.12 of the gap, 0.17-0.20 with the cuts they open), but the LP with all
three families does not beat A1: below it at the roots (a median 0.34 of the gap against 0.43) and level with it at
the units (0.40 against 0.39 on the same nine, and A1's 0.42 over all 46 nodes of gap 700 and up), not clearly above,
at 2-4 minutes an instance against A1's 30 s.  Where the separation finished, no flat cycle
is violated and most of the gap remains, so by the rule fixed in advance the gap that the field-rank bounds leave is
not cycle-shaped: it is in what the rank and cover cuts express, the odd cores of the anatomy, not in local
acyclicity.  The desk study's second candidate is no new family in the frame model: a stopping set `W` (every live
factor of a member listed twice within `W`) stays one when the pool shrinks, so its peeling cut `x(W) ≤ |W| − 1` is
the cover cut of an unpromotable set, no stronger than that of a minimal conflict inside it, and its sharper form
`x(S) ≤` the largest peelable subset is a rank cut; both families are in (a).

## Files

* `cycles.jsonl`: one line an instance, the output of the command above (fields in `wall/cycles.rs`'s `measure`:
  the bounds of each stage under its name, `lp0` and `lp_cap` the LP without cuts and with its row `Σ z ≤ max_add`,
  `gf2`, `target` and `gap`, each stage's seconds, rounds, whether it was capped and the cuts it added, the cycle cuts
  by their number of movers, the counts of hexagons and octagons, the movers by live factors, the LP's size and
  pivots).
* `table.py`: the tables of this entry from `cycles.jsonl` and the A1 measurement's files; `table_output.md` is its
  output.
* `shapes.py`: the shapes of the hexagons and octagons of the maximal-factor graph (`shapes.py 3441`, whose output is
  `shapes_output.md`).
* `literature.md`: the desk study's report.
