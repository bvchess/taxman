# Status

Updated 2026-10-20.

What is in flight, what is paused, what is decided and what waits on a decision, as of the date above.  This page is
the only place for the current state: it is rewritten, not appended to, whenever work lands or a decision is made
(with the date), and it stays under 150 lines.  History goes to an entry of the [lab notebook](../notebook/README.md);
how far the chain reaches and what proves each part are in [results/README.md](../../results/README.md); the research
avenues and the projections are in the [agenda](AGENDA.md).

## The priority: a larger n

Brian's priority (2026-10-16): a larger n, the end of the chain pushed further; certifying U past 2175 is not one for
now.  Nothing is published: `optimal.json` stays at 1300.

**The wall** (Brian, 2026-10-16): the first n whose own search (not time spent waiting behind another game) takes more
than 24 hours on one `c8g.16xlarge` with the run's build and settings (`solve --game-hours 24`: the hours the game has
been the oldest of the window with its search running).  The next run stops there, keeping the game's checkpoint, or
when its budget is spent ([runbook/wall.md](runbook/wall.md#what-needs-brians-approval-and-what-it-costs)).  The chain's
launch is [runbook/chain.md](runbook/chain.md), the unattended run
[runbook/wall.md](runbook/wall.md); each needs Brian's approval.

The cost of a searched game grows tenfold for about every 1,040 in n, in patches; [AGENDA.md, "The diagnosis of the
wall"](AGENDA.md#2-the-diagnosis-of-the-wall) has the projections, and why only a tighter bound in the search changes
that slope.  On 64 threads the search loses nothing to threads (3.4 thousand nodes a second a thread at 3441, the same
on 16, 32 and 64), so what remains is cheaper nodes and smaller trees.  Every avenue tried for the bound is closed but
the certificates (A2, paused): [AGENDA.md, "Ranked avenues"](AGENDA.md#3-ranked-avenues).  For smaller trees, learning
each conflict's closure under cheap twin exchanges takes 21% off the steps and 16% off the CPU of the 46 problems of the
tuning suite (37-49% off three hard units' steps, 4.5% of the CPU off its six whole games), in the studies build only so
far ([cheap exchanges](../notebook/conflicts-closed-under-cheap-exchanges-2026-10/README.md)).

## Recently closed

* **The run to 5000** (2026-10-18 to 20): games 3915-4547 appended to the committed chain, 167 of them from the bounds
  store with no search, for 40.4 instance-hours and $27.32, both referees passing; the budget ended it in game 4548,
  whose checkpoint is in S3 ([the entry](../notebook/the-run-to-5000-2026-10/README.md)).
* **The run to the wall** (2026-10-18): 1..3440 solved again with the fixed build (the counter that could wrap), with
  the committed chain's scores and proofs, game for game, and 3441-3914 appended, both referees passing ([the
  entry](../notebook/the-run-to-the-wall-2026-10/README.md)).
* **The bound's avenues** (2026-10-16 to 18): A1, A3 and A5, the Lagrangian bound over the nogoods refitted at a node,
  and flat-cycle cuts; why the trees are big ([the agenda, sections 2 and 3](AGENDA.md#3-ranked-avenues)).
* **The landscape past the chain** (2026-10-17): no game of 3441-8191 meets its standalone bounds, and the smallest
  standalone gap is 1,196 ([the
  entry](../notebook/the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md), for $1.10).
* **Finders for lower bounds** (2026-10-17): large neighbourhood search from the start incumbent finds opt(n) in 64% of
  253 games of 2176-3440 in 5 CPU seconds of one thread; it is in the ledger, not in the solver ([the
  entry](../notebook/finders-for-good-games-that-are-not-sound-2026-10/README.md)).  Seeding the chain with it is worth
  about 15% of the search time past 2175 and none of the dear games' excess ([the dear
  games](../notebook/the-dear-games-the-problem-or-the-chains-path-2026-10/README.md)).
* **Floors for upper bounds** (2026-10-18): a search under a floor proves "nothing weighs more than F" far more cheaply
  than the proof (a decade of nodes per 147 points); it is in the ledger ([the
  entry](../notebook/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md)).

## Decided

* **The window is 4** (2026-10-17), the default of `taxman solve --window` (it was 1) and of `chain.sh` (it was 16): at
  64 threads a window of 16 made the games past 2225 20 to 32% slower than 1, and 4 was the best measured; it changes
  how threads are shared, not a score or a proof ([round
  B](../notebook/round-b-the-chain-on-many-threads-2026-10/README.md); [scaling to 192
  threads](../notebook/scaling-to-192-threads-2026-10/README.md)).
* **The forcing window is `all`** (2026-10-16; it was 80): on the proofs of seven games at the end of the chain on 64
  threads it took 0.84 of the time and 0.61 of the nodes, each game faster, and on one thread no problem of the suite
  was more than 4% slower ([the tuning
  campaign](../notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)).

## Paused

* **The claim-by-claim rule** (Lemma M5⁺ claim by claim, for the certificates): until after the weekly reset
  (Brian, 2026-10-16: Monday night).  Its branch `worktree-agent-a2db9341d6d4e8cd9` holds three commits (the third is the edit of
  THEORY.md) and is on the old layout (`rust/`, `heuristics/THEORY.md`): its paths need the new ones before it merges.
* **Certifying 2176-3440** ([runbook/certify.md](runbook/certify.md)): ready, not
  approved, shelved by Brian on 2026-10-16.  About 3,023 thread-hours, 47 hours of one `c8g.16xlarge`, $32 at spot
  ($121 on demand), the pilot first ($3); phases A and B on the build of 14beb7a, phase C on the claim-by-claim build
  ([certificates past 2175](../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md),
  [the claims](../notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md)).

## Dropped

* E3 main (A1's large cloud run): dropped after the pilot.
* E6 and E9(a) (the theory track's cloud measurements): superseded by the certificates.

## Waiting on Brian

* Outside review of Theorems 1, 10 and 12 by Robert Moniot.
* Publishing (`optimal.json` past 1300, OEIS).

## In flight

* **The sweep**, built 2026-10-17 and not run: blocks of consecutive games of 3441..8191, one thread and a limited
  search a game each, passes over the merged `explore.jsonl` until nothing changes
  ([runbook/sweep.md](runbook/sweep.md); `taxman solve --keep-going`); about $3.3 at 180 s a
  game.  The file is not a chain and never goes into `strict.jsonl`.  The ledger (next) may supersede it.
* **The ledger**, built 2026-10-17 (Brian's idea: L and U for every board of a stretch at once, limited searches with
  the dual bound of a stopped search, the relations between boards proven in THEORY.md, the effort where a proof looks
  cheapest; `examples/ledger/`; [the entry](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md)), with **wedge and
  climb** (a board next to a proven one is searched first, from its game), **the finder** (large neighbourhood search
  for a lower bound, replayed by the referee) and **the bounds store** (`results/bounds.jsonl`, append-only; `ledger
  export`, `run --bounds`, `verify-bounds` and `solver/tools/check_bounds.py`; [results/README](../../results/README.md#boundsjsonl)).
  On 2176..3440 with the truth hidden every proven board is at its committed score: 916 of 1,265 in 48 core-hours without the
  climbs and the finder, 876 in 24 with both (the first run needed about 38 for as many).
* **The ledger past 4400** (Brian approved up to $25, 2026-10-17): stage 1 (4400..8191, 360 core-hours, about $3.80)
  proved 242 boards in 4400..4799; stage 2 (2026-10-18, 4300..4999) took the store to 343, and the survey to 382, the
  last 4783.  **The chain draws on it** (2026-10-18, `solve --use-prev`: a board that the store, the committed chain or
  `optimal.json` has proven is recorded with no search, citing it;
  [runbook/wall.md](runbook/wall.md#launch-watch-stop-clean-up)).  The run to 5000 took 167 games from it; 215 of its
  boards lie past the chain's end, in 4548..4783, so 238 of the 453 games from 4548 to 5000 need a search.
* **Floor-guided drilling** (2026-10-18, f775cc0 and 26c5c71; [the entry's
  section](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-18-floor-guided-drilling)): the ledger
  steps an open board down by searches under floors (150 points, a tenfold of cost a step; each lowers U), fits a line
  to each board's steps that forecasts its proof, and orders the boards by it.  On 2576..2700 at 900 CPU s 69 and 73
  boards were proven against 61; dry-run, not run in the cloud; the next stage resumes stage 2's ledger with it
  ([runbook/ledger.md](runbook/ledger.md)).
