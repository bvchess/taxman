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
* 3440 is unconfirmed: a counter in the GF(2) search's workspace could wrap after about 92 million steps of one
  worker in one game, and 3440 took 63-70% of that (fixed in 6460c86;
  [the wrap](experiments/cheaper-gf2-nodes-same-trees-2026-10.md#later-2026-10-the-stamps-wrap-and-the-committed-chain)).
  It is to be re-solved with the fixed build before 3441 resumes on its ceiling; no other committed game came within a
  factor of two.
* 3441 is unfinished: 626 million nodes after 5.9 hours on 64 threads, its checkpoint (format 4, which the present
  build reads) in the project's bucket under `chain-2176/ckpt`
  ([the frontier chain](experiments/the-frontier-chain-to-3440-2026-10.md)).  It was made with the forcing window of
  80, which a header of format 4 or 5 implies, so the default's search (window `all`) resumes it only as `--tactic
  force_window=80`.

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
