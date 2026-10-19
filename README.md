# Taxman

This project finds optimal games of Taxman, a number game, and proves that they are optimal.  It also has heuristic
strategies, which play good games quickly but without a proof.

![game 10 move 3](web/img/10.3.png)

## The game

Taxman is played on the numbers 1 to n, the *board*; the numbers still in play are the *pot*.  The player takes a
number that still has a proper divisor in the pot and scores it.  The taxman then takes every divisor of that number
still in the pot.  The game ends when no number left in the pot has a proper divisor in it, and the taxman takes what
remains.  An *optimal game* scores the most.  For game 4, the player takes 3 (the taxman takes 1), then 4 (the taxman
takes 2), and scores 7, the best possible.

The best scores for n = 1, 2, 3, ... are the Taxman sequence, [A019312](https://oeis.org/A019312) in the OEIS.  Finding
an optimal game is a search over an enormous tree, and the hard part is proving that no game scores more.  The [project
wiki](https://github.com/bvchess/taxman/wiki) explains the game and the algorithm, with a walkthrough of game 21.  The
[glossary](docs/GLOSSARY.md) defines the project's words.

## The results

The committed chain, `results/strict.jsonl`, holds an optimal game for every n from 1 to its end.  Each game is built
from the one before, and each is proven with nothing assumed beyond the frame model.  The first part of the chain is
also proven on the whole board by a second program that shares no code with the first.  The rest is proven within the
*frame model*, a decomposition of the game that is exact given one open statement about it, called U.
[results/README.md](results/README.md) gives how far the chain reaches, how far each kind of proof reaches, and what
supports every score.

The table of optimal games, [results/optimal.json](results/optimal.json), holds the Java solver's games 1 to 1000 and
the frame solver's 1001 to 1300.  The [results](results/README.md) directory also keeps the bounds on opt(n), the best
score, found for the games past the chain, and the certificates of U.

## The programs

* **The frame solver** ([solver/frame](solver/frame/README.md)), in Rust, finds the games.  It reimplements the
  algorithm of the original Java solver, which the wiki describes: it splits the board into independent groups of
  numbers, the *frames*, and searches for the best set of numbers to play beyond the ones the frames settle.  It runs
  on many CPUs, and its long runs resume after an interruption.
* **The prover** ([solver/noframe](solver/noframe/README.md)), also in Rust, proves games optimal on the whole board,
  without the frame model.  It shares no code with the frame solver, so each checks the other.
* **The heuristics** ([heuristics](heuristics/README.md)), in Python, play near-optimal games in polynomial time.
* **Robert Moniot's seven heuristic strategies** ([moniot](moniot)), in C++, redistributed with his permission.

The Java solver reached n = 1000; it is no longer in the tree, and the git tag `java-final` is the last commit that
has it.  [solver/README.md](solver/README.md) maps the Rust programs' documents, and [web/img](web/img) holds the
pictures of this page.

## Running it

The frame solver runs on Linux or macOS.  It needs Rust and a C linker, and nothing to download.  From the top of the
repository:

```
cd solver/frame && cargo build --release && cd ../..
solver/frame/target/release/taxman solve 1-22
solver/frame/target/release/taxman verify-results results/strict.jsonl
```

The first command builds the solver, the second solves the games 1 to 22, and the third checks every game of the
committed chain.  [The frame solver's README](solver/frame/README.md) walks through this first run and documents every
command.

## Where to read on

* To use the frame solver: [its README](solver/frame/README.md), and the [RUNBOOK](solver/docs/RUNBOOK.md) for a large
  cloud machine.
* To check the results: [results/README.md](results/README.md).
* For the mathematics: [docs/THEORY.md](docs/THEORY.md), the theorems both programs rest on, and [the prover's
  README](solver/noframe/README.md).
* For how the frame solver works and why: [DESIGN.md](solver/docs/DESIGN.md).
* For the state of the work and what was tried: [STATUS.md](solver/docs/STATUS.md) and the [lab
  notebook](solver/notebook/README.md).

## References and links

* The On-Line Encyclopedia of Integer Sequences has the Taxman sequence, the optimal score of each game, as
  [A019312](https://oeis.org/A019312).
* Franklín and Moniot, [The difficulty of beating the Taxman](https://arxiv.org/abs/2211.00461), *Discrete Applied
  Mathematics* 339 (2023), 166–171: the upper bound the project's checks use, and the NP-hardness of a variant of the
  game.
* Taxman can be played online at <http://davidbau.com/archives/2008/12/07/taxman_game.html> and
  <https://www.cryptool.org/en/cto/taxman>.

## Acknowledgements

Thanks to EJ Technologies for the use of their excellent [Java
profiler](https://www.ej-technologies.com/products/jprofiler/overview.html).
