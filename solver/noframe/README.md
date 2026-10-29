# The prover

The prover proves the frame solver's games optimal on the whole board, without the [frame
model](../../docs/GLOSSARY.md#the-frame-model); [proofs.md](docs/proofs.md) says how far its proofs reach.  It is
a branch and bound over sets of moves, which takes the recorded score of game n as its target and proves that no set of
moves beats it.  It shares no proof code with the frame solver or with the heuristics, so the frame solver's answers are
checked without assuming anything of the frame model.  Both programs read the committed chain,
`results/optimal.jsonl.gz`, through [the gzip decoder](../gzip/README.md), whose CRC-32 check makes a decoding error
fail loudly rather than give both programs the same wrong game.  The [glossary](../../docs/GLOSSARY.md) defines the
project's words.

Why that matters: the frame solver ([its README](../frame/README.md)) searches the promotions of the frame model, whose
answer is opt(n) exactly when [statement U](../../docs/GLOSSARY.md#the-frame-model) holds at n: some optimal game plays
every source with three or more prime factors that setup keeps
([docs/theory/exactness.md](../../docs/theory/exactness.md), where the model's other rules are proven;
[solver/docs/design/frame-model.md](../docs/design/frame-model.md)).  Where the prover proves opt(n), U holds.  The
sources that setup keeps, [U\*](../../docs/GLOSSARY.md#the-frame-model), are the heaviest playable set of sources
wherever [the cardinality conjecture (★) (S4)](../../docs/theory/sources.md#the-cardinality-conjecture) holds ([the
uniqueness theorem (Q1)](../../docs/theory/uniqueness-and-longest-games.md#the-uniqueness-theorem)).

The prover also lists every optimal set of a game, finds the longest games, certifies conjecture (★) at scale, and
searches for adversarial instances.  It rests only on [the theory's](../../docs/THEORY.md) theorems and on elementary
bounds, and every exact method in it is tested against brute force.  The theorems are these:

* [the playability theorem (P2)](../../docs/theory/playability.md#the-playability-theorem) and [the peel theorem
  (P4)](../../docs/theory/playability.md#the-peel-theorem), with peel as the playability oracle: it decides playability
  by peel;
* [the residual theorem (P5)](../../docs/theory/playability.md#the-residual-theorem), [the core theorem
  (P7)](../../docs/theory/playability.md#the-core-theorem) and [the exclusion theorem
  (P8)](../../docs/theory/playability.md#the-exclusion-theorem);
* [the parity vectors](../../docs/theory/parity-bound.md#the-parity-vectors) of [the parity bound
  (W1)](../../docs/theory/parity-bound.md#the-parity-bound);
* [the bottom-layers theorem (F4)](../../docs/theory/frame-model.md#the-bottom-layers-theorem), the bottom layers of
  every optimal set, in the proofs that start from its roles, which is the default of `exact`.

proofs.md lists what each proof rests on.  This document says *pick* for a
[move](../../docs/GLOSSARY.md#the-game), *forced picks* for the numbers fixed in at a node (the frame solver's [forced
member](../../docs/GLOSSARY.md#the-search) is the same idea for its own search), and [upper
part](../../docs/GLOSSARY.md#the-frame-model) for a set's sources.

What the prover established, assumption by assumption
([before-and-after.md](docs/before-and-after.md#what-existed-before) has the evidence before it):

| assumption | what the prover established | grade |
|---|---|---|
| (★): descending greedy is maximum-cardinality on every component and truncation of the sources' instance, hence U\* is the heaviest playable set of sources ([the sources conjecture (S1)](../../docs/theory/sources.md#the-sources-conjecture)) | **every game n = 2 .. 100,000** (602,777,907 greedy rejections) and **902 games to 2,000,000** (60,183,674 more): every rejection a GF(2) linear dependency, which certifies (★) for the whole game; the five components that the earlier evidence left undecided settled; no counterexample | certified per game (a bound met), open in general |
| greedy removal of impossible sources in frames (`removeImpossible`) | at the root (where it shapes the search space) it *is* descending greedy on each component of the sources, so it is certified with (★) for every n ≤ 100,000 | certified per game |
| the frame model as a whole: `unzip`'s mandatory moves, freedoms, closed frames inert, promotions one frame forward, greedy removal | **opt(n) proven equal to the recorded score, on the whole board, for every n = 2 .. 2175**, the committed chain as far as it reached when the proofs were made | verified (exact, on the whole board) |
| uniqueness: one optimal set per game, and every optimal game's upper part U\* | **U\* is the unique heaviest playable set of sources wherever (★) holds** ([Q1](../../docs/theory/uniqueness-and-longest-games.md#the-uniqueness-theorem)), so to 100,000; every optimal set enumerated, on the whole board, in 954 of the games 2..989 (the other 34 with the sources fixed to U\*): **the optimal set is unique for every n ≤ 424, not at 425**; 30 games have two or three; every optimal set found has upper part U\* | proven (the sources, given (★)); measured (whole game) |
| the longest games: an optimal game plays the most moves a game of the board can (the wiki's open question) | **false**: at n = 833, 834 and 835 every optimal game is one move short of the longest; every other game to 1300 is a longest game, the difference never above one | refuted (on the whole board); measured to 1300 |

Its files, in [docs/](docs/):

* [star.md](docs/star.md) certifies (★) by a statement of linear algebra, **(♦)**: every member
  that descending greedy rejects is a GF(2) combination of members it kept above it.  Statement (♦) is cheap to check
  exactly (game 2,000,000 takes 2.7 minutes), and it holds in every game checked.
* [proofs.md](docs/proofs.md) proves the recorded scores to 2175 by a branch and bound over the picks, limited
  by the Franklín–Moniot bound and the *parity bound*, whose gap is about a tenth of the Franklín–Moniot bound's.  The
  chain's games past 2175 are proven within the frame model alone, not by the prover.
* [star.md](docs/star.md#adversarial-search-what-protects-greedy) also searches Taxman-shaped synthetic instances for
  what protects greedy: the *window law*, taken with respect to the same ranking of the primes that decides which
  products fall in the window (n/2, n].
* [uniqueness.md](docs/uniqueness.md) lists every optimal set of a game, for about the cost of proving its score.
  The optimal set is unique for every n ≤ 424; from 425 on, some games have two or three, with the same upper part.
* [longest-games.md](docs/longest-games.md) finds the longest games.  An optimal game need not be one:
  game 833 has a game of 363 moves, while every optimal game has 362.
* [reproducing.md](docs/reproducing.md) gives the commands, and [before-and-after.md](docs/before-and-after.md) the
  evidence before the prover and what remains open.
