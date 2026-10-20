# The node choice: where the strong bound pays (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-18.  Concluded 2026-10-18.

**Question.**  The search prunes a node with the GF(2) bound, which is cheap (a step costs 0.14 to 0.34 ms at
2700-3441).  The Lagrangian bound over the nogoods, its multipliers refitted at the node, is stronger and slow.  Over
the nodes of a proof, what share does a refit cut as a function of the node's gap, and above what forecast subtree does
computing it pay?  **Answer.**  It never pays: the refit cuts 92% of the open nodes at gaps under 50 and none at 300 or
more, where the cost is, it costs a median 164-234 times the GF(2) bound, and every policy loses (computing it
everywhere: −85 CPU s, −2%).

[Multipliers fitted once at the root](../root-fitted-nogood-multipliers-at-the-nodes-2026-10/README.md) cut only 2-6%
of the open nodes at 2700-3440; [multipliers fitted near the node](../how-far-can-nogoods-go-2026-10/README.md) cut 84%
and 62% of those at the probe's nodes, at 12-46 ms a node.  A search under a floor grows a decade of steps per 147
points ([floors](../proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md)), which is what
makes the gap a forecast of the subtree.

## Method

**The instrument** is two commands of the `wall` example, defined in
[`examples/wall/choice.rs`](../../../frame/examples/wall/choice.rs), whose module comment defines every field of every
line: `choice-prepare` makes a game's file (the nogood pool harvested
at the root as [the nogoods entry](../nogoods-from-the-finders-work-2026-10/README.md) did, the multipliers fitted
there, and the nodes), and `choice` measures every node of such files.  For each node it records:

