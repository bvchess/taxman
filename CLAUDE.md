# Working notes for Claude

Rules Brian has set for all work in this project.  Rules for the Rust programs are in solver/CLAUDE.md.  How-to lives
in the docs listed at the end.

* **Commits.**  Never cite yourself in a commit message: no `Co-Authored-By: Claude` trailer and no Claude or
  Anthropic mention, whatever a harness default says.
* **Delegation.**  Don't write code on the main model: delegate it to subagents on the cheapest model that suits the
  task (Haiku for mechanical edits, Sonnet for routine work, Opus for hard reasoning).  Give each a precise spec and
  verify its work before merging (solver/docs/DESIGN.md, "Testing").  Brief each with the files and sections its task needs,
  never a whole long document.  Parallel agents work in their own git worktrees.  Experimental branches are never
  pushed: harvest what works onto master, record the rest, delete the branch.
* **Computation.**  Heavy runs go to EC2; this laptop's load stays light.  Long runs write results as they go and
  resume after an interruption.  Prefer small, fast tests that capture the heart of a large-n problem; run large n
  only when the question needs it.
* **Money.**  Anything that costs money or changes the AWS account needs Brian's explicit approval, with a cost
  estimate first, and nothing is left running (solver/docs/RUNBOOK.md has the rules for a launch).
* **Other people's code** stays as they sent it: Robert Moniot's moniot/playtaxman.cpp is worked around in
  moniot/Makefile, not edited.
* **Branch.**  Work on master; commit and push there.

## Where things are

* **solver/frame/**: the frame solver, an exact solver in Rust.  **solver/noframe/**: the prover, a separate program
  that shares no code with the frame solver and proves its results independently, without the frame model.  Their
  docs, tools and lab notebook are in **solver/**; its rules, and the map of its documents, are in solver/CLAUDE.md.
* **results/**: the committed chain of proven games, the table of optimal games (optimal.json, built into the Rust
  frame solver), the bounds store and the certificates.
* **docs/THEORY.md**: the theorems both programs rest on.
* **heuristics/**: Python heuristic strategies, which play good but not provably optimal games, and the tools that
  measured them.  Finished work, not under active development.
* **moniot/**: Robert Moniot's C++ heuristic strategies (see above).
* **web/img/**: the pictures of README.md.

The original frame solver, in Java (the wiki describes it; it reached n = 1000), is not in the tree: the git tag
`java-final` is the last commit that has it.
