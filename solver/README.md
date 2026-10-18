# The solvers

Two Rust programs find and prove optimal taxman games.  They share no code, so each is a check on the other.

* **The frame solver**, [frame](frame/README.md) (crate `taxman`), searches the promotions of the frames of a board
  (the frame model) on many CPUs, with runs that resume.  It has solved every board to n = 3440 in one chain from
  n = 1 ([results/strict.jsonl](../results/strict.jsonl)).  Each game is proven optimal within the frame model, which is
  exact given one statement about the game, U.
* **The prover**, [noframe](noframe/README.md) (crate `taxman-prover`), proves the same games on the whole board,
  with no frame model, from the theorems of [docs/THEORY.md](../docs/THEORY.md) alone.  It has proven every game to
  2175, and with it U to 2175.

Around them: [docs](docs) (the current state of the work in STATUS.md, the design, the guide to running on EC2, the
research agenda), [notebook](notebook/README.md) (the lab notebook: one folder for each experiment, with its data) and
[tools](tools) (the referees of the results files, the cloud scripts, the quick check).  Cargo commands run in
`solver/frame` or `solver/noframe`; every other command in the docs runs from the top of the repository.
