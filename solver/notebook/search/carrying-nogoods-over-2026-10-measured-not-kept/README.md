# Carrying nogoods over (2026-10): measured, not kept

Part of the [lab notebook](../../README.md).

Started 2026-10-12.  Concluded 2026-10-12.

**Question.**  Does a search start faster when its tables hold the nogoods of the game before it, or of the search it
resumes?  **Answer.**  No: from the nogoods of 1225, 1227, the best case, takes 10% fewer nodes and 3% more time, the
chain 1201-1231 went from 617 s to 941 s, and a resume with empty tables beats one with the tables kept (1220: 143 s
against 191 s).

## Method

A nogood is a fact about the root of a game's frame model, and consecutive games have nearly the same model.  So the
nogoods a search ends with could start the tables of the next game's search, each checked at the new root first: a set
that cannot be promoted from the root (an exact test, frame by frame) is a nogood of it whatever it came from, and
`conflict_using` makes it minimal again.  For the same reason a checkpoint could store the tables, and a resumed search
start with them.

A prototype measured both: `search::valid_nogoods`, the solver's tactic `carry_nogoods`, `ladder --carry` and
`examples/resume_carry.rs` (commit f41fff8).  The runs are on one thread unless a row says 4, with strict tactics,
seeded from n − 1 and capped as the solver does.

## Results

**Nearly every nogood survives into the next game:** 1374 of 1374 from 1024 at 1025, 482 of 494 from 1219 at 1220, 1216
of 1251 from 1229 at 1230.  Game n − 1 often does not search, since a prime takes the shortcut, and 1214, 1224 and 1226
reach the cap with their seed; so in a range the nogoods come from the last game searched (1227's from 1225, through
1226).

**Learning costs little.**  Half of what a hard game learns is learned in the first 3-27% of its search, 90% by 18-64%,
and the last nogood in its last 1-17% (1024, 1025, 1215, 1219, 1220, 1225, and 1230 stopped at 560 s).  Each is one
conflict computed where the table knew none, a few thousand in a search of millions of nodes.  So a preloaded table
could save not the learning but the search's knowing smaller conflicts sooner, which changes the tree.

**The tree changes, and not for the better** (search seconds and nodes):

| game | tables start empty | with the nogoods of |
|---|---|---|
| 1025 | 158 s 8.50M | 1024: 136 s 6.30M |
| 1025, proof only (seeded with its optimum) | 99 s 5.25M | 1024: 104 s 4.97M |
| 1220 | 186 s 13.72M | 1219: 232 s 16.47M |
| 1220, proof only | 177 s 13.51M | 1219: 240 s 17.81M |
| 1227 | 260 s 17.42M | 1225: 267 s 15.72M |
| 1230 | open at 560 s (best 36570 at 118 s) | 1229: open at 560 s (36570 at 186 s) |
| 1025, 4 threads | 35.2 s 8.51M | 1024: 38.4 s 6.82M |
| 1220, 4 threads | 54.0 s 17.16M | 1219: 68.0 s 15.91M |

1227 is the ceiling: 1225-1229 have the same frames and candidates (the same search, 17,417,736 nodes on one thread), so
1227 started with all that 1225 learned is the best a carried table can be.  The chain
`taxman solve 1201-1231 --threads 4 --tactic reuse_prev=off --tactic revise_freedoms=off` from 1200 in
`results/strict.jsonl` took 617 s (154.7M nodes) without the carried tables and 941 s (174.7M) with them, every searched
game slower (1230: 305 s against 445 s), because the table grows from game to game.

**Why: a larger table costs every node.**  A lookup scans every nogood's signature, and the nogoods of each new
promotion rule candidates out, so nodes are slower the more the table holds.  1227 runs at 59k nodes/s from a full table
against 67k/s from an empty one.  On 4 threads, whose tables start with the union of all threads' nogoods, 1025 runs at
177k/s against 242k/s.

**Keeping the tables over a resume.**  A search stopped every K nodes and resumed from its checkpoint, with the tables
as they were when it stopped (in memory, what a stored table would give), reproduces the uninterrupted search.  Resumed
with empty tables it is a different tree, and faster:

| game, stopped every | uninterrupted | resumed with empty tables | with the tables kept |
|---|---|---|---|
| 1024, 250k nodes | 41 s 2.28M | 34 s 2.32M | 42 s 2.26M |
| 1215, 500k | 66 s 5.49M | 60 s 6.22M | 69 s 5.47M |
| 1220, 1.5M | 186 s 13.72M | 143 s 11.91M | 191 s 13.48M |

**Noticed on the way:** many games of a range have exactly the frames and candidates of an earlier one, so they have the
same best promotion set, which a search of one of them has already established.  In 1201-1231 these are 1202-1203 (those
of 1201), 1205 (1204), 1207 (1206), 1210-1211 (1209), 1213-1214 (1212), 1217 (1216), 1219 (1218), 1222-1223 (1221),
1226-1229 (1225) and 1231 (1230).  Only the cap differs, and where it is higher than the optimum the search is repeated
in full.  In the strict chain above that is 1203, 1205, 1207, 1211, 1219 and 1227: 90 s of the 617 s.

## Decision

Neither is kept, and the prototype was taken out.  A game's search starts with empty tables, and a checkpoint stores
none, since a resume loses nothing by starting empty.  That old nogoods cost more than they help led to [forgetting
nogoods](../forgetting-nogoods-2026-10/README.md), which clears each searcher's table every 50,000 steps.  [Carrying a
search's learning to the next game](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md) tried again on
the GF(2) path and found the same.  A table whose size did not cost every node would reopen the question.

## Files

The folder holds this README.md alone.

## Commits

* f41fff8: the carry-over prototype, for measurement (`search::valid_nogoods`, `carry_nogoods`, `ladder --carry`,
  `examples/resume_carry.rs`).
