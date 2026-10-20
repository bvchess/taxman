# Dominance and symmetry (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  A proof's nodes are about `a + c·G` decades for a root gap `G`, with `c` the same at every n, so only a
lower cost per unit of gap cuts the slope.  A search that must rule out every promotion set inside the gap pays
separately for every near-optimal variant.  How much of a proof's tree is *dominated* (another promotion set does at
least as well with every completion, so the node can be pruned) or explored twice under interchangeable promotions?
Which exchanges are provably safe, what share of nodes would a sound rule remove, and does that share hold or grow with
n?  **Answer.**  Literal dominance is rare (0-1% of the probe's nodes, 0-5% of sampled proof nodes) and there are no
twins or transpositions.  But the frame model has symmetries, exchanges of primes within classes (3.8-5.9 classes a
game at 851-1350, moving 11-14% of the candidates), and breaking them is sound and removes about a seventh of the nodes
(geometric means 0.88 in the solver, 0.86 in the GF(2) branch and bound): a constant factor, not a lower cost per unit
of gap.

## Method

No solver code changed.  The study is `examples/dominance.rs` and `examples/common/symmetry.rs`, research code since
removed from master (they build at commit 8753189, with `matroid.rs`: `git checkout 8753189`, then `cargo build
--release --example dominance --example matroid` in `rust/`), run on one thread, at most two processes, on the shared
laptop.

**Definitions.**  A node `(P, C)` is dominated by a set `P'` (a candidate set, feasible, disjoint from `C`) when every
completion `S ⊆ C` of the node that could beat the incumbent is promotable from `P'`'s state and `P' ∪ S` is worth
more.  Pruning such nodes is sound: the pruned sets map to feasible sets worth strictly more (for two-for-two
exchanges of equal sums, larger as binary numbers), so the heaviest feasible set, the largest of them in that order, is
never pruned.  The completions that could beat the incumbent are within the node's *relevant* candidates: those that
can be promoted and with which the matching bound, plus themselves, still reaches what the node needs.  `P'`'s state
*covers* the node's when, frame by frame, its core has no move the node's core lacks and every factor of the node's
core that its moves or the incoming candidates can use (the proof is the comment of `covers` in
`examples/dominance.rs`); the cover is also tried with moves relabelled and up to an exchange of two primes
(`covers_mapped`).  A *sampled* estimate replaces the proof by 64 random maximal completions within the relevant
candidates: an upper estimate of what any cover could prove (1,024 samples gave the same counts within two nodes in a
hundred).

**Nodes**: the probe's fixtures (units of the proofs of 936 and 1215, mostly shallow: 13-16 promotions); nodes of the
solver's own proofs sampled uniformly over their steps (`dominance sample --count K`: the deterministic one-thread
proof stopped after `k·T/(K+1)` steps and the node it stopped in read back from its checkpoint), 40-100 nodes each of
900, 936, 950, 1000, 1100, 1184, 1215, 1250, and of 1000 and 1250 with the incumbent at `P*` + 100 and + 200; and nodes
of the GF(2) branch and bound's trees (`matroid bnb --bounds gf2`; one in 60 to one in 200 of 930, 990, 1020, 1210, 1230
and 1300).  `dominance nodes` tries at each node every exchange of one excluded candidate added, one for one (`a > b`),
and, with `--pairs`, two for one and two for two; every exchange of two primes applied to `P`; twins among the
candidates (same home frame, same live factors below, same users above: an automorphism of the node's problem); and
the symmetry rule of Results 2.

**Two pilots** (code not kept).  The *solver pilot* copied `src/search/searcher.rs`, `refine.rs` and `lagrange.rs` into
an example and added one hook in `Stepper::check`, after the matching bound, where the symmetry rule may cut the node;
with the rule off it reproduces the proofs node for node (900 4,194; 936 41,653; 1250 709,893, and four more).  The
*GF(2) pilot* copied `examples/matroid.rs`'s frame matroids and its `bnb` loop (bound `gf2`) and added the rule after
the bound; with it off it reproduces that study's trees (936 3,392; 1250 2,543, and two more).  Both run the incumbent
as a value, `P*` or `P*` + δ, as `slope prove` does.

## Results

### 1. No transpositions, no twins, and literal dominance is rare

* **Transpositions**: in the GF(2) branch and bound's trees of 930, 990, 1020, 1210, 1230 and 1300 (133,006 nodes) no
  reduced state is reached by two different promotion sets; the states that repeat (19-21% of the nodes) are the
  conflict branching's siblings, the same `P` with fewer candidates.  **Twins**: none at any node of any sample.
* **Adding an excluded candidate** (`P + a`): proved at no node.  At 57% of the fixtures' nodes some excluded candidate
  can be added and no single relevant candidate blocks it, yet some random maximal completion always does.
* **Literal covers** (one for one, two for one or two, with or without relabelled moves) at the solver's nodes: 0-1%
  of the fixtures' nodes and 0-5% of the sampled proof nodes; the sampled estimates are far higher (up to 53%).  For
  example, at a node of 936 the swap 426 → 438 (6·71 → 6·73) passes every sample: the families {284, 426, 568, 852} and
  {292, 438, 584, 876} are the same graph in the node's frames, which a literal cover cannot see because the numbers
  differ.  The exchanges behind the gap are the subject of 2.

### 2. The frame model has symmetries: exchanges of primes

Exchanging the exponents of two primes `p < q` in every number is an automorphism of divisibility.  When it maps the
root's frames onto themselves (moves to moves, factors to factors, the factor graph among them onto itself) and the
candidates onto themselves, it maps every promotable set `S` to a promotable set worth `(q − p)·(c_p(S) − c_q(S))`
more, `c_r(S)` being the sum of `x / r` over the members of `S` divisible by `r` (`root_symmetry` and `root_pairs` in
`examples/common/symmetry.rs`; no candidate is divisible by both or by a square in any pair found).  The pairs are found
by checking that directly, not by the primes' multiples on the board (at 851, 59, 61 and 67 are one class though their
floors differ: the frames see only some multiples).  At 1300 the classes are {41, 43}, {59, 61}, {67, 71}, {73, 79},
{89, 97, 101, 103, 107}, {109, 113, 127, 131, 137, 139} and {149, 151, 157}.

