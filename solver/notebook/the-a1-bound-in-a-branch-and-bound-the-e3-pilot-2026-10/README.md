# The A1 bound in a branch and bound: the E3 pilot (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The pilot of experiment E3 of avenue A1 (the advisor's second consultation on A1, its sections S3-S5,
after [E1](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md) passed strongly): in the matroid study's plain
branch and bound (the GF(2) bound and its conflict branching), with the A1 bound at the nodes the GF(2) bound leaves
open and its multipliers carried from parent to child, how much smaller are the trees, at what cost a node, and how
many pricings does a node need from its parent's state?  *Variants*: `v0` (the GF(2) bound alone); `v1` (*Tier 1*: one
evaluation of L at the root fit's multipliers, at every node the GF(2) bound leaves open); `r1`, `r2`, `r2c` (Tier 1 at
the state the parent hands down, then, at every node Tier 1 leaves open, a *refit* of at most K = 20 pricings: R1
deflected Polyak, R2 the warm master, R2c the warm master with node-local signings; the node's final state goes to
its children).  Games 1215 (root GF(2) gap 213) and 1472 (gap 581).  An *audit* at a sample of the Tier-1-open nodes
(chosen by a hash of the node): from the same inherited state, the *reference* (E1n's node mode: 100 Polyak steps, the
certified master with separation, 10 s) and each method's bound after 1 to 20 pricings.  Kill criterion, fixed in
advance: stop E3 if at 1472 the best variant's tree is less than 3 times smaller than v0's **and** the audit shows the
best method reaching fewer than half of the reference's cuts within 20 pricings.

*What was built* (research code; no solver change).  `examples/common/bnb.rs`: the plain branch and bound, moved out
of `examples/matroid.rs` (9e3ecae) behind an extra-bound trait, with node records and paired dives of Knuth's
estimator; `matroid bnb` prints the same lines as before (936, 1100, 1215 with `gf2` and `gf2warm`, 600, 700, 936 with
`matching`).  `common/dw.rs` (0afde37): the column generation as phases that `bound` and the warm master share
(`bound`'s results unchanged at 45 regression roots of 300-1100, every arm), `FitMode::Search` (no exact checks), the
warm states (a child takes its parent's master columns that hold the members just promoted and avoid the one left
out, those taken out; its rank inequalities restricted to the child's elements, ranks taken again in the child's
matroids; its multipliers restricted likewise), root fits as files (E2's format since the merge with it:
`common/fit.rs`), Tier 1, R1 (direction
`g + β d_prev`, Camerini, Fratta and Maffioli, steps toward the target) and R2 (the master warm-started from the
inherited columns and inequalities, Wentges-stabilized pricing from the inherited multipliers, cheap separation only).
`fields bnb` (bc33638, 33fb719): the variants, refit policies (`all`, `excess:E`, `tier1:F`), the audit, per-node logs
with subtree sizes, checkpoints that resume to the same tree, paired Knuth dives; `solver/notebook/the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/e3.py`, the tables.

*Validity.*  Nothing new to prove: every evaluation of L is a bound at any multipliers and any signings
([THEORY.md](../../../docs/theory/bounds.md#signed-frame-matroids-the-frame-bound-over-other-fields), Theorems 14 and 15,
at a node by Theorem 14 (2)), the bound is ⌊L⌋ evaluated in integers, and the master only proposes multipliers.  The
tests (`cargo t --release --example fields`, 48 pass, one ignored and run once): a child's inherited columns common
independent in its matroids and within `max_add`, and its inequalities, ranks taken again, holding for 20 random
promotable completions, along 200 random paths of 300-700; every variant's bound at least `w(S* − promoted)` at every
node of the optimum's path at 300, 350, ..., 1300; with the incumbent `P* − 1`, every variant finds a set worth `P*` at
each of the 472 non-prime games of 30-600 with `P* > 0`; r2's and r2c's trees subtrees of v0's at 20 games of
300-900; a run resumed from a checkpoint gives the tree and log of a run straight through; the fit's round trip; R1's
and R2's traces; and (ignored, 148 s) Knuth's dives against the estimator's exact distribution, computed from the exact
trees of 1000, 1100, 1215 and 1300 (Kolmogorov–Smirnov distances 0.012-0.027, below the 1% value 0.036).  Every run
asserts that no completion beats `P*` and that a node the extra bound cuts has no promotable GF(2) heaviest set.

*A defect found on the way: L could wrap around* (bb47db9).  The first attempt at r2's tree at 1472 stopped after
509,112 nodes on the crate's guard "a negative cycle in the exchange graph of an extreme set".  The cause was the
study's: an R2 master whose simplex returned *optimal* at a basis that is not primal feasible (1,922 rows, 1,921 of
them inherited rank inequalities, duals near 10^56) proposed multipliers beyond 10^50; the grid took them as `i64::MAX`,
and `c[e] −= λ` and `value += g` in `evaluate_grid` wrapped around, giving the intersection weights near 2^63 (and some
just above `x·S`), on which the crate's distances overflowed.  Wrapping could also have made L wrong without a panic.
The sums now saturate: bit for bit the same wherever nothing overflowed, and L is valid at any multipliers (the
positive reweighted weights are exact and below 2^29; a saturated L is `i64::MAX`, still a bound).  Not ties, and not
a defect of the crate: the search's own weights, the candidates' numbers, are far inside the crate's documented limit.
Every figure below is the fixed build's (33fb719, which also counts the master's *breakdowns*, solves that end
optimal at a basis that is not feasible): at 1215 the trees and per-node logs are line for line the first attempt's,
at 1472 r2's tree was made again from the same root fit and its first 509,112 nodes are line for line the first
attempt's, and the paired dives gave the same estimates dive for dive.  A breakdown wastes the pricings of its refit
(every proposal saturates) and nothing else: 3 in 1,270 refits at 1215, 1,233 in 190,266 at 1472 (0.65%), where 2
evaluations saturated.