1. **the gap**: the search's bound (`NodeBound`, the GF(2) bound of a unit) less what the node needs to beat the game's
   incumbent (the known optimum, as in a proof; for 3441, which the chain has not reached, the incumbent of the tuning
   suite's checkpoint);
2. **the subtree**: the steps of the search to solve the node to the end, as `tune solve` does (one thread, fresh
   tables, the default forgetting), stopped after `LIMIT` CPU seconds (900; none stopped);
3. **the strong bound**: the refit at the node over the nogoods live there, `NODE_STEPS` (200) subgradient steps from
   the root's multipliers, stopping at the first step that cuts; the lowest bound and the seconds after every step, and
   the same fit with the nogoods that the solve's searcher held at its end besides the pool's (`sub`: hindsight, an
   upper limit on what the searcher's own table could give at the node);
4. **the times** of the search's bound at the node (`today_s`, from the root's matroids as a search starts a unit) and
   of the matroids built afresh (`graph_s`) and the GF(2) bound solved on them (`plain_s`), the base of the ratio;
5. **the A1 bounds**, with `--a1` ([below](#the-a1-bounds)).

**The nodes:**

| source | games | nodes | what they are |
|---|---|---|---|
| suite | 3441, 3440, 3300, 3425 | 10 a game | the tuning suite's units ([tuning campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)): the end of the chain, gaps 746-1,001, subtrees of 33,000 to 1.25 million steps (9 to 314 CPU s); 3441's incumbent is the checkpoint's |
| made | 3213, 3260, 3129, 2976, 3267, 2700 | 60 a game | units of the proof of the game seeded with its optimum (`perf make-nodes N --every 2000 --keep 300`, as for [the root-fitted entry](../root-fitted-nogood-multipliers-at-the-nodes-2026-10/README.md); proofs of 110,000 to 440,000 steps), the ones the search's bound leaves open, spread over their gaps |
| probe | 936, 1215 | 60 a game | the probe's fixtures (`testdata/perf/n0936.nodes`, `n1215.nodes`), the same way |
| path | every game | 3 a game | nodes on the way to the optimum, whose best completion is known: only a check that no step of a fit goes below it |

A node that the search's bound cuts has no subtree to speak of and is not kept.  That left 553 nodes, 520 of them open
with a completed solve and the rest path nodes.

**The policy** "compute the strong bound where the forecast subtree exceeds N" has a net saving: the CPU seconds of the
subtrees it cuts less those of the bounds it computes.  The forecast is a fit of log10(subtree) to the gap on the
completed solves, or the floor entry's decade per 147 points with the intercept fitted; `analyze.py --steps K` reads
only the first K steps of every fit.

**The run** (2026-10-18 17:54:17Z to 18:02:38Z, commit 9d9dae3) used one `c8g.4xlarge` (16 CPUs, 16 workers of one
thread, the binaries from the build cache) launched at 17:48:31Z, which powered itself off at the end, for a few cents.
It made the six fixtures from proofs (2 minutes), the pool of nogoods and the root fit of each of the twelve games
(30 s), then measured the 553 nodes (6 minutes).  The pools held 263 to 5,825 nogoods (about 3,300 at the end of the
chain) and the root fits closed 0.23 to 0.52 of the root gap; the nodes took 4,636 CPU s, 4,141 of them the suite's
forty.  With the setup's build (about 10 minutes; none from the cache) the machine is up about 40 minutes, for about
30 cents at spot ($0.17 an hour).

### The A1 bounds

The question: does [the A1 bound](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md) (the GF(2)
frame matroids and signed GF(3) ones, coupled by multipliers fitted by column generation), fitted fully at a node, cut
the nodes that head big subtrees?  Its [E3 pilot](../the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md)
cut trees fivefold in a branch and bound; this asks it at the nodes that matter, as the nogood bound was asked.  The
code is [`examples/wall/fields.rs`](../../../frame/examples/wall/fields.rs) (the study's, from commit 8753189,
adapted), and at each node `choice --a1` measures two bounds beside the nogood bound:

* **r2**, the study's warm refit: the multipliers of the game's root fit (one evaluation of the bound), then at most
  `--a1-pricings` (20) more evaluations of a column generation started from the root's master columns and rank
  inequalities that still hold at the node, with no exact check and no separation;
* **full**, the node's own master: from the root fit's multipliers and signings, the column generation with the exact
  checks and the separation of the node's own cores, stopping at the first bound that cuts, within `--a1-seconds`
  (30); a node it did not cut within that is *capped*.

Each is the lowest value of the bound it evaluated (valid at any multipliers and signings: Theorems 14 and 15 of
docs/THEORY.md), so a bound that ran out of time is a bound, not a failure.  `choice-prepare --fit-seconds S` makes
each game's root fit once (`solver/frame/testdata/fits` has 1215's, 2700's and 3300's).  The options and the results are
in [the A1 bound at the nodes](../the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/README.md).

## Results

The refit with all steps, the share of the open nodes it cuts (`cloud-20261018/analysis.txt`):

| gap | nodes | root's multipliers | refit at the node | subtree, median steps | refit, median seconds |
|---|---|---|---|---|---|
| 0-49 | 121 | 51% | 92% | 2 | 0.002 |
| 50-99 | 92 | 22% | 65% | 6 | 0.008 |
| 100-199 | 126 | 6% | 19% | 22 | 0.13 |
| 200-299 | 72 | 0% | 4% | 58 | 0.28 |
| 300 and more | 109 | 0% | 0% | 185 to 676,000 | 0.3 to 0.5 |

* **The refit cuts only small subtrees.**  Every node with a subtree of a hundred steps or more (138 of the 520, among
  them all the suite's) is cut by nothing; the refit cuts 76% of the nodes whose subtrees are under ten steps and 18%
  of those of ten to ninety-nine.
* **The subtree follows the gap**: log10(steps) = 0.32 + 0.00573 gap (a decade per 174 points, r² 0.878; a decade per
  147 with the floor entry's slope).
* **The refit is dear.**  It costs a median 164 to 234 times the search's bound at the node at 2700-3441 (13 and 32
  times at 936 and 1215).
* **Every policy loses, at every N.**  Searching all 520 nodes costs 4,460 CPU s; computing the refit at every node cuts
  198 of them and loses 85 CPU s (2%), at nodes forecast above ten steps 77 s, above a hundred 45 s, above ten thousand
  24 s, above a million 1 s.  By source the loss at N = 0 is 62 s of 363 (17%) on the six made games, 1.5 s of 0.6
  (252%) on the probe's nodes and 21 s of 4,100 (1%) on the suite.  The result is the same with the floor entry's slope
  (`analysis-floor.txt`), and with only the first 50 steps of each fit (`analysis-steps50.txt`: cuts of 92%, 63%, 16%
  and 0% by the same gaps; a loss of 22 s, 1%).
* **The bound was never wrong.**  The 33 path nodes only check that no fit goes below their best completion, and none
  did (the faults file was never written).

## Decision

**Not worth building.**  The refit cuts where the node is within about 200 points of the incumbent, and there the
subtree is a few dozen steps, cheaper to search (0.14 to 0.34 ms a step) than to fit.  Where the subtrees are large,
at gaps of 300 and more and everywhere in the end of the chain's dear searches (gaps 746-1,001), it cuts nothing.  The
cost of a game comes from the nodes with large gaps, so a stronger bound of this kind at the nodes does not change the
tenfold per 1,040 in n.

The measurement leaves some things open, which could reopen the question:

* **The sample is not a search's population.**  Nodes are spread over gaps (the made and probe sources) or picked for
  size (the suite), so the shares of the total are the sample's; the cut share by gap and the break-even forecast are
  what carry over.
* **Each node is a problem of its own.**  The saving counted is the whole subtree when the bound cuts the node; a
  search that applies the policy goes on to apply it at the children of a node it does not cut, which a node's line does
  not show.
* **Costs are charged generously.**  The matroids are built afresh for the fit, and the ratio is against a bound solved
  from the root's matroids; a search keeps its matroids and its levels along the path, which is cheaper still.
* **The pool is the root's, with multipliers only.**  The nogoods are those the finder and the sound search learn from
  the standalone start in 10 CPU s, not the table of the searcher at the node; `sub` is the other end (the table at
  the end of the node's own solve), so the truth for a searcher's table lies between them.  The fit reweights the
  pool and adds no nogood (the relax-and-cut of [how far nogoods can go](../how-far-can-nogoods-go-2026-10/README.md),
  which adds the conflicts of each heaviest set, is not here).
* **One thread, the default tactics, the optimum as the incumbent.**  The suite's 3441 units are solved from the
  checkpoint's incumbent, which a subtree could beat (none did in the suite's making).
* **The A1 bounds start from the root's fit**, as there is no parent to hand a node its state: `r2` has the root's
  columns that hold the node's promotions (few at depth), where the pilot's had its parent's, so its cut share here is
  a lower bound on what a tree would get.

## Files

* `cloud.sh`: the script of the run.  Its comment has the environment (`WORKERS`, `GAMES_SUITE`, `GAMES_MADE`,
  `GAMES_PROBE`, `KEEP`, `PATHS`, `CPU`, `ROOT_STEPS`, `NODE_STEPS`, `LIMIT`, `SUB`, `A1`, `FITS`, `FIT_SECONDS`,
  `A1_PRICINGS`, `A1_SECONDS`, `OUT`, `S3_DEST`, `POWEROFF`).  Its stages: the six proofs that make the fixtures, one
  process each (`perf make-nodes`, a few minutes); `wall choice-prepare` for the twelve games, one process each (the
  finder and the sound search 10 CPU s each for the pool, the root fit, the sampling of the nodes: about a minute);
  `WORKERS` processes of `wall choice`, each taking the nodes of a game one by one, the heaviest first, by making a file
  of the node's name in `claims/`, game after game (the suite's first); and `analyze.py` three ways.  A run again with
  the same `OUT` goes on where it was.  The script exits with status 1 if `faults` is not empty (a bound below a node's
  best completion, a harvested nogood that can be promoted, a cut of a node that holds a better set: bugs).
* `analyze.py` reads the lines and prints the sample; the share of the open nodes each bound cuts by gap and by the
  decade of the subtree; how the A1 bounds ended; and, for a range of N, the policy table (for the nogood bounds and
  for the A1 bounds): the nodes it is computed at, the nodes it cuts, and the net saving as a share of what searching
  every sampled node costs, with the best N.  `--model fit|floor` picks the forecast and `--steps K` reads the first K
  steps of every fit.
* `test_analyze.py` checks the arithmetic on made-up lines (`python3 -I test_analyze.py`).
* `cloud-20261018/`, the output of the run:
  * `analysis.txt`, `analysis-floor.txt`, `analysis-steps50.txt`: the script's three analyses (all the steps; the
    floor entry's slope; the first 50 steps);
  * `log`: the stages' times (UTC) and the commit;
  * `prepare.jsonl`: the line each `wall choice-prepare` printed (suite, made, probe games in the script's order): the
    nogood pools, the root's gap, the 200-step root fit (`relaxed_root`, `share` of the gap it closes, `fit_cpu_s`) and
    the nodes kept;
  * `fixtures.txt`: the six proofs that made the fixtures (`perf make-nodes`): runs, nodes and distinct units;
  * `workers.jsonl`: a line for each worker and game: the nodes it measured, skipped or left to other workers, and its
    CPU seconds (4,636 in all);
  * `measure.jsonl.gz`: the 553 lines of `measure/worker-0.jsonl` to `worker-15.jsonl`, joined in the order of the
    workers and gzipped.  `gunzip -c measure.jsonl.gz > measure.jsonl`, then `python3 -I analyze.py measure.jsonl
    [--model floor] [--steps 50]` makes the analyses again, byte for byte.

Not kept: `prep/` (the files of `wall choice-prepare`, up to 2 MB a game), `fixtures/` (the nodes of the six proofs,
70-250 KB a game) and `logs/` (each worker's output), which the commands make again or `fixtures.txt` and
`prepare.jsonl` hold.

On a laptop, a handful of nodes, from the top of the repository after `cargo build --profile test --features studies
--example wall --example perf` in `solver/frame` (`P` a scratch directory):

```text
solver/frame/target/debug/examples/wall choice-prepare --known results/strict.jsonl \
    --fixtures solver/frame/testdata/perf --prep P/prep \
    --cpu 2 --steps 200 --keep 12 --paths 3 936 1215
solver/frame/target/debug/examples/wall choice --prep P/prep --results P/out --node-steps 50 --sub-nogoods --cap 120 \
    936 1215
python3 -I solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/analyze.py P/out
```

With the A1 bounds, add `--fit-seconds 60 --fits solver/frame/testdata/fits` to the preparation and `--a1
--a1-seconds 10` to the measurement.  A suite game is prepared with `--suite
solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl`; `choice` skips the nodes
`P/out` already has, and `--claims DIR` makes a node taken by whichever worker makes its file in `DIR` first.  The
tests, in `solver/frame`, are `cargo test --features studies --example wall -- choice` (a game below 1000: no step of a
fit, and neither A1 bound, goes below the best completion that an exhaustive search of the node finds, a cut agrees with
the solve from three incumbents, and a run again goes on where it was) and `-- fields` (the A1 code: the simplex, the
signed matroids, the cores; its intersection against the crate's bound at 120, 936, 2700 and 3300; its root fits of
`solver/frame/testdata/fits` at 987, 1215, 2700 and 3300 giving their recorded bounds again).

The run in the cloud, on one `c8g.4xlarge`, once `STUDIES=1 solver/tools/cloud/setup.sh` has the release examples
(`STUDIES=1` for the study tools, `wall` among them, which a default build skips), with user data like [the
ledger's](../../../tools/ledger/cloud.sh); the [rules for every
launch](../../../docs/runbook/launch.md#rules-for-every-launch) apply:

```text
#!/bin/bash
shutdown -h +90
export AWS_DEFAULT_REGION=us-east-1
dnf install -y -q git tmux
sudo -u ec2-user bash -lc 'cd ~ && git clone -q https://github.com/bvchess/taxman.git && cd taxman &&
    git checkout -q COMMIT'
sudo -u ec2-user tmux new-session -d -s choice -c /home/ec2-user/taxman \
    'export S3_DEST=s3://taxman-013764204840-us-east-1/node-choice-DATE
     STUDIES=1 BUILD="--bin taxman --example wall --example perf" REQUIRE_CACHED=1 SKIP_TESTS=1 \
         solver/tools/cloud/setup.sh > ~/setup.log 2>&1 && POWEROFF=1 cloud.sh;
     sudo shutdown -h now'                          # also if the setup failed
```

The binaries come from the build cache (`REQUIRE_CACHED=1`: the commit must have been released, and with `STUDIES=1`,
because `wall` is a study tool that a default build does not hold; see [the build
cache](../../../docs/runbook/build-cache.md)); a setup that fails ends the machine and the measurement does not start.

## Commits

* 8e2235e: `wall choice-prepare` and `choice`.
* 166a4d7: the script, the analysis and the README; merged as 9d9dae3, the build of the run.
* 189ffbe: the pools and root fits of the twelve games, the fixture proofs and the workers' CPU seconds of the cloud
  run, from the logs that were not kept.
* a7e4072: the entry.
* e8af0ea: the A1 bounds in `choice --a1` (`wall/fields.rs`), merged as 260e74f.
