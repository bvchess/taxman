# A search at the end of the chain, on the laptop: node rate by thread count and where a node's time goes

Game 3441, resumed from the c8g.16xlarge chain's checkpoint (`n3441.ckpt`, 625,763,557 nodes, format 4, written when
the chain was stopped on 2026-10-15), on the M1 Max (8 performance cores) with the release build of master ad9fcac
(`debug = 1`), one process at a time under `nice -n 10` (the eight-copies run excepted), with other jobs and a video
call on the machine (load 5 to 16).  Wall rates are therefore not trustworthy; the rates to use are per CPU-second
(user + system from `/usr/bin/time -l`, net of the setup-only run on as many threads) and the proportions of the
profiles.  Neither the checkpoint nor `rust/results/strict.jsonl` is modified: each run works on copies.

| file | what it is |
|---|---|
| `run.sh` | one run: `TAXMAN_BENCH_SRC=DIR TAXMAN_BENCH_OUT=DIR bench/chain_end/run.sh THREADS SECONDS TAG [PROFILE_SECONDS]` from `rust/` (SRC holds the checkpoint and games 1..3440 of `strict.jsonl`); `SECONDS` 0 is the setup alone |
| `runs.jsonl` | `summarize.py RUNS CLOUD_STRICT setup1 setup2 setup4 setup8 t1 t2 t4 t8 copy1 ... copy8`: counts, rusage, the window's rates (from 20 s into the search), the solver's counters, and the net cycles, instructions and IPC; `copies` pools eight independent one-thread runs made at once (the hardware's share of the loss with threads) |
| `cloud.jsonl` | the same figures from the c8g's 64-thread progress lines for games 3441 and 3440 |
| `profile_t1.txt`, `profile_t8.txt` | `groups.py target/release/taxman sample.txt` on `run.sh 1 100 prof1 60` and `run.sh 8 100 prof8 60` (macOS `sample`, 1 ms, from 25 s into the run): groups, kernels, inclusive and self time of the workers' busy samples |

Commands as run: `t1`, `t2`, `t4`, `t8` = `run.sh T 130 tT`; `setupT` = `run.sh T 0 setupT`; `copyK` = eight at once of
`run.sh 1 130 copyK`.  `groups.py` and `tools/profile/attribute.py` share one parser and symbolicator (`atos -i`).
The search began about 42 s after the start on one thread (12 s on eight: the start incumbent's local search is
parallel) and its clock starts then; the cloud log's "5.9 hours" of game 3441 were 5 hours of waiting in the window
for game 3440 and a 50-minute search.

## The calibration on the cloud (2026-10-16)

What it asked and found: [the entry](../../docs/experiments/the-calibration-for-the-run-to-4000-2026-10.md).  The same
game resumed from the same checkpoint on one `c8g.16xlarge` (64 CPUs; 2026-10-16, 16:41:00Z to 17:07:23Z, commit 97db411),
and four proofs of the ladder.  The files are in `calib-20261016/`:

| file | what it is |
|---|---|
| `calib.sh` | the script as it ran, from `~/taxman` after `setup.sh`: `taxman solve 3441 --threads T --time-limit 180 --checkpoint-interval 300 --progress 10` for T = 1, 16, 32 and 64, each on copies of the checkpoint and of `results/strict.jsonl`, under `perf stat -d` and `/usr/bin/time -v`; then `ladder --threads T --seed-opt --known results/strict.jsonl [--tactic forget_nogoods=off] 3232 3360 3388 3179` (60 s a game) at 64 threads with the default tactics, at 64 with `forget_nogoods=off`, and at 32 with the default |
| `out/calib.log` | its log: the machine (`lscpu`), the stages' times, the last two progress lines of each resumed run and the ladder's rows (one of its lines is the S3 download's progress, left in) |
| `out/t1/`, `out/t16/`, `out/t32/`, `out/t64/` | for each thread count: `progress.txt` (the solver's progress line every 10 s), `perf.txt` (cycles, instructions, branch and L1 misses), `time.txt` (`/usr/bin/time -v`) and `stdout.txt` (the game's line: stopped, incomplete) |
| `out/ladder_default64.txt`, `out/ladder_noforget64.txt`, `out/ladder_default32.txt` | the ladder's rows, a game a line (the nodes, the rate and the engine's counters; all four are `OPEN` at 60 s), each with its `.time` (`/usr/bin/time -v`) |

Not kept: the checkpoint (5.3 MB) and the four copies of the chain's results file that each run wrote (`out.jsonl`, 17
MB each), which the S3 prefix of the run held.
