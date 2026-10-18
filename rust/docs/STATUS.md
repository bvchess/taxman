# Status

Updated 2026-10-18.

What is in flight, what is paused, what is decided and what waits on a decision, as of the date above.  This page is
the only place for the current state: it is rewritten, not appended to, whenever work lands or a decision is made
(with the date), and it stays under 150 lines.  History goes to an entry of the [lab notebook](EXPERIMENTS.md); the
results and how they are proven are in the [README](../README.md); the research avenues are in the
[agenda](AGENDA.md).

## The results

* The committed chain, `results/strict.jsonl`, reaches 3914.  Games 2-2175 are proven on the whole board by the
  prover; games 2176-3914 within the frame model, which is exact given statement U ([README, "What optimal
  means"](../README.md#what-optimal-means)).  `optimal.json` stays at 1300; nothing is published.
* The run to the wall solved 1..3440 again with the fixed build (the counter that could wrap), with the committed
  chain's scores and proofs in all, game for game; 3440 took 1.76 hours and 1.36 billion nodes (5.05 hours and 3.72
  billion before).  Its games 3441-3914 are appended, both referees passing.

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
* **The strong bound at the nodes: closed** (2026-10-18).  Refitted at a node it cuts 92% of the open nodes with a gap
  under 50, 19% at 100-199 and none from 300, where the cost is, and costs 164-234 GF(2) bounds: every policy of
  computing it loses ([the entry](experiments/the-node-choice-where-the-strong-bound-pays-2026-10.md)).
* **A5, decomposing across the frames: not tried** (agenda A5; high risk).

**The landscape past the chain** (2026-10-17): the standalone bounds of every game of 3441-8191 are measured
([bench/landscape/](../bench/landscape/README.md), for $1.10).  None meets, the smallest standalone gap is 1,196
(4176-4179) and the median rises from about 1,800 at 3441-4440 to 4,600-8,300 from 5441, so with today's heuristic no
game past 3440 is within a few minutes of one-thread search ([the
entry](experiments/the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10.md)).

**Finders for lower bounds** (2026-10-17): a lower bound needs only a legal game, so a search that is not sound may
find it.  Large neighbourhood search from the start incumbent (keep a random 70-90% of its promotions, search the rest
exactly as a unit, repeat) finds the optimum of 64% of the games of 2176-3440 in 5 CPU seconds of one thread (the sound
search for the same time: 21%), a mean shortfall of 28 against 297; the sound search seeded with its game takes 0.44 of
the nodes and 0.48 of the CPU, finder included, of the search from today's start.  It is in the ledger (below), not in
the solver (`wall finders` and `wall sound` measured it and are removed; [the
entry](experiments/finders-for-good-games-that-are-not-sound-2026-10.md)).

**Floors for upper bounds** (2026-10-18): a search from a floor F, an incumbent worth F with no set
(`search::search_above`, THEORY.md Lemma F), proves "nothing weighs more than F" for an F above the optimum far more
cheaply than the proof: on 14 games of 2265-3259, proving down a quarter, a half and three quarters of the gap took a
median 1/20,000, 1/740 and 1/26 of the full proof's nodes, the log of the nodes linear in the distance (a decade per
147; r² 0.996).  It is in the ledger (`wall target` measured it and is removed; [the
entry](experiments/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10.md)).

## Decided

* **The window is 4** (2026-10-17), the default of `taxman solve --window` (it was 1) and of `chain.sh` (it was 16).
  At 64 threads a window of 16 made the games past 2225 20 to 32% slower than 1, and 4 was the best measured
  ([round B](experiments/round-b-the-chain-on-many-threads-2026-10.md); [scaling to 192
  threads](experiments/scaling-to-192-threads-2026-10.md)).  It changes how threads are shared, not a score or proof.
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

* **The run to 5000**, launched 2026-10-18 at commit ca357bf with `--use-prev` (the chain draws on the committed
  chain, the bounds store and `optimal.json` and starts at 3915: [RUNBOOK
  12.4](../tools/cloud/RUNBOOK.md#124-launch-watch-stop-clean-up)), budget 36 instance-hours.  The **run to the wall**
  before it ([RUNBOOK section 12](../tools/cloud/RUNBOOK.md#12-the-unattended-run-to-the-wall), launched 2026-10-17 at
  00:15 PDT with eb84a6f) was stopped at 3914 on 2026-10-18 after 35.7 instance-hours and $24.14, the wall far off: its
  dearest game took 2.77 hours ([the entry](experiments/the-run-to-the-wall-2026-10.md)).
* **The sweep**, built 2026-10-17 and not run: blocks of consecutive games of 3441..8191, one thread and a limited
  search a game each, passes over the merged `explore.jsonl` until nothing changes
  ([RUNBOOK section 13](../tools/cloud/RUNBOOK.md#13-the-sweep); `taxman solve --keep-going`); about $3.3 at 180 s a
  game.  The file is not a chain and never goes into `strict.jsonl`.  The ledger (next) may supersede it.
* **The ledger**, built 2026-10-17 (Brian's idea: L and U for every board of a stretch at once, limited searches with
  the dual bound of a stopped search, the relations between boards proven in THEORY.md, the effort where a proof looks
  cheapest; `examples/ledger/`; [the entry](experiments/the-ledger-bounds-from-both-sides-2026-10.md)), with **wedge and
  climb** (a board next to a proven one is searched first, from its game), **the finder** (large neighbourhood search
  for a lower bound, replayed by the referee) and **the bounds store** (`results/bounds.jsonl`, append-only; `ledger
  export`, `run --bounds`, `verify-bounds` and `tools/check_bounds.py`; [results/README](../results/README.md#boundsjsonl)).
  On 2176..3440 with the truth hidden every proven board is at the optimum: 916 of 1,265 in 48 core-hours without the
  climbs and the finder, 876 in 24 with both (the first run needed about 38 for as many).
* **The ledger past 4400** (Brian approved up to $25, 2026-10-17): stage 1 (4400..8191, 360 core-hours, about $3.80)
  proved 242 boards in 4400..4799; stage 2 (2026-10-18, 4300..4999) took the store to 343.  **The chain draws on it**
  (2026-10-18, `solve --use-prev`: a board that the store, the committed chain or `optimal.json` has proven is recorded
  with no search, citing it; not run in the cloud; [RUNBOOK 12.4](../tools/cloud/RUNBOOK.md#124-launch-watch-stop-clean-up)).
* **Floor-guided drilling** (2026-10-18, f775cc0 and 26c5c71; [the entry's
  section](experiments/the-ledger-bounds-from-both-sides-2026-10.md#later-2026-10-18-floor-guided-drilling)): the ledger
  steps an open board down by searches under floors (150 points, a tenfold of cost a step; each lowers U), fits a line
  to each board's steps that forecasts its proof, and orders the boards by it.  On 2576..2700 at 900 CPU s 69 and 73
  boards were proven against 61; dry-run, not run in the cloud; the next stage resumes stage 2's ledger with it
  ([RUNBOOK 14](../tools/cloud/RUNBOOK.md#14-the-ledgers-experiment)).
