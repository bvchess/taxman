# R3 and the slope test: U4 of A1 (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-16.  Concluded 2026-10-16.

**Question.**  Can a refit of the reference's strength (the exact checks and the core separation), at the nodes high in
the tree, take a fixed *share* off each node's gap, so that the multi-field bound's tree grows more slowly with the
root's gap and not just by a constant factor?  **Answer.**  No.  The ratio of the plain tree to the A1 tree falls with
the gap (β = −2.98 ± 0.97 and −3.68 ± 1.20 × 10⁻³ decades per unit of gap for the two R3 policies, t = −3.1 for both),
and the criterion fixed in advance (kill when s_eff is below 0.3) killed A1 in the search.  A1 stays a root-level tool.

## Method

The [E3 pilot](../the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md) found r2 cutting trees by a factor
of 5.  The advisor's fourth consultation on A1 (step U4) read that as what a refit of R2's strength gives, a roughly
fixed *amount* off each node's gap, and fixed five phases with their criteria in advance:

* Phase 0, build R3 and fix the master's two defects.
* Phase 1, R3 against the reference on saved node states, with a stop rule.
* Phase 2, the slope test on small games whose plain trees are known exactly, with a kill line and a pass line.
* Phase 3, patch mode (carrying the cores one game's search met into the next game's fit) with R3 on small patches.
* Phase 4, R3's cost at 3300-3400 and the projection to a gap of 1,400.

The project's owner asked to record and stop if the stop rule or the kill line fired.

**What was built** (research code, no solver change; 9f750a3, 2537604, 627def0).

* `common/dw.rs`, the pilot's two defects fixed.  A refit's state keeps only the rank inequalities that bind in its
  final master (a positive dual or a slack of at most 10⁻⁶), at most 400, the largest duals first; the pilot rebuilt
  each child's master from up to about 2,000 inherited ones, most of R2's time at 1472.  A master solve that ends
  optimal at a basis whose primal violation exceeds 10⁻⁶, or with a dual above 10⁸, has *broken down*: it is counted and
  solved again from the empty set's basis, and a second breakdown ends the computation with the lowest L found.
