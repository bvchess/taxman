# The ledger: bounds between boards

Part of [the theory](../THEORY.md).

The lemmas below pass bounds on opt(n) between games, both ways, so that many games can be proven at once.  The chain
proves the game of n from the game of n − 1 (the ceiling, the prime formula) or by a search of its own.  The ledger
(`solver/frame/examples/ledger/`) keeps, for many boards at once, a lower bound L(n), the score of a game it holds, and
an upper bound U(n) ≥ opt(n), and passes bounds between boards by the lemmas below; a board is proven when the two meet.
Each lemma is an inequality between optimal scores, with the games that carry the lower bound across; the upper bounds
cross by the inequality alone.  A *finished game* of n is one the referee (`replay_finished`) accepts: legal, over (no
legal move left), and opening with the largest prime ≤ n.  Every optimal game is finished (Lemma A of
`accelerate::prime_shortcut`), so opt(n) is the best score of a finished game, and every lower bound of the ledger is
the score of a finished game, replayed before it counts.

**Lemma L1 (monotone).**  The best score never falls: opt(n − 1) ≤ opt(n).  If n is composite and G is a finished game
of n − 1, then G, followed by n if n is still legal after it, is a finished game of n scoring at least ΣG.

*Proof.*  n divides no other number of the board 1..n, so G's moves are legal there with the same taxes and leave the
same pot with n added.  No number below n has a divisor in that pot (G is over, and n divides none of them), so n is the
only move that can be left, and playing it only takes numbers out of the pot: after it, or without it, the game is over.
G opens with the largest prime ≤ n − 1, which is the largest ≤ n since n is composite.  For a prime n the inequality
holds too (Lemma L3 gives more).  ∎

**Lemma L2 (the ceiling, and the game below it).**  No game of n scores more than the ceiling: opt(n) ≤ opt(n − 1) + n.
If n is composite and G is a finished game of n, then G without the move n, followed by legal moves for as long as there
are any, is a finished game of n − 1 scoring at least ΣG − n (ΣG if G does not play n).

*Proof.*  Call G' the moves of G other than n.  Before each move of G', its pot holds the pot of G before the same move,
less n: both start full; a move of both takes the same number and its divisors out of each pot; G's move n takes numbers
out of G's pot only.  So a move of G', which paid in G with a divisor in G's pot, has that divisor in its own: G' is
legal on 1..n − 1.  The moves added after it are legal by choice and leave the game over.  It opens as G does, with the
largest prime ≤ n, which is not n and so is the largest prime ≤ n − 1.  Applied to an optimal game of n this is opt(n
− 1) ≥ opt(n) − n, the ceiling of Theorem 9 and `board::promotion_cap`.  ∎

**Lemma L3 (the prime formula, both ways).**  Let p be prime and q the largest prime below it.  Then opt(p) = opt(p −
1) + p − q.  A finished game of p − 1 with its first move q exchanged for p is a finished game of p, and a finished game
of p with its first move p exchanged for q is a finished game of p − 1; the score changes by p − q and by q − p.

*Proof.*  The equality is Theorem 9 (prime), proven again with the exchange in `accelerate::prime_shortcut` (Lemmas A
and B there).  The exchange needs of the game only that it opens with q (with p), which a finished game does, not that
it is optimal: p and q are inert on both boards (2q > p by Bertrand's postulate, p being the next prime after q, and p
is the largest number), each takes only 1 when it opens, so every later move keeps its payer, and at the end the pots
differ only in q and p, which have no divisor left once 1 is gone, so the game stays over.  (p = 2: the game of 1 has no
move, and [2] is the game of 2.)  ∎

The design of the ledger stated the formula as opt(p − 1) + p − 1; it is + p − q, and the construction needs no optimal
game, only a finished one.

**Lemma L4 (repeats).**  If boards n and m pose the same search problem, Π_n = Π_m (solver/docs/design/same-search.md),
then every promotion set S of it, played on the board of n (`solution_for_promotions`), is a legal game of n scoring
in_the_bag(n) + ΣS; and, given U at n, opt(n) − in_the_bag(n) ≤ opt(m) − in_the_bag(m).

