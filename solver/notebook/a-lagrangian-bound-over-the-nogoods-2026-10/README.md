# A Lagrangian bound over the nogoods (2026-10)

Part of the [lab notebook](../README.md).

The question (solver/docs/IDEAS.md, rank 2): the pilot ([IDEAS.md 4.2](../../docs/IDEAS.md#42-a-lagrangian-bound-over-the-nogoods-rank-2))
folded the nogoods into the matching bound by Lagrangian multipliers fitted once at the root, and cut the nodes of
proofs by 13 to 67%, but it solved the weighted matching from scratch, 7 times slower per node.  Can the bound be kept
incrementally, cheaply enough to pay in cycles?  Yes, and with the forced members its potentials give it pays far more
than the pilot's node counts suggested: proofs from 1184 to 1300 take 2 to 6 times fewer cycles, about 3 times, with no
trend in n.  Kept as the tactic `lagrange`, on by default (commit e6b37d2; DESIGN.md, section 4, "The Lagrangian bound
over the nogoods", has the design, and `src/matchbound/lagrange.rs` the argument).

**How it was measured.**  One-thread proof-only searches (`ladder --threads 1 --seed-opt --tactic
revise_freedoms=off`, the ladder's `--node-limit` for prefixes), whose node counts are exact, and the process's
cycles (`/usr/bin/time -l`), against master (c6cf8ab) built the same way.  With `--tactic lagrange=off` the new
build gives master's trees node for node (the tripwire rows `lagrange=off`) and the probe's fingerprint
6197a47eae6cf8c8 in the same cycles (4.21 G against 4.20 G, three runs each, instructions +0.06%).  The laptop was
shared with other agents the whole time (load average 3 to 12), so cycles and counts only; four processes at most.

**The solver.**  `matchbound`'s levels and Hungarian repairs became generic over how a level weighs its candidates
(the matching bound's code and trees unchanged), and `matchbound::lagrange` keeps a second stack of levels with
weights of their own: a level is made for a node when the bound is asked for there, by copying the nearest level of
its path and applying what changed (promotions; candidates gone, whose live nogoods die and raise the weights of their
other members).  A weight that rises needs a repair only where a potential must rise past a tight edge (0.4 per
evaluation at 1215).  When the multipliers change, the levels are dropped and solved afresh, so that no level mixes
two sets of multipliers.  Weights are in 64ths of a number and the multipliers integers, so the bound is exact
integer arithmetic and nothing rounds.  Checked against the reference min-cost flow (with the live nogoods, weights
and constant worked out afresh) on random walks with random relaxations that change mid-walk, against a solver
started afresh, by the optimality certificate of every level, and against exhaustive completions with real nogoods.

**Step 1: the pilot's setting, incrementally.**  The nogoods of a warm-up of the same game (the pilot's: the whole
proof for 936 and 1100, its first 20k, 300k, 500k and 300k nodes for 1184, 1215, 1225, 1250; the warm-up not
counted), 200 subgradient steps at the root, the bound tried at every check of a node or child that the matching
bound and refinement left open:

| proof | nodes, master | nodes | G cycles, master | G cycles | cycles |
|---|---|---|---|---|---|
| 936 | 59,055 | 51,207 | 1.21 | 1.47 | +22% |
| 1100 | 55,543 | 41,006 | 1.31 | 1.49 | +14% |
| 1184 | 167,960 | 52,065 | 3.24 | 1.67 | −49% |
| 1215 | 5,485,294 | 3,851,527 | 108.5 | 106.1 | −2% |
| 1225 | 12,071,677 | 7,124,951 | 322.4 | 272.9 | −15% |
| 1250 | 2,792,227 | 2,021,225 | 54.6 | 53.6 | −2% |

The node counts are the pilot's (1184: 52,065 against its 54,795), at 30-55% more cycles per step.  What a bound
costs: run without cutting (computed and thrown away, on master's tree; 1215, first million nodes), 604k bounds took
18.7 → 24.8 G cycles, 10k cycles each, +30% per step; tried only at the checks of nodes, not of children (where 78%
of its cuts were), 207k bounds took 5.8 G against 6.4 G in the same run (each derived from further up the path, and
the work for a parent's level repeated for each child); keeping the multipliers of dead nogoods, which needs no weight
to rise, saved a tenth of it (24.2 G).  A micro-benchmark of the root's expansion loop (`bench_lagrange_1215`, ignored
test) puts a Lagrangian level at the price of a matching level (3.6-3.7 µs a step against 3.3).  So the second matching
costs about what the node's own matching checks cost, twice over (no strong-branching level to reuse, and its own
optimum, which the search does not branch along): this setting pays only where nodes fall by more than a third.

**Step 2: multipliers fitted in the search.**  Each searcher fits its own at the root, to the nogoods it learned (its
table's and those of its multipliers so far), first after 10,000 steps, then before each forgetting, 50 subgradient
steps: 936 +15%, 1100 −1%, 1184 −41%, 1215 −13%, 1225 −25%, 1250 −13% (nodes −11% to −59%), with no warm-up to
pay.  With the first refit after 2,000 steps, on ten proofs from 1196 to 1285, −4% to −32% (1220 +14%), −10% in
all.  The fitting is 3% of the time (13.6 ms a refit at 1215), the levels solved afresh after it about 1%.  Variants,
with this setting (cycles, 1215 and 1250, against 84.1 and 45.2 G): the bound tried only at nodes of at most 8, 16,
24 or 32 promotions 108.2, 92.9, 81.7, 85.1 G and 54.1, 56.2, 57.3, 53.1 G (the deep cuts matter); the multipliers
fitted at the node in the middle of the path at refit time 96.5 and 45.9 G, at the deepest 135.6 and 55.8 G (the
root's are best).  With the first refit after 10,000 steps, 20 or 100 subgradient steps gave 119.8 and 112.2 G, and
45.7 and 46.5 G (against 94.5 and 47.5 G with 50), and refits every 10,000 steps 96.1 and 53.6 G: as noisy as the
trees, since a different fit makes a different tree.

**Step 3: forced members.**  Where the bound leaves a node open, the members of its heaviest matching without which
it would cut, read off its potentials as `bound_excluding` reads the matching's, are forced: every completion that
beats the incumbent promotes them (the live nogoods' inequalities hold for every completion, so the multipliers and
the constant stay without a candidate).  With `fix_forced` the child promotes them with the matching's forced
members, and strong branching is skipped.  This is where the gain is (one thread, proof-only; master against the
tactic as committed, schedule of step 2):

| proof | nodes, master | nodes | G cycles, master | G cycles | cycles |
|---|---|---|---|---|---|
| 880 | 31,416 | 25,398 | 0.62 | 0.65 | +6% |
| 936 | 59,055 | 41,653 | 1.21 | 1.20 | −1% |
| 1050 | 115,154 | 46,527 | 3.45 | 2.11 | −39% |
| 1100 | 55,543 | 29,614 | 1.31 | 1.06 | −19% |
| 1184 | 167,960 | 52,259 | 3.24 | 1.60 | −51% |
| 1196 | 1,942,366 | 216,076 | 36.3 | 5.9 | −84% |
| 1204 | 3,570,480 | 498,121 | 69.2 | 14.3 | −79% |
| 1215 | 5,485,294 | 1,111,451 | 108.5 | 34.2 | −68% |
| 1220 | 8,375,382 | 2,503,016 | 206.7 | 89.0 | −57% |
| 1225 | 12,071,677 | 2,178,362 | 322.4 | 85.8 | −73% |
| 1240 | 4,374,708 | 496,460 | 104.5 | 18.5 | −82% |
| 1250 | 2,792,227 | 709,893 | 54.6 | 20.7 | −62% |
| 1254 | 20,245,006 | 4,830,651 | 397.1 | 147.1 | −63% |
| 1257 | 12,769,713 | 3,686,322 | 244.8 | 108.1 | −56% |
| 1272 | 25,067,509 | 4,341,863 | 473.7 | 127.9 | −73% |
| 1280 | 15,352,847 | 3,080,373 | 354.6 | 109.3 | −69% |
| 1285 | 16,902,075 | 3,442,845 | 379.4 | 117.0 | −69% |
| 1288 | 23,625,803 | 4,822,476 | 536.8 | 166.5 | −69% |
| 1290 | more than 75.1M (stopped at 600 s) | 19,473,016 | more than 1872 | 734.4 | below −61% |
| 1300 | more than 79.6M (stopped at 600 s) | 28,024,916 | more than 1872 | 957.2 | below −49% |

From 1184 to 1288 the ratio of cycles is 2.0 to 6.1, 3.3 as a geometric mean, and it does not grow with n: a least
squares line through its logarithm falls by 12% per 100 in n, well inside the scatter from game to game (1220 2.3,
1240 5.6).  At ×10 per 55-70 in n (solver/notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md) a constant factor of 3.3 moves the cost curve down by
half a decade, about 30 to 35 in n, and leaves its slope as it was.  Each step costs about half as much again
(the window of the probe, 50,000 steps resumed from the 1215 proof: 25.3k cycles a step against 16.9k), so the
games where the search spends its time finding the optimum late, not proving it, lose.

Beyond proof-only searches:

* **The strict chain 1201-1231** (`taxman solve 1201-1231 --strict --threads 4`, in memory, from optimal.json's 1200
  with `--allow-tainted-prev`): 283.5 s of game time and 3,448 G cycles with the tactic off, 117.2 s and 1,411 G with
  it on; every searched game faster (in wall time on four threads: 2.0× for 1212 to 3.6× for 1203 and 1204), every
  score the same.
* **Four threads**, the proof of 1225: 9.93M nodes, 289 G cycles, 23.8 s off; 2.92M nodes, 124 G cycles, 10.3 s on.
* **Seeded from n − 1** (one thread, what a chain does): 1215 −71%, 1220 −60%, 1240 −83%, 1250 −70%; but 992, whose
  optimum is found late, +35% (4.20M → 4.02M nodes, 114 → 155 G cycles), and 1025 +4%.  The bench set (936, 1050,
  1100, 1133, 1170, 1184, seeded): −2%, −24%, +11%, +16%, −36%, −52%, −24% in all; 1133's search is 2,386 steps,
  and its one refit (after 2,000) is most of the difference.  A search of fewer than 2,000 steps is unchanged.
* **Correctness at scale**: `taxman solve 761-1000 --strict --force-search` on four threads matches optimal.json on
  all 240 games, and `taxman regress --to 1000` on all 1000.

Measured, not kept (with forced members, in the build of the experiments, cycles against 32.3 G at 1215 and 151.3 G
at 1254 with the schedule kept): 20 subgradient steps 35.1 and 170.0 G; 100 steps 35.7 and 145.2 G; the first refit
after 500 steps 30.7 and 150.0 G; refits every 10,000 steps 31.3 and 157.9 G (four to five times as many refits).
All within ±12%, so the schedule stays the simple one.

**Verdict.**  Kept, on by default: a constant factor of about 3 on the proofs that dominate the frontier, with no
change of slope; even on short searches, a loss where finding the optimum dominates.

**What would change it, and what is left.**

* *Refinement may no longer pay.*  With the tactic on, `match_refine=0` took 1215 29.3 G (against 32.3 G in the same
  build), 1254 112.0 G (151.3), 1240 18.4 G (18.4), 1250 17.6 G (20.5): 0 to 26% fewer.  (Without refinement a child's
  check stops at the matching bound, the Lagrangian bound included, and its first step checks it fully.)  Refinement
  is 12-20% of the time; it needs its own measurement over the bench set and small games before its default moves.
  (Measured in ["Faster nodes with the Lagrangian bound"](../faster-nodes-with-the-lagrangian-bound-same-trees-2026-10/README.md):
  a few percent on average, ±20% per game, even on the strict chain 1201-1231; the default stays 1.)
* *Refits without solving afresh*: re-weighting the levels in place (weights that fall unmatch their candidates if
  promoted, as weights that rise do the rest) and warm-starting the fit, about 4% now.
* *Many threads*: each searcher fits its own multipliers to its own table, which at 192 threads is a smaller sample of
  a smaller region; measured on four threads only.
* *The second matching's cost* (half a step): what a bound costs is set by the changes along the path, not by how
  often it is asked (step 1), so asking less often saves nothing; a cheaper bound would have to come from fewer
  levels or cheaper repairs.
* *The late-optimum games* (992, 1025) are where it does not pay; a search that knows it has not yet found the
  optimum (incumbent improvements still coming) could hold the bound back.

**The whole strict chain with both of this month's shortcuts** (`same_search` and `lagrange`, build a2da702):
`taxman solve 1-1300 --strict --threads 4` from n = 1, on the laptop while two other jobs ran, took 1038 s of wall
time in all (3975 s of CPU).  Every one of the 1300 scores equals the committed `results/strict.jsonl`'s, no game is
tainted, and both referees pass it; its proofs are same_search 545, exhausted 416, prime formula 211, cap 115,
trivial 13.  Search seconds by range, against the committed run (bd9bb5d, 6 threads): 1..1000 51.8 against 82.8,
1001..1231 184.6 against 409.7, 1232..1300 775.7 against 2866.3, so 3.7 times less wall time on two thirds of the
threads at the frontier.  The slowest games are now 1232 (142 s, was 439), 1295 (107, was 259), 1300 (88, was 179) and
1287 (85, was 273).

**The committed chain replaced** (build 243af9b, which has both shortcuts): `results/strict.jsonl` is now a run of
`taxman solve 1-1300 --strict --threads 4` from n = 1 with that build, in one invocation, on the laptop while other jobs
ran: 1078 s of wall time and 4014 s of CPU, against 1038 s and 3975 s of the run above; search seconds 51.3, 187.8 and
813.2 for 1..1000, 1001..1231 and 1232..1300, against 51.8, 184.6 and 775.7; the same 1300 scores and the same proofs
(same_search 545, exhausted 416, prime formula 211, cap 115, trivial 13).  It replaces the file of commit bd9bb5d (6
threads, schema 3; `git show cd3d32f:rust/results/strict.jsonl`), which "the committed `strict.jsonl`" and "the
committed run" in the entries above mean (the figures of the same-search round, which come from `repeats --known`, need
that file: on the new one the repeats cost 0.0 s, since it did not search them).  Its scores are that file's, and its
moves too but for 1221..1224, which have at least two optimal sets, and the two runs found different ones
([results](../../../results/README.md#strictjsonl)).
