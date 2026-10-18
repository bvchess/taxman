# Going higher and coming back down (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The chain uses the game below: the ceiling opt(n) ≤ n + opt(n − 1) (the cap), the prime formula,
`same_search` and the seed.  Can the games above n (their optima, optimal games, search problems and proofs) tighten
n's upper bound, raise its incumbent or settle it, so that an expensive game is derived from a cheaper one above it?
Since a proof's cost is set by the root's gap ([slope](../what-drives-the-slope-2026-10/README.md); the GF(2) gap for a search
built on the [GF(2) bound](../matroid-intersection-bounds-2026-10/README.md)), a relation from above counts if it beats that gap
where the searches are dear.

*What was kept.*  Nothing but this entry; the solver is unchanged.  The study's code lived on a branch that was never
pushed and has been deleted, as the project does with research branches once they are harvested.  It was two example
programs (one that set up the boards n and n + 1..n + 3 and measured how their search problems relate and what each
optimal set is worth on the other board; one that tests search problems for equivalence), a table script and the data
they made, and one-line mentions of the programs in DESIGN.md and README.  The method is written out below to be
redone, with the one result that lived only in the code, the sufficient condition for one problem to relax another,
stated and proved.  What it read is on master: the chain (`results/strict.jsonl`), the GF(2) root gaps
(`solver/notebook/matroid-intersection-bounds-2026-10/roots.jsonl`) and the one-thread proofs (`solver/notebook/what-drives-the-slope-2026-10/slope_proofs.jsonl`).

