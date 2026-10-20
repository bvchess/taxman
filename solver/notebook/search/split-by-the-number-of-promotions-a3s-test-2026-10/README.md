# Split by the number of promotions: A3's test (2026-10)

Part of the [lab notebook](../../README.md).

*Question.*  [Agenda A3](../../../docs/AGENDA.md#a3-split-by-the-number-of-promotions-a-sound-tightening-constant-factor-maybe-more):
every promotion set has one size, so a node can be closed if, for every total size `k` that can still beat the
optimum's promotion sum `P*`, the node's promoted weight plus the heaviest common independent set of the node's two
GF(2) frame matroids of size `k` less the promotions is at most `P*` (sound by partition, no conjecture).  The weights
`best(k)` of the heaviest set of each size are concave in `k`, and a size can win only if the *root's* `best(k)` exceeds
`P*`: the *window*, here one to two consecutive sizes.  Inside the window the node's bound for its heaviest completion's
size is the plain bound, so only a node whose heaviest completion has a total size *outside* the window can be closed.
Step 1 (a diagnostic, the kill fixed in advance: under 10% of the open nodes outside the window at every game) counts
them; Step 2 (the exact-size bound of those, and its cost) was to run only if Step 1 passed.

*What was run* (research code, no solver change; f7661cb).  `examples/promotions.rs`: `best(k)` for every `k` from the
intersection run on the weights plus a uniform amount above their sum (every augmentation then gains, so the run passes
through the heaviest set of each size up to the largest common independent set; `best(k)` is its weight less the
amount times `k`), checked against the enumeration of all common independent sets of the roots and of nodes on the
boards of 20-60 (tests) and, at every fixture node, against the plain bound and its set's size.  The fixtures: the
probe's 300 nodes each of 936 and 1215, and the 212 of 2112 (the units that its proof, one thread, handed the searcher,
made with `perf make-nodes` into `solver/notebook/search/split-by-the-number-of-promotions-a3s-test-2026-10/n2112.nodes`; 2170, the larger game proposed, takes 588 s on one
thread, so 2112 was taken).  `P*` is
`strict.jsonl`'s score less `in_the_bag`, equal to each fixture's incumbent.  A node is *open* when its plain bound
(at most `max_add` candidates) exceeds `P*` less its promotions' sum.

| game | `P*` | root gap | `max_total` | `best(k) − P*` just below the window, in it | window | root's heaviest set, largest common independent set | nodes, open | open below, inside, above |
|---|---|---|---|---|---|---|---|---|
| 936 | 20,507 | 132 | 52 | k = 51: −134; 52: +132 | 52 | 52, 52 | 300, 137 | 0, 137, 0 |
| 1215 | 35,050 | 213 | 73 | 68: −48; 69: +213 | 69 | 69, 69 | 300, 133 | 0, 133, 0 |
| 2112 | 103,056 | 517 | 126 | 115: −304; 116: +201, 117: +517 | 116-117 | 117, 117 | 212, 212 | 0, 212, 0 |

*Result: killed at Step 1.*  No open node lies outside the window at any of the three games (0 of 482; the criterion was
under 10% at every game), and neither does any node that is closed (0 of all 812 with candidates): every node's heaviest
completion has, with its promotions, the root's total size, 52, 69 and 117 (at 2112 the window's upper size).  The
window is narrow because the size below it falls short of `P*` (by 134, 48 and 304), and the nodes never leave it: the
root's heaviest set is a *largest* common independent set at all three games, so no size above it exists (a node's
promotions and any completion are a common independent set of the root's matroids), and every node's promotions and
heaviest completion again total that largest size, so nothing lies below.  The exact-size bound of a node inside the
window is its plain bound, so A3 closes nothing here.  This agrees with C4 (the root's heaviest set has the optimum's
size) and says more: the sizes that A3 would split by are all one, because a heaviest set takes every size the two
matroids allow.

*Limits.*  The fixtures are the units a search hands out (the open ones hold up to 40, 44 and 78 promotions), not every
node of a tree; nodes inside a unit's subtree, after more promotions, were not sampled.  A heaviest completion that ties
in size with a larger set is counted by its smaller size, which cannot matter at a node whose total is already the root's
largest.  Games whose root has a smaller heaviest set than the optimum's (1320 and 1322, C4) were not run.  Step 2, and
so the cost of the per-size computation, was not run; nor a split finer than the total size (by the promotions out of
each frame).

*Validity.*  Tests (`cargo t --release --example promotions`): `best(k)` equals the enumeration's at the roots and at
nodes of every board of 20-60 that has candidates (both algorithms, whatever the limit); and, with every promotable set
enumerated and every incumbent from 0 to one above the optimum, no node closed by the exact-size bound has a completion
that beats the incumbent (22,266 pairs; on these boards, whose games promote at most 3 numbers, the rule never closes
a node that the plain bound leaves open, so the test checks that closing loses nothing, not that it gains).

*Cost.*  Seconds on one thread for the runs (`perf make-nodes` 2 s, the three games 5 s); nothing on EC2.

## Later (2026-10): the program removed from master

The example `promotions`, with the files of `examples/common/` it included, was removed from master on 2026-10-17.  It
builds at commit 8753189: `git checkout 8753189`, then `cargo build --release --example promotions` in `rust/`.

## Files

`promotions.jsonl` is the output of `examples/promotions.rs`, the diagnostic of [agenda A3](../../../docs/AGENDA.md): for
each game, the root's *window* of total sizes that can still beat the optimum's promotion sum, and, for every node of
the game's fixture, whether the total size of its heaviest completion lies below, inside or above it (one `node` line
a node, one `game` line a game).  The entry is [Split by the number of
promotions](README.md).

* 936 and 1215 read the probe's fixtures (`testdata/perf`): `promotions --known results/strict.jsonl 936 1215`.
* 2112 reads `n2112.nodes` here, made by `perf make-nodes 2112 --known results/strict.jsonl --every 500 --keep 300
  --fixtures solver/notebook/search/split-by-the-number-of-promotions-a3s-test-2026-10` (the proof's 15,288 nodes handed the searcher 212 distinct units; all are kept):
  `promotions --known results/strict.jsonl --fixtures solver/notebook/search/split-by-the-number-of-promotions-a3s-test-2026-10 2112`.

Both are deterministic and take seconds on one thread; the commands run from `rust/`.  The example `promotions` was
removed from master (2026-10-17); it builds at commit 8753189 (`git checkout 8753189`, then `cargo build --release
--example promotions` in `rust/`).
