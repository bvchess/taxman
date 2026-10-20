# The ledger's experiment

Part of the [runbook](../RUNBOOK.md).

`solver/tools/ledger/cloud.sh FIRST-LAST` runs the ledger over a stretch of games for a budget of CPU, with the truth
hidden from it, on one `c8g.16xlarge` at spot.  The ledger (`examples/ledger/`; [README, "Keeping going past
limits"](../../frame/README.md#keeping-going-past-limits-and-the-sweep)) proves games from both sides, bounds from below
and above, in the order that its scheduler judges cheapest.  The experiment asks whether it proves much of a stretch
that the chain has proven, for a small share of what the chain spent there: on 2176..3440, the run to the wall re-proved
the stretch in about 4.4 hours of 64 threads, about 280 core-hours.  [The ledger's
entry](../../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md) records the laptop's dry run of 1301..1500,
every launch and what it found, and how the scheduler works.  A launch needs Brian's approval with the cost below
([rules for every launch](launch.md#rules-for-every-launch)).

`cloud.sh` runs `ledger run` with no proven game, then `ledger report --known results/strict.jsonl`.  The report exits
with status 1 if a bound is on the wrong side of the chain's score or a game does not replay: that is a bug, so stop and
report it.  The script's comment documents its environment.

**Cost.**  About 1.3 instance-hours, **about $0.9 at spot** ($0.68 an hour) and about $3.3 on demand: the setup (a build
of about 10 minutes, or none with the commit's binaries in the build cache, [build-cache.md](build-cache.md)), the run
(`CPU_HOURS` 64, an hour of the 64 CPUs, and the searches still going when it is spent, each limited to the budget left
over the workers), and the report (a minute).  The timer of 150 minutes bounds it at about $1.7 at spot, and the 40 GB
volume costs a few cents.

**Launch** as [Launch](launch.md#launch) says, with these values, `ZONE` the zone of the lowest price that day
([machine.md](machine.md)) and the ceiling of [Launch, watch, stop, bring the results
home](sweep.md#launch-watch-stop-bring-the-results-home):

```
INSTANCE_TYPE=c8g.16xlarge ZONE=... VOLUME_GB=40 PURPOSE=ledger MAX_PRICE=1.0
```

Pass `--user-data file://ledger-user-data.sh` in place of the `--user-data` argument of [launch.md](launch.md#launch):
the file sets the timer itself, so `TIMER_MINUTES` is not needed.  `ledger-user-data.sh` is, with `COMMIT` the pushed
master that has the ledger and `DATE` the day:

```
#!/bin/bash
shutdown -h +150
export AWS_DEFAULT_REGION=us-east-1
dnf install -y -q git tmux
sudo -u ec2-user bash -lc 'cd ~ && git clone -q https://github.com/bvchess/taxman.git && cd taxman &&
    git checkout -q COMMIT'
sudo -u ec2-user tmux new-session -d -s ledger -c /home/ec2-user/taxman \
    'export S3_DEST=s3://taxman-013764204840-us-east-1/ledger-DATE
     BUILD="--bin taxman --example ledger" REQUIRE_CACHED=1 SKIP_TESTS=1 solver/tools/cloud/setup.sh > ~/setup.log 2>&1 &&
         POWEROFF=1 solver/tools/ledger/cloud.sh 2176-3440;
     sudo shutdown -h now'                          # also if the setup failed
```

The setup runs in the tmux session, after the job's `S3_DEST` is exported and before the job, so that a setup that fails
ends the machine and the job never starts.  It fails with no tested binaries in the cache under `REQUIRE_CACHED=1`, or,
with `RELEASE_TESTS=1` in its place, with a suite that fails (`RELEASE_TESTS_FAILED` in `S3_DEST`).

**Options.**  The environment of `cloud.sh` sets the workers and the budget (`WORKERS`, `CPU_HOURS`), the searches'
limits (`BASE`, `MAX_LIMIT`), the games the run is given (`PROVEN`), and the scheduler's three aids, each described in
its section of the entry:

* **The climbs** ([wedge and
  climb](../../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-17-wedge-and-climb)): from
  each board it proves, the scheduler searches the board above and the board below at once, seeded with its game.
  `CLIMB` is their ways (`up`, `down`, `both` or `off`; unset, the program's `both`), and `CLIMB_LIMIT` their limit in
  seconds (unset, the program's 60).  `CLIMB=off` or `CLIMB_LIMIT=0` runs without climbs.  The events and the report say
  what each way cost and proved.
* **The finder**, large neighbourhood search ([the finder in the
  ledger](../../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-17-the-finder-in-the-ledger)),
  runs on a worker after a board's map and before a search whose best seed has changed.  It finds a legal game, not a
  proof, and its game is a lower bound once the referee has replayed it.  `LNS_SECONDS` is the seconds of one thread it
  gets (unset, the program's 1; 0 runs without it).  Its events (`lns_s`, `lns_from`, `lns_found`, `seed_from`,
  `below_proven`, `above_proven`, on the maps and the searches) and the report's `finder:` line say what it did.
* **The floor steps** ([floor-guided
  drilling](../../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-18-floor-guided-drilling)):
  an open board that is not climbed to is stepped down before its proof.  A step is a search under a floor below the
  board's upper bound; if it completes having found nothing, the floor becomes the upper bound (the rule `floor`), and
  each step costs about ten times the one before.  `FLOOR_STEP` is the points a step goes down (unset, the program's
  150; 0 turns the steps off, and the order goes back to `level + gap / gap_unit`).  `FLOOR_SLACK` sets a step's limit,
  in times its forecast (unset, 4; at least `BASE`), and the limit doubles each time it stops the step.  A stopped step
  is kept, with its checkpoint `ckpt/nN.floor.ckpt` beside the board's `nN.ckpt`.  After the climbs, the boards go
  cheapest first by the forecast of their proofs.  The status line counts the steps that have ended, and how (lowered U,
  proved their board, found a set, stalled), and the boards forecast within an hour, a day, or longer; the events give
  each job's `forecast` (log10 of nodes) and `forecast_s`, and `solver/tools/ledger/events.py` compares the forecasts
  with the proofs.

*Going on from a ledger written before the floor steps* (the stage on `ledger-frontier-2/out/run`, written by afe4513):
the run reads it as it is.  The boards get their root bounds at the start (a fraction of a second a board, on the
workers) and have no steps, and the checkpoints of their ordinary searches stay where they are, resumed when the board
comes to its proof; nothing is dropped.  A board with a wide gap takes its steps first, so a board whose old search was
far along is forecast at least twice what that search spent, and waits behind cheaper boards, as `level` made it wait.

**The bounds store.**  `results/bounds.jsonl` keeps the bounds that runs find ([results/README,
"bounds.jsonl"](../../../results/README.md#boundsjsonl)).  With `BOUNDS=results/bounds.jsonl`, the file in the checkout
that the machine cloned, the run starts from the file's best bounds of its stretch, and a board with both bounds needs
no map (the maps of 3441..8191 cost about 65 core-hours); run `ledger verify-bounds` on the file first.  After the run,
on your machine with the results home (below), append what improved and check the file with both referees; `ledger` is
the example's binary there (`solver/frame/target/debug/examples/ledger`, built as [the ledger's
README](../../tools/ledger/README.md) says):

```
ledger export solver/notebook/the-ledger-bounds-from-both-sides-2026-10/cloud-DATE/run --bounds results/bounds.jsonl \
    --source NAME --checkpoints s3://BUCKET/ledger-DATE/out/run/ckpt                  # prints what improved
ledger verify-bounds results/bounds.jsonl
ledger games results/bounds.jsonl > games.jsonl
python3 -I solver/tools/check_bounds.py results/bounds.jsonl --games games.jsonl
```

Then commit the file.  The checkpoints are in S3 only for the searches still going: a proven board has none.

**Watch, stop, bring the results home.**

* **Watch** `tail ~/ledger/run.err` and `~/ledger/log`.  `run.err` has a status line a minute: the CPU spent, the boards
  proven, the searches going, the climbs of each way ended, those that proved their board and those that stopped at
  their limit (parked), and how long the main thread has been busy and results waited for it.  After the maps, the CPU
  spent should rise by nearly 64 a second; if it does not, look at those last two numbers first.
* **Stop** cleanly with `pkill -TERM -f 'examples/ledger run'`: each search writes its checkpoint, the ledger is saved,
  and the script goes on to the report.  **Resume** by running `cloud.sh` again; on a replacement machine, copy
  `S3_DEST/out` back to `~/ledger` first.
* **Bring the results home**, then read them on your machine, and do the end of the rules for every launch:

  ```
  L=solver/notebook/the-ledger-bounds-from-both-sides-2026-10/cloud-DATE
  aws s3 sync S3_DEST/out $L/ --exclude 'run/ckpt/*'
  solver/frame/target/debug/examples/ledger report $L/run --known results/strict.jsonl
  solver/tools/ledger/events.py $L/run
  ```
