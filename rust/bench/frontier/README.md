# The frontier search on the laptop: node rate by thread count and where a node's time goes

Game 3441, resumed from the c8g.16xlarge chain's checkpoint (`n3441.ckpt`, 625,763,557 nodes, format 4, written when
the chain was stopped on 2026-10-15), on the M1 Max (8 performance cores) with the release build of master ad9fcac
(`debug = 1`), one process at a time under `nice -n 10` (the eight-copies run excepted), with other jobs and a video
call on the machine (load 5 to 16).  Wall rates are therefore not trustworthy; the rates to use are per CPU-second
(user + system from `/usr/bin/time -l`, net of the setup-only run on as many threads) and the proportions of the
profiles.  Neither the checkpoint nor `rust/results/strict.jsonl` is modified: each run works on copies.

| file | what it is |
|---|---|
| `run.sh` | one run: `TAXMAN_BENCH_SRC=DIR TAXMAN_BENCH_OUT=DIR bench/frontier/run.sh THREADS SECONDS TAG [PROFILE_SECONDS]` from `rust/` (SRC holds the checkpoint and games 1..3440 of `strict.jsonl`); `SECONDS` 0 is the setup alone |
| `runs.jsonl` | `summarize.py RUNS CLOUD_STRICT setup1 setup2 setup4 setup8 t1 t2 t4 t8 copy1 ... copy8`: counts, rusage, the window's rates (from 20 s into the search), the solver's counters, and the net cycles, instructions and IPC; `copies` pools eight independent one-thread runs made at once (the hardware's share of the loss with threads) |
| `cloud.jsonl` | the same figures from the c8g's 64-thread progress lines for games 3441 and 3440 |
| `profile_t1.txt`, `profile_t8.txt` | `groups.py target/release/taxman sample.txt` on `run.sh 1 100 prof1 60` and `run.sh 8 100 prof8 60` (macOS `sample`, 1 ms, from 25 s into the run): groups, kernels, inclusive and self time of the workers' busy samples |

Commands as run: `t1`, `t2`, `t4`, `t8` = `run.sh T 130 tT`; `setupT` = `run.sh T 0 setupT`; `copyK` = eight at once of
`run.sh 1 130 copyK`.  `groups.py` and `tools/profile/attribute.py` share one parser and symbolicator (`atos -i`).
The search began about 42 s after the start on one thread (12 s on eight: the start incumbent's local search is
parallel) and its clock starts then; the cloud log's "5.9 hours" of game 3441 were 5 hours of waiting in the window
for game 3440 and a 50-minute search.
