# Working notes for Claude: the Rust programs

Rules Brian has set for the frame solver and the prover, on top of the project's (../CLAUDE.md).  How-to lives in the
docs listed at the end.

* **Read docs/STATUS.md first.**  It is the current state of the work; brief subagents with it and the notebook
  entries their task needs.
* **Nothing unproven.**  Every rule the frame solver relies on is proven (heuristics/THEORY.md) and tested.
  Committed results are evidence, and runs never overwrite them.
* **Record every experiment.**  Search the register at the top of docs/EXPERIMENTS.md before proposing one, and
  record each when it ends, failures included, as the notebook's front page says (one file an entry in
  docs/experiments/, with its rows in the register and the index).

## Where things are

* **README.md**: the guide, glossary and reading order.
* **DESIGN.md**: the design, and how changes are tested.
* **docs/STATUS.md**: the current state: what is in flight, paused, decided, and waiting on a decision.
* **docs/EXPERIMENTS.md**: the lab notebook's register and index, with its rules; its entries are in
  docs/experiments/.
* **docs/AGENDA.md**: the research avenues.
* **tools/cloud/RUNBOOK.md**: the guide to running on EC2.
* **results/**: the committed chain of proven games.
* **prover/**: the prover (prover/README.md).
