# Same search

Part of the [design](../DESIGN.md).

Moving from one board to the next often leaves the search untouched.  The new number lands in the part of the game that
setup settles.  The frames that the search sees, and their candidates, stay as they were.  The game then poses exactly
the same search problem as an earlier one, so it has the same best promotions.  The tactic `same_search` (proven, on by
default) recognizes this and reuses the earlier game's promotions instead of searching again.  About half of the
committed chain's games that need a search are such repeats ([results/README.md](../../../results/README.md#strictjsonl)
has the counts).  Skipping their searches saves a large share of a chain's search time; [the
entry](../../notebook/same-search-2026-10/README.md) has the measurements.

*The search problem.*  A game's *search problem* Π is what setup gives the search, less what only changes how the search
runs.  It has four parts:

* the model: the factor graph among the frames' numbers, and each number's home frame;
* the root: every frame's moves, factors and freedoms;
* the candidates;
* `max_total`, the most promotions the frames' freedoms allow.

The searcher also gets the tactics (`search::prepare`), the ceiling and the seed (`search::search`).  They change how
the search runs, not
what it finds ([search.md](search.md)).  The greedy set and the matching graph are functions of Π.  The best promotion
sum P\*(Π) is the largest sum of a *promotable* set of Π: a set of at most `max_total` candidates that can be promoted
from the root, keeping every frame winnable and its freedoms at least 0.

Which sets are promotable depends only on the frames and on the graph's edges among their numbers.  Promotion and
reduction see only the numbers in the frames.  An edge is the maximal-factor relation (y = x/p, p prime), a property of
the two numbers that does not depend on n.

The type `search::Problem` writes Π out canonically, as numbers, so that the form does not depend on the set width.  It
lists the frames front first, each with its freedoms, moves and factors; then the candidates and `max_total`; then, for
every number of the frames, its home frame and the lists of maximal factors and maximal composites that the model's
graph keeps for it.  Those lists name some numbers that reduction removed and the search never sees, which errs on the
side of telling problems apart.  Equal forms are equal problems, with the same promotable sets and the same P\*.  A
results line records the form's 64-bit digest (`setup.digest`), but nothing rests on the digest alone ("Finding m",
below).

*Which proofs settle P\*.*  A repeat is only as good as the earlier game's proof.  Four kinds of proof of a game m
establish P\*(Π_m):

* `exhausted`: the search explored the whole tree of Π_m.  So the sum of m's promotions is P\*(Π_m), given that the
  pruning is exact, which every exhausted proof rests on.
* `cap`: m's promotions S are promotable and sum to ceiling(m) = m + score(m − 1) − in_the_bag(m), the ceiling as a
  promotion sum.  So P\*(Π_m) ≥ ceiling(m).  Conversely, every promotable set T of Π_m is a legal game of m scoring
  in_the_bag(m) + sum(T).  This is the soundness of the frame model (THEORY.md, Theorem M).  Every search relies on it
  each time it offers an incumbent, and the referee replays every game built from one; none has ever failed.  So
  in_the_bag(m) + P\*(Π_m) ≤ opt(m) ≤ m + opt(m − 1) = m + score(m − 1).  The last step needs the game of m − 1 to be
  optimal: the proof by the ceiling rests on that, and m's assumptions record it.  So P\*(Π_m) = ceiling(m), the sum of
  m's promotions, whatever tactics shaped the search.
* `same_search`, by induction: m repeats a game k with Π_k = Π_m whose proof settles P\*(Π_k), and m's promotions are
  k's.
* `known`, where an upper bound of the bounds store ended m's search: the search found a promotable set of the largest
  sum the bound allows, so no set sums to more, given the bound, whose assumptions m's line records
  (`Proof::settles_search`).  A game only taken from a known source, with no search, sets none up.

No other proof settles P\*.  A trivial game has no candidates and sets no search up.  A prime game is the prime
formula's and sets none up either.  Neither is ever repeated, and the verifiers refuse a repeat that names one.  An
unfinished search settles nothing.

*The repeat.*  Suppose game n poses Π_n = Π_m, and m's proof settles it.  Then P\*(Π_n) = P\*(Π_m) = P, the sum of m's
promotions S, and S is promotable in Π_n.  The game of n is S played on the board of n (`solution_for_promotions`).  It
goes through `conclude` like any game.  It must replay legally, be over, and open with the largest prime.  It must score
in_the_bag(n) + P, stay within `score(n − 1) ≤ score(n) ≤ n + score(n − 1)`, and match optimal.json where the table has
n.

The game is optimal on the premise of an exhausted proof of n: that the best game of n scores in_the_bag(n) + P\*(Π_n),
which is U at n ([frame-model.md](frame-model.md)).  Only P\*(Π_n) is established differently, by m's proof instead of a
search.  So the proof `same_search` rests on m, not on the game of n − 1, and it assumes what m assumes.

*Finding m.*  Before it searches game n, the solver looks for an earlier game whose problem is Π_n.  It looks among the
completed games of the results file and of the run, in the `SAME_SEARCH_WINDOW` = 8 games before n, latest first,
for one that set a search up and whose proof settles it.  It does not look when the search resumes from a checkpoint, or
with `force_search`.

The digest finds candidates; the whole problem decides.  A game whose recorded digest differs is passed over.  For one
whose digest agrees, the solver sets its problem up again from its n, which takes milliseconds
(`board::search_problem`), and compares it with Π_n whole.  When the run set that board up itself, the solver takes the
problem from memory instead: the problem its setup posed, which is the same problem.  The first equal one is m.

Setting m up again with this build stands for the problem that m's own build searched.  The digest that m's build
recorded ties the two: a problem with the recorded digest is that problem, barring a 64-bit collision.  The problem set
up again must also have the sizes that m's line records: frames, candidates and `max_total`.

In the committed chain, every repeat is of the last game before it that set a search up: n − 1, or n − 2 past a prime.
So the window loses nothing there.  It also keeps to eight the setups that a range starts with, and it makes m a
function of the window's games.

*The verifiers.*  The first, `taxman verify-results`, checks a repeat's line against m's:

* m is a completed game on an earlier line of the same file, and its proof settles its search;
* the line's record of m (proof, assumptions, digest) is m's;
* the promotions are m's and sum to the recorded promotion sum;
* both problems, set up again from the two values of n, are as large as their lines record, have the recorded digest,
  and are equal;
* both games score their board's in_the_bag plus that sum.

It then derives the proof and the assumptions (m's) as for any line.

The second, `solver/tools/check_results.py`, shares no code with the solver and cannot build the model.  It checks the
same structure from the lines alone: m is earlier, completed and settles its search; the recorded digests and sizes are
equal; the promotions are equal; score − in_the_bag is equal; the assumptions are inherited.  It says that it takes the
problems' equality on trust, as it takes an exhausted search.
