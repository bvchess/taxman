# The proofs of opt(n)

Part of [the prover's README](../README.md).

The prover proves opt(n) = V(n), the recorded score, for every n ≤ 2531, in one run of one build and a one-game step ([the
run](#the-run-of-2-to-2530)).  `src/game.rs` proves upper bounds on opt(n) from proven facts only:

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
optimal set beating V(n) would obey them.  Its lines say `"fix_low":1`, and the plain root's `"fix_low":0`.  All 795
searches of `data/exact_parity_2_2531.jsonl` start from the roles, so its proofs rest on F4 besides the other theorems;
`data/exact_2_1231.jsonl` is from the plain root, from before the field, and does not.  `tools/summarize.py exact` and
`check` count the proofs that rest on it.  Its game (`src/game.rs`) knows nothing of frames, greedy, `unzip` or
promotions; U\* and descending greedy (`src/peel.rs`) enter through the roles of F4 and the checks of the conjectures.

## The run of 2 to 2530

**One run of one build proved every game of 2..2530, and game 2531 followed.**  `data/exact_parity_2_2531.jsonl` holds
the run's 2,529 lines and one for 2531, one a game.  The command was `../tools/cloud/prover.sh 2 2530` ([the
runbook](../../docs/runbook/prover.md#on-a-machine-of-your-own)), from commit 9015f79, on the 88 threads of one machine
(two Xeon E5-2699 v4 processors, 756 GiB) at idle priority, in passes of 3600 s.  It started on 2026-11-06 at 05:02 PST
and ended "complete" on 2026-11-07 at 00:21 PST: 19.3 hours and 20 passes, with no interruption and no error.  It cost
no money, only its hours.  [The notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/the-model-free-proofs-of-2-2530-on-one-machine-2026-11/README.md)
has the account.  Game 2531 is prime, so it was added afterward by a run of `exact --list 2531` alone: the prime step,
no search, 0.3 s, from the proven game 2530.

The check, `tools/summarize.py check 2 2531 data/exact_parity_2_2531.jsonl`, prints `OK: opt(n) is proven equal to the
recorded score for every n in 2..2531`: every game has a line with the recorded score, and no search found a set that beats it.  Every
game's recorded moves obey the roles of F4, and the roles' parity bound is at least its score, which `exact` asserts
before any search.  With `tools/summarize.py exact data/exact_parity_2_2531.jsonl`:

| | games |
|---|---|
| Franklín–Moniot bound already equals V(n) | 45 |
| parity bound at the root already equals V(n) | 199 |
| proven by a search | 795 |
| `chain`, at its ceiling V(n) = n + V(n−1), predecessor proven | 712 |
| `prime`, by the prime formula, predecessor proven | 320 |
| `semiprime`, by the semiprime formula, predecessor proven | 459 |
| **every n in 2 .. 2531** | **2,530** |

The 795 searches took 1,957.8 million nodes and **1,698.5 thread-hours** (search seconds), 3.12 ms a node on average.
The other 1,735 games took 616 seconds in all.  The machine was not otherwise idle: from about 09:00 PST on the first
day, experiments of the frame solver ran beside the run on up to 8 threads at nice 19, so the seconds and the
milliseconds a node below run a little long.  The nodes of a search on one thread do not depend on that, and those of a
search on several threads vary a little with the timing ([a game on several
threads](#a-game-on-several-threads)).  `tools/summarize.py cost --band 100 data/exact_parity_2_2531.jsonl` gives the
cost by band, with the dearest game of each in brackets:

| n | searches | the root's parity gap (mean) | median nodes | most nodes | nodes | thread-hours | ms a node |
|---|---|---|---|---|---|---|---|
| 2–899 | 208 | 50 | 13 | 1,286 (800) | 19,900 | < 0.01 | 1.37 |
| 900–999 | 32 | 166 | 390 | 9,780 (990) | 44,265 | 0.02 | 1.22 |
| 1000–1099 | 37 | 123 | 438 | 12,829 (1023) | 65,312 | 0.02 | 1.26 |
| 1100–1199 | 33 | 111 | 127 | 3,729 (1197) | 12,320 | 0.01 | 1.62 |
| 1200–1299 | 35 | 173 | 3,162 | 14,066 (1287) | 116,983 | 0.04 | 1.32 |
| 1300–1399 | 36 | 277 | 6,744 | 217,589 (1380) | 1,486,863 | 0.67 | 1.63 |
| 1400–1499 | 43 | 450 | 66,885 | 847,263 (1475) | 4,866,369 | 2.53 | 1.87 |
| 1500–1599 | 39 | 458 | 24,510 | 365,270 (1520) | 1,975,263 | 1.00 | 1.82 |
| 1600–1699 | 34 | 345 | 9,958 | 74,165 (1696) | 513,421 | 0.17 | 1.19 |
| 1700–1799 | 33 | 306 | 7,037 | 26,521 (1720) | 342,081 | 0.11 | 1.11 |
| 1800–1899 | 33 | 423 | 41,534 | 177,450 (1859) | 1,757,520 | 0.54 | 1.11 |
| 1900–1999 | 28 | 301 | 5,490 | 94,997 (1995) | 457,430 | 0.22 | 1.73 |
| 2000–2099 | 41 | 380 | 10,141 | 99,062 (2000) | 640,090 | 0.31 | 1.77 |
| 2100–2199 | 39 | 570 | 26,271 | 1,218,046 (2184) | 3,869,567 | 2.62 | 2.44 |
| 2200–2299 | 37 | 727 | 184,120 | 13,021,992 (2288) | 35,043,723 | 43.1 | 4.43 |
| 2300–2399 | 39 | 976 | 10,824,087 | 297,725,337 (2346) | 1,523,252,740 | 1,410.3 | 3.33 |
| 2400–2499 | 37 | 742 | 482,280 | 117,479,896 (2499) | 241,256,583 | 147.8 | 2.21 |
| 2500–2530 | 11 | 901 | 6,362,874 | 60,920,815 (2500) | 142,069,911 | 89.0 | 2.25 |

**The cost is in the games past 2175.**  They took 99.6% of the thread-hours, and 2300–2399 alone 83%.
The dearest five, 2346, 2376, 2379, 2340 and 2375, took 44% (295.2, 129.1, 126.5, 113.0 and 80.1 thread-hours), the
ten dearest 65% and the fifty dearest 98%.  71 searches took a million nodes or more, 27 ten million or more and 5 a
hundred million or more; 63 took a thread-hour or more, 23 ten or more and 4 a hundred or more.  The dearest, 2346
(root gap 1,346), took 297,725,337 nodes and 1,062,725 thread-seconds on up to 88 threads, over the whole run: 19.3
hours.

**The cost follows the root's parity gap.**  The gap is the root's parity bound less V(n).  From n = 900 on, log10 nodes
grows 0.498 ± 0.008 per 100 points of gap (tenfold per 201 points, R² 0.88), and 0.217 ± 0.009 per 100 in n (R² 0.52),
where log10 seconds grows 0.225 ± 0.008 per 100 in n: a search costs 1.7 times as much per 100 in n.  The gap's mean
swings between 277 and 458 from 1300 to 2099 with no steady rise, then rises: 570, 727 and 976 over 2100–2399.  The
tenfold per 210 points of the cloud run of 2026-10 (490 searches, R² 0.61) is the slope that `POINTS_PER_DECADE` in
`src/game.rs` weighs a conflict by.

**The time of a node** was 1.1 to 1.9 ms from 900 through 2099, then 2.4, 4.4, 3.3 and 2.2 ms by the hundreds to 2499,
and 2.3 at 2500–2530.  In 2200–2530 the games that ran on one thread took 2.49 ms a node, those on 2 to 8 threads 2.68,
those on 9 to 87 threads 3.24, and 2346 on 88 threads 3.57.  These are different games, so the rise is not a clean
effect of the threads.

**The next 100 games.**  39 of the 100 games of 2531–2630 need a search, with root gaps of 341 to 942 (mean 643, against
742 at 2400–2499).  The fit on the gap, made on the searches of gap below 1,000, prices them at about 90 million nodes:
55 to 90 thread-hours at 2.2 to 3.6 ms a node, an hour on 88 threads.  On the run's own games of gap below 1,000 the
same fit overprices by 1.4 times.  It is an extrapolation of one slope, and the entry gives the other fits.

## The search

A search is a branch and bound over the picks: a bound on what the picks still free can add, a playability check of the
heaviest set, and a branch on a conflict found there.

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
    explicitly, so the bound rests on the theorem and weak duality, not on the algorithm.  The check reduces every
    circuit, and confirms every number it bars as dependent, afresh against the elimination, not trusting the circuits
    the search kept or took from the parent.
  * It cuts the node, bars the free numbers that depend on the forced picks, and forces the members whose barring alone
    brings it to the target.  The node branches on its heaviest set's conflicts instead of the matching's.
  * It also bars a number outside its heaviest set whose forcing alone brings it to the target: no playable set that
    beats the target holds that number.  The same weight split bounds the forced number by one exchange in each
    parity, the number's weight less the lightest member of its circuit in the set (`Meet::forcing_cut`), so the
    test costs no further solve.
  * Its gap is about a tenth of the Franklín–Moniot bound's: 130 points against 1,200 at n = 1000-1300.
* **Branching.**  If the bound's picks are playable, they are a game worth the bound.  Otherwise a minimal unplayable
  subset, found by a deletion filter with peel, must lose a member, and the children partition on which.
  * The node learns, as nogoods, conflicts of its set that share no free member with one another, up to four: the
    nogoods inside the set first, then the conflicts of what is left of the set.  The filter leaves out free members
    before forced ones, so a conflict holds as many forced members as it can and has fewer children.
  * It branches on the conflict with the smallest expected tree.  Each child bars one free member, whose bound with it
    barred the parity solve gives, and a search grows tenfold per 210 points of its gap, so a conflict weighs the sum
    of 10^(g / 210) over its free members, g the bound with the member barred less the target.
* **Forced picks, cuts and nogoods.**  A pick is forced when barring it alone already brings the bound to the target,
  which the dual shows.  The *conflict-packing cut* takes the nogoods that the matching violates and whose members lie
  far apart in the graph: each must be broken, so their dual costs add.  Every minimal unplayable set found is a
  nogood, propagated and reused.  A long search holds tens of thousands of them, so a node reads only what it changes
  of the table (`src/nogoods.rs`).  Each nogood watches two members that are not forced, as a SAT solver's clause
  does, and a node looks at the nogoods that watch the numbers it forces.  The table also keeps, for each nogood, how
  many members lie outside the last set asked about, so the nogoods inside a set come from the few numbers that left or
  entered it.
* **A Lagrangian bound over the nogoods.**  A playable set picks at most |K| − 1 members of each nogood K.  So a second
  matching, with each pick's weight lowered by the multipliers of the unbroken nogoods holding it, bounds the search
  too; the multipliers are fitted by subgradient steps at the search's root, then fixed.  It runs alongside the first,
  repaired incrementally the same way, and cut the tree 3.3-fold at n = 793.  Beside the parity bound it cut nothing
  at 990, so `exact` uses it only without the parity bound.
* **Pausing.**  A search still running at the end of a run saves its open subtrees and nogoods, and resumes in the
  next run, depth first.  So a game can take any number of runs.
* **Threads.**  A game's search can run on several threads (`src/team.rs`), each searching open subtrees depth first
  with a bound and a nogood table of its own.  A thread with nothing to do asks the thread that holds the largest open
  subtree, by the expected tree of its gap, which gives it away and never searches it.  A pause saves the open subtrees
  of every thread in the game's one checkpoint.  The game is proven only once its threads have proven every subtree
  it ever had ([A game on several threads](#a-game-on-several-threads)).

The target is the recorded score V(n).  The search proves that no playable set beats it, or, from the roles of F4, none
that obeys them, which is as good.  The recorded moves replay by the rules to V(n), so opt(n) = V(n).

Every rule above is tested against brute force or against the recorded score.  The search must still reach the
committed chain's score from one below it, with each rule on, on every seventh game to 420, and from the roles of F4 on
every third game of 187-700 that needs a search.  The parity bound's intersection is checked against enumeration on
random pairs of matroids.

**From the roles of F4.**  Over the games of 2..999, the 391 searches took 588,026 nodes and 357 s from the roles,
against 614,900 nodes and 712 s from the plain root ([the notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/theorem-b-by-default-in-the-prover-and-the-hunts-probes-2026-10/README.md)).
The nodes are within 5%, and the time of a node is halved, since the parity solves have the primes and the numbers of
layer 2 settled.  The frames of layer 2 are closed on every board from 187 on, and on most below it.  The roles bar the
numbers of layer 2 up to n/2 where the frames are closed, and leave them free on the 80 boards of 2..1300 where they
are not, all of them at most 186 (`upper::tests`).

## The semiprime formula needs no search

An odd game n with two prime factors, counted with multiplicity, is proven from the game before it, with no search, when
V(n) = V(n − 1) + n − x and V(n − 1) is proven.  [The semiprime formula
(J3)](../../../docs/theory/arrival.md#the-semiprime-formula) says that opt(n) = opt(n − 1) + n − x where n closes a
cycle with the numbers of layer 2 in U\*(n − 1), and x is the smallest number of the cycle other than n.  The prover
needs only the bound from above: opt(n) ≤ opt(n − 1) + n − x = V(n), and the recorded moves, which replay by the rules,
gives opt(n) ≥ V(n).

The cycle lies in a graph with the primes up to n/2 and one ground vertex as its vertices, where pq is the edge p–q and
p² the edge p–ground.  The numbers of layer 2 in U\*(n − 1) form a forest in it, and n, as one more edge, closes a cycle
when the forest already joins its ends; x is the smallest number on that path.  Game 39 = 3·13 has U\*(38) with the
numbers of layer 2 {21, 22, 25, 26, 33, 34, 35, 38}, and its edge 3–13 closes the cycle 3–11–2–13 with 33, 22 and 26.
So x = 22, and V(39) = V(38) + 39 − 22 = 503.

The prover uses it as one more relation, beside `chain` and `prime`:

* `src/semiprime.rs` finds x from the board's own U\*(n − 1), descending greedy (`src/peel.rs`), not from the recorded
  moves.  `relation` in `src/game.rs` asks for it only for an odd n with two prime factors whose score is not at its
  ceiling, and names the game `semiprime` when V(n) = V(n − 1) + n − x.
* A score off the formula stops the program, naming n.  Above it, the recorded scores contradict a theorem; below it,
  the recorded moves for n are not optimal.  Where V(n − 1) is not recorded, the relation is `none`.
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

*Details.*  Of the 875 games of 1301–2175, the formula settles 176 with no search, and `chain` and `prime` 270 and 115,
leaving 314 to search; of the 355 games of 2176–2530 it settles 72, and `chain` and `prime` 105 and 43, leaving 135.
The cloud run of 2026-10, made before the formula, had searched the 176, and they took 84.0 of its 226.0 thread-hours.

## Earlier runs

The runs before the one above proved shorter ranges with the builds of their day, and the notebook holds their accounts.
The files of the last two are retired, since one run of one build redid them.

**With the Franklín–Moniot bound (to 989).**  `data/exact_2_1231.jsonl`, where `relation: chain` marks a game at its
ceiling, V(n) = n + V(n − 1), and `relation: prime` one given by the prime formula:

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

**With the parity bound (to 1300).**  The parity bound finished those open searches in 4,000 to 215,000 more nodes each
([the notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/the-parity-bound-in-the-model-free-proofs-2026-10/README.md)),
and proved every game to 1300 twice.  The run of 2..2530 proves them from scratch, from the roles: 990, 992 and 996 in
9,780, 8,850 and 8,741 nodes, and 993 and 995 by the semiprime formula.  And no rule cuts an optimal game: for every
game that needs a search, the same search with the target one below V(n) finds a set worth V(n), which replays by the
rules (`examples/paritycheck.rs`; all 665 such games to 1300, `data/paritycheck_2_1300.jsonl`, 31 minutes).

**On a cloud machine (1301 to 2175).**  The first run past 1300 proved the 875 games from the plain root, one game a
thread, on one `c8g.4xlarge` (Graviton4, 16 vCPUs) at spot, with the build of commit 4db7a14, in 15.9 hours for $3.84.
It searched 490 games in 309.2 million nodes and 226.0 thread-hours.  [The notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)
has the account, and the projection by which it was planned.  Its build lacked the later cuts: the roles, the bars
outside the heaviest set, and the choice of the conflict by expected tree.  On the 314 games that the run of 2..2530
also searched, it took 194.1 million nodes, against 13.2 million (6.3 thread-hours) for the run of 2..2530.

## A game on several threads

On four threads, a game's search ran about four times as fast as on one, on the two games measured.  Its tree is not
one thread's, since each thread keeps nogoods of its own: 1540 took 118,246 nodes on one thread, and 138,021 and
146,688 on two and four.  On one thread the search is the one-thread search of before, node for node (12,771 nodes at
1548, 118,246 at 1540).  `exact` from the roles of F4, 1540 to its end and 1475 for five minutes:

| game | threads | nodes | seconds | thread-seconds | nodes a second |
|---|---|---|---|---|---|
| 1540 | 1 | 118,246 | 309 | 309 | 382 |
| 1540 | 2 | 138,021 | 146 | 291 | 946 |
| 1540 | 4 | 146,688 | 73 | 290 | 2,001 |
| 1475 | 1 | 105,472 | 301 | 301 | 351 |
| 1475 | 4 | 473,528 | 301 | 1,200 | 1,574 |

The run of 2..2530 put up to 88 threads on a game: 58 of its 795 searches ran on more than one thread, the dearest, 2346,
on 88.  No such game was searched on one thread to compare.

*Details.*  The runs above were release builds at nice 19, on a machine whose other threads ran other work at idle
priority, where two runs of one binary on one game differed by up to 20% (1548 on one thread: 20.3 and 24.4 s).

* **Subtrees moved.**  Four threads moved 84 subtrees of 1540 from one thread to another, and 12 of 1475 in five
  minutes.  A pause left 234 open subtrees of 1475 on four threads, against 90 on one.  A run that resumes them
  restarts each cold, and that costs little: four threads resumed 1475 at 1,321 nodes a second over their first
  minute.
* **Nogoods.**  A thread that joins a game starts with no nogoods, and the game's first thread of a run with the
  checkpoint's, the union of its threads' tables.  When every thread started from those 14,509 nogoods, the resumed
  1475 ran 637 nodes a second, since a node scans every nogood its thread holds.  Taking the other threads' nogoods
  every 10 seconds (`--share 10`) did not pay at 1540: 89.7 s and 143,729 nodes, against 84.3 s and 149,864 without,
  in a build where a joining thread started from the game's nogoods.

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
search of the theory track ([the certificate theorem
(E2)](../../../docs/theory/extended-problem.md#the-certificate-theorem)).  That is why the files are kept: U\* less a
game's open members is that search's fixed set.

The tests check the certificates against every playable set that obeys F4, on boards to 60 (1,403 checks):

* the certificates are computed again at each claim T of V − 1, V − 2 and V − 5, and checked against every such set
  worth more than T; a member certified against V says nothing of the sets worth V − 4;
* a deliberately wrong rule, which certifies a member also when its bound is at most T + 10, is caught by the same
  check (12 false certifications over the boards and claims, 15 (set, member) misses on 12 boards), so that the test
  can see the failure it guards against;
* F4's roles are checked against every optimal set there and the recorded moves to 1300.

Four files, each made on one thread of the laptop:

| file | games | needing it | time | what for |
|---|---|---|---|---|
| `data/upper_2_1300.jsonl` | the table's lines for 2..1300, 1,299 | 665 (417 of them with nothing open; 1.0% of the members open) | 65 s | |
| `data/upper_1301_2175.jsonl` | the committed chain's lines for 1301–2175, 875 | 490 | 300 s | |
| `data/upper_2176_3314.jsonl` | the copy of the cloud chain's results taken on 2026-10-15, games that the committed chain also holds, which [the gap past 2175](https://github.com/bvchess/taxman-notebook/blob/main/search/the-gap-past-2175-2026-10/README.md) measured; 1,139 | 635 | 1,272 s, 4.1 s for the slowest game | |
| `data/upper_3315_3440.jsonl` | the rest of the committed chain as it stood then, 126 | 73, 3424, 3425 and 3440 among them | 277 s, 4.3 s for the slowest game | |

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
  Past the prover's proofs, on 2531-3440, 15.2% are open (68,144 of 449,033).
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
