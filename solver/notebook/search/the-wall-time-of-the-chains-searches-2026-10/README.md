# The wall time of the chain's searches (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-22.

**Question.**  Where does a game of the chain spend its wall time on the machine of 88 threads, 72 of them the chain's,
and what would make the searches finish sooner without risking a wrong result?  The angles taken first: whether the end
of a game leaves threads idle, how far the start incumbent lies below the best and what that costs, and the threads the
chain leaves unused.  When conflict branching should be used at a node is another entry's question.  The [live
lab](../the-live-lab-2026-10/README.md) has the studies of 2026-10-22 that this entry starts from.

All times are UTC.  A game is named by n; a search's *best* is the promotion sum of its incumbent, and a *start* is the
incumbent the search begins with ([search.md, the
start](../../../docs/design/search.md#the-starting-incumbent-the-ceiling-and-the-seed)).  A game's *own search* is the
time it has been the oldest game of the window with its search running ([engine.md, the
window](../../../docs/design/engine.md#the-window-games-in-flight)), which leaves out the time it waited behind an older
game.  A *seed* is a game whose promotions a search starts from, and its *survivors* are those still promotable on the
board searched.

## 2026-10-22: where the hours of the chain went

**The progress lines show no idle tail and no idle threads.**  `timeline.py` reads the chain's progress lines
(`~/taxman-work/logs/chain.err`, one a minute) and gives each game searched since the chain moved to this machine its
steps, its own search, its start and its best:

| game | steps | own search | steps/s | start | best | start below best |
|---|---:|---:|---:|---:|---:|---:|
| 4875 | 183,500,449 | 55 min | 55,300 | 537,937 | 538,080 | 143 |
| 4879 | 178,132,934 | 56 min | 52,700 | 540,552 | 540,552 | 0 |
| 4880 | 289,485,544 | 1 h 43 min | 46,900 | 542,420 | 542,420 | 0 |
| 4884 | 172,697,276 | 60 min | 48,300 | 543,308 | 543,308 | 0 |
| 4888 | 157,095,254 | 55 min | 47,300 | 543,860 | 543,860 | 0 |
| 4890 | 283,314,868 | 1 h 35 min | 49,500 | 544,107 | 544,107 | 0 |
| 4893 | 614,467,192 | 3 h 19 min | 51,500 | 543,739 | 543,874 | 135 |

* The engine's efficiency, the busy time over the threads' time, is 99 to 100% on every line of a game's own search,
  to its last minute, and the idle time of the tail is 0.0% everywhere.  The engine splits work whenever a worker is
  idle, so a game ends within a minute of its queue running dry.  A game's wall time is its steps over the rate: there
  is no tail to cut.
* The rate is 47,000 to 55,000 steps a second on 72 threads, about 700 a thread.  The search of 4875 ran on 88 threads,
  and the lab's 16 threads stand idle unless a study runs.
* The start is the incumbent when the finder starts, at a million steps; a progress line comes later.  At 4875 the
  finder lifted it from 537,937 to 538,051 in 15 s, and the search reached 538,080 within 4 minutes.
* The clocks of `chain.out` overstate the cost of a game: 4890's 9,087 s include the 58 minutes it waited, suspended,
  behind 4888, and 4893's 17,594 s include an hour and a half behind 4890.

**The start of 4893 lay 135 below its best, and the search took 3.3 hours.**  Its start was 543,739; the finder ran its
60 s at a million steps and found nothing.  The search itself raised the best to 543,747 at 145 million steps, then to
543,787, 543,796, 543,849 and 543,868 between 418 and 474 million, and to 543,874 at 589 million; it ended at 614
million steps.  The other searches of this machine took 157 to 289 million steps; five of the six started at their
best, and 4875 reached its best in 4 minutes.  4893's search took two to four times the steps of its neighbours.

## 2026-10-22: the start of 4893 came from the bounds store

**The bounds store's game of 4893 displaced the game of 4892 as the seed, and its start lay 135 below the best.**  With
`--use-prev`, a search has two seeds: the game of n − 1, and the game of n that the bounds store's lower bound holds
(`results/bounds.jsonl`, made by the ledger's large neighbourhood search on 2026-10-18).  `SearchJob::search` in
`src/solver.rs` keeps the one whose survivors, its candidates still promotable here, sum to more, and the start (the
dive and the local search) and the finder's seed rounds both use that one alone.  At 4893 the store's game is worth
543,739, every member of it survives, and it beat what survives of 4892's game.  Its start stayed at 543,739.

Two runs of `taxman solve 4893 --use-prev` on one thread, a step limit of 2,000, a copy of the chain's results to 4892
as `--out`, decide it:

| bounds store | the search's best at its first step |
|---|---:|
| as committed, with the game of 4893 | 543,739 |
| without the game of 4893 (a copy of the repository's `results/` with that line taken out) | 543,874, the best |

So the start made from 4892's game reaches 4893's best before the first step, in about 30 s on one thread.
`examples/finder.rs --start-only`, which seeds with the game of n − 1 alone, gives the same 543,874.

**How often the store's game lies below the best** (`storegap.py`): of the 52 searched games of the chain from 3915 on
whose board the store holds a game, in 48 that game is the best, one lies 1 to 49 below, and three lie 135 to 153 below:
4875 (153), 4879 (153) and 4893 (135).  At 4879 the game of 4878 won the comparison of survivors, and the search started
at its best.  At 4875 the store's game won, and its start, 537,927 on one thread (537,937 in the chain), beat the
537,693 that the game of 4874 alone gives; the finder lifted it to 538,051 in 15 s.  At 4893 the finder's seed rounds
started from the store's game too, and found nothing in 60 s.  The store holds a game for 80 of the 106 boards from 4895
to 5000, so the trap is set again wherever its game is a local optimum below the best and keeps more than the game
of n − 1.

**What the start cost 4893: about three times the steps.**  A fresh search of 4893 from the store's start, on one
thread, stopped at 200,696 steps with its best still 543,739 (`taxman solve 4893 --use-prev --threads 1 --node-limit
200000 --finder-after 0`, on a copy of the chain's results to 4892).  `tune units` cut 66 units from its checkpoint
(`units-4893.jsonl`), and `runpairs.sh` solved 23 of them, of gaps 406 to 1,374 against the start, on one thread each:
once from the checkpoint's incumbent, the start, and once from the known optimum, with a limit of 300 CPU s for the 14
smaller and 600 s for the 9 larger (`pairs.py`, `pairs.jsonl`).  Every run from the start kept 543,739: these subtrees
hold no better game, so the start costs them pure proof.

* The 21 units that finished both ways took 699,629 steps and 1,542 CPU s from the start, and 199,599 steps and 484
  CPU s from the optimum: 3.51 times the steps and 3.18 times the CPU.  The ratio of a unit lies between 1.44 and 5.70,
  median 3.2.  The two that the limit stopped from the start (4893-5 and 4893-15, gaps 1,374 and 1,324) took at least
  2.03 and 1.48 times.
* The notebook's line summed over all 66 units gives the same factor: 1.66 × 10⁸ steps against the start and 5.5 ×
  10⁷ against the optimum (3.0); the fit of 4880 gives 3.4.
* The chain's search of 4893 raised its incumbent itself, most of it after 418 million of its 614 million steps, so a
  start at the best would have saved somewhat less than two thirds of its steps: about 400 million steps, or two of its
  3.3 hours (an estimate).

**A finder of 15 minutes finds the best from the store's start.**  The same search with `--finder-after 1
--finder-seconds 900`, on one thread beside the finder's own, at nice 19 on the loaded machine (`e4.txt`): the finder
started from 543,739, reached 543,789 and 543,833 after 194 s (round 13, from the seed's survivors), 543,862 after 607 s
(round 40, from the incumbent) and the best, 543,874, after 611 s (round 41, from the seed).  The chain's finder
runs 60 s, four rounds of 15 s; at 4893 the best was ten minutes of rounds away.

## 2026-10-22: the rule of both seeds, a prototype

**The rule.**  Make the start from each seed the search has, the game of n − 1 and the store's game of n, each through
the whole of `search::start_incumbent` (its survivors, then the dive and the local search), and begin the search from
the best set any of them reached.  The cost is one more start a game: a start took 26.5 to 32.3 s on one thread at nice
19 on the loaded machine (`examples/finder.rs --start-only`, games 4875 to 4895, `starts.txt`), and the chain runs the
local search's refills on all the search's threads.  A game's own search takes 55 minutes to 3.3 hours.

**Why it is exact.**  The rule changes only which promotable set the search starts from.  Every set that
`start_incumbent` returns can be promoted, and `search::offer_found` checks it (`Prepared::can_promote` and its sum)
before the search takes it.  The search is exhaustive from any promotable incumbent: a node is cut only when its bound,
an upper bound on its completions ([search.md, "The default step"](../../../docs/design/search.md#the-default-step)),
is below the node's need, and a heavier incumbent raises the need only to what a set already held reaches.  The finder's
offers and the window's seeds rest on the same argument ([search.md, "The
finder"](../../../docs/design/search.md#the-finder)).  A start that reaches the ceiling proves the game by the ceiling,
as any start does.

**The prototype** (the local branch `research-wall` of this entry's clone on the chain's machine, not pushed):
`SearchJob::search` in `src/solver.rs`, before the search, makes the start of each seed it did not choose, with a
control of its own and the ceiling, and offers it through `offer_found`.  On one thread, `--use-prev`, the committed
store and a step limit of 1,000 (`starts.txt`):

| game | the seed chosen (survivors) and its start | the other seed's start | the search's first best | the best |
|---|---|---:|---:|---:|
| 4875 | the store's, 537,927 | 537,693 (n − 1) | 537,927 | 538,080 |
| 4879 | n − 1's, 540,552 | 540,399 (the store) | 540,552 | 540,552 |
| 4893 | the store's, 543,739 | 543,874 (n − 1) | 543,874 | 543,874 |

At 4875 and 4879 the rule changes nothing; at 4893 the search starts at its best, where the chain's started 135 below
and took 614 million steps.  At 4880, 4884, 4888 and 4890 the store's game is the best, and nothing changes either.

**Small boards that set the same trap** (`examples/seeds.rs --scan` of the local branch, `seeds-scan.jsonl`).  The store
holds no game below 3915, so the scan makes one: for each board n of 2 to 1300 with a game of n − 1 in
`optimal.json`, the start made with no seed (greedy, dive, local search), a local optimum of the local search, stands in
for the store's game.  Of 1,280 boards with both seeds, in 231 the stand-in keeps more survivors than the game of n − 1,
so the rule of bcfde26 picks it, and in 53 of those its start is lower than the start from n − 1's game, by 5 to 214
points.  The smallest is n = 187: the game of 186 keeps 748 and its start reaches 762, the stand-in is worth 751, and
the rule starts at 751.  The largest margins are 1197 (214), 1035 (176) and 1190 (143).  These boards make the test
that fails without the rule, and the builder has them.

**Boards ahead of the chain** (`examples/seeds.rs 4895 5000 --proxy`, `seeds-ahead.jsonl`).  Past 4895 the chain has
no game of n − 1 yet, so the store's game of n − 1 stands in for it.  The ledger made the store's games one from
another, so neighbouring games are often the same set and the stand-in is weak: the chain's own game of n − 1 can do
better or worse.  Of 106 boards, the store's game of n keeps more survivors on 25, and on 5 of those its start is lower
than the start from the stand-in: 4920 by 128 (552,041 against 552,169), 4940 by 182 (554,052 against 554,234), 4944 by
17, 4968 by 33 and 4983 by 29.  At 4895, the one board with the chain's game of n − 1, the game of 4894 keeps 542,539
against the store's 542,404, and the rule of bcfde26 already took it.  A start took a median 27 s and at most 63 s on
one thread at nice 19.

**The fix on master** (fea57c3) is the rule: `start_incumbent` takes every seed, makes the greedy set and the dive
once, runs the local search from each distinct set the seeds' starts reached, and keeps the best, the store's game
first among equals.  The finder's `Plan` holds the seeds too, and its odd rounds take them in turn.  A resumed search
takes no seeds, so its finder has the game of n − 1 alone.  Built from master at fa6bfdc and run as the prototype was
(`starts.txt`), it starts 4875 at 537,927, 4879 at 540,552 and 4893 at 543,874, the prototype's starts.

## 2026-10-22: the finder's seeds, and its threads

**The finder needs both seeds.**  `examples/finder.rs` of the local branch takes `--store`, the store's game as the
only seed, as the rule of bcfde26 chose at 4875 and 4893, and ends a run once the incumbent reaches `--target`
(`finderruns.sh`, the runs in `finder-runs.jsonl`).  One thread a run, at nice 19, beside the chain:

* 4875 from the store's start, 537,927: four runs of 600 s found nothing better at all, toward the best 538,080.
* 4875 from the start of the game of 4874 alone, 537,693: four runs reached 538,080 in 1.2, 2.6, 2.6 and 10.0 s.

So at 4875 only the rounds from the game of n − 1 find the best, where at 4893 the store's start was the trap and the
game of n − 1's start the best.  The fix on master covers both: its start keeps the better start, and its finder's odd
rounds alternate between the two seeds.

**After the fix, neither seed's start reaches the best in 2 of 52 games** (`pastseeds.py`, `seeds-past.txt`; the
starts by `examples/seeds.rs N N --known`, one thread, no ceiling).  On the 52 searched games of the chain from 4548 to
4893 whose board the store holds a game of, the start of the game of n − 1 alone reaches the best in 38, the store's
game alone in 48, the rule of bcfde26 in 49 and the better of the two in 50.  The two left are 4864 (4 below) and 4875
(153 below, where the finder's rounds from the game of 4874 find the best in seconds).  On a board without a store
game, 26 of the 106 from 4895 to 5000, the game of n − 1 is the only seed, and its start reached the best in 38 of the
52 games.

**More threads for the finder save minutes on the rare game that needs it.**  The time for the first of k independent
runs to reach the best (`firstofk.py`, the mean over every k of the runs; a run the budget stopped counts its budget)
approximates a finder of k threads, and understates its gain, since threads would share their improvements:

| start | runs | k = 1 | k = 2 | k = 4 | k = 8 | k = 16 |
|---|---:|---:|---:|---:|---:|---:|
| 4893 from the store's start, 135 below (900 s each) | 16 | 433 s | 268 s | 171 s | 120 s | 82 s |
| 4890 with no seed, 323 below (600 s each) | 8 | 243 s | 118 s | 54 s | 24 s | |
| 4875 from the game of 4874, 387 below (300 s each) | 4 | 4 s | 2 s | 1 s | | |

At 4893, 3 of the 16 runs had not reached the best after 900 s, and 13 had, after 82 to 798 s; at 4890 one of 8 had
not after 600 s.  Eight threads would cut the time to the best by three to ten times.  What that buys is small: during
the minutes at the low incumbent the search does about two thirds of its work in vain (the subtrees of 4893 took 3.5
times the steps from 135 below), so eight threads save about two to four minutes of a game's wall time, and only in
the games where neither seed's start is the best, 2 of 52 above.  A finder of many threads is not worth building now;
the finder of 900 s on one thread covers those games, at the cost of one thread for 15 minutes.

## 2026-10-22: the threads the chain leaves idle, and heavy units

**Whether 88 threads beat 72 is not settled by the chain's lines.**  The machine has 44 cores of two hyperthreads each.
The chain ran on all 88 threads until 03:10 and on 72 since, and the lab's 16 stand idle when no study runs.  The rates
of the progress lines (`timeline.txt`, from `timeline.py chain.err 19`) do not separate the threads from the phase of a
search:

* 4875, the one search wholly on 88 threads, ran at 55,300 steps a second; the six on 72 at 46,900 to 52,700.  The
  games differ.
* 4879 ran its first 57.4 million steps on 88 threads at 49,800 a second, and the rest on 72 at 53,800.
* The first 19 minutes of a search run 2 to 16% slower than the rest on the same threads (4888: 43,100 against 49,600),
  so 4879's two parts differ in phase as well as in threads.

Hyperthreads on this kind of search may add a little or nothing.  The lab channel's command `threads` (`total`) changes
the run's budget while it runs, so 72 and 88 can alternate in blocks of 15 minutes within one long game and be compared
in the same phase; the gain at stake is a few percent of every game.  Since 551f883 (`--background`) the chain's
workers run at idle priority and take every thread the lab leaves, so the rates of its lines from then on, with and
without a study running, measure the same question.

**A unit's depth or candidates do not predict its steps beyond its gap** (`residuals.py`, `residuals.txt`).  On the 82
finished runs of units of 4875, 4880, 4884 and 4893 (the studies' runs and this entry's pairs), the residual from the
notebook's line, log10 steps less 0.511 + 0.356 · gap/100, correlates 0.23 with the depth and −0.22 with the number of
candidates.  The game's own offset dominates: the median residual is +0.34 at 4880, +0.36 at 4884 and −0.16 at 4893.

## 2026-10-22: what the findings add up to

The chain's wall time is its steps over its rate; no thread idles at the end of a game.  The one large waste found is a
start below the best, and it is mostly mended:

1. **Both seeds** (fea57c3, on master).  At 4893 the start would have been the best, and the search about 400 million
   steps shorter, two of its 3.3 hours by estimate.  Of the 52 past games with a store game, the start reaches the best
   in 50 against 49 before.  Ahead, 5 of 106 boards show the same trap against the store's own game of n − 1.  The
   check: each game's line from now on starts at least at both seeds' starts, and 4920 and 4940 are the boards to watch.
2. **A finder of 900 s** (`--finder-seconds 900`).  It covers the games whose start is below the best from both seeds,
   2 of 52, and a start like 4893's from the store, which one thread left in 82 to 798 s in 13 of 16 runs.  Its cost is
   one thread for 15 minutes a game.
3. **No finder of many threads.**  Eight threads would save two to four minutes on those 2 games in 52.
4. **72 against 88 threads.**  Unsettled by the lines so far; with `--background` the chain's own lines with and
   without a study running will tell.
5. **The window's hints.**  A speculative search given a better seed of n − 1 is offered only its survivors
   (`Job::hint`), not a start made from it.  At 4895 the survivors of 4894's final game reached what the full start
   gives, so nothing was lost there; a start made from the final seed would close the gap where they do not.

## Files

On the machine of the chain, the runs are in `~/taxman-lab/work/research-wall/` and the prototypes on the local branch
`research-wall` of the clone there (963a2b8, not in the repository): the start of the seed not chosen offered before the
search (`src/solver.rs`), a floor for `tune` (`TAXMAN_TUNE_FLOOR`, not used in the end), `examples/finder.rs` with
`--store` and the stop at `--target`, and `examples/seeds.rs`.

* `timeline.py`, `timeline.txt`: the games' steps, own search, rates, starts, efficiency and tail from the chain's
  progress lines (`timeline.py chain.err 19`).
* `storegap.py`: how far the store's game lies below each searched game's best (`storegap.py bounds.jsonl
  strict.jsonl`).
* `units-4893.jsonl`, `runpairs.sh`, `pairs.py`, `pairs.jsonl`: the 66 units of the one-thread search of 4893 from the
  store's start, and the 23 solved from the start and from the best (`runpairs.sh TUNE 4893 units-4893.jsonl
  strict.jsonl OUT JOBS LIMIT IDS`; `pairs.py units-4893.jsonl pairs.jsonl`).
* `e4.txt`: the finder of 900 s beside the search of 4893 from the store's start.
* `starts.txt`: the prototype's starts at 4875, 4879 and 4893, the starts from the game of n − 1 alone, and those of
  master at fa6bfdc.
* `seeds-scan.jsonl`: the 231 boards of 2 to 1300 where the stand-in store game keeps more survivors than the game of
  n − 1 (`seeds 2 1300 --scan`; fields `prev_*` for the game of n − 1, `other_*` for the stand-in, `chain` the rule of
  bcfde26, `both` the better start).
* `seeds-ahead.jsonl`: the boards 4895 to 5000 (`seeds 4895 5000 --proxy --known strict.jsonl`).
* `seeds-past.jsonl`, `pastseeds.py`, `seeds-past.txt`: the 52 searched games with a store game (`seeds N N --known
  strict.jsonl`; `pastseeds.py strict.jsonl seeds-past.jsonl`).
* `finderruns.sh`, `finder-runs.jsonl`, `firstofk.py`: the finder alone, a random seed a run, and the first of k
  (`finderruns.sh FINDER JOBS OUT GAME TARGET SECONDS SEEDS --known strict.jsonl [--store | --no-seed]`;
  `firstofk.py finder-runs.jsonl`).
* `residuals.py`, `residuals.txt`: the units' residuals from the notebook's line (`residuals.py` over the snapshots'
  units and runs, and `units-4893.jsonl` with `pairs.jsonl`).

## Commits

* fea57c3: a search with several seeds starts from the best start of any of them, and the finder's odd rounds take
  the seeds in turn (built from this entry's rule).
