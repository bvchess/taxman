# The theory, with proofs

This is the proof companion to [README.md](README.md): every theorem
the ledger there cites lives here in full.  The README states what is
claimed and how well it is tested; this file is *why*.

Everything below supports three headline facts:

* **Proven.**  `peel` succeeds on a set **iff** the set is
  playable **iff** it contains no self-covering core (Theorems 1-6).
  Acceptance is not a heuristic anywhere in this project: it is a
  decision procedure for playability, proven in both directions.
* **Open.**  The upper-half optimality conjecture: in every game, no
  playable subset of the upper half outweighs the one descending
  greedy selects (the set the README calls U\*).  "The open problem"
  below states it precisely, gives its weight-free working form —
  conjecture (★) — and maps the exact boundary between what is
  proven and what remains.  A linear-algebra certificate, (♦),
  settles it game by game: every game to 100,000 is certified.
* **Proven, but for one statement.**  The frame solver's answer
  (`rust/`) is opt(n) exactly when *some optimal game plays every
  member of U\* with three or more prime factors*, on every board
  the solver supports (Lemma Ω, Theorems L, B and M; "The frame
  model: what is proven and what remains").  That statement, U, is
  open in general and holds wherever the optimum has been proven on
  the whole board (every n ≤ 1300).

Empirical support for each claim is collected in the evidence table
at the end.

## Definitions

The *maximal factors* of c are the divisors f with c/f prime — the
divisors reached by deleting one prime from c (mf(4) = {2},
mf(6) = {2, 3}; a prime's only maximal factor is 1).  A *reduction
instance* is a set of members C, a factor pool F disjoint from C, and
each member's factor list mf(c) ∩ F; `core.peel` reduces such
instances by forced moves only (a member with one live factor takes it;
a factor with one live lister goes to that lister).  The upper half of
game n — the numbers above n/2, which never divide one another — forms
one naturally, with F the union of the members' maximal factors.  A
number a game takes is a *pick* (the solver's and the wiki's *move*);
Theorem 1 says *selection*.

## The decision procedure

**Theorem 1 (playability).**  A set S of selections can all be
played, in some order, **iff** there is a matching assigning each
member a distinct maximal factor not in S such that the precedence
relation "a before b whenever a's assigned factor divides b, or a
divides b" is acyclic.  Any topological order of the precedence is a
legal game.

*Proof.*  One direction is a construction: play the topological
order, and each pick's reserved payment provably survives until its
turn.  The other is the Franklín–Moniot lifting argument turned
inward.  Whatever divisor d a pick actually surrenders lifts to a
maximal factor f with d | f | c; f must still be in the pot
(anything that had removed f would have removed d with it), and f
cannot itself be a selection, or the pick's sweep would destroy an
unplayed number.  So every legal game already pays each pick with at
least one distinct outside maximal factor.  ∎

Maximal factors are therefore the only factor notion the whole
project needs.  One boundary is worth marking: *feasibility* is
pool-invariant, but raw play orders are not.  `cascade`, which plays
peel's sequence directly under real sweeps, still needs true
divisors — at n=5 a maximal-factor pool emits [4, 5], and playing 4
sweeps the 1 that 5 needed.

**Theorem 2 (schedulability).**  Whenever `peel` succeeds on a
reduction instance (members C, factor pool F with C ∩ F = ∅, each
member's list L(c) = mf(c) ∩ F), its returned assignment pay(·) has
an acyclic precedence "a before b whenever pay(a) | b, or a | b" —
so a legal play order always exists, and the ordering step can never
fail.

*Proof.*  Every payment is a maximal factor of its member, so
Ω(pay(x)) = Ω(x) − 1 exactly (Ω = prime factors with multiplicity;
primes pay 1, Ω(1) = 0).  Follow the potential Ω(pay(aᵢ)) around a
supposed cycle.  If an edge has a | b (whether or not pay(a) | b
also holds), then Ω(pay(a)) = Ω(a) − 1 < Ω(b) − 1 = Ω(pay(b)),
strictly.  Otherwise pay(a) | b properly (payments lie in F,
disjoint from C), so Ω(pay(a)) ≤ Ω(b) − 1 = Ω(pay(b)).  A cycle
forces equality everywhere: no member-divides-member edges, and on
every edge Ω(pay(a)) = Ω(b) − 1, which for a divisor of b means
b/pay(a) is prime — pay(a) is a *maximal factor of b*.  Since
pay(a) ∈ F, it lies in L(b): every cycle lives on pool edges.  (A
prime paying 1 has out-edges to everything, but equality forces its
cycle successor to be prime with L = {1}, and the same collision
below.)

Now let c be the earliest-peeled member of such a cycle and lean on
two invariants of the implementation: a factor leaves a member's
live list only when globally consumed as some pair's payment, and
`comps_of[f]` holds exactly the live members listing f.  If c was
front-peeled, its live list at that step was {pay(c)}; its cycle
predecessor a peels later, so pay(a) is unconsumed and still in c's
live list — pay(a) = pay(c), contradicting distinct payments.  If c
was back-peeled, pay(c) was listed by no other live member; its
cycle successor d peels later, is live, and lists pay(c) —
contradicting that uniqueness.  ∎

The theorem covers exactly the assignments the code can emit: the
fast Kuhn-matching tier checks acyclicity explicitly before
accepting, and falls back to peel — whose output the theorem
covers — whenever its matching goes cyclic.  The proof *requires*
maximal-factor payments; the potential argument dies with
all-divisor pools.  The n=39 set {39, 33, 22, 26} — introduced
properly in the next section — shows why peeling is structurally
immune to cycles: a cycle core has no degree-1 vertex, so the
forced-move discipline stalls and refuses rather than threads
through it.

**Theorem 3 (completeness).**  If a set is playable, peel
succeeds on it.  (Contrapositive: every peel refusal is a
proof of unplayability.)

*Proof.*  Three lemmas.  *Front moves preserve playability*: if live
member c has exactly one live factor f, every covering matching must
assign f to c, and removing the pair leaves an acyclic covering
matching of the residue (restriction of acyclic is acyclic).  *Back
moves preserve playability*: if live factor f is listed by exactly
one live member c, take any acyclic covering matching M — nobody but
c can be matched to f, so M minus c's own pair covers everyone else
while avoiding f, and survives the removal of (c, f).  The invariant
here is playability of the *residual instance*, which depends only
on which pair was removed — not on the fact that the code pays c
with f while M may pay it with something else (that discrepancy
belongs to Theorem 2, not this one).  *A stall is unplayable*: if no
forced move exists with members remaining, every live factor has
zero or at least two live members.  Take any covering matching of
the residue: each member a's payment is a live factor with at least
one lister (a itself), hence at least two — so some other live
member b lists pay(a), and pay(a) | b is a precedence edge a → b.
Every vertex has an out-edge, so the finite digraph contains a
directed cycle; every covering matching is precedence-cyclic, and
the residue is unplayable.  Chaining: a playable input stays
playable through every forced move, a playable residue can never
strand a member at zero factors, and a halt with members remaining
would make the (playable) residue unplayable — contradiction.  ∎

Note what each theorem needs.  Completeness holds for *any* pool of
proper divisors — maximality unused — but the converse (Theorem 2)
genuinely requires maximal factors: with lists {6: [1], 9: [3]} the
unique matching is a perfect 2-cycle (1 | 9 and 3 | 6), the set is
unplayable, and peeling would accept it anyway.  Only in the
maximal-factor regime do the two theorems interlock into the full
equivalence: **peel succeeds ⟺ the set is playable.**

## Unplayable sets: cores and the residual

A *self-covering* set is a nonempty subset in which every member's
every maximal factor is also some other member's maximal factor.
*Dismantling* repeatedly removes any member holding a factor no
other remaining member lists; a set with no self-covering subset
dismantles to nothing.  The n=39 specimen {39, 33, 22, 26} is the
minimal self-covering core.

Standing hypothesis for this section: the instance is **separated**
— no member is a maximal factor of another member.  Every instance
this project computes with satisfies it automatically: members of an
upper-half or band instance all exceed n/2 (or the band floor),
while a maximal factor of such a member is at most half of it.
Separation buys a decisive simplification: under it, every member's
live list is its full maximal-factor set in every subset, so
self-covering and dismantling are intrinsic notions, stable under
taking subsets.  The hypothesis is not decorative.  With members
doubling as each other's factors the pool shifts as members come and
go, and Theorems 5 and 6 below become false: for K = {2, 4, 8}
(residual {4, 8}), the exclusion E = {2, 8} leaves the playable {4}
— 2 re-enters the pool — yet K∖(E∩R) = {2, 4} strands 4, violating
Theorem 6; and the extension lemma dies at S = {8, 33, 37} with
x = 4, whose arrival as a member removes 4 from the pool and
strands 8.

**Theorem 4 (survival and confluence; no separation needed).**  A
self-covering subset survives every dismantling sequence; all
dismantling sequences of an instance end at the same fixpoint, the
*residual*; a nonempty residual is itself self-covering, and the
residual equals the union of all self-covering subsets.  In
particular the residual is empty iff no self-covering subset exists.

*Proof.*  If the next deletion removes x with private factor f and x
belonged to a self-covering C, then f — like every factor of x — is
listed by another member of C, all of C being alive; that
contradicts privacy, so dismantling never touches C.  A nonempty
fixpoint is self-covering by definition of "no forced deletion
remains" (a member with an empty list is vacuously covered and never
removable).  Two fixpoints R₁, R₂ of different sequences: R₁ is
self-covering, hence survives the second sequence, so R₁ ⊆ R₂, and
symmetrically.  ∎

**Extension lemma (separated instances).**  Let S be playable and
let x ∉ S be a member that is not a maximal factor of any member of
S, with some f ∈ mf(x) listed by no member of S.  Then S ∪ {x} is
playable.

*Proof.*  Adding x leaves every old member's list intact (x sits in
no one's maximal factors; the pool only grows).  Take an acyclic
covering assignment of S and extend it by pay(x) = f; distinctness
holds because every payment is listed by its payer and f is listed
by no member of S.  Suppose the extended precedence has a cycle.
The precedence among old members is unchanged, so the cycle passes
through x.  Now reuse the first half of Theorem 2's proof, which is
assignment-agnostic — it needs only that every payment is a maximal
factor of its member, so Ω(pay(v)) = Ω(v) − 1 exactly: along a
member-divides-member edge the potential strictly increases, so a
cycle contains none; along a payment edge a → b it increases weakly,
with equality forcing pay(a) ∈ mf(b).  A cycle forces equality
everywhere — in particular on the cycle's edge leaving x, which then
requires f ∈ mf(z) for some z ∈ S, contradicting f's privacy.  ∎

**Theorem 5 (core characterization, separated instances).**  A set
is playable iff its residual is empty — equivalently, iff it
contains no self-covering subset.

*Proof.*  (⇒, no separation needed)  A self-covering C is
unplayable: a member with an empty list can never pay, and otherwise
any covering assignment gives each x ∈ C a payment listed by another
member of C — an out-edge within C — so the finite digraph has a
cycle and every covering assignment is precedence-cyclic.  Downward
closure of playability finishes.  (⇐)  Let x₁, …, x_k be a complete
dismantling order (empty residual) and T_j = {x_j, …, x_k}.
T_{k+1} = ∅ is playable; x_j has a factor private in T_j, is no
member's maximal factor (separation), so the extension lemma lifts
playability from T_{j+1} to T_j.  Induction ends at T₁ = M.  ∎

Without separation the characterization has never been observed to
fail, but remains unproven (see the evidence table).

**Theorem 6 (exclusion confinement, separated instances).**  Let K
have residual R.  If K ∖ E is playable, so is K ∖ (E ∩ R).

*Proof.*  Suppose K ∖ (E ∩ R) is unplayable.  By Theorem 5 it
contains a self-covering C; by Theorem 4, C ⊆ R.  C avoids E ∩ R and
lies in R, so it avoids E entirely: C ⊆ K ∖ E.  But K ∖ E is
playable and by Theorem 5 contains no self-covering subset.  ∎

Corollaries.  Minimal exclusion sets lie inside the residual
(otherwise E ∩ R is a smaller valid exclusion).  Every
maximum-weight playable subset — any positive weights, cardinality
included — contains everything outside the residual.  Exact searches
restricted to subsets of the residual are therefore sound: the
reduction every exhaustive campaign below leans on.

## The open problem

*Descending greedy* considers the members of an instance in
decreasing numeric order and admits each one whenever the set
selected so far, plus it, is still playable (peel is the test).  Run
on the upper-half instance of game n, this is `optimize_mini`, and
the set it returns is the U\* of the README.

> **Conjecture (upper-half optimality).**  For every game n, no
> playable subset of the upper-half instance outweighs the set
> descending greedy selects.

Everything in this section locates the exact boundary between what
is proven about this conjecture and what remains open.

### Where exchange breaks

For members with at most two distinct primes the core notion turns
geometric: map each factor value to a vertex (plus a ground vertex
for primes and prime powers, whose single factor makes a pendant
edge) and each member to the edge joining its two factors.  A core
is exactly a subgraph of minimum degree 2, so **playable ⟺ forest**.
Forests trade well: removing an edge and adding another never traps
you, which is why greedy is bulletproof in that regime.  Members
with three or more distinct primes are hyperedges glued onto the
graph, and there the trading breaks.  The smallest failure is at
**n = 2873**, on six members built from {11, 13, 17}:

    A = {1573, 2057, 2431, 2873}                (playable, size 4)
    B = {1573, 1859, 2057, 2197, 2873}          (playable, size 5)

Both members of B∖A jam: A+1859 and A+2197 each contain a core.  Two
maximal playable sets of *different sizes* — so playable sets do not
support the exchange argument that justifies greedy in the textbook
setting, and the optimality conjecture cannot be proven that way.

And yet greedy survives: in every three-prime gadget tested,
descending greedy still returns the maximum-weight playable
set — at n=2873 it takes a heavy size-5 set, {1859, 2057, 2197,
2431, 2873}, and never lands on the stuck size-4 A, because the
stuck base is also the *light* one.  The arithmetic is load-bearing:
on the same incidence structure, 48 of the 720 orderings of the six
members make greedy fail — the actual member values land in the
safe majority, and random abstract weightings do not.

### The frontier argument, and exactly where it stops

The natural direct proof gets remarkably far.  Compare greedy's set
G to a supposedly heavier playable set T on the same instance, at
their largest disagreement d.  T
cannot be the one holding d (T would contain greedy's own rejection
certificate — downward closure).  So d ∈ G, T skips it, and if T is
maximum-weight then T+d must be unplayable: every core of T+d
contains d (T is playable) and dips below d — the part of T+d at or
above d is exactly G ∩ {≥ d}, playable *because* step one put d in
G, so no core fits inside it.  Let R be the union of those cores —
the 2-core of T+d in hypergraph terms.  Repairs live entirely inside
R (Theorem 6 with K = T+d); a sacrifice t repairs T+d exactly when t
lies in **every** minimal core of R (the "victim law"); and when
every member of R has at most two distinct primes, R−d is a forest,
R has *exactly one* minimal core (a forest plus one edge has one
cycle), so a below-d victim always exists and the sacrifice T+d−t
both restores playability and gains weight — contradiction.
**Greedy is provably weight-optimal wherever the jam is
hyperedge-free.**

Beyond that regime the single-sacrifice step is **false**.  At
n = 9170, d = 6137 = 17·19², the six members {4913, 5491, 6137,
6647, 7429, 8303} (all built from 17, 19, 23; the hyperedge is
7429 = 17·19·23) form an R with **two** minimal cores through d
sharing nothing else — their below-d parts are {4913} and {5491},
disjoint — so no single below-d sacrifice repairs, and every single
repair costs at least d.  Greedy is still right there: every
d-avoiding way to break both cores costs more than d (any single
alternative victim is heavier than d; any two members sum past n),
and the full-context optimum does take d — skipping it costs 1,224
points net.  But proving that in general is now visibly a
*non-local, weighted* question: whenever no single lighter sacrifice
repairs the jam, the would-be improver's kept members must be
charged for what they lock out elsewhere in the component, not just
at the frontier.  That is the open problem's irreducible form, with
n=9170 as the first test any proof must pass.

### The cardinality reformulation

The weights can be removed from the open problem entirely.  Taxman's
weights are the members themselves, so any weight assignment is
consistent with the value ordering — and for order-consistent
weights, the classic majorization fact applies: greedy's set
outweighs a competitor for *every* consistent weighting iff it
matches or beats the competitor's cardinality within every top
segment of the ordering.  Since descending greedy's decisions above
any threshold never depend on anything below it (its restriction to
{> t} IS its run on the truncated instance), the optimality
conjecture becomes one unweighted statement:

> **(★)** For every game n, on every connected component of the
> incidence graph of its upper-half instance — members the numbers
> in (n/2, n], factors their maximal factors — and every truncation
> {> t} of one, descending greedy returns a **maximum-cardinality**
> playable subset.

(★) implies the optimality conjecture by summing the truncations
(layer-cake); by majorization it is also *necessary* if optimality
is to hold for every weighting consistent with the value order, so
nothing is lost — or added — in dropping the weights.

Three facts sharpen what a proof of (★) must look like.

*The ordering is the protection, not the magnitudes.*  Every one of
the 48 trap orderings on the n=2873 (equally the n=9170) incidence
structure is a pure cardinality upset: greedy is steered into
excluding both members of a factor-sharing pair, keeping 4 where 5
fit.  No magnitude-style axiom blocks that — near-uniform weights
realize it, and "all members exceed n/2" kills none of the 48.  The
real numeric order simply never lands among the 48: at n=2873 the
trap needs the sharing pair ranked last, and 11²·13 sits below both
sharers.  A proof must therefore use the arithmetic of the ordering
— the *window law*: among the members f·p sharing a factor f, the
value order is the order of the cofactor primes, f·q < f·p iff
q < p — because the incidence structure alone provably permits the
trap.

*The component is the unit of proof; sub-instances lie.*  On the
bare n=9170 sextet, greedy's 4-of-6 is not maximum — dropping only d
leaves a playable 5-set — and it stays non-maximum with the first
ring of attachments added.  What restores it is 6859 = 19³, farther
out in the component, whose demand for the factor 361 tightens the
contest.  Cardinality dominance genuinely fails on fragments and
holds on the full component.  (Exact searches stay tractable there
because Theorem 6 confines them to the component's residual —
n=2873's 377-member component has a 70-member residual.)

*Matching certifies most components.*  Playable ⊆ matchable gives
the free bound |G ∩ K| ≤ max-cardinality ≤ ν(K), the component's
maximum-matching size (the *matching-number bound*); greedy meets ν on
86.8% of the large components, certifying (★) there with no search.
Where ν overshoots, every exactly-probed case found greedy still optimal —
the surplus was sets that admit a matching but no acyclic one, the
cardinality twin of the weight slack in the Franklín–Moniot bound
(first visible at n=21, where a 145-sum set is matchable but the
true optimum is 144).

A census of the full 3-prime shape space (parameters in the table)
closed the known jam shapes at the ordering level.  Inclusion-
minimal self-covering shapes can never trap greedy — removing any
member of one leaves no self-covering subset, so every victim
repairs it.  Trap potential lives only in *padded* jams (a core plus
outside members competing for the core's factors), and appears **iff the shape's
maximal playable subsets come in two adjacent sizes**: trap
potential is precisely the failure of the exchange property, the
n=2873 phenomenon, now a characterization rather than an anecdote.
Of the 22,909 trap orderings found, all but seven are monomially
impossible (they demand an exponent-vector inversion like p²q above
pq², which no prime triple realizes) or never realized; the seven
realizable ones were defeated at every realization tested by a
*window guard* — a member (q³ or q²r, in the shape's primes
p < q < r) that fixed comparisons among the shape's own members
force into (n/2, n], breaking the trap.
The general lemma this points at — every trap ordering requires
either a monomial inversion or the absence of a window resident the
shape's own realization forces to exist — is the sharpest known form
of why the integers protect greedy.  The guard's *sufficiency* in
general still rests on measurement.

The proof obligation, sharpened: *within one component's jam region
— its residual, where Theorem 6 confines every exclusion — the
numeric ordering makes greedy's exclusion count minimal* — with the
window law as the engine, and n=9170's component as the first test
any proof must pass.

### The span certificate (♦)

Linear algebra over GF(2) gives a far stronger bound than matching.
Give each member its incidence vector: a 1 at each of its maximal
factors.

**Lemma (playable sets are independent).**  The incidence vectors of
a playable set are linearly independent over GF(2) (indeed over any
field).

*Proof.*  A playable set dismantles completely (Theorems 4 and 5): in
some order x₁, x₂, … each xᵢ has a factor that no later member lists.
In a vanishing sum over a nonempty set of members, the coordinate of
the earliest member's private factor is 1.  ∎

So no playable subset of a set K is larger than the GF(2) rank of K
(the *rank bound*).
On members with at most two distinct primes this is the graphic
matroid's rank, exact there; on three-prime members, whose factors
pq, pr, qr are the edges of a triangle on the primes, a dependency is
a closed surface mod 2 — every factor an even number of times, so a
minimal dependent set is a core.

**Theorem 7 ((♦) implies (★)).**  Suppose that whenever descending
greedy rejects a member x, x's vector is a sum of vectors of members
it kept above x.  Then greedy is maximum-cardinality on every
truncation {≥ t} of every component, and its set outweighs every
playable set for every weighting consistent with the order.

*Proof.*  Greedy's kept members above t are playable, hence
independent, and span every member above t (the kept ones trivially,
the rejected ones by hypothesis): they are a basis of K_{≥t}, so
greedy keeps rank(K_{≥t}) members, which bounds every playable
subset.  The weighted claim is greedy optimality on the linear
matroid, whose bases greedy's sets are.  ∎

(♦) is stronger than (★): a rejection forced only by an *odd* core
(self-covering, with no subset listing every factor an even number of
times) would break it without breaking (★).  Odd cores exist in
Taxman — the exchange failure at n = 2873 needs one, since
GF(2)-independent sets do satisfy exchange — but descending greedy has
never rejected because of one.  Checking (♦) is greedy itself plus
Gaussian elimination, a few seconds per game at n = 500,000, and it
held in every game checked (Evidence, below).  The open problem
becomes: **prove (♦)**.  The adversarial experiments of the prover
(`rust/prover`, a branch and bound over sets of picks on the whole
board) locate what it needs: it holds under every order of
the real incidence structure that obeys the window law, even
adversarially chosen ones, and under the window law reversed, but
fails as often as a uniformly random order under the window law for a
random ranking of the primes, or for the primes perturbed by more
than a few percent — while generalized primes perturbed by 50%, with
the members and factors rebuilt from them, never fail.  The
protection is the window law *for the ranking of the primes that also
decides which products lie in the window*.

## Uniqueness of the optimal set

Many move orders play the same set; the question here is whether
different *sets* can be optimal.  Two statements must be kept apart:
the upper-half instance on its own, where the theory above settles
it, and the whole game, where only measurement does.

**Theorem 8 (the heaviest playable upper set is unique).**  Suppose
(★) holds in game n, as it does wherever (♦) holds (Theorem 7), so in
every game n ≤ 100,000.  Then every playable subset S of the
upper-half instance satisfies |S ∩ {≥ t}| ≤ |U\* ∩ {≥ t}| for every
t: listed in decreasing order, the i-th member of S is at most the
i-th member of U\*.  Consequently, for every strictly increasing
positive weighting w (the values themselves among them),
w(S) < w(U\*) unless S = U\*.

*Proof.*  Playability splits over the components of the incidence
graph: a member's factors are listed only by members of its own
component, so the part of a self-covering set in each component is
self-covering, and by Theorem 5 a set is playable iff its part in
every component is.  So descending greedy restricted to a component
K is descending greedy run on K, and U\* ∩ K ∩ {≥ t} is its set on
the truncation K_{≥t}.  S ∩ K ∩ {≥ t} is playable (a subset of S),
so (★) bounds its size by that set's; summing over the components
gives the count inequality.  (If S's i-th member s_i exceeded U\*'s
i-th, the threshold t = s_i would hold i members of S and at most
i − 1 of U\*.)  Now index the values of the members c₁ > c₂ > … > c_m
and put w(c_{m+1}) = 0.  Layer by layer,

    w(S) = Σ_j (w(c_j) − w(c_{j+1})) · |S ∩ {≥ c_j}|,

with every coefficient positive, so term by term w(S) ≤ w(U\*), and
equality needs |S ∩ {≥ c_j}| = |U\* ∩ {≥ c_j}| for every j.  The jump
of that count at c_j says whether c_j is a member, so then S = U\*.  ∎

This is the textbook fact that distinct weights give a matroid a
single heaviest basis, transplanted.  The playable sets are not a
matroid (n = 2873, above), but under (♦) they lie inside the GF(2)
linear matroid of Theorem 7, of which U\* is the greedy basis; the
count form needs only (★).  So the optimal upper set is unique in
every game to 100,000, where the brute force of
`evaluation/certify.py` had reached 62.

What Theorem 8 does not say is that every optimal *game* has upper
part U\*.  An optimal game's upper part is a playable upper set (a
subset of a playable set is playable), so it is either U\* or
strictly lighter above n/2; a game with a lighter upper part would
have to win the difference back below n/2, where the upper picks'
payments compete with the lower picks.  Whether that ever happens is
the strong form of what `evaluation/verify.py` measures, and it is
open; the enumerations below never find it.  For the members of U\*
with one or two prime factors it never happens (Theorem B, in "The
frame model: what is proven and what remains"), and the weaker
statement that some optimal game plays the rest of U\* is the one
the frame solver's answers rest on (U, there).

Two elementary facts tie a game's optimal sets to its predecessor's.

**Lemma (the prime of every optimal set).**  Every optimal set of a
game n ≥ 2 contains q, the largest prime ≤ n, and no other prime.

*Proof.*  A prime's only maximal factor is 1, so a playable set holds
at most one prime (Theorem 1: distinct payments).  q > n/2 (Bertrand)
divides no other number of the board, so it is nobody's maximal
factor.  Let S be playable without q, and S′ = S minus its prime, if
any.  S′ is playable and none of its members lists 1, so S′ ∪ {q} is
playable (q pays with 1; adding a number that is nobody's factor
removes nothing from anyone's list) and outweighs S, since any prime
in S is below q.  ∎

**Theorem 9 (optimal sets from the game before).**  Let p be the
largest prime below n.

* (At the ceiling.)  If opt(n) = n + opt(n−1), the most opt(n) can be
  (the *ceiling*), the optimal sets of n are exactly the playable sets
  T ∪ {n} with T an optimal set of n−1.  In particular opt(n) =
  n + opt(n−1) iff some optimal set of n−1 stays playable with n
  added.
* (Prime.)  If n is prime, opt(n) = n + opt(n−1) − p, and
  T ↦ T − p + n is a bijection from the optimal sets of n−1 onto
  those of n.

*Proof.*  A playable set of n without n is a playable set of n−1 (n
is nobody's factor), worth at most opt(n−1).  At the ceiling: an
optimal S of n is worth more than opt(n−1), so it holds n, and S − n,
playable as a subset, is worth opt(n−1): an optimal set of n−1.  Conversely a
playable T ∪ {n} with T optimal for n−1 is worth opt(n).  Prime: let
T be optimal for n−1; by the lemma it holds p and no other prime.  p
and n are nobody's maximal factor on either board (2p > n − 1, and
2p ≠ n), and both pay only with 1, so swapping one for the other
preserves playability: T − p + n is playable, and opt(n) ≥
opt(n−1) − p + n.  Conversely an optimal S of n holds n (it is worth
more than opt(n−1), as n > p), hence no other prime, and S − n + p is
a playable set of n−1, so opt(n) − n + p ≤ opt(n−1).  Equality holds
throughout, both maps are well defined between optimal sets, and they
are inverse to each other.  ∎

So the number of optimal sets is the same at a prime n as at n−1, can
only fall at a game at its ceiling, and can grow only at a composite n
below its ceiling.  For a solver this is reuse made exact: when the
optimal set of n−1 is unique, one peel of T ∪ {n} decides whether n
is at its ceiling (if not, opt(n) < n + opt(n−1)), and when it is,
the optimal set of n is known with no search.

**Remark (no exact matroid relaxation of the whole game).**  Game
425 has two optimal sets (they differ by {110, 132, 168} ↔
{98, 150, 162}; both replay by the rules).  So no matroid on
{2, …, n} in which every playable set is independent can have the
optimum as its heaviest independent set, values as weights: both
optimal sets would be heaviest independent sets, and distinct
positive weights allow only one (Theorem 8's layer-cake argument,
for any matroid).  The certificate that settles the upper half has
no analogue that is exact for the whole game.

What is measured (`rust/prover`, "Is the optimal set unique?"):
the prover enumerates every optimal set in 954 of the games 2..989
(the other 34, the most expensive, with the upper half fixed).
The optimal set is unique in every game n ≤ 424; 30 games between 425
and 779 have two or three, always with the same upper part, U\*, and
the same number of members, differing by an exchange of equally many
lower-half numbers of equal sum, each a maximal factor of a member of
U\*.  At 686, a game at its ceiling, the recorded optimal game of 685
cannot take 686, while another optimal set of 685 can: a step to the
ceiling needs all the optimal sets of n−1, not one.

## Longest games

Does an optimal game play as many moves as any game of the board
can?  The wiki's "Questions" page asks it (its third question), and
the Java's `reviseFreedoms` rests on a form of it (a comment in
`UpperBoundEstimator.java` states, as a theorem it has no proof of,
that the promotion set with the largest sum also has the most
promotions).  It is false.  Let M(n) be the most moves of a game of
n: a game's moves are its picks, so M(n) is the size of the largest
playable subset of {2, …, n}.  (A game that ends with no legal move
left can still be shorter than M(n): maximal playable sets can have
different sizes, as n = 2873 shows above.)

**Lemma (bounds on the most moves).**

* M(n) ≤ ν(n), the matching number of the board's maximal-factor
  graph (each number joined to its maximal factors).
* M(n−1) ≤ M(n) ≤ M(n−1) + 1, and M(n) = M(n−1) for a prime n ≥ 3.
* Given (★) in game n, a playable set has at most |U\*| members
  above n/2.

*Proof.*  Theorem 1 pays the picks of a playable set with distinct
maximal factors outside it: a matching whose pick side is the set.
A playable set of n−1 is one of n, and a playable set of n less the
number n is one of n−1 (n is nobody's factor).  For a prime n, a
playable set holds at most one prime (primes pay only with 1), and
one that holds n becomes, with n swapped for p, the largest prime
below n, a playable set of n−1 as large (p is nobody's maximal factor
there and pays with 1, as in Theorem 9).  The part above n/2 of a
playable set is a playable upper set, which Theorem 8's count
inequality at t = n/2 bounds by |U\*|.  ∎

So a legal game of ν(n) moves is a longest game, a certificate that
costs one matching.  ν(n) can overshoot M(n), since a matchable set
need not be playable (the n = 21 phenomenon above, counted in moves):
first at n = 51, where ν = 23 and M = 22.

**Proposition (an optimal game need not be a longest one).**  At
n = 833 = 7²·17 every optimal game has 362 moves, and a legal game
has 363.  The heaviest game of 363 moves scores 218,197, 29 less than
opt(833) = 218,226; it keeps the optimal game's upper part, U\*, and
differs from it below n/2 alone, by four of the optimal game's picks
for five lighter ones:

    optimal game:            {250, 252, 294, 400}         sum 1,196
    heaviest longest game:   {116, 180, 200, 320, 351}    sum 1,167

*Proof (computational, by the prover).*  opt(833) = 218,226, and the
optimal set is unique, with 362 members (`rust/prover`, sections 3
and 5).  The longer set replays by the rules, and ν(833) = 363, so
M(833) = 363.  The branch and bound of `rust/prover`, weighting the
pick of c by K + c with K above every sum, proves that no playable
set of 363 members sums to more than 218,197.  ∎

The mechanism is a trade of weight for count below n/2.  Every
payment is a maximal factor, so at most n/2; a pick above n/2 uses one
number of the lower half (its payment) and a pick below it uses two
(itself and its payment), so a playable S has
|S| + |S ∩ [1, n/2]| ≤ ⌊n/2⌋.  At 833 the optimal game's 48 lower
picks and 362 payments use 410 of the 416 lower numbers; the longest
game's 49 and 363 use 412.  The optimal game leaves two more lower
numbers idle and gives up a move for heavier lower picks: 1,196
against 1,167.

The same exchange is a promotion set of the frame model: the optimal
game promotes 48 numbers and the longer one 49, which is `max_total`,
so the "theorem" of the Java's comment is false for the whole frame
game of 833 (`rust/tests/soundness.rs`).  The Java's `reviseFreedoms`
applied it only to the two-frame game of frames 0 and 1, where it stayed
unproven; the frame solver had it as the tactic `revise_freedoms`, measured
that it no longer paid, and removed it.

What is measured (`rust/prover`, "Does an optimal game play the
most moves?"): to n = 1300 the recorded optimal game is a longest
game in every game but 833, 834 and 835, where it is one move short.
In 1,293 of the 1,299 games its moves meet ν(n), the certificate
above; at 51, 182 and 183 ν(n) overshoots by one and a search shows
that M(n) is the optimal game's.  So to 1300 an optimal game is at
most one move short of a longest one.  A search that pruned the
branches unable to reach M(n) moves would be unsound: at 833 it would
return 218,197.  Whether the gap is bounded in general is open.

## The parity matroids: a bound on the whole game

The span certificate's lemma (playable sets are independent) is about
the upper half, whose members are never anyone's factors.  It extends
to the whole board once each pick's own coordinate is taken out, and
the extension splits by the parity of Ω into two matroids, so that the
heaviest set independent in both is an upper bound on opt(n) that
polynomial algorithms compute.

Fix a board n and h = ⌊n/2⌋.  The coordinates are the numbers 1..h
(every maximal factor of a number up to n is at most n/2).  Each number
s of 2..n has the vector v(s) over GF(2) with a 1 at each of its
maximal factors, and each s ≤ h also the unit vector e_s.

**Lemma (playable sets with their own coordinates are independent).**
If S ⊆ {2, …, n} is playable, the family

    {v(s) : s ∈ S} ∪ {e_s : s ∈ S, s ≤ h}

is linearly independent over GF(2).

*Proof.*  Play S in a legal order x₁, …, x_k (Theorem 1 gives one).
The move xᵢ surrenders a proper divisor d still in the pot; lift it to
a maximal factor fᵢ = xᵢ/p with p a prime dividing xᵢ/d, so
d | fᵢ | xᵢ.  fᵢ is in the pot when xᵢ is played: whatever removed it
earlier, playing it or taking it as tax for a multiple, would have
removed d with it.  Hence (a) fᵢ divides no earlier move xⱼ (j < i),
which would have taken it, so it is no maximal factor of one; and
(b) fᵢ ∉ S: it is not xᵢ (a proper divisor), not an earlier move (it
is still in the pot), and not a later one (xᵢ takes it).  Let u(s) be
v(s) with the coordinates in S deleted.  A nonempty vanishing sum of
some u(xᵢ) is impossible: for the largest i in it, coordinate fᵢ
survives the deletion by (b), is 1 in u(xᵢ), and is 0 in every other
term by (a).  So the u(s), s ∈ S, are independent.  Now let
Σ_{s∈A} v(s) + Σ_{s∈B} e_s = 0 with A ⊆ S and B ⊆ S ∩ [2, h].
Deleting the coordinates in S removes every e_s and leaves
Σ_{s∈A} u(s) = 0, so A = ∅, and then Σ_{s∈B} e_s = 0 gives B = ∅.  ∎

For S in the upper half no coordinate lies in S and no e_s appears: the
lemma is then the span certificate's (a dismantling order is a play
order read backwards).  It is also stronger than Theorem 1's matching:
independent u(s) have a nonzero maximal minor, and a nonzero term of
its expansion matches each s ∈ S to a distinct maximal factor outside
S, which is the Franklín–Moniot relaxation; a matchable set need not be
independent.  Independence sits exactly between matchability and
playability (an odd number of payment schemes onto some payment set,
against at least one and exactly one): Theorem 12, in "Relation to
Franklín and Moniot", which also derives the lemma from their Theorem 1.

**Theorem 10 (the parity matroids).**  Split the coordinates by the
parity of Ω (1 counts as even).  For q ∈ {0, 1} let M_q be the linear
matroid on {2, …, n} in which s is represented by

* v(s), if Ω(s) and q have different parities;
* e_s, if they have the same parity and s ≤ h;
* a coloop, if they have the same parity and s > h.

Every playable set is independent in M₀ and in M₁.  Hence opt(n) is at
most the heaviest weight Σ_{s∈I} s of a set I independent in both.
More generally, for forced picks P (the numbers a node of a search
fixes in) and barred numbers X (those it fixes out), every playable
S with P ⊆ S and S ∩ X = ∅ weighs at most w(P) plus the heaviest
common independent set of M₀/P ∖ X and M₁/P ∖ X (and there is none
unless P is independent in both).

*Proof.*  A maximal factor f of s has Ω(f) = Ω(s) − 1, so v(s) lies in
the coordinates whose parity differs from Ω(s)'s and e_s in those of
Ω(s)'s own: every vector of the lemma's family lies in one of the two
coordinate subspaces E₀ (Ω even) and E₁ (Ω odd).  Since GF(2)^h is
their direct sum, such a family is independent iff its members in E₀
are and its members in E₁ are.  For a playable S, the members in E_q
are the vectors representing in M_q the elements of S that are not
coloops of M_q, so they are independent, and adding coloops keeps a set
independent: S is independent in M_q.  opt(n) is the weight of a
heaviest playable set.  At a node, a playable S ⊇ P is independent in
M_q, so P is, and S ∖ P is independent in the contraction M_q/P; it
avoids X.  ∎

Three facts make it usable as evidence.

* **It is computable.**  The heaviest common independent set of two
  matroids is a polynomial problem (weighted matroid intersection;
  Lawler, Frank), and over GF(2) with these sparse vectors a solve
  takes milliseconds: 22 ms at n = 990 and 43 ms at 1300 from scratch.
* **Its value can be certified without trusting the algorithm.**  For
  any split w = w₀ + w₁, every T independent in both matroids has
  w(T) = w₀(T) + w₁(T) ≤ max_{M₀} w₀ + max_{M₁} w₁ (weak duality).
  And in one matroid an independent I is a heaviest independent set,
  over all sizes and for weights of either sign, iff (1) w(x) ≥ 0 for
  x ∈ I, (2) w(y) ≤ 0 whenever I + y is independent, and (3)
  w(y) ≤ w(x) whenever x lies in the circuit of I + y.  (Add r dummy
  coloops of weight 0 and truncate to rank r: the independent sets,
  padded with dummies, become the bases, and (1)–(3) are the classical
  criterion that no single exchange improves a basis.)  So a common
  independent I with a split satisfying (1)–(3) in both matroids proves
  that w(I) is the maximum, whatever produced it; the same split bounds
  the maximum with one member x of I barred, since the heaviest
  independent set avoiding x is I − x plus the heaviest y with
  I − x + y independent, if that weighs more than 0.
* **It is never above the Franklín–Moniot bound** (by the minor argument
  above).  It is not one matroid, so the remark that no matroid
  relaxation is exact for the whole game (game 425) does not touch it.

What it misses are the *odd cores*: unplayable sets whose vectors, with
their own coordinates taken out, are still independent; these are the
self-covering sets no subset of which lists every factor an even number
of times.  Over another field it would miss more: over GF(3) or the
rationals the unsigned vectors of a triangle of moves are independent.
The same construction, frame by frame, bounds the frame model's
promotions (Theorem 11, below), and the two agree at every root
measured ([rust/docs/EXPERIMENTS.md, "Matroid intersection
bounds"](../rust/docs/EXPERIMENTS.md#matroid-intersection-bounds-2026-10)).
What the bound buys the prover's search is in the evidence table
below.

## The frame matroids: a bound on the promotions

Theorem 10's lemma, applied frame by frame, bounds the frame model's
promotions in the same way: two matroids, one of the even frames and
one of the odd frames, in both of which every promotable set is
independent.  This is the solver's GF(2) bound (tactic `gf2`;
rust/DESIGN.md, section 4, "The GF(2) bound").

**Theorem 11 (the frame matroids).**  Give each frame h of a state
the coordinates of its factors, each mover u paying in h (a move of
h, or a number of home h + 1 promoted into it) the vector v_h(u) of
its maximal factors among them, and each factor y of h the unit vector
e_y.  If a set Q of candidates can be promoted, then in every frame h
the vectors of its moves, the v_h(x) of Q's members promoted into h
and the e_y of Q's members promoted out of h are linearly independent.
So Q is independent in the matroid of each frame on the candidates
that touch it (the frame's moves contracted); a candidate of home h
touches frames h and h − 1, one even and one odd, so the even frames'
matroids are the blocks of a matroid A on the candidates and the odd
frames' of a matroid B, and Q is independent in both.  The heaviest
common independent set of A and B bounds the best promotion set.  At
a node reached by promoting P, a completion F makes P ∪ F promotable
from the root, so F is independent in A and B contracted by P: the
bound of the node is the heaviest common independent set of the
contractions.

*Proof.*  Promoting Q succeeds exactly when every frame is winnable
with all of Q applied at once, its moves plus Q's members promoted
into it and its factors less Q's members promoted out of it
(rust/DESIGN.md, section 4; `model::promote`).  A winnable frame
peels completely (`model::peel`): each move dropped has a live factor
that no move still there has, so, as in the proof of Theorem 10's
lemma (the peel's order read backwards is the play order there), the
movers' vectors restricted to the live factors are independent, which
is independence together with the unit vectors of the factors taken
out.  The moves of a frame are fixed, so the frame's condition is
independence of Q's part in the contraction by them; frames have
disjoint coordinates, and the even and odd frames' contractions are
the blocks of A and B.  At a node, F completes P exactly when P ∪ F is
promotable from the root, so P ∪ F is independent in A and in B, and F
is independent in their contractions by P (which P, independent, makes
well defined).  ∎

Three facts about it.  It is never above the matching bound of
rust/DESIGN.md, by Theorem 10's minor argument frame by frame: an
assignment of the movers to distinct live factors in which each unit
vector e_y takes y itself, so that no promoted number pays as a
factor, is the matching bound's relaxation.  On movers with at most
two live factors (graph edges, with a ground vertex for one)
independence over GF(2) is being a forest, which is winnability
itself, so what it misses are, as for Theorem 10, the odd cores of
movers with three or more factors (the frames' residuals at the roots
of 936-1300 hold 18-65 movers, and Lorea's matroid misses them too).  And
Theorem 10's bound equals in_the_bag plus Theorem 11's at every root
of 880-1300; its gap to opt(n) is a tenth of the Franklín–Moniot
bound's to 1300.  The measurements are in
rust/docs/EXPERIMENTS.md ("Matroid intersection bounds", "The GF(2)
bound in the search"); the search's forced members rest on weak
duality for a weight split, as Theorem 10's certificate does
(`gf2::meet`).

## Prime exchanges: symmetries of the promotions

Exchanging two primes in every number is an automorphism of
divisibility.  Where it maps the frame model's root onto itself, it
maps promotion sets to promotion sets, and no optimal set is improved
by it, which orders the two primes' shares in every optimal set at
once.  This is the solver's tactic `symmetry` (rust/DESIGN.md,
section 4, "Prime exchanges"; rust/docs/EXPERIMENTS.md, "Dominance
and symmetry", found the exchanges and measured them).

For primes p < q let π = π_{p,q} send p^a q^b m (m prime to pq) to
p^b q^a m.  For a set S of numbers and a prime r let
c_r(S) = Σ { x/r : x ∈ S, r | x }, the sum of S's cofactors of r.

**Theorem 13 (prime exchanges).**  Suppose that π maps the moves of
every frame of the root onto themselves, the factors of every frame
onto themselves, and the candidates onto themselves.  Then:

1. a set S of candidates is promotable exactly when π(S) is;
2. Σ π(S) − Σ S = Σ_{x∈S} (π(x) − x), which is
   (q − p)(c_p(S) − c_q(S)) when no candidate is divisible by pq, p²
   or q²;
3. every heaviest promotion set S\* has c_p(S\*) ≤ c_q(S\*), for all
   the pairs (p, q) that meet these conditions at once.

*Proof.*  (1) π is multiplicative and a bijection of the positive
integers that sends primes to primes, so y = x/r with r prime exactly
when π(y) = π(x)/π(r): π preserves the maximal-factor relation.
Promoting S succeeds exactly when every member of S is a factor of its
home frame h ≥ 1 and every frame is winnable with all of S applied at
once, its moves plus S's members promoted into it and its factors less
S's members promoted out of it (rust/src/model/promote.rs, from the
peel); a promotion set also leaves no frame with negative freedoms and
has at most max_total members.  π maps each frame's factors onto
themselves, so π(x) has the home frame of x, and π maps frame h with S
applied onto frame h with π(S) applied: moves to moves, factors to
factors, and each mover's maximal factors in the frame to its image's.
The two are the same game of Taxman Mini, winnable together; the
freedoms a frame loses are the members promoted into and out of it,
counts that π keeps, as it keeps the size of S.  π is an involution,
so the converse is the same argument for π(S).
(2) The first form is the definition of the sums.  Under the
condition, a member x of S is divisible by at most one of p and q, and
by it once: π(x) = xq/p, a gain of (q − p)·x/p, if p | x; π(x) = xp/q,
a loss of (q − p)·x/q, if q | x; and π(x) = x otherwise.
(3) π(S\*) is a promotion set by (1), so Σ π(S\*) ≤ Σ S\*, which by
(2) is c_p(S\*) ≤ c_q(S\*).  The argument for one pair uses no other,
so every heaviest set keeps every pair's inequality.  ∎

Within a class of primes any two of which are such a pair (the
exchanges compose: π_{p,r} = π_{p,q} π_{q,r} π_{p,q}), (3) says that
every optimal set gives the larger primes the larger cofactor sums,
the order in which the rearrangement inequality puts the class's
arrangements at their heaviest.  No sorting argument is needed,
though, nor any condition across classes: a set that breaks one
pair's inequality is beaten by its image under that pair's exchange
alone, whatever its members' other primes.

*Pruning with the inequalities.*  A branch and bound for a heaviest
promotion set may discard every node whose completions F all leave
P ∪ F (P the node's promotions) breaking some pair's inequality.  If
the incumbent is not optimal, every heaviest set beats it and keeps
every inequality, so such a node holds none of them; and the other
prunings (bounds above every completion, members that every
completion beating the incumbent promotes, sets that cannot be
promoted) hold for the completions that keep the inequalities as for
all completions.  With C the node's candidates, every completion has
c_p(P ∪ F) ≥ c_p(P) and c_q(P ∪ F) ≤ c_q(P) + c_q(C).  So a node with
c_p(P) > c_q(P) + c_q(C) has no completion that keeps the pair's
inequality (it is *hopeless*), and a candidate x = mp with
c_p(P) + m > c_q(P) + c_q(C) is in none.  And for any μ ≥ 0, a
completion that keeps the inequality has

    Σ F ≤ Σ F + μ (c_q(P ∪ F) − c_p(P ∪ F))
        = Σ_{x∈F} w_μ(x) + μ (c_q(P) − c_p(P)),

with w_μ(mp) = m(p − μ), w_μ(mq) = m(q + μ) and w_μ(x) = x for the
other candidates.  By Theorem 11, F is common independent in the
node's contracted matroids, so their heaviest common independent set
under w_μ (of members of positive weight, and no more than the node
can still promote), plus μ (c_q(P) − c_p(P)), bounds every such
completion: a Lagrangian relaxation of the inequality (weak duality,
as for Theorem 11's weight split).  The solver uses the first two;
the third was measured and not kept (rust/docs/EXPERIMENTS.md, "Prime
exchanges in the search").

These hold for the search of the root's problem, whose subtrees all
are; a search of one node's completions for their own best (as some
tests run) finds the best of those that keep the inequalities.

## The frame model: what is proven and what remains

The frame solver (`rust/`) does not search the games of n.  It
searches the *promotion sets* of the frame model, and its answer is
in_the_bag plus the heaviest of them (rust/README.md, section 1, and
its glossary define the words used here: sources, frames, `unzip`
and its mandatory moves, impossible sources, freedoms, closed frames,
the numbering, candidates, promotion sets, `max_total`).  The model
was built on five assumptions that were checked against data and
never proven: setup's greedy removal of impossible sources, the
mandatory moves of `unzip`, freedoms = factors − sources, closed
frames taking no part, and a promoted number paying only in the frame
one forward.  This section replaces all five with theorems, which
leave two statements:

* **N (clean numbering)**, a fact about a board, which holds on every
  board the solver supports (to 8191) and which setup checks on every
  board it builds;
* **U (the upper part)**, a statement about a game, and open: *some
  optimal set of n plays every source with three or more prime
  factors that setup keeps.*

Wherever N holds, the frame solver's answer for a game whose proof
rests on its search (`exhausted`, `trivial`, `same_search`) is opt(n)
exactly when U holds at n (the corollary at the end).  The games
proven at their ceiling or by the prime formula never needed the model.

*Words.*  The *layer* of a number x is Ω(x), its number of prime
factors counted with multiplicity: layer 0 is {1}, layer 1 the primes,
layer 2 the products pq and the squares p², and so on.  A maximal
factor x/p of x lies one layer below x.  The *sources* are the numbers
above n/2 (nobody's factors).  U\* is the set of sources setup keeps:
descending greedy on each frame, which is the U\* of "The open
problem" (M1, below).  For a set S, S_k is its part in layer k; so
U\*_k is the part of U\* in layer k, and U\*_{≥3} its part in the
layers from 3 up.

### Frames are layers

**Lemma Ω (layers).**  The frames as setup forms them, before any is
merged, are the components of the graph that joins each source to its
maximal factors.  In each frame every source has one Ω, k, and every
factor has Ω = k − 1: the frame *lies in layer k*.  Every number up to
n/2 is a factor of some frame.  A frame in front of a frame of layer k
(one that holds a maximal factor of one of its numbers) lies in layer
k − 1.

*Proof.*  An edge joins a source s to s/p, one layer lower.  A path
between two sources alternates source, factor, source, so its layers
go k, k − 1, k, …: the sources of a component share a layer and its
factors lie one below.  A number x ≤ n/2 has a prime p with n/2 < xp
≤ n (Bertrand), and xp is a source with the maximal factor x.  The
maximal factors of a source of a frame are factors of that frame
itself.  A maximal factor y of a factor of a frame of layer k lies in
layer k − 2 and is at most n/2, so it is a factor of some frame, whose
factors lie one layer below its sources: that frame lies in layer
k − 1.  ∎

So no number has a proper divisor in its own frame, and a promoted
number never taxes the frame it came from.  The frames of a board are
the components of its layers: at n = 4000, layers 1 to 11, with one or
two frames each but five in layer 9.

### Playability splits by layer

For S ⊆ {2, …, n}, the *layer instance* of S at k is the reduction
instance whose members are S_k, whose pool P_k is the numbers of layer
k − 1 up to n/2 that are not in S (P_1 = {1}), and in which each member
lists its maximal factors in P_k.

**Theorem L (layer decomposition).**  S is playable iff every layer
instance of S is playable.

*Proof.*  By Theorem 1, S is playable iff its own reduction instance
is: members S, pool the maximal factors of the members that are not
in S, each member listing its maximal factors in the pool.  A maximal
factor of a member of layer k lies in layer k − 1 and is at most n/2,
so the member lists in S's instance exactly what it lists in the layer
instance at k, and a factor of layer k − 1 is listed by members of
layer k only.  So S's instance is the disjoint union of its layer
instances: no factor is listed in two of them.  On a disjoint union a
move of peel is a move within one part and changes no count of
another, so peel pairs every member of the union iff it pairs every
member of each part; and peel decides playability on both sides
(Theorems 2 and 3).  ∎

A game is thus a chain of layer games coupled only through the
numbers up to n/2 it plays: a played x of layer j is a member of layer
j and missing from the pool of layer j + 1.  The frame model is this
chain with the sources fixed to U\*, its frames the components of the
layers.  The parity matroids of Theorem 10 group the layers by parity.

### The bottom layers

**Theorem B (the bottom layers).**  Let q be the largest prime ≤ n and
S an optimal set of n ≥ 2.  Then S_1 = {q}; the numbers of S_2 above
n/2 are exactly U\*_2; and the numbers of S_2 up to n/2 keep U\*_2 a
forest, in the graph on the primes up to n/2 and a *ground* vertex in
which pq is the edge p–q and p² the edge p–ground.  So wherever the
frames of layer 2 are closed (U\*_2 spans every prime up to n/2), S
plays no number of layer 2 up to n/2.

*Proof.*  S_1 = {q} is the lemma of the prime of every optimal set
("Uniqueness of the optimal set").  So S holds no prime up to n/2, and
the pool of layer 2 is every prime up to n/2.  The layer instance of
layer 2 is separated (its members are not primes), and a member lists
its one or two primes, so by Theorem 5 a set of it is playable iff it
has no self-covering subset, that is no set of edges in which every
prime it touches is touched twice.  A forest has none (in a forest, a
tree with an edge has two leaves, and one of them is a prime touched
once), and a cycle is one: *a set of layer 2 is playable in S's game
iff it is a forest* (the two-factor proposition of "What the parity
bound keeps" is the same fact).  So the playable sets of layer 2 are the independent sets
of a graphic matroid, and setup's greedy on the sources of layer 2 is
Kruskal's algorithm: U\*_2 is the heaviest forest of sources.

Let W = S_2 ∪ U\*_2 and G the forest descending greedy picks from W.
The sources of W come before its other members, and among them greedy
keeps exactly U\*_2: setup's greedy kept each member of U\*_2 because
the sources above it do not span it, so neither do those of W above
it; and it rejected every other source because the members of U\*_2
above it span it, and these are in W and kept.  So
G = U\*_2 ∪ A, with A among S_2's numbers up to n/2.  In a matroid
greedy finds the heaviest independent set, so ΣG ≥ ΣS_2.  Replace S_2
by G in S: layer 2 stays playable; the pool of layer 3 only grows,
since A ⊆ S_2, and a larger pool keeps every payment scheme; the other
layers do not change.  By Theorem L the new set is playable, and it
weighs at least as much as S.  S is optimal, so ΣG = ΣS_2: S_2 is a
heaviest independent subset of W too, and with distinct positive
weights there is only one, so S_2 = G.  Where the frames of layer 2
are closed, U\*_2 has as many edges as there are primes up to n/2, so
it spans every one of them, and any further edge closes a cycle: A is
empty.  ∎

The frames of layer 2 have freedoms on 80 boards, all at most 186,
and are closed on every other board to 8191
(`rust/bench/structure/`).  Theorem B settles the members of U\* that
the barred parity bounds leave open in layers 1 and 2: the largest
prime (open at 1000 and 1300) and the rest of those (with the prime,
13 of the 17 open members at 600, 24 of 36 at 1000, 15 of 18 at 1300
and 92 of 194 at 2175).

### Clean numbering

Setup numbers the frames that have freedoms breadth first from the
front: number 0 for those with no frame with freedoms in front of
them, h + 1 for those directly behind a frame numbered h and not yet
numbered.

**N (clean numbering), on the board n.**  Every frame with freedoms
directly in front of a frame numbered h is numbered h − 1.

N is a property of one board, which a program decides in milliseconds.
By Lemma Ω, one frame forward is one layer down, so N says that the
numbering is by layer: frame h is the frames with freedoms of layer
ℓ₀ + h.  The census of every board to 8191, the largest the solver
supports (`rust/examples/structure.rs`; the data in
`rust/bench/structure/`), finds N on every one, with ℓ₀ = 3 on 8,100
boards and ℓ₀ = 2 on the 80 where the frames of layer 2 have freedoms;
and setup refuses a board where N fails (`frames::build_frames`).  On
a board with holes, which only the sacrifice study builds, N can fail
(56 without 39).

### The frame model is exact, given its upper part

What setup does, in the words of the code (`rust/src/frames/`):

1. It forms the frames and runs `removeImpossible` on each: descending
   greedy over the frame's sources against its factors, peel deciding
   (Theorems 2 and 3).  Then, round after round until nothing changes,
   `unzip` and `removeImpossible` again on the frames' *remaining*
   sources and factors (those not in a mandatory pair).
2. `unzip` settles mandatory pairs (s, f), a source and a factor, each
   a *claim* of f by s.  A *back* claim: f is wanted by a single
   remaining source s, f cannot go forward (no frame holding a maximal
   factor of f could take f on as a source), and f has no outside move
   (no maximal composite c of f, in another frame with freedoms, could
   be taken on as a source by f's frame).  A *front* claim: s has a
   single unclaimed factor f, and f has no outside move.
3. It numbers the frames with freedoms and merges the frames of one
   number.  The candidates of number h are collected from its frames
   one by one, before they are merged and after the frames of number
   h − 1 have been: a remaining factor x of frame A is a candidate if
   (a) A's remaining sources stay playable against its remaining
   factors less x, and (b) some frame holding a maximal factor of x
   could take x on as a source.
4. After the last merge it simplifies every frame again (a merged
   frame starts with nothing claimed).
5. The search's frame h is the merged frame h's remaining sources and
   factors, *reduced*: a move left with one factor is paid by it, and
   the pair dropped.  A number's *home* is the frame whose factors hold
   it then.  A promotion set L is a set of candidates, each a factor of
   a home h ≥ 1, such that every frame is winnable with L applied (its
   moves plus the members of L of home h + 1, its factors less the
   members of home h), with at most `max_total` members.

For a set L of numbers up to n/2 and a number h, the *instance*
I_h(L) has as members U\* ∩ frame h and the members of L that are
factors of frames numbered h + 1; its pool is the factors of frame h
(as merged) that are not in L; each member lists its maximal factors
in the pool.  A *front move* removes from an instance a member that
lists one live factor, with that factor; a *back move* a factor listed
by one live member, with that member.  Both keep a playable instance
playable (Theorem 3), and a sequence of them that pairs every member
is an acyclic payment scheme (Theorem 2).  The lemmas tie each step of
setup to these instances; for the soundness half, recall from the
proof of Theorem 2 that a cycle of the precedence of any scheme that
pays with maximal factors is *flat*: its members lie in one layer and
on each edge a → b, pay(a) is a maximal factor of b.

**M1 (the kept sources are U\*).**  The first `removeImpossible` is
descending greedy on each frame, so it keeps U\* ∩ frame.  The later
passes keep them all: the pairs of `unzip` are front and back moves of
the instance (U\* ∩ frame, its factors), by M5's argument with L
empty, so the remaining sources stay playable against the remaining
factors, and greedy keeps every member of a playable set.  Merged
frames share no factor, so their kept sources stay playable together.
So the game with no promotions plays exactly U\*, and in_the_bag =
ΣU\*.

**M2 (closed frames are inert).**  Let C be a closed frame, with as
many factors as kept sources.  In a payment scheme of a playable
S ⊇ U\*, the sources of C pay with distinct factors of C (all their
maximal factors are in C), so every factor of C pays a source of C:
none is in S, and none pays for anything else.

**M3 (one frame forward; this is where N is needed).**  Let S = U\* ∪ L
be playable with a payment scheme, and x ∈ L a factor of the frame A.
By M2, A is not closed, so it has a number h.  x pays with one of its
maximal factors, which lies in a frame X in front of A (Lemma Ω: not
in A), and X is not closed (M2).  By N, X has the number h − 1: so
h ≥ 1, and x pays in the merged frame h − 1.  In the scheme, every
member of L pays in the frame the model promotes it into, and every
source in its own frame.

**M4 (the candidate filter is necessary).**  With S and x as in M3, x
passes both tests of step 3.  (b) Restrict the scheme to the sources
of frame h − 1 and x: they are playable against its factors, so frame
h − 1, merged by the time x's candidacy is decided (a merged frame can
take x on iff one of its parts can, as they share nothing), can take x
on.  (a) Restrict the scheme to U\* ∩ A: these sources pay with factors
of A other than x.  The claims made in A before the merge are front
and back moves of that instance without x: a front claim of x would
leave its source nothing; a back claim of x needs x unable to go
forward, but the frame where x pays could take it on (restrict the
scheme to that frame's sources and x, as in (b)).  So the residual,
A's remaining sources against its remaining factors less x, is
playable.

**M5 (the claims are moves of the instances).**  Let L ⊆ candidates,
and either U\* ∪ L playable (then I_h(L) is playable: restrict the
scheme, by M3) or L a promotion set (then no claimed factor is in L: a
claimed factor is in no frame of the root, so it has no home).  The
claims of a numbered frame h, in the order made, are front and back
moves of I_h(L), and no claimed factor is in L.

*Front claim (s, f).*  f is s's only unclaimed factor, so after the
earlier pairs s lists f alone in I_h(L), unless f ∈ L.  In the first
case f ∈ L would leave s nothing in a residual that the earlier moves
kept playable.

*Back claim (s, f).*  Only s among the remaining sources wants f; what
matters is that **no candidate lists f**.  Suppose a candidate c had f
as a maximal factor.  c is a factor of a numbered frame Y, and f's
frame lies in front of Y, so by N, Y has the number h + 1.  When c
passed test (b), the frames that could have taken it were the frames
in front of Y: a closed one cannot take a source on, and those with
freedoms all have the number h and had been merged into one frame,
with the sources and factors that frame h had when the claim was made.
The claim says that frame could not take c on (f had no outside
move), so c failed (b): a contradiction.  So in I_h(L), f is listed by
s alone; and f ∉ L (in the first case f would pay in a frame in front,
by M3, which could then take it on, so f could go forward).  It is a
back move.

Two quirks of the code, kept for parity with the Java, can only make
`unzip` claim less, and the argument covers whatever it claims: a
frame not yet simplified counts as having freedoms, and a composite
left in a frame that has been merged away counts as an outside move.

**M6 (the reduction).**  With L as in M5, a move left with one live
factor in a frame of the root is a front move of I_h(L) after the
claims, and its factor is not in L, for the reasons M5 gives for a
front claim (a factor reduced away is in no frame of the root either).
And reducing never changes whether a frame is winnable, before or
after promotions (the reduction lemma, `rust/src/model/peel.rs`).

**Theorem M (exactness).**  Assume N at n, and let Π_n be the frame
solver's root problem (its frames, candidates and `max_total`).  For
every L ⊆ {2, …, n/2}: U\* ∪ L is playable iff L is a promotion set
of Π_n, and then U\* ∪ L weighs in_the_bag + ΣL.

*Proof.*  (Only if.)  Let U\* ∪ L be playable.  By M2 and M3, every
member of L is a factor of a numbered frame h ≥ 1 and pays in frame
h − 1; by M4, L ⊆ candidates.  Each I_h(L) is playable (restrict the
scheme), so by M5 and M6 its claims and reductions are front and back
moves, and no member of L is claimed or reduced away: each keeps its
home.  By Theorem 3 what is left, which is frame h of the search with
L applied, is playable, that is, winnable.  So L can be promoted from
the root, and |L| ≤ `max_total`, a count that winnability implies
(`rust/src/model/frame.rs`).  (If.)  Let L be a promotion set.  In
each numbered frame, the claims, the reductions and then a peel of
what is left with L applied pair every member of I_h(L); by M5 and M6
that is a sequence of front and back moves, an acyclic scheme
(Theorem 2).  Closed frames pay their sources inside.  So every member
of U\* ∪ L is paid, with distinct maximal factors outside the set.  A
cycle of the union's precedence is flat, and a flat edge a → b stays
in one frame: b moves in the frame of pay(a), as one of its sources
or, by N, as a member of L promoted into it.  The one exception is
pay(a) in a closed frame: then a is one of that frame's sources (a
closed frame pays nothing else), and no edge enters a closed frame
from outside (an edge into a source of it comes from a member paying
in it).  So a cycle lies in one frame's scheme, which is acyclic: U\*
∪ L is playable (Theorem 1), and it weighs ΣU\* + ΣL = in_the_bag +
ΣL (M1).  ∎

*Why the candidate filter matters.*  Without test (b) a back claim
would hide a cycle.  At n = 98, the source 77 = 7·11 claims 11 (back);
22 = 2·11 is a factor of frame 1, and frame 0 holds the six sources
57, 62, 65, 91, 93 and 95, a path on the factors 2, 31, 3, 19, 5, 13,
7.  With 11 claimed, the model would take 22 into frame 0 with the
factor 2, and the frame would stay winnable; but on the board 22 and
77 close the path into a cycle of eight numbers on eight factors, each
listed twice, so U\* ∪ {22} is not playable.  22 is no candidate,
because frame 0 cannot take 22 on: only the filter keeps it out.  The
same happens on 27 boards to 2000, all in 98–120 and 182–186
(`rust/tests/model_exact.rs`).

### The upper part

**U, at n.**  Some optimal set of n contains U\*_{≥3}.

By Theorems B and M this is all that stands between the frame model
and opt(n):

**Corollary (what the frame solver's answer is).**  If N holds at n,
then in_the_bag + P\*(Π_n), the heaviest promotion set's weight added
to in_the_bag, is opt(n) if U holds at n, and less than opt(n) if not.

*Proof.*  By Theorem M, in_the_bag + P\*(Π_n) is the weight of the
heaviest playable set containing U\* (every playable set containing
U\* is U\* ∪ L with L up to n/2, since U\* is a maximal playable set of
sources: greedy kept every source it could).  If U holds, take an
optimal set S ⊇ U\*_{≥3}; by Theorem B it holds U\*_1 and U\*_2 as
well, so S ⊇ U\*, and opt(n) = ΣS ≤ in_the_bag + P\*(Π_n) ≤ opt(n).  If
U fails, no optimal set contains U\*, so every playable set containing
it is lighter than opt(n).  ∎

So a result proven `exhausted` or `trivial` is opt(n) given U at n
(and the code), and so is a repeat (`same_search`) of a game m (when m
was proven at its ceiling, Theorem M's soundness at m makes that
ceiling m's heaviest promotion set).  A result proven at the ceiling
or by the prime formula rests on the game before it, not on the model.

*What is known of U.*

* It holds wherever the prover has proven the optimum on the whole
  board, every n ≤ 2175: the recorded game plays U\* and is optimal.
  In its strong form, every optimal set contains U\*, it holds in
  every game whose optimal sets were enumerated (954 games of 2..989;
  `rust/tests/bottom_layers.rs` checks Theorem B on all of them).
* U\*_{≥3} is nearly all the sources of three or more prime factors:
  greedy rejects none of layer 4 or more on any board to 8191, and in
  layer 3 none before 741 and 10 of 523 at 4000 (the census).  So U
  says: some optimal game plays every source of four or more prime
  factors, and every source of layer 3 that greedy keeps.
* The barred parity bounds (Theorem 10, with layers 1 and 2 fixed by
  Theorem B and one member u of U\*_{≥3} barred) certify most members of
  U\*_{≥3} game by game: when the bound is at most the recorded score V,
  every set beating V that obeys Theorem B plays u.  Every game leaves
  some open: over the 490 games of 1301..2175 that rest on the frame
  model, 7.5% of the members (1 to 102 a game, 84% to 99.7% certified),
  and over the 635 of the EC2 chain's 2176..3314, 14.9% (15 to 317 a
  game, 54% to 98% certified), the open share growing with the root's
  gap by 2 to 3 points per 100.  The root's heaviest set plays all of
  U\*_{≥3} in every one of these games.  (`rust/prover`, `bin/upper.rs`;
  the data in `rust/prover/data/upper_*.jsonl` and the table in its
  README, section 3.)

*Where a proof stops.*  Take an optimal S without U\*_{≥3}, and d the
largest source where S and U\* disagree, in layer k ≥ 3.  If d ∈ S ∖ U\*,
S holds d and the members of U\*_k above it, which greedy found
unplayable even with all their maximal factors in the pool; S's pool
is smaller: a contradiction.  So d ∈ U\* ∖ S, and S + d is unplayable in
layer k alone (Theorem L; d is nobody's factor).  The layer instance
is separated, so Theorems 4-6 and the victim law hold in it, and every
core of S_k + d holds d.  If one member v < d of layer k lies in every
minimal core, S − v + d is playable and heavier (a v up to n/2 returns
to the pool of layer k + 1, which only helps); so is S − y + d if
giving up one number y of layer k − 1 that S plays (returning y to the
pool of layer k) breaks every core.  So a counterexample needs d in two
or more minimal cores of its layer instance with no common victim
below d: the n = 9170 phenomenon of the frontier argument, now inside a
layer and with numbers up to n/2 among the members that could be given
up.  Where the cores are graph-like (members with at most two live
factors) the argument closes, as in layer 2; the gap is the odd cores,
whose members have three distinct primes.

## Relation to Franklín and Moniot

A. F. Franklín and R. K. Moniot, "The difficulty of beating the
Taxman", *Discrete Applied Mathematics* 339 (2023) 166–171
(arXiv:2211.00461; theorem numbers below are the arXiv version's), is
the published foundation of the README's yardsticks.  This section
restates its objects in this file's notation, shows that its central
theorem is Theorem 1, and places the parity bound (Theorem 10 and its
lemma, Theorem 7's lemma, Theorem 11 frame by frame) exactly between
their characterization and their bound.  The paper uses no linear
algebra: everything below about GF(2) is this file's.

### What the paper proves

They play Taxman on any finite poset graded by a rank function, with
weights: a move takes an element that still has something below it in
play, scores its weight, and removes it and everything below it.
Taxman is the divisibility order on 1..n, ranked by Ω and weighted by
value.  Their graph G joins two elements when one covers the other;
for divisibility, when one is a maximal factor of the other.  So G is
the maximal-factor graph, with the edge f–c weighing c.  A cycle of G
is *alternating* for a matching M when its edges are alternately in M
and not, and *flat* when its vertices lie in two consecutive ranks.

* Theorem 1: optimal play is a maximum-weight matching of G with no
  flat alternating cycle.  The proof works set by set in both
  directions.  From a play, it matches each pick with the largest
  number its move taxes, which is a maximal factor of the pick.  From
  such a matching, it builds a play of exactly its upper ends.  So
  what it proves is: *S is playable iff S is the set of upper ends of
  a matching of G with no flat alternating cycle.*
* Theorem 2: the game on a graded poset is NP-hard, even with unit
  weights.  On two ranks every cycle is flat, and the game becomes
  the maximum alternating-cycle-free matching of a bipartite graph,
  NP-complete by H. Müller ("Alternating cycle-free matchings",
  *Order* 7 (1990) 11–21).
* Theorem 3: the born-free matching has no flat alternating cycle.
  Theorem 4: restricted to the primes up to 5, it takes more than half
  the pot for N ≥ 847.  Theorem 5: the game can be won for every
  N ∉ {1, 3}, by the born-free matching except at 7 and 13, which get
  games by hand.  Theorem 6: a matching with no flat alternating cycle
  is put in order in O(N log N) by removing degree-one vertices, rank
  pair by rank pair: peel's forced moves, on a matching already found.
* The bounds (unnumbered): without the flat-cycle condition, the
  maximum-weight matching is an upper bound (`evaluation/bound.py`);
  breaking its cycles with a feedback-arc-set heuristic gives a lower
  one.  The paper has no sharper bound: Theorem 10's is never above
  the matching (its lemma's minor argument).

Notation for the rest of the section.  For a pick set S ⊆ {2, …, n},
F_S is the set of maximal factors of S's members that are not in S.
A *payment scheme* of S pays each member with a distinct maximal
factor of its own from F_S.  It is exactly a matching of G whose upper
ends are S (no member can be a lower end): Theorem 1's matching
without its precedence condition.  For T ⊆ F_S with |T| = |S|,
per(S, T) is the number of payment schemes whose payment set is T.  It
is the permanent of A_S[S, T], where A_S has a row per member, a
column per factor of F_S, and a 1 where the factor is a maximal factor
of the member.  The rows of A_S are the u(s) of Theorem 10's lemma:
v(s) with the coordinates in S deleted.

### Their Theorem 1 is Theorem 1

**Lemma (the bridge).**  Let M be a payment scheme of S with payment
set T.  The following are equivalent: (a) M has a flat alternating
cycle in G; (b) the precedence of Theorem 1 is cyclic under M;
(c) another payment scheme of S has the payment set T.

*Proof.*  An alternating cycle has its matched edge at every vertex,
so its vertices are members and their payments.  (a) ⇒ (b): in a flat
one the members are the upper rank, matched down, and each unmatched
edge joins pay(a) to a member b of which it is a maximal factor, a
precedence edge a → b.  (b) ⇒ (a) is the potential argument of
Theorem 2's proof, which needs only maximal-factor payments.  On a
precedence edge a → b, pay(a) is a proper divisor of b (when a | b,
because pay(a) | a), so Ω(pay(a)) ≤ Ω(b) − 1 = Ω(pay(b)), with
equality iff pay(a) ∈ mf(b).  Around a cycle equality holds
throughout.  So a simple cycle a₁ → ⋯ → a_m gives the cycle
a₁, pay(a₁), a₂, …, a_m, pay(a_m) of G: alternating, with all its
members at one rank and all its payments one below, flat.
(a) ⇒ (c): swapping the matched and unmatched edges of the cycle
gives another scheme with the same payments.  (c) ⇒ (a): the two
schemes differ on disjoint cycles that alternate for M, and each is
flat: an unmatched edge joins a payment pay(a) to a member b with
pay(a) ∈ mf(b), so Ω(b) = Ω(pay(a)) + 1 = Ω(a).  ∎

So Theorem 1 and their Theorem 1, read set by set, are the same
statement about the same objects.  Both directions of Theorem 1's
proof are in theirs.  Their play-to-matching direction is the lifting
argument: they take the largest number a move taxes, Theorem 1 any
maximal factor above the surrendered divisor.  Their matching-to-play
direction repeatedly plays a member with no other payment below it,
which is a topological order of the precedence.  Their rank count is
the first half of Theorem 2's proof.  Theirs is the more general
statement (every finite graded poset).  Theorem 1's form adds that
every topological order is a legal game.  Their proof also shows the
precedence's second clause is redundant: a | b implies pay(a) | b.

### One count, three conditions

**Theorem 12 (one count, three conditions).**  For S ⊆ {2, …, n}:

1. S is matchable (it has a payment scheme, the condition behind the
   Franklín–Moniot bound) iff per(S, T) ≥ 1 for some T;
2. the family of Theorem 10's lemma (v(s) for s ∈ S, e_s for s ∈ S
   with s ≤ n/2) is independent over GF(2) iff per(S, T) is odd for
   some T;
3. S is playable iff per(S, T) = 1 for some T.

So playable ⇒ independent ⇒ matchable.  opt(n), the parity bound of
Theorem 10 and the Franklín–Moniot bound are the weights of the
heaviest subsets of {2, …, n} meeting the three conditions in turn.

*Proof.*  (1) is the definition.  (2) The family is independent iff
the rows of A_S are (the last step of the lemma's proof, read both
ways), iff some square A_S[S, T] is nonsingular over GF(2).  Over
GF(2) the signs of a determinant's expansion vanish, and the nonzero
terms of A_S[S, T] are its perfect matchings, so its determinant is
per(S, T) mod 2.  (3) By their Theorem 1 (or Theorem 1), S is playable
iff some scheme has no flat alternating cycle (acyclic precedence).
By the bridge, that holds iff some scheme is the only one with its
payment set.  ∎

Both implications are strict.  The triangle {6, 10, 15} (two
schemes) is matchable and dependent, and no board below 15 has such a
set.  {12, 18, 20, 30, 45} (three schemes, below) is independent and
unplayable, and no board below 45 has such a set.

*The house example, n = 21, in both languages.*  S = {10, 14, 15, 21}
is a 4-cycle on the primes: the edges 2–5, 2–7, 3–5, 3–7.  In G, S
and the primes form the 8-cycle 2–10–5–15–3–21–7–14–2, all at ranks 1
and 2.  S's payment schemes are its two perfect matchings, each the
other flipped around that 8-cycle, which is flat and alternating for
both.  By their Theorem 1, S is unplayable.  Here, the scheme 10:2,
14:7, 21:3, 15:5 has the precedence cycle 10 → 14 → 21 → 15 → 10 (the
other scheme the reverse).  per(S, {2, 3, 5, 7}) = 2 is even, and
v(10) + v(14) + v(15) + v(21) = 0.  All four have Ω = 2, so they are
v(s) in M₁, and S is dependent there.  The F–M bound's 145-set
{10, 12, 14, 15, 16, 18, 19, 20, 21} contains it.  It has one payment
set, {1, …, 9}, with two schemes, both carrying the same 8-cycle.  So
the F–M bound reads 145 at n = 21 where the parity bound reads
opt(21) = 144.

*Theorem 10 on the published foundation.*  Theorem 12 gives Theorem
10's lemma a second proof: their Theorem 1, the bridge, and the
determinant mod 2.  The rest of Theorem 10 is linear algebra on top.
Theorem 7's lemma is the upper-half case (no member is a factor,
nothing is deleted).  Theorem 11's frame step is the same argument in
each frame.  A frame is their game on a poset of two ranks (factors
below the movers that list them) in which every mover must be taken.
By their Theorem 1, a winnable frame has a scheme that is the only
one onto its payment set, so its determinant is odd.  The frame
model's own premise (promoting Q succeeds iff every frame is winnable
with Q applied) is untouched.  Both proofs give more than GF(2): a
scheme unique onto T makes A_S[S, T] unit triangular after reordering
(the play order of the lemma's proof does it), so a playable set's
rows are independent over every field.

### What the parity bound keeps, and what it drops

Fix a scheme M with payment set T.  By the bridge, the schemes onto T
are M flipped along the sets of pairwise disjoint flat alternating
cycles of M, the empty set giving M itself.  So per(S, T) counts those
sets.  Their characterization asks for a scheme with no flat
alternating cycle (per = 1).  The parity bound asks only that the sets
be odd in number for some T.  It keeps the count mod 2, the part of
the condition that is linear (hence a matroid intersection, hence
polynomial), and drops uniqueness.  GF(2) is the field for this.  Over
the rationals independence needs a nonzero *signed* count, which an
odd count implies, since a sum of an odd number of ±1s is odd.  The
triangle {6, 10, 15} has two schemes of the same sign (determinant
±2): independent over the rationals, dependent over GF(2).

**Proposition (two factors).**  Suppose every member of S has at most
two maximal factors outside S.  Let G_S be the multigraph whose
vertices are the factors and a ground vertex, and whose edges are the
members, each joining its two factors (its one factor and the ground;
with none, a loop at the ground).  Then S is playable iff it is
independent iff G_S is a forest.  The flat alternating cycles of a
scheme are exactly the cycles of G_S, each traversed so that every
edge pays at its forward end.

*Proof.*  A GF(2) dependency of the rows is a nonempty set of edges
with even degree at every factor.  The handshake lemma makes the
ground's degree even too, so the set contains a cycle; conversely a
cycle is a dependency (a loop is a zero row).  So independent iff
forest.  A forest has a scheme that
is the only one onto its payment set: root each tree at the ground, or
anywhere if the tree has no ground, and pay each edge at its lower
end.  A leaf can be paid only by its own edge, and so on inward.  By
Theorem 12 (3) a forest is playable, and a playable set is always
independent.  An alternating cycle s₁, f₁, s₂, f₂, … (fᵢ = pay(sᵢ),
fᵢ ∈ mf(sᵢ₊₁)) is the cycle f₁ f₂ ⋯ of G_S through the edges
s₂, s₃, …, s₁, each paying at its forward end.  Conversely, take a
component with a cycle and a scheme.  It has at least as many edges as
vertices (it is connected with a cycle) and at most as many as factors
(the scheme), so it has no ground and is a bijection from edges to
factors with a single cycle.  Its trees pay from their leaves inward,
which leaves the cycle's edges to pay the cycle's vertices: around the
cycle in one of its two directions.  ∎

So on such sets their condition is acyclicity, and the parity bound
loses nothing: a nonzero per(S, T) is 2^c for c cyclic components, odd
only without cycles.  This is the graph-like regime of "Where exchange
breaks" and of Theorem 11's remark.  What the bound drops are the
*odd cores*, now exactly: unplayable sets with an odd number of
schemes onto some payment set.  By the proposition, each has a member
with three or more maximal factors outside it.

**Proposition (the first odd core).**  No board n ≤ 44 has an odd
core, so there the parity bound equals opt(n).  On the board 45,
{12, 18, 20, 30, 45} is one.

*Proof.*  Let S be independent and unplayable, so peel fails on it
(Theorem 3).  Peel's moves keep the rows, restricted to the live
factors, independent.  A back move deletes the only row with a 1 at
some factor, together with that factor.  A front move deletes a row
e_f and the column f, and a dependency of what is left would be one of
the rows before, with or without e_f.  So peel never strands a member
(a zero row): it stalls with members left.  Each has at least two live
factors, and every live factor that one of them lists is listed by at
least two.  Suppose every row had exactly two entries.  The entries
number at least two per column and exactly two per row, and
independence allows no more rows than columns.  So every column would
hold exactly two, and the rows would be the edges of a 2-regular
graph, a union of cycles, which is dependent.  So some member of the
stall has three or more live factors, each listed by a second member
of S.  Below 45, the numbers with three or more maximal factors are 30
and 42.  Their factors 15 and 21 have no other multiple on the board
(the next are 45 and 63), so neither can be that member.

On the board 45, the rows of {12, 18, 20, 30, 45} over the factors
4, 6, 9, 10, 15 (none of them a member) are 12: {4, 6}, 18: {6, 9},
20: {4, 10}, 30: {6, 10, 15} and 45: {9, 15}.  Every factor is listed
twice or more, so the set is self-covering, hence unplayable
(Theorem 5's first half, which needs no separation).
The payment set is all five factors.  30 pays 6, 10 or 15, and the
path 10–4–6–9–15 (edges 20, 12, 18, 45) then pays uniquely, so
per = 3 is odd.  In their language, every scheme has two flat
alternating cycles, both through 30, so never disjoint: three sets
(none, either one).  ∎

The bound is exact longer than the proof reaches.  It equals opt(n)
for every n ≤ 77 and first exceeds it at 78, with 1,992 against 1,991
(`rust/prover/data/exact_parity_2_1300.jsonl`).  So odd cores exist
from 45 on, and none weighs in before 78.

### The hardness

Their Theorem 2 makes the game NP-hard on posets of two ranks.  There
a playable set is the upper side of a uniquely restricted matching
(M. C. Golumbic, T. Hirst and M. Lewenstein, "Uniquely restricted
matchings", *Algorithmica* 31 (2001): a matching with no alternating
cycle, equivalently the only perfect matching on its vertices).  The
bridge, Theorem 12 and the parity split use nothing beyond a graded
poset (rank parity for Ω parity).  So the parity matroids bound their
general game too, in polynomial time.  On two ranks with unit weights
the bound is the GF(2) rank of the bipartite adjacency matrix, which
lies between the uniquely restricted matching number and the matching
number.  Three consequences:

* Unless P = NP, no bound computable in polynomial time equals the
  optimum on every graded poset.  For the parity bound the difference
  is the odd cores.
* The bound *is* exact on posets in which every element covers at
  most two others, since the two-factor proposition holds verbatim.
  There their game is solved in polynomial time by weighted matroid
  intersection.  The hardness needs elements that cover three or more
  others.  In Taxman these are the numbers with three or more distinct
  primes, which is exactly where the odd cores live.
* The paper's hardness is for the general game.  The complexity of
  Taxman itself is open, and nothing here settles it.  But the upper
  half (members above the factors they list) and the frames are
  instances of the two-rank problem that their Theorem 2 shows hard in
  general, and the gap left by Theorem 11 is the odd cores of frames
  1–3 (rust/docs/EXPERIMENTS.md, "The anatomy of the gap").  So
  wherever (♦) or the parity bound is exact in Taxman, the exactness
  comes from the arithmetic, not the structure.  This matches what the
  adversarial experiments on (♦) found.

### Status

*Proven:* the bridge; Theorem 12; Theorem 10's lemma (with Theorem 7's,
and Theorem 11 frame by frame) from their Theorem 1; the two-factor
proposition; no odd core below the board 45, so the parity bound is
exact for n ≤ 44; the odd core at 45.

*Checked by computation* (`python3 -m evaluation.fm_relation` from
heuristics/, two minutes).  Playability is decided by the rule alone.
Their condition is checked literally, as a scheme with no flat
alternating cycle in G.

* All 2,097,152 pick sets of {2, …, 22}.  Playability, their
  condition, per = 1 for some T, and peel agree on every set.
  Independence of the lemma's family, independence in both parity
  matroids, and an odd per for some T also agree on every set.  The
  hierarchy holds.  1,112 sets are matchable and dependent, the
  smallest {6, 10, 15}.  There are no odd cores.  The heaviest
  playable, independent and matchable sets give opt(n), the parity
  bound and the F–M bound, which equal the prover's values at
  every n ≤ 22.
* 18,200 random matchable sets of up to 12 members on the boards
  30–120.  Each was grown around a member with three or more distinct
  primes, the regime in which the hierarchy is strict.  The same equivalences hold
  on every set, including 500 odd cores and 570 sets that are
  matchable and dependent.
* Board 44 has no odd core of at most six members (the proposition
  says none at all).  On board 45, {12, 18, 20, 30, 45} is the only
  minimal odd core of at most six members.

*Measured:* the parity bound's exactness to 77, above.

*Open:* a condition between "odd" and "exactly one" that is computable
in polynomial time and exact for Taxman (by their Theorem 2 none is
exact on every graded poset unless P = NP); a description of the odd
cores that weigh in, from 78 on; the complexity of Taxman itself.

## Evidence

Standing status for (★): zero counterexamples anywhere, and certified
in every game checked.  The span certificate (♦) holds — every one
of greedy's rejections is a GF(2) dependency, which certifies (★) for
every component and truncation and the upper-half conjecture for the
game (Theorem 7) — in every game n = 2..100,000 (602,777,907
rejections) and in 902 sampled games to 2,000,000 (60,183,674 more).
Threshold by threshold, every game n = 2..20,000 (24,058,430
rejection thresholds in 58,306 rejection-bearing components) is
certified by bounds alone: one exclusion is always needed, the
matching-number bound, Lorea's hypergraphic-matroid bound, or the rank
bound — no search.  The earlier campaign (all n = 6..1500, forty
samples to 4000, n=2873, n=9170: 3,231 of 3,236 rejection-bearing
components verified or certified) had left five components undecided,
n = 3103, 3872, 3936, 4000 and 9170; the rank bound settles all five,
n=9170's (residual jam of 304 members, greedy's 37 exclusions, the
matching-number bound proving only 19) in milliseconds.  Data, programs
and the adversarial experiments: `rust/prover/README.md`.

| Claim | Evidence | Status |
| --- | --- | --- |
| Maximal-factor pool reproduces solvent's picks (and runs ~2x faster) | identical pick sets, 1000/1000 games | proven (Thm 1) and measured |
| Victim law: a single sacrifice repairs iff it lies in every minimal core | 22,998 trials, 0 exceptions | machine-cross-checked lemma |
| Greedy weight-optimal where the jam is hyperedge-free | — | proven (frontier argument) |
| Playable sets are GF(2)-independent; (♦) implies (★) and the upper-half conjecture | — | proven (span certificate, Thm 7) |
| (♦): every greedy rejection is a GF(2) dependency of the kept members above it | every game n = 2..100,000 and 902 samples to 2,000,000; 0 violations | certified per game; open in general |
| U\* is the unique heaviest playable upper set, every playable upper set termwise below it | follows from (★); brute force (`certify.py`) to 62 | proven given (★) (Thm 8), so certified per game to 100,000 |
| Optimal sets of n from those of n−1 (at the ceiling: T ∪ {n}; prime: T − p + n, a bijection); every optimal set holds the largest prime and no other | derivation = direct enumeration on every game at its ceiling and every prime game to 700 | proven (Thm 9, lemma) |
| The optimal set is unique | all optimal sets enumerated by the prover: every n ≤ 424; two at 425 | measured; false in general |
| Every optimal game's upper part is U\*; a game's optimal sets have equal sizes and differ by equal-sum exchanges below n/2 | every completely enumerated game (`rust/prover`, section 5) | measured; open |
| M(n) ≤ ν(n); M(n−1) ≤ M(n) ≤ M(n−1) + 1, with equality on the left at a prime n | brute force to 22 | proven (Lemma, "Longest games") |
| Playable sets, with each pick's own coordinate taken out, are GF(2)-independent; the parity matroids bound opt(n) | every playable set independent in both matroids on every board to 18 (enumerated); the bound at least opt(n), and the recorded game independent in both, at every n ≤ 1300; the certified intersection equal to enumeration on 3,000 random pairs of matroids | proven (Thm 10) |
| Promotable sets of the frame model are independent in the even and the odd frames' GF(2) matroids, whose heaviest common independent set bounds the best promotion set, at every node in their contractions | every promotable set of every board with ≤ 22 candidates independent; the bound at least the best promotion set at every n ≤ 1300; never below the best completion at the 600 nodes of the solver's probe; the solver's chain 1..2175 with it reproduces every score to 1300 (optimal.json's, made without it) | proven (Thm 11) |
| A prime exchange that maps the frame model's root onto itself maps promotion sets to promotion sets worth (q − p)(c_p − c_q) more, so every heaviest set keeps c_p ≤ c_q for every such pair at once | the images of the optimal set and of random maximal promotion sets promotable and worth the formula at every board with pairs to 400 and a sample to 1300, every optimum keeping every inequality; every promotion set of the boards with pairs and ≤ 22 candidates likewise, the best that keeps the inequalities the optimum; the solver with the rule finds the optimum of every board with pairs to 600 and its chains reproduce every score to 1300 and on 2126-2175 | proven (Thm 13) |
| The frames are the components of the layers by Ω (Lemma Ω); playability splits by layer (Theorem L) | every frame of every board 2..8191 in one layer, the frames in front one layer down (the census, `rust/examples/structure.rs`, check A) | proven |
| Every optimal set holds the largest prime alone in layer 1 and exactly U\*_2 among the sources of layer 2, and its numbers of layer 2 up to n/2 keep U\*_2 a forest (Theorem B) | every game of optimal.json (2..1300), `results/strict.jsonl` (2..2175) and the chain 2176..2276; every optimal set the prover enumerated (987 sets of 2..989, 57 of 798..1019 with the upper part fixed); every optimal set of 2..48 by exhaustive search; 0 exceptions; numbers of layer 2 up to n/2 only on 76 boards, all ≤ 186 (`rust/tests/bottom_layers.rs`) | proven |
| N: the numbering of the frames is clean | every board 2..8191, every board the solver supports (the census, check C); frame h is the frames with freedoms of layer 3 + h (2 + h on 80 boards ≤ 186); setup refuses a board without it | checked per board, on all of them |
| Given N, U\* ∪ L is playable iff L is a promotion set of the frame solver's root problem, and weighs in_the_bag + ΣL (Theorem M) | every set of every board ≤ 100 both ways (11,635 sets; 101..120, 167,681, in an ignored test); random sets both ways on boards 60..1300 (and 2000..8191, ignored); no candidate lists a back claim's factor, none is in frame 0, every one has a home, U\* is an independent greedy's, on every board to 8191 (the census, checks D-G); the true near misses the candidate filter alone keeps out, 27 boards to 2000 (`rust/tests/model_exact.rs`) | proven (given N) |
| U: some optimal set contains U\*_{≥3}; with N, the frame solver's answer is opt(n) iff U holds at n (the corollary) | every n ≤ 2175 (the prover's proofs on the whole board); the strong form in every game enumerated; barred parity bounds certify 84-99.7% of U\*_{≥3} per game to 2175 and 54-98% on the EC2 chain's 2176..3314, and close no game alone (`rust/prover/data/upper_*.jsonl`) | open; holds to 2175 |
| opt(n) equals the recorded score: the frame model's answer is the true optimum | the prover's exact searches on the whole board (playability theorems, the Franklín–Moniot and parity bounds, the ceiling and the prime formula; `rust/prover`, section 3): every n = 2..2175, the whole committed chain | verified (exact, on the whole board) to 2175 |
| An optimal game has the most moves, M(n) | one move short at n = 833, 834 and 835, proven by the prover; a longest game for every other n ≤ 1300, 1,293 of them by the matching certificate (`rust/prover`, section 6) | **false** (n = 833); the gap is at most one to 1300, open beyond |
| (★) on rejection-bearing components | every threshold of every game n = 2..20,000 certified by bounds; the earlier 5 undecided settled | certified per game; open in general |
| (♦) and (★) under orders obeying the window law, adversarial or random, or its reverse | 1,981 instances, n = 2000..20,000, 0 failures (uniformly random orders: 16–50% fail; window law for a random prime ranking: 26–57%) | measured |
| n=2873 component: greedy's 6 exclusions minimal | 12.4M playability checks over the 70-member residual | verified |
| Exclusion sets confined to the residual | validated 35/35 vs. unrestricted search | proven (Thm 6) |
| Core characterization without separation | 30,000 fuzzed instances, 0 failures | unproven |
| Separation hypothesis is necessary | 86 violations in 6,507 unseparated instances; 0 in 25,615 separated | measured |
| Monomial-impossibility criterion (exponent-vector majorization) | 300 sampled prime constellations, 0 disagreements | validated |
| 3-prime census: shapes ≤ 8 members, paddings ≤ 2 (≤ 7 exhaustive, size 8 sampled 10,000/shape) | 77 minimal + 428 padded shapes; 22,909 trap orderings: 22,891 impossible, 11 unrealized, 7 realizable, 0 unprotected | measured |
| Window guards defeat every realizable breaker | 18/18 realizations, components to 6,186 members | measured |
