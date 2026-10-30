# Price reuse

Part of the [design](../DESIGN.md).

With the tactic `price_reuse`, on by default, the choice of the conflict keeps the prices it makes and takes them
again, and a node's later steps and its descendants cost the CPU of a few prices instead of all of them.  [Conflict
branching](search.md#conflict-branching) prices the members of a node's heaviest set at every check (`Gf2Bound::prices_excluding`):
the price of a member is what leaving it out costs the bound, read off the heaviest set's exchange graph, and the node
branches on the conflict of least weight by those prices.  A node's loop checks again at every step, and a step changes the
node little (it goes on without the member it branched on); a child's first check follows its parent's, and its problem
is the parent's with some members promoted.  So a price made at one check is nearly the price at the next.

*What is kept.*  Each depth of a searcher's path holds the prices of its node (`PriceMemo` in `search::price_memo`),
keyed by the GF(2) bound's number for the node, and the ordered conflicts of the choice (`conflicts_ordered`).

* **A node's later steps** take the prices made at its earlier steps, and make only those of members that have none.
  These steps pay most: the node's heaviest set has changed since it priced, so its pricing cannot carry cycles from its
  parent's, and each price is a search of the exchange graph.
* **A child's first check** takes its parent's prices, for up to six generations down a line, as long as the bound
  (the promotions and the GF(2) bound together) has not fallen more than 40 since they were made, and makes the prices
  it lacks.  The child's pricing was cheap already, since it carries its parent's cycles, so the gain here is smaller.
* **The ordered conflicts** of the check before are taken too, those still within the heaviest set, and are made again
  when none is.
* **A check that makes no price** gives the pricing of its children the cycles of its parent's
  (`Gf2Bound::inherit_pricing`), so that their pricing, when it does search, still carries cycles from above
  ([gf2-bound.md](gf2-bound.md)).

The schedule is in `Reuse::STANDARD`: no limit on the checks of a node or on the bound's fall within it, six
generations, a fall of 40.  The schedule trades the staleness of the ranking against its cost.  Prices that are never made
again raise the steps of the hard games by 17%, and a schedule that makes them at every check is the tactic off.

*Why it is exact.*  The prices only rank conflicts, and the node branches on a true conflict of its state whatever the
ranking, as [conflict branching](search.md#conflict-branching) argues.  A conflict kept from an earlier check of the node,
or of an ancestor, is still a conflict: a stuck set survives every larger set of promotions ([the residual theorem
(P5)](../../../docs/theory/playability.md#the-residual-theorem)), and the node's promotions only grow along a line.  The
choice takes it only while it lies within the heaviest set.  Carried cycles are checked where the pricing uses them
(gf2-bound.md).  A checkpoint records the tactic and does not bind it ([trust.md](trust.md)).

*What it does.*  On the 315 units of games 4920-4949, which are subtrees of the games' searches, a search with the
tactic takes 0.82 of the CPU of one without it in the hard games and 0.86 in the easy, with 0.86 and 0.97 of the steps:
the choice falls from 23 to 25% of the CPU to 17 to 18%.  Every one of the 14 games falls.  The steps change with the
ranking in either direction, so the gain in CPU is surer than the gain in steps.  A lower gate with the tactic, 150, takes
0.81 and 0.92 of gate 300's CPU without it, behind gate 300 with the tactic.  [The
measurements](../../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-price-reuse-in-the-choice-q6)
have the settings tried.
