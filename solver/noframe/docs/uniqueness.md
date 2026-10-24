# Is the optimal set unique?

Part of [the prover's README](../README.md).

The optimal set is unique for every n ≤ 424, but not at 425: from there some games have two or three, which differ by an
exchange of numbers below n/2 of equal sum.  Many move orders play the same set of picks; the question is whether
different *sets* can be optimal.  If the optimal set were unique, or unique up to a well-described exchange, a solver
could lean on it: to reuse the game of n − 1 when solving n, to justify shortcuts, or to break symmetry.

## What is proven

The theory, in [the uniqueness theorem
(Q1)](../../../docs/theory/uniqueness-and-longest-games.md#the-uniqueness-theorem) and [the step theorem
(N6)](../../../docs/theory/neighbors.md#the-step-theorem):

* **The heaviest playable set of sources is unique wherever (★) holds** (Q1), so in every game n ≤ 100,000
  ([Conjecture (★) at scale](star.md#conjecture--at-scale)).  More precisely, U\* dominates every playable set of
  sources S termwise: the i-th largest member of S is at most the i-th largest of U\*, so S weighs strictly less unless
  S = U\*.  The playable sets are not a matroid, but under (♦) they lie inside the GF(2) matroid of which U\* is the
  greedy basis, and a matroid with distinct weights has one heaviest basis.  This is about the sources' instance alone.
  It does *not* say that every optimal game's upper part is U\*: a game with a lighter upper part could in principle win
  the difference back below n/2.
* **The prime of every optimal set**: every optimal set of n holds the largest prime q ≤ n, and no other prime.
* **Optimal sets from the game before** (N6).  If opt(n) = n + opt(n − 1), a game at its ceiling, the optimal
  sets of n are exactly the playable T ∪ {n}, for T optimal for n − 1.  If n is prime, T ↦ T − p + n, with p the
  largest prime below n, is a bijection from the optimal sets of n − 1 onto those of n.  So a game has as many optimal
  sets as its predecessor at a prime, and no more at a game at its ceiling; only a composite game below its ceiling can
  have more.
* **No matroid relaxation is exact for the whole game**: game 425 has two optimal sets (below), and a matroid with
  distinct positive weights has one heaviest independent set.

## Method

`src/bin/optsets.rs` lists every optimal set of a game, on the whole board, with the branch and bound of
[proofs.md](proofs.md) in an enumeration mode (`Prover::enumerate`).  Every pruning rule of the search cuts only sets
worth at most its target, so after a playable set P is found the search goes on: it partitions the rest of P's node by
the first number on which a set disagrees with P.  With the target V(n) − 1, the recorded score minus one, it lists
every playable set worth at least V(n), so every optimal set; it would stop on one worth more, refuting V(n).  The games
at their ceiling and the prime games take their sets from the predecessor's by N6 (`derive_optimal_sets`).

Optionally (`--upper fixed` / `other`) the question splits in two:

* the optimal sets whose upper part is U\*: a search with the sources fixed to U\*, which is cheap;
* those whose upper part is not: a search whose root is split on the largest source where the set and U\* disagree,
  which costs as much as a proof of optimality.

The checks:

* the enumeration equals brute force on every n ≤ 19, at the recorded score and below it, where sets containing a
  found one also qualify;
* the derivations equal direct enumeration on every game at its ceiling and every prime game to 700, and to 260 in
  the test suite;
* `tools/crosscheck_optsets.py` compares the lists with `heuristics/evaluation/certify.py`'s brute force over raw pot
  states, which uses no theory, and they agree on every n ≤ 57;
* both optimal sets of 425 replay by the rules (`heuristics/core.py`'s ordering and `check_sequence`);
* every optimal set found holds the largest prime and no other.

Enumerating at V(n) − 1 costs about what the proof at V(n) costs.  Over the 154 games to 989 whose proof took at least
10,000 nodes, the ratio runs from 0.91 to 1.49, 4% more in total.  So a proof of opt(n) by the prover can return the
complete list of optimal sets almost for free.

## Results

`data/optsets_2_989.jsonl` (every game 2..989; `tools/summarize.py optsets`):

| n | games | enumerated completely | 1 optimal set | 2 sets | 3 sets | sources fixed only |
|---|---|---|---|---|---|---|
| 2..424 | 423 | 423 | 423 | | | |
| 425..500 | 76 | 76 | 64 | 12 | | |
| 501..600 | 100 | 100 | 99 | | 1 | |
| 601..700 | 100 | 100 | 92 | 6 | 2 | |
| 701..800 | 100 | 97 | 88 | 9 | | 3 |
| 801..900 | 100 | 96 | 96 | | | 4 |
| 901..989 | 89 | 62 | 62 | | | 27 |
| **total** | **988** | **954** | **924** | **27** | **3** | **34** |

Searches with a budget of 4 million nodes, each under about seven minutes, covered every game whose proof of optimality
had taken fewer than 3.6 million; the games at their ceiling and the prime games were derived.  The 34 others are the
most expensive proofs of [proofs.md](proofs.md) (798–804, 945–954, 969–983, 987–989) and the games at their ceiling and
the prime games that follow them.  Their lines record the budget each was given: 4 million nodes for 798–800 and
803–804, and a short 300,000 for the others, which were not attempted in full.

For them, `data/optsets_fixed_798_1019.jsonl` lists the optimal sets whose upper part is U\*, with the sources fixed:
10 seconds to 2 minutes each.  It does so for 990–1019 too.  Each of those 34 games, and 990, 991 and 999–1019, has
exactly one optimal set with upper part U\* (57 games).  For 992–998, whose optimal games the frame model's search
also reaches late, the search with the sources fixed spent its 2.5 or 4 million nodes without reaching even the
recorded set.

The games with more than one optimal set fall in six clusters.  Each starts with an exchange new at its first game,
which games at their ceiling or primes carry from game to game, and some games meeting neither:

| games | sets | exchange (canonical set's numbers ↔ the other set's) | sum | size |
|---|---|---|---|---|
| 425–428 | 2 | {110, 132, 168} ↔ {98, 150, 162} | 410 | 3 ↔ 3 |
| 432–439 | 2 | {132, 147, 180, 208} ↔ {156, 160, 162, 189} | 667 | 4 ↔ 4 |
| 539 | 3 | {128, 168, 240} ↔ {120, 192, 224}; {120, 154, 189, 220, 234} ↔ {147, 180, 182, 198, 210}, between the two non-canonical sets; the canonical set and the third are 7 ↔ 7 apart | 536, 917, 1333 | 3, 5, 7 |
| 684–687 | 3, 3, 2, 2 | {44, 198, 238, 325} ↔ {68, 196, 266, 275}; {44, 238, 252, 260, 294} ↔ {68, 234, 250, 266, 270}; {198, 234, 250, 270, 325} ↔ {196, 252, 260, 275, 294} | 805, 1088, 1277 | 4, 5, 5 |
| 696–699 | 2 | {198, 234, 250, 270, 325} ↔ {196, 252, 260, 275, 294} (one of 684's) | 1277 | 5 ↔ 5 |
| 770–773, 775–779 | 2 | {228, 231, 252, 260, 280, 320, 325} ↔ {189, 250, 270, 273, 275, 315, 324} | 1896 | 7 ↔ 7 |

At 684–685 the third exchange is between the two non-canonical sets.  Game 774 has one optimal set, and 775 brings
770's exchange back.  No completely enumerated game from 780 to 989 has more than one optimal set.

What the data shows, observed and not proven:

* **The optimal set is unique for every n ≤ 424.**  Game 425 is the first with two.  Of the 954 games enumerated
  completely, 30 have several optimal sets, at most three, all between 425 and 779.
* **Every optimal set of every completely enumerated game has upper part U\***.  This is the strong form of
  `heuristics/evaluation/verify.py`'s measurement: no optimal game ever trades weight above n/2 for weight below it.
* **All the optimal sets of a game have the same number of members, and differ by an exchange of equally many numbers
  below n/2 of equal sum**, from 3 ↔ 3 up to 7 ↔ 7.  Every exchanged number lies in (n/5, 0.49 n) but 44 and 68 at
  684–687.  Every exchanged number is a maximal factor of a member of U\*, so an exchange trades which of those factors
  are picked and which are free to pay for their multiples above n/2.  At 425, with peel's payments: in one set 110,
  132 and 168 are picked, and 98, 150 and 162 pay for 196, 300 and 324; in the other, 98, 150 and 162 are picked, and
  110, 132 and 168 pay for 330, 396 and 336.
* **The canonical set survives every step to a game at its ceiling or to a prime** observed from a game with several
  sets: 13 of 13 such steps.  The canonical set is the largest in decreasing lexicographic order, and canonical(n) =
  canonical(n − 1) + n at a ceiling, or − p + n at a prime.  The prime part is a theorem, since the bijection preserves
  the lexicographic order.  The table optimal.json records the canonical set in 23 of the 30 games with several sets;
  not at 684 and 685, where it records the third of three, or at 775–779, the second of two.
* **The recorded game of n − 1 is not always extendable.**  At 686, a game at its ceiling, the recorded optimal game of
  685 plus 686 is unplayable.  The equality opt(686) = 686 + opt(685) holds through another optimal set of 685, the
  canonical one.

## What it buys a solver

* **The sources.**  Fixing the sources to U\*, as the frame model does, gives up nothing that ties.  The set U\* is the
  unique heaviest playable set of sources (proven given (★), so to 100,000), and every optimal game observed has upper
  part U\*.  The second statement is the one still open in general.
* **Reuse of n − 1 (N6).**  The prime step is exact: the optimal sets of a prime n are those of n − 1 with p
  swapped for n.  The step to a game at its ceiling is decidable from the complete list of the optimal sets of n − 1:
  n meets the bound n + opt(n − 1) iff one of them stays playable with n added, and then the optimal sets of n are
  known without search.  With only one recorded set the test is not decisive (685 → 686).  The lists are short, 1–3
  sets so far.  The part with upper part U\* is cheap to get with the sources fixed: seconds to two minutes per game to
  1019, except 990–998 (six and seven minutes for 990 and 991, out of budget for 992–998).  That part is the whole list
  wherever every optimal set has upper part U\*, which is observed but not proven.
* **Symmetry breaking.**  A proof of optimality, with target V and a set worth more than V, already cuts every tie, so a
  rule of canonical sets prunes nothing there.  It matters where ties are accepted: a search for a game reaching its
  ceiling, or a step reusing the previous game.  There, carrying the canonical set, rather than whichever optimal set a
  search happened to return, is consistent with every observed step to a game at its ceiling or to a prime, and well
  defined.
