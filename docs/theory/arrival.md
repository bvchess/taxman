# The arrival of n

Part of [the theory](../THEORY.md).

Game n has the numbers of game n − 1 and one more, n, and much of the frame model passes from one game to the next
unchanged.  In plain words, [the arrival lemma (J1)](#the-arrival-lemma) says: *if n joins only closed frames, and they
stay closed, then opt(n) can quickly be derived from opt(n − 1).*  The root problem does not change, so opt(n) = opt(n −
1) + ΣU\*(n) − ΣU\*(n − 1), with no search.  [Its corollary (J2)](#the-layer-2-corollary) says that a number with two
prime factors that joins only closed frames leaves them closed, with no conjecture.  [The semiprime formula
(J3)](#the-semiprime-formula) proves the score of an odd number with two prime factors from that of the game before, on
the whole board, with no statement U.  Three measured laws describe what is seen: [the shadow law](#the-shadow-law),
which move an exchange gives up, [the half law](#the-half-law), which even games keep the root problem of the game
before, and [the cofactor rule](#the-cofactor-rule), which games pose it.  The facts about neighboring games that rest
on the rules alone, the ceiling and the half lemma among them, are in [neighbors.md](neighbors.md).

*Words.*  [Frames, layers, moves, freedoms, closed frames, candidates, claims and the root problem
Π_n](frame-model.md#the-frame-model) are frame-model.md's, and [descending greedy and U\*](sources.md#descending-greedy)
sources.md's.  Ω(x) is the number of prime factors of x, counted with multiplicity, and so its layer.  The
[ceiling](../GLOSSARY.md#the-game) and q(n) are those of [neighbors.md](neighbors.md#neighboring-games).

## The arrival lemma

Only the frames that hold a maximal factor of n feel its arrival.  In plain words: *if n joins only closed frames, and
they stay closed, then opt(n) can quickly be derived from opt(n − 1).*  The frames that n *joins* are those of game n −
1 that hold a maximal factor of n, n/2 included for even n, which is still a source there.  Once n has arrived they are
the frame that holds n and, for even n, what is left of the frame of n/2.  The lemma states the condition in the frames
of game n alone.

The maximal factors of n have one prime factor fewer than n.  When n is odd they are factors of game n − 1 too, since
⌊n/2⌋ does not change.  When n is even, n/2 is a source of game n − 1, and in game n it is a factor that only n uses,
since its other multiples exceed n.  So n/2 always lies in the frame that holds n.  U\*(n) is U\* of game n, and v(s)
the [incidence vector](playability.md#independence) of a source s.

**Lemma J1 (the arrival lemma).**  Let n ≥ 3, let M be the frame of game n that holds n, and suppose that M is closed.

1. *Odd n.*  If M is closed without n too, that is, if descending greedy keeps as many of the other sources of M as M
   has factors, then game n has the root problem of game n − 1.  If [(♦)](sources.md#the-span-conjecture) holds in
   games n − 1 and n, then U\*(n) = U\*(n − 1) + n − x, where x is the smallest of the sources that M keeps without n
   whose vectors sum, over GF(2), to v(n).
2. *Even n.*  If the frames of game n that hold the maximal factors of n/2 are closed, then game n has the root problem
   of game n − 1, and U\*(n) = U\*(n − 1) + n.

*Proof.*  The idea: the root problem sees only the frames with freedoms, n changes only M and, for even n, the frame
that held n/2, and the hypotheses say that both were closed in game n − 1 and are closed in game n.

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
3. *Odd n: what changes.*  The two boards have the same numbers up to n/2, and game n has the one more source n.  Its
   maximal factors lie in frames D_1, …, D_r of game n − 1, all of layer Ω(n) ([the frame lemma
   (F1)](frame-model.md#the-frame-lemma)), and n joins them into M, with the factors F_{D_1} ∪ ⋯ ∪ F_{D_r}.  Every other
   frame keeps its sources and factors, so its moves too (step 2).  M less n is the D_i, with their sources and factors,
   so descending greedy, which splits over them by the argument of step 2, keeps on it the moves of the D_i in game n −
   1.  Their freedoms are never negative and add up to those of M without n, none; so every D_i is closed.
4. *Even n: what changes.*  Game n has the factor n/2 more and the source n/2 less, and the source n.  Let D′ be the
   frame of game n − 1 that holds n/2, and D_1, …, D_r those that hold the other maximal factors of n.  No edge of game
   n leaves D′ less n/2: n's other maximal factors lie in the D_i, one layer up, and n/2, now a factor, is joined to n
   alone.  So D′ less n/2 is a union of frames of game n, its *pieces*.  Every maximal factor of n/2 lies in one, and
   each holds at least one, since D′ was joined through n/2: the pieces are exactly the frames of game n that hold a
   maximal factor of n/2.  And n joins D_1, …, D_r and n/2 into M.  The other frames are unchanged.
5. *Even n: the moves.*  In D′, n/2 is the smallest source, so descending greedy decided every other source before
   reaching it, against sets that split by pieces: each piece keeps its moves of game n − 1, and the pieces' freedoms
   add up to those of D′, plus one if n/2 was a move.  They are closed, so D′ was closed and n/2 ∉ U\*(n − 1).  In M, a
   set T of old sources is playable with n against the factors of M iff T is playable against F_{D_1} ∪ ⋯ ∪ F_{D_r}.
   Indeed n can pay with n/2, which divides no member of T, so the joining lemma puts n after a scheme of T; and no
   member of T lists n/2, so restriction gives the converse.  So descending greedy keeps n, then decides the old sources
   as in game n − 1: U\*(n) ∩ M is n with the moves of the D_i, M has their freedoms together, and since M is closed, so
   is every D_i.  And U\*(n) = U\*(n − 1) + n.
6. *The root problem.*  In both cases the frames that change are closed before and after: the D_i, and D′ for even n,
   in game n − 1; M, and the pieces for even n, in game n.  The other frames are the same, with the same moves.  So the
   frames with freedoms are those of game n − 1, with the same moves, and step 1 gives Π_n = Π_{n−1}.
7. *Odd n: the move given up.*  Let B be the moves of the D_i, |F_M| of them by step 3, whose vectors are linearly
   independent on the coordinates F_M ([the independence lemma (P10)](playability.md#the-independence-lemma)) and so
   span every vector of a source of M.  Given (♦) in games n − 1 and n, U\* is in each game the greedy basis of the
   sources' vectors ([the greedy-basis proposition (W3)](parity-bound.md#the-greedy-basis-proposition)).  The frames
   have disjoint factors, so the sources' vectors lie on disjoint blocks of coordinates, a block for each frame, and a
   set of sources is independent exactly when its part in each frame is ([linear
   independence](primer/linear-algebra.md#linear-independence)).  So [the greedy algorithm](primer/greedy.md#the-greedy-algorithm) decides
   each source among the sources of its own frame, and the greedy basis is the union of the frames' greedy bases: B is
   the greedy basis of the old sources of M, and U\*(n) ∩ M that of all its sources.  These two facts are all that the
   step uses of (♦): by step 2 and the split by frames, they say that descending greedy agrees with the greedy basis on
   each D_i in game n − 1 and on M in game n.  B spans v(n), and its vectors are independent, so exactly one C ⊆ B sums
   to v(n); let x be its smallest member.  Then B′ = B + n − x is a basis, and it meets [the optimality
   criterion](primer/greedy.md#the-optimality-criterion): for a source y of M outside B′, the members of the circuit of B′ + y
   all exceed y.  For y = x that circuit is C + n, whose other members exceed x.  For y ≠ x, the members of the circuit
   H of B + y exceed y, since B is heaviest.  If x ∉ H, H is the circuit of B′ + y.  If x ∈ H, adding v(n) and the
   vectors of C to those of H − y gives v(y) as the sum over ((H − y) △ C) + n ⊆ B′, the circuit of B′ + y less y: the
   members of H − y exceed y, those of C exceed x, which exceeds y, and n exceeds them all.  The values are distinct and
   positive, so B′ is the one heaviest basis, the greedy basis.  So U\*(n) ∩ M = B′, and with step 3,
   U\*(n) = U\*(n − 1) + n − x.  ∎

Read in game n − 1, the hypotheses say that the frames that n joins are closed, and for even n that n/2 was not a move
(steps 3 to 5).  Conversely, these give back the hypotheses, for odd n given (♦) in game n, which makes M closed
(below).

The first two hypotheses earn their place.

* *M closed without n (odd n).*  In game 25 the frame of layer 2 that holds 25 = 5² is closed, but not without 25: 25
  fills the frame's last freedom instead of taking the place of a move.  The optimal game of 24 used that freedom to
  promote 9, and in game 25 the edge 3–g of 9 would close the cycle 3–g–5–3 with 25 and 15, so 9 cannot be promoted:
  opt(25) = opt(24) + 25 − 9.  To 4949, n closes its frame by filling a freedom in 15 odd games, and the best promotions
  change in 10 of them.
* *The frames of n/2's maximal factors closed (even n).*  In game 12 the frame that holds 12 is closed, but 6 = 12/2
  was a move of game 11, so its old frame loses a move: the frame of 10, with the factors 2 and 5, has a freedom in
  game 12, and Π_12 has a root frame while Π_11 has none.  At 128 the same loss of the move 64 lets the promotions
  grow: opt(128) = opt(127) + 128 − 64 + 32.

The third, M closed for odd n, follows from M closed without n given (♦) in game n alone: B spans every vector of a
source of M, so the greedy basis of M's sources has |F_M| members, and with (♦) in game n that greedy basis is U\*(n) ∩
M (step 7).  Without (♦) the proof needs the hypothesis, and no game is known where it fails.

The odd case uses (♦) only on M and the D_i: descending greedy must agree there with the greedy basis.  In layers 1 and
2 it always does.  In layer 1 every source has the vector e₁ and each game keeps one.  In layer 2 descending greedy is
the scan of the forests of the graph G_n of [the bottom-layers theorem](frame-model.md#the-bottom-layers-theorem), its
step 3: it keeps an edge unless the edge closes a cycle with the edges kept.  Over GF(2) each edge is the sum of the
unit vectors of its two ends, and a set of edges is independent exactly when it is a forest
([forests](primer/linear-algebra.md#forests)).  The incidence vectors are these vectors with the coordinate of g left out, which keeps
their dependencies: in a sum of edges, the coordinate of g is the sum of the others, since each edge has two ends.

### The layer-2 corollary

**Corollary J2 (the layer-2 corollary).**  Let n have two prime factors, counted with multiplicity.  If n joins only
closed frames, they stay closed: game n has the root problem of game n − 1, and U\*(n) = U\*(n − 1) + n − x for odd n,
with x as in J1, and U\*(n) = U\*(n − 1) + n for even n.  No conjecture is needed.

*Proof.*  For odd n = pp′, n joins the frames of layer 2 that hold p and p′, and with them closed, M is closed without n
(step 3).  M lies in layer 2, where descending greedy agrees with the greedy basis (the paragraph above), which is all
that step 7 and the remark on M closed use of (♦); so M is closed, and x is as in J1.  For even n = 2p, n joins the
frame of layer 2 that holds 2 and the frame of layer 1, which holds p as a source.  The frame of layer 1 has the one
factor 1 and keeps only the largest prime, which exceeds p by Bertrand's postulate, so p is no move; what is left of it
in game n holds p's one maximal factor 1 and is closed.  M has the freedoms of the frame that holds 2, none (step 5).  ∎

A number with two prime factors joins only closed frames on every board from 188 on, by [the square theorem
(F10)](frame-model.md#the-square-theorem), since from 187 on layer 2 is one closed frame.  Below 188 it does wherever
the frames it joins are closed, as at 49 = 7², whose frame of 7 in game 48 is closed though another frame of layer 2 is
not.

Game 39 = 3·13 lies in the one frame of layer 2 of game 39, which is closed, and closed without 39, since the frame of
layer 2 of game 38 is.  There v(39) is the sum of v(22), v(26) and v(33), the path 3–11–2–13 in G_38, and game 39 gives
up the smallest, x = 22.  Game 58 = 2·29 lies in the frame of layer 2 that holds 2 and 29, which is closed, and its half
29 has the maximal factor 1, in the closed frame of layer 1.  So game 58 has the root problem of game 57, and
U\*(58) = U\*(57) + 58.

The lemma generalizes the prime formula.  A prime n has the one maximal factor 1, and lies in the frame of layer 1,
always closed, with the one factor 1 and one move, which is q(n − 1) without n.  So game n has the root problem of game
n − 1, with no need of (♦), and x = q(n − 1): U\*(n) = U\*(n − 1) + n − q(n − 1).  With the repeat lemma this is [the
prime formula (N5)](neighbors.md#the-prime-formula), which neighbors.md proves from the rules alone, without U.

When the lemma applies, [the repeat lemma (R6)](search-rules.md#the-repeat-lemma) carries the score over.  Given
[statement U](exactness.md#statement-u) at n and at n − 1, opt(n) − opt(n − 1) = ΣU\*(n) − ΣU\*(n − 1), which is n − x
for odd n, given (♦) or by J2, and n for even n.  For even n, U at n − 1 alone suffices: the optimal game that a
heaviest promotion set makes plays U\*(n − 1) ∪ L, which leaves n/2 out, and [the half lemma
(N7)](neighbors.md#the-half-lemma) applies.  For odd n with two prime factors, [the semiprime formula
(J3)](#the-semiprime-formula) proves the difference n − x with no U at all.  Whether x is always the shadow of n is [the
shadow law](#the-shadow-law), measured.  Nothing that proves a score rests on the arrival lemma and J2: the solver finds
a repeated root problem by comparing the problems themselves.

## The semiprime formula

At an odd number with two prime factors the best score rises by an exact amount, as it does at a prime: n takes the
place of one source of layer 2, and the rest of every optimal set stays.  The formula holds on the whole board, with no
statement U and no conjecture, since [the bottom-layers theorem (F4)](frame-model.md#the-bottom-layers-theorem) fixes
layer 2 of every optimal set, and the exchange touches no other layer.  So once opt(n − 1) is proven, such a game needs
no search.

*Words.*  G_n is the graph of F4: its vertices are the primes up to n/2 and one more vertex g, and its edges the
numbers of layer 2 up to n, pq the edge p–q and p² the edge p–g.  U\*_2(n) is the part of U\*(n) in layer 2, a forest
in G_n, and S_k the part of a set S in layer k.

**Theorem J3 (the semiprime formula).**  Let n be odd with two prime factors, counted with multiplicity, and suppose
that n closes a cycle with U\*_2(n − 1) in G_n.  Let x be the smallest member of that cycle other than n.  Then:

1. U\*_2(n) = U\*_2(n − 1) − x + n;
2. opt(n) = opt(n − 1) + n − x;
3. S ↦ (S ∖ {n}) ∪ {x} maps the optimal sets of game n one to one onto those of game n − 1, and T ↦ (T ∖ {x}) ∪ {n}
   is its inverse.

The hypothesis holds for every odd n ≥ 189 with two prime factors.

*Proof.*  F4 makes U\*_2 the sources of layer 2 of every optimal set, part 1 says that U\*_2 changes by the exchange
of x for n, and no layer but layer 2 can tell the two boards apart.

1. *The two boards.*  Since n is odd, ⌊(n − 1)/2⌋ = ⌊n/2⌋: games n − 1 and n have the same numbers up to n/2, and
   G_{n−1} and G_n have the same vertices, G_n with the one more edge n.  The sources of layer 2 of game n are those of
   game n − 1 and n.  The number x is a source of game n − 1, so x > n/2, like n.  And q(n − 1) = q(n) > n/2, since n
   is composite.
2. *Part 1.*  Write B = U\*_2(n − 1), and call the cycle less n *the path of n*: the path of B that joins the ends
   of n, whose smallest member is x.  In layer 2, descending greedy keeps a source y exactly when y closes no cycle
   with the sources of layer 2 above y (F4, step 3), and in game n − 1 the members of B above y, B_{>y}, join the same
   pairs of vertices as all those sources do ([part (b) of the greedy
   algorithm](primer/greedy.md#the-greedy-algorithm)).  Above each y < n, game n has the sources of layer 2 of game
   n − 1, and n.  So game n keeps n, the largest; it rejects every source that game n − 1 rejects; and it rejects a
   member y of B exactly when y closes a cycle with B_{>y} + n.  The ends of y lie in two different trees of B_{>y},
   since B is a forest, so that happens exactly when the ends of n lie one in each: when the ends of n are joined in
   B_{>y} + y by a path through y.  Such a path lies in B, so it can only be the path of n.  So game n rejects y exactly
   when the path of n lies in B_{>y} + y and holds y: when y is its smallest member, x.
3. *The exchange.*  Let S be a set of game n with q(n) its only prime, n ∈ S and x ∉ S, and S′ = (S ∖ {n}) ∪ {x}, a
   set of game n − 1, such that S_2 and S′_2 are forests in G_n.  At every layer k ≠ 2, S and S′ have the same [layer
   instance (F3)](frame-model.md#the-layer-theorem): n and x lie in layer 2, so S_k = S′_k, and the pool, the numbers
   of layer k − 1 up to n/2 outside the set, is the same on both boards (step 1), where the two sets agree.  At layer 2
   both pools hold every prime up to n/2, since q(n) > n/2, and a set of layer 2 that is a forest in G_n is playable
   against them ([the two-factor proposition (P11)](playability.md#the-two-factor-proposition), as in step 2 of F4's
   proof).  So by F3, S is playable in game n exactly when S′ is playable in game n − 1.
4. *opt(n) ≤ opt(n − 1) + n − x.*  Let S be an optimal set of game n.  By F4, q(n) is its only prime, its sources of
   layer 2 are U\*_2(n), which holds n and not x by part 1, and S_2 is a forest in G_n.  In S_2 the ends of x are
   joined by the path of n less x, with n: a path of U\*_2(n), and so the only one in the forest S_2.  So S_2 less n
   leaves them apart, and S′_2 = S_2 − n + x is a forest too.  By step 3, S′ is playable in game n − 1, so
   opt(n − 1) ≥ ΣS′ = opt(n) − n + x.
5. *opt(n) ≥ opt(n − 1) + n − x.*  Let T be an optimal set of game n − 1.  By F4, q(n) is its only prime, its sources
   of layer 2 are B, which holds x, and T_2 is a forest in G_{n−1}.  In T_2 the ends of n are joined by the path of n,
   the only path between them, which passes through x.  So T_2 − x + n is a forest in G_n, and step 3, with S′ = T,
   makes (T ∖ {x}) ∪ {n} playable in game n.  So opt(n) ≥ ΣT − x + n.
6. *Parts 2 and 3.*  Steps 4 and 5 give part 2.  So the set S′ of step 4 is worth opt(n − 1), an optimal set of game
   n − 1, and the set that step 5 makes is worth opt(n), an optimal set of game n.  Every optimal set of game n holds n
   and not x, and every optimal set of game n − 1 holds x and not n, so the two maps undo each other.
7. *The hypothesis from 189 on.*  For odd n ≥ 189, the frames of layer 2 of game n − 1 are closed ([the square theorem
   (F10)](frame-model.md#the-square-theorem)), so B is a spanning tree of G_{n−1} (F4, step 7).  The ends of n are
   vertices of it (step 1), so n closes a cycle with B.  ∎

Game 39 = 3·13 is the example of J2 above.  U\*_2(38) is {21, 22, 25, 26, 33, 34, 35, 38}, and 39, the edge 3–13,
closes the cycle 3–11–2–13 with 33, 22 and 26, so x = 22.  So opt(39) = opt(38) + 39 − 22 = 486 + 17 = 503, and the
optimal set of game 38, {18, 20, 21, 22, 24, 25, 26, 27, 28, 30, 32, 33, 34, 35, 36, 37, 38}, with 22 exchanged for
39, is the optimal set of game 39; both are unique.  [The repeat lemma (R6)](search-rules.md#the-repeat-lemma) gives
the same bound only given U at 39.  The theorem reaches games that are no repeat too.  In game 50 the frame of layer 2
that holds 3 and 17 has a freedom, and game 51 = 3·17 poses another root problem than game 50, but 51 closes the cycle
3–13–2–17 with 39, 26 and 34, so x = 26 and opt(51) = opt(50) + 25 = 833; the optimal set of game 51 is that of game
50 with 26 exchanged for 51, and both are unique.

The hypothesis is needed: without a cycle there is no x, and the score can rise by n or by less.  Game 21 = 3·7 closes
no cycle with U\*_2(20) = {14, 15} and falls one short of its ceiling, opt(21) = 144 = opt(20) + 20, while game 33 =
3·11 closes none with U\*_2(32) = {21, 22, 25, 26} and reaches it.  Nor does 187 = 11·17, which joins the two frames of
layer 2 of game 186 ([the cofactor rule](#the-cofactor-rule)); from 189 on, every odd number with two prime factors
closes a cycle (step 7).  The theorem asks for odd n because at an even n = 2p the prime p is a vertex of G_n but not
of G_{n−1}, and n is its only edge, so n closes no cycle; such a game is at its ceiling by the half lemma, since an
optimal game of n − 1 plays no prime but q(n − 1) > p ([the prime lemma (N1)](neighbors.md#the-prime-lemma)).

The scores of the examples are those of `results/optimal.jsonl.gz`, which the prover proves on the whole board, and the
optimal sets of 38, 39, 50 and 51 are those that the exhaustive search of `heuristics/evaluation/certify.py` lists, in
under two minutes each.

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
study](https://github.com/bvchess/taxman-notebook/blob/main/the-wikis-open-questions-2026-10/README.md#7-which-n) had
found.

The law has a form for U\* that reads no recorded game.

*The shadow law for U\** (measured).  Let n ≥ 3 be odd.  If game n has the root problem of game n − 1, then
U\*(n) = U\*(n − 1) + n − x, with x the shadow of n.

Where [the arrival lemma (J1)](#the-arrival-lemma) applies, it makes x the smallest member of the circuit of v(n), and
this form says that member is the shadow.  At a prime both are q(n − 1), with no measurement.  The form holds in all
2,651 odd games to 8191 whose root problem is that of the game before: 1,027 primes, 1,603 composites whose joined
frames are closed, so that J1 applies, and 21 composites that J1 does not cover: each joins a frame of layer 3 with
freedoms, yet poses the root problem of game n − 1 and gives up the shadow.  These 21 start at 2197 = 13³, and each has
a prime factor 13, 17 or 19.

The law says which move an exchange gives up, not when the exchange happens.  It covers only the recorded optimal sets,
and a game may have others.  That a recorded game is optimal is proved on the whole board for the games the prover has
proved, and for the rest given [statement U](exactness.md#statement-u).  [evidence.md](evidence.md#observations) gives
the range and what one must trust, and [the notebook entry on the corner of a near
repeat](https://github.com/bvchess/taxman-notebook/blob/main/search/the-corner-of-a-near-repeat-2026-10/README.md#how-often-it-happens)
the count.  A proof would say why the shadow is the one move of game n − 1 that n displaces.

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
notebook entry on the arrival of
n](https://github.com/bvchess/taxman-notebook/blob/main/the-arrival-of-n-in-the-frame-model-2026-10/README.md) has the
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
shares, given U, and at the ceiling the step theorem gives its optimal sets.  Where the cofactor is an odd prime, [the
semiprime formula (J3)](#the-semiprime-formula) gives the score with no U.

The hypothesis is needed.  Every game from 28 on that breaks the rule has a freedom in layer 2, in game n or in game m:
25 games, the last 187, whose root problem is not that of 186.  A freedom does not force a break, since 19 games with
one keep the rule.

The other two cases are explained in part.  A number 2pP whose cofactor is twice a prime has the half pP, which the
closed layer 2 rejects, so the half lemma puts game n at its ceiling, given that the optimal game of n − 1 leaves pP
out, and the half law gives the repeat, though the frames of layer 3 it joins have freedoms.  A number 9P joins a frame
of layer 3 with freedoms, and its root problem changes; that it reaches its ceiling is measured.  On every board from
188 to 8191 the root problems built from the definitions agree: the 1,585 odd numbers with two prime factors and the 540
numbers 2P repeat, as J2 says, and so do the 1,092 numbers 2pP, which the arrival lemma does not cover ([the notebook
entry on the arrival of
n](https://github.com/bvchess/taxman-notebook/blob/main/the-arrival-of-n-in-the-frame-model-2026-10/README.md)).

The rule's last two cases are measured on the games whose root problem the chain records, and the boards up to 27 record
none.  Why the numbers 2pP repeat, and why the numbers 9P reach their ceiling, is open.
[evidence.md](evidence.md#observations) gives the range and what one must trust, and [the notebook entry on the cofactor
of
n](https://github.com/bvchess/taxman-notebook/blob/main/search/which-games-need-no-search-the-cofactor-of-n-2026-10/README.md)
the counts, with the cofactors that are always searched.
