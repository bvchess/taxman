# Next, once the chain passes 5000

Until the chain passed 5000, work went only to what could speed it within days.  These are the ideas set aside for
after, the most promising first.  Each links to the entry that has its evidence.

Choosing the conflict to branch on is where a hard game's extra CPU goes below the gate: at gate 150 the careful choice
takes the hard games to 0.57 of the steps, but pricing the candidates costs more than that saves ([the root gap,
Q4](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-a-cheaper-choice-of-the-conflict-below-the-gate-q4)).
The first three items attack that cost.  Any true conflict gives a correct proof, so a cheaper or better choice never
puts an answer at risk.

1. **The rest of a cheaper choice.**  Keeping a node's prices from one step to the next is built and on the chain since
   2026-10-23 (the tactic `price_reuse`: 0.82 of the CPU on the hard units, [the root gap,
   Q6](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-price-reuse-in-the-choice-q6)),
   and the choice fell from about a quarter of the CPU to 17-18%.  What is left is a cheaper check of the candidate
   conflicts (`FrameGraph::conflicts_ordered`), about a quarter of the choice's cost before reuse.
2. **The choices of the game before, keyed by the node.**  A postmortem of game n − 1 cannot name the conflict for game
   n by the conflict alone: most of the overlap is the search's own memory or is shared by every game, and nothing
   cheap picks the right one among the hundreds that fit a node.  A table keyed by the node was right at 99-100% of the
   nodes it found, but found a third of the choices in two hard adjacent pairs and almost none elsewhere, on samples of
   a twentieth of each game ([the root gap,
   Q5](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-a-postmortem-of-the-game-before-as-a-guide-to-the-choice-q5)).
   At that share it saves about 7% at gate 300.  The next test is a table from the whole earlier game, about 10⁸
   choices.  Held in memory, a whole game's table is about 5-10 GB, against the chain machine's 755 GB.  The window
   gives threads to the oldest game first, so a game does most of its search after the game before it concludes, and
   the table can be armed then, as the near-repeat cut is.
3. **A ranking of members carried from the game before.**  A postmortem of game n − 1 gives an order over members
   (numbers), and game n uses it to choose cheaply instead of pricing the candidates.  The record killed "the branching
   order carried" by tree overlap, since consecutive games' trees share 0.15% of their nodes ([carrying a search's
   learning](../notebook/search/carrying-a-searchs-learning-to-the-next-game-2026-10/README.md#8-the-two-criteria-for-carrying-work-in-the-present-solver)),
   but a ranking of members needs no overlap with the old tree, and it is untested.  The evidence near it is lukewarm:
   recency in the choice of conflict had no direction ([the pilots,
   4.6](../notebook/search/ideas-from-other-fields-the-pilots-2026-10/README.md#46-recency-in-the-choice-of-conflict-killed)),
   and the hindsight study of 2026-10-20 found no cheap signal ([the headroom of branching in
   hindsight](../notebook/search/the-headroom-of-branching-in-hindsight-2026-10/README.md)).
4. **The corner of a near repeat.**  Two consecutive games sometimes differ by one swapped move, and then the proof of
   game n could be a short proof about that move's corner of the game, reusing the proof of game n − 1 for the rest.  A
   sketch reduces it to the ways of playing n that use up every factor that can pay for the old move, which the
   near-repeat cut cannot reach.  Near repeats were 10% of the nodes searched from 4850 to 4955, and 8.4% beyond what the
   cut already settles.  The next step is to prove the reduction as a lemma, then to measure the short proof on 1209,
   1833 and 2301 ([the corner of a
   near repeat](../notebook/search/the-corner-of-a-near-repeat-2026-10/README.md)).  A related question: when the
   promotions of n − 1 and n are the same, can the move that n's optimal game gives up be named from the factorization
   of n?  At odd semiprimes it is p′q′, the product of the primes just below p and q, and at the near repeats of the
   4953 kind 2·11·p gives way to 3·13·q with q the next prime after p.  Giving up the smallest move that makes room for
   n is not such a rule: it falls short at 1,125 of 4,954 games ([the smallest single
   loss](../notebook/the-smallest-single-loss-over-all-moves-2026-10/README.md)).
5. **Carry the hard core, not the search.**  Consecutive games refute the same core along different paths, and their
   trees share 0.15% of their nodes ([carrying a search's
   learning](../notebook/search/carrying-a-searchs-learning-to-the-next-game-2026-10/README.md), results 3 to 5 and the
   decision), so replaying the old search does not pay.  Two angles are untested.  The first is to amortize the core's
   results across a run of games whose frames 1-3 do not change: the decomposition across frames ([agenda
   A5](AGENDA.md#a5-decompose-across-the-frames-andor-search)) was judged within one game only.  The second is to carry
   the game before's conflicts only to rule candidates out of children, in a store indexed so that a lookup does not
   scan, and never to choose the branching.  Carrying them into the nogood table lost, because the table changes the
   tree and every lookup scans it: 1227 from 1225's nogoods took 10% fewer nodes and 3% more time, at 59k nodes a
   second against 67k ([carrying nogoods
   over](../notebook/search/carrying-nogoods-over-2026-10-measured-not-kept/README.md)).
6. **A fast way to bring the chain's results home**, as a recipe in [the runbook's guide to the chain](runbook/chain.md).
   Bringing home 49 games on 2026-10-23 took over an hour: a fresh debug build of the ledger, a rebase, and a wait loop
   that never ended.
7. **Quick-check limits for a busy machine.**  On a machine shared with the chain, about ten tests exceed their 2 s CPU
   limit on master itself.  Scale the limits by a calibration workload's slowdown, capped near 4 times, from a
   baseline taken without the chain ([testing.md](design/testing.md)).  The limits are the project owner's decision.
8. **One way for agents to build and test on the chain's machine.**  A script that applies the machine's rules (nice 19,
   four jobs, the shared target directories, the quick check, named release tests only), allowed by the agents'
   permissions, and a check when an agent finishes that stops it while its waiting loops or background jobs still run.
   A guard hook already refuses `pgrep -f`, `pkill -f` and the full release suite, but it sees only the command line,
   not a script an agent writes for itself.
9. **The lab's studies**, off since 2026-10-23: they steered the chain too slowly and cost it threads.  They come back
   only with a cheaper way to steer ([lab.md](design/lab.md)).
