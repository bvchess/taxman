# Node choice: at which nodes is a strong bound worth computing?

The search prunes a node with the GF(2) bound, which is fast (a step of the search costs about 0.3 ms at the end of the
chain) and leaves the gap that sets the cost of a game.  A stronger bound exists: the Lagrangian bound over nogoods,
with the GF(2) bound as its inner problem, whose multipliers are fitted at the node.  It is slow (a fit is tens of
GF(2) bounds: 50 steps cost 0.09 s at a node of 3425) and it cuts nodes the GF(2) bound leaves open, but the nodes it
cuts have to be worth the time: a node with a subtree of ten steps is cheaper to search than to fit.  [Root-fitted multipliers](../../docs/experiments/root-fitted-nogood-multipliers-at-the-nodes-2026-10.md)
cut only 2-6% of the open nodes at 2700-3440; [multipliers fitted near the node](../../docs/experiments/how-far-can-nogoods-go-2026-10.md)
cut 84% and 62% of those at the probe's nodes of 936 and 1215, at 12-46 ms a node.  The question here: **over the
nodes of a proof, what share does a refit cut as a function of the node's gap, and above what forecast subtree does
computing it pay?**  [A search under a floor](../../docs/experiments/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10.md)
grows about tenfold per 147 points of gap, which is what makes the gap a forecast of the subtree.

Defined in [`examples/wall/choice.rs`](../../examples/wall/choice.rs), whose module comment defines every field of
every line: two commands of the `wall` example, `choice-prepare` (a game's file: the nogood pool harvested at the root
as [the nogoods entry](../../docs/experiments/nogoods-from-the-finders-work-2026-10.md) did, the multipliers fitted
there, and the nodes) and `choice` (every node of such files).  For each
node it records:

1. **the gap**: today's bound (`NodeBound`, the GF(2) bound of a unit) less what the node needs to beat the game's
   incumbent (the known optimum, as in a proof; for 3441, which the chain has not reached, the incumbent of the tuning
   suite's checkpoint);
2. **the subtree**: the engine's steps for today's search to solve the node to the end from the incumbent, as `tune
   solve` does (one thread, fresh tables, the default forgetting; the line of a suite unit has the suite's own count in
   `hint`, and the two were the same on the smallest unit of 3425, 33,333 steps), stopped after `LIMIT` CPU seconds
   (then `completed` is false);
3. **the strong bound**: the refit at the node over the nogoods live there, `NODE_STEPS` subgradient steps from the
   root's multipliers, stopping at the first step that cuts; the lowest bound after every step (`lows`), the seconds
   into the fit at each (`ts`) and the CPU seconds in all (`s`); and the same fit with the nogoods that the solve's
   searcher held at its end besides the pool's (`sub`: hindsight, an upper limit on what the searcher's own table
   could give at the node);
4. **the times** of today's bound at the node (`today_s`, from the root's matroids as a search starts a unit),
   and of the matroids built afresh (`graph_s`) and the GF(2) bound solved on them (`plain_s`), the base of the ratio.

`analyze.py` reads the lines and prints the tables the entry will quote: the sample; the share of the open nodes each
bound cuts by gap and by the decade of the subtree; and for a range of N, the policy "compute the strong bound where
the forecast subtree exceeds N": the nodes it is computed at, the nodes it cuts, and the net saving, the CPU seconds of
the subtrees it cuts less the CPU seconds of the bounds computed, as a share of what searching every sampled node
costs, with the best N.  The forecast is a fit of log10(subtree) to the gap on the completed solves (`--model fit`) or
the floor entry's decade per 147 points with the intercept fitted (`--model floor`); `--steps K` reads only the first K
steps of every fit.  `test_analyze.py` checks the arithmetic on made-up lines (`python3 -I bench/node_choice/test_analyze.py`).

## The nodes

| source | games | nodes | what they are |
|---|---|---|---|
| suite | 3441, 3440, 3300, 3425 | 10 a game | the tuning suite's units ([tuning/README](../tuning/README.md)): the end of the chain, gaps 746-1001, subtrees of 33,000 to 1.25 million steps (9 to 314 CPU s); 3441's incumbent is the checkpoint's |
| made | 3213, 3260, 3129, 2976, 3267, 2700 | `KEEP` (60) a game | units of the proof of the game seeded with its optimum (`perf make-nodes N --every 2000 --keep 300`, as for [the root-fitted entry](../../docs/experiments/root-fitted-nogood-multipliers-at-the-nodes-2026-10.md); proofs of 110,000 to 440,000 steps), the ones today's bound leaves open, spread over their gaps |
| probe | 936, 1215 | `KEEP` (60) a game | the probe's fixtures (`testdata/perf/n0936.nodes`, `n1215.nodes`), the same way |
| path | every game | `PATHS` (3) a game | nodes on the way to the optimum, whose best completion is known: only a check that no step of a fit goes below it |

About 520 nodes, and the path nodes.  A node today's bound cuts has no subtree to speak of, so it is not kept.

## Commands

On a laptop, a handful of nodes (from `rust/`, with `cargo build --profile test --features studies --example wall
--example perf`, the non-release build; `P` a scratch directory):

```text
target/debug/examples/wall choice-prepare --known results/strict.jsonl --fixtures testdata/perf --prep P/prep \
    --cpu 2 --steps 200 --keep 12 --paths 3 936 1215
target/debug/examples/wall choice --prep P/prep --results P/out --node-steps 50 --sub-nogoods --cap 120 936 1215
python3 -I bench/node_choice/analyze.py P/out
```

