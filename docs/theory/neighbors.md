# Neighboring games

Part of [the theory](../THEORY.md).

Game n differs from game n − 1 by one number, and much passes between them.  The best score never falls, and never rises
by more than n.  At a prime it rises by an exact amount, and at its ceiling a game's optimal sets are those of the game
before that stay playable with n added.  The proofs rest on the rules alone and two facts about primes; only one example
below, and the two measured laws at the end, [the shadow law](#the-shadow-law) and [the cofactor
rule](#the-cofactor-rule), draw on the rest of the theory.  Monotonicity, [the ceiling lemma](#the-ceiling-lemma) and
the prime formula each come with a construction that turns a game of one board into a game of the other, and needs of
that game only that it be finished, not that it be optimal.  So the lower bounds carried between games are the scores of
real games, which anyone can replay by the rules ([search-rules.md](search-rules.md#bounds-carried-between-games)); an
upper bound crosses by the inequalities alone.

The words of the game (board, pot, move, tax, score, opt(n), optimal set, playable, ceiling) are the
[glossary's](../GLOSSARY.md#the-game), and so is the notation S\* for an optimal set.  A *game of n* is a legal sequence
of moves on the board 1, …, n, and ΣG is its score.  Write q(n) for the largest prime at most n.

Two facts about primes are used throughout.

* **A prime can only open a game.**  The number 1 divides every number, so the first move takes 1 as tax.  A prime's
  only proper divisor is 1, so a prime is legal only as the first move, and a game plays at most one prime.
* **The largest prime of a board divides nothing else on it.**  Bertrand's postulate puts a prime p with m < p ≤ 2m
  for every m ≥ 1.  With m = ⌊n/2⌋ it puts a prime in (n/2, n] for every n ≥ 2, so q(n) > n/2 and 2q(n) > n: on the
  board of n, q(n) divides no other number and is never tax.

## The prime lemma

An optimal game always opens with the largest prime of its board.

**Lemma N1 (the prime lemma).**  Let n ≥ 2.  Every optimal game of n plays q(n), as its first move, and no other prime.
So every optimal set of game n holds q(n) and no other prime.

*Proof.*  A game without q(n) can always take it, either in place of its first move or in front of it, since q(n) takes
only 1 and is never anyone's tax.  Let G be a game of n that does not play q(n).  We build a game that scores more.

1. *G is empty.*  The game whose one move is q(n) scores q(n) > 0.
2. *G opens with a prime r.*  Then r < q(n), since r ≤ n is a prime and not q(n), the largest one.  Let G′ be G with r
   replaced by q(n).  After the first move, G′'s pot less r equals G's pot less q(n): both are the board less 1, r and
   q(n).  Take a later move c of G, with this equality holding before it.  The move c is neither r nor q(n), since a
   prime can only open a game, so c, which is in G's pot, is in G′'s pot too.  In G it took some proper divisor d from
   the pot, and d ≠ q(n), since q(n) divides no other number; so d is in G′'s pot too, and c is legal in G′.  It takes
   the move and its divisors out of each pot, which keeps the equality.  So G′ is a game of n, and it scores q(n) − r
   more than G.
3. *G opens with a composite c.*  Let G′ be q(n), followed by the moves of G.  After q(n) the pot is the board less 1
   and q(n).  The composite c has a proper divisor other than 1, which q(n) did not take, so c is legal.  It takes what
   it took in G, except 1, which is already gone.  So after the first two moves of G′ its pot is G's pot after c, less
   q(n).  The number q(n) is never tax and never a move of G, so every later move of G is legal in G′ with the same tax.
   So G′ is a game of n, and it scores q(n) more than G.

So an optimal game plays q(n).  It plays it first and plays no other prime, since a prime can only open a game.  ∎

On the board of 21, q(21) = 19, and 19, 9, 15, 21, 14, 18, 12, 20, 16 is an optimal game, scoring 144.  The game 17, 9,
15, 21, 14, 18, 12, 20, 16 is case 2: it scores 142, and 19 in place of 17 gains 2.  The game 9, 15, 21, 14, 18, 12, 20,
16 is case 3: it scores 125, and 19 in front of it gains 19.

## Finished games

A *finished game* of n ≥ 2 is a game of n that opens with q(n) and is over: no number left in the pot has a proper
divisor in the pot, so no move is left.  The optimal game of 21 above is finished.  The game 9, 15, 21, 14, 18, 12,
20, 16 is over, since it leaves 11, 13, 17 and 19, which have no divisor left, but it is not finished, since it does not
open with 19.

**Lemma N2 (the finished-game lemma).**  Every optimal game of n ≥ 2 is finished.  So opt(n) is the best score of a
finished game of n.

*Proof.*  An optimal game opens with q(n), by [the prime lemma (N1)](#the-prime-lemma).  It is over, since a move still
left could be played for a higher score.  ∎

The constructions of the lemmas below take finished games to finished games.

## Monotonicity

The best score never falls from one game to the next: a game of n − 1 is a game of n.

**Lemma N3 (monotonicity).**  Let n ≥ 2.  A game of n − 1 is a game of n, with the same taxes, and a game of n that does
not play n is a game of n − 1.  So opt(n − 1) ≤ opt(n), and a set that does not hold n is playable in game n exactly
when it is playable in game n − 1.  If n is composite and G is a finished game of n − 1, then G, followed by n if n is
still legal after G, is a finished game of n, which scores ΣG or ΣG + n.

*Proof.*  The number n divides no smaller number, so it takes no part in any move below it.

1. *The same game on both boards.*  The tax of a move c < n is its proper divisors in the pot, and n is not one of them.
   So a sequence of moves below n takes the same numbers out of the pot on the board of n as on the board of n − 1,
   leaves n in the pot on the board of n, and is legal on one board exactly when it is legal on the other.  This gives
   the first two claims; the inequality follows from an optimal game of n − 1.
2. *Finished.*  Let P be the pot that G leaves on the board of n − 1.  On the board of n it leaves P with n added.  No
   number of P has a proper divisor in P, since G is over, and n is not a proper divisor of anything on the board.  So n
   is the only move that can be left.  Playing it takes numbers out of the pot, and what remains is part of P, in which
   no number has a proper divisor.  Either way the game is over.  It opens with q(n − 1), which is q(n), since n is
   composite.  ∎

On the board of 22 the optimal game of 21 above, followed by 22, is a finished game; it scores 166, which is opt(22).
For a prime n the game G is over on the board of n too, but it does not open with q(n) = n; [the prime formula
(N5)](#the-prime-formula) handles that case.

## The ceiling lemma

The best score never rises by more than n from one game to the next: deleting the move n from a game of n leaves a game
of n − 1.  The bound opt(n − 1) + n is the [ceiling](../GLOSSARY.md#the-game) of game n, and a game scoring it is *at
its ceiling*.

**Lemma N4 (the ceiling lemma).**  Let n ≥ 2.  If G is a game of n, the moves of G other than n, in G's order, are a
game of n − 1.  So opt(n) ≤ opt(n − 1) + n, and if a set S is playable in game n, then S without n is playable in game
n − 1.  If n is composite and G is a finished game of n, then G without the move n, followed by legal moves for as long
as there are any, is a finished game of n − 1, which scores at least ΣG − n.

*Proof.*  Deleting the move n only leaves more in the pot for the other moves.  Let G′ be the moves of G other than n,
in G's order, played on the board of n − 1.

1. *An invariant.*  Before each move of G′, its pot holds G's pot before the same move, less n.  Both pots start full.
   A move common to both removes the move and its divisors from each pot, which keeps the one pot holding the other.
   The move n of G takes numbers out of G's pot only.
2. *G′ is legal.*  A move c of G′ is in G's pot, and is not n, so it is in G′'s pot.  In G it took a proper divisor d
   from the pot, and d ≠ n since d < c < n.  So d is in G′'s pot too.
3. *The bound.*  Applied to an optimal game of n, this gives a game of n − 1 scoring at least opt(n) − n, so opt(n − 1)
   ≥ opt(n) − n.  For a set S played by G, the moves of G′ are S without n.
4. *Finished.*  The moves added after G′ are legal by choice, and they stop when no move is left, so the game ends over.
   It opens as G does, with q(n).  Since n is composite, q(n) is not n, so q(n) = q(n − 1).  It scores ΣG − n, or ΣG
   if G does not play n, plus what the added moves score.  ∎

The construction needs n composite: a finished game of a prime n opens with n, so without that move it does not open
with q(n − 1).  [The prime formula (N5)](#the-prime-formula) exchanges n for q(n − 1) instead.

Game 21 is not at its ceiling: opt(20) + 21 = 124 + 21 = 145, and opt(21) = 144.  Game 22 is: opt(22) = 166 = 144 + 22.
[The step theorem (N6)](#the-step-theorem) says what keeps 21 from its ceiling.  The construction, applied to the
optimal game of 21 above, gives 19, 9, 15, 14, 18, 12, 20, 16, a finished game of 20 scoring 123, one less than
opt(20).

The solver proves a game at its ceiling with no search: a game that scores opt(n − 1) + n is optimal by this lemma.

## The prime formula

At a prime p the best score rises by exactly p − q(p − 1), the gap from the prime before it.  For p = 23, q(22) = 19 and
opt(23) = opt(22) + 4 = 170, and an optimal game of 23 is an optimal game of 22 with its first move, 19, exchanged for
23.

**Lemma N5 (the prime formula).**  Let p ≥ 3 be prime, so that q(p − 1) is the largest prime below it.  A game of p − 1
that opens with q(p − 1) becomes a game of p when that move is exchanged for p.  A game of p that opens with p becomes a
game of p − 1 when that move is exchanged for q(p − 1).  Each exchange keeps a game over if it was over, so it takes
finished games to finished games, and it changes the score by p − q(p − 1) and by q(p − 1) − p respectively.  Hence

    opt(p) = opt(p − 1) + p − q(p − 1).

(For p = 2, opt(1) = 0, since the board of 1 has no move, and opt(2) = 2.)

*Proof.*  Each of p and q(p − 1) takes only 1 when it opens a game, and neither divides another number of either board,
so the rest of the game cannot tell them apart.

1. *Neither divides another number.*  The prime p is the largest number of its board.  As the largest prime of the
   board of p − 1, q(p − 1) has 2q(p − 1) > p − 1, and 2q(p − 1) ≠ p since p is odd; so 2q(p − 1) > p, and q(p − 1)
   divides no other number of the board of p.
2. *From p − 1 to p.*  Let G open with q(p − 1), and let G′ be G with q(p − 1) exchanged for p.  After the first move,
   G's pot is 2, …, p − 1 less q(p − 1), and G′'s pot is 2, …, p − 1: G′'s pot is G's with q(p − 1) added.  No later
   move is q(p − 1) or p, since a prime can only open a game, and none has q(p − 1) as a divisor, by step 1.  So each
   later move is legal in G′ with the same tax, and G′'s pot stays G's with q(p − 1) added.  If G ends over, so does
   G′, since q(p − 1) has no proper divisor left, 1 being gone, and is a proper divisor of nothing.
3. *From p to p − 1.*  Let G open with p, and let G′ be G with p exchanged for q(p − 1).  After the first move, G's pot
   is 2, …, p − 1, and G′'s pot is the same less q(p − 1).  No later move is q(p − 1), and q(p − 1) is a proper divisor
   of none, so each later move of G took a divisor other than q(p − 1) and is legal in G′, with the same tax.  The pot
   of G′ stays G's less q(p − 1).  If G ends over, so does G′, since a number with no proper divisor in a pot has none
   in any part of it.
4. *The formula.*  An optimal game of p − 1 is finished ([N2](#finished-games)), so it opens with q(p − 1), and step 2
   makes it a game of p scoring opt(p − 1) + p − q(p − 1).  An optimal game of p opens with p, and step 3 makes it a
   game of p − 1 scoring opt(p) − p + q(p − 1).  So opt(p) ≥ opt(p − 1) + p − q(p − 1) and opt(p − 1) ≥ opt(p) − p + q(p
   − 1).  ∎

The solver proves a prime game by this formula, with an optimal game of p − 1, exchanged, as its optimal game.

## The step theorem

At the ceiling and at a prime, the optimal sets of game n follow from those of game n − 1.

**Theorem N6 (the step theorem).**  Let n ≥ 2.

* *At the ceiling.*  If opt(n) = opt(n − 1) + n, the optimal sets of game n are exactly the sets T ∪ {n} playable in
  game n, with T an optimal set of game n − 1.
* *Which games reach it.*  Game n is at its ceiling exactly when some optimal set of game n − 1 stays playable with n
  added.
* *At a prime.*  If n = p is a prime p ≥ 3, then T ↦ (T ∖ {q(p − 1)}) ∪ {p} is a one-to-one map from the optimal sets
  of game p − 1 onto the optimal sets of game p.

*Proof.*  [The ceiling lemma (N4)](#the-ceiling-lemma) and [the prime formula (N5)](#the-prime-formula) give the scores;
what remains is to follow the sets.

1. *At the ceiling, an optimal set of n.*  Let S\* be an optimal set of game n.  It scores opt(n − 1) + n, more than
   opt(n − 1), so it holds n: a set without n that is playable in game n is playable in game n − 1, by [monotonicity
   (N3)](#monotonicity), and scores at most opt(n − 1).  Then S\* ∖ {n} is playable in game n − 1, by the ceiling lemma,
   and scores opt(n) − n = opt(n − 1): it is an optimal set of game n − 1.
2. *At the ceiling, the converse.*  If T is an optimal set of game n − 1 and T ∪ {n} is playable in game n, then T ∪
   {n} scores opt(n − 1) + n = opt(n), so it is optimal.
3. *Which games reach it.*  If game n is at its ceiling, step 1 gives an optimal set T of game n − 1 with T ∪ {n}
   playable.  Conversely, if such a T exists, T ∪ {n} is playable and scores opt(n − 1) + n, so opt(n) ≥ opt(n − 1) +
   n, and the ceiling lemma gives equality.
4. *At a prime.*  Let T be an optimal set of game p − 1 and G an optimal game that plays it.  It opens with q(p − 1) and
   plays no other prime ([N1](#the-prime-lemma), [N2](#finished-games)), so the exchange of the prime formula makes it a
   game of p that plays (T ∖ {q(p − 1)}) ∪ {p} and scores opt(p − 1) + p − q(p − 1) = opt(p): an optimal set.
   Conversely, an optimal game of p opens with p and plays no other prime, so for its set S the exchange makes it a game
   of p − 1 that plays (S ∖ {p}) ∪ {q(p − 1)} and scores opt(p) − p + q(p − 1) = opt(p − 1).  The two maps undo each
   other, since an optimal set of p − 1 holds q(p − 1) and not p, and one of p holds p and not q(p − 1).  ∎

The optimal set of game 21 is unique, {9, 12, 14, 15, 16, 18, 19, 20, 21}, and 22 is at its ceiling, so the optimal set
of game 22 is that set with 22 added.  At the prime 23 the optimal set is that of 22 with 19 exchanged for 23.  Game 21
is below its ceiling, and the step theorem says why: the optimal set of game 20 is unique, {10, 12, 14, 15, 16, 18, 19,
20}, and with 21 added it holds 10, 14, 15 and 21.  Their [maximal factors](../GLOSSARY.md#the-game) form a cycle on 2,
3, 5 and 7, and no game of 21 plays all four ([the example under payment schemes](playability.md#payment-schemes)), so
opt(21) < 145.

By the step theorem, the number of optimal sets is the same at a prime as at the game before.  At a ceiling, S\* ↦ S\*
∖ {n} maps the optimal sets of game n one to one into those of game n − 1, so their number cannot grow there.  It falls
there when some optimal set of n − 1 does not stay playable with n: game 685 has three optimal sets, only two of them
stay playable with 686, and game 686, at its ceiling, has two.  The number can grow only at a composite game below its
ceiling.

A step to the ceiling needs every optimal set of n − 1, not just one.  At 686 a test of one optimal set of 685 can miss
the ceiling, if it tests the one set of the three that does not stay playable with 686; [the measurements of
uniqueness](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board) say which set the table of optimal games
records.  Where the optimal set of n − 1 is known to be unique, one test of T ∪ {n} decides whether n is at its ceiling,
and if it is, the optimal set of n is known with no search.

The games of this file are replayed by the rules with `heuristics/core.py`.  The optimal sets of games 20 to 23 are
each unique, by the exhaustive search of `heuristics/evaluation/certify.py`, which takes under a second for these
boards.  The three optimal sets of 685 are listed, with the exchanges between them, in
[solver/noframe/docs/uniqueness.md](../../solver/noframe/docs/uniqueness.md), which enumerates them all.

## The shadow law

Two neighboring games often differ by a single exchanged move, and the move that n replaces then follows from the
factorization of n.  At a prime this is the exchange of the step theorem; elsewhere it is measured.

The *shadow* of an odd number n is the number made from n by replacing each of its prime factors with the prime just
below it, counted with multiplicity.  The shadow of 39 = 3·13 is 2·11 = 22, that of 49 = 7² is 5² = 25, and that of a
prime p is q(p − 1).  An even number has no shadow, since no prime lies below 2.

*The shadow law* (measured).  Let n ≥ 3.  If the recorded optimal sets of games n − 1 and n have the same [promotion
set](frame-model.md#the-frame-model), and the set of n is that of n − 1 with one move x exchanged for n, then x is the
shadow of n when n is odd, and n/2 when n is even.

Game 39 plays the optimal set of game 38 with 22 exchanged for 39, and game 49 that of game 48 with 25 exchanged for 49.
Such an exchange is rare at an even n, and each even n where it happens is a power of 2 or three times one: game 8
exchanges 4 for 8, and game 12 exchanges 6 for 12.  At a prime n, [the step theorem (N6)](#the-step-theorem) proves the
law for every optimal set, with x = q(n − 1).  For an odd semiprime pq the law gives up p′q′, the product of the primes
below p and q, which [an earlier study](../../solver/notebook/the-wikis-open-questions-2026-10/README.md#7-which-n)
had found.

The law says which move an exchange gives up, not when the exchange happens.  It covers only the recorded optimal sets,
and a game may have others.  That a recorded game is optimal is proved on the whole board for the games the prover has
proved, and for the rest given [statement U](exactness.md#statement-u).  [evidence.md](evidence.md#observations) gives
the range and what one must trust, and [the notebook entry on the corner of a near
repeat](../../solver/notebook/search/the-corner-of-a-near-repeat-2026-10/README.md#how-often-it-happens) the count.  A
proof would say why the shadow is the one move of game n − 1 that n displaces.

## The cofactor rule

Many games need no search of their own, since they pose the root problem of the game before them or reach their ceiling,
and which ones do is told mostly by n divided by its largest prime factor.

The *cofactor* of n is n divided by its largest prime factor: 3002 = 2·19·79 has the cofactor 38, and a prime has the
cofactor 1.  It is the cofactor x/r of [the prime-exchange theorem](search-rules.md#the-prime-exchange-theorem), with
x = n and r its largest prime factor.

*The cofactor rule* (measured).  Let n ≥ 188.  If the cofactor of n is a prime or twice a prime, the [root
problem](frame-model.md#the-frame-model) of game n is that of game n − 1, or of game n − 2 when n − 1 is prime.  If the
cofactor is 9, game n is at its ceiling.

Game 3002, of cofactor 38 = 2·19, has the root problem of game 3000, since 3001 is prime, and game 1341 = 9·149 is at
its ceiling.  With the cofactor 1, n is prime and [the prime formula (N5)](#the-prime-formula) gives opt(n).  So a game
whose cofactor is 1, a prime, twice a prime or 9 needs no search of its own: [the repeat lemma
(R6)](search-rules.md#the-repeat-lemma) carries over the score of the game whose root problem it shares, given U, and
at the ceiling the step theorem gives its optimal sets.  The rule fails below 188, last at 187 = 11·17, whose root
problem is not that of 186.

The rule is measured on the games whose root problem the chain records, and nothing yet shows why the cofactor, rather
than something else about n, decides.  [evidence.md](evidence.md#observations) gives the range and what one must trust,
and [the notebook entry on the cofactor of
n](../../solver/notebook/search/which-games-need-no-search-the-cofactor-of-n-2026-10/README.md) the counts, with the
cofactors that are always searched.
