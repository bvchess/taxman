# Writing the docs

The docs should be readable by someone outside the project: a programmer new to the code, or a mathematician who knows
Taxman but not this solver.  These rules get them there; the sources at the end give the reasons.

## 1. Know which kind of document it is

Each document serves one need, and the kinds stay apart, linked to each other.  A *how-to* gives the steps of a task
(the RUNBOOK, the README's commands).  A *reference* describes, neutrally, for looking up (the glossary, the file
formats).  An *explanation* says how something works and why (DESIGN.md, THEORY.md).  A *tutorial* teaches by doing.
Two more kinds sit outside these four: the lab notebook is the record of what was tried, and STATUS and AGENDA are
time-bound.  A passage in the wrong kind of document moves, or becomes a link.  A how-to gives the advice that holds for
every run; the plan, the values and the record of one campaign belong to its notebook entry, and the how-to links it as
an example.  A checklist of what to do every time is a how-to (the rules for every launch); a list of what is done and
what remains is a scratchpad, not a doc, and STATUS holds the state of the work.

## 2. Lead with the point

A section, and each paragraph, opens with its point in plain words.  Then it says why the point matters, then gives the
details: the code names, the cases, the proofs.  Provenance (commits, runs, dates, timings) comes last, or behind a
link.

Not "Of the 2946 games of the committed chain 1..3440 that need a search, 1457 pose the problem of the game before them"
but "Moving from one board to the next often leaves the search untouched."

## 3. End each sentence with the new point

A sentence starts from what the reader already has and ends with what is new, where the emphasis falls.  A sentence is
too long when it carries more points than it has ends, not at some word count; so the fix is one point per sentence, not
a mechanical split.  Use one parenthesis at most, and never nest them.  Put three or more parallel facts in a list.

## 4. Define each term before using it

Say what a word means where it first appears, or link the [glossary](GLOSSARY.md).  Give each
idea one name and keep to it: two names make the reader wonder whether they are two ideas.  Internal labels (A1, N3,
R4, statement U) appear only after the text has said in words what they stand for.  Put a small example beside a
general statement: "12 has the maximal factors 6 and 4."

## 5. Give every fact that can change one home

Numbers, commands, status and terms each live in one place, and the other docs link to it.  Explanations are said once
too, but not to the point of sending the reader to look up every idea: a few words that explain an idea where it is used
are a kindness, and a link gives the rest, while a longer explanation, such as the rules of the game, lives in one
place.  A copied number is never fine, because it drifts ("the committed chain 1..3440" was wrong as soon as the
chain grew).  Avoid time-relative words such as "currently", "now" and "new": state the fact, or date it.  A rewrite
keeps every fact its reader needs, and moves one that matters to few readers to a paragraph of details at the end of its
section.  History and provenance that the notebook, git or another doc already holds go, or become a link; a fact with
no other home stays, among the details.

## 6. Mathematics

Define notation before using it.  State the result first, then the proof.  Never start a sentence with a symbol.  Say
where each statement stands: proved, proved given U, measured on some games, or open.  The theory has rules of its own
besides these, in [its guide](theory/writing.md).

## 7. The notebook is the record

An entry is a working log while its work runs, and a record once the work concludes.  While the work runs, the entry
gains dated sections, and nothing in it is rewritten.  When the work concludes, the entry is rewritten as a record for
posterity: its question in a sentence and its answer in a sentence or two, then the method, the results with the
numbers that decided them, and the decision.  The record keeps failures as well as successes and links to its files,
and it leaves out the ups and downs of the journey, which git keeps.  Every entry says when its work started and, once
it has, when it concluded.  The [notebook's front page](../solver/notebook/README.md#how-this-notebook-is-kept) has the
rules for filing it.

## 8. Give each task one file

A reader should be able to read what one task needs and little else.  A file stays under 6,000 words, and should stay
under 5,000.  A longer document becomes a folder of files, split by task rather than by size: a file holds what one kind
of task needs, whole, and two files a task always needs together are one.  The file that bears the document's name,
beside its folder (RUNBOOK.md for runbook/), stays as its index, under 1,000 words, with a line a file saying which task
it serves.  Files have plain names (`runbook/prover.md`), and a citation names the file and, where it needs one, the
heading, never a section number.  In a doc every citation is a link, so that the reader can look the thing up in one
click; a code comment, where links do not render, gives the path and the heading.  A split copies nothing: a passage
that two tasks need lives in one file, and the other links it (rule 5).  The notebook's entries are records and keep
their length.

## 9. Write plainly, modestly and briefly

Say what the reader needs in plain words, and no more:

* **Say a thing once.**  Leave out what a word already implies: "optimal" already means proven.
* **Say what a thing does for the reader**, not how it was built or what came before it.  An overview names no
  implementation language, and history belongs in the notebook and in git (rule 5).
* **Prefer plain nouns to the project's labels.**  Name the file rather than the label the project gave it; a label the
  reader does need is defined first (rule 4).
* **Claim modestly and exactly.**  Say what a program can do, and no more, with no drama.
* **"We" is fine. Passive voice is bad.**
* **Name roles, not people.**  "Whoever owns the AWS account approves it", not a name: anyone may work on the project.
* **Write for every reader.**  People and agents read the same docs, and the docs address neither apart.
* **Describe the project as it is.**  No map from old section numbers, and no placeholder kept so that a number stays:
  when a passage moves, the links to it change.
* **In an overview, short beats complete.**  The details live a link away, and a line that does not serve the reader
  goes.

Not "This project finds optimal games of Taxman, a number game, and proves that they are optimal" but "This project
finds optimal games of Taxman, a number game."  Not "The committed chain, `results/strict.jsonl`, holds an optimal game
for every n" but "The file `results/strict.jsonl` holds a chain of optimal games for every n."

## 10. Spell the American way

Docs, comments and the names of code use American spelling: neighbor, labeled, analyze, behavior, center, color,
catalog, optimize.  The committed data keeps the names it was written with, and quoted titles keep theirs.

## The check

The docs test, `solver/frame/tests/docs.rs`, runs in `solver/tools/quick_check.py`.  It checks that every relative link
resolves, that every heading cited by name or by number exists (and every paragraph or list item cited by the bold or
italic words it opens with), and that every document but the notebook's entries keeps rule 8's limits.  It also checks
that neither the guides nor the comments of the code and the scripts use the words the project has retired, or a
person's name where a role belongs, and that the prose and the comments are spelled the American way (rule 10).

## Sources

* [Diátaxis](https://diataxis.fr/compass/): the kinds of document.
* Gopen and Swan, [The Science of Scientific
  Writing](https://www.americanscientist.org/article/the-science-of-scientific-writing) (1990): the order of information
  in a sentence.
* Google, [Technical Writing One](https://developers.google.com/tech-writing/one/) and [timeless
  documentation](https://developers.google.com/style/timeless-documentation).
* Halmos, [How to Write Mathematics](https://www-users.cse.umn.edu/~cberkesc/5385/Spring2018/halmosWrite.pdf) (1970),
  and Knuth, Larrabee and Roberts, [Mathematical
  Writing](https://jmlr.csail.mit.edu/reviewing-papers/knuth_mathematical_writing.pdf).
* Schnell, [Ten Simple Rules for a Computational Biologist's Laboratory
  Notebook](https://journals.plos.org/ploscompbiol/article?id=10.1371/journal.pcbi.1004385) (2015).
