# Split by the number of promotions: A3's test (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-16.  Concluded 2026-10-16.

**Question.**  Does [agenda
A3](../../../docs/AGENDA.md#a3-split-by-the-number-of-promotions-a-sound-tightening-constant-factor-maybe-more) close
nodes that the plain bound leaves open: is the total size of a node's heaviest completion ever outside the window of
sizes that can still beat P\*?  **Answer.**  No.  Of 482 open nodes at three games, 0 lie outside the window, so the
bound for each size equals the plain bound and A3 closes nothing.

## Method

Every promotion set has one size.  A node can be closed if, for every total size `k` that can still beat P\*, the node's
promoted weight plus the heaviest common independent set of size `k` in the node's two GF(2) frame matroids, less the
promotions, is at most P\*.  This is sound by partition and needs no conjecture.  The weights `best(k)` of the heaviest
set of each size are concave in `k`, and a size can win only if the root's `best(k)` exceeds P\*: those sizes are the
*window*, one or two consecutive sizes here.  Inside the window the bound for a node's heaviest completion is the plain
bound, so only a node whose heaviest completion has a total size outside the window can be closed.

Step 1 counts those nodes, with its kill fixed in advance: fewer than 10% of the open nodes outside the window at every
game.  Step 2, the exact-size bound of those nodes and its cost, was to run only if Step 1 passed.

* **The program.**  `examples/promotions.rs`, research code with no solver change.  It runs the intersection on the
  weights plus a uniform amount above their sum, so that every augmentation gains and the run passes through the
  heaviest set of each size; `best(k)` is that set's weight less the amount times `k`.  Tests check it against the
  enumeration of all common independent sets, at the roots and at nodes of every board of 20 to 60.  With every
  incumbent from 0 to one above the optimum, no node closed by the exact-size bound has a completion that beats the
  incumbent (22,266 pairs).  Those games promote at most 3 numbers, so the rule never closes a node that the plain
  bound leaves open there: the test shows that closing loses nothing, not that it gains.
* **The fixtures.**  The probe's 300 nodes each of 936 and 1215, and the 212 nodes of 2112 that its proof (one thread)
  handed the searcher.  Game 2170, the larger game proposed, takes 588 s on one thread, so 2112 was taken.  P\* is
  `strict.jsonl`'s score less `in_the_bag`, equal to each fixture's incumbent.
* **Open nodes.**  A node is open when its plain bound (at most `max_add` candidates) exceeds P\* less its promotions'
  sum.
* **Cost.**  Seconds on one thread: `perf make-nodes` 2 s, the three games 5 s.

## Results

| game | P\* | root gap | `max_total` | `best(k) − P*` below the window, in it | window | root's heaviest set, largest | nodes, open | open below, in, above |
|---|---|---|---|---|---|---|---|---|
| 936 | 20,507 | 132 | 52 | k = 51: −134; 52: +132 | 52 | 52, 52 | 300, 137 | 0, 137, 0 |
| 1215 | 35,050 | 213 | 73 | 68: −48; 69: +213 | 69 | 69, 69 | 300, 133 | 0, 133, 0 |
| 2112 | 103,056 | 517 | 126 | 115: −304; 116: +201, 117: +517 | 116-117 | 117, 117 | 212, 212 | 0, 212, 0 |

Killed at Step 1.  No open node lies outside the window (0 of 482, against a criterion of under 10% at every game), and
neither does any closed node (0 of all 812 nodes with candidates).  Every node's heaviest completion has, with its
promotions, the root's total size: 52, 69 and 117 (at 2112 the window's upper size).

The window is narrow because the size below it falls short of P\* (by 134, 48 and 304).  The nodes never leave it,
because the root's heaviest set is a largest common independent set at all three games.  No size above it exists, since
a node's promotions and any completion are a common independent set of the root's matroids, and every node's promotions
and heaviest completion again total that largest size, so nothing lies below.  This agrees with C4 (the root's heaviest
set has the optimum's size) and says more: the sizes that A3 would split by are all one.

Limits:

* The fixtures are the units a search hands out (the open ones hold up to 40, 44 and 78 promotions), not every node of a
  tree; nodes deeper in a unit's subtree were not sampled.
* Games whose root has a smaller heaviest set than the optimum's (1320 and 1322, C4) were not run.
* Step 2 was not run, nor a split finer than the total size (by the promotions out of each frame).

## Decision

Not kept: A3 closes nothing at 936, 1215 and 2112.  A game whose root has a smaller heaviest set than the optimum's,
such as 1320 and 1322, would reopen it.

## Files

* `promotions.jsonl`: the output of `examples/promotions.rs`.  For each game, the root's window of total sizes that can
  still beat P\*, and, for every node of the game's fixture, whether the total size of its heaviest completion lies
  below, inside or above it (one `node` line a node, one `game` line a game).  936 and 1215 read the probe's fixtures
  (`testdata/perf`): `promotions --known results/strict.jsonl 936 1215`.  2112 reads `n2112.nodes`:
  `promotions --known results/strict.jsonl --fixtures
  solver/notebook/search/split-by-the-number-of-promotions-a3s-test-2026-10 2112`.
* `n2112.nodes`: the 212 nodes of 2112, all the distinct units that the proof's 15,288 nodes handed the searcher:
  `perf make-nodes 2112 --known results/strict.jsonl --every 500 --keep 300 --fixtures
  solver/notebook/search/split-by-the-number-of-promotions-a3s-test-2026-10`.

Both commands run from `rust/` and take seconds on one thread.  The example `promotions`, with the files of
`examples/common/` it included, was removed from master on 2026-10-17; it builds at commit 8753189 (`git checkout
8753189`, then `cargo build --release --example promotions` in `rust/`).

## Commits

* `f7661cb` (2026-10-16): the diagnostic, `examples/promotions.rs`.
* `8753189` (2026-10-17): builds the example, which master no longer has.
