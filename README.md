# Taxman
A program to find optimal solutions to the taxman game along with some heuristic
approaches to more quickly find approximate solutions.

![game 10 move 3](web/img/10.3.png)

**The game.**  The pot holds the numbers 1 to n; each turn the player takes a number that still has a proper divisor
in the pot and scores it, and the taxman takes every divisor of it still in the pot.  The game ends when no number in
the pot has a divisor left in it, and an optimal game scores the most (n = 4: take 3, then 4, for 7).

See the [project wiki](https://github.com/bvchess/taxman/wiki) for more information about the game and the
algorithm implemented by this program. Heuristic approaches are [here](heuristics).

Robert Moniot's C++ program implementing seven heuristic strategies is included in
[moniot](moniot), redistributed with his permission.

## The frame solver, in Rust

The frame solver, a Rust reimplementation of the exact algorithm, built to use many CPUs and for long resumable runs,
has solved every board up to n = 3440 in one contiguous chain from n = 1, each game proven optimal within its frame
model with nothing assumed (the Java program, which got to 1000, is no longer in the tree: the git tag `java-final` is
the last commit that has it; the table of optimal games, [results/optimal.json](results/optimal.json), has the frame
solver's games for 1001 to 1300, and its games to 3440 are in [results/strict.jsonl](results/strict.jsonl)).  Every
game up to 2175 is also proven optimal on the whole board by the prover in [solver/noframe](solver/noframe/README.md),
which uses no frame model (Theorem 10 of [docs/THEORY.md](docs/THEORY.md)).  The games 2176 to 3440 are proven within
the frame model, which is exact given one statement about the game, U, which holds to 2175 (the prover) and is not yet
certified past it
([docs/THEORY.md](docs/THEORY.md#the-frame-model-what-is-proven-and-what-remains)).
It is a Cargo crate in [solver/frame](solver/frame) with its own build, run and test instructions: start with
[solver/frame/README.md](solver/frame/README.md) (`cd solver/frame && cargo build --release`), and see
[solver/docs/RUNBOOK.md](solver/docs/RUNBOOK.md) for running it on a large cloud machine.

## Where things are

* [solver](solver/README.md): the frame solver (`solver/frame`) and the prover (`solver/noframe`), with their docs
  (the current state of the work, the design, the guide to the cloud), their tools, and the lab notebook of the
  experiments that made them (`solver/notebook`).
* [results](results/README.md): the committed chain of proven games, `strict.jsonl`; the table of optimal games,
  `optimal.json`; the bounds on the boards past the chain; and the certificates.
* [docs](docs): `THEORY.md`, the theorems both programs rest on.
* [heuristics](heuristics): the Python heuristic strategies and the tools that measured them; [moniot](moniot): the C++
  ones; [web/img](web/img): the pictures of this page.

## References and Links
- The On-Line Encyclopedia of Integer Sequences documents the Taxman Sequence (the optimal player score)
as [A019312](https://oeis.org/A019312).
- Interactive online implementations of the taxman game are <http://davidbau.com/archives/2008/12/07/taxman_game.html> and
<https://www.cryptool.org/en/cto/taxman>.

## Acknowledgements
Thanks to EJ Technologies for the use of their excellent
[Java profiler](https://www.ej-technologies.com/products/jprofiler/overview.html).
