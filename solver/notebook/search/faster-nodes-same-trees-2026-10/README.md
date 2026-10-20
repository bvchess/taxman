# Faster nodes, same trees (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-12.  Concluded 2026-10-12.

**Question.**  Can the search's inner loop run faster with no change to any answer or tree?  **Answer.**  Yes: seven
changes together cut the cycles of the proof of game 1215 by 30%, from 151.3 G to 106.2 G (1.42 times as fast), with
every tree unchanged.

## Method

Every change had to keep the node counts of every game and the fingerprints of the probe (`examples/perf.rs`; [the
performance guide](../../../docs/design/performance.md)).  The measure is the cycles and instructions retired of the
process, since other load moves wall times by up to 3 times, on one thread and a plain release build.  The table gives
the thousands of cycles a step in the probe's window of game 1215, the giga-cycles of games 936 and 1184, and the
giga-cycles of the proof of game 1215.

## Results

Each row of the table adds one change to those above it:

| commit | change | 1215 window k cycles/step | 936 G | 1184 G | 1215 proof G |
|---|---|---|---|---|---|
| e3b8ddc | before | 22.1 | 1.59 | 4.33 | 151.3 |
| b4cb2b6 | set widths of 20, 24 and 28 words | 20.4 | 1.59 | 4.05 | 142.3 |
| ab76772 | bitset heap bucket, packed slots, settle when dirty | 18.8 | 1.47 | 3.65 | 129.3 |
| 130e4ec | `MAX_FRAMES` 12 to 8 (no board to 4095 needs more) | 18.7 | 1.46 | 3.58 | 125.9 |
| 9bfb4d6 | nogood lookups over compact member indices | 18.2 | 1.43 | 3.52 | 123.2 |
| 319753b | no peel when the new move has a factor of its own | 18.0 | 1.41 | 3.53 | 122.5 |
| ad1237c | nogood rule-outs over the compact sets too | 17.8 | 1.35 | 3.49 | 118.9 |
| 4b6ef38 | radix heap push and pop, `add_inner` inlined (300 calls a step) | 16.1 | 1.23 | 3.16 | 106.2 |

The set widths are described in [the engine](../../the-engine/README.md).  The bitset heap change also packs the
search's per-vertex fields into one slot and settles a level only when something changed.  A factor of a move's own is
one that no other move uses; a sixth of the promotions have one for their new move, and need no peel.

In all, the cycles fell 30% (1.42 times as fast) and the instructions 28% on the proof of game 1215 (33 s instead of
47 s), the cycles 27% on game 1184 and 23% on game 936, and the probe's windows 27% (game 1215) and 26% (game 936).
Profile-guided optimization, which gained 12% on the code before, adds 0-2% on the code after.  The strict chain 1..1231
on 8 threads took 279 s of game time (games 1201 to 1231: 145 s), every score as committed.  At the proof of game
1215 the time went to these parts:

| | Hungarian repairs | peels | bounds from the potentials | nogood table | level copies | node's own work |
|---|---|---|---|---|---|---|
| before | about 50% | 19% | 5% | 8% | 4% | |
| after | 46% | 24% | 7% | 3% | 4% | 7% |

Tried and not kept (cycles, one thread):

* **An undo trail for the matching levels** instead of a copy per push: pushes cost little, but logging every change
  of a repair costs more (windows +0.8% and +0.9%, instructions +4%).
* **The peel in frame-local form** (each frame's movers and factors numbered within it, kept in the frame set): with the
  bit-parallel peel in rounds the checks of single promotions were 17-26% cheaper, but the frames of forced sets need
  many rounds (windows +8% and +16%); with the counting peel on local indices, +0.5% and +0.8%.
* **Caching frame checks:** 14-17% of the peels in a window repeat an earlier one exactly, but only 5% repeat one of the
  last eight.
* **Five smaller changes:** the no-peel test above for the several moves of a forced set; promoting a forced set
  straight into the child's state; stopping the bound from the potentials once its first part decides; the Dijkstra
  horizon (no events later than a known settling one: windows +0.4% and +2.5%, as in an earlier measurement); one
  branch for `reach`'s two tests.  Each was within ±0.5% or worse.
* **Unchecked indexing in the Hungarian search:** -4% instructions but -0.7% cycles, not worth `unsafe`.
* **Widths of 12 and 48 words:** 3-4% at games 512 to 767 (solved) and some at games 2048 to 3071, for 12 s more of
  every build.

## Decision

The seven changes are kept; the others are not.  Four ideas were not tried, each for its reason: `i32` potentials (no
bound on the potentials is proven, so the change is not exact by construction); a warm-started unit rebuild (another
optimal matching can change the tree after a split or a resume); candidate-index sets for every search-level set (a
large refactor); and watched members for the nogoods (the table takes 3% of the time after the changes).

## Commits

The first column of the table, all of 2026-10-12.  e3b8ddc is the baseline: the performance probe with replayable
fixtures, and the bench at the frontier.
