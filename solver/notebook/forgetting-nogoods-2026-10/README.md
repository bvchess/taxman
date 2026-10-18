# Forgetting nogoods (2026-10)

Part of the [lab notebook](../README.md).

The measurements of carrying nogoods over pointed the other way: a search
resumed with empty tables was faster than one that kept them.  So each
searcher now clears its table after each 50,000 of its own steps (tactic
`forget_nogoods`, `search::NOGOOD_PERIOD`; sound, since the table is a cache
and every search starts with an empty one anyway; deterministic on one
thread).

The interval, on one thread (search time of 1024, 1025, 1215 and 1220 added
up, against never forgetting run alongside): every 10,000 steps 22% less
time, 25,000 22%, 50,000 26%, 100,000 25%, 250,000 21%, 1,000,000 15%.  The
node counts move both ways (1025 +7% to +28%, 1220 −11% to −28%); what the
interval buys is cheaper nodes, since a lookup scans the whole table (1215:
110k steps/s forgetting every 10,000 steps, 83k/s never).  A game of fewer
steps than the interval is unchanged.  Tried and not kept:

* Forgetting by size, clearing the table whenever it holds 64, 128 or 256
  nogoods: 15%, 22% and 17% less time on those four, but the smaller games
  slower, since they clear while still learning (936: 0.51 s against
  0.85/0.61/0.53 s; 1050: 3.96 s against 5.19/4.49/4.33 s).
* Keeping the nogoods used since the last clearing (returned by a lookup, or
  ruling a candidate out of a child): every 100,000 steps 15% less time,
  every 500,000 6%.  The ones kept cost lookups as much as the others and help
  no more.

Never forgetting against forgetting every 50,000 steps (search seconds and
nodes; strict tactics, seeded from n − 1 and capped as the solver does; the
fast set 880–1184 is unchanged to ±4%):

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

The strict chain 1201–1231 on 4 threads (as above) takes 377 s instead of
571 s, with every score the same (1230: 158 s instead of 275 s; 1225 33 s
instead of 55 s; 1220 39 s instead of 62 s).
