# The theory, with proofs

This is the proof companion to [README.md](README.md): every theorem
the ledger there cites lives here in full.  The README states what is
claimed and how well it is tested; this file is *why*.

Standing definitions, used throughout.  The *maximal factors* of c
are the divisors f with c/f prime — the divisors reached by deleting
one prime from c (mf(4) = {2}, mf(6) = {2, 3}; a prime's only maximal
factor is 1).  A *reduction instance* is a set of members C, a factor
pool F disjoint from C, and each member's candidate list mf(c) ∩ F;
`core.solve_mini` peels such instances by forced moves only.  The
upper half of game n — the numbers above n/2, which never divide one
another — forms one naturally, with F the union of the members'
maximal factors.

## The playability characterization

> **Playability.**  A set S of selections can all be played, in some
> order, **iff** there is a matching assigning each member a distinct
> maximal factor not in S such that the precedence relation "a before
> b whenever a's assigned factor divides b, or a divides b" is
> acyclic.  Any topological order of the precedence is a legal game.

One direction is a construction (play the topological order; each
pick's reserved payment provably survives until its turn).  The other
is the Franklín–Moniot lifting argument turned inward: whatever
divisor d a pick actually surrenders lifts to a maximal factor f with
d | f | c; f must still be in the pot (anything that had removed f
would have removed d with it), and f cannot itself be a selection, or
the pick's sweep would destroy an unplayed number.  So every legal
game already pays each pick with a distinct outside maximal factor —
maximal factors are the only factor notion the whole project needs.
(Restricting solvent's payment pool from all divisors to maximal
factors reproduced identical pick sets in 1000/1000 games and runs
~2x faster.  The one boundary: feasibility is pool-invariant, raw
play orders are not — `cascade`, which plays solve_mini's sequence
directly under real sweeps, still needs true divisors: at n=5 a
maximal-factor pool emits [4, 5], and playing 4 sweeps the 1 that 5
needed.)

## The schedulability theorem

**Theorem.** Whenever `solve_mini` succeeds on a reduction instance
(members C, factor pool F with C ∩ F = ∅, each member's list L(c) =
mf(c) ∩ F), its returned assignment pay(·) has an acyclic precedence
"a before b whenever pay(a) | b, or a | b" — so a legal play order
always exists, and the ordering step can never fail.

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
below.)  Now let c be the earliest-peeled member of such a cycle and
lean on two invariants of the implementation: a factor leaves a
member's live list only when globally consumed as some pair's
payment, and `comps_of[f]` holds exactly the live members listing f.
If c was front-peeled, its live list at that step was {pay(c)}; its
cycle predecessor a peels later, so pay(a) is unconsumed and still
in c's live list — pay(a) = pay(c), contradicting distinct payments.
If c was back-peeled, pay(c) was listed by no other live member; its
cycle successor d peels later, is live, and lists pay(c) —
contradicting that uniqueness.  ∎

Two notes.  The theorem covers exactly the assignments the code can
emit: the fast Kuhn-matching tier checks acyclicity explicitly before
accepting, and falls back to solve_mini — whose output the theorem
covers — whenever its matching goes cyclic.  And the proof *requires*
maximal-factor payments (the potential argument dies with all-divisor
pools), so the maximal-factor refactor, adopted for speed, is what
made the conjecture provable.  The 39 → 33 → 22 → 26 cycle shows why
peeling is structurally immune: a cycle core has no degree-1 vertex,
so the forced-move discipline stalls and refuses rather than threads
through it.

## The completeness theorem

**Theorem.** If a set is playable, solve_mini succeeds on it.
(Contrapositive: every solve_mini refusal is a proof of
unplayability.)

