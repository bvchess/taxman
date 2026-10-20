# Root-fitted nogood multipliers at the nodes (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-18.  Concluded 2026-10-18.

**Question.**  A Lagrangian bound over a few seconds' nogoods, with the GF(2) bound inside and multipliers fitted at
the root, closes about 0.38 of the root's GF(2) gap at 3016-3396
([nogoods from the finder's work](../nogoods-from-the-finders-work-2026-10/README.md)).  A search could fit once and
use the multipliers unchanged at every node, at about the cost of a second GF(2) bound.  What share of the nodes that
the GF(2) bound leaves open would they cut?  [How far nogoods can go](../how-far-can-nogoods-go-2026-10/README.md)
found the matching form's root multipliers weak at the nodes; this asks the same of the GF(2) form.  **Answer.**  No.
Multipliers that close 0.22-0.52 of the root gap cut 2% of the open nodes at 2700-3440 with the plain constant and 6%
with the tight one (7% and 10% at 936 and 1215), and none of the units at the end of the chain.  Built into the
search, they give about 6% fewer steps at twice the CPU a step.

## Method

**The bound.**  Take multipliers `λ_N ≥ 0` over nogoods `N` of the root, and a node with promotions `P`.  Every
completion `F` has `|F ∩ N| ≤ |N| − 1 − |P ∩ N|`.  So the heaviest common independent set of at most `max_additional`
of the node's candidates in its GF(2) matroids, each candidate `x` weighing `x − Σ_{N ∋ x} λ_N` (those of no positive
weight left out), plus `Σ λ_N (|N| − 1 − |P ∩ N|)`, bounds every completion (weak duality, Theorem 11, any
multipliers).  Since also `|F ∩ N| ≤ |N ∩ C|` for the node's candidates `C`, the *tight* constant
`Σ λ_N (|N − P| − max(1, |N − P − C|))` holds too.  The weights stay the root's either way, so along a path only
the matroids change.  The *root-λ bound* below is this bound with multipliers fitted at the root.

**The run.**  `wall relaxed` (`examples/wall/relaxed.rs`) on the laptop, about 12 CPU minutes in all.  For each game
with a known optimum: the pool, the nogoods that the finder and the sound search learn from the standalone start in 10
CPU s each (263-6,008 distinct); the fit, 200 subgradient steps at the root towards the optimum plus one (36-630
nogoods left with a positive multiplier); then at each node the GF(2) bound as the search computes it and the root-λ
bound with each constant.  The nodes are 300 units each of the proofs of 936, 1215 and six games of 2700-3267 (seeded
with the optimum), the tuning suite's 10 units each of 3300, 3425 and 3440 with their subtrees, and, to check validity,
50 nodes a game on the way to the optimum.  A node is *open* when the GF(2) bound reaches what a completion needs to
beat the optimum, and the root-λ bound *cuts* it when it is below that need.

**The pilot.**  Before this measurement the bound was built into the search as an option on a second GF(2) level stack,
with the tight constant, tried where the GF(2) bound left a node open: cutting only, or cutting and adding the members
it shows forced.  It ran on the proofs of 936, 1215 and 2700 from the optimum, with the finder's pool of 2-5 CPU s
and 200 fit steps.

## Results

At the open nodes the root's multipliers cut little (fractions of the open nodes of the fixtures and the suite):

| n | gap | root share | open nodes | cut | cut, tight constant |
|---|---|---|---|---|---|
| 936 | 132 | 0.27 | 137 | 0.04 | 0.07 |
| 1215 | 213 | 0.23 | 133 | 0.09 | 0.12 |
| 2700 | 731 | 0.44 | 300 | 0.01 | 0.03 |
| 2976 | 776 | 0.45 | 300 | 0.02 | 0.07 |
| 3129 | 924 | 0.36 | 300 | 0.00 | 0.01 |
| 3213 | 934 | 0.40 | 300 | 0.01 | 0.06 |
| 3260 | 943 | 0.46 | 300 | 0.01 | 0.06 |
| 3267 | 795 | 0.52 | 300 | 0.05 | 0.13 |
| 3300 | 1,443 | 0.33 | 10 | 0 | 0 |
| 3425 | 1,407 | 0.35 | 10 | 0 | 0 |
| 3440 | 1,555 | 0.32 | 10 | 0 | 0 |

*Root share* is the share of the root's GF(2) gap that the fit closes.  The *drop* is the GF(2) bound less the root-λ
bound, and the *excess* is how far the GF(2) bound lies above the node's need, the drop that would cut it.

* **Pooled over 2700-3440.**  The root-λ bound cuts 2% of the 1,830 open nodes with the plain constant and 6% with the
  tight one.  The plain constant gives a bound above the GF(2) bound at most nodes (median drop −56).  The tight one
  gives a lower bound at 60% of them, with a median drop of 18 against a median excess of 160.
* **The suite.**  Its 30 units, whose subtrees hold 10.8 million of the default's nodes, are cut by neither constant:
  the drop is 85-285 and the excess 860-880.
* **By depth and by gap** (tight constant, 2700-3440).  The gain is at the top of the tree and gone below: the median
  drop is 235 at depth 0-9 (excess 794) and −2 at depth 60 and deeper (112), where 1,092 of the 1,830 nodes are.  The
  share cut is 0-8% at every depth, because the nogoods lose their pull as the path promotes or drops their members.
  Nor does the saving grow with the gap: the games of gap 924-943 are cut least (1-6%), beside 3267 (gap 795, the best
  root share, 13%), and the suite's games of gap 1,400-1,555 not at all.
* **Validity.**  At all 550 nodes on the way to the optimum, both root-λ bounds and the GF(2) bound were at least the
  best completion's known lower end, and at every root the root-λ bound was above the optimum.  At each of the 2,980
  nodes, matroids built afresh from the node's state gave the GF(2) bound.
* **Cost.**  One root-λ evaluation from scratch costs a median 1.04-1.14 times a plain GF(2) bound from scratch, and
  1.8-3.1 times the search's bound at a unit: in a search it is a second GF(2) bound along the path.

The pilot agrees with the nodes (steps against the GF(2) bound alone; CPU as a multiple of its CPU):

| proof | root share | steps, GF(2) bound alone | cut only | cut and forced | CPU, cut only | CPU, cut and forced |
|---|---|---|---|---|---|---|
| 936 | 0.27 | 390 | 394 (+1%) | 413 (+6%) | 1.4 | 2.0 |
| 1215 | 0.22 | 3,704 | 3,549 (−4%) | 3,199 (−14%) | 1.5 | 1.9 |
| 2700 | 0.42 | 112,464 | 106,190 (−6%) | 115,992 (+3%) | 2.0 | 2.7 |

At 2700 the cut-only search cut 1,261 of 73,751 checks.

## Decision

Not kept, as a bound at the nodes or as a search option; the pilot's code was dropped, because the project's owner asked
for the node measurement instead.  A Lagrangian over the nogoods would need multipliers fitted near each node, which
[how far nogoods can go](../how-far-can-nogoods-go-2026-10/README.md) found too dear in every cheap form.

For a later attempt: the GF(2) levels (`src/gf2.rs`) read an element's weight as its number in four places (a child's
inherited value and a certified child's, `Losses::from_history` and `Losses::reach`), which a bound with other weights
must change.  `Gf2Graph::reweighted` also drops the elements of no positive weight, which a path solver must keep to
contract a promotion of one.

## Files

The runs went one process a game, four at once under `nice -n 10`, with the test-profile build (`cargo build --profile
test --example wall --example perf` in `rust/`), from `rust/`, against `results/strict.jsonl`, which gives each game its
optimum.  The times are CPU seconds on a shared laptop.

| file | command | what it is |
|---|---|---|
| `nodes.jsonl` | `wall relaxed --cpu 10 --steps 200 --paths 50 --fixtures DIR --suite solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl --known results/strict.jsonl N`, one process a game, for N = 936, 1215, 2700, 2976, 3129, 3213, 3260, 3267, 3300, 3425 and 3440, the lines joined in order of N | a line a game: the pool of nogoods, the fit at the root, and every node measured: the GF(2) bound and the root-λ bound with both constants, and their times |
| `tables.py`, `tables.txt` | `./tables.py > tables.txt`, in this folder | the tables the entry quotes |

`DIR` held the nodes fixtures: the probe's `testdata/perf/n0936.nodes` and `n1215.nodes`, and for 2700, 2976, 3129,
3213, 3260 and 3267 fixtures not kept (each about 240 kB), made by `perf make-nodes N --known results/strict.jsonl
--every 2000 --keep 300 --fixtures DIR`.  That command runs the proof of N, seeded with the optimum on one thread, in
2,000-step pieces that each resume the last one's checkpoint, and keeps 300 of the 854 to 3,911 distinct units it
handed its searcher, evenly spaced.  3300, 3425 and 3440 have no fixture: their nodes are the tuning suite's units.

The command `relaxed` was removed from master on 2026-10-18; it builds at commit 9d9dae3 (`git checkout 9d9dae3`), and
the module comment of `examples/wall/relaxed.rs` defines every field of a line.  The sampling of its nodes stays in
`examples/wall/sample.rs`, which `wall choice-prepare` uses.

## Commits

* `a76c408` (2026-10-18): the command `wall relaxed` and this entry's data.  `wall nogoods`'s fit now returns its
  multipliers, and its harvest, fit and pool union are shared with `relaxed`.
* `9d9dae3` (2026-10-18): builds the command, which master no longer has.
