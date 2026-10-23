# Going higher and coming back down (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  The chain uses the game below: the ceiling opt(n) ≤ n + opt(n − 1) (the cap), the prime formula,
`same_search` and the seed.  Can the games above n (their optima, optimal games, search problems and proofs) tighten
n's upper bound, raise its incumbent or settle it, so that an expensive game is derived from a cheaper one above it?
A proof's cost is set by the root's gap ([slope](../what-drives-the-slope-2026-10/README.md); the GF(2) gap for a search
built on the [GF(2) bound](../matroid-intersection-bounds-2026-10/README.md)), so a relation from above counts if it
beats that gap where the searches are dear.  **Answer.**  No.  A game above bounds n through a relaxation of its search
problem, whose root bound is never smaller than n's own (none of 972 relaxing pairs), so an exact cap comes only from a
problem at least as hard, and a cheaper one leaves a cap that misses and prunes nothing.  Of the 416 exhausted games
only 1023 can be derived from above, from 1024, which saves 0.55% of the nodes of the exhausted searches of 1001-1300.

## Method

Nothing in the solver changed, and one search was run (result 7).  The study's code lived on a branch that was never
pushed and has been deleted: two example programs (one set up the boards n and n + 1..n + 3 and measured how their
search problems relate; one tested search problems for equivalence), a table script and their data.  What they read is
on master: the chain (`results/strict.jsonl`), the GF(2) root gaps
(`solver/notebook/search/matroid-intersection-bounds-2026-10/roots.jsonl`) and the one-thread proofs
(`solver/notebook/search/what-drives-the-slope-2026-10/slope_proofs.jsonl`).

For each board n of 2..1297 (primes and trivial boards included) and each m = n + 1..n + 3, both boards were set up as
`taxman solve` sets them up (default tactics) and the optimal games read from the chain (a game's P\* is its score less
`in_the_bag`).  Recorded for each pair: whether the two search problems are *equal* (the test of `same_search`,
[DESIGN.md, "Same search"](../../../docs/design/same-search.md)) and whether each *relaxes* the other; how m's root
differs from n's, frame by frame; what each optimal promotion set is worth on the other board, after *repairing*
(dropping the smallest member of a conflict until the rest can be promoted) and *filling* (adding the largest candidates
that keep it promotable); and, for n, the incumbent its chain search starts from and a *look-ahead* start (the incumbent
the search of m starts from when seeded from the game of n − 1, repaired and filled on n's board).  A game is
*exhausted* when the chain proved it by exhausting the search (416 of 1..1300).

**When one problem relaxes another.**  Problem Π′ *relaxes* Π when every promotion set that Π can promote from its
root, Π′ can promote from its own.  Then P\*(Π) ≤ P\*(Π′).  The study used this sufficient condition, checked on the two
roots as the solver holds them (a problem without candidates is relaxed by every problem):

* (a) every candidate of Π is a candidate of Π′ with the same home frame, and Π′ has at least as many frames; and
* (b) for every frame i of Π: Π′'s frame i has every factor of Π's frame i, and each move a of Π′'s frame i that Π's
  frame i lacks has a maximal factor y that is a factor of Π′'s frame i but not of Π's, and that no other number which
  could be a move of frame i lists: no other move of Π′'s frame i, and no candidate of Π of home i + 1.

*Proof.*  A set T of candidates can be promoted from a root exactly when each member is a factor of its home frame
h ≥ 1 and every frame i is winnable with T applied all at once (the module comment of `model::promote`).  Let T be
promotable in Π.  By (a) it applies to Π′.  For a frame i of Π, let B be Π's frame with T applied and R Π′'s.  R's
factors contain B's, by (b), and R's moves are B's plus the set A of moves of Π′'s frame i that Π's lacks.  Each a in A
has the y of (b), which no other move of R lists, so y is private to a.  A frame is winnable exactly when the peel
(`model::peel`) drops all its moves: drop A first, by their private factors; what is left is moves of B over factors
that include B's, and winnability only gets easier with fewer moves and more factors.  ∎

**Equivalent problems.**  Two problems with the same feasible sets have the same P\*, equal or not.  The test (for each
non-prime non-trivial board of 2..1300 against the 8 earlier boards that set a search up) reduces a problem to a *form*
and compares forms.  A frame's winnability is a property of the bipartite graph between its movers (its moves and the
candidates of home i + 1) and its factors, so the feasible sets are fixed by the candidates with their homes and, per
frame, that graph with the candidates labeled; the other numbers are *anonymous*.  The form strips each frame's graph
by peeling, in the hardest state, every mover that lists a live anonymous factor no other mover lists, and two problems
are equivalent when their forms have an isomorphism that fixes every candidate and maps anonymous movers and factors to
their kind.  The isomorphism is found by color refinement and then checked edge by edge, so a hash collision can only
lose an equivalence, never invent one.

