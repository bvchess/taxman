# The decision procedure, and unplayable sets

Part of [the theory](../THEORY.md).

## The decision procedure

**Theorem 1 (playability).**  A set S of moves can all be played, in some order, **iff** there is a matching assigning
each member a distinct maximal factor not in S such that the precedence relation "a before b whenever a's assigned
factor divides b, or a divides b" is acyclic.  Any topological order of the precedence is a legal game.

*Proof.*  One direction is a construction: play the topological order, and each move's reserved payment provably
survives until its turn.  The other is the Franklín–Moniot lifting argument turned inward.  Whatever divisor d a move
actually surrenders lifts to a maximal factor f with d | f | c; f must still be in the pot (anything that had removed f
would have removed d with it), and f cannot itself be a move of S, or the move's sweep would destroy an unplayed
number.  So every legal game already pays each move with at least one distinct outside maximal factor.  ∎

Maximal factors are therefore the only factor notion the whole project needs.  One boundary is worth marking:
*feasibility* is pool-invariant, but raw play orders are not.  The function `cascade`, which plays peel's sequence
directly under real sweeps, still needs true divisors: at n=5 a maximal-factor pool emits [4, 5], and playing 4 sweeps
the 1 that 5 needed.

**Theorem 2 (schedulability).**  Whenever `peel` succeeds on a reduction instance (members C, factor pool F with C ∩ F =
∅, each member's list L(c) = mf(c) ∩ F), its returned assignment pay(·) has an acyclic precedence "a before b whenever
pay(a) | b, or a | b".  So a legal play order always exists, and the ordering step can never fail.

*Proof.*  Every payment is a maximal factor of its member, so Ω(pay(x)) = Ω(x) − 1 exactly (primes pay 1, and Ω(1) =
0).  Follow the potential Ω(pay(aᵢ)) around a supposed cycle.  If an edge has a | b (whether or not pay(a) | b also
holds), then Ω(pay(a)) = Ω(a) − 1 < Ω(b) − 1 = Ω(pay(b)), strictly.  Otherwise pay(a) | b properly (payments lie in F,
disjoint from C), so Ω(pay(a)) ≤ Ω(b) − 1 = Ω(pay(b)).  A cycle forces equality everywhere: no member-divides-member
edges, and on every edge Ω(pay(a)) = Ω(b) − 1, which for a divisor of b means b/pay(a) is prime: pay(a) is a *maximal
factor of b*.  Since pay(a) ∈ F, it lies in L(b): every cycle lives on pool edges.  (A prime paying 1 has out-edges to
everything, but equality forces its cycle successor to be prime with L = {1}, and the same collision below.)

Now let c be the earliest-peeled member of such a cycle and lean on two invariants of the implementation: a factor
leaves a member's live list only when globally consumed as some pair's payment, and `comps_of[f]` holds exactly the live
members listing f.  If c was front-peeled, its live list at that step was {pay(c)}; its cycle predecessor a peels later,
so pay(a) is unconsumed and still in c's live list, and pay(a) = pay(c), contradicting distinct payments.  If c was
back-peeled, pay(c) was listed by no other live member; its cycle successor d peels later, is live, and lists pay(c),
contradicting that uniqueness.  ∎

The theorem covers exactly the assignments the code can emit: the fast Kuhn-matching tier checks acyclicity explicitly
before accepting, and falls back to peel, whose output the theorem covers, whenever its matching goes cyclic.  The proof
*requires* maximal-factor payments; the potential argument dies with all-divisor pools.  The n=39 set {39, 33, 22, 26},
introduced properly in the next section, shows why peeling is structurally immune to cycles: a cycle core has no
degree-1 vertex, so the forced-move discipline stalls and refuses rather than threads through it.

**Theorem 3 (completeness).**  If a set is playable, peel succeeds on it.  (Contrapositive: every peel refusal is a
proof of unplayability.)

*Proof.*  Three lemmas.  *Front moves preserve playability*: if live member c has exactly one live factor f, every
covering matching must assign f to c, and removing the pair leaves an acyclic covering matching of the residue
(restriction of acyclic is acyclic).  *Back moves preserve playability*: if live factor f is listed by exactly one live
member c, take any acyclic covering matching M.  Nobody but c can be matched to f, so M minus c's own pair covers
everyone else while avoiding f, and survives the removal of (c, f).  The invariant here is playability of the *residual
instance*, which depends only on which pair was removed, not on the fact that the code pays c with f while M may pay it
with something else (that discrepancy belongs to Theorem 2, not this one).  *A stall is unplayable*: if no forced move
exists with members remaining, every live factor has zero or at least two live members.  Take any covering matching of
the residue: each member a's payment is a live factor with at least one lister (a itself), hence at least two, so some
other live member b lists pay(a), and pay(a) | b is a precedence edge a → b.  Every vertex has an out-edge, so the
finite digraph contains a directed cycle; every covering matching is precedence-cyclic, and the residue is unplayable.
Chaining: a playable input stays playable through every forced move, a playable residue can never strand a member at
zero factors, and a halt with members remaining would make the (playable) residue unplayable, a contradiction.  ∎

Note what each theorem needs.  Completeness holds for *any* pool of proper divisors, maximality unused, but the converse
(Theorem 2) genuinely requires maximal factors: with lists {6: [1], 9: [3]} the unique matching is a perfect 2-cycle (1
| 9 and 3 | 6), the set is unplayable, and peeling would accept it anyway.  Only in the maximal-factor regime do the two
theorems interlock into the full equivalence: **peel succeeds ⟺ the set is playable.**

## Unplayable sets: cores and the residual

An unplayable set is unplayable because of a *core*, and every way of taking members out ends at the same one.  A
*self-covering* set is a nonempty subset in which every member's every maximal factor is also some other member's
maximal factor.  *Dismantling* repeatedly removes any member holding a factor no other remaining member lists; a set
with no self-covering subset dismantles to nothing.  The n=39 specimen {39, 33, 22, 26} is the minimal self-covering
core.

Standing hypothesis for this section: the instance is **separated**, that is no member is a maximal factor of another
member.  Every instance this project computes with satisfies it automatically: members of a sources' or band instance
all exceed n/2 (or the band floor), while a maximal factor of such a member is at most half of it.  Separation buys a
decisive simplification: under it, every member's live list is its full maximal-factor set in every subset, so
self-covering and dismantling are intrinsic notions, stable under taking subsets.  The hypothesis is not decorative.
With members doubling as each other's factors the pool shifts as members come and go, and Theorems 5 and 6 below become
false: for K = {2, 4, 8} (residual {4, 8}), the exclusion E = {2, 8} leaves the playable {4}, since 2 re-enters the
pool, yet K∖(E∩R) = {2, 4} strands 4, violating Theorem 6; and the extension lemma dies at S = {8, 33, 37} with x = 4,
whose arrival as a member removes 4 from the pool and strands 8.

**Theorem 4 (survival and confluence; no separation needed).**  A self-covering subset survives every dismantling
sequence; all dismantling sequences of an instance end at the same fixpoint, the *residual*; a nonempty residual is
itself self-covering, and the residual equals the union of all self-covering subsets.  In particular the residual is
empty iff no self-covering subset exists.

*Proof.*  If the next deletion removes x with private factor f and x belonged to a self-covering C, then f, like every
factor of x, is listed by another member of C, all of C being alive; that contradicts privacy, so dismantling never
touches C.  A nonempty fixpoint is self-covering by definition of "no forced deletion remains" (a member with an empty
list is vacuously covered and never removable).  Take two fixpoints R₁, R₂ of different sequences: R₁ is self-covering,
hence survives the second sequence, so R₁ ⊆ R₂, and symmetrically.  ∎

**Extension lemma (separated instances).**  Let S be playable and let x ∉ S be a member that is not a maximal factor of
any member of S, with some f ∈ mf(x) listed by no member of S.  Then S ∪ {x} is playable.

*Proof.*  Adding x leaves every old member's list intact (x sits in no one's maximal factors; the pool only grows).
Take an acyclic covering assignment of S and extend it by pay(x) = f; distinctness holds because every payment is listed
by its payer and f is listed by no member of S.  Suppose the extended precedence has a cycle.  The precedence among old
members is unchanged, so the cycle passes through x.  Now reuse the first half of Theorem 2's proof, which is
assignment-agnostic: it needs only that every payment is a maximal factor of its member, so Ω(pay(v)) = Ω(v) − 1
exactly.  Along a member-divides-member edge the potential strictly increases, so a cycle contains none; along a payment
edge a → b it increases weakly, with equality forcing pay(a) ∈ mf(b).  A cycle forces equality everywhere, in
particular on the cycle's edge leaving x, which then requires f ∈ mf(z) for some z ∈ S, contradicting f's privacy.  ∎

