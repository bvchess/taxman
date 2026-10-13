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
| the frame model as a whole: `unzip`'s mandatory moves, free factor count, closed hives inert, one-level promotions, greedy removal | brute force (model-free) to n = 62; agreement with the Java's optimal.json, made by the same model, to 1000 | **opt(n) proven equal to the recorded score, model-free, for every n = 2 .. 986** (and 989 of the games to 998) | verified (exact, model-free) |

What the evidence is, in one paragraph each:

* **A linear-algebra certificate for (★).**  A playable set's incidence vectors over GF(2) are linearly
  independent (dismantle it: each member has a factor no later member lists).  If every member descending
  greedy rejects is a GF(2) combination of members it kept above it — call this **(♦)** — then greedy's kept
  set is a basis at every threshold, so nothing playable is larger: (★) for every component and truncation at
  once, and greedy optimal for every weighting consistent with the order.  (♦) is cheap to check exactly (game
  2,000,000 takes 2.7 minutes), and it holds in every game checked.
* **A model-free exact search for opt(n).**  A branch and bound over which numbers are picked, bounded by the
  Franklín–Moniot matching relaxation (re-optimized incrementally from the parent node's optimal dual),
  branching on minimal unplayable sets found by peel, with forced picks from the dual and learned nogoods; with
  the proven relations opt(n) ≤ n + opt(n−1) and the prime formula, it proves the recorded optimum for every
  n ≤ 986, sixteen times the old brute-force reach.
* **An adversarial search** over Taxman-shaped synthetic instances locates what protects greedy: the window
  law, taken with respect to the same ranking of the primes that decides which products fall in the window.

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
removes everything), the Franklín–Moniot matching bound, opt(n) ≤ n + opt(n−1), and the prime formula's upper
bound (a game playing a prime n plays it first; swapping it for the largest prime below n gives a game of
n − 1).  It knows nothing of hives, frames, greedy, `unzip` or promotions.  The search:

* **Bound**: the maximum-weight matching between numbers and their maximal factors, with forced picks matched
  downward and barred numbers only paying, solved once by successive shortest paths and then re-optimized in
  each child from the parent's optimal matching and dual (each freed vertex repaired by one Hungarian step),
  which made the search 160 times faster than solving each node from scratch.
* **Branching**: if the matching's picks are playable they are a game worth the bound; otherwise a minimal
  unplayable subset (deletion filter with peel) must lose a member, and the children partition on which.
* **Forced picks** (from the dual: barring the pick alone already brings the bound to the target) and
  **nogoods** (every minimal unplayable set found, propagated and reused).
* **Pausing**: a search still running at the end of a run saves its open subtrees and nogoods and resumes in the
  next run, so a game can take any number of ten-minute runs.

The target is the recorded optimum V(n); the search proves that no playable set beats it, and the recorded game
replays by the rules to V(n), so opt(n) = V(n).  Results (`data/exact_2_1231.jsonl`, `tools/summarize.py
exact`):

| | games |
|---|---|
| matching bound already equals V(n) | 45 |
| proven by the search | 474 |
| chain relation V(n) = n + V(n−1), predecessor proven | 313 |
| prime relation, predecessor proven | 153 |
| **every n in 2 .. 986 proven** | **985** |

The searches took 3.9 hours of CPU in all; the cost is far from monotone in n (the matching bound's gap is
400–900 points from n = 500 on, and the hardest games are those whose optimum it overrates most, such as
798–804 at 7–12 million nodes, and 987–993).  So the frame model's answer is the true optimum for every
n ≤ 986: sixteen times the previous model-free reach (62), and evidence for every frame-model assumption at
once.

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

## 5. Reproducing

Rust (std only; `cargo` from this directory), Python 3 standard library for the tools.

```
cargo test --release                       # brute-force checks of every bound and search (about a minute)
cargo build --release
./target/release/span --from 2 --to 100000 --threads 6 --seconds 540 --out OUT.jsonl     # resumable; ~10 min
./target/release/span --from 101000 --to 1000000 --step 1000 --threads 6 --seconds 540 --out OUT.jsonl
./target/release/star --from 2 --to 20000 --threads 6 --seconds 540 --out OUT.jsonl
./target/release/exact --from 2 --to 1231 --threads 6 --seconds 540 --out OUT.jsonl    # rerun until done
bash tools/adversary_campaign.sh                                                          # data/adversary/
tools/summarize.py span|star|exact|adversary FILE...
tools/crosscheck_greedy.py data/span_2_100000.jsonl.gz --max 1000
cargo run --release --example inspect 9170 4611     # a jam, its factorizations and bounds
cargo run --release --example exactdiag 600 112623  # the model-free search's counters
```

Every campaign appends one JSON line per game and skips games already in its output, so a stopped run is resumed
by running it again; `exact` also checkpoints searches in progress (`work/checkpoints`).  The committed data is
gzipped where large (`gunzip -k` to resume into it).

## 6. What remains open

* (★) and (♦) are certified game by game, not proven.  The sharpest form found: **prove (♦)**, that every
  rejection of descending greedy on an upper-half instance is a GF(2) dependency of what it kept above — a
  statement about cores and closed surfaces mod 2 that the adversarial results tie to the window law with the
  real ranking of the primes.
* The model-free cross-check stops at n = 986 for now; games 987–993 and 995–1000 need tens of millions of nodes
  each and are being resumed; past 1000 the matching bound's looseness grows.  A stronger relaxation (the
  matching bound misses the sweep of every divisor; the upper half's own GF(2) structure is not used) is the
  next lever.
* The frame model's individual assumptions other than the greedy removal are tested only end to end (through
  the optimum), not one by one.