## Results

**1. What the game above proves: opt(n) ≤ opt(n + K) − Σ m/2 − (q − P).**  The sum is over the even m in (n, n + K], P
and q are the largest primes at most n and n + K, and K < (n + 3)/2.  *Proof.*  An optimal game G of n opens with P
(Lemma A in the proof of `accelerate::prime_shortcut`).  For an even m in (n, n + K], f = m/2 divides nothing else on
1..n, and its only multiples up to n + K are f and m.  On 1..n + K, play G with each such f deleted and, if q > P, its
first move P replaced by q; then every such m.  Deleting a move leaves more in the pot, so G's later moves stay legal;
q is inert; and each f stays until its m, which it pays.  The game scores at least opt(n) + Σ (m − f) + q − P.  ∎  With
K = 1 it gives opt(n) ≤ opt(n + 1) − (n + 1)/2 for odd n, and the prime shortcut's equality for a prime n + 1; it gives
nothing for an odd composite n + 1.

It holds for every n and K checked (2..1299, K up to 39), and is tight 223 times at K = 1 (the 210 odd primes and 13
even m: 8, 12, 16, 24, 32, 48, 64, 96, 256, 384, 512, 768, 1024), 11 at K = 2 and once at K = 3.  Of the 416 exhausted
games, the bound from n + 1 settles 107: 106 through a prime n + 1, whose search problem is n's own (each of the 204
primes whose predecessor sets a search up poses that problem, so nothing is learned), and 1023 through 1024.

**2. Neighboring problems differ by a local edit, and the edit runs one way.**  From odd n to n + 1 = 2f, f leaves the
moves of a frame h (a freedom more there) and becomes a candidate factor of frame h + 1 that pays for the new move 2f;
at 2^k a new top frame appears.  From even n to an odd composite n + 1, n + 1 joins frame 0 or 1 as a new move, or with
a new hive, a freedom less.  So after an odd n, n + 1 relaxes n or poses the same problem (all but 6 of 434), and after
an even n, n relaxes n + 1 or neither (pairs of games that are neither prime nor trivial, 2..1297):

| n | equal | n + 1 relaxes n | n relaxes n + 1 | neither |
|---|---|---|---|---|
| odd | 226 | 202 | 0 | 6 |
| even | 244 | 0 | 85 | 106 |

A relaxation gives P\*(n) ≤ P\*(m), a cap from above in the frame model.  Over the 2708 and 1582 relaxing pairs within
three of each other (equal problems included), no P\* breaks it and every optimal set of the relaxed problem can be
promoted in the relaxation.

**3. A cap from above is exact only where the problem above is no cheaper.**  172 of the exhausted games have a
relaxation among n + 1..n + 3 that is not their own problem; its cap is exact for 5: 30, 45, 510, 765 and 1023, each at
a change of the top frame that is worth nothing.  Elsewhere the cap misses by the worth of what the edit frees, and a
cap that misses prunes nothing: a node is cut by its own bound, and the cap only stops a search whose incumbent reaches
it.  At 23 of the 66 games of 880-1297 that have one, the cap from above is closer than the root's GF(2) bound (1235: 12
against 164, 1254 and 1281: 24 against 204 and 206, 1110: 8 against 134), but at the dearest games it is absent or
farther: of the 12 dearest exhausted games of 1001-1300 (7.6 to 53 million chain nodes; GF(2) gaps 147 to 282), only
1235 has a much closer cap, 1269 a slightly closer one (148 against 167), 1025, 1230, 1287, 1290 and 1295 farther ones
(323, 532, 309, 646 and 648), and 1232, 1236, 1264 and 1274 none (1300 was not measured).

