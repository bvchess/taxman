# Decomposing across the frames: A5's cheapest experiment (2026-10)

Part of the [lab notebook](../../README.md).

**Question.**  [Agenda A5](../../../docs/AGENDA.md#a5-decompose-across-the-frames-andor-search): the frames form a path (a
candidate of home h touches frames h and h − 1), so fixing the candidates of one home separates the frames in front from
those behind, and an AND/OR search could turn the product of the two sides' trees into a sum.  At the nodes that head big
subtrees, does some home split the open candidates' conflicts into two sides that do not interact, and do the subtrees
resolve the sides independently?  The kill condition: the cores sit in adjacent frames, so no separator splits them.

**Answer.**  A5 dies, and for a sharper reason than the kill condition.  A home always splits the conflicts (every
conflict lies in two adjacent homes, so none crosses a separator, and all sit in frames 1-3), but the separator is where
the work is: it touches 8-10 of the heaviest set's cores and a quarter to a half of its home's candidates, the bound's
error sits at that home, and with the separator fixed as the best completion has it the two sides cost 0.01-0.5% of
the subtree at the roots (0.03-2% over all the nodes).  The subtree is 390 to 11,400 times the product of the sides'
trees at the roots, not that product.  The AND/OR search has to enumerate the separator too, and its OR part alone took
5.7 to 154 times the plain tree's steps.

## What was run

`wall decompose` (new, `solver/frame/examples/wall/decompose.rs`, with a test; built with `--profile test`, one thread,
`nice -n 10`).  Games 936, 1215, 2112 and 2170 (proofs seeded with the optimum, the default tactics).  Nodes: the root;
for 1215 and 2170 the open units of pauses of the proof (3 of 1,024 steps; 5 of 60,000); and for 936 and 1215 the probe's
300 fixture nodes.  412 nodes that today's bound leaves open were measured (330 more were cut by it and left out).  Per
node: the cores of the GF(2) heaviest set (up to six a frame) and the nogoods the node's subtree learned (its last 50,000
steps: 3,693 cores and 7,504 nogoods in all), placed against every separator home; the node's best completion (the
optimum's, for a node that holds it, else an optimizing solve from the greedy completion within 3,000 to 100,000
steps), from which the bound's error by home and the two sides under the best completion's separator ("oracle"); and
at the largest nodes the AND/OR search itself, the OR part deciding the home's candidates (largest first, or by the
conflict of the heaviest set) with the bound of the whole as its test, each surviving assignment's sides solved by the
engine.  About 20 CPU minutes in all (the four runs of the data: 14, 146, 63 and 310 s), none on EC2.  A first run of
2170 with a best-completion limit of 120,000 steps was stopped: off the optimal path that solve costs far more than the
proof, so at 2170 only 10 nodes have a best completion: eight small ones, and the root and one unit of 51,388 steps,
which hold the optimum.

## Results

**Structure.**  None of the 11,197 conflicts crosses a separator, as the model says (a conflict is one frame's residual:
members of homes f and f + 1); all but a few small ones are in frames 1-3.  The nodes of 100 steps or more, by their
subtrees' weight (the conflicts are the nogoods the subtree learned):

| game | nodes | steps | a home splits them | balanced (smaller side at least 20%) | cheapest balanced separator, candidates (share of its home) | width 16 or less | cores of the heaviest set touching it |
|---|---|---|---|---|---|---|---|
| 936 | 2 | 491 | 79% | 0% | - | 0% | 5 of 5 |
| 1215 | 28 | 15,047 | 100% | 96% | 27.5 (33%) | 3% | 8 of 9 |
| 2112 | 1 | 10,458 | 100% | 100% | 35 (43%) | 0% | 8 of 13 |
| 2170 | 51 | 623,948 | 100% | 100% | 22 (28%) | 1% | 8 of 14 |

A separator that costs nothing (no conflict touches the home, with conflicts on both sides) holds for at most 1% of the
weight at any game; one of 16 candidates or fewer for 3% (1215) and 1% (2170), none from a subtree of 1,000 steps at 1215
or 10,000 at 2170, and the width grows with the subtree (`analysis.txt`).  So the share of nodes, by the size of their
subtrees, with a separating home is about 100% and with a small one 1-3%.

