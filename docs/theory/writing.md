# Writing the theory

The theory lays out the mathematics that the project rests on, and each proof is an argument meant to convince a
reader.  These rules add to [the rules for all the docs](../WRITING.md), which hold here too.

## 1. Write for a reader with an undergraduate background

Assume the reader knows proofs by induction and by contradiction, sets, graphs, and linear algebra over the real
numbers, and has the rules of the game from the [glossary](../GLOSSARY.md).  Anything beyond that, such as GF(2),
matroids or matroid intersection, is explained in [the primer](primer.md), and a proof links the primer where it uses
it.  The primer explains standard mathematics and advances nothing; the other files advance the theory and link the
primer.  A reader is convinced by what they understand, so a proof explains its steps as it goes: why each one is
needed, not only that it holds.

## 2. Rank the results by what rests on them

A result is *load-bearing* when the proof of a score depends on it: the exactness of the frame model, each rule by which
a search prunes, the certificates, and the bounds carried between games.  A load-bearing result gets a complete proof,
with every case.  A result is *explanatory* when it shows why something holds or why an approach fails; it gets the idea
of its proof, marked as a sketch, or a link to the details.  An *observation* is stated as measured, beside its
evidence.  Each file leads with its load-bearing results.  A long proof of a result that nothing rests on costs the
reader effort for little, so it goes to the end of its file or to the notebook.

## 3. Give the idea first, then the steps

Each proof opens with its idea in a sentence or two, so that a reader who believes the idea can move on: "the order of
play is itself an order in which every payment survives".  A proof longer than a couple of paragraphs goes in numbered
steps, each a claim with its reason, so that a reader who doubts one step can read that step alone.  Write no
"clearly", "obviously" or "it is easy to see": give the reason, or leave the word out.

## 4. State each result completely

A statement holds all of its hypotheses, so that it can be quoted alone in [the table of statements](statements.md):
never "under this section's standing hypothesis".  Each hypothesis earns its place: say what fails without it, with the
smallest example that shows the failure.  The exclusion theorem, read on the board, needs separated sets, and the set
{2, 4} on the board of 4 shows why.  Pair an example with a non-example where the line between them teaches something.

## 5. Give each thing one name, and the standard name where there is one

Each result has a name that says what it is about, such as the ceiling lemma, and a short label: the letter of its
file and its number there, as in N4.  The first mention of a result in a file gives its name, its label and a link
to its heading; later mentions may use the label alone where the result is cited often, and the name where it is not.
The open statements that have symbols keep them: U, (★) and (♦).  Each symbol keeps one meaning across the theory, and the
notation table in [statements.md](statements.md) lists them; a letter that serves one proof only must not be one of
those.  Introduce a symbol only for what the text uses again and again, and use words for the rest.

Where mathematics has a standard term, such as topological order or matroid intersection, use it, or give it beside
the project's word.  Flag a project word that means something else elsewhere.  A *factor* here is a number up to n/2,
a role on the board, while in ordinary mathematics a factor is a divisor of some given number.  And in Taxman *greedy*
usually means the strategy that takes the largest legal move, so the theory never says greedy alone: *descending
greedy* is the procedure that builds U\*, and the primer's greedy algorithm is a matroid's.

## 6. Define each term before using it, in one place

The game's terms, from the pot and a move to a maximal factor and a source, are defined in the glossary.  The theory
links them at their first use in each file and does not restate them.  The theory defines its own objects precisely,
the frame model among them, and the glossary gives each of those a sentence and links the theory's definition.  Each
term is defined in one file, and the others link that definition.

## 7. Write about mathematics, not about the code

The theory states and proves facts about mathematical objects and the procedures that compute them.  The frame model is
such an object, and the heart of the theory: its frames, numbering, candidates and promotion sets are defined as
mathematics, and setup's steps as operations on the board.  The names of functions, files, tactics and options stay
out, and so do timings and the history of the code and of past errors, unless a point that carries forward needs them.
That the programs compute what the theory defines is a claim of another kind, which the solver's tests support
([trust.md](../../solver/docs/design/trust.md)).  A section may end with a line that says where the solver uses its
result.

## 8. Choose small examples, and compute them

Put a small example beside each definition, and beside each theorem whose meaning is not plain from its statement.  Use
the game of 21 where it shows the point.  Where a phenomenon first appears in a larger game, use a game near the
smallest one where it appears, and take from it only the numbers that show it: the six numbers of game 2873 that break
the exchange argument, not the whole board.  Compute every example with the project's tools rather than from memory,
and keep each one small enough to check by hand.

## 9. Draw what is easier seen than read

A graph, a frame or a promotion is often clearer drawn than described.  Figures are SVG files in the folder `img/`,
made by the script beside them so that they can be redrawn, and with a white background so that they read on a dark
page.  Never tell two things apart by red against green, or orange against green: use shape, line style or a label,
with color as a second cue at most.

## 10. Say what a computation proves and how to repeat it

A statement proved, certified or measured by a computation names the program, its input, how long it runs, and what the
reader must trust to accept the result: the program's correctness, or a data file.  The check of the relation to
[Franklín and Moniot's paper](franklin-moniot.md) is an example: one command that runs in two minutes.

## 11. Give standing and evidence one home each

The standing of each statement lives in [the table of statements](statements.md#the-statements), which also defines the
words for it, and the computations behind it in [evidence.md](evidence.md).  A file states and
proves its results, and links those two for the rest.
