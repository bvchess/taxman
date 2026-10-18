# Status

Updated 2026-10-17.

What is in flight, what is paused, what is decided and what waits on a decision, as of the date above.  This page is
the only place for the current state: it is rewritten, not appended to, whenever work lands or a decision is made
(with the date), and it stays under 150 lines.  History goes to an entry of the [lab notebook](EXPERIMENTS.md); the
results and how they are proven are in the [README](../README.md); the research avenues are in the
[agenda](AGENDA.md).

## The results

* The committed chain, `results/strict.jsonl`, reaches 3440.  Games 2-2175 are proven on the whole board by the
  prover; games 2176-3440 within the frame model, which is exact given statement U
  ([README, "What optimal means"](../README.md#what-optimal-means)).  `optimal.json` stays at 1300; nothing is
  published.
* 3440 is confirmed: the run to the wall re-proved it with the fixed build (the counter that could wrap,
  [the wrap](experiments/cheaper-gf2-nodes-same-trees-2026-10.md#later-2026-10-the-stamps-wrap-and-the-committed-chain)),
  the same score and proof in 1.76 hours and 1.36 billion nodes (5.05 hours and 3.72 billion before); its games 1..3440
  equal the committed chain's scores and proofs, game for game (2026-10-17).
* Past 3440: the run to the wall has proven 3441 onward (3491 by 10:45 PDT on 2026-10-17), not yet appended to
  `results/strict.jsonl` (they will be, once both referees pass on the joined file).

## The priority: a larger n

Brian's priority (2026-10-16) is a larger n, the end of the chain pushed further; certifying U past 2175 is not a
priority for now.

The cost of a searched game grows tenfold for about every 1,040 in n, in patches: the valley 2576-2975 cost minutes,
the last forty games of the run held three of its four dearest (3440 alone 5 hours on 64 threads).  At the run's own
trend, one `c8g.16xlarge` at spot ($0.68 an hour) reaches about:

| more machine time | n reached | cost |
|---|---|---|
| 1 day | 3,740 | $16 |
| 3 days | 4,050 | $49 |
| 10 days | 4,500 | $160 |
| a month | 5,000 | $530 |

A single run can fall well short of its projection (the last one projected 4,100 and reached 3440).  Every doubling of
speed or of spending is worth about 310 in n; only a tighter bound in the search changes the tenfold per 1,040.  The
chain's launch is [RUNBOOK section 5](../tools/cloud/RUNBOOK.md#5-the-production-chain); it needs Brian's approval.

**The wall** (Brian, 2026-10-16): the first n whose own search (not time spent waiting behind another game) takes
more than 24 hours on one `c8g.16xlarge` with the run's build and settings (`solve --game-hours 24`: the hours the game
has been the oldest of the window with its search running).  The next run stops there, keeping the game's checkpoint,
or when its budget (about $50) is spent.  On 64 threads the search loses nothing to threads (3.5
thousand nodes a second a thread at 3441, the same on 16, 32 and 64), so what remains is cheaper nodes and smaller
trees.

What has been tried for the bound, and where it stands:

* **A1, the multi-field bound in the search: closed.**  At the roots it closes 0.4-0.6 of the gap; in a branch and
  bound its trees are smaller by a factor that falls as the gap grows
  ([E3 pilot](experiments/the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10.md),
  [R3 and the slope test](experiments/r3-and-the-slope-test-u4-of-a1-2026-10.md)).
* **Carrying a search's work to the next game: closed.**  The trees of neighbours share 0.15% of their steps
  ([carrying](experiments/carrying-a-searchs-learning-to-the-next-game-2026-10.md)).
* **A3, splitting by the number of promotions: closed.**  At 936, 1215 and 2112 every node's heaviest completion
  already has the root's size, so a bound by size equals the plain one
  ([A3's test](experiments/split-by-the-number-of-promotions-a3s-test-2026-10.md)).
* **A5, decomposing across the frames: not tried** (agenda A5; high risk).

**The landscape past the chain** (2026-10-17): the standalone bounds of every game of 3441-8191 are measured
([bench/landscape/](../bench/landscape/README.md), for $1.10).  None meets, the smallest standalone gap is 1,196
(4176-4179) and the median rises from about 1,800 at 3441-4440 to 4,600-8,300 from 5441, so with today's heuristic no
game past 3440 is within a few minutes of one-thread search
([the entry](experiments/the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10.md)).

## Decided

* **The window is 4** (2026-10-17), the default of `taxman solve --window` (it was 1) and of `chain.sh` (it was 16).
  At 64 threads a window of 16 made the games past 2225 20 to 32% slower than 1, and 4 was the best measured
  ([round B](experiments/round-b-the-chain-on-many-threads-2026-10.md); [scaling to 192
  threads](experiments/scaling-to-192-threads-2026-10.md)).  It changes how threads are shared, never a score or a
  proof.
* **The forcing window is `all`** (2026-10-16; it was 80).  The tuning campaign (one `c8g.16xlarge`, 3.8 hours, about
  $2.50) measured it on subtrees of 3300, 3425, 3440 and 3441 and six whole games on one thread, and on complete
  proofs of seven games (3179-3395, 3300) on 64 threads: 0.60-0.63 of the CPU and 0.41-0.43 of the nodes on the
  subtrees, 0.84 of the time and 0.61 of the nodes on the proofs (by the rule above, about 80 in n), and no problem
  more than 4% slower.  A node costs 1.4 times more and the trees fall by more.  The run to the wall has this default
  and no other change: `strong_limit=4` and `more_forced=off` are within a few percent of it, shorter nogood periods
  and the Lagrangian bound gain nothing, and cheaper branching at depth is 4 to 6 times worse
  ([the campaign](experiments/the-tuning-campaign-at-the-end-of-the-chain-2026-10.md)).

## Paused

* **The claim-by-claim rule** (Lemma M5⁺ claim by claim, for the certificates): until after the weekly reset
  (Brian, 2026-10-16: Monday night).  Its branch `worktree-agent-a2db9341d6d4e8cd9` holds two commits and an
  uncommitted edit of THEORY.md in its worktree.
* **Certifying 2176-3440** ([RUNBOOK section 11](../tools/cloud/RUNBOOK.md#11-certifying-u-stage-4)): ready, not
  approved, shelved by Brian on 2026-10-16.  About 3,023 thread-hours, 47 hours of one `c8g.16xlarge`, $32 at spot
  ($121 on demand), the pilot first ($3); phases A and B on the build of 14beb7a, phase C on the claim-by-claim build
  ([certificates past 2175](experiments/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10.md),
  [the claims](experiments/the-claims-of-unzip-in-the-extended-problem-2026-10.md)).

## Dropped

* E3 main (A1's large cloud run): dropped after the pilot.
* E6 and E9(a) (the theory track's cloud measurements): superseded by the certificates.

## Waiting on Brian

* Nothing for the run itself: Brian approved it (2026-10-16); periodic updates need his AWS session (12 hours each).
* Outside review of Theorems 1, 10 and 12 by Robert Moniot.
* Publishing (`optimal.json` past 1300, OEIS).
* The ledger's experiment with the climbs on 2176..3440 (about an hour of a `c8g.16xlarge`, about $0.9 at spot;
  "In flight").

## In flight

* The engineering pass before the run is done: a cheaper node with identical trees (the tableaux kept per level,
  cheaper cycle searches), the knobs for nogood retention, the forcing window and strong branching, the harness that
  tunes them on subtrees of searches at the end of the chain, and the campaign that used it (see "Decided";
  [the profile at the end of the chain](../bench/chain_end/README.md)).
* **The run to the wall**, launched 2026-10-17 00:15 PDT: `RUN=to-the-wall rust/tools/cloud/wall.sh status`
  ([RUNBOOK section 12](../tools/cloud/RUNBOOK.md#12-the-unattended-run-to-the-wall)).  The chain from n = 1 with
  commit eb84a6f (`force_window=all`, the cheaper node), on one `c8g.16xlarge` at spot (at most $0.75 an hour) in an
  Auto Scaling group that replaces an interrupted machine; it ends at the wall (a game past 24 hours), at 72
  instance-hours (about $49), or on 2026-10-22.  Its games 1..3440 are compared with the committed chain (3440 is
  re-proven by the fixed build), and the games past 3440 are appended once verified.  The rehearsal of
  [RUNBOOK 12.5](../tools/cloud/RUNBOOK.md#125-the-rehearsal-designed-not-run) passed (the wall, a replacement, the
  budget).
* **The sweep**, built 2026-10-17 and not run (Brian's plan: as many proven games of 3441..8191 as one cheap machine
  gives, holes allowed): `sweep.sh FIRST LAST` splits the stretch into blocks of consecutive games, one thread and a
  limited search a game each, goes on past the games that do not finish, and goes over the merged `explore.jsonl`
  again until each game has seen the game before it as it is now
  ([RUNBOOK section 13](../tools/cloud/RUNBOOK.md#13-the-sweep); `taxman solve --keep-going`).  3441..8191 at 180 s a
  game is at most 314 core-hours: 4.9 hours of a `c8g.16xlarge`, about $3.3 at spot; a launch needs Brian's approval.
  The file is not a chain and never goes into `strict.jsonl`.
* **The ledger**, built 2026-10-17 (Brian's idea: L and U for every board of a stretch at once, limited searches with
  the dual bound of a stopped search, the relations between boards proven in THEORY.md, the effort where a proof looks
  cheapest; `examples/ledger/`).  The laptop's dry run of 1301..1500 with the truth hidden proves all 200 boards at
  the optimum, 0 violations, for 1.28 times the one-thread chain's CPU (573 s) in 30% of its wall time on six workers
  ([the entry](experiments/the-ledger-bounds-from-both-sides-2026-10.md)).  The experiment of the design, 2176..3440,
  was launched twice (2026-10-17).  The first was stopped after 9 minutes ($0.18): the machine was 95% idle, the main
  thread replaying a game at every step of each map's cascade; fixed the same day
  ([the fix](experiments/the-ledger-bounds-from-both-sides-2026-10.md#later-2026-10-17-the-first-cloud-attempt-stopped-and-the-fix)).
  The second (48 core-hours on 64 workers) proved 916 of 1,265 boards, every one at the optimum, but paid about 2.2
  times the chain's cost on the same games, because most searches began before the board below was proven.  **Wedge
  and climb**, built the same day: the board above a proven board, and then the board below it, is searched ahead of
  every other job, seeded with its game, for 60 s (`--climb up|down|both|off`, `--climb-limit`; both by default, so
  that the next cloud run says what each way costs and proves); a climb that stops parks the board in the ordinary
  queue; a board next to a climb waits.  On the laptop 1301..1500 takes 547 to 576 CPU s against 598 without the
  climbs, all proven at the optimum, 0 violations; 2576..2700 shows no difference beyond the noise
  ([the entry](experiments/the-ledger-bounds-from-both-sides-2026-10.md#later-2026-10-17-wedge-and-climb)).  A cloud
  run with the climbs is ready, not run, and needs Brian's approval
  ([RUNBOOK section 14](../tools/cloud/RUNBOOK.md#14-the-ledgers-experiment)).
