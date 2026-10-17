# The ledger: bounds from both sides (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

**Question.**  Brian's idea (2026-10-17): instead of proving opt(n) one game at a time from a proven opt(n − 1), keep
for every n of a stretch a lower bound L(n), the score of a game held, and an upper bound U(n), tighten both across
many games at once, and spend the effort where progress is cheapest; a game is proven when L(n) = U(n).  Does it prove
much of a stretch for a small share of what the chain spends there?  This entry builds it and runs it on the laptop on
1301..1500 with the truth hidden; the experiment of the design, 2176..3440 for an hour of a `c8g.16xlarge`, is ready
and not run ([RUNBOOK section 14](../../tools/cloud/RUNBOOK.md#14-the-ledgers-experiment)).

**What was built.**

* **The dual bound of a stopped search** (`search::dual_bound`, `src/search/dual.rs`): the largest of the incumbent and,
  over the open units of the final checkpoint, each node's promotion sum plus the GF(2) bound the search computes when
  it starts the unit (`search::NodeBound`, which `examples/tune.rs` now reads its units' bounds through).  Valid by the
  search's exactness and Theorem 11 (heuristics/THEORY.md, Lemma D); until now a stopped search threw it away.
* **The relations, proven** (THEORY.md, "The ledger: bounds between boards"): L1 monotone, opt(n − 1) ≤ opt(n), with
  the game of n − 1 followed by n if still legal; L2 the ceiling, opt(n) ≤ opt(n − 1) + n, with the game below it (the
  game of n without the move n stays legal, a pot-inclusion argument, then completed); L3 the prime formula both ways,
  opt(p) = opt(p − 1) + p − q for q the largest prime below p, the games by exchanging the opening q and p; L4 repeats,
  U(n) − bag(n) ≤ U(m) − bag(m) for equal search problems, and m's promotions played on n's board.  Two corrections to
  the design: the prime formula is + p − q, not + p − 1; and none of the constructions needs an optimal game, only a
  finished one (legal, over, opening with the largest prime), which every game the ledger holds is.  What the
  constructions do need: a repeat carries a lower bound only as the promotions of a game made from them (any other
  game's candidate moves are tried and kept only if the referee accepts them), and its upper half rests on statement U
  at the board it is carried to, as everything from the frame model does.
* **The ledger** (`examples/ledger/`: `book.rs` the bounds and relations, `schedule.rs` the scheduler, `report.rs` the
  report and check; the module comment of `main.rs` has the commands and the files).  A Rust example rather than a
  Python driver of the binaries: the searches need what no command gives, a seed from any game and the ledger's U as
  their ceiling, the dual bound read from each checkpoint, and the relations need the boards' problems and candidates;
  one process with a thread a search shares them and the build.  Every board searched first gets its map (a search
  stopped before its first node: the start incumbent and the root bound, as `wall landscape`); then searches of one
  thread, `--base` seconds the first time and twice as long each time after, each going on from the board's checkpoint,
  seeded with the best of the games held for n − 1, n and n + 1, with U(n) as the ceiling; only one board of a repeat
  class is searched, and no prime whose board before it is in the ledger.  The order: boards whose bounds a proven
  neighbour has changed (the ceiling and the prime formula next to a proven board are exact), smallest gap first; then
  least `level + gap / 175` (a decade of the chain's steps per about 580 of standalone gap,
  [the landscape](the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10.md)).  After each search the
  relations run to a fixpoint; every game they make is replayed, L > U stops the run; a proven board's search is stopped
  and its checkpoint deleted, a falling U is given to a running search as its ceiling.  The ledger file and the event
  log let a run go on after an interruption, and the CPU budget counts all runs.
* **The cloud script** `bench/ledger/cloud.sh` (the run with no proven game, then `ledger report --known
  results/strict.jsonl`, synced to S3), with RUNBOOK section 14's user data.  For 2176..3440 at an hour of 64 CPUs:
  about 1.3 instance-hours with the build, **about $0.9 at spot**, $3.3 on demand.

**Tests (the quick suite).**  The dual bound: a search stopped before its first node holds the root unit alone and its
dual bound is the root's GF(2) bound (14 boards of 60..1300); one-thread searches of 996, 1020 and 1218 stopped at
2,047, 3,071, 5,119 and 9,215 steps (the engine checks its limit every 1,024) have dual bounds between the optimum and
the root bound, never rising, and the completed search's is the optimum.  The bound falls slowly: of the root gaps
(318, 223 and 146 over the optimum) it had closed 23%, 61% and 29% at 9,215 steps, 65%, 96% and 88% of the way through
their trees (14,200, 9,554 and 10,481 steps).  A depth-first search keeps the shallowest, largest subtrees open until
late, and their bounds are the root's or near it.  The relations: proven games at every tenth board of 2..1300 give
every board both bounds, all on the right side of optimal.json and every game legal, each relation used for both
bounds, exact at the primes and repeats of the proven boards; loose bounds on 1001..1300 with a proven game every
fiftieth stay on the right side; a lower bound above an upper one is refused, offered or derived.  The scheduler: 901..960
from millisecond searches is proven at the optimum, by stopped and completed searches, primes and repeats, with no
checkpoint left, and a second run on the same directory finds nothing to do and keeps every bound; given proven games
are fixed points.

**The dry run** (`bench/ledger/`: data, commands and the script of the numbers).  1301..1500 on the laptop, the
non-release build, six workers, a budget of 2 CPU-hours, no proven game (optimal.json and `results/strict.jsonl` only
for the check afterwards), through `cloud.sh`.  It ended with every board proven after **595 CPU seconds** (145 s of
wall time), and the check found **0 violations**: every game replays to its L, L <= opt <= U at all 200 boards, and all
200 proven values are the committed chain's.

| CPU seconds | 60 | 119 | 179 | 238 | 298 | 357 | 417 | 476 | 536 | 595 |
|---|---|---|---|---|---|---|---|---|---|---|
| boards proven | 40 | 90 | 112 | 127 | 142 | 164 | 182 | 189 | 195 | 200 |

* How (the rules of the upper bound, then of the lower): by searches 83, all exhausted (69 with the search's own game,
  12 with the start incumbent, 2 with a game from a relation), none by a dual bound alone; by relations 117: repeats 79,
  the ceiling 21 (14 with the game below a proven board carried up, 5 with a search's game at its ceiling, 2 by a
  repeat's game), the prime formula 17.
* Where the time went: the boards' facts and problems 1.8 s; 89 maps, 20.1 s; 182 searches of the 89 boards searched,
  565.6 s; 87 completed (3 at their ceiling), 95 stopped by their limit, a stopped search costing a median 0.14 s beyond
  its limit (setup, start incumbent, dual bound).  111 boards were never searched (primes and repeats), 36 once, 26
  twice, up to 6 times (1475).
* The dual bound lowered U in 51 of the 95 stopped searches, by 3,833 in all (a median 74), and was a median 96 below
  the board's root bound (at most 297); it proved nothing alone.
* **Against the chain.**  The chain over the same stretch on one thread with the same build, standalone at 1301 (no
  game of 1300), took **447.5 s**: 89 searches (80 exhausted, 9 at the ceiling), 27 primes, 84 repeats.  The ledger
  spent 1.33 times that CPU, in a third of the wall time on six workers.  It searched the same 89 boards, and the
  dearest boards cost it what they cost the chain, each by an exhausted search of its own (1475 69.7 s against 51.4,
  1472 34.6 against 36.7, 1476 34.2 against 24.4, 1430 28.6 against 28.3).  The extra third: 35 s on the 9 boards the
  chain settled at its ceiling with no search, searched here before the board below them was proven; 84 s more, net, on
  the boards both searched to the end, searched here in pieces and at first without the seed and ceiling of a proven
  neighbour; and the 20 s of maps.  (The committed
  chain's one-thread re-solve of 1301..2175, with the release build of ee03872, spent 461 s of search on 1301..1500,
  `bench/wall/resolved_1301_2175.jsonl`.)
* A first dry run of the same stretch, before the map was forced, made only 6 maps: the relations gave every board both
  bounds (a ceiling carried up, a game carried across) before its map came due, and the scheduler took a board with
  bounds as mapped.  It also proved every board at the optimum, in 590 CPU seconds; each board searched now gets its own
  map first (`mapped`), as the design says, so that its order starts from its own bounds.

**Findings.**

* The ledger is correct on the stretch it was tried on: every bound brackets the optimum and every proven value is the
  chain's, with the truth hidden from the run.
* On an easy stretch it is the chain run out of order on many threads: the same searches prove the hard boards, the
  relations prove the rest (117 of 200 boards here, as the chain's repeats and primes do), and the out-of-order work
  costs a third more CPU.  The dual bound of a depth-first search helps the order (it lowers U), but at these limits
  it closes no board by itself; that would need a search that closes its shallow subtrees first.
* What it can do on 2176..3440, where the chain spent about 280 core-hours and its dearest games hours of 64 threads,
  is the open question of the cloud experiment: the boards whose own searches fit in the budget are proven, and each
  proof brings its repeats and primes (55% of the chain's games there) and the exact ceiling of the next.

**Not measured.**  The cloud experiment (2176..3440, an hour of 64 CPUs: not launched; it needs Brian's approval); a
stretch where searches do not finish; other values of `--base` and `--gap-unit`; the release build.

**Commits.**  4466311 (the dual bound), 83f6405 (the ledger and its relations), then this record.
