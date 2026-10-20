# Forgetting nogoods (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-12.  Concluded 2026-10-12.

**Question.**  Does a searcher that clears its nogood table every N steps solve the hard games faster than one that
never does?  **Answer.**  Yes: clearing after each 50,000 of a searcher's own steps takes 26% less time on 1024, 1025,
1215 and 1220 together, and the strict chain 1201-1231 on 4 threads falls from 571 s to 377 s with every score the same.

## Method

[Carrying nogoods over](../carrying-nogoods-over-2026-10-measured-not-kept/README.md) had shown that a search resumed
with empty tables was faster than one that kept them.  Each searcher clears its table after each N of its own steps: the
tactic `forget_nogoods`, with `search::NOGOOD_PERIOD` as N.  It is sound, since the table is a cache and every search
starts with an empty one anyway, and deterministic on one thread.

* **The interval:** the search time of 1024, 1025, 1215 and 1220, added up on one thread, against never forgetting run
  alongside.
* **The comparison of record:** never forgetting against forgetting every 50,000 steps, with strict tactics, seeded from
  n − 1 and capped as the solver does, on one thread and on 4 threads (two runs each).

## Results

**The interval.**  The time saved against never forgetting, on the four games added up:

| steps between clearings | 10,000 | 25,000 | 50,000 | 100,000 | 250,000 | 1,000,000 |
|---|---|---|---|---|---|---|
| less time | 22% | 22% | 26% | 25% | 21% | 15% |

The node counts move both ways (1025 +7% to +28%, 1220 −11% to −28%).  What the interval buys is cheaper nodes, since a
lookup scans the whole table: 1215 runs at 110k steps/s forgetting every 10,000 steps and at 83k/s never forgetting.  A
game of fewer steps than the interval is unchanged, and the fast set 880-1184 changes by at most 4%.

**Never against forgetting every 50,000 steps** (search seconds and nodes):

| game | one thread, never | one thread, forgetting | 4 threads, never (two runs) | 4 threads, forgetting |
|---|---|---|---|---|
| 992 | 73.3 s 4.00M | 52.6 s 4.20M | 19.5 s, 16.9 s | 18.0 s, 16.5 s |
| 996 | 69.0 s 4.17M | 61.9 s 4.78M | | |
| 1024 | 43.0 s 2.28M | 34.3 s 2.38M | | |
| 1025 | 168.5 s 8.50M | 140.5 s 10.11M | 35.3 s, 35.3 s | 32.1 s, 32.3 s |
| 1215 | 69.6 s 5.49M | 50.6 s 5.16M | 21.8 s, 21.0 s | 15.3 s, 16.1 s |
| 1216 | 22.4 s 2.10M | 21.1 s 2.22M | | |
| 1220 | 192.2 s 13.72M | 125.4 s 10.51M | 60.6 s, 55.1 s | 38.8 s, 40.5 s |
| 1225 | 262.4 s 17.42M | 154.2 s 12.07M | 54.7 s, 55.2 s | 31.9 s, 33.4 s |

The strict chain 1201-1231 on 4 threads takes 377 s instead of 571 s (1230: 158 s instead of 275 s; 1225: 33 s instead
of 55 s; 1220: 39 s instead of 62 s).

**Tried and not kept:**

* Forgetting by size, clearing the table whenever it holds 64, 128 or 256 nogoods, takes 15%, 22% and 17% less time on
  the four games, but the smaller games run slower, since they clear while still learning (936: 0.51 s against 0.85,
  0.61 and 0.53 s; 1050: 3.96 s against 5.19, 4.49 and 4.33 s).
* Keeping the nogoods used since the last clearing (returned by a lookup, or ruling a candidate out of a child) takes
  15% less time when clearing every 100,000 steps and 6% every 500,000.  The kept nogoods cost lookups as much as the
  others and help no more.

## Decision

Kept: the tactic `forget_nogoods` with a period of 50,000 steps, on a flat optimum (22-26% from 10,000 to 100,000).  The
gain is the cost of a lookup, so a table that a lookup does not scan whole would reopen the choice.  [Keeping and
forgetting nogoods at the end of the chain](../keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/README.md)
measured the period again on the hardest games and left it.

## Files

The folder holds this README.md alone.

## Commits

* 576e2e2: forget nogoods every 50,000 steps (the tactic `forget_nogoods`).
