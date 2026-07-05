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

## Running it

No dependencies beyond Python 3.8+ (pytest for the test suite).

```
python3 verify.py                  # check N=1..1000 against optimal.json
python3 verify.py --max-n 200 -v   # smaller range, per-game detail
python3 -m pytest test_taxman_mini.py
```

## Files

| file | contents |
|---|---|
| `taxman_mini.py` | the algorithm: `maximal_factors`, `solve_mini`, `optimize_mini`, `order_for_real_game`, `solve_upper_half` |
| `verify.py` | checks the theory against `optimal.json` for N=1..1000 |
| `test_taxman_mini.py` | unit tests anchored to the wiki's examples |
