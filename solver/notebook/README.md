# The lab notebook

The record of what was tried on the frame solver, the prover and the theory under them, kept or not, with the numbers
that decided it.  Each experiment, or round of experiments, is an entry: a folder of this directory, or of `search/` for
the search's experiments.  This page holds the rules for filing an entry and for the register.
[DESIGN.md](../docs/DESIGN.md) says how the frame solver works, and [STATUS.md](../docs/STATUS.md) gives the state of
the work.

An entry names the commits that made each step, so `git show COMMIT` finds the code.  It is true of the code at its
dates, and DESIGN.md is the authority on the code.
The [glossary](../../docs/GLOSSARY.md) defines the terms, and
[solver/docs/design/performance.md](../docs/design/performance.md) the measuring tools: the ladder, the probe and
`taxman bench`.

* [Register of everything tried](#register-of-everything-tried): a row per experiment, by subject, with what decided
  it.  Look here first.
* [terms-changed.md](terms-changed.md): the old words of the dated entries, the words that replaced them, and the
  programs removed since.

## How this notebook is kept

* **What to read.**  Before proposing an experiment, read the register's file of its subject, or search the register
  (`grep -ri WORD solver/notebook/register/`), and read only the entries a task needs.
* **One folder an entry.**  An experiment, or a round of them, is one folder with a README.md in it, the entry, named
  by its title's anchor (`the-frontier-chain-to-3440-2026-10/` for "The frontier chain to 3440 (2026-10)"): a folder of
  `search/` for an experiment on the search (its bounds, branching, learning and tactics), a folder of this directory
  for any other.  A date in the folder's name is the month the work started.  The README.md opens with `# Title
  (YYYY-MM)`, the line `Part of the [lab notebook](../README.md).` (`../../README.md` in `search/`), and, after a blank
  line, `Started YYYY-MM-DD.`, to which `Concluded YYYY-MM-DD.` is added when the work ends.
* **A log while the work runs, a record when it ends.**  While the work runs, the entry is a working log: it opens with
  its question, gains dated sections (`## YYYY-MM-DD: ...`), and nothing in it is rewritten.  When the work concludes,
  the entry is rewritten as a record for posterity: its question in a sentence and its answer in a sentence or two, then
  the method, the results with the numbers that decided them, the decision and what would reopen it, its files and its
  commits ([docs/WRITING.md, rule 7](../../docs/WRITING.md#7-the-notebook-is-the-record)).  The record keeps failures
  and kills, and leaves out the ups and downs of the journey, which git keeps.  A finding on concluded work starts a new
  entry, which links the record.
* **Files.**  The folder holds the data and the scripts the entry made, and a section `## Files` at the end of the
  README.md says what each file is and the command that made it.  A tool that outlives its experiment goes to
  `solver/tools/` instead.
* **The register points to every entry.**  An entry adds its rows to the register's file of their subject (the date,
  the question, the answer with the number that decided it, and where the details are).  The docs test refuses an
  entry that no file of the register links to.
* **Links.**  Link an entry's README.md by its relative path, from another document or from another entry; from an
  entry to this page, `../README.md#ANCHOR` (`../../README.md#ANCHOR` in `search/`).  The docs test refuses a link to
  an entry through this page (`README.md#an-entry`).

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
fields](search/ideas-from-other-fields-the-pilots-2026-10/README.md), that page describes what was computed, on which
games and with which settings, well enough to repeat it.  The date is the day the work started.  The numbers are those
of the page the row points to; a game is named by n, and times are search seconds.

The register is a file a subject, in [register/](register/).  Read the file of your subject whole, or search them all
with `grep -ri WORD solver/notebook/register/`:

* [node-bound.md](register/node-bound.md): the bound computed at every node (the matching and GF(2) bounds) and its
  machinery: conflicts, refinement, forced members, cycle searches, the cost of a node.
* [stronger-bounds.md](register/stronger-bounds.md): the bounds tried on top of it: Lagrangian, LP, cuts and covers,
  the multi-field bound (A1), the root gaps.
* [search.md](register/search.md): the rest of the search: branching, the start and the ceiling, decomposition,
  symmetry, sampling, hard games run alone, the diagnosis of the wall.
* [nogoods-and-conflicts.md](register/nogoods-and-conflicts.md): learning from conflicts: nogoods, their propagation,
  their tables, and carrying them from one game to the next.
* [engine-and-parallelism.md](register/engine-and-parallelism.md): the threads, the splits and the stealing, the
  checkpoints, the set widths, and the build.
* [tactics-and-shortcuts.md](register/tactics-and-shortcuts.md): the shortcuts that settle a game with little or no
  search (the prime shortcut, the previous game's promotions, known games) and the tactics around them.
* [theory-and-the-prover.md](register/theory-and-the-prover.md): the theory under the search, the prover's proofs, and
  the Python heuristics.
