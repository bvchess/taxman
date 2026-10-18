# What drives the slope (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The cost of a game grows about ×10 per 55-70 in n near the frontier
([solver/notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md](../benchmarks-the-probe-pgo-and-scaling/cost-curve.md#results)), and every improvement so far (same search, the Lagrangian bound,
faster nodes) moved the curve down without changing its slope.  What drives the slope: the room for doubt at the root
(the gap between the root's upper bound and the optimum, which the tree must rule out), the size of the problem (n,
candidates, `max_total`), or something else?  And so what would change it?

*Method.*  No search code changed.  `examples/slope.rs` (commit db7faf1) measures the root of a game and runs proofs
with the incumbent set by value, through the library's public interfaces (`Board`, `MatchBound`,
`matchbound::lagrange`, `engine::run`); `solver/notebook/what-drives-the-slope-2026-10/slope.py` makes the tables below (and more) from its two data
files, `solver/notebook/what-drives-the-slope-2026-10/slope_roots.jsonl` and `solver/notebook/what-drives-the-slope-2026-10/slope_proofs.jsonl`, and `results/strict.jsonl` as of 2487671
(the 1..1300 chain of the older build, which the script reads from git).  The solver of 331eb09 with
`--tactic revise_freedoms=off` (the strict tactics; the tactic has been removed since, and the commands below, which no
longer pass it, run the same searches), one thread a process and at most two processes, on the laptop while another
agent ran the test suite.  `P*` is a game's optimum promotion sum (its score minus `in_the_bag`).

* *Roots* (`slope roots --known results/strict.jsonl --sums solver/notebook/benchmarks-the-probe-pgo-and-scaling/bands.jsonl 880-1300` and the games past 1300
  that `bands.jsonl` solved; about a second a game): the root's matching bound; the matching bound refined one to three
  levels deep (the search's refinement, computed to a value); and a Lagrangian bound over nogoods of the root, made
  without a search: the conflicts the refinement found, then up to 8,000 subgradient steps towards `P* + 1` (Polyak's
  rule damped by halving after 80 steps without a lower bound), each adding up to 12 new conflicts of the weighted
  matching's candidates; it settles on 140-840 nogoods (band means).  The *matching*, *refined* and *Lagrangian gaps*
  are these bounds minus `P*`.  Also the chain's start incumbent and cap, and the candidates each bound leaves undecided
  with the incumbent at the optimum.
* *Proofs* (`slope prove --known results/strict.jsonl --deltas ... N`, with `--sums` as above past 1300): the search of
  the game on one thread, its incumbent a value `P* + δ` with no set behind it, no seed, no cap.  With δ = 0 it is the
  proof of the optimum (the `--seed-opt` ladder's node counts: 29,614 for 1100, 19,473,016 for 1290 and 28,024,916 for
  1300, as in ["A Lagrangian bound over the nogoods"](../a-lagrangian-bound-over-the-nogoods-2026-10/README.md)), for all 143 games
  of 880-1300 that `strict.jsonl` searched to exhaustion (the others are repeats, caps and primes) and for 1309, 1350
  and 1352.  With δ > 0 nothing beats the incumbent, and every cut is the cut the search would make with every bound it
  uses (matching, refined, Lagrangian, forced members) lowered by δ: a *margin scan* over δ = 25 to 800 measures how the
  tree shrinks as all the bounds tighten uniformly, for 19 games of 900-1300, and at δ = 100-300 for 1302, 1305, 1309,
  1350 and 1352.  With δ < 0 the search starts below the optimum.  (`--diag` runs, whose lines carry `steps_by_depth`,
  have the same trees and slower times.)
* The recorded node counts of `strict.jsonl` are of four threads from the chain's start incumbent: 1.35 times the
  proof's (geometric mean) for the 87 games that start at the optimum, 1.63 times for the 56 that start below it (63
  below on average).  Every fit gives the same answer on either count; the tables use the proofs.

**1. The node count follows the gap, not the size.**  Least squares of log10 of the proof's nodes over the 143
searched games:

| variable | slope per unit | R² | residual sd (decades) |
|---|---|---|---|
| n | 0.0045 ± 0.0007 | 0.23 | 0.97 |
| candidates | 0.017 ± 0.002 | 0.26 | 0.96 |
| `max_total` | 0.067 ± 0.009 | 0.27 | 0.95 |
| matching gap | 0.00374 ± 0.00012 | 0.87 | 0.41 |
| refined gap (3 levels) | 0.00427 ± 0.00014 | 0.86 | 0.41 |
| Lagrangian gap | 0.00758 ± 0.00021 | 0.90 | 0.35 |
| cap gap (cap − `P*`) | 0.0000 ± 0.0005 | 0.00 | 1.11 |
| greedy gap (`P*` − greedy) | 0.0020 ± 0.0002 | 0.42 | 0.85 |
| start gap (`P*` − start) | 0.0045 ± 0.0018 | 0.04 | 1.09 |

