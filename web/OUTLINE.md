# Outline of the site

The plan of the site *Playing optimal games of Taxman*, page by page, for review before any prose is written.  Each
entry says the point of a page, the beats of its main story, its figures, what its right column holds and the findings
it carries as asides.  The entries are meant to be moved, merged and cut.

## What holds on every page

* **The three columns.**  Navigation on the left.  The main story in the center, for a high school senior who knows
  primes and factors.  On the right, the technical layer: each result's precise statement, its standing and a link to
  its proof, technical notes, and links to the code, the data and the literature.  A reader who never looks right misses
  nothing of the story.  So no page needs to be set apart: the search is a chapter like the others, and its depth
  sits on the right.
* **Standings.**  Every claim says where it stands: proved, proved given U, proved by computation, measured, or open.
  The words are the theory's ([statements.md](../docs/theory/statements.md#the-statements)).
* **Numbers come from the data.**  Counts, scores and shares are read from `results/optimal.jsonl.gz` when the page is
  built or loaded, never typed.
* **One name for each idea**, the glossary's.  A term is defined where it first appears and in the right column.
* **"To our knowledge"** appears once, on the About page, where the site says what this work adds.

## The arc

The main story explains how we found optimal games past 5000.  Each chapter takes away part of the problem, and the
front page shows the problem shrinking, chapter by chapter, for game 5060:

| step | what is left to choose | about |
|---|---|---|
| the game as it is played | an order of moves | 10^16,548 orders |
| 2. a game is a set | a set of moves | 10^1,523 sets |
| 3. the chain | (60% of games are settled here) | |
| 4. the top half | which numbers up to n/2 to play | 10^762 sets |
| 5. frames | which candidates to promote | 10^415 sets of 1,380 candidates |
| 6. the search | the nodes a search explores | 3.2 billion nodes |

The figures are rough counts (n!, 2^n, 2^(n/2), 2^candidates) and say so in the right column; the last is the
recorded search of game 5060.

## The front page

**Point.**  We play an optimal game of Taxman for every n up to the chain's end, and this site explains how.

* **Headline:** *Playing optimal games of Taxman.*
* **Lede:** Taxman in one sentence; what we did in two: an optimal game for every n from 1 to the chain's end, proven on
  the whole board to 2175 and, past it, given one open statement.
