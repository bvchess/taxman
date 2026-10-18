# Working notes for Claude: the Rust programs

Rules Brian has set for the frame solver and the prover, on top of the project's (../CLAUDE.md).  How-to lives in the
docs listed at the end.  Paths here are from the top of the repository; Cargo commands run in `solver/frame` or
`solver/noframe`.

* **Read solver/docs/STATUS.md first.**  It is the current state of the work; brief subagents with it and the notebook
  entries their task needs.
* **Nothing unproven.**  Every rule the frame solver relies on is proven (docs/THEORY.md) and tested.
  Committed results are evidence, and runs never overwrite them.
* **Record every experiment.**  Search the register at the top of solver/notebook/README.md before proposing one, and
  record each when it ends, failures included, as the notebook's front page says (one folder an entry in
  solver/notebook/, its README.md the entry, with its rows in the register and the index).

* **Writing.**  Docs follow docs/WRITING.md.
* **Keep the quick check quick.**  Every change passes `solver/tools/quick_check.py`, which fails on a slow test or a
  slow suite; how to keep a test inside its limits is in solver/docs/DESIGN.md, section 9.

## Where things are

* **solver/frame/README.md**: the guide, glossary and reading order.
* **solver/docs/DESIGN.md**: the design, and how changes are tested.
* **solver/docs/STATUS.md**: the current state: what is in flight, paused, decided, and waiting on a decision.
* **solver/notebook/README.md**: the lab notebook's register and index, with its rules; its entries are the folders
  beside it.
* **solver/docs/AGENDA.md**: the research avenues.
* **solver/docs/RUNBOOK.md**: the guide to running on EC2.
* **results/**: the committed chain of proven games.
* **solver/noframe/**: the prover (solver/noframe/README.md).
