# The theory, with proofs

This is the proof companion to [README.md](README.md): every theorem
the ledger there cites lives here in full.  The README states what is
claimed and how well it is tested; this file is *why*.

Everything below supports two headline facts:

* **Proven.**  `peel` succeeds on a set **iff** the set is
  playable **iff** it contains no self-covering core (Theorems 1-6).
  Acceptance is not a heuristic anywhere in this project: it is a
  decision procedure for playability, proven in both directions.
* **Open.**  The upper-half optimality conjecture: in every game, no
  playable subset of the upper half outweighs the one descending
  greedy selects (the set the README calls U\*).  "The open problem"
  below states it precisely, gives its weight-free working form —
  conjecture (★) — and maps the exact boundary between what is
  proven and what remains.

Empirical support for each claim is collected in the evidence table
at the end.

## Definitions

The *maximal factors* of c are the divisors f with c/f prime — the
divisors reached by deleting one prime from c (mf(4) = {2},
mf(6) = {2, 3}; a prime's only maximal factor is 1).  A *reduction
instance* is a set of members C, a factor pool F disjoint from C, and
each member's candidate list mf(c) ∩ F; `core.peel` reduces such
instances by forced moves only.  The upper half of game n — the
numbers above n/2, which never divide one another — forms one
naturally, with F the union of the members' maximal factors.

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
maximum-matching size; greedy meets ν on 86.8% of the large
components, certifying (★) there with no search.  Where ν
overshoots, every exactly-probed case found greedy still optimal —
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

## Evidence

Standing status for (★): zero counterexamples anywhere.  Across
every game size tested (all n = 6..1500, forty samples to 4000,
n=2873, n=9170), 3,231 of 3,236 rejection-bearing components —
those where greedy excludes at least one member, the only place (★)
could fail — are verified (exact search over the residual, or brute
force) or certified (greedy meets the matching bound).  The five
undecided — n = 3103, 3872, 3936, 4000, and 9170 — have exact
search budgets of 2.7×10⁹ to 1.1×10⁴⁷ subsets; the n=9170 component
(1,174 members, residual 306, 37 greedy exclusions, any improving
exclusion forced by the matching bound to have size 19..36) will be
settled only by a theorem.

| Claim | Evidence | Status |
| --- | --- | --- |
| Maximal-factor pool reproduces solvent's picks (and runs ~2x faster) | identical pick sets, 1000/1000 games | proven (Thm 1) and measured |
| Victim law: a single sacrifice repairs iff it lies in every minimal core | 22,998 trials, 0 exceptions | machine-cross-checked lemma |
| Greedy weight-optimal where the jam is hyperedge-free | — | proven (frontier argument) |
| (★) on rejection-bearing components | 3,231 of 3,236 verified or certified; 5 undecided | open |
| n=2873 component: greedy's 6 exclusions minimal | 12.4M playability checks over the 70-member residual | verified |
| Exclusion sets confined to the residual | validated 35/35 vs. unrestricted search | proven (Thm 6) |
| Core characterization without separation | 30,000 fuzzed instances, 0 failures | unproven |
| Separation hypothesis is necessary | 86 violations in 6,507 unseparated instances; 0 in 25,615 separated | measured |
| Monomial-impossibility criterion (exponent-vector majorization) | 300 sampled prime constellations, 0 disagreements | validated |
| 3-prime census: shapes ≤ 8 members, paddings ≤ 2 (≤ 7 exhaustive, size 8 sampled 10,000/shape) | 77 minimal + 428 padded shapes; 22,909 trap orderings: 22,891 impossible, 11 unrealized, 7 realizable, 0 unprotected | measured |
| Window guards defeat every realizable breaker | 18/18 realizations, components to 6,186 members | measured |
