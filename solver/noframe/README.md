# The prover

The prover proves the frame solver's games optimal on the whole board, without the [frame
model](../../docs/GLOSSARY.md#the-frame-model); [section 3](#3-the-proofs-of-optn) says how far its proofs reach.  It is
a branch and bound over sets of moves, which takes the recorded score of game n as its target and proves that no set of
moves beats it.  It shares no code with the frame solver or with the heuristics, so the frame solver's answers are
checked without assuming anything of the frame model.  The [glossary](../../docs/GLOSSARY.md) defines the project's
words.

Why that matters: the frame solver ([its README](../frame/README.md)) searches the promotions of the frame model, whose
answer is opt(n) exactly when [statement U](../../docs/GLOSSARY.md#the-frame-model) holds at n: some optimal game plays
every source with three or more prime factors that setup keeps ([THEORY.md, "The frame model: what is proven and what
remains"](../../docs/THEORY.md#the-frame-model-what-is-proven-and-what-remains), where the model's other rules are
proven; [DESIGN.md, section 2](../docs/DESIGN.md#what-optimal-means)).  Where the prover proves opt(n), U holds.  The
sources that setup keeps, [U\*](../../docs/GLOSSARY.md#the-frame-model), are the heaviest playable set of sources
wherever THEORY.md's conjecture (★) holds (Theorem 8; [THEORY.md](../../docs/THEORY.md#the-open-problem)).

The prover also lists every optimal set of a game, finds the longest games, certifies conjecture (★) at scale, and
searches for adversarial instances.  It rests only on THEORY.md's theorems and on elementary bounds, and every exact
method in it is tested against brute force.  The theorems are these:

* Theorem 1, with peel as the playability oracle: it decides playability by peel;
* the residual theorems;
* the parity matroids of Theorem 10;
* Theorem B, the bottom layers of every optimal set, in the proofs that start from its roles, which is the default of
  `exact`.

Section 3 lists what each proof rests on.  This document says *pick* for a [move](../../docs/GLOSSARY.md#the-game),
*forced picks* for the numbers fixed in at a node (the frame solver's [forced member](../../docs/GLOSSARY.md#the-search)
is the same idea for its own search), and [upper part](../../docs/GLOSSARY.md#the-frame-model) for a set's sources.

What the prover established, assumption by assumption ([section 1](#1-what-existed-before) has the evidence before
it):

| assumption | what the prover established | grade |
|---|---|---|
| (★): descending greedy is maximum-cardinality on every component and truncation of the sources' instance, hence U\* is the heaviest playable set of sources (the upper-half conjecture) | **every game n = 2 .. 100,000** (602,777,907 greedy rejections) and **902 games to 2,000,000** (60,183,674 more): every rejection a GF(2) linear dependency, which certifies (★) for the whole game; the five components that the earlier evidence left undecided settled; no counterexample | certified per game (a bound met), open in general |
| greedy removal of impossible sources in frames (`removeImpossible`) | at the root (where it shapes the search space) it *is* descending greedy on each component of the sources, so it is certified with (★) for every n ≤ 100,000 | certified per game |
| the frame model as a whole: `unzip`'s mandatory moves, freedoms, closed frames inert, promotions one frame forward, greedy removal | **opt(n) proven equal to the recorded score, on the whole board, for every n = 2 .. 2175**, the committed chain as far as it reached when the proofs were made | verified (exact, on the whole board) |
| uniqueness: one optimal set per game, and every optimal game's upper part U\* | **U\* is the unique heaviest playable set of sources wherever (★) holds** (Theorem 8), so to 100,000; every optimal set enumerated, on the whole board, in 954 of the games 2..989 (the other 34 with the sources fixed to U\*): **the optimal set is unique for every n ≤ 424, not at 425**; 30 games have two or three; every optimal set found has upper part U\* | proven (the sources, given (★)); measured (whole game) |
| the longest games: an optimal game plays the most moves a game of the board can (the wiki's open question) | **false**: at n = 833, 834 and 835 every optimal game is one move short of the longest; every other game to 1300 is a longest game, the difference never above one | refuted (on the whole board); measured to 1300 |

The sections:

* [Section 2](#2-conjecture--at-scale) certifies (★) by a statement of linear algebra, **(♦)**: every member that
  descending greedy rejects is a GF(2) combination of members it kept above it.  Statement (♦) is cheap to check
  exactly (game 2,000,000 takes 2.7 minutes), and it holds in every game checked.
* [Section 3](#3-the-proofs-of-optn) proves the recorded scores to 2175 by a branch and bound over the picks, limited
  by the Franklín–Moniot bound and the *parity bound*, whose gap is about a tenth of the Franklín–Moniot bound's.  The
  chain's games past 2175 are proven within the frame model alone, not by the prover.
* [Section 4](#4-adversarial-search-what-protects-greedy) searches Taxman-shaped synthetic instances for what protects
  greedy: the *window law*, taken with respect to the same ranking of the primes that decides which products fall in
  the window (n/2, n].
* [Section 5](#5-is-the-optimal-set-unique) lists every optimal set of a game, for about the cost of proving its score.
  The optimal set is unique for every n ≤ 424; from 425 on, some games have two or three, with the same upper part.
* [Section 6](#6-does-an-optimal-game-play-the-most-moves) finds the longest games.  An optimal game need not be one:
  game 833 has a game of 363 moves, while every optimal game has 362.
* [Section 7](#7-reproducing) gives the commands, and [section 8](#8-what-remains-open) what remains open.

## 1. What existed before

Before the prover, this was the state of the evidence.

* **Proven** (THEORY.md):
  * playability ⟺ a maximal-factor matching with acyclic precedence (Theorem 1), and peel decides it (Theorems 2, 3);
  * for separated instances, playable ⟺ no self-covering subset, and the minimal exclusion sets lie in the residual
    (Theorems 4–6);
  * greedy is weight-optimal wherever the frontier jam is hyperedge-free;
  * the bounds opt(n) ≤ n + opt(n − 1), the prime formula, and the Franklín–Moniot bound.
* **(★), the upper-half conjecture.**  THEORY.md's "Evidence" had 3,231 of 3,236 rejection-bearing components (all n
  = 6..1500, forty samples to 4000, n = 2873 and 9170) verified by exact search over the residual or certified by the
  matching-number bound.  Five were undecided (n = 3103, 3872, 3936, 4000, 9170, with search budgets of 2.7×10⁹ to
  1.1×10⁴⁷), the last "to be settled only by a theorem"; the uncertified 0.2% were exactly those five.  Besides:
  * an exhaustive check that U\* is the max-playable set of sources for n = 2..99, and a brute force over pot states
    (`heuristics/evaluation/certify.py`) that the heaviest set of sources is unique to n = 62, with one optimal game
    recorded per n;
  * the census of 3-prime padded jams: 428 shapes, 22,909 trap orderings, none unprotected;
  * `heuristics/evaluation/verify.py`: U\* equals the sources of the optimal game for n ≤ 1000.

  The scripts and data of the (★) campaign and of the census were never committed, only their numbers, in THEORY.md
  and the commit messages of e04f7c4, 20822ea and 43936d3.  So that evidence could not be reproduced; this crate's
  checks can.
* **The frame model** ([DESIGN.md, section 2](../docs/DESIGN.md#what-optimal-means)).  Its assumptions (the greedy
  removal of impossible sources, `unzip`'s mandatory moves, freedoms = factors − sources, closed frames inert,
  promotions one frame forward) were validated end to end:
  * the solver reproduces the Java's optimal.json to 1000, which the same model made, so not independently;
  * `src/brute.rs` matches it to 40 in its tests, and once to 52, by the example `sacrifice`, removed from master;
  * `heuristics/evaluation/certify.py`'s brute force over pot states confirms it to n = 62.

  Beyond 1000 (`results`), nothing was independent of the frame model.

## 2. Conjecture (★) at scale

Conjecture (★) is certified for every game to 100,000, and for 902 sampled games to 2,000,000, by a check of (♦) that
is greedy itself.

### The certificate

For a set of members of a reduction instance, give each member its incidence vector over GF(2): a 1 at each of its
factors.

* **Playable sets are independent.**  A playable set dismantles completely (Theorems 4, 5).  In a dismantling order
  x₁, x₂, … each xᵢ has a factor that no later member lists, so in any vanishing sum the first member's private
  coordinate is 1.  So no playable subset of a set K is larger than rank(K).  On members with at most two factors this
  is the rank of a graphic matroid, exact there.  The factors pq, pr and qr of a three-prime member are the edges of a
  triangle on the primes, and the dependencies of such members are closed surfaces mod 2, invisible to counting
  bounds.
* **(♦) ⟹ (★).**  Suppose every member that descending greedy rejects is a sum of members it kept above it.  Above any
  threshold t, greedy's kept members are independent (playable) and span every member above t, so they are a basis:
  greedy keeps rank(K_{≥t}) members, the most any playable subset has.  That is (★) for every component and every
  truncation.  Since greedy is optimal on a matroid, it is also the upper-half conjecture for every weighting
  consistent with the order.
* **Checking (♦) is greedy itself** (`src/span.rs`).  A member with a factor no kept member lists is kept (the
  extension lemma).  A member the kept ones span is rejected, since dependent sets are unplayable.  Only the rest are
  peeled, and a rejection among them would be a violation.  Everything runs on the components of the residual, where
  all cores and all dependencies live.

Statement (♦) is stronger than (★).  A rejection forced by an "odd" core, self-covering but with no factor-even
subset, would violate (♦) without breaking (★).  Such cores exist in Taxman (the exchange failure at n = 2873 needs
one), but descending greedy has never rejected a member because of one.

### Coverage

| range | games | greedy rejections | violations of (♦) | largest residual component | CPU |
|---|---|---|---|---|---|
| n = 2 .. 100,000, every game | 99,999 | 602,777,907 | 0 | 8,850 members | 61 min |
| n = 101,000 .. 1,000,000 every 1000th, 1,500,000, 2,000,000 | 902 | 60,183,674 | 0 | 159,870 members | 3.7 h |

`data/span_2_100000.jsonl.gz` and `data/span_samples_101000_2000000.jsonl.gz` have one line per game: the residual,
the kept set's size and sum, the rejections, how many the span certified, and the violations.

The check of (★) threshold by threshold (`src/star.rs`) decides each rejection threshold's new jam by the cheapest
sufficient argument.  It ran on every game n = 2 .. 20,000 (`data/star_2_20000.jsonl.gz`):

| argument | thresholds | share |
|---|---|---|
| one exclusion is always needed (the jam is a nonempty residual) | 295,020 | 1.2% |
| matching-number bound \|P\| − ν(P) | 20,730,083 | 86.2% |
| Lorea's hypergraphic-matroid bound | 2,004,572 | 8.3% |
| rank bound (the GF(2) rank of the incidence vectors) | 1,028,755 | 4.3% |
| exact search needed | 0 | |
| **total** | **24,058,430** | |

The old campaign's unit was the component: 58,306 rejection-bearing components, 3,119 of them in n ≤ 1500.  By
component, the hardest argument needed was the matching-number bound, or one exclusion, for 21,015, Lorea's bound for
19,798 and the rank bound for 17,493.  The five components left undecided before all fall to the rank bound.  The jam
of n = 9170, with P = 304 members and greedy's 37 exclusions, of which the matching-number bound proves only 19, is
certified in milliseconds.

*Details.*  Before the rank bound was found, two exact searches had pushed n = 9170 to its first 16 thresholds (P =
127, g = 13): `src/maxcard.rs`, from scratch, and `src/plus1.rs`, which proves each threshold's "+1" given the earlier
thresholds, with the laminar family of earlier jams as cardinality constraints and cores as learned clauses.  Both
engines stay as the fallback for a violation of (♦), and are checked against brute force and against each other on
random instances where greedy fails.  As cross-checks, the prover's greedy equals `heuristics/core.py`'s `optimize_mini`
on every n ≤ 1000 and 62 samples to 6000, and its kept set's sum equals the sum of the sources of optimal.json's game
for every n ≤ 1000 (`tools/crosscheck_greedy.py`).

## 3. The proofs of opt(n)

The prover proves opt(n) = V(n), the recorded score, for every n ≤ 2175.  `src/game.rs` proves upper bounds on opt(n)
from proven facts only:

* Theorem 1 with peel: a set of picks is playable iff dismantling the instance of the picks, each listing its maximal
  factors that are not picks, removes everything;
* the Franklín–Moniot bound, and the parity bound (THEORY.md, Theorem 10);
* opt(n) ≤ n + opt(n − 1);
* the prime formula's upper bound: a game playing a prime n plays it first, and swapping it for the largest prime
  below n gives a game of n − 1;
* in a search that starts from the roles of Theorem B (the default of `exact`; `--fix-low 0` starts from the plain
  root), THEORY.md's Theorem B.  Every optimal set holds the largest prime and no other, and holds exactly U\*_2 among
  the sources of layer 2.  On a board whose frames of layer 2 are closed, it holds no number of layer 2 up to n/2.
  The frames of layer 2 are closed when U\*_2 spans every prime up to n/2, which `src/upper.rs` decides for each board
  from its own U\*.

A search from the roles proves that no playable set that obeys them beats V(n).  That proves opt(n) ≤ V(n), because an
optimal set beating V(n) would obey them.  Its lines say `"fix_low":1`, and the plain root's `"fix_low":0`; the
committed `data/exact_*.jsonl` are all from the plain root, and rest on no Theorem B.  `tools/summarize.py exact` and
`check` count the proofs that rest on it.  The prover knows nothing of frames, greedy, `unzip` or promotions.

The search:

* **Bound.**  The maximum-weight matching between numbers and their maximal factors, with forced picks matched downward
  and barred numbers only paying.  It is solved once by successive shortest paths, then re-optimized in each child from
  the parent's optimal matching and dual, each freed vertex repaired by one Hungarian step.  That made the search 160
  times faster than solving each node from scratch.
* **The parity bound** (`src/parity.rs`, `Prover::with_parity`; on in `exact` since 2026-10).  Every playable set is
  independent in two linear matroids over GF(2), split by the parity of Ω (THEORY.md, [the parity
  matroids](../../docs/THEORY.md#the-parity-matroids-a-bound-on-the-whole-game)).  So at a node, the heaviest set
  independent in both, with the forced picks contracted and the barred numbers deleted, bounds every completion.
  * It is computed by shortest augmenting paths, a child starting from its parent's set and weight split.
  * It is reported only with a weight split that satisfies the local optimality conditions of both matroids, checked
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

The target is the recorded score V(n).  The search proves that no playable set beats it, or, from the roles of Theorem
B, none that obeys them, which is as good.  The recorded game replays by the rules to V(n), so opt(n) = V(n).

Every rule above is tested against brute force or against the recorded score.  The search must still reach
optimal.json's score from one below it, with each rule on, and from the roles of Theorem B too, on every third game of
187-700 that needs a search.  The parity bound's intersection is checked against enumeration on random pairs of
matroids.

**From the roles of Theorem B** (the default since 2026-10; the committed proofs below are from the plain root).  Over
the games of 2..999, the 391 searches took 588,026 nodes and 357 s from the roles, against 614,900 nodes and 712 s from
the plain root (`data/exact_parity_2_1300.jsonl`).  The nodes are within 5%, and the time of a node is halved, since the
parity solves have the primes and the numbers of layer 2 settled.  Every game of 2..2175 has the recorded game obey the
roles, and the roles' parity bound is at least its score; `exact --budget 1` checks both for every game before any
search.  The frames of layer 2 are closed on every board from 187 on, and on most below it.  The roles bar the numbers
of layer 2 up to n/2 where the frames are closed, and leave them free on the 80 boards of 2..1300 where they are not,
all of them at most 186 (`upper::tests`).

### With the Franklín–Moniot bound (to 989)

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
bound's gap: 140–430 points around n = 500–700, and 650–1200 from 800 on.  The hardest games are those whose score it
overrates most: 798–804 at 7–12 million nodes, and 987–989 at 19–53 million.  Games 990, 992, 993, 995 and 996 took
50–70 million nodes each, and 1020 and 1023–1027 several million, without finishing; their searches are saved in
`data/checkpoints/exact_open_searches.tar.gz`.

### With the parity bound (to 1300)

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
| **every n in 2 .. 1300**, the whole committed chain | **1299** |

The parity searches of this file took 2.7 hours of thread time on three threads; the warm starts came in for its last
96 games.  The hardest was 1287, at 146,855 nodes and 279 seconds.  So the frame model's answer is opt(n) for every
game n ≤ 1300 of the committed chain, including the solver's own results past the Java's 1000.

`data/exact_parity_2_1300.jsonl` proves every game again from scratch with the parity bound, with no resumed search
and no proof by the Franklín–Moniot bound alone (the final build, two threads).  It is a second, self-standing proof of
every n ≤ 1300 by a different bound, and the cost curve below.  And no rule cuts an optimal game: for every game that
needs a search, the same search with the target one below V(n) finds a set worth V(n), which replays by the rules
(`examples/paritycheck.rs`; all 665 such games to 1300, `data/paritycheck_2_1300.jsonl`, 31 minutes).

### What a proof costs, and how far it reaches

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

The cost follows the root's parity gap as the frame solver's follows its root gap.  Over 900–1300 a search's nodes
grow tenfold per 84 points of gap (1.19 ± 0.05 decades per 100 points, R² 0.76).  The gap grows about 14 points per 100
in n, with swings of ±50 between bands; that is the matroid study's measurement, which these roots reproduce.  Against
n, log10 nodes grows 0.16 ± 0.04 and log10 seconds 0.23 ± 0.04 per 100 in n over 900–1300 (R² 0.06 and 0.11: the
gap's swings dominate), and 0.33 and 0.34 from n = 78 on.  The search with the Franklín–Moniot bound alone had spent
50–70 million nodes on each of 990–996 without finishing; the parity bound proves them from scratch in 44,000–68,000.

The prover's proofs reach 2175, where the committed chain (`../../results/strict.jsonl`) ended when they were made; the
next subsection is the run that made them.

*Details.*  An extrapolation from the bands above, 0.23 decades per 100 in n, made before that run, gave about 57 hours
of thread time to 2000, and about 3 hours for the hardest proofs.  It was low: 1301–1999 took 164 hours, and the hardest
proof, of 1475, 4.9.

### To 2175, on a cloud machine

`data/exact_parity_1301_2175.jsonl` is `exact` over the 875 games of 1301–2175, each proven from scratch with the
parity bound.  It ran on one `c8g.4xlarge` (Graviton4, 16 vCPUs, one game a thread) on spot in us-east-1b, run by
`../tools/cloud/prover.sh` ([solver/docs/runbook/prover.md](../docs/runbook/prover.md)), with
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
entry](../notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md) has the run's account,
the projection game by game, and what the run says about the projection's model.

### The upper part, game by game

Past the prover's proofs, the frame solver's answer for a game that rests on its search is opt(n) exactly when U holds
at n: some optimal set plays U\*_{≥3}, the members of U\* with three or more prime factors
([THEORY.md](../../docs/THEORY.md#the-upper-part)).  `src/bin/upper.rs` (`src/upper.rs`) asks the parity bound which
of those members every set beating the recorded score V must play.

* It fixes layers 1 and 2 as Theorem B says every optimal set has them: the largest prime and no other; U\*_2 among the
  sources of layer 2; and, where U\*_2 spans every prime up to n/2, which it decides for each board, no number of layer
  2 up to n/2.
* It bars one member u at a time, and calls u *certified* when the bound is then at most V.  Every set that beats V
  and obeys Theorem B then plays u, so if opt(n) > V, every optimal set plays every certified member.
* The members it does not certify are *open*, with the bound's margin over V.

Only the games that rest on the frame model need it: those neither at the ceiling nor given by the prime formula, whose
lines say `"needs":1`.  Certified members can be fixed in a search that leaves the open ones optional, the
integrated search of the theory track, which the certificates of U carry out ([DESIGN.md, "Certificates of
U"](../docs/DESIGN.md#certificates-of-u)).  That is why the files
are kept: U\* less a game's open members is that search's fixed set.

The tests check the certificates against every playable set that obeys Theorem B, on boards to 60 (1,403 checks):

* the certificates are computed again at each claim T of V − 1, V − 2 and V − 5, and checked against every such set
  worth more than T; a member certified against V says nothing of the sets worth V − 4;
* a deliberately wrong rule, which certifies a member also when its bound is at most T + 10, is caught by the same
  check (12 false certifications over the boards and claims, 15 (set, member) misses on 12 boards), so that the test
  can see the failure it guards against;
* Theorem B's roles are checked against every optimal set there and every recorded game to 1300.

Four files, each made on one thread of the laptop:

| file | games | needing it | time | what for |
|---|---|---|---|---|
| `data/upper_2_1300.jsonl` | the table's games 2..1300, 1,299 | 665 (417 of them with nothing open; 1.0% of the members open) | 65 s | the tests of `taxman certify` |
| `data/upper_1301_2175.jsonl` | the committed chain's games 1301–2175, 875 | 490 | 300 s | |
| `data/upper_2176_3314.jsonl` | the copy of the cloud chain's results taken on 2026-10-15, games that the committed chain also holds, which [the gap past 2175](../notebook/the-gap-past-2175-2026-10/README.md) measured; 1,139 | 635 | 1,272 s, 4.1 s for the slowest game | |
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

* **No game is certified whole.**  Every one leaves members open: from 1 (1305) to 102 (2172) up to 2175, and from 15
  (2610) to 317 (2346) past it.  In all, 7.5% of the members are open to 2175 (18,596 of 247,482), 14.9% on 2176-3314
  (77,195 of 518,992), and 19.9% on 3315-3440 (14,704 of 73,968, 172 to 271 a game, the root's gap 1,103 to 1,555).
* **The open share follows the root's gap, not n.**  It grows 3.1 points per 100 of gap to 2175 and 2.0 past it (R²
  0.73 in each file), and it falls back with the gap (2600-2699).  The root's gap is the one the frame solver's search
  costs by: with layers 1 and 2 fixed, it equals the solver's GF(2) gap at the root on all 400 games past 2175 that
  both measured (`../notebook/the-gap-past-2175-2026-10/gaps_chain.jsonl`).  The margins grow with it, and the largest
  is close to the gap.
* **The root's heaviest set plays all of U\*_{≥3} in every one of the 1,125 games that need it**, as at every root
  measured before; no game is flagged.
* **The open members are mostly of layer 3** (52% to 2175 and 58% past it), then of layer 5 (21%, 27%) and 4 (19%, 9%),
  and the rest in layers 6 to 11.

## 4. Adversarial search: what protects greedy

What protects greedy is the window law, taken with respect to the ranking of the primes that also decides which
products lie in the window (n/2, n].  `src/adversary.rs` builds Taxman-shaped instances, each keeping one thing from
the real game and varying another.  It checks (♦) on each and, where (♦) fails, (★) threshold by threshold
(`tools/adversary_campaign.sh`, `data/adversary/`):

| family | instances | (♦) fails | (★) fails |
|---|---|---|---|
| real sources, n = 2000..20,000 step 100 | 181 | 0 | 0 |
| real incidence, **uniformly random order**, n = 3000 / 5000 / 10,000 | 600 | 25% / 16% / 50% | 8% / 6.5% / ≥ 31.5% |
| real incidence, values × (1 ± ε) noise, ε = 1% / 5% | 200 | 0 | 0 |
| same, ε = 20% / 100% | 200 | 27% / 40% | 21% / 20% |
| real incidence, random orders obeying the **window law** (members f·p sharing f in decreasing p), n = 3000..20,000 | 900 | 0 | 0 |
| window law, adversarial orders: smallest allowed first (n = 2000..20,000), three-factor members first, or last | 781 | 0 | 0 |
| **reversed** window law (increasing p), random | 300 | 0 | 0 |
| window law for a **random ranking** of the primes, n = 5000 / 10,000 | 500 | 26% / 57% | 26% / ≥ 38% |
| window law for the primes ranked by p × (1 ± ε), ε = 5% | 100 | 0 | 0 |
| same, ε = 20% / 50% | 200 | 30% / 41% | 28% / 33% |
| generalized primes p × (1 ± ε), ε = 5% / 20% / 50%, members and factors rebuilt from them (Beurling) | 350 | 0 | 0 |
| random generalized primes with the prime density (Cramér), n = 10,000 / 20,000 | 150 | 0 | 0 |

"≥" marks families where some thresholds stayed undecided within the search budget.  At n = 1000 every family passed,
20 instances each: traps need three-prime jams, and THEORY.md finds the first failure of exchange at n = 2873.

This sharpens THEORY.md's "the ordering is the protection, not the magnitudes":

* **The window law suffices; the rest of the order does not matter.**  Every order of the real incidence structure that
  keeps the members sharing a factor in the order of their cofactor primes, random or chosen adversarially, satisfied
  (♦).  Uniformly random orders fail often.
* **But only for the right ranking of the primes.**  The window law with respect to a random ranking of the primes
  fails as often as a random order, and so does the window law with respect to the primes perturbed by more than a few
  percent.  With respect to the primes reversed, it holds.  What protects greedy is that the order of the members
  sharing a factor agrees with the ranking of the primes that also decides which products lie in the window (n/2, n].
  Generalized primes, which rebuild the members and factors from the perturbed primes, never fail, even perturbed by
  50%.
* So the open problem can be sharpened to a statement without magnitudes beyond the window: **(♦) for multiplicatively
  generated instances under any order obeying their window law.**

## 5. Is the optimal set unique?

The optimal set is unique for every n ≤ 424, but not at 425: from there some games have two or three, which differ by an
exchange of numbers below n/2 of equal sum.  Many move orders play the same set of picks; the question is whether
different *sets* can be optimal.  If the optimal set were unique, or unique up to a well-described exchange, a solver
could lean on it: to reuse the game of n − 1 when solving n, to justify shortcuts, or to break symmetry.

### What is proven

THEORY.md, ["Uniqueness of the optimal set"](../../docs/THEORY.md#uniqueness-of-the-optimal-set):

* **Theorem 8: the heaviest playable set of sources is unique wherever (★) holds**, so in every game n ≤ 100,000
  (section 2).  More precisely, U\* dominates every playable set of sources S termwise: the i-th largest member of S is
  at most the i-th largest of U\*, so S weighs strictly less unless S = U\*.  The playable sets are not a matroid, but
  under (♦) they lie inside the GF(2) matroid of which U\* is the greedy basis, and a matroid with distinct weights has
  one heaviest basis.  This is about the sources' instance alone.  It does *not* say that every optimal game's upper
  part is U\*: a game with a lighter upper part could in principle win the difference back below n/2.
* **The prime of every optimal set**: every optimal set of n holds the largest prime q ≤ n, and no other prime.
* **Theorem 9: optimal sets from the game before.**  If opt(n) = n + opt(n − 1), a game at its ceiling, the optimal
  sets of n are exactly the playable T ∪ {n}, for T optimal for n − 1.  If n is prime, T ↦ T − p + n, with p the
  largest prime below n, is a bijection from the optimal sets of n − 1 onto those of n.  So a game has as many optimal
  sets as its predecessor at a prime, and no more at a game at its ceiling; only a composite game below its ceiling can
  have more.
* **No matroid relaxation is exact for the whole game**: game 425 has two optimal sets (below), and a matroid with
  distinct positive weights has one heaviest independent set.

### Method

`src/bin/optsets.rs` lists every optimal set of a game, on the whole board, with the branch and bound of section 3 in
an enumeration mode (`Prover::enumerate`).  Every pruning rule of the search cuts only sets worth at most its target, so
after a playable set P is found the search goes on: it partitions the rest of P's node by the first number on which a
set disagrees with P.  With the target V(n) − 1, the recorded score minus one, it lists every playable set worth at
least V(n), so every optimal set; it would stop on one worth more, refuting V(n).  The games at their ceiling and the
prime games take their sets from the predecessor's by Theorem 9 (`derive_optimal_sets`).

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

Enumerating at V(n) − 1 costs about what the proof at V(n) costs.  Over 68 games whose proof took at least 10,000
nodes, the ratio runs from 0.94 to 1.20, 4% more in total.  So a proof of opt(n) by the prover can return the complete
list of optimal sets almost for free.

### Results

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

Searches with a budget of 4 million nodes, each under about seven minutes, covered every game whose proof of
optimality had taken fewer than 3.6 million; the games at their ceiling and the prime games were derived.  The 34
others are the most expensive proofs of section 3 (798–804, 945–954, 969–983, 987–989) and the games at their ceiling
and the prime games that follow them.  Their lines record the budget each was given: 4 million nodes for 798–800 and
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
| 539 | 3 | {128, 168, 240} ↔ {120, 192, 224}; {120, 154, 189, 220, 234} ↔ {147, 180, 182, 198, 210}; the two non-canonical sets are 7 ↔ 7 apart | 536, 917, 1333 | 3, 5, 7 |
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

### What it buys a solver

* **The sources.**  Fixing the sources to U\*, as the frame model does, gives up nothing that ties.  The set U\* is the
  unique heaviest playable set of sources (proven given (★), so to 100,000), and every optimal game observed has upper
  part U\*.  The second statement is the one still open in general.
* **Reuse of n − 1 (Theorem 9).**  The prime step is exact: the optimal sets of a prime n are those of n − 1 with p
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

## 6. Does an optimal game play the most moves?

No: at n = 833, 834 and 835 every optimal game is one move short of the longest game.  The wiki's "Questions" page asks
it as its third question, left open, with the belief that the answer is yes: does an optimal game take as many moves as
any game of the board can, or does some longer game score less?  If optimal games were always longest, a solver could
stop searching every branch that cannot reach the most moves.  Neither the Java solver nor the frame solver does that
(below), but the Java's `reviseFreedoms` rests on a local form of the claim.

### What is proven

THEORY.md, ["Longest games"](../../docs/THEORY.md#longest-games).  Let M(n) be the most moves of a game of n, the size
of the largest playable set of picks.

* **M(n) <= ν(n)**, the matching number of the board's maximal-factor graph, which joins each number to its maximal
  factors: a playable set pays each pick with a distinct maximal factor outside it (Theorem 1).  So a legal game of
  ν(n) moves is a longest game, a certificate that costs one matching.
* **M(n − 1) <= M(n) <= M(n − 1) + 1**, and **M(n) = M(n − 1) for a prime n >= 3**: the count forms of the ceiling and
  the prime formula.
* Given (★), a playable set has at most |U\*| members above n/2 (Theorem 8's count inequality): a longest game cannot
  gain on an optimal game whose upper part is U\* except below n/2.

### Method

`src/moves.rs` and `src/bin/moves.rs` work the games in increasing order.  The number M(n) lies between two bounds:

* below, the longer of the recorded optimal game and the longest game of n − 1;
* above, the smaller of ν(n), computed for every board at once by augmenting paths, and what M(n − 1) allows.

Where the bounds meet, M(n) is settled.  Elsewhere the branch and bound of section 3 decides it, with every pick
weighing 1 ([`Prover::weighted`](src/game.rs)), in a mode that raises its target to each longer set it finds
([`Prover::optimize`](src/game.rs)), with segment cuts from the smaller games' bounds.  Where a game longer than the
recorded one exists, the heaviest longest game comes from the same search with the pick of c weighing K + c, where K is
above every sum, so that a set's weight orders by moves, then by sum.  That search starts from the heaviest longest
game with the recorded game's upper part, found with the sources fixed, which is cheap.  Every longest game written is
replayed by the rules, in the order Theorem 1 gives from peel's payments (`play_order`).

The checks: M(n) and the heaviest longest game against brute force for n <= 22, ν(n) against the matching solver for
every board to 400, and `play_order` against the recorded games to 400.

### Results

`data/moves_2_1300.jsonl` holds every game 2..1300 (`tools/summarize.py moves`).  It took seven minutes on one thread,
nearly all of it the three searches for the heaviest longest game, 2.5 to 2.6 million nodes each.

| | games |
|---|---|
| the recorded optimal game has ν(n) moves: a longest game by the certificate | 1,293 |
| ν(n) = M(n) + 1, the recorded game a longest one by a search (51, 182, 183; at most 173 nodes) | 3 |
| **the recorded game one move short of M(n) = ν(n)** (833, 834, 835) | **3** |

| n | optimal game: moves (above n/2) | M(n) | heaviest longest game | short of the optimal game by | its exchange of the optimal game's moves below n/2 |
|---|---|---|---|---|---|
| 833 = 7²·17 | 362 (314) | 363 | 218,197 | 29 | {250, 252, 294, 400} out, {116, 180, 200, 320, 351} in |
| 834 | 363 (315) | 364 | 219,031 | 29 | the same (834 is a game at its ceiling: both games of 833 plus 834) |
| 835 = 5·167 | 363 (315) | 364 | 219,377 | 29 | the same |

The two games of 833 in full (the optimal one is optimal.json's): above n/2 both play the same 314 moves, U\*; below
it,

```
optimal game (48):   245 250 252 258 261 273 280 288 294 297 300 306 310 315 322 330 333 336 338 342 343 345 350 357
                     363 364 368 369 372 375 376 378 380 385 387 390 392 396 399 400 402 405 406 408 410 412 414 416
longest game (49):   the same with 250 252 294 400 out and 116 180 200 320 351 in
```

Both replay by the rules (in `src/moves.rs`, the test `game_833_has_a_longer_game_than_its_optimal_one`), and the frame
model plays both (`solver/frame/tests/soundness.rs`).  The optimal games of 834 and 835 hold the same four numbers
below n/2, and the same exchange makes their heaviest longest games.

What the data shows:

* **An optimal game need not be a longest one.**  The first counterexample is n = 833.  Its optimal set is unique
  (section 5) and has 362 members, and a legal game has 363.  The heaviest 363-move game keeps the optimal game's upper
  part, U\*, and exchanges four of its picks below n/2 for five lighter ones, 29 points for one move.  That needs
  nothing of the frame model: the longer game replays by the rules, ν(833) = 363 bounds every game, and the search
  proves no 363-move game worth more than 218,197, less than the recorded game's 218,226.
* **The difference is at most one move to 1300, and one only at 833-835.**  Every other recorded optimal game is a
  longest game, 1,293 of them by the matching certificate alone.  At 836 the optimal game reaches ν again.  Since a
  game's optimal sets all have the same size wherever they were enumerated (section 5), "the optimal game's moves" is
  well defined there; at 833-835 the optimal set is unique.
* **The mechanism is a trade of weight for moves below n/2.**  A pick above n/2 uses one number up to n/2, its payment;
  a pick below uses two, itself and its payment.  At 833 the optimal game's 48 picks below n/2 and 362 payments use 410
  of the 416 numbers up to n/2, and the longest game's 49 and 363 use 412.  Four heavy picks below n/2 (1,196)
  outweigh five light ones (1,167).
* **ν(n) is a tight bound on moves.**  It exceeds M(n) only at 51, 182 and 183, by one, where every maximum matching is
  precedence-cyclic.  The Franklín–Moniot bound, weighted by value, is off by hundreds of points.

### The solvers and the wiki

* **Neither the Java solver nor the frame solver prunes a branch for having fewer moves than possible.**  Counts enter
  the Java's search only as upper bounds.
  * A frame's *freedoms* are its factors minus its moves (`search/Frame.java:43`).
  * `FrameSet.computeMaxNumberOfPromotions` (`search/FrameSet.java:136-151`) turns them into `maxTotalPromotions`, the
    most promotions the freedoms allow (`search/SearchManager.java:121-122`, `maxAdditionalPromotions` at :192-197).
  * The upper bound then sums at most that many of the largest candidates, and a branch is cut on that *sum*
    (`search/BranchExplorer.java:77`).
  * *In the bag* is a score, what the game with no promotions makes (`frame/FrameSolver.java:155-160`), not a count.

  The frame solver's `max_total` (`search/setup.rs`), `max_additional` (`bounds.rs`) and `in_the_bag` (`board.rs`)
  are the same.
* **The belief appears once, in `reviseFreedoms`** (`search/UpperBoundEstimator.java:41-90`).  Its comment states as
  a theorem, without proof, that the promotion set with the largest sum also has the most promotions.  It solves the
  two-frame game of frames 0 and 1 for its heaviest promotion set, and lowers frame 0's freedoms to that set's size,
  which can cut sets with *more* promotions into frame 0, never fewer.  The frame solver ported it as the tactic
  `revise_freedoms`, measured it and removed it ([which tactics earn their
  keep](../notebook/which-tactics-earn-their-keep-2026-10/README.md)).  `SearchManager.finalReport` (:185-188)
  reports an optimal game with fewer promotions than `maxTotalPromotions` as "max promotions was over-estimated".
* **833 refutes the comment's "theorem" for the whole frame game.**  The exchange above is a promotable set: the
  optimal game promotes 48 numbers and the longest 49, which is the frame model's `max_total`
  (`solver/frame/tests/soundness.rs`, `the_heaviest_promotion_set_of_833_is_not_the_largest`).  So at 833 `max_total`
  was not an over-estimate.  It does not decide the two-frame form that the tactic used: at 833 the tactic searched the
  two-frame game (`Searched { size: 21 }`) and left `max_total` at 49, and the scores of 833-835 are proven on the
  whole board.

### What it would buy a solver

A sound statement about move counts can only bound them from above, as `max_total` and ν(n) do.  Such a bound on
counts limits a sum only through the largest candidates, which the frame solver's matching bound already outdoes.  The
tempting unsound one, pruning every branch that cannot reach M(n) moves, would return 218,197 at n = 833 instead of
218,226, and 29 too little at 834 and 835.  In the frame model, pruning the branches that cannot reach `max_total`
promotions would do the same.  The certificate is still useful as a check: a game with ν(n) moves is a longest game,
in milliseconds.

## 7. Reproducing

The prover needs Rust, with the standard library only; the tools need Python 3, with its standard library.  `cargo`
and the commands below run in `solver/noframe`.

```
cargo test --release                       # brute-force checks of every bound and search (about a minute)
cargo build --release
./target/release/span --from 2 --to 100000 --threads 6 --seconds 540 --out OUT.jsonl     # resumable; ~10 min
./target/release/span --from 101000 --to 1000000 --step 1000 --threads 6 --seconds 540 --out OUT.jsonl
./target/release/star --from 2 --to 20000 --threads 6 --seconds 540 --out OUT.jsonl
./target/release/exact --from 2 --to 1300 --threads 3 --seconds 540 --fix-low 0 --checkpoints work/fresh --out OUT.jsonl
                                    # with the parity bound, rerun until done (data/exact_parity_2_1300.jsonl; without
                                    # --fix-low 0 the searches start from the roles of Theorem B, the default)
./target/release/exact --from 2 --to 1231 --parity 0 --fix-low 0 --threads 6 --seconds 540 --out OUT.jsonl
                                    # the Franklín–Moniot bound alone (data/exact_2_1231.jsonl, which left open
                                    # searches:)
tar -xzf data/checkpoints/exact_open_searches.tar.gz -C work   # to resume them with the parity bound:
./target/release/exact --from 990 --to 1300 --fix-low 0 --threads 3 --seconds 540 --out data/exact_parity_990_1300.jsonl
cargo run --release --example parityroot 2 1300     # the root's parity bound, checked, for every game
cargo run --release --example paritycheck 2 1300    # one below each recorded score, the search finds it (~30 minutes)
bash tools/adversary_campaign.sh                                                          # data/adversary/
./target/release/optsets --from 2 --to 989 --searched-only 1 --threads 3 --seconds 540 --budget 4000000 \
    --out OUT.jsonl                                     # the games that need a search; rerun until done
./target/release/optsets --from 2 --to 989 --out OUT.jsonl     # then the games at their ceiling and the primes, derived
./target/release/optsets --list 798,799,800 --upper fixed --threads 1 --seconds 540 --out FIXED.jsonl
./target/release/moves --from 2 --to 1300 --seconds 540 --out OUT.jsonl         # one thread; ~7 minutes
EXTRA_ARGS='--fix-low 0' ../tools/cloud/prover.sh 1301 2175   # `exact` over a range in resumable passes, all CPUs,
                                    # then the check (data/exact_parity_1301_2175.jsonl; without EXTRA_ARGS the
                                    # searches start from the roles of Theorem B)
tools/summarize.py check 1301 2175 data/exact_parity_2_1300.jsonl data/exact_parity_1301_2175.jsonl
                                    # every n in 2..2175 proven, scores recorded, and how many proofs rest on Theorem B
./target/release/exact --from 2 --to 2175 --budget 1 --out PRE.jsonl    # no search, only the roles' checks (4 min)
./target/release/upper --from 1301 --to 2175 --out data/upper_1301_2175.jsonl          # one thread, 5 minutes
./target/release/upper --from 2176 --to 3314 --results ../../results/strict.jsonl --results CHAIN.jsonl \
    --out data/upper_2176_3314.jsonl          # CHAIN.jsonl: the EC2 chain's results (21 minutes)
./target/release/upper --from 3315 --to 3440 --results ../../results/strict.jsonl \
    --out data/upper_3315_3440.jsonl          # the committed chain's games 3315 to 3440 (5 minutes)
./target/release/exact --list 1575,1584 --out B.jsonl                   # E6: the sources of layer >= 3 free
./target/release/exact --list 1575,1584 --fix-upper 1 --out D.jsonl     # ... and fixed to U*
tools/summarize.py sources B.jsonl D.jsonl          # E6's table and criterion (its entry for the cloud run)
tools/summarize.py span|star|exact|cost|adversary|optsets|moves FILE...
tools/crosscheck_optsets.py data/optsets_2_989.jsonl --max 57 --budget 120     # theory-free, about 8 minutes
tools/crosscheck_greedy.py data/span_2_100000.jsonl.gz --max 1000
cargo run --release --example inspect 9170 4611     # a jam, its factorizations and bounds
cargo run --release --example exactdiag 990 310049 --parity 1   # the search's counters
```

Every campaign appends one JSON line per game and skips the games already in its output, so a stopped run is resumed by
running it again; `exact` also checkpoints the searches in progress (`work/checkpoints`).  The committed data is gzipped
where large (`gunzip -k` to resume into it).  The commands marked E6 measure whether leaving the sources of layer 3 or
more free costs the prover's search anything, the theory track's measurement E6 ([its
entry](../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md#later-2026-10-the-procedures-of-the-two-runs-from-the-runbook)).

## 8. What remains open

* **Conjectures (★) and (♦)** are certified game by game, not proven.  The sharpest form found is to prove (♦): that
  every rejection of descending greedy on the sources' instance is a GF(2) dependency of what it kept above.  That is a
  statement about cores and closed surfaces mod 2, which the adversarial results tie to the window law with the real
  ranking of the primes.
* **The prover's proofs reach 2175**, where the committed chain ended when they were made; the chain reaches further
  within the frame model.  1301–2175 took 226 thread-hours (section 3), and a longer chain needs more.  What the
  parity bound still misses are the odd cores (THEORY.md, Theorem 10).  Its root gap is 130 points on average at
  1000–1300 and 384 over the searched games of 1301–2175 (125 to 767, with no steady rise), and the cost follows it,
  tenfold the nodes per 210 points.  The conflicts of its heaviest sets are large: 20–45 members at 990.
* **The optimal sets of 990–1300 are not enumerated.**  The enumeration mode works with the parity bound (tested
  against brute force), so `optsets` with it would extend section 5 at about the cost of the proofs.
* **Statement U past 2175.**  The frame model's individual assumptions are no longer open: they are theorems given a
  per-board fact that holds on every board to 8191
  ([THEORY.md](../../docs/THEORY.md#the-frame-model-what-is-proven-and-what-remains), Theorems B and M).  What the
  model's answers rest on is one statement, U: some optimal game plays every source of three or more prime factors that
  setup keeps.  The prover's proofs establish U at every n ≤ 2175.  Beyond, the committed games past 2175 that rest on
  U, by an exhausted search or a repeat, have it uncertified.  The barred parity bounds certify about 85% of those
  sources per game on 2176–3314 and 80% on 3315–3440, but close no game alone ([section
  3](#the-upper-part-game-by-game)).  The rest needs a search that leaves the open sources optional: the certificates of
  U ([DESIGN.md, "Certificates of U"](../docs/DESIGN.md#certificates-of-u)), which superseded the measurement E6 of
  section 7.
* **Whether every optimal game's upper part is U\*.**  Theorem 8 settles the sources' instance alone.  For the whole
  game, a frontier argument would start from an optimal set S whose upper part S_U is not U\*.  Termwise domination
  puts the largest disagreement d in U\* ∖ S_U, and S + d is unplayable, so the question is whether some repair costs
  less than d.  The picks below n/2 are all below d, and every exchange observed between optimal sets turns on numbers
  below n/2 that are maximal factors of members of U\*, which is where to look.  Also observed but unexplained: a
  game's optimal sets all have the same size.  The cheapest next evidence is the other half for the 34 games enumerated
  only with the sources fixed.  Their enumeration costs about what their proofs did (5 to 60 minutes each, some 6
  CPU-hours in all), so it wants `exact`'s pausing and Lagrangian bound with the target lowered by one and the
  enumeration mode on, which would let `exact` return every optimal set with each proof.
