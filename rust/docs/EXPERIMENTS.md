# Experiments: the lab notebook of the Rust solver

What was tried, in what order, and what it was worth.  [DESIGN.md](../DESIGN.md) says how the solver works now;
this page is the record of how it got there: a register of every experiment, kept or not, and every round of
improvements with the measurements that decided it, the ideas that were measured and not kept among them, with their
numbers.  Entries are dated by month (the whole record so far is from 2026-10 unless a section says otherwise) and
name the commits that made each step, so `git show COMMIT` finds the code.  Statements are true of the code at the
time of the entry; where a later entry changed the picture it says so, and DESIGN.md is the authority on the present.

Terms (conflict, nogood, forced member, step, `W`, ...) are defined in the [glossary of the README](../README.md#7-glossary)
and in DESIGN.md; the measurements were made with the ladder, the probe and `taxman bench`, which DESIGN.md
("Performance") describes.

* [Register of everything tried](#register-of-everything-tried): one row per experiment, by subject, with where its
  details are and what decided it.  Look here first.
* [Index of rounds](#index-of-rounds): the rounds of improvements in the order they happened, with their commits.
* The rounds themselves (from [the port](#the-port-and-its-first-improvements) to
  [the benchmarks](#benchmarks-the-probe-pgo-and-scaling)), which hold the details; the pilots of ideas borrowed from
  other fields are in [IDEAS.md](IDEAS.md).

## Register of everything tried

One row per experiment or idea tried on the exact solver and on the theory under it, kept or not, so that nothing is
tried twice.  **Check it before proposing an experiment, and add a row when one finishes, failures included.**  The
details stay where the row points: a round below, [IDEAS.md](IDEAS.md), [DESIGN.md](../DESIGN.md), the
[evidence](../evidence/README.md), the [results](../results/README.md), or the module comment of the code of a study.
Where an experiment's code was discarded (the pilots of IDEAS.md), the page it points to describes what was computed,
on which games and with which settings, well enough to repeat it.  The [index of rounds](#index-of-rounds) lists the
rounds in the order they happened, with their commits.

Outcomes: **kept** (in the solver or the evidence, or the finding stands), **not kept** (tried and taken out, or
never adopted because it did not pay or is false), **superseded** (kept for a while, replaced by something better),
**open** (proposed, not tried or not decided), **measured** (a study whose result changed no code).  The numbers are
those of the page the row points to; a game is named by its board size, and times are search seconds.

### Search and bounds

| tried | details | outcome | what decided it |
|---|---|---|---|
| Reimplement the Java's search and check its upper bound | [port](#the-port-and-its-first-improvements) | kept, with a fix | the Java's runner-up can be too low (root of n = 114: 358 against the best completion's 360); the sound runner-up is the default, `legacy_bound` reproduces the Java's |
| A simpler sound runner-up: the largest candidate not in the bid | [port](#the-port-and-its-first-improvements) | not kept | n = 700 took over 10 minutes instead of 18 s |
| Matching bound (max-weight bipartite matching, kept incrementally with dual potentials) instead of `blameGame` | [after parity](#improvements-after-parity) | kept | mean gap at the root 31 against the Java bound's 76 on the 94 golden games that need search; 1.5 to 85 times fewer nodes at n = 480-720 |
| The minimum of `blameGame` and the matching bound | [after parity](#improvements-after-parity) | not kept | `blameGame` pruned nowhere the matching bound did not (0 at every check) |
| Seed the incumbent from the game of n − 1 and stop at the cap n + opt(n − 1) (`seed_incumbent`, `prev_cap`) | [after parity](#improvements-after-parity) | kept | 415 of the 1300 proofs of the strict chain of commit bd9bb5d are the cap, reached before the first node; the committed chain, which also has `same_search`, has 115 and 545 repeats ([results](../results/README.md#what-supports-the-scores)) |
| Branch on a member of the heaviest matching's conflict instead of the largest candidate | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept, the decisive change | 936: 90.8 s to 31.8 s (12.3M to 2.62M nodes); 1000: open at 120 s to 27.5 s (10 threads) |
| Strong branching: the member whose removal lowers the bound most, its level kept for the node (`conflict-strong`) | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept, the default | 936: 29.5 s to 17.7 s, 2.62M to 909k nodes |
| The other branchings, `conflict-smallest` and `conflict-largest` | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept as switches | smallest gives the smallest proof trees at 880 and 936 but finds good incumbents late; largest follows the dive and finds them early |
| Propagate the known conflicts into the children | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept | −17% nodes |
| Refinement (bound the branches of the conflict) on the Java's branching | [conflicts](#conflicts-of-the-matching-bound-2026-10) | not kept | nodes 2-70 times fewer but dearer: 880 from 4.8M to 68k nodes at depth 7, yet only 31 s to 21 s, and slower than none at depth 1-3 |
| Refinement one level deep with conflict branching (`match_refine` = 1) | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept | 2-15% faster than none on the benchmarks; deeper is slower |
| Promote a member of the conflict; a lookahead that scores the child with the other member of a pair out | [conflicts](#conflicts-of-the-matching-bound-2026-10) | not kept | the heaviest matching promotes them already; the lookahead did worse |
| Branch on the members of every failing frame's conflict | [conflicts](#conflicts-of-the-matching-bound-2026-10) | not kept | 936: 754k to 1.0M nodes |
| Choose the frame by the smallest core and minimize only it | [conflicts](#conflicts-of-the-matching-bound-2026-10) | not kept | much cheaper nodes, but its conflicts branch far worse (1000 unfinished in 2 minutes) |
| A complete `can_promote` filter of the candidates (the Java's is partial) | [conflicts](#conflicts-of-the-matching-bound-2026-10) | not kept | nodes −25-35%, cost per node ×4 |
| A per-frame cache of conflict answers; a radix heap in the Hungarian search | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept | −5%; −16-19% |
| Local search on the starting incumbent | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept | reaches the optimum at the start for 945 and 988 in 20-40 ms |
| Local search on every incumbent found; dives at search nodes | [conflicts](#conflicts-of-the-matching-bound-2026-10) | not kept | nothing measurable |
| A one-step lookahead in the root dive (the dive-variants experiment) | [conflicts](#conflicts-of-the-matching-bound-2026-10) | not kept | no better than the plain dive, which is itself weaker than greedy on most games |
| Forced members (`fix_forced`): the child promotes every forced member of the conflict | [forced members](#forced-members-and-nogoods-2026-10-second-round) | kept | nodes −31% (880) to −60% (1170), time −21% to −36% |
| Keep the child's check as the verdict of its first step; one candidate list per child | [forced members](#forced-members-and-nogoods-2026-10-second-round) | kept | −3% to −23% time |
| The bound without a member from the dual potentials alone, instead of an exact solve | [forced members](#forced-members-and-nogoods-2026-10-second-round) | kept | same trees, 13-28% less time; settles 89% of the forced members at 1184 |
| The forced members of the whole heaviest matching, from the potentials | [forced members](#forced-members-and-nogoods-2026-10-second-round) | kept | nodes −8% (936) to −52% (1184), time −2% to −39% |
| Cheaper children: no Java promotability checks with the matching bound, forced members promoted together, no `blameGame` | [forced members](#forced-members-and-nogoods-2026-10-second-round) | kept | −30% to −59% time, nodes within ±8% |
| No refinement at a node that has forced members | [forced members](#forced-members-and-nogoods-2026-10-second-round) | kept | nodes +3% to +11%, time −2% to −8% |
| Prove more members forced with the forced ones required (a second pass of the potentials' bounds) | [forced members](#forced-members-and-nogoods-2026-10-second-round) | not kept | pruned nothing more |
| Skip strong branching's exact solves when the potentials found a forced member, or choose the branch by the potentials alone | [forced members](#forced-members-and-nogoods-2026-10-second-round) | not kept | each traded nodes for time at about even |
| Round the heaviest matching into an incumbent at every node | [forced members](#forced-members-and-nogoods-2026-10-second-round) | not kept | 5 times slower per node; the optimum of 1050 was found at the same step |
| A large-neighbourhood search before the search | [forced members](#forced-members-and-nogoods-2026-10-second-round) | not kept | 1025: 25080 in 1.3 s, but the optimum only from the search, at about the same point |
| The Java's depth-2 preclusion table | [forced members](#forced-members-and-nogoods-2026-10-second-round) | superseded by nogoods (`preclusions` off) | without it 992 and 1215 take +5% and −4%; 1101-1217 on 8 threads spent 53 of 131 s building tables |
| Faster nodes, same trees: bucket 0 of the radix heap a bitset, packed per-vertex slots, `MAX_FRAMES` 12 to 8, exact nogood lookups, no peel for a factor no other move uses, the heap inlined | [faster nodes](#faster-nodes-same-trees-2026-10) | kept | 30% fewer cycles at the 1215 proof (151.3 G to 106.2 G), the set widths below included |
| An undo trail for the matching levels | [faster nodes](#faster-nodes-same-trees-2026-10) | not kept | windows +0.8% and +0.9% cycles, instructions +4% |
| The peel in frame-local form | [faster nodes](#faster-nodes-same-trees-2026-10) | not kept | bit-parallel peel +8% and +16% on forced sets; counting peel +0.5% and +0.8% |
| Caching frame checks | [faster nodes](#faster-nodes-same-trees-2026-10) | not kept | 14-17% of the peels repeat one exactly, but only 5% one of the last eight |
| Unchecked indexing in the Hungarian search | [faster nodes](#faster-nodes-same-trees-2026-10) | not kept | −4% instructions, −0.7% cycles; not worth `unsafe` |
| Five smaller changes: no peel for the several moves of a forced set, a forced set promoted straight into the child's state, stopping the potentials' bound once its first part decides, the Dijkstra horizon, one branch for `reach`'s two tests | [faster nodes](#faster-nodes-same-trees-2026-10) | not kept | each within ±0.5% or worse (the horizon: windows +0.4% and +2.5%) |
| Profile-guided optimization (`tools/pgo.sh`) | [benchmarks](#benchmarks-the-probe-pgo-and-scaling) | kept, optional | 12% on the code before the inner-loop round, 0-2% after it |
| `i32` potentials; a warm-started unit rebuild; candidate-index sets for every search-level set; watched members for the nogoods | [faster nodes](#faster-nodes-same-trees-2026-10) | open | no bound on the potentials is proven; another optimal matching can change the tree; a large refactor; the nogood table is 3% of the time |
| Cheaper matching repairs; incumbents for games whose optimum the search reaches late (992, 1025) | [benchmarks](#benchmarks-the-probe-pgo-and-scaling) | open | the repairs are 46-67% of the time at the frontier; 992 on 8 threads finds its optimum at 11.8 s of 12.0 s |
| Where the time goes in the hardest games (sampling profiles of 1232, 1287, 1299, 1295) | [IDEAS 2](IDEAS.md#2-where-the-time-goes-in-the-hardest-games) | measured | matching repairs 56-67%, strong branching 28-34%, conflict computation under 1%, the engine under 0.1% |
| Same search: conclude a game whose search problem an earlier game of the chain settled, with that game's promotions (tactic and proof `same_search`; IDEAS rank 1) | [same search](#same-search-2026-10); [IDEAS 4.1](IDEAS.md#41-memoizing-searches-that-repeat-rank-1) | kept: on by default and in strict runs; results schema 4 | 545 of the 1076 searched games of 1-1300 repeat (n − 1, or n − 2 past a prime); 953 of 3359 s (28.4%) of the search of the strict chain of commit bd9bb5d, 851 of 2866 s in 1232-1300; 1299 in 0.08 s instead of 265 s |
| A Lagrangian bound over the nogoods, kept incrementally, with forced members read off its potentials (tactic `lagrange`; IDEAS rank 2) | [Lagrangian bound](#a-lagrangian-bound-over-the-nogoods-2026-10); [IDEAS 4.2](IDEAS.md#42-a-lagrangian-bound-over-the-nogoods-rank-2) | kept: on by default | proofs 1184-1300 take 2-6 times fewer cycles (3.3 geometric mean, no trend in n); the strict chain 1201-1231 2.4 times less time; the pilot alone, incremental but without forced members, about break-even; 992 (optimum found late) +35% |
| Variants of the Lagrangian bound: depth limits 8-32, multipliers fitted mid-path or at the deepest node, bound only at node re-checks, 20 or 100 subgradient steps, first refit at 500 steps, refits every 10,000 steps | [Lagrangian bound](#a-lagrangian-bound-over-the-nogoods-2026-10) | not kept | each within ±12% of the default or worse; the bound's cost follows the changes along the path, not how often it is asked |
| Where the time goes with the Lagrangian bound (sampling profiles of 1254, 1272, 1288, 1290, 1300 as the chain runs them; the 1215 proof by source line) | [faster nodes with the Lagrangian bound](#faster-nodes-with-the-lagrangian-bound-same-trees-2026-10) | measured | Hungarian searches 55-60% (the matching's 33-39%, the Lagrangian levels' 16-18%), peels 17-20%, the Lagrangian levels 25-28% in all (refits 4%); the searches are large (88% of the vertices scanned in trees of 64-511 of 686) |
| Faster nodes with the Lagrangian bound, same trees: strong branching takes the level refinement solved for its first member, the bound from the potentials stops at a tight edge, one settle for the forced members, a membership test for known conflicts, the Hungarian search's fields apart and a fixed-size code set, the Lagrangian bound's forced members only among those not forced yet | [faster nodes with the Lagrangian bound](#faster-nodes-with-the-lagrangian-bound-same-trees-2026-10) | kept | the probe's windows −10.6% and −10.2% cycles (−15.3% and −14.0% instructions), with `lagrange=off` −11.6% and −11.1%; proofs 936-1254 −5% to −10%; the strict chain 1201-1231 −8.1%; every node count and both fingerprints unchanged |
| One key for a search slot's epoch and tag; `-C target-cpu=native`; skipping forced-member checks where `value − x ≥ need`; strong branching skipping members by the lower bound `value − x`; refinement after the Lagrangian bound | [faster nodes with the Lagrangian bound](#faster-nodes-with-the-lagrangian-bound-same-trees-2026-10) | not kept | instructions −0.02% and ±0.05%, cycles within noise; the third applies to 0.17 of 19.8 checks a step (1250); the fourth can never apply (members are solved in increasing order); the last changes trees |
| `match_refine` = 0 with the Lagrangian bound | [faster nodes with the Lagrangian bound](#faster-nodes-with-the-lagrangian-bound-same-trees-2026-10) | measured: default kept at 1 | 19 proofs (936-1288) −3.0% geometric mean (±2.4% standard error), −5.3% in all, from −21% (1254) to +20% (1225); the strict chain 1..1000 −9.4%, 1201-1231 +0.4% |
| Reduced-cost fixing of candidates from the dual | [IDEAS 4.3](IDEAS.md#43-reduced-cost-fixing-killed) | not kept | fixes 70.9 of 159 candidates outside `Q*` per node at 936, yet changes no tree: the matching relaxation is integral |
| Component decomposition (AND/OR search, #SAT) | [IDEAS 4.4](IDEAS.md#44-component-decomposition-killed) | not kept | exactly one component holds candidates at the root of 936-1300 and at all 600 probe nodes |
| Disjoint conflicts from the dual (MaxSAT's disjoint cores, the pair cut) | [IDEAS 4.5](IDEAS.md#45-disjoint-conflicts-from-the-dual-killed) | not kept | between 2.7% fewer and 0.3% more nodes for 12% more cycles |
| A portfolio: another branching on some threads for finding incumbents | [IDEAS 4.7](IDEAS.md#47-a-portfolio-for-finding-incumbents-weak) | not kept | the frontier's cost is the proof; 8 of the 11 slowest games of 1232-1300 start at their optimum |
| Reliability branching (pseudocosts) in place of strong branching's exact solves | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | strong branching is 28-34% of the time; not piloted |
| Knuth's estimate of a tree's size, to budget games and split work | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | not piloted |
| Ideas considered and not piloted: restarts, bound nogoods and backjumping, cube-and-conquer splitting, dominance rules, symmetry breaking, best-bound node selection, cutting planes | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | each has its reason there (with the optimum known the tree is fixed by the bound; the matching LP is integral; no symmetries found; ...) |
| What drives the slope: the root's gaps (matching, refined, Lagrangian over the root's own nogoods), the size (n, candidates, `max_total`), the cap and the start incumbent against the one-thread proofs of the 143 searched games of 880-1300 (`examples/slope.rs`) | [slope](#what-drives-the-slope-2026-10) | measured | log10 nodes against the Lagrangian gap R² 0.90 (sd 0.35), the matching gap 0.87, n 0.23, the candidates 0.26; with the gap in the fit n adds nothing (t 1.1); the slope in n is ×10 per ~130 of gap times the gap's growth (56 per 100 n, jagged: +304 from 1151-1200 to 1251-1300, flat to 1352) |
| Margin scans: proofs with every bound lowered by δ (the incumbent a value, `P*` + δ) | [slope](#what-drives-the-slope-2026-10) | measured | 19 games of 900-1300: 0.005-0.012 decades fewer nodes per unit of δ, no trend in n; a large game tightened costs what a small one with the same remaining gap costs |
| The root's converged Lagrangian relaxation (its own nogoods, damped subgradient) as every searcher's starting relaxation | [slope](#what-drives-the-slope-2026-10) | not kept (pilot) | proofs −5% (1254) to −76% (1184) nodes, the least at the frontier: a constant factor; the gap that counts is the one at every node |
| Damped Polyak steps in the refit (`fit`) | [slope](#what-drives-the-slope-2026-10) | not kept (pilot) | on the root's nogoods 50 damped steps reach the bound of 50 undamped ones within 8; one refit from zero multipliers stays 50-540 above the converged bound, by its step count |
| A bound whose gap at every node grows more slowly with n, screened by its gaps at the root and at the probe's nodes before it is built | [slope](#what-drives-the-slope-2026-10) | open | the lever on the slope: a gap predicts a proof's nodes (×10 per ~130 of Lagrangian gap, ~270 of matching gap), and the relaxation's error, not the size, is what grows |

### Nogoods and conflicts

| tried | details | outcome | what decided it |
|---|---|---|---|
| Learn each conflict as a nogood of the root and use the table everywhere (`nogoods`) | [forced members](#forced-members-and-nogoods-2026-10-second-round) | kept | 936 from 8.05 s (320k nodes) to 1.09 s (50k); only 854 distinct nogoods in 291k conflicts at 1184 |
| Nogood propagation alone, every conflict computed and made a nogood | [forced members](#forced-members-and-nogoods-2026-10-second-round) | not kept | cut nodes less (880 to 61k against 39k with lookups) and cost more than it saved |
| 256-bit signatures to skip nogoods in a scan | [forced members](#forced-members-and-nogoods-2026-10-second-round) | superseded by exact lookups over compact member indices | −5% to −10% from the signatures, then about −2% more from the exact form |
| Carry the nogoods of game n − 1 into game n | [carrying](#carrying-nogoods-over-2026-10-measured-not-kept) | not kept | 1227, the ceiling: −10% nodes, +3% time; the strict chain 1201-1231 617 s to 941 s |
| Store the nogood tables in checkpoints | [carrying](#carrying-nogoods-over-2026-10-measured-not-kept) | not kept | a resume with empty tables loses nothing and is faster (1220: 143 s against 191 s with the tables kept) |
| Forget each searcher's nogoods every N steps (`forget_nogoods`, N = 50,000) | [forgetting](#forgetting-nogoods-2026-10) | kept | 26% less time on 1024, 1025, 1215 and 1220 together; the chain 1201-1231 on 4 threads 571 s to 377 s |
| Forget by table size (64, 128, 256) | [forgetting](#forgetting-nogoods-2026-10) | not kept | 15-22% less time on the hard four, but the smaller games slower (936: 0.51 s against 0.85/0.61/0.53 s) |
| Keep the nogoods used since the last clearing | [forgetting](#forgetting-nogoods-2026-10) | not kept | every 100,000 steps 15% less time, 500,000 6%: the kept ones cost lookups as much and help no more |
| Filter out candidates that cannot be promoted from the root (`root_filter`), the single-member conflicts as root facts | [single-member](#single-member-conflicts-belong-to-the-node-2026-10) | not kept | 0 of 225, 273, 287 and 302 candidates dropped at 936, 1100, 1184 and 1215, node counts identical; the frame builder's candidates pass for every game 2-1230 |
| How useful an old nogood is, what a table costs, and how often threads learn the same nogood (the nogood study) | [IDEAS 3](IDEAS.md#3-nogoods-with-very-many-threads) | measured | uses fall 20 times in the first 10,000 steps; a table of S nogoods costs about 450 + 2.3 S cycles of a 16-20k-cycle step; 88-89% of the nogoods learned at 64-128 workers are held by another |
| Never forget (at one thread, 1215) | [IDEAS 3](IDEAS.md#3-nogoods-with-very-many-threads) | not kept | −0.7% nodes, +19% cycles |
| Share every nogood between the threads | [IDEAS 3](IDEAS.md#3-nogoods-with-very-many-threads) | not kept | 1225 at 128 workers 14.38M nodes against 10.9-11.5M, the tables 3-8 times as large |
| Share only the nogoods of at most 3 members | [IDEAS 3](IDEAS.md#3-nogoods-with-very-many-threads) | not kept | 1225: 11.56M at 32 workers and 12.89M at 128, no fewer nodes than clearing |
| Keep the nogoods of at most 3 members at a clear (a glue-like quality) | [IDEAS 3](IDEAS.md#3-nogoods-with-very-many-threads) | not kept | no measurable change; the nogoods of the search are large (11,483 of 17,556 have 8 or more members) and the large ones answer most lookups |
| Prefer the most recently learned of the smallest conflicts (VSIDS-like recency) | [IDEAS 4.6](IDEAS.md#46-recency-in-the-choice-of-conflict-killed) | not kept | −2.8% to +20% nodes across the fast set and two proofs, no direction |
| A clearing period of about 30,000 steps at 192 threads | [IDEAS 3](IDEAS.md#3-nogoods-with-very-many-threads) | open | the tables learn about 1.7 times faster at 128 workers; to test on the big machine |

### Engine and parallelism

| tried | details | outcome | what decided it |
|---|---|---|---|
| A yielding searcher leaves out the nodes with nothing left to do | [engine](#the-engine) | kept | runs of at most one poll fell from 60 and 91 of 256 and 305 to 13 and 10 of 227 and 200 |
| Splits sized for many cores: one open request per idle worker, any running worker may answer | [engine](#the-engine) | kept | the first design (one request at a time) serialized the distribution of work; all workers busy within 3-8 ms |
| Per-worker deques with stealing instead of the one locked deque | [engine](#the-engine) | not kept, not built | units exist only after yields and a rebuild costs 30-170 µs against about a microsecond under the lock, so with every worker busy nobody takes it; revisit if a many-core run shows lock wait (`Counters::lock_wait`) |
| Checkpoint pauses at any interval | [engine](#the-engine) | kept, a fix | an interval of 0 paused again at once and the workers never ran |
| Set widths of 20, 24 and 28 words between 16 and 32 | [engine](#the-engine) | kept | `Bits<20>` for `Bits<32>`: 6% fewer cycles at 1184 and 1215 |
| Set widths of 12 and 48 words | [faster nodes](#faster-nodes-same-trees-2026-10) | not kept | 3-4% at 512-767 (solved) and some at 2048-3071, for 12 s more of every build |
| Thread scaling on the M1 Max | [benchmarks](#benchmarks-the-probe-pgo-and-scaling), [bench/curve](../bench/curve/README.md#thread-scaling-step-4-the-proof-of-1225-seeded-with-its-optimum) | measured | 1225 proof 150.9 s on 1 thread, 18.0 s on 8 (the tree also shrinks); the node rate falls 19% per thread |
| Emulate many workers on few cores (`EngineConfig::cores`, `ladder --cores`) | [IDEAS 3](IDEAS.md#3-nogoods-with-very-many-threads) | kept | 64-128 workers split, steal and learn as they would on 64-128 cores, in counts if not in time |
| The tree grows at many workers | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | 10-20% more nodes at 32-128 emulated workers than at 4-8 on 1225, with 10-25 times the unit starts; suspect: what a rebuilt unit forgets |
| Anything above 8 threads, Graviton5, NUMA | [RUNBOOK 8](../tools/cloud/RUNBOOK.md#8-what-has-not-been-measured) | open | not measured; the 10x to 19x assumed for 192 cores comes from the design |
| The cost of a game against n beyond the frontier (cost-curve study) | [bench/curve](../bench/curve/README.md#results) | measured | ×10 per 55-70 in n, jagged (×18 from 1250 to 1300); a standalone game costs 1.9 times the chained one where it matters |

### Tactics and shortcuts

| tried | details | outcome | what decided it |
|---|---|---|---|
| The prime shortcut (`prime_shortcut`) | [DESIGN 5](../DESIGN.md#5-tactics) | kept, proven | tight for every prime; the proof is with `accelerate::prime_shortcut` |
| Reuse the previous game's promotions without a search (`reuse_prev`) | [DESIGN 5](../DESIGN.md#5-tactics) | kept, unproven; off in a strict run | with several optimal sets of n − 1 the result depends on which was recorded: the recorded game of 685 plus 686 is unplayable though opt(686) = 686 + opt(685) |
| The sacrifice bound (games on punctured boards) as a proof for `reuse_prev` | [`src/sacrifice.rs`](../src/sacrifice.rs) | not kept | tight on 57 of the 152 `reuse_prev` games, and where it proves it costs 1.1-8.7 times the direct search; the theorem stays and `tests/wide.rs` uses it |
| Lower frame 0's freedoms by solving the two-frame game (`revise_freedoms`) | [DESIGN 4](../DESIGN.md#revise_freedoms) | not kept: its premise is false, so it is to be removed (decided 2026-10-13) | assumes the max-sum set also has the max count, which fails on about one perturbed game in 1,300, and for the whole frame game of n = 833 (48 promotions optimal, 49 feasible: [evidence 6](../evidence/README.md#6-does-an-optimal-game-play-the-most-moves)); the two-frame form it applies is undecided |
| What `reuse_prev` still saves in a strict chain that has `same_search`, the cap and the seed (from the setups, no search) | [tactics review](#reuse_prev-nothing-left-to-save) | measured; to be removed | of 2..1300 it accepts 670 games, every one settled anyway (545 repeats, 115 caps at once, 10 searches of at most 1.6 s): 3.8 s of the chain's 1052 s of search, 0.5 s of 813 s in 1232..1300; no score or move differs |
| What `revise_freedoms` changes in the proofs at the frontier | [tactics review](#revise_freedoms-the-same-trees) | measured | it changes the problem of every searched game of 1232..1300 (frame 0's freedoms down 4 to 6, `max_total` 0 to 5), yet the proofs of 1215, 1225, 1240, 1254, 1272, 1287 and 1300 have the same trees, node for node |
| `preclusions`, `match_bound=off`, `legacy_bound` and the branchings other than `conflict-strong`, again with the Lagrangian bound | [tactics review](#the-alternatives-that-are-off) | measured; to be removed | the proof of 1215 on one thread: `preclusions` +80% cycles and 4.5 s for its table, `conflict-largest` ×6.3, `largest`, `conflict-smallest` and `match_bound=off` unfinished at 180 s (more than ×17); `legacy_bound` acts only without the matching bound |
| `match_refine=0` with the Lagrangian bound | [tactics review](#the-alternatives-that-are-off) | open | the proof of 1215: cycles −12%, nodes +1%; its default needs the bench set and the small games |
| Support for old file formats: result schemas 1 to 3, the committed schema-1 files, checkpoint compatibility, the golden tests of the Java's tactics | [tactics review](#old-file-formats) | open: to go with the tactics | an inventory, with what deleting each takes along |
| Re-solve 1..1000 and 1001-1300 with no unproven tactic, and the non-primes of 1001-1231 standalone | [results](../results/README.md#what-supports-the-scores) | kept | `strict.jsonl` reproduces all 1000 scores of optimal.json; the 197 standalone searches agree with the chain |
| The Java's merged-away hive branch (the stale-hive quirk) in the frame builder | [`src/hives.rs`](../src/hives.rs) | measured | reached once (n = 114) for 2 ≤ n ≤ 2000, and forcing it off changes no frame |

### Theory and evidence

The model-free work of [evidence](../evidence/README.md) and the proofs of [THEORY.md](../../heuristics/THEORY.md).

| tried | details | outcome | what decided it |
|---|---|---|---|
| Validate the frame model's assumptions against exhaustive search | [DESIGN 2](../DESIGN.md#what-optimal-means) | kept | `brute` agrees with the frame model to n = 40 in the tests (52 in `sacrifice validate`) and `certify.py` to 62; the model-free proofs below extend it to 989 |
| A model-free exact search for opt(n): matching bound, branching on minimal unplayable sets, incremental dual repair, forced picks, the conflict-packing cut, nogoods | [evidence 3](../evidence/README.md#3-model-free-exact-cross-check-of-optn) | kept | proves optimal.json's score for every n ≤ 989 and 999-1019 in 10.0 CPU hours; the incremental repair made it 160 times faster than solving each node from scratch |
| A Lagrangian bound over the nogoods in the model-free search | [evidence 3](../evidence/README.md#3-model-free-exact-cross-check-of-optn) | kept | 3.3 times fewer nodes at n = 793 |
| Cap the upper half at W(U\*) through a Lagrangian in the model-free search | [evidence 8](../evidence/README.md#8-what-remains-open) | not kept | gains only 4% |
| The model-free search past 989: games 990, 992, 993, 995, 996 and from 1020 | [evidence 8](../evidence/README.md#8-what-remains-open) | open | each needs tens of millions of nodes; the searches are saved to be resumed; the matching bound's gap is about 1,170 at n = 1000; a relaxation that sees more of the game (a pick sweeps every divisor) is the next lever |
| The span certificate (♦) for the upper-half conjecture (★) | [evidence 2](../evidence/README.md#2-conjecture--at-scale) | kept | holds for every n = 2..100,000 (602,777,907 rejections) and 902 sampled games to 2,000,000, 0 violations |
| The threshold-by-threshold check of (★) by the cheapest sufficient bound | [evidence 2](../evidence/README.md#2-conjecture--at-scale) | kept | n = 2..20,000: 24,058,430 thresholds, none needing search (matching bound 86.2%, Lorea 8.3%, GF(2) rank 4.3%, one exclusion 1.2%) |
| Exact maximum-cardinality searches over the residual (`maxcard`, `plus1`) | [evidence 2](../evidence/README.md#2-conjecture--at-scale) | superseded by the bounds, kept as the fallback | n = 9170 reached only its first 16 thresholds (P = 127, g = 13); the rank bound certifies it in milliseconds |
| The earlier (★) campaign: exact searches over the residual components | [evidence 1](../evidence/README.md#1-what-existed-before) | superseded by (♦) | 3,231 of 3,236 components, n ≤ 1500 and samples to 4000, plus 2873 and 9170; five undecided (3103, 3872, 3936, 4000, 9170), all settled by the rank bound; its scripts were never committed |
| An adversarial search over Taxman-shaped synthetic instances | [evidence 4](../evidence/README.md#4-adversarial-search-what-protects-greedy) | measured | what protects greedy is the window law taken for the real ranking of the primes: 0 failures in 1,981 such instances, against 16-57% for uniformly random orders and for the window law with a random ranking of the primes |
| Enumerate every optimal set of every game, model-free; the theorems on uniqueness (Theorems 8 and 9) | [evidence 5](../evidence/README.md#5-is-the-optimal-set-unique) | kept | unique for every n ≤ 424; 30 of the 954 games completely enumerated, all in 425-779, have two or three, every one with upper part U\* |
| Does every optimal game have upper part U\*? | [evidence 8](../evidence/README.md#8-what-remains-open) | open | true of every game enumerated; Theorem 8 settles the upper half alone |
| Prove (♦), or (★), in general | [evidence 8](../evidence/README.md#8-what-remains-open) | open | certified game by game only; the sharpest form is (♦) for multiplicatively generated instances under any order obeying the window law |
| The exchange property (a matroid) for playable sets, to justify greedy | [THEORY: where exchange breaks](../../heuristics/THEORY.md#where-exchange-breaks) | not kept: false | n = 2873 has two maximal playable sets of different sizes; greedy survives because the stuck one is the light one |
| The single-sacrifice repair behind the frontier argument | [THEORY: the frontier argument](../../heuristics/THEORY.md#the-frontier-argument-and-exactly-where-it-stops) | not kept: false in general | at n = 9170 two minimal cores through d share nothing below it, so no single below-d sacrifice repairs |
| A matroid relaxation that is exact for the whole game | [evidence 5](../evidence/README.md#5-is-the-optimal-set-unique) | not kept: false | game 425 has two optimal sets, and a matroid with distinct positive weights has one heaviest independent set |
| Does an optimal game play the most moves, M(n)?  (The wiki's open question; in its whole-game form, the conjecture behind `revise_freedoms`.)  M(n) model-free from the matching number ν(n) and a unit-weight search, against the optimal game's moves, n = 2..1300 | [evidence 6](../evidence/README.md#6-does-an-optimal-game-play-the-most-moves); [THEORY: longest games](../../heuristics/THEORY.md#longest-games) | not kept: false | at n = 833, 834 and 835 every optimal game is one move short of the longest (the heaviest 363-move game of 833 scores 29 less); every other game to 1300 is a longest game, 1,293 by ν(n) alone; pruning branches that cannot reach M(n) moves would be unsound |

### The Python heuristics project

The strategies (solvent, onetax, maxturn, cascade, continuation), the certificates and the experiments that did not
pay (a 2-tax rule for OneTax, the fork oracle, trusting certificates as cutoffs, the matroid frame, matching-existence
as acceptance, bitsets) are in [heuristics/README.md, "Dead ends worth remembering"](../../heuristics/README.md#dead-ends-worth-remembering),
with what is proven, conjectured and measured in [its ledger](../../heuristics/README.md#the-theory-the-upper-half-is-a-matching-problem)
and [THEORY.md's table of evidence](../../heuristics/THEORY.md#evidence).  Entries that also bear on the Rust solver
(the matroid frame, the single-sacrifice route) have their row above.

## Index of rounds

| round | question | outcome | commits |
|---|---|---|---|
| [The port](#the-port-and-its-first-improvements) | does a fresh implementation of the Java's algorithm reproduce its answers? | yes, and the Java's upper bound had a defect (the runner-up), fixed behind `legacy_bound` | before 2026-10 |
| [Matching bound](#improvements-after-parity) | can a relaxation to bipartite matching bound the nodes better than `blameGame`? | kept; the default bound | before 2026-10 |
| [Seed and cap](#improvements-after-parity) | start from the previous game and stop at `n + opt(n − 1)` | kept (`seed_incumbent`, `prev_cap`) | before 2026-10 |
| [Conflicts of the matching bound](#conflicts-of-the-matching-bound-2026-10) | does the conflict of the heaviest matching make a better branching and a tighter bound? | kept: conflict branching, strong branching, refinement; 1000 from open at 120 s to 11 s | f8d922d to 38cf168 |
| [Forced members and nogoods](#forced-members-and-nogoods-2026-10-second-round) | can the search promote what every completion promotes, and learn conflicts as facts of the root? | kept: `fix_forced`, `nogoods`, bounds from the potentials, cheaper children; 1184 from 21.6 s to 1.3 s | 53a442a to 8bcfd36 |
| [Carrying nogoods over](#carrying-nogoods-over-2026-10-measured-not-kept) | start a game's table with the nogoods of the last | not kept: a larger table costs every node | f41fff8 (taken out again) |
| [Forgetting nogoods](#forgetting-nogoods-2026-10) | clear each searcher's table every N steps | kept (`forget_nogoods`, N = 50,000); 15–40% less time on hard games | 576e2e2 |
| [Single-member conflicts](#single-member-conflicts-belong-to-the-node-2026-10) | are single-member conflicts root facts that can be filtered once? | no: nothing to filter | (a prototype, not kept) |
| [Faster nodes, same trees](#faster-nodes-same-trees-2026-10) | make the inner loop cheaper without changing any tree | kept: 30% fewer cycles at the 1215 proof | e3b8ddc to 4b6ef38 |
| [Engine](#the-engine) | scaling to many cores, and checkpoints at any interval | kept: split requests per idle worker, pauses that cannot starve the workers | 2026-10 |
| [Same search](#same-search-2026-10) | conclude a game whose search problem an earlier game already settled? | kept (`same_search`, proven); 28.4% of the strict chain's search | f5a2af3 to 06efb4c |
| [Lagrangian bound](#a-lagrangian-bound-over-the-nogoods-2026-10) | can a Lagrangian bound over the nogoods be kept incrementally and pay in cycles? | kept (`lagrange`); proofs at the frontier about 3.3 times fewer cycles | e6b37d2, b4f6b05 |
| [Which tactics earn their keep](#which-tactics-earn-their-keep-2026-10) | which tactics pay in a strict chain at the frontier, and what support for old file formats can go? | recommended: remove `reuse_prev`, `revise_freedoms`, `legacy_bound`, `preclusions`, `match_bound=off` and the branchings other than `conflict-strong`; keep the proven switches | the solver of 243af9b |
| [Faster nodes with the Lagrangian bound](#faster-nodes-with-the-lagrangian-bound-same-trees-2026-10) | where do the cycles go with the Lagrangian bound, and what can be cheaper without changing any tree?  Does `match_refine` = 0 now pay? | kept: the probe's windows about 10.5% fewer cycles, proofs 5-10%; `match_refine` stays 1 | dd4f89c to 31b9c95 |
| [Benchmarks](#benchmarks-the-probe-pgo-and-scaling) | what the bench set, the probe, PGO and the thread scaling showed | numbers | e3b8ddc and after |
| [What drives the slope](#what-drives-the-slope-2026-10) | what makes a game's cost grow ×10 per 55-70 in n, and what would change the slope? | the root's gap: ×10 nodes per ~130 of it at every n, nothing from the size at a given gap; the slope is the gap's growth; seeding the search with a better root relaxation is a constant factor | db7faf1 (no solver change) |

## The port and its first improvements

The Rust solver reimplements the algorithm of the Java solver rather than translating it (DESIGN.md, "The Java
solver and this one").  Its setup had to reproduce the Java's values exactly (the golden data in
`testdata/golden/` is the check) and every final score had to equal optimal.json; the search itself was
designed for speed from scratch.  One defect of the Java was found and fixed on the way:

* **Upper bound runner-up (a defect of the Java's bound, fixed in the port; `legacy_bound` brings it back)** (`UpperBoundEstimator.getRunnerUp`): when
  `largestThatFitInFrames` skips candidates because frame 1 or the last frame
  is at its cap, the Java replaces the failing number with the next candidate
  below the smallest bid member.  A skipped candidate from the failing
  number's frame can be larger, so the Java bound can be too low (exhaustive
  checks find it below the best completion, e.g. at the root of n=114: 358
  against 360).  Sound version: the heaviest set within the caps that leaves
  out some bid member at or above the failure, which by matroid exchange is
  the bid with one member swapped for the best outside candidate the caps
  allow.  (Simply taking the largest candidate not in the bid is also sound
  but swaps skipped candidates of a full frame in for members of other frames;
  it made n=700 take over 10 minutes instead of 18 s.)  (Re-solving n=2..760
  in Java with the simple fix changed no answers; n=740 timed out.)
* `Memo.test` computes its value twice — moot, `Memo` is not ported.
* `TxSet.MAX_VALUE = 1023` — gone; set width is chosen from n.

## Improvements after parity

* **Matching bound** (sound).  Relax winnability (peel) to plain
  matchability across the whole frame chain: maximize the promoted sum
  subject to every original move and every promoted number getting a
  distinct maximal factor in its frame, and no promoted number being used
  as a factor.  Edges v → v/p change Ω(v) by one, so this is a max-weight
  *bipartite* matching (min-cost flow), exactly solvable.  Measured at the
  root of all 94 golden games that need search (scratch study): mean gap to
  the optimum 31 vs 76 for the Java bound; on the bench games 107–292 vs
  363–769.  The cardinality cap never binds.  To be used incrementally
  (re-optimized along the DFS path), at first combined with the Java bound
  by taking the minimum.  (That was dropped later: `blameGame` never pruned
  where the matching bound did not, so the shipped bound is the matching
  bound alone.)
  *Implemented* in module `matchbound` (used by `search`, tactic
  `match_bound`): an optimal matching and its dual potentials per level of the DFS path,
  repaired by Hungarian searches after each change; 0.7–2.4 µs per
  evaluation in a DFS at n=480–760, against 30–170 µs from scratch.  A test
  DFS that starts with the optimum as incumbent and evaluates the matching
  bound only where blameGame fails to prune visits 85, 34, 1.5, 2.7, 1.7,
  32 and 35 times fewer nodes at n=480, 500, 600, 640, 660, 700 and 720
  (taking 57, 26, 1.5, 2.5, 1.6, 24 and 28 times less time), and at n=760
  finishes in 90k nodes where blameGame alone passes 3M.  The cap never
  bound there either.
* Seeding the incumbent from n−1 and stopping at n + opt(n−1) (tactics
  `seed_incumbent`, `prev_cap`).

## Conflicts of the matching bound (2026-10)

Measured with `examples/ladder.rs` (see DESIGN.md, "Performance") on 1 thread for 760,
880 and 936 and on 10 threads for 1000 and the hard games near it.  With the
matching bound alone, n=1000 was not proven in 4 minutes on 10 threads.

What the diagnostics showed first (n=760–1000, `--diag`): the matching bound
does nearly all the pruning (blameGame almost never prunes where it does not),
and at the nodes it fails to prune it is 25–45 above what was needed
(median); the heaviest matching there essentially never promotes a feasible
set; the tree peaks at 25–40 promotions deep.  Making the bound tighter was
the lever, not cheaper nodes.

* **Conflicts** (`Model::conflict_using`).  When the heaviest matching's
  candidates `Q*` cannot all be promoted, some frame is unwinnable with them;
  the moves a peel of it cannot drop (its *core*) are stuck however the order
  goes, and the members of `Q*` the core needs (promoted into the frame and
  core moves, or promoted out of it and factors of core moves) cannot be
  promoted together.  Minimized by peeling only the core, these conflicts are
  tiny: two members at the median and p75 at n=760, p50 2 / p90 5 at 996 (a
  quarter are single members: a candidate that cannot be promoted from that
  node's state, though it can from the root; see "Single-member conflicts
  belong to the node").  A subset of a promotable set is promotable, so
  every completion leaves out some member of a conflict — the basis of all
  that follows, and of `search::refine::Refiner`'s soundness proof.
* **Refinement** (`match_refine`).  Bound the branches of the conflict too
  (`k₁` out; `k₁` in and `k₂` out; …), a few levels deep.  Alone, on the
  Java's branching, it cut nodes 2–70x but cost more per node than it saved
  (n=880: 4.8M → 68k nodes at depth 7, 31 s → 21 s; at depth 1–3 slower than
  without).  Caching known conflicts along the path and branching disjointly
  made it pay at 760; with conflict branching it is worth one level (default
  1; 2–15% faster than 0 on the benchmarks, deeper is slower).
* **Conflict branching** (`branching`).  Branch on a member of the conflict
  instead of the largest candidate, so that neither the child nor the node
  going on can reproduce the matching: the decisive change.  `conflict-strong`
  (default) branches on the member whose removal lowers the matching bound
  most (one matching solve per member; the level for the chosen one is kept
  for the node, `MatchBound::keep_excluded`).  Promoting a member is not worth
  looking at: the heaviest matching promotes them all already; and a lookahead
  that also scores the child with the other member of a pair out did worse.
  `conflict-smallest` gives the smallest proof trees at 880/936 but finds
  good incumbents late; `conflict-largest` follows the dive and finds them
  early.  Branching on the members of every failing frame's conflict instead
  of the smallest conflict was worse (936: 754k → 1.0M nodes).
* **Exact matchings**.  A heaviest matching that can be promoted is the
  node's best completion: offered as incumbent, node done.
* **Propagation**.  A known conflict of which a child promotes all but one
  member rules that member out of the child's candidates.  Without it the
  child's matching bound equals the node's (its matching promotes the child's
  number already).  −17% nodes.
* **Starting incumbent**.  `dive` rounds the root's matching (drop the
  smallest member of a conflict until feasible) — weaker than greedy on most
  games; a one-step lookahead in the dive was no better.  A local search
  (`Neighbourhood::improve`: fill greedily, then drop one or two members and
  refill) reaches the optimum at the start for 945 and 988 and gets close for
  950/984/990 (20–40 ms).  Running it on every incumbent found during the
  search, and dives at search nodes, gave nothing measurable.
* **Cheaper nodes**: a per-frame cache of conflict answers (consecutive
  matchings change few frames; −5%); a radix heap in the Hungarian search
  (−16–19%); stopping the scan of frames at a single-member conflict.
  Choosing the frame by the smallest *core* and minimizing only it was much
  cheaper per node but its conflicts branch far worse (n=1000 unfinished in
  2 min).  Checking every candidate a promotion affects (a complete
  `can_promote` filter instead of the Java's partial one) cut nodes 25–35% but
  quadrupled the cost per node.  The preclusion table still pays on long
  searches (936: 15.7 s → 11.8 s) though it costs ~1.4 s of setup on one
  thread.  (No longer: see "Forced members and nogoods", below.)

Per change (search seconds and nodes; 1 thread for 760/880/936 seeded from
n−1, 10 threads for 1000; 10-thread times vary with when the optimum is
found):

| commit | change | 760 | 880 | 936 | 1000 (10 thr.) |
|---|---|---|---|---|---|
| f8d922d | conflicts, ladder, diagnostics (Java's branching) | 1.13 s 205k | 30.7 s 4.78M | 90.8 s 12.3M | open at 120 s |
| 6c4e6ee | branch on conflict (largest member) | 0.13 s 10.1k | 11.2 s 909k | 31.8 s 2.62M | 27.5 s |
| 264d1ab | cache of frames' conflicts | 0.12 s 10.1k | 10.5 s 909k | 29.5 s 2.62M | 30.7 s |
| d1c256c | strong branching | 0.12 s 7.2k | 3.8 s 243k | 17.7 s 909k | 22.8 s |
| 75155b4 | propagation into children | 0.12 s 6.5k | 3.6 s 202k | 16.8 s 759k | 18.7 s |
| 5ef83ea | keep the strong-branching level | 0.11 s 6.5k | 3.25 s 202k | 15.0 s 759k | 18.7 s |
| f6aef0f | local search on the start | 0.12 s 6.5k | 3.2 s 202k | 14.8 s 758k | 17.1 s |
| 1e7a3de | radix heap | 0.11 s 6.5k | 2.7 s 202k | 12.0 s 758k | 12.6 s |
| 38cf168 | refine one level | 0.09 s 4.0k | 2.6 s 142k | 11.1 s 500k | 11.1 s |

The ladder (10 threads, standalone from the frame builder, seeded from n−1):
before, 992 and 1000 were open at 100 s (992's incumbent not even optimal),
960 took 37 s and 800 13 s, and 945, 948, 950, 969, 976, 984, 987, 988, 990,
996, 999 were open at 20–30 s.  After, all of them complete: 760 0.03 s, 800
0.5 s, 840 0.1 s, 880 0.3 s, 900 0.1 s, 960 1.6 s, 945–999 1.4–24 s except
992 (62 s, of which 42 s finding the optimum) and 996 (69 s, all proof).
The whole range 761–1000 through `taxman solve` (prime shortcut and
`reuse_prev` on; 81 games searched) takes about 5 minutes of game time on 10
threads, every score matching optimal.json; 1001–1100 about 6.5 minutes
(36 searched, slowest 1025 at 100 s) and 1101–1200 about 2.5 minutes (with
`reuse_prev` and `revise_freedoms`, as the Java's results).  Threads scale
linearly to the 8 performance cores; the 2 efficiency cores add little.

Left to try: making the per-node conflict work cheaper (a quarter of the
time: peeling each frame `Q*` touches), fewer matching solves per node, and
incumbents for games like 992 whose optimum is found late.

## Forced members and nogoods (2026-10, second round)

Measured with the ladder on one thread (deterministic node counts) with the
frontier's tactics (`revise_freedoms=off`), seeded from n−1 (from
`results/strict.jsonl` past 1000, `--known`), on a fast set that each finish
in under 25 s: 880, 936, 1050 (optimum found late), 1100, 1170, 1184 (proof
only).  (The first rows were measured while a production run used 6 cores,
the rest on an idle machine; the first row measured again idle came out the
same to 1%.)  Where the time went at 1184 (`sample`): peeling for conflicts 29%
(nearly all of it minimizing them), matching repairs for refinement 18% and
for strong branching 16%, the candidate filter's peels 16%.

The tree is mostly chains: at four nodes in five the node's loop ends right
after its first child, because removing the member that strong branching
chose already brings the bound below what the node needs.  That member is
*forced*: every completion that beats the incumbent promotes it.  Strong
branching computes the bound without each member of the conflict, and at
1184 more than one member was forced at 54% of the nodes (two at 16%, three
at 10%, four or more at 28%).  Before, the child promoted one of them and
found the others again, one level each, further down.

* **Forced members** (tactic `fix_forced`, sound).  The child promotes every
  forced member of the conflict, not only the chosen one: the child's
  completions are some of the node's, so those that beat the incumbent
  promote them all, and if one cannot be promoted any more (or the child's
  candidate filter dropped it) the child has no such completion at all.
  Nodes −31% (880) to −60% (1170), time −21% to −36%.
* **Reusing the child check.**  A child's bound is checked (with refinement
  and its conflict) when it is made; its loop's first step repeated that with
  the same need for the same verdict.  It now keeps the verdict.  And a child
  that promotes several numbers makes its candidate list once, after all of
  them.  −3% to −23% time.
* **Nogoods** (tactic `nogoods`, sound).  A conflict `K` found at a node with
  promotions `P` is relative to `P`; a minimal subset of `P ∪ K` that keeps
  a frame of the *root* unwinnable is a *nogood*: it cannot be promoted from
  the root, so at every node anywhere in the tree (any thread, any path, and
  after a resume) the nogood minus that node's promotions is a conflict of
  the node (a subset of a promotable set is promotable; the proof is in
  `model::Nogoods`).  Measured first: at 1184, 291k conflicts were computed
  but they were only 854 distinct nogoods — the search found the same few
  hundred over and over.  So each searcher keeps a table of them (at most
  65536, forgotten all at once when full; about a thousand are learned near
  n = 1200), and a node's conflict is the smallest the table knows within
  `P ∪ Q*` (of those as small, the one with the largest member); only when it knows none
  is one computed, and then learned.  Nogoods also rule candidates out of a
  child (a nogood all of whose other members the child promotes, indexed by
  member), beyond the conflicts known along the path.  Looking conflicts up
  rather than computing them is cheaper, and also better for the tree: the
  smallest of the many conflicts known is often smaller than the one the
  minimization of a single frame finds, and branching on it and on the
  child's propagation cut nodes (936: 320k → 50k).  The tree now depends on
  what the searcher has learned, so on its history (threads, interruptions),
  not only on the node; answers do not.  Alone, lookups gave most of it
  (880 → 39k nodes, 936 → 57k, 1184 → 287k); propagation alone, with every
  conflict computed and made a nogood, cut nodes less (880 → 61k) and cost
  more than it saved.
* **All members forced.**  When every member of the conflict is forced, no
  completion beats the incumbent: the node is done (free once strong
  branching has computed the bounds; only matters with `match_refine=0`,
  since refinement prunes those nodes first).
* **Bounds without a member from the potentials** (`MatchBound::bound_excluding`).
  With the conflicts looked up, matching repairs were two thirds of the
  time, nearly all of them for the bound without one member of a conflict
  (strong branching, and refinement's branches).  The level's potentials are
  a dual solution whose sum is its bound; without the member as a candidate
  they stay feasible, and lowering the member's potential and that of the
  factor it paid with as far as their remaining edges allow gives an upper
  bound at once: `value − x + y′ₓ + y′_w`.  At 1184 it shows 89% of the forced
  members forced (and is never below the exact value, which the matching
  tests check).  Strong branching (with `fix_forced`) and refinement solve
  only for the members it does not settle; when some member is forced, the
  node's loop ends after the child that promotes them (it covers every
  completion that can beat the incumbent), so no level is kept for it.  The
  same trees, 13–28% less time.
* **Forced members of the whole matching.**  The bound from the potentials
  is cheap enough to compute for every member of the heaviest matching, not
  only for the conflict's: at each check that refines (or wants a conflict)
  the forced members of `Q*` are found that way.  If the nogood table knows
  a conflict all of whose members are forced, or the conflict found is, the
  node is done (diagnostics had shown that refining by the fresh minimal
  conflict instead of the table's would prune 16% of the nodes left open:
  other conflicts matter).  Otherwise, if any are forced, the child promotes
  them all and strong branching is not needed; only when none is does strong
  branching solve for its conflict's members.  The table is scanned once
  for both (the smallest conflict, and any conflict all forced), its
  nogoods' members ordered with the conflict they came from first, as those
  are the likeliest to be missing at another node.  Nodes −8% (936) to −52%
  (1184), time −2% to −39%; 992 seeded with its optimum (the proof alone,
  one thread): 2.89M → 2.11M nodes.

  Tried and not kept: proving more members forced with the forced ones
  required (a second pass of the bounds from the potentials on a level with
  them promoted) pruned nothing more; skipping the exact solves of strong
  branching when the bounds from the potentials found some member forced
  (instead of whenever they found any candidate forced), or of the
  refinement's branches, or choosing the member to branch on by those bounds
  alone, each traded nodes for time at about even.
* **Cheaper children.**  Peeling had become the largest cost (37% of the
  time at 1215: promotions and the candidate filter's checks).  With the
  matching bound, a child's candidate list leaves out the Java's
  promotability checks: a candidate that cannot be promoted is found by the
  conflicts and nogoods anyway, and nodes stayed within ±8%.  The forced
  members are promoted together, each frame checked once
  (`Model::promote_all_using`) instead of two frames per member (together
  they fit the frames' freedoms exactly when no frame they touch ends below
  zero).  And blameGame is not computed with the matching bound: it never
  pruned where the matching bound did not (diagnostics: 0 at every check).
  −30% to −59% time.
* **Nogood signatures.**  The table's scans were then 17% of the time at
  1215 (a few hundred nogoods, a lookup per check).  Each nogood keeps a
  256-bit signature (bit `x mod 256` per member), and a scan skips those
  whose signature is not within the node's; the conflicts chosen are the
  same as before (the smallest, then the one with the largest member).
  −5% to −10%.
* **No preclusion table** (`preclusions` now off by default).  With the
  nogoods it no longer pays for its setup: the searches of 992 and 1215
  seeded with their optimum take +5% and −4% without it (one thread), and
  1101–1217 through `taxman solve` on 8 threads spent 53 s of its 131 s
  building tables (0.8 s per searched game, 4.4 s on one thread at 1215) and
  72 s searching; without them 80 s in all, the search 74 s.
* **No refinement where members are forced.**  A node with forced members
  has a single child, which promotes them; refining the node's conflict is
  left to that child.  Nodes +3% to +11%, time −2% to −8% (992 and 1215
  seeded with their optimum: −3.5% and −6.5%).

Tried and not kept in this round:

* Rounding the heaviest matching into an incumbent at every node (drop the
  smallest member of a conflict until the rest can be promoted, then fill
  greedily): five times slower per node, and the optimum of 1050 was found
  at the same step.
* A large-neighbourhood search before the search: free a window of the
  incumbent's members (by frame pair, or 15–40 consecutive members), keep
  the rest promoted, and search that unit with a node limit.  At 1025 it
  reached 25080 in 1.3 s (the search takes 11 s to), but the optimum, 7
  members out and 8 in from there across frames 1–4, was found only by the
  search, at about the same point (one thread: 160 s instead of 170 s).
  The incumbents of 1025 and 1050 improve late because the search reaches
  the subtree that holds them late, not because a nearby set is missed.

One thread, fast set (search seconds and nodes; 880 and 936 seeded from
optimal.json, the rest from `results/strict.jsonl`):

| commit | change | 880 | 936 | 1050 | 1100 | 1170 | 1184 |
|---|---|---|---|---|---|---|---|
| 8686be0 | before | 2.72 s 142k | 11.65 s 500k | 24.8 s 598k | 4.81 s 161k | 4.58 s 165k | 21.6 s 786k |
| 53a442a | forced members | 2.14 s 98k | 8.61 s 320k | 18.2 s 342k | 3.17 s 74k | 3.09 s 66k | 14.7 s 338k |
| 261b342 | reuse the child check | 1.97 s 98k | 8.05 s 320k | 16.8 s 340k | 2.65 s 74k | 2.35 s 66k | 11.9 s 338k |
| c33c3c2 | nogoods | 0.66 s 34k | 1.09 s 50k | 11.8 s 318k | 2.12 s 75k | 1.18 s 46k | 6.41 s 276k |
| 667b24d | bounds from the potentials | 0.52 s 34k | 0.85 s 50k | 8.72 s 318k | 1.65 s 75k | 0.91 s 46k | 4.87 s 276k |
| 356ef92 | forced members of the whole matching | 0.46 s 31k | 0.83 s 45k | 6.45 s 241k | 1.31 s 51k | 0.92 s 35k | 2.94 s 133k |
| fe73c1c | cheaper children | 0.29 s 30k | 0.53 s 47k | 4.68 s 264k | 0.73 s 51k | 0.40 s 35k | 1.28 s 126k |
| 372c3f9 | nogood signatures | 0.26 s 30k | 0.49 s 47k | 4.43 s 264k | 0.68 s 51k | 0.38 s 35k | 1.21 s 126k |
| 49517a1 | no refinement where members are forced | 0.24 s 32k | 0.45 s 52k | 3.99 s 290k | 0.67 s 56k | 0.37 s 38k | 1.13 s 130k |
| 8bcfd36 | no preclusion table | 0.23 s 31k | 0.50 s 57k | 3.88 s 289k | 0.63 s 55k | 0.36 s 38k | 1.34 s 159k |

(Setup is left out: the preclusion table took 1.3–3.9 s of it on one thread
for these games, now nothing.)

The hard games, standalone in the ladder as above, on an idle machine (search
seconds; with several threads the times vary from run to run with when the
optimum is found, so two runs are shown for the last version; in
parentheses, when the final incumbent was found):

| game | 8686be0, 8 threads | 8bcfd36, 8 threads | 8bcfd36, 2 threads |
|---|---|---|---|
| 1215 | 195.6 s (71.4) | 12.1 s (7.4), 9.8 s (2.6) | 35.7 s (13.8) |
| 1216 | 51.5 s (start) | 3.1 s, 3.3 s | 10.4 s |
| 1025 | 100.1 s (76.7) | 12.5 s (4.6), 23.9 s (15.0) | 93.5 s (70.0) |
| 992 | 71.4 s (56.8) | 11.8 s (11.4), 12.0 s (11.8) | 40.0 s (39.2) |
| 996 | 76.4 s (start) | 10.2 s, 10.0 s | 31.5 s |

`taxman solve 1001-1217 --threads 8 --tactic reuse_prev=off --tactic
revise_freedoms=off` (no unproven tactic) reproduces every score of
`results/strict.jsonl` in 164 s of game time; those results took 1244 s on
10 threads.  Its slowest games: 1025 21 s (110 s before), 1215 10 s (161 s),
1020 6 s (40 s).

Left to try: the incumbents of games like 992 and 1025, whose optimum the
search reaches late (992 on 8 threads: at 11.8 s of 12.0 s; seeded with its
optimum it takes about 6 s); the matching repairs, then about 40% of the
time (strong branching where nothing is forced, and refinement; see "Faster
nodes, same trees" for later); and the multi-thread search: each thread
learns its own nogoods (sharing them did not look worth it: a proof on 8
threads takes about as many nodes as on one).

## Carrying nogoods over (2026-10): measured, not kept

A nogood is a fact about the root of a game's frame model, and consecutive
games have nearly the same model, so the nogoods a search ends with could
start the tables of the next game's search, each checked at the new root
first: a set that cannot be promoted from the root (an exact test, frame by
frame) is a nogood of it whatever it came from, and `conflict_using` makes it
minimal again.  For the same reason a checkpoint could store the tables and a
resumed search start with them.  A prototype measured both (commit f41fff8:
`search::valid_nogoods`, the solver's tactic `carry_nogoods`, `ladder
--carry`, `examples/resume_carry.rs`); neither pays, and both were taken out
again.

* Nearly every nogood survives into the next game: 1374 of 1374 from 1024 at
  1025, 482 of 494 from 1219 at 1220, 1216 of 1251 from 1229 at 1230.  But
  game n − 1 often does not search: a prime takes the shortcut, and 1214,
  1224 and 1226 reach the cap with their seed (no nodes), so in a range the
  nogoods come from the last game searched (1227's from 1225, through 1226).
* The table never stops growing, but learning costs little.  On one thread,
  half of what a hard game learns is learned in the first 3–27% of its search,
  90% by 18–64%, and the last nogood in its last 1–17% (1024, 1025, 1215,
  1219, 1220, 1225, and 1230 stopped at 560 s); each is one conflict computed
  where the table knew none, a few thousand in a search of millions of nodes.
  So what a preloaded table could save is not the learning but the search's
  knowing smaller conflicts sooner, which changes the tree.
* It changes, but not for the better (one thread, strict tactics, seeded from
  n − 1 and capped as the solver does; search seconds and nodes):

  | game | tables start empty | with the nogoods of |
  |---|---|---|
  | 1025 | 158 s 8.50M | 1024: 136 s 6.30M |
  | 1025, proof only (seeded with its optimum) | 99 s 5.25M | 1024: 104 s 4.97M |
  | 1220 | 186 s 13.72M | 1219: 232 s 16.47M |
  | 1220, proof only | 177 s 13.51M | 1219: 240 s 17.81M |
  | 1227 | 260 s 17.42M | 1225: 267 s 15.72M |
  | 1230 | open at 560 s (best 36570 at 118 s) | 1229: open at 560 s (36570 at 186 s) |
  | 1025, 4 threads | 35.2 s 8.51M | 1024: 38.4 s 6.82M |
  | 1220, 4 threads | 54.0 s 17.16M | 1219: 68.0 s 15.91M |

  1227 is the ceiling: 1225–1229 have the same frames and candidates (the
  same search, 17,417,736 nodes on one thread), so 1227 started with all that
  1225 learned is the best a carried table can be, and it takes 10% fewer
  nodes and 3% more time.  The chain `taxman solve 1201-1231 --threads 4
  --tactic reuse_prev=off --tactic revise_freedoms=off` from 1200 in
  `results/strict.jsonl` took 617 s (154.7M nodes) without and 941 s (174.7M)
  with it, every searched game slower (1230: 305 s against 445 s); there the
  table grows from game to game.
* Why: a larger table costs every node.  A lookup scans every nogood's
  signature and the nogoods of each new promotion rule candidates out, so
  nodes are slower the more the table holds (1227: 59k nodes/s from a full
  table against 67k/s from an empty one; on 4 threads, whose tables start
  with the union of all threads' nogoods, 177k/s against 242k/s at 1025).
* Keeping the tables over a resume: a search stopped every K nodes and
  resumed from its checkpoint, with the tables as they were when it stopped
  (in memory, what a stored table would give), reproduces the uninterrupted
  search; resumed with empty tables it is a different tree, and faster (one
  thread):

  | game, stopped every | uninterrupted | resumed with empty tables | with the tables kept |
  |---|---|---|---|
  | 1024, 250k nodes | 41 s 2.28M | 34 s 2.32M | 42 s 2.26M |
  | 1215, 500k | 66 s 5.49M | 60 s 6.22M | 69 s 5.47M |
  | 1220, 1.5M | 186 s 13.72M | 143 s 11.91M | 191 s 13.48M |

  So a resume loses nothing by starting empty, and checkpoints stay as they
  were.  That old nogoods cost more than they help is taken up in
  "Forgetting nogoods", below.

Noticed on the way: many games of a range have exactly the frames and
candidates of an earlier one (in 1201–1231: 1202–1203 those of 1201, 1205 of
1204, 1207 of 1206, 1210–1211 of 1209, 1213–1214 of 1212, 1217 of 1216, 1219
of 1218, 1222–1223 of 1221, 1226–1229 of 1225, 1231 of 1230), so the same best
promotion set, which a search of one of them has already established.  Only
the cap differs: where it is higher than the optimum, the search is repeated
in full.  In the strict chain above that is 1203, 1205, 1207, 1211, 1219 and
1227: 90 s of the 617 s.

## Forgetting nogoods (2026-10)

The measurements of carrying nogoods over pointed the other way: a search
resumed with empty tables was faster than one that kept them.  So each
searcher now clears its table after each 50,000 of its own steps (tactic
`forget_nogoods`, `search::NOGOOD_PERIOD`; sound, since the table is a cache
and every search starts with an empty one anyway; deterministic on one
thread).

The interval, on one thread (search time of 1024, 1025, 1215 and 1220 added
up, against never forgetting run alongside): every 10,000 steps 22% less
time, 25,000 22%, 50,000 26%, 100,000 25%, 250,000 21%, 1,000,000 15%.  The
node counts move both ways (1025 +7% to +28%, 1220 −11% to −28%); what the
interval buys is cheaper nodes, since a lookup scans the whole table (1215:
110k steps/s forgetting every 10,000 steps, 83k/s never).  A game of fewer
steps than the interval is unchanged.  Tried and not kept:

* Forgetting by size, clearing the table whenever it holds 64, 128 or 256
  nogoods: 15%, 22% and 17% less time on those four, but the smaller games
  slower, since they clear while still learning (936: 0.51 s against
  0.85/0.61/0.53 s; 1050: 3.96 s against 5.19/4.49/4.33 s).
* Keeping the nogoods used since the last clearing (returned by a lookup, or
  ruling a candidate out of a child): every 100,000 steps 15% less time,
  every 500,000 6%.  The ones kept cost lookups as much as the others and help
  no more.

Never forgetting against forgetting every 50,000 steps (search seconds and
nodes; strict tactics, seeded from n − 1 and capped as the solver does; the
fast set 880–1184 is unchanged to ±4%):

| game | one thread, never | one thread, forgetting | 4 threads, never (two runs) | 4 threads, forgetting |
|---|---|---|---|---|
| 992 | 73.3 s 4.00M | 52.6 s 4.20M | 19.5 s, 16.9 s | 18.0 s, 16.5 s |
| 996 | 69.0 s 4.17M | 61.9 s 4.78M | | |
| 1024 | 43.0 s 2.28M | 34.3 s 2.38M | | |
| 1025 | 168.5 s 8.50M | 140.5 s 10.11M | 35.3 s, 35.3 s | 32.1 s, 32.3 s |
| 1215 | 69.6 s 5.49M | 50.6 s 5.16M | 21.8 s, 21.0 s | 15.3 s, 16.1 s |
| 1216 | 22.4 s 2.10M | 21.1 s 2.22M | | |
| 1220 | 192.2 s 13.72M | 125.4 s 10.51M | 60.6 s, 55.1 s | 38.8 s, 40.5 s |
| 1225 | 262.4 s 17.42M | 154.2 s 12.07M | 54.7 s, 55.2 s | 31.9 s, 33.4 s |

The strict chain 1201–1231 on 4 threads (as above) takes 377 s instead of
571 s, with every score the same (1230: 158 s instead of 275 s; 1225 33 s
instead of 55 s; 1220 39 s instead of 62 s).

## Single-member conflicts belong to the node (2026-10)

Many of the conflicts the search computes have one member (`--diag`'s minimal
conflict sizes: the 25th percentile is 1 at 936, the 10th at 1184).  They
looked like candidates that the Java's filter lets through and that cannot be
promoted from the root at all, which every searcher, work unit and resume
would find again; testing each candidate once from the root and dropping the
failures would then be free.  They are not root facts:

* **The frame builder's candidates all pass that test.**  A hive's promotion
  candidates are its factors that can leave it (the remaining sources can
  still be matched without them) and that some hive can take on as a source,
  which comes to what `Model::can_promote` checks at the root (the home frame
  without the factor, the frame below with it as a move).  For every game from
  2 to 1230 none of the candidates fails that check (a test), so a filter at
  the root drops nothing.  A version built as a tactic (the checkpoint header
  recording the candidates it leaves, older checkpoints refused as a different
  candidate set) searched 936, 1100, 1184 and 1215 on one thread in exactly the
  same nodes as without it (58,903, 56,399, 167,960 and 5,161,817), with 0 of
  225, 273, 287 and 302 candidates dropped; it was not kept.
* **The conflicts are relative to the node.**  A nogood of one member would be
  a candidate that cannot be promoted from the root, so the table never holds
  one.  A single-member conflict is a candidate that cannot be promoted once the
  node's promotions are made (its frame may have used it up in a forced pair,
  say); in another branch, with other promotions, it is a nogood of two or more
  members.  Looking it up is cheap, and the node's loop then goes on without
  the candidate.
* **Little is left to save.**  The promotions that fail are 2,282 of 58,903
  steps at 936 (3.9%) and 698 of 167,960 at 1184 (0.4%) (`--diag`, one thread),
  which bounds what a search that never tried a dead candidate could gain.

## Faster nodes, same trees (2026-10)

The inner loop, made faster without changing any answer or tree: every
change keeps the node counts of every game and the fingerprints of the
probe (`examples/perf.rs`, see DESIGN.md, "Performance"), and was measured in cycles and
instructions retired of the process (other load moves wall times by up to
3×), on one thread, plain release build.  Before, at the 1215 proof: the
Hungarian repairs about 50% of the time, the peels 19%, the bounds from the
potentials 5%, the nogood table 8%, level copies 4%.

| commit | change | 1215 window k cycles/step | 936 G cycles | 1184 | 1215 proof |
|---|---|---|---|---|---|
| e3b8ddc | before | 22.1 | 1.59 | 4.33 | 151.3 |
| b4cb2b6 | set widths of 20, 24 and 28 words (see "The engine", below) | 20.4 | 1.59 | 4.05 | 142.3 |
| ab76772 | radix heap's bucket 0 a bitset, the search's per-vertex fields in one slot, a level settled only when something changed | 18.8 | 1.47 | 3.65 | 129.3 |
| 130e4ec | `MAX_FRAMES` 12 → 8 (every board up to 4095 has at most 8) | 18.7 | 1.46 | 3.58 | 125.9 |
| 9bfb4d6 | nogood lookups exact over compact member indices, not signatures and member loops | 18.2 | 1.43 | 3.52 | 123.2 |
| 319753b | no peel for a promotion whose new move has a factor no other move uses (a sixth of them) | 18.0 | 1.41 | 3.53 | 122.5 |
| ad1237c | nogood rule-outs over the compact sets too | 17.8 | 1.35 | 3.49 | 118.9 |
| 4b6ef38 | the radix heap's push and pop and `add_inner` inlined (they were calls, 300 a step) | 16.1 | 1.23 | 3.16 | 106.2 |

In all 30% fewer cycles (1.42×) and 28% fewer instructions on the proof of
1215 (33 s instead of 47 s), 27% at 1184, 23% at 936; the probe's windows
27% (1215) and 26% (936).  Profile-guided optimization, 12% on the code
before, now adds 0–2% (see DESIGN.md, "Performance").  The strict chain 1..1231 on 8
threads takes 279 s of game time (1201–1231: 145 s), every score as
committed.  Where the time goes now (1215): the Hungarian repairs 46%, the
peels 24%, the bounds from the potentials 7%, the node's own work 7%,
level copies 4%, nogood lookups 3%.

Measured and not kept (cycles, one thread):

* An undo trail for the matching levels instead of a copy per push: pushes
  cost little, but logging every change of a repair costs more (windows
  +0.8% and +0.9%, instructions +4%).
* The peel in frame-local form (each frame's movers and factors numbered
  within it, kept in the frame set): with the bit-parallel peel in rounds the
  checks of single promotions were 17–26% cheaper, but the frames of forced
  sets need many rounds (windows +8% and +16%); with the counting peel on
  local indices, +0.5% and +0.8%.
* Caching frame checks: 14–17% of the peels in a window repeat an earlier
  one exactly, but only 5% one of the last eight.
* The no-peel test above for the several moves of a forced set; promoting a
  forced set straight into the child's state; stopping the bound from the
  potentials once its first part decides; the Dijkstra horizon (no events
  later than a known settling one: windows +0.4% and +2.5%, as in an earlier
  measurement); one branch for `reach`'s two tests: each within ±0.5% or
  worse.
* Unchecked indexing in the Hungarian search: −4% instructions but −0.7%
  cycles, not worth `unsafe`.
* Widths of 12 and 48 words: 3–4% at 512–767 (solved) and some at 2048–3071,
  for 12 s more of every build.
* Not done: `i32` potentials (no bound on the potentials is proven, so not
  exact by construction); a warm-started unit rebuild (another optimal
  matching can change the tree after a split or a resume); candidate-index
  sets for every search-level set (a large refactor); watched members for
  the nogoods (now 3% of the time).


## The engine

How the engine ([DESIGN.md, "Engine"](../DESIGN.md#6-engine); the protocol is in the module comment of
`src/engine.rs`) came to be as it is.  All of this is from 2026-10 unless noted.

* **Yields leave out nodes with nothing to do.**  The Taxman searcher leaves out of its yield the nodes of its stack
  that have no candidates left (a node whose loop `fix_forced` ended): as units they would only be runs of one step
  that end the loop.  At 2 threads on 936, 1100 and 1184 (twice each, `ladder --threads 2,2`) the runs of at most
  one poll fell from 60 and 91 of 256 and 305 to 13 and 10 of 227 and 200, the nodes and times unchanged.
* **Splits, sized for many cores** (for a 192-core machine).  The first design answered one split request at a time,
  which serialized the distribution of work at many threads.  The engine now keeps one open split request per idle
  worker that the queued and held units will not serve, and any running worker can claim one (an atomic decrement
  of a count in the word `poll` reads), so as many workers split at once as there are idle workers to feed.  From
  one unit, all workers are busy after a few rounds in which every running worker splits, not after one split per
  idle worker.  A worker answers only after 64 polls of its current unit, so a fresh unit first gets deep enough
  to have something to give, and no worker restarts its unit at every step while the demand lasts.
* **One deque.**  The deque stays a single locked one: units exist only after yields, each costs its searcher a
  state rebuild (for Taxman a matching solved from scratch, 30–170 µs already at n = 480–760, against about a
  microsecond under the lock), and with every worker busy nobody takes the lock.  Per-worker deques with stealing
  would remove contention that is not there, at the price of distributed ordering, spill, checkpoint and
  termination arguments.  The lock's wait time is measured (`Counters::lock_wait`) so that a many-core run shows
  whether that ever changes.
* **Checkpoint pauses at any interval.**  Before, an interval of 0 paused again at once without letting go of the
  lock, and the workers never ran again.  Now a periodic pause begins only once the workers released from the last
  one all run again, and no sooner than the last pause lasted; and a worker answers it (as a split request) only
  after 64 polls of its current unit, so every pause is followed by work.  A pause costs every worker the wait for
  the last one to park, bounded by the longest stretch between two polls, one Taxman step: about 10 µs, over a
  millisecond about once in 10⁵ steps, the longest measured 10–15 ms at n = 1215–1225, plus the write and fsync.
* **Checkpoint headers (2026-10-12).**  Headers before version 3 (magic `TXS2`) recorded neither n nor the frames: a
  checkpoint of 140 passed for one of 141 (same candidates, same frame 0, other frames).  They are refused with that
  reason rather than accepted unchecked.
* **Set widths.**  The widths of 20, 24 and 28 words between 16 and 32 pay near the frontier: `Bits<20>` instead of
  `Bits<32>` takes 6% fewer cycles at 1184 and 1215 (2026-10).  Each width is a copy of the search's code: the
  release build took 25 s with four widths, 50 s with the seven now built (62 s with 12 and 48 too, which were left
  out: 512–767 is solved, and 2048–3071 far off).  A checkpoint records its width, and a search resumes at that
  width if this build has it (an older build chose 32 for 1024–2047).

## Benchmarks, the probe, PGO and scaling

What the measuring tools were first used to find ([DESIGN.md, "Performance"](../DESIGN.md#10-performance) says how to
use them).

* **The bench set.**  Until 2026-10 `taxman bench` ran n = 480–760 (W = 8 and 16, milliseconds per game), which no
  longer exercised the frontier's hot paths; the Java took 2.0–65.6 s for those on one thread and 1.4–12.4 s on ten.
  The set is now 936, 1050, 1100, 1133, 1170 and 1184, about a second each on one thread.  A window of a long proof is
  not a game the solver can run, so it is in the probe.
* **The Java's best run, for comparison** (the owner's notes of 2022-09-09): the chain 1..1000 on an EC2 r6a.metal
  (192 vCPUs, 1,536 GB, $3.60 an hour), with `reuse_prev` and `revise_freedoms` on.  Its ten slowest games took
  29,766 s together, over 8 hours: 990 15,858 s, 987 3,896, 996 3,624, 992 3,519, 988 1,559, 999 516, 1000 314,
  833 188, 969 175, 984 119.  The committed `results/strict.jsonl` (commit 243af9b, 2026-10-13, nothing unproven)
  took 58.7 s of game time for the whole of 1..1000 on 4 threads of the laptop; its times for those ten games are
  4.9, 0.84, 11.2, 12.9, 2.7, 3.6, 1.3, 0.09, 0.63 and 0.21 s (140 to 4,600 times less wall time on 4 threads
  against 192).  The same cluster, 987-999, holds the hardest games of both.  On this laptop's single thread, against
  the Java's on the same machine (2026-10-12): 700 18.0 s against 0.030 s, 720 23.1 s against 0.026 s, 760 65.6 s
  against 0.047 s, each from `optimal.json`'s game of n − 1.
* **The probe** (`examples/perf.rs`), about 5 s in all: F1, 300 search nodes of each of the proofs of 936 and 1215
  with every kernel over all of them; F2, the frame checks of those promotions; F3, a checkpoint of each proof
  (1215 after 1.5M nodes) resumed on one thread for 50,000 steps, one period of forgetting nogoods.  Cycles and
  instructions retired of the process are what it reports, since other load on the machine barely moves them,
  unlike wall time (up to 3x).
* **Profile-guided optimization** (`tools/pgo.sh`).  On the code before "Faster nodes, same trees" it took 12% off
  the cycles of the proof of 1215 (151.1 → 132.5 G), 13% at 1184, 6% at 936 and 9–11% on games it was not trained on
  (880, 1050, 1100, 1170), mostly, it turned out, by inlining the Hungarian search's heap; with that inlined by
  hand it adds 0–2% (1215: 106.0 → 105.1–106.4 G, 1184: 3.16 → 3.10 G).  Profile the plain build: with PGO the
  search is inlined into a few functions and `sample` sees little.
* **Thread scaling.**  On the M1 Max (8 performance cores, with another job on up to 4 of them) with the proof of 1215
  (`ladder --seed-opt --known results/default.jsonl --tactic revise_freedoms=off`): 62.5 s on one thread, 24.6 s on
  2, 15.8 s on 4 and 12.9 s on 6, effective parallelism 99.9–100%, all workers busy within 3 ms; node counts vary
  by ±10% with the threads (what each thread's nogood table has learned).  Games that take minutes on one thread
  (1215, 1220, 1225 with `--seed-opt`: about 60, 150 and 200 s) suit a many-core machine; at 192 threads a game of a
  minute is a third of a second, mostly ramp-up and tail.  [bench/curve/README.md](../bench/curve/README.md) has
  the later measurement of the proof of 1225 on 1, 2, 4 and 8 threads and the cost curve beyond the frontier.
* **The ladder before and after the conflicts round** (10 threads, standalone from the frame builder, seeded from
  n − 1): before, 992 and 1000 were open at 100 s (992's incumbent not even optimal), 960 took 37 s and 800 13 s,
  and 945, 948, 950, 969, 976, 984, 987, 988, 990, 996, 999 were open at 20–30 s.  After, all complete: 760 0.03 s,
  800 0.5 s, 840 0.1 s, 880 0.3 s, 900 0.1 s, 960 1.6 s, 945–999 1.4–24 s except 992 (62 s, of which 42 s finding the
  optimum) and 996 (69 s, all proof).
* **Whole ranges through `taxman solve`** (after the conflicts round).  761–1000 (prime shortcut and `reuse_prev` on;
  81 games searched) about 5 minutes of game time on 10 threads, every score matching optimal.json; 1001–1100 about
  6.5 minutes (36 searched, slowest 1025 at 100 s) and 1101–1200 about 2.5 minutes (with `reuse_prev` and
  `revise_freedoms`, as the Java's results).  Threads scale linearly to the 8 performance cores; the 2 efficiency
  cores add little.
* **The strict chain after the forced-members round.**  `taxman solve 1001-1217 --threads 8 --tactic reuse_prev=off
  --tactic revise_freedoms=off` reproduces every score of `results/strict.jsonl` in 164 s of game time; those
  results took 1244 s on 10 threads.  Its slowest games: 1025 21 s (110 s before), 1215 10 s (161 s), 1020 6 s
  (40 s).  After "Faster nodes, same trees" the strict chain 1..1231 on 8 threads takes 279 s of game time
  (1201–1231: 145 s), every score as committed.
* **The hard games, one and many threads, before and after the second round** (see the table in "Forced members and
  nogoods").  Where the time went at 1184 before it (`sample`): peeling for conflicts 29%, matching repairs for
  refinement 18% and for strong branching 16%, the candidate filter's peels 16%.  At the 1215 proof after
  "Faster nodes, same trees": the Hungarian repairs 46%, the peels 24%, the bounds from the potentials 7%, the node's
  own work 7%, level copies 4%, nogood lookups 3%.
* **The test suite.**  About 28 s with two test threads in the test profile (`opt-level = 2`), 24 s in release,
  after the tests of the wide-board, fixture, tripwire, soundness and CLI rounds and the trimming (before: 30 s and
  26 s).  A few tests sample what an exhaustive run would cover (every 18th golden board for the peeling agreement,
  every sixth board above 511 for the root promotability of the frame builder's candidates, every second line of the
  Java fixtures); the comment on each says what the whole run costs.
* **Left to try** (from the end of the second round): the incumbents of games like 992 and 1025, whose optimum the
  search reaches late (992 on 8 threads: at 11.8 s of 12.0 s; seeded with its optimum it takes about 6 s); the
  matching repairs (still the largest cost); and `i32` potentials (no bound on the potentials is proven, so not exact
  by construction), a warm-started unit rebuild (another optimal matching can change the tree after a split or a
  resume), candidate-index sets for every search-level set (a large refactor), and watched members for the nogoods
  (now 3% of the time).

## Same search (2026-10)

*Question.*  Rank 1 of [IDEAS.md](IDEAS.md#1-the-recommended-order): many games pose exactly the search problem of an
earlier game of the chain (the same frames, candidates, `max_total` and factor graph; only the cap differs), so they
have its best promotion sum, yet the solver proves it again.  How many, what does it cost, and does concluding them
from the earlier game hold up?  The pilot ([IDEAS.md 4.1](IDEAS.md#41-memoizing-searches-that-repeat-rank-1)) hashed the root frames, the
candidates and `max_total` with Rust's `DefaultHasher`; this round makes it a proof kind (commits f5a2af3 and
1cc4067; DESIGN.md, ["Same search"](../DESIGN.md#same-search), has the argument).

*Method.*  The problem is written out canonically (`search::Problem`, which also records the model's factor lists and
home frames for every number of the frames) and compared whole; the digest only finds candidates.  The repeats are
found from the setups alone, with no search: `cargo run --release --example repeats -- 1 1300 --known
results/strict.jsonl` sets up every game of 1..1300 that is neither prime nor trivial as a strict run does, finds
the latest earlier game with an equal problem, and adds up the `seconds.search_all_runs` that the committed strict
chain spent on each repeat (8.4 s for the 1076 setups, about 8 ms a game; 25–30 ms near 1300).

*How many, and how far back.*  545 of the 1076 games of 1..1300 that set a search up repeat an earlier game: 416 in
1..1000, 100 in 1001..1231 (12 of them in 1201..1231) and 29 in 1232..1300.  Each repeats the last board size before
it that set a search up: n − 1 for 471, n − 2 for 74, and for every one of those 74, n − 1 is prime.  So a window of 8
board sizes (`SAME_SEARCH_WINDOW`) misses none, and the setups a range must make for lines that record no digest
(schema 3) stay at eight.

*What they cost in the committed `strict.jsonl`* (6 threads, commit bd9bb5d), so what a rerun of its chain would not
spend:

| range | repeats | their search | the range's search | share |
|---|---|---|---|---|
| 1..1000 | 416 | 29.0 s | 82.8 s | 35.0% |
| 1001..1231 | 100 | 72.9 s | 409.7 s | 17.8% |
| (1201..1231) | 12 | 45.8 s | 276 s | 16.6% |
| 1232..1300 | 29 | 851.2 s | 2866.3 s | 29.7% |
| 1..1300 | 545 | 953.1 s | 3358.8 s | 28.4% |

The repeats of 1232..1300 (game ← the game it repeats, its committed search seconds): 1234←1233 0.1, 1238←1236 0.1,
1241←1240 6.7, 1243←1242 3.6, 1244←1243 0.1, 1246←1245 0.1, 1247←1246 3.6, 1252←1251 0.1, 1253←1252 3.8, 1255←1254
24.2, 1257←1256 23.0, 1258←1257 0.1, 1261←1260 48.2, 1262←1261 0.1, 1263←1262 51.3, 1266←1265 0.1, 1267←1266 63.6,
1268←1267 0.1, 1270←1269 0.1, 1271←1270 75.3, 1273←1272 38.5, 1276←1275 0.1, 1282←1281 0.1, 1285←1284 36.1,
1286←1285 0.1, 1293←1292 207.1, 1294←1293 0.1, 1298←1296 0.1, 1299←1298 265.1.  The pilot named the first game of
each class (1299 repeats 1296, 1263 and 1261 repeat 1260); the solver takes the latest, which is the same problem.
The searches of 0.1 s are the ones that reached the cap at once: a repeat saves little there, and much where the
earlier game's proof is a long exhausted search.

*At the frontier, without big runs.*  The committed `strict.jsonl` cut after 1298 (its lines for n ≤ 1298, incomplete
ones included) and extended with `taxman solve 1299 --strict --threads 2 --out FILE`: 1299 is a repeat of 1298, 0.079
s for the game and 0.46 s for the whole process (opening and checking the 1301 lines, and setting the window's eight
schema-3 games up again), score 531389 as committed, against 265 s of search in the committed run.  The same from the
file cut after 1292: 1293 repeats 1292 (0.081 s, 526200; 207 s committed) and 1294 repeats 1293, itself a repeat
(0.082 s); cut after 1270: 1271 repeats 1270 (0.064 s, 509880; 75 s committed).  Every file passes `taxman
verify-results` and `tools/check_results.py`.

*Chains with the tactic on and off.*  The strict chain 1..300 concludes 107 games as repeats with the tactic on
(proofs: trivial 13, prime 62, cap 34, exhausted 84, same_search 107), against cap 101 and exhausted 124 with it off,
every score the same; its repeats are exactly the 107 that the setups alone find.  With `revise_freedoms` on as well
(only `reuse_prev` off) there are 107 repeats too, 69 of the 300 games resting on `revise_freedoms`.  With the default
tactics `taxman regress --to 400` has no repeat at all (trivial 13, prime 78, reuse 200, repeat 0, search 109):
`reuse_prev`, which comes first, accepts every game whose problem is its predecessor's; `--strict` gives repeat 151,
search 158.  The probe's fingerprint is unchanged (6197a47eae6cf8c8; the shortcut is outside the search), and
`cargo test --release` passes, with `tests/same_search.rs` (2.7 s) and the forged repeats of `tests/results.rs`.

*Decision.*  Kept: tactic `same_search`, on by default and in strict runs; proof `same_search`, results schema 4.  A
rerun of the strict chain to 1300 would skip 28% of its search time, and the frontier games that repeat cost
nothing; it changes no score, tree or node count of a game that is searched.  Verifying a repeat sets two problems up
again (the verifier caches them by board size), so a schema-4 chain to 1300 will take a few seconds more to open.

## A Lagrangian bound over the nogoods (2026-10)

The question (docs/IDEAS.md, rank 2): the pilot ([IDEAS.md 4.2](IDEAS.md#42-a-lagrangian-bound-over-the-nogoods-rank-2)) folded the nogoods
into the matching bound by Lagrangian multipliers fitted once at the root, and cut the nodes of proofs by 13 to 67%,
but it solved the weighted matching from scratch, 7 times slower per node.  Can the bound be kept incrementally,
cheaply enough to pay in cycles?  Yes, and with the forced members its potentials give it pays far more than the
pilot's node counts suggested: proofs from 1184 to 1300 take 2 to 6 times fewer cycles, about 3 times, with no trend
in n.  Kept as the tactic `lagrange`, on by default (commit e6b37d2; DESIGN.md, section 4, "The Lagrangian bound over
the nogoods", has the design, and `src/matchbound/lagrange.rs` the argument).

**How it was measured.**  One-thread proof-only searches (`ladder --threads 1 --seed-opt --tactic
revise_freedoms=off`, the ladder's `--node-limit` for prefixes), whose node counts are exact, and the process's
cycles (`/usr/bin/time -l`), against master (c6cf8ab) built the same way.  With `--tactic lagrange=off` the new
build gives master's trees node for node (the tripwire rows `lagrange=off`) and the probe's fingerprint
6197a47eae6cf8c8 in the same cycles (4.21 G against 4.20 G, three runs each, instructions +0.06%).  The laptop was
shared with other agents the whole time (load average 3 to 12), so cycles and counts only; four processes at most.

**The solver.**  `matchbound`'s levels and Hungarian repairs became generic over how a level weighs its candidates
(the matching bound's code and trees unchanged), and `matchbound::lagrange` keeps a second stack of levels with
weights of their own: a level is made for a node when the bound is asked for there, by copying the nearest level of
its path and applying what changed (promotions; candidates gone, whose live nogoods die and raise the weights of their
other members).  A weight that rises needs a repair only where a potential must rise past a tight edge (0.4 per
evaluation at 1215).  When the multipliers change, the levels are dropped and solved afresh, so that no level mixes
two sets of multipliers.  Weights are in 64ths of a number and the multipliers integers, so the bound is exact
integer arithmetic and nothing rounds.  Checked against the reference min-cost flow (with the live nogoods, weights
and constant worked out afresh) on random walks with random relaxations that change mid-walk, against a solver
started afresh, by the optimality certificate of every level, and against exhaustive completions with real nogoods.

**Step 1: the pilot's setting, incrementally.**  The nogoods of a warm-up of the same game (the pilot's: the whole
proof for 936 and 1100, its first 20k, 300k, 500k and 300k nodes for 1184, 1215, 1225, 1250; the warm-up not
counted), 200 subgradient steps at the root, the bound tried at every check of a node or child that the matching
bound and refinement left open:

| proof | nodes, master | nodes | G cycles, master | G cycles | cycles |
|---|---|---|---|---|---|
| 936 | 59,055 | 51,207 | 1.21 | 1.47 | +22% |
| 1100 | 55,543 | 41,006 | 1.31 | 1.49 | +14% |
| 1184 | 167,960 | 52,065 | 3.24 | 1.67 | −49% |
| 1215 | 5,485,294 | 3,851,527 | 108.5 | 106.1 | −2% |
| 1225 | 12,071,677 | 7,124,951 | 322.4 | 272.9 | −15% |
| 1250 | 2,792,227 | 2,021,225 | 54.6 | 53.6 | −2% |

The node counts are the pilot's (1184: 52,065 against its 54,795), at 30-55% more cycles per step.  What a bound
costs: run without cutting (computed and thrown away, on master's tree; 1215, first million nodes), 604k bounds took
18.7 → 24.8 G cycles, 10k cycles each, +30% per step; tried only at the checks of nodes, not of children (where 78%
of its cuts were), 207k bounds took 5.8 G against 6.4 G in the same run (each derived from further up the path, and
the work for a parent's level repeated for each child); keeping the multipliers of dead nogoods, which needs no weight to rise,
saved a tenth of it (24.2 G).  A micro-benchmark of the root's expansion loop (`bench_lagrange_1215`, ignored test)
puts a Lagrangian level at the price of a matching level (3.6-3.7 µs a step against 3.3).  So the second matching
costs about what the node's own matching checks cost, twice over (no strong-branching level to reuse, and its own
optimum, which the search does not branch along): this setting pays only where nodes fall by more than a third.

**Step 2: multipliers fitted in the search.**  Each searcher fits its own at the root, to the nogoods it learned (its
table's and those of its multipliers so far), first after 10,000 steps, then before each forgetting, 50 subgradient
steps: 936 +15%, 1100 −1%, 1184 −41%, 1215 −13%, 1225 −25%, 1250 −13% (nodes −11% to −59%), with no warm-up to
pay.  With the first refit after 2,000 steps, on ten proofs from 1196 to 1285, −4% to −32% (1220 +14%), −10% in
all.  The fitting is 3% of the time (13.6 ms a refit at 1215), the levels solved afresh after it about 1%.  Variants,
with this setting (cycles, 1215 and 1250, against 84.1 and 45.2 G): the bound tried only at nodes of at most 8, 16,
24 or 32 promotions 108.2, 92.9, 81.7, 85.1 G and 54.1, 56.2, 57.3, 53.1 G (the deep cuts matter); the multipliers
fitted at the node in the middle of the path at refit time 96.5 and 45.9 G, at the deepest 135.6 and 55.8 G (the
root's are best).  With the first refit after 10,000 steps, 20 or 100 subgradient steps gave 119.8 and 112.2 G, and
45.7 and 46.5 G (against 94.5 and 47.5 G with 50), and refits every 10,000 steps 96.1 and 53.6 G: as noisy as the
trees, since a different fit makes a different tree.

**Step 3: forced members.**  Where the bound leaves a node open, the members of its heaviest matching without which
it would cut, read off its potentials as `bound_excluding` reads the matching's, are forced: every completion that
beats the incumbent promotes them (the live nogoods' inequalities hold for every completion, so the multipliers and
the constant stay without a candidate).  With `fix_forced` the child promotes them with the matching's forced
members, and strong branching is skipped.  This is where the gain is (one thread, proof-only; master against the
tactic as committed, schedule of step 2):

| proof | nodes, master | nodes | G cycles, master | G cycles | cycles |
|---|---|---|---|---|---|
| 880 | 31,416 | 25,398 | 0.62 | 0.65 | +6% |
| 936 | 59,055 | 41,653 | 1.21 | 1.20 | −1% |
| 1050 | 115,154 | 46,527 | 3.45 | 2.11 | −39% |
| 1100 | 55,543 | 29,614 | 1.31 | 1.06 | −19% |
| 1184 | 167,960 | 52,259 | 3.24 | 1.60 | −51% |
| 1196 | 1,942,366 | 216,076 | 36.3 | 5.9 | −84% |
| 1204 | 3,570,480 | 498,121 | 69.2 | 14.3 | −79% |
| 1215 | 5,485,294 | 1,111,451 | 108.5 | 34.2 | −68% |
| 1220 | 8,375,382 | 2,503,016 | 206.7 | 89.0 | −57% |
| 1225 | 12,071,677 | 2,178,362 | 322.4 | 85.8 | −73% |
| 1240 | 4,374,708 | 496,460 | 104.5 | 18.5 | −82% |
| 1250 | 2,792,227 | 709,893 | 54.6 | 20.7 | −62% |
| 1254 | 20,245,006 | 4,830,651 | 397.1 | 147.1 | −63% |
| 1257 | 12,769,713 | 3,686,322 | 244.8 | 108.1 | −56% |
| 1272 | 25,067,509 | 4,341,863 | 473.7 | 127.9 | −73% |
| 1280 | 15,352,847 | 3,080,373 | 354.6 | 109.3 | −69% |
| 1285 | 16,902,075 | 3,442,845 | 379.4 | 117.0 | −69% |
| 1288 | 23,625,803 | 4,822,476 | 536.8 | 166.5 | −69% |
| 1290 | more than 75.1M (stopped at 600 s) | 19,473,016 | more than 1872 | 734.4 | below −61% |
| 1300 | more than 79.6M (stopped at 600 s) | 28,024,916 | more than 1872 | 957.2 | below −49% |

From 1184 to 1288 the ratio of cycles is 2.0 to 6.1, 3.3 as a geometric mean, and it does not grow with n: a least
squares line through its logarithm falls by 12% per 100 in n, well inside the scatter from game to game (1220 2.3,
1240 5.6).  At ×10 per 55-70 in n (bench/curve/README.md) a constant factor of 3.3 moves the cost curve down by
half a decade, about 30 to 35 in n, and leaves its slope as it was.  Each step costs about half as much again
(the window of the probe, 50,000 steps resumed from the 1215 proof: 25.3k cycles a step against 16.9k), so the
games where the search spends its time finding the optimum late, not proving it, lose.

Beyond proof-only searches:

* **The strict chain 1201-1231** (`taxman solve 1201-1231 --strict --threads 4`, in memory, from optimal.json's 1200
  with `--allow-tainted-prev`): 283.5 s of game time and 3,448 G cycles with the tactic off, 117.2 s and 1,411 G with
  it on; every searched game faster (in wall time on four threads: 2.0× for 1212 to 3.6× for 1203 and 1204), every
  score the same.
* **Four threads**, the proof of 1225: 9.93M nodes, 289 G cycles, 23.8 s off; 2.92M nodes, 124 G cycles, 10.3 s on.
* **Seeded from n − 1** (one thread, what a chain does): 1215 −71%, 1220 −60%, 1240 −83%, 1250 −70%; but 992, whose
  optimum is found late, +35% (4.20M → 4.02M nodes, 114 → 155 G cycles), and 1025 +4%.  The bench set (936, 1050,
  1100, 1133, 1170, 1184, seeded): −2%, −24%, +11%, +16%, −36%, −52%, −24% in all; 1133's search is 2,386 steps,
  and its one refit (after 2,000) is most of the difference.  A search of fewer than 2,000 steps is unchanged.
* **Correctness at scale**: `taxman solve 761-1000 --strict --force-search` on four threads matches optimal.json on
  all 240 games, and `taxman regress --to 1000` on all 1000.

Measured, not kept (with forced members, in the build of the experiments, cycles against 32.3 G at 1215 and 151.3 G
at 1254 with the schedule kept): 20 subgradient steps 35.1 and 170.0 G; 100 steps 35.7 and 145.2 G; the first refit
after 500 steps 30.7 and 150.0 G; refits every 10,000 steps 31.3 and 157.9 G (four to five times as many refits).
All within ±12%, so the schedule stays the simple one.

**Verdict.**  Kept, on by default: a constant factor of about 3 on the proofs that dominate the frontier, with no
change of slope; even on short searches, a loss where finding the optimum dominates.

**What would change it, and what is left.**

* *Refinement may no longer pay.*  With the tactic on, `match_refine=0` took 1215 29.3 G (against 32.3 G in the same
  build), 1254 112.0 G (151.3), 1240 18.4 G (18.4), 1250 17.6 G (20.5): 0 to 26% fewer.  (Without refinement a child's
  check stops at the matching bound, the Lagrangian bound included, and its first step checks it fully.)  Refinement
  is 12-20% of the time; it needs its own measurement over the bench set and small games before its default moves.
  (Measured in ["Faster nodes with the Lagrangian bound"](#faster-nodes-with-the-lagrangian-bound-same-trees-2026-10):
  a few percent on average, ±20% per game, even on the strict chain 1201-1231; the default stays 1.)
* *Refits without solving afresh*: re-weighting the levels in place (weights that fall unmatch their candidates if
  promoted, as weights that rise do the rest) and warm-starting the fit, about 4% now.
* *Many threads*: each searcher fits its own multipliers to its own table, which at 192 threads is a smaller sample of
  a smaller region; measured on four threads only.
* *The second matching's cost* (half a step): what a bound costs is set by the changes along the path, not by how
  often it is asked (step 1), so asking less often saves nothing; a cheaper bound would have to come from fewer
  levels or cheaper repairs.
* *The late-optimum games* (992, 1025) are where it does not pay; a search that knows it has not yet found the
  optimum (incumbent improvements still coming) could hold the bound back.

**The whole strict chain with both of this month's shortcuts** (`same_search` and `lagrange`, build a2da702):
`taxman solve 1-1300 --strict --threads 4` from n = 1, on the laptop while two other jobs ran, took 1038 s of wall
time in all (3975 s of CPU).  Every one of the 1300 scores equals the committed `results/strict.jsonl`'s, no game is
tainted, and both referees pass it; its proofs are same_search 545, exhausted 416, prime formula 211, cap 115,
trivial 13.  Search seconds by range, against the committed run (bd9bb5d, 6 threads): 1..1000 51.8 against 82.8,
1001..1231 184.6 against 409.7, 1232..1300 775.7 against 2866.3, so 3.7 times less wall time on two thirds of the
threads at the frontier.  The slowest games are now 1232 (142 s, was 439), 1295 (107, was 259), 1300 (88, was 179) and
1287 (85, was 273).

**The committed chain replaced** (build 243af9b, which has both shortcuts): `results/strict.jsonl` is now a run of
`taxman solve 1-1300 --strict --threads 4` from n = 1 with that build, in one invocation, on the laptop while other jobs
ran: 1078 s of wall time and 4014 s of CPU, against 1038 s and 3975 s of the run above; search seconds 51.3, 187.8 and
813.2 for 1..1000, 1001..1231 and 1232..1300, against 51.8, 184.6 and 775.7; the same 1300 scores and the same proofs
(same_search 545, exhausted 416, prime formula 211, cap 115, trivial 13).  It replaces the file of commit bd9bb5d (6
threads, schema 3; `git show cd3d32f:rust/results/strict.jsonl`), which "the committed `strict.jsonl`" and "the
committed run" in the entries above mean (the figures of the same-search round, which come from `repeats --known`, need
that file: on the new one the repeats cost 0.0 s, since it did not search them).  Its scores are that file's, and its
moves too but for 1221..1224, which have at least two optimal sets, and the two runs found different ones
([results](../results/README.md#strictjsonl)).

## Which tactics earn their keep (2026-10)

*Question.*  The owner's: a tactic that is unproven and does not pay has no reason to stay.  Which tactics and
alternatives give meaningful acceleration in the case that matters, long strict chains at the frontier (n of 1200 to
1400 now, more on a 192-core machine, each game built from the one before, as `results/strict.jsonl` is), and which
should be recorded here and removed?  And what support for old file formats can go with them?  Strict is to be the
default, and a doubtful tactic that stays must be switched on explicitly.  This entry measures and recommends; the
removals are a later step.

*How it was measured.*  The solver code of commit 243af9b, which made `results/strict.jsonl` (unchanged to 72601a2).
Searches are the ladder's proof-only ones on one thread (`ladder --threads 1 --seed-opt --known results/strict.jsonl
--seconds S [--tactic …] N`), whose node counts are exact, with the process's cycles from `/usr/bin/time -l`, two at a
time on the laptop while another agent measured cycles on it, so cycles are good to a few percent.  What `reuse_prev`
and `revise_freedoms` do was worked out from setups alone, with no search, by a scratch example (not kept):

* `reuse_prev`: for every n of 2..1300 that is neither prime nor trivial, `accelerate::reuse_prev` with the strict
  chain's game of n − 1, its game compared with n's line in `strict.jsonl` (score, moves, proof, and
  `seconds.search_all_runs`); and a chain simulated with the tactic on, in the solver's order: prime, trivial,
  `reuse_prev`, else a repeat of the latest of the 8 board sizes before n that set a search up and poses the same
  problem (compared whole, as `examples/repeats.rs` does), else a search, with the derived games as n − 1 where it
  accepted.
* `revise_freedoms`: `Board::prepare` with the tactic on and off, one thread, for each searched or repeated game of
  1232..1300: what revising did, how long it took, and whether the two problems differ.

### `reuse_prev`: nothing left to save

With the strict chain's games as n − 1 it accepts 670 games of 2..1300, and the strict chain settles every one of them
without a search of any length:

| what the strict chain did with them | games | at the ceiling n + score(n − 1) | their search seconds |
|---|---|---|---|
| a repeat (`same_search`) | 545 | 300 | 0 |
| the cap, reached before the first node | 115 | 115 | 1.89 |
| an exhausted search | 10 | 0 | 1.92 |

The ten searches are of 51, 111, 119, 141, 183, 185 and 741 (each under 0.03 s), 1083 (0.11 s), 1183 (0.14 s) and
1209 (1.64 s, the only one over a second); the other 406 searches of the chain it rejects.  So a rerun of the chain
with the tactic on would save at most 3.8 s of its 1052 s of search (0.36%): 0.6 s of 51 s in 1..1000, 2.7 s of 188 s
in 1001..1231, and 0.5 s of 813 s in 1232..1300, all of it there the setups of seven proofs by the cap.  The simulated
chain agrees: 670 derived, 406 searched and no repeat at all, since the tactic comes first and takes every game whose
problem is its predecessor's (as `regress` showed in the same-search round), and no game that the strict chain repeats
would have to be searched instead.  No derived game differs from the strict chain's, in its score or in a single move,
so there is no counterexample either.  255 of the 670 are accepted below the ceiling, where only the conjecture says
that they are optimal; 245 of those are repeats, proven.  What the tactic once saved, the cap with the seed (every
game at the ceiling) and `same_search` (the rest) now prove.

### `revise_freedoms`: the same trees

Its premise is false for the whole frame game ([n = 833](../evidence/README.md#6-does-an-optimal-game-play-the-most-moves),
`tests/soundness.rs`), and the owner decided to remove it; measuring stopped there.  Measured before that: in every
one of the 60 searched or repeated games of 1232..1300 it searches the two-frame game (282 to 2,238 nodes, 16 to 27 ms,
1.2 s for the 60) and lowers frame 0's freedoms by 4 to 6 and `max_total` by 0 to 5, so every problem changes; yet
seven frontier proofs have the same trees with it on and off, node for node:

| proof | nodes, on and off | G cycles, on | G cycles, off |
|---|---|---|---|
| 1215 | 1,111,451 | 32.6 | 32.7 |
| 1225 | 2,178,362 | 86.8 | 86.7 |
| 1240 | 496,460 | 18.7 | 18.7 |
| 1254 | 4,830,651 | 149.9 | 149.5 |
| 1272 | 4,341,863 | 129.2 | 128.8 |
| 1287 | 20,972,332 | 803.8 | 803.5 |
| 1300 | 28,024,916 | 985.0 | 982.2 |

The lower count of promotions and frame 0's lower freedoms cut nothing that the matching bound and the conflicts do not
cut first.  In the chain the tactic adds about 20 ms of setup a game and a taint, nothing else.

### The alternatives that are off

The measurements that retired `preclusions`, `match_bound=off` and the other branchings predate the Lagrangian bound,
so they were repeated with it (one thread, proof only, the strict tactics with the one named changed; a limit of
180 s):

| proof | tactics | nodes | search s | G cycles | against the strict tactics |
|---|---|---|---|---|---|
| 1184 | strict | 52,259 | 0.47 | 1.6 | |
| 1184 | `preclusions=on` | 54,883 | 0.52, and 3.45 building the table | 12.5 | nodes +5%, cycles ×7.7 |
| 1215 | strict | 1,111,451 | 10.6 | 32.6 | |
| 1215 | `preclusions=on` | 1,569,468 | 14.6, and 4.46 building the table | 58.7 | nodes +41%, cycles +80% |
| 1215 | `branching=conflict-largest` | 9,098,354 | 66.6 | 204.8 | nodes ×8.2, cycles ×6.3 |
| 1215 | `branching=conflict-smallest` | more than 21.5 M | stopped at 180 | 554.9 | more than ×17 |
| 1215 | `branching=largest` | more than 32.0 M | stopped at 180 | 562.0 | more than ×17 |
| 1215 | `match_bound=off` | more than 67.7 M | stopped at 180 | 558.8 | more than ×17 |
| 1215 | `match_refine=0` | 1,123,153 | 9.3 | 28.7 | nodes +1%, cycles −12% |

The branchings other than `conflict-strong` lose more than before because the forced members (the matching's and the
Lagrangian bound's, tactic `fix_forced`) work only with it; without `match_bound` nothing of the last two rounds works
at all.  `legacy_bound` needs no measurement: it changes the runner-up of `blameGame`, which the default search never
computes, so it acts only with `match_bound=off`, and there it is unsound.  The last row is the open question of the
Lagrangian round, whether refinement still pays (there 0 to 26% fewer cycles without it on four proofs); its default
needs the bench set and the small games before it moves.

### The proven tactics' switches

None of the proven tactics has code of its own for the off position worth the name: each is a condition or an
`Option` that the on position needs anyway.  A switch costs a slot in `Tactics`, a bit of the checkpoint header, and
its share of the tests (each variant of `tests/solver.rs`'s `every_tactic_gives_the_same_scores` solves 1..300 with
`force_search`; `tests/common`'s variants and `tests/soundness.rs`'s configurations; tripwire rows).

| switch | code only for the off position | tests that use the off position |
|---|---|---|
| `same_search` | one condition (`solver::search_game`) | `tests/same_search.rs` (chains with it off are the reference), `derivations_can_be_switched_off` |
| `lagrange` | none (an `Option`, `None` without `nogoods` too) | 5 tripwire rows (the trees of the build before it), the probe's `--tactic lagrange=off` fingerprint, a variant |
| `fix_forced` | the `forcing` flag: about five conditions in `searcher.rs` and `refine.rs` | 6 tripwire rows, variants and soundness configurations |
| `nogoods` | none (the table is an `Option`) | 6 tripwire rows, variants and configurations |
| `forget_nogoods` | none (the period is an `Option`) | configurations (never, every step, every third), a variant |
| `prev_cap`, `seed_incumbent` | one filter each (`solver::search_game`); the ladder's `--no-cap` and `--no-seed` | a variant each; the standalone re-solves of `bench/curve/calib.jsonl` used both flags |
| `prime_shortcut` | one condition (`solver`) | `derivations_can_be_switched_off` |
| `match_refine` (0 to 7) | the early return of a child's check at level 0 (three lines); `Refiner::top` takes any depth; `MAX_MATCH_REFINE` and 3 header bits | 6 tripwire rows (0 and 3), variants and configurations |

### Recommendation

| tactic or alternative | grade | measured benefit in the frontier chain | recommendation | what removal deletes (details below) |
|---|---|---|---|---|
| `reuse_prev` (on by default) | unproven | 3.8 s of the strict chain's 1052 s of search (0.36%), 0.5 s of 813 s in 1232..1300; every game it accepts is settled anyway by the cap or a repeat | remove | `accelerate::reuse_prev`, method `reuse` and proof `reuse_prev`, the Java acceleration fixtures |
| `revise_freedoms` (on by default) | unproven, premise refuted at n = 833 | none: the same trees in seven frontier proofs, 20 ms of setup a game | remove (premise refuted at n = 833) | `Revise` and the two-frame search, `setup.revise` and `seconds.revise`, frame 0 in the checkpoint header |
| `legacy_bound` (off) | unsound | none: it acts only with `match_bound=off` | remove | `RunnerUp::Legacy` and its taint |
| `preclusions` (off) | sound | a loss: 1215 +80% cycles, 4.5 s to build the table | remove | `bounds::Preclusions`, `seconds.preclusions`, `stats.precluded` |
| `match_bound=off` (the Java's search) | sound | a loss: 1215 unfinished at 180 s, more than 17 times the default | remove | `blameGame` and the promotability checks, frame certificates, `stats.upper_bounds`, `can_promotes`, `certificates` |
| `branching=largest` | sound | a loss: more than ×17 | remove | the tactic `branching` (with the next two) |
| `branching=conflict-smallest` | sound | a loss: more than ×17 | remove | (with `branching`) |
| `branching=conflict-largest` | sound | a loss: ×6.3 | remove | (with `branching`) |
| `same_search`, `lagrange`, `fix_forced`, `nogoods`, `forget_nogoods`, `prev_cap`, `seed_incumbent`, `prime_shortcut`, `match_refine` | proven or sound | on by default, the frontier's speed (earlier rounds); their off positions are the baselines | keep, as switches for A/B measurements | nothing |

With `reuse_prev`, `revise_freedoms` and `legacy_bound` gone no unproven tactic is left, so every run is strict and
there is nothing doubtful to switch on: `--strict`, `tactics::UNPROVEN`, `Tactics::strict`, `Tactics::unproven_on`,
the line's `unproven_tactics_on` and the rules of a game's own taint (`results::own_taint`, `own_taint` in
`tools/check_results.py`) go, with their tests (`strict_turns_off_exactly_the_unproven_tactics`,
`a_game_made_with_unproven_tactics_on_but_not_used_is_clean`, the `--strict` cases of `tests/cli.rs`,
`design_grades_the_unproven_tactics_as_the_code_lists_them`), and a taint can then only be inherited from
`optimal.json` (whose games 1..1000 the Java made with those tactics on, so `--allow-tainted-prev` stays).  Of `SearchTactics` there remain `match_refine`,
`fix_forced`, `nogoods`, `forget_nogoods` and `lagrange`.

### What each removal deletes

* **`reuse_prev`.**  `accelerate::reuse_prev` (and the module comment's half on it); step 4 of `solver::solve_one`;
  `Method::Reuse` and `Proof::ReusePrev` with their rules in `src/results.rs` and `tools/check_results.py`;
  `FrameColumns::estimate_max_promotions` (its only user, with its check against the Java's `estimateMaxPromotions` in
  `tests/golden_hives.rs`) and the frames test `the_frames_candidates_are_the_plain_apiarys` (it guards the
  tactic's candidates); `examples/sacrifice.rs`'s `--select reuse` (the theorem of `src/sacrifice.rs` stays; its module
  comment's motivation changes); tests: `acceleration_matches_the_java_beyond_the_golden_data` with
  `testdata/accel_java.txt`, the `reuse_prev` half of `acceleration_matches_java` (its prime half can stay), the Java's
  count of 146 derived games in `a_range_reproduces_optimal_json_and_the_javas_counts`, and the tests in
  `tests/integrity.rs` and `tests/cli.rs` that take a `reuse_prev` game as their tainted game (they can take one built
  on `optimal.json`); DESIGN.md's rows and step 4.
* **`revise_freedoms`.**  In `src/search/setup.rs`: `revise_freedoms()`, the enum `Revise` with its eight values
  (`Off`, `NotNeeded`, `GreedyIsMax`, `Unchanged`, `Searched`, `Stopped`, `FromCheckpoint`, `ResumedAsBuilt`),
  `Revise::parse`, `frame_zero` and `sets_frame_zero`, `resumed_root`'s revised branch, `Prepared::unrevised` and
  `PrepareTimings::revise`; `FrameSet::with_frame_zero_freedoms` and `front_pair` (no other user); the checkpoint
  header's `frame0_freedoms` and tactic bit 2; the `revise` argument of `board::search_problem` and
  `search::problem_of`, and of the solver's `Earlier`; the solver's handling of a stopped revision; the line's
  `setup.revise` and `seconds.revise`, with their contradiction checks and taint rule in both verifiers; tests:
  `a_game_resumed_with_revised_freedoms_rests_on_revise_freedoms` and
  `a_game_resumed_with_the_built_freedoms_rests_on_nothing_unproven` (`tests/solver.rs`),
  `a_limit_also_stops_the_search_of_revise_freedoms` and the revised half of the frame-0 forgeries
  (`tests/robustness.rs`), the `setup.revise` cases of `tests/results.rs`, the revised half of the perturbed games of
  `tests/soundness.rs` (`Game::revised` and the exclusion of the games that break the conjecture), the Java's
  `revisedFrame0Freedoms` and `maxTotalPromotions` in `search/tests.rs`'s `check_against_java` (its
  `maxPromotionsUnrevised` takes their place); the `revise_freedoms=off` prefix of the tripwire rows of 1133 (the rows
  of 480 to 936 were taken with it on and should not move, since it changed no tree measured here; if one does, it is
  re-baselined).  The test of n = 833 stays as a fact about the frame model.
* **`legacy_bound`.**  `RunnerUp::Legacy` and `SearchTactics::rule`, the `legacy_bound` argument of `own_taint` and
  its rule in `tools/check_results.py`, header bit 1, the legacy comparisons of `bounds.rs`'s `upper_bound_is_sound`
  and the `legacy_bound` cases of `search/tests.rs`; all of `RunnerUp` goes with `match_bound=off`.
* **`preclusions`.**  `bounds::Preclusions` with `explore_preclusions` and `below`, the preclusion step of
  `Bounds::new_candidate_list_after`, the table's build in `prepare_state` and `PrepareTimings::preclusions`, the
  line's `seconds.preclusions` and `stats.precluded`, header bit 0; tests: `preclusions_do_not_depend_on_threads`,
  `preclusions_match_the_java_procedure` (the Java's `findPreclusions`) and `preclusion_lookup` in `bounds.rs`, the
  tripwire row of 480, a variant and a soundness configuration, the case of `tests/robustness.rs` that resumes with it
  changed (another tactic serves), and the preclusions of the random walks of `matchbound.rs`'s tests.
* **`match_bound=off`.**  The `blameGame` path of `bounds.rs` (`Bounds::upper_bound`, `reaches`, `largest_that_fit`,
  `upper_bound`, `new_candidate_list`'s promotability checks and `can_promote_certified`, `first_failure`, `Blame`,
  `RunnerUp`: about 300 lines, and about 300 of tests: `first_failure_matches_promoting_one_by_one`,
  `upper_bound_is_sound`, `candidate_list_check_limit`, the bound half of `greedy_and_bound_at_the_root_of_85`, the
  ignored benches); the frame certificates that only those checks use (`Certificate` and its recorder in
  `model/peel.rs`, `certify_using`, `winnable_with_move_using` and `winnable_without_factor_using` in
  `model/promote.rs`, and their model test); the searcher's `cut::BLAME` tests and the optional matching graph (it
  becomes mandatory); `search::Diagnostics`' `blameGame` gaps; the line's `stats.upper_bounds`, `can_promotes` and
  `certificates`; header bit 3; `matchbound.rs`'s ignored `matchbound_pruning` study against `blameGame`; a variant and
  a soundness configuration; DESIGN.md's account of the Java's loop as the baseline (section 4).  What stays of
  `bounds.rs`: `max_additional`, `greedy` (the Java's, checked against its golden data) and the narrowing of the
  children's candidates with its scratch space.
* **The other branchings.**  The tactic `branching` itself: the enum `Branching`, its names and parser, the match in
  `Stepper::choose_branch` (the fallback to the largest candidate where there is no conflict stays), `wants_conflict`,
  `fix_forced`'s dependence on `conflict-strong`, header bits 7 and 8; tests: two tripwire rows of 600, three variants,
  four soundness configurations, the branching cases of `src/tactics.rs`.

### Old file formats

Everything that exists only to read what older builds wrote, and what deleting it takes with it:

* **Results lines of schemas 1 to 3.**  In `src/results.rs`, `parse_line`'s schema branches (schema 1: no `schema`
  field, `reached_cap` and `in_the_bag` derived, no proof, taint or `prev` record), `taint_by_schema` (before schema
  3 a proof by the cap counted the game's own taint), the `FromCheckpoint` exception of schemas 1 and 2 in
  `contradiction`, the optional digest of lines before schema 4, and the module comment's account of them; the same in
  `tools/check_results.py` (its `schema` branches, `legacy` `FromCheckpoint`, the schema-1 cap and `in_the_bag`).  In
  the solver, the problems set up again stay (nothing rests on a digest alone), but the optional digest of
  `Earlier` and the schema-3 reason given for `SAME_SEARCH_WINDOW` go.  Tests:
  `a_chain_from_schema_3_lines_repeats_their_searches` with `testdata/strict_schema3_1_90.jsonl` (124 KB); the schema-2
  cases of `lines_whose_fields_contradict_each_other_are_refused_by_both_verifiers`; the schema-1 counts of
  `the_committed_results_pass_the_verifier`.
* **The committed schema-1 files.**  `results/default.jsonl` (1001..1231, the first run past 1000, its games resting on
  `reuse_prev` and `revise_freedoms`) has nothing left to show that `strict.jsonl` does not (its scores, and its moves
  but for 1221..1224, are `strict.jsonl`'s): delete it (git keeps it).  Its readers: `tests/results.rs`'s pinned
  counts, `tests/golden_hives.rs` (the games behind `accel_java.txt`, which goes with `reuse_prev`), the default
  `--results` of `evidence/src/bin/exact.rs` and `moves.rs` (a fallback past `optimal.json`, which now reaches 1300;
  point it at `strict.jsonl`), `COMMITTED_RESULTS` in `tools/cloud/env.sh` (used by `setup.sh` and `verify.sh`), and
  the text of README.md, `results/README.md`, DESIGN.md and RUNBOOK.md.  `bench/curve/chain.jsonl` (the cost-curve
  study's strict chain 1001..1231 of commit e710d16) is that study's raw data: `analysis.py` reads its seconds, and
  `--known` (`Optimal::with_results`) and `tools/cloud/scaling.sh` only its scores and moves, which work with any
  schema.  Keep it as data, like `calib.jsonl`, and take it out of what the verifiers check (`tests/results.rs`,
  `COMMITTED_RESULTS`, the README's and `results/README.md`'s commands); or delete it with the study's scripts
  reading it from git.  The notebook's entries that cite these files stay as history.
* **`strict.jsonl` itself is schema 4.**  The removals above change the line (`setup.revise`, `seconds.revise`,
  `seconds.preclusions`, the `stats` of the Java's search, `unproven_tactics_on`, the switches in `tactic_switches`),
  which makes schema 5 and schema 4 an old format.  Either the verifiers keep reading schema 4, or `strict.jsonl` is
  replaced by a run of the new build (about 18 minutes on four threads of the laptop, as for commit 2487671), after
  which only schema 5 is read.  The second leaves no old format at all.
* **Checkpoints.**  `src/search/header.rs` refuses version 2 (`TXS2`) with a message of its own, and records the
  tactics added after the format (`fix_forced`, `nogoods`, `forget_nogoods`, `lagrange`) by the bits of their
  non-default values so that older checkpoints read back with them on; `search::checkpoint_width` and
  `solver::solve_one`'s widening resume a checkpoint at the set width an older build chose (32 for 1024..2047).  The
  removals change the header anyway (the bits of the removed tactics, `frame0_freedoms`), so a version 4 with the
  remaining tactics recorded plainly can refuse everything older with one message.  Fixtures:
  `testdata/n1133_after_1000_nodes.ckpt` (W = 32, version 3, made before `lagrange`; `tests/wide.rs`'s
  `a_committed_checkpoint_of_a_w32_game_resumes`) is made again at the width the build gives 1133 (W = 20) and keeps
  its role as the format's tripwire; the width widening goes with the old width unless set widths are expected to
  change again; `testdata/perf/n0936.window.ckpt` and `n1215.window.ckpt` (made before `lagrange`, read back with it
  on) are made again with `perf make-window`, with a new fingerprint for DESIGN.md.  The `.nodes` fixtures and
  `testdata/n1215_deep_units.txt` are lists of numbers, not checkpoints, and stay; the engine's own format
  (`TXCKPT01`, version 1) has no older version to read.
* **The Java's golden data.**  What checks the frame builder and the setup stays, since the default search starts from
  it: `testdata/golden` (318 boards to 760) and `testdata/golden_wide` (11 boards to 2048) with `apiary_matches_java`,
  `frames_match_java` and `wide_boards_match_the_java` (hives, frames, the reduced frame set, the unrevised
  `max_total`, the greedy set), `testdata/games_java.txt` with
  `promotion_sets_become_the_java_game_beyond_the_golden_data` and `promotions_become_the_java_game` (the hives turning
  promotions into the Java's game), `bounds.rs`'s `golden_greedy`, `search/tests.rs`'s
  `golden_games_up_to_400_match_the_java` (the Java's promotion sums), the Java quirks of `hives.rs`, and
  `tools/golden/export.sh`.  What reproduces the Java's tactics goes with them:
  `testdata/accel_java.txt` and the `reuse_prev` checks (with `reuse_prev`), the revised freedoms (with
  `revise_freedoms`), `findPreclusions` (with `preclusions`), and `blameGame` with its runner-up (with
  `match_bound=off` and `legacy_bound`).  The golden files' fields that then nobody reads (`searchManager`'s
  `revisedFrame0Freedoms`, `solve.accelerated` if the prime half goes too) can stay as exported.

*Decision.*  None yet: phase 2 removes what the owner accepts, once the rewrite of the inner loop has merged.

## Faster nodes with the Lagrangian bound, same trees (2026-10)

*Question.*  With the Lagrangian bound ("A Lagrangian bound over the nogoods", above) proofs take about 3.3 times fewer
cycles, but each step costs about half as much again (the probe's window of 1215: 24.6k cycles a step against 16.4k
with `--tactic lagrange=off`).  Where do the cycles go now, and what can be made cheaper without changing any tree?
And does `match_refine` = 0, which the last round flagged, now pay?

*Method.*  As in "Faster nodes, same trees": every change keeps both fingerprints of the probe (36981d0eaeeae319 with
the defaults, 6197a47eae6cf8c8 with `--tactic lagrange=off`) and the node counts of one-thread proof-only searches
(`ladder --threads 1 --seed-opt --known results/strict.jsonl --tactic revise_freedoms=off N`), and is measured in
cycles and instructions retired (the probe's own, the fastest of its repetitions, three runs of each build,
interleaved; for whole searches `/usr/bin/time -l`).  The laptop was shared with a chain run and other agents (load
2 to 30), so instructions separate signal from noise where cycles move by a percent or two.

**The profile.**  `tools/profile/profile.sh N 60 2` (commit a2da702) on the hardest games of the strict chain as the
chain runs them, two threads: 1254 (done in 25.3 s), 1272 (17.2 s), 1288 (34.1 s), 1290 and 1300 (stopped at 60 s).
`attribute.py` now charges the Lagrangian levels' work to kinds of its own (commit dd4f89c).  Shares of busy
samples:

| kind of work | 1254 | 1272 | 1288 | 1290 | 1300 |
|---|---|---|---|---|---|
| matching repairs (Hungarian searches) | 34.3% | 33.4% | 36.0% | 38.8% | 36.8% |
| Lagrangian levels: repairs (Hungarian searches) | 17.5% | 18.1% | 17.7% | 16.4% | 16.0% |
| Lagrangian levels: refits (50 solves from scratch each) | 4.2% | 4.3% | 4.0% | 3.7% | 4.1% |
| Lagrangian levels: forced members (from the potentials) | 3.1% | 3.3% | 2.9% | 3.3% | 2.9% |
| Lagrangian levels: applying changes (`derive`) | 1.9% | 1.5% | 2.2% | 2.0% | 1.7% |
| Lagrangian levels: copies | 1.4% | 1.7% | 1.5% | 1.3% | 1.5% |
| peels and promotions | 19.1% | 19.9% | 17.4% | 17.1% | 18.3% |
| the node's own work | 5.0% | 4.8% | 6.4% | 6.0% | 6.5% |
| nogood table | 5.1% | 4.8% | 4.0% | 4.0% | 4.2% |
| bounds from the potentials (the matching's) | 4.4% | 4.5% | 4.2% | 4.0% | 4.3% |
| matching level copies | 2.7% | 2.7% | 2.4% | 2.3% | 2.4% |
| other copies (frame sets, nodes) | 0.8% | 0.7% | 0.7% | 0.8% | 0.8% |

| purpose | 1254 | 1272 | 1288 | 1290 | 1300 |
|---|---|---|---|---|---|
| the Lagrangian bound of a node or child, and its forced members | 24.3% | 25.0% | 25.0% | 23.5% | 22.7% |
| making the child (its promotion, candidate list, rule-outs) | 22.4% | 23.5% | 20.6% | 20.3% | 21.9% |
| strong branching (exact bounds without each member) | 16.2% | 13.4% | 16.6% | 19.9% | 18.4% |
| refinement (bounds of the branches of the conflict) | 13.5% | 13.6% | 14.8% | 14.2% | 13.6% |
| the matching bound of a node or child | 8.8% | 9.9% | 8.8% | 8.8% | 9.0% |
| forced members and the conflict (`Refiner::top`) | 5.5% | 5.5% | 5.0% | 4.9% | 5.3% |
| conflict lookup, or computation | 4.1% | 3.7% | 4.4% | 4.3% | 4.5% |
| refits of the multipliers | 4.1% | 4.3% | 4.0% | 3.6% | 4.0% |

So the Hungarian searches are 55-60% of the time, the matching's and the Lagrangian levels' together; a Lagrangian
level costs about what a matching level does.  The searches are large: at the proof of 1215 (one thread, its first
300,000 steps) 88% of the vertices the searches put in their trees are in trees of 64 to 511 vertices, of the
graph's 686, so their work is per edge, not per search.  And two counts at 1250 (one thread, per step): strong
branching ran 0.26 times with 0.70 exact solves, each run after refinement had solved the level without the
conflict's smallest member (0.45 of those a step), the very level strong branching solves first.

`sample` lists the several addresses of a leaf on one line and the attribution symbolicates only the first, which is
right to the function but coarse below it (`attribute.py` says so).  For the picture by source line the proof of
1215 was recorded, after the first change below, with Instruments' Time Profiler (`xcrun xctrace record --template 'Time Profiler' --launch --
...`, every sample with its own address), which agrees on the kinds (the matching's searches 31.9%, the Lagrangian
levels' 19.0%, peels 18.4%) and shows the Hungarian search's edges at about 20 instructions when an edge is skipped
and 45 when it pushes an event, at about 3.3 instructions a cycle.

**Changes kept.**  Cumulative, all builds measured together (k cycles a step in the probe's windows, the defaults and
`lagrange=off`; G cycles of the proof of 1250):

| commit | change | window 936 | window 1215 | off: 936 | off: 1215 | proof 1250 |
|---|---|---|---|---|---|---|
| a2da702 | before | 24.15 | 24.56 | 17.35 | 16.44 | 20.56 |
| 40bc726 | strong branching takes the level refinement solved for the first member | 22.41 | 22.85 | 15.78 | 15.01 | 19.66 |
| 602a4b6 | the bound from the potentials stops its scans at a tight edge | 21.98 | 22.53 | 15.52 | 14.78 | 19.25 |
| 34648e1 | one settle for the forced members, a membership test for known conflicts | 21.86 | 22.36 | 15.36 | 14.71 | 19.04 |
| 5755c3e | the Hungarian search's fields apart while reaching, a fixed-size code set | 21.63 | 22.22 | 15.29 | 14.68 | 18.99 |
| 31b9c95 | the Lagrangian bound's forced members only among those not forced yet | 21.59 | 22.04 | 15.35 | 14.61 | 18.91 |

* **Refinement's first branch for strong branching** (40bc726).  Where nothing is forced, refinement solves the node
  without the smallest member of its conflict, and strong branching then solved that level again.  Refinement now
  keeps it (`MatchBound::keep_solved`, in the spare slot that holds strong branching's kept level), `value_excluding`
  of that member takes it, and a node that goes on without that member (a conflict of one) takes it as its kept level;
  anything else done to the node drops it.  The same level, so the same tree; the walk of matchbound's tests keeps
  levels solved ahead and checks each one taken against the level solved afresh.  The windows −7.2% and −7.0%,
  `lagrange=off` −9.1% and −8.7%, the proof of 1250 −4.3%.
* **The bound from the potentials stops at a tight edge** (602a4b6).  `bound_excluding` lowers the potentials of `x`
  and of the factor it pays with as far as their edges allow, a scan of every mover above each.  No counted edge has
  negative slack, so neither can need more than its potential: a scan stops when it gets there, the same value (the
  agreement tests check every member against the full scan).  −1.4% to −2.1%.
* **Cheaper node work** (34648e1): the matching's forced members found with one settle of the level instead of one
  per member (the scan shared with the Lagrangian bound's), and whether a known conflict can rule out a candidate of
  a child that promotes one number is a membership test, not a set intersection.  −0.5% to −1.1%.
* **The Hungarian search's fields apart** (5755c3e).  `add_inner` reached each neighbour through `&mut self`, so the
  slots' place and the epoch were loaded again at every edge (an event pushed might have moved them); taken apart,
  they stay in registers.  And bucket 0's code set is a bitset of fixed size, whose insertion checks nothing.
  Instructions −2.0% to −2.2%, cycles −0.2% to −1.1% (five runs each when it was committed: −1.3% and −1.1% in
  the windows, none on the proof of 1250).
* **Forced members only among those not forced yet** (31b9c95): the Lagrangian bound's forced members join the
  matching's, so its heaviest matching's members already among them need no bound.  Instructions −0.5%, cycles
  −0.8% to +0.4%.

In all, the windows −10.6% and −10.2% cycles (−15.3% and −14.0% instructions), with `lagrange=off` −11.6% and −11.1%
(−16.2% and −15.6%); the probe in all −5.5% (its kernels of peels and frame checks are unchanged).  Whole proofs,
before and after (G cycles, nodes unchanged): 936 1.22 → 1.11 (−9.1%), 1050 2.11 → 1.97 (−6.3%), 1100 1.07 → 1.01
(−5.4%), 1170 0.56 → 0.52 (−5.7%), 1184 1.65 → 1.51 (−8.5%), 1215 33.1 → 30.1 (−9.2%), 1250 20.6 → 18.9 (−8.0%), 1254
146.9 → 134.0 (−8.8%); with `lagrange=off` 936 −8.3%, 1100 −6.0%, 1184 −9.5%, 1215 107.3 → 96.0 (−10.5%).  (1133,
2,122 steps, is mostly setup: +2.3% cycles, −1.7% instructions.)  The strict chain 1201-1231 on one thread (`taxman
solve 1201-1231 --strict --allow-tainted-prev`, from optimal.json's 1200): 1092.0 G cycles (345 s) before, 1003.2 G
(315 s) after, −8.1% (instructions −14.0%), every game's nodes the same; `taxman regress --to 600 --strict` −1%.
1254 as the chain runs it on two threads, profiled again: 21.4 s against 25.3 s.  A step of the window of 1215 now costs 22.0k cycles with the Lagrangian bound and
14.6k without.

Where the time goes now (the proof of 1215, one thread, by Instruments): the matching's searches 32.6%, the
Lagrangian levels' 19.2%, peels 18.9%, the node's own work 9.7%, the nogood table 3.7%, refits 3.2%, bounds from the
potentials 3.0% and the Lagrangian bound's 2.6%, level copies 2.3% and 1.7%, the Lagrangian levels' changes 1.7%;
by purpose the Lagrangian bound 25.8%, making children 23.5%, refinement 13.8%, strong branching 11.4% (16-20%
before), the matching bound of a node or child 10.5%.

**Measured and not kept** (instructions and cycles, one thread):

* One key for a search slot's epoch and tag (the epoch times 4 plus the tag, one comparison): instructions −0.02%,
  cycles within ±0.8%.  The compiler had made the two tests cheap already.
* `-C target-cpu=native` (not the default build): instructions ±0.05%, cycles within noise.
* Skipping the forced-member check of a member `x` with `value − x ≥ need` (its bound without `x` cannot fall below
  `need`): counted first, it applies to 0.17 of the 19.8 checks of a step at 1250 (0.009 of 17.8 for the Lagrangian
  bound's); not built.
* Strong branching skipping a member whose lower bound `value − x` is above the best so far and at least `need`: it
  can never apply, since the members are solved in increasing order and `value − x` is below the exact bound of every
  member before `x`.
* Refinement only where the Lagrangian bound does not cut: refinement one level deep has no side effect but its
  levels, yet a Lagrangian level is derived from the levels made before it, and which optimal matching it reaches
  (so which members its potentials show forced) depends on that history: it changes trees.  Not built.
* A level as one record per vertex (potential, weight, partner and role together, so that an edge loads one place):
  not built; at 24 bytes a vertex against 11 now, its copies (4% of the time) would cost what it saves.

**`match_refine` = 0.**  One-thread proof-only searches with the build of 31b9c95 (G cycles and nodes; the ratio is
of cycles, 0 against 1):

| proof | nodes, 1 | nodes, 0 | G cycles, 1 | G cycles, 0 | cycles |
|---|---|---|---|---|---|
| 936 | 41,653 | 44,119 | 1.14 | 1.12 | −1.5% |
| 1050 | 46,527 | 48,049 | 2.00 | 1.86 | −6.8% |
| 1100 | 29,614 | 30,690 | 1.03 | 1.01 | −2.3% |
| 1133 | 2,122 | 2,156 | 0.36 | 0.36 | −0.3% |
| 1170 | 8,013 | 8,136 | 0.51 | 0.51 | −1.0% |
| 1184 | 52,259 | 54,083 | 1.50 | 1.46 | −2.7% |
| 1196 | 216,076 | 258,764 | 5.38 | 5.89 | +9.5% |
| 1204 | 498,121 | 530,763 | 12.9 | 12.7 | −1.6% |
| 1215 | 1,111,451 | 1,123,153 | 29.7 | 27.8 | −6.6% |
| 1220 | 2,503,016 | 2,739,817 | 80.1 | 80.1 | 0.0% |
| 1225 | 2,178,362 | 2,875,317 | 79.6 | 95.3 | +19.7% |
| 1240 | 496,460 | 567,144 | 17.0 | 17.5 | +2.9% |
| 1250 | 709,893 | 669,488 | 19.0 | 16.7 | −12.1% |
| 1254 | 4,830,651 | 4,075,698 | 134.4 | 106.2 | −21.0% |
| 1257 | 3,686,322 | 3,447,999 | 97.0 | 83.5 | −13.9% |
| 1272 | 4,341,863 | 4,420,692 | 116.4 | 108.7 | −6.6% |
| 1280 | 3,080,373 | 3,930,824 | 99.2 | 113.7 | +14.6% |
| 1285 | 3,442,845 | 3,692,516 | 106.2 | 105.2 | −0.9% |
| 1288 | 4,822,476 | 4,398,614 | 150.4 | 123.8 | −17.7% |

Thirteen proofs are cheaper without refinement, five dearer, one even: a geometric mean of −3.0% (the logarithms'
standard error 2.4%), −5.3% in all (953.8 G against 903.3 G), −3.3% on the thirteen from 1196 up.  Without
refinement nodes are cheaper and usually more; but what the tree becomes (the nogoods learned, the Lagrangian
bound's multipliers and forced members) swings each game by up to 20% either way.  The strict chains, one thread, the
same build: `taxman regress --to 600 --strict` 5.76 G against 5.70 G (−1%, two runs each, within their spread);
`--to 1000` 513.4 G (162.1 s) against 465.0 G (145.4 s), −9.4%; and the frontier, `taxman solve 1201-1231 --strict
--allow-tainted-prev` from optimal.json's 1200, 1003.2 G (315 s) against 1007.3 G (317 s), +0.4% (1215 8.5 → 13.5 s,
1221 20.2 → 16.7 s, 1225 24.8 → 29.9 s, 1230 201.0 → 197.4 s).  (The last round's four proofs gave 0 to 26% less;
refinement's first branch, now shared with strong branching, narrowed that by 2 to 5 points.)

*Decision.*  Kept: the five changes, the same trees with about 10% fewer cycles at the frontier.  `match_refine` stays
1: refinement no longer pays on average, by a few percent, but the evidence is not clear across the range, a mean
of about one standard error with games swinging 20% either way, and the frontier's chain is even.  It would take a
larger sample of the frontier (the searched games of a strict chain past 1231, on the many-core machine) to move it.

*What is left.*  The Hungarian searches are over half the time and large, and their edges are already cheap; fewer
of them would change trees (refits that keep the levels, fewer Lagrangian levels) or take a refactor of the levels
(see the record per vertex above).  The peels, 17-21%, are the work "Faster nodes, same trees" took on and left.

## What drives the slope (2026-10)

*Question.*  The cost of a game grows about ×10 per 55-70 in n near the frontier
([bench/curve](../bench/curve/README.md#results)), and every improvement so far (same search, the Lagrangian bound,
faster nodes) moved the curve down without changing its slope.  What drives the slope: the room for doubt at the root
(the gap between the root's upper bound and the optimum, which the tree must rule out), the size of the problem (n,
candidates, `max_total`), or something else?  And so what would change it?

*Method.*  No search code changed.  `examples/slope.rs` (commit db7faf1) measures the root of a game and runs proofs
with the incumbent set by value, through the library's public interfaces (`Board`, `MatchBound`,
`matchbound::lagrange`, `engine::run`); `bench/curve/slope.py` makes the tables below (and more) from its two data
files, `bench/curve/slope_roots.jsonl` and `bench/curve/slope_proofs.jsonl`, and `results/strict.jsonl`.  The solver
of 331eb09 with `--tactic revise_freedoms=off` (the strict tactics), one thread a process and at most two processes, on
the laptop while another agent ran the test suite.  `P*` is a game's optimum promotion sum (its score minus
`in_the_bag`).

* *Roots* (`slope roots --known results/strict.jsonl --sums bench/curve/bands.jsonl --tactic revise_freedoms=off
  880-1300` and the games past 1300 that `bands.jsonl` solved; about a second a game): the root's matching bound; the
  matching bound refined one to three levels deep (the search's refinement, computed to a value); and a Lagrangian
  bound over nogoods of the root, made without a search: the conflicts the refinement found, then up to 8,000
  subgradient steps towards `P* + 1` (Polyak's rule damped by halving after 80 steps without a lower bound), each
  adding up to 12 new conflicts of the weighted matching's candidates; it settles on 140-840 nogoods (band means).  The
  *matching*, *refined* and *Lagrangian gaps* are these bounds minus `P*`.  Also the chain's start incumbent and cap,
  and the candidates each bound leaves undecided with the incumbent at the optimum.
* *Proofs* (`slope prove --known results/strict.jsonl --tactic revise_freedoms=off --deltas ... N`, with `--sums` as
  above past 1300): the search of the game on one thread, its incumbent a value `P* + δ` with no set behind it, no
  seed, no cap.  With δ = 0 it is the proof of the optimum (the `--seed-opt` ladder's node counts: 29,614 for 1100,
  19,473,016 for 1290 and 28,024,916 for 1300, as in
  ["A Lagrangian bound over the nogoods"](#a-lagrangian-bound-over-the-nogoods-2026-10)), for all 143 games of 880-1300
  that `strict.jsonl` searched to exhaustion (the others are repeats, caps and primes) and for 1309, 1350 and 1352.
  With δ > 0 nothing beats the incumbent, and every cut is the cut the search would make with every
  bound it uses (matching, refined, Lagrangian, forced members) lowered by δ: a *margin scan* over δ = 25 to 800
  measures how the tree shrinks as all the bounds tighten uniformly, for 19 games of 900-1300, and at δ = 100-300 for
  1302, 1305, 1309, 1350 and 1352.  With δ < 0 the search starts below the optimum.  (`--diag` runs, whose lines
  carry `steps_by_depth`, have the same trees and slower times.)
* The recorded node counts of `strict.jsonl` are of four threads from the chain's start incumbent: 1.35 times the
  proof's (geometric mean) for the 87 games that start at the optimum, 1.63 times for the 56 that start below it (63
  below on average).  Every fit gives the same answer on either count; the tables use the proofs.

**1. The node count follows the gap, not the size.**  Least squares of log10 of the proof's nodes over the 143
searched games:

| variable | slope per unit | R² | residual sd (decades) |
|---|---|---|---|
| n | 0.0045 ± 0.0007 | 0.23 | 0.97 |
| candidates | 0.017 ± 0.002 | 0.26 | 0.96 |
| `max_total` | 0.067 ± 0.009 | 0.27 | 0.95 |
| matching gap | 0.00374 ± 0.00012 | 0.87 | 0.41 |
| refined gap (3 levels) | 0.00427 ± 0.00014 | 0.86 | 0.41 |
| Lagrangian gap | 0.00758 ± 0.00021 | 0.90 | 0.35 |
| cap gap (cap − `P*`) | 0.0000 ± 0.0005 | 0.00 | 1.11 |
| greedy gap (`P*` − greedy) | 0.0020 ± 0.0002 | 0.42 | 0.85 |
| start gap (`P*` − start) | 0.0045 ± 0.0018 | 0.04 | 1.09 |

With the Lagrangian gap in the fit, n adds nothing (+0.0003 ± 0.0003 per n, t = 1.1), nor do the candidates (t =
1.1) or `max_total` (t = 1.0); the residuals of the gap's fit are uncorrelated with n (0.08) and with the
candidates (0.08), while those of the fit on n correlate 0.82 with the gap.  With the matching gap instead, n comes in
negative (−0.0015 ± 0.0004).  Matched pairs agree: 342 pairs of searched games within 15 of each other in Lagrangian
gap and at least 100 apart in n differ by −0.07 decades per 100 in n (sd 0.29); 349 pairs within 30 in n and at
least 100 apart in the gap differ by +0.64 decades per 100 of gap.  The natural experiment is the dip of 1050-1190:
more candidates than 950-1050, gaps half as large, a hundred times fewer nodes (the bands in 3).

**2. A margin costs what a gap costs.**  The margin scans (log10 of the proof's nodes; the slope is fitted over
δ = 0-200, and "δ at 100 nodes" is interpolated):

| n | matching gap | Lagrangian gap | δ = 0 | 100 | 200 | 400 | 600 | decades per unit of δ | δ at 100 nodes |
|---|---|---|---|---|---|---|---|---|---|
| 900 | 460 | 99 | 3.62 | 3.03 | 2.12 | 0.30 | 0.00 | 0.0076 | 216 |
| 936 | 618 | 201 | 4.62 | 3.46 | 2.78 | 1.32 | 0.00 | 0.0089 | 283 |
| 950 | 756 | 262 | 5.66 | 4.40 | 3.36 | 2.09 | 1.08 | 0.0117 | 418 |
| 992 | 951 | 477 | 6.32 | 5.34 | 4.26 | 2.64 | 1.75 | 0.0105 | 543 |
| 1000 | 911 | 398 | 5.56 | 4.36 | 3.38 | 2.16 | 1.56 | 0.0111 | 454 |
| 1025 | 852 | 434 | 6.61 | 5.53 | 4.45 | 2.96 | 1.82 | 0.0107 | 568 |
| 1050 | 629 | 258 | 4.67 | 3.56 | 3.19 | 1.85 | 0.30 | 0.0073 | 377 |
| 1088 | 421 | 103 | 3.79 | 2.80 | 1.83 | 0.30 | 0.00 | 0.0096 | 181 |
| 1100 | 508 | 181 | 4.47 | 3.38 | 2.57 | 1.30 | 0.00 | 0.0089 | 305 |
| 1150 | 542 | 121 | 3.69 | 2.87 | 2.17 | 1.00 | 0.00 | 0.0076 | 247 |
| 1184 | 666 | 153 | 4.72 | 3.49 | 3.21 | 1.70 | 0.30 | 0.0080 | 358 |
| 1196 | 893 | 274 | 5.33 | 4.72 | 4.05 | 2.63 | 1.41 | 0.0063 | 504 |
| 1215 | 966 | 366 | 6.05 | 5.10 | 4.39 | 3.21 | 1.90 | 0.0085 | 584 |
| 1225 | 1200 | 529 | 6.34 | 5.44 | 4.88 | 3.96 | 3.20 | 0.0074 | over 800 |
| 1240 | 1208 | 385 | 5.70 | 5.23 | 4.70 | 3.49 | 2.76 | 0.0053 | 744 |
| 1250 | 1087 | 396 | 5.85 | 5.06 | 4.43 | 3.30 | 2.27 | 0.0068 | 664 |
| 1254 | 1251 | 474 | 6.68 | 5.57 | 4.99 | 3.56 | 2.73 | 0.0087 | 771 |
| 1288 | 1216 | 456 | 6.68 | 5.69 | 5.00 | 3.69 | 2.64 | 0.0082 | 731 |
| 1300 | 1421 | 529 | (7.45) | 6.44 | 5.39 | 3.83 | 2.82 | 0.0103 (from δ = 50) | 794 |

Every tree shrinks by 0.005-0.012 decades per unit of margin (×10 per 85-190), with no trend in n, and at about the
rate the searched games show per unit of Lagrangian gap (0.0076).  Over the 112 scan points with δ ≤ 200 and at least
32 nodes, log10 nodes = a + c (Lagrangian gap − δ) fits with c = 0.0074 ± 0.0002, R² 0.91, residual sd 0.36, as
well as the 143 proofs fit their gaps; adding n gives +0.0009 ± 0.0003 per n (with the matching gap, −0.0023 ±
0.0005), a tenth of a decade per 100 in n at most.  A large game tightened costs what a small game with the same
remaining gap costs: at a remaining Lagrangian gap of about 100, 900 untouched 3.62, 1088 3.79, 1184 at δ = 50 4.13,
1250 at δ = 300 3.68; at about 200, 936 4.62, 1050 at δ = 50 4.02, 1250 at δ = 200 4.43.

**3. The slope is the gap's growth.**  By bands (means of the searched games; past 1300 the games of `bands.jsonl`):

| n | games | candidates | matching gap | Lagrangian gap | log10 proof nodes |
|---|---|---|---|---|---|
| 851-900 | 6 | 216 | 440 | 115 | 3.78 |
| 901-950 | 16 | 224 | 638 | 201 | 4.56 |
| 951-1000 | 16 | 238 | 772 | 279 | 5.21 |
| 1001-1050 | 19 | 253 | 788 | 363 | 5.62 |
| 1051-1100 | 18 | 267 | 537 | 197 | 4.46 |
| 1101-1150 | 17 | 277 | 524 | 167 | 3.82 |
| 1151-1200 | 16 | 290 | 664 | 171 | 4.52 |
| 1201-1250 | 19 | 305 | 1100 | 439 | 6.13 |
| 1251-1300 | 16 | 322 | 1282 | 475 | 6.90 |
| 1301-1310 | 7 | 332 | 1445 | 554 | 7.14 (1309 only) |
| 1350-1352 | 3 | 343 | 1255 | 465 | 6.88 (1350, 1352) |

Over the 361 non-prime games of 880-1300 the matching gap grows by 142 ± 9 per 100 in n and the Lagrangian gap by 56
± 5, jaggedly (R² 0.39 and 0.24): the Lagrangian gap halves from 1001-1050 to 1101-1150 and nearly triples to
1251-1300.  The cost's slope in n is the product of the cost per unit of gap and the gap's growth: 0.0076 × 0.56 =
0.0042 decades per n over 880-1300, against 0.0045 fitted on n; and from 1151-1200 to 1251-1300 the Lagrangian gap
rises by 304, which predicts 2.31 decades against 2.38 measured.  The ×10 per 55-70 of the cost-curve study
(standalone times of bands from 1200 to 1350) is this rise, and at 1350 the cost of finding the optimum (94% of its
standalone search came before the final incumbent).  Past 1300, out of sample, the gaps are those of 1251-1300 or a
little more, and so are the proofs: the fits of section 1 predict 1309, 1350 and 1352 (7.14, 7.24, 6.52) within 0.26
decades by the matching gap (7.40, 7.03, 6.64), and 0.16-0.55 too low by the Lagrangian gap (6.98, 6.69, 6.09); the
margin scans of 1302, 1305 and 1309 fall within 0.25 of the scan fit, those of 1350 and 1352 0.5-0.9 above it, so
the Lagrangian gap misses something at 1350 that the matching gap does not.

**4. The other candidates.**

* *The start incumbent.*  It explains nothing of the variation (R² 0.04 above).  Starting the search below the
  optimum (δ < 0) costs 0.9-1.8 times the proof's nodes at 10 to 100 below and 1.4-3.0 times at 200 below (936,
  1100, 1184, 1215, 1250, 1254); the chain's start is the optimum in 87 of 143 games and 63 below in the others.
* *Depth.*  The trees are deep and their depth is the size: the deepest node has 42 (900) to 66 (1300) promotions,
  5-13 below `max_total`, and cheap games are as deep as dear ones (1150: 4,858 nodes to depth 55; 1215: 1.1M to
  64).  What differs is the width: the steps per level grow by 0.045-0.064 decades a level up to the busiest level,
  near the bottom.
* *The cap* (cap − `P*`, R² 0.00) and the greedy set (0.42, through its correlation with the gap) do not drive it.

**5. Two pilots on the Lagrangian bound** (one thread, code not kept).

* *The root's converged relaxation as the search's starting one.*  Each searcher began with the nogoods and damped
  multipliers of the root's fit above (43-81 nogoods with positive multipliers), instead of none until its first
  refit; its refits then went on as usual.  At the root that bound is 310-690 below the refined matching bound (the
  search's check there until its first refit), which by the margin scans' rate would be worth two decades and more if
  the search's bound were that much lower at every node:

  | proof | nodes | nodes, seeded | s | s, seeded |
  |---|---|---|---|---|
  | 936 | 41,653 | 24,492 (−41%) | 0.30 | 0.19 |
  | 1100 | 29,614 | 22,057 (−26%) | 0.24 | 0.19 |
  | 1184 | 52,259 | 12,681 (−76%) | 0.38 | 0.11 |
  | 1215 | 1,111,451 | 889,919 (−20%) | 9.5 | 7.7 |
  | 1250 | 709,893 | 481,946 (−32%) | 6.0 | 4.4 |
  | 1254 | 4,830,651 | 4,570,450 (−5%) | 43.9 | 39.8 |

  A constant factor, and the smallest at the frontier: the nodes are 30-45 promotions deep, where the root's
  multipliers fit little.  The gap that counts is the relaxation's at every node; the root's measures it only because
  the gaps at the nodes scale with it.
* *Damped steps in the refit* (`matchbound::lagrange::fit`, halving after 3 or 5 steps without progress): on the
  root's nogoods, 50 damped steps reach the bound of 50 undamped ones within 8, both 50-540 above the converged
  bound over the same nogoods (band means, the most past 1250; section 9 of `slope.py`'s output).  The
  deficit of a refit from zero multipliers is its step count, not its rule; the search's refits start from the last
  multipliers, and how far its multipliers are from converged was not measured.

*Conclusion.*  log10 of a proof's nodes is about a + c·G, where G is the root's gap and c is the same within a game
(the margin scans) as across games, with no trend in n from 880 to 1352: ×10 per about 130 of Lagrangian gap, or 270
of matching gap.  At a given gap the size of the problem adds nothing measurable.  So the slope in n is c × dG/dn:
the growth of the room for doubt, which is jagged (it halved over 1050-1150 and nearly tripled over 1190-1300) and
which past 1300, on the eleven games sampled, has not gone on rising.

*What it implies.*

* *A bound whose gap grows more slowly* changes the slope, and is the lever: closing a fixed amount of the gap
  everywhere is a constant factor (×10 per 130 of Lagrangian gap); closing a share of it lowers the slope by that
  share, and a gap that grows half as fast halves it.  What grows is the relaxation's error, not the problem.  The
  Lagrangian bound over the root's nogoods (an LP with their cover inequalities) leaves 26-46% of the matching gap and
  its own gap grows too, so a better bound must see what nogood inequalities cannot; and by the first pilot it must be
  tighter at every node, not only at the root.
* *Reuse of n − 1's proof*: no evidence for it.  No part of the cost scales with the size of the problem at a given
  gap, so there is no size-driven work for a proof of n − 1 to save; games with the same problem are repeats already
  (`same_search`), and the costs of the others follow their own gaps, which move by hundreds between neighbours.  It
  could save a constant at best.
* *Dominance among interchangeable alternatives* would act on c, the nodes per unit of gap, not on the gap; since
  the slope is c × dG/dn, lowering c by a share would lower the slope by the same share.  But nothing here shows that
  interchangeable alternatives make c (no symmetries were found, IDEAS.md section 5), and c does not grow with n.
  Untested.
* *Something else*: the gap is a cheap predictor of a proof's cost (a second at the root; 0.35 decades residual sd,
  0.41 with the matching gap, which needs only milliseconds), for budgeting games before their search and for
  forecasting the curve where `P*` is known or closely bounded.  Finding the optimum costs a factor of 1 to 3 (section
  4), not the slope.

*Next.*  Screen candidate bounds by their gaps before building them into the search, since a gap predicts a proof's
nodes: (a) decompose the root's gap by frame (the weighted matching's promotions against the optimum's, frame by
frame) over 880-1352, to see where the relaxation's error lives and which frames carry its growth; (b) compute a
candidate relaxation's gap at the root and at the probe's 300 search nodes of 936 and 1215 (`testdata/perf`) next to
the current bounds', and predict the proof's nodes by 10^(−c·Δgap) with c from the margin scans; build only a bound
whose gap at the nodes is smaller by a share that grows with n.  (Evidence section 8 names one: a relaxation that
sees that a move sweeps every divisor.)
