# The proofs of opt(n)

Part of [the prover's README](../README.md).

The prover proves opt(n) = V(n), the recorded score, for every n ≤ 2175.  `src/game.rs` proves upper bounds on opt(n)
from proven facts only:

* [the playability theorem (P2)](../../../docs/theory/playability.md#the-playability-theorem) with [the core theorem
  (P7)](../../../docs/theory/playability.md#the-core-theorem): a set of picks is playable iff dismantling the instance
  of the picks, each listing its maximal factors that are not picks, removes everything;
* the Franklín–Moniot bound, and [the parity bound (W1)](../../../docs/theory/parity-bound.md#the-parity-bound);
* opt(n) ≤ n + opt(n − 1);
* the prime formula's upper bound: a game playing a prime n plays it first, and swapping it for the largest prime
  below n gives a game of n − 1;
* [the semiprime formula (J3)](../../../docs/theory/arrival.md#the-semiprime-formula), for an odd n with two prime
  factors (relation `semiprime`, [below](#the-semiprime-formula-needs-no-search)): opt(n) = opt(n − 1) + n − x, where x
  is the smallest number other than n in the cycle that n closes with the numbers of layer 2 in U\*(n − 1);
* [the bottom-layers theorem (F4)](../../../docs/theory/frame-model.md#the-bottom-layers-theorem), in a search that
  starts from its roles (the default of `exact`; `--fix-low 0` starts from the plain root).  Every optimal set holds the
  largest prime and no other, and holds exactly U\*_2 among the sources of layer 2.  On a board whose frames of layer 2
  are closed, it holds no number of layer 2 up to n/2.  The frames of layer 2 are closed when U\*_2 spans every prime up
  to n/2, which `src/upper.rs` decides for each board from its own U\*.

A search from the roles proves that no playable set that obeys them beats V(n).  That proves opt(n) ≤ V(n), because an
optimal set beating V(n) would obey them.  Its lines say `"fix_low":1`, and the plain root's `"fix_low":0`; the
committed `data/exact_*.jsonl` are all from the plain root, from before the field, and do not rest on F4.
`tools/summarize.py exact` and `check` count the proofs that rest on it.  Its game (`src/game.rs`) knows nothing of
frames, greedy, `unzip` or promotions; U\* and descending greedy (`src/peel.rs`) enter through the roles of F4
and the checks of the conjectures.

The search:

* **Bound.**  The maximum-weight matching between numbers and their maximal factors, with forced picks matched downward
  and barred numbers only paying.  It is solved once by successive shortest paths, then re-optimized in each child from
  the parent's optimal matching and dual, each freed vertex repaired by one Hungarian step.  That made the search 160
  times faster than solving each node from scratch.
* **The parity bound** (`src/parity.rs`, `Prover::with_parity`; on in `exact` since 2026-10).  Every playable set is
  independent at both parities: its vectors over GF(2), split by the parity of Ω, are linearly independent in each part
  ([the parity vectors](../../../docs/theory/parity-bound.md#the-parity-vectors), W1).  So at a node, the heaviest set
  independent at both parities together with the forced picks, and outside the barred numbers, bounds every completion.
  * It is computed by shortest augmenting paths, a child starting from its parent's set and weight split.  The child
    takes the elimination over GF(2) and the circuits that its parent's solve left, less what its own roles change,
    rather than building them again; they are the same, so the solve and its bound are too.
  * It is reported only with a weight split that satisfies the local optimality conditions of both parities, checked
    explicitly, so the bound rests on the theorem and weak duality, not on the algorithm.
  * It cuts the node, bars the free numbers that depend on the forced picks, and forces the members whose barring alone
    brings it to the target.  The node branches on its heaviest set's conflicts instead of the matching's.
  * Its gap is about a tenth of the Franklín–Moniot bound's: 130 points against 1,200 at n = 1000-1300.
* **Branching.**  If the bound's picks are playable, they are a game worth the bound.  Otherwise a minimal unplayable
  subset, found by a deletion filter with peel, must lose a member, and the children partition on which.
* **Forced picks, cuts and nogoods.**  A pick is forced when barring it alone already brings the bound to the target,
  which the dual shows.  The *conflict-packing cut* takes the nogoods that the matching violates and whose members lie
  far apart in the graph: each must be broken, so their dual costs add.  Every minimal unplayable set found is a
  nogood, propagated and reused.
* **A Lagrangian bound over the nogoods.**  A playable set picks at most |K| − 1 members of each nogood K.  So a second
  matching, with each pick's weight lowered by the multipliers of the unbroken nogoods holding it, bounds the search
  too; the multipliers are fitted by subgradient steps at the search's root, then fixed.  It runs alongside the first,
  repaired incrementally the same way, and cut the tree 3.3-fold at n = 793.  Beside the parity bound it cut nothing
  at 990, so `exact` uses it only without the parity bound.
* **Pausing.**  A search still running at the end of a run saves its open subtrees and nogoods, and resumes in the
  next run, depth first.  So a game can take any number of ten-minute runs.

The target is the recorded score V(n).  The search proves that no playable set beats it, or, from the roles of F4, none
that obeys them, which is as good.  The recorded game replays by the rules to V(n), so opt(n) = V(n).

Every rule above is tested against brute force or against the recorded score.  The search must still reach the
committed chain's score from one below it, with each rule on, on every seventh game to 420, and from the roles of F4 on
every third game of 187-700 that needs a search.  The parity bound's intersection is checked against enumeration on
random pairs of matroids.

**From the roles of F4** (the default since 2026-10; the committed proofs below are from the plain root).  Over
the games of 2..999, the 391 searches took 588,026 nodes and 357 s from the roles, against 614,900 nodes and 712 s from
the plain root (`data/exact_parity_2_1300.jsonl`).  The nodes are within 5%, and the time of a node is halved, since the
parity solves have the primes and the numbers of layer 2 settled.  Every game of 2..2175 has the recorded game obey the
roles, and the roles' parity bound is at least its score; `exact --budget 1` checks both for every game before any
search.  The frames of layer 2 are closed on every board from 187 on, and on most below it.  The roles bar the numbers
of layer 2 up to n/2 where the frames are closed, and leave them free on the 80 boards of 2..1300 where they are not,
all of them at most 186 (`upper::tests`).

## The semiprime formula needs no search

An odd game n with two prime factors, counted with multiplicity, is proven from the game before it, with no search, when
V(n) = V(n − 1) + n − x and V(n − 1) is proven.  [The semiprime formula
(J3)](../../../docs/theory/arrival.md#the-semiprime-formula) says that opt(n) = opt(n − 1) + n − x where n closes a
cycle with the numbers of layer 2 in U\*(n − 1), and x is the smallest number of the cycle other than n.  The prover
needs only the bound from above: opt(n) ≤ opt(n − 1) + n − x = V(n), and the recorded game, which replays by the rules,
gives opt(n) ≥ V(n).

The cycle lies in a graph with the primes up to n/2 and one ground vertex as its vertices, where pq is the edge p–q and
p² the edge p–ground.  The numbers of layer 2 in U\*(n − 1) form a forest in it, and n, as one more edge, closes a cycle
when the forest already joins its ends; x is the smallest number on that path.  Game 39 = 3·13 has U\*(38) with the
numbers of layer 2 {21, 22, 25, 26, 33, 34, 35, 38}, and its edge 3–13 closes the cycle 3–11–2–13 with 33, 22 and 26.
So x = 22, and V(39) = V(38) + 39 − 22 = 503.

The prover uses it as one more relation, beside `chain` and `prime`:

* `src/semiprime.rs` finds x from the board's own U\*(n − 1), descending greedy (`src/peel.rs`), not from the recorded
  game.  `relation` in `src/game.rs` asks for it only for an odd n with two prime factors whose score is not at its
  ceiling, and names the game `semiprime` when V(n) = V(n − 1) + n − x.
* A score off the formula stops the program, naming n.  Above it, the recorded scores contradict a theorem; below it,
  the recorded game of n is not optimal.  Where V(n − 1) is not recorded, the relation is `none`.
* `exact` does not search a `semiprime` game (unless `--all 1`) and writes `"relation":"semiprime"`.  `upper` writes
  `"needs":0` for it, and `optsets` derives its optimal sets from those of n − 1 by exchanging x for n (part 3 of J3).
* `tools/summarize.py` counts the game as proven when n − 1 is, in `exact` and `check`, so that
  `../tools/cloud/prover.sh --verify` agrees; `order` puts it with the games that need no search, last.

The hypothesis of J3 holds for every odd n from 189 on with two prime factors.  Below that, some such games close no
cycle, such as 21 and 187; they meet another relation or are searched.

The tests check the formula against the prover's search from nothing on every odd n to 300 with two prime factors: the
relation is `semiprime` exactly where n closes a cycle, and then opt(n) − opt(n − 1) = n − x.  A rule that takes the
largest number of the path for x instead of the smallest is caught on every such game, so that the test can see the
failure it guards against.  On the committed chain to 1300, every such n that closes a cycle meets the formula.

*Details.*  `exact --from 1301 --to 2175 --budget 1` stops every search at once, so it only classifies.  It gives, for
the 875 games the cloud run proved:

| relation | games |
|---|---|
| `chain` | 270 |
| `prime` | 115 |
| `semiprime` | 176 |
| `none`, which needs a search | 314 |

The cloud run (`data/exact_parity_1301_2175.jsonl`) was made before the formula and searched 490 of these games, the 314
and the 176.  Those 176 took 84.0 of its 226.0 thread-hours.  Its file is a record and stays as it was; the lines of the
176 say `"relation":"none"`.  Put together with the 561 lines of the classification that rest on a relation, it passes
`tools/summarize.py check 1301 2175`, which counts the 176 as proven by `semiprime`.

## With the Franklín–Moniot bound (to 989)

`data/exact_2_1231.jsonl`, where `relation: chain` marks a game at its ceiling, V(n) = n + V(n − 1), and
`relation: prime` one given by the prime formula:

| | games |
|---|---|
| Franklín–Moniot bound already equals V(n) | 45 |
| proven by the search | 487 |
| `chain`, at its ceiling V(n) = n + V(n−1), predecessor proven | 321 |
| `prime`, by the prime formula, predecessor proven | 156 |
| **every n in 2 .. 989**, and 999 .. 1019 | **1009** |

The searches took 10.0 hours of CPU in all.  The cost is far from monotone in n, and follows the Franklín–Moniot
bound's gap: 140–510 points at n = 500–699, and 540–1180 from 800 on.  The hardest games are those whose score it
overrates most: 798–804 at 7–12 million nodes, and 987–989 at 19–53 million.  Games 990, 992, 993, 995 and 996 took
50–70 million nodes each, and 1020 and 1023–1027 several million, without finishing; their searches are saved in
`data/checkpoints/exact_open_searches.tar.gz`.

## With the parity bound (to 1300)

`data/exact_parity_990_1300.jsonl` continues that file: the open searches resumed with the parity bound, then every
game from 1028 to 1300, then 999–1019 again.  The open searches finished in 4,000 to 215,000 more nodes each (17 to
595 seconds; 990: 89,639 nodes after its 70.5 million; 992: 214,329 after 65.2 million), and with them 991, 994, 997
and 998 by the ceiling and the prime formula.  With the two files (`tools/summarize.py exact data/exact_2_1231.jsonl
data/exact_parity_990_1300.jsonl`):

| | games |
|---|---|
| Franklín–Moniot bound already equals V(n) | 45 |
| proven by a search (to 989 with the Franklín–Moniot bound alone, from 990 with the parity bound) | 649 |
| `chain`, at its ceiling V(n) = n + V(n−1), predecessor proven | 407 |
| `prime`, by the prime formula, predecessor proven | 198 |
| **every n in 2 .. 1300**, the committed chain to 1300 | **1299** |

The parity searches of this file took 2.7 hours of thread time on three threads; the warm starts came in for its last
96 games.  The hardest was 1287, at 146,855 nodes and 279 seconds.  So the frame model's answer is opt(n) for every
game n ≤ 1300 of the committed chain, including the solver's own results past the Java's 1000.

`data/exact_parity_2_1300.jsonl` proves every game again from scratch with the parity bound, with no resumed search
and no proof by the Franklín–Moniot bound alone (the final build, two threads).  It is a second, self-standing proof of
every n ≤ 1300 by a different bound, and the cost curve below.  And no rule cuts an optimal game: for every game that
needs a search, the same search with the target one below V(n) finds a set worth V(n), which replays by the rules
(`examples/paritycheck.rs`; all 665 such games to 1300, `data/paritycheck_2_1300.jsonl`, 31 minutes).

## What a proof costs, and how far it reaches

A proof's cost follows the root's parity gap, the root's parity bound less V(n), not n.  `tools/summarize.py cost
data/exact_parity_2_1300.jsonl` gives the 557 games that need a search, proven from scratch with the parity bound in
1.17 thread-hours (two threads; about 1.6 ms a node from 990 on, the warm starts included):

| n | searches | the root's parity gap (mean) | median nodes | most nodes | seconds, all searches |
|---|---|---|---|---|---|
| 2–499 | 127 | 3–24 | ≤ 21 | 83 | 2 |
| 500–799 | 154 | 44–78 | 74–402 | 2,673 | 44 |
| 800–899 | 57 | 86 | 382 | 5,474 | 55 |
| 900–999 | 53 | 164 | 2,406 | 68,260 (996) | 611 |
| 1000–1099 | 54 | 120 | 2,358 | 91,928 (1023) | 974 |
| 1100–1199 | 56 | 111 | 789 | 41,153 (1199) | 264 |
| 1200–1300 | 56 | 169 | 18,247 | 146,855 (1287) | 2,276 |

The cost follows the root's parity gap as the frame solver's follows its root gap.  Over 900–1300 a search's nodes grow
tenfold per 84 points of gap (1.19 ± 0.05 decades per 100 points, R² 0.76).  The gap grows about 14 points per 100 in n,
with swings of ±50 between bands; that is the measurement of [matroid intersection
bounds](https://github.com/bvchess/taxman-notebook/blob/main/search/matroid-intersection-bounds-2026-10/README.md),
which these roots reproduce.  Against n, log10 nodes grows 0.16 ± 0.04 and log10 seconds 0.23 ± 0.04 per 100 in n over
900–1300 (R² 0.06 and 0.11: the gap's swings dominate), and 0.33 and 0.34 from n = 78 on.  The search with the
Franklín–Moniot bound alone had spent 50–70 million nodes on each of 990–996 without finishing; the parity bound proves
them from scratch in 44,000–68,000.

The prover's proofs reach 2175, where the committed chain (`../../results/optimal.jsonl.gz`) ended when they were
made; the next subsection is the run that made them.

*Details.*  An extrapolation from the bands above, 0.23 decades per 100 in n, made before that run, gave about 57 hours
of thread time to 2000, and about 3 hours for the hardest proofs.  It was low: 1301–1999 took 164 hours, and the hardest
proof, of 1475, 4.9.

## To 2175, on a cloud machine

`data/exact_parity_1301_2175.jsonl` is `exact` over the 875 games of 1301–2175, each proven from scratch with the
parity bound.  It ran on one `c8g.4xlarge` (Graviton4, 16 vCPUs, one game a thread) on spot in us-east-1b, run by
`../tools/cloud/prover.sh` ([solver/docs/runbook/prover.md](../../docs/runbook/prover.md)), with
the build of commit 4db7a14.  It ran from 2026-10-14 23:29 to 2026-10-15 15:22 UTC: 190 passes of 300 s, every one
ending with status 0, with no interruption; 15.9 hours of run and 16.0 instance-hours at $0.2402, **$3.84**.

The check, `../tools/cloud/prover.sh --verify 1301 2175`, run on the instance and again at home on this file, prints
`OK: opt(n) is proven equal to the recorded score for every n in 2..2175`: every game has a line with the recorded
score, and no search found a set that beats it.  With `tools/summarize.py exact data/exact_parity_2_1300.jsonl
data/exact_parity_1301_2175.jsonl`:

| | games |
|---|---|
| Franklín–Moniot bound already equals V(n) | 45 |
| parity bound at the root already equals V(n) | 199 |
| proven by a search | 1,047 (557 to 1300, 490 from 1301) |
| `chain`, at its ceiling V(n) = n + V(n−1), predecessor proven | 607 (337 and 270) |
| `prime`, by the prime formula, predecessor proven | 276 (161 and 115) |
| **every n in 2 .. 2175**, the committed chain to there | **2174** |

Of the 875 games of 1301–2175, 490 needed a search, and every one ran to the end; the root's parity bound exceeds the
recorded score in each, by 125 to 767, with a mean of 384.  The other 385 are at their ceiling or meet the prime
formula, and took 83 seconds in all.  The searches took 309.2 million nodes and **226.0 thread-hours** (search
seconds), 2.63 ms a node on average.  `tools/summarize.py cost --band 100 data/exact_parity_1301_2175.jsonl` gives the
cost by band, with the dearest game of each in brackets:

| n | searches | the root's parity gap (mean) | median nodes | most nodes | nodes (millions) | thread-hours | ms a node |
|---|---|---|---|---|---|---|---|
| 1301–1399 | 60 | 297 | 53,346 | 2,639,109 (1393) | 31.6 | 22.7 | 2.58 |
| 1400–1499 | 58 | 447 | 730,285 | 6,328,930 (1475) | 67.2 | 47.3 | 2.54 |
| 1500–1599 | 58 | 458 | 335,164 | 2,889,122 (1520) | 30.9 | 19.9 | 2.32 |
| 1600–1699 | 53 | 340 | 132,489 | 1,879,179 (1696) | 13.1 | 7.2 | 1.99 |
| 1700–1799 | 55 | 313 | 110,480 | 633,344 (1700) | 9.8 | 5.6 | 2.03 |
| 1800–1899 | 54 | 433 | 727,936 | 5,333,481 (1849) | 82.7 | 52.7 | 2.29 |
| 1900–1999 | 52 | 294 | 92,155 | 1,898,240 (1995) | 11.7 | 9.1 | 2.79 |
| 2000–2099 | 56 | 374 | 176,716 | 4,136,842 (2064) | 23.5 | 20.1 | 3.08 |
| 2100–2175 | 44 | 527 | 262,534 | 6,868,466 (2172) | 38.7 | 41.5 | 3.86 |

What the cost follows is the root's parity gap, not n.  Over the 490 searches, log10 nodes grows 0.477 ± 0.017 per 100
points of gap: tenfold per 210 (R² 0.61, residual sd 0.45 decades), where 900–1300 had tenfold per 84.  Against n it
grows −0.009 ± 0.013 per 100 in n (R² 0.00; log10 seconds 0.007 ± 0.013): the bands' means swing between 294 and 527
of gap with no steady rise.

*Details.*  The cost of a node does not grow steadily with n.  It is 2.0–2.6 ms through 1899: 2.58 at 1301–1399 and
1.99 at 1600–1699, against 2.12 ms for 1300 alone on one Graviton4 core (the calibration's) and 1.7 on two quiet laptop
threads at 1200–1300.  Then it is 2.8, 3.1 and 3.9 ms by the hundreds to 2175, and 4.10 to 4.15 for 2170–2173.  The
dear games are many: 77 searches took a million nodes or more and 57 an hour or more, the ten dearest 24% of the
thread-hours and the fifty dearest 59%.

The run took 1.5 times the nodes projected before it, just above the projected range, and 1.6 times the thread-hours,
inside it.  [The notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)
has the run's account, the projection game by game, and what the run says about the projection's model.

## The upper part, game by game

Past the prover's proofs, the frame solver's answer for a game that rests on its search is opt(n) exactly when U holds
at n: some optimal set plays U\*_{≥3}, the members of U\* with three or more prime factors
([statement U](../../../docs/theory/exactness.md#statement-u)).  `src/bin/upper.rs` (`src/upper.rs`) asks the parity
bound which of those members every set beating the recorded score V must play.

* It fixes layers 1 and 2 as [F4](../../../docs/theory/frame-model.md#the-bottom-layers-theorem) says every optimal set
  has them: the largest prime and no other; U\*_2 among the sources of layer 2; and, where U\*_2 spans every prime up to
  n/2, which it decides for each board, no number of layer 2 up to n/2.
* It bars one member u at a time, and calls u *certified* when the bound is then at most V.  Every set that beats V
  and obeys F4 then plays u, so if opt(n) > V, every optimal set plays every certified member.
* The members it does not certify are *open*, with the bound's margin over V.

Only the games that rest on the frame model need it: those neither at the ceiling nor given by the prime formula or the
semiprime formula, whose lines say `"needs":1`.  The four files below were made before the semiprime formula, so their
lines for the games it proves say `"needs":1` and `"relation":"none"`.  Certified members can be fixed in a search that leaves the open ones optional, the integrated
search of the theory track, which the certificates of U carry out
([solver/docs/design/certificates.md](../../docs/design/certificates.md)).  That is why the files are kept: U\* less a
game's open members is that search's fixed set.

The tests check the certificates against every playable set that obeys F4, on boards to 60 (1,403 checks):

* the certificates are computed again at each claim T of V − 1, V − 2 and V − 5, and checked against every such set
  worth more than T; a member certified against V says nothing of the sets worth V − 4;
* a deliberately wrong rule, which certifies a member also when its bound is at most T + 10, is caught by the same
  check (12 false certifications over the boards and claims, 15 (set, member) misses on 12 boards), so that the test
  can see the failure it guards against;
* F4's roles are checked against every optimal set there and every recorded game to 1300.

Four files, each made on one thread of the laptop:

| file | games | needing it | time | what for |
|---|---|---|---|---|
| `data/upper_2_1300.jsonl` | the table's games 2..1300, 1,299 | 665 (417 of them with nothing open; 1.0% of the members open) | 65 s | the tests of `taxman certify` |
| `data/upper_1301_2175.jsonl` | the committed chain's games 1301–2175, 875 | 490 | 300 s | |
| `data/upper_2176_3314.jsonl` | the copy of the cloud chain's results taken on 2026-10-15, games that the committed chain also holds, which [the gap past 2175](https://github.com/bvchess/taxman-notebook/blob/main/search/the-gap-past-2175-2026-10/README.md) measured; 1,139 | 635 | 1,272 s, 4.1 s for the slowest game | |
| `data/upper_3315_3440.jsonl` | the rest of the committed chain as it stood then, 126 | 73, 3424, 3425 and 3440 among them | 277 s, 4.3 s for the slowest game | the input of the certificates of 2176-3440 (`../tools/certify/`) |

By band, the open members and the margins of each game (the share is of all members):

| n | games | the root's gap (mean) | members of U\*_{≥3} | open (least, median, most) | open | margin (median, largest) |
|---|---|---|---|---|---|---|
| 1301-1399 | 60 | 297 | 371-400 | 1, 12, 47 | 4.5% | 86, 322 |
| 1400-1499 | 58 | 447 | 401-432 | 19, 38, 59 | 9.2% | 110, 388 |
| 1500-1599 | 58 | 458 | 433-467 | 4, 42, 65 | 8.8% | 127, 432 |
| 1600-1699 | 53 | 340 | 467-496 | 14, 32, 58 | 6.5% | 77, 398 |
| 1700-1799 | 55 | 313 | 497-525 | 5, 33, 62 | 5.9% | 66, 401 |
| 1800-1899 | 54 | 433 | 525-556 | 11, 77, 86 | 11.6% | 155, 480 |
| 1900-1999 | 52 | 294 | 558-587 | 9, 22, 68 | 4.4% | 68, 422 |
| 2000-2099 | 56 | 374 | 587-619 | 11, 43, 69 | 6.3% | 102, 530 |
| 2100-2175 | 44 | 527 | 620-640 | 30, 68, 102 | 10.3% | 144, 691 |
| 2176-2299 | 73 | 724 | 641-681 | 59, 89, 152 | 14.1% | 221, 910 |
| 2300-2399 | 56 | 936 | 681-710 | 88, 112, 317 | 20.8% | 298, 1270 |
| 2400-2499 | 60 | 745 | 711-741 | 62, 109, 171 | 14.8% | 247, 1003 |
| 2500-2599 | 58 | 793 | 741-774 | 78, 116, 163 | 14.9% | 262, 1004 |
| 2600-2699 | 55 | 515 | 774-807 | 15, 69, 105 | 7.5% | 141, 645 |
| 2700-2799 | 49 | 817 | 808-844 | 113, 129, 142 | 15.8% | 289, 796 |
| 2800-2899 | 53 | 691 | 845-871 | 97, 110, 127 | 13.0% | 215, 708 |
| 2900-2999 | 48 | 660 | 871-899 | 59, 115, 135 | 11.7% | 205, 774 |
| 3000-3099 | 59 | 915 | 899-925 | 120, 134, 194 | 15.3% | 336, 1163 |
| 3100-3199 | 58 | 1118 | 925-959 | 133, 170, 255 | 18.6% | 435, 1312 |
| 3200-3314 | 66 | 1030 | 959-993 | 113, 150, 246 | 16.1% | 372, 1355 |
| 3315-3440 | 73 | 1259 | 994-1035 | 172, 197, 271 | 19.9% | 531, 1467 |

* **No game past 1300 is certified whole.**  Every one leaves members open: from 1 (1305) to 102 (2172) up to 2175, and
  from 15
  (2610) to 317 (2346) past it.  In all, 7.5% of the members are open to 2175 (18,596 of 247,482), 14.9% on 2176-3314
  (77,195 of 518,992), and 19.9% on 3315-3440 (14,704 of 73,968, 172 to 271 a game, the root's gap 1,103 to 1,555).
* **The open share follows the root's gap, not n.**  It grows 3.1 points per 100 of gap to 2175 and 2.0 past it (R²
  0.73, 0.73 and 0.91 in the three files), and it falls back with the gap (2600-2699).  The root's gap is the one the
  frame solver's search costs by: with layers 1 and 2 fixed, it equals the solver's GF(2) gap at the root on all 400
  games past 2175 that both measured
  ([taxman-notebook/search/the-gap-past-2175-2026-10/gaps_chain.jsonl](https://github.com/bvchess/taxman-notebook/blob/main/search/the-gap-past-2175-2026-10/gaps_chain.jsonl)).
  The margins grow with it, and the largest is close to the gap.
* **The root's heaviest set plays all of U\*_{≥3} in every one of the 1,198 games that need it**, as at every root
  measured before; no game is flagged.
* **The open members are mostly of layer 3** (52% to 2175 and 58% past it), then of layer 5 (21%, 27%) and 4 (19%, 9%),
  and the rest in layers 6 to 11.
