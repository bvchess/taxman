# The solvers

Two Rust programs find optimal Taxman games and prove them optimal.  They share no code, so each checks the other.
This page maps them and their documents.

* **The frame solver**, [frame/](frame/README.md) (the crate `taxman`), finds the games.  It searches the promotions
  of a board's frames on many CPUs, in runs that resume after an interruption.  Each game it proves is optimal within
  the *frame model*, which is exact given one open statement about the game, called U.
* **The prover**, [noframe/](noframe/README.md) (the crate `taxman-prover`), proves games optimal on the whole board,
  with no frame model, from the theorems of [docs/THEORY.md](../docs/THEORY.md) alone.  Where it has proven a game,
  it has proven U there too.

[results/README.md](../results/README.md#what-supports-the-scores) says how far the games of each program reach.
Cargo commands run in `solver/frame` or `solver/noframe`; every other command in the docs runs from the top of the
repository.

## The documents

Each document is of one kind: a *guide* to a task, a *reference* to look things up in, an *explanation* of how
something works and why, the *record* of what was tried, or a *time-bound* page about the state of the work.

| document | kind | what it holds |
|---|---|---|
| [frame/README.md](frame/README.md) | guide and reference | the frame solver: what it computes, a first run, its commands, its output and why its answers can be trusted |
| [docs/GLOSSARY.md](../docs/GLOSSARY.md) | reference | the project's words, each with the one name the docs use |
| [results/README.md](../results/README.md) | reference | the committed results files, how each was made, and what supports their scores |
| [noframe/README.md](noframe/README.md) | explanation and reference | the prover: its proofs and checks of the frame solver's results on the whole board, and its checks of the conjectures |
| [docs/DESIGN.md](docs/DESIGN.md) | explanation | how the frame solver works and why: its goals, the frame model, the architecture, the search, the tactics and their grades, the engine, the pipeline, the trust model, the tests and performance |
| [docs/THEORY.md](../docs/THEORY.md) | explanation | the theorems both programs rest on: maximal factors, playable sets, the bounds and the frame model's exactness |
| [docs/RUNBOOK.md](docs/RUNBOOK.md) | guide | running the frame solver, and the prover's proofs, on a cloud machine of many CPUs |
| [docs/STATUS.md](docs/STATUS.md) | time-bound | what is in flight, paused, decided and waiting on a decision; read it first |
| [docs/AGENDA.md](docs/AGENDA.md) | time-bound | the research agenda: what limits the end of the chain, the avenues ranked, the conjectures, and what the cloud runs should measure |
| [docs/IDEAS.md](docs/IDEAS.md) | time-bound | ideas borrowed from other fields (SAT, MaxSAT, MIP, #SAT, ...), ranked, with pilot measurements, the profile of the hardest games and the study of nogoods at many threads |
| [notebook/README.md](notebook/README.md) | record | the lab notebook: the register of everything tried, kept or not (search it before proposing an experiment), and the index of rounds; each entry is a folder beside it |
| [the cost curve](notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md) | record | how the cost of a game grew with n past the end of the chain, with projections for a larger machine |
| [docs/WRITING.md](../docs/WRITING.md) | guide | how the docs are written |
| the [wiki](https://github.com/bvchess/taxman/wiki) | explanation | the Java solver's algorithm and definitions |

## The tools

[tools/](tools) holds the scripts around the programs:

* `quick_check.py`, the check that every change passes (formatting, lints and the tests, within time limits);
* `check_results.py` and `check_bounds.py`, the Python verifiers of the results files and of the bounds store, which
  share no code with the Rust programs;
* `pgo.sh`, a profile-guided build of the frame solver;
* `cloud/`, the scripts of the RUNBOOK, and `certify/`, the work list of the certificates of U
  ([its README](tools/certify/README.md)).