With the Lagrangian gap in the fit, n adds nothing (+0.0003 ± 0.0003 per n, t = 1.1), nor do the candidates (t =
1.1) or `max_total` (t = 1.0); the residuals of the gap's fit are uncorrelated with n (0.08) and with the
candidates (0.08), while those of the fit on n correlate 0.82 with the gap.  With the matching gap instead, n comes in
negative (−0.0015 ± 0.0004).  Matched pairs agree: 342 pairs of searched games within 15 of each other in Lagrangian
gap and at least 100 apart in n differ by −0.07 decades per 100 in n (sd 0.29); 349 pairs within 30 in n and at
least 100 apart in the gap differ by +0.64 decades per 100 of gap.  The natural experiment is the dip of 1050-1190:
more candidates than 950-1050, gaps half as large, a hundred times fewer nodes (the bands in 3).

**2. A margin costs what a gap costs.**  The margin scans (log10 of the proof's nodes; the slope is fitted over
δ = 0-200, and "δ at 100 nodes" is interpolated):

| n | matching gap | Lagrangian gap | δ = 0 | 100 | 200 | 400 | 600 | decades per unit of δ | δ at 100 nodes |
|---|---|---|---|---|---|---|---|---|---|
| 900 | 460 | 99 | 3.62 | 3.03 | 2.12 | 0.30 | 0.00 | 0.0076 | 216 |
| 936 | 618 | 201 | 4.62 | 3.46 | 2.78 | 1.32 | 0.00 | 0.0089 | 283 |
| 950 | 756 | 262 | 5.66 | 4.40 | 3.36 | 2.09 | 1.08 | 0.0117 | 418 |
| 992 | 951 | 477 | 6.32 | 5.34 | 4.26 | 2.64 | 1.75 | 0.0105 | 543 |
| 1000 | 911 | 398 | 5.56 | 4.36 | 3.38 | 2.16 | 1.56 | 0.0111 | 454 |
| 1025 | 852 | 434 | 6.61 | 5.53 | 4.45 | 2.96 | 1.82 | 0.0107 | 568 |
| 1050 | 629 | 258 | 4.67 | 3.56 | 3.19 | 1.85 | 0.30 | 0.0073 | 377 |
| 1088 | 421 | 103 | 3.79 | 2.80 | 1.83 | 0.30 | 0.00 | 0.0096 | 181 |
| 1100 | 508 | 181 | 4.47 | 3.38 | 2.57 | 1.30 | 0.00 | 0.0089 | 305 |
| 1150 | 542 | 121 | 3.69 | 2.87 | 2.17 | 1.00 | 0.00 | 0.0076 | 247 |
| 1184 | 666 | 153 | 4.72 | 3.49 | 3.21 | 1.70 | 0.30 | 0.0080 | 358 |
| 1196 | 893 | 274 | 5.33 | 4.72 | 4.05 | 2.63 | 1.41 | 0.0063 | 504 |
| 1215 | 966 | 366 | 6.05 | 5.10 | 4.39 | 3.21 | 1.90 | 0.0085 | 584 |
| 1225 | 1200 | 529 | 6.34 | 5.44 | 4.88 | 3.96 | 3.20 | 0.0074 | over 800 |
| 1240 | 1208 | 385 | 5.70 | 5.23 | 4.70 | 3.49 | 2.76 | 0.0053 | 744 |
| 1250 | 1087 | 396 | 5.85 | 5.06 | 4.43 | 3.30 | 2.27 | 0.0068 | 664 |
| 1254 | 1251 | 474 | 6.68 | 5.57 | 4.99 | 3.56 | 2.73 | 0.0087 | 771 |
| 1288 | 1216 | 456 | 6.68 | 5.69 | 5.00 | 3.69 | 2.64 | 0.0082 | 731 |
| 1300 | 1421 | 529 | (7.45) | 6.44 | 5.39 | 3.83 | 2.82 | 0.0103 (from δ = 50) | 794 |

