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
   near-repeat cut cannot reach.  Near repeats were 10% of the nodes searched from 4850 to 4955, and 8.4% beyond what
   the cut already settles.  The next step is to prove the reduction as a lemma, then to measure the short proof on
   1209, 1833 and 2301 ([the corner of a near
   repeat](../notebook/search/the-corner-of-a-near-repeat-2026-10/README.md)).  A related law, measured and not proven:
   when the optimal games of n − 1 and n differ by one move and promote the same numbers, the move given up is the
   *shadow* of n, n with each prime factor replaced by the prime just below it (2486 = 2·11·113 for 4953 = 3·13·127), or
   n/2 for even n, in all 1,729 such pairs of games 1 to 4955 ([the shadow
   law](../../docs/theory/neighbors.md#the-shadow-law)).  Why, and whether it says when such a swap happens, are open.
   Giving up the smallest move that makes room for n is not such a rule: it falls short at 1,125 of 4,954 games ([the
   smallest single loss](../notebook/the-smallest-single-loss-over-all-moves-2026-10/README.md)).  What the law gives
   the corner:
   * **The search already starts from the swap.**  The seed, game n − 1, survives whole in a swap, since the
     promotions are the same, so every searched swap starts from its optimal set and spends all its nodes on the
     proof that nothing beats it.  Believing the law would skip no search: it gives a lower bound, which the seed
     already has, and a game needs an upper bound.  The 78 searched swaps of the committed chain took 5.85 × 10⁹
     nodes, 6.9% of all the nodes searched to 4949, and from 4500 on 6 of the 71 searched games and 8.5% of their
     nodes, 4953 alone 505 million.
   * **It names the corner before the search.**  The old move is the shadow, known from n alone, so a cheap test that
     the shadow swap is playable (a peel) can arm the short proof before setup.  For odd n the old move's largest
     prime factor is the prime just below n's, so a proof of the law, or of the reduction, could compare the
     corners of those two primes.
   * **A theorem would settle games.**  One that bounds opt(n) by opt(n − 1) + n − shadow(n) under conditions that
     can be checked would prove those swaps without a search.  No route to it is known.
   * **The lemma needs room.**  It would go in [the theory](../../docs/THEORY.md), but search-rules.md is at 5,952 of
     its 6,000 words, so it needs a file of its own or a split.
5. **A proof of the cofactor rule from the square theorem.**  The cofactor rule, measured, says that game n poses the
   root problem of the game m before it (n − 2 when n − 1 is prime) when n divided by its largest prime factor is a
   prime or twice a prime, and is at its ceiling when it is 9, wherever layer 2 has no freedom in game n and in game m
   ([the cofactor rule](../../docs/theory/neighbors.md#the-cofactor-rule)).  That holds in all 1,946 such games from 28
   to 4949, and from 188 on in every game, since 186 is the last board whose layer 2 has a freedom, by [the square
   theorem](../../docs/theory/frame-model.md#the-square-theorem): the source 187 = 11·17 joins the frame of 11 and 13,
   which holds the squares 121 and 169, to the rest.  Every game that breaks the rule has a freedom in layer 2.  Read in
   layers, the first case is every n with two prime factors, a number of layer 2, and the second every even n with
   three. A sketch, not checked: where layer 2 has no freedom in either game, the frames with freedoms lie in layers 3
   and up.  An odd n of layer 2 adds only a source to the closed layer, and no number up to n/2, since ⌊n/2⌋ does not
   change.  An even n = 2P adds the prime P up to n/2, in layer 1, which no frame with freedoms uses.  If the claims of
   the layers from 3 on are unchanged too, the root problem repeats, and the rule's first case is a theorem.  The cases
   2pP and 9P add a source of layer 3 and need more.  It is theory work, with no computation; [the notebook
   entry](../notebook/search/which-games-need-no-search-the-cofactor-of-n-2026-10/README.md) has the counts.
6. **No search where n joins only closed frames.**  A general form of the cofactor rule, proposed on 2026-10-24 and not
   yet checked.  The search works only on the frames with freedoms; a closed frame takes no part.  From game n − 1 to
   game n, an odd n changes one thing: the source n arrives, and every number up to n/2 stays as it was.  Suppose every
   frame that n touches, the frames of all of its maximal factors, is closed in game n − 1.  Then n joins them with a
   move and no factor, so one of their sources can no longer be paid and drops out, and they stay closed.  The frames
   with freedoms are untouched, so game n poses the root problem of game n − 1, and [the repeat
   lemma (R6)](../../docs/theory/search-rules.md#the-repeat-lemma) settles it with no search: opt(n) = opt(n − 1) + n −
   x, where x is the source that drops out, which [the shadow law](../../docs/theory/neighbors.md#the-shadow-law) names
   as the shadow of n.  An even n changes a second thing: n/2 turns from a source into a factor, so the frame of n/2
   must be closed too.  When n − 1 is prime, the comparison is with game n − 2, as in the cofactor rule.  If a frame
   that n touches has a freedom, n joins it and the problem changes, so the game needs a search unless something else
   settles it, such as the ceiling.  The cofactor rule (item 5) is the case of n in layer 2, where that layer is one
   closed frame from 188 on; the general form would reach every layer, and setup could test it from the frames alone,
   before any search.  The first step is a check on the committed chain: for each game, whether every frame that n
   (and n/2 for even n) touches is closed in the game before, against whether the game was a repeat.  Then a proof,
   as a lemma beside the repeat lemma.
7. **Carry the hard core, not the search.**  Consecutive games refute the same core along different paths, and their
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
8. **A fast way to bring the chain's results home**, as a recipe in [the runbook's guide to the chain](runbook/chain.md).
   Bringing home 49 games on 2026-10-23 took over an hour: a fresh debug build of the ledger, a rebase, and a wait loop
   that never ended.
9. **Quick-check limits for a busy machine.**  On a machine shared with the chain, about ten tests exceed their 2 s CPU
   limit on master itself.  Scale the limits by a calibration workload's slowdown, capped near 4 times, from a
   baseline taken without the chain ([testing.md](design/testing.md)).  The limits are the project owner's decision.
10. **One way for agents to build and test on the chain's machine.**  A script that applies the machine's rules (nice 19,
   four jobs, the shared target directories, the quick check, named release tests only), allowed by the agents'
   permissions, and a check when an agent finishes that stops it while its waiting loops or background jobs still run.
   A guard hook already refuses `pgrep -f`, `pkill -f` and the full release suite, but it sees only the command line,
   not a script an agent writes for itself.
11. **The lab's studies**, off since 2026-10-23: they steered the chain too slowly and cost it threads.  They come back
   only with a cheaper way to steer ([lab.md](design/lab.md)).
