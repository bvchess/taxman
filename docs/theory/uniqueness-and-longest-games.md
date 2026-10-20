# Uniqueness of the optimal set, and longest games

Part of [the theory](../THEORY.md).

## Uniqueness of the optimal set

On the sources the optimal set is unique, given (★); on the whole board it is not, from game 425 on.  Many move orders
play the same set; the question here is whether different *sets* can be optimal.  Two statements must be kept apart: the
sources' instance on its own, where the theory above settles it, and the whole game, where only measurement does.

**Theorem 8 (the heaviest playable set of sources is unique).**  Suppose (★) holds in game n, as it does wherever (♦)
holds (Theorem 7), so in every game n ≤ 100,000.  Then every playable subset S of the sources' instance satisfies |S ∩
{≥ t}| ≤ |U\* ∩ {≥ t}| for every t: listed in decreasing order, the i-th member of S is at most the i-th member of U\*.
Consequently, for every strictly increasing positive weighting w (the values themselves among them), w(S) < w(U\*)
unless S = U\*.

*Proof.*  Playability splits over the components of the incidence graph: a member's factors are listed only by members
of its own component, so the part of a self-covering set in each component is self-covering, and by Theorem 5 a set is
playable iff its part in every component is.  So descending greedy restricted to a component K is descending greedy run
on K, and U\* ∩ K ∩ {≥ t} is its set on the truncation K_{≥t}.  The set S ∩ K ∩ {≥ t} is playable (a subset of S), so
(★) bounds its size by that set's; summing over the components gives the count inequality.  (If S's i-th member s_i
exceeded U\*'s i-th, the threshold t = s_i would hold i members of S and at most i − 1 of U\*.)  Now index the values of
the members c₁ > c₂ > … > c_m and put w(c_{m+1}) = 0.  Layer by layer,

    w(S) = Σ_j (w(c_j) − w(c_{j+1})) · |S ∩ {≥ c_j}|,

with every coefficient positive, so term by term w(S) ≤ w(U\*), and equality needs |S ∩ {≥ c_j}| = |U\* ∩ {≥ c_j}| for
every j.  The jump of that count at c_j says whether c_j is a member, so then S = U\*.  ∎

This is the textbook fact that distinct weights give a matroid a single heaviest basis, transplanted.  The playable sets
are not a matroid (n = 2873, above), but under (♦) they lie inside the GF(2) linear matroid of Theorem 7, of which U\*
is the greedy basis; the count form needs only (★).  So the optimal set of sources is unique in every game to 100,000,
where the brute force of `evaluation/certify.py` had reached 62.

What Theorem 8 does not say is that every optimal *game* has upper part U\*.  An optimal game's upper part is a playable
set of sources (a subset of a playable set is playable), so it is either U\* or strictly lighter above n/2; a game with
a lighter upper part would have to win the difference back below n/2, where the payments of the moves above n/2 compete
with the moves up to n/2.  Whether that ever happens is the strong form of what `evaluation/verify.py` measures, and it
is open; the enumerations below never find it.  For the members of U\* with one or two prime factors it never happens
(Theorem B, in "The frame model: what is proven and what remains"), and the weaker statement that some optimal game
plays the rest of U\* is the one the frame solver's answers rest on (U, there).

Two elementary facts tie a game's optimal sets to its predecessor's.

**Lemma (the prime of every optimal set).**  Every optimal set of a game n ≥ 2 contains q, the largest prime ≤ n, and no
other prime.

*Proof.*  A prime's only maximal factor is 1, so a playable set holds at most one prime (Theorem 1: distinct payments).
The prime q > n/2 (Bertrand) divides no other number of the board, so it is nobody's maximal factor.  Let S be playable
without q, and S′ = S minus its prime, if any.  The set S′ is playable and none of its members lists 1, so S′ ∪ {q} is
playable (q pays with 1; adding a number that is nobody's factor removes nothing from anyone's list) and outweighs S,
since any prime in S is below q.  ∎

**Theorem 9 (optimal sets from the game before).**  Let p be the largest prime below n.

* (At the ceiling.)  If opt(n) = n + opt(n−1), the most opt(n) can be (the *ceiling*), the optimal sets of n are exactly
  the playable sets T ∪ {n} with T an optimal set of n−1.  In particular opt(n) = n + opt(n−1) iff some optimal set of
  n−1 stays playable with n added.
* (Prime.)  If n is prime, opt(n) = n + opt(n−1) − p, and T ↦ T − p + n is a bijection from the optimal sets of n−1 onto
  those of n.

