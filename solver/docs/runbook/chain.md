# The production chain

Part of the [runbook](../RUNBOOK.md).

## What is being computed

`chain.sh N` runs `taxman solve 1-N` into one results file: every game from n = 1, each built from the one before.
Nothing comes from `optimal.json`, which the run only compares with, to n = 1300, so the chain assumes nothing.  The
committed `results/strict.jsonl` is exactly such a file: a laptop made its first 2175 games and cloud machines the rest
([results/README.md](../../../results/README.md) has its range and how each part was made).  A run from n = 1 reproduces
its scores, and `verify.sh` checks the run against it.  The production chain can instead start from the committed file,
as [Run](#run) says, so that the run solves only the games above its end.  The solver is the same code as on a laptop;
the machine only changes how long it takes.

**What it costs.**  The games to 2175 take minutes on a big machine.  Past 2175 the search time of a game grows about
tenfold for every 1,040 in n, in waves, and one day on 64 CPUs reached n = 3440 ([the chain to
3440](../../notebook/the-frontier-chain-to-3440-2026-10/README.md)).  A `c8g.48xlarge` ran the chain past 2225 1.9 to
2.7 times as fast as 64 threads, for 1.1 to 1.9 times the cost a game ([the scaling
run](../../notebook/scaling-to-192-threads-2026-10/README.md)), and the
[calibration](../../notebook/graviton-calibration-2026-10/README.md) gives the rate and the price of the games past 2175
on a `c8g.16xlarge`.  Estimate the cost of a target N from those entries, at the spot price of the hour (a
`c8g.16xlarge` costs about $0.68 an hour), and let the first hours of the real run decide: the log has each game's
seconds.  A single hard game can take hours.  Choose N by money and patience; running `chain.sh` again with a larger N
later continues the same file.

## Run

Start in tmux, with a deadline and with the state copied off the machine:

```
tmux new -s tx
DEADLINE_HOURS=30 POWEROFF=always S3_DEST=s3://taxman-013764204840-us-east-1/chain-run1 solver/tools/cloud/chain.sh 1400
```

The bucket is the one of [Access, and what the account holds](launch.md#access-and-what-the-account-holds), with a
prefix of its own for each run.  In a second window, on spot, start `spot-watch.sh`.  It logs to
`~/taxman-work/logs/spot-watch.log` as well as to its terminal, so its notice and its save leave a trace after the
window is gone; the output of the save is in `save.log` beside it.

```
solver/tools/cloud/spot-watch.sh
```

`chain.sh N` is the whole command.  It runs the chain 1..N into `~/taxman-work/results/strict.jsonl` on all CPUs, with
four games in flight at once (`WINDOW`), a checkpoint every 300 s and a progress line every 60 s.  The top of the script
documents its variables, all optional: `THREADS`, `WINDOW`, `CHECKPOINT_INTERVAL`, `PROGRESS`, `EXTRA_ARGS`,
`MAX_FAILS`, `S3_DEST`, `SAVE_EVERY`, and the [cost guardrails](launch.md#cost-guardrails) (`GAME_HOURS`,
`DEADLINE_HOURS`, `BUDGET_HOURS`, `RUN_UNTIL`, `POWEROFF`; the budget and the date outlive the machine,
[wall.md](wall.md)).

`WINDOW` is the number of consecutive games kept in flight at once (`taxman solve --window`).  While the solver still
proves game n, it sets up and searches game n + 1 from n's best game so far, and the threads move to the searches that
keep them busy, so one game's search does not leave a machine of many threads idle.  Games still conclude and record in
order, with the scores, proofs and repeats of a window of 1, which solves them one after the other ([the
README](../../frame/README.md#games-in-flight); [DESIGN.md](../design/engine.md#the-window-games-in-flight)).  The
default is 4, in `chain.sh` and in `taxman solve --window`: on 64 and 192 threads the [scaling
run](../../notebook/scaling-to-192-threads-2026-10/README.md) found 4 the best of 1, 4, 16 and 32 on the games past
2225, and a window of 16 20 to 32 % slower than 1 on 64 threads.

* **Where the chain starts.**  From an empty `~/taxman-work`, `chain.sh` solves from n = 1, which reproduces the
  committed `results/strict.jsonl` (the scores equal; the moves may differ).  The first 2175 games take minutes on the
  big machine, and `verify.sh` then checks the machine and the build against the committed file.  To skip those games,
  copy the committed file in before the first run, once:

  ```
  mkdir -p ~/taxman-work/results && cp results/strict.jsonl ~/taxman-work/results/strict.jsonl
  solver/tools/cloud/chain.sh 8191                    # skips the games it holds and solves the next on
  ```

  `chain.sh` checks the file when it opens it.  The file holds no incomplete line, its `known` games
  assume statement U, and its lines keep the builds that made them ([results/README.md](../../../results/README.md)
  names them), as
  the lines added will keep the cloud's.  The comparison of `verify.sh` with the committed file is then trivial for the
  games it holds, so the referees alone check the games above them (each game replayed, the chain bounds,
  Franklín–Moniot, the heuristic chain); a run from n = 1 is the cross-check of the machine and the build.  On a
  replacement machine, restore the saved state instead ([Surviving a spot
  interruption](launch.md#surviving-a-spot-interruption)), never the committed file over it.
* **Stop on purpose** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/chain.pid)`.  The solver writes a final
  checkpoint, and `chain.sh` ends with status 143.  **Start again** by running the same command: it skips every
  completed game and resumes the game with a checkpoint.  The thread count, the window, `S3_DEST`, `DEADLINE_HOURS` and
  N may differ between runs.  The first deadline stays in `~/taxman-work/deadline`: delete the file to set another (a
  replacement machine: [Surviving a spot interruption](launch.md#surviving-a-spot-interruption)).
* **What it does by itself.**  `chain.sh` retries a crash (a status other than 0, 1, 2, 4, 124, 130 and 143) after 10 s.
  It does not retry an error status (1 or 2: a refused checkpoint, a game that failed a check), because it would repeat:
  read `~/taxman-work/logs/chain.err`.
* **Use the same build between a stop and its resume.**  A checkpoint records the game, the frames the build made, the
  candidates and the search tactics, and the solver refuses one from a build that sets these up differently, with the
  reason: delete it, or
  run the old build.  The width of the sets is part of that: a build that gives a game another width refuses its old
  checkpoint.  Do not `git pull` in the middle of a long game without a reason.  Changing only code generation (PGO,
  `target-cpu`) is harmless.
* **Past 4095.**  The solver supports games up to n = 8191 (`taxman::solver::MAX_N`), those from 4096 with sets of 128
  words ([the entry on boards past 4095](../../notebook/boards-past-4095-2026-10/README.md)).  A build from before
  commit 4316168 stops at n = 4095, the widest set it has.  To take a chain past 4095, stop it (or let `chain.sh 4095`
  finish), build the newer commit in the same checkout (`git pull`, then `setup.sh`, or `cargo build --release` in
  `solver/frame`), and run `chain.sh N` again with N up to 8191 (or `CHAIN_TO=N`).  It skips the completed games, and
  the checkpoint of a game below 4096 that the old build wrote resumes, since the game keeps its width, frames,
  candidates and tactics (tried with checkpoints of 1133 and 3600).  The solver refuses a game past 8191 ("beyond the
  largest supported board"), which ends the script with an error.

## Monitoring

```
tail -f ~/taxman-work/logs/chain.err          # the progress lines of the game in progress
tail -n 3 ~/taxman-work/logs/chain.out        # the last finished games
tail ~/taxman-work/logs/chain.log             # this script's own messages
tail -n 1 ~/taxman-work/results/strict.jsonl | cut -c1-40        # the last completed game
ls -l ~/taxman-work/ckpt                      # a checkpoint no older than ~5 minutes while a long game runs
htop                                          # all CPUs near 100 %
```

A healthy progress line at 192 threads shows `eff` of 95 % or more, `lock` under 1 %, small `idle`, `queue` of a few
hundred at most, `spilled 0`, `pauses N (x%)` with x under 1, and `best` rising early and then holding still (the
incumbent is often, not always, found long before the proof ends).  The node rate grows with the thread count: one M1
Max thread does 9 thousand nodes a second at n = 1472 and 8 thousand at 2172 (the table of [round
B](../../notebook/round-b-the-chain-on-many-threads-2026-10/README.md)).  On a Graviton4 core the proof of 2172 does 6.5
thousand a second on one thread, and the chain's searches of 2277..2336 did 3.3 to 6.2 thousand a thread on 192 threads
(0.6 to 1.2 million a second) and 3.9 to 5.3 thousand on 64 (0.25 to 0.34 million) ([the scaling
run](../../notebook/scaling-to-192-threads-2026-10/README.md)).  Estimate when a long game will end from the previous
ones: the log has each game's seconds, and past 1300 they come in waves of a few hundred n, not on a smooth curve.

## Verifying, and bringing the results home

`chain.sh` runs `verify.sh` when the range is complete.  On any copy of the file, anywhere:

```
solver/tools/cloud/verify.sh results-from-cloud/strict.jsonl
```

`verify.sh` runs `taxman verify-results` and `solver/tools/check_results.py` on the file together with the committed
`results/strict.jsonl`.  The two verifiers check that:

* every game replays legally, finishes, and opens with the largest prime;
* each score agrees with `optimal.json`;
* the chain bounds `score(n-1) <= score(n) <= n + score(n-1)` hold, and each line's arithmetic and provenance;
* the Franklín–Moniot bound and the best heuristic chain hold;
* every game that the file shares with the committed file has the same score.

For a chain from 1, the report says `assumptions: none in N of N games`.  Any assumption or problem line means the run
is not what it should be: stop and read the line.  (A count of incomplete lines is not a problem: every search that
stopped and resumed left one.)

A run with `--use-prev` ([Launch, watch, stop, clean up](wall.md#launch-watch-stop-clean-up)) holds the committed
chain's games as `known` lines at the start of its own file, so its file does not continue the committed file as it
stands.  Append its completed lines past the committed file's last game, byte for byte, and leave out its `known` copies
of the committed games and its incomplete lines (the committed file has none); then run the referees on the committed
file.  The first appended line records its game of n − 1 with the proof `known`, as the run's own line, while the
committed file's line has the proof it was made with.  Both referees accept that record, because the score, the moves
and the assumptions are the line's.

Then decide what to keep.  [Storage](machine.md#storage) gives the file's size.  Scripts must not overwrite the
committed `results/`: put a
file next to them (say `results/strict-1-1400.jsonl`) and update `results/README.md` and the list of results files in
`tests/results.rs`.  The scaling logs under
`solver/notebook/` (in a folder of their own, or in `solver/notebook/graviton-calibration-2026-10/`) are your call.

To extend the table of optimal games, run `taxman extend-optimal` on the verified file from a checkout:
`taxman extend-optimal results-from-cloud/strict.jsonl --out /tmp/optimal.json`.  It appends the games past the table's
end that the file completed, and says where it stopped.  Check the result with `taxman verify-optimal /tmp/optimal.json`
and `diff`, copy it over `results/optimal.json`, and rebuild, because the binary carries the table it was built with.