*Method.*  No solver code changed; one search was run (section 7).  For each board n of 2..1297 (primes and trivial
boards included) and each m = n + 1..n + 3, both boards were set up as `taxman solve` sets them up (default tactics:
the root's frames after reduction, the candidates with their home frames, `max_total`, `in_the_bag`), and the optimal
games read from the chain (a game's P\* is its score less `in_the_bag`).  Recorded for each pair:

* whether the two search problems are *equal* (the test of `same_search`, DESIGN.md
  "[Same search](../../docs/DESIGN.md#same-search)") and whether each *relaxes* the other (below);
* how m's root differs from n's, frame by frame: the moves and factors m has and n has not and the other way, the
  change of the frame's freedoms, the candidates gained and lost, the change of `max_total`;
* what each optimal promotion set is worth on the other board: the set's members that are candidates there, whether
  they can all be promoted from the root, and the sum after *repairing* (dropping the smallest member of a conflict
  until the rest can be promoted) and *filling* (adding the largest candidates that keep it promotable);
* for n, the incumbent its chain search starts from (the search stopped before its first node, seeded from the game of
  n − 1), and a *look-ahead* start: the incumbent the search of m starts from when seeded from the game of n − 1, which
  an incumbent for n may use (n's own optimum, and m's, may not be), repaired and filled on n's board.

A game is *exhausted* when the chain proved it by exhausting the search (416 of 1..1300).  The tables come from a
script over those records and the files named above; "nodes" are the chain's, except in section 5, which uses the
one-thread proofs.

*When one problem relaxes another.*  Say problem Π′ *relaxes* Π when every promotion set that Π can promote from its
root, Π′ can promote from its own.  Then P\*(Π) ≤ P\*(Π′), and every optimal set of Π is feasible in Π′.  The study
used this sufficient condition, checked on the two roots as the solver holds them (reduced, with home frames; a problem
without candidates is relaxed by every problem):

* (a) every candidate of Π is a candidate of Π′ with the same home frame, and Π′ has at least as many frames; and
* (b) for every frame i of Π: Π′'s frame i has every factor of Π's frame i, and each move a of Π′'s frame i that Π's
  frame i lacks has a maximal factor y that is a factor of Π′'s frame i but not of Π's, and that no other number which
  could be a move of frame i lists among its maximal factors: no other move of Π′'s frame i, and no candidate of Π of
  home i + 1 (which becomes a move of frame i when promoted).

*Proof.*  A set T of candidates can be promoted from a root exactly when each member is a factor of its home frame
h ≥ 1 and every frame i is winnable with T applied all at once: its moves plus the members of T with home i + 1, its
factors minus the members with home i, with no reduction in between (the module comment of `model::promote`;
reduction never changes winnability).  Let T be promotable in Π.  By (a) its members are candidates of Π′ with the
same homes, so T applies to Π′, and we show that each frame of Π′ is winnable with T applied.  Frames of Π′ above
Π's top are untouched by T (no member has its home there or the one above) and winnable at the root, since the empty
set can be promoted.  Take a frame i of Π; let B be Π's frame i with T applied, winnable by hypothesis, and R Π′'s.
Write T_h for the members of T with home h.

* R's factors are Π′'s factors of frame i less T_i, and so contain B's, which are Π's less T_i, by (b).
* R's moves are Π′'s moves of frame i and T_{i+1}; B's are Π's and T_{i+1}.  Let A be the moves of Π′'s frame i that Π's
  lacks.  No a in A is in T_{i+1} (a is a move of Π′'s frame i, and T_{i+1} are factors of its frame i + 1), so the
  moves of R outside A are moves of B.
* Each a in A has the y of (b).  It is a factor of R (it is not a factor of Π's frame i, and T_i consists of factors of
  Π's frame i), and no other move of R lists it (R's other moves are moves of Π′'s frame i or members of T_{i+1}, which
  are candidates of Π of home i + 1).  So y is private to a, and stays private as other moves are dropped.

A frame is winnable exactly when the *peel* drops all its moves: repeatedly drop a move that has a private live factor
(it can be taken last), in any order (`model::peel`).  Peel R by dropping all of A first.  What is left is R without
A: moves of B, over factors that include all of B's.  Winnability only gets easier with fewer moves and more factors
(a winning order of B, restricted to the moves left, still finds a factor of each move in the pot, which only has
more in it), so the rest of the peel drops every move, and R is winnable.  ∎

*Equivalent problems* (section 6) are tested by a separate method, described there.

**1. What the game above proves: opt(n) ≤ opt(n + K) − Σ m/2 − (q − P).**  The sum is over the even m in
(n, n + K], P and q are the largest primes at most n and n + K, and K < (n + 3)/2.  *Proof.*  Let G be an optimal
game of n; it opens with P (every optimal game opens with the largest prime: Lemma A in the proof of
`accelerate::prime_shortcut`).  For an even m in (n, n + K], f = m/2 is above n/2, so it divides nothing else on 1..n,
and its multiples up to n + K are f and m (3f > n + K, since K < (n + 3)/2).  On 1..n + K, play G with each such f
deleted (if G played it) and, if q > P, with its first move P replaced by q (as in the prime shortcut; q > n, as no
prime lies in (P, n]); then every such m.  Deleting a move leaves more in the pot, so every later move of G keeps a
proper divisor; q is inert (2q > n + K) and removes only 1, like P; each f stays in the pot until its m, which it pays,
because the only multiples of f up to n + K are f and m, and no m removes another's f: m′/f = 2m′/m lies in (4/3, 3),
so it is an integer only for m′ = m.  The game scores at least opt(n) + Σ (m − f) + q − P (at least: when f was not
played, or is P, the deleted score was not there to lose).  ∎  With K = 1 it is opt(n) ≤ opt(n + 1) − (n + 1)/2 for odd
n, and the prime shortcut's equality for a prime n + 1; nothing for an odd composite n + 1 = pf (p ≥ 3 its least
prime), where making room for it can cost the moves f, 2f, …, (p − 1)f, as much as it brings.

On the chain it holds for every n and K checked (2..1299, K up to 39), and it is tight 223 times at K = 1 (the 210 odd
primes and 13 even m), 11 at K = 2 and once at K = 3.  The 13 even m have opt(m) = opt(m − 1) + m/2: 8, 12, 16, 24, 32,
48, 64, 96, 256, 384, 512, 768, 1024.  Of the 416 exhausted games, the bound from n + 1 settles 107: 106 through a prime
n + 1, whose search problem is n's own (each of the 204 primes whose predecessor sets a search up poses that problem,
so nothing is learned), and 1023 through 1024.  The slack is large elsewhere: an even n + 1 above an exhausted odd game
gains about 70, 220 and 450 more than (n + 1)/2 at the quartiles (118 games).

**2. Neighbouring problems differ by a local edit, and the edit runs one way.**  From odd n to n + 1 = 2f, f leaves
the moves of a frame h (a freedom more there) and becomes a candidate factor of frame h + 1 that pays for the new move
2f; at 2^k a new top frame appears.  From even n to an odd composite n + 1, n + 1 joins frame 0 or 1 as a new move,
or with a new hive (2f and 3f over f), a freedom less, at times while another hive leaves the frame.  So after an odd
n, n + 1 *relaxes* n or poses the same problem (all but 6 of 434), and after an even n, n relaxes n + 1 or neither
(pairs of games that are neither prime nor trivial, 2..1297):

| n | equal | n + 1 relaxes n | n relaxes n + 1 | neither |
|---|---|---|---|---|
| odd | 226 | 202 | 0 | 6 |
| even | 244 | 0 | 85 | 106 |

A relaxation gives P\*(n) ≤ P\*(m), a cap from above in the frame model (and P\*(m) ≤ P\*(n) when n relaxes m).
Over the 2708 and 1582 relaxing pairs within three of each other (equal problems included), no P\* breaks it and every
optimal set of the relaxed problem can be promoted in the relaxation.

**3. A cap from above is exact only where the problem above is no cheaper.**  172 of the exhausted games have a
relaxation among n + 1..n + 3 that is not their own problem; its cap (P\* of the relaxation) is exact for 5: 30, 45,
510, 765 and 1023, each at a change of the top frame that is worth nothing.  At 32, 512 and 1024 a new top frame
appears (the move 3·2^(k−2) over the factors 2^(k−2) and 3·2^(k−3): 24 over 8 and 12 at 32; candidates never worth
promoting); at 48 and 768 the move 24, resp. 384, becomes a factor (a freedom more in the top frame).  Elsewhere the cap
misses by the worth of what the edit frees (a freedom of frame h, the candidate f), and a cap that misses prunes
nothing: a node is cut by its own bound, and the cap only stops a search whose incumbent reaches it.  At 23 of the 66
games of 880-1297 that have one, the cap from above is closer than the root's GF(2) bound (1235: 12 against 164, 1254
and 1281: 24 against 204 and 206, 1110: 8 against 134), and at the dearest games it is absent or farther.  The 12
dearest exhausted games of 1001-1300 (the root's GF(2) gap is the matroid study's `roots.jsonl`, less P\*; of the 20
dearest 11 have a cap from above and 4 are closer than GF(2)):

