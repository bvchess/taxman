# The solvers

Two Rust programs find and prove optimal taxman games.  They share no code, so each checks the other.

* **The frame solver**, [frame](frame/README.md) (crate `taxman`), searches the promotions of a board's frames on many
  CPUs, in runs that resume.  It has solved every board to n = 3440 in one chain
  ([results/strict.jsonl](../results/strict.jsonl)); each game is proven optimal within the frame model, which is exact
  given one statement about the game, U.
* **The prover**, [noframe](noframe/README.md) (crate `taxman-prover`), proves games on the whole board with no frame
  model, from the theorems of [docs/THEORY.md](../docs/THEORY.md) alone.  It has proven every game to 2175, and U with
  them.

Around them: [docs](docs) (the current state of the work, the design, the guide to EC2, the research agenda), the lab
notebook ([notebook](notebook/README.md)) and [tools](tools).  Cargo commands run in `solver/frame` or
`solver/noframe`; every other command in the docs runs from the top of the repository.
