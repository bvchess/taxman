# Single-member conflicts belong to the node (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-12.  Concluded 2026-10-12.

**Question.**  Many conflicts the search computes have one member: are they candidates that cannot be promoted from the
root at all, so that testing each candidate once at the root and dropping the failures would save every searcher the
work?  **Answer.**  No.  The frame builder's candidates all pass that test, and a filter at the root dropped 0 of 225,
273, 287 and 302 candidates at 936, 1100, 1184 and 1215; a single-member conflict is a fact about a node.

## Method

* **The sign.**  `--diag` reports the sizes of the minimal conflicts: the 25th percentile is 1 at 936 and the 10th is 1
  at 1184.
* **The test.**  The frame builder's candidates were checked against `Model::can_promote` at the root: the home frame
  without the factor, and the frame below with the factor as a move.  A hive's candidates are its factors that can
  leave it and that some hive can take on as a source, which is what that check asks.  The test is
  `the_frame_builders_candidates_can_all_be_promoted_from_the_root` in `solver/frame/src/search/tests.rs`.
* **The filter.**  A tactic (`root_filter`) tested each candidate once from the root and dropped the failures.  The
  checkpoint header recorded the candidates it left, so older checkpoints were refused as a different candidate set.
  It searched 936, 1100, 1184 and 1215 on one thread.

## Results

Every candidate of every game from 2 to 1230 passes the root test (the test covers every game to 511 and every sixth
from 512 to 1230).  The filter changed nothing:

| game | candidates dropped | steps with the filter | steps without it |
|---:|---:|---:|---:|
| 936 | 0 of 225 | 58,903 | 58,903 |
| 1100 | 0 of 273 | 56,399 | 56,399 |
| 1184 | 0 of 287 | 167,960 | 167,960 |
| 1215 | 0 of 302 | 5,161,817 | 5,161,817 |

A nogood of one member would be a candidate that cannot be promoted from the root, so the nogood table never holds
one.  A single-member conflict is a candidate that cannot be promoted once the node's promotions are made: its frame
may have used it up in a forced pair, say.  In another branch, with other promotions, the same candidate is part of a
nogood of two or more members.

Little is left to save either way.  The promotions that fail are 2,282 of 58,903 steps at 936 (3.9%) and 698 of 167,960
at 1184 (0.4%) (`--diag`, one thread), which bounds what a search that never tried a dead candidate could gain.

## Decision

Not kept: the filter dropped nothing.  The search looks up a single-member conflict at the
node, which is cheap, and the loop goes on without the candidate.  [Search design](../../../docs/design/search.md)
states the result.  A game with a candidate that fails the root test would reopen the question; the test covers
games to 1230 only.

## Commits

* `86b1918` (2026-10-12) records the finding and adds the test.
