# The ledger: bounds from both sides (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-17.  Concluded 2026-10-18.

**Question.**  Instead of proving opt(n) one game at a time from a proven opt(n − 1), can a program keep for every board
of a stretch a lower bound L(n), the score of a game held, and an upper bound U(n), tighten both across many games at
once, and spend the effort where progress is cheapest, so that it proves much of a stretch for a small share of what the
chain spends there?  A game is proven when L(n) = U(n).  **Answer.**  It proves correctly, with 0 violations in every
run, but not more cheaply where the chain's searches are cheap: on 1301..1500 it proved all 200 boards for 1.2 to 1.3
times the chain's CPU, and the second cloud run (2176..3440, 48 core-hours) proved 916 of 1,265 boards for about 2.2
times the chain's cost.  Climbs from a proven board, a finder of good games and searches under floors cut that cost, and
past the chain's end the ledger proved 242 of 3,792 boards of 4400..8191 in 360 core-hours and keeps every bound it
found in `results/bounds.jsonl`.

## Method

**The dual bound of a stopped search** (`search::dual_bound`) is the largest of the incumbent and, over the open units
of the final checkpoint, each node's promotion sum plus the GF(2) bound the search computes when it starts the unit
(Lemma D in [the ledger's theory](../../../../docs/theory/ledger.md)).  **The relations** between boards (the same page)
are L1, monotone, opt(n − 1) ≤ opt(n); L2, the ceiling, opt(n) ≤ opt(n − 1) + n; L3, the prime formula both ways, opt(p)
= opt(p − 1) + p − q for q the largest prime below p; and L4, repeats, U(n) − bag(n) ≤ U(m) − bag(m) for equal search
problems.  Each comes with a construction of a game that needs a finished game (legal, over, opening with the largest
prime), not an optimal one, and every game the ledger holds is finished.  A repeat carries a lower bound only as the
promotions of a game made from them, and its upper half rests on statement U at the board it is carried to.

**The program** (`examples/ledger/`; the module comment of `main.rs` has the commands, fields and events) is a Rust
example rather than a Python driver of the binaries, since its searches need a seed from any game, the ledger's U as
their ceiling and the dual bound read from each checkpoint.  Every board it searches first gets its **map**, a search
stopped before its first node, which gives the start incumbent and the root bound.  Then come searches of one thread,
`--base` seconds the first time and twice as long each time after, each going on from the board's checkpoint, seeded
with the best of the games held for n − 1, n and n + 1, with U(n) as the ceiling.  Boards whose bounds a proven
neighbour has changed go first, smallest gap first; the rest go by least `level + gap / 175`, a decade of the chain's
steps per about 580 of standalone gap ([the
landscape](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md)). After each search the
relations run to a fixpoint, every game they make is replayed by the referee, and L > U stops the run.

`solver/tools/ledger/cloud.sh` runs the ledger with no proven game, then `ledger report --known results/strict.jsonl`
([the runbook](../../../docs/runbook/ledger.md)).  For 2176..3440 at an hour of 64 CPUs it takes about 1.3
instance-hours with the build, about $0.9 at spot and $3.3 on demand.

## Results

### The first dry run, 1301..1500 (2026-10-17)

On the laptop (the non-release build, six workers, a budget of 2 CPU-hours, no proven game; [dry/](dry/)), the ledger
proved every board after **595 CPU seconds** (145 s of wall time), with **0 violations**: every game replays to its L,
L ≤ opt ≤ U at all 200 boards, and all 200 proven values are the committed chain's.  Searches proved 83 boards, all by
exhaustion; relations proved 117 (repeats 79, the ceiling 21, the prime formula 17).

* **The dual bound** only falls, slowly.  Of the root gaps of games 996, 1020 and 1218 (318, 223 and 146 over the
  optimum) it had closed 23%, 61% and 29% at 9,215 steps, 65%, 96% and 88% of the way through their trees (14,200, 9,554
  and 10,481 steps), since a depth-first search keeps the shallowest, largest subtrees open until late, and their bounds
  are the root's or near it.  In the dry run it lowered U in 51 of the 95 stopped searches, by 3,833 in all, and proved
  nothing alone.
* **Against the chain** over the same stretch on one thread, standalone at 1301: 447.5 s (89 searches, 27 primes, 84
  repeats).  The ledger spent 1.33 times that CPU in a third of the wall time on six workers.  The dearest boards cost
  it what they cost the chain; the extra third is 35 s on the 9 boards the chain settled at its ceiling with no search,
  84 s net on the boards both searched to the end (here in pieces, at first without the seed and ceiling of a proven
  neighbour), and 20 s of maps.

On an easy stretch the ledger is the chain run out of order on many threads: the same searches prove the hard boards,
the relations prove the rest, and the out-of-order work costs a third more CPU.

### Later (2026-10-17): the first cloud attempt, stopped, and the fix

`solver/tools/ledger/cloud.sh 2176-3440` on a `c8g.16xlarge` with 64 workers was stopped by a signal after 9 minutes at
1,299 CPU s (0.36 core-hours, about $0.18).  The machine was 95% idle (2.3 of its 64 CPUs in use), the ledger's main
thread at 100% with the workers waiting on it: 390 of the 1,265 boards had their map, and no search had finished
([cloud-20261017-stopped/](cloud-20261017-stopped/)).

The relations were the cause.  The maps came one at a time, and every cascade of relations ran on through boards that
had no map yet: the first map was in at 1.45 s and its relations ran until 59.1 s (2,863 changes, a game built and
replayed at each), and the 390 maps made 161,615 changes, which took 528 of the 540 seconds.  What grows is boards times
maps.

The fix: the maps come first, all of them on the workers, and the relations carry them once to every board (`settle`); a
game is made only if it can beat the target's lower bound; games are completed in one pass; and the referee finds a
move's divisors up to its square root and plays a game once, not twice (4.66 ms to 0.13 ms at game 3400, 0.43 to 0.02 at
game 1000).  Every bound is the bound it was, and each lower one a game the referee replays.  On the laptop (8 workers,
2176..2400, 360 CPU s) the main thread's busy time fell from 17.5 s to 0.8 s and the workers' use of their CPU rose from
87% to 95%.  On 2176..3440 (2,520 CPU s) the settle takes 5.4 s for 1,767 changes and the workers use 96% of their CPU.
The dry run of 1301..1500 again proved all 200 boards, 0 violations, in **573 CPU s and 133 s**.

### Later (2026-10-17): wedge and climb

**Why.**  The second cloud run (2176..3440, the truth hidden, 48 core-hours on 64 workers) proved 916 of 1,265 boards,
every one at the optimum, but paid about 2.2 times what the chain paid on the same games, because it searched most
boards before the board below them was proven: only 103 of the 356 boards proven by their own search had n − 1 proven
when their final search began.  The searches that began with n − 1 proven used 6.7 core-hours and completed 106 times;
those that began without used 40.8 and completed 265 times.  A proven board below gives the search of n the ceiling
(exact in about a third of the boards) and a start that is the optimum about 88% of the time.  The ledger's order finds
*wedges*, boards proven with no help from a neighbour; the change adds climbing from them, in both directions.

**The change** (`examples/ledger/schedule.rs`; [the runbook](../../../docs/runbook/ledger.md) describes the settings).
A searched board that is not proven and has its board below proven, however that was proven, gets a *climb* ahead of
every other job but the maps: its ordinary search, seeded with the proven game below and capped by U(n), for
`--climb-limit` seconds.  When a climb proves its board, the board above is climbable at once, and so on up the stretch.
The same holds for the first unproven, searched board below a proven one (a climb down), seeded with the game above; the
game that L2 makes there is exactly the optimum for n − 1 in about 35% of the composite pairs of 2176..3440, and the
ledger's ceiling is loose.  A climb that ends with the board unproven *parks* it, and it goes by the ordinary order with
its checkpoint. No ordinary search starts on a board next to a board being climbed to, for at most the climb's limit
(the hold).

**The default limit, 60 s.**  From the second cloud run's events (`climb_limit.py`): 565 boards were searched and 371
finished by a search.  A climb of a given limit on every one of them, costing a board its total search seconds in the
run (the limit if that is less or if the board was not finished), would have made

| limit (s) | 10 | 20 | 30 | 45 | 60 | 90 | 120 | 180 | 300 | 600 |
|---|---|---|---|---|---|---|---|---|---|---|
| proofs | 34 | 98 | 129 | 168 | 190 | 225 | 248 | 269 | 299 | 331 |
| core-hours | 1.5 | 2.9 | 4.1 | 5.9 | 7.5 | 10.5 | 13.2 | 18.3 | 27.6 | 48.4 |
| core-s a proof added since the limit before | | 76 | 146 | 162 | 263 | 309 | 421 | 870 | 1,126 | 2,338 |

The run's own proofs by an exhausted search cost 109 core-s each in its first 8 core-hours, then 929, 1,920, 1,800,
1,516 and 2,618 in each next 8, which would put the limit at 120 to 180 s.  The default is 60 s because the table counts
every board once while a board whose climb parks has had up to the whole limit for nothing but the checkpoint (the worst
case at 60 s is 565 boards × 60 s = 9.4 core-hours, at 180 s 28), and because at 60 s the price of a proof added is
still under 300 core-s and rises steeply past 90 s.

**The dry runs** (the laptop, the non-release build, the truth hidden; [climb/](climb/)), back to back: `up` with the
build of eb9dc9c, `off`, `both` and `down` with the build of 55d2b2a.  All had 0 violations.

| run | proven | CPU s | wall s | climbs up | climbs down |
|---|---|---|---|---|---|
| 1301..1500, 6 workers, off | 200 of 200 | 598 | 151 | | |
| up | 200 of 200 | **547** | 157 | 50 proved, 363 s | |
| both | 200 of 200 | 565 | 138 | 50 proved, 323 s | 8 proved, 36 s |
| down | 200 of 200 | 576 | 143 | | 12 proved, 83 s |

* **1301..1500**: the climbs cost less CPU than none: 547, 565 and 576 against 598, 1.22 to 1.29 times the one-thread
  chain's 447.5 s (1.33 before).  50 boards were proven by climbs up (a median 4 s), and no climb parked.  The wall time
  did not fall with the climbs up alone (157 s against 151), since the hold put the long boards 1475 and 1476 one after
  the other; with both it was 138 s.  The climbs down are few and cheap, 4.5 and 7 s each.
* **2576..2700** (a valley, where the chain spent minutes a game), 8 workers: at 600 CPU s the runs proved 46 (off), 49
  (up), 56 (both) and 51 (down) of 125 boards, and at 1800 CPU s 107, 108, 108 and 111, differences that are the noise
  of the runs.  The up-only curve of boards proven against CPU lies above the run without climbs in the middle (at 722
  CPU s 56 against 51, at 1084 s 70 against 61) and meets it at the end: the climbs bring proofs forward and prove no
  more by 1800 CPU s.  Their effect in the cloud, with 1,265 boards, 64 workers and searches begun before their board
  below was proven, was not measured at this point.

### Later (2026-10-17): the finder in the ledger

**Why.**  [The finders experiment](../finders-for-good-games-that-are-not-sound-2026-10/README.md) found that large
neighbourhood search (LNS: keep a random 70-90% of the incumbent's promotions, search the rest exactly as a unit,
repeat) reaches the optimum of 64% of the games of 2176..3440 in 5 CPU seconds of one thread, against 21% for the sound
search in the same time, and that the sound search seeded with 1 second of its game costs 0.48 of the CPU, finder
included.  A lower bound needs only a legal game, so the ledger may take its L(n) and its seeds from LNS if the referee
replays every game; nothing LNS finds is a proof, and the upper bounds are what they were.

**The change** (`examples/common/lns.rs`, `examples/ledger/`; [the runbook](../../../docs/runbook/ledger.md) describes
the settings).  On the worker, for `--lns-seconds S` of one thread (default 1), the finder runs after a board's map from
the start incumbent, and before a search whose best seed has changed since the finder last ran on the board (a climb
always counts as changed).  The search is then seeded with the finder's game if that is better.  The game goes through
the referee and is offered as the board's L with the rule `lns` only if it beats the game the finder began from.

**The dry runs** (the laptop, the build of 4621d97, the truth hidden; [finder/](finder/)): 1301..1500 on six workers
with the finder at 1 s and without it, back to back; 2576..2700 on eight workers with a budget of 900 CPU s, four runs
in the order on, off, off, on.

| run | proven | CPU s | wall s | searches (s) | finder runs (s) |
|---|---|---|---|---|---|
| 1301..1500, finder 1 s | 200 of 200 | 612 | 122 | 113 (438) | 142 (143) |
| finder off | 200 of 200 | 557 | 145 | 128 (535) | |
| 2576..2700, finder 1 s, twice | 61 of 125 | 908, 909 | 116 | 132 (695), 136 (694) | 87 (88) |
| off, twice | 61 | 907, 910 | 115, 116 | 136 (783), 139 (784) | |

* **1301..1500**: with the finder the run took 10% more CPU and 16% less wall time (the workers were busy 5.0 of 6
  against 3.8, because the stretch's tail was shorter).  **51 of the 200 boards, 57% of the 89 that are searched, end
  with its game as their lower bound**, all 51 proven; 70 of its 142 runs found a better game (by 10,599 in all, a
  median 132), the 89 after the maps doing almost all of it.  It proves later at first: 63 boards were proven at 150 CPU
  s against 113 without it, and both end proven.
* **2576..2700**: 61 of 125 boards proven in each run, finder on or off.  The curve differs: boards proven at 150, 300,
  450, 600, 750 and 900 CPU s were 0, 31, 44, 46/48, 54 and 58/61 with the finder against 12, 37, 46/47, 56, 58 and 61
  without. On this valley the finder neither proves more in 900 CPU s nor costs proofs at the end; it moves the work to
  the front.
* **The reading**: the finder's gain is at the maps, where it improves the start incumbent by a median 132; the ledger's
  own seeds are already close, so it rarely improves on them before a search.

**The prep run** (cloud, 2026-10-18; [cloud-20261018-prep/](cloud-20261018-prep/)): 2176..3440 as two ledgers side by
side on one machine, each with 32 workers and 24 core-hours, the games to 2175 given and the climbs both ways, `a/` with
the finder (1,042 runs) and `b/` without.  They proved 876 and 837 boards besides the given 2175, all at the optimum
with 0 violations, against 806 for the second cloud run at the same core-hours.  The climbs proved their board 53-69% of
the time at about 50 CPU s a proof.

### Later (2026-10-17): the bounds store

**Why.**  What a run finds is knowledge even where no game is proven: for every board a game worth L and a bound U, with
how each was derived.  Those lived only in a run's directory, and the maps of 3441..8191 alone cost about 65 core-hours.
The project's owner wanted a durable, committed place for them, and future runs that start from it.  A board is proven
when its best L equals its best U, so the store is the record of those proofs too.

**What was built** (`examples/ledger/`: `store.rs`, `export.rs`, `verify.rs`, `pool.rs`; `solver/tools/check_bounds.py`;
the format is in [results/README, "bounds.jsonl"](../../../../results/README.md#boundsjsonl)).  The file
`results/bounds.jsonl` is append-only, one line a bound, a new line only when it improves the file's best of its board
and side.  `ledger export` appends what improves, `ledger run --bounds FILE` starts from the file's best bounds of the
stretch, and `ledger verify-bounds`, `ledger games` and `check_bounds.py` are the two referees.

**Choices.**

* **A lower bound's game in the most compact form.**  Promotions, where the game was made from them: about 330 bytes at
  game 900 and 1.1 kilobytes at 3440, against 1.7 and 9 for the moves.  A game that a relation made of another board's
  game (monotone, ceiling, prime) is *cited*, not written (`from` and `from_value`; 135 bytes).  Without the cited form
  the dry run's file was 75 kilobytes, not 45 (1,299 bytes a board, not 778), and the file of the boards past the chain
  would have been of the order of 26 megabytes, not 12.  The file cannot be rewritten once committed, so the form had to
  be there from the first export.
* **The Python referee** shares no code with the crate, so it cannot turn promotions into moves.  `ledger games FILE`
  writes the moves of every promotion set, and `check_bounds.py` replays those by its own rules and checks that they
  contain the promotions and score the line's value; the cited games it rebuilds with its own port of `complete` and the
  prime exchange.
* **What an upper bound rests on** is `rests_on`, the boards at which statement U is assumed: its own board for a root,
  dual or exhausted bound or a given game, what the bound it comes from rests on for a relation, and its own board added
  for a repeat.  Where the chain has a game of a board that the file also has bounds of, both referees require
  L ≤ score ≤ U, so a run on boards the chain has proven is a test.

**The dry run** ([store/](store/)).  901..960 on the laptop with the truth hidden (except the chain's lines of 900, 930
and 945): 6 CPU s proved 15 of 60 boards, and the export appended 116 lines for 58 boards.  A second run, 3 CPU s from
the file, proved all 60, and its export appended the 55 lines that improved.  The file: 171 lines of 45 kilobytes, 58
proven boards, **778 bytes a board**; by kind, 46 games of promotions (329 bytes each), 19 cited (135), 7 of moves (1.7
kilobytes) and 99 upper bounds (155).  Both referees accept it against the real `results/strict.jsonl` (`verify-bounds`
in 0.2 s on 4 workers, the Python referee in 0.1 s), and they refuse tampered copies: moves swapped, a value raised, a
ceiling bound lowered by one, a cited line removed, a board dropped from `rests_on`, a bound below the chain's score.
At larger boards (2900..2939) the file took 1,834 bytes a board.  Past the chain (4400..8191) a promotion set is about 2
kilobytes, a game of moves about 15 and a cited game about 150 bytes, so the first export of 3,800 boards is of the
order of 12 megabytes (3 kilobytes a board), against 26 without the cited form.

### Later (2026-10-18): past 4400, stage 1

**Why.**  The prep run proved 876 boards with the climbs and the finder, and the project's owner approved up to $25 to
prove games the run to the wall will not reach: find the easiest spots past 4500, drill on them, and spread both ways.

**The run.**  4400..8191 (3,792 boards) on one `c8g.16xlarge`, commit b3ed8ac, 64 workers, the finder 10 s, climbs both
ways with a limit of 1 hour, nothing given: 360 core-hours (about $3.80), 2026-10-18 02:41-08:22 UTC.

**Results.**  Every board was mapped and given the finder's game (it improved 1,720 of 2,042 runs) within about 50
minutes; then 242 boards were proven: 4446-4475, 4477-4507, 4540-4542, 4554-4622, 4626-4639, 4664-4757 and 4779, by
repeats 96, their own search 95 (82 of them from the finder's game), the ceiling 35, primes 16.  Proven against
core-hours: 112 at 72, 164 at 180, 212 at 288, 242 at 360.  The 4400..4799 boards took 60% of the search time and gave
every proof; 4800..5399 took 30% and gave none.  After the finder the smallest U − L past 4800 is 1,664 (medians 2,054
at 4800..5199, rising to 7,631 at 8000..8191).  Climbs: 22 of 30 up and 2 of 8 down proved their board.  The bounds of
all 3,792 boards are the first lines of `results/bounds.jsonl` (8.4 MB), both referees passing.  Stage 2 (4300..4999 for
12 hours from stage 1's state, commit afe4513) and stage 3 went on from it; their surveys are under Files.  Stage 3,
3900..4999 with floor steps, ran on one `c8g.16xlarge` on spot at commit 92c1e44, 2026-10-18 17:18-18:02 UTC, for about
$0.49: floors lowered U on 1,269 boards, and 382 boards of 3900..4999 were proven, 39 of them newly; its bounds are in
`results/bounds.jsonl` (ca357bf).

### Later (2026-10-18): floor-guided drilling

**Why.**  The ordinary search of an open board doubles its limit and resumes its checkpoint, and until it finishes it
leaves little on the upper side; and the order, `level + gap / gap_unit`, has no idea what a board's proof will cost.  A
search under a floor F ([searches under a
floor](../proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md), `search::search_above`) that
completes having found nothing proves "nothing weighs more than F" (Lemma F), at a cost log-linear in the distance below
the root's bound: a decade of nodes per 147 points, a quarter of the gap for 1/20,000 of the proof.  So a step leaves a
proven upper bound behind for a small share of the proof, and two or three steps give a board its own line, a forecast
of what its proof costs.  The leverage between games is the proven board below (a search with n − 1 proven costs about
2.4 times less), so the climbs stay first.

**The design** (`examples/ledger/cost.rs`; [the runbook](../../../docs/runbook/ledger.md) describes the settings).  A
*floor step* is the job of an open, searched board that is not climbed to and whose gap `U − L` is more than two steps:
`search_above` under the floor one step below U, on one thread, limited to `--floor-slack` (4) times its forecast time.
If it completes having found nothing, U falls to the floor (the upper rule `floor`) and the step is kept as a point of
the board's line; if it found sets and completed, the last is the optimum (Lemma F) and the board is proven; if its
limit stopped it with a set, L rises to that game; if its limit stopped it with nothing, it is kept with its checkpoint
and resumed under the same floor with the limit doubled.

* **The step** is the same factor of cost whatever the board: `--floor-step` (150) points at the median slope, a tenfold
  of nodes, and on a board with its own line 150 · 0.0068 / slope points.  The board goes to its proof when its gap is
  within *two* steps.  On the 14 games measured under floors (`floor/ladder.py`), the steps and the proof together cost
  a median 1.05 times the proof alone (steps of 100 points 1.14, of 200 1.01), and 1.49 if the proof comes only when the
  gap is within one step.
* **The line and the forecast.**  log10(nodes) = intercept + slope · distance below the root's bound, fitted through the
  board's completed steps of 100 nodes or more, the slope pulled to the median 0.0068; with no point, the median line
  (0.5 + 0.0068 · distance).  The forecast of the proof is the line at the distance down to the lower bound, and not
  less than twice what the board's ordinary searches and climbs have spent without finishing.  On the 14 games its
  median absolute error, in log10 of nodes, is 0.59 with no point, 0.35 with one, 0.27 with two and 0.13 with three.
* **The order**: after the maps, the climbs; the boards a proven neighbour has boosted, smallest gap first; then the
  boards that have yet to give their line two points, each a cheap step; then the board whose forecast seconds are
  least.  The exploring tier came from the first build's dry run at game 4500, where a board with no line is forecast by
  the median line, which is far off, so the few boards that had a line looked 5 decades cheaper than the rest and kept
  the workers.
* A ledger written before the steps goes on: the root bound of each board that is mapped, unproven and searched is
  computed at the start (0.15 s at game 5000, on the workers), and the checkpoints of its ordinary searches are kept and
  resumed when the board comes to its proof, forecast at least twice what the old search spent.

**The dry runs** (the laptop, the non-release builds, the truth hidden; [floor/](floor/)): `base` is the scheduler
before this change (93056fd), `first` the build of f775cc0 and `new` that of 26c5c71 (the boards exploring first, and
the checkpoint of a step that found a set kept).  All had 0 violations; 1301..1500 took 619 to 620 CPU s in all three
builds, most of its boards having gaps within two steps.

| stretch, workers, budget | build | proven | CPU s | searches (s) | of them floor steps (s) | U − L left, summed (boards) |
|---|---|---|---|---|---|---|
| 2576..2700, 8, 900 CPU s | base | 61 | 911 | 127 (867) | | 37,550 (64) |
| | first | 69 | 909 | 177 (788) | 84 (38) | 14,543 (56) |
| | new | 73 | 908 | 212 (807) | 124 (62) | 9,703 (52) |
| 3076..3200, 8, 900 CPU s | base | 0 | 911 | 109 (648) | | 132,877 (125) |
| | first | 0 | 915 | 79 (655) | 39 (149) | 128,735 (125) |
| | new | 0 | 906 | 195 (646) | 193 (584) | 65,806 (125) |
| 4500..4523, 4, 540 CPU s | base | 0 | 558 | 33 (426) | | 40,043 (24) |
| | new | 0 | 544 | 64 (407) | 64 (394) | 26,301 (24) |

* **On 2576..2700 the steps bring proofs forward**: at 900 CPU s the `first` and `new` builds proved 69 and 73 boards
  against 61 (four earlier runs of the same budget with a build before the steps each had 61), and the gaps the open
  boards have left sum to 14,543 and 9,703 against 37,550.  At 1800 CPU s all three have 116 to 118 of the 125.  The
  `new` build is behind `base` up to 700 CPU s and ahead from 900 to 1300 (65 against 59 at 900, 82 against 68 at 1080).
  The 8 to 12 boards at 900 CPU s are about the 5 by which repeated runs of one build differ: the gain in proofs is real
  but modest here, and in the gaps it is large.  The steps cost 38 to 62 s of the 900.
* **On 3076..3200 nothing can be proven in the budget** (the maps take 320 of the 906 CPU s), and the steps spend the
  rest on bounds: with every board exploring first, 193 steps (584 s) lower U on all 56 searched boards, and the gaps
  sum to 65,806 against 132,877, half; the first build stepped eight boards only.  Of the 584 s, 322 were steps that
  completed, 62 were steps that a neighbour's bounds ended (moot), and 200 were steps that the end of the budget
  stopped.
* **At game 4500** the first points of the line have a slope of 0.0033 to 0.0039 a point (about 0.0035), about half the
  median 0.0068 of the 14 games: the steps of one board went down 150, 150, 150, 150, 195, 257 and 289 points as the
  line flattened.  On the 24 boards (12 of them searched) the steps lowered the gaps' sum from 40,043 to 26,301 in 540
  CPU s.
* **The forecast** of a board's proof after its last step, against the nodes of the searches that proved it (log10,
  forecast less actual), has a median −0.55 (59 boards) on 1301..1500, −0.33 (32) and −0.22 (52) on 2576..2700: a factor
  1.7 to 3.5 too small, the largest errors in the proofs of a few thousand nodes.  Of the steps, 13 of 193 on
  3076..3200, one of 124 on 2576..2700 and eight of 64 at game 4500 stalled, all but one in the last seconds of the run.

### Later (2026-10-18): the chain draws on the known games

**Why.**  The store holds 343 proven boards in 4300..4783, which the run to the wall will reach and would prove again,
at the hours a game costs there.  The design extends the flag that already means "use what is known", `--use-prev`, to
every proven game the repository holds (`optimal.json`, the committed chain and the store), where the first brief was a
new option `taxman solve --bounds FILE`.

**What was built** ([the frame solver's README, "Known games"](../../../frame/README.md#known-games)).  The store's
line, relations and replay moved into the library (`src/store.rs`), and `src/known.rs` reads the three sources.
`--use-prev` records a board that a source has proven with no search (method `known`, proof `known`); the game of n − 1
from a source gives the ceiling and the seed; a board that the store has not proven seeds its search with its lower
bound's game and ends it at its upper bound.  The best source is the committed chain's (it assumes nothing), then the
store's, then the table's. A bound of the store holds wherever statement U holds at the boards its upper bound rests on,
and both referees look the cited lines up in the sources.

**Measured.**  `taxman solve 4650-4759 --use-prev` on the real store (110 proven boards, the laptop, 4 threads) took 4.9
s (15.8 s of CPU, nearly all of it the referee's replay of the lower bounds' promotion sets at game 4,700), and wrote
110 lines of proof `known` in 1.4 megabytes, assuming statement U at 83 boards in all; `taxman verify-results` takes 0.2
s on them and the Python referee 0.7 s.  The chain's own cost for those boards is hours (game 3440 alone took 5 hours on
64 threads).  On boards 900..960 with a hand-made store of six proven boards, the six lines cost no nodes and the other
55 have the scores of the chain without it.  Not run: the cloud, and the store's open boards on a game of the wall's
size.  A known game sets up no search problem, so the first game after a known stretch cannot repeat a known game's
search, and searches a problem that the chain would have settled by a repeat; games 3441 and 4300 on are where that
could cost.

## Decision

Kept, all of it: the ledger with the climbs (both ways, a limit of 60 s), the finder (1 s a call), the floor steps (150
points, a slack of 4) and the order by the forecast of the proofs; the bounds store, checked by two referees that share
no code; and `--use-prev`'s draw on the known games.  The ledger's proofs are the frame model's searches and relations;
what it adds is the order, and the bounds it keeps for the boards it cannot prove.  `ledger run --bounds` and `solve
--use-prev` take the store's upper bounds on its word, so `ledger verify-bounds` comes first.

Open, and what would reopen it:

* The climbs' effect on 2176..3440 in the cloud, where the dry runs saw them bring proofs forward and prove no more;
  other values of `--base`, `--gap-unit`, `--climb-limit`, `--floor-step` and `--floor-slack` (the simulation on the 14
  games says a step of 100 to 200 points costs 1.01 to 1.14 times the proof alone, but the dry runs used 150 and 4); the
  release build; a finder that scales its seconds with the board (the start incumbent alone costs 1 to 7 s there).
* Floor steps on boards past 4500, where the gaps are 1,200 and more and the median line is off, on many workers on a
  stretch of walls (coupled boards end each other's steps: 11% of the step seconds on 3076..3200), and on many threads;
  a bias correction of the forecast, which is a factor 1.7 to 3.5 too small at the median.
* The check of a `dual` or `exhausted` bound against its checkpoint, which `--checkpoints` records and nothing reads;
  citing a repeat of a game that had no promotions (7 of 72 games in the dry run), which needs the frame model on both
  sides.

## Files

The program is `examples/ledger/` (its module comment defines every field of `ledger.jsonl` and `events.jsonl`), run
from the top of the repository with the non-release build of the laptop (`cargo build --profile test --example ledger
--bin taxman` in `solver/frame`), so `ledger` is `solver/frame/target/debug/examples/ledger` and `taxman`
`solver/frame/target/debug/taxman`.  Everything here ran under `nice -n 10`.  The launcher `cloud.sh` and the reader of
the events, `events.py`, are in [solver/tools/ledger/](../../../tools/ledger/README.md).  Each run directory has
`report.txt` (the run's own), `log`, `run.err` (its status lines), `events.txt` (`./events.py` on `run`) and
`run/ledger.jsonl.gz` and `run/events.jsonl.gz` unless a row says otherwise.

| file | command | what it is |
|---|---|---|
| `dry/` | `OUT=DIR LEDGER_BIN=target/debug/examples/ledger WORKERS=6 CPU_HOURS=2 solver/tools/ledger/cloud.sh 1301-1500` (2026-10-17, 21:11:44 to 21:13:57 UTC, after the fix) | the first dry run of 1301..1500, the truth hidden, with `events.txt` made by `./events.py dry/run dry/chain.jsonl`; the run before the fix (595 CPU s, 145 s) has its files in d7390e1 |
| `dry/chain.jsonl.gz` | `taxman solve 1301-1500 --threads 1 --window 1 --progress 0 --out dry/chain.jsonl`, then gzipped | the chain over the same stretch on one thread, standalone at 1301, for the comparison: 447.5 s |
| `climb/` | back to back (2026-10-17 and 18): `... WORKERS=6 CPU_HOURS=2 [CLIMB=up\|down\|both\|off] solver/tools/ledger/cloud.sh 1301-1500` (`dry-*`), `WORKERS=8 CPU_HOURS=0.1667 ... 2576-2700` (`mid-*`), `CPU_HOURS=0.5` (`long-*`); `*-up` the build of eb9dc9c, `*-off` with `CLIMB_LIMIT=0`, `*-both` and `*-down` the build of 55d2b2a | wedge and climb |
| `finder/` | back to back (2026-10-18, 01:01 to 01:15 UTC), the build of 4621d97: `... WORKERS=6 CPU_HOURS=2 LNS_SECONDS=1 ... 1301-1500` (`dry-on`; `dry-off` with `LNS_SECONDS=0`), `WORKERS=8 CPU_HOURS=0.25 LNS_SECONDS=1\|0 ... 2576-2700` (`mid-on-a`, `mid-off-a`, `mid-off-b`, `mid-on-b`, in this order) | the finder in the ledger |
| `store/` | the laptop (2026-10-17, evening): `ledger run 901-960 --dir run1 --cpu-hours 0.0015 --workers 4 --lns-seconds 0.1 --base 0.05`, `ledger export run1 --bounds bounds.jsonl --source dry-1 --checkpoints s3://bucket/ledger/out/run/ckpt --strict S` (S: the chain's lines of 900, 930 and 945 only), then `ledger run 901-960 --dir run2 --cpu-hours 0.01 ... --bounds bounds.jsonl` and `ledger export run2 --bounds bounds.jsonl --source dry-2 --strict S --date 2026-10-18` | the bounds store: `bounds.jsonl` (171 lines), `verify.txt` (`ledger verify-bounds bounds.jsonl --strict results/strict.jsonl`) and `check.txt` (`python3 -I solver/tools/check_bounds.py bounds.jsonl --games G --strict results/strict.jsonl`, G the output of `ledger games`) |
| `floor/` | the laptop (2026-10-18), each `OUT=DIR LEDGER_BIN=BIN WORKERS=W CPU_HOURS=H nice -n 10 solver/tools/ledger/cloud.sh RANGE`: `*-dry` W=6 H=2 on 1301-1500, `*-mid` W=8 H=0.25 on 2576-2700, `*-hard` W=8 H=0.25 on 3076-3200, `*-mid2` W=8 H=0.5 on 2576-2700, `*-scale` W=4 H=0.15 `LNS_SECONDS=1` on 4500-4523, `first-scale1` W=1 H=0.05 on 4500-4500; `base-*`, `first-*` and `new-*` are the three builds | floor-guided drilling; `ladder.py` and `ladder.txt` (`./floor/ladder.py > floor/ladder.txt`) lay the steps on the 14 games measured under floors (`../proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/target.jsonl`) |
| `climb_limit.py`, `climb_limit.txt` | `./climb_limit.py cloud-20261017/run` | what a longer climb limit would cost, from the second cloud run's events: the table the default of 60 s rests on |
| `cloud-20261017-stopped/` | `solver/tools/ledger/cloud.sh 2176-3440` on a `c8g.16xlarge` with 64 workers (2026-10-17, 20:31:21 to 20:40:21 UTC, stopped by a signal) | the first cloud attempt, before the fix (no `run/`; the 21 MB events file and the 8 MB ledger are not kept) |
| `cloud-20261017/` | `WORKERS=64 CPU_HOURS=48 solver/tools/ledger/cloud.sh 2176-3440` on a `c8g.16xlarge` (2026-10-17, 22:04:55 to 22:50:11 UTC): the truth hidden, nothing given, no climbs, no finder | the second cloud run: 916 of the 1,265 boards proven, every one at the optimum, 0 violations, 48.02 core-hours; `run/events.jsonl.gz` (which `climb_limit.py` reads); `results.html`, the page of the run, made by `report_page.py RUN_DIR results.html` from the run's `events.jsonl` and `ledger.jsonl` (the 8.4 MB ledger is not kept, so the page is the record) |
| `cloud-20261018-prep/` | `cloud.sh 2176-3440` as two ledgers at once on one machine, each `WORKERS=32 CPU_HOURS=24`, the games to 2175 given, the climbs both ways (2026-10-18, 01:30:04 to 02:15:31 UTC): `a/` with the finder, `b/` without (`LNS_SECONDS=0`) | the prep run: 877 and 838 boards proven, all at the optimum, 0 violations; `compare.html`, made by `./compare_page.py compare.html` from the events of the three cloud runs: the proofs against core-hours, the climbs and the finder, stretch by stretch |
| `past4300/` | `./past4300/past_chart.py LEDGER OUT.html` and `./past4300/where_chart.py LEDGER ../../results/strict.jsonl OUT.html`, with LEDGER a `ledger.jsonl` | the survey of the boards past 3900 (2026-10-18), two pages with their scripts.  `bounds-stage2-midway.html`: every board of 4300..4999, where its bounds started (the quick bounds of `../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/map.jsonl.gz`) and where they stood midway through stage 2 (304 of 700 proven; stage 2's final ledger has 343).  `where-3899-5000.html`: games 3899..5000 proven or open, the gap of each open game and two forecasts of the search left for each (the floor-step line, and the chain's model, log10 core-seconds = 1.89 + gap / 553), from the survey's ledger (3900..4999, floor steps).  The ledgers (8-12 MB) are not kept: the bounds are in `results/bounds.jsonl` |

## Commits

* 4466311 (2026-10-17): the dual bound of a stopped search.
* 83f6405 (2026-10-17): the ledger and its relations; d7390e1, its cloud script and the first dry run's files.
* c9af16a (2026-10-17): the fix of the first cloud attempt.
* eb9dc9c, 55d2b2a (2026-10-17): the climbs up, then the climbs down and the ways.
* 4621d97 (2026-10-17): the finder in the ledger; b3ed8ac, the merge that stage 1 ran on.
* 14f36d6 (2026-10-17): the bounds store, its commands and the Python referee; e93d035, `ledger export` on a thread with
  a big stack, replaced by 3e88fe1, which follows citations with a worklist; afe4513, the merge of the store, the build
  of stage 2; 0883903 (2026-10-18), the first lines of `results/bounds.jsonl`.
* f775cc0, 26c5c71 (2026-10-18): the floor steps, the forecast and the order, then the boards exploring first and the
  checkpoint of a step that found a set; 6088c21, a climb's source is the board just outside the class; 93056fd, the
  scheduler before them.
* 92c1e44 (2026-10-18): the merge of floor-guided drilling, the build of stage 3; ca357bf, stage 3's bounds of 3900..4999
  in `results/bounds.jsonl`.
* 8ce120a, 113f468 (2026-10-18): the store's format moved into the library, then `--use-prev` with the known games,
  their referees and the tests.
