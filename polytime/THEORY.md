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

The two theorems yield a static characterization: a set is playable
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