*Proof.*  Three lemmas.  *Front moves preserve playability*: if live
member c has exactly one live factor f, every covering matching must
assign f to c, and removing the pair leaves an acyclic covering
matching of the residue (restriction of acyclic is acyclic).  *Back
moves preserve playability*: if live factor f is listed by exactly
one live member c, take any acyclic covering matching M — nobody but
c can be matched to f, so M minus c's own pair covers everyone else
while avoiding f, and survives the removal of (c, f).  The invariant
here is playability of the *residual instance*, which depends only on
which pair was removed — not on the fact that the code pays c with f
while M may pay it with something else (that discrepancy belongs to
the schedulability theorem, not this one).  *A stall is unplayable*:
if no forced move exists with members remaining, every live factor
has zero or at least two live members.  Take any covering matching of
the residue: each member a's payment is a live factor with at least
one lister (a itself), hence at least two — so some other live member
b lists pay(a), and pay(a) | b is a precedence edge a → b.  Every
vertex has an out-edge, so the finite digraph contains a directed
cycle; every covering matching is precedence-cyclic, and the residue
is unplayable.  Chaining: a playable input stays playable through
every forced move, a playable residue can never strand a member at
zero factors, and a halt with members remaining would make the
(playable) residue unplayable — contradiction.  ∎

Note what each theorem needs: completeness holds for *any* pool of
proper divisors — maximality unused — but the converse
(schedulability) genuinely requires maximal factors: with lists
{6: [1], 9: [3]} the unique matching is a perfect 2-cycle (1 | 9 and
3 | 6), the set is unplayable, and peeling would accept it anyway.
Only in the maximal-factor regime do the two theorems interlock into
the full equivalence: **solve_mini succeeds ⟺ the set is playable.**
Acceptance in this project is not a heuristic anywhere: it is a
decision procedure for playability, proven in both directions.

## Cores, forests, and where exchange breaks

The peel theorems suggest — and "The residual theorems" at the end
of this file now prove — a static characterization: a set is playable
iff it contains no *self-covering core* — a nonempty subset in which
every member's every maximal factor is also some other member's
maximal factor (dismantle by repeatedly removing any member holding a
factor no one else lists; the order never matters).  The n=39
specimen {39, 33, 22, 26} is the minimal celebrity core.

For members with at most two distinct primes the characterization
turns geometric: map each factor value to a vertex (plus a ground
vertex for primes and prime powers, whose single factor makes a
pendant edge) and each member to the edge joining its two factors —
a core is exactly a subgraph of minimum degree 2, so **playable ⟺
forest**.  Forests famously trade well: removing an edge and adding
another never traps you, which is why greedy is bulletproof in that
regime.  Members with three or more distinct primes are hyperedges
glued onto the graph, and there the trading breaks.  The smallest
failure is at **n = 2873**, on six members built from {11, 13, 17}:

    A = {1573, 2057, 2431, 2873}                (playable, size 4)
    B = {1573, 1859, 2057, 2197, 2873}          (playable, size 5)

Both members of B∖A jam: A+1859 and A+2197 each contain a core.  Two
maximal playable sets of *different sizes* — so playable sets do not
support the exchange argument that justifies greedy in the textbook
setting, and greedy optimality for the upper half cannot be proven
that way.

And yet greedy survives: in every three-prime gadget for n = 700..9000
(1,108 of them), descending-weight greedy still returns the
maximum-weight playable set — at n=2873 it takes a heavy size-5 set,
{1859, 2057, 2197, 2431, 2873} (heavier even than B), and never
lands on the stuck size-4 A, because the stuck base
is also the *light* one.  That is the precise remaining open problem
for "the upper half is guaranteed optimal": prove that in these
gadgets the heaviest maximal playable set is always reachable by
descending-weight greedy — an argument that must use the weights
(a member is its own weight), since the unweighted structure
provably cannot carry it.

How load-bearing the arithmetic is was measured directly: taking the
n=2873 gadget's incidence structure and trying all 720 weight
orderings, **48 of them make greedy fail** — the actual member
values land in the safe majority.  Random abstract graphic+hyperedge
systems violate dominance readily; the number-realizable ones, never
(zero violations across every gadget tested, n = 741..1850 dense plus
spot ranges to 10,072).

## The frontier argument, and exactly where it stops

