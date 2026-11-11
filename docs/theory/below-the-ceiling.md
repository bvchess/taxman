# Below the ceiling

Part of [the theory](../THEORY.md).

[The ceiling lemma (N4)](neighbors.md#the-ceiling-lemma) bounds the rise of the best score from game n − 1 to game n by
n.  At some multiples of a prime the rise falls short of n by an amount that a few numbers fix.  With r the prime before
p, and r close enough to p, game 8p falls at least 4r short of its ceiling ([the 8p theorem (L2)](#the-8p-theorem)), on
the whole board, and game 12p at least 6r ([the 12p theorem (L4)](#the-12p-theorem)), given an optimal set that holds
four numbers, which [statement U](exactness.md#statement-u) supplies.  A game of n that reaches such a bound is optimal,
so it settles game n from game n − 1 with no search.  Both proofs delete n from an optimal set of game n and put back,
in game n − 1, numbers worth the loss or more, changing a few payments on the way ([the re-payment lemma
(L1)](#the-re-payment-lemma)).  [The cycle lemma (L3)](#the-cycle-lemma) shows why game 12p cannot lose less.

*Words.*  A game of n, q(n), the largest prime up to n, and the [ceiling](../GLOSSARY.md#the-game) are
[neighbors.md](neighbors.md)'s.  Instances, with their members, pools and lists, payment schemes, pay(x) and the
precedence are [playability.md](playability.md#payment-schemes)'s, mf(x) is the set of maximal factors of x, and S_k is
the part of a set S in layer k.  A set S of game n is playable iff each of its layer instances is, by [the layer theorem
(F3)](frame-model.md#the-layer-theorem); the one at k has the members S_k and, as its pool, the numbers of layer k − 1
up to n/2 that are not in S.  [U\*](sources.md#descending-greedy), the sources that descending greedy keeps, is
sources.md's.  A number f is a maximal factor of fs for each prime s and of no other number, so on the board of n it is
a maximal factor of the numbers fs ≤ n alone.

## The re-payment lemma

A member's payment can change, and a number can join a layer, without making a cycle, when the member ends with no edge
in or no edge out.

Within a layer instance, an edge a → b of the precedence ([P2](playability.md#the-playability-theorem)) means that b
lists pay(a): a does not divide b, which lies in its layer, and pay(a), one layer below b, divides it only as a maximal
factor.  The edges depend only on the payments, so a factor that no member pays can enter or leave the pool, and each
scheme stays a scheme, with the same precedence.

**Lemma L1 (the re-payment lemma).**  Take a layer instance with an acyclic payment scheme.

1. Let a member a, or a number a of the layer that joins the members, pay with a factor f that it lists and that no
   other member pays.  The scheme stays acyclic if each other member that lists f has no edge out, or if no other member
   pays with a factor that a lists.
2. If a number x of the layer lists a factor f, and each member that lists f lists a factor of its own, one that no
   other member lists and x does not, then x can join the members with an acyclic scheme.

*Proof.*  A change of one payment changes only the edges at that member.

1. The edges out of a go to the other members that list f, and those into a come from the members whose payments a
   lists.  Under either condition no cycle passes through a, and a cycle that misses a is one of the old scheme.  When a
   joins and no other member lists f, this is [the extension lemma (P6)](playability.md#the-extension-lemma).
2. Each member y that lists f pays with its own factor, by part 1, since no other member lists it, and y then has no
   edge out, since no other member lists its payment and x does not.  Then nobody pays with f, and x joins paying with
   f, by part 1.  ∎

## The 8p theorem

At n = 8p, with p a prime and r the prime before p, game n falls at least 4r short of its ceiling when n ≥ 187 and r is
close enough to p.  The factor 8 can pay for only one of n and 8r.  As in game 248 below, it pays for 8r in game n − 1,
which leaves 4r free to be a move, and for n in game n, so 8r pays with 4r.  The bound holds on the whole board, with no
statement U.

**Theorem L2 (the 8p theorem).**  Let n = 8p ≥ 187 with p a prime, let r = q(p − 1) be the largest prime below p, and
suppose that 9r > n.  Then opt(n) ≤ opt(n − 1) + n − 4r.  So a game of n that scores opt(n − 1) + n − 4r is optimal.

*Proof.*  Take n out of an optimal set of game n, and one of 4p, 4r and 8r can join what is left in game n − 1: it pays
with a factor that n paid with, or that at most one other member lists, and that member has a factor of its own to move
to.

Let S be an optimal set of game n.  It holds n ([monotonicity (N3)](neighbors.md#monotonicity)); let T = S ∖ {n}.  We
find an x ∉ S, at least 4r, with T ∪ {x} playable in game n, and so in game n − 1, as it does not hold n (N3).  Then
opt(n − 1) ≥ ΣT + x = opt(n) − n + x.  The layer instances below are those of sets of game n, and S's scheme at k is an
acyclic payment scheme of S's layer instance at k, which exists by F3.

1. *What lists what.*  The numbers 12p, 9p and 10p exceed n, and so do 12r, 9r and 10r, since 9r > n.  So 4p, 3p, 4r and
   3r are maximal factors of 8p, 6p, 8r and 6r alone, 2p of 4p and 6p alone, and 2r of 4r and 6r alone.  And as p and r
   exceed 3, mf(n) = {4p, 8}, mf(8r) = {4r, 8}, mf(4p) = {2p, 4} and mf(4r) = {2r, 4}.  The numbers n and 8r lie in
   layer 4 and above n/2, as r > p/2, so in no pool.  The numbers 4p, 4r, 6p, 6r and 8 lie in layer 3, and 2p, 3p, 2r
   and 3r in layer 2, and all but 6p and 6r are at most n/2 = 4p.
2. *The pool of layer 3.*  As n ≥ 187, S holds no number of layer 2 up to n/2 ([the square theorem
   (F10)](frame-model.md#the-square-theorem)), so the pool of S's layer instance at 3 holds 2p, 3p, 2r and 3r.
3. *A number of layer 3 joins.*  Let (x, y, f, g) be (4p, 6p, 2p, 3p) or (4r, 6r, 2r, 3r), with x ∉ S.  By steps 1 and
   2, x lists f and not g, only x and y list f, and g, which only y lists, is in the pool.  So S_3 + x is playable
   against the pool of S's layer instance at 3: x joins S's scheme there by L1 (part 2).
4. *Case A: 4p ∉ S.*  Take x = 4p.  Only the layer instances of T ∪ {4p} at 3 and 4 differ from S's, since n, of layer
   4, lies in no pool, and 4p, of layer 3, only in the pool at 4.  At 3 the members are S_3 + 4p, playable by step 3.
   At 4 they are S_4 − n, and 4p leaves the pool.  S's scheme at 4, restricted to them, pays none with 4p, which only n
   lists, so it is an acyclic scheme there.
5. *Case B: 4p ∈ S.*  Then n lists only 8 in S's layer instance at 4, so it pays with 8, and 8 ∉ S.
   * *Case B1: 8r ∈ S.*  Then 8r cannot pay with 8, n's payment, so it pays with 4r, and 4r ∉ S; take x = 4r.  As in
     case A, layer 3 is step 3's, and at 4 the members are S_4 − n and 4r leaves the pool.  In S's scheme at 4,
     restricted to S_4 − n, 8r can pay with 8: nobody else pays with 8, which n paid, or with 4r, which only 8r lists,
     so L1 (part 1) applies.  Then nobody pays with 4r, which can leave the pool.
   * *Case B2: 8r ∉ S.*  Take x = 8r, of layer 4 and in no pool, so that only layer 4 changes, where 8r takes the place
     of n.  It joins S's scheme at 4, restricted to S_4 − n, paying with 8, and as in case B1 nobody else pays with a
     factor that it lists (L1, part 1).
6. *The bound.*  In each case every layer instance of T ∪ {x} is playable, so T ∪ {x} is playable in game n (F3), and
   x ≥ 4r.  ∎

The hypothesis n ≥ 187 enters only in step 2, through the square theorem, which needs it; so L2 rests on F10's
computation of its finite part ([evidence.md](evidence.md#observations)).  The hypothesis 9r > n enters in step 1:
12r > n and 10r > n need only r > 2p/3 and r > 4p/5, but the proof cannot spare 9r > n, which keeps 3r for 6r alone.
Some hypothesis is needed.  Game 184 = 8·23, with r = 19, meets neither, and rises by 136 over game 183, more than the
184 − 76 = 108 that the bound would allow.  Its recorded optimal set plays 91 = 7·13, of layer 2 up to n/2, against step
2, and both 114 = 6r and 171 = 9r, which list 57 = 3r, so that 6r has no factor of its own for step 3.  Nor are the
hypotheses sharp: games 152 = 8·19, below 187, and 232, 296 and 424, where 9r ≤ n, rise by exactly n − 4r all the same.

Game 248 = 8·31, with r = 29, is the first that the theorem covers.  In game 247, 232 = 8r pays with 8 and 116 = 4r is a
move; in game 248, 124 = 4p is a move, so 248 pays with 8 and 232 with 116.  The bound, opt(247) + 248 − 116 = 19,291,
is opt(248), and the recorded optimal set of game 247 with 116 exchanged for 248 is that of game 248.  The same exchange
reaches the bound in every game of the committed chain that the theorem covers, so with the theorem it proves game n
from game n − 1 ([the standing](statements.md#below-the-ceiling), [the computation](evidence.md#observations)).

## The cycle lemma

Four numbers of layer 4 share four factors in a cycle, and a set that holds them cannot also play the two numbers that
would leave them nothing else.

**Lemma L3 (the cycle lemma).**  Let p and r be distinct primes of 5 or more.  A playable set that holds 8p, 8r, 12p and
12r holds at most one of 6p and 6r.

*Proof.*  Let S hold the four, and 6p and 6r too.  As mf(8p) = {4p, 8}, mf(8r) = {4r, 8}, mf(12p) = {6p, 4p, 12} and
mf(12r) = {6r, 4r, 12}, the four list nothing but 4p, 4r, 8 and 12 in S's own instance.  Each of these is a maximal
factor of exactly two of them: 4p of 8p and 12p, 4r of 8r and 12r, 8 of 8p and 8r, and 12 of 12p and 12r.  So every
factor that one of the four lists is listed by another, and the four are a
[core](playability.md#cores-and-the-residual).  By [the core theorem (P7)](playability.md#the-core-theorem), S is not
playable.  ∎

As edges between the two factors each lists, the four are the cycle 4p – 8 – 4r – 12 – 4p, like the cycle of four edges
of game 21 ([payment schemes](playability.md#payment-schemes)).  Game 372 = 12·31, with r = 29, would lose less by
giving up 172 = 4·43 than 174 = 6r.  But the recorded optimal set of game 371 holds 172, 186 = 6p, 174, 248, 232 and
348, so with 172 exchanged for 372 it holds the six numbers of the lemma, and it is not playable.

## The 12p theorem

At n = 12p, with p a prime and r the prime before p, game n falls at least 6r short of its ceiling, when n ≥ 187 and r
is close enough to p, given an optimal set of game n that holds four numbers.  The factor 12 can pay for only one of n
and 12r.  As in game 372 below, it pays for 12r in game n − 1, which leaves 6r free to be a move, and for n in game n,
so 12r pays with 6r.  By the cycle lemma game n cannot keep both 6p and 6r, as losing a smaller number such as 4t would.

**Theorem L4 (the 12p theorem).**  Let n = 12p ≥ 187 with p a prime, let r = q(p − 1) be the largest prime below p, and
suppose that 15r > n.  If some optimal set of game n holds 8p, 8r, 12r and 8t for a prime t with 4p/3 < t ≤ 3p/2, then
opt(n) ≤ opt(n − 1) + n − 6r.

Given statement U at n, the hypothesis holds whenever 8p, 8r, 12r and 8t lie in U\*(n): they lie in layer 4, so the
optimal set that U provides holds them all.  That is where U enters, and the only place, and descending greedy finds
U\*(n) with no search.

*Proof.*  Take n out of the optimal set, and in game n − 1 put back 6p or 6r, or one of them and 4t in place of 4p or
4r.  Each joins through a factor whose other listers have factors of their own to move to, and the cycle lemma keeps 6p
and 6r from being moves together.

Let S be an optimal set of game n that holds 8p, 8r, 12r and 8t.  It holds n ([N3](neighbors.md#monotonicity)); let
T = S ∖ {n}.  We find a set T′ of game n without n, playable in game n and so in game n − 1 (N3), with ΣT′ ≥ ΣT + 6r;
then opt(n − 1) ≥ opt(n) − n + 6r.  As in the 8p theorem, the layer instances are those of sets of game n, and S's
scheme at k is an acyclic payment scheme of S's layer instance at k.  Each T′ below differs from S only in numbers of
layer 4, which lie in no pool, and of layer 3, which lie only in the pool at 4, so only its layer instances at 3 and 4
differ from S's.

1. *What lists what.*  The numbers 18p, 20p, 15p and 14p exceed n, so 6p is a maximal factor of n alone, 4p of 8p and n
   alone, 5p of 10p alone, and 2p of 4p, 6p and 10p alone.  Since 15r > n, r > 4p/5, and 18r, 20r, 15r, 21r and 22r
   exceed n, so 6r is a maximal factor of 12r alone, 4r of 8r and 12r alone, 5r of 10r alone, 7r of 14r alone, and 2r of
   4r, 6r, 10r and 14r alone.  Since 4p/3 < t ≤ 3p/2, 8t is on the board and 12t, 10t and 9t exceed n, so 4t is a
   maximal factor of 8t alone, 2t of 4t and 6t alone, and 3t of 6t alone.  And mf(n) = {6p, 4p, 12},
   mf(12r) = {6r, 4r, 12}, mf(8p) = {4p, 8}, mf(8r) = {4r, 8} and mf(8t) = {4t, 8}.  The numbers n, 12r, 8p, 8r and 8t
   lie in layer 4 and above n/2, so in no pool.  The numbers 6p, 4p, 6r, 4r, 4t, 8, 12, 10p, 10r, 14r and 6t lie in
   layer 3, and 2p, 5p, 2r, 5r, 7r, 2t and 3t in layer 2; 6p is n/2, and 4p, 6r, 4r, 4t, 8 and 12 are below it.
2. *The pool of layer 3.*  As n ≥ 187, S holds no number of layer 2 up to n/2
   ([F10](frame-model.md#the-square-theorem)), so the pool of S's layer instance at 3 holds 2p, 5p, 2r, 5r, 2t, 3t, and
   7r when 14r ≤ n, as at 276 and 444.
3. *Numbers of layer 3 join.*  Against the pool of S's layer instance at 3, with an acyclic scheme of members M: 6p can
   join through 2p when 4p ∉ M, since 10p has 5p of its own; 6r can join through 2r when 4r ∉ M, since 10r and 14r have
   5r and 7r; and 4t can join through 2t, since 6t has 3t (step 1, and L1, part 2).
4. *Case A1: 6p ∉ S and 4p ∉ S.*  Take T′ = T ∪ {6p}.  At layer 3, 6p joins (step 3).  At layer 4, n leaves the members
   and 6p the pool, which only n lists, so S's scheme restricted to S_4 − n serves.
5. *Case A2: 6p ∉ S and 4p ∈ S.*  Then 8p lists only 8, and pays with it, so 8 ∉ S, and 8t pays with 4t, so 4t ∉ S.
   Take T′ = T − 4p + 6p + 4t, which gains 2p + 4t > 6p, as 4t > 16p/3.  At layer 3, from S's scheme less 4p, 6p and 4t
   join (step 3).  At layer 4 the members are S_4 − n, and 4p enters the pool while 6p and 4t leave it.  In S's scheme
   at 4 restricted to S_4 − n, 8p pays with 4p, which no other member lists now, and then 8t with 8, released by 8p, as
   no other member pays 4t or 8 (L1, part 1, twice).  Then nobody pays 6p or 4t, which leave the pool.
6. *Case B: 6p ∈ S.*  By the cycle lemma (L3), 6r ∉ S, and n lists only what S leaves of 4p and 12.
   * *Case B1: 4r ∈ S.*  Then 8r lists only 8, and pays with it, so 8 ∉ S; 8p pays with 4p, so 4p ∉ S; n pays with 12,
     so 12 ∉ S; 12r pays with 6r; and 8t pays with 4t, so 4t ∉ S.  Take T′ = T − 4r + 6r + 4t, which gains 2r + 4t > 6r,
     as t > r.  At layer 3, from S's scheme less 4r, 6r and 4t join (step 3).  At layer 4 the members are S_4 − n, 4r
     enters the pool, and 6r and 4t leave it.  In S's scheme restricted to S_4 − n, 12r pays with 12, released by n,
     then 8r with 4r, then 8t with 8, released by 8r: each takes a factor that nobody pays, and no other member pays a
     factor that it lists (L1, part 1).  Then nobody pays 6r or 4t, which leave the pool.
   * *Case B2: 4r ∉ S.*  Take T′ = T ∪ {6r}.  At layer 3, 6r joins (step 3).  At layer 4 the members are S_4 − n, and 6r
     leaves the pool.  Let σ be S's scheme at 4.  It pays n with 4p or 12, and 12r with 6r, 4r or 12.
     * *If σ pays 12r with 4r or 12*, nobody pays 6r, and σ restricted to S_4 − n serves.
     * *If σ pays 12r with 6r and n with 12*, then 12r pays with 12 instead, released by n.  A cycle would enter 12r
       from a member that pays a factor 12r lists, and only 8r, paying 4r, can be one; if 8r pays 8, 12r has no edge in
       at all.  A cycle would enter 8r from a member h that pays 8.  If nobody pays 8, there is no cycle.  If h = 8p, 8p
       has no edge in, as nobody pays 4p now and 8 is its own, so no cycle reaches it.  Otherwise 8p pays 4p in σ, and σ
       has the edges h → 8p, 8p → n and n → b, for the member b that follows 12r on the cycle, which lists 12.  The
       cycle's walk from b to h misses 12r and 8r, so it is a walk of σ, and σ would hold the closed walk
       n → b ⇝ h → 8p → n, which an acyclic scheme cannot.
     * *If σ pays 12r with 6r and n with 4p*, then 4p ∉ S, 8p pays with 8 and 8r with 4r.  Then 8p pays with 4p, which
       only it lists now, and has no edge out; 8r pays with 8, released by 8p, and has no edge in, as nobody else pays
       4r or 8; and 12r pays with 4r, released by 8r.  If 12 is a move or unpaid, 12r has no edge in.  If some member
       12s pays with 12, a cycle would run 12r → 8r → b ⇝ 12s → 12r, since 8r is the only other member that lists 4r,
       with b a member that lists 8.  The walk from b to 12s misses 12r, 8r and 8p, which has no edge out, so it is a
       walk of σ.  And σ has the edges 8p → b, 12s → n and n → 8p, so it would hold the closed walk
       8p → b ⇝ 12s → n → 8p, which it cannot.  In each of the three, nobody pays 6r at the end, and it leaves the pool.
7. *The bound.*  In each case every layer instance of T′ is playable, so T′ is playable in game n (F3), and it gains at
   least 6r.  ∎

The hypothesis n ≥ 187 enters only in step 2, through the square theorem, so L4 rests on F10's computation of its finite
part.  The hypothesis 15r > n enters in step 1, and the proof cannot spare it only where it keeps 5r for 10r alone: the
other uses need only r > 2p/3.  The prime t enters only in cases A2 and B1, through 9t > n, which keeps 3t for 6t alone,
through 10t > n and 12t > n, which follow from t > 4p/3, and through 8t ≤ n.  Some hypothesis is needed.  Game
204 = 12·17, with r = 13 and t = 23, meets all but 15r > n, as its recorded optimal set holds 136, 104, 156 and 184,
and it rises by 129 over game 203, more than the 204 − 78 = 126 that the bound would allow.  That set plays 130 = 10r
and 195 = 15r, which both list 65 = 5r, so 10r has no factor of its own for step 3.  Nor are the hypotheses sharp: games
156 = 12·13, below 187, 228 = 12·19, with no prime t, and 348 = 12·29, where 15r ≤ n, rise by exactly n − 6r all the
same.

Game 372 = 12·31, with r = 29 and t = 43, is case B2.  In game 371, 348 = 12r pays with 12 and 174 = 6r is a move.  In
game 372, 344 = 8t pays with 8, as 172 = 4t is a move, so 248 = 8p pays with 124, 372 with 12, 232 = 8r with 116 and 348
with 174.  The bound, opt(371) + 372 − 174 = 43,698, is opt(372), and the recorded optimal set of game 371 with 174
exchanged for 372 is that of game 372.  The same exchange reaches the bound in every game of the committed chain that
meets the theorem's hypotheses with its recorded optimal set of game n, the first at 276.  Where the four numbers lie in
U\*(n), the theorem then proves game n from game n − 1 given U at n ([the standing](statements.md#below-the-ceiling),
[the computation](evidence.md#observations)).  The scores and sets of the examples are those of
`results/optimal.jsonl.gz`.