| n | nodes (chain) | GF(2) gap | cap from above, its gap |
|---|---|---|---|
| 1025 | 7,953,672 | 147 | 323 |
| 1230 | 12,706,114 | 154 | 532 |
| 1232 | 53,015,201 | 190 | none |
| 1235 | 7,641,761 | 164 | 12 |
| 1236 | 9,727,370 | 164 | none |
| 1264 | 12,573,516 | 176 | none |
| 1269 | 7,708,479 | 167 | 148 |
| 1274 | 10,149,185 | 206 | none |
| 1287 | 24,699,434 | 282 | 309 |
| 1290 | 23,160,535 | 174 | 646 |
| 1295 | 30,184,598 | 174 | 648 |
| 1300 | 31,165,285 | 151 | not measured (1301 unsolved) |

Why it cannot be otherwise: a relaxation has more factors, fewer moves, more candidates, and new moves only with
private new factors (the condition above), so its matching graph and its GF(2) frame matroids contain those of the
problem it relaxes (a move with a private coordinate contracts nothing the others use), and its root bound is at least
as large: none of the 972 relaxing pairs with both root bounds known has a smaller GF(2) or matching bound.  So gap(m)
≥ gap(n) − (P\*(m) − P\*(n)): a problem above that is cheaper than n's own leaves a cap that misses, and an exact cap
comes from a problem at least as hard.  The cheap neighbours above are the restrictions (after an even n), and they
bound n from below only.  The relaxations from below do no better than the chain's cap: 70 exhausted games have one,
none exact and none tighter than the cap.

