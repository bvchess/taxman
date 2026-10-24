# Bounds on the promotions

Part of [the theory](../THEORY.md).

The frame model's promotions obey linear algebra frame by frame, as the moves of the whole game do in [the parity bound
(W1)](parity-bound.md#the-parity-bound).  In a [frame](frame-model.md#frames), give the factors coordinates, each move
and each number promoted into the frame a vector of its maximal factors there, with any nonzero entries, and each number
promoted out of it its unit vector.  A promotion set makes these vectors linearly independent in every frame, over every
field: [the signed frame theorem (B1)](#the-signed-frame-theorem).  With every entry 1 over GF(2), the even frames and
the odd frames make two matroids on the candidates, and their heaviest common independent set bounds the best promotion
set, at the root and at every node of a search: [the frame bound (B2)](#the-frame-bound).  The same holds for the
extended problem ([B3](#the-frame-bound-for-the-extended-problem)), whose searches make the certificates of [statement
U](exactness.md#statement-u).  These three are load-bearing: proofs of scores rest on them.  Two more results serve
studies of stronger bounds, and no proof of a score rests on them: signed matroids join the frame bound through Lagrange
multipliers in [the decomposition bound (B4)](#the-decomposition-bound), and over GF(3) the signings together lose
nothing that winnability keeps ([the GF(3) proposition, B5](#the-gf3-proposition), explanatory).  All five are proved.

*Words.*  Frames, their numbering, [candidates, homes, winnable frames and promotion
sets](frame-model.md#the-frame-model), and [nodes, completions, state(P) and the promotion
limit](frame-model.md#searching-the-frame-model) are defined in frame-model.md.  Fields are explained in [the
primer](primer.md#finite-fields-and-gf2), and linear matroids, parallel elements, loops, coloops, contraction, deletion
and direct sums in [its section on matroids](primer.md#matroids).

## Signed frame matroids

Fix a *state* of the frame model: the root of a root problem such as Π_n, or the state(P) that a node's promotions P
leave.  Its frames are numbered h = 0, 1, 2, …; write Q_h for the factors of frame h in the state.  Its *candidates*
are, at the root, the root problem's candidates, and in state(P), the candidates outside P that are still factors of
their homes there.  A candidate of home h ≥ 1 is one of the factors Q_h.  The *movers* of frame h are its moves and the
candidates of home h + 1, which become its moves when promoted.  The candidates that *touch* frame h, T_h, are those of
home h, which leave Q_h when promoted, and those of home h + 1.

**Definition (signing).**  An *incidence* of frame h is a mover z of frame h together with a maximal factor f of z in
Q_h.  A *signing* σ of frame h over a field 𝔽 gives each incidence a value σ(z, f) of 𝔽 other than 0.  A mover z then
has the vector v_σ(z) = Σ_f σ(z, f) e_f of 𝔽^{Q_h}, and each factor y in Q_h has its unit vector e_y.  The *plain*
signing gives every incidence the value 1; over GF(2) it is the only signing.

**Definition (signed frame matroid).**  The *signed frame matroid* N_h^σ is the linear matroid on T_h that represents
each candidate in 𝔽^{Q_h} modulo the span of the vectors v_σ of frame h's moves: a candidate x of home h + 1 by the
class of v_σ(x), and a candidate y of home h by the class of e_y.  Equivalently, it is the linear matroid of the moves'
and the candidates' vectors, with the moves contracted ([the primer](primer.md#deletion-contraction-direct-sums-and-truncation)).
When the moves' vectors are independent, the quotient has dimension |Q_h| less the number of moves, the frame's
freedoms, so no frame's matroid has rank above its freedoms.

**Definition (the frame matroids).**  The *frame matroids* A and B of the state are its signed frame matroids under the
plain signing over GF(2), joined by the parity of the frame: A is the direct sum of the N_h with h even, and B the
direct sum of those with h odd.  A candidate of home h touches frames h and h − 1 and no other, one of them even and one
odd.  So the sets T_h of the even frames are disjoint and cover the candidates, as do those of the odd frames, and A and
B are two matroids on the same set, the candidates.

*Example.*  On the board of 21, setup leaves two frames with freedoms: frame 0, with the moves 14 and 21 and the factors
2, 3 and 7, and frame 1, with the moves 12 and 18 and the factors 4, 6 and 9 (15 has taken 5, and 20 has taken 10).
The board's candidates are 4 and 9, both of home 1.  In frame 0 they are movers, with the vectors e₂ and e₃.  The moves
span e₂ + e₇ and e₃ + e₇, so both classes equal that of e₇, and 4 and 9 are parallel in N₀, which is A.  In frame 1
they are factors, with the vectors e₄ and e₉, and the moves span e₄ + e₆ and e₆ + e₉, so again both classes are e₆'s,
and 4 and 9 are parallel in N₁, which is B.  Each frame has one freedom, and each matroid has rank 1.  The sets
independent in both are ∅, {4} and {9}, and the heaviest, {9}, is worth P\* = 9: on this board the frame bound is
exact.  The matroids do no work here, though: the promotion limit of 1 alone allows only one of the two candidates, and
so bounds P\* by 9 too.

![Game 21's root frames 1 and 0, with the candidates 4 and 9 as factors of frame 1 and, by dashed arrows, as movers of frame 0](img/frame-matroids-21.svg)

*Each candidate touches one odd and one even frame: a factor of frame 1 (B = N₁) and a mover of frame 0 (A = N₀).
Dashed arrow: a candidate to the maximal factor that would pay for it in frame 0.*

## The signed frame theorem

**Theorem B1 (the signed frame theorem).**  Let L be a promotion set of a root problem Π of the frame model, h a frame
of Π, and σ a signing of frame h over a field 𝔽.  Then the vectors v_σ of frame h's moves and of L's members of home
h + 1, and the unit vectors e_y of L's members y of home h, are linearly independent; that is, L ∩ T_h is independent in
N_h^σ.  Moreover:

1. *Loops.*  A candidate that is a loop of N_h^σ, its class 0, lies in no promotion set.
2. *Nodes.*  At a node with the promotions P, every completion P ∪ X has X ∩ T_h independent in the contraction
   N_h^σ / P.  On the candidates of state(P), this contraction is the signed frame matroid of frame h of state(P), with
   the values of σ on the incidences that remain.  A candidate of home h that state(P) has taken out of its home is a
   loop of it.

Since each frame's condition concerns that frame's vectors alone, the signings of different frames, and their fields,
can be chosen independently: every promotion set is independent in all of these matroids at once.

*Proof.*  The idea: a winnable frame peels completely, and the order of its peel makes its vectors triangular, whatever
their nonzero entries.

1. *Frame h peels completely.*  L is a promotion set, so frame h is winnable with L applied: the instance whose members
   are the moves of frame h and L's members of home h + 1, and whose pool is Q_h less L's members of home h, each member
   listing its maximal factors in the pool, is playable.  By [the peel theorem (P4)](playability.md#the-peel-theorem),
   peel pairs all of its members.
2. *The vectors are independent.*  By [the peel-order lemma (P9)](playability.md#the-peel-order-lemma), part 2, the
   members' vectors v_σ, for any nonzero values, are then linearly independent together with the unit vectors of the
   coordinates outside the pool, which are the e_y of L's members of home h.
3. *Independence in N_h^σ.*  A family made of the moves' vectors and some others is independent exactly when the moves'
   vectors are independent and the classes of the others, modulo the moves' span, are independent ([contraction is a
   quotient](primer.md#deletion-contraction-direct-sums-and-truncation)).  The family of step 2 is such a family, so the classes
   of L ∩ T_h are independent: L ∩ T_h is independent in N_h^σ.
4. *Loops.*  An independent set holds no element whose class is 0, so by step 3 no promotion set holds a loop.
5. *Nodes.*  A completion P ∪ X is a promotion set, so by step 3, (P ∪ X) ∩ T_h is independent in N_h^σ; so is its
   subset P ∩ T_h, and X ∩ T_h is independent in the contraction N_h^σ / P, by the definition of contraction.
6. *The contraction is the node's matroid.*  Contracting P's members of home h + 1 adds their vectors to the moves'
   span: they join the moves, as in state(P), where they are moves of frame h.  Contracting P's members y of home h
   quotients by their e_y, which deletes the coordinates y: they leave the factors, as in state(P).  The frames of
   state(P) are then reduced, as [its definition](frame-model.md#searching-the-frame-model) has it: a move a left with a
   single factor f among the factors that remain is dropped with f.  Modulo the coordinates already deleted, v_σ(a) is
   σ(a, f) e_f, so the quotient by it deletes the coordinate f, and every other number's class is its vector on the
   factors left, as in the reduced frame.  If f is a candidate of home h, its class e_f is now 0: it is a loop.  ∎

*Example.*  At the node of game 21 that promotes 9, root frame 1 loses the factor 9, so 18 is left with 6 alone and
drops with it, and then 12 drops with 4: state({9}) has taken 4 out of its home.  Contracting 9 makes 4 a loop of
N₁/{9}, so of B/{9}; it is a loop of A/{9} too, since frame 0, with 9 as a move, has no freedom left.

## The frame bound

**Corollary B2 (the frame bound).**  Let Π be a root problem of the frame model and A and B its frame matroids.  Every
promotion set of Π is independent in A and in B, and has no more members than the promotion limit, so

    P*(Π) ≤ max { ΣI : I independent in A and in B, with no more members than the promotion limit }.

At a node with the promotions P and the candidates C, every completion P ∪ X has X independent in A/P and in B/P, with
no more members than the node's promotion limit, so

    ΣX ≤ max { ΣI : I ⊆ C independent in A/P and in B/P, with no more members than the node's promotion limit }.

The right side plus ΣP is the node's *frame bound*; at the root, where P is empty, it is the first right side.  By
[B1](#the-signed-frame-theorem) (2), A/P and B/P are the frame matroids of state(P) on its candidates, and every
candidate outside P that state(P) has taken out of its home is a loop of one of them.

*Proof.*  B1 with the plain signing over GF(2) makes a promotion set's part L ∩ T_h independent in N_h, for every frame
h.  A set is independent in a direct sum exactly when its part in each block is, and L's part in the block N_h is
L ∩ T_h, so L is independent in A and in B.  A promotion set has no more members than the promotion limit, and a
completion P ∪ X has |X| at most the node's promotion limit ([the promotion-limit lemma
(F9)](frame-model.md#the-promotion-limit-lemma), part 2).  At a node, P ∪ X is a promotion set, so it is independent in
A and B, and X is then independent in A/P and B/P, by the definition of contraction.  ∎

In game 21, the node that promotes 9 has the frame bound 9: its candidate 4 is a loop of both matroids, and its
promotion limit is 0.

*What it misses.*  Where every member of a frame's instance, with a set applied, lists at most two factors of the pool,
winnability and independence over GF(2) are the same condition, that the lists form a forest ([the two-factor
proposition (P11)](playability.md#the-two-factor-proposition)), so on such frames the frame bound loses nothing.  It
misses the unwinnable frames whose vectors are independent over GF(2), each of which holds an [odd
core](playability.md#odd-cores), with a mover of three or more factors.

*Against the matching bound.*  The frame bound is never above [the matching bound](../GLOSSARY.md#bounds)
([solver/docs/design/search.md](../../solver/docs/design/search.md#the-matching-bound)), which asks only that each move
and each promoted number get a distinct maximal factor in its frame, none of them a promoted number.  If a frame's
family of vectors is independent, the matrix with a row for each vector has a nonsingular square submatrix on as many
columns as rows, and a nonzero term of its determinant gives each row a distinct column where its entry is nonzero.  A
unit vector e_y can only take y, so the movers take distinct factors of the frame that are not promoted: the matching
bound's assignment.  So every set the frame bound admits, the matching bound admits.

*Against the parity bound.*  The parity bound is the same construction on the whole board, and at every root where both
were measured it equals ΣU\* plus the frame bound ([evidence.md](evidence.md#observations)).

The frame solver's node bound is the frame bound, under the solver's name for it, the GF(2) bound; it keeps each node's
frame matroids as contractions of the root's ([solver/docs/design/gf2-bound.md, "The GF(2)
bound"](../../solver/docs/design/gf2-bound.md#the-gf2-bound)).

## The frame bound for the extended problem

The [extended problem](extended-problem.md#the-extended-problem) Π⁺(O) leaves the sources O ⊆ U\*_{≥3} optional.  Its
promotion sets (K, L) hold *one-sided* candidates K ⊆ O ∪ J, sources that are moves of their own frame if promoted and
touch no other frame, besides the two-sided candidates L, which are the frame model's kind.  The two kinds are disjoint
(the one-sided are sources, the two-sided at most n/2), so below a promotion set is written as the set K ∪ L.  The
*freedoms* of a pair of Taxman Mini are its factors less its moves ([searching the frame
model](frame-model.md#searching-the-frame-model)).

**Corollary B3 (the frame bound for the extended problem).**  Let O ⊆ U\*_{≥3}.  Form the frame matroids A and B of
Π⁺(O) on its two-sided candidates as for a root problem of the frame model, and add each one-sided candidate s to both:
in the matroid of the parity of its own frame h, to the block of frame h, represented by the class of its vector v(s)
on frame h's factors; in the other matroid, as a coloop.  Then:

1. every promotion set K ∪ L of Π⁺(O) is independent in A and in B;
2. for every subset P of a promotion set K ∪ L, such as the promotions of a node, (K ∪ L) ∖ P is independent in A/P
   and in B/P;
3. if P is itself a promotion set, the two-sided members of (K ∪ L) ∖ P number at most the promotion limit of the
   freedoms of the root frames of Π⁺(O) with P applied, and each one-sided member of it has an own frame with a freedom
   there.

So at a node with the promotions P and the candidates C, every completion P ∪ X has ΣX at most the heaviest ΣI over the
sets I ⊆ C independent in A/P and B/P, with no more members than that promotion limit plus the number of one-sided
candidates in C.

*Proof.*

1. *Each frame.*  It is the proof of [B1](#the-signed-frame-theorem) with the plain signing over GF(2) and one more kind
   of mover.  Frame h of Π⁺(O) is winnable with K ∪ L applied: its instance has as members the fixed sources of the
   frame, which are its moves, the members of K whose own frame it is, and the members of L of home h + 1, and as pool
   its factors less the members of L of home h.  By [P4](playability.md#the-peel-theorem) and
   [P9](playability.md#the-peel-order-lemma) their vectors, with the unit vectors of L's members of home h, are
   independent, so the part of K ∪ L in the block of frame h is independent modulo the moves.
2. *The direct sums.*  A member of K touches no frame but its own, so in the other matroid it is a coloop, which joins
   any independent set.  So K ∪ L is independent in both direct sums: part 1.  Part 2 follows by the definition of
   contraction, as in B1, step 5.
3. *The count.*  Let r_h be the freedoms of root frame h with P applied, and let (K ∪ L) ∖ P have x_h two-sided members
   of home h and g_h one-sided members whose own frame is h.  Applying them to that frame adds g_h + x_{h+1} moves and
   removes x_h factors, and it stays winnable, so it keeps at least as many factors as moves: x_h + x_{h+1} + g_h ≤ r_h.
   No candidate has the home 0, so x_0 = 0, and with no frame beyond the last the sequence (x_h) is one of those that
   the promotion limit of the freedoms r_h ranges over, as in the proof of
   [F9](frame-model.md#the-promotion-limit-lemma), part 2.  So Σx_h is at most that limit, and g_h ≥ 1 only if r_h ≥ 1.
   The bound at a node follows from parts 2 and 3, since a completion's one-sided members are among the one-sided
   candidates of C.  ∎

[The certificate theorem (E2)](extended-problem.md#the-certificate-theorem) searches Π⁺(O), and its search cuts by this
bound ([solver/docs/design/extended-mode.md](../../solver/docs/design/extended-mode.md)).

## The decomposition bound

Several signed frame matroids combine with A and B into one bound by splitting the weights among them, as weak duality
allows ([the primer](primer.md#weak-duality-and-lagrangian-relaxation)).  Nothing that proves a score rests on this
bound; studies of stronger bounds use it.

**Theorem B4 (the decomposition bound).**  Let Π be a root problem of the frame model, A and B its frame matroids, and
N_{h_1}^{σ_1}, …, N_{h_t}^{σ_t} signed frame matroids of Π, each of one frame h_j and one signing σ_j over some field
(several may share a frame).  For a weight function ω on the candidates, let MI(ω) be the largest ω(I) of a set I
independent in A and in B with no more members than the promotion limit (I = ∅ included); for one on T_{h_j}, write
max_j for the largest weight of a set independent in N_{h_j}^{σ_j}.  Then for all real vectors λ_1, …, λ_t, with λ_j on
T_{h_j} and 0 off it, every promotion set L of Π satisfies

    ΣL ≤ MI(w − Σ_j λ_j) + Σ_j max_j λ_j = dec(λ),

where w(x) = x.  The same holds with the loops of every N_{h_j}^{σ_j} left out of the sets I.  At a node with the
promotions P, it holds for ΣX of every completion P ∪ X, with the contractions by P of A, B and the N_{h_j}^{σ_j}, and
the node's promotion limit.

*Proof.*  Split the weight: ΣL = (w − Σ_j λ_j)(L) + Σ_j λ_j(L ∩ T_{h_j}).  L is independent in A and B with no more
members than the promotion limit ([B2](#the-frame-bound)), so the first term is at most MI(w − Σ_j λ_j).  L ∩ T_{h_j} is
independent in N_{h_j}^{σ_j} ([B1](#the-signed-frame-theorem)), so the j-th term of the sum is at most max_j λ_j.  L
holds no loop of any N_{h_j}^{σ_j} (B1 (1)), so leaving the loops out of the sets I keeps L among them.  At a node, the
same argument runs with X in place of L, by B1 (2) and B2.  ∎

Each term is a standard computation: MI is a weighted matroid intersection, and max_j the greedy algorithm of
N_{h_j}^{σ_j} ([the primer](primer.md#the-greedy-algorithm-of-a-matroid)).  At λ = 0 the sum is 0 and dec(0) is the
frame bound, so the best multipliers do at least as well.  The least value of dec over the multipliers is that of a
linear program over the matroids' polytopes (the primer, the
paragraph "The best bound"), whose parts have the polytopes that the
primer, the paragraph "For matroids", describes.  That says what the
bound can reach; it holds at every λ, so nothing rests on how the multipliers are found.
[solver/docs/design/gf2-bound.md, "The decomposition
bound"](../../solver/docs/design/gf2-bound.md#the-decomposition-bound) says how a study's code evaluates it exactly.

## The GF(3) proposition

Over GF(3), the signings of a frame together see every way the frame can fail: a frame that is not winnable has a
signing that shows it.  This is explanatory, and nothing rests on it; it says why signed matroids over GF(3) can close
what the frame bound misses.

**Proposition B5 (the GF(3) proposition).**  Let L be a set of candidates of a state of the frame model, and h a frame
of the state.  Frame h is winnable with L applied if and only if L ∩ T_h is independent in N_h^σ for every signing σ of
frame h over GF(3).

*Proof.*  The idea: the members that a peel cannot pair list each of their remaining factors at least twice, so values
that cancel at every such factor make the vectors dependent.

1. *Only if.*  This is [B1](#the-signed-frame-theorem)'s argument, steps 1 to 3, which uses only that frame h is
   winnable with L applied.
2. *The stall.*  Suppose frame h is not winnable with L applied.  Run peel on its instance (the moves and L's members of
   home h + 1, over the pool Q_h less L's members of home h) until it stalls.  By [P4](playability.md#the-peel-theorem)
   some members are left unpaired: the *stalled* members, with the factors still live.  Each live factor that a stalled
   member lists is listed by at least two of them, or peel would make a back move.
3. *The signing.*  For each live factor listed by stalled members, give their incidences with it values in GF(3) that
   sum to 0: 1 and 2 for two members, 1, 1 and 1 for three, and for more, values 1 and 2 by twos with at most one triple
   of 1s.  Give every other incidence the value 1.
4. *The block structure.*  Order the members as [P9](playability.md#the-peel-order-lemma) does, with the stalled ones
   between its two kinds: peel's back moves in the order made, then the stalled members, then peel's front moves in the
   reverse of the order made; and order the pool's factors the same way, the back moves' factors, the live factors,
   the front moves' factors.  The argument of P9, part 1, holds with the stalled members counted as members paired
   after every move: a back move's factor is listed by no member that was live at its turn but its own, and the stalled
   members were live throughout; a front move's member lists no factor that was live at its turn but its own, and the
   live factors were live throughout.  So the vectors, restricted to the pool, form a block upper triangular matrix:

       rows \ columns    back moves'     live            front moves'
       back moves        triangular      anything        anything
       stalled           0               columns sum 0   anything
       front moves       0               0               triangular

   with nonzero diagonals in the two triangular blocks, and the stalled members' columns at the live factors summing to
   0 by step 3.
5. *A dependency.*  Give the stalled rows the coefficient 1 and the back rows 0.  The combination vanishes on the back
   moves' factors and on the live ones, and the triangular front block gives the front rows the coefficients that make
   it vanish on the front moves' factors too.  So it vanishes on the pool, and with the unit vectors e_y of L's members
   of home h it gives a vanishing combination of the whole family, with the coefficient 1 on every stalled member.
6. *L ∩ T_h is dependent.*  Modulo the moves' span this is a relation among the classes of L ∩ T_h.  If all of its
   coefficients on members of L were 0, it would be a vanishing combination of the moves' vectors alone with the
   coefficient 1 on the stalled members, which are then moves.  But every frame of a state is winnable with nothing
   applied, since the promotions P of a state form a promotion set (at the root P is empty, a promotion set by [the
   exactness theorem (X7)](exactness.md#the-exactness-theorem), as U\* is playable).  So the moves alone are
   independent under σ, by B1's argument, some coefficient on L is not 0, and L ∩ T_h is dependent in N_h^σ.  ∎

The same holds over every field with three or more elements, by D. Hershkowitz and H. Schneider's characterization of
the zero patterns all of whose matrices have the same rank ("Ranks of zero patterns and sign patterns", *Linear and
Multilinear Algebra* 34 (1993) 3–19): over such a field, every matrix with a given pattern of nonzero entries has full
row rank exactly when some matching of all the rows to columns is the only perfect matching between those rows and the
columns it uses, and such a matching orders them triangularly, which lets peel pair every member.  Over GF(2) the
proposition fails: the plain signing is the only one there, and an odd core is independent under it.

*Examples.*  The odd core {12, 18, 20, 30, 45} of the board of 45 lists the factors 4, 6, 9, 10 and 15 (12: 4, 6; 18:
6, 9; 20: 4, 10; 30: 6, 10, 15; 45: 9, 15), each by two of its members or, for 6, three, so peel has no move on it.
Under the plain signing its five vectors have the determinant 3: odd, so independent over GF(2), but divisible by 3, so
the plain signing over GF(3) already shows the core.  With the values of step 3, 1 and 2 at 4, 9, 10 and 15 and 1, 1, 1
at 6, the five vectors sum to 0 over GF(3).

![The odd core {12, 18, 20, 30, 45} as members joined to the factors 4, 6, 9, 10 and 15, with an entry of 1 or 2 on each edge so that the entries at every factor add up to 0 modulo 3](img/gf3-45.svg)

*The core with one choice of the values of step 3: the entries at every factor sum to 0 (mod 3).*

The signing matters for the odd core {20, 27, 28, 30, 42, 50, 63, 75} of the board of 75, which lists 4 (20 and 28), 6
(30 and 42), 9 (27 and 63), 10 (20, 30 and 50), 14 (28 and 42), 15 (30 and 75), 21 (42 and 63) and 25 (50 and 75), and
no part of which is a core.  Under the plain signing its eight vectors have the determinant −1, so they are independent
over GF(2), over GF(3) and over the rationals alike.  With the values of step 3, 1, 1, 1 at 10 and 1 and 2 at each other
factor, the eight vectors sum to 0 over GF(3).