The natural direct proof gets remarkably far.  Compare greedy G to a
supposed heavier playable T at their largest disagreement d.  T
cannot be the one holding d (T would contain greedy's own rejection
certificate — downward closure).  So d ∈ G, T skips it, and if T is
maximum-weight then T+d must be unplayable: every core of T+d
contains d (T is playable) and dips below d — the part of T+d at or
above d is exactly G ∩ {≥ d}, playable *because* step one put d in G,
so no core fits inside it.  Let R be the union of those cores — the
2-core of T+d in hypergraph terms.  Three short lemmas are theorems
(machine-cross-checked at every step): repairs live entirely inside
R; a sacrifice t repairs T+d exactly when t lies in **every** minimal
core of R (the "victim law", 0 exceptions in 22,998 trials); and when
every member of R has at most two distinct primes, R−d is a forest,
R has *exactly one* minimal core (a forest plus one edge has one
cycle), so a below-d victim always exists and the sacrifice
T+d−t both restores playability and gains weight — contradiction.
**Greedy is provably weight-optimal wherever the jam is
hyperedge-free.**

Beyond that regime the single-sacrifice step is **false**.  At
n = 9170, d = 6137 = 17·19², the six members {4913, 5491, 6137,
6647, 7429, 8303} (all built from 17, 19, 23; the hyperedge is
7429 = 17·19·23) form an R with **two** minimal cores through d
sharing nothing else — their below-d parts are {4913} and {5491},
disjoint — so no single *below-d* sacrifice repairs: every single
repair (6647, 7429, 8303, or d itself) costs at least d, verified
directly.
Greedy is still right there: every d-avoiding way to break both
cores costs more than d (any single alternative victim is heavier
than d; any two members sum past n), and the full-context optimum
does take d — skipping it costs 1,224 points net.  But proving that
in general is now visibly a *non-local, weighted* question: whenever
no single lighter sacrifice repairs the jam, the would-be improver's
kept members must be charged for what they lock out elsewhere in the
component, not just at the frontier.  That is
the open problem's irreducible form, with n=9170 as the first test
any proof must pass.

## The cardinality reformulation

The weights can be removed from the open problem entirely.  Taxman's
weights are the members themselves, so any weight assignment is
consistent with the value ordering — and for order-consistent
weights, the classic majorization fact applies: greedy's set
outweighs a competitor for *every* consistent weighting iff it
matches or beats the competitor's cardinality within every top
segment of the ordering.  Since descending greedy's decisions above
any threshold never depend on anything below it (its restriction to
{> t} IS its run on the truncated instance), the whole conjecture
becomes one unweighted statement:

> **(★)** On every connected component of the member–factor
> incidence graph — and every truncation of one — descending greedy
> returns a **maximum-cardinality** playable subset.

(★) implies weight-optimality by summing the truncations
(layer-cake); by majorization it is also *necessary* if optimality
is to hold for every order-consistent weighting, so it is the exact
combinatorial content of the conjecture, not a strengthening.

What the reformulation exposed, each fact machine-checked:

* **The ordering is the protection, not the magnitudes.**  On the
  abstract incidence structure of the n=2873 gadget (equally of the
  n=9170 sextet), 48 of the 720 weight orderings make greedy fail —
  and every one of the 48 is a pure *cardinality* upset: greedy is
  steered into excluding both members of a factor-sharing pair,
  landing on a 4-set while a playable 5-set exists.  No
  magnitude-style axiom can block that (near-uniform weights realize
  it), and in particular the "all members exceed n/2" axiom kills
  none of the 48.  The real numeric order simply never lands among
  the 48 — at n=2873 the trap needs the sharing pair ranked last,
  and 11²·13 sits below both sharers.  A proof of (★) must therefore
  use the arithmetic of the ordering (the window law: within the
  star of f, f·q < f·p iff q < p), because the incidence structure
  alone provably permits the trap.
* **The component is the unit of proof; sub-instances lie.**  On the
  bare n=9170 sextet, greedy's 4-of-6 is NOT maximum — dropping only
  d leaves a playable 5-set — and it stays non-maximum with the
  first ring of attachments added.  What restores it is 6859 = 19³,
  farther out in the component, whose demand for the factor 361
  tightens the contest.  Cardinality dominance genuinely fails on
  fragments and holds on the full component.
