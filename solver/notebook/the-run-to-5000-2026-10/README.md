# The run to 5000 (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-18.  Concluded 2026-10-20.

**Question.**  How far does one more unattended cloud run take the [committed
chain](../../../docs/GLOSSARY.md#results-and-trust), when it takes the games already proven from that chain and from the
[bounds store](../../../docs/GLOSSARY.md#results-and-trust) and searches only the rest?  **Answer.**  From game 3914 to
game 4547: 633 new games for 40.4 instance-hours and $27.32 at spot.  The budget ended the run in the middle of game
4548, with the wall far off: the longest search was in flight 4.4 hours of the wall's 24.  The store gave 167 of the
games with no search.

## Method

The run went from 2026-10-18 at 19:27:41Z to 2026-10-20 at 12:31:00Z, on commit ca357bf with a clean tree.  `wall.sh
launch` set up an Auto Scaling group of one spot `c8g.16xlarge` (64 threads) that replaces an interrupted machine ([the
unattended run to the wall](https://github.com/bvchess/taxman/blob/3ca11bf/solver/docs/runbook/wall.md)).  The machine ran `taxman solve 1-8191 --use-prev` with a
[window](../../../docs/GLOSSARY.md#the-search) of 4 games in flight, the default tactics of that commit and the
[wall](../../../docs/GLOSSARY.md#the-search) at 24 hours (`--game-hours 24`).  With `--use-prev` it took games 1..3914
from the committed chain and each game the store had proven from the store, and searched the others from 3915.  The
first machine ran the release tests first (`RELEASE_TESTS=1`: 699 passed, 0 failed, 28 ignored).  The account's owner
approved 36 instance-hours ($24.34 at the $0.676 an hour the run counts at), and later 4.5 more (about $3) to reach a
stretch of 84 games, 4446..4529, that the store had proven; the first budget had ended the run at game 4430.

## Results

Four machines ran it in turn:

| machine | solver ran (Z) | solver hours | ended by | games done |
|---|---|---|---|---|
| 1 | 10-18 19:40:49 to 10-19 02:08:24 | 6.46 | a spot interruption | 3939 |
| 2 | 10-19 02:11:36 to 21:18:47 | 19.12 | a spot interruption | 4339 |
| 3 | 10-19 21:22:41 to 10-20 07:28:07 | 10.09 | the budget of 36 hours | 4430 |
| 4 | 10-20 08:01:52 to 12:30:57 | 4.48 | the budget of 40.5 hours | 4547 |

Each interruption cost about 3 minutes before the group started the next machine, which restored the checkpoints and the
results from S3 and went on.  Machine 1 spent 12.5 minutes on the build and the release tests before its solver
started; the others took 0.4 minutes, with the tested binary from the build cache.  Machine 4 stopped in the middle of
game 4548, 151.6 million nodes and 1,603.8 s into its search; the checkpoint is in S3
(`s3://taxman-013764204840-us-east-1/wall-to-5000/ckpt/`), and nothing is left running.

**The relaunch that idled.**  The machine that `wall.sh launch --continue` started to carry on past the 36 hours sat
idle for about 25 minutes, about $0.30.  The user data of `wall.sh` on master started `solver/tools/cloud/wall-node.sh`,
but the pinned commit ca357bf has the layout from before the restructure of 2026-10-18, with the script in
`rust/tools/cloud/`.  The tmux session died at once and tmux reported success, so the machine recorded no boot, counted
no hours and wrote no log.  It was found through SSM, the group set to capacity 0 and `wall.sh` fixed (2611d42, 07:59Z):
the user data finds the script in either layout, ends the run if neither exists, and ends it if the tmux session is gone
a few seconds after the start.  Machine 4 then started at 08:00:56Z and resumed game 4431 from its checkpoint.

**The cost.**  The ledger counted 40.4166 instance-hours, $27.32 at $0.676: the solver's 40.15 hours (144,557 s) and
setup, mostly machine 1's 13 minutes.  The idle machine is outside the ledger, so the whole run cost about $27.6.  The
lines average 206 thousand nodes per solver second, so a billion nodes cost 1.35 solver hours, about $0.91.

**What it made.**  The 633 games 3915..4547, each built from the one before:

* 207 searches (150 exhausted, 57 at the ceiling), 204 repeats and 55 primes, and 167 games taken from the store.
* The lines are in `results/strict.jsonl`, appended byte for byte (all 633 equal).  The run's own file also holds
  `known` copies of 1..3914 and three incomplete lines (3940, 4340 and 4548), which stay out.
* The 167 store games assume [statement U](../../../docs/GLOSSARY.md#the-frame-model) at 129 boards in all; the other
  466 lines assume nothing beyond the frame model.
* Appending needed a narrower rule in both referees (eff2aab): the run's first line, 3915, records its game of 3914 as
  `known`, and a record of the proof `known` for the file's own game n − 1 needs only the same score, moves digest and
  assumptions.  [The runbook's page on the chain](../../docs/runbook/chain.md#verifying-and-bringing-the-results-home)
  has the splice.

| stretch | games | from the store | searches | nodes | search hours, summed | median of the exhausted searches |
|---|---|---|---|---|---|---|
| 3915-4000 | 86 | 16 | 30 | 10.29 G | 23.4 | 992 s |
| 4001-4100 | 100 | 22 | 39 | 1.16 G | 2.4 | 95 s |
| 4101-4200 | 100 | 1 | 45 | 3.15 G | 6.6 | 379 s |
| 4201-4300 | 100 | 1 | 44 | 4.68 G | 10.7 | 498 s |
| 4301-4400 | 100 | 39 | 25 | 2.82 G | 4.7 | 959 s |
| 4401-4500 | 100 | 55 | 19 | 7.06 G | 15.5 | 4,083 s |
| 4501-4547 | 47 | 33 | 5 | 0.59 G | 1.2 | 770 s |
| all | 633 | 167 | 207 | 29.75 G | 64.5 | |

A game's search hours are the hours it was in flight, shared with the games beside it, so their sum is more than the
solver's 40.15 hours.

**The dearest searches**, by nodes: 3940 at 1.545 G (2.98 hours in flight, over two invocations), 3930 at 1.402 G, 4225
at 1.366 G, 3942 at 1.198 G and 3936 at 1.160 G.  The longest in flight was 3942 at 4.43 hours; 178 of its 265 progress
lines, a minute apart, show under 100,000 nodes, the search waiting behind 3940.  22 searches were in flight over an
hour.  The 57 proofs at the ceiling took 3,093 nodes at the most; 3933 was in flight 5,785 s, nearly all of it waiting
behind 3930.

## Decision

The 633 games are committed, and the run stopped in game 4548.  What a next run should know:

* **The cost of a stretch varies tenfold.**  A hundred games took from 0.6 to 10.3 billion nodes (1 to 14 solver hours),
  and the last stretch did not predict the next: the median exhausted search was 95 s over 4001..4100 and 4,083 s over
  4401..4500.  115 searches of 30 million nodes or more took 29.24 G of the 29.75 G nodes.  The dearest, 3940, is twice
  the dearest of the third run's new games (3441, 0.79 G).
* **The store reaches 4783.**  It held 215 proven boards in 4548..4783 (the first is 4550) and none in 4784..5000, so
  238 of the 453 games to 5000 are not in it.
* **A checkpoint belongs to its build.**  The checkpoint of 4548 resumes only on ca357bf; another build starts the game
  over.
* **A pinned commit has its own layout.**  `wall.sh` of master starts the script of the commit it pins; since 2611d42 it
  finds the script in either layout and ends the run if the session dies.

Not kept in the repository: the run's own results file (30 MB; its lines of 3915..4547 are the committed ones), the
checkpoint of 4548 (4.6 MB) and `run/boots/` (one object, the last machine's).  The S3 prefix holds all three.

## Files

The run's data is in S3 under `s3://taxman-013764204840-us-east-1/wall-to-5000/`; this directory holds its logs, its
clock and the script that makes the numbers above.  The games are in [results/strict.jsonl](../../../results/README.md).

| file | command | what it is |
|---|---|---|
| `run_stats.py` | `./run_stats.py` (from anywhere; standard library) | the kinds of the 633 lines, the table by stretch, the dearest searches, the machines, the cost, the store's boards past the run's end and the stopped game; from the committed file, `results/bounds.jsonl`, `logs/` and `run/` |
| `run/config`, `run/ledger`, `run/ended` | written by `wall.sh launch` and the machines | the settings (commit, window, wall, budget 40.5 h, `--use-prev`, price, `RELEASE_TESTS`), the clock (40.4166 hours at the end) and the ending (2026-10-20T12:31:00Z, the budget) |
| `logs/chain.log`, `logs/chain.out.gz`, `logs/chain.err.gz` | the `.gz` are gzipped | `chain.sh`'s log for the four machines; the solver's output, a line a game; its progress lines, a minute apart for each game in flight |
| `logs/setup.log`, `logs/release-tests.txt` | | `setup.sh` on each machine (the build, the smoke checks of `optimal.json`, both referees and the first 400 games) and the release suite's summary (311 s on 64 threads) |
| `logs/wall-node.log`, `logs/spot-watch.log`, `logs/save.log.gz` | | the node script's events, the two spot interruption notices (02:08:24Z and 21:18:46Z on 10-19) and the saves to S3 every 10 minutes |

## Commits

* ca357bf: the build the run pinned (the store's stage 3 of 3900..4999).
* 2611d42: `wall.sh`, the user data finds the script in either layout.
* eff2aab: the referees accept a record of the proof `known` for the game before.
* e542c51: games 3915..4547 appended, with the tests and the results README.
* cef0884: the merge.
* a69a597: this entry.