*Proof.*  A playable set of n without n is a playable set of n−1 (n is nobody's factor), worth at most opt(n−1).  At the
ceiling: an optimal S of n is worth more than opt(n−1), so it holds n, and S − n, playable as a subset, is worth
opt(n−1): an optimal set of n−1.  Conversely a playable T ∪ {n} with T optimal for n−1 is worth opt(n).  Prime: let T be
optimal for n−1; by the lemma it holds p and no other prime.  The primes p and n are nobody's maximal factor on either
board (2p > n − 1, and 2p ≠ n), and both pay only with 1, so swapping one for the other preserves playability: T − p + n
is playable, and opt(n) ≥ opt(n−1) − p + n.  Conversely an optimal S of n holds n (it is worth more than opt(n−1), as
n > p), hence no other prime, and S − n + p is a playable set of n−1, so opt(n) − n + p ≤ opt(n−1).  Equality holds
throughout, both maps are well defined between optimal sets, and they are inverse to each other.  ∎

So the number of optimal sets is the same at a prime n as at n−1, can only fall at a game at its ceiling, and can grow
only at a composite n below its ceiling.  For a solver this is reuse made exact: when the optimal set of n−1 is unique,
one peel of T ∪ {n} decides whether n is at its ceiling (if not, opt(n) < n + opt(n−1)), and when it is, the optimal set
of n is known with no search.

**Remark (no exact matroid relaxation of the whole game).**  Game 425 has two optimal sets (they differ by {110, 132,
168} ↔ {98, 150, 162}; both replay by the rules).  So no matroid on {2, …, n} in which every playable set is independent
can have an optimal set as its heaviest independent set, values as weights: both optimal sets would be heaviest
independent sets, and distinct positive weights allow only one (Theorem 8's layer-cake argument, for any matroid).  The
certificate that settles the sources has no analogue that is exact for the whole game.

What is measured (`solver/noframe`, "Is the optimal set unique?"): the prover enumerates every optimal set in 954 of the
games 2..989 (the other 34, the most expensive, with the sources fixed to U\*).  The optimal set is unique in every game
n ≤ 424; 30 games between 425 and 779 have two or three, always with the same upper part, U\*, and the same number of
members, differing by an exchange of equally many numbers up to n/2, of equal sum, each a maximal factor of a member of
U\*.  At 686, a game at its ceiling, the recorded optimal game of 685 cannot take 686, while another optimal set of 685
can: a step to the ceiling needs all the optimal sets of n−1, not one.

## Longest games

An optimal game need not play as many moves as a game of the board can: at n = 833 it plays one fewer.  The wiki's
"Questions" page asks whether it must (its third question), and the Java's `reviseFreedoms` rests on a form of it (a
comment in `UpperBoundEstimator.java` states, as a theorem it has no proof of, that the promotion set with the largest
sum also has the most promotions).  The most moves of a game of n, M(n), is the size of the largest playable subset of
{2, …, n}.  (A game that ends with no legal move left can still be shorter than M(n): maximal playable sets can have
different sizes, as n = 2873 shows above.)

**Lemma (bounds on the most moves).**

* M(n) ≤ ν(n), the matching number of the board's maximal-factor graph (each number joined to its maximal factors).
* M(n−1) ≤ M(n) ≤ M(n−1) + 1, and M(n) = M(n−1) for a prime n ≥ 3.
* Given (★) in game n, a playable set has at most |U\*| members above n/2.

*Proof.*  Theorem 1 pays the moves of a playable set with distinct maximal factors outside it: a matching whose move
side is the set.  A playable set of n−1 is one of n, and a playable set of n less the number n is one of n−1 (n is
nobody's factor).  For a prime n, a playable set holds at most one prime (primes pay only with 1), and one that holds n
becomes, with n swapped for p, the largest prime below n, a playable set of n−1 as large (p is nobody's maximal factor
there and pays with 1, as in Theorem 9).  The part above n/2 of a playable set is a playable set of sources, which
Theorem 8's count inequality at t = n/2 bounds by |U\*|.  ∎

So a legal game of ν(n) moves is a longest game, a certificate that costs one matching.  The bound ν(n) can overshoot
M(n), since a matchable set need not be playable (the n = 21 phenomenon above, counted in moves): first at n = 51, where
ν = 23 and M = 22.

**Proposition (an optimal game need not be a longest one).**  At n = 833 = 7²·17 every optimal game has 362 moves, and a
legal game has 363.  The heaviest game of 363 moves scores 218,197, 29 less than opt(833) = 218,226; it keeps the
optimal game's upper part, U\*, and differs from it below n/2 alone, by four of the optimal game's moves for five
lighter ones:

    optimal game:            {250, 252, 294, 400}         sum 1,196
    heaviest longest game:   {116, 180, 200, 320, 351}    sum 1,167

*Proof (computational, by the prover).*  The prover finds opt(833) = 218,226, and the optimal set unique, with 362
members (`solver/noframe`, sections 3 and 5).  The longer set replays by the rules, and ν(833) = 363, so M(833) = 363.
The branch and bound of `solver/noframe`, weighting the move c by K + c with K above every sum, proves that no playable
set of 363 members sums to more than 218,197.  ∎

The mechanism is a trade of weight for count below n/2.  Every payment is a maximal factor, so at most n/2; a move above
n/2 uses one number up to n/2 (its payment), and a move up to n/2 uses two (itself and its payment), so a playable S has
|S| + |S ∩ [1, n/2]| ≤ ⌊n/2⌋.  At 833 the optimal game's 48 moves up to n/2 and 362 payments use 410 of the 416 numbers
up to n/2; the longest game's 49 and 363 use 412.  The optimal game leaves two more of those numbers idle, and gives up
a move for heavier moves up to n/2: 1,196 against 1,167.

The same exchange is a promotion set of the frame model: the optimal game promotes 48 numbers and the longer one 49,
which is `max_total`, so the "theorem" of the Java's comment is false for the whole frame game of 833
(`solver/frame/tests/soundness.rs`).  The Java's `reviseFreedoms` applied it only to the two-frame game of frames 0 and
1, where it stayed unproven; the frame solver had it as the tactic `revise_freedoms`, measured that it no longer paid,
and removed it.

What is measured (`solver/noframe`, "Does an optimal game play the most moves?"): to n = 1300 the recorded optimal game
is a longest game in every game but 833, 834 and 835, where it is one move short.  In 1,293 of the 1,299 games its moves
meet ν(n), the certificate above; at 51, 182 and 183 ν(n) overshoots by one and a search shows that M(n) is the optimal
game's.  So to 1300 an optimal game is at most one move short of a longest one.  A search that pruned the branches
unable to reach M(n) moves would be unsound: at 833 it would return 218,197.  Whether the gap is bounded in general is
open.