* **Minimal exclusion sets live inside the residual** (the
  component's 2-core under private-factor dismantling) — Lemma A
  generalized from one skipped pick to the whole component;
  validated 35/35 against unrestricted search, and the workhorse
  that made the big components decidable (n=2873's 377-member
  component has a 70-member residual).
* **Matching certifies most components, and its failures are the
  n=21 phenomenon.**  Playable ⊆ matchable gives the free bound
  |G ∩ K| ≤ max-cardinality ≤ ν(K); greedy meets ν on 86.8% of the
  large components, certifying (★) there with no search.  Where ν
  overshoots, every exactly-probed case found greedy still optimal —
  the surplus was matchable-but-unplayable phantom edges, the
  cardinality twin of the F–M bound's weight slack.

Standing evidence for (★): zero counterexamples anywhere.  Across
every game size tested (all n = 6..1500, forty samples to 4000,
n=2873, n=9170), of 3,236 rejection-bearing components, 99.8% are
verified (exact search: brute force, or exhaustion over the
residual) or certified (greedy meets the matching bound), including
the n=2873 celebrity component (greedy's 6 exclusions proven minimal
by 12.4M playability checks over its residual).  The handful still
undecided are single-cluster jam regions whose exact search is
super-exponential — n=9170's component (|K| = 1,174, residual 306,
37 greedy exclusions, any improving exclusion forced by the matching
bound to have size 19..36) is the standing monster.  The proof
obligation, sharpened: *within one connected jam cluster, under the
numeric ordering, greedy's exclusion count is minimal* — with the
window law as the engine, and n=9170's cluster as the first test any
proof must pass.

For the two known jam shapes, that obligation is now discharged at
the ordering level (machine-classified; the fixed-comparison
criterion is exact majorization of exponent vectors, validated
against 300 sampled prime constellations with zero disagreements).
Every one of the n=2873 shape's 48 trap orderings demands p²q ranked
above pq² — impossible for every prime triple; one universal
inequality retires the entire class.  The n=9170 shape's 48 split
46 impossible the same way, one never realized in the searched
constellation space, and exactly one realizable: the true numeric
descending order itself.  Greedy's real behavior on that shape IS an
abstract trap — standalone, it keeps 4 where 5 fit — and what
rescues it, in every realization tested (three constellations, both
window edges), is the sandwiched guard: q³ > pq² > n/2 and
q³ < q²r ≤ n force q³ into the game wherever the shape exists
(proven — two lines), its only maximal factor is the contested q²,
and the oracle's refusal of the trap's 5-set names it verbatim
("q³ has no remaining factor").  Deleting q³ alone springs the trap
at the tight end of the window (load-bearing, 4/6 realizations);
at the loose end other ambient contention over-determines the
defense.  The general lemma this points at — every trap ordering of
every jam shape requires either a monomial inversion
(majorization-impossible) or the absence of a window resident that
the shape's own realization forces into (n/2, n] — is the sharpest
known form of why the integers protect greedy, and the guard's
*sufficiency* in general is the part still resting on measurement.

## The residual theorems: characterization and exclusion confinement

Standing hypothesis for this section: the instance is **separated** —
no member is a maximal factor of another member.  Every instance this
project computes with satisfies it automatically: members of an
upper-half or band instance all exceed n/2 (or the band floor), while
a maximal factor of such a member is at most half of it.  Separation
buys a decisive simplification: the pool of a subset S is the union
of its members' maximal factors with the members subtracted, and
under separation the subtraction never bites — every member's live
list is its full maximal-factor set, in every subset, so
self-covering and dismantling are intrinsic notions, stable under
taking subsets.

The hypothesis is not decorative (adversarial review found this the
hard way, breaking an earlier draft that claimed more).  With members
doubling as each other's factors the pool shifts as members come and
go, and two of the theorems below become false: for K = {2, 4, 8}
(residual {4, 8}, since 4 and 8 have empty lists there), the
exclusion E = {2, 8} leaves the playable {4} — 2 re-enters the pool —
yet K∖(E∩R) = {2, 4} strands 4, violating Theorem R3; and the
extension lemma dies at S = {8, 33, 37} with x = 4, whose arrival as
a member removes 4 from the pool and strands 8.  Fuzzing found 86
such confinement violations in 6,507 member-divides-member instances
— and zero in 25,615 separated ones.

