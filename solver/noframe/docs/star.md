# Conjecture (★) at scale, and what protects greedy

Part of [the prover's README](../README.md).

## Conjecture (★) at scale

Conjecture (★) is certified for every game to 100,000, and for 902 sampled games to 2,000,000, by a check of (♦) that
is greedy itself.

### The certificate

For a set of members of a reduction instance, give each member its incidence vector over GF(2): a 1 at each of its
factors.

* **Playable sets are independent.**  A playable set dismantles completely ([the residual theorem
  (P5)](../../../docs/theory/playability.md#the-residual-theorem) and [the core theorem
  (P7)](../../../docs/theory/playability.md#the-core-theorem)).  In a dismantling order x₁, x₂, … each xᵢ has a factor
  that no later member lists, so in any vanishing sum the first member's private coordinate is 1.  So no playable subset
  of a set K is larger than rank(K).  On members with at most two factors this is the size of a largest forest of the
  graph the members make, exact there.  The factors pq, pr and qr of a three-prime member are the edges of a triangle on
  the primes, and the dependencies of such members are closed surfaces mod 2, invisible to counting bounds.
* **(♦) ⟹ (★).**  Suppose every member that descending greedy rejects is a sum of members it kept above it.  Above any
  threshold t, greedy's kept members are independent (playable) and span every member above t, so they are a basis:
  greedy keeps rank(K_{≥t}) members, the most any playable subset has.  That is (★) for every component and every
  truncation.  Since [the greedy algorithm](../../../docs/theory/primer.md#the-greedy-algorithm) keeps a heaviest
  independent set of vectors for every weighting, it is also [the sources conjecture
  (S1)](../../../docs/theory/sources.md#the-sources-conjecture) for every weighting consistent with the order.
* **Checking (♦) is greedy itself** (`src/span.rs`).  A member with a factor no kept member lists is kept ([the
  extension lemma (P6)](../../../docs/theory/playability.md#the-extension-lemma)).  A member the kept ones span is
  rejected, since dependent sets are unplayable.  Only the rest are peeled, and a rejection among them would be a
  violation.  Everything runs on the components of the residual, where all cores and all dependencies live.

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

## Adversarial search: what protects greedy

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
20 instances each: traps need three-prime jams, and [the theory](../../../docs/theory/sources.md#where-exchange-breaks)
finds the first failure of exchange at n = 2873.

This sharpens [what a proof must use](../../../docs/theory/sources.md#what-a-proof-must-use), that the order of the
integers protects descending greedy and not their sizes:

* **The window law suffices; the rest of the order does not matter.**  Every order of the real incidence structure that
  keeps the members sharing a factor in the order of their cofactor primes, random or chosen adversarially, satisfied
  (♦).  Uniformly random orders fail often.
* **But only for the right ranking of the primes.**  The window law with respect to a random ranking of the primes
  fails as often as a random order, and so does the window law with respect to the primes perturbed by more than a few
  percent.  With respect to the primes reversed, it holds.  What protects greedy is that the order of the members
  sharing a factor agrees with the ranking of the primes that also decides which products lie in the window (n/2, n].
  Generalized primes, which rebuild the members and factors from the perturbed primes, never fail, even perturbed by
  50%.
* So [the sources conjecture](../../../docs/theory/sources.md#the-sources-conjecture) can be sharpened to a statement
  without magnitudes beyond the window: **(♦) for multiplicatively generated instances under any order obeying their
  window law.**
