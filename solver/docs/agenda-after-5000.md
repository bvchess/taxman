# Next, once the chain passes 5000

Until the chain passed 5000, work went only to what could speed it within days.  These are the ideas set aside for
after, the most promising first.  Each links to the entry that has its evidence.

Choosing the conflict to branch on is where a hard game's extra CPU goes below the gate: at gate 150 the careful choice
takes the hard games to 0.57 of the steps, but pricing the candidates costs more than that saves ([the root gap,
Q4](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-a-cheaper-choice-of-the-conflict-below-the-gate-q4)).
The first three items attack that cost.  Any true conflict gives a correct proof, so a cheaper or better choice never
puts an answer at risk.

1. **The best choice, found in hindsight.**  At the top nodes of some twenty units of hard games, solve every child of
   every candidate conflict and keep the choice with the smallest subtree.  That bounds what a perfect choice could
   save, and the winners are examples to learn a cheap rule from.
2. **The rest of a cheaper choice** (the same Q4): carry a node's prices from one step to the next instead of pricing
   again, and a cheaper check of the candidate conflicts, about a quarter of the choice's cost.  The ceiling is about
   16% of the hard games' CPU.
3. **The choices of the game before, keyed by the node.**  A postmortem of game n − 1 cannot name the conflict for game
   n by the conflict alone: most of the overlap is the search's own memory or is shared by every game, and nothing
   cheap picks the right one among the hundreds that fit a node.  A table keyed by the node was right at 99-100% of the
   nodes it found, but found a third of the choices in two hard adjacent pairs and almost none elsewhere, on samples of
   a twentieth of each game ([the root gap,
   Q5](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-a-postmortem-of-the-game-before-as-a-guide-to-the-choice-q5)).
   At that share it saves about 7% at gate 300.  The next test is a table from the whole earlier game, about 10⁸
   choices.  If it pays, the chain would run one game at a time instead of four, so that n − 1 is finished when n
   starts: with games hours long, a game's ramp-up and tail cost little, but check the chain's logs for the CPU the
   younger games take first.
4. **A fast way to bring the chain's results home**, as a recipe in [the runbook's guide to the chain](runbook/chain.md).
   Bringing home 49 games on 2026-10-23 took over an hour: a fresh debug build of the ledger, a rebase, and a wait loop
   that never ended.
5. **Quick-check limits for a busy machine.**  On a machine shared with the chain, about ten tests exceed their 2 s CPU
   limit on master itself.  Scale the limits by a calibration workload's slowdown, capped near 4 times, from a
   baseline taken without the chain ([testing.md](design/testing.md)).  The limits are the project owner's decision.
6. **One way for agents to build and test on the chain's machine.**  A script that applies the machine's rules (nice 19,
   four jobs, the shared target directories, the quick check, named release tests only), allowed by the agents'
   permissions, and a check when an agent finishes that stops it while its waiting loops or background jobs still run.
   A guard hook already refuses `pgrep -f`, `pkill -f` and the full release suite, but it sees only the command line,
   not a script an agent writes for itself.
7. **The lab's studies**, off since 2026-10-23: they steered the chain too slowly and cost it threads.  They come back
   only with a cheaper way to steer ([lab.md](design/lab.md)).
