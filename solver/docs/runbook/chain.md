# The chain of optimal games (`chain.sh`)

Part of the [runbook](../RUNBOOK.md).

## What is being computed

`chain.sh N` runs `taxman solve 1-N` into one results file: every game from n = 1, each built from the one before.
Nothing comes from the built-in table, which the run only compares with, so the chain assumes nothing.  The
committed `results/optimal.jsonl.gz` is exactly such a file, compressed
([results/README.md](../../../results/README.md#optimaljsonlgz) has its range, and
[results/runs.md](../../../results/runs.md) how each part was made).  A run from n = 1 reproduces its scores, and
`verify.sh` checks the run against it.  The production chain can instead start from the committed file,
as [Run](#run) says, so that the run solves only the games above its end.  The solver is the same code as on a laptop;
the machine only changes how long it takes.

**What it costs.**  The games to 2175 take minutes on a big machine.  Past 2175 the search time of a game grows about
tenfold for every 1,040 in n, in waves, and one day on 64 CPUs reached n = 3440 ([the chain to
3440](https://github.com/bvchess/taxman-notebook/blob/main/the-frontier-chain-to-3440-2026-10/README.md)).  A
`c8g.48xlarge` ran the chain past 2225 1.9 to 2.7 times as fast as 64 threads, for 1.1 to 1.9 times the cost a game
([the scaling run](https://github.com/bvchess/taxman-notebook/blob/main/scaling-to-192-threads-2026-10/README.md)), and
the [calibration](https://github.com/bvchess/taxman-notebook/blob/main/graviton-calibration-2026-10/README.md) gives the
rate and the price of the games past 2175 on a `c8g.16xlarge`.  Estimate the cost of a target N from those entries, at
the spot price of the hour (a `c8g.16xlarge` costs about $0.68 an hour), and let the first hours of the real run decide:
the log has each game's seconds.  A single hard game can take hours.  Choose N by money and patience; running `chain.sh`
again with a larger N later continues the same file.

## Run

Start in tmux, with a deadline and with the state copied off the machine:

```
tmux new -s tx
DEADLINE_HOURS=HOURS POWEROFF=always S3_DEST=s3://taxman-013764204840-us-east-1/PREFIX solver/tools/cloud/chain.sh N
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
`MAX_FAILS`, `S3_DEST`, `SAVE_EVERY`, and the [cost guardrails](launch.md#cost-guardrails) (`DEADLINE_HOURS`,
`BUDGET_HOURS`, `RUN_UNTIL`, `POWEROFF`).  The search itself needs no flag: the solver's defaults are the chain's
policy, a gate of 1,200 with the gate trial, `price_reuse` and `warm_start` on, and a finder of 900 seconds
([tactics.md](../design/tactics.md)), so `EXTRA_ARGS` holds only what depends on the machine and the run
(`--background`, `--carried-dir`, `--profile`).

On a machine shared with other work, leave `THREADS` unset (one worker per hardware thread) and give
`EXTRA_ARGS="--background"`.  The workers then run under the idle scheduling policy, so that any other process, even
one at `nice -n 19`, runs before them and the search takes the threads the rest leaves ([the
flag](../REFERENCE.md#1-the-flags-of-solve)).  A fixed `THREADS` below the machine's leaves threads idle whenever the
rest is quiet.  Such a machine keeps the chain's checkout and data in [the machine's folders](#the-machines-folders),
and `chain.sh` runs there with `TAXMAN_WORK` set to the chain's data folder instead of `~/taxman-work`.

`WINDOW` is the number of consecutive games kept in flight at once (`taxman solve --window`).  While the solver still
proves game n, it sets up and searches game n + 1 from n's best game so far, and the threads move to the searches that
keep them busy, so one game's search does not leave a machine of many threads idle.  Games still conclude and record in
order, with the scores, proofs and repeats of a window of 1, which solves them one after the other ([the
README](../../frame/README.md#games-in-flight); [DESIGN.md](../design/engine.md#the-window-games-in-flight)).  The
default is 4, in `chain.sh` and in `taxman solve --window`: on 64 and 192 threads the [scaling
run](https://github.com/bvchess/taxman-notebook/blob/main/scaling-to-192-threads-2026-10/README.md) found 4 the best of
1, 4, 16 and 32 on the games past 2225, and a window of 16 20 to 32 % slower than 1 on 64 threads.

* **Where the chain starts.**  From an empty `~/taxman-work`, `chain.sh` solves from n = 1, which reproduces the
  committed `results/optimal.jsonl.gz` (the scores equal; the moves may differ).  The first 2175 games take minutes on
  the big machine, and `verify.sh` then checks the machine and the build against the committed file.  To skip those
  games, decompress the committed file into the run's file before the first run, once:

  ```
  mkdir -p ~/taxman-work/results && gzip -dc results/optimal.jsonl.gz > ~/taxman-work/results/strict.jsonl
  solver/tools/cloud/chain.sh 8191                    # skips the games it holds and solves the next on
  ```

  `chain.sh` checks the file when it opens it.  The file holds no incomplete line, its `known` games assume [statement
  U](../../../docs/theory/exactness.md#statement-u), and its lines keep the builds that made them
  ([results/README.md](../../../results/README.md) names them), as the lines added will keep the cloud's.  The
  comparison of `verify.sh` with the committed file is then trivial for the games it holds, so the referees alone check
  the games above them (each game replayed, the chain bounds, Franklín–Moniot, the heuristic chain); a run from n = 1 is
  the cross-check of the machine and the build.  On a replacement machine, restore the saved state instead ([Surviving a
  spot interruption](launch.md#surviving-a-spot-interruption)), never the committed file over it.
* **Stop on purpose** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/chain.pid)`.  The solver writes a final
  checkpoint, and `chain.sh` ends with status 143.  **Start again** by running the same command: it skips every
  completed game and resumes the game with a checkpoint.  The thread count, the window, `S3_DEST`, `DEADLINE_HOURS` and
  N may differ between runs.  The first deadline stays in `~/taxman-work/deadline`: delete the file to set another (a
  replacement machine: [Surviving a spot interruption](launch.md#surviving-a-spot-interruption)).
* **What it does by itself.**  `chain.sh` retries a crash (a status other than 0, 1, 2, 124, 130 and 143) after 10 s.
  It does not retry an error status (1 or 2: a refused checkpoint, a game that failed a check), because it would repeat:
  read `~/taxman-work/logs/chain.err`.
* **Use the same build between a stop and its resume.**  A checkpoint records the game, the frames the build made, the
  candidates and the search tactics, and the solver refuses one from a build that sets these up differently, with the
  reason: delete it, or
  run the old build.  The width of the sets is part of that: a build that gives a game another width refuses its old
  checkpoint.  Do not `git pull` in the middle of a long game without a reason.  Changing only code generation (PGO,
  `target-cpu`) is harmless.
* **Past 4095.**  The solver supports games up to n = 8191 (`taxman::solver::MAX_N`), those from 4096 with sets of 128
  words ([the entry on boards past
  4095](https://github.com/bvchess/taxman-notebook/blob/main/boards-past-4095-2026-10/README.md)).  The solver refuses a
  game past 8191 ("beyond the largest supported board"), which ends the script with an error.

## The machine's folders

A machine that runs the chain beside other work keeps all of it in one folder of the home directory, `taxman_work`.  Its
name is `TAXMAN_HOME`, relative to the home; [`layout.sh`](../../tools/cloud/layout.sh) derives the first four folders
from it for `status.sh`, and no script names them anywhere else.  The last two rows are local conventions
that no script names.

| folder | what it is |
|---|---|
| `~/taxman_work/chain` | the chain's checkout, which it builds and runs from |
| `~/taxman_work/chain-data` | the chain's results, checkpoints (`ckpt/`), logs and profiles |
| `~/taxman_work/archive` | archived data, read only |
| `~/taxman_work/research` | a second checkout, for work on the repository beside the chain |
| `~/taxman_work/lab` | work beside the chain: builds, measurements and scratch runs, in `work/` |
| `~/taxman_work/run.sh` | the script that starts the chain: a local file, not in the repository |

`run.sh` starts `chain/solver/tools/cloud/chain.sh` with `TAXMAN_WORK` set to the chain's data folder,
`$HOME/taxman_work/chain-data`.  `chain.sh` takes its results file, checkpoints and logs from `TAXMAN_WORK`, and needs
no other variable for the layout; a profile directory in `EXTRA_ARGS` (`--profile`) goes in `chain-data` too.

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
B](https://github.com/bvchess/taxman-notebook/blob/main/round-b-the-chain-on-many-threads-2026-10/README.md)).  On a
Graviton4 core the proof of 2172 does 6.5 thousand a second on one thread, and the chain's searches of 2277..2336 did
3.3 to 6.2 thousand a thread on 192 threads (0.6 to 1.2 million a second) and 3.9 to 5.3 thousand on 64 (0.25 to 0.34
million) ([the scaling
run](https://github.com/bvchess/taxman-notebook/blob/main/scaling-to-192-threads-2026-10/README.md)).  Estimate when a
long game will end from the previous ones: the log has each game's seconds, and past 1300 they come in waves of a few
hundred n, not on a smooth curve.

### From another machine

`solver/tools/cloud/status.sh [--recorded N] --host USER@HOST` reports a chain that runs on a machine you reach by ssh
(`chain.sh` in the tmux session `tx`, its state in the chain's data folder of [the machine's
folders](#the-machines-folders)).  It reads that state in one ssh call (key authentication, `BatchMode`), and needs no
AWS profile and makes no aws call.  It prints the host and its `uptime` line; whether the chain runs (the session exists
and a `taxman solve` process is alive); `ENDED:` and the line of `chain.outcome`, if there is one; the window and the
last game of the range, from the last line of `logs/chain.log` that starts the chain; when `logs/chain.err` last
changed, as `last progress line`; and `recorded through`, taken from the last completed game of the results file; then
one table of the last 6 games recorded and the games in flight ([the window](../../frame/README.md#games-in-flight)).
`--recorded N` shows the last N games recorded instead of 6, and 0 leaves them out:

```
host user@machine
  09:12:44 up 8 days, 11:09,  2 users,  load average: 90.48, 90.55, 92.09
  the chain is running (the tmux session tx exists, a taxman solve process is alive)
  chain 1-5060, window 4
  last progress line: 2026-10-18 15:17 PDT, 5 min 24 s ago
  recorded through 4871 (score 7,461,794)
  the last 6 games recorded and the 4 in flight (window 4):
      n  own search           nodes       best  state
   4866  0.0s                     0    537,485  recorded, score 7,446,689 (repeat, same_search of 4865)
   4867  1m49s           34,218,766    538,284  recorded, score 7,449,976 (search, exhausted)
   4868  50m11s       1,133,462,769    538,760  recorded, score 7,453,452 (search, exhausted)
   4869  0.0s                     0    538,745  recorded, score 7,456,914 (start, cap)
   4870  0.0s                     0    538,760  recorded, score 7,459,353 (repeat, same_search of 4868)
   4871  0.0s                     0          -  recorded, score 7,461,794 (prime, prime_formula)
   4872  1h15m00s     1,086,013,550    341,541  running, 242.8k nodes/s
   4873  -                        -          -  no recent progress line
   4874  -                        -          -  no recent progress line
   4875  -                       63    341,541  suspended
```

The recorded games come first, in increasing n, and the games in flight follow.  Scores, nodes and best sums have
thousands separators; game numbers and times do not.  A recorded game's row comes from its line in `logs/chain.out`
([the summary line](../REFERENCE.md#the-summary-line-stdout-one-per-game)), the last one if a replacement machine
printed the game again after an interruption.  That file holds only the games that the host's own run recorded, so a run
that has recorded none shows the games in flight alone:

* `own search` and `best`: the line's `own=` and `best=`, the game's own search time over every run of it
  ([glossary](../../../docs/GLOSSARY.md#the-search)) and its best promotion sum, the score less `in_the_bag`.  Both are
  `-` where the solver that printed the line did not print them, and `best` is `-` for a prime or known game; a game that
  ran no search has an own search of 0.0s;
* `nodes`: the line's nodes;
* `state`: `recorded`, the score, and in parentheses the line's method and proof, then `optimal ok` when the score is
  the one in the built-in table.

The state of a game in flight is told from its last progress line in `logs/chain.err` (the last 3000 lines are read):

* `running`: 1000 nodes a second or more in its last interval;
* `suspended`: its threads belong to another game;
* `search done`: the line says `done` or `optimal` and the game is not recorded yet (`being recorded` for the oldest
  game, else `waiting for the game before`);
* `search stopped` or `failed`;
* `no recent progress line`: the game has no line, or three lines of another game follow its line.  A setup, a prime or
  repeated game and a search not started look alike in the logs.

For a game in flight, `own search` is the clock that the oldest game keeps in `ckpt/nNNNN.clock` (up to a checkpoint
interval old; `-` for a game without one), and `best` the progress line's sum of promotions so far.  Times are in the
local time zone (`TZ`) with its abbreviation.

## Verifying, and bringing the results home

`chain.sh` runs `verify.sh` when the range is complete.  On any copy of the file, anywhere:

```
solver/tools/cloud/verify.sh results-from-cloud/chain.jsonl
```

`verify.sh` runs `taxman verify-results` and `solver/tools/check_results.py` on the file together with the committed
`results/optimal.jsonl.gz`.  The two verifiers check that:

* every game replays legally, finishes, and opens with the largest prime;
* each score agrees with the built-in table;
* the chain bounds `score(n-1) <= score(n) <= n + score(n-1)` hold, and each line's arithmetic and provenance;
* the Franklín–Moniot bound and the best heuristic chain hold;
* every game that the file shares with the committed file has the same score.

For a chain from 1, the report says `assumptions: none in N of N games`.  Any assumption or problem line means the run
is not what it should be: stop and read the line.  (A count of incomplete lines is not a problem: every search that
stopped and resumed left one.)

A run with `--use-prev` holds the committed chain's games as `known` lines at the start of its own file, so its file does not continue the committed file as it
stands.  Bring the games of any run home by appending the games past the committed file's end to it as one gzip member,
which leaves every earlier byte of the file as it was.  In the run's own file (the plain JSONL that the run wrote, not
in the repository), find the numbers of the lines that are the first completed line of each game past the committed
file's last game, leaving out the run's `known` copies of the committed games and its incomplete lines (the committed
file has none).  A is the number of the first of those lines and B of the last.  Where an incomplete line sits between
two games, give the sed address of each stretch of consecutive lines (`A,Bp;C,Dp`).  Then run both verifiers on the
file:

```
grep -n '"completed":true' FILE | cut -c1-40         # each completed line's number, and its first characters
sed -n 'A,Bp' FILE | gzip -9 -n >> results/optimal.jsonl.gz       # the lines, byte for byte, as one member
taxman verify-results results/optimal.jsonl.gz
python3 solver/tools/check_results.py results/optimal.jsonl.gz
```

The first appended line records its game of n − 1 with the proof `known`, as the run's own line, while the committed
file's line has the proof it was made with.  Both referees accept that record, because the score, the moves and the
assumptions are the line's.  The built-in table takes the appended games at the next build, because the binary carries
the file as it was when it was built.

Then decide what to keep.  [Storage](machine.md#storage) gives the file's size.  Scripts must not overwrite the
committed `results/`: put a file next to them (say `results/chain-1-N.jsonl`) and update
[`results/README.md`](../../../results/README.md) and the list of results files in `tests/results.rs`.  Logs that a
measurement needs go to its notebook entry.

## Examples

[The chain to 3440](https://github.com/bvchess/taxman-notebook/blob/main/the-frontier-chain-to-3440-2026-10/README.md)
ran `chain.sh 4095` on one `c8g.16xlarge` for a day, from a copy of the committed games to 2175;
[results/runs.md](../../../results/runs.md) records each run that made the committed chain.
