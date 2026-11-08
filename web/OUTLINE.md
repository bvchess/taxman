# Outline of the site

The plan of the site *Taxman: Playing optimal games*, page by page.  Each entry below is one page of the site: the
introduction, which is the front page, then one page for each chapter of the story.  An entry says the point of its
page, the beats of its story, its figures, what its right column holds and the findings it carries as asides.  The
entries are meant to be moved, merged and cut.

## What holds on every page

* **The three columns.**  Navigation on the left.  The story in the center, for a high school senior who knows
  primes and factors.  On the right, the technical layer: each result's precise statement, its standing and a link to
  its proof, technical notes, and links to the code, the data and the literature.  A reader who never looks right misses
  nothing of the story.  So no page needs to be set apart: the search is a chapter like the others, and its depth
  sits on the right.
* **Standings.**  Every claim says where it stands: proved, conditional (proved given statement U),
  proved by computation, measured, or open.
  The words are the theory's ([statements.md](../docs/theory/statements.md#the-statements)).
* **Numbers come from the data.**  Counts, scores and shares are read from `results/optimal.jsonl.gz` when the page is
  built or loaded, never typed.
* **One name for each idea**, the site glossary's (`glossary.html`).  A term is defined where it first appears, and a
  page links it to the glossary.
* **"To our knowledge"** appears once, on the About page, where the site says what this work adds.

## How the site is written

The rules the owner's reviews have set, on top of [docs/WRITING.md](../docs/WRITING.md):

* **Lead with what the reader came for.**  The game, the play and the result come first; qualifications such as what
  a score rests on go lower, or to the right, never in a page's first lines.
* **Say less.**  A sentence that does not move the story goes: history, a statistic the paragraph does not need, a
  pointer to another page.
* **Do not explain the site to its reader.**  No note on what a column is for, no "defined in the glossary", no link to
  the style page.  The notation page is there for a reader who wants it.
* **Be exact about standing, even in an example.**  A passing example must be as true as a theorem: taking the largest
  prime first is proven (the prime lemma), so it is never offered as a heuristic.
* **Name things unambiguously.**  "The blue button", when a figure has several.
* **One look, one meaning.**  A control never looks like a standing pill; a hover never resizes anything; a color means
  one thing everywhere.
* **Calm interaction.**  Nothing on the page moves while a reader plays a figure.
* **A figure says one thing.**  What does not fit its idea leaves it.
* **The site is self-contained.**  Its own glossary, notation and fonts; GitHub is for depth: proofs, code and data.
* **Game n is not a game of n.**  Game n, the game on 1 to n, has a board, a best score, a ceiling and optimal sets;
  a game of n, one play of it, has moves, opens with a prime and scores.  Never give game n a play's verbs ("game 22
  opens with 19" is "an optimal game of 22 opens with 19"), and never say one is the other ("the moves of a game of
  n − 1 are a legal game of n", not "a game of n − 1 is a game of n").
* **The prose never cites the site's own code.**  Notes point to the theory and to the Rust programs, never to the
  JavaScript that draws the figures.
* **Say why a narrow case matters.**  A result stated for a narrow case says what part of the problem the case is, as
  the loop rule's paragraph says that layer 2 is its case and chapter 3 relies on it.
* **A note stands beside what it is about.**  A margin note on what a collapsed along-the-way shows appears only while
  it is open.
* **Let the reader play the idea** wherever an idea can be played, as the game on the maximal-factor graph lets them
  play the lifting lemma.
* **Link lightly.**  A glossary term links once a section, at its first use, with the quiet `gloss` style.
* **Say what a fact means.**  A result comes with why it matters: "splitting the board into frames costs the bound
  nothing" is what the equal bounds at the root mean.
* **One topic a paragraph.**  A sentence on another subject gets a paragraph of its own, or goes.
* **Name the thing.**  "Weighted matroid intersection", not "a classical algorithm".  One idea has one name, given
  where the idea first appears, and the glossary defines every name a page relies on.
* **Spell it out.**  No shorthand that makes the reader decode it: "one number lower", "some choice of factors gets
  an odd number" and "the test of chapter 5" each say plainly what they mean instead.  Steps go in the order they
  happen.
* **Keep the words true to the pictures.**  A number is a node; the lines are its relations to its factors.  The text
  takes a figure's examples in turn, the figure's heads say who decides, and its caption says what its marks are.
* **Active voice.**  Say who does what: "descending greedy keeps the source", not "the source is kept".  The passive
  voice is unwelcome.
* **Titles say what a part is for.**  "Comparing bounds", not "Counting payment schemes".  A chapter's subhead is
  general enough to stand on its own, and a chapter goes from technique to results without turning back.

## The arc