*Proof.*  The first is the soundness half of Theorem M, which every search relies on when it offers an incumbent.  For
the second, equal problems have the same promotion sets, so P\*(Π_n) = P\*(Π_m); by the soundness at m, in_the_bag(m) +
P\*(Π_m) ≤ opt(m); and given U at n, opt(n) = in_the_bag(n) + P\*(Π_n) (the corollary of "The upper part").  ∎

So the lower bound crosses a repeat only as the promotions of a game that was made from them (a search's or the start's
game); the ledger also tries the moves of any other game that are candidates, and keeps the result only if the referee
accepts it, as any game.  The upper bound crosses as U(n) ≤ U(m) − in_the_bag(m) + in_the_bag(n), and rests on U at n
besides what U(m) rests on.

**Lemma D (the dual bound of a stopped search).**  Let a search of Π_n that a limit stopped have the incumbent worth I
(a promotion sum) and, in its final checkpoint, the open units u = (P_u, C_u): the promotions of a node and the
candidates its loop has still to try.  Then P\*(Π_n) ≤ max(I, max_u (ΣP_u + B_u)), where B_u is the heaviest common
independent set over C_u of Theorem 11's matroids contracted by P_u, of at most `max_additional` members.

*Proof.*  The search is exact (solver/docs/design/search.md): each pruning drops only sets no heavier than the
incumbent of its time (at most I), sets that cannot be promoted, and, with the tactic `symmetry`, sets that break an
inequality every heaviest set keeps (Theorem 13).  The checkpoint's units are the roots of every subtree not yet
explored, which is what makes a resume from it exact.  So a heaviest promotion set H that keeps the inequalities is
either no heavier than I, or is P_u ∪ F with F ⊆ C_u for an open unit u.  Then F is a completion of u's node: by Theorem
11 it is common-independent in the contractions, and it has no more members than the freedoms allow
(`bounds::max_additional`), so ΣF ≤ B_u.  ∎

The root's GF(2) bound, the upper end of `wall landscape`, is the case of a search stopped before its first node (one
unit, the root).

**Lemma F (the floor).**  Let a search of Π_n start from a *floor* F, a promotion sum: an incumbent worth F and no
promotion set, so that it prunes against F as against a set of that weight, and a set it finds heavier than F becomes
the incumbent as usual.  If the search completes and has found no set, then P\*(Π_n) ≤ F.  If it finds a set, that set
is a promotion set worth more than F, and if the search completes, the last one it found is worth P\*(Π_n).

*Proof.*  Let H be a heaviest promotion set that keeps the inequalities; it weighs P\*(Π_n) (Theorem 13).  The incumbent
only rises, so it is worth at least F at every moment.  Each pruning of the search drops only sets no heavier than the
incumbent of its time, sets that cannot be promoted, and, with the tactic `symmetry`, sets that break an inequality
every heaviest set keeps (Theorem 13), as in Lemma D; the search reads the incumbent's value and never its set, so a
value with no set prunes as a set of that weight does.  The set H is not of the last two kinds, so a completed search
either reached H and offered it, or dropped it as no heavier than the incumbent of that time.  Either way ΣH ≤ I, the
incumbent at the end.  If nothing was found, I = F and P\*(Π_n) = ΣH ≤ F.  If a set S was found, I = ΣS, and S is a
promotion set, so ΣS ≤ P\*(Π_n) = ΣH ≤ ΣS.  ∎

A floor at P\* is a search seeded with an optimal set, tree for tree; a floor above it prunes more, and a floor below it
finds an optimal set.  A floor at or above the root's bound, or the ceiling, ends the search at its first check.  For a
search that a limit stopped, Lemma D holds with the floor as the incumbent when nothing was found: P\*(Π_n) ≤ max(I,
max_u (ΣP_u + B_u)), I being the larger of F and the weight of the heaviest set found.

