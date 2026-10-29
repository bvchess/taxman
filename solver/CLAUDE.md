# Working notes: the Rust programs

The rules for the frame solver and the prover, on top of the project's ([../CLAUDE.md](../CLAUDE.md)).  How-to lives in
the docs listed at the end.  Paths here are from the top of the repository; Cargo commands run in `solver/frame`,
`solver/noframe` or `solver/gzip`.

* **Read [solver/docs/STATUS.md](docs/STATUS.md) first.**  It is the current state of the work; brief subagents with it
  and the notebook entries their task needs.
* **Nothing unproven.**  Every rule the frame solver relies on is proven ([docs/THEORY.md](../docs/THEORY.md)) and
  tested.  Committed results are evidence, and runs never overwrite them.
* **Record every experiment.**  Read the register's file of its subject (solver/notebook/register/) before proposing
  one, and record each when it ends, failures included, as the notebook's front page says (one folder an entry in
  solver/notebook/ or solver/notebook/search/, its README.md the entry, with its rows in the register).

* **Writing.**  Docs follow [docs/WRITING.md](../docs/WRITING.md).
* **Keep the quick check quick.**  Every change passes `solver/tools/quick_check.py`, which fails on a slow test or a
  slow suite; how to keep a test inside its limits is in [solver/docs/design/testing.md](docs/design/testing.md).

## Where things are

* **[solver/README.md](README.md)**: the map of the solvers and their documents.
* **[solver/frame/README.md](frame/README.md)**: the frame solver's guide: build, a first run, the commands.
* **[solver/docs/REFERENCE.md](docs/REFERENCE.md)**: the frame solver's reference: every flag and command, the output,
  the proofs.
* **[docs/GLOSSARY.md](../docs/GLOSSARY.md)**: every term, defined once.
* **[solver/docs/DESIGN.md](docs/DESIGN.md)**: the design, the index of solver/docs/design/, a file a part
  ([testing.md](docs/design/testing.md) says how changes are tested).
* **[solver/docs/STATUS.md](docs/STATUS.md)**: the current state: the priority, and the projects in flight.
* **[solver/notebook/README.md](notebook/README.md)**: the lab notebook's rules, and the map of its register (register/,
  a file a subject); its entries are the folders beside it and in search/.
* **[solver/docs/AGENDA.md](docs/AGENDA.md)**: the research avenues.
* **[solver/docs/RUNBOOK.md](docs/RUNBOOK.md)**: the guide to running on EC2, the index of solver/docs/runbook/, a file
  a job ([launch.md](docs/runbook/launch.md) has the rules for every launch).
* **results/**: the committed chain of proven games.
* **solver/noframe/**: the prover ([solver/noframe/README.md](noframe/README.md)).