**4. Incumbents from above are n's own optimum, reflected.**  The chain's search starts below the optimum in 110 of the
415 exhausted games of 2..1297.  The optimal set of n + 1, on n's board, reaches P\* in 91 of them; but n + 1 comes
after n and is often built on it (98 of the 416 exhausted games are followed by a chain game, opt(n + 1) = opt(n) +
n + 1, whose optimal set is n's plus n + 1), so this is n's optimum coming back.  An incumbent n may use, the
look-ahead start of n + 1..n + 3 seeded from the game of n − 1, beats n's own start in 7 of the 110 (462, 736, 837,
1020, 1107, 1110, 1290) and reaches P\* in 1 (1290, from 1292's and 1293's).  Starting at the optimum would save at most
what a start below costs, 0.9-1.8 times the proof at 10-100 below ([slope](../what-drives-the-slope-2026-10/README.md), section 4).

**5. The gap is the same across a chain step.**  The 37 exhausted games of 880-1299 followed by a chain game (opt(n +
1) = opt(n) + n + 1) have the same GF(2) root gap as that game ([matroid](../matroid-intersection-bounds-2026-10/README.md), section
1): the edit adds the same to the bound and the optimum, so the hard part of the problem does not change, and solving n
+ 1 first would face n's gap.  More broadly, take the exhausted games of 880-1300 in order and cut the sequence where
two consecutive ones are more than 6 apart or have different GF(2) gaps: 77 of the 143 games fall into 32 runs of two to
five games with equal gaps (1235 and 1236, 1254 and 1256, 1274 with 1280, 1281 and 1284, 1290 and 1295, ...).  The
one-thread proofs of a run beyond its dearest add up to 51,990,258 of the 262,711,582 proof nodes of the 143 games
(19.8%).  That is the ceiling of a search that would prove a run's shared part once; equal root gaps do not make equal
trees (proofs within a run differ by up to ×3.9, median ×1.31), and no mechanism was built.

**6. Equivalent problems, a wider same search: almost nothing.**  Two problems with the same feasible sets have the
same P\*, whether or not they are equal; `same_search` sees only equal problems.  The test (run for each non-prime
non-trivial board of 2..1300 against the 8 earlier boards that set a search up, latest first) reduces a problem to a
*form* and compares forms.

* *The form.*  A set T of candidates is promotable exactly when each member is a factor of its home frame and each frame
  is winnable with T applied (the proof above).  A frame's winnability is a property of the bipartite graph between
  its *movers* (its moves and the candidates of home i + 1) and its factors, a mover listing its maximal factors (the
  model's lists, restricted to the frame's factors); so the feasible sets are fixed by the candidates (with homes) and,
  per frame, that graph with the candidates labelled.  The other numbers, the frame's moves and its factors that are
  not candidates, are *anonymous* and can be renamed freely.
* *Stripping.*  In each frame, peel the movers in the frame's *hardest state*, where every candidate of home i + 1 is a
  mover and every candidate factor is gone: repeatedly drop a mover that lists a live anonymous factor no other
  remaining mover lists.  What a peel drops there never matters: the factor it used is anonymous, so every state has
  it, and is listed by no other mover, which holds in any state with T applied (fewer movers, more factors), so the
  same drop is available at the same point, or the mover is a candidate that T does not promote and is absent.  With T
  applied the frame is winnable exactly when a peel of the movers the hardest state could not drop succeeds, over the
  factors they list.  The form of a frame is that remainder: the surviving candidate movers and anonymous movers, the
  factors they list (candidate and anonymous), and the edges.
* *Comparing.*  Two problems are equivalent when they have the same candidates with the same homes, the same number
  of frames, and in each frame an isomorphism of the forms that fixes every candidate (mover and factor) and carries
  the anonymous movers onto anonymous movers and anonymous factors onto anonymous factors, edges onto edges.  Then T
  is winnable in a frame of one problem exactly when it is in the other's, so the feasible sets, and P\*, are the
  same.  (`max_total` is a count every feasible set obeys, so it is not compared.)  The isomorphism is found by colour
  refinement and then checked: an anonymous mover starts with the colour of the set of candidate factors it lists, an
  anonymous factor with that of the candidate movers that list it; a vertex's next colour hashes its colour with the
  sorted colours of its anonymous neighbours, until the number of colour classes stops growing; the anonymous
  vertices of the two frames are paired by sorting on colour (a mismatch of the colour classes means not equivalent),
  and the pairing is *checked* edge by edge: every mover's list must map onto its image's list exactly.  Only a checked
  isomorphism counts, so a hash collision can only lose an equivalence, never invent one, and so can refinement on
  symmetric forms.