Every tree shrinks by 0.005-0.012 decades per unit of margin (×10 per 85-190), with no trend in n, and at about the
rate the searched games show per unit of Lagrangian gap (0.0076).  Over the 112 scan points with δ ≤ 200 and at least
32 nodes, log10 nodes = a + c (Lagrangian gap − δ) fits with c = 0.0074 ± 0.0002, R² 0.91, residual sd 0.36, as
well as the 143 proofs fit their gaps; adding n gives +0.0009 ± 0.0003 per n (with the matching gap, −0.0023 ±
0.0005), a tenth of a decade per 100 in n at most.  A large game tightened costs what a small game with the same
remaining gap costs: at a remaining Lagrangian gap of about 100, 900 untouched 3.62, 1088 3.79, 1184 at δ = 50 4.13,
1250 at δ = 300 3.68; at about 200, 936 4.62, 1050 at δ = 50 4.02, 1250 at δ = 200 4.43.

**3. The slope is the gap's growth.**  By bands (means of the searched games; past 1300 the games of `bands.jsonl`):

| n | games | candidates | matching gap | Lagrangian gap | log10 proof nodes |
|---|---|---|---|---|---|
| 851-900 | 6 | 216 | 440 | 115 | 3.78 |
| 901-950 | 16 | 224 | 638 | 201 | 4.56 |
| 951-1000 | 16 | 238 | 772 | 279 | 5.21 |
| 1001-1050 | 19 | 253 | 788 | 363 | 5.62 |
| 1051-1100 | 18 | 267 | 537 | 197 | 4.46 |
| 1101-1150 | 17 | 277 | 524 | 167 | 3.82 |
| 1151-1200 | 16 | 290 | 664 | 171 | 4.52 |
| 1201-1250 | 19 | 305 | 1100 | 439 | 6.13 |
| 1251-1300 | 16 | 322 | 1282 | 475 | 6.90 |
| 1301-1310 | 7 | 332 | 1445 | 554 | 7.14 (1309 only) |
| 1350-1352 | 3 | 343 | 1255 | 465 | 6.88 (1350, 1352) |

Over the 361 non-prime games of 880-1300 the matching gap grows by 142 ± 9 per 100 in n and the Lagrangian gap by 56
± 5, jaggedly (R² 0.39 and 0.24): the Lagrangian gap halves from 1001-1050 to 1101-1150 and nearly triples to
1251-1300.  The cost's slope in n is the product of the cost per unit of gap and the gap's growth: 0.0076 × 0.56 =
0.0042 decades per n over 880-1300, against 0.0045 fitted on n; and from 1151-1200 to 1251-1300 the Lagrangian gap
rises by 304, which predicts 2.31 decades against 2.38 measured.  The ×10 per 55-70 of the cost-curve study
(standalone times of bands from 1200 to 1350) is this rise, and at 1350 the cost of finding the optimum (94% of its
standalone search came before the final incumbent).  Past 1300, out of sample, the gaps are those of 1251-1300 or a
little more, and so are the proofs: the fits of section 1 predict 1309, 1350 and 1352 (7.14, 7.24, 6.52) within 0.26
decades by the matching gap (7.40, 7.03, 6.64), and 0.16-0.55 too low by the Lagrangian gap (6.98, 6.69, 6.09); the
margin scans of 1302, 1305 and 1309 fall within 0.25 of the scan fit, those of 1350 and 1352 0.5-0.9 above it, so
the Lagrangian gap misses something at 1350 that the matching gap does not.

**4. The other candidates.**

* *The start incumbent.*  It explains nothing of the variation (R² 0.04 above).  Starting the search below the
  optimum (δ < 0) costs 0.9-1.8 times the proof's nodes at 10 to 100 below and 1.4-3.0 times at 200 below (936,
  1100, 1184, 1215, 1250, 1254); the chain's start is the optimum in 87 of 143 games and 63 below in the others.
* *Depth.*  The trees are deep and their depth is the size: the deepest node has 42 (900) to 66 (1300) promotions,
  5-13 below `max_total`, and cheap games are as deep as dear ones (1150: 4,858 nodes to depth 55; 1215: 1.1M to
  64).  What differs is the width: the steps per level grow by 0.045-0.064 decades a level up to the busiest level,
  near the bottom.
* *The cap* (cap − `P*`, R² 0.00) and the greedy set (0.42, through its correlation with the gap) do not drive it.

**5. Two pilots on the Lagrangian bound** (one thread, code not kept).