**Theorem 5 (core characterization, separated instances).**  A set is playable iff its residual is empty, or
equivalently iff it contains no self-covering subset.

*Proof.*  (⇒, no separation needed)  A self-covering C is unplayable: a member with an empty list can never pay, and
otherwise any covering assignment gives each x ∈ C a payment listed by another member of C, an out-edge within C, so the
finite digraph has a cycle and every covering assignment is precedence-cyclic.  Downward closure of playability
finishes.  (⇐)  Let x₁, …, x_k be a complete dismantling order (empty residual) and T_j = {x_j, …, x_k}.  The empty set
T_{k+1} is playable; x_j has a factor private in T_j and is no member's maximal factor (separation), so the extension
lemma lifts playability from T_{j+1} to T_j.  Induction ends at T₁ = M.  ∎

Without separation the characterization has never been observed to fail, but remains unproven (see the evidence table).

**Theorem 6 (exclusion confinement, separated instances).**  Let K have residual R.  If K ∖ E is playable, so is K ∖ (E
∩ R).

*Proof.*  Suppose K ∖ (E ∩ R) is unplayable.  By Theorem 5 it contains a self-covering C; by Theorem 4, C ⊆ R.  The set
C avoids E ∩ R and lies in R, so it avoids E entirely: C ⊆ K ∖ E.  But K ∖ E is playable and by Theorem 5 contains no
self-covering subset.  ∎

Corollaries.  Minimal exclusion sets lie inside the residual (otherwise E ∩ R is a smaller valid exclusion).  Every
maximum-weight playable subset, for any positive weights, cardinality included, contains everything outside the
residual.  Exact searches restricted to subsets of the residual are therefore sound: the reduction every exhaustive
campaign below leans on.