| n | games | candidates | classes of primes | largest class | candidates moved | log10 of the group's order |
|---|---|---|---|---|---|---|
| 851-900 | 50 | 211 | 3.8 | 4.3 | 23.9 (11%) | 4.5 |
| 951-1000 | 50 | 236 | 4.1 | 6.1 | 28.5 (12%) | 5.1 |
| 1051-1100 | 50 | 266 | 4.6 | 6.3 | 28.3 (11%) | 5.2 |
| 1151-1200 | 50 | 289 | 4.6 | 6.7 | 28.9 (10%) | 5.9 |
| 1251-1300 | 50 | 322 | 5.8 | 6.7 | 41.5 (13%) | 6.7 |
| 1301-1350 | 50 | 336 | 5.9 | 5.9 | 48.4 (14%) | 7.4 |

**The rule.**  A set worth beating the incumbent with `c_p > c_q` for a symmetric pair has an image worth more, so only
sets with `c_p ≤ c_q` for every pair need searching, and a heaviest set always keeps them all: a node can be cut when,
for some pair, no completion that keeps the constraint can reach what the node needs.  The constraints of a class are
the order of its primes by their cofactor sums.  Three tests of that, from cheap to dear: *hopeless* (no completion can
lift `c_q` to `c_p`: `c_p(P) > c_q(P) + c_q(C)`); the *matching bound of the constraint* (each subset `U` of the pair's
multiples among the candidates, at most 10 or 12, promoted, the others left out, the rest relaxed; solver pilot and
`dominance nodes`); and the *Lagrangian of the constraint* on the GF(2) bound (each multiple `m·p` weighing `m(p − μ)`
and `m·q` weighing `m(q + μ)`, plus `μ(c_q(P) − c_p(P))`, for `μ = (q − p)/2` and `q − p`; GF(2) pilot).  The two dearer
tests run only for pairs whose constraint the bound's heaviest set breaks.

**Checks** (`dominance symcheck --known results/strict.jsonl 2-1300`, `symcheck.jsonl`).  In all 1,170 games with
pairs, the images of the optimal set and of 50 random maximal promotable sets are promotable and worth what the formula
says; on the 22 boards with pairs and at most 22 candidates every promotable set (97,839 in all) is checked and the best
that keeps the constraints is `P*`; the brute-force oracle scores the known optimum to n = 40.  Both pilots with the
incumbent at `P*` − 1, so that they must find an optimal set themselves, find one in every game of 2-900 (880 games
each; the rule cut in 573 and 353 of them).

### 3. What the rule and the covers remove: a sixth to a third of the nodes, a constant share

At the sampled nodes (`dominance_output.md` has the per-game tables): "proved" is any of the covers, the prime
exchanges and the rule's matching test; "sampled" its upper estimate.

| tree | n | proved | sampled |
|---|---|---|---|
| solver proofs | 900, 936, 950, 1000 | 0%, 25%, 32%, 43% | 0%, 46%, 53%, 50% |
| | 1100, 1184, 1215, 1250 | 36%, 8%, 10%, 25% | 39%, 20%, 12%, 35% |
| incumbent `P*` + 100 | 1000, 1250 | 28%, 8% | 52%, 15% |
| incumbent `P*` + 200 | 1000, 1250 | 0%, 12% | 50%, 27% |
| GF(2) branch and bound | 930, 990, 1020 | 54%, 25%, 16% | 55%, 25%, 21% |
| | 1210, 1230, 1300 | 7%, 19%, 35% | 10%, 20%, 39% |

Two-member exchanges add little (sampled, with everything else: 49%, 34%, 48% of the nodes of 936, 1184, 1250; 25%, 7%,
46% of the GF(2) nodes of 1020, 1210, 1300).  In the GF(2) trees what is proved is nearly all that is sampled, and it
is the rule; in the solver's trees the rest of the estimate is single swaps along a symmetry (426 → 438 above) and
exchanges of primes that are alike only at the node (29 and 31, 17 and 19, 61 and 67 at nodes of 936).

