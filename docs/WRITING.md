# Writing the docs

The docs should be readable by someone outside the project: a programmer new to the code, or a mathematician who knows
Taxman but not this solver.  These rules get them there; the sources at the end give the reasons.

## 1. Know which kind of document it is

Each document serves one need, and the kinds stay apart, linked to each other.  A *how-to* gives the steps of a task
(the RUNBOOK, the README's commands).  A *reference* describes, neutrally, for looking up (the glossary, the file
formats).  An *explanation* says how something works and why (DESIGN.md, THEORY.md).  A *tutorial* teaches by doing.
Two more kinds sit outside these four: the lab notebook is the record of what was tried, and STATUS and AGENDA are
time-bound.  A passage in the wrong kind of document moves, or becomes a link.

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
idea one name and keep to it: two names make the reader wonder whether they are two ideas.  Internal labels (A1, L3,
Lemma D, statement U) appear only after the text has said in words what they stand for.  Put a small example beside a
general statement: "12 has the maximal factors 6 and 4."

## 5. Give every fact that can change one home

Numbers, commands, status and terms each live in one place, and the other docs link to it.  A short summary of an idea
plus a link is fine; a copied number is not, because it drifts ("the committed chain 1..3440" was wrong as soon as the
chain grew).  Avoid time-relative words such as "currently", "now" and "new": state the fact, or date it.  A rewrite
keeps every fact its reader needs, and moves one that matters to few readers to a paragraph of details at the end of its
section.  History and provenance that the notebook, git or another doc already holds go, or become a link; a fact with
no other home stays, among the details.

## 6. Mathematics

Define notation before using it.  State the result first, then the proof.  Never start a sentence with a symbol.  Say
where each statement stands: proved, proved given U, measured on some games, or open.

## 7. The notebook is the record

An entry is dated, is never erased or rewritten to these rules, records failures as well as successes, and links to its
files.  The [notebook's front page](../solver/notebook/README.md#how-this-notebook-is-kept) has the rules for filing it.
A new entry opens with its question in a sentence and its answer in a sentence or two, then gives the method, the
results and the decision.

## Voice

Write plainly, modestly and briefly:

* **Say a thing once.**  Leave out what a word already implies: "optimal" already means proven.
* **Say what a thing does for the reader**, not how it was built or what came before it.  An overview names no
  implementation language, and history belongs in the notebook and in git (rule 5).
* **Prefer plain nouns to the project's labels.**  Name the file rather than the label the project gave it; a label the
  reader does need is defined first (rule 4).
* **Claim modestly and exactly.**  Say what a program can do, and no more, with no drama.
* **"We" is fine. Passive voice is bad.**
* **In an overview, short beats complete.**  The details live a link away, and a line that does not serve the reader
  goes.

Not "This project finds optimal games of Taxman, a number game, and proves that they are optimal" but "This project
finds optimal games of Taxman, a number game."  Not "The committed chain, `results/strict.jsonl`, holds an optimal game
for every n" but "The file `results/strict.jsonl` holds a chain of optimal games for every n."

## The check

The docs test, `solver/frame/tests/docs.rs`, runs in `solver/tools/quick_check.py`.  It checks that every relative link
resolves, and that neither the guides nor the comments of the code and the scripts use the words the project has
retired.

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