A suite game is prepared with `--suite bench/tuning/suite.jsonl`; `choice` skips the nodes `P/out` already has, and
`--claims DIR` makes a node taken by whichever worker makes its file in `DIR` first.  Tests: `cargo test --features
studies --example wall -- choice` (a game below 1000: no step of a fit goes below the best completion that an
exhaustive search of the node finds, a cut agrees with the solve from three incumbents, and a run again goes on where it was).

In the cloud, on one `c8g.4xlarge` (16 CPUs), once `STUDIES=1 tools/cloud/setup.sh` has the release examples, built or
taken from the build cache (`STUDIES=1` for the study tools, `wall` among them, which a default build skips):
[`cloud.sh`](cloud.sh), whose comment has the environment (`WORKERS`, `GAMES_SUITE`, `GAMES_MADE`, `GAMES_PROBE`,
`KEEP`, `PATHS`, `CPU`, `ROOT_STEPS`, `NODE_STEPS`, `LIMIT`, `SUB`, `OUT`, `S3_DEST`, `POWEROFF`).  Its stages, from
`rust/`:

1. the six proofs that make the fixtures, one process each (`perf make-nodes`, a few minutes);
2. `wall choice-prepare` for the twelve games, one process each (the finder and the sound search 10 CPU s each for the
   pool, the root fit, the sampling of the nodes: about a minute);
3. `WORKERS` processes of `wall choice`, each taking the nodes of a game one by one, the heaviest first, by making a
   file of the node's name in `claims/`, game after game (the suite's first);
4. `analyze.py` three ways: all the steps (`analysis.txt`), the floor entry's slope (`analysis-floor.txt`) and the
   first 50 steps (`analysis-steps50.txt`).

A run again with the same `OUT` goes on where it was (a fixture or a prepared file that is there is kept, and the nodes
in `measure/*.jsonl` are not measured again).  The script exits with status 1 if `faults` is not empty (a bound below a
node's best completion, a harvested nogood that can be promoted, a cut of a node that holds a better set: bugs).  The
lead launches it ([the rules for every launch](../../tools/cloud/RUNBOOK.md#rules-for-every-launch) apply), with user
data like [the ledger's](../ledger/cloud.sh):

```text
#!/bin/bash
shutdown -h +90
export AWS_DEFAULT_REGION=us-east-1
dnf install -y -q git tmux
sudo -u ec2-user bash -lc 'cd ~ && git clone -q https://github.com/bvchess/taxman.git && cd taxman &&
    git checkout -q COMMIT'
sudo -u ec2-user tmux new-session -d -s choice -c /home/ec2-user/taxman/rust \
    'export S3_DEST=s3://taxman-013764204840-us-east-1/node-choice-DATE
     STUDIES=1 BUILD="--bin taxman --example wall --example perf" REQUIRE_CACHED=1 SKIP_TESTS=1 \
         tools/cloud/setup.sh > ~/setup.log 2>&1 && POWEROFF=1 bench/node_choice/cloud.sh;
     sudo shutdown -h now'                          # also if the setup failed
```

The binaries come from the build cache (`REQUIRE_CACHED=1`: the commit must have been released, and with `STUDIES=1`,
because `wall` is a study tool that a default build does not hold; [RUNBOOK section
15](../../tools/cloud/RUNBOOK.md#15-the-build-cache-and-the-release-suite)); a setup that fails ends the machine and the
measurement does not start.

## Cost and time

The work is about 6,000 CPU s, an estimate: the suite's subtrees 3,650 CPU s (the suite's own figure, measured on an M1
Max at about the speed of a `c8g` thread), the made units' subtrees about 1,500, and the fits (`NODE_STEPS` 200, about
2 ms a step at the end of the chain, twice with `SUB`, stopping at a cut) about 800.  Packed on 16 CPUs that is 6 to 12
minutes (the dearest node is 314 s), after the fixtures (the proof of 3213, 440,000 steps, sets the pace: 2 to 4
minutes) and the pool (about a minute).  With the build of `setup.sh` (about 10 minutes; none from the build cache) the machine is up for **about
40 minutes**, and the run costs **about 30 cents at spot** ($0.17 an hour for a `c8g.4xlarge`, a quarter of the
`c8g.16xlarge`'s $0.68, and the volume and the timer's slack; $0.64 an hour on demand).  The timer of 90 minutes
bounds it at about $0.26 at spot for the machine.

## What it does not measure

* **The sample is not a search's population.**  Nodes are spread over gaps (the made and probe sources) or picked for
  size (the suite), so the shares of the total are the sample's; the cut share by gap, and the break-even forecast,
  are what carries over.
* **Each node is a problem of its own.**  The saving counted is the whole subtree when the bound cuts the node; a
  search that applies the policy goes on to apply it at the node's children when it does not cut, which a node's line
  does not show.
* **Costs are charged generously.**  The matroids are built afresh for the fit (a search has them along its path),
  and the ratio to `today_s` is against a bound solved from the root's matroids (a search keeps its levels, which is
  cheaper still).
* **The pool is the root's.**  The nogoods are those the finder and the sound search learn from the standalone start in
  10 CPU s, not the table of the searcher at the node; `sub` is the other end (the table at the end of the node's
  own solve, after the forgetting of the default), so the truth for a searcher's table lies between them.
* **Multipliers only.**  The fit reweights the pool's nogoods; it adds none (the relax-and-cut of [how far nogoods can
  go](../../docs/experiments/how-far-can-nogoods-go-2026-10.md), which adds the conflicts of each heaviest set, is not
  here).
* **One thread, the default tactics, the optimum as the incumbent.**  The suite's 3441 units are solved from the
  checkpoint's incumbent, which a subtree could beat (none did in the suite's making).