* *The root's converged relaxation as the search's starting one.*  Each searcher began with the nogoods and damped
  multipliers of the root's fit above (43-81 nogoods with positive multipliers), instead of none until its first
  refit; its refits then went on as usual.  At the root that bound is 310-690 below the refined matching bound (the
  search's check there until its first refit), which by the margin scans' rate would be worth two decades and more if
  the search's bound were that much lower at every node:

  | proof | nodes | nodes, seeded | s | s, seeded |
  |---|---|---|---|---|
  | 936 | 41,653 | 24,492 (−41%) | 0.30 | 0.19 |
  | 1100 | 29,614 | 22,057 (−26%) | 0.24 | 0.19 |
  | 1184 | 52,259 | 12,681 (−76%) | 0.38 | 0.11 |
  | 1215 | 1,111,451 | 889,919 (−20%) | 9.5 | 7.7 |
  | 1250 | 709,893 | 481,946 (−32%) | 6.0 | 4.4 |
  | 1254 | 4,830,651 | 4,570,450 (−5%) | 43.9 | 39.8 |

  A constant factor, and the smallest at the frontier: the nodes are 30-45 promotions deep, where the root's
  multipliers fit little.  The gap that counts is the relaxation's at every node; the root's measures it only because
  the gaps at the nodes scale with it.
* *Damped steps in the refit* (`matchbound::lagrange::fit`, halving after 3 or 5 steps without progress): on the
  root's nogoods, 50 damped steps reach the bound of 50 undamped ones within 8, both 50-540 above the converged
  bound over the same nogoods (band means, the most past 1250; section 9 of `slope.py`'s output).  The
  deficit of a refit from zero multipliers is its step count, not its rule; the search's refits start from the last
  multipliers, and how far its multipliers are from converged was not measured.

*Conclusion.*  log10 of a proof's nodes is about a + c·G, where G is the root's gap and c is the same within a game
(the margin scans) as across games, with no trend in n from 880 to 1352: ×10 per about 130 of Lagrangian gap, or 270
of matching gap.  At a given gap the size of the problem adds nothing measurable.  So the slope in n is c × dG/dn:
the growth of the room for doubt, which is jagged (it halved over 1050-1150 and nearly tripled over 1190-1300) and
which past 1300, on the eleven games sampled, has not gone on rising.

*What it implies.*

* *A bound whose gap grows more slowly* changes the slope, and is the lever: closing a fixed amount of the gap
  everywhere is a constant factor (×10 per 130 of Lagrangian gap); closing a share of it lowers the slope by that
  share, and a gap that grows half as fast halves it.  What grows is the relaxation's error, not the problem.  The
  Lagrangian bound over the root's nogoods (an LP with their cover inequalities) leaves 26-46% of the matching gap and
  its own gap grows too, so a better bound must see what nogood inequalities cannot; and by the first pilot it must be
  tighter at every node, not only at the root.
* *Reuse of n − 1's proof*: no evidence for it.  No part of the cost scales with the size of the problem at a given
  gap, so there is no size-driven work for a proof of n − 1 to save; games with the same problem are repeats already
  (`same_search`), and the costs of the others follow their own gaps, which move by hundreds between neighbours.  It
  could save a constant at best.
* *Dominance among interchangeable alternatives* would act on c, the nodes per unit of gap, not on the gap; since
  the slope is c × dG/dn, lowering c by a share would lower the slope by the same share.  But nothing here shows that
  interchangeable alternatives make c (no symmetries were found, IDEAS.md section 5), and c does not grow with n.
  Untested.
* *Something else*: the gap is a cheap predictor of a proof's cost (a second at the root; 0.35 decades residual sd,
  0.41 with the matching gap, which needs only milliseconds), for budgeting games before their search and for
  forecasting the curve where `P*` is known or closely bounded.  Finding the optimum costs a factor of 1 to 3 (section
  4), not the slope.

*Next.*  Screen candidate bounds by their gaps before building them into the search, since a gap predicts a proof's
nodes: (a) decompose the root's gap by frame (the weighted matching's promotions against the optimum's, frame by
frame) over 880-1352, to see where the relaxation's error lives and which frames carry its growth; (b) compute a
candidate relaxation's gap at the root and at the probe's 300 search nodes of 936 and 1215 (`testdata/perf`) next to
the current bounds', and predict the proof's nodes by 10^(−c·Δgap) with c from the margin scans; build only a bound
whose gap at the nodes is smaller by a share that grows with n.  (Evidence section 8 names one: a relaxation that
sees that a move sweeps every divisor.)

## Later (2026-10): the program removed from master

The example `slope`, with the files of `examples/common/` it included, was removed from master on 2026-10-17.  It builds
at commit 8753189: `git checkout 8753189`, then `cargo build --release --example slope` in `rust/`.
