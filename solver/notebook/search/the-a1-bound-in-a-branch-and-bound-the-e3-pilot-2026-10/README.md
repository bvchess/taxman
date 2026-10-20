# The A1 bound in a branch and bound: the E3 pilot (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-15.  Concluded 2026-10-15.

**Question.**  The pilot of experiment E3 of avenue A1, after
[E1](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md) passed strongly: in the matroid study's plain
branch and bound (the GF(2) bound and its conflict branching), with the A1 bound at the nodes the GF(2) bound leaves
open and its multipliers carried from parent to child, how much smaller are the trees, at what cost a node, and how
many pricings does a node need from its parent's state?  The kill criterion, fixed in advance: stop E3 if at 1472 the
best variant's tree is less than 3 times smaller than v0's **and** the audit shows the best method reaching fewer than
half of the reference's cuts within 20 pricings.  **Answer.**  No kill, and no change of slope.  The best variant, r2,
has trees 4.95 (1215) and 5.04 (1472) times smaller than the GF(2) bound's, a constant factor, at 23 times v0's cost a
node at 1472 (4.61 times its time); within 20 pricings it reaches only 0.41 of the reference's cuts.

## Method

The variants:

* `v0`: the GF(2) bound alone.
* `v1` (*Tier 1*): one evaluation of the A1 bound L at the root fit's multipliers, at every node the GF(2) bound leaves
  open.
* `r1`, `r2`, `r2c`: Tier 1 at the state the parent hands down, then, at every node Tier 1 leaves open, a *refit* of at
  most K = 20 pricings; the node's final state goes to its children.  R1 is a deflected Polyak step, R2 the warm master
  (Wentges-stabilized pricing from the inherited multipliers, cheap separation only), R2c the warm master with
  node-local signings.

The games are 1215 (root GF(2) gap 213) and 1472 (gap 581), with root fits made in certified mode for 300 s (1215
converged at 35,168 in 2.6 s; 1472 at 52,871, unconverged), because search mode is much weaker at the roots (at 600,
840, 900, 1000 and 1100 it closed 0.38, 0.34, 0.65, 0.31 and 0.25 of the gap against certified's 0.60, 0.57, 0.97, 0.48
and 0.54).  An *audit* at a sample of the Tier-1-open nodes (every 25th at 1215, every 500th at 1472, by a hash of the
node) compares, from the same inherited state, the *reference* (100 Polyak steps and the certified master with
separation, 10 s) with each method's bound after 1 to 20 pricings.

