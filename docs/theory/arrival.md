# The arrival of n

Part of [the theory](../THEORY.md).

Game n has the numbers of game n − 1 and one more, n, and much of the frame model passes from one game to the next
unchanged.  [The arrival lemma (J1)](#the-arrival-lemma) says when the root problem does not change at all: when the
frames that n joins are closed, given (♦) for odd n and provided U\* left n/2 out for even n.  Its corollary for the
numbers with two prime factors ([J2](#the-layer-2-corollary)) needs no (♦).  Three measured laws describe what is
seen: [the shadow law](#the-shadow-law), which move an exchange gives up, [the half law](#the-half-law), which even
games keep the root problem of the game before, and [the cofactor rule](#the-cofactor-rule), which games pose it.  The
facts about neighboring games that rest on the rules alone, the ceiling and the half lemma among them, are in
[neighbors.md](neighbors.md).

*Words.*  [Frames, layers, moves, freedoms, closed frames, candidates, claims and the root problem
Π_n](frame-model.md#the-frame-model) are frame-model.md's, and [descending greedy and U\*](sources.md#descending-greedy)
sources.md's.  Ω(x) is the number of prime factors of x, counted with multiplicity, and so its layer.  The
[ceiling](../GLOSSARY.md#the-game) and q(n) are those of [neighbors.md](neighbors.md#neighboring-games).

## The arrival lemma

Only the frames that hold a maximal factor of n feel its arrival.  When they are closed, game n poses the root problem
of game n − 1, given (♦) for odd n and provided U\*(n − 1) leaves n/2 out for even n.

The maximal factors of n have one prime factor fewer than n.  When n is odd they are factors of game n − 1 too, since
⌊n/2⌋ does not change, and they lie in frames of layer Ω(n) ([the frame lemma (F1)](frame-model.md#the-frame-lemma)).
When n is even, n/2 is a source of game n − 1 and lies in a frame of layer Ω(n) − 1.  The frames of game n − 1 that
hold a maximal factor of n, as a factor or, for n/2, as a source, are the frames that n *joins*.  U\*(n) is U\* of game
n, and v(s) the [incidence vector](playability.md#independence) of a source s.

**Lemma J1 (the arrival lemma).**  Let n ≥ 3, and suppose that every frame that n joins is closed in game n − 1.

1. *Odd n.*  If [(♦)](sources.md#the-span-conjecture) holds in game n, game n has the root problem of game n − 1, and
   the frame of game n that holds n is closed.  If (♦) holds in game n − 1 too, U\*(n) = U\*(n − 1) + n − x, where x
   is the smallest of the members of U\*(n − 1) whose vectors sum, over GF(2), to v(n).
2. *Even n.*  If n/2 is not in U\*(n − 1), game n has the root problem of game n − 1, and U\*(n) = U\*(n − 1) + n.

*Proof.*  The idea: the root problem sees only the frames with freedoms, and n merges only the frames it joins, which
stay closed.

1. *The root problem sees only the frames with freedoms.*  By its definition, Π_n is fixed by the frames with
   freedoms, with their layers, moves and factors.  The numbering reads their layers, and frame h their moves and
   factors.  The candidates of frame h are tested against frame h and frame h − 1.  The claims of frame h read frame h,
   frame h + 1 and the frames of the layer below that can take a number on, and a closed frame can take nothing on.  So
   it is enough to show that games n − 1 and n have the same frames with freedoms, with the same moves.
2. *U\* is decided frame by frame.*  A source lists only factors of its own frame, so a set of sources is playable iff
   its part in each frame is playable against that frame's factors, by restriction and [the joining lemma
   (F2)](frame-model.md#the-joining-lemma).  So descending greedy keeps a source of a frame D iff it is playable with
   the sources it kept before it in D: U\* ∩ D is descending greedy on the sources of D alone (sources.md, the paragraph
   "So does descending greedy").
3. *Odd n: the frames.*  The two boards have the same numbers up to n/2, and game n has the one more source n, whose
   maximal factors lie in the frames D_1, …, D_r that n joins.  So in the frame graph of game n, n merges D_1, …, D_r
   into one frame M, with the factors F_M = F_{D_1} ∪ ⋯ ∪ F_{D_r}, and every other frame keeps its sources and factors,
   so its moves too (step 2).
4. *Odd n: M is closed.*  The moves of each D_i are playable, so their vectors are independent ([the independence lemma
   (P10)](playability.md#the-independence-lemma)), and there are |F_{D_i}| of them, since D_i is closed.  The D_i have
   disjoint factors, so the union B of their moves is |F_M| independent vectors on the coordinates F_M, which span
   every vector of a source of M.  Given (♦) in game n, U\*(n) is the greedy basis of the sources' linear matroid ([the
   greedy-basis proposition (W3)](parity-bound.md#the-greedy-basis-proposition)), which is the direct sum of the frames'
   parts, since the frames have disjoint factors; so U\*(n) ∩ M is the greedy basis of the sources of M, and has |F_M|
   members.  So M is closed, the frames with freedoms are those of game n − 1, with the same moves, and step 1 gives
   Π_n = Π_{n−1}.
5. *Odd n: the move given up.*  Given (♦) in game n − 1 too, B is likewise the greedy basis of the old sources of M.
   The vectors of U\*(n − 1) are independent (P10), so at most one set of them sums to v(n); B spans v(n), so that set
   is a C ⊆ B.  Let x be the smallest member of C.  Then B′ = B + n − x is a basis, and it meets [the optimality
   criterion](primer.md#the-optimality-criterion): for a source y of M outside B′, the members of the circuit of
   B′ + y all exceed y.  For y = x that circuit is C + n, whose other members exceed x.  For y ≠ x, the members of the
   circuit H of B + y exceed y, since B is heaviest.  If x ∉ H, H is the circuit of B′ + y.  If x ∈ H, adding v(n) and
   the vectors of C to those of H − y gives v(y) as the sum over ((H − y) △ C) + n ⊆ B′, the circuit of B′ + y less y:
   the members of H − y exceed y, those of C exceed x, which exceeds y, and n exceeds them all.  The values are distinct
   and positive, so B′ is the one heaviest basis, the greedy basis.  So U\*(n) ∩ M = B′, and with step 3,
   U\*(n) = U\*(n − 1) + n − x.
6. *Even n: the frames.*  Game n has the factor n/2 more and the source n/2 less, and the source n.  The number n/2 is
   a maximal factor of n alone, since its other multiples exceed n.  Let D′ be the frame of game n − 1 that holds n/2,
   and D_1, …, D_r those that hold the other maximal factors of n.  No edge of game n leaves D′ less n/2: n's other
   maximal factors lie in the D_i, one layer up, and n/2, now a factor, is joined to n alone.  So D′ less n/2 is a union
   of frames of game n, its *pieces*.  Each factor of D′ keeps a source besides n/2 (F1 on the board of n), one of D′,
   since a factor of D′ lies in layer Ω(n) − 2 and so is no maximal factor of n.  And n merges D_1, …, D_r and n/2 into
   one frame M, with the factors F_{D_1} ∪ ⋯ ∪ F_{D_r} + n/2.  The other frames are unchanged.
7. *Even n: the moves.*  In D′, n/2 is the smallest source and not kept, so descending greedy decided every other
   source before reaching it, against sets that split by pieces: each piece keeps its moves of game n − 1, and the
   pieces' freedoms add up to those of D′, none.  In M, a set T of old sources is playable with n against F_M iff T is
   playable against F_{D_1} ∪ ⋯ ∪ F_{D_r}.  Indeed n can pay with n/2, which divides no member of T, so the joining
   lemma puts n after a scheme of T; and no member of T lists n/2, so restriction gives the converse.  So descending
   greedy keeps n, then decides the old sources as in game n − 1: U\*(n) ∩ M is n with the moves of the D_i, and M has
   their freedoms together, none.  Step 1 finishes the proof, and U\*(n) = U\*(n − 1) + n, since n/2 ∉ U\*(n − 1).  ∎

Each hypothesis earns its place.  Without n/2 ∉ U\*(n − 1) the even case fails at 12: every frame of game 11 is closed,
and 12 joins only closed frames, but 6 is a move of game 11; in game 12 the frame of 10, with the factors 2 and 5, has a
freedom, so Π_12 has a root frame and Π_11 has none.  The closed frame D′ is more than the proof needs: it serves only
to keep the pieces closed, and an open D′ that stays in one piece would do.

The odd case uses (♦) only on the frame that holds n and on the frames it joins: descending greedy must agree there
with the greedy basis.  In layers 1 and 2 it always does.  In layer 1 every source has the vector e₁ and each game keeps
one.  In layer 2 descending greedy is the scan of the graphic matroid of the graph G_n of [the bottom-layers
theorem](frame-model.md#the-bottom-layers-theorem), its step 3, and the incidence vectors are that matroid's vectors
over GF(2) ([the primer](primer.md#linear-and-graphic-matroids)) with the coordinate of g left out, which keeps their
dependencies: in a sum of edges, the coordinate of g is the sum of the others, since each edge has two ends.

### The layer-2 corollary

**Corollary J2 (the layer-2 corollary).**  Let n ≥ 5 have two prime factors, counted with multiplicity, and suppose that
layer 2 has no frame with a freedom in game n − 1.  Then game n has the root problem of game n − 1.  For odd n, U\*(n) =
U\*(n − 1) + n − x, with x as in J1; for even n, U\*(n) = U\*(n − 1) + n.  By [the square theorem
(F10)](frame-model.md#the-square-theorem), this holds for every such n ≥ 188.

*Proof.*  An odd n = pr joins only the frames of layer 2 that hold p and r, which are closed, and J1's odd case applies
with no (♦), by the paragraph above.  An even n = 2r joins the frame of layer 2 that holds 2, which is closed, and the
frame of layer 1, which holds r as a source; that frame is always closed, with the one factor 1, and it keeps only
q(n − 1), which exceeds r, by Bertrand's postulate.  So r ∉ U\*(n − 1), and J1's even case applies.  From n = 188 on,
game n − 1 is a board of 187 or more, where layer 2 is one closed frame.  ∎

Game 39 = 3·13 joins the one frame of layer 2 of game 38, which is closed.  There v(39) is the sum of v(22), v(26) and
v(33), the path 3–11–2–13 in G_38, and game 39 gives up the smallest, x = 22.  Game 58 = 2·29 joins the closed frame of
layer 1, which holds 29 as a source that descending greedy rejected, and the closed frame of layer 2 of game 57, which
holds 2.  So game 58 has the root problem of game 57, and U\*(58) = U\*(57) + 58.

The lemma generalizes the prime formula.  A prime n has the one maximal factor 1, which lies in the frame of layer 1,
always closed, with the one move q(n − 1).  So game n has the root problem of game n − 1, with no need of (♦), and
x = q(n − 1): U\*(n) = U\*(n − 1) + n − q(n − 1).  With the repeat lemma this is [the prime formula
(N5)](neighbors.md#the-prime-formula), which neighbors.md proves from the rules alone, without U.

When the lemma applies, [the repeat lemma (R6)](search-rules.md#the-repeat-lemma) carries the score over.  Given
[statement U](exactness.md#statement-u) at n and at n − 1, and for odd n the formula for U\*(n) (from (♦) in both games,
or from J2), opt(n) = opt(n − 1) + n − x for odd n and opt(n) = opt(n − 1) + n for even n.  For even n, U at n − 1 alone
suffices: the optimal game that a heaviest promotion set makes plays U\*(n − 1) ∪ L, which leaves n/2 out, and [the half
lemma (N7)](neighbors.md#the-half-lemma) applies.  Whether x is always the shadow of n is [the shadow
law](#the-shadow-law), measured.  Nothing that proves a score rests on these results: the solver finds a repeated root
problem by comparing the problems themselves.

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
exchanges 4 for 8, and game 12 exchanges 6 for 12.  At a prime n, [the step theorem (N6)](neighbors.md#the-step-theorem)
proves the law for every optimal set, with x = q(n − 1).  For an odd semiprime pq the law gives up p′q′, the product of
the primes below p and q, which [an earlier
study](../../solver/notebook/the-wikis-open-questions-2026-10/README.md#7-which-n) had found.

The law has a form for U\* that reads no recorded game.

*The shadow law for U\** (measured).  Let n ≥ 3 be odd.  If game n has the root problem of game n − 1, then
U\*(n) = U\*(n − 1) + n − x, with x the shadow of n.

Where [the arrival lemma (J1)](#the-arrival-lemma) applies, it makes x the smallest member of the circuit of v(n), and
this form says that member is the shadow.  At a prime both are q(n − 1), with no measurement.  The form holds in all
2,651 odd games to 8191 whose root problem is that of the game before: 1,027 primes, 1,603 composites under J1's
hypothesis, and 21 composites outside it, from 2197 = 13³ on, each with a prime factor 13, 17 or 19, which join a frame
of layer 3 with freedoms and still give up the shadow.

The law says which move an exchange gives up, not when the exchange happens.  It covers only the recorded optimal sets,
and a game may have others.  That a recorded game is optimal is proved on the whole board for the games the prover has
proved, and for the rest given [statement U](exactness.md#statement-u).  [evidence.md](evidence.md#observations) gives
the range and what one must trust, and [the notebook entry on the corner of a near
repeat](../../solver/notebook/search/the-corner-of-a-near-repeat-2026-10/README.md#how-often-it-happens) the count.  A
proof would say why the shadow is the one move of game n − 1 that n displaces.

## The half law

At an even n, whether game n poses the root problem of game n − 1 is told by n/2 alone.

*The half law* (measured).  Let n ≥ 4 be even, and n ≠ 14.  Game n has the root problem of game n − 1 iff n/2 is not
in U\*(n − 1).

On every even board to 8190, the 1,840 games whose half U\*(n − 1) leaves out pose the root problem of the game before,
and the 2,253 whose half it keeps do not.  The one exception, 14, leaves 7 out but joins a frame of layer 2 with a
freedom.  The arrival lemma proves the law's "if" where the frames that n joins are closed, in 555 of the 1,840 games;
why the other 1,285 repeat is open.  When n/2 is in U\*(n − 1), it leaves its frame as a move, and the root problem
changed in every game measured, among them the exchanges of n/2 for n that the shadow law finds at the powers of 2 and
three times them.  With [the half lemma (N7)](neighbors.md#the-half-lemma), which puts game n at its ceiling when an
optimal game of n − 1 leaves n/2 out, an even game whose half U\*(n − 1) leaves out needs no search of its own.  [The
notebook entry on the arrival of n](../../solver/notebook/the-arrival-of-n-in-the-frame-model-2026-10/README.md) has the
run.

## The cofactor rule

Many games need no search of their own, since they pose the root problem of the game before them or reach their
ceiling.  Wherever layer 2 has no freedom, which ones do is told by n divided by its largest prime factor.

The *cofactor* of n is n divided by its largest prime factor: 3002 = 2·19·79 has the cofactor 38, and a prime has the
cofactor 1.  It is the cofactor x/r of [the prime-exchange theorem](search-rules.md#the-prime-exchange-theorem), with
x = n and r its largest prime factor.  A cofactor that is a prime means that n has two prime factors, counted with
multiplicity, so that n lies in layer 2; one that is twice a prime means that n is even with three.

*The cofactor rule* (measured, the first case proved).  Let n ≥ 28, and let m be n − 1, or n − 2 when n − 1 is prime.
Suppose that no frame of layer 2 has a freedom in game n or in game m.  If the cofactor of n is a prime or twice a
prime, the [root problem](frame-model.md#the-frame-model) of game n is that of game m.  If the cofactor is 9, game n is
at its ceiling.

The first case is proved: [the layer-2 corollary (J2)](#the-layer-2-corollary) gives it, with the prime case of the
arrival lemma when n − 1 is prime, since a prime arrival changes no frame of layer 2.  The other two are measured.

By [the square theorem (F10)](frame-model.md#the-square-theorem), the hypothesis holds for every n ≥ 188.  On the board
of 186, layer 2 has two frames: one of 11 and 13, which holds the only prime squares between 93 and 186, 121 and 169,
and one of every other prime up to 89, which holds no square and so has a freedom.  The source 187 = 11·17 joins them,
and from 187 on layer 2 is one frame with a square, and has no freedom.

Game 3002, of cofactor 38 = 2·19, has the root problem of game 3000, since 3001 is prime, and game 1341 = 9·149 is at
its ceiling.  With the cofactor 1, n is prime and [the prime formula (N5)](neighbors.md#the-prime-formula) gives opt(n).
So a game whose layer 2 has no freedom, and whose cofactor is 1, a prime, twice a prime or 9, needs no search of its
own: [the repeat lemma (R6)](search-rules.md#the-repeat-lemma) carries over the score of the game whose root problem it
shares, given U, and at the ceiling the step theorem gives its optimal sets.

The hypothesis is needed.  Every game from 28 on that breaks the rule has a freedom in layer 2, in game n or in game m:
25 games, the last 187, whose root problem is not that of 186.  A freedom does not force a break, since 19 games with
one keep the rule.

The other two cases are explained in part.  A number 2pP whose cofactor is twice a prime has the half pP, which the
closed layer 2 rejects, so the half lemma puts game n at its ceiling, given that the optimal game of n − 1 leaves pP
out, and the half law gives the repeat, though the frames of layer 3 it joins have freedoms.  A number 9P joins a frame
of layer 3 with freedoms, and its root problem changes; that it reaches its ceiling is measured.  On every board from
188 to 8191 the root problems built from the definitions agree: the 1,585 odd numbers with two prime factors and the 540
numbers 2P repeat, as J2 says, and so do the 1,092 numbers 2pP, outside the arrival lemma's hypothesis ([the notebook
entry on the arrival of n](../../solver/notebook/the-arrival-of-n-in-the-frame-model-2026-10/README.md)).

The rule's last two cases are measured on the games whose root problem the chain records, and the boards up to 27 record
none.  Why the numbers 2pP repeat, and why the numbers 9P reach their ceiling, is open.
[evidence.md](evidence.md#observations) gives the range and what one must trust, and [the notebook entry on the cofactor
of n](../../solver/notebook/search/which-games-need-no-search-the-cofactor-of-n-2026-10/README.md) the counts, with the
cofactors that are always searched.
