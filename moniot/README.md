# playtaxman — a collection of heuristic strategies

`playtaxman.cpp` is a standalone C++ program by **Robert K. Moniot** that
plays Taxman using any of seven heuristic strategies.  It was provided by
its author for inclusion here and is redistributed under the MIT license
(see the SPDX header in the source).

Moniot is a co-author of the Franklín–Moniot upper bound that this project
measures against ([The difficulty of beating the
Taxman](https://arxiv.org/abs/2211.00461), *Discrete Applied Mathematics*
339, 166–171, 2023), implemented here in
[`heuristics/evaluation/bound.py`](../heuristics/evaluation/bound.py).  His
published comparison of these strategies is at
<https://www.dsm.fordham.edu/~moniot/taxman-strategies-comparison.html>;
its n ≤ 128 table is committed as
[`heuristics/results/moniot_table.json`](../heuristics/results/moniot_table.json).

The source header also cites L. A. Carmony and R. L. Holliday, "An Example
from Artificial Intelligence for CS1," *SIGCSE Bulletin* 25(1) (1993); R.
Moniot, "The Taxman Game," *Math Horizons* 14 (Feb. 2007); and J. Trono,
"Taxman Revisited," *SIGCSE Bulletin* 26(4) (Dec. 1994).

## Build

```
make
```

Requires a C++17 compiler.  There is no dependency beyond POSIX
`getopt_long`.

## Usage

```
./playtaxman [options]
```

The board size is read from stdin — a single `N`, a range `minN-maxN`, a
list `N1,N2,...`, or a combination.

| option | effect |
|---|---|
| `-h`, `--heuristic=NAME` | strategy to play (default `OneTax`) |
| `-p`, `--print=FORMAT` | `Human` (default), `CSV`, `Math`, `JSON` |
| `-s`, `--score` | print the score |
| `-m`, `--moves` | print the move sequence |
| `-f`, `--fraction` | print the fraction of the pot won |
| `-d`, `--describe` | describe the selected heuristic and exit |
| `-?`, `--help` | usage page |

`Human` format prompts for input; the others do not, so they can be piped
or redirected.  With no output option, score and moves are printed.

```
$ echo 21 | ./playtaxman -h OneTax -s -m -p Human
$ echo 1-1000 | ./playtaxman -h MaxTurn+ -s -p CSV > maxturn_plus.csv
```

## The strategies

Run `./playtaxman -d -h NAME` for the author's description of each.
Scores below were measured over n = 2..1000 against
[`optimal.json`](../src/main/resources/optimal.json); "point share" is the
strategy's total as a fraction of the optimal total, and "exact" counts
games where it found an optimal score.

| strategy | point share | avg % of optimal | worst game | exact |
|---|---|---|---|---|
| OneTax (default) | 98.983% | 98.917% | 96.00% | 42/999 |
| GreedyOneTax | 98.921% | 98.849% | 95.99% | 42/999 |
| PureOneTax | 97.450% | 97.222% | 80.95% | 25/999 |
| MaxTurn+ | 94.685% | 95.069% | 91.89% | 22/999 |
| MaxTurn | 90.294% | 90.554% | 86.12% | 14/999 |
| OddsEvens | 83.426% | 83.999% | 76.54% | 13/999 |
| MaxPick | 61.952% | 62.030% | 40.00% | 3/999 |

All seven reproduce the n ≤ 128 published table exactly on the three
columns it shares (`OneTax`, `MaxTurn+`, `MaxTurn`).

## Relationship to `heuristics/`

[`heuristics/`](../heuristics) contains independent Python
reimplementations of two of these strategies, used as comparison baselines
for that project's own strategies.  Verified over n = 1..1000 against this
program:

* `strategies/maxturn.py` matches `MaxTurn` on all 1000 games.
* `strategies/onetax.py` with `refined=False` matches `PureOneTax` on all
  1000 games.
* `strategies/onetax.py` with `refined=True` **diverges** from `OneTax`,
  first at n = 128, on 313 of 1000 games.

The divergence is a difference in the refinement rule, and the two
sources disagree about which rule is intended: the prose description in
`playtaxman.cpp` (lines 53–56) requires the rescued multiple to have no
other active multiple of its own, which `strategies/onetax.py` implements
and `takeOneTax` does not.  `takeOneTax` also takes the smallest
qualifying multiple where the Python takes the largest.