The program is research code with no solver change: `examples/common/bnb.rs` (the plain branch and bound with an
extra-bound trait, node records and paired dives of Knuth's estimator), `common/dw.rs` (the column generation as phases,
with warm states for a child) and `fields bnb`.  Every evaluation of L is a bound at any multipliers and any signings
([THEORY.md](../../../../docs/theory/bounds.md#signed-frame-matroids-the-frame-bound-over-other-fields), Theorems 14
and 15), so the tests check the machinery: the inherited states, every variant's bound along the optimum's path at 300,
350, ..., 1300, a set worth P\* found at each of the 472 non-prime games of 30-600 with P\* > 0, a resumed run giving
the tree of a run straight through, and Knuth's dives against the estimator's exact distribution at 1000, 1100, 1215 and
1300 (Kolmogorov–Smirnov distances 0.012-0.027, below the 1% value 0.036).

One defect turned up (bb47db9).  The first attempt at r2's tree at 1472 stopped after 509,112 nodes on the crate's guard
"a negative cycle in the exchange graph of an extreme set".  An R2 master whose simplex returned *optimal* at a basis
that is not primal feasible (duals near 10^56) proposed multipliers beyond 10^50, and the integer sums in
`evaluate_grid` wrapped around, giving weights near 2^63; wrapping could also have made L wrong without a panic.  The
sums now saturate (a saturated L is `i64::MAX`, still a bound), bit for bit the same wherever nothing overflowed.  Every
figure below is of the fixed build (33fb719).  A *breakdown*, a master solve that ends optimal at an infeasible basis,
wastes only its refit's pricings: 3 in 1,270 refits at 1215, 1,233 in 190,266 at 1472 (0.65%).

## Results

The trees are exact, on one thread of the M1 Max under `nice`.  *Cost* is the ms a node over v0's.

| game | variant | nodes | v0 / nodes | refits | pricings a refit | ms a node | cost | time against v0 |
|---|---|---|---|---|---|---|---|---|
| 1215 | v0 | 24,516 | 1 | | | 0.67 | 1 | 1 |
| 1215 | v1 | 18,774 | 1.31 | | | 1.13 | 1.69 | 1.30 |
| 1215 | r1 | 12,307 | 1.99 | 2,554 | 17.7 | 2.93 | 4.38 | 2.20 |
| 1215 | r2 | 4,955 | 4.95 | 1,270 | 14.7 | 4.27 | 6.38 | 1.29 |
| 1215 | r2c | 3,818 | 6.42 | 1,069 | 18.4 | 6.80 | 10.16 | 1.58 |
| 1472 | v0 | 3,401,305 | 1 | | | 0.94 | 1 | 1 |
| 1472 | r2 | 675,502 | 5.04 | 190,266 | 16.9 | 21.75 | 23.2 | 4.61 |

The r2 tree at 1472 took 4.1 hours of one thread, v0's 0.9.  v1, r1 and r2c were not run as trees there, within the
pilot's thread-hours; their paired Knuth estimates say they cannot beat r2's tree by a factor that matters.  At 1472 a
refit costs 67 ms: the simplex about 8, the intersections about 6 and the greedies about 2, and the remaining 50 ms goes
to building the master from the inherited columns and up to two thousand inherited inequalities, and to the cheap
separation.  The GF(2) bound costs 0.94 ms a node there, a Tier-1 evaluation 2.8 ms.

**The audit** (the reference against each method within 20 pricings; *reach* is the share of the reference's cuts the
method also makes):

| game | host | audited | reference cuts | its pricings, ms | r1 reach | r2 reach | r2c reach |
|---|---|---|---|---|---|---|---|
| 1215 | v1 | 127 | 127 (100%) | 106, 89 | 0.09 | 0.25 | 0.26 |
| 1215 | r1 | 94 | 94 (100%) | 101, 98 | 0.18 | 0.59 | 0.59 |
| 1215 | r2 | 40 | 40 (100%) | 144, 229 | 0.03 | 0.28 | 0.30 |
| 1215 | r2c | 36 | 36 (100%) | 129, 269 | 0.14 | 0.44 | 0.44 |
| 1472 | r2 | 387 | 366 (94.6%) | 177, 952 | 0.21 | 0.41 | 0.41 |

At 1472 the reference cuts 100% of the audited nodes of GF(2) excess at most 50, 98% at 51-100, 92% at 101-200 and 65%
above 200; r2 within 20 pricings cuts 76%, 67%, 39%, 16% and 15%.  The nodes left open in r2's tree are the
high-excess ones near the top: of the refits at GF(2) excess above 200 (10,083 nodes), 1.2% cut, and their subtrees
hold the whole tree.

**The criterion.**  At 1472 r2's tree is 5.04 times smaller than v0's, so no kill, although the best method reaches
only 0.41 of the reference's cuts (the design: "a weak audit with a strong ratio points at the method").  The ratio is
the same at both gaps, 4.95 at 213 and 5.04 at 581.  The plain tree grows a decade per 172 of gap (24,516 to
3,401,305), and r2's by the same law (4,955 to 675,502): refits of 20 pricings make the A1 bound a constant factor of 5
at 20 to 40 v0 nodes a node, not a change of slope.  By the design's projection that is a loss of about 110 to 160 of
affordable gap, below its kill line of +200.  From the same inherited states the reference cuts 95-100% of the nodes r2
leaves open, in 100-180 pricings (0.1-1 s).

**Larger budgets and policies at 1215** (the same root fit; `--refit K`, `--policy`), as v0 / nodes and time against
v0:

| variant | K | policy | v0 / nodes | time against v0 |
|---|---|---|---|---|
| r1 | 20, 50, 100, 200 | all | 1.99, 2.27, 2.40, 2.48 | 2.20, 3.56, 4.90, 5.47 |
| r2 | 20, 50, 100, 200 | all | 4.95, 6.62, 6.89, 6.99 | 1.29, 1.69, 2.01, 1.99 |
| r2 | 20 | excess:50 | 2.56 | 1.27 |
| r2 | 20 | tier1:0.5 | 1.63 | 1.36 |

R2 stops on its own after about 27 pricings whatever K (nothing prices out and no cheap inequality is violated), and R1
creeps.  What the reference has and R2 lacks is the exact checks and the separation (R2c's local signings gain a quarter
of the nodes at 1215).  [E2](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md#e2-past-2175-2026-10)
found the same at the roots: in search mode the cheap inequalities describe a relaxation of the LP, and the fit stops
above it (0.27 of the gap at E1's roots), where a certified fit of 60 s gets within 5% of it everywhere.  The policies
save refits but lose more tree than time.

**Knuth's estimator** (paired dives, 2,000, the same for every variant).  At 1472 the GF(2) tree's estimate is 741,585
with a 90% interval of [389,979; 1,166,791], against an exact 3,401,305; r2's is 341,789 [137,227; 637,208] against
675,502.  The estimator follows its exact distribution, but that distribution is so heavy-tailed that 2,000 dives miss
the mean (at 1472 two dives make 43% of the GF(2) sum).  The ratios are better but loose (r2's 2.17 [1.48; 4.12] against
5.04), and paired dives compare variants dive for dive (r2's below r1's on 685, above on 12, and below v1's on 935,
never above; r2c's below r2's on 345, above on 11).  For games past what an exact tree allows, plain Knuth estimates are
not good enough.

## Decision

The instrument stays and no solver code changed.  The A1 bound in a branch and bound gives a constant factor of 5 at 23
times the cost a node, so it does not pass E3's criterion as measured.  The variants to carry are r2 and r2c (the
smallest trees; r2c's dives at 1472 say smaller than r2's) with the policy `all` and K = 20, since K = 50 gains 1.3 in
nodes at 1.3 in time at 1215.  What would reopen it, as the pilot points:

* a reference-strength refit (the exact checks and the core separation at Tier-1-open nodes, within a budget), which the
  audit says cuts almost every node r2 leaves open; [R3 and the slope
  test](../r3-and-the-slope-test-u4-of-a1-2026-10/README.md) took it up;
* the master's breakdowns and its rebuild from every inherited inequality (about 50 of R2's 67 ms a refit at 1472;
  keep the inequalities binding at the parent);
* an estimator validated against the exact trees of 1472, for the larger games.

E3 main as designed (v0, v1 and two A1 variants, exact trees on the seven games of 1215-1521, Knuth beyond) would take
about 30 thread-hours.  The pilot deviated from the design, soundly, in three ways: r2's tree at 1472 had a budget of 10
hours (the design's 2 would have stopped it at 0.3 million nodes); the audit samples every 500th node at 1472 (every
25th would have taken over a day); and the core table of S3.4 (cores cached across nodes), which saves time only, was
not built.

## Files

The data were made with the commands and the build of [the fields study's
files](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md#files-and-the-commands-that-made-them).  The
root fits of 1215 and 1472 are in [`solver/frame/testdata/fits/`](../../../frame/testdata/fits/README.md).

| file | command | what it is |
|---|---|---|
| `bnb.jsonl`, `audit.jsonl`, `logs/` | 1215: `fields bnb K --variants v0,v1,r1,r2,r2c --refit 20 --policy all --fit solver/frame/testdata/fits --fit-seconds 300 --fit-mode certified --audit 25 --audit-out audit.jsonl --log logs --checkpoint DIR --out bnb.jsonl 1215`; 1472: the same with `--variants r2 --audit 500 --seconds 36000`, then `--variants v0` with no audit and no log; the paired dives `--variants r2,r2c,v1,r1 --knuth 2000 --seed 1 1472`; the exploration `--variants r1,r2 --refit 50` (100, 200) and `--variants r2 --policy excess:50` (`tier1:0.5`) at 1215; the code of 33fb719 (the root fits made by bc33638's, from the same code but for the saturating sums), the logs gzipped afterwards | the pilot's data: a line per run (tree or dives), a line per audited node, and per run the per-node log (`logs/n{N}-{variant}-{policy}-k{K}.jsonl.gz`; per dive run, the dives) |
| `e3.py`, `e3_output.md` | `./e3.py > e3_output.md` | the pilot's tables |

The examples `fields` and `matroid`, with the files of `examples/common/` they included, were removed from master on
2026-10-17.  They build at commit 8753189: `git checkout 8753189`, then `cargo build --release --example fields
--example matroid` in `rust/`.

## Commits

* `9e3ecae` (2026-10-15): the plain branch and bound moved from `matroid.rs` to `examples/common/bnb.rs`.
* `0afde37` (2026-10-15): `dw.rs`, the column generation as phases, with warm states.
* `bc33638` (2026-10-15): `fields bnb`.
* `bb47db9` (2026-10-15): the integer evaluation of L saturates instead of wrapping around.
* `33fb719` (2026-10-15): `fields bnb` counts breakdowns and saturated evaluations; the build of every figure.
* `8753189` (2026-10-17): builds the examples, which master no longer has.