* **Hero:** the playable game (option D's), n = 10.
* **How we did it:** the shrinking problem above, as a figure, each step linked to its chapter.
* **The chain:** the chart of every game: the player's share, and how each game was settled.
* **Along the way:** the findings as cards, each linked to the chapter that explains it.
* **Right column:** what "optimal" means here, the data and how to check it, how to cite the site.

## The main story

### 1. The game

**Point.**  Taxman is easy to play well and hard to play perfectly.

* The rules, and a game to play (n from 2 to 30, with a button that plays an optimal game).
* The taxman's share: even good play leaves him over a third of the board.
* Playing well is easy: a simple strategy with no search scores 99.85% of the optimal points up to 1000.  The last
  fraction of a percent is the whole problem, because only a perfect game is optimal, and only a proof shows it is.
* How many ways there are to play: the orders grow like n!.

**Figures.**  The game.  A chart of the simple strategy's score against the optimal score (from `heuristics/results`).

**Right column.**  opt(n) and S\*(n) defined; the Taxman sequence (OEIS A019312); the heuristics' claims and how they
were measured (`heuristics/README.md`).

### 2. A game is a set

**Point.**  Which numbers you take decides the score; the order follows, and whether a set can be played is quick to
check.

* Every order of game 21's optimal set that is a legal game, and why they all score the same.
* A move needs only one of its maximal factors in the pot: tax as payment.
* A set can be played exactly when each member can be paid for by its own factor, in an order with no circle.
* Peel: two kinds of safe step decide it, and it never needs to guess.
* The loop: 10, 14, 15 and 21 on the primes 2, 5, 3 and 7; try to play all four.

**Figures.**  The orders of game 21.  Peel, step by step, on a small set.  The loop, played by hand.

**Right column.**  The lifting lemma (P1), the playability theorem (P2), Franklín and Moniot's Theorem 1, which says P2
in the language of matchings, the peel theorem (P4), the two-factor proposition (P11).

**Asides.**  *Three factors of tax:* game 40 has an optimal game in which one move pays 6, 10 and 15, yet no game to
2175 needs one (its home in the repository is to be written; the wiki's Questions page has it).  *More moves, fewer
points:* in game 833 every optimal game makes 362 moves, but a game can make 363 (the length proposition, Q4).

### 3. Walking up the chain

**Point.**  Most games are easy if you know an optimal game of the one before.

* Game n − 1's games are games of n, so the score never falls.  Removing n from a game of n leaves a game of n − 1, so
  it rises by at most n: the ceiling.
* When an optimal set of n − 1 still plays with n added, game n is at its ceiling, and done.
* At a prime n, swap the old largest prime for n: the prime formula.
* Often game n poses exactly the search problem of the game before it, and that game's answer carries over: a repeat.
* Together these settle about 60% of the games with no search.  That is why the result is a chain: game 5000 is reached
  by walking there.

**Figures.**  Adding n to an optimal game of n − 1, animated, with the case where it fits and the case where it does
not.  The prime swap.  A strip over n of how each game was settled.

**Right column.**  The prime lemma (N1), monotonicity (N3), the ceiling lemma (N4), the prime formula (N5), the step
theorem (N6), the half lemma (N7), the semiprime formula (J3), the repeat lemma (R6, its last part given U).

**Asides.**  *The cofactor rule:* whether a game needs a search can usually be told from n alone (measured).

### 4. The top half

**Point.**  In the games that need a search, the numbers above n/2 carry about 93% of the score (measured), and one
greedy pass chooses them.

* A number above n/2 divides nothing else on the board, so it is never tax: a move or nothing.  These are the sources.
* Layers: a number's count of prime factors.  A number's maximal factors sit one layer down.
* Descending greedy: from n down, keep each source that can still be played with those kept.  What it keeps is U\*.
* In layers 1 and 2 it is provably right.  A number with two prime factors is an edge between two primes; a set of them
  can be played exactly when it has no loop, and greedy builds the heaviest forest.
* Above layer 2, why it might fail: at game 2873 the sources lack the property that makes greedy safe in general.  Yet
  U\* is certified best among the sources in every game to 100,000.
* Whether an optimal game of the whole board plays all of U\* is statement U, the one open statement the frame solver's
  scores rest on past 2175.

**Figures.**  Game 21 in layers (the game-21 figure, from the pot to the sources).  The layer-2 graph of a mid-size
game, greedy adding edges from the largest down and refusing the ones that close a loop.  A chart of the top half's
share of the score in every searched game.

**Right column.**  The bottom-layers theorem (F4), the square theorem (F10), the sources conjecture (S1) and its
certificates ((★), (♦)), where exchange breaks (game 2873), statement U, the uniqueness theorem (Q1, given (★)).

**Asides.**  *The greedy that shouldn't work* (game 2873).

### 5. Frames

**Point.**  What is left splits into frames, small games that barely touch, and only the promotions are left to choose.

* A frame: some sources with every factor that can pay for them.  Game 21 has four.
* A frame with as many factors as moves is closed; the others have freedoms.
* To promote a factor is to play it as a move, one frame forward.  Game 21's promotion of 9.
* The frame model searches only promotion sets.  Its best score is the best score of a game that plays U\*, which is
  opt(n) exactly when U holds.
* How much is left: in game 5060, 268 promotions from 1,380 candidates, worth about 7% of the score.

**Figures.**  The game-21 figure, from the frames to the finished game.  A row of a large game's frames, drawn small,
to show the scale.

**Right column.**  The frame lemma (F1), the layer theorem (F3), the winnability lemma (F5), the promotion-limit lemma
(F9), the exactness theorem (X7), the corollary of U (X8); the solver's design (`solver/docs/design/frame-model.md`)
and code.

**Asides.**  *Two optimal sets:* every game to 424 has one optimal set; game 425 has two.

### 6. The search

**Point.**  A search tries promotion sets, and an upper bound proves when nothing better is left.

* Branch and bound: promote a candidate or not; a bound says how much a branch could still gain; a branch that cannot
  beat the best game found is dropped.
* The bound: a quick test that every playable set of promotions passes, though some unplayable sets pass it too.  The
  heaviest set that passes bounds what a branch can gain.
* Learning from dead ends: a set of promotions that fails is remembered, so no branch tries it again, and promotions
  that differ only by exchanging two primes are tried once.
* A game is proven when the whole tree is explored, or when the best game reaches the ceiling.
* What it costs: the search grows steeply with the gap between the bound and the best game, and that is why the chain
  stops where it does.

**Figures.**  A small search tree, with the bound and the best game at each node.  The gap and the cost of each
searched game against n.

**Right column.**  The state lemma (R1), the signed frame theorem (B1) and the frame bound (B2) over GF(2), the odd
cores that are all the bound misses (measured), the prime-exchange theorem (R2), the dual-bound lemma (R4), the
near-repeat lemma (R7); how a subtree grows with the gap (measured); what was tried and did not help (the notebook's
register); the design (`solver/docs/DESIGN.md`, `search.md`, `gf2-bound.md`, `tactics.md`).

### 7. Trust

**Point.**  Every score is checked, and every score says what it rests on.

* A referee replays every game: legal, finished, scoring what is recorded.
* A second program, the prover, shares no proof code with the frame solver and proves every score to 2175 on the whole
  board, with no frames.
* Its bound: a set that can be played is independent over GF(2), and that bound's gap is about a tenth of the best
  published bound's.
* Past 2175 the scores are exact given U, and here is what supports U.
* What each game's record says: how it was proven and what it assumes.

**Figures.**  What rests on what: the proof kinds and their assumptions.  The ladder of three conditions, at least one,
an odd number, exactly one payment scheme, on a small example.  The parity bound and Franklín and Moniot's bound against
opt(n).

**Right column.**  The independence lemma (P10), the parity bound (W1), the counting theorem (C2), the first odd core
(C3); the verifiers and the proof kinds (`solver/docs/REFERENCE.md`); the prover's README.

**Asides.**  *Exact to 77:* the parity bound equals opt(n) for every n up to 77 and fails first at 78, where an odd core
weighs in.

### 8. The results

**Point.**  Here are the games, how each was proven, and what they show.

* The explorer: choose n, see its optimal game, its frames, its promotions and its proof.
* The player's share: Hensley proved it tends to a constant between 1/2 and 35/48; our games put it near 0.63
  (measured).
* What is open: statement U, the sources conjecture, the complexity of Taxman.
* The data, and how to check it.

**Figures.**  The explorer.  The player's share against n, with Hensley's bounds.

**Right column.**  Hensley (1988), Franklín and Moniot (2023), OEIS A019312, `results/README.md`, the verifiers'
commands, the open questions (`docs/theory/statements.md#open-questions`).

## Along the way

An index of the findings, each a card linked to the page that explains it: three factors of tax (40), more moves and
fewer points (833), two optimal sets (425), the greedy that shouldn't work (2873), exact to 77, the cofactor rule, and
the player's share.

## About

* What this work adds, where "to our knowledge" appears once: the chain of optimal games, its proofs and its trust
  model, the frame model and its exactness, the peel theorem, the bottom-layers theorem, the counting theorem and the
  parity bound, and the findings above.
* Prior work: the Taxman sequence's earlier terms on OEIS, Hensley, Franklín and Moniot, the published heuristics,
  Robert Moniot's strategies.
* How to cite the site, the license, and the repository.

## Open decisions

* Whether the heuristics deserve more than chapter 1's paragraph and an aside.
* Whether the explorer is in the first version of the site.
* Whether trust (7) comes before the search chapter (6), so the reader knows what "proven" means before the search
  proves anything.
* Where the theory's proofs are read: on GitHub, as now, or as pages of the site in the same style.
