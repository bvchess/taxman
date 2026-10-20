# The ledger: bounds between boards

Part of [the theory](../THEORY.md).

The lemmas below pass bounds on opt(n) between games, both ways, so that many games can be proven at once.  The chain
proves the game of n from the game of n − 1 (the ceiling, the prime formula) or by a search of its own.  The ledger
(`solver/frame/examples/ledger/`) keeps, for many boards at once, a lower bound L(n), the score of a game it holds, and
an upper bound U(n) ≥ opt(n), and passes bounds between boards by the lemmas below; a board is proven when the two meet.
Each lemma is an inequality between optimal scores, with the games that carry the lower bound across; the upper bounds
cross by the inequality alone.  A *finished game* of n is one the referee (`replay_finished`) accepts: legal, over (no
legal move left), and opening with the largest prime ≤ n.  Every optimal game is finished (Lemma A of
`accelerate::prime_shortcut`), so opt(n) is the best score of a finished game, and every lower bound of the ledger is
the score of a finished game, replayed before it counts.

**Lemma L1 (monotone).**  The best score never falls: opt(n − 1) ≤ opt(n).  If n is composite and G is a finished game
of n − 1, then G, followed by n if n is still legal after it, is a finished game of n scoring at least ΣG.

*Proof.*  n divides no other number of the board 1..n, so G's moves are legal there with the same taxes and leave the
same pot with n added.  No number below n has a divisor in that pot (G is over, and n divides none of them), so n is the
only move that can be left, and playing it only takes numbers out of the pot: after it, or without it, the game is over.
G opens with the largest prime ≤ n − 1, which is the largest ≤ n since n is composite.  For a prime n the inequality
holds too (Lemma L3 gives more).  ∎

**Lemma L2 (the ceiling, and the game below it).**  No game of n scores more than the ceiling: opt(n) ≤ opt(n − 1) + n.
If n is composite and G is a finished game of n, then G without the move n, followed by legal moves for as long as there
are any, is a finished game of n − 1 scoring at least ΣG − n (ΣG if G does not play n).

*Proof.*  Call G' the moves of G other than n.  Before each move of G', its pot holds the pot of G before the same move,
less n: both start full; a move of both takes the same number and its divisors out of each pot; G's move n takes numbers
out of G's pot only.  So a move of G', which paid in G with a divisor in G's pot, has that divisor in its own: G' is
legal on 1..n − 1.  The moves added after it are legal by choice and leave the game over.  It opens as G does, with the
largest prime ≤ n, which is not n and so is the largest prime ≤ n − 1.  Applied to an optimal game of n this is opt(n
− 1) ≥ opt(n) − n, the ceiling of Theorem 9 and `board::promotion_cap`.  ∎

**Lemma L3 (the prime formula, both ways).**  Let p be prime and q the largest prime below it.  Then opt(p) = opt(p −
1) + p − q.  A finished game of p − 1 with its first move q exchanged for p is a finished game of p, and a finished game
of p with its first move p exchanged for q is a finished game of p − 1; the score changes by p − q and by q − p.

*Proof.*  The equality is Theorem 9 (prime), proven again with the exchange in `accelerate::prime_shortcut` (Lemmas A
and B there).  The exchange needs of the game only that it opens with q (with p), which a finished game does, not that
it is optimal: p and q are inert on both boards (2q > p by Bertrand's postulate, p being the next prime after q, and p
is the largest number), each takes only 1 when it opens, so every later move keeps its payer, and at the end the pots
differ only in q and p, which have no divisor left once 1 is gone, so the game stays over.  (p = 2: the game of 1 has no
move, and [2] is the game of 2.)  ∎

The design of the ledger stated the formula as opt(p − 1) + p − 1; it is + p − q, and the construction needs no optimal
game, only a finished one.

**Lemma L4 (repeats).**  If boards n and m pose the same search problem, Π_n = Π_m (solver/docs/design/same-search.md),
then every promotion set S of it, played on the board of n (`solution_for_promotions`), is a legal game of n scoring
in_the_bag(n) + ΣS; and, given U at n, opt(n) − in_the_bag(n) ≤ opt(m) − in_the_bag(m).