**Where the gap is.**  The bound's error by home (the heaviest set's weight at the home less the best completion's)
is mostly at one interior home: home 2 at the root of 1215 (+221 on a gap of 213), home 3 at the root of 2170 (+544 of
767, 71%); the median node has 53-59% of its absolute error at one home (936 spreads it over homes 1, 2 and 3), the two
largest nodes of 1215 97% at home 2.  With a separator fixed as the best completion has it, the separator's choice
resolves a median 76% of the gap (1215; 71% at 2170, 91% at 936) and the sides apart cost 0.3% of the nodes' ideal proofs
at 1215 (801 of 237,567 steps), 0.03% at 2170 (101 of 400,040) and 1.8% at 936 (442 of 24,910):

| root | its subtree | gap to its optimum | oracle separator | sides' steps | subtree / product of the sides | AND/OR: OR evaluations (limit) | times the subtree |
|---|---|---|---|---|---|---|---|
| 936 | 390 | 132 | home 2 | 1, 1 | 390 | 60,000, not finished | 154 |
| 1215 | 3,704 | 213 | home 2 | 4, 2 | 463 | 60,000, not finished | 16 |
| 2112 | 10,458 | 517 | home 2 | 1, 1 | 10,458 | 60,000, not finished (home 3) | 5.7 |
| 2170 | 342,492 | 767 | home 2 | 1, 30 | 11,416 | 20,000, not finished | 0.06 |

**The AND/OR search.**  Besides the roots, five nodes of 1215 (638 to 2,997 steps) took 60,000 evaluations without
finishing (20 to 94 times their trees), and the one node that finished (936, a tree of 101 steps) took 12,027 and 12,210
evaluations for 257 assignments (119 and 121 times).  The sides of a sample of eight surviving assignments take 1 to 19
steps (mean 1-8): nearly everything is the OR part, which the sum of the sides' bounds prunes poorly.  At 2170 the limit
(20,000 evaluations, 0.06 of the root's tree, 154 to 236 survivors) decides nothing; the oracle rows and the widths are
what speak there.

## Decision

A5 is closed and not built.  The saving an AND/OR search can make is the sides' share of a tree, 2% or less at the
separator the best completion has; the rest is deciding the separator's candidates, which the plain search's strong
branching and nogoods already do.  Caveats: the nodes are those of proofs to 2170 (trees to 3·10⁵ steps), not of the
wall's 10⁹; the nogoods are a sample of a subtree's conflicts; and the OR part is the plain version (a cleverer one is
not excluded, but the oracle bounds what any can save).  It agrees with
[the anatomy of the gap](../the-anatomy-of-the-gap-2026-10/README.md): the error is in the promotions from frame 2 into
frame 1 and from 3 into 2, the homes 2 and 3 that are the separators.  The lever left is a relaxation that sees the odd
cores of frames 1 and 2 (the per-frame and A1 bounds, closed at the nodes), not a way to split them.

Commits: bb71f89 (`wall decompose`, its test) and the commit of this entry (no solver change).

## Files

* `decompose.jsonl.gz`: the lines of the four runs, one game after another (`game`, `node`; the fields are defined in
  the module comment of `solver/frame/examples/wall/decompose.rs`).  Made, from `solver/frame`, by
  `cargo build --profile test --example wall --features studies`, then `nice -n 10 target/debug/examples/wall decompose
  --known ../../results/strict.jsonl` with, for 936: `--fixtures testdata/perf --keep 300 --pauses 2 --span 1024
  --best-cap 100000 --and-or-cap 60000 --and-or-min 100`; 1215: `--fixtures testdata/perf --keep 300 --pauses 3 --span
  1024 --best-cap 100000 --and-or-cap 60000 --and-or-min 600`; 2112: `--best-cap 20000 --and-or-cap 60000 --and-or-min
  5000`; 2170: `--pauses 5 --span 60000 --unit-cap 400000 --best-cap 3000 --and-or-cap 20000 --and-or-min 40000`; the
  four outputs joined.
* `analyze.py`: the tables, `python3 -I analyze.py decompose.jsonl.gz`; `analysis.txt` is its output.
