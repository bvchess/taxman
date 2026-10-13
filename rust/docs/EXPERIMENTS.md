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
| Reduced-cost fixing of candidates from the dual | [IDEAS 4.3](IDEAS.md#43-reduced-cost-fixing-killed) | not kept | fixes 70.9 of 159 candidates outside `Q*` per node at 936, yet changes no tree: the matching relaxation is integral |
| Component decomposition (AND/OR search, #SAT) | [IDEAS 4.4](IDEAS.md#44-component-decomposition-killed) | not kept | exactly one component holds candidates at the root of 936-1300 and at all 600 probe nodes |
| Disjoint conflicts from the dual (MaxSAT's disjoint cores, the pair cut) | [IDEAS 4.5](IDEAS.md#45-disjoint-conflicts-from-the-dual-killed) | not kept | between 2.7% fewer and 0.3% more nodes for 12% more cycles |
| A portfolio: another branching on some threads for finding incumbents | [IDEAS 4.7](IDEAS.md#47-a-portfolio-for-finding-incumbents-weak) | not kept | the frontier's cost is the proof; 8 of the 11 slowest games of 1232-1300 start at their optimum |
| Reliability branching (pseudocosts) in place of strong branching's exact solves | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | strong branching is 28-34% of the time; not piloted |
| Knuth's estimate of a tree's size, to budget games and split work | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | not piloted |
| Ideas considered and not piloted: restarts, bound nogoods and backjumping, cube-and-conquer splitting, dominance rules, symmetry breaking, best-bound node selection, cutting planes | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | each has its reason there (with the optimum known the tree is fixed by the bound; the matching LP is integral; no symmetries found; ...) |

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
| Lower frame 0's freedoms by solving the two-frame game (`revise_freedoms`) | [DESIGN 4](../DESIGN.md#revise_freedoms) | kept, unproven; off in a strict run | assumes the max-sum set also has the max count, which fails on about one perturbed game in 1,300, and for the whole frame game of n = 833 (48 promotions optimal, 49 feasible: [evidence 6](../evidence/README.md#6-does-an-optimal-game-play-the-most-moves)); the two-frame form it applies is undecided |
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
| [Benchmarks](#benchmarks-the-probe-pgo-and-scaling) | what the bench set, the probe, PGO and the thread scaling showed | numbers | e3b8ddc and after |

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
