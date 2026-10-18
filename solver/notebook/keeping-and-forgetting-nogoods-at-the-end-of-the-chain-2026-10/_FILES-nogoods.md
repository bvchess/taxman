# Policies for keeping and forgetting nogoods

The sweep of [the entry](../../docs/experiments/keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10.md):
games of 2211-3260 proved on one thread by the ladder, seeded with their own optimum, under the default policy and the
other policies of the tactics `nogood_period`, `forget_nogoods`, `nogood_keep` and `nogood_limit` (DESIGN.md, section
5).  A search on one thread is deterministic, so node counts compare exactly, and they are the measure; CPU seconds and
cycles are the process's over the search (`getrusage`, `proc_pid_rusage`) on a laptop that other jobs shared (load 6
to 200), and the later runs went eight at a time, with about 8% fewer instructions per CPU-second than one at a time,
so CPU figures compare only between runs made under the same load.

| file | what it is |
|---|---|
| `run.sh` | the sweep: `nice -n 10 bench/nogoods/run.sh OUT "GAME ..." SECONDS POLICY...` from `rust/`; one line per run appended to OUT as it ends, runs already in OUT skipped, so an interrupted sweep resumes |
| `parallel.sh` | `nice -n 10 bench/nogoods/parallel.sh JOBS 8`: a file of `run.sh` arguments, one line a run, eight at a time |
| `summarize.py` | `summarize.py RESULTS [--games=G,...] [--policies=P,...]`: nodes by game and policy, each policy against the default by game with the geometric mean and its standard error, the totals with the CPU time of a node, and the counters of the tables |
| `sweep.txt` | 152 runs: six games (2211, 2910, 2250, 3050, 2862, 3260) under 20 policies, four more (2625, 3135, 3228, 2316) under 8; each line is `GAME<TAB>POLICY<TAB>the ladder's line` |
| `sweep.md` | `summarize.py sweep.txt`, for the six games and for the ten games of the eight policies run on all of them |

The ladder was `target/release/examples/ladder` of commit ea0eda6 (`--seed-opt --known results/strict.jsonl --threads
1`); the policies are the `--tactic` specs of the first column of the tables (`forget_nogoods=off,nogood_limit=256` is
two of them).  Every `default` run has the node count of the build before the knobs (506,416 at 2211, 415,764 at 2910,
683,587 at 2250, 761,406 at 3050, 814,937 at 2862), and the merge with master's node-speed round (6460c86) changes none
(2910 again: 415,764).  The summary's column of CPU microseconds a node weighs the games by their nodes, so it moves with
which game a policy made large; compare a game with itself.

The plan went on to three games whose recorded proofs took 4-10 million nodes (2260, 3243, 3340) under the default and
four periods; the sweep moved to the cloud machine's tuning campaign (`bench/tuning/`) before one of those runs ended,
so there are none.
