# Taxman Mini: the upper half of a Taxman game in polynomial time

This is a small, clean implementation built to test a theory about the
[Taxman game](https://github.com/bvchess/taxman/wiki/The-Game):

> There is a polynomial-time algorithm for obtaining all of the numbers
> greater than N/2 in the optimal answer, along with the optimal sequence
> for these numbers.

## Result

**The theory holds for every game from N=1 to N=1000.**

Checked against the known optimal solutions in
[`optimal.json`](../src/main/resources/optimal.json) - which were
themselves produced by the frame-based solver, so see
"What is independently certified" below for how much of this is
verified without shared assumptions (everything through n=54,
exhaustively):

```
games checked:        1000
set matches:          1000/1000   (selections > N/2 exactly match the optimal solution)
playable sequences:   1000/1000   (every produced sequence is a legal game)
identical order:      12/1000     (informational; optimal orderings are not unique)
```

## How it works

Every number greater than N/2 is a source node in the maximal factor graph
(no number in the game is a multiple of it), and all of its
[maximal factors](https://github.com/bvchess/taxman/wiki/Definitions#maximal-factor)
are at most N/2.  So

```
C = { c : N/2 < c <= N }
F = union of the maximal factors of the members of C
```

is a bipartite game in the style of
[Taxman Mini](https://github.com/bvchess/taxman/wiki/Taxman-Mini), and the two
procedures from that wiki page apply directly:

* `solve_mini(C, F)` repeatedly either takes a selection with only one
  remaining factor (play it now) or retires a factor needed by only one
  selection (play that selection last).  If neither rule applies, the
  remaining selections form a core in which tax demand exceeds factor
  supply, so not all of C can be selected.
* `optimize_mini(C, F)` considers selections from largest to smallest and
  keeps each one that leaves the accepted set solvable.  It is a
  generalization of the "take the highest prime" strategy.

### Ordering for a real game

One wrinkle: `solve_mini`'s recursion only reasons about maximal factors,
but in a real taxman game a selection sweeps **all** of its remaining
divisors from the pot.  For example, at N=21 the number 16 has the single
maximal factor 8, so `solve_mini` is free to emit it first — but played
first, 16 would also sweep 1, 2, and 4, starving 19, 14, and 12.

`solve_mini` does, however, produce a perfect matching: each selection is
assigned the factor it pays.  A sequence is playable in a real game exactly
when it respects the precedence *"a before b whenever a's assigned factor
divides b"*: then a consumes its factor before b can sweep it, and b cannot
rob a later selection of its factor.  A cycle in that precedence would
require two selections to share an assigned factor, which a matching
forbids, so a topological order always exists (`order_for_real_game`).
This plays the same role as solving the frames front-to-back in the wiki's
[N=21 walkthrough](https://github.com/bvchess/taxman/wiki/Walkthrough-for-N=21).

### Complexity

For a game N with |C| ≈ N/2 selections and E = Σ ω(c) selection–factor
edges: one `solve_mini` pass is O(|C| + |F| + E), `optimize_mini` runs it
once per candidate for O(N·E) = O(N² log log N), and the final ordering is
O(|C|²).  Comfortably polynomial; the full N=1..1000 verification runs in
under a minute of pure Python.

### Why the upper-half answer can be trusted

Three provable facts pin the upper half down.  (1) A number above N/2
has no multiples in the game, so it can never be taken as tax: it is a
pure prize, either claimed by the player or inherited by the taxman at
the end.  (2) Claiming a prize requires paying tax, and every divisor
of a prize lies at or below N/2, so each claim spends one single-use
lower-half coupon - distinct prizes need distinct coupons.  Sets of
prizes that can be assigned distinct coupons form a transversal
matroid, where greedy-by-value is provably optimal: **no legal game's
upper half can outscore the greedy coupon-assignable prize set.**
(3) Coupons must also survive until spent (a claim sweeps every
remaining divisor of the claimed number), which is the peeling check.

Why does greedy-largest-first survive here when it fails for the game
at large?  What makes Taxman hard is the one-for-two trade: skipping a
larger number so that two smaller ones, summing to more, become
claimable.  Above N/2 that trade cannot exist: every claim spends
exactly one coupon, so skipping a prize frees exactly one coupon,
which funds at most one other prize - a one-for-one market, where you
always keep the bigger prize.  At the matching level this is provable
(if one prize individually blocks each of two others, Hall's condition
forces all three onto the same coupon, so the two block each other
too - the matroid exchange property in plain clothes).  The trades
that require backtracking live below N/2, where a number is dual-use -
prize or coupon - and skipping one decision cascades value through
chains of reassignments.  That is the promotions/bin-packing step, and
it is why solving the one-for-one part exactly in O(n^2) collapses the
search to the ~13% of selections that are contested.

Measured against that proven matroid ceiling, the playable optimum
sits a whisker below: the pure-matching bound overshoots in 911/999
games, but only by ~101 points on average (257 points at N=1000, or
0.09% of the upper sum).  N=39 is the minimal specimen: matching
allows {22, ...} worth 486, but no claiming order delivers it, and the
true optimum takes 21 for 485 - exactly this project's set.  The
ordering constraint sits outside the one-for-one argument above, so
greedy is not formally immune there - but brute force shows unique
optimal upper sets through n=54, and no optimal game through n=1000
ever skips a greedily-feasible prize above N/2.

### What is independently certified, and what is not

There is a circularity risk in "the known optima agree with this
algorithm 1000/1000": optimal.json was produced by the frame/mini-game
solver, and this project's algorithm is built on the same structural
theory.  If the shared decomposition assumption were wrong, both could
miss the same better solutions.  `independent.py` therefore certifies
what it can using only arguments with no shared assumptions:

* **Replay** - every recorded solution is a legal game with a matching
  score, so every entry of optimal.json is a sound lower bound.
* **Brute force** - an exact solver over raw pot states (no frames, no
  matchings, no maximal factors) reproduces every optimal score for
  **n = 1..54** and shows this project's upper-half set is the
  **unique** optimal upper set in all 54 games.  The wall is n=55
  (memoized state space, ~150s/game in pure Python).
* **Matching bound** - the maximum-weight-matching upper bound,
  recomputed here from scratch, equals the recorded score in 27 games;
  all of them lie below the brute-force frontier, so it adds nothing
  above n=54 (its tightness dies out as N grows).
* **Certificate chain** - opt(n) <= n + opt(n-1) is proved with no
  structural theory; a recorded solution achieving n + score(n-1) is
  certified given game n-1.  332 games satisfy the identity, but they
  are scattered in runs of at most 4, and none sits directly on the
  frontier, so the chain currently certifies nothing further.

Bottom line: games 1..54 are certified unconditionally, including the
uniqueness of the upper-half set; for 55..1000, optimal.json is the
best known result and the 1000/1000 agreement between it and this
algorithm is strong consistency between two implementations of one
theory - not an independent proof of either.  The matroid ceiling and
the 0.09% sliver are pure mathematics and stand regardless.  The one
unproven step remains: that a full-game optimum never sacrifices upper
value for lower gain; every prize outweighs any single coupon
(prize > N/2 >= coupon), and no counterexample exists anywhere in the
certified range.

## How much of the game is in the lower half?

Since the upper half is solvable in polynomial time, the hard part of
Taxman is choosing the selections at or below N/2.  From the known
optimal solutions (`python3 halves.py`):

| N range | lower-half selections | share of moves | share of points | max share |
|---|---|---|---|---|
| 2-100 | 1.9 | 8.6% | 4.28% | 8.11% |
| 101-300 | 10.0 | 11.6% | 6.68% | 8.90% |
| 301-600 | 24.8 | 12.7% | 7.35% | 8.08% |
| 601-1000 | 45.2 | 13.0% | 7.53% | 7.94% |

At N=1000, 58 of the 435 optimal selections are below N/2, worth 7.66%
of the score.  The share never exceeds 8.90% (N=114), so playing the
upper half perfectly already guarantees more than 91% of the optimal
score — the entire computationally hard part of the game is fighting
over the last ~8%.

## Approximating the full game in O(n²)

`approx.py` extends the upper-half machinery to the whole game.  It is
built on a characterization of playability that generalizes the
ordering argument above:

> A set of selections can all be played, in some order, **iff** there is
> a matching assigning each selection a distinct divisor still in the
> game, such that the precedence relation *"a before b whenever a's
> assigned divisor divides b, or a divides b"* is acyclic.  Any
> topological order of the precedence is a legal game.

The **greedy** strategy is then the full-game generalization of "take
the highest prime": consider n, n-1, ..., 2 and accept each number if
an augmenting path can add it to the matching (if no augmenting path
exists, no matching covers it — a permanent, well-founded rejection)
and a precedence-respecting assignment can be found.  This is O(n)
augmenting searches of O(n log n) each, plus one acyclicity check per
acceptance — about O(n² log n) worst case, a few hundred milliseconds
at n=1000 in pure Python.

Results for N=1..1000 against the known optima, alongside the pure
band-by-band cascade of upper-half mini games (**cascade**) and two
heuristics from [Robert Moniot's strategy comparison]
(https://www.dsm.fordham.edu/~moniot/taxman-strategies-comparison.html)
reimplemented here (**onetax**, **maxturn** — our implementations match
his published N≤128 table in 128/128 games for MaxTurn and 127/128 for
OneTax, where at N=128 ours scores 5289 vs his 5193):

| strategy | mean % of optimal | worst game | exactly optimal |
|---|---|---|---|
| greedy (this project) | 98.97% | 97.81% | 89/999 |
| onetax (Moniot) | 99.10% | 96.00% | 42/999 |
| cascade (this project) | 93.02% | 91.10% | 19/999 |
| maxturn (Carmony & Holliday) | 90.55% | 86.12% | 14/999 |

Selected games:

| n | optimal | greedy | onetax | cascade | maxturn |
|---|---|---|---|---|---|
| 21 | 144 | **144** | 144 | 135 | 135 |
| 100 | 3164 | **3161** | 3148 | 2976 | 2904 |
| 128 | 5301 | **5289** | 5289 | 4945 | 4816 |
| 500 | 78934 | 77631 | **78284** | 72849 | 71100 |
| 1000 | 315426 | 311260 | **312350** | 291258 | 286608 |

So an O(n²)-class algorithm gets within about 1% of optimal: greedy has
the best worst case (never below 97.8%) and finds the most exact optima
(89, including every game up to N=52); OneTax has a slightly better
mean for large N.  The cascade line quantifies what the verified
upper-half theory achieves entirely on its own, with no promotions —
about 93%, matching the lower-half share analysis above.

## Does OneTax get the upper half right?

No (`python3 upper_fidelity.py`).  Over N=2..1000 OneTax selects the
wrong set of numbers above N/2 in **83.7%** of games, and in none of
those does it still score optimal.  Its upper-half errors are always
omissions, never wrong picks: OneTax only selects a number once it is
down to a single divisor, and composite upper numbers whose divisor
counts never drain to one (e.g. 488, 506, 513, 522... for N around
970) simply never get picked.  The errors are self-compensating,
though: summed over all games OneTax gives up 1,143,256 points in the
upper half but wins 158,870 points *more* than optimal in the lower
half, because every skipped upper number leaves its would-be tax in
play to be farmed.

`one_tax_forced_upper` in `approx.py` tests the obvious repair: run
OneTax, but only allow a pick if the remaining optimal upper selections
are still solvable afterwards (a solve_mini feasibility check), and
when OneTax stalls, play the cheapest still-solvable upper selection.
Two designs matter here:

* Pinning each upper selection to a reserved factor (the maximal-factor
  matching) fails badly — 94.9% of optimal — because optimal play needs
  the upper half's tax demands to stay *flexible*.
* The dynamic-feasibility version wins: **99.264%** of all optimal
  points vs OneTax's 99.065% (better in 537 games, equal in 102, worse
  in 360).  For small games (N ≤ 300) plain OneTax still edges it out,
  and no strategy dominates game-by-game: forcing the upper half
  shifts tax pressure into the lower half, which sometimes costs more
  than the forced upper numbers are worth.

The count of upper numbers OneTax misses predicts the winner sharply
(`python3 portfolio.py`): when it misses 0-2, forcing nets negative;
from 3 up, forcing is nearly a free win.  That tension is resolved by
the fork oracle below.

## The fork oracle: pricing picks, then comparing continuations

`one_tax_oracle` in `approx.py` merges the two approaches.  The upper
machinery acts as an economist rather than a dictator: it tracks the
achievable set of upper selections (initially the provably optimal
upper half), any pick that keeps the set solvable is free, and a pick
that breaks it is charged the drop in achievable upper value
(re-derived by optimize_mini) - allowed only if the pick is worth more
than the loss, with the protected set shrinking accordingly.

Per-pick pricing alone turns out to converge to the hard veto: a
single pick is always worth less than the upper number it destroys,
but OneTax's advantage comes from *bundles* of unconstrained picks -
the same locality lesson as the 2-tax result, from the other side.  So
the oracle compares continuations instead of picks: wherever the
priced spine plays a different move than plain OneTax would, it
snapshots the position, and afterwards plays a plain OneTax tail out
of every snapshot.  The answer is the best of the spine and all tails,
which by construction is at least as good as both plain OneTax and the
forced-upper hybrid on **every single game** (asserted at runtime).

| strategy | share of all optimal points, N=1..1000 | exact optima |
|---|---|---|
| **fork oracle (this project)** | **99.414%** | 56/999 |
| forced-upper OneTax (this project) | 99.264% | 50/999 |
| onetax (Moniot) | 99.065% | 42/999 |
| greedy (this project) | 98.601% | 89/999 |
| cascade (this project) | 92.520% | 19/999 |
| maxturn (Carmony & Holliday) | 90.294% | 14/999 |

The oracle alone beats the old max(onetax, hybrid) portfolio (99.312%)
and makes both parents obsolete: neither is the sole best strategy in
a single game.  The four-way portfolio with greedy - which still wins
192 games outright, mostly via its 89 exact optima - reaches
**99.427%**, leaving about 0.57% of optimal on the table.

## Why a 2-tax rule cannot be bolted onto OneTax

The tax census above suggested an attractive idea: optimal games make
2-tax moves in 946 of 999 games, carrying more net value (875K points)
than OneTax's entire gap to optimal, so teach OneTax to pay two taxes.
It does not work, for reasons worth recording (`one_tax(two_tax=True)`
keeps the provably-safe version of the rule; it never fires).

1. **There is no endgame slack.**  A "stranded harvest" - pick a number
   whose two remaining divisors are dead, since the taxman was getting
   all three anyway - can never trigger: in all 999 games OneTax
   terminates with every pot number at zero divisors.  OneTax pays
   exactly one divisor per pick, so nothing is ever wasted; the pot is
   always drained bone-dry.

2. **Losses are sniping, not sticking.**  Tracing the upper numbers
   OneTax misses (e.g. 494 at N=970): the number drops to its last
   divisor (38), and next turn OneTax hands that divisor to a larger
   claimant (646 = 17·38).  The number dies at count 0, not count 2.

3. **A local 2-tax rescue is either unnecessary or unaffordable.**
   Consider x at two divisors whose rivals are the current one-tax
   claimants c1 and c2.  If the rivals are smaller than x, x needs no
   rescue: when one divisor is consumed, x reaches one divisor and
   outranks them.  If the rivals are larger, the rescue price exceeds
   2x and can never be worth x.  Forcing the exchange whenever
   x > c1 + c2 loses 1.75M points over N=1..1000 (it fires exactly in
   the cases where x would have won anyway, paying double).

So the 875K points that optimal games route through 2-tax moves are
**coordination gains**: they are profitable only because optimal
simultaneously reassigns which taxes the surrounding numbers pay.  Any
strategy that wants them must reason about the assignment of divisors
to selections - the matching machinery - not about individual picks.

## Performance

Per-game wall time in pure Python (best of 3, `python3 bench.py`), with
the empirical scaling exponent k in time ~ n^k:

| component | n=125 | n=250 | n=500 | n=1000 | ~n^k |
|---|---|---|---|---|---|
| onetax | 0.2 ms | 0.6 ms | 1.9 ms | 6.1 ms | 1.6 |
| maxturn | 0.3 ms | 1.2 ms | 4.4 ms | 19 ms | 1.9 |
| upper half (`solve_upper_half`) | 2.7 ms | 10 ms | 38 ms | 168 ms | 2.0 |
| cascade | 3.8 ms | 16 ms | 66 ms | 310 ms | 2.1 |
| hybrid (forced upper) | 5.8 ms | 24 ms | 91 ms | 430 ms | 2.1 |
| greedy | 3.8 ms | 62 ms | 377 ms | 1.2 s | 2.7 |
| oracle (fork) | 6.1 ms | 62 ms | 215 ms | ~2 s | ~3 |

OneTax is indeed nearly free.  Determining the optimal >N/2 moves is a
clean O(n²): optimize_mini runs one O(n) feasibility check per
candidate.  The oracle is the expensive one - profiling shows
essentially all of its time inside `price()`: every candidate pick
triggers a solve_mini feasibility check, and every break triggers an
optimize_mini re-derivation (tens of thousands of solve_mini calls per
game).  Pricing now bails out as soon as the accumulated loss reaches
the pick's value, which saves ~1.4x; the remaining cost is dominated
by the members that are *kept* during re-derivation, so the next real
speedup would be an incremental feasibility structure rather than
rebuilding the mini graph per check.

## Running it

No dependencies beyond Python 3.8+ (pytest for the test suite).

```
python3 verify.py                  # check the upper-half theory, N=1..1000
python3 verify.py --max-n 200 -v   # smaller range, per-game detail
python3 halves.py                  # lower-half share of the optimal score
python3 approx.py                  # full-game strategy comparison (~9 min)
python3 upper_fidelity.py          # OneTax upper-half errors + forced-upper hybrid (~7 min)
python3 -m pytest test_taxman_mini.py
```

## Files

| file | contents |
|---|---|
| `taxman_mini.py` | the core algorithm: `maximal_factors`, `solve_mini`, `optimize_mini`, `order_for_real_game`, `solve_upper_half` |
| `verify.py` | checks the upper-half theory against `optimal.json` for N=1..1000 |
| `halves.py` | how much of the optimal score comes from selections ≤ N/2 |
| `approx.py` | full-game approximation strategies and the Moniot comparison |
| `upper_fidelity.py` | OneTax's upper-half fidelity and the forced-upper hybrid |
| `independent.py` | theory-free certification of optimal.json (brute force, matching bound, certificate chain) |
| `bound.py` | the Franklín-Moniot upper bound (max-weight matching over maximal-factor edges) |
| `bench.py` | per-strategy timing, scaling exponents, and profiling |
| `moniot_table.json` | Robert Moniot's published N≤128 results (for validation) |
| `approx_results.json` | per-game scores of every strategy for N=1..1000 |
| `test_taxman_mini.py` | unit tests anchored to the wiki's examples |
