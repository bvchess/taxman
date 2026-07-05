# Taxman Mini: the upper half of a Taxman game in polynomial time

This is a small, clean implementation built to test a theory about the
[Taxman game](https://github.com/bvchess/taxman/wiki/The-Game):

> There is a polynomial-time algorithm for obtaining all of the numbers
> greater than N/2 in the optimal answer, along with the optimal sequence
> for these numbers.

## Result

**The theory holds for every game from N=1 to N=1000.**

Checked against the known optimal solutions in
[`optimal.json`](../src/main/resources/optimal.json):

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
  in 360), making it the strongest O(n²)-class strategy measured here.
  For small games (N ≤ 300) plain OneTax still edges it out, and no
  strategy dominates game-by-game: forcing the upper half shifts tax
  pressure into the lower half, which sometimes costs more than the
  forced upper numbers are worth.

| strategy | share of all optimal points, N=1..1000 |
|---|---|
| forced-upper OneTax (this project) | **99.264%** |
| onetax (Moniot) | 99.065% |
| greedy (this project) | 98.601% |
| cascade (this project) | 92.520% |
| maxturn (Carmony & Holliday) | 90.294% |

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
| `moniot_table.json` | Robert Moniot's published N≤128 results (for validation) |
| `approx_results.json` | per-game scores of every strategy for N=1..1000 |
| `test_taxman_mini.py` | unit tests anchored to the wiki's examples |