*The trees* (exact, one thread of the M1 Max under `nice`, other jobs on the machine; ms a node without the audit's
time; *c* the design's cost model `2.5 + 3p`, `p` the refit pricings per Tier-1 evaluation, in v0 nodes; *measured*,
the ms a node over v0's):

| game | variant | nodes | v0 / nodes | Tier-1 evaluations | refits | pricings a refit | p | c | ms a node | measured | time against v0 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1215 | v0 | 24,516 | 1 | | | | | 1 | 0.67 | 1 | 1 |
| 1215 | v1 | 18,774 | 1.31 | 4,059 | | | 0 | 2.5 | 1.13 | 1.69 | 1.30 |
| 1215 | r1 | 12,307 | 1.99 | 3,289 | 2,554 | 17.7 | 13.8 | 43.8 | 2.93 | 4.38 | 2.20 |
| 1215 | r2 | 4,955 | 4.95 | 1,856 | 1,270 | 14.7 | 10.0 | 32.6 | 4.27 | 6.38 | 1.29 |
| 1215 | r2c | 3,818 | 6.42 | 1,540 | 1,069 | 18.4 | 12.8 | 40.8 | 6.80 | 10.16 | 1.58 |
| 1472 | v0 | 3,401,305 | 1 | | | | | 1 | 0.94 | 1 | 1 |
| 1472 | r2 | 675,502 | 5.04 | 245,314 | 190,266 | 16.9 | 13.1 | 41.8 | 21.75 | 23.2 | 4.61 |

The r2 tree at 1472 took 4.1 hours of one thread, v0's 0.9; v1, r1 and r2c were not run as trees there, within the
pilot's thread-hours: their paired Knuth estimates are below, and they cannot beat r2's tree by a factor that matters
for the criterion (v1's dives estimate 1.01 times fewer nodes than v0's, r1's 1.23).  At 1472 a refit costs 67 ms
(16.9 pricings): the simplex about 8 ms, the intersections about 6 and the greedies about 2; the rest, about 50 ms, is
building the master from the inherited columns and up to two thousand inherited inequalities, and the cheap
separation.  The GF(2) bound costs 0.94 ms a node there, a Tier-1 evaluation 2.8 ms (2.2 of it building the signed
matroids).

*The audit* (the reference from the inherited state against each method within 20 pricings; reach: the share of the
reference's cuts the method also makes):

| game | host | audited | reference cuts | its pricings, ms | r1 reach | r2 reach | r2c reach |
|---|---|---|---|---|---|---|---|
| 1215 | v1 | 127 | 127 (100%) | 106, 89 | 0.09 | 0.25 | 0.26 |
| 1215 | r1 | 94 | 94 (100%) | 101, 98 | 0.18 | 0.59 | 0.59 |
| 1215 | r2 | 40 | 40 (100%) | 144, 229 | 0.03 | 0.28 | 0.30 |
| 1215 | r2c | 36 | 36 (100%) | 129, 269 | 0.14 | 0.44 | 0.44 |
| 1472 | r2 | 387 | 366 (94.6%) | 177, 952 | 0.21 | 0.41 | 0.41 |

(R2c separates only once R2 has stopped, so within 20 pricings it differs from R2 only where R2 stopped early.)  At
1472 the reference cuts 100% of the audited nodes of GF(2) excess at most 50, 98% at 51-100, 92% at 101-200 and 65%
above 200; r2 within 20 pricings 76%, 67%, 39%, 16% and 15%.  The nodes left open in r2's tree are the high-excess
ones near the top: of the refits at GF(2) excess above 200 (10,083 nodes), 1.2% cut, their subtrees holding the whole
tree.  The audit's sample is every 25th Tier-1-open node at 1215 and every 500th at 1472 (by the hash), so that the
reference's 10 s did not swamp the run.

*The pilot's criterion.*  At 1472 the best variant measured, r2, has a tree 5.04 times smaller than v0's, so **no
kill**, although the best method reaches only 0.41 of the reference's cuts within 20 pricings (the design: "a weak
audit with a strong ratio points at the method").

*What the trees say beyond the criterion.*  The ratio is the same at both gaps: 4.95 at 213 and 5.04 at 581.  The plain
branch and bound's tree grows a decade per 172 of gap (24,516 to 3,401,305) and r2's by the same law (4,955 to
675,502, a decade per 172): with refits of 20 pricings the A1 bound is a constant factor of 5 at a cost of about 20 to
40 v0 nodes a node, not a change of slope.  With the design's projection for E3 (`G_max = (9.5 − a − log10 c)/b`, b
unchanged) that is a loss of about 110 to 160 of affordable gap, below its kill line of +200.  The audit says what is
missing: from the same inherited states the reference cuts 95-100% of the nodes r2 leaves open, in 100-180 pricings
(0.1-1 s); the high-excess nodes are where it matters.

*Exploratory runs at 1215* (beyond the design; the same root fit; `fields bnb ... --refit K` and `--policy`):

| variant | K | policy | nodes | v0 / nodes | pricings a refit | ms a node | time against v0 |
|---|---|---|---|---|---|---|---|
| r1 | 20, 50, 100, 200 | all | 12,307, 10,809, 10,225, 9,886 | 1.99, 2.27, 2.40, 2.48 | 17.7, 40.0, 64.0, 77.3 | 2.93, 5.40, 7.87, 9.08 | 2.20, 3.56, 4.90, 5.47 |
| r2 | 20, 50, 100, 200 | all | 4,955, 3,703, 3,557, 3,508 | 4.95, 6.62, 6.89, 6.99 | 14.7, 24.7, 27.3, 27.8 | 4.27, 7.48, 9.28, 9.30 | 1.29, 1.69, 2.01, 1.99 |
| r2 | 20 | excess:50 | 9,582 | 2.56 | 17.2 | 2.18 | 1.27 |
| r2 | 20 | tier1:0.5 | 15,049 | 1.63 | 12.9 | 1.48 | 1.36 |

A larger budget hardly helps: R2 stops on its own (nothing prices out and no cheap inequality is violated) after about
27 pricings whatever K, R1 creeps.  What the reference has and R2 lacks is the exact checks and the separation (R2c's
local signings gain a quarter of the nodes at 1215).  [E2](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md#e2-past-2175-2026-10) found the same at the roots: in
search mode (R2's) the cheap inequalities describe a relaxation of the LP, and the fit stops above it (0.27 of the
gap at E1's roots), where a certified fit of 60 s gets within 5% of it everywhere.  The policies save refits but lose
more tree than time.

*Knuth's estimator.*  Paired dives (2,000, the same dives for every variant) at 1472: the GF(2) tree's estimate is
741,585 with a 90% interval of [389,979; 1,166,791], against an exact 3,401,305; r2's 341,789 [137,227; 637,208] against
675,502.  The estimator follows its exact distribution (the ignored test), but that distribution is so heavy-tailed
that 2,000 dives miss the mean: one dive's standard deviation is 1.4 billion at 1215 (24,516 nodes), where 2,000
dives' 99% interval covers the count in 52-78% of samples; at 1472 two dives make 43% of the GF(2) sum.  The ratios
were better (r2's 2.17 [1.48; 4.12] against 5.04; at 1000-1300 the four exact ratios inside), and the paired dives
compare variants dive for dive (r2c's estimate below r2's on 345 dives, above on 11; r2's below r1's on 685, above on
12, and below v1's on 935, never above).  For E3's games past what an exact tree allows, plain Knuth estimates are not
good enough.

*For E3 main* (the advisor's decision): the variants to carry are r2 and r2c (the smallest trees; r2c's dives at 1472
say smaller than r2's), with the policy `all`; K = 20 is enough for R2 as it stands (K = 50 gains 1.3 in nodes at 1.3 in
time at 1215).  Measured as they are, they will not pass E3's criterion (a constant factor of 5).  What the pilot points
at, before E3 main: a reference-strength refit (the exact checks and the core separation at Tier-1-open nodes, within
a budget), which the audit says cuts almost every node r2 leaves open; the master's breakdowns and its rebuild from
every inherited inequality (about 50 of R2's 67 ms a refit at 1472; keep the inequalities binding at the parent); and
an estimator validated against the exact trees of 1472 (v0's and r2's, with r2's per-node log) for the larger games.
E3 main as designed (v0, v1 and two A1 variants, exact trees on the seven games of 1215-1521, Knuth beyond): at 1472
the four take about 0.9 (v0), 1.5 (v1: v0's time and 2.8 ms of Tier 1 at each of its 720,000 open nodes), 4.1 (r2) and
5 (r2c, from 1215's cost a node and tree) hours of one thread; with the trees' decade per 172 of gap the seven games
(gaps 204-590) come to about 30 thread-hours, a `c8g.4xlarge` (16 threads, $0.64 an hour) for about 7 hours (its
longest run, r2c at 1475), about $4-5; the 2170s (gap 767) and the games past 2175 only with a validated estimator
(their exact r2 trees would take days).

*Choices and deviations* (all sound; none changes a bound's validity):

1. The root fits are the main arm's in certified mode, 300 s (1215 converged at 35,168 in 2.6 s; 1472 at 52,871 after
   300 s, unconverged), not search mode, which is much weaker at the roots (here at 600, 840, 900, 1000, 1100 it
   closed 0.38, 0.34, 0.65, 0.31, 0.25 of the gap against certified's 0.60, 0.57, 0.97, 0.48, 0.54; E2 measured it at
   54 roots and did not keep it).  The root node's bound is Tier 1 at the fit (recomputed), not the fit file's.  The
   two fit files were written in this branch's first format, holding the final family's lowest L, and converted at
   the merge with E2 to `common/fit.rs`'s, every value kept (the 1215 runs come out the same from them:
   [solver/notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md](../../frame/testdata/fits/README.md)); a fit `bnb` computes now is E2's, the lowest L evaluated.
2. The audit samples by a hash of the node (every 25th at 1215, every 500th at 1472, not the design's every 25th at
   1472, whose references would have taken over a day); it runs in the host variant's tree from that tree's states.
3. Only v0's and r2's trees were run at 1472; v1, r1 and r2c only as paired dives (above).  r2's run had a budget of
   10 hours (the design's 2 would have stopped it at 0.3 million nodes).  The first attempt ran as checkpointed pieces
   around a pause of the machine (its times leave the pause out); the run reported, straight through.
4. A child's inherited columns drop the members that are not the child's elements (as well as the design's taken-out
   promotions); inherited inequalities whose rank equals their size are not added (redundant).  Node-local signings
   are seeded past the root's, at most 2 a frame a node and 8 a frame a path.
5. Knuth's test (S5.2's test 5) checks the dives against the exact distribution, not the counts against the 99%
   intervals, which the counts at 1000 and 1215 miss although the dives are right (above).
6. Two Knuth processes ran together for 20 minutes by mistake and wrote the same dives twice (one line interleaved);
   the dive logs were deduplicated (every duplicate equal, the dives deterministic) and every dive equals the first
   attempt's.
7. The core table of S3.4 (cores cached across nodes) was not built: it saves time only.

## Later (2026-10): the program removed from master

The examples `fields` and `matroid`, with the files of `examples/common/` they included, were removed from master on
2026-10-17.  They build at commit 8753189: `git checkout 8753189`, then
`cargo build --release --example fields --example matroid` in `rust/`.

## Files

The data of the pilot, made with the commands and the build described in [the fields study's files](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md#files-and-the-commands-that-made-them):

| file | command | what it is |
|---|---|---|
| `bnb.jsonl`, `audit.jsonl`, `fits/`, `logs/` | 1215: `fields bnb K --variants v0,v1,r1,r2,r2c --refit 20 --policy all --fit solver/frame/testdata/fits --fit-seconds 300 --fit-mode certified --audit 25 --audit-out audit.jsonl --log logs --checkpoint DIR --out bnb.jsonl 1215`; 1472: the same with `--variants r2 --audit 500 --seconds 36000`, then `--variants v0` with no audit and no log; the paired dives `--variants r2,r2c,v1,r1 --knuth 2000 --seed 1 1472`; the exploration `--variants r1,r2 --refit 50` (100, 200) and `--variants r2 --policy excess:50` (`tier1:0.5`) at 1215; the code of 33fb719 (the root fits `fits/n1215.json` and `fits/n1472.json` made by bc33638's, from the same code but for the saturating sums, in that branch's first format and converted at the merge with E2: below), the logs gzipped afterwards | E3's pilot: a line per run (tree or dives), a line per audited node, the root fits, and per run the per-node log (and per dive run the dives) |
| `e3_output.md` | `./e3.py > e3_output.md` | the pilot's tables |

The root fits of 1215 and 1472 are in [`solver/frame/testdata/fits/`](../../frame/testdata/fits/README.md).
