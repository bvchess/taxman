# The node choice: where the strong bound pays (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The search prunes a node with the GF(2) bound, which is cheap (a step costs 0.14 to 0.34 ms at 2700-3441)
and leaves the gap that sets the cost of a game.  The Lagrangian bound over nogoods with the GF(2) bound inside, its
multipliers refitted at the node, is stronger and slow (tens of GF(2) bounds a fit).  [Multipliers fitted once at the
root](root-fitted-nogood-multipliers-at-the-nodes-2026-10.md) cut only 2-6% of the open nodes at 2700-3440; [multipliers
fitted near the node](how-far-can-nogoods-go-2026-10.md) cut 84% and 62% of those at the probe's nodes, at 12-46 ms a
node.  Over the nodes of a proof, what share does a refit cut as a function of the node's gap, and above what forecast
subtree does computing it pay (its cost against the subtree it saves)?  A search under a floor grows a decade of steps
per 147 points ([floors](proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10.md)), which is what
makes the gap a forecast of the subtree.

*What was run* (2026-10-18 17:54:17Z to 18:02:38Z, commit 9d9dae3):
[bench/node_choice/cloud.sh](../../bench/node_choice/README.md) on one `c8g.4xlarge` (16 CPUs, 16 workers of one thread, the binaries from the build cache), eight minutes: the six
fixtures from proofs (2 minutes), the pool of nogoods and the root fit of the twelve games (30 s), then the 553 nodes
(6 minutes).  The nodes: the tuning suite's units of 3441, 3440, 3300 and 3425 (10 each; gaps 746-1,001, subtrees of
33,000 to 1.25 million steps), 60 each from the proofs of 2700, 2976, 3129, 3213, 3260 and 3267 seeded with their
optima, and 60 each of the probe's 936 and 1215, spread over their gaps.  For each: the gap (today's bound less what the
node needs to beat the incumbent), the subtree (the steps of today's search to solve the node to the end, one thread,
fresh tables, stopped at 900 CPU s: none was), and the refit at the node (up to 200 subgradient steps from the root's
multipliers, stopping at the first that cuts; `sub` the same with the nogoods of the node's own solve too).  520 are
open nodes with a completed solve; the other 33 lines are nodes on the way to the optimum, which only check that no fit
goes below their best completion (none did: the faults file was never written).  The machine was launched at 17:48:31Z
and powered itself off at the end, a few cents; the data is `bench/node_choice/cloud-20261018/`.

*Results* (`analysis.txt`; the refit with all steps, the share of the open nodes it cuts):

| gap | nodes | root's multipliers | refit at the node | subtree, median steps | refit, median seconds |
|---|---|---|---|---|---|
| 0-49 | 121 | 51% | 92% | 2 | 0.002 |
| 50-99 | 92 | 22% | 65% | 6 | 0.008 |
| 100-199 | 126 | 6% | 19% | 22 | 0.13 |
| 200-299 | 72 | 0% | 4% | 58 | 0.28 |
| 300 and more | 109 | 0% | 0% | 185 to 676,000 | 0.3 to 0.5 |

Every node with a subtree of a hundred steps or more (138 of the 520, among them all the suite's) is cut by nothing: the
refit cuts 76% of the nodes whose subtrees are under ten steps and 18% of those of ten to ninety-nine.  The subtree
follows the gap, log10(steps) = 0.32 + 0.00573 gap (a decade per 174 points, r² 0.878; a decade per 147 with the floor
entry's slope).  The refit costs a median 164 to 234 times today's bound at the node at 2700-3441 (13 and 32 times at
936 and 1215).

*The policy* "compute the strong bound where the forecast subtree exceeds N" has a net saving, the CPU seconds of the
subtrees it cuts less those of the bounds it computes.  Searching all 520 nodes costs 4,460 CPU s; computing the refit
at every node cuts 198 of them and loses 85 CPU s (2%), at nodes forecast above ten steps 77 s, above a hundred 45 s,
above ten thousand 24 s, above a million 1 s: **every policy loses, at every N, so never computing it is the best**.
By source the loss at N = 0 is 62 s of 363 (17%) on the six made games, 1.5 s of 0.6 (252%) on the probe's nodes and 21 s
of 4,100 (1%) on the suite.  The result is the same with the floor entry's slope, and with only the first 50 steps of
each fit (cuts of 92%, 63%, 16% and 0% by the same gaps; a loss of 22 s, 1%).

*What it says.*  The refit cuts where the node is within about 200 points of the incumbent, and there the subtree is
a few dozen steps, cheaper to search (0.14 to 0.34 ms a step) than to fit; where the subtrees are large, at gaps of 300
and more and everywhere in the end of the chain's dear searches (gaps 746-1,001), it cuts nothing.  The cost of a game
comes from the nodes with large gaps, so a stronger bound of this kind at the nodes does not change the tenfold per
1,040 in n.  Not measured (the README of the directory lists them): a search's own population of nodes, the refit's
effect on the children of a node it does not cut, pools other than the root's, new nogoods added at the node.

*Commits.*  8e2235e (`wall choice-prepare` and `choice`), 166a4d7 (the script, the analysis and the README), merged as
9d9dae3; the run's files and what they are in [bench/node_choice](../../bench/node_choice/README.md).
