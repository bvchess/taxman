# Evidence for the assumptions behind "optimal"

The solver's optimal games (`rust/results`, `src/main/resources/optimal.json`) are optimal *within the frame
model*, whose assumptions are validated, not proven ([DESIGN.md §2](../DESIGN.md#what-optimal-means)), and the
frame model's handling of the numbers above n/2 rests on an open conjecture, the upper-half optimality
conjecture, equivalently conjecture (★) ([THEORY.md](../../heuristics/THEORY.md#the-open-problem)).  This crate
collects empirical evidence for those assumptions, as much and as independent as could be had cheaply.  It
shares no code with the solver or with the Python heuristics; it rests only on THEORY.md's theorems (Theorem 1
with peel as the playability oracle, and the residual theorems) and on elementary bounds, and every exact
method in it is tested against brute force.

| assumption | before this work | now | grade |
|---|---|---|---|
| (★): descending greedy is maximum-cardinality on every component and truncation of the upper-half instance, hence U\* is the heaviest playable upper set (the upper-half conjecture) | 3,231 of 3,236 rejection-bearing components, n ≤ 1500 and 40 samples to 4000, plus n = 2873 and 9170; 5 undecided (n = 3103, 3872, 3936, 4000, 9170); scripts not kept | **every game n = 2 .. 100,000** (602,777,907 greedy rejections) and **902 games to 2,000,000** (60,183,674 more): every rejection a GF(2) linear dependency, which certifies (★) for the whole game; the five undecided components settled; no counterexample | certified per game (a bound met), open in general |
| greedy removal of impossible sources in hives (`removeImpossible`) | agreement with optimal.json to 1000 | at the root (where it shapes the search space) it *is* descending greedy on each upper-half component, so it is certified with (★) for every n ≤ 100,000 | certified per game |
| the frame model as a whole: `unzip`'s mandatory moves, free factor count, closed hives inert, one-level promotions, greedy removal | brute force (model-free) to n = 62; agreement with the Java's optimal.json, made by the same model, to 1000 | **opt(n) proven equal to the recorded score, model-free, for every n = 2 .. 1300**, the whole committed chain (300 of them past the Java's 1000, the Rust solver's own results) | verified (exact, model-free) |
| uniqueness: one optimal set per game, and every optimal game's upper part U\* | the unique heaviest upper set by brute force to n = 62; one optimal game recorded per n | **U\* is the unique heaviest playable upper set wherever (★) holds** (Theorem 8), so to 100,000; every optimal set enumerated, model-free, in 954 of the games 2..989 (the other 34 with the upper half fixed): **the optimal set is unique for every n ≤ 424, not at 425**; 30 games have two or three; every optimal set found has upper part U\* | proven (upper half, given (★)); measured (whole game) |
| the longest games: an optimal game plays the most moves a game of the board can (the wiki's open question; `revise_freedoms` assumes it for a two-frame game) | open; believed true (the wiki), unproven (a comment in the Java) | **false**: at n = 833, 834 and 835 every optimal game is one move short of the longest; every other game to 1300 is a longest game, the gap never above one | refuted (model-free); measured to 1300 |

What the evidence is, in one paragraph each:

* **A linear-algebra certificate for (★).**  A playable set's incidence vectors over GF(2) are linearly
  independent (dismantle it: each member has a factor no later member lists).  If every member descending
  greedy rejects is a GF(2) combination of members it kept above it — call this **(♦)** — then greedy's kept
  set is a basis at every threshold, so nothing playable is larger: (★) for every component and truncation at
  once, and greedy optimal for every weighting consistent with the order.  (♦) is cheap to check exactly (game
  2,000,000 takes 2.7 minutes), and it holds in every game checked.
* **A model-free exact search for opt(n).**  A branch and bound over which numbers are picked, bounded by the
  Franklín–Moniot matching relaxation (re-optimized incrementally from the parent node's optimal dual) and by the
  *parity bound*: every playable set is independent in two linear matroids over GF(2), split by the parity of Ω
  (THEORY.md, Theorem 10), and their heaviest common independent set, certified by a weight split, overrates the
  optimum by a tenth of what the matching does.  It branches on minimal unplayable sets found by peel, with forced
  picks from the duals and learned nogoods; with the proven relations opt(n) ≤ n + opt(n−1) and the prime formula,
  it proves the recorded optimum for every n ≤ 1300, the whole committed chain (the matching bound alone had
  reached 989).
* **Every optimal set.**  The same search, run in an enumeration mode one below the optimum, lists every optimal
  set of a game for about the cost of proving the optimum.  The optimal set is unique for every n ≤ 424; from 425
  some games have two or three, always with the same upper part, U\*, and differing by an exchange of lower-half
  numbers of equal sum.  The upper half's uniqueness is a theorem wherever (★) holds; the whole game's is not.
* **An adversarial search** over Taxman-shaped synthetic instances locates what protects greedy: the window
  law, taken with respect to the same ranking of the primes that decides which products fall in the window.
* **The longest games.**  A playable set pays each pick with a distinct maximal factor, so no game has more moves
  than the matching number of the board's maximal-factor graph; the recorded optimal game meets it in 1,293 of the
  games to 1300, and the same search with unit weights decides the others.  Game 833 has a game of 363 moves,
  while every optimal game has 362: the optimum trades five light picks below n/2 for four heavy ones, a move for
  29 points.  So an optimal game need not be a longest one, and pruning by move counts would be unsound.

## 1. What existed before

* **Proven** (THEORY.md): playability ⟺ maximal-factor matching with acyclic precedence (Theorem 1); peel decides
  it (Theorems 2, 3); playable ⟺ no self-covering subset, and minimal exclusion sets lie in the residual, for
  separated instances (Theorems 4–6); greedy is weight-optimal wherever the frontier jam is hyperedge-free; the
  bounds opt(n) ≤ n + opt(n−1), the prime formula, and the Franklín–Moniot matching bound.
* **(★) / the upper-half conjecture.**  THEORY.md's "Evidence": 3,231 of 3,236 rejection-bearing components
  (all n = 6..1500, forty samples to 4000, n = 2873 and 9170) verified by exact search over the residual or
  certified by the matching bound; five undecided (n = 3103, 3872, 3936, 4000, 9170, search budgets 2.7×10⁹ to
  1.1×10⁴⁷), the last "to be settled only by a theorem".  Exhaustive U\* = max-playable for n = 2..99.  The 3-prime
  padded-jam census (428 shapes, 22,909 trap orderings, zero unprotected).  `heuristics/evaluation/verify.py`:
  U\* equals the optimal game's upper half for n ≤ 1000.  The scripts and data of the (★) campaign and the census
  were never committed (only their numbers, in THEORY.md and the commit messages of e04f7c4, 20822ea and
  43936d3), so that evidence could not be reproduced; this crate's checks can.  (The uncertified 0.2% were
  exactly those five components.)
* **The frame model** ([DESIGN.md §2](../DESIGN.md#what-optimal-means)): its assumptions (greedy removal of
  impossible sources, `unzip`'s mandatory moves, free factor count = factors − sources, closed hives inert,
  promotions one level down) were validated end to end: the Rust solver reproduces the Java's optimal.json to
  1000 (made by the same model, so not independent), `src/brute.rs` matches it to 40 in its tests (52 in
  `examples/sacrifice validate`), and `heuristics/evaluation/certify.py`'s model-free brute force confirms it to
  n = 62.  Beyond 1000 (`rust/results`), nothing model-free.

## 2. Conjecture (★) at scale

### The certificate

For a set of members of a reduction instance, give each member its incidence vector over GF(2): a 1 at each of
its factors.

* **Playable sets are independent.**  A playable set dismantles completely (Theorems 4, 5); in a dismantling
  order x₁, x₂, … each xᵢ has a factor that no later member lists, so in any vanishing sum the first member's
  private coordinate is 1.  So no playable subset of a set K is larger than rank(K).  On members with at most two
  factors this is the graphic-matroid rank, exact there; on three-prime members, whose factors pq, pr, qr are
  the edges of a triangle on the primes, dependencies are closed surfaces mod 2, invisible to counting bounds.
* **(♦) ⟹ (★).**  Suppose every member that descending greedy rejects is a sum of members it kept above it.
  Above any threshold t, greedy's kept members are independent (playable) and span every member above t, so
  they are a basis: greedy keeps rank(K_{≥t}) members, the most any playable subset has.  That is (★) for every
  component and every truncation, and (greedy on a matroid) the upper-half conjecture for every weighting
  consistent with the order.
* **Checking (♦) is greedy itself** (`src/span.rs`): a member with a factor no kept member lists is kept (the
  extension lemma); a member the kept ones span is rejected (dependent sets are unplayable); only the rest are
  peeled, and a rejection among them would be a violation.  Everything runs on the components of the residual,
  where all cores and all dependencies live.

(♦) is stronger than (★): a rejection forced by an "odd" core (self-covering, but no factor-even subset) would
violate it without breaking (★).  Such cores exist in Taxman (the exchange failure at n = 2873 needs one), but
descending greedy has never rejected because of one.

### Coverage

| range | games | greedy rejections | violations of (♦) | largest residual component | CPU |
|---|---|---|---|---|---|
| n = 2 .. 100,000, every game | 99,999 | 602,777,907 | 0 | 8,850 members | 61 min |
| n = 101,000 .. 1,000,000 every 1000th, 1,500,000, 2,000,000 | 902 | 60,183,674 | 0 | 159,870 members | 3.7 h |

`data/span_2_100000.jsonl.gz`, `data/span_samples_101000_2000000.jsonl.gz`: one line per game (residual, kept
set's size and sum, rejections, how many the span certified, violations).

The threshold-by-threshold check of (★) (`src/star.rs`), which decides each rejection threshold's new jam by
the cheapest sufficient argument, ran on every game n = 2 .. 20,000 (`data/star_2_20000.jsonl.gz`):

| argument | thresholds | share |
|---|---|---|
| one exclusion is always needed (the jam is a nonempty residual) | 295,020 | 1.2% |
| matching bound \|P\| − ν(P) | 20,730,083 | 86.2% |
| Lorea's hypergraphic-matroid bound | 2,004,572 | 8.3% |
| GF(2) incidence-rank bound | 1,028,755 | 4.3% |
| exact search needed | 0 | |
| **total** | **24,058,430** | |

By component (the unit of the old campaign: 58,306 rejection-bearing components, 3,119 of them in n ≤ 1500), the
hardest argument needed was the matching bound (or one exclusion) for 21,015, Lorea's bound for 19,798 and the
rank bound for 17,493.  The five components left undecided before all fall to the rank bound: n = 9170's jam
(P = 304 members, greedy's 37 exclusions; the matching bound proves only 19) is certified in milliseconds.

Before the rank bound was found, the exact searches (`src/maxcard.rs` from scratch; `src/plus1.rs`, which
proves each threshold's "+1" given the earlier thresholds, with the laminar family of earlier jams as cardinality
constraints and cores as learned clauses) had pushed n = 9170 to its first 16 thresholds (P = 127, g = 13); both
engines stay as the fallback for a violation of (♦), and are checked against brute force and against each
other on random instances where greedy fails.

Cross-checks: the Rust greedy equals `heuristics/core.py`'s `optimize_mini` on every n ≤ 1000 and 62 samples to
6000, and its kept set's sum equals the upper half of optimal.json's game for every n ≤ 1000
(`tools/crosscheck_greedy.py`).

## 3. Model-free exact cross-check of opt(n)

`src/game.rs` proves upper bounds on opt(n) from proven facts only: Theorem 1 with peel (a set of picks is
playable iff dismantling the instance of the picks, each listing its maximal factors that are not picks,
removes everything), the Franklín–Moniot matching bound, the parity bound (THEORY.md, Theorem 10),
opt(n) ≤ n + opt(n−1), and the prime formula's upper bound (a game playing a prime n plays it first; swapping it for
the largest prime below n gives a game of n − 1).  It knows nothing of hives, frames, greedy, `unzip` or
promotions.  The search:

* **Bound**: the maximum-weight matching between numbers and their maximal factors, with forced picks matched
  downward and barred numbers only paying, solved once by successive shortest paths and then re-optimized in
  each child from the parent's optimal matching and dual (each freed vertex repaired by one Hungarian step),
  which made the search 160 times faster than solving each node from scratch.
* **The parity bound** (`src/parity.rs`, `Prover::with_parity`; on in `exact` since 2026-10).  Every playable set
  is independent in two linear matroids over GF(2), split by the parity of Ω (THEORY.md, [the parity
  matroids](../../heuristics/THEORY.md#the-parity-matroids-a-bound-on-the-whole-game)), so at a node the heaviest set
  independent in both, with the forced picks contracted and the barred numbers deleted, bounds every completion.
  It is computed by shortest augmenting paths (a child starting from its parent's set and weight split) and reported
  only with a weight split that satisfies the local optimality conditions of both matroids, checked explicitly, so
  the bound rests on the theorem and weak duality, not on the algorithm.  It cuts the node, bars the free numbers
  that depend on the forced picks, forces the members whose barring alone brings it to the target, and the node
  branches on its heaviest set's conflicts instead of the matching's.  Its gap is about a tenth of the matching
  bound's (130 points against 1,200 at n = 1000-1300).
* **Branching**: if the bound's picks are playable they are a game worth the bound; otherwise a minimal
  unplayable subset (deletion filter with peel) must lose a member, and the children partition on which.
* **Forced picks** (from the dual: barring the pick alone already brings the bound to the target), the
  **conflict-packing cut** (nogoods violated by the matching whose members are far apart in the graph must each
  be broken, and their dual costs add), and **nogoods** (every minimal unplayable set found, propagated and
  reused).
* **A Lagrangian bound** over the nogoods: a playable set picks at most |K| − 1 members of each nogood K, so a
  second matching, with each pick's weight lowered by the multipliers of the unbroken nogoods holding it (fitted
  by subgradient steps at the search's root, then fixed), bounds the search too; it runs alongside the first,
  repaired incrementally the same way, and cut the tree 3.3-fold at n = 793.  Beside the parity bound it cut
  nothing at 990, so `exact` uses it only without the parity bound.
* **Pausing**: a search still running at the end of a run saves its open subtrees and nogoods and resumes in the
  next run (depth first), so a game can take any number of ten-minute runs.

The target is the recorded optimum V(n); the search proves that no playable set beats it, and the recorded game
replays by the rules to V(n), so opt(n) = V(n).  Every rule above is tested against brute force or against the
optimum (the search must still reach optimal.json's score from one below it, with each rule on, and the parity
bound's intersection is checked against enumeration on random pairs of matroids).

### With the matching bound (to 989)

`data/exact_2_1231.jsonl`:

| | games |
|---|---|
| matching bound already equals V(n) | 45 |
| proven by the search | 487 |
| chain relation V(n) = n + V(n−1), predecessor proven | 321 |
| prime relation, predecessor proven | 156 |
| **every n in 2 .. 989**, and 999 .. 1019 | **1009** |

The searches took 10.0 hours of CPU in all.  The cost is far from monotone in n: the matching bound's gap is
140–430 points around n = 500–700 and 650–1200 from 800 on, and the hardest games are those whose optimum it
overrates most: 798–804 at 7–12 million nodes, 987–989 at 19–53 million.  Games 990, 992, 993, 995 and 996 took
50–70 million nodes each, and 1020 and 1023–1027 several million, without finishing; their searches are saved in
`data/checkpoints/exact_open_searches.tar.gz`.

### With the parity bound (to 1300)
### With the parity bound (to 1300)

`data/exact_parity_990_1300.jsonl` continues that file: the open searches resumed with the parity bound, then every
game from 1028 to 1300, then 999–1019 again.  The open searches finished in 4,000 to 215,000 more nodes each (17 to
595 seconds; 990: 89,639 nodes after its 70.5 million, 992: 214,329 after 65.2 million), and with them 991, 994, 997
and 998 by the chain and prime relations.  With the two files (`tools/summarize.py exact data/exact_2_1231.jsonl
data/exact_parity_990_1300.jsonl`):

| | games |
|---|---|
| matching bound already equals V(n) | 45 |
| proven by a search (to 989 with the matching bound alone, from 990 with the parity bound) | 649 |
| chain relation V(n) = n + V(n−1), predecessor proven | 407 |
| prime relation, predecessor proven | 198 |
| **every n in 2 .. 1300**, the whole committed chain | **1299** |

The parity searches of this file took 2.7 hours of thread time (three threads; the warm starts came in for its last
96 games), the hardest 1287 at 146,855 nodes and 279 seconds.  So the frame model's answer is the true optimum for
every game of the committed chain, n ≤ 1300, including the Rust solver's own results past the Java's 1000.

`data/exact_parity_2_1300.jsonl` proves every game again from scratch with the parity bound (no resumed search, no
matching-bound proof needed; the final build, two threads): a second, self-standing proof of every n ≤ 1300 by a
different bound, and the cost curve below.  And no rule cuts an optimal game: for every game that needs a search,
the same search with the target one below V(n) finds a set worth V(n), which replays by the rules
(`examples/paritycheck.rs`; all 665 such games to 1300, `data/paritycheck_2_1300.jsonl`, 31 minutes).

### What a proof costs, and how far it reaches

`tools/summarize.py cost data/exact_parity_2_1300.jsonl`: the 557 games that need a search, proven from scratch with
the parity bound in 1.17 thread-hours (two threads; about 1.6 ms a node from 990 on, the warm starts included).

| n | searches | the root's parity gap (mean) | median nodes | most nodes | seconds, all searches |
|---|---|---|---|---|---|
| 2–499 | 127 | 3–24 | ≤ 21 | 83 | 2 |
| 500–799 | 154 | 44–78 | 74–402 | 2,673 | 44 |
| 800–899 | 57 | 86 | 382 | 5,474 | 55 |
| 900–999 | 53 | 164 | 2,406 | 68,260 (996) | 611 |
| 1000–1099 | 54 | 120 | 2,358 | 91,928 (1023) | 974 |
| 1100–1199 | 56 | 111 | 789 | 41,153 (1199) | 264 |
| 1200–1300 | 56 | 169 | 18,247 | 146,855 (1287) | 2,276 |

The cost follows the root's parity gap, as the solver's follows its root gap: over 900–1300 a search's nodes grow
×10 per 84 points of gap (1.19 ± 0.05 decades per 100 points, R² 0.76), and the gap grows about 14 points per 100 in
n with swings of ±50 between bands (the matroid study's measurement, which these roots reproduce).  Against n, log10
nodes grows 0.16 ± 0.04 and log10 seconds 0.23 ± 0.04 per 100 in n over 900–1300 (R² 0.06 and 0.11: the gap's swings
dominate), 0.33 and 0.34 from n = 78 on.  The matching-bound search had spent 50–70 million nodes on each of
990–996 without finishing; the parity bound proves them from scratch in 44,000–68,000.

1300 is not the limit.  At the measured 0.23 decades per 100 in n, from the 2,240 seconds of 1200–1299, the
searches of 1300–1399 would take about an hour of thread time, those to 1500 about 3 hours, to 2000 about 57 hours (a
day on three threads), with the hardest single proofs near 2000 about 3 hours each, which checkpointing handles.  What
stops the proofs at 1300 is the recorded optima: past the committed chain there is no V(n) to prove, so going further
needs the solver's chain first (or the search's own `optimize` mode, which finds opt(n) instead of checking it, at a
higher cost).

## 4. Adversarial search: what protects greedy

`src/adversary.rs` builds Taxman-shaped instances, each keeping one thing from the real game and varying
another, and checks (♦) and, where it fails, (★) threshold by threshold (`tools/adversary_campaign.sh`,
`data/adversary/`):

| family | instances | (♦) fails | (★) fails |
|---|---|---|---|
| real upper halves, n = 2000..20,000 step 100 | 181 | 0 | 0 |
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

("≥": some thresholds stayed undecided within the search budget.)  At n = 1000 every family passed (20
instances each): traps need three-prime jams, and THEORY.md finds the first failure of exchange at n = 2873.

What this says about the protection, sharpening THEORY.md's "the ordering is the protection, not the
magnitudes":

* **The window law suffices; the rest of the order does not matter.**  Every order of the real incidence
  structure that keeps members sharing a factor in the order of their cofactor primes — random, or chosen
  adversarially — satisfied (♦), while uniformly random orders fail often.
* **But only for the right ranking of the primes.**  The window law with respect to a random ranking of the
  primes fails as often as a random order, and with respect to the primes perturbed by more than a few percent
  it fails too; with respect to the primes reversed it holds.  What protects greedy is that the order of the
  members sharing a factor agrees with the ranking of the primes that also decides which products lie in the
  window (n/2, n]: generalized primes, which rebuild the members and factors from the perturbed primes, never
  fail, even perturbed by 50%.
* So the open problem can be sharpened to a statement without magnitudes beyond the window: **(♦) for
  multiplicatively generated instances under any order obeying their window law.**

## 5. Is the optimal set unique?

Many move orders play the same set of picks; the question is whether different *sets* can be optimal.  If the
optimal set were unique, or unique up to a well-described exchange, a solver could lean on it: to reuse the game
of n − 1 when solving n, to justify shortcuts, to break symmetry.

### What is proven

THEORY.md, ["Uniqueness of the optimal set"](../../heuristics/THEORY.md#uniqueness-of-the-optimal-set):

* **Theorem 8: the heaviest playable upper set is unique wherever (★) holds**, so in every game n ≤ 100,000
  (section 2).  More precisely, every playable upper set S is dominated termwise by U\*: the i-th largest member
  of S is at most the i-th largest of U\*, so S weighs strictly less unless S = U\*.  (The playable sets are not a
  matroid, but under (♦) they lie inside the GF(2) matroid of which U\* is the greedy basis, and a matroid with
  distinct weights has one heaviest basis.)  This is about the upper-half instance alone.  It does *not* say that
  every optimal game's upper part is U\*: a game with a lighter upper part could in principle win the difference
  back below n/2.
* **The prime of every optimal set**: every optimal set of n holds the largest prime q ≤ n and no other prime.
* **Theorem 9: optimal sets from the game before.**  If opt(n) = n + opt(n − 1) (a *chain* game), the optimal
  sets of n are exactly the playable T ∪ {n}, T optimal for n − 1.  If n is prime, T ↦ T − p + n (p the largest
  prime below n) is a bijection from the optimal sets of n − 1 onto those of n.  So a game has as many optimal
  sets as its predecessor at a prime, no more at a chain game, and only a composite game meeting neither relation
  can have more.
* **No matroid relaxation is exact for the whole game**: game 425 has two optimal sets (below), and a matroid
  with distinct positive weights has one heaviest independent set.

### Method

`src/bin/optsets.rs` lists every optimal set of a game, model-free, with the branch and bound of section 3 in an
enumeration mode (`Prover::enumerate`): every pruning rule of the search cuts only sets worth at most its target,
so after a playable set P is found the search goes on, partitioning the rest of P's node by the first number on
which a set disagrees with P.  With target V(n) − 1, the recorded optimum minus one, it lists every playable set
worth at least V(n), so every optimal set (and would stop on one worth more, refuting V(n)).  Games meeting the
chain or prime relation take their sets from the predecessor's by Theorem 9 (`derive_optimal_sets`).  Optionally
(`--upper fixed` / `other`) the question splits in two: the optimal sets whose upper part is U\* (a search with
the upper half fixed, which is cheap), and those whose upper part is not (a search whose root is split on the
largest upper number where the set and U\* disagree, which costs as much as a proof of optimality).

Checks: the enumeration equals brute force on every n ≤ 19 at the optimum and below it (where sets containing a
found one also qualify); the derivations equal direct enumeration on every chain and prime game to 700 (and to 260
in the test suite); `tools/crosscheck_optsets.py` compares the lists with `heuristics/evaluation/certify.py`'s
theory-free brute force over raw pot states, which agrees on every n ≤ 57; both optimal sets of 425 replay by the
rules (`heuristics/core.py`'s ordering and `check_sequence`); every optimal set found holds the largest prime and
no other.

Enumerating at V(n) − 1 costs about what the proof at V(n) costs: over 68 games whose proof took at least 10,000
nodes, the ratio runs from 0.94 to 1.20, 4% more in total.  So a model-free proof of opt(n) can return the complete
list of optimal sets almost for free.

### Results

`data/optsets_2_989.jsonl` (every game 2..989; `tools/summarize.py optsets`):

| n | games | enumerated completely | 1 optimal set | 2 sets | 3 sets | upper half fixed only |
|---|---|---|---|---|---|---|
| 2..424 | 423 | 423 | 423 | | | |
| 425..500 | 76 | 76 | 64 | 12 | | |
| 501..600 | 100 | 100 | 99 | | 1 | |
| 601..700 | 100 | 100 | 92 | 6 | 2 | |
| 701..800 | 100 | 97 | 88 | 9 | | 3 |
| 801..900 | 100 | 96 | 96 | | | 4 |
| 901..989 | 89 | 62 | 62 | | | 27 |
| **total** | **988** | **954** | **924** | **27** | **3** | **34** |

Searches with a budget of 4 million nodes (each under about seven minutes) covered every game whose optimality
proof had taken fewer than 3.6 million; the chain and prime games were derived.  The 34 others are the most
expensive proofs of section 3 (798–804, 945–954, 969–983, 987–989) and the chain and prime games that follow
them; their lines record the budget each was given (4 million nodes for 798–800 and 803–804, a short 300,000
for the others, which were not attempted in full).  For them `data/optsets_fixed_798_1019.jsonl` lists the optimal sets whose upper part is U\* (the upper
half fixed: 10 seconds to 2 minutes each), and so it does for 990–1019.  Each of those 34 games, and 990, 991
and 999–1019, has exactly one optimal set with upper part U\* (57 games); for 992–998, whose optima the frame
model's search also reaches late, the fixed-upper search spent its 2.5 or 4 million nodes without reaching even
the recorded set.

The games with more than one optimal set fall in six clusters, each started by an exchange new at its first game
and carried by the chain and prime relations (and by some games meeting neither):

| games | sets | exchange (canonical set's numbers ↔ the other set's) | sum | size |
|---|---|---|---|---|
| 425–428 | 2 | {110, 132, 168} ↔ {98, 150, 162} | 410 | 3 ↔ 3 |
| 432–439 | 2 | {132, 147, 180, 208} ↔ {156, 160, 162, 189} | 667 | 4 ↔ 4 |
| 539 | 3 | {128, 168, 240} ↔ {120, 192, 224}; {120, 154, 189, 220, 234} ↔ {147, 180, 182, 198, 210}; the two non-canonical sets are 7 ↔ 7 apart | 536, 917, 1333 | 3, 5, 7 |
| 684–687 | 3, 3, 2, 2 | {44, 198, 238, 325} ↔ {68, 196, 266, 275}; {44, 238, 252, 260, 294} ↔ {68, 234, 250, 266, 270}; {198, 234, 250, 270, 325} ↔ {196, 252, 260, 275, 294} | 805, 1088, 1277 | 4, 5, 5 |
| 696–699 | 2 | {198, 234, 250, 270, 325} ↔ {196, 252, 260, 275, 294} (one of 684's) | 1277 | 5 ↔ 5 |
| 770–773, 775–779 | 2 | {228, 231, 252, 260, 280, 320, 325} ↔ {189, 250, 270, 273, 275, 315, 324} | 1896 | 7 ↔ 7 |

(At 684–685 the third exchange is between the two non-canonical sets; 774 has one optimal set, and 775 brings
770's exchange back.)  No completely enumerated game from 780 to 989 has more than one optimal set.

What the data shows (observed, not proven):

* **The optimal set is unique for every n ≤ 424.**  Game 425 is the first with two.  Of the 954 games enumerated
  completely, 30 have several optimal sets (at most three), all between 425 and 779.
* **Every optimal set of every completely enumerated game has upper part U\***, the strong form of
  `heuristics/evaluation/verify.py`'s measurement: no optimal game ever trades upper weight for lower weight.
* **All optimal sets of a game have the same number of members, and differ by an exchange of equally many
  lower-half numbers of equal sum** (3 ↔ 3 up to 7 ↔ 7), every exchanged number in (n/5, 0.49 n) but 44 and 68
  at 684–687.  Every exchanged number is a maximal factor of a member of U\*, so an exchange trades which of those
  factors are picked and which are free to pay for their upper multiples.  At 425, with peel's payments: in one
  set 110, 132 and 168 are picked and 98, 150 and 162 pay for 196, 300 and 324; in the other, 98, 150 and 162 are
  picked and 110, 132 and 168 pay for 330, 396 and 336.
* **The canonical set (the largest in decreasing lexicographic order) survives every chain and prime step**
  observed from a game with several sets: canonical(n) = canonical(n − 1) + n (chain), or − p + n (prime; this
  part is a theorem, the lexicographic order being preserved by the bijection): 13 of 13 such steps.  optimal.json
  records the canonical set in 23 of the 30 games with several sets; not at 684 and 685 (it records the third of
  three) or at 775–779 (the second of two).
* **The recorded game of n − 1 is not always extendable.**  At 686, a chain game, the recorded optimal game of 685
  plus 686 is unplayable; opt(686) = 686 + opt(685) holds through another optimal set of 685 (the canonical one).

### What it buys a solver

* **The upper half.**  Fixing the upper half to U\*, as the frame model does, gives up nothing that ties: U\* is
  the unique heaviest playable upper set (proven given (★), so to 100,000), and every optimal game observed has
  upper part U\*.  The second statement is the one still open in general.
* **Reuse of n − 1 (Theorem 9).**  The prime step is exact: the optimal sets of a prime n are those of n − 1 with
  p swapped for n.  The chain step is decidable from the complete list of n − 1's optimal sets: n meets the bound
  n + opt(n − 1) iff one of them stays playable with n added, and then the optimal sets of n are known without
  search.  With only one recorded set the test is not decisive (685 → 686).  The lists are short (1–3 sets so
  far), and the part with upper part U\* is cheap to get with the upper half fixed: seconds to two minutes per
  game to 1019, except 990–998 (six and seven minutes for 990 and 991, out of budget for 992–998).  That part is
  the whole list wherever every optimal set has upper part U\*, which is observed but not proven.
* **Symmetry breaking.**  A proof of optimality (target V, a set worth more than V) already cuts every tie, so a
  canonical-set rule prunes nothing there.  It matters where ties are accepted: a search for a game reaching a
  cap, or a step reusing the previous game, such as the solver's `reuse_prev`.  There, carrying the canonical set
  (rather than whichever optimal set a search happened to return) is consistent with every chain and prime step
  observed, and well defined.

## 6. Does an optimal game play the most moves?

The wiki's "Questions" page asks it (its third question, left open, with the belief that the answer is yes): does an
optimal game take as many moves as any game of the board can, or does some longer game score less?  If optimal
games were always longest, a solver could stop searching every branch that cannot reach the most moves.  Neither
solver does that (below), but the Java's `reviseFreedoms` rests on a local form of the claim.

### What is proven

THEORY.md, ["Longest games"](../../heuristics/THEORY.md#longest-games).  Let M(n) be the most moves of a game of n,
the size of the largest playable set of picks.

* **M(n) <= ν(n)**, the matching number of the board's maximal-factor graph (each number joined to its maximal
  factors): a playable set pays each pick with a distinct maximal factor outside it (Theorem 1).  So a legal game of
  ν(n) moves is a longest game, a certificate that costs one matching.
* **M(n − 1) <= M(n) <= M(n − 1) + 1**, and **M(n) = M(n − 1) for a prime n >= 3** (the count forms of the chain
  and prime relations).
* Given (★), a playable set has at most |U\*| members above n/2 (Theorem 8's count inequality): a longest game
  cannot gain on an optimal game whose upper part is U\* except below n/2.

### Method

`src/moves.rs` and `src/bin/moves.rs`.  The games are worked in increasing order: M(n) lies between the longer of
the recorded optimal game and the longest game of n − 1, and the smaller of ν(n) (computed for every board at once
by augmenting paths) and what M(n − 1) allows.  Where the bounds meet, M(n) is settled; elsewhere the branch and
bound of section 3, with every pick weighing 1 ([`Prover::weighted`](src/game.rs)), decides it, in a mode that
raises its target to each longer set it finds ([`Prover::optimize`](src/game.rs)), with segment cuts from the
smaller games' bounds.  Where a game longer than the recorded one exists, the heaviest longest game comes from the
same search with the pick of c weighing K + c (K above every sum, so a set's weight orders by moves, then by sum),
started from the heaviest longest game with the recorded game's upper part (the upper half fixed: cheap).  Every
longest game written is replayed by the rules, in the order Theorem 1 gives from peel's payments (`play_order`).
Checks: M(n) and the heaviest longest game against brute force for n <= 22, ν(n) against the matching solver for
every board to 400, and `play_order` against the recorded games to 400.

### Results

`data/moves_2_1300.jsonl` (every game 2..1300, seven minutes on one thread, nearly all of it the three searches for
the heaviest longest game, 2.5 to 2.6 million nodes each; `tools/summarize.py moves`):

| | games |
|---|---|
| the recorded optimal game has ν(n) moves: a longest game by the certificate | 1,293 |
| ν(n) = M(n) + 1, the recorded game a longest one by a search (51, 182, 183; at most 173 nodes) | 3 |
| **the recorded game one move short of M(n) = ν(n)** (833, 834, 835) | **3** |

| n | optimal game: moves (above n/2) | M(n) | heaviest longest game | below the optimum | its exchange of the optimal game's moves below n/2 |
|---|---|---|---|---|---|
| 833 = 7²·17 | 362 (314) | 363 | 218,197 | 29 | {250, 252, 294, 400} out, {116, 180, 200, 320, 351} in |
| 834 | 363 (315) | 364 | 219,031 | 29 | the same (834 is a chain game: both games of 833 plus 834) |
| 835 = 5·167 | 363 (315) | 364 | 219,377 | 29 | the same |

The two games of 833 in full (the optimal one is optimal.json's): above n/2 both play the same 314 moves (U\*);
below it,

```
optimal game (48):   245 250 252 258 261 273 280 288 294 297 300 306 310 315 322 330 333 336 338 342 343 345 350 357
                     363 364 368 369 372 375 376 378 380 385 387 390 392 396 399 400 402 405 406 408 410 412 414 416
longest game (49):   the same with 250 252 294 400 out and 116 180 200 320 351 in
```

Both replay by the rules (in `src/moves.rs`, the test `game_833_has_a_longer_game_than_its_optimal_one`), and the
frame model plays both (`rust/tests/soundness.rs`).  The optimal games of 834 and 835 hold the same four numbers
below n/2, and the same exchange makes their heaviest longest games.

What the data shows:

* **An optimal game need not be a longest one.**  The first counterexample is n = 833: its optimal set is unique
  (section 5) and has 362 members, and a legal game has 363.  The heaviest 363-move game keeps the optimal game's
  upper part, U\*, and exchanges four of its picks below n/2 for five lighter ones, 29 points for one move.  That
  needs nothing of the frame model: the longer game replays by the rules, ν(833) = 363 bounds every game, and the
  search proves no 363-move game worth more than 218,197, less than the recorded game's 218,226.
* **The gap is at most one move to 1300, and one only at 833-835.**  Every other recorded optimal game is a longest
  game, 1,293 of them by the matching certificate alone.  At 836 the optimal game reaches ν again.  Since a game's
  optimal sets all have the same size wherever they were enumerated (section 5), "the optimal game's moves" is well
  defined there; at 833-835 the optimal set is unique.
* **The mechanism is a trade of weight for moves below n/2.**  A pick above n/2 uses one number of the lower half
  (its payment), a pick below uses two (itself and its payment).  At 833 the optimal game's 48 lower picks and 362
  payments use 410 of the 416 lower numbers, the longest game's 49 and 363 use 412.  Four heavy lower picks
  (1,196) outweigh five light ones (1,167).
* **ν(n) is a strong bound on moves.**  It exceeds M(n) only at 51, 182 and 183 (by one, where every maximum
  matching is precedence-cyclic), against the value-weighted matching bound's gap of hundreds of points.

### The solvers and the wiki

* **Neither solver prunes a branch for having fewer moves than possible.**  Counts enter the Java's search only as
  upper bounds.  A frame's *freedoms* are its factors minus its moves (`search/Frame.java:43`).
  `FrameSet.computeMaxNumberOfPromotions` (`search/FrameSet.java:136-151`) turns them into `maxTotalPromotions`, a
  cap on the *number* of promotions (`search/SearchManager.java:121-122`, `maxAdditionalPromotions` at :192-197).
  The upper bound then sums at most that many of the largest candidates, and a branch is cut on that *sum*
  (`search/BranchExplorer.java:77`).  *In the bag* is a score, what the game with no promotions makes
  (`frame/FrameSolver.java:155-160`), not a count.  The Rust's `max_total` (`search/setup.rs`), `max_additional`
  (`bounds.rs`) and `in_the_bag` (`board.rs`) are the same.
* **The belief appears once, in `reviseFreedoms`** (`search/UpperBoundEstimator.java:41-90`).  Its comment states
  as a theorem, without proof, that the promotion set with the largest sum also has the most promotions.  It
  solves the two-frame game of frames 0 and 1 for its heaviest promotion set and lowers frame 0's freedoms to that
  set's size, which can cut sets with *more* promotions into frame 0, never fewer.  The Rust keeps it as the
  tactic `revise_freedoms`, graded unproven and off in strict runs.  (`SearchManager.finalReport`, :185-188,
  reports an optimum with fewer promotions than `maxTotalPromotions` as the cap "over-estimated".)
* **833 refutes the comment's "theorem" for the whole frame game.**  The exchange above is a feasible promotion
  set: the optimal game promotes 48 numbers and the longest 49, which is the frame model's `max_total`
  (`rust/tests/soundness.rs`, `the_heaviest_promotion_set_of_833_is_not_the_largest`), so at 833 the cap was not an
  over-estimate.  It does not decide the two-frame form the tactic uses: at 833 the tactic searched the two-frame
  game (`Searched { size: 21 }`) and left `max_total` at 49, and the scores of 833-835 are proven model-free.

### What it would buy a solver

A sound statement about move counts can only cap them, as `max_total` and ν(n) do, and a cap on counts bounds a sum
only through the largest candidates, which the solver's matching bound already outdoes.  The tempting unsound one,
pruning every branch that cannot reach M(n) moves, would return 218,197 at n = 833 instead of 218,226, and 29 too
little at 834 and 835; in the frame model, pruning the branches that cannot reach `max_total` promotions would do
the same.  The certificate is still useful as a check: a game with ν(n) moves is a longest game, in milliseconds.

## 7. Reproducing

Rust (std only; `cargo` from this directory), Python 3 standard library for the tools.

```
cargo test --release                       # brute-force checks of every bound and search (about a minute)
cargo build --release
./target/release/span --from 2 --to 100000 --threads 6 --seconds 540 --out OUT.jsonl     # resumable; ~10 min
./target/release/span --from 101000 --to 1000000 --step 1000 --threads 6 --seconds 540 --out OUT.jsonl
./target/release/star --from 2 --to 20000 --threads 6 --seconds 540 --out OUT.jsonl
./target/release/exact --from 2 --to 1300 --threads 3 --seconds 540 --checkpoints work/fresh --out OUT.jsonl
                                    # with the parity bound, rerun until done (data/exact_parity_2_1300.jsonl)
./target/release/exact --from 2 --to 1231 --parity 0 --threads 6 --seconds 540 --out OUT.jsonl  # the matching
                                    # bound alone (data/exact_2_1231.jsonl, which left open searches:)
tar -xzf data/checkpoints/exact_open_searches.tar.gz -C work   # to resume them with the parity bound:
./target/release/exact --from 990 --to 1300 --threads 3 --seconds 540 --out data/exact_parity_990_1300.jsonl
cargo run --release --example parityroot 2 1300     # the root's parity bound, checked, for every game
cargo run --release --example paritycheck 2 1300    # one below each optimum, the search finds it (~30 minutes)
bash tools/adversary_campaign.sh                                                          # data/adversary/
./target/release/optsets --from 2 --to 989 --searched-only 1 --threads 3 --seconds 540 --budget 4000000 \
    --out OUT.jsonl                                     # the games that need a search; rerun until done
./target/release/optsets --from 2 --to 989 --out OUT.jsonl     # then the chain and prime games, derived
./target/release/optsets --list 798,799,800 --upper fixed --threads 1 --seconds 540 --out FIXED.jsonl
./target/release/moves --from 2 --to 1300 --seconds 540 --out OUT.jsonl         # one thread; ~7 minutes
tools/summarize.py span|star|exact|cost|adversary|optsets|moves FILE...
tools/crosscheck_optsets.py data/optsets_2_989.jsonl --max 57 --budget 120     # theory-free, about 8 minutes
tools/crosscheck_greedy.py data/span_2_100000.jsonl.gz --max 1000
cargo run --release --example inspect 9170 4611     # a jam, its factorizations and bounds
cargo run --release --example exactdiag 990 310049 --parity 1   # the model-free search's counters
```

Every campaign appends one JSON line per game and skips games already in its output, so a stopped run is resumed
by running it again; `exact` also checkpoints searches in progress (`work/checkpoints`).  The committed data is
gzipped where large (`gunzip -k` to resume into it).

## 8. What remains open

* (★) and (♦) are certified game by game, not proven.  The sharpest form found: **prove (♦)**, that every
  rejection of descending greedy on an upper-half instance is a GF(2) dependency of what it kept above — a
  statement about cores and closed surfaces mod 2 that the adversarial results tie to the window law with the
  real ranking of the primes.
* The model-free cross-check reaches the end of the committed chain, 1300, and could go on (about a day of three
  threads to 2000, section 3) if there were recorded optima to check.  What the parity bound still misses are the
  odd cores (THEORY.md, Theorem 10): its root gap, 130 points on average at 1000–1300, grows about 14 per 100 in n,
  and the cost with it; the conflicts of its heaviest sets are large (20–45 members at 990).
* The optimal sets of 990–1300 are not enumerated.  The enumeration mode works with the parity bound (tested
  against brute force), so `optsets` with it would extend section 5 at about the cost of the proofs.
* The frame model's individual assumptions other than the greedy removal are tested only end to end (through
  the optimum), not one by one.
* Whether every optimal game's upper part is U\*.  Theorem 8 settles the upper half alone; for the whole game, a
  frontier argument would start from an optimal S whose upper part S_U is not U\*: termwise domination puts the
  largest disagreement d in U\* ∖ S_U, and S + d is unplayable, so the question is whether some repair costs less
  than d.  Lower picks are all below d, and every exchange observed between optimal sets turns on lower numbers
  that are maximal factors of members of U\*, which is where to look.  Also observed but unexplained: a game's
  optimal sets all have the same size.  The cheapest next evidence is the other half for the 34 games enumerated
  only with the upper half fixed: their enumeration costs about what their proofs did (5 to 60 minutes each,
  some 6 CPU-hours in all), so it wants `exact`'s pausing and Lagrangian bound with the target lowered by one
  and the enumeration mode on, which would let `exact` return every optimal set with each proof.
