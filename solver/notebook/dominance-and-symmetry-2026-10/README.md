# Dominance and symmetry (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The slope study found a proof's nodes to be about `a + c·G` decades for a root gap `G`, with `c` the
same at every n, so a lower cost per unit of gap cuts the slope as a tighter bound does.  A search that must rule
out every promotion set inside the gap pays separately for every near-optimal variant; the uniqueness study found
optimal sets that differ by exchanging equal-sum groups of lower-half numbers.  How much of a proof's tree is
*dominated* (another promotion set does at least as well with every completion, so the node can be pruned) or
explored twice under interchangeable promotions?  Which exchanges are provably safe, what share of nodes would a
sound rule remove, and does that share hold or grow with n?  IDEAS.md section 5 had dominance rules and symmetry
breaking as considered, not piloted, and "no symmetries found".

*Definitions.*  A node `(P, C)` is dominated by a set `P'` (a candidate set, feasible, disjoint from `C`) when every
completion `S ⊆ C` of the node that could beat the incumbent is promotable from `P'`'s state and `P' ∪ S` is worth
more.  Pruning such nodes is sound: the pruned sets map to feasible sets worth strictly more (for two-for-two
exchanges of equal sums, larger as binary numbers), so the heaviest feasible set, the largest of them in that
order, is never pruned.  The completions that could beat the incumbent are within the node's *relevant* candidates:
those that can be promoted and with which the matching bound, plus themselves, still reaches what the node needs.
`P'`'s state *covers* the node's when, frame by frame, its *core* (the frame for the edits the relevant candidates
can make, less the moves that can always go last and the factors nobody can use) has no move the node's core lacks
and every factor of the node's core that its moves or the incoming candidates can use: then every completion
promotable from the node's state is promotable from `P'`'s (the proof is the comment of `covers` in
`examples/dominance.rs`).  The cover is also tried with moves relabelled (a move stands in for another with the same
live factors) and up to an exchange of two primes (`covers_mapped`).  A *sampled* estimate replaces the proof by 64
random maximal completions within the relevant candidates (infeasibility only grows with a set, so maximal sets are
the ones to try): an upper estimate of what any cover could prove; 1,024 samples gave the same counts within two
nodes in a hundred.

*Method.*  No solver code changed.  `examples/dominance.rs` (commits aa56590 to d2de73f, on the cleaned-up build)
and `examples/common/symmetry.rs`; `bench/dominance/` holds the node samples, the outputs and `dominance.py`, which
makes the tables below (`dominance_output.md`).  One thread, at most two processes, on the shared laptop.

* *Nodes.*  The probe's fixtures (units of the proofs of 936 and 1215, mostly shallow: 13-16 promotions); nodes of
  the solver's own proofs sampled uniformly over their steps (`dominance sample --count K`: the deterministic
  one-thread proof stopped after `k·T/(K+1)` steps and the node it stopped in read back from its checkpoint; 40-100
  nodes each of 900, 936, 950, 1000, 1100, 1184, 1215, 1250, and of 1000 and 1250 with the incumbent at `P*` + 100
  and + 200); and nodes of the GF(2) branch and bound's trees (`matroid bnb --bounds gf2`; one in 60 to one in 200
  of 930, 990, 1020, 1210, 1230 and 1300).  `dominance nodes` tries at each node every exchange of one excluded
  candidate added, one for one (`a > b`), and, with `--pairs`, two for one and two for two; every exchange of two
  primes applied to `P`; twins among the candidates (same home frame, same live factors below, same users above:
  an automorphism of the node's problem); and the symmetry rule below.
* *Two pilots* (code not kept; how to rebuild them is here).  The *solver pilot* copied `src/search/searcher.rs`,
  `refine.rs` and `lagrange.rs` into an example (paths adapted, the diagnostics probe stubbed) and added one hook in
  `Stepper::check`, after the matching bound: the symmetry rule may cut the node (cut reason `refined`).  With the
  rule off it reproduces the proofs node for node (900 4,194; 936 41,653; 950 460,813; 1000 366,846; 1100 29,614;
  1184 52,259; 1250 709,893).  The *GF(2) pilot* copied `examples/matroid.rs`'s frame matroids and its `bnb` loop
  (bound `gf2`) and added the rule after the bound; with it off it reproduces B's trees (936 3,392; 1100 1,874; 1184
  1,093; 1250 2,543).  Both run the incumbent as a value, `P*` or `P*` + δ, as `slope prove` does.

**1. No transpositions, no twins, and literal dominance is rare.**

* *Transpositions*: in the GF(2) branch and bound's trees of 930, 990, 1020, 1210, 1230 and 1300 (133,006 nodes) no
  reduced state is reached by two different promotion sets; the states that repeat (19-21% of the nodes) are the
  conflict branching's siblings, the same `P` with fewer candidates.  Nothing is explored twice literally.
* *Twins*: none at any node of any sample (the candidates' live neighbourhoods differ, as IDEAS.md expected).
* *Adding an excluded candidate* (`P + a`, the "the node went on without `a`, but `a` fits anyway" case): proved at
  no node; at 57% of the fixtures' nodes some excluded candidate can be added and no single relevant candidate
  blocks it, yet some random maximal completion always does.  Excluded candidates stay blockable.
* *Literal covers* (one for one, two for one or two, with or without relabelled moves) at the solver's nodes: 0-1%
  of the fixtures' nodes, 0-5% of the sampled proof nodes; the sampled estimates are far higher (up to 53%), and the
  exchanges behind the gap are the subject of 2.  For example, at a node of 936 the swap 426 → 438 (6·71 → 6·73)
  passes every sample: the families {284, 426, 568, 852} and {292, 438, 584, 876} are the same graph in the node's
  frames, and a literal cover cannot see it because the numbers differ.

**2. The frame model has symmetries: exchanges of primes.**  Exchanging the exponents of two primes `p < q` in
every number is an automorphism of divisibility.  When it maps the root's frames onto themselves (moves to moves,
factors to factors, the factor graph among them onto itself) and the candidates onto themselves, it maps every
promotable set `S` to a promotable set worth `(q − p)·(c_p(S) − c_q(S))` more, `c_r(S)` being the sum of `x / r`
over the members of `S` divisible by `r` (`examples/common/symmetry.rs`, `root_symmetry` and `root_pairs`; no
candidate is divisible by both or by a square in any pair found).  The pairs are found by checking that directly;
they are not just the primes with the same multiples on the board (at 851, 59, 61 and 67 are one class though
851/59, 851/61 and 851/67 have different floors: the frames see only some multiples).  At 1300 the classes are
{41, 43}, {59, 61}, {67, 71}, {73, 79}, {89, 97, 101, 103, 107}, {109, 113, 127, 131, 137, 139} and {149, 151, 157}.

| n | games | candidates | classes of primes | largest class | candidates moved | log10 of the group's order |
|---|---|---|---|---|---|---|
| 851-900 | 50 | 211 | 3.8 | 4.3 | 23.9 (11%) | 4.5 |
| 951-1000 | 50 | 236 | 4.1 | 6.1 | 28.5 (12%) | 5.1 |
| 1051-1100 | 50 | 266 | 4.6 | 6.3 | 28.3 (11%) | 5.2 |
| 1151-1200 | 50 | 289 | 4.6 | 6.7 | 28.9 (10%) | 5.9 |
| 1251-1300 | 50 | 322 | 5.8 | 6.7 | 41.5 (13%) | 6.7 |
| 1301-1350 | 50 | 336 | 5.9 | 5.9 | 48.4 (14%) | 7.4 |

*The rule.*  A set worth beating the incumbent with `c_p > c_q` for a symmetric pair has an image worth more, so
only sets with `c_p ≤ c_q` for every pair need searching, and a heaviest set always keeps them all (otherwise its
image would be heavier): a node can be cut when, for some pair, no completion that keeps the constraint can reach
what the node needs.  The constraints of a class are the order of its primes by their cofactor sums, which is all
that value can say about the class's whole symmetric group.  Three tests of that, from cheap to dear: *hopeless*,
no completion can lift `c_q` to `c_p` (`c_p(P) > c_q(P) + c_q(C)`); the *matching bound of the constraint*, each
subset `U` of the pair's multiples among the candidates (at most 10 or 12) promoted, the others left out, the rest
relaxed (solver pilot and `dominance nodes`); the *Lagrangian of the constraint* on the GF(2) bound, each multiple
`m·p` weighing `m(p − μ)` and `m·q` weighing `m(q + μ)`, plus `μ(c_q(P) − c_p(P))`, for `μ = (q − p)/2` and `q − p`
(a bound on the completions that keep the constraint for any `μ ≥ 0`; GF(2) pilot).  The two dearer tests run only
for pairs whose constraint the bound's heaviest set breaks.

*Checks* (`dominance symcheck --known results/strict.jsonl 2-1300`, `bench/dominance/symcheck.jsonl`): in all 1,170
games with pairs, the images of the optimal set and of 50 random maximal promotable sets are promotable and worth
what the formula says; on the 22 boards with pairs and at most 22 candidates every promotable set (97,839 in all) is
checked and the best that keeps the constraints is `P*`; the brute-force oracle scores the known optimum to n = 40.
Both pilots with the incumbent at `P*` − 1, so that they must find an optimal set themselves, find one in every
game of 2-900 (880 games each; the rule cut in 573 and 353 of them).

**3. What the rule and the covers remove: a sixth to a third of the nodes, a constant share.**  At the sampled
nodes (`dominance_output.md` has the per-game tables): "proved" is any of the covers, the prime exchanges and the
rule's matching test; "sampled" its upper estimate.

| tree | n | proved | sampled |
|---|---|---|---|
| solver proofs | 900, 936, 950, 1000 | 0%, 25%, 32%, 43% | 0%, 46%, 53%, 50% |
| | 1100, 1184, 1215, 1250 | 36%, 8%, 10%, 25% | 39%, 20%, 12%, 35% |
| incumbent `P*` + 100 | 1000, 1250 | 28%, 8% | 52%, 15% |
| incumbent `P*` + 200 | 1000, 1250 | 0%, 12% | 50%, 27% |
| GF(2) branch and bound | 930, 990, 1020 | 54%, 25%, 16% | 55%, 25%, 21% |
| | 1210, 1230, 1300 | 7%, 19%, 35% | 10%, 20%, 39% |

Two-member exchanges add little (sampled, with everything else: 49%, 34%, 48% of the nodes of 936, 1184, 1250; 25%,
7%, 46% of the GF(2) nodes of 1020, 1210, 1300).  In the GF(2) trees what is proved is nearly all that is sampled,
and it is the rule; in the solver's trees the rest of the estimate is single swaps along a symmetry (426 → 438
above, half of an exchange of 71 and 73) and exchanges of primes that are alike only at the node (29 and 31, 17 and
19, 61 and 67 at nodes of 936, where the multiples that tell them apart are decided).

The pilots, nodes with the rule against without, the incumbent at `P*`:

| search, rule | games | nodes, geometric mean ratio | total | fewest | most | log ratio on log nodes | time |
|---|---|---|---|---|---|---|---|
| solver, hopeless | 880-1090 by 10 (22) | 0.962 | 0.960 | 0.85 (950) | 1.07 (1040) | −0.002 | +15% |
| solver, matching bound of the constraint | the same | 0.878 | 0.829 | 0.67 (1000) | 1.16 (940) | −0.028 | ×1.9 |
| GF(2) B&B, hopeless | 880-1300 by 10 (43) | 0.948 | 0.925 | 0.71 (940) | 1.00 | −0.016 | −3% |
| GF(2) B&B, Lagrangian of the constraint | the same | 0.859 | 0.826 | 0.57 (930) | 0.98 (1100) | −0.030 | −3% |

In the solver the rule's cuts change what the searcher learns, so a tree can grow: 1184 52,259 to 66,250 (+27%) with
the matching test, 940 +16%; 1250 709,893 to 678,415 (−4%).  The margin scans of the GF(2) pilot (δ = 0, 25, 50, 100)
keep the ratio per game: 930 0.57, 0.60, 0.61, 0.92; 990 0.77, 0.77, 0.77, 0.79; 1300 0.69, 0.68, 0.67, 0.75.  The
share does not grow with n (the log ratio changes by ±0.02 decades per 100 in n in both pilots) and barely with the tree
(−0.03 decades per decade of nodes: `c` lowered by about 3%), although the symmetric part of the problem does grow
(the table of 2): most rearrangements within a class differ in value by more than the bound leaves, and the bound
already cuts them.

*Conclusion.*  Literal dominance and transpositions are absent from the trees; what makes interchangeable
alternatives is a real symmetry of the frame model, the exchange of primes within classes that the frame builder
treats alike, which IDEAS.md's "no symmetries found" missed.  Breaking it is sound (proved, and checked to 1300) and
removes about a seventh of the nodes on average (geometric mean 0.86 in the GF(2) branch and bound, 0.88 in the
solver), up to 43% in some games: a constant factor, not a lower cost per unit of gap.  Exchanges that hold only
at a node add a little more in the solver's trees and almost nothing in the GF(2) trees, and cost a state comparison
per exchange, far more than a step.

*What it implies.*

* *Not a slope lever*, and not worth a phase of its own.  If the GF(2) bound goes into the search, the symmetry rule
  is a cheap addition there: the hopeless test from cofactor sums kept along the path (nearly free, about 5% of
  the nodes) and the constraint's Lagrangian at the nodes whose heaviest set breaks a pair (about 14%, at one or two
  intersections there; time-neutral in the pilot, where every bound is computed from scratch).  In the matching
  solver the constraint costs more than it saves.
* The pairs are found at the root in about 20 ms a game (the setup included); `root_pairs` is the definition to reuse.
* The equal-sum exchanges of the uniqueness study are not symmetries (e.g. 425's {110, 132, 168} ↔ {98, 150, 162});
  they are coincidences of value that only the bound can rule out.

## Later (2026-10): the program removed from master

The examples `dominance` and `matroid`, with the files of `examples/common/` they included, were removed from master on
2026-10-17.  They build at commit 8753189: `git checkout 8753189`, then
`cargo build --release --example dominance --example matroid` in `rust/`.