**Theorem R1 (survival and confluence; no separation needed).**  A
self-covering subset survives every dismantling sequence; all
dismantling sequences of an instance end at the same fixpoint, the
*residual*; a nonempty residual is itself self-covering, and the
residual equals the union of all self-covering subsets.  In
particular the residual is empty iff no self-covering subset exists.

*Proof.*  If the next deletion removes x with private factor f and x
belonged to a self-covering C, then f — like every factor of x — is
listed by another member of C, all of C being alive; that contradicts
privacy, so dismantling never touches C.  A nonempty fixpoint is
self-covering by definition of "no forced deletion remains" (a member
with an empty list is vacuously covered and never removable).  Two
fixpoints R₁, R₂ of different sequences: R₁ is self-covering, hence
survives the second sequence, so R₁ ⊆ R₂, and symmetrically.  ∎

**Extension lemma (separated instances).**  Let S be playable and let
x ∉ S be a member that is not a maximal factor of any member of S,
with some f ∈ mf(x) listed by no member of S.  Then S ∪ {x} is
playable.

*Proof.*  Adding x leaves every old member's list intact (x sits in
no one's maximal factors; the pool only grows).  Take an acyclic
covering assignment of S and extend it by pay(x) = f; distinctness
holds because every payment is listed by its payer and f is listed by
no member of S.  Suppose the extended precedence has a cycle.  The
precedence among old members is unchanged, so the cycle passes
through x.  Now reuse the first half of the schedulability theorem's
proof, which is assignment-agnostic — it needs only that every
payment is a maximal factor of its member, so Ω(pay(v)) = Ω(v) − 1
exactly: along a member-divides-member edge the potential strictly
increases, so a cycle contains none; along a payment edge a → b it
increases weakly, with equality forcing pay(a) ∈ mf(b).  A cycle
forces equality everywhere — in particular on the cycle's edge
leaving x, which then requires f ∈ mf(z) for some z ∈ S,
contradicting f's privacy.  ∎

**Theorem R2 (core characterization, separated instances).**  A set
is playable iff its residual is empty — equivalently, iff it contains
no self-covering subset.

*Proof.*  (⇒, no separation needed)  A self-covering C is unplayable:
a member with an empty list can never pay, and otherwise any covering
assignment gives each x ∈ C a payment listed by another member of C —
an out-edge within C — so the finite digraph has a cycle and every
covering assignment is precedence-cyclic.  Downward closure of
playability finishes.  (⇐)  Let x₁, …, x_k be a complete dismantling
order (empty residual) and T_j = {x_j, …, x_k}.  T_{k+1} = ∅ is
playable; x_j has a factor private in T_j, is no member's maximal
factor (separation), so the extension lemma lifts playability from
T_{j+1} to T_j.  Induction ends at T₁ = M.  ∎

The ⇐ direction is the new content: the project had asserted the
characterization ("the two theorems yield…") without proving this
half — the adversarial referee's 8,000-instance validation was, until
now, its only support.  Without separation the statement has never
failed (30,000 fuzzed instances, 21,000 of them member-divides-member)
but remains unproven.

**Theorem R3 (exclusion confinement, separated instances).**  Let K
have residual R.  If K ∖ E is playable, so is K ∖ (E ∩ R).

*Proof.*  Suppose K ∖ (E ∩ R) is unplayable.  By R2 it contains a
self-covering C; by R1, C ⊆ R.  C avoids E ∩ R and lies in R, so it
avoids E entirely: C ⊆ K ∖ E.  But K ∖ E is playable and by R2
contains no self-covering subset.  ∎

Corollaries.  Minimal exclusion sets lie inside the residual
(otherwise E ∩ R is a smaller valid exclusion).  Every maximum-weight
playable subset — any positive weights, cardinality included —
contains everything outside the residual.  Exact searches restricted
to subsets of the residual are therefore sound: the reduction the
(★) campaigns leaned on (validated 35/35 against unrestricted search
before this proof existed).  The frontier argument's Lemma A
("repairs live entirely inside R") is the special case K = T + d.
