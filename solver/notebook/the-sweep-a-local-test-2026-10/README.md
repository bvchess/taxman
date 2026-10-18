# The sweep: a local test (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

**Question.**  Brian's plan of 2026-10-17 is as many proven games of 3441..8191 as one cheap machine gives, holes
allowed: blocks of consecutive games, one thread each, a search of at most 180 s a game, a game that does not finish
leaves its best game as a seed for the next, and passes over the merged file so that each game sees the game before it
whichever block made it ([RUNBOOK section 13](../../tools/cloud/RUNBOOK.md#13-the-sweep);
`taxman solve --keep-going`).  Does it do that on a range small enough for a laptop, and does a block's first game end
as it would in one unbroken chain when the game before it is proven?

**What was run.**  `rust/tools/cloud/sweep.sh` on the laptop (ten CPUs) with the binary of `cargo build --profile test`
(opt-level 2, debug assertions on), the committed games to 1300 or to 1410 as `KNOWN`, and `tools/cloud/test/run.sh
sweep`, which runs it against the stand-in for aws and the real `verify.sh` (both referees, and the agreement with
the committed chain).

* **1301..1340, four blocks of ten (`BLOCKS=4`), one second a game.**  The first block chained from the committed games.
  Pass 1: the four blocks ended after 4, 6, 10 and 12 s.  22 of the 40 games completed (11 exhausted searches, 7
  repeats, 4 prime games); 18 ended with an unfinished line, 1320..1322 and 1326..1340.  Pass 2 searched again the two
  blocks whose first game had no game before it (1321 and 1331; the game before each was unfinished too, not proven)
  and added 2 lines, both unfinished again, in 2 s.  Pass 3 found nothing to do, so the games were as an unbroken chain
  at that limit would have left them.  `verify-results` accepted the merged file in 10 s (most of it the committed chain
  it is compared with) and `check_results.py` in 4 s; every completed score equals the committed chain's.
* **The boundary the passes are for: 1411..1416, two blocks (1411..1412, 1413..1416, the plan written by hand), 0.3 s a
  game,** after the committed games to 1410.  The chain settles some games at the ceiling its game before gave, with no
  search; alone they cost a search.  Of the nine games of 1301..1500 whose committed line is a `cap` proof with 0 nodes
  and 0.08 s, run alone for a second: 1335 took 18,499 steps, 1341 16,073, 1344 14,991 (finished), 1359 5,998 (finished),
  1372 12,226, 1413 10,340, 1428 9,917, 1455 and 1467 9,688, and seven of the nine did not finish.  So 1413, the first
  game of its block with no game before it, was an unfinished line in pass 1 (2,951 steps in 0.45 s, as were 1414..1416,
  built on the best game of the one before: 3,012, 2,980 and 2,723).  In pass 2 it had 1412 proven (a repeat of the
  first block), and it completed at once with the `cap` proof, 0 nodes in 0.01 s; 1414 and 1415 followed as repeats of
  it in the same process, each on the game just proven; 1416, which the chain searches for 4.4 s, was unfinished again
  (2,435 steps in 0.4 s), as it is in a chain at that limit.  Pass 3: nothing.  Both referees accepted the file.
* **A SIGTERM during 1301..1340** ended the sweep with status 143 and the games done merged (no line for the game in
  progress); the same command went on to the end and both referees accepted the file.  A second run of the command on
  a finished sweep left the merged file byte for byte as it was.

**The sizes of the real run** (`tools/sweep_plan.py blocks 3441 8191 64 180`; computed, not run): 4,751 games in 64
blocks, the first of 95 games (3441..3535) and the last of 50 (8142..8191), each of 4.83 to 4.97 core-hours of expected
work.  The expected work of a game is its setup (5.1 s at 3441, 22.6 at 5000, 90.7 at 7000, 173.5 at 8191 by the power
laws through the four measured points: 1 s at 2200, 3 at 3000, 9 at 4000, 48 at 6000) and the limit of its search: 76
core-hours of setup and at most 238 of search, 314 in all, 4.9 hours on 64 CPUs, about $3.3 at the spot price of
2026-10-17 (RUNBOOK 13.2).

**Findings.**

* The three things that rest on opt(n − 1), the ceiling, the prime formula and the tripwire of a score above the
  ceiling, are refused on an unfinished game and kept on a proven one; both verifiers refuse a line that claims
  otherwise (`tests/keep_going.rs`, `tests/results.rs`).  Proofs that rest on nothing of n − 1 stand.
* A search started again can end below an earlier one (a restart has no checkpoint's incumbent).  The seed is therefore
  the best of a game's unfinished lines, and what the game "had" from the game before it is what its latest attempt had;
  with the best attempt's record a game could look improved for ever.  The planner and the solver agree on both.
* No checkpoints: a game costs at most `GAME_SECONDS`, a game searched again starts again, and a checkpoint cannot
  block the prime formula or a repeat (a game with a checkpoint is resumed, never derived).  A pass is one process a
  block over an excerpt of the merged file: the solver checks the file it opens, and every line in it needs the game
  before it, so the excerpt is the block, the 8 board sizes before it and what their lines rest on.

**Not measured.**  What share of 3441..8191 finishes in 180 s (most will not); the setup past 4000 (extrapolated from
four points); the passes at the scale of 64 blocks; anything on EC2.

**Commits.**  The branch with `--keep-going`, `--retry`, `--retry-improved` and `--pass` in `taxman solve`, the referees'
checks of unfinished lines, `sweep.sh` and `tools/sweep_plan.py`.
