# The ledger: bounds from both sides (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  Brian's idea (2026-10-17): instead of proving opt(n) one game at a time from a proven opt(n − 1), keep
for every n of a stretch a lower bound L(n), the score of a game held, and an upper bound U(n), tighten both across
many games at once, and spend the effort where progress is cheapest; a game is proven when L(n) = U(n).  Does it prove
much of a stretch for a small share of what the chain spends there?  This entry builds it and runs it on the laptop on
1301..1500 with the truth hidden; the experiment of the design, 2176..3440 for an hour of a `c8g.16xlarge`, is ready
and not run ([RUNBOOK section 14](../../docs/RUNBOOK.md#14-the-ledgers-experiment)).

**What was built.**

* **The dual bound of a stopped search** (`search::dual_bound`, `src/search/dual.rs`): the largest of the incumbent and,
  over the open units of the final checkpoint, each node's promotion sum plus the GF(2) bound the search computes when
  it starts the unit (`search::NodeBound`, which `examples/tune.rs` now reads its units' bounds through).  Valid by the
  search's exactness and Theorem 11 (docs/THEORY.md, Lemma D); until now a stopped search threw it away.
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
  [the landscape](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md)).  After each search the
  relations run to a fixpoint; every game they make is replayed, L > U stops the run; a proven board's search is stopped
  and its checkpoint deleted, a falling U is given to a running search as its ceiling.  The ledger file and the event
  log let a run go on after an interruption, and the CPU budget counts all runs.
* **The cloud script** `solver/tools/ledger/cloud.sh` (the run with no proven game, then `ledger report --known
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

**The dry run** (`dry/`: data, commands and the script of the numbers).  1301..1500 on the laptop, the
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
  `solver/notebook/the-gap-past-2175-2026-10/resolved_1301_2175.jsonl`.)
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

**The attempt.**  `solver/tools/ledger/cloud.sh 2176-3440` on a `c8g.16xlarge` with 64 workers (20:31 UTC), stopped by a signal
after 9 minutes at 1,299 CPU s (0.36 core-hours, about $0.18).  The machine was 95% idle (the run used 2.3 of its 64
CPUs), the ledger's main thread at 100% and the workers waiting on it: 390 of the 1,265 boards had their map, no search
had finished, no board was proven ([cloud-20261017-stopped/](cloud-20261017-stopped/); the 21 MB events file and
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
* The dry run again (1301..1500, six workers, the truth hidden, [dry/](dry/), whose
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
  `CLIMB=` in `solver/tools/ledger/cloud.sh`), and `--climb-limit 0` turns every climb off.
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

**The default, 60 s.**  From the second cloud run's events ([climb_limit.txt](climb_limit.txt), made
by `solver/notebook/the-ledger-bounds-from-both-sides-2026-10/climb_limit.py`): 565 boards were searched, 371 finished by a search, 194 not.  A climb of a given
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
[climb/](climb/)).  The runs of one stretch were made back to back: first with the climbs
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

## Later (2026-10-17): the finder in the ledger

**Why.**  The finders experiment ([the entry](../finders-for-good-games-that-are-not-sound-2026-10/README.md)) found that large
neighbourhood search (LNS: keep a random 70-90% of the incumbent's promotions, search the rest exactly as a unit, repeat)
reaches the optimum of 64% of the games of 2176..3440 in 5 CPU seconds of one thread, against 21% for the sound search in
the same time, and that the sound search seeded with 1 second of its game costs 0.48 of the CPU, finder included (cheaper
on 36 of 40 games, at worst 1.16 times).  A lower bound needs only a legal game, so the ledger may take its L(n) and its
searches' seeds from LNS, if every game is replayed by the referee before it counts, as every lower bound already is.
Nothing LNS finds is a proof: the upper bounds are what they were.  The next cloud run is to learn what sets up a run past
3440 best, so the events carry what it needs to be digested.

**The change** (`examples/common/lns.rs`, `examples/ledger/`; one accessor in `src/`):

* LNS moved from `wall/finders.rs` to `common/lns.rs` (`Lns`, `Options`, `run_unit`), which `wall finders` and the ledger
  share; `wall finders` plays as before (its tests pass, the random choices are drawn in the same order).  Unsound code
  stays out of `src/`; the one line there, `Control::stopped`, tells the finder that its board was proven or the run was
  stopped.
* On the worker, for `--lns-seconds S` of one thread (default 1; 0 turns it off; `LNS_SECONDS` in `cloud.sh`): after a
  board's map from the start incumbent (unless that reaches the root's bound); and before a search whose best seed has
  changed since the finder last ran on the board, from that seed (the ledger keeps the score it left the board with,
  `lns_score`; a climb, whose seed is a proven neighbour's game, always counts as changed, an ordinary search that only
  doubles its limit on the same seed does not), unless the seed already reaches the ceiling.  The search is then seeded
  with the finder's game if that is better.
* The game goes through the same referee as a search's game (`refereed`: replayed, and its score must be the bag and the
  promotions' sum) on the worker, and is offered as the board's L with the rule `lns` only if it beats the game the finder
  began from; a search that began from it and found nothing better leaves the credit with the finder.  Its time counts in
  the CPU budget (the process's) and in the board's `search_s`; the search keeps its own limit, and `seconds` in the
  events is the search's alone.
* Events: the `init` and `search` events have `lns_s` (the finder's seconds, 0 if it did not run), `lns_from` and
  `lns_found` (the scores of its input and of its game, null if it did not run), `seed_from` (which game seeded the
  search: `n-1`, `n`, `n+1` (the one that keeps the most promotions, the last of equals) or `lns`; null for a map) and
  `below_proven` and `above_proven` (whether n − 1 and n + 1 were proven when the job started); the `start` event has
  `lns_s`.  `ledger report` has a `finder:` line (the runs, their seconds, how many found a better game and by how much in
  all, the boards whose final lower bound is its game and how many of those are proven), and the status line says how
  many runs and improvements; `solver/tools/ledger/events.py` adds the finder's line, the games that seeded the searches, and the
  searches by whether n − 1 and n + 1 were proven.

**Tests (the quick suite).**  A map is followed by the finder from the start incumbent and the game it returns replays at
its score (1218, whose start is 204 short of the optimum); a search whose best seed is another than the finder last ran
from runs it from that seed and is seeded with its game; the same seed again, a seed that reaches the ceiling, and no
seconds do not run it, and a climb does; the referee refuses promotions that do not play as the game they claim (all the
candidates at once); `finish` credits the lower bound to the finder where its game beats the search's or equals the one
the search began from, and to the search otherwise, keeps the score the finder left (and the ledger's file keeps it), and
the event has its fields; a run of 901..960 with the finder on proves every board at the optimum, every event has the
fields, the finder's scores lie between its input's and the optimum, and the report counts the runs; with the finder off
no event, board or report shows any, and the jobs the scheduler hands out are the same as with it on.  The earlier run
tests set `lns_seconds` to 0, which also keeps them as fast as they were.

**The dry runs** (the laptop, the non-release build of 4621d97 under `nice -n 10`, the truth hidden, through `cloud.sh`,
the files in [finder/](finder/)).  1301..1500 on six workers with the finder at 1 s
(`dry-on`) and, back to back, without it (`dry-off`, the same build and the climbs both ways); 2576..2700 on eight workers
with a budget of 900 CPU s, four runs in the order on, off, off, on.  The earlier dry runs of these stretches are in
[wedge and climb](#later-2026-10-17-wedge-and-climb) (another session, so another load on the laptop).

| run | proven | check | CPU s | wall s | searches (s) | finder runs (s) |
|---|---|---|---|---|---|---|
| 1301..1500, finder 1 s | 200 of 200 | 0 violations | 612 | 122 | 113 (438) | 142 (143) |
| finder off | 200 of 200 | 0 violations | 557 | 145 | 128 (535) | |
| 2576..2700, finder 1 s, `mid-on-a` | 61 of 125 | 0 violations | 908 | 116 | 132 (695) | 87 (88) |
| off, `mid-off-a` | 61 | 0 violations | 907 | 115 | 136 (783) | |
| off, `mid-off-b` | 61 | 0 violations | 910 | 116 | 139 (784) | |
| finder 1 s, `mid-on-b` | 61 | 0 violations | 909 | 116 | 136 (694) | 87 (88) |

* **1301..1500**: every board proven at the optimum, 0 violations, with the finder in **612 CPU s and 122 s** against 557
  and 145 without it in the same session (the earlier session's runs with the climbs took 547 to 576 CPU s and 138 to 157
  s, and 598 CPU s and 151 s without climbs).  The finder cost 143 s and the searches fell by 97 s (438 s against 535;
  128 searches fell to 113), the net being 10% more CPU and 16% less wall time: with the finder the workers were busy
  5.0 of 6 against 3.8, because the stretch's tail was shorter (the long boards 1475 and 1476 cost 66 s of search with
  the finder and 93 s without it, whose run ended with the climbs to them, 44 s and 19 s, one after the other).  Its
  share of the lower bounds: **51 of the 200 boards, 57% of the 89 that are searched, end with its game as their lower bound**, all 51
  proven (46 by an exhausted search, 5 at the ceiling); the other searched boards end with the search's game (26) or the
  start's (12); 70 of its 142 runs found a better game than they began from (by 10,599 in all, a median 132).  The 89
  runs after the maps were the ones that did it (68 improved, by 10,539, for 89 s); the 53 before searches (49 of them
  before climbs) cost 53 s and improved the game twice, by 60.  It proves later at first, as it should: 63 boards were
  proven at 150 CPU s against 113 without it (the maps wait for it), 172 against 187 at 450, and both end proven.
* **2576..2700**, four runs of 900 CPU s: **61 of 125 boards proven in each**, finder on or off, 0 violations, 907 to 910
  CPU s in 115 to 116 s (earlier runs of this stretch differed by about 5 boards for the same settings, these did not).
  The finder's runs of 87 each are the same game for game (42 improved, by 8,738; 28 boards, 13 of them proven, end with
  its game).  What differs is the curve: boards proven at 150, 300, 450, 600, 750 and 900 CPU s were 0, 31, 44, 46/48, 54
  and 58/61 with the finder against 12, 37, 46/47, 56, 58 and 61 without it, the finder's 88 s going to the maps first
  (the first proof at 26 s against 18) and its runs before the 31 searches that had one finding a better game twice (by
  105); the searches' seconds fell by about the finder's (695 s against 783).  So on this valley the finder neither
  proves more in 900 CPU s nor costs proofs at the end; it moves the work to the front.
* The reading: the finder's gain is at the maps (when it improves the start incumbent it does so by a median 132, and on
  1301..1500 its game is the optimum of 51 of the 89 boards searched), and the ledger's own seeds (the games of n − 1,
  n and n + 1, the proven game of the board below in a climb) are already close, so that it rarely improves on them
  before a search.  Whether it shortens the long searches of 2176..3440 and what it does for the order of the boards is
  what the cloud run measures; `seed_from`, `lns_found` and `below_proven` of its events say it per search.

**Not done.**  The cloud run with the finder and the climbs (it needs Brian's approval); a finder that scales its seconds
with the board (the start incumbent alone costs 1 to 7 s there); skipping it before a climb.

**Commits.**  4621d97 (the finder in the ledger and its tests), then this record.

## Later (2026-10-17): the bounds store

**Why.**  A run on 4400..8191 is going, and what it finds is knowledge even where no game is proven: for every board a
game worth L and a bound U, with how each was derived.  Those lived only in a run's directory (`ledger.jsonl`), on the
laptop or in S3, and the maps of 3441..8191 alone cost about 65 core-hours.  The owner wants a durable, committed place
for them, and future runs that start from it.  A board is proven when its best L equals its best U, so the store is the
record of those proofs too, and `explore.jsonl` stays the home of `taxman solve --keep-going`.

**What was built** (`examples/ledger/`: `store.rs` the line and its checks, `export.rs`, `verify.rs`, `pool.rs`;
`solver/tools/check_bounds.py`; the format is in [results/README, "bounds.jsonl"](../../../results/README.md#boundsjsonl), the
commands in the module comment of `main.rs`):

* **The file** `results/bounds.jsonl`, append-only, one line a bound, a new line only when it improves the file's best
  of its board and side; not created (the first real export is the lead's, after review).
* **`ledger export RUN_DIR --bounds FILE --source NAME [--checkpoints PREFIX]`**: appends what improves on the file's
  best, for the boards `results/strict.jsonl` has not proven, lower bounds first and each upper bound after the bounds it
  cites; prints the lines, boards, first and improved bounds of each side, and the boards newly proven.  It checks its
  lines with the checker's own `Chain` before it writes them.  **`ledger run ... --bounds FILE`** starts from the file's
  best bounds of the stretch (each game replayed by the referee, a board with both bounds not mapped).
  **`ledger verify-bounds FILE`**, **`ledger games FILE`** and **`solver/tools/check_bounds.py FILE --games G`**: the two
  referees (below).
* `book::upper_by`, `carries` and `carry_game`: the arithmetic of Lemmas L1 to L4 and the constructions of L1 to L3, one
  copy each, which the book's relations (`derive`) use as the store's checks do.

**Choices.**

* **A lower bound's game in the most compact form there is.**  Promotions, where the game was made from them (a
  search's, the finder's, the start's; a repeat of such a game): about 330 bytes at n = 900 and 1.1 kilobytes at n = 3440,
  against 1.7 and 9 for the moves.  A game that a relation made of another board's game (monotone, ceiling, prime) is
  *cited*, not written: `from` and `from_value`, no game, its game the lemma's construction applied to the game of the
  line before it that gives `from` that value (135 bytes).  Everything else, a repeat of a game that had no promotions
  and a game given from a results file, is its moves.  Without the cited form the dry run's file was 75 kilobytes, not 45
  (1,299 bytes a board, not 778: 26 of its 72 games in moves, against 7 and 19 cited), and the file of the boards past
  the chain would have been of the order of 26 megabytes, not 12; the file cannot be rewritten once it is committed,
  so the form had to be there from the first export.
  The exporter cites only when the ledger's game *is* the construction applied to the ledger's lower bound of `from` and
  the file has that line by then (else it writes the moves), and never cites a game of the chain (a run need not have it).
* **The Python referee and promotions.**  A game made from promotions needs the frame model to become moves, which the
  Python referee does not have and should not (it shares no code with the crate).  So `ledger games FILE` writes the moves
  of every promotion set, and `check_bounds.py` replays those by its own rules (loaded from `check_results.py`, the
  results files' referee: one copy of "legal, over, opens with the largest prime") and checks that they contain the
  promotions and score the line's value.  Whatever the frame model did to make them, a game that is not a game of its value
  is refused; the link between the promotions and the moves (that `solution_for_promotions` made them) is the Rust
  referee's, which replays the same promotion sets.  The cited games it rebuilds itself, with its own port of `complete`
  and the prime exchange, so it checks the constructions of L1 to L3 on the data.  The line carries nothing for the
  Python referee beyond that: the alternative, moves beside every promotion set, would have made the file several times
  larger.
* **What an upper bound rests on** is `rests_on`, the boards at which statement U is assumed: its own board for a root,
  dual or exhausted bound or a given game, what the bound it comes from rests on for a relation, and its own board added
  for a repeat (Lemma L4's upper half).  A board of the chain that a bound cites counts as resting on itself (the
  chain's proof of it may rest on the boards before it as well; its lines say so).  A relation's line cites the bound it
  came from, `from` and `from_value`, and the checkers require a line before it (or a game of the chain) with a bound of
  that board at most `from_value`, and that its `rests_on` holds what that line's does; its value may be above what the
  arithmetic gives (a weaker bound is a bound), never below.
* **The export cites what there is.**  The final ledger holds each bound's `from` but not the value of the source it was
  derived from; the export takes the source's value from the file after the export, so that a line derived from a bound
  the file has since bettered cites the better one, and says so with a smaller value (still a bound the lemma gives).
  The ledger of a run that started from the file has the file's bounds as its own, so a second export appends only what
  that run improved.
* **The chain bounds the file.**  Where the chain has a game of a board that the file also has bounds of (a board that
  entered the chain later), both referees require L ≤ score ≤ U.  A run on boards the chain has proven is then a test:
  the dry run's proven values are the chain's scores.

**Tests (the quick suite)**, `store_tests.rs`: a line is written and read back, and a malformed one is refused (a field
not in the format, no `rests_on`, a rule of the other side, a relation with no board or the wrong boards, a game on an
upper bound, promotions out of order, a game both written and cited, a repeat or a wrong direction cited); the dates
(`civil_date` at known days); `upper_by` against the lemmas and the boards it refuses; `append` refuses a torn file and
keeps every old byte; an export of a hand-made ledger of 901..960 (with the chain's games of 920 and 940, three found
boards, every rule of both sides, loose bounds, the root bound recomputed) puts lower bounds first and each line after
the line it cites, cites games, leaves out the chain's boards, records `rests_on`, `checkpoint`; a second export appends
nothing; after more is found an export appends exactly the improvements, each strictly better, in an order the checker
accepts, and the old bytes are untouched; the export of a real run of 901..960 (the chain's 900 and 930 given, the rest
hidden) proves 59 boards and a second export appends nothing and changes no byte; `verify-bounds` and the Python referee
accept it (the Python one with `ledger games`' output, and refuse it without: "was not replayed"); a chain of three
thousand boards, each citing the next, is exported in order and checked by both referees (the export keeps the boards
that wait for a cited line on a worklist, not in recursive calls, so the depth of a chain costs heap, and the test runs
on a 2 MB thread under Python's default limit of 1,000 calls); and tampered copies
are refused by both and the line named: a game of moves with its first two moves swapped, a promotion set's value
raised, a ceiling bound lowered by one, a line a relation cites removed, the board a repeat rests on dropped, its own
board dropped, a line that improves nothing, a bound below the chain's score, a value that a carried game cites changed,
a carried game moved before the line it cites, the game another line cites made not a game; and two that only the
Rust referee finds, a root bound lowered by one (it recomputes it) and a repeat's bag changed.  A run that starts from a
by-hand file with the loose bounds of every board of 910..960 maps none of them (the same run without it maps them),
proves every board at the optimum, and a file whose game is not a game stops the run at its start, naming the board (or,
if other lines cite the game, a board that cites it).

**The dry run** ([store/](store/)).  901..960 on the laptop with the truth hidden
(except the chain's lines of 900, 930 and 945, which the export leaves out): 6 CPU s proved 15 of 60 boards, and the
export appended 116 lines for 58 boards (17 proven; 600 bytes a board); a second run, 3 CPU s from the file, proved all
60, and its export appended the 55 lines that improved (41 boards newly proven).  The file: 171 lines of 45 kilobytes,
58 proven boards, **778 bytes a board**; by kind, 46 games of promotions (329 bytes each), 19 cited (135), 7 of moves
(1.7 kilobytes), 99 upper bounds (155).  `verify-bounds` takes 0.2 s (4 workers) and the Python referee 0.1 s; both
accept the file against the real `results/strict.jsonl`, where the 58 boards are games of the chain, and the proven
values are its scores.  The root bounds of n = 3000, 5000 and 8000 take 0.45 CPU s in all, so `verify-bounds` on a file of
3,800 boards is a matter of a few minutes of one thread (the games' frame setups are the most of it), and the Python referee
replays a game of n = 3440 in 0.6 ms.  A second dry run at larger boards, 2900..2939 for 108 CPU s (mostly maps and the
finder; 8 boards proven, by relations, at the chain's scores): 38 boards, 76 lines, 70 kilobytes, 1,834 bytes a board; a
promotion set 870 bytes, a game of moves 6.4 kilobytes, a cited game 140, an upper bound 182; both referees accept it
against the real chain, `verify-bounds` in 0.4 s of 6 workers.  Past the chain (4400..8191) a promotion set is about 2
kilobytes, a game of moves about 15, a cited game about 150 bytes and an upper bound about 220: the first export of 3,800
boards of the order of 12 megabytes (3 kilobytes a board), against 26 megabytes without the cited form.

**Not done.**  The first real export, and its commit (the lead's, after review); a run from the committed file on the
cloud (`BOUNDS=` in `solver/tools/ledger/cloud.sh`, which needs Brian's approval to launch).  A repeat of a game that had no
promotions could be cited too (its game is the source's moves filtered by the candidates, played on the board), but needs
the frame model on both sides; it is 7 of 72 games here.  The check of a `dual` or `exhausted` bound against its
checkpoint, which `--checkpoints` records but nothing reads.

**Commits.**  14f36d6 (the store, its commands, the Python referee and the tests), e93d035 (the export on a thread with a big
stack, a workaround), then the commit that follows citations with a worklist and drops the thread, and this record.

## Later (2026-10-18): past 4400, stage 1

**Why.**  The prep run (2176..3440, two ledgers side by side, 24 core-hours each, given the games to 2175) proved 876
boards with the climbs and the finder against 837 with the climbs alone and 806 for the first run at the same
core-hours; climbs proved their board 53-69% of the time at about 50 CPU s a proof.  Brian then approved up to $25 to
prove games the run to the wall will not reach: find the easiest spots past 4500, drill on them, and spread both ways.

**The run.**  4400..8191 (3,792 boards) on one `c8g.16xlarge`, commit b3ed8ac, 64 workers, the finder 10 s, climbs both
ways with a limit of 1 hour, nothing given; 360 core-hours (about $3.80), 2026-10-18 02:41-08:22 UTC
(`s3://taxman-013764204840-us-east-1/ledger-frontier/out`).  The run data kept in the repository for the cloud runs
of this entry is described in [solver/notebook/the-ledger-bounds-from-both-sides-2026-10/README.md](README.md).

**Results.**  Every board was mapped and given the finder's game (it improved 1,720 of 2,042 runs) within about 50
minutes; then 242 boards were proven: 4446-4475, 4477-4507, 4540-4542, 4554-4622, 4626-4639, 4664-4757 and 4779,
by repeats 96, their own search 95 (82 of them from the finder's game), the ceiling 35, primes 16.  Proven against
core-hours: 112 at 72, 164 at 180, 212 at 288, 242 at 360.  The 4400..4799 boards took 60% of the search time and gave
every proof; 4800..5399 took 30% and gave none.  After the finder the smallest U - L past 4800 is 1,664 (medians 2,054
at 4800..5199 rising to 7,631 at 8000..8191).  Climbs: 22 of 30 up and 2 of 8 down proved their board.  The bounds of
all 3,792 boards are the first lines of `results/bounds.jsonl` (8.4 MB), both referees passing.

**Next.**  Stage 2: 4300..4999 for 12 hours from stage 1's state (its ledger and the checkpoints of 4400..4999 copied
to `ledger-frontier-2`), commit afe4513.

## Later (2026-10-18): floor-guided drilling

**Why.**  The ordinary search of an open board doubles its limit and resumes its checkpoint, and until it finishes it
leaves little on the upper side (the dual bound of a stopped depth-first search is the root's bound or near it); and the
order, `level + gap / gap_unit`, has no idea what a board's proof will cost.  A search under a floor F
([the entry](../proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md), `search::search_above`) that
completes having found nothing proves "nothing weighs more than F" (THEORY.md, Lemma F), and its cost is log-linear in
the distance below the root's bound: a decade of nodes per 147 points (a median over 14 games of 2265-3259; 0.53 to
0.89 decades per 100 a game), a quarter of the gap for 1/20,000 of the proof.  So a step leaves a proven upper bound
behind for a small share of the proof, and two or three steps give a board its own line, a forecast of what its proof
costs.  The goal is every game in the end, so the total cost matters, not the count early; and the leverage between
games is the proven board below (a search with n − 1 proven costs about 2.4 times less), so the climbs stay first.

**The design** (`examples/ledger/cost.rs` the line, the forecast and the plan; `schedule.rs` the jobs and their
results; the module comment of `main.rs` has the fields, the events and the settings).

* **A floor step** is the job of an open, searched board that is not climbed to and whose gap `U − L` is more than two
  steps: `search_above` under the floor one step below U (a score; the promotion sum is that less the bag), capped by
  U, with no seed and no finder, on one thread, limited to `--floor-slack` (4) times its forecast time (at least
  `--base`).  It ends in one of five ways.  *Held*: it completed having found nothing, the floor is U (the upper rule
  `floor`), and the step, with its nodes and seconds, is kept on the board (`steps`: a point of its line).  *Proved*: it
  found sets and completed, so the last is the optimum (Lemma F): L and U are that game, which the referee replayed on
  the worker (`refereed`) as it replays every game; the board is proven.  *Found*: its limit stopped it with a set, L
  rises to that game (replayed), and the floor is below L now: the step is dropped, and its checkpoint, a search from
  that set, becomes the board's own for its proof if the board has none (else it is deleted).  *Stalled*: its limit
  stopped it with nothing, so it is **kept**: `stalled` (floor, nodes, seconds, tries) on the board and its checkpoint
  `ckpt/nN.floor.ckpt`, apart from the board's `nN.ckpt` that the proof and the climbs use, which a floor step cannot
  continue; the board's next job is that step again, resumed under the same floor, with the limit doubled each try; if
  the bounds move so that the floor is not between L and U, the step is moot and the next one starts afresh (its old
  checkpoint is deleted then).  *Moot*: a relation proved the board, or lowered U to the floor, while it went on (the
  search then ended at the ceiling with nothing explored).  A stopped step gives no dual bound: the open units of a
  depth-first search are the root's and near it.
* **The step** is the same factor of cost whatever the board: `--floor-step` (150) points at the median slope, a
  tenfold of nodes; on a board with its own line, 150 · 0.0068 / slope points, so a steeper board takes fewer.  It goes
  down from the board's upper bound, which a relation may have lowered below the root's, so a step costs a tenfold of
  the depth already proven.  The steps cost a share of the proof that falls by a factor of ten each step at best, and
  a half if the last lands close to the proof; so the board goes to its proof when its gap is within *two* steps
  (`PROOF_STEPS`), not one.  On the 14 measured games, laid on their measured costs from the root's bound
  (`solver/notebook/the-ledger-bounds-from-both-sides-2026-10/floor/ladder.py`), the steps and the proof cost together a median 1.05 times the proof alone (1.06 by
  nodes; steps of 100 points 1.14, of 200 1.01), and 1.49 (1.60) if the proof comes only when the gap is within one
  step.  The proof is the ordinary search as before: seeded with the best game, capped by U, the finder first, resuming
  `nN.ckpt` if the board has one.
* **The line and the forecast** (`cost::line`, `forecast`).  log10(nodes) = intercept + slope · distance below the
  root's bound, fitted by least squares through the board's completed steps of 100 nodes or more (the shortest trees
  have a floor of their own), the slope pulled to the median 0.0068 as a pair of points 141 apart would pull it and
  kept in 0.003 to 0.012; with one point, the median slope through it; with none, the median line (0.5 + 0.0068 ·
  distance, which has 1,000 nodes prove down 370 points, 10,000 down 514 and 100,000 down 661, as measured).  A
  stalled step lifts the line to pass no lower than its nodes.  The forecast of the proof is the line at the distance
  from the root's bound down to the lower bound, and not less than twice the steps that the board's ordinary searches
  and climbs have spent without finishing, so that a board whose proof keeps being stopped looks dearer each time (the
  charge `level` made); in seconds of one thread at the board's own rate (its steps, once they have taken a second),
  else the run's, else 1,000 a second.  On the 14 games, the forecast made from the line of the steps so far is, in
  log10 of nodes (forecast less actual): with no point 0.59 median absolute error (the median 1.9 times too
  dear), with one 0.35, two 0.27, three 0.13.
* **The order**: after the maps, the climbs; the boards a proven neighbour has boosted, smallest gap first; then the
  boards that have yet to give their line two points (`cost::exploring`: a step to take and fewer than two completed
  steps of 100 nodes or more), each a cheap step; then the board whose forecast seconds are least.  The forecast is in
  the events (`forecast`, log10 of nodes, and `forecast_s`) and in the status line (the boards forecast within an hour
  of one thread, within a day, longer).  A board whose forecast is far beyond the rest waits until the cheaper work is
  done or running.  `--floor-step 0` is the ordinary searches and the old order.  The exploring tier came from the
  first build's dry run at 4500 (below): a board with no line is forecast by the median line, which is far off there,
  so the few boards that had a line looked 5 decades cheaper than the rest and kept the workers.
* **Climbs** are as they were (seeded with the proven game, capped by the ceiling, `--climb-limit`), with the two loose
  ends of 6088c21: the games that seed a search of the board of a repeat class include those of the boards just
  outside the class (the proven one a climb into the class goes from: `b + 1` for a climb down into `a..b`), and while
  a climb into a class runs, the boards just outside the whole class wait (`held`; it held only the boards next to the
  board searched).
* **The store** accepts the upper rules `floor` and `nogoods` on record, as `exhausted` and `dual` are (a bound of the
  frame model at its own board, `rests_on` that board): `store.rs`, the export (no change: it writes them as the
  ledger has them), `verify-bounds` and `solver/tools/check_bounds.py` list them among the bounds they take on their record,
  and `results/README.md` defines them.  `nogoods` is the Lagrangian bound over checked nogoods that `wall nogoods`
  makes; nothing in the ledger makes it.
* **Resume.**  A ledger written before the steps goes on: `root`, `steps` and `stalled` default when absent; the root
  of every board that is mapped, unproven and searched but has none is computed at the start (the GF(2) bound of the
  root, 0.15 s at 5000, on the workers).  The checkpoints of its ordinary searches are **not dropped**: floor steps
  have a checkpoint of their own, so the old one stays and is resumed when the board comes to its proof, and a board
  whose old search was far along is forecast at least twice what it spent, so that it waits behind the cheaper boards,
  as `level` made it wait.

**Tests (the quick suite).**  `cost.rs`: the fit recovers a line and is pulled to the median slope by few points; a
board's line and forecast come from its steps of enough nodes, a stalled step lifts the line, and ordinary searches
that spent more than the line gives raise the forecast; the rate is the board's, the run's or a default; the step is
150 points at the median slope, fewer or more on another, and the proof comes when the gap is within two steps; a
stalled step is resumed while its floor is between the bounds and moot outside; a board explores until its line has
two points of 100 nodes or more; the forecast orders boards by their lines.  `floor_tests.rs`: a step above the optimum
completes and lowers U by the rule `floor` (the step kept on the board, the event saying so, the proof next); a step
below the optimum finds the optimum, L is a game the referee replays, and the board is proven with no checkpoint left;
a step stopped with a set drops its floor and hands its checkpoint to the board's proof, or deletes it where the board
has one; a step stopped with nothing is kept and the next job is that step resumed under its floor, which completes
where it left off; a step ended at the ceiling is moot; the jobs go in order: the climbs, the boosted board, the boards
with no line by n, then the cheapest line, the board whose proof is a gap of 1, the dearest line, the board whose
searches spent most; a climb into the class {925, 926} holds 924 and 927 and a climb down into it is seeded with 927's
game; a run of 901..960 with steps of 20 proves every board at its optimum with the events and the report saying how
each step ended; a ledger written without the three new fields, with a stopped ordinary search's checkpoint, goes on
and proves its board.  `store_tests.rs`: the hand-made books have `floor` and `nogoods` bounds, which both referees
accept and list.  The earlier tests of the order of the jobs run with `floor_step` 0.

**The dry runs** (the laptop, the non-release builds under `nice -n 10`, the truth hidden, through `cloud.sh`; the
commands and files are in [floor/](floor/)): `base` is the scheduler before this change
(93056fd), `first` the build of f775cc0 and `new` the build of 26c5c71 (the same with the boards exploring first and the
checkpoint of a step that found a set kept), each stretch run for each in turn, the new builds first.  "Searches" counts
every job after the maps, the floor steps and climbs among them; the wall times are of a shared laptop and say little.

| stretch, workers, budget | build | proven | CPU s | wall s | searches (s) | of them floor steps (s) | U − L left, summed (boards) |
|---|---|---|---|---|---|---|---|
| 1301..1500, 6, 2 h | base | 200 | 619 | 120 | 97 (590) | | |
| | first | 200 | 619 | 134 | 106 (587) | 12 (2) | |
| | new | 200 | 620 | 132 | 186 (653) | 94 (8) | |
| 2576..2700, 8, 900 CPU s | base | 61 | 911 | 131 | 127 (867) | | 37,550 (64) |
| | first | 69 | 909 | 117 | 177 (788) | 84 (38) | 14,543 (56) |
| | new | 73 | 908 | 120 | 212 (807) | 124 (62) | 9,703 (52) |
| 2576..2700, 8, 1800 CPU s | base | 118 | 1802 | 271 | 183 (1681) | | 3,400 (7) |
| | first | 118 | 1801 | 272 | 259 (1685) | 92 (56) | 1,468 (7) |
| | new | 116 | 1802 | 282 | 297 (1851) | 122 (62) | 1,901 (9) |
| 3076..3200, 8, 900 CPU s | base | 0 | 911 | 119 | 109 (648) | | 132,877 (125) |
| | first | 0 | 915 | 120 | 79 (655) | 39 (149) | 128,735 (125) |
| | new | 0 | 906 | 128 | 195 (646) | 193 (584) | 65,806 (125) |
| 4500..4523, 4, 540 CPU s | base | 0 | 558 | 143 | 33 (426) | | 40,043 (24) |
| | first | 0 | 540 | 143 | 33 (435) | 33 (422) | 31,119 (24) |
| | new | 0 | 544 | 138 | 64 (407) | 64 (394) | 26,301 (24) |

Every run has **0 violations** (the check of `ledger report` against the chain on the first four stretches: every game
replays to its L, L ≤ opt ≤ U, and every proven value is the chain's score; 4500 is past the chain).  Proven boards
against CPU seconds on 2576..2700 at 900 CPU s (each tenth of the budget): base 0, 0, 23, 42, 44, 45, 50, 57, 59, 61;
new 0, 0, 14, 36, 44, 48, 56, 60, 70, 73; at 1800: base 0, 44, 50, 57, 59, 68, 84, 96, 107, 118; new 0, 36, 44, 56, 65,
82, 88, 96, 106, 116.

**Findings.**

* **The easy stretch is unchanged**: 1301..1500 in 619 to 620 CPU s with and without the steps (the earlier runs of the
  finder and the climbs took 547 to 612); most of its boards have gaps within two steps and go straight to their
  proofs, and the steps taken, 12 by the first build and 94 by the new one (which explores), cost 2 and 8 s in all.
* **On 2576..2700 the steps bring proofs forward**: at 900 CPU s the first and the new build have proven 69 and 73
  boards against 61 (four earlier runs of the same budget with a build before the steps, in the finder's section, each
  had 61), and the gaps the open boards have left sum to 14,543 and 9,703 against 37,550; at 1800 CPU s all three have
  116 to 118 of the 125 (the same 7 to 9 boards remain, with gaps of 200 and more: the walls of this stretch).  The
  curves of boards proven against CPU: the new build is behind the base up to 700 CPU s (its exploring and first steps
  come before any proof), ahead from 900 to 1300 (65 against 59 at 900, 82 against 68 at 1080) and level from 1440.
  The difference at 900 CPU s is 8 to 12 boards, about the 5 boards by which the repeated runs of one build differ, and
  each curve is one sample: the gain of the steps in proofs is real but modest here, and in the gaps it is large.  The
  steps cost 38 to 62 s of the 900.
* **On 3076..3200 nothing can be proven in the budget** (the maps take 320 of the 906 CPU s), and the steps spend the
  rest on bounds: with every board exploring first, 193 steps (584 s) on all 56 boards that are searched (the rest of
  the 125 are repeats and primes, whose bounds follow) lower U on every one, and the gaps sum to 65,806 against
  132,877, half; the first build, with the boards that had a line keeping the workers, stepped eight boards (128,735).
  Of the 584 s, 322 were steps that completed, 62 were steps that the bounds of a neighbour ended (moot: a step on a
  board lowers U of its neighbours and repeats by the relations, and a step running on one of them then ends at its
  ceiling, having explored a tree whose bound the relations had already given), and 200 were steps that the end of the
  budget stopped, which are kept.
* **At 4500** (a single board on one worker, then 24 boards on four, with the finder at 1 s): a step costs about 0.07 s
  of setup beyond its search (the first four steps' searches take 0.04, 0.04, 0.10 and 0.30 s), so the steps that
  cost 4 to 600 nodes are no matter.  The first points of the line have a slope of 0.0033 to 0.0039 a point, about half
  the median 0.0068 of the 14 games (2265-3259): the steps of one board went down 150, 150, 150, 150, 195, 257 and 289
  points as the line flattened, and the median line forecast a proof 5 decades too dear until a board had two points,
  which is why the boards explore first.  On the 24 boards (12 of them searched; the rest are repeats and primes) the
  steps lowered the gaps' sum from 40,043 to 26,301 in 540 CPU s.  A thread made 2,200 to 3,200 steps a second there.
* **The forecast** of a board's proof after its last step, against the nodes of the searches that proved it (log10, forecast
  less actual): a median -0.55 (59 boards) on 1301..1500, -0.33 (32) and -0.22 (52) on 2576..2700, with median absolute
  errors of 0.55, 0.48 and 0.28: the forecast is a factor 1.7 to 3.5 too small at the median, the largest errors in the
  proofs of a few thousand nodes, which cost more than the line gives (an intermediate build's list: 0.4 to 1.1 decades
  too small at 8,000 to 40,000 nodes; the shortest trees have a floor of their own).  The 14 games' simulation, with
  two points, has an error of 0.27.
* **The limit and the stalls**: of 193 steps on 3076..3200 13 stalled, of 124 on 2576..2700 one, of 64 at 4500 eight.
  All but one were in the last seconds of the run, where a job's limit is what is left of the budget (the board keeps
  the stalled step, which the next stage resumes).  The one, in both runs of 2576..2700, was board 2637 at 450 points,
  where the median line forecast 3,600 steps and the board needed more than 15,800: stopped by the limit of four times
  the forecast (2.7 s), kept, and resumed by the board's next job.

**Not done.**  The cloud run (it needs Brian's approval).  Not measured: other values of `--floor-step` and
`--floor-slack` (the simulation on the 14 games says that a step of 100 to 200 points costs 1.01 to 1.14 times the
proof alone, but the dry runs used 150 and 4 only); boards past 3440 beyond the 4500 probe, where the gaps are 1,200
and more and the 14 games' median line is off (the slope there is half, and the cost of the proof of a board at 4500
is not known: none was proven, the dearest step reached 260,000 steps); many workers on a stretch of walls, where the
steps of the cheapest boards run side by side and coupled boards end each other's steps (11% of the step seconds on
3076..3200); floor steps on many threads; a bias correction of the forecast (the proofs cost a median 1.7 to 3.5 times
the forecast at these sizes).  The prior's intercept and slope are the 14 games' median; the boards that explore first
make them matter little, but a forecast for a stretch far from 2265-3259 is the stretch's own lines, not the prior's.

**Commits.**  f775cc0 (the steps, the forecast and the order, the store's rules, the tests), 26c5c71 (the boards
exploring first, and the checkpoint of a step that found a set), then the commit of the documents and this record.

## Later (2026-10-18): the chain draws on the known games

**Why.**  The store holds 343 proven boards in 4300..4783, which the run to the wall will reach and would prove again,
at the hours a game costs there.  Brian's brief was `taxman solve --bounds FILE`; the lead's correction was to give no new
option and to extend the flag that already means "use what is known", `--use-prev`, to every proven game the repository
holds: `optimal.json`, the committed chain and the store.

**What was built** ([README, "Known games"](../../frame/README.md#known-games); DESIGN.md, section 8):

* *One copy of the store's format.*  The line, the file, the best bounds, the relations between boards and the referee's
  replay of a lower bound's game moved from `examples/ledger/` to `src/store.rs`; the ledger re-exports them and keeps the
  checks of a line against the lines before it.  `src/known.rs` reads the three sources.
* *`--use-prev` draws on them.*  A board that a source has proven is recorded with no search (method `known`, proof
  `known`, a `known` field citing the source); the game of n - 1 from a source gives the ceiling and the seed
  (`prev.source` `strict` or `bounds`); a board that the store has not proven seeds its search with its lower bound's
  game and ends it at its upper bound.  The best source is the committed chain's (it assumes nothing), then the store's,
  then the table's.  `--force-search` leaves the boards to their searches; `regress --use-prev` keeps its old meaning.
* *What a line assumes.*  The table's games are assumed optimal, the committed chain's assume what their lines do, and a
  bound of the store holds wherever statement U holds at the boards its upper bound rests on, one assumption a board
  (`statement U holds at n=B`), inherited by the games that rest on it.  A game found by a search that stopped at the
  store's bound is proven by it, and settles its search problem for a later repeat.
* *The referees.*  Both look the cited lines up in the sources (the repository's `results`, by default) and refuse a
  missing, changed or other line, a citation that is not there, other assumptions, and a store that was not given; neither
  checks the sources again.  `solver/tools/check_results.py` no longer reads `bounds.jsonl` as a file of results when it is run
  with no argument (it did since the store was added, and failed).
* *Schema 5 stays.*  The new key is there only on a line that has one, and the new method and proof are new names; every
  committed file reads as it did.

**Measured.**  `taxman solve 4650-4759 --use-prev` on the real store (110 proven boards, the laptop, 4 threads): 4.9 s
(15.8 s of CPU, nearly all of it the referee's replay of the lower bounds' promotion sets at n of 4,700), 110 lines of
proof `known` in 1.4 megabytes, assuming statement U at 83 boards in all; `taxman verify-results` takes 0.2 s on them and
the Python referee 0.7 s.  The chain's own cost for those boards is hours (3,440 alone took 5 hours on 64 threads).  On
boards 900..960 with a hand-made store of six proven boards, the six lines cost no nodes and the other 55 have the scores
of the chain without it; with the store's upper bound alone, the dearest search of the range stops when it reaches the
bound (fewer nodes), and a lower bound's game at the optimum seeds a standalone search of that board to fewer nodes than
the unseeded one.

**Not done, and what to know.**  No cloud run, and the store's open boards have not been used on a game of the wall's size:
the seed and the ceiling are tested at 900..960 only.  A known game sets up no search problem, so the first game after a
known stretch cannot repeat a known game's search, and searches (from the known game of n - 1, with its ceiling and seed) a
problem that the chain would have settled by a repeat: the games 3441 and 4300 on are where that could cost, and the lines
of the run's own searches are repeated as ever.  A chain from n = 1 with `--use-prev` re-proves nothing to 3440, so it is
not the run that checks a build (RUNBOOK section 12.4).  `ledger run --bounds` and `solve --use-prev` take the store's
upper bounds on its word: `ledger verify-bounds` comes first.

**Commits.**  The move of the format into the library, then `--use-prev` with the known games, their referees and the
tests (`tests/known.rs`), then the documents and this record.

## Files

The program is `examples/ledger/` (its module comment defines every field of `ledger.jsonl` and `events.jsonl`), run
from the top of the repository with the non-release build of the laptop (`cargo build --profile test --example ledger
--bin taxman` in `solver/frame`: opt-level 2, debug assertions on), so `ledger` is
`solver/frame/target/debug/examples/ledger` and `taxman` `solver/frame/target/debug/taxman`.
Everything here ran under `nice -n 10`.

### Files and the commands that made them

| file | command | what it is |
|---|---|---|
| `dry/` | `OUT=DIR LEDGER_BIN=target/debug/examples/ledger WORKERS=6 CPU_HOURS=2 solver/tools/ledger/cloud.sh 1301-1500` (2026-10-17, 21:11:44 to 21:13:57 UTC, after the fix of the first cloud attempt), with DIR a scratch directory whose files were copied here | the laptop's dry run of 1301..1500, the truth hidden (the run before the fix, 20:01:50 to 20:04:16 UTC, took 595 CPU s and 145 s; its files are in d7390e1): `run/ledger.jsonl.gz` and `run/events.jsonl.gz` (gzipped after the run; the checkpoints were all deleted, every board being proven), `run.err` (its status lines), `log` (the stages' times); `report.txt` is the run's own (`cloud.sh` makes it) |
| `dry/chain.jsonl.gz` | `taxman solve 1301-1500 --threads 1 --window 1 --progress 0 --out dry/chain.jsonl`, then gzipped | the chain over the same stretch on one thread with the same build, standalone at 1301 (no game of 1300), for the comparison: 447.5 s in all (`/usr/bin/time`: 449 s wall, 446.7 s user) |
| `dry/events.txt` | `./events.py dry/run dry/chain.jsonl` | the output the entry quotes |
| `climb/` | runs of one stretch back to back (2026-10-17 and 18, 23:43 to 00:36 UTC), the non-release build, the truth hidden: `OUT=DIR LEDGER_BIN=target/debug/examples/ledger WORKERS=6 CPU_HOURS=2 [CLIMB=up\|down\|both\|off] solver/tools/ledger/cloud.sh 1301-1500` (`dry-*`), `WORKERS=8 CPU_HOURS=0.1667 ... 2576-2700` (`mid-*`), `CPU_HOURS=0.5` (`long-*`); `*-up` and `*-off` are the climbs up only (the build of eb9dc9c, the default then: `CLIMB_LIMIT` unset, 60 s) and none (`CLIMB_LIMIT=0`), `*-both` and `*-down` the build of 55d2b2a | wedge and climb, [the entry's section](README.md#later-2026-10-17-wedge-and-climb); each directory has `report.txt` (the run's own; before 55d2b2a it has one line of climbs, and for the runs without any it reads "-0 s", a rounding the report no longer makes), `log`, `run.err`, `events.txt` (`./events.py` on `run`) and `run/ledger.jsonl.gz`, `run/events.jsonl.gz` |
| `finder/` | runs back to back (2026-10-18, 01:01 to 01:15 UTC), the non-release build of 4621d97, the truth hidden: `OUT=DIR LEDGER_BIN=target/debug/examples/ledger WORKERS=6 CPU_HOURS=2 LNS_SECONDS=1 solver/tools/ledger/cloud.sh 1301-1500` (`dry-on`, and `dry-off` with `LNS_SECONDS=0`, last), `WORKERS=8 CPU_HOURS=0.25 LNS_SECONDS=1\|0 ... 2576-2700` (`mid-on-a`, `mid-off-a`, `mid-off-b`, `mid-on-b`, in this order) | the finder in the ledger, [the entry's section](README.md#later-2026-10-17-the-finder-in-the-ledger); each directory has `report.txt`, `log`, `run.err`, `events.txt` (`./events.py` on `run`) and `run/ledger.jsonl.gz`, `run/events.jsonl.gz` |
| `store/` | the laptop (2026-10-17, evening), the non-release build: `ledger run 901-960 --dir run1 --cpu-hours 0.0015 --workers 4 --lns-seconds 0.1 --base 0.05` (15 of 60 boards proven, 6 CPU s), `ledger export run1 --bounds bounds.jsonl --source dry-1 --checkpoints s3://bucket/ledger/out/run/ckpt --strict S` (S: the chain's lines of 900, 930 and 945 only, so that the other boards got lines); then `ledger run 901-960 --dir run2 --cpu-hours 0.01 --workers 4 --lns-seconds 0.1 --base 0.05 --bounds bounds.jsonl` (all 60 proven in 3 CPU s, from the file) and `ledger export run2 --bounds bounds.jsonl --source dry-2 --strict S --date 2026-10-18` | the bounds store ([the entry's section](README.md#later-2026-10-17-the-bounds-store), [results/README, "bounds.jsonl"](../../../results/README.md#boundsjsonl)): `bounds.jsonl` (the file the two exports made: 171 lines, 45 kilobytes), `verify.txt` (`ledger verify-bounds bounds.jsonl --strict results/strict.jsonl`) and `check.txt` (`python3 -I solver/tools/check_bounds.py bounds.jsonl --games G --strict results/strict.jsonl`, G the output of `ledger games`): both referees accept the file, and the 58 boards it proves are the chain's games at the chain's scores |
| `floor/` | the laptop (2026-10-18), the non-release builds (`cargo build --profile test --example ledger`), the truth hidden, each `OUT=DIR LEDGER_BIN=BIN WORKERS=W CPU_HOURS=H nice -n 10 solver/tools/ledger/cloud.sh RANGE`: `*-dry` with W=6, H=2 on 1301-1500, `*-mid` with W=8, H=0.25 on 2576-2700, `*-hard` with W=8, H=0.25 on 3076-3200, `*-mid2` with W=8, H=0.5 on 2576-2700, `*-scale` with W=4, H=0.15 and `LNS_SECONDS=1` on 4500-4523, `first-scale1` with W=1, H=0.05 on 4500-4500; `base-*` is the scheduler before the floor steps (93056fd), `first-*` the build of f775cc0 and `new-*` that of 26c5c71 (the boards exploring first), the new builds run before the base | floor-guided drilling, [the entry's section](README.md#later-2026-10-18-floor-guided-drilling); each directory has `report.txt` (the run's own), `log`, `run.err` (its status lines), `events.txt` (`./events.py` on `run`) and `run/ledger.jsonl.gz`, `run/events.jsonl.gz`; `ladder.py` and `ladder.txt` (`./floor/ladder.py > floor/ladder.txt`) lay the steps on the 14 games measured under floors (`../target/target.jsonl`): the steps and the proof cost a median 1.05 times the proof alone, and the forecast's error by the points of the line |
| `climb_limit.py`, `climb_limit.txt` | `./climb_limit.py cloud-20261017/run` (the second cloud run: 2176..3440, 48 core-hours on 64 workers; its events file is kept there gzipped, its 9 MB ledger is not) | what a longer climb limit would cost, from that run's events: the table the default of 60 s rests on |
| `cloud-20261017-stopped/` | `solver/tools/ledger/cloud.sh 2176-3440` on a `c8g.16xlarge` with 64 workers (2026-10-17, 20:31:21 to 20:40:21 UTC, stopped by a signal after 9 minutes), the three small files of `~/ledger` and the output of `./events.py` on its run | the first cloud attempt, before the fix: `report.txt` (nothing proven, 1,299 CPU s), `log`, `run.err` (its status lines), `events.txt`; the 21 MB events file and the 8 MB ledger are not kept |
| `cloud-20261017/` | `solver/tools/ledger/cloud.sh 2176-3440` with `WORKERS=64 CPU_HOURS=48` on a `c8g.16xlarge` (2026-10-17, 22:04:55 to 22:50:11 UTC), after the fix of the first attempt: the truth hidden, nothing given, no climbs, no finder (the second cloud run) | `report.txt` (916 of the 1,265 boards proven, every one at the optimum, 0 violations, 48.02 core-hours), `log`, `run.err` (its status lines), `events.txt` (`./events.py` on `run`, made while the ledger was at hand), `run/events.jsonl.gz` (the 1.8 MB events gzipped; `climb_limit.py` reads it); `results.html`, the page of the run (the games proven against core-hours, the same games both ways against the chain's cost, where the ledger proved games, what is left open, every game's bounds, what proved the games), made by `report_page.py RUN_DIR results.html` with RUN_DIR holding the run's `events.jsonl` and `ledger.jsonl` (the 8.4 MB ledger is not kept, so the page is the record; the script reads the chain's cost per game from `solver/notebook/the-run-to-the-wall-2026-10/reproof.jsonl.gz`, the optima from `results/strict.jsonl` and the quick bounds from `solver/notebook/the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/cal.jsonl`); the checkpoints are not kept |
| `cloud-20261018-prep/` | the prep run: `cloud.sh 2176-3440` as two ledgers at once on one machine, each `WORKERS=32 CPU_HOURS=24`, the games to 2175 given and the climbs both ways, the truth of 2176..3440 hidden (2026-10-18, 01:30:04 to 02:15:31 UTC): `a/` with the finder (1,042 runs), `b/` without (`LNS_SECONDS=0`) | in `a/` and `b/` `report.txt` (877 and 838 boards proven, 876 and 837 besides the given 2175, all at the optimum, 0 violations), `log`, `run.err`, `events.txt` (`./events.py` on `run`) and `run/events.jsonl.gz`; `compare.html`, made by `./compare_page.py compare.html` from the events of the three runs (this one's `a/` and `b/` and `../cloud-20261017/`): the proofs against core-hours, the climbs and the finder, stretch by stretch; the ledgers (8.4 MB each) and the checkpoints are not kept |
| `past4300/` | `./past4300/past_chart.py LEDGER OUT.html` and `./past4300/where_chart.py LEDGER ../../results/strict.jsonl OUT.html`, with LEDGER a `ledger.jsonl` | the survey of the boards past 3900 (2026-10-18), two pages made as they are, with their scripts.  `bounds-stage2-midway.html`: every board of 4300..4999, where its bounds started (the quick bounds of `../landscape/map.jsonl.gz`) and where they stood midway through stage 2 (304 of 700 proven; stage 2's final ledger has 343).  `where-3899-5000.html`: games 3899..5000 proven or open, the gap of each open game and two forecasts of the search left for each (the floor-step line, and the chain's model, log10 core-seconds = 1.89 + gap / 553), from the survey's ledger (3900..4999, floor steps) when the chain's file stood at 3905 games, so that its chain row ends there (with today's file, at 3914).  The ledgers (8-12 MB) are not kept: the bounds are in `../../results/bounds.jsonl` and the pages are the record |


The launcher `cloud.sh` and the reader of the events, `events.py`, are tools of the repository: [solver/tools/ledger/](../../tools/ledger/README.md).
