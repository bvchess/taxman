# The run past 5000 (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-20.  Concluded 2026-10-21.

**Question.**  How far does one more unattended cloud run take the [committed
chain](../../../docs/GLOSSARY.md#results-and-trust) from game 4547, when it takes the games already proven from that
chain and from the [bounds store](../../../docs/GLOSSARY.md#results-and-trust) and searches only the rest, on a build
with the per-step speed-ups?  **Answer.**  To game 4874: 327 new games for 18.86 instance-hours and $13.58 at the
ledger's price, about $12.5 at spot.  The account's owner stopped the run in game 4875, which had searched 6.89 billion
nodes and had not finished.  The stop was the owner's choice, to plan different hardware for the next run; the budget
(47 instance-hours), the date and the wall were far off.  The store gave 215 of the games with no search, all that it
had proven from 4548 to 4783; the 91 games after it cost 7.45 billion nodes, and 4875 took 6.89 more.

## Method

The run went from 2026-10-21 at 02:04:13Z to 20:42:30Z, on commit 4974a71 with a clean tree.  `wall.sh launch` set up an
Auto Scaling group of one spot `c8g.16xlarge` (64 threads) with the lowest-price strategy, a spot ceiling of $0.72 an
hour and the zone us-east-1b ([the unattended run to the wall](https://github.com/bvchess/taxman/blob/3ca11bf/solver/docs/runbook/wall.md)).  The machine ran `taxman
solve 1-8191 --use-prev` with a [window](../../../docs/GLOSSARY.md#the-search) of 4 games in flight, the default tactics
of that commit and the [wall](../../../docs/GLOSSARY.md#the-search) at 24 hours (`--game-hours 24`).  With `--use-prev`
it took games 1..4547 from the committed chain and each game the store had proven from the store, and searched the
others from 4548.  The account's owner approved $35 for it: a budget of 47 instance-hours (the ledger counts at the spot
ceiling, so $33.84), until 2026-10-25T00:00Z.

The machine's user data ran at 01:51:23Z.  `setup.sh` built the solver (2 min 35 s), checked the committed results with
both referees and ran the release suite (`RELEASE_TESTS=1`: 723 passed, 0 failed, 28 ignored, 305 s for the tests and
585 s with the build of the tests, 4 min 39 s of it).  It stored the binaries in the build cache, and the solver started
at 02:04:13Z, 12.8 minutes after the machine's script.

## Results

One machine ran the whole run, and nothing interrupted it: a rebalance recommendation at 15:13:00Z was the only notice
the spot watch logged.  The solver made one invocation of 67,097 s (18.64 hours).

**The stop.**  The account's owner ran `wall.sh stop` at 20:42:12Z.  The node script found the request at 20:42:30Z and
told `chain.sh` to stop; the solver took the signal, wrote a final checkpoint and exited 143.  The run ended "stopped on
request" at 20:42:33Z, and `wall.sh cleanup` deleted everything the launch had made, so nothing is left running.  The
budget had 28 instance-hours left, the date three days, and game 4875's search had been in flight 9.1 hours of the
wall's 24.

**The cost.**  The ledger counted 18.86 instance-hours, $13.58 at its $0.72: the solver's 18.64 hours and the setup's
0.22. The zone's spot price stayed between $0.658 and $0.662 an hour during the run, so the instance cost about $12.45,
and a 40 GB volume for a day about $0.10.  The solver searched 16.08 G nodes in its 67,097 s: 9.185 G in the 327
recorded games and 6.893 G in 4875.  That is about 240 thousand nodes per solver second, against 206 thousand for [the
run to 5000](../the-run-to-5000-2026-10/README.md), which counted the nodes of its recorded games only and played other
games; so a billion nodes cost 1.16 solver hours.

**What it made.**  The 327 games 4548..4874, each built from the one before:

* 35 searches, all exhausted;
* 14 proofs at the ceiling, of method `start` (the search took no step);
* 13 primes and 50 repeats (`same_search`);
* 215 games known from the store with no search: all 215 that it had proven in 4548..4783, in the stretches 4550-4640,
  4650-4759 and 4770-4783.  The store lacked the other 21 games of 4548..4783.

The lines are in `results/strict.jsonl`, appended byte for byte, but for 14 lines in which `"method":"search"` became
`"method":"start"`.  That is the rule of commit 7987c86 for the lines of a build before it (a completed search with 0
nodes); the n are 4644, 4785, 4794, 4797, 4810, 4816, 4818, 4824, 4833, 4842, 4844, 4845, 4850 and 4869.  The run's own
file also holds `known` copies of 1..4547 and the incomplete line of 4875, which stay out.  Every kept line records the
build 4974a71 (`dirty` false), 64 threads, one invocation, the `default` tactics and the option `use_prev`.  The 215
store games assume [statement U](../../../docs/GLOSSARY.md#the-frame-model) at 169 boards in all, and the other 112
assume nothing beyond the frame model; in the whole chain 382 games assume U, at 298 boards.

Both referees pass the spliced chain: `taxman verify-results` reports 4874 completed games with none missing, and 4547
games in more than one file with equal scores in all, and `solver/tools/check_results.py` ends "all checks passed" ([the
splice](../../docs/runbook/chain.md#verifying-and-bringing-the-results-home)).  The bounds store's referees pass too, on
`results/bounds.jsonl`, which has not changed since commit 4974a71.  `ledger verify-bounds results/bounds.jsonl --strict
results/strict.jsonl --workers 4` ended "all checks passed" in 4 min 6 s: the store proves 382 boards (the best lower
bound equals the best upper bound), and 960 of its boards are games of the chain, whose scores its bounds enclose.
`ledger games`, then `solver/tools/check_bounds.py results/bounds.jsonl --games games.jsonl`, ended "all checks passed"
as well.  The run took 215 of the 382 proven boards as games.

**How the cost grew.**

| stretch | games | from the store | searches | nodes | search hours, summed | median of the exhausted searches |
|---|---|---|---|---|---|---|
| 4548-4597 | 50 | 48 | 1 | 0.15 G | 0.1 | 396 s |
| 4598-4647 | 50 | 43 | 1 | 0.15 G | 0.1 | 410 s |
| 4648-4697 | 50 | 48 | 1 | 0.07 G | 0.1 | 201 s |
| 4698-4747 | 50 | 50 | 0 | 0 | 0 | |
| 4748-4797 | 50 | 26 | 9 | 2.07 G | 3.5 | 1,248 s |
| 4798-4847 | 50 | 0 | 16 | 1.66 G | 3.6 | 771 s |
| 4848-4874 | 27 | 0 | 7 | 5.08 G | 7.5 | 3,878 s |
| all | 327 | 215 | 35 | 9.18 G | 14.9 | |

A search here is a game of method `search`; the 14 of method `start` took no step.  A game's search hours are the hours
it was in flight, shared with the games beside it, so their sum is more than the solver's 18.64 hours.

The store's help ended at 4783: it held 215 of the 236 games of 4548..4783 and none after.  The 91 games 4784..4874 came
from the frame model alone and took 7.45 G of the 9.18 G nodes: 27 searches, 13 proofs at the ceiling, 41 repeats and 10
primes, a median of 780 s for the 27 exhausted searches.  The first game, 4548, which the previous run had left in
flight on another build, was searched from its start: 150.9 million nodes and 396 s.

**The dearest games**, by nodes: 4864 at 1.133 G (4,636 s), 4865 at 1.012 G (8,995 s), 4872 at 0.945 G (4,238 s), 4860
at 0.857 G (3,878 s), 4856 at 0.532 G (2,419 s) and 4764 at 0.437 G (3,078 s).  By time: 4865, 4864, 4872, 4860, 4764
and 4767 (2,501 s, 0.250 G).  4865 was in flight longest because it waited: 77 of its 149 progress lines, a minute
apart, show under 100,000 nodes, the search waiting behind 4864 (24 of 4764's 51 wait the same way).  Four searches took
over an hour and 11 over half an hour.  In the recorded games the queue never held more than 5,617 units, and none
spilled.

**Game 4875, in flight at the stop.**  It had searched 6,893,066,218 nodes in 32,754 s, the waits included: it waited
about 71 minutes behind 4872 (70 progress lines under 100,000 nodes) and then searched 28,517 s (7.92 hours) on the
wall's clock.  That is 75% of the nodes of the 327 recorded games, and 1.85 times those of the committed chain's dearest
game, 3440 (3.72 G).  Its best sum of promotions is 538080, a score of 7476374 if it holds; that is not proven.  The
search began at 537927, and its progress lines first show 537962 at 1.02 G nodes, 537971 at 1.48 G, 538068 at 4.05 G and
538080 at 4.30 G; the 2.59 G nodes and about 3 hours after that found nothing better.  Its queue held 10,058 units at
most, and none spilled.  Its checkpoint and clock (`n4875.ckpt`, 16.5 MB, and `n4875.clock`) are kept outside the
repository, in the run's download, and resume on commit 4974a71 or on f48df13 (master).

**The finder.**  While the run went on, another line of work found a game of 538080 for 4875 in seconds with a heuristic
finder, where the run's search took 4.3 billion nodes to reach it.  That work built `--finder-after NODES
--finder-seconds S`, which starts a finder beside a long search ([the
entry](../search/the-finder-on-long-searches-and-game-4875-2026-10/README.md) has the option and its measurements).  Its
resume test resumed a copy of this run's 4875 checkpoint on the new build with no refusal.  A switch to that build after
4875 was planned, at a cost of about 0.3 instance-hours, and dropped when the owner stopped the run.

## Decision

The 327 games are committed, and the run stopped in game 4875 by the owner's choice.  What a next run should know:

* **The next run waits on a hardware plan.**  The owner stopped this one to plan different hardware for it.
* **Game 4875 has 6.89 G nodes behind it** and an unproven best of 538080.  Its checkpoint is kept, and the finder is an
  option for it and for any search that proves long.
* **The store has nothing past 4783.**  Its 382 proven boards end there, so the 126 games 4875..5000 are all searches or
  shortcuts.
* **The cost jumped past 4848.**  The last 27 games took 5.08 G nodes; 4864, 4865, 4872 and 4860 alone took 3.95 G, 43%
  of the run's, and 4875 took 6.89 G more without finishing.

Not kept in the repository: the run's own results file (34 MB; its lines of 4548..4874 are the committed ones), the
checkpoint of 4875 and its clock (16.5 MB) and `run/boots/` (one object, the machine's).  They stay in the run's
download, outside the repository.

## Files

The run's data was on the machine and in its saves; this directory holds its logs and its run files.  The games are in
[results/strict.jsonl](../../../results/README.md).

| file | what it is |
|---|---|
| `run/config`, `run/ledger`, `run/ended`, `run/stop` | written by `wall.sh launch`, the machine and `wall.sh stop`: the settings (commit, window, wall, budget 47 h, `--use-prev`, price $0.72, `RELEASE_TESTS`, until 2026-10-25), the clock (18.86 hours at the end), the ending (2026-10-21T20:42:33Z, stopped on request) and the stop request |
| `logs/chain.log`, `logs/chain.out.gz`, `logs/chain.err.gz` | `chain.sh`'s log; the solver's output, a line a game; its progress lines, a minute apart for each game in flight |
| `logs/setup.log`, `logs/release-tests.txt` | `setup.sh` (the build, the smoke checks of `optimal.json`, both referees and the first 400 games) and the release suite's summary |
| `logs/wall-node.log`, `logs/spot-watch.log`, `logs/save.log.gz` | the node script's events, the rebalance recommendation, and the saves every 10 minutes |

The stretch table, the dearest games and the clock above come from [run_stats.py of the previous
run](../the-run-to-5000-2026-10/run_stats.py).  The script reads the committed chain at a range and a list of stretches
written in its constants, so it could not take this run's as it stands.  We ran a copy of it over the logs and run files
of this folder, with the range 4548..4874, stretches of 50 games, the run's own results file (the committed lines of
these games are the same, but for the 14 relabeled) and a line that names `start` a completed search of 0 nodes, as the
committed file does.  The copy is not kept.  The waits, the queue and the incumbent of 4875 come from the progress lines
of `logs/chain.err.gz`.

## Commits

* 4974a71: the build the run pinned.
* 7987c86: the method `start`, the rule that relabels 14 of the run's lines.
* c1c1a2a, af1b0e9 and b6fa5e5: `wall.sh status` gained, during the run, an exit 0 before the first save with labeled
  group lines, the local time zone in its header comment, and a table of the last 6 games recorded above the games in
  flight (`--recorded N`).
* 738465d: the finder beside a long search; f48df13, its entry.
* The games 4548..4874 appended to `results/strict.jsonl`: `git log --oneline -- results/strict.jsonl`.
