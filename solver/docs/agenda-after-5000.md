# Next, once the chain passes 5000

Until the chain passed 5000, work went only to what could speed it within days.  These are the ideas set aside for
after, the most promising first.  Each links to the entry that has its evidence.

1. **Steer a game's choice of conflict by a postmortem of the game before.**  Choosing the conflict to branch on is
   where a hard game's extra CPU goes below the gate: at gate 150 the careful choice takes the hard games to 0.57 of
   the steps, but pricing the candidates costs more than that saves ([the root gap,
   Q4](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-a-cheaper-choice-of-the-conflict-below-the-gate-q4)).
   If the choices made in game n − 1 recur in game n, n could take them without pricing them and stay exact, since
   any true conflict gives a correct proof.  A first measurement of how often they recur was started on 2026-10-23.
2. **The best choice, found in hindsight.**  At the top nodes of some twenty units of hard games, solve every child of
   every candidate conflict and keep the choice with the smallest subtree.  That bounds what a perfect choice could
   save, and the winners are examples to learn a cheap rule from.
3. **The rest of a cheaper choice** (the same Q4): carry a node's prices from one step to the next instead of pricing
   again, and a cheaper check of the candidate conflicts, about a quarter of the choice's cost.  The ceiling is about
   16% of the hard games' CPU.
4. **A fast way to bring the chain's results home**, as a recipe in [the runbook's guide to the chain](runbook/chain.md).
   Bringing home 49 games on 2026-10-23 took over an hour: a fresh debug build of the ledger, a rebase, and a wait loop
   that never ended.
5. **Quick-check limits for a busy machine.**  On a machine shared with the chain, about ten tests exceed their 2 s CPU
   limit on master itself.  Scale the limits by a calibration workload's slowdown, capped near 4 times, from a
   baseline taken without the chain ([testing.md](design/testing.md)).  The limits are the project owner's decision.
6. **The lab's studies**, off since 2026-10-23: they steered the chain too slowly and cost it threads.  They come back
   only with a cheaper way to steer ([lab.md](design/lab.md)).
