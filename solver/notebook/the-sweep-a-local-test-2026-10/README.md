# The sweep: a local test (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-17.  Concluded 2026-10-17.

**Question.**  Does a sweep (blocks of consecutive games, one thread each, a search of at most 180 s a game, passes over
the merged file) work on a range small enough for a laptop, and does a block's first game end as it would in one
unbroken chain when the game before it is proven?  **Answer.**  Yes: on 1301..1340 three passes left the games as an
unbroken chain at that limit would have, with every completed score equal to the committed chain's, and on 1411..1416
pass 2 completed 1413 at once with the `cap` proof once 1412 was proven; both referees accepted both files.

## Method

The project owner's plan of 2026-10-17 is as many proven games of 3441..8191 as one cheap machine gives, holes allowed.
A game that does not finish leaves its best game as a seed for the next, and passes over the merged file give each game
the game before it, whichever block made it ([the runbook's page on the sweep](../../docs/runbook/sweep.md);
`taxman solve --keep-going`).

The test ran `solver/tools/cloud/sweep.sh` on the laptop (ten CPUs) with the binary of `cargo build --profile test`
(opt-level 2, debug assertions on) and the committed games to 1300 or to 1410 as `KNOWN`.
`solver/tools/cloud/test/run.sh sweep` ran it against the stand-in for aws and the real `verify.sh`, which runs both
referees and compares the scores with the committed chain.

## Results

**1301..1340, four blocks of ten (`BLOCKS=4`), one second a game.**  The first block chained from the committed games.

* **Pass 1:** the four blocks ended after 4, 6, 10 and 12 s.  22 of the 40 games completed (11 exhausted searches, 7
  repeats, 4 prime games); 18 ended with an unfinished line, 1320..1322 and 1326..1340.
* **Pass 2** searched again the two blocks whose first game had no game before it (1321 and 1331; the game before each
  was unfinished too, not proven).  It added 2 lines, both unfinished again, in 2 s.
* **Pass 3** found nothing to do.
* **The referees:** `verify-results` accepted the merged file in 10 s (most of it the committed chain it is compared
  with) and `check_results.py` in 4 s; every completed score equals the committed chain's.

**1411..1416, two blocks (1411..1412 and 1413..1416, the plan written by hand), 0.3 s a game,** after the committed
games to 1410.  This is the boundary the passes are for.  The chain settles some games at the ceiling the game before
gave, with no search, and alone they cost a search.  Of the nine games of 1301..1500 whose committed line is a `cap`
proof with 0 nodes and 0.08 s, run alone for a second, seven did not finish:

| game | 1335 | 1341 | 1344 | 1359 | 1372 | 1413 | 1428 | 1455 | 1467 |
|---|---|---|---|---|---|---|---|---|---|
| steps | 18,499 | 16,073 | 14,991 (finished) | 5,998 (finished) | 12,226 | 10,340 | 9,917 | 9,688 | 9,688 |

* **Pass 1:** 1413, the first game of its block with no game before it, was an unfinished line (2,951 steps in 0.45 s).
  So were 1414..1416, built on the best game of the one before (3,012, 2,980 and 2,723 steps).
* **Pass 2:** 1413 had 1412 proven (a repeat of the first block), and it completed at once with the `cap` proof, 0 nodes
  in 0.01 s.  1414 and 1415 followed as repeats of it in the same process, each on the game just proven.  1416, which
  the chain searches for 4.4 s, was unfinished again (2,435 steps in 0.4 s), as it is in a chain at that limit.
* **Pass 3:** nothing.  Both referees accepted the file.

**A SIGTERM during 1301..1340** ended the sweep with status 143 and the games done merged (no line for the game in
progress).  The same command went on to the end, and both referees accepted the file.  A second run of the command on a
finished sweep left the merged file byte for byte as it was.

## Decision

Built and tested, and not run on EC2.  The tests fixed three points of the design:

* **Nothing rests on an unfinished game.**  The three things that rest on opt(n − 1), the ceiling, the prime formula and
  the tripwire of a score above the ceiling, are refused on an unfinished game and kept on a proven one; both verifiers
  refuse a line that claims otherwise (`tests/keep_going.rs`, `tests/results.rs`).  Proofs that rest on nothing of n − 1
  stand.
* **The seed is the best of a game's unfinished lines.**  A search started again can end below an earlier one, since a
  restart has no checkpoint's incumbent.  What the game "had" from the game before it is what its latest attempt had;
  with the best attempt's record a game could look improved for ever.  The planner and the solver agree on both.
* **No checkpoints.**  A game costs at most `GAME_SECONDS`, a game searched again starts again, and a checkpoint cannot
  block the prime formula or a repeat (a game with a checkpoint is resumed, never derived).  A pass is one process a
  block over an excerpt of the merged file: the solver checks the file it opens, and every line in it needs the game
  before it, so the excerpt is the block, the 8 board sizes before it and what their lines rest on.

**Not measured.**  What share of 3441..8191 finishes in 180 s (most will not); the setup past 4000 (extrapolated from
four points); the passes at the scale of 64 blocks; anything on EC2.  A run on EC2 would answer these.

### Later (2026-10): the cloud sweep of 3441..8191, as the runbook planned it

The runbook's page on the sweep held the plan of this campaign until its guide was made general (2026-10-20).  The
project owner chose 3441 to 8191 for it (2026-10-17); the chain had stopped at 3441, whose search took 2.8 hours on 64
threads.  The plan, as it stood:

**What it would cost: about $3.3 for pass 1.**  By `solver/tools/sweep_plan.py blocks 3441 8191 64 180` (computed, not
run), 3441..8191 at `GAME_SECONDS` 180 is 4,751 games in 64 blocks, the first of 95 games (3441..3535) and the last of
50 (8142..8191), each of 4.83 to 4.97 core-hours of expected work.  The expected work of a game is its setup and the
limit of its search.  The setup is 5.1 s at 3441, 22.6 at 5000, 90.7 at 7000 and 173.5 at 8191 by the power laws through
the four measured points (1 s at 2200, 3 at 3000, 9 at 4000, 48 at 6000).  Together that is 76 core-hours of setup and
at most 238 of search, so at most 314 core-hours, 4.9 hours of 64 CPUs.  A `c8g.16xlarge` at spot cost $0.66 to $0.81 an
hour on 2026-10-17, counted at $0.676 as in [What needs approval, and what it
costs](../../docs/runbook/wall.md#what-needs-approval-and-what-it-costs), so pass 1 costs about $3.3.  The later passes
cost a fraction of that, since they search again only the games whose game before has gained.  A deadline of 8 hours
(`DEADLINE_HOURS`) bounds the run at about $5.4 at $0.676 an hour ($6.5 at $0.81), and the timer of 9 hours bounds the
machine at $9 at the ceiling of $1.0; the 40 GB volume costs a few cents.  The launch builds nothing: it takes the
solver from the build cache, which must already hold the commit ([Launch, watch, stop, bring the results
home](../../docs/runbook/sweep.md#launch-watch-stop-bring-the-results-home)).  Releasing a commit is a launch of its
own, $0.15 to $0.25 ([The release: the suite, then the
cache](../../docs/runbook/build-cache.md#the-release-the-suite-then-the-cache)).

*Details.*  The estimate is an upper bound for pass 1: a game that a repeat or the prime formula settles costs its setup
alone.  The setup times past 4000 are extrapolated from four measured points.  Nobody has measured what finishes in 180
s past 3441.  Most searches will not, since the cost of a game grows tenfold in about 1,040 in n, so expect most lines
unfinished, with the proofs among the primes, the repeats after a proven game and the easy searches.

**The launch: one instance, not a group,** after the approval of whoever owns the AWS account.  Its user data runs the
sweep by itself: it sets the timer, installs, clones the pinned commit, sets up from the build cache, restores what S3
holds, and runs the sweep in tmux as `ec2-user`.  `POWEROFF=always` shuts the machine down when the sweep ends, for any
reason, and the timer is the backstop.  Launch as [Launch](../../docs/runbook/launch.md#launch) says, with these values,
`ZONE` the zone of the lowest price that day ([machine.md](../../docs/runbook/machine.md)) and `MAX_PRICE` the ceiling
of the run to the wall:

```
INSTANCE_TYPE=c8g.16xlarge ZONE=... VOLUME_GB=40 PURPOSE=sweep MAX_PRICE=1.0
```

Pass `--user-data file://sweep-user-data.sh` in place of the `--user-data` argument of
[launch.md](../../docs/runbook/launch.md#launch): the file sets the timer itself, so `TIMER_MINUTES` is not needed.
`sweep-user-data.sh` is, with `COMMIT` the id of the pushed master:

```
#!/bin/bash
shutdown -h +540                                    # the timer: DEADLINE_HOURS below, and an hour
dnf install -y git tmux || { shutdown -h now; exit 1; }
sudo -u ec2-user git clone https://github.com/bvchess/taxman.git /home/ec2-user/taxman
sudo -u ec2-user git -C /home/ec2-user/taxman checkout COMMIT
sudo -u ec2-user -i tmux new-session -d -s tx -c /home/ec2-user/taxman \
    'export S3_DEST=s3://taxman-013764204840-us-east-1/sweep-3441-8191
     BUILD="--bin taxman" REQUIRE_CACHED=1 SKIP_TESTS=1 NO_PULL=1 solver/tools/cloud/setup.sh &&
         { solver/tools/cloud/save.sh --restore; DEADLINE_HOURS=8 POWEROFF=always GAME_SECONDS=180 \
               solver/tools/cloud/sweep.sh 3441 8191; }
     sudo shutdown -h now'                          # also if the setup failed
```

Its results would come home with
`aws s3 sync s3://taxman-013764204840-us-east-1/sweep-3441-8191/results/ results-from-cloud/`.

## Files

The folder holds this README.md alone.  The tools it tested live in `solver/tools/`: `cloud/sweep.sh`, `sweep_plan.py`
and `cloud/test/run.sh sweep`.

## Commits

* c51dbf2: `solve --keep-going` with `--retry`, `--retry-improved` and `--pass` (a game that a limit stops no longer
  ends the range), and the referees' checks of unfinished lines.
* 8046a78: `sweep.sh`, the passes and `sweep_plan.py`.
* 8f00dfd: the merge of the sweep.
* 29ac93c: the documentation of the sweep and of `--keep-going`, with this entry.