The reason: a relaxation has more factors, fewer moves, more candidates, and new moves only with private new factors, so
its matching graph and its GF(2) frame matroids contain those of the problem it relaxes, and its root bound is at least
as large.  None of the 972 relaxing pairs with both root bounds known has a smaller GF(2) or matching bound.  So gap(m)
≥ gap(n) − (P\*(m) − P\*(n)): a cheaper problem above leaves a cap that misses, and an exact cap comes from a problem at
least as hard.  The relaxations from below do no better than the chain's cap: 70 exhausted games have one, none exact
and none tighter than the cap.

**4. Incumbents from above are n's own optimum, reflected.**  The chain's search starts below the optimum in 110 of the
415 exhausted games of 2..1297.  The optimal set of n + 1, on n's board, reaches P\* in 91 of them, but n + 1 comes
after n and is often built on it (98 of the 416 exhausted games are followed by a chain game, opt(n + 1) = opt(n) +
n + 1), so this is n's optimum coming back.  The look-ahead start, which n may use, beats n's own start in 7 of the 110
(462, 736, 837, 1020, 1107, 1110, 1290) and reaches P\* in 1 (1290).  Starting at the optimum would save at most what
a start below costs, 0.9-1.8 times the proof at 10-100 below ([slope](../what-drives-the-slope-2026-10/README.md),
section 4).

**5. The gap is the same across a chain step.**  The 37 exhausted games of 880-1299 followed by a chain game have the
same GF(2) root gap as that game: the edit adds the same to the bound and the optimum.  More broadly, 77 of the 143
exhausted games of 880-1300 fall into 32 runs of two to five nearby games (no two consecutive more than 6 apart) with
equal gaps.  The one-thread proofs of a run beyond its dearest add up to 51,990,258 of the 262,711,582 proof nodes of
the 143 games (19.8%), the ceiling of a search that would prove a run's shared part once.  Equal root gaps do not make
equal trees (proofs within a run differ by up to ×3.9, median ×1.31), and no mechanism was built.

**6. Equivalent problems, a wider same search: almost nothing.**  Over 2..1300 the test finds the 545 equal problems
that `same_search` finds, and 8 equivalent ones more: 21 (to 18), 51 (50), 111 (110), 115 (112), 119 (118), 141 (140),
183 (182), 185 (184), each with the same P\*, all below 186, and exhausted in 18 nodes in all.  The near repeats it
misses (1083 after 1080, 1183 after 1180, 1209 after 1208) have the same P\* and candidates but differ in one hive or
one move of frame 0.  A finer test could save at most what such games cost: among the exhausted games with an earlier
game within 8 of the same P\* and as many candidates, 14 games of 1..1000 (11,262 nodes) and three of 1001..1300
(643,010 nodes).

**7. A chain that comes back down.**  A chain could solve 1024 before 1023 (seeded from 1022) and conclude 1023 from it:
Π_1024 relaxes Π_1023 and 1024's promotions are 1023's optimum.  1024's proof costs the same from 6 below its optimum
(one thread: 1,597,518 nodes at δ = 0 and 1,568,913 at δ = −6), so the saving is 1023's search: 1,730,169 of the
315,916,127 nodes (0.55%) and 5.3 of the 975 s of the exhausted searches of 1001-1300.  No other game of 1001-1300
changes; whether the same happens at 1536 or 2048 is beyond the chain.

## Decision

Going higher and coming back down does not make the expensive games cheap, and nothing was built.  The exact relations
that exist are the prime formula (whose problem is n's) and changes of the top frame (1023 in 1001-1300), and a proof
kind for the last would save 0.55% of 1001-1300, not worth the reordering of the chain it needs.  What the study does
show is that neighboring problems share their hard part: the edit between them changes the bound and the optimum alike,
so the GF(2) gap is the same across every chain step after an exhausted game, and equal across runs of nearby searched
games that hold a fifth of the proof nodes of 880-1300.

One avenue was left open: one tree that proves a run of such neighbors together could save up to a fifth of the proof
nodes.  It was measured later, with the GF(2) bound in the search: the trees of consecutive games share 0.15% of their
steps, and a run costs at least its dearest game, so perfect reuse saves 18-29% of the nodes at best ([carrying a
search's learning](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md)).  Since every exact relation
preserves the gap, the slope is set by the shared core's gap, and closing it (the GF(2) bound in the search) is what
changes the cost of these games.

## Commits

* `e9dcd8e` (2026-10-13): this entry.  The study's code is not kept.