*In the ledger.*  Every lower bound is a finished game, so L(n) ≤ opt(n) holds by the referee alone.  Every upper bound
is a bound of the frame model at some board (a root bound, a dual bound, a completed search, a completed search under a
floor with nothing found: each bounds in_the_bag + P\*, which is opt given U at that board, by the corollary of "The
upper part"), carried by the lemmas, which hold unconditionally but for the second half of L4 (U at the board it is
carried to).  So a board whose bounds meet is proven within the frame model, as the committed games past the prover's
proofs are, given U at the boards its upper bound came through.  An L above a U would be a bug, or a counterexample to U
at one of those boards, since L is a real game.

## The near-repeat cut

A search can cut its own nodes with the bound of a game proven before.  Where a node's promotions leave, in the proven
game, frames no harder than the ones they leave in the game searched, every completion of the node can be promoted in
the proven game as well, so none is worth more than that game's best promotion sum.  Consecutive games often pose
nearly the same problem, one move swapped for another, say: a *near repeat*.  There the cut ends most of the search, as
[a notebook entry](../../solver/notebook/search/what-carries-from-one-games-search-to-the-next-2026-10/README.md)
measured.

*Words.*  A node of a search has the promotions P and the candidates C that its loop has still to try; its completions
are the promotion sets P ∪ X with X ⊆ C.  The *state* σ(P) of a promotion set P is the root's frames with P applied,
then reduced.  Applying P to a frame adds to its moves the members of P promoted into it, and takes from its factors
those promoted out of it.  Reducing drops a move left with one live factor together with that factor, until no such
move is left; on winnable frames every order of the drops ends at the same frames (the reduction lemma,
`solver/frame/src/model/peel.rs`).  The search keeps σ(P) as the node's state ([solver/docs/design/search.md, "Search
state and order"](../../solver/docs/design/search.md#search-state-and-order)).  A frame is *touched* by C when it is the
home h of a member of C, or the frame h − 1 that the member would be promoted into.

**The state lemma.**  A node's state decides which of its completions can be promoted.  Formally: let P be a promotion
set of a problem Π, and X a set of candidates of Π disjoint from P.  Then P ∪ X is a promotion set iff every member of X
is a factor of its home frame in σ(P), and every frame of σ(P) is winnable with X applied.

