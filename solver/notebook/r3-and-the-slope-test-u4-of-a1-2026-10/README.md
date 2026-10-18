# R3 and the slope test: U4 of A1 (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  After the [E3 pilot](the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10.md), the advisor's fourth
consultation on A1 read r2's factor of 5 as what a refit of R2's strength gives (a roughly fixed *amount* off each
node's gap) and put the question that decides A1 in the search: can a refit of the reference's strength (the exact
checks and the core separation), at the nodes high in the tree, take a fixed *share* off each node's gap, so that the
tree grows more slowly with the root's gap and not just by a constant factor?  And can the cores one game's search met
at its nodes be carried into the next game's fit (patch mode)?  Its step U4 fixed the phases and their criteria in
advance: Phase 0, build R3 and fix the master's two defects; Phase 1, R3 against the reference on saved node states,
with a stop rule; Phase 2, the slope test on small games whose plain trees are known exactly, with kill and pass lines;
Phase 3, patch mode with R3 on small patches; Phase 4, R3's cost at 3300-3400 and the projection to a gap of 1,400.
The owner asked to record and stop if Phase 1's stop rule or Phase 2's kill line fired.

*What was built* (research code, no solver change; 9f750a3, 2537604, 627def0).  `common/dw.rs`: the master's two
defects of the pilot fixed: (a) a refit's state keeps only the rank inequalities that bind in its final master (a
positive dual or a slack of at most 10⁻⁶), at most 400, the largest duals first (the pilot rebuilt each child's master
from up to about 2,000 inherited ones, most of R2's time at 1472); (b) a master solve that ends optimal at a basis whose
primal violation exceeds 10⁻⁶, or with a dual above 10⁸, has *broken down*: it is counted and solved again from the
empty set's basis with the same columns and inequalities, and a second breakdown ends the computation
(`Stop::Breakdown`) with the lowest L found.  R3 (`refit_r3`): from the inherited state as R2 starts, rounds of (A)
R2's pricing with the cheap separation, at most 30 pricings, (B) the exact check of each extra matroid that the
master's point has mass on, its violated rank inequalities added (at most 5 a matroid), (C) the cores of the master's
support columns merged into node-local signings (at most 2 new a frame a node and 8 a frame a path), back to (A)
after any addition, within T₃ seconds (1) and P₃ pricings (300); switches for (B) and (C), and an option for their
order (`cores_first`, (C) before (B) as the reference has them; below).  `bound_offered` (cores placed before a
root's fit), and the times of the exact checks, the separation and the masters' builds.  `fields bnb`: variant `r3`
with the policies `r3-all` and `r3-top:A` (R3 at the Tier-1-open nodes whose GF(2) excess is at least A times the
root's GF(2) gap, R2 at the others), R3's settings in the run's key, `--label`, `--save-states DIR --every K` (the
state of every K-th Tier-1-open node: its path, candidates, `need`, the state it inherited, and what the run's refit
made of it), `--harvest FILE` (every distinct core the nodes' separation finds), `--max-depth` for dives; `fields
replay` (refits and the reference run again on saved states); `fields fit` (a root's fit from a carried fit, pruned
with `--prune K`, with offered cores).  `bench/fields/u4.py`, the tables.

*Validity.*  Nothing new to prove: every bound is ⌊L⌋ at some multipliers and signings, valid by Theorems 14 and 15
whatever the master proposes.  The tests (`cargo t --release --example fields`, 54 pass, one ignored): R3's traces and
states at 300-700, with each switch off and in both orders; R3 along the optimum's path at 300, 350, ..., 1300
(`r3-all` and `r3-top:0.25`), at least `w(S* − promoted)` at every node; with the incumbent `P* − 1`, `r3-top:0.25`
finds a set worth `P*` at each of the 472 non-prime games of 30-600; r3's trees subtrees of v0's at 20 games of
300-900; a breakdown injected into a master solve (a basic value set to −1 after an optimal solve) detected, counted
and recovered to the same optimum, and two running ending a generation at `Stop::Breakdown` with the bound kept; the
states that r2 and r3 trees save at 840 replayed with the host's method giving the host's refit exactly (bound and
final state); 841's fit from 840's, with 840's harvested cores offered, placing some, its bound at least `P*` and the
optimal promotions independent in every matroid.  `bound`'s lines at 45 regression roots (300-1100 every 18th, every
arm) and `matroid bnb`'s (936, 1100, 1215 with `gf2` and `gf2warm`; 600, 700, 936 with `matching`) are unchanged.
Every tree below completed and asserts that nothing beats `P*`, so each proves the plain search's optimum; the three v0
trees run again (990, 1112, 1197) have the matroid study's counts.  The fixed master broke down 54 times in the trees'
146,760 refits (0.04%; the pilot's R2 0.65% at 1472), each recovered by its restart.

*Phase 1: R3 against the reference on saved states.*  Hosts: r2 with the fixed master (K = 20) at 992 (gap 318; its
root fit certified, 60 s, 0.50 of the gap closed) and at 1215 (the pilot's fit), saving every 10th and every 5th
Tier-1-open node: 631 and 298 states.  Their trees: 20,339 and 6,080 nodes, 3.81 and 4.03 times fewer than v0's (the
pilot's r2 at 1215, rebuilding each master from every inherited inequality: 4,955 nodes in 21 s; now 6,080 in 19 s).
On each state, from the state the node inherited: R2; R3 at T₃ = 0.1, 0.3 and 1 s; R3 without (C) (R2 and the exact
checks) and without (B) (R2 and the separation) at 1 s; the reference (E1n's node mode, 10 s); and, after the first
results, R3 in the reference's order (the separation first) at the three budgets.  *Reach*: the share of the
reference's cuts that the method also makes, both games (the reference cuts 881 of the 929 states, and 90 of the 123
of GF(2) excess above 100):

| method | reach | above 100 | excess 0-50 | 51-100 | 101-200 | ms a state, mean (median) | out of time |
|---|---|---|---|---|---|---|---|
| R2 | 0.45 | 0.03 | 0.67 | 0.32 | 0.03 | 6.2 (6.1) | 0 |
| R3, 0.1 s | 0.72 | 0.18 | 0.93 | 0.63 | 0.18 | 42 (10) | 31.8% |
| R3, 0.3 s | 0.81 | 0.32 | 0.97 | 0.77 | 0.33 | 94 (10) | 22.3% |
| R3, 1 s | 0.90 | 0.60 | 0.98 | 0.88 | 0.61 | 211 (10) | 13.5% |
| R3 without (C), 1 s | 0.85 | 0.53 | 0.94 | 0.83 | 0.54 | 187 (10) | 10.0% |
| R3 without (B), 1 s | 0.78 | 0.40 | 0.93 | 0.72 | 0.40 | 37 (9) | 0.1% |
| R3 separation first, 0.1 s | 0.81 | 0.36 | 0.97 | 0.77 | 0.36 | 34 (10) | 22.2% |
| R3 separation first, 0.3 s | 0.88 | 0.51 | 0.98 | 0.86 | 0.52 | 70 (10) | 15.4% |
| R3 separation first, 1 s | 0.93 | 0.66 | 0.99 | 0.93 | 0.66 | 154 (10) | 9.9% |
| reference, 10 s | 1 | 1 | 1 | 1 | 1 | 610 (50) | 2.5% |

(Of the 4 states above 200, the reference cut one and no method did.)  **The stop rule** (fixed: stop if R3 at 1 s
reaches less than 0.6 of the reference's cuts above 100): 0.60, 54 of 90 (0.60 at each game), exactly on the line, so
Phase 2 ran.  R3's misses there are its budget: 35 of the 36 ran out of their second, mostly in the exact checks (157 of
R3's 211 ms a state; the pricing 10, the simplex 10), which the design's order runs in every round before the
separation, so that at a dear node R3 spends its second certifying points the separation would have moved; without
the checks R3 settles early (53 of its 54 misses settled), without the separation it is weaker and as slow.  In the
reference's order R3 reaches 0.66 (59 of 90) for 27% less time; its misses above 100 are again its budget (30 of 31).
Phase 1 picks T₃ = 1 s (the only budget at the line) and, from this ablation, the reference's order for Phase 2 (the
design's order ran beside it as context).  The fixed master's breakdowns, which cost the pilot 0.65% of R2's refits at
1472, are now rare: 4 in the 9,290 replays, none ending a refit.

*Phase 2: the slope test.*  Games, by the design's rule fixed before the runs: from the 146 games of
`bench/matroid/bnb.jsonl`, those with a root GF(2) gap of at least 120, six spread evenly by gap in each band of
120-149, 150-199 and 200-249 (1215 left out of the third, being taken anyway), all five of 250-299 and the three of 300
and above: 27 games of 987-1352, gaps 126-323, plain trees of 813 to 87,841 nodes.  1472 was not run: its r3 tree
would take about 10 hours by the trees below (R3 about 0.55 s a refit at gaps near 300, its plain tree 39 times 1287's),
over the design's 2.  Root fits: the main arm, certified, 60 s (all but 1350's converged; 1215 the pilot's).  Variants:
r2 (the fixed master), r3 with `r3-top:0.25` and `r3-top:0.5` (R3 at 1 s in the reference's order, R2 at the other
Tier-1-open nodes, K = 20); v0 from `bnb.jsonl`.  Each tree's ratio to v0's (geometric means by band of gap; *time* the
run's over v0's; *f₃* the share of the tree's nodes that got R3):

| gap | games | v0 nodes | r2 ratio, time | r3-top:0.25 ratio, time | f₃, R3 ms a refit | r3-top:0.5 ratio, time | f₃, R3 ms |
|---|---|---|---|---|---|---|---|
| 120-149 | 6 | 1,659 | 8.2, 0.5 | 24.0, 1.8 | 0.198, 163 | 17.6, 1.6 | 0.110, 269 |
| 150-199 | 6 | 15,967 | 3.8, 1.0 | 14.1, 6.5 | 0.216, 315 | 7.6, 4.3 | 0.051, 503 |
| 200-249 | 7 | 15,835 | 3.8, 1.0 | 16.7, 5.7 | 0.197, 344 | 7.3, 3.7 | 0.033, 566 |
| 250-299 | 5 | 34,963 | 3.1, 1.5 | 10.6, 13.3 | 0.211, 489 | 5.0, 6.5 | 0.028, 756 |
| 300-323 | 3 | 60,580 | 3.0, 1.9 | 6.6, 24.6 | 0.122, 554 | 3.7, 6.3 | 0.008, 906 |
| all | 27 | | 4.2, 1.0 | 14.5, 6.2 | 0.196, 348 | 7.7, 3.7 | 0.050, 559 |

The regression of `y = log10(N_v0 / N_variant)` on the root gap over the 27 games (`b₀`, the slope of `log10 N_v0`,
is 8.15 ± 1.18 × 10⁻³ decades per unit of gap, a decade per 123):

| variant | β (10⁻³ decades per unit of gap) | t | s_eff = β / b₀ | the variant's own slope (10⁻³) |
|---|---|---|---|---|
| r2 | −2.28 ± 0.88 | −2.6 | −0.28 | 10.43 |
| r3-top:0.25 | −2.98 ± 0.97 | −3.1 | −0.37 | 11.13 |
| r3-top:0.5 | −3.68 ± 1.20 | −3.1 | −0.45 | 11.83 |

**Phase 2's criterion** (fixed: kill A1 in the search if every R3 policy has `t` below 2 or `s_eff` below 0.3; pass at
`s_eff` ≥ 0.45 with `t` ≥ 2): **kill**.  Not only does the ratio not grow with the gap: it falls, by a decade per
270-340 of gap for R3 and per 440 for R2, significantly (t near −3), so that the A1 trees grow faster with the gap than
the plain ones.  Over the 24 games whose v0 tree has at least 2,000 nodes (the design's reason for a gap of at least
120, A1's trees off their floor) the same: β −3.66 ± 0.88 (t −4.1) and −4.24 ± 1.12 (t −3.8).  R3 in the design's
order (the exact checks first), run with `r3-top:0.25` at 11 of the games as context, the same again: β −4.08 ± 1.62
(t −2.5), its trees 14.6 times smaller than v0's there, 32.7 at gaps 120-149 and 6.3 at 300-323.  In time the R3
trees are a loss: 6.2 times v0's over the 27 (1.8 at gaps 120-149, 24.6 at 300-323).

*What the trees say* (a reading).  A refit's strength falls with the node's GF(2) excess: Phase 1's bands show R3 at
1 s making 0.98 of the reference's cuts at excess 0-50 and 0.61 at 101-200, and the reference itself cutting 75% of
the states at 101-200 and one of four above 200.  In the trees (the per-node logs of `r3-top:0.25`, `u4.py`'s table)
R3 cut 0.52-0.88 of the nodes it ran at of excess 51-100 (at the games with 50 such nodes or more); at 101-200 it cut
0.50-0.63 of them at the games of gap 201-213 but 0.33-0.39 at those of 282-323; above 200, 0-0.14.  The nodes that
stay open are the high-excess ones near the top, whose subtrees grow with the root's gap.  So the A1 bound in the
search takes off about a fixed amount of a node's excess, not a fixed share of it, and as the gap grows more of the
tree hangs from nodes whose excess it cannot close; the advisor's U1 simulation, which cut each high-excess node with
the audit's probability whatever the gap, could not see this.  By the trend the ratio would be smaller still at the
gaps of 1,300-1,450 of the chain's hardest games.  For A1: it stays a root-level tool (the ceilings, the prover's
certificates, E1 and E2's share of the root's gap), E4 is not built, and patch mode has no search to serve (U5: its
root-level gains still serve a root-level A1).

*Not run.*  Phase 3 (patch mode with R3 on 1197-1212 and 1274-1284) and Phase 4 (R3's cost at 3300 and 3400 and the
projection), as the owner asked once Phase 2's kill line fired.  Their instruments are built and tested (`fit --start
--prune --offer`, `bnb --harvest`, `bnb --knuth --max-depth --save-states`, `replay`), and the own root fits of the
patches' games are in `bench/fields/fits`.

*Cost.*  3.6 thread-hours of runs on the laptop, two processes at a time under `nice` (about 2.5 hours of wall
time): Phase 1's hosts and replays 0.4, the 32 root fits 0.06, Phase 2's 79 trees 2.5, the design's order 0.6, the
three v0 trees 0.01; nothing on EC2.

*Choices and deviations* (none changes a bound's validity):

1. Phase 2's R3 is in the reference's order (the separation before the exact checks), picked by Phase 1's ablation
   (stronger at the dear nodes, 0.66 against 0.60 of the reference's cuts, and 27% cheaper); the design's order, which
   passed the stop rule exactly on its line, ran beside it at 11 of the games as context (above).
2. The games of each band were spread evenly by gap (the design asked for six a band, not which); the band 120-149 so
   holds three games with plain trees under 2,000 nodes, which the design's parenthesis expected none to have, and the
   regression over the games of 2,000 nodes and more is given beside the criterion's.
3. Phase 1 saved 929 states (the design expected 150-300 a game): r2's tree at 992 has 6,316 Tier-1-open nodes.  The
   states themselves (28 MB) are not kept: the hosts' trees are deterministic from their committed root fits (no refit
   meets a time budget), so the commands of `bench/fields/README.md` make them again in about a minute.
4. The root's state keeps all of its fit's inequalities (the fit files hold no duals); only the refits' states are
   filtered, so the root's children rebuild from up to about 2,000 at 1472 once.
5. R3 runs with a budget in seconds, so its trees depend a little on the machine's load (13.5% of R3's refits at
   1 s ran out of time in Phase 1, 9.9% in the reference's order); the trees were run two at a time under `nice` beside
   other jobs.
6. The exact checks in R3 are those of the root's certification (`check`), each run to its column limit as at the
   root; no cheaper variant was tried.

## Later (2026-10): the program removed from master

The examples `fields` and `matroid`, with the files of `examples/common/` they included, were removed from master on
2026-10-17.  They build at commit 8753189: `git checkout 8753189`, then
`cargo build --release --example fields --example matroid` in `rust/`.