The pilots, nodes with the rule against without, the incumbent at `P*`:

| search, rule | games | nodes, geometric mean ratio | total | fewest | most | log ratio on log nodes | time |
|---|---|---|---|---|---|---|---|
| solver, hopeless | 880-1090 by 10 (22) | 0.962 | 0.960 | 0.85 (950) | 1.07 (1040) | −0.002 | +15% |
| solver, matching bound of the constraint | the same | 0.878 | 0.829 | 0.67 (1000) | 1.16 (940) | −0.028 | ×1.9 |
| GF(2) B&B, hopeless | 880-1300 by 10 (43) | 0.948 | 0.925 | 0.71 (940) | 1.00 | −0.016 | −3% |
| GF(2) B&B, Lagrangian of the constraint | the same | 0.859 | 0.826 | 0.57 (930) | 0.98 (1100) | −0.030 | −3% |

In the solver the rule's cuts change what the searcher learns, so a tree can grow: 1184 52,259 to 66,250 (+27%) with the
matching test, 940 +16%; 1250 709,893 to 678,415 (−4%).  The GF(2) pilot's margin scans (δ = 0, 25, 50, 100) keep the
ratio per game (930 0.57, 0.60, 0.61, 0.92; 990 0.77 to 0.79; 1300 0.67 to 0.75).  The share does not grow with n (the
log ratio changes by ±0.02 decades per 100 in n in both pilots) and barely with the tree (−0.03 decades per decade of
nodes: `c` lowered by about 3%), although the symmetric part of the problem does grow: most rearrangements within a
class differ in value by more than the bound leaves, and the bound already cuts them.

## Decision

**Not a slope lever, and not worth a phase of its own.**  The exchange of primes within classes is sound (proved, and
checked to 1300) and removes about a seventh of the nodes, up to 43% in some games: a constant factor.  Exchanges that
hold only at a node add a little more in the solver's trees and almost nothing in the GF(2) trees, and cost a state
comparison per exchange, far more than a step.

* **In the search with the GF(2) bound** the symmetry rule is a cheap addition: the hopeless test from cofactor sums
  kept along the path (nearly free, about 5% of the nodes) and the constraint's Lagrangian at the nodes whose heaviest
  set breaks a pair (about 14%, at one or two intersections there; time-neutral in the pilot, where every bound is
  computed from scratch).  [Prime exchanges in the search](../prime-exchanges-in-the-search-2026-10/README.md) took it
  up.  In the matching solver the constraint costs more than it saves.
* The pairs are found at the root in about 20 ms a game, the setup included; `root_pairs` is the definition to reuse.
* The equal-sum exchanges of the uniqueness study are not symmetries (e.g. 425's {110, 132, 168} ↔ {98, 150, 162});
  they are coincidences of value that only the bound can rule out.

## Files

The commands are those of the removed example (`cargo run --release --example dominance -- COMMAND ...`):

* `samples/solver/`, `samples/solver_d100/`, `samples/solver_d200/` (`nNNNN.nodes`): the nodes of the solver's proofs,
  with the incumbent at `P*`, `P*` + 100 and `P*` + 200 (`dominance sample --known results/strict.jsonl --count K
  --delta D --out DIR N...`); `samples/gf2/`: the nodes of the GF(2) branch and bound's trees (`matroid bnb --bounds
  gf2`).
* `nodes_*.jsonl`: a line a node with the screenings of Results 1 and 3 (`dominance nodes FIXTURE...`), for the probe's
  fixtures (`nodes_fixtures`) and each sample (`nodes_solver`, `nodes_solver_d100`, `nodes_solver_d200`, `nodes_gf2`);
  `nodes_pairs_solver` (936, 1184, 1250) and `nodes_pairs_gf2` (1020, 1210, 1300) add `--pairs`.
* `symmetries.jsonl`: the root's prime pairs of each game of 851-1352 (`dominance symmetries 851-1352`);
  `symcheck.jsonl`: the checks of Results 2 (`dominance symcheck --known results/strict.jsonl 2-1300`).
* `pilots/`: a line a game of the two pilots, with the rule off, hopeless and the constraint's bound (`solver_off`,
  `solver_hopeless`, `solver_bound` on 880-1090 by 10; `gf2_off`, `gf2_hopeless`, `gf2_lagrange` on 880-1300 by 10),
  the GF(2) pilot's margin scan (`gf2_delta`: 930, 990 and 1300 at δ = 0, 25, 50, 100) and the checks with the
  incumbent at `P*` − 1 on 2-900 (`solver_sound`, `gf2_sound`).
* `dominance.py` makes the tables from these files (`python3 -I dominance.py`); `dominance_output.md` is its output.

## Commits

* aa56590 to d2de73f: `examples/dominance.rs` (the work in progress at aa56590; at 90739d7 the prime exchanges and root
  symmetries in `examples/common/symmetry.rs` and the true-tree sampling; at 129e099 `symcheck`; at dea37a0 the
  build against the cleaned-up tactics), and at d2de73f its final form with this folder's data and tables.
* 58bf758: the entry.
* 8753189: the commit the removed examples build at.
