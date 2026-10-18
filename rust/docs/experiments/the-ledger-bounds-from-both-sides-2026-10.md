# The ledger: bounds from both sides (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

**Question.**  Brian's idea (2026-10-17): instead of proving opt(n) one game at a time from a proven opt(n − 1), keep
for every n of a stretch a lower bound L(n), the score of a game held, and an upper bound U(n), tighten both across
many games at once, and spend the effort where progress is cheapest; a game is proven when L(n) = U(n).  Does it prove
much of a stretch for a small share of what the chain spends there?  This entry builds it and runs it on the laptop on
1301..1500 with the truth hidden; the experiment of the design, 2176..3440 for an hour of a `c8g.16xlarge`, is ready
and not run ([RUNBOOK section 14](../../tools/cloud/RUNBOOK.md#14-the-ledgers-experiment)).

**What was built.**

* **The dual bound of a stopped search** (`search::dual_bound`, `src/search/dual.rs`): the largest of the incumbent and,
  over the open units of the final checkpoint, each node's promotion sum plus the GF(2) bound the search computes when
  it starts the unit (`search::NodeBound`, which `examples/tune.rs` now reads its units' bounds through).  Valid by the
  search's exactness and Theorem 11 (heuristics/THEORY.md, Lemma D); until now a stopped search threw it away.
* **The relations, proven** (THEORY.md, "The ledger: bounds between boards"): L1 monotone, opt(n − 1) ≤ opt(n), with
  the game of n − 1 followed by n if still legal; L2 the ceiling, opt(n) ≤ opt(n − 1) + n, with the game below it (the
  game of n without the move n stays legal, a pot-inclusion argument, then completed); L3 the prime formula both ways,
  opt(p) = opt(p − 1) + p − q for q the largest prime below p, the games by exchanging the opening q and p; L4 repeats,
  U(n) − bag(n) ≤ U(m) − bag(m) for equal search problems, and m's promotions played on n's board.  Two corrections to
  the design: the prime formula is + p − q, not + p − 1; and none of the constructions needs an optimal game, only a
  finished one (legal, over, opening with the largest prime), which every game the ledger holds is.  What the
  constructions do need: a repeat carries a lower bound only as the promotions of a game made from them (any other
  game's candidate moves are tried and kept only if the referee accepts them), and its upper half rests on statement U
  at the board it is carried to, as everything from the frame model does.
* **The ledger** (`examples/ledger/`: `book.rs` the bounds and relations, `schedule.rs` the scheduler, `report.rs` the
  report and check; the module comment of `main.rs` has the commands and the files).  A Rust example rather than a
  Python driver of the binaries: the searches need what no command gives, a seed from any game and the ledger's U as
  their ceiling, the dual bound read from each checkpoint, and the relations need the boards' problems and candidates;
  one process with a thread a search shares them and the build.  Every board searched first gets its map (a search
  stopped before its first node: the start incumbent and the root bound, as `wall landscape`); then searches of one
  thread, `--base` seconds the first time and twice as long each time after, each going on from the board's checkpoint,
  seeded with the best of the games held for n − 1, n and n + 1, with U(n) as the ceiling; only one board of a repeat
  class is searched, and no prime whose board before it is in the ledger.  The order: boards whose bounds a proven
  neighbour has changed (the ceiling and the prime formula next to a proven board are exact), smallest gap first; then
  least `level + gap / 175` (a decade of the chain's steps per about 580 of standalone gap,
  [the landscape](the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10.md)).  After each search the
  relations run to a fixpoint; every game they make is replayed, L > U stops the run; a proven board's search is stopped
  and its checkpoint deleted, a falling U is given to a running search as its ceiling.  The ledger file and the event
  log let a run go on after an interruption, and the CPU budget counts all runs.
* **The cloud script** `bench/ledger/cloud.sh` (the run with no proven game, then `ledger report --known
  results/strict.jsonl`, synced to S3), with RUNBOOK section 14's user data.  For 2176..3440 at an hour of 64 CPUs:
  about 1.3 instance-hours with the build, **about $0.9 at spot**, $3.3 on demand.

**Tests (the quick suite).**  The dual bound: a search stopped before its first node holds the root unit alone and its
dual bound is the root's GF(2) bound (14 boards of 60..1300); one-thread searches of 996, 1020 and 1218 stopped at
2,047, 3,071, 5,119 and 9,215 steps (the engine checks its limit every 1,024) have dual bounds between the optimum and
the root bound, never rising, and the completed search's is the optimum.  The bound falls slowly: of the root gaps
(318, 223 and 146 over the optimum) it had closed 23%, 61% and 29% at 9,215 steps, 65%, 96% and 88% of the way through
their trees (14,200, 9,554 and 10,481 steps).  A depth-first search keeps the shallowest, largest subtrees open until
late, and their bounds are the root's or near it.  The relations: proven games at every tenth board of 2..1300 give
every board both bounds, all on the right side of optimal.json and every game legal, each relation used for both
bounds, exact at the primes and repeats of the proven boards; loose bounds on 1001..1300 with a proven game every
fiftieth stay on the right side; a lower bound above an upper one is refused, offered or derived.  The scheduler: 901..960
from millisecond searches is proven at the optimum, by stopped and completed searches, primes and repeats, with no
checkpoint left, and a second run on the same directory finds nothing to do and keeps every bound; given proven games
are fixed points.

**The dry run** (`bench/ledger/`: data, commands and the script of the numbers).  1301..1500 on the laptop, the
non-release build, six workers, a budget of 2 CPU-hours, no proven game (optimal.json and `results/strict.jsonl` only
for the check afterwards), through `cloud.sh`.  It ended with every board proven after **595 CPU seconds** (145 s of
wall time), and the check found **0 violations**: every game replays to its L, L <= opt <= U at all 200 boards, and all
200 proven values are the committed chain's.

| CPU seconds | 60 | 119 | 179 | 238 | 298 | 357 | 417 | 476 | 536 | 595 |
|---|---|---|---|---|---|---|---|---|---|---|
| boards proven | 40 | 90 | 112 | 127 | 142 | 164 | 182 | 189 | 195 | 200 |

* How (the rules of the upper bound, then of the lower): by searches 83, all exhausted (69 with the search's own game,
  12 with the start incumbent, 2 with a game from a relation), none by a dual bound alone; by relations 117: repeats 79,
  the ceiling 21 (14 with the game below a proven board carried up, 5 with a search's game at its ceiling, 2 by a
  repeat's game), the prime formula 17.
* Where the time went: the boards' facts and problems 1.8 s; 89 maps, 20.1 s; 182 searches of the 89 boards searched,
  565.6 s; 87 completed (3 at their ceiling), 95 stopped by their limit, a stopped search costing a median 0.14 s beyond
  its limit (setup, start incumbent, dual bound).  111 boards were never searched (primes and repeats), 36 once, 26
  twice, up to 6 times (1475).
* The dual bound lowered U in 51 of the 95 stopped searches, by 3,833 in all (a median 74), and was a median 96 below
  the board's root bound (at most 297); it proved nothing alone.
* **Against the chain.**  The chain over the same stretch on one thread with the same build, standalone at 1301 (no
  game of 1300), took **447.5 s**: 89 searches (80 exhausted, 9 at the ceiling), 27 primes, 84 repeats.  The ledger
  spent 1.33 times that CPU, in a third of the wall time on six workers.  It searched the same 89 boards, and the
  dearest boards cost it what they cost the chain, each by an exhausted search of its own (1475 69.7 s against 51.4,
  1472 34.6 against 36.7, 1476 34.2 against 24.4, 1430 28.6 against 28.3).  The extra third: 35 s on the 9 boards the
  chain settled at its ceiling with no search, searched here before the board below them was proven; 84 s more, net, on
  the boards both searched to the end, searched here in pieces and at first without the seed and ceiling of a proven
  neighbour; and the 20 s of maps.  (The committed
  chain's one-thread re-solve of 1301..2175, with the release build of ee03872, spent 461 s of search on 1301..1500,
  `bench/wall/resolved_1301_2175.jsonl`.)
* A first dry run of the same stretch, before the map was forced, made only 6 maps: the relations gave every board both
  bounds (a ceiling carried up, a game carried across) before its map came due, and the scheduler took a board with
  bounds as mapped.  It also proved every board at the optimum, in 590 CPU seconds; each board searched now gets its own
  map first (`mapped`), as the design says, so that its order starts from its own bounds.

**Findings.**

* The ledger is correct on the stretch it was tried on: every bound brackets the optimum and every proven value is the
  chain's, with the truth hidden from the run.
* On an easy stretch it is the chain run out of order on many threads: the same searches prove the hard boards, the
  relations prove the rest (117 of 200 boards here, as the chain's repeats and primes do), and the out-of-order work
  costs a third more CPU.  The dual bound of a depth-first search helps the order (it lowers U), but at these limits
  it closes no board by itself; that would need a search that closes its shallow subtrees first.
* What it can do on 2176..3440, where the chain spent about 280 core-hours and its dearest games hours of 64 threads,
  is the open question of the cloud experiment: the boards whose own searches fit in the budget are proven, and each
  proof brings its repeats and primes (55% of the chain's games there) and the exact ceiling of the next.

**Not measured.**  The cloud experiment (2176..3440, an hour of 64 CPUs: not launched; it needs Brian's approval); a
stretch where searches do not finish; other values of `--base` and `--gap-unit`; the release build.

**Commits.**  4466311 (the dual bound), 83f6405 (the ledger and its relations), then this record.

## Later (2026-10-17): the first cloud attempt, stopped, and the fix

**The attempt.**  `bench/ledger/cloud.sh 2176-3440` on a `c8g.16xlarge` with 64 workers (20:31 UTC), stopped by a signal
after 9 minutes at 1,299 CPU s (0.36 core-hours, about $0.18).  The machine was 95% idle (the run used 2.3 of its 64
CPUs), the ledger's main thread at 100% and the workers waiting on it: 390 of the 1,265 boards had their map, no search
had finished, no board was proven ([the files](../../bench/ledger/cloud-20261017-stopped/); the 21 MB events file and
the 8 MB ledger are not kept).

**The cause, measured.**  The first map was in at 1.45 s, and the relations of that one result ran until 59.1 s: 2,863
changes, a cascade through the boards, a game built and replayed at each step.  The 390 maps made 161,615 changes (the
upper bound: by the ceiling 81,861, by repeats 55,543, by primes 12,001; the lower: by monotone 6,614, repeats 2,995, the
ceiling 1,249, primes 1,163), and the relations took 528 of the 540 seconds (a median 0.17 s after a map, at most 58 s).
The maps came one at a time, and every cascade ran on through boards that had no map of their own yet, so that each
map's cascade made their bounds over again (a board's lower bound improved a median 9 times, up to 21), the changes
taken in the order of a stack.  Each game made was dear.  The same on the laptop, 2176..2400 (8 workers, the non-release
build, 360 CPU s), with the main thread timed: **17.5 s of its 53 s** applying 153 results, 16.3 s of that in 22,545
steps of the relations, 3,729 of which made a game: the referee 8.5 s (2.3 ms a game: two plays of it, each a division
by every number below every move), repeat games 4.8 s (7 ms: the frame model's setup), the monotone and ceiling games
3.0 s (the ceiling's replays the whole game after each move it adds), and 1.2 s for 6,977 events written a piece at a
time.  The 101 maps took 31 s there; on 2176..2700 the 235 maps took 91 s for 443 s of work (55 s on eight workers), no
search finished before 91.8 s, the workers used 72% of their CPU, and the budget of 540 CPU s went to 15 searches.  What
grows is boards times maps.

**The fix** (`examples/ledger/`, `game.rs`; every bound is the bound it was, each lower one a game the referee replays):

* The maps are made first, all of them on the workers, their bounds kept as they come; when the last is in the
  relations carry them once to every board (`settle`), and then the searches start.
* A game is made only if it can beat the target's lower bound: the prime exchange scores exactly p − q more or less than
  the source's game, a repeat game its bag and the promotions, the game of n − 1 followed by n at most n more.  The
  monotone and ceiling games are made (cheap now) and shown to the referee only if the sum of their moves is more than
  the target has.  The upper bounds are the arithmetic they were.
* The games are completed in one pass (`game::complete`: the largest legal move again and again, found going down the
  board once, as the pot only shrinks), not by a replay after each move.
* The referee finds a move's divisors up to its square root, and `replay_finished` plays a game once, not twice:
  4.66 ms to 0.13 ms at n = 3400, 2.18 to 0.07 at 2300, 0.43 to 0.02 at 1000.
* The changes go in sweeps up the boards (a chain up in one sweep, a chain down in the next), the events are buffered and
  written after each result, a result's changes carry one stamp, and the status line and the `end` event say how long
  the main thread was busy and how long results waited for it.

**After** (the laptop, 8 workers, non-release; CPU used is of the 8 workers over the whole run):

| stretch and budget | wall | maps in at | CPU used | main thread busy | searches |
|---|---|---|---|---|---|
| 2176..2400, 360 CPU s, before | 52 s | 31 s | 87% | 17.5 s | 53 |
| after | 48 s | 24 s | 95% | 0.8 s (the settle 0.65 s) | 61 |
| 2176..2700, 540 CPU s, before | 96 s | 91 s | 72% | at least 52 s | 15 |
| after | 71 s | 58 s | 96% | 1.8 s (the settle 1.7 s) | 33 |
| 2176..3440, 2,520 CPU s, after | 329 s | 230 s | 96% | 6.3 s (the settle 5.4 s) | 209 |

* From the end of the settle to the end the workers used 96% (2176..2400), 96% (2176..2700) and 98% (2176..3440) of
  their CPU, and the 776 results of the last run cost the main thread 0.8 s in all, about a millisecond each; results
  waited 0.05 s in all.  The map phase used 96 to 98% of the CPUs.  The settle, 5.4 s for 1,767 changes on the whole
  stretch (the cascade of the first map alone took 58 s in the cloud), is the one pause: most of it, probably, the
  about 550 repeat games at 7 to 12 ms.
* The dry run again (1301..1500, six workers, the truth hidden, [bench/ledger/dry](../../bench/ledger/dry/), whose
  files are now this run's; the first run's are in d7390e1): all 200 boards proven, **0 violations**, **573 CPU s in
  133 s** (595 and 145 s before), the main thread busy 0.7 s.  The workers used 97% of their CPU in the first minute;
  the stretch ends with the long searches of a few boards (1475 alone), so the whole run's 4.3 of 6 CPUs (4.1 before) is
  the stretch's tail, not the main thread's.
* Tests: the referee's divisors against the sweep over every number below, and the whole fast referee against the
  slow one on the table's games and on those games with moves swapped, dropped, repeated and replaced; `complete`
  against the slow loop; the games the relations make from the table's games score what the lemmas say, at every n to
  1299; a run's events show the maps, one settle, then the searches.  One expectation changed: a wrong upper bound of
  1000 one below what the relations derive is found at 1000 or at the board the sweep reaches first.

**Not done.**  The cloud run is not repeated (it needs Brian's approval).  The repeat games are still made on the main
thread: a search that improves the game of a class of eight makes seven of them, about 70 ms.

**Commits.**  c9af16a (the fix and its tests), then this record.

## Later (2026-10-17): wedge and climb

**Why.**  The second cloud run (2176..3440, the truth hidden, 48 core-hours on 64 workers) proved 916 of 1,265 boards,
every one at the optimum, but paid about 2.2 times what the chain paid on the same games, because it searched most
boards before the board below them was proven.  Of the 356 boards proven by their own search only 103 had n − 1 proven
when their final search began, and of the 20.8 core-hours spent on those boards 11.4 went into searches made before
n − 1 was proven (it was proven later).  The searches that began with n − 1 proven used 6.7 core-hours and completed 106
times; those that began without used 40.8 and completed 265 times.  A proven board below gives the search of n the
ceiling (opt(n) ≤ opt(n − 1) + n, exact in about a third of the boards) and a start that is the optimum about 88% of the
time, so those searches are far cheaper.  The order of the ledger (smallest `level + gap / gap_unit`, boosted boards
first) finds *wedges*, boards proven with no help from a neighbour; the change adds climbing from them.  The same day
the lead asked for the other direction too, for the next cloud run (whose purpose is to learn what sets up a run past
3440 best), so that the run says what each direction costs and proves.

**The change** (`examples/ledger/schedule.rs`; `book.rs` has one field, `parked`; nothing in `src/`):

* **Climb up.**  A board that is searched (not slaved by the prime formula, not a repeat of a smaller board), not
  proven, not on its way, not parked, and has its board below proven gets a *climb* ahead of every other job but the
  maps: its ordinary search (resumed from its checkpoint if it has one, seeded with the best of the games held for
  n − 1, n and n + 1, so with the proven game of n − 1, and capped by U(n), the ceiling the relations derive) for
  `--climb-limit` seconds.  "However the board below was proven" is the rule, which makes the climb skip the boards the
  relations settle: in 901..960 with the games of 900 and 906 given, the relations prove 901 and 902 (repeats of 900) and
  904-908 (906, its repeats, and the prime 907), and the climbs go to 903 and 909.  When a climb proves its board the
  board above it is climbable at once, and so on up the stretch; climbs from different wedges run side by side.  The
  candidates are taken smallest gap first, then smallest n.  A climb leaves the board's `level` (and so the limit of
  its next ordinary search) as it was; it counts in `attempts`, `search_s` and `nodes`.
* **Climb down.**  The same for a board with its board *above* proven: the first unproven, searched board below a proven
  one.  Its search is seeded with the game above (the game that Lemma L2 makes for the board below, the game without the
  move n, completed, is exactly the optimum for n − 1 in about 35% of the composite pairs of 2176..3440, by the lead's
  count, and the relations have put it in the ledger already), and its ceiling is the ledger's, which is loose (U(n − 1) ≤ opt(n)); the
  same limit, and a proof brings the climb to the board below.  The climbs up go first when both wait; a board next to
  a proven one on both sides is climbed to up.  `--climb up|down|both|off` picks the ways (default both;
  `CLIMB=` in `bench/ledger/cloud.sh`), and `--climb-limit 0` turns every climb off.
* **Park.**  A climb that ends with the board unproven (it stopped at its limit) *parks* the board: `parked` in its
  ledger line (so a resumed run does not climb to it again, either way), and it goes by the ordinary order with its
  checkpoint.  The climb from that wedge ends there; the board next to it is climbed to when that board is proven, by
  the ordinary order or by a relation.  A climb that ended after a signal parks its board too (the board keeps its
  checkpoint and goes by the ordinary order after the resume).
* **Hold.**  No ordinary search starts on a board next to a board that is being climbed to: that board may be proven
  within the limit, and then a climb to this one starts with its game (and, going up, its exact ceiling).  It waits at
  most the climb's limit, and only the board right next to the one climbed to (a climb to m holds m − 1 and m + 1; one of
  them is proven, the one it goes from): a board whose neighbour is not being climbed to is not held, though a climb may
  soon settle that neighbour too (a repeat or a prime of the board climbed to).
* **Events and counts.**  The search events have `job`, `ordinary` or `climb`, and a climb its `way` (`up` or `down`) and
  its `outcome`: `proved` (its search proved the board), `parked`, or `moot` (a relation proved the board while it went
  on); the `start` event has `climb_limit_s`, `climb_up` and `climb_down`; the status line says, for each way, how many
  climbs have ended, how many proved their board and how many parked; `ledger report` has a line for each way (how
  many, their seconds, the outcomes).

**The default, 60 s.**  From the second cloud run's events ([climb_limit.txt](../../bench/ledger/climb_limit.txt), made
by `bench/ledger/climb_limit.py`): 565 boards were searched, 371 finished by a search, 194 not.  A climb of a given
limit on every one of them, costing a board its total search seconds in the run (the limit if that is less or if the
board was not finished), would have made

| limit (s) | 10 | 20 | 30 | 45 | 60 | 90 | 120 | 180 | 300 | 600 |
|---|---|---|---|---|---|---|---|---|---|---|
| proofs | 34 | 98 | 129 | 168 | 190 | 225 | 248 | 269 | 299 | 331 |
| core-hours | 1.5 | 2.9 | 4.1 | 5.9 | 7.5 | 10.5 | 13.2 | 18.3 | 27.6 | 48.4 |
| core-s a proof added since the limit before | | 76 | 146 | 162 | 263 | 309 | 421 | 870 | 1,126 | 2,338 |

The run's own proofs by an exhausted search cost 109 core-s each in its first 8 core-hours, then 929, 1,920, 1,800,
1,516 and 2,618 in each next 8.  A longer climb is worth its price while it adds proofs for less than the ordinary
order's late ones, which would put the limit at 120 to 180 s; 60 s is chosen because (1) the table is an upper bound on
a climb's cost, but it counts every board once, and a board whose climb parks has had up to the whole limit for nothing
but the checkpoint: the worst case at 60 s is 565 boards × 60 s = 9.4 core-hours, at 180 s 28; (2) at 60 s the price of
a proof added is still under 300 core-s, and past 90 s it rises steeply; (3) a parked board loses nothing but its turn,
its work being in its checkpoint, which the ordinary order resumes.  It is an option: `--climb-limit`.

**Tests (the quick suite).**  `next_job` and `finish` on a stretch of 901..960 with every board mapped: with the games
of 900 and 906 given, the boards climbed to are 903 and 909 (not 901 or 908), each ahead of the ordinary order (a
boosted board, and the cheapest board), seeded with the proven game below and capped by at most the ceiling, the two
climbs side by side; with both on their way the ordinary job is the boosted board and then the cheapest, not 910, the
cheapest of all, whose board below is climbed to; when the climb proves 909 the next job is the climb to 910 with 909's
game.  A climb that stops parks its board (not proven, `parked`, `attempts` 1 and `level` 0, kept by the ledger's JSON);
the board is then the next ordinary search with the base limit, and with the climbs off (`climb_limit` 0 or neither
way) no job is a climb.  The climbs down: with the game of 906 given the climb up (909) goes first and the climb down
(903) second, seeded with the proven game above (904) and capped by the ledger's bound; a climb down that stops parks
its board and its event says `"way":"down"`; with only the climbs down they go first, with only the climbs up there is
none down; a board below a board climbed down to has no ordinary search meanwhile (924 below 925), and has one again
when the climb is gone.  A run of 901..960 given the game of 906, with climbs of 10 ms: every board proven at the
optimum, each board climbed to once, a board parked in the ledger exactly when its climb says `parked`, a climb down
to 903 and climbs up, the report counting each way, and with one way only, neither, or `climb_limit` 0, only the climbs
asked for.

**The dry runs** (the laptop, the non-release build, the truth hidden, through `cloud.sh`, the files in
[bench/ledger/climb/](../../bench/ledger/climb/)).  The runs of one stretch were made back to back: first with the climbs
up only (`up`: the build of eb9dc9c, before the climbs down were added) and without (`off`), then, with the build that
has both ways (55d2b2a), with both and with the climbs down only.  Wall times are not clean: the searches of the
2576..2700 runs took a fifth more wall seconds than they used CPU, so the laptop had other work, and the 1301..1500 run
without climbs took 598 CPU s and 151 s where the one before the change had taken 573 and 133.

| run | proven | check | CPU s | wall s | searches (s) | climbs up | climbs down |
|---|---|---|---|---|---|---|---|
| 1301..1500, 6 workers, off | 200 of 200 | 0 violations | 598 | 151 | 179 (576) | | |
| up | 200 of 200 | 0 violations | **547** | 157 | 129 (525) | 50 proved, 363 s | |
| both | 200 of 200 | 0 violations | 565 | 138 | 130 (543) | 50 proved, 323 s | 8 proved, 36 s |
| down | 200 of 200 | 0 violations | 576 | 143 | 178 (654) | | 12 proved, 83 s |
| 2576..2700, 8 workers, 600 CPU s, off | 46 of 125 | 0 violations | 606 | 95 | 128 (587) | | |
| up | 49 | 0 violations | 609 | 96 | 114 (586) | 15: 12 proved, 3 parked, 174 s | |
| both | 56 | 0 violations | 608 | 78 | 103 (483) | 12: 10 proved, 2 parked, 140 s | 1 proved, 3 s |
| down | 51 | 0 violations | 607 | 78 | 115 (482) | | 1 proved, 3 s |
| 2576..2700, 1800 CPU s, off | 107 | 0 violations | 1805 | 285 | 228 (2070) | | |
| up | 108 | 0 violations | 1806 | 287 | 213 (2077) | 24: 21 proved, 3 parked, 406 s | |
| both | 108 | 0 violations | 1807 | 235 | 198 (1682) | 22: 19 proved, 3 parked, 362 s | 1 proved, 3 s |
| down | 111 | 0 violations | 1807 | 234 | 215 (1682) | | 1 proved, 2 s |

* **1301..1500**: every board proven at the optimum in all four runs, 0 violations.  The climbs cost less CPU than none:
  547 (up), 565 (both) and 576 (down) against 598 (and the 573 of the last dry run before them), 1.22 to 1.29 times the
  one-thread chain's 447.5 s (1.33 before), with 50 boards proven by climbs up (a median 4 s, none past 52 s of the 60),
  8 by climbs down in 36 s in the run with both and 12 in 83 s with down only; searches fell from 179 to about 130 (to
  178 with only the climbs down, which prove 12 boards).  No climb was parked.  The wall time did not fall with the
  climbs up alone (157 s against 151): of the 6 workers 3.5 were busy against 4.0, because the stretch ends with the
  long boards 1475 and 1476, which without climbs are searched side by side and with them one after the other (1475's
  climb of 52 s, then 1476's of 28 s: the hold, and then the climb); with both it was 138 s.  The hold costs wall time
  when little else is left to search and no CPU; on a stretch of 1,265 boards it is not met until the end.
* **The climbs down are few and cheap**: 8 and 12 climbs on 1301..1500, all proving their board, 4.5 and 7 s each, and
  one on 2576..2700.  Presumably (not counted) the board below a wedge is mostly a member of a repeat class that the
  relations settle with it, so that a climb down has somewhere to go only where a class is a single board.
* **2576..2700** (a valley: the chain spent minutes a game there; the maps take 150 s of the first 180 CPU s): at
  600 CPU s 49 boards are proven with the climbs up, 56 with both, 51 with down only, against 46 with none; at 1800 CPU
  s 108, 108 and 111 against 107.  These differences are the noise of the runs and not the climbs: the run with the
  climbs down only made one climb of 3 s in each budget and has 5 and 4 boards more than the run without any, because
  the searches are limited by wall time on a loaded laptop and a proof comes in lumps (one search proves a whole
  repeat class).  What can be read is the up-only curves of boards proven against CPU, which lie above the run
  without climbs in the middle (at 722 CPU s 56 against 51, at 1084 s 70 against 61) and meet at the end: the climbs
  bring proofs forward and prove no more by 1800 CPU s.
* **What it says about the cloud run:** the climbs cost nothing measurable where they were tried and saved CPU where
  the stretch was finished; neither stretch is the cloud run's situation (1,265 boards, 64 workers, searches that
  had begun before their board below was proven), where the table above is the evidence, so the effect there is not
  measured; and what the climbs down prove there, and at what price against the climbs up, is what that run is for (the
  `way` of each climb event and the two lines of `ledger report`).

**Not done.**  The cloud run with the climbs (it needs Brian's approval).  Not measured: other values of
`--climb-limit`; the hold relaxed to wait only while there is no other board to search (the tail of 1301..1500 would
be shorter).

**Commits.**  eb9dc9c (the climbs up and their tests), 55d2b2a (the climbs down, the ways), then this record.

