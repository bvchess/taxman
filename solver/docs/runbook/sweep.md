# The sweep (`sweep.sh`)

Part of the [runbook](../RUNBOOK.md).

`sweep.sh FIRST LAST` proves as many games of a long stretch as a cheap machine allows, with holes, on one machine.  The
chain of [chain.md](chain.md) proves every game in turn and stops at the first it cannot.  The sweep instead gives each
game at most `GAME_SECONDS` (default 180) of one thread, goes on past the games that do not finish, and keeps what does.
A launch needs the approval of whoever owns the AWS account, with the estimate of [What it costs](#what-it-costs)
([rules for every launch](launch.md#rules-for-every-launch)).

## What it does

The sweep runs in passes over blocks of consecutive games, each block on one thread.  The top of `sweep.sh` documents
the passes, the exit statuses and the variables.

* **Pass 1.**  FIRST..LAST is split into `BLOCKS` blocks (default one a CPU), sized so that their expected work is about
  equal.  A game costs its setup and at most the limit of its search, and the setup grows steeply with n: on one thread
  about 1 s at 2200, 3 s at 3000, 9 s at 4000 and 48 s at 6000 (`solver/tools/sweep_plan.py` interpolates).  Each block
  is one `taxman solve --threads 1 --time-limit $GAME_SECONDS --keep-going` over its range, in increasing n, into a file
  of its own.
* **What a game gets from the one before.**  A game whose search finishes is a proof.  One that reaches the limit leaves
  an unfinished line with its best game, and the next game takes that game as a seed and nothing more: the ceiling n +
  opt(n − 1), the prime formula and a repeat of an earlier search come only from a proven game ([README, "Keeping going
  past limits"](../../frame/README.md#keeping-going-past-limits-and-the-sweep)).  A block's first game has no game
  before it and is solved standalone, but for the first block's.  If `KNOWN` (default the committed
  `results/strict.jsonl`) holds a proven game of FIRST − 1, the first block's file starts as a copy of KNOWN's games,
  and the solver goes on from the last, as it extends a chain.
* **Passes 2, 3, ...**  A block's first game began without the game before it, which another block made.  So each pass
  searches again, with the same limit, every game that has only unfinished searches and whose game before it, as the
  merged file holds it now, has more to give than when the game was last tried: it is now proven, or it has a better
  best game than the seed it gave, or it gave none.  A pass runs one `taxman solve --keep-going --retry-improved` a
  block that has such a game, on one thread, in increasing n, so that a game it has just proven helps the next.  Each
  reads an excerpt of the merged file: `sweep_plan.py passes` reads that file once and cuts all the excerpts, each the
  block, the 8 values of n before it that a repeat can repeat, and what their lines rest on, so that the solver's checks
  of its file hold.  Passes repeat until one finds nothing to do, at most `MAX_PASSES` (default 20) of them.  The games
  are then as an unbroken chain would have left them, with what it could prove at this limit.
* **No checkpoints.**  A game that is searched again starts from the start, since a game costs at most `GAME_SECONDS`.

**The files.**  The blocks are merged into `$TAXMAN_WORK/results/explore.jsonl` every `MERGE_EVERY` seconds and at the
end.  The merge drops repeated lines (`awk '!seen[$0]++'`), so it can run any number of times.  Beside it,
`results/explore-blocks/` holds `plan` (the blocks, kept so that a rerun finds them), `block-NNN.jsonl` (each block's
file, which only grows) and `passes` (the last pass made).  `save.sh` copies the directory, so both reach S3.  A pass
works in `$TAXMAN_WORK/sweep-pass/`, which is not saved; a pass that was stopped leaves its lines there, and the next
start merges them first.  Each line says what its game had from n − 1 in `prev`, and which pass made it in
`options.pass` ([REFERENCE.md, "What a game had from n - 1"](../REFERENCE.md#what-a-game-had-from-n---1)).  Pass 1 is
the blocks, and a rerun after an interruption counts as another pass.

**`explore.jsonl` is not a chain.**  It has holes, and games built on unfinished ones, so it never goes into
`results/strict.jsonl`.  [`verify.sh`](chain.md#verifying-and-bringing-the-results-home) accepts it (both referees, and
the agreement of its scores with the committed chain), and `sweep.sh` runs it when the passes are done.

## What it costs

**Pass 1 costs at most its core-hours.**  `solver/tools/sweep_plan.py` sizes the blocks from the setup times above and
the limit: a game costs its setup and at most `GAME_SECONDS` of search, so FIRST..LAST costs at most the sum of its
setups and its searches, in core-hours, over the machine's CPUs, at the spot price of the hour
([machine.md](machine.md#quota-prices-and-spot-interruptions)).  The later passes cost a fraction of that, since they
search again only the games whose game before has gained.  `DEADLINE_HOURS` bounds the run, and the timer, an hour
longer, bounds the machine at the price ceiling; a 40 GB volume costs a few cents.  The launch builds nothing: it takes
the solver from the build cache, which must already hold the commit; releasing a commit is a launch of its own, $0.15
to $0.25 ([The release: the suite, then the cache](build-cache.md#the-release-the-suite-then-the-cache)).

*Details.*  The estimate is an upper bound for pass 1: a game that a repeat or the prime formula settles costs its setup
alone.  The setup times past 4000 are extrapolated from four measured points.  Past the end of the committed chain most
searches will not finish in 180 s, since the cost of a game grows tenfold in about 1,040 in n, so expect most lines
unfinished, with the proofs among the primes, the repeats after a proven game and the easy searches.

## Launch, watch, stop, bring the results home

**Launch one instance, not a group,** after the account owner's yes.  Its user data runs the sweep by itself: it sets
the timer, installs, clones the pinned commit, sets up from the build cache, restores what S3 holds, and runs the sweep
in tmux as `ec2-user`.  `POWEROFF=always` shuts the machine down when the sweep ends, for any reason, and the timer is
the backstop.  Launch as [Launch](launch.md#launch) says, with `ZONE` the zone of the lowest price that day
([machine.md](machine.md)); one `c8g.16xlarge` at spot suits a stretch of a few thousand games:

```
INSTANCE_TYPE=c8g.16xlarge ZONE=... VOLUME_GB=40 PURPOSE=sweep MAX_PRICE=1.0
```

Pass `--user-data file://sweep-user-data.sh` in place of the `--user-data` argument of [launch.md](launch.md#launch):
the file sets the timer itself, so `TIMER_MINUTES` is not needed.  `sweep-user-data.sh` is, with `COMMIT` the id of the
pushed master, `HOURS` the deadline, `MINUTES` the timer (60 × (`HOURS` + 1)) and `PREFIX` the run's own prefix of the
bucket (`sweep-FIRST-LAST`):

```
#!/bin/bash
shutdown -h +MINUTES                                # the timer: DEADLINE_HOURS below, and an hour
dnf install -y git tmux || { shutdown -h now; exit 1; }
sudo -u ec2-user git clone https://github.com/bvchess/taxman.git /home/ec2-user/taxman
sudo -u ec2-user git -C /home/ec2-user/taxman checkout COMMIT
sudo -u ec2-user -i tmux new-session -d -s tx -c /home/ec2-user/taxman \
    'export S3_DEST=s3://taxman-013764204840-us-east-1/PREFIX
     BUILD="--bin taxman" REQUIRE_CACHED=1 SKIP_TESTS=1 NO_PULL=1 solver/tools/cloud/setup.sh &&
         { solver/tools/cloud/save.sh --restore; DEADLINE_HOURS=HOURS POWEROFF=always GAME_SECONDS=180 \
               solver/tools/cloud/sweep.sh FIRST LAST; }
     sudo shutdown -h now'                          # also if the setup failed
```

* **Setup.**  `setup.sh` takes the solver from the build cache and runs its smoke checks.  With `REQUIRE_CACHED=1` the
  commit must have been released ([The release: the suite, then the
  cache](build-cache.md#the-release-the-suite-then-the-cache)), or the machine ends at once.  Without it, a miss builds
  the solver with `target-cpu=native`, a few minutes on 64 vCPUs, untested.
* **Watch.**  `tail ~/taxman-work/logs/sweep.log` shows a status line every `STATUS_EVERY` seconds: the pass, the blocks
  done, the lines in the merged file.  `~/taxman-work/logs/sweep-*.err` is a block's own log.
* **Stop** cleanly with `kill -TERM $(cat ~/taxman-work/sweep.pid)`: the games in progress have no line, and the others
  are merged and saved.  **Go on** after a stop or an interruption with the same command; on a replacement machine, run
  `save.sh --restore` first, with the same `S3_DEST`.  The plan is in the saved directory, so the sweep finds the same
  blocks.
* **Bring the results home** and check them:

  ```
  aws s3 sync s3://taxman-013764204840-us-east-1/PREFIX/results/ results-from-cloud/
  solver/tools/cloud/verify.sh results-from-cloud/explore.jsonl
  ```

  `taxman verify-results results-from-cloud/explore.jsonl` reports the completed games, the missing ones, the games that
  only an unfinished search is recorded for, and the proofs.
* **Spend more on the failures** by hand: search them again with a longer limit,
  `taxman solve A-B --keep-going --retry --time-limit 600 --out BLOCKFILE` on a block's file, and run `sweep.sh` again.
  It merges the new lines, and its passes search again the games next to the ones that improved.
* **End** with the checks of the [rules for every launch](launch.md#rules-for-every-launch); the bucket's prefix goes
  once the results are home.

*Details.*  The tests of the script are `test/run.sh sweep` and, for the planner,
`python3 -I solver/tools/test_sweep_plan.py`.  `test/run.sh sweep` runs `verify.sh`, so it replays the committed games
three times; set `TAXMAN_BIN` to a non-release build if there is no release build.

## An example

[The sweep's local test](../../notebook/the-sweep-a-local-test-2026-10/README.md) ran it on the laptop, and [its later
section](../../notebook/the-sweep-a-local-test-2026-10/README.md#later-2026-10-the-cloud-sweep-of-34418191-as-the-runbook-planned-it)
has the plan and the estimate of a cloud sweep of 3441..8191.
