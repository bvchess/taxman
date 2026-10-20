# The lab notebook

The record of what was tried on the frame solver, the prover and the theory under them, kept or not, with the numbers
that decided it.  Each experiment, or round of experiments, is an entry: a folder of this directory.  This page holds
the rules for filing an entry and for the register.  [DESIGN.md](../docs/DESIGN.md) says how the frame solver works, and
[STATUS.md](../docs/STATUS.md) gives the state of the work.

An entry is dated by month and names the commits that made each step, so `git show COMMIT` finds the code.  It is true
of the code at its date; where a later entry changed the picture it says so, and DESIGN.md is the authority on the code.
The [glossary](../../docs/GLOSSARY.md) defines the terms, and [DESIGN.md, section 10](../docs/DESIGN.md#10-performance)
the measuring tools: the ladder, the probe and `taxman bench`.

* [Register of everything tried](#register-of-everything-tried): its rules, and its files, one a subject, with a row per
  experiment, where its details are and what decided it.  Look here first.
* [index.md](index.md): the entries in the order they happened, with their commits.
* [terms-changed.md](terms-changed.md): the old words of the dated entries, the words that replaced them, and the
  programs removed since.

## How this notebook is kept

* **What to read.**  Before proposing an experiment, read the register's file of its subject, or search the register
  (`grep -ri WORD solver/notebook/register/`), and read only the entries a task needs.
* **One folder an entry.**  An experiment, or a round of them, is one folder of this directory with a README.md in it,
  the entry, named by its title's anchor (`the-frontier-chain-to-3440-2026-10/` for "The frontier chain to 3440
  (2026-10)").  The README.md opens with `# Title (YYYY-MM)` and the line `Part of the [lab notebook](../README.md).`
  Then it gives its question in a sentence and its answer in a sentence or two, then the method, the results with their
  numbers and the decision ([docs/WRITING.md, rule 7](../../docs/WRITING.md#7-the-notebook-is-the-record)), and its
  commits.  It is written when the experiment ends, failures and kills included.
* **Files.**  The folder holds the data and the scripts the entry made, and a section `## Files` at the end of the
  README.md says what each file is and the command that made it.  A tool that outlives its experiment goes to
  `solver/tools/` instead.
* **Entries are records.**  An entry is not rewritten later: a later finding goes in a new entry, or in a dated section
  at the end of the old one (`## Later (YYYY-MM): ...`), and the old text stays.  A link that moved is the one change.
* **The register and the index point to every entry.**  An entry adds its rows to the register's file of their subject
  (the date, the question, the answer with the number that decided it, and where the details are) and a row to the
  [index](index.md).  The docs test refuses an entry that neither links to.
* **Links.**  From another document, link `solver/notebook/NAME/README.md` by its relative path; between entries,
  `../NAME/README.md`; from an entry to this page, `../README.md#ANCHOR`.  The docs test refuses a link to an entry
  through this page (`README.md#an-entry`).

## Register of everything tried

One row per experiment or idea tried on the solver and on the theory under it, kept or not, so that nothing is tried
twice.  **Search it before proposing an experiment, and add a row when one finishes, failures included.**

A row gives the date, the question, the answer and where the details are.  The answer opens with the outcome, then gives
the finding in a sentence with the number that decided it:

* **kept**: in the solver or the prover, or the finding stands;
* **not kept**: tried and taken out, or never adopted because it did not pay or is false;
* **superseded**: kept for a while, then replaced by something better;
* **open**: proposed, and not tried or not decided;
* **measured**: a study whose result changed no code;
* **built**: in the code, and not yet run where it matters.

A few words may follow the outcome: "kept, the default", "not kept, as a switch".

The details are where the row points: an entry, [DESIGN.md](../docs/DESIGN.md), the [prover's
README](../noframe/README.md), the [results](../../results/README.md), or the module comment of a study's code.  Where
an experiment's code was discarded, as for the [pilots of ideas from other
fields](ideas-from-other-fields-the-pilots-2026-10/README.md), that page describes what was computed, on which games and
with which settings, well enough to repeat it.  The date is the earliest commit the entry names, or the date of the
entry's later section that the row links, or its month.  The numbers are those of the page the row points to; a game is
named by n, and times are search seconds.

The register is a file a subject, in [register/](register/).  Read the file of your subject whole, or search them all
with `grep -ri WORD solver/notebook/register/`:

* [node-bound.md](register/node-bound.md): the bound that the search computes at every node, the matching bound and the
  GF(2) bound, and their machinery: conflicts, refinement, forced members, cycle searches, levels, the cost of a node.
* [stronger-bounds.md](register/stronger-bounds.md): the bounds tried on top of it: the Lagrangian bounds, LP
  relaxations, cuts and covers, the fields study and the multi-field bound (A1), and the root gaps.
* [search.md](register/search.md): the rest of the search: branching, the incumbent, the start and the ceiling,
  decomposition, symmetry, sampling, standalone runs of hard games, and the diagnosis of the wall.
* [nogoods-and-conflicts.md](register/nogoods-and-conflicts.md): learning from conflicts: nogoods, their propagation,
  their tables, and carrying them from one game to the next.
* [engine-and-parallelism.md](register/engine-and-parallelism.md): the threads, the splits and the stealing, the
  checkpoints, the set widths, and the build.
* [tactics-and-shortcuts.md](register/tactics-and-shortcuts.md): the shortcuts that settle a game with little or no
  search (the prime shortcut, the previous game's promotions, known games) and the tactics around them.
* [theory-and-the-prover.md](register/theory-and-the-prover.md): the theory under the search, the prover's proofs, and
  the Python heuristics.
