# Bounds on the promotions

Part of [the theory](../THEORY.md).

The frame model's promotions obey linear algebra frame by frame, as the moves of the whole game do ([the parity
bound](parity-bound.md)).  In a [frame](frame-model.md#frames), give the factors coordinates, each move and each number
promoted into the frame a vector of its maximal factors there, with any nonzero entries, and each number promoted out of
it its unit vector.  A promotion set makes these vectors linearly independent in every frame, over every field (the
signed frame theorem, B1).  With every entry 1 over GF(2), the even frames and the odd frames make two matroids on the
candidates, and their heaviest common independent set bounds the best promotion set, at the root and at every node of a
search (the frame bound, B2).  The same holds for the extended problem (B3).  The frame solver bounds its nodes by B2,
and the certificates of statement U rest on B3.  Two further results serve studies: over GF(3), the signings together
lose nothing that winnability keeps (the GF(3) proposition, B4, explanatory), and signed matroids join the frame bound
through Lagrange multipliers in the decomposition bound (B5).  All five are proved.

The frame model's terms, from frames and their numbering to [candidates, homes, winnable frames and promotion
sets](frame-model.md#the-frame-model) and [nodes, completions, state(P) and the promotion
limit](frame-model.md#searching-the-frame-model), are defined in frame-model.md.  Fields, linear matroids, contraction,
deletion, direct sums and loops are explained in [the primer](primer.md#matroids).

## Signed frame matroids

Fix a state of the frame model: a root problem such as Π_n, or the state(P) that a node's promotions P leave.  Its
frames are numbered h = 0, 1, 2, …, and each of its candidates is a factor of its home, a frame h ≥ 1.  Write F_h for
the factors of frame h, and T_h for the candidates that *touch* frame h: those of home h, which leave its factors when
promoted, and those of home h + 1, which become its moves.

**Definition (signing).**  A *signing* σ of frame h over a field 𝔽 gives a value σ(u, f) in 𝔽, other than 0, to each
pair of a number u that is a move of frame h or a candidate of home h + 1 and a maximal factor f of u in F_h.  Such a u
has the vector v_σ(u) = Σ_f σ(u, f) e_f in 𝔽^{F_h}, and each factor y of frame h has its unit vector e_y.  The *plain*
signing gives every pair the value 1; over GF(2) it is the only signing.

**Definition (signed frame matroid).**  Let W be the span of the vectors v_σ(m) of frame h's moves.  The *signed frame
matroid* N_h^σ is the linear matroid on T_h that represents each candidate in the quotient space 𝔽^{F_h} / W: a
candidate x of home h + 1 by the class of v_σ(x), and a candidate y of home h by the class of e_y.  Equivalently, it is
the linear matroid of the moves' and the candidates' vectors, with the moves contracted.  When the moves' vectors are
independent, the quotient has dimension |F_h| minus the number of moves, the frame's freedoms, so no frame's matroid has
rank above its freedoms.

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
exact.

![Game 21's root frames 1 and 0, with the candidates 4 and 9 as factors of frame 1 and, by dashed arrows, as movers of frame 0](img/frame-matroids-21.svg)

*Each candidate touches one odd and one even frame: a factor of frame 1 (B = N₁) and a mover of frame 0 (A = N₀).
Dashed arrow: a candidate to the maximal factor that would pay for it in frame 0.*

## The signed frame theorem

**Theorem B1 (the signed frame theorem).**  Let L be a promotion set of a root problem Π of the frame model, h a frame
of Π, and σ a signing of frame h over a field 𝔽.  Then the vectors v_σ(m) of frame h's moves, v_σ(x) of L's members of
home h + 1, and e_y of L's members of home h are linearly independent; that is, L ∩ T_h is independent in N_h^σ.
Moreover:

1. *Loops.*  A candidate that is a loop of N_h^σ, its class 0, lies in no promotion set.
2. *Nodes.*  At a node with the promotions P, every completion P ∪ X has X ∩ T_h independent in the contraction
   N_h^σ / P.  On the candidates that are still factors of their homes in state(P), this contraction is the signed
   frame matroid of frame h of state(P), with the values of σ on the pairs that remain.

Since each frame's condition concerns that frame's vectors alone, the signings of different frames, and their fields,
can be chosen independently: every promotion set is independent in all of these matroids at once.

*Proof.*  The idea: a winnable frame peels completely, and the order of its peel makes its vectors triangular, whatever
their nonzero entries.

1. *Frame h peels completely.*  L is a promotion set, so frame h is winnable with L applied: the instance whose members
   are the moves of frame h and L's members of home h + 1, and whose pool is F_h less L's members of home h, each member
   listing its maximal factors in the pool, is playable.  By [the peel theorem (P4)](playability.md#the-peel-theorem),
   peel pairs all of its members.
2. *The vectors are independent.*  By [the peel-order lemma (P9)](playability.md#the-peel-order-lemma), the members'
   vectors v_σ(u), for any nonzero values, are then linearly independent together with the unit vectors of the
   coordinates outside the pool, which are the e_y of L's members of home h.
3. *Independence in N_h^σ.*  A family made of the moves' vectors and some others is independent exactly when the moves'
   vectors are independent and the classes of the others, modulo the moves' span W, are independent.  The family of step
   2 is such a family, so the classes of L ∩ T_h are independent: L ∩ T_h is independent in N_h^σ.
4. *Loops.*  An independent set holds no element whose class is 0, so by step 3 no promotion set holds a loop.
5. *Nodes.*  A completion P ∪ X is a promotion set, so by step 3, (P ∪ X) ∩ T_h is independent in N_h^σ; so is its
   subset P ∩ T_h, and X ∩ T_h is independent in the contraction N_h^σ / P, by the definition of contraction.
6. *The contraction is the node's matroid.*  Contracting P's members of home h + 1 adds their vectors to W: they join
   the moves, as in state(P), where they are moves of frame h.  Contracting P's members y of home h quotients by their
   e_y, which deletes the coordinates y: they leave the factors, as in state(P).  The frames of state(P) are then
   reduced, as [its definition](frame-model.md#searching-the-frame-model) has it: a move m left with a single factor q
   among the factors that remain is dropped with q.  Modulo the coordinates already deleted, v_σ(m) is σ(m, q) e_q, so
   the quotient by it deletes the coordinate q, and every other number's class is its vector on the factors left, as in
   the reduced frame.  A candidate that the reduction takes out of its home is no longer a factor of its home in
   state(P), which is why the identity holds on the others.  ∎

The frame solver uses B1 (2) to keep the frame matroids of the nodes along its search path by contraction ([the GF(2)
bound](../../solver/docs/design/gf2-bound.md#the-gf2-bound)).

## The frame bound

**Corollary B2 (the frame bound).**  Let Π be a root problem of the frame model and A and B its frame matroids.  Every
promotion set of Π is independent in A and in B, and it has no more members than the promotion limit, so

    P*(Π) ≤ max { ΣI : I independent in A and in B, with no more members than the promotion limit }.

At a node with the promotions P and the candidates C, every completion P ∪ X has X independent in A/P and in B/P, so

    ΣX ≤ max { ΣI : I ⊆ C independent in A/P and in B/P, with no more members than the promotion limit less |P| },

and by B1 (2), A/P and B/P are the frame matroids of state(P) on the candidates still at their homes.  The right side
plus ΣP is the node's *frame bound*; at the root, where P is empty, it is the first right side.

*Proof.*  B1 with the plain signing over GF(2) makes a promotion set's part L ∩ T_h independent in N_h, for every frame
h.  A set is independent in a direct sum exactly when its part in each block is, and L's part in the block N_h is L ∩
T_h, so L is independent in A and in B.  A promotion set has no more members than the promotion limit ([the
promotion-limit lemma (F6)](frame-model.md#the-promotion-limit-lemma)).  At a node, P ∪ X is a promotion set, so it is
independent in A and B with no more members than the promotion limit; X is then independent in A/P and B/P, by B1 (2) or
the definition of contraction, and has no more members than the promotion limit less |P|.  ∎

*What it misses.*  When each member of a frame's instance, with a set applied, lists at most two factors of the pool,
the members are the edges of a graph on the pool, with a ground vertex for a member that lists one.  Then both
winnability and independence over GF(2) say that the edges form a forest, as in [the two-factor proposition
(C3)](franklin-moniot.md#the-two-factor-proposition), and on such frames the frame bound loses nothing.  What it misses
are the unwinnable frames whose vectors are independent over GF(2), each of which holds an [odd
core](playability.md#odd-cores), with a mover of three or more factors.  The cores it misses at large roots are measured
in [matroid intersection bounds](../../solver/notebook/search/matroid-intersection-bounds-2026-10/README.md).

*Against the matching bound.*  The frame bound is never above [the matching
bound](../../solver/docs/design/search.md#the-matching-bound), which asks only that every move and every promoted number
get a distinct maximal factor in its frame, other than a promoted number.  If a frame's family of vectors is
independent, some square submatrix of it, a column for each vector, is nonsingular, and a nonzero term of its
determinant gives each vector a distinct coordinate where its entry is nonzero.  A unit vector e_y can only take y, so
the movers take distinct factors of the frame that are not promoted: the matching bound's assignment.  So every set the
frame bound admits, the matching bound admits.

*Against the parity bound.*  The parity bound is the same construction on the whole board, and at every root where both
were measured, it equals ΣU\* plus the frame bound ([matroid intersection
bounds](../../solver/notebook/search/matroid-intersection-bounds-2026-10/README.md)).

The frame solver's default node bound, the GF(2) bound, is the frame bound ([solver/docs/design/gf2-bound.md, "The GF(2)
bound"](../../solver/docs/design/gf2-bound.md#the-gf2-bound)).

## The frame bound for the extended problem

The [extended problem](extended-problem.md#the-extended-problem) Π⁺(O) leaves the sources O ⊆ U\*_{≥3} optional, so its
promotion sets (K, L) hold *one-sided* candidates K, sources that touch one frame only, besides the two-sided candidates
L of the frame model.

**Corollary B3 (the frame bound for the extended problem).**  Let O ⊆ U\*_{≥3}.  Form the frame matroids A and B of
Π⁺(O) on its two-sided candidates as for a root problem of the frame model, and add each one-sided candidate s, a source
of frame h of Π⁺(O), to both: in the matroid of frame h's parity, to the block of frame h, represented by the class of
v(s), its maximal factors among frame h's factors; in the other matroid, as a coloop.  Then for every promotion set (K,
L) of Π⁺(O), K ∪ L is independent in A and in B, so ΣK + ΣL is at most the heaviest sum of a set independent in both.
For every subset P of K ∪ L, such as the promotions of a node, (K ∪ L) ∖ P is independent in A/P and in B/P.

*Proof.*  It is the proof of B1 with the plain signing over GF(2), with one more kind of mover.  In frame h of Π⁺(O),
winnable with (K, L) applied, the members of the instance are the fixed sources of the frame (its moves), the members of
K in it and the members of L of home h + 1, and its pool is its factors less the members of L of home h.  By P4 and P9
their vectors, with the unit vectors of L's members of home h, are independent, so (K ∪ L)'s part in the block of frame
h is independent modulo the moves.  A member of K lies in one frame and touches no other, so in the other matroid it is
a coloop, which is independent together with any independent set.  So K ∪ L is independent in both direct sums.  The
contraction by P is that of B1, step 5; contracting a one-sided candidate contracts its vector in its frame's block and
deletes its coloop in the other matroid.  ∎

[The certificate theorem (E4)](extended-problem.md#the-certificate-theorem) searches Π⁺(O), and B3 is the bound of that
search.

## The GF(3) proposition

*Explanatory: nothing rests on this proposition.  It says why the study of the multi-field bound signs its matroids over
GF(3).*

Over GF(2) the frame bound misses the odd cores.  Over GF(3) the signings together miss nothing: a frame that is not
winnable has a signing that shows it.

**Proposition B4 (the GF(3) proposition).**  Let L be a set of candidates of a state of the frame model, and h a frame
of the state that is winnable with nothing applied.  Frame h is winnable with L applied if and only if L ∩ T_h is
independent in N_h^σ for every signing σ of frame h over GF(3).

The hypothesis holds in every state, whose frames are winnable; without it, the empty set would be independent in the
matroid of a frame that is not winnable.

*Proof.*  The idea: the members a peel cannot pair cover each of their factors at least twice, so values that cancel
in every such factor make the vectors dependent.

1. *Only if.*  This is B1's argument, steps 1 to 3, which uses only that frame h is winnable with L applied.
2. *The stall.*  Suppose frame h is not winnable with L applied.  Run peel on its instance (the moves and L's members of
   home h + 1, over the pool F_h less L's members of home h) until it stalls.  By P4, a nonempty set C of members is
   left unpaired, with its live factors.  Each live factor that a member of C lists is listed by at least two of them,
   or peel would take a back move.
3. *The signing.*  For each live factor listed by members of C, give those pairs values in GF(3) that sum to 0: 1 and 2
   for a pair of members, 1, 1 and 1 for three, and for more, pairs and at most one triple.  Give every other pair the
   value 1.
4. *The block structure.*  Order the members as peel's front moves in the order made, then C, then peel's back moves in
   the reverse of the order made, and order the pool's factors in the same way: the front moves' factors, the live
   factors, the back moves' factors.  A front move, at its turn, lists one live factor, its own, so its row is 0 on the
   live factors at the end and on every factor paired after it; it is also 0 on each factor that a back move paired
   before it, since that factor, at its turn, was listed by one live member only.  A back move's factor, at its turn,
   was listed by its own member alone among the live members, so its column is 0 on C and on the members paired after
   it.  So the matrix of the vectors restricted to the pool is block lower triangular,

       front rows:  T_F  0   0
       C:           X    K   0
       back rows:   Y    Z   T_B

   with T_F and T_B square, lower triangular with nonzero diagonals, and K the rows of C on the live factors, whose
   columns sum to 0 by step 3.
5. *A dependency.*  Give the rows of C the coefficient 1, the back rows 0, and the front rows the coefficients a that
   solve a T_F = −(the sum of the rows of X), which exist since T_F is invertible.  The combination vanishes on every
   column: on the front factors by the choice of a, on the live ones because K's columns sum to 0 and the front rows are
   0 there, on the back ones because every row with a nonzero coefficient is 0 there.  So it vanishes on the pool, and
   with the unit vectors e_y of L's members of home h it gives a vanishing combination of the whole family, with the
   coefficient 1 on each member of C.
6. *L ∩ T_h is dependent.*  Modulo the moves' span this is a relation among the classes of L ∩ T_h.  If all of its
   coefficients on members of L were 0, the combination would be a vanishing combination of the moves' vectors alone,
   with the coefficient 1 on the members of C, which are then moves.  The moves alone are independent under σ, by B1's
   argument for frame h with nothing applied, which is winnable.  So some coefficient on L is not 0, and L ∩ T_h is
   dependent in N_h^σ.  ∎

The same holds over every field with three or more elements, by D. Hershkowitz and H. Schneider's characterization of
the zero patterns all of whose matrices have the same rank ("Ranks of zero patterns and sign patterns", *Linear and
Multilinear Algebra* 34 (1993) 3–19): over such a field, every matrix with a given pattern of nonzero entries has full
row rank exactly when some matching of all the rows to columns is the only matching between its ends, and such a
matching orders the rows and columns triangularly, which lets peel pair every member.  Over GF(2) the proposition
fails: the plain signing is the only one there, and an odd core is independent under it.

*Example.*  The odd core {12, 18, 20, 30, 45} of the board of 45 lists the factors 4, 6, 9, 10 and 15 (12: 4, 6; 18:
6, 9; 20: 4, 10; 30: 6, 10, 15; 45: 9, 15).  Peel has no move on it, and every factor is listed by two of its members
or, for 6, three.  With the values 1 in every pair, the determinant of the five vectors is 3, odd, so over GF(2) they
are independent.  With the values of step 3, 1 and 2 on the pairs at 4, 9, 10 and 15 and 1, 1, 1 at 6, the five
vectors sum to 0 over GF(3).

![The odd core {12, 18, 20, 30, 45} as members joined to the factors 4, 6, 9, 10 and 15, with an entry of 1 or 2 on each edge so that the entries at every factor add up to 0 modulo 3](img/gf3-45.svg)

*The core with one choice of the values of step 3: the entries at every factor sum to 0 (mod 3).*

## The decomposition bound

Several signed frame matroids combine with A and B into one bound by splitting the weights among them, as weak duality
allows ([primer](primer.md#weak-duality-and-lagrangian-relaxation)).

**Theorem B5 (the decomposition bound).**  Let Π be a root problem of the frame model, A and B its frame matroids, and
N_1, …, N_t signed frame matroids of Π, each N_j of one frame h_j and one signing over some field (several may share a
frame).  For weights c on the candidates, let MI(c) be the largest c(I) of a set I independent in A and in B with at
most as many members as the promotion limit (I = ∅ included); for weights μ on T_{h_j}, let G_j(μ) be the largest μ(J)
of a set J independent in N_j.  Then for all real vectors λ_1, …, λ_t, with λ_j on T_{h_j} and 0 off it, every promotion
set L of Π satisfies

    ΣL ≤ MI(w − Σ_j λ_j) + Σ_j G_j(λ_j) = D(λ),

where w(x) = x.  The same holds with the loops of every N_j left out of the sets I.  At a node with the promotions P, it
holds for ΣX of every completion P ∪ X, with the contractions by P of A, B and the N_j and the promotion limit less |P|.

*Proof.*  Split the weight: ΣL = (w − Σ_j λ_j)(L) + Σ_j λ_j(L ∩ T_{h_j}).  L is independent in A and B with no more
members than the promotion limit (B2), so the first term is at most MI(w − Σ_j λ_j).  L ∩ T_{h_j} is independent in N_j
(B1), so the j-th term of the sum is at most G_j(λ_j).  L holds no loop of any N_j (B1 (1)), so leaving the loops out of
the sets I keeps L among them.  At a node, the same argument runs with X in place of L, by B1 (2) and B2.  ∎

Each term is a standard computation: MI is a weighted matroid intersection, and G_j the greedy algorithm of N_j
([primer](primer.md#the-greedy-algorithm-of-a-matroid)).  At λ = 0 every G_j is 0 and D(0) is the frame bound, so the
best multipliers do at least as well.  The least value of D over the multipliers is the largest w·z over the points z of
the convex hull of the sets independent in A and B with no more members than the promotion limit, whose restriction to
each T_{h_j} lies in the convex hull of N_j's independent sets.  This is Lagrangian decomposition (M. Guignard and S.
Kim, "Lagrangean decomposition: a model yielding stronger Lagrangean bounds", *Mathematical Programming* 39 (1987)
215–228).  J. Edmonds' theorems on the polytope of a matroid and on the intersection of two describe both hulls by rank
inequalities, which makes the least value a linear program ("Submodular functions, matroids, and certain polyhedra",
1970, cited in full in [the parity bound](parity-bound.md#computing-and-checking-the-bound)).  That says what the bound
can reach; the bound holds at every λ, so nothing rests on how the multipliers are found.

The study of the multi-field bound measured it ([a bound over GF(2) and GF(3) signed frame
matroids](../../solver/notebook/search/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md)), and
[solver/docs/design/gf2-bound.md, "The decomposition
bound"](../../solver/docs/design/gf2-bound.md#the-decomposition-bound) says how its code evaluates it exactly.