*Proof.*  The first is the soundness half of Theorem M, which every search relies on when it offers an incumbent.  For
the second, equal problems have the same promotion sets, so P\*(Π_n) = P\*(Π_m); by the soundness at m, in_the_bag(m) +
P\*(Π_m) ≤ opt(m); and given U at n, opt(n) = in_the_bag(n) + P\*(Π_n) (the corollary of "The upper part").  ∎

So the lower bound crosses a repeat only as the promotions of a game that was made from them (a search's or the start's
game); the ledger also tries the moves of any other game that are candidates, and keeps the result only if the referee
accepts it, as any game.  The upper bound crosses as U(n) ≤ U(m) − in_the_bag(m) + in_the_bag(n), and rests on U at n
besides what U(m) rests on.

**Lemma D (the dual bound of a stopped search).**  Let a search of Π_n that a limit stopped have the incumbent worth I
(a promotion sum) and, in its final checkpoint, the open units u = (P_u, C_u): the promotions of a node and the
candidates its loop has still to try.  Then P\*(Π_n) ≤ max(I, max_u (ΣP_u + B_u)), where B_u is the heaviest common
independent set over C_u of Theorem 11's matroids contracted by P_u, of at most `max_additional` members.

*Proof.*  The search is exact (solver/docs/design/search.md): each pruning drops only sets no heavier than the
incumbent of its time (at most I), sets that cannot be promoted, and, with the tactic `symmetry`, sets that break an
inequality every heaviest set keeps (Theorem 13).  The checkpoint's units are the roots of every subtree not yet
explored, which is what makes a resume from it exact.  So a heaviest promotion set H that keeps the inequalities is
either no heavier than I, or is P_u ∪ F with F ⊆ C_u for an open unit u.  Then F is a completion of u's node: by Theorem
11 it is common-independent in the contractions, and it has no more members than the freedoms allow
(`bounds::max_additional`), so ΣF ≤ B_u.  ∎

The root's GF(2) bound, the upper end of `wall landscape`, is the case of a search stopped before its first node (one
unit, the root).

**Lemma F (the floor).**  Let a search of Π_n start from a *floor* F, a promotion sum: an incumbent worth F and no
promotion set, so that it prunes against F as against a set of that weight, and a set it finds heavier than F becomes
the incumbent as usual.  If the search completes and has found no set, then P\*(Π_n) ≤ F.  If it finds a set, that set
is a promotion set worth more than F, and if the search completes, the last one it found is worth P\*(Π_n).

*Proof.*  Let H be a heaviest promotion set that keeps the inequalities; it weighs P\*(Π_n) (Theorem 13).  The incumbent
only rises, so it is worth at least F at every moment.  Each pruning of the search drops only sets no heavier than the
incumbent of its time, sets that cannot be promoted, and, with the tactic `symmetry`, sets that break an inequality
every heaviest set keeps (Theorem 13), as in Lemma D; the search reads the incumbent's value and never its set, so a
value with no set prunes as a set of that weight does.  The set H is not of the last two kinds, so a completed search
either reached H and offered it, or dropped it as no heavier than the incumbent of that time.  Either way ΣH ≤ I, the
incumbent at the end.  If nothing was found, I = F and P\*(Π_n) = ΣH ≤ F.  If a set S was found, I = ΣS, and S is a
promotion set, so ΣS ≤ P\*(Π_n) = ΣH ≤ ΣS.  ∎

A floor at P\* is a search seeded with an optimal set, tree for tree; a floor above it prunes more, and a floor below it
finds an optimal set.  A floor at or above the root's bound, or the ceiling, ends the search at its first check.  For a
search that a limit stopped, Lemma D holds with the floor as the incumbent when nothing was found: P\*(Π_n) ≤ max(I,
max_u (ΣP_u + B_u)), I being the larger of F and the weight of the heaviest set found.

*In the ledger.*  Every lower bound is a finished game, so L(n) ≤ opt(n) holds by the referee alone.  Every upper bound
is a bound of the frame model at some board (a root bound, a dual bound, a completed search, a completed search under a
floor with nothing found: each bounds in_the_bag + P\*, which is opt given U at that board, by the corollary of "The
upper part"), carried by the lemmas, which hold unconditionally but for the second half of L4 (U at the board it is
carried to).  So a board whose bounds meet is proven within the frame model, as the committed games past the prover's
proofs are, given U at the boards its upper bound came through.  An L above a U would be a bug, or a counterexample to U
at one of those boards, since L is a real game.