Over 2..1300 it finds the 545 equal problems that `same_search` finds, and 8 equivalent ones more: 21 (to 18), 51 (50),
111 (110), 115 (112), 119 (118), 141 (140), 183 (182), 185 (184), each with the same P\*, all below 186, and exhausted
in 18 nodes in all.  The near repeats it misses are not isomorphic: 1083 after 1080, 1183 after 1180 and 1209 after
1208 have the same P\* and candidates, and frame 0 (which still has 76-83 anonymous movers over 102-111 factors after
stripping) differs in one hive (578 and 867 over 289 at 1080, 722 and 1083 over 361 at 1083; 638 and 957 over 319 at
1208, 806 and 1209 over 403 at 1209) or one move (605 at 1180, 1183 at 1183), with every other frame equal.  Any finer
test of equivalence could save at most what such games cost: among the exhausted games with an earlier game within 8
of the same P\* and as many candidates, 1..1000 14 games (11,262 nodes), 1001..1300 three (643,010 nodes).

**7. A chain that comes back down.**  With every relation above, a chain could solve 1024 before 1023 (seeded from
1022) and conclude 1023 from it: Π_1024 relaxes Π_1023 and 1024's promotions are 1023's optimum.  1024's proof costs
the same from 6 below its optimum (one thread, the slope study's `prove` with the incumbent a value: 1,597,518 nodes
at δ = 0 and 1,568,913 at δ = −6), so the saving is 1023's search: 1,730,169 of the 315,916,127 nodes (0.55%) and 5.3
of the 975 s of the exhausted searches of 1001-1300.  Whether the same happens at 1536 or 2048 is beyond
the chain.  No other game of 1001-1300 changes.

*Conclusion.*  Going higher and coming back down does not make the expensive games cheap.  The games above bound n
through relaxations of its problem (for an odd n, n + 1 relaxes it or poses the same problem but for 6 of 434), and a
relaxation's root bound is at least n's own, so an exact cap from above comes from a problem at least as hard as n's,
and a cheaper problem above leaves a cap that misses and prunes nothing.  The exact relations that exist are the
prime formula (whose problem is n's) and changes of the top frame (1023 in 1001-1300).  Incumbents from above are
n's optimum reflected; the look-ahead that does not use it beats the chain's start in 7 of 110 and reaches P\* once.
What the study does show is that neighbouring problems share their hard part: the edit between them changes the bound
and the optimum alike, so the GF(2) gap is the same across every chain step after an exhausted game, and equal across
runs of nearby searched games that hold a fifth of the proof nodes of 880-1300.

*What it implies.*

* *Nothing to implement in the search.*  A proof kind for the exact relaxation from above (1023 from 1024) would
  save 0.55% of 1001-1300; not worth the reordering of the chain it needs now.
* *Shared cores (open)*: a search that proves a run of neighbours with equal gaps together (for a relaxing pair, one
  tree over the relaxation that keeps both problems' incumbents and cuts a node only when both bounds allow) could
  save up to a fifth of the proof nodes at 880-1300; whether the trees overlap enough is unmeasured, and is worth
  measuring only once the GF(2) bound is in the search, which will change the trees (it is the bound whose gap the
  runs share).  (Measured since, with the GF(2) bound in the search: the trees of consecutive games share 0.15% of
  their steps, and a run costs at least its dearest game, so perfect reuse saves 18-29% of the nodes at best: ["Carrying
  a search's learning"](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md).)
* *The bound is the lever, not the neighbours*: since every exact relation above preserves the gap, the slope is set
  by the shared core's gap, and closing it (the GF(2) bound in the search) is what changes the cost of these games.
