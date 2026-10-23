# The finder on long searches, and game 4875 (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-21.

**Question.**  Why did game 4875's search take more than 4.5 billion steps in the run past 5000, and does a finder that
starts once a search proves long cut such a cost?  The option is built and its first measurements are in this folder;
the rest are deferred, so the entry is still a log.  Game 4875 itself was still searching when this was written.

## 2026-10-21: the option, and what was measured

**What was built.**  `taxman solve --finder-after NODES --finder-seconds S` starts one finder thread for a game once its
search has taken NODES steps (counted over every run of the search), for S seconds (default 60) or until the search
ends.  The finder is a large neighbourhood search on one thread, moved from `examples/common/lns.rs` into the library
(`src/search/finder.rs`, the generator in `src/rng.rs`).  It runs rounds of 15 s from the seed of the game of n − 1 and
from the search's incumbent, and offers each better game to the search after the checks of a checkpoint's incumbent
(`search::offer_found`).  The search stays exhaustive, and its proof is its own.  The two settings are flags of `solve`,
not tactics, so they are in no checkpoint's header and a checkpoint resumes with any value.  A line records what the
finder did in `engine.finder`.  [The design](../../../docs/design/finder.md) and [the
reference](../../../docs/REFERENCE.md#1-the-flags-of-solve) have the rest.

**Checks.**

* The unit and integration tests pass: the finder reaches the best known game of 6 small games; a set that cannot be
  promoted, or is not worth what it claims, is refused; games 600 to 604 have `optimal.json`'s scores with the finder
  forced on early and with it off; a search stopped by a node limit resumes with the finder off, on and off again.
* The relaunch of the run's files (`relaunch.txt`, `referees.txt`): the command of `chain.sh` with the finder flags, on a
  copy of `past5000.jsonl` and each of the run's two checkpoints of 4875, on 2 threads, stopped by SIGINT.  From the
  first (3.56 billion steps, incumbent 537971; `--finder-after 1000000 --finder-seconds 30`) the finder found 538080 in
  0.3 s, from the seed's survivors, and the search took it.  From the second (4.58 billion steps, 538080; the flags
  planned for the run, `--finder-after 100000000 --finder-seconds 60`, on the build of 738465d) the search resumed with
  no refusal, the finder ran its 60 s and found nothing better, and the incumbent stayed 538080.  The file without the
  lines of 4873 and 4874 solved them again as repeats of 4872 (0 steps, the same scores and moves), and skipped the games
  it holds.  `taxman verify-results` accepts the mixed file.  `solver/tools/check_results.py` flags 14 lines of the run's
  build, of method `search` with 0 steps (4644, 4785, 4794, 4797, 4810, 4816, 4818, 4824, 4833, 4842, 4844, 4845, 4850
  and 4869: the rule of 7987c86), and none of the new lines.

**A whole game.**  Game 4064 of the run to 4547 (14.3 million steps on 64 threads; its start lies 198 below its best), on
one thread with `taxman solve 4064 --use-prev --force-search --threads 1`, with `--finder-after 100000` and without:

| finder | steps | search s | CPU s | score |
|---|---|---|---|---|
| off | 18,793,190 | 2,539.5 | 2,536 | 5197961 |
| on (60 s) | 16,024,709 | 2,220.9 | 2,280 | 5197961 |

The finder lifted the incumbent from 371913 to 372111, the game's best, 0.3 s after it started at 100,351 steps.  The
search took 0.85 of the steps and 0.90 of the CPU, the finder's included.  It is one game on one thread.

**What the data of the run past 5000 shows so far** (`analysis.py`, `analysis.txt`):

* 4875's board is like 4864's, 4865's and 4872's (9 frames, 1,343 to 1,344 candidates), which took about a billion steps
  each, and its root gap, 2,139 against its first checkpoint's incumbent, lies in the range of the landscape's 19 games
  (1,011 to 2,406).  It is the one search of the run over 100 million steps that began far below its best: 153 below,
  against 4 for 4864 and 0 for the other 20.
* The old finder reached 538080, 109 above the run's incumbent at its first checkpoint, in 1.2 to 10.6 s on one thread
  from the game of 4874 (`results-A.txt`; the game replays at 7476374).
* 20 of 4875's open subtrees take 4.4 times the steps from 537927 as from 538080 (`results-B.jsonl`), 0.45 decades per
  100 of incumbent.  A fit over them puts the work left at the second checkpoint near 7 billion steps, uncertain by a
  factor of 2 to 3, with 86% of it in 50 units beyond the range the fit was made on.
* The finder alone, from the start a search begins at, on one thread (`finder-alone.jsonl`): 10 of 12 of the hardest games
  reached their final best in 60 s and 9 in 5 s, in a median 0.5 s when they did; 4225 and 4416 were not improved at all.

**Deferred.**  The write-up of these measurements, the finder alone on more games, whole games with the finder on and off
(only 4064 is done), and the effect on 64 threads.  The entry concludes with them.

## Files

* `analysis.py`, `analysis.txt`: the numbers of the data of the run.  `analysis.py DATA_DIR` reads `past5000.jsonl`,
  `chain.err.3`, `results-B.jsonl`, `units.jsonl`, `units-b.jsonl` and `landscape.jsonl` from `DATA_DIR`.  The run's
  results, log and checkpoints, and the units files (`tune units 4875 CKPT --known past5000.jsonl --sample 100000 --seed
  1 --out units.jsonl`, 7 MB each), are not copied here.
* `results-A.txt`, `finder-seed.patch`: experiment A, the old finder on one thread (`wall sound --from lns --cap 0.01
  --cpu S --seed K --known past5000.jsonl N`, built at 9d9dae3; the patch seeds the start with the game of n − 1).
* `results-B.jsonl`, `tune-incumbent.patch`, `tableB.py`: experiment B, 20 subtrees solved from five incumbents
  (`tune solve 4875 --units units.jsonl --ids ID --known past5000.jsonl`, with the patch's `TUNE_INCUMBENT_SUM`).
* `game-4875-538080.json`, `replay.py`: the finder's game, and the independent checker (`replay.py FILE`).
* `landscape.jsonl`: the root gaps of 19 games (`wall landscape`).
* `progress-4875.txt`: the lines of the run's log where 4875's incumbent changed.
* `relaunch.txt`, `referees.txt`: the relaunch of the run's files, and the referees on the mixed file.
* `finder-alone.jsonl`, `finder_alone.py`: the finder alone (`examples/finder.rs GAME --target SUM --known FILE`) and the
  table of it.
* `whole-games/`, `whole_games.py`: the runs of game 4064, and the table of them.

## Commits

* 738465d: the finder in the library, the flags `--finder-after` and `--finder-seconds`, `engine.finder` in the line, the
  tests, `examples/finder.rs` and the docs.
* 7987c86: the commit that work started from; the first measurements were made on its working tree with these changes
  uncommitted.
* 4974a71: the build of the run past 5000, which made the checkpoints and the results file the tests start from.
