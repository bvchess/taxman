# Carried nogoods

Part of the [design](../DESIGN.md).

With the tactic `carried_nogoods`, off by default, a search takes the nogoods that another search learned, keeps those
that are nogoods of its own root, and uses them only to prune.  A *nogood* is a set of candidates that cannot all be
promoted from the root ([search.md](search.md#forced-members-and-nogoods)), so it depends on the root alone, and the
searches of neighboring games learn many of the same ones: their frames are nearly the same, and a bad patch of the
chain refutes the same hard core game after game.  Each searcher's own table forgets its nogoods every 50,000 steps
and starts empty in each game.  A *store* of carried nogoods keeps them, with three rules that carrying the tables
themselves broke ([carrying nogoods
over](../../notebook/search/carrying-nogoods-over-2026-10-measured-not-kept/README.md)):

* the store is apart from the searcher's table, and never gives the conflict a node branches on, so it does not steer
  the branching;
* every set is checked at the root before it is used, so the store holds only facts of this game;
* a question to the store costs the nogoods that watch the members it is about, not a scan of the store.

## What the search does with a store

The store is `model::CarriedNogoods`, made by `CarriedNogoods::validated` from sets of numbers (or `read` from a file,
one nogood a line, its members in decimal).  A set is kept when all its members are candidates and a peel at the root
finds a conflict within it (`FrameGraph::conflict_using`, the check by which the search learns its own conflicts), and
what is kept is that conflict: a subset of the set that leaves one of the root's frames unwinnable, minimal for that
frame.  The search uses the
store in three places, all of them prunings:

* **A unit's node** drops each candidate that some nogood has as its one member not promoted (`CarriedNogoods::start`).
* **A child** drops each candidate that some nogood has as its one member left once the child's promotions are made
  (`CarriedNogoods::promote`, in step 7 of [the default step](search.md#the-default-step)), beside the candidates that
  the searcher's table and the known conflicts rule out.
* **A check** ends a node, as a cut `refined`, when some nogood lies within its promotions and its forced members with a
  member among the forced ones (`CarriedNogoods::all_forced`, at step 2, before the searcher looks for a conflict): every
  completion that beats the incumbent promotes the forced members, and so the whole nogood.

*Watched members.*  Each searcher keeps `model::Watches` of the store along its path, as SAT solvers watch two literals
of each clause.  Each nogood watches two of its members that the node has not promoted.  A nogood that watches two such
members has two left, so a child can complete it only by promoting a watched member, and a child looks only at the
nogoods that watch what it promotes: each moves its watch to another member not promoted, or, with none, rules out the
member it still watches.  Backtracking costs nothing, since what a node has not promoted its ancestors have not promoted
either.  A nogood watches the rarest of its members it can, those in the fewest nogoods of the store, which the paths
that many nodes take promote least.  A check's question looks only at the nogoods that watch a forced member.

*Why it is exact.*  Every nogood in the store cannot be promoted from the root: the validation found it so with the
check the search learns by.  So at every node with promotions `P`, a nogood `N` less `P` is a conflict of the node, as
for the searcher's own nogoods (fact 4 of [the default step's](search.md#the-default-step) exactness): a candidate that
is the one member of `N` left cannot be promoted, and no completion holds it.  The cut rests on fact 3: every completion
that beats the incumbent promotes the forced members, and if they complete a nogood no completion is promotable.  The
store never chooses a conflict, so the second fact is untouched.  A checkpoint records the tactic (bit 13) and does not
bind it: a store holds facts of the root only, so the units of a search may be searched with it and without it, and the
run that resumes a search says whether it has a store ([trust.md](trust.md)).

## Where a store comes from

`taxman solve --carried-dir DIR` turns the tactic on and gives each game's search a store: at its start and at each
resume, the search reads the files `DIR/nogoods-N.txt` of its own game and the games before it, the latest game's
first and at most 20,000 sets (`CarriedNogoods::from_dir`), and checks them at its root.  It writes its own store
there too, `DIR/nogoods-n.txt` for game n (`model::Harvest`): each searcher hands on the nogoods of its table when it
forgets them and when it ends a unit, and the store keeps them once each, the latest 20,000.  The search writes it
before each checkpoint and at its end, to a file beside it that it then renames, so that a search that reads it
meanwhile reads a whole store; and a resumed search starts from the store of its earlier runs, so that its own keeps
them.  Without a store the tactic does nothing.  What it is worth, measured on
units of game 4984 with the nogoods that units of 4983 learned, is in [the root gap and the size of a
game](../../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-carrying-the-hard-cores-nogoods-into-the-next-game-q7).