The story starts with the introduction, the front page, which explains the game and gives the headline.  The chapters
after it explain how we found optimal games past 5000, a chapter a page.  Each chapter takes away part of the problem.
The introduction shows one game's problem shrinking, for game 5060, in the chapters that shrink it; its rows carry the
chapters' titles but no numbers, since the chain, statement U and the results do not shrink one game's problem, and
the bound's work shows in the search's row:

| step | what is left to choose | about |
|---|---|---|
| the game as it is played (the front page) | an order of moves | 10^16,548 orders |
| a game is a set | a set of moves | 10^1,523 sets |
| the top half | which numbers up to n/2 to play | 10^762 sets |
| frames | which candidates to promote | 10^415 sets of 1,380 candidates |
| the search | the nodes a search explores | 1.2 billion nodes |

The chain settles 65% of the games with no search at all, which the chain's chart shows.  The figures are rough counts
(n!, 2^n, 2^(n/2), 2^candidates) and say so in the right column; the last is the recorded search of game 5060.

## Introduction (the front page)

**Point.**  Taxman is easy to play well and hard to play perfectly; we play an optimal game for every n up to the
chain's end, and this site explains how.

* **Headline:** *Taxman*, with the subtitle *Playing optimal games*.
* **Lede:** what we did: an optimal game for every n from 1 to the chain's end; this site explains how.
* **The game:** the rules as a list, ending with what it means to beat the taxman, then a game to play (n from 2 to 30,
  with a button that plays an optimal game).
* **Playing well is easy:** a simple strategy with no search scores 99.85% of the optimal points up to 1000.  The last
  fraction of a percent is the whole problem, because only a perfect game is optimal, and only a proof shows it is.
* **How we did it:** the shrinking problem above, as a figure, each step linked to its chapter.
* **The chain:** the chart of every game: the player's share, and how each game was settled.
* **Along the way:** the findings as cards, each linked to the chapter that explains it: three factors of tax (40),
  one search for eight games (2512 to 2519), more moves and fewer points (833) and two optimal sets (425).
* **Check it yourself:** the data, the programs and the theory on GitHub.
* **Right column:** the rules precisely, opt(n) and the Taxman sequence (OEIS A019312); the heuristics' claims and how
  they were measured (`heuristics/README.md`); how to check the chain; what supports the scores (proven outright to
  2531, given statement U past it), beside "Check it yourself" and not at the top, until chapter 7 gives it its home;
  how to cite the site.

## The story

### 1. A game is a set

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