* R3 (`refit_r3`).  From the inherited state, rounds of (A) R2's pricing with the cheap separation, at most 30
  pricings; (B) the exact check of each extra matroid that the master's point has mass on, with its violated rank
  inequalities added (at most 5 a matroid); (C) the cores of the master's support columns merged into node-local
  signings (at most 2 new a frame a node and 8 a frame a path); back to (A) after any addition, within 1 s and 300
  pricings.  An option (`cores_first`, the reference's order, the separation before the exact checks) reorders them.
* `fields bnb` with the variant `r3` and the policies `r3-all` and `r3-top:A` (R3 at the Tier-1-open nodes whose GF(2)
  excess is at least A times the root's GF(2) gap, R2 at the others), `--save-states`, `--harvest`, `--max-depth`;
  `fields replay` (refits and the reference run again on saved states); `fields fit` (a root's fit from a carried fit,
  pruned with `--prune K`, with offered cores).

**Validity.**  Nothing new to prove: every bound is ⌊L⌋ at some multipliers and signings, valid by Theorems 14 and 15
whatever the master proposes.  The tests (`cargo t --release --example fields`, 54 pass, one ignored) check R3 against
an optimal set along its path at 300, 350, ..., 1300, against its own subtrees at 20 games of 300-900, and on all 472
non-prime games of 30-600 from the incumbent P\* − 1.  Every tree below completed and asserts that nothing beats P\*.
The fixed master broke down 54 times in the trees' 146,760 refits (0.04%, against the pilot's R2 0.65% at 1472), each
recovered by its restart.

**Phase 1 setup.**  The hosts were r2 with the fixed master (K = 20) at 992 (gap 318; its root fit certified, 60 s, 0.50
of the gap closed) and at 1215, saving every 10th and every 5th Tier-1-open node: 631 and 298 states, from trees of
20,339 and 6,080 nodes (3.81 and 4.03 times fewer than v0's; the pilot's r2 at 1215 took 4,955 nodes in 21 s, now 6,080
in 19 s).  On each state the methods ran from the state the node inherited: R2; R3 at 0.1, 0.3 and 1 s; R3 without
(C) and without (B) at 1 s; the reference (the node mode of E1n, 10 s); and R3 in the reference's order.  The *reach* of
a method is the share of the reference's cuts that it also makes.

**Phase 2 setup.**  The games, by a rule fixed before the runs, came from the 146 games of the [matroid
study](../matroid-intersection-bounds-2026-10/bnb.jsonl) with a root GF(2) gap of at least 120: six spread evenly by gap
in each band of 120-149, 150-199 and 200-249, all five of 250-299 and the three of 300 and above.  That is 27 games of
987-1352 with gaps 126-323 and plain trees of 813 to 87,841 nodes.  1472 was not run: its r3 tree would take about 10
hours.  Root fits were certified, 60 s.  The variants were r2, r3 with `r3-top:0.25` and `r3-top:0.5` (R3 at 1 s in the
reference's order, R2 at the other Tier-1-open nodes, K = 20), and v0.

Three choices changed nothing about validity.  Phase 2's R3 runs in the reference's order, picked by Phase 1's ablation,
and the design's order ran beside it at 11 games as context.  The band 120-149 holds three games with plain trees under
2,000 nodes, which the design expected none to have, so the regression over the games of 2,000 nodes and more is given
too.  R3 runs with a budget in seconds, so its trees depend a little on the machine's load (13.5% of R3's refits at 1 s
ran out of time in Phase 1, 9.9% in the reference's order), and the trees ran two at a time under `nice` beside other
jobs.

## Results

**Phase 1: R3 against the reference on saved states.**  The reference cuts 881 of the 929 states, and 90 of the 123
with GF(2) excess above 100.

| method | reach | above 100 | excess 0-50 | 51-100 | ms a state, mean (median) | out of time |
|---|---|---|---|---|---|---|
| R2 | 0.45 | 0.03 | 0.67 | 0.32 | 6.2 (6.1) | 0 |
| R3, 0.1 s | 0.72 | 0.18 | 0.93 | 0.63 | 42 (10) | 31.8% |
| R3, 0.3 s | 0.81 | 0.32 | 0.97 | 0.77 | 94 (10) | 22.3% |
| R3, 1 s | 0.90 | 0.60 | 0.98 | 0.88 | 211 (10) | 13.5% |
| R3 separation first, 1 s | 0.93 | 0.66 | 0.99 | 0.93 | 154 (10) | 9.9% |
| reference, 10 s | 1 | 1 | 1 | 1 | 610 (50) | 2.5% |

Without (C) R3 at 1 s reaches 0.85 (0.53 above 100), and without (B) 0.78 (0.40).  Of the 4 states above 200, the
reference cut one and no method did.  **The stop rule** (fixed: stop if R3 at 1 s reaches less than 0.6 of the
reference's cuts above 100) read 0.60, 54 of 90 (0.60 at each game), exactly on the line, so Phase 2 ran.  R3's misses
there are its budget: 35 of the 36 ran out of their second, mostly in the exact checks (157 of R3's 211 ms a state),
which the design's order runs in every round before the separation, so that at a dear node R3 spends its second
certifying points the separation would have moved.  In the reference's order R3 reaches 0.66 (59 of 90) for 27% less
time, and Phase 1 picked that order and T₃ = 1 s for Phase 2.  The fixed master broke down 4 times in the 9,290 replays,
none ending a refit.

**Phase 2: the slope test.**  The ratio of v0's tree to the variant's, by geometric means (time is the run's over
v0's):

| gap | games | v0 nodes | r2 ratio, time | r3-top:0.25 ratio, time | r3-top:0.5 ratio, time |
|---|---|---|---|---|---|
| 120-149 | 6 | 1,659 | 8.2, 0.5 | 24.0, 1.8 | 17.6, 1.6 |
| 150-199 | 6 | 15,967 | 3.8, 1.0 | 14.1, 6.5 | 7.6, 4.3 |
| 200-249 | 7 | 15,835 | 3.8, 1.0 | 16.7, 5.7 | 7.3, 3.7 |
| 250-299 | 5 | 34,963 | 3.1, 1.5 | 10.6, 13.3 | 5.0, 6.5 |
| 300-323 | 3 | 60,580 | 3.0, 1.9 | 6.6, 24.6 | 3.7, 6.3 |
| all | 27 | | 4.2, 1.0 | 14.5, 6.2 | 7.7, 3.7 |

The regression of y = log10(N_v0 / N_variant) on the root gap over the 27 games has the slope of v0's own log10 N, b₀,
equal to 8.15 ± 1.18 × 10⁻³ decades per unit of gap (a decade per 123):

| variant | β (10⁻³ decades per unit of gap) | t | s_eff = β / b₀ | the variant's own slope (10⁻³) |
|---|---|---|---|---|
| r2 | −2.28 ± 0.88 | −2.6 | −0.28 | 10.43 |
| r3-top:0.25 | −2.98 ± 0.97 | −3.1 | −0.37 | 11.13 |
| r3-top:0.5 | −3.68 ± 1.20 | −3.1 | −0.45 | 11.83 |

**The criterion** (fixed: kill A1 in the search if every R3 policy has t below 2 or s_eff below 0.3; pass at s_eff ≥
0.45 with t ≥ 2) **killed it.**  The ratio does not grow with the gap: it falls, by a decade per 270-340 of gap for R3
and per 440 for R2, so the A1 trees grow faster with the gap than the plain ones.  Over the 24 games whose v0 tree has
at least 2,000 nodes it is the same: β −3.66 ± 0.88 (t −4.1) and −4.24 ± 1.12 (t −3.8).  R3 in the design's order, run
with `r3-top:0.25` at 11 games, agrees: β −4.08 ± 1.62 (t −2.5), its trees 14.6 times smaller than v0's there, 32.7 at
gaps 120-149 and 6.3 at 300-323.  In time the R3 trees are a loss: 6.2 times v0's over the 27 (1.8 at gaps 120-149, 24.6
at 300-323).

**Why (a reading of the per-node logs).**  A refit's strength falls with the node's GF(2) excess.  R3 at 1 s makes 0.98
of the reference's cuts at excess 0-50 and 0.61 at 101-200, and the reference itself cuts 75% of the states at 101-200
and one of four above 200.  In the trees R3 cut 0.52-0.88 of the nodes it ran at excess 51-100; at 101-200 it cut
0.50-0.63 at the games of gap 201-213 but 0.33-0.39 at those of 282-323; above 200, 0-0.14.  The nodes that stay open
are the high-excess ones near the top, whose subtrees grow with the root's gap.  So the multi-field bound in the search
takes off about a fixed amount of a node's excess, not a fixed share, and as the gap grows more of the tree hangs from
nodes whose excess it cannot close.  The advisor's U1 simulation, which cut each high-excess node with the audit's
probability whatever the gap, could not see this.

**Cost.**  3.6 thread-hours on the laptop, two processes at a time under `nice` (about 2.5 hours of wall time): Phase 1
0.4, the 32 root fits 0.06, Phase 2's 79 trees 2.5, the design's order 0.6, the three v0 trees 0.01.  Nothing ran on
EC2.

## Decision

A1 stays a root-level tool (the ceilings, the prover's certificates, and E1 and E2's share of the root's gap).  E4, the
multi-field bound in the search, is not built, and patch mode has no search to serve (U5: its root-level gains still
serve a root-level A1).  By the trend the ratio would be smaller still at the gaps of 1,300-1,450 of the chain's hardest
games.  A refit that takes a share, not an amount, off the high-excess nodes would reopen the question.

Not run, as the project's owner asked once the kill line fired: Phase 3 (patch mode with R3 on 1197-1212 and 1274-1284)
and Phase 4 (R3's cost at 3300 and 3400 and the projection).  Their instruments are built and tested (`fit --start
--prune --offer`, `bnb --harvest`, `bnb --knuth --max-depth --save-states`, `replay`), and the root fits of the patches'
games are in `solver/frame/testdata/fits`.

## Files

The examples `fields` and `matroid`, with the files of `examples/common/` they included, were removed from master on
2026-10-17 and build at commit 8753189: `git checkout 8753189`, then `cargo build --release --example fields --example
matroid` in `rust/`.  The commands below ran from `rust/`, with `K` for `--known results/strict.jsonl`, with the
binaries built at the commits named.

* `u4/trees.jsonl`: a line per tree, run two at a time under `nice`.  Phase 1's hosts:
  `fields bnb K --variants r2 --fit solver/frame/testdata/fits` with
  `--fit-seconds 60 --fit-mode certified --save-states STATES --every 10 992`, and
  `--fit solver/frame/testdata/fits/n1215.json --every 5 1215`, the code of 9f750a3 (they are Phase 2's r2 at those
  games).  Phase 2, for each of the 27 games N (987 to 1352, the games of the file):
  `fields bnb K --fit solver/frame/testdata/fits --variants r2 N`, and
  `fields bnb K --fit solver/frame/testdata/fits --variants r3` with `--policy r3-top:A --r3-order cores-first N` for
  A = 0.25 and 0.5, the code of 2537604; the design's order as context, `--variants r3 --policy r3-top:0.25` at 987,
  988, 990, 992, 999, 1105, 1192, 1197, 1215, 1221, 1250; and v0 at 990, 1112, 1197, the code of 627def0.
* `u4/replay.jsonl`: Phase 1, a line per saved state and method (9,290).  Made by `fields replay K --methods
  r2,r3:0.1,r3:0.3,r3:1,r3x:1,r3c:1,ref:10 STATES/n992-r2-all-k20.states.jsonl` and the same for `n1215-r2-all-k20`, the
  code of 9f750a3; then `--methods r3s:0.1,r3s:0.3,r3s:1` on both, the code of 2537604.
* `u4/fits.jsonl`, and the root fits in `solver/frame/testdata/fits`: a line per root fit (their `carried_signings`, 3,
  is the plain signings' count: an own fit carries nothing).  Made by `fields fit K --seconds 60 --label own --save-fit
  solver/frame/testdata/fits N...` for the 25 Phase 2 games but 992 and 1215, and the patches' games 1200, 1204, 1208,
  1209, 1212, 1280 and 1284, the code of 2537604 (992's fit was made by its Phase 1 host, 1215's is the pilot's).
* `u4/logs/`: the per-node logs (`--log`) of Phase 2's r3 trees with `r3-top:0.25`, gzipped, showing where R3 cuts in
  the trees by GF(2) excess.
* `u4.py` and `u4_output.md`: the tables, made by `./u4.py > u4_output.md`.

The saved states (`STATES`, 28 MB) are not kept: the hosts' trees are deterministic from their committed root fits (no
refit of theirs meets a time budget), so the host commands make them again, in about 20 s (1215) and 50 s (992); [the
fields study](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md) has the commands of the root fits.

## Commits

* 9f750a3: U4 Phase 0, R3 and the instruments of its tests (`dw.rs`'s breakdown handling).
* c62a908: Phase 1, R3 against the reference on 929 saved states.
* 2537604: R3 in the reference's order as an option.
* 627def0: `fields fit --prune K`.
* f2d9af4: Phase 2, the slope test, killed.
* ddc375f: the merge of U4.
* 8753189: the last commit with the `fields` and `matroid` examples.