*Proof.*  A set of candidates is a promotion set iff each member is a factor of its home, a frame h ≥ 1 of the root, and
every frame of the root is winnable with the set applied: the count `max_total` follows from winnability ([the frame
model](frame-model.md#the-frame-model-is-exact-given-its-upper-part), step 5 of setup and the proof of Theorem M).  A
member of X with no home fails both sides of the lemma.  Fix a frame, and let R be the frame with P applied.  Reducing R
gives the frame of σ(P) by the drops (m₁, q₁), (m₂, q₂), …, each move mᵢ left at its turn with the single live factor
qᵢ.  Applying X to R adds moves and takes factors away.  By the reduction lemma, a drop (mᵢ, qᵢ) keeps whether the frame
is winnable after any such edits that leave qᵢ in place.  If X leaves every qᵢ in place, R with X applied is therefore
winnable iff the frame of σ(P) with X applied is.  Otherwise let qᵢ be the first that X takes away.  The drops before it
keep whether the frame is winnable with X applied, and after them mᵢ has the single live factor qᵢ, which X takes away:
mᵢ is left with no factor, so R with X applied is unwinnable.  And qᵢ, a member of X, is no longer a factor of its home
in σ(P).  In σ(P) a member of X is missing from its home only if a drop took it, since applying P takes away P's own
members only.  So the two sides of the lemma hold or fail together.  ∎

**Lemma R (the near-repeat cut).**  Let the promotions and the candidates of a node of game N's search be candidates of
a game M proven before, and let its promotions leave in M frames no harder than in N, wherever its candidates can
reach.  Then no completion of the node is worth more than M's best promotion sum.  Formally: let M and N be games, B a
proven bound with P\*(Π_M) ≤ B, and a node of a search of Π_N with the promotions P and the candidates C.  Write σ_M(P)
and σ_N(P) for the states of P in the two problems.  Suppose that

1. every member of P and of C is a candidate of Π_M, and every member of C has the same home in Π_M as in Π_N;
2. P is a promotion set of Π_M; and
3. every frame j touched by C, with the moves A_M and the factors F_M in σ_M(P) and the moves A_N and the factors F_N in
   σ_N(P), has A_M ⊆ A_N, and every factor in F_N but not in F_M is no member of C, and is a maximal factor of no member
   of A_M and of no member of C of home j + 1.

Then every completion of the node that is a promotion set of Π_N is one of Π_M as well, so it is worth at most B.  If
the incumbent is worth at least B, no completion of the node beats it, and the search may cut the node.

*Proof.*  Let P ∪ X be a completion that is a promotion set of Π_N, with X ⊆ C.  By the state lemma in Π_N, every member
of X is a factor of its home h in σ_N(P), and every frame of σ_N(P) is winnable with X applied.  By conditions 1 and 2,
P ∪ X is a set of candidates of Π_M, P is a promotion set of Π_M, and each member of X has the same home h there; so by
the state lemma in Π_M it is enough to show the same two facts of σ_M(P).

*Each member x of X is a factor of its home h in σ_M(P).*  C touches the frame h, x is in its factors F_N, and x is in
C; so condition 3 puts x in F_M.

*Each frame j of σ_M(P) is winnable with X applied.*  If X does not touch frame j, the frame is as σ_M(P) has it, and
winnable: P is a promotion set of Π_M, and reducing keeps a frame winnable.  If X touches frame j, so does C.  Let X_in
be the members of X of home j + 1, which X makes moves of frame j, and X_out those of home j, which leave its factors.
Some order wins N's frame, the moves A_N ∪ X_in against the factors F_N less X_out; keep in it only the moves A_M ∪
X_in.  At its turn in the whole order, each kept move a found among F_N less X_out a maximal factor q of its own that
no move before it had taken.  Since a is a member of A_M or a member of C of home j + 1, condition 3 puts q in F_M,
and q is not in X_out.  No kept move b before a has taken q: q was still in N's frame at b's turn, and taking b takes
all of its maximal factors still there, so q is no maximal factor of b.  So each kept move can be taken at its turn,
and the kept order wins M's frame, the moves A_M ∪ X_in against the factors F_M less X_out.

So P ∪ X is a promotion set of Π_M, and Σ(P ∪ X) ≤ P\*(Π_M) ≤ B.  ∎

*What it rests on.*  The proof compares the two problems and nothing else: their frames, candidates and homes.  It needs
no statement U, and nothing of the boards outside the problems: the new number of N is a source, never a candidate, and
a completion is worth the sum of its members in either game.  So it holds with any root problem of the frame model in
place of Π_M, of a board with holes say, whose best promotion sum has a proven bound.  It does not treat the extended
problem's one-sided candidates.  The order of the promotions does not matter, by the state lemma.  The incumbent need
not be a set, as a floor serves (Lemma F).  The cut drops only sets that cannot be promoted in Π_N and sets no heavier
than the incumbent of its time, as a bound does, so the exactness of the search, Lemma D and Lemma F hold with it; it
learns no conflict and no nogood.  The node's candidates may be fewer than the root's, since the search drops some by
the freedoms, by the conflicts and nogoods it knows, and with the tactic `symmetry` by Theorem 13: the completions that
hold them are dropped by those rules, and the cut speaks of the rest.

*Standing.*  Proved.  A search that cuts by it proves its answer given that B bounds P\*(Π_M), so the answer rests on
what B rests on.  Where a search of the same problem Π_M, with the same digest, completed, B can be what that search
proved of P\*(Π_M), and the cut adds nothing to what the answer rests on.  Otherwise B can be score(M) − in_the_bag(M),
with M's recorded score.  Every promotion set L of Π_M gives a game worth in_the_bag(M) + ΣL (Theorem M), so P\*(Π_M) ≤
opt(M) − in_the_bag(M) ≤ B wherever M's score bounds opt(M).  The answer then rests on M's upper bound too: on what M's
proof assumes, and on U at the boards that bound came through, as the ceiling of a game rests on the game before it.

*What a search checks.*  The search follows the lemma word for word.  At setup, with the cut on:

* M is proven, and B is one of the two bounds above, read from M's result.  A provisional game of M, the best of an
  unfinished search, bounds nothing: a search started before M is proven may not cut until it is.
* Π_M is set up as Π_N is.  The candidates of Π_N that are candidates of Π_M are marked, and among them those with the
  same home in both.
* The incumbent, or the floor, is worth at least B.  The incumbent only rises, so this holds at every later node.

At a node, with its promotions P, its candidates C and its state σ_N(P):

1. every member of P is marked, and every member of C is marked as having the same home;
2. P, promoted from the root of Π_M, gives σ_M(P) (`FrameGraph::promote_all_using`);
3. on every frame j that is the home h of a member of C, or h − 1: every move of σ_M(P)'s frame j is a move of
   σ_N(P)'s; and every factor y of σ_N(P)'s frame j that σ_M(P)'s lacks is not in C, and no number in its list
   `composites(y)` of Π_N's factor graph is a move of σ_M(P)'s frame j or a member of C.  That list holds y's maximal
   composites among the moves of frame j and the factors of frame j + 1 as setup forms them, so it holds every move of
   σ_N(P)'s frame j and every member of C of home j + 1 that y pays for, and no other member of C.

*Why each check is needed.*

* *Candidates of Π_M, not only numbers that Π_M's model can promote.*  The model promotes any factor of a frame h ≥ 1
  that keeps the frames winnable, candidate or not, and the second check at a node asks no more.  A set holding a number
  that is no candidate of Π_M is no promotion set of it, and P\*(Π_M) does not bound it: at n = 98 the model would
  promote 22, which only the candidate filter keeps out ([the frame
  model](frame-model.md#the-frame-model-is-exact-given-its-upper-part)).
* *The same home.*  A promoted number leaves its home for the frame in front of it; with another home in Π_M it would
  change frames that the check did not compare.
* *A proven B.*  The completions are worth at most P\*(Π_M), so the cut is exact only when the incumbent reaches a bound
  of it.  A provisional game of M is worth P\*(Π_M) or less, and cutting at its value could drop a heavier set.

*What needs no check.*  The freedoms: a frame's freedoms are its factors less its moves, never negative where it is
winnable, so the winnability that the lemma compares implies them.  The factor graph: the model lists for each number
its maximal factors among the factors of the frame it moves in, as setup forms that frame
(`solver/frame/src/model/provider.rs`), and a state's frame keeps only some of those factors, so on the frames compared
both problems' lists are the maximal factors among the frame's factors, as winnability reads them.  The number of
frames, and the frames that C does not touch: those stay as σ_M(P) has them, winnable.

*Cases.*

* The check of the notebook's study (`study-rules.patch` beside the entry) asks of every frame j of σ_M(P) the moves and
  the freedoms of frame j of σ_N(P), and the same factors but for those that are not in C and that no move of the
  frame and no member of C of home j + 1 has as a maximal factor, with every candidate of Π_N checked at setup to be a
  candidate of Π_M with the same home.  It implies conditions 1 to 3.
* A repeat, Π_N = Π_M (Lemma L4): conditions 1 to 3 hold at the root, where P is empty, so a search whose incumbent is
  worth P\*(Π_M) ends there.
* Condition 3 allows N's frame moves that M's lacks, and factors that pay only for those.  Condition 1 holds at the
  nodes whose promotions and candidates leave out the candidates that Π_M lacks, as where P\* rises from M to N.  The
  notebook's entry lists both as ways to widen the cut.