**Asides.**  *Three factors of tax:* in game 40, one move of an optimal game pays 6, 10 and 15, yet no game to
2175 needs one (its home in the repository is to be written; the wiki's Questions page has it).  *More moves, fewer
points:* in game 833 every optimal game makes 362 moves, but a game can make 363 (the length proposition, Q4).

### 2. Walking up the chain

**Point.**  Most games are easy if you know an optimal game of the one before.

* Game n − 1's games are games of n, and n slips into one, so the score always rises.  Removing n from a game of n
  leaves a game of n − 1, so it rises by at most n: the ceiling.  The two bounds tie each game to the one before, and
  the games hang together like the links of a chain.
* When an optimal set of n − 1 still plays with n added, game n is at its ceiling, and done.
* At a prime n, swap the old largest prime for n: the prime formula.
* Often game n poses exactly the search problem of the game before it, and that game's answer carries over: a repeat.
  The solver looks for one once the problem is set up, before it searches (chapter 5).
* Together these settle about 65% of the games with no search.  That is why the result is a chain: game 5000 is reached
  by walking there.
* Which games need a search can usually be told from n alone, from its cofactor: n over its largest prime factor
  (measured).  Those still get a start and a ceiling from game n − 1 (chapter 5).

**Examples.**  Games 9 to 13, small enough to follow by hand: game 12 from game 11 in the opening, 10 fits, 12 does not,
and 13 is a prime.  The repeat is game 39, which the semiprime formula also settles.

**Figures.**  Game 12 built from game 11's optimal game, animated.  The chain of bounds, games 1 to 16.  Game n − 1's
optimal game over game n's, where n fits (10) and where it does not (12), each after its paragraph.  The prime swap
(13), in the same rows.  The whole chain, a cell for each game, shaded by how it was proven.

**Right column.**  The prime lemma (N1), monotonicity (N3), the ceiling lemma (N4), the prime formula (N5), the step
theorem (N6), the half lemma (N7), the semiprime formula (J3), the repeat lemma (R6, its last part given statement U).

**Asides.**  *When n is twice a prime:* n always fits, since Bertrand's postulate puts a prime between n/2 and n;
game 14.  *One search, eight games:* games 2512 to 2519 all rest on one search, the longest such run.

### 3. The top half

**Point.**  In the games that need a search, the numbers above n/2 carry about 93% of the score (measured), and one
greedy pass chooses them.

* Where n fits or is a prime, game n − 1 has settled game n (chapter 2); game 21 is neither, so it starts here.
* A number above n/2 divides nothing else on the board, so it is never tax: a move or nothing.  These are the sources.
* Layers: a number's count of prime factors.  A number's maximal factors sit one layer down.
* Descending greedy: from n down, keep each source that can still be played with those kept.  What it keeps is U\*.
* In layers 1 and 2 it is provably right.  A number with two prime factors is an edge between two primes; a set of them
  can be played exactly when it has no loop, and greedy builds the heaviest forest.
* Above layer 2, why it might fail: at game 2873 the sources lack the property that makes greedy safe in general, and
  48 of the 720 orders of six of them trap greedy at four, though five fit.  Taken from the largest down, they do not.
  Yet U\* is certified best among the sources in every game to 100,000.
* Whether an optimal game of the whole board plays all of U\* is statement U, the one open statement the frame solver's
  scores rest on past 2531.

**Figures.**  Game 21 in layers (the game-21 figure, from the pot to the sources).  The layer-2 graph of a mid-size
game, greedy adding edges from the largest down and refusing the ones that close a loop.  A chart of the top half's
share of the score in every searched game.

**Right column.**  The bottom-layers theorem (F4), the square theorem (F10), the sources conjecture (S1) and its
certificates ((★), (♦)), where exchange breaks (game 2873), statement U, the uniqueness theorem (Q1, given (★)).

### 4. Frames

**Point.**  What is left splits into frames, small games that barely touch, and only the promotions are left to choose.

* A frame: some sources with every factor that can pay for them.  Game 21 has four.
* A frame with as many factors as moves is closed; the others have freedoms.
* To promote a factor is to play it as a move, one frame forward.  Game 21's promotion of 9.
* The frame model searches only promotion sets.  Its best score is the best score of a game that plays U\*, which is
  opt(n) exactly when statement U holds.
* How much is left: in game 5060, 268 promotions from 1,380 candidates, worth about 7% of the score.

**Figures.**  The game-21 figure, from the frames to the finished game.  A row of a large game's frames, drawn small,
to show the scale.

**Right column.**  The frame lemma (F1), the layer theorem (F3), the winnability lemma (F5), the promotion-limit lemma
(F9), the exactness theorem (X7), the corollary of statement U (X8); the solver's design
(`solver/docs/design/frame-model.md`) and code.

**Asides.**  *Two optimal sets:* every game to 424 has one optimal set; game 425 has two.

### 5. Search

**Point.**  A search tries promotion sets until it proves that none beats the best one found.

* A repeat first: once the frames are set, a game whose search problem an earlier game posed takes that game's
  promotions (game 39, whose new number lands in a closed frame); game 45 changes frame 1 and needs a search.
* Branch and bound: promote a candidate or not; a bound says how much a branch could still gain; a branch that cannot
  beat the best game found is dropped.
* The bound: the figure's simple one, the sum of every candidate that could be promoted alone, is loose (107 at game
  45's root, where the best adds 39).  The frame solver's is of the same kind, the heaviest set that passes a test
  every legal set passes, and far sharper (chapter 6).
* Learning from dead ends: a set of promotions that fails is remembered, so no branch tries it again, and promotions
  that differ only by exchanging two primes are tried once.
* A head start: the search starts from game n − 1's optimal set, near the best (game 5060, measured), and the
  near-repeat cut drops a branch that poses part of game n − 1's problem.
* A game is proven when the whole tree is explored, or when the best game reaches the ceiling, some at their start.
* What it costs: the search grows steeply with the gap between the bound and the best game, and that is why the chain
  stops where it does.

**Figures.**  A small search tree, with the bound and the best game at each node.  The gap and the cost of each
searched game against n.

**Right column.**  The state lemma (R1), the prime-exchange theorem (R2), the dual-bound lemma (R4), the near-repeat
lemma (R7); how a subtree grows with the gap (measured); what was tried and did not help (the notebook's register);
the design (`solver/docs/DESIGN.md`, `search.md`, `gf2-bound.md`, `tactics.md`).

### 6. The parity bound

**Point.**  A bound limits how high a score can go, and the sharper it is, the less a search has to explore.  Ours is
a quick test of even and odd, on the frames of a search or on the whole board.

* The test: list, for each number of a set, the maximal factors outside the set that could pay for it; if some of the
  numbers list every factor an even number of times, the set cannot be played (the loop of chapter 1).  It is linear
  algebra over GF(2), quick to check, and the solver applies it to the promotions frame by frame, at every node.  Odd
  cores are all it misses.
* The whole board: the same test needs no frames, and on the whole board it bounds opt(n), the parity bound; at the
  root, the frame bound plus ΣU* equals it on every board from 2 to 8191 (measured).  It is the prover's bound
  (chapter 7).
* Odd cores: game 45's is the first; the parity bound is exact to 77 and first misses at 78, where an odd core weighs
  in (measured).
* Comparing bounds, last because its second half rests on measurements: first the theory, that Franklín and Moniot's
  bound is looser, a set being allowed by theirs, by the parity bound, or playable when some set of factors gets at
  least one, an odd number, or exactly one scheme; then the measurements, its gap about ten times the parity bound's
  in every game to the end of the chain, and a search with it exploring far more (the prover's searches of every game
  to 1,000 under each bound).

**Figures.**  Game 78's odd core.  The ladder of three conditions, at least one, an odd number, exactly one payment
scheme, on small examples, and the tables of two of them.  Each bound's height above opt(n) in every game of the chain.

**Right column.**  The signed frame theorem (B1) and the frame bound (B2) over GF(2), the parity bound (W1), the first
odd core (C3), the parity bound exact to 77 (measured), the counting theorem (C2), the two gaps (measured).

### 7. Statement U

**Point.**  One open statement stands between the frame model and the best score, and it is proven in every game up
to 2531.

* The frame model's best is opt(n) exactly when statement U holds (X8); it is proved for the numbers of U* with one or
  two prime factors (F4) and open for the rest.
* Every score rests, followed down, on the frame model of some game; up to 2531 that is proven, and past it the scores
  rest on statement U from the first game that needs a search (2532 today).
* The prover settles statement U one game at a time: it finds opt(n) on the whole board, with no frames, bounded by
  the parity bound of chapter 6, and has proven every game up to 2531.
* The evidence past 2531: the lower layers (F4), the sources conjecture to 100,000, the barred bounds (about 80% of the
  numbers U asks about pass, in 2532 to 5060), and the root's heaviest set, which holds them all in every game.
* Where a proof stops: the exchange that settles layer 2 fails at an odd core.

**Right column.**  The corollary of statement U (X8), statement U and its evidence (`docs/theory/evidence.md`), the
prover (its README and `proofs.md`).

The checks that replay every game, the referee and the two verifiers, are not part of the story: chapter 8's "The data,
and how to check it" gives the commands, and the repository documents them.

### 8. Results

**Point.**  Here are the games, how each was proven, and what they show.

* The explorer: choose n, see the moves of an optimal game, its frames, its promotions and how its score was proven.
* The player's share: Hensley proved it tends to a constant between 1/2 and 35/48; our games put it near 0.63
  (measured).
* What the games show: a game usually has one optimal set, and game 425 is the first with two; an optimal game need
  not make the most moves its board allows (games 833 to 835).

**Figures.**  The explorer.  The player's share against n, with Hensley's bounds.

**Right column.**  A game's record, precisely; Hensley (1988); Franklín and Moniot's Theorem 5; the share near 63%
(measured); one optimal set to game 424 (measured); how far an optimal game falls short (measured).

### 9. Summary

**Point.**  The optimal games in brief, for a reader who wants them now; the chapters before it never need it.  The
introduction offers it at its top: "In a hurry? Go straight to the summary."

* The optimal games: how far they reach and what the proofs rest on, the Taxman sequence (OEIS A019312), the file
  results/optimal.jsonl.gz, and commands that download it and take out the moves.  The two verifiers that check every
  line, what a replay does and doesn't show, and commands that run them.
* Game 21, step by step: the bounds from game 20, then the eight steps of the game 21 figure, each linked to the
  chapter that explains it, ending at 144.

**Figures.**  Game 21, all eight steps.

## Glossary

The site's words, each with its meaning and an example, grouped in the order of the story, with their precise forms
in the right column.  Pages link a term to it, as the front page links *proper divisor*; no page links the repository's
glossary.

## Notation

What the marks, colors, standing pills and symbols mean: the three columns, the legend of the marks flat and raised,
the two colors, each standing with its meaning, and the table of symbols.  The navigation links it.  The style
page, a guide for those who build the site, is not linked.

## About

* What this work adds, where "to our knowledge" appears once: the chain, how each score was proven and what it rests
  on, the frame model and its exactness, the peel theorem, the bottom-layers theorem, the counting theorem and the
  parity bound, and the findings above.
* Prior work: the Taxman sequence's earlier terms on OEIS, Hensley, Franklín and Moniot, the published heuristics,
  Robert Moniot's strategies.
* How to cite the site, the license, and the repository.

## Open decisions

* Whether the heuristics deserve more than the front page's paragraph and an aside.
* Whether the explorer is in the first version of the site.
* How many pages: one a chapter for now, merged or split once the pages' lengths are known.  Statement U stays after
  the search and the bound.
* Where the theory's proofs are read: on GitHub, as now, or as pages of the site in the same style.
