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
| Reimplement the Java's search and check its upper bound | [port](#the-port-and-its-first-improvements) | kept, with a fix; the Java's bound removed 2026-10 | the Java's runner-up can be too low (root of n = 114: 358 against the best completion's 360); the sound runner-up was the default and `legacy_bound` reproduced the Java's, until the matching bound left `blameGame` nothing to do and both went ([tactics review](#which-tactics-earn-their-keep-2026-10)) |
| A simpler sound runner-up: the largest candidate not in the bid | [port](#the-port-and-its-first-improvements) | not kept | n = 700 took over 10 minutes instead of 18 s |
| Matching bound (max-weight bipartite matching, kept incrementally with dual potentials) instead of `blameGame` | [after parity](#improvements-after-parity) | kept | mean gap at the root 31 against the Java bound's 76 on the 94 golden games that need search; 1.5 to 85 times fewer nodes at n = 480-720 |
| The minimum of `blameGame` and the matching bound | [after parity](#improvements-after-parity) | not kept | `blameGame` pruned nowhere the matching bound did not (0 at every check) |
| Seed the incumbent from the game of n − 1 and stop at the cap n + opt(n − 1) (`seed_incumbent`, `prev_cap`) | [after parity](#improvements-after-parity) | kept | 415 of the 1300 proofs of the strict chain of commit bd9bb5d are the cap, reached before the first node; the committed chain, which also has `same_search`, has 115 and 545 repeats ([results](../results/README.md#what-supports-the-scores)) |
| Branch on a member of the heaviest matching's conflict instead of the largest candidate | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept, the decisive change | 936: 90.8 s to 31.8 s (12.3M to 2.62M nodes); 1000: open at 120 s to 27.5 s (10 threads) |
| Strong branching: the member whose removal lowers the bound most, its level kept for the node (`conflict-strong`) | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept, the default | 936: 29.5 s to 17.7 s, 2.62M to 909k nodes |
| The other branchings, `conflict-smallest` and `conflict-largest` | [conflicts](#conflicts-of-the-matching-bound-2026-10) | kept as switches, removed 2026-10 ([tactics review](#the-alternatives-that-are-off)) | smallest gives the smallest proof trees at 880 and 936 but finds good incumbents late; largest follows the dive and finds them early |
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
| The Java's depth-2 preclusion table | [forced members](#forced-members-and-nogoods-2026-10-second-round) | superseded by nogoods (`preclusions` off), removed 2026-10 ([tactics review](#the-alternatives-that-are-off)) | without it 992 and 1215 take +5% and −4%; 1101-1217 on 8 threads spent 53 of 131 s building tables |
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
| Ideas considered and not piloted: restarts, bound nogoods and backjumping, cube-and-conquer splitting, best-bound node selection, cutting planes | [IDEAS 5](IDEAS.md#5-every-idea-considered) | open | each has its reason there (with the optimum known the tree is fixed by the bound; the matching LP is integral; ...); dominance rules and symmetry breaking were piloted later (rows below) |
| What drives the slope: the root's gaps (matching, refined, Lagrangian over the root's own nogoods), the size (n, candidates, `max_total`), the cap and the start incumbent against the one-thread proofs of the 143 searched games of 880-1300 (`examples/slope.rs`) | [slope](#what-drives-the-slope-2026-10) | measured | log10 nodes against the Lagrangian gap R² 0.90 (sd 0.35), the matching gap 0.87, n 0.23, the candidates 0.26; with the gap in the fit n adds nothing (t 1.1); the slope in n is ×10 per ~130 of gap times the gap's growth (56 per 100 n, jagged: +304 from 1151-1200 to 1251-1300, flat to 1352) |
| Margin scans: proofs with every bound lowered by δ (the incumbent a value, `P*` + δ) | [slope](#what-drives-the-slope-2026-10) | measured | 19 games of 900-1300: 0.005-0.012 decades fewer nodes per unit of δ, no trend in n; a large game tightened costs what a small one with the same remaining gap costs |
| The root's converged Lagrangian relaxation (its own nogoods, damped subgradient) as every searcher's starting relaxation | [slope](#what-drives-the-slope-2026-10) | not kept (pilot) | proofs −5% (1254) to −76% (1184) nodes, the least at the frontier: a constant factor; the gap that counts is the one at every node |
| Damped Polyak steps in the refit (`fit`) | [slope](#what-drives-the-slope-2026-10) | not kept (pilot) | on the root's nogoods 50 damped steps reach the bound of 50 undamped ones within 8; one refit from zero multipliers stays 50-540 above the converged bound, by its step count |
| A bound whose gap at every node grows more slowly with n, screened by its gaps at the root and at the probe's nodes before it is built | [slope](#what-drives-the-slope-2026-10) | open | the lever on the slope: a gap predicts a proof's nodes (×10 per ~130 of Lagrangian gap, ~270 of matching gap), and the relaxation's error, not the size, is what grows |
| The heaviest common independent set of two GF(2) frame matroids (the even frames' and the odd frames'; every promotable set is independent in both) as a bound on the promotions (`examples/matroid.rs`) | [matroid](#matroid-intersection-bounds-2026-10) | measured; made the search's bound ([GF(2) search](#the-gf2-bound-in-the-search-2026-10)) | root gap 136 on average over 880-1300 against the Lagrangian bound's 269 and the matching bound's 760, growing 4.6 ± 2.4 per 100 n (14 ± 1 over 600-1300, model-free) against 56 and 142; below the root's Lagrangian bound at 346 of 371 roots; cuts 163 and 167 of the probe's 300 open nodes of 936 and 1215 (the matching bound none); 0.3-1.3 ms from scratch |
| A plain branch and bound with the GF(2) bound alone, against the solver's one-thread proofs (`matroid bnb`) | [matroid](#matroid-intersection-bounds-2026-10) | measured | the 146 searched games of 880-1352: 12 (901-950) to 613 (1251-1300) times fewer nodes, log10 nodes +0.14 ± 0.05 per 100 n against +0.48 ± 0.07; 905 s against 3,408 s with the bound from scratch at every node; the matching bound in the same skeleton leaves 880 and 936 unfinished at 200,000 nodes |
| A Lagrangian over the matching with nogoods of the node's own, fitted at each probe node (2,000 steps), against the GF(2) bound | [matroid](#matroid-intersection-bounds-2026-10) | measured | cuts 66 and 50 of 100 nodes (936, 1215) against 55 and 59, not the same ones (either: 69 and 67), at about a second a node |
| Warm-start the intersection from the parent's heaviest set | [matroid](#matroid-intersection-bounds-2026-10) | not kept | extreme at 2 of the 2,543 nodes of 1250 and none of the 1,874 of 1100: deleting the member branched on breaks extremality |
| Start the intersection from the heaviest uncontended candidates | [matroid](#matroid-intersection-bounds-2026-10) | kept (research code) | 20% fewer augmentations, the same values |
| Forced members of the GF(2) bound from a weight split of its heaviest set (Frank) | [matroid](#matroid-intersection-bounds-2026-10) | measured | where the bound does not cut, 26-28 members a node are forced; the split finds 60-66% of them at two greedy runs a member |
| A Lagrangian over nogoods with the GF(2) intersection as its inner problem, at the root | [matroid](#matroid-intersection-bounds-2026-10) | measured | closes 7-36% of the remaining gap at 936-1300; the heaviest set's conflicts have 7-18 members |
| Lorea's hypergraphic matroid per frame, for what the GF(2) bound misses | [matroid](#matroid-intersection-bounds-2026-10) | not kept | sees none of the 14 cores that the root's GF(2) heaviest set runs into at 936-1300 (each subset lists more factors than movers) |
| The GF(2) bound as the search's node bound (tactic `gf2`): the frame matroids carried by contraction along the path, conflicts of its heaviest set, forced members from the weight split, exact strong branching | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | kept, the default | the 146 one-thread proofs of the slope study 45 times faster (76 s against 3,408 s), 426 times fewer nodes; log10 nodes +0.18 per 100 n against +0.48, seconds +0.24 against +0.48; the chain 1001-1300 on 2 threads 60 s against `strict.jsonl`'s 988 s on 4 |
| Keep each level's dominant sets (every set an augmentation passes through) and start a child, or the node without a member, from the last one the changes keep; a child whose parent's heaviest set survives inherits it | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | kept | replaces the warm start from the parent's heaviest set, which deletion breaks (row above); a strong-branching solve still makes 22-32 augmentations, from the uncontended prefix |
| A tableau of bitset rows per block, updated by one pivot per exchange, instead of spans rebuilt per element; lists of members and outside elements kept along each path; Bellman–Ford pulling only from changed members | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | kept | the bound from scratch 32% fewer cycles at 1215's probe nodes, the split 40%; 2.4-2.5 Bellman–Ford rounds an augmentation |
| Branchless pivots (the row selected by a mask from the pivot bit) | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | kept | 11% less time at 1254 and 1300 |
| A greedy lower bound (the heaviest set less the member, completed greedily) before each strong-branching solve | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | kept | 10% less time at 1254 |
| The GF(2) bound's branchings: by the split's bound alone, the largest member, exact solves only to find the forced members | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | not kept (`Gf2Branching` knobs) | against exact strong branching, 2.9, 3.6 and 1.8 times the nodes and 2.0, 2.4 and 1.5 times the cycles (geometric means of 7 proofs, 936-1288) |
| Strong branching with GF(2) solving at most one or three members, or stopping at the first forced member | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | not kept (knobs) | 1.67, 1.21 and 1.05 times the cycles |
| The matching bound as a first check before the GF(2) bound | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | not kept (knob) | the same trees, 1-4% more cycles |
| The Lagrangian bound over nogoods after the GF(2) bound (`lagrange` with `gf2`) | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | not kept: `lagrange` off by default | no tree changes but by one to three nodes at 992 and 1025, 0-7% more cycles |
| `nogoods`, `fix_forced` and `forget_nogoods` with the GF(2) bound | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | kept | without nogoods 1.22 times the cycles, without forced members 2.76; forgetting changes nothing below 50,000 steps a searcher |
| Cancel the negative cycles of the optimum less the member, to start a strong-branching solve next to its answer | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | open (untried) | strong branching makes 70% of the augmentations, each solve 22-32 of them |
| The chain past 1300 with the GF(2) search (`solve 1301-4095 --use-prev`, 2 threads), the cost curve and the reach of a 192-core machine | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | measured | reached 2175 in 92 minutes of game time (875 games); cost in waves that follow the root's GF(2) gap, no growth a fit resolves (−0.012 ± 0.033 decades per 100 n); a day on 192 cores reaches 4095 at every slope the data support, about 3200 at the steepest GF(2) slope measured |
| The frame-model search with the GF(2) bound against the model-free search with the parity bound (`evidence` crate, `exact`) on 990, 1232 and 1300 | [GF(2) search](#the-gf2-bound-in-the-search-2026-10) | measured | 24-36 times fewer cycles; trees within a factor of two, a node 15-17 times cheaper (contracted frame matroids kept by pivots) |
| The cover LP over all conflicts at the root over the matching (exact separation: the best any conflicts and multipliers give a Lagrangian bound over the matching) | [nogoods](#how-far-can-nogoods-go-2026-10) | measured | 0.2-11.6 below the slope study's root Lagrangian band by band (mean gap 264 against 269 over 880-1300), growing +54.7 per 100 in n against +56.0: the root's own conflicts were already all there is; it closes 67% of the matching gap, a share that does not grow |
| Every minimal conflict of one to three members at the root (enumerated frame by frame) | [nogoods](#how-far-can-nogoods-go-2026-10) | not kept | close 24-37% of the matching gap against 57-66% for all conflicts, and nothing over GF(2); the binding conflicts are large |
| The search's refits against the LP over the same nogoods, at the root | [nogoods](#how-far-can-nogoods-go-2026-10) | measured | warm-started, the 50 steps end 7.8 above that LP on average (1215, 23 refits): good multipliers at the root; the slope study's 50-540 was a refit from zero |
| The search's Lagrangian bound at its own open checks against the best its nogoods allow there (traces of 936 and 1215) | [nogoods](#how-far-can-nogoods-go-2026-10) | measured | the LP over the nogoods it holds would cut 74% and 67% of the checks it leaves open, all conflicts 97% and 95%; refitting its relaxation's nogoods alone 5% and 24% (1 and 9 of them live at a typical node): its multipliers are the root's |
| An extra cut by the cover LP over all conflicts at every open check of the search (matching inner, instrumented build) | [nogoods](#how-far-can-nogoods-go-2026-10) | not kept | 5-31 times fewer steps on 9 games of 880-1184 (1000 and 1196 unfinished, under 14x), log-log slope 0.81 ± 0.14, the advantage shrinking as δ → 0 in margin scans: a constant factor; 5-40 ms a check, 1,000-5,000 steps' worth |
| Cheap node-level forms: the LP over the nogoods held, refits from the search's multipliers (5-50 steps) with or without conflicts added (relax-and-cut), per-subtree multipliers (an LP fit every 4-16 promotions), the ideal only at shallow nodes | [nogoods](#how-far-can-nogoods-go-2026-10) | not kept | 2.3-7 times fewer steps at 880, 936 and 1184 with their forced members, for 80-550 steps' cost a check (the per-subtree fits 25-70): 11-100 times slower; at nodes of at most 16 promotions the ideal saves 5% |
| A plain branch and bound with the cover LP over all conflicts at every node (matching inner, most fractional branching) | [nogoods](#how-far-can-nogoods-go-2026-10) | measured | 10-63 times fewer nodes than the solver's steps at 880-1184, log-log slope 1.15 ± 0.17 against them |
| The cover LP over all conflicts with the GF(2) intersection as the inner problem, at the root and the probe's nodes | [nogoods](#how-far-can-nogoods-go-2026-10) | measured | closes 46% of the root's GF(2) gap, a share falling 3.8 points per 100 in n (the residual grows +8.4 against GF(2)'s +4.6); cuts 84% and 62% of the probe's nodes GF(2) leaves open (12 and 46 ms a node); relax-and-cut with 3-30 intersections 19-63% |
| A branch and bound with GF(2) and the cover LP over all conflicts against the GF(2) branch and bound (146 games) | [nogoods](#how-far-can-nogoods-go-2026-10) | not kept | 6.2 times fewer nodes (geometric mean), less as n grows (−0.046 decades per 100 in n), the same 0.011 decades per unit of root GF(2) gap: a constant factor; 24% slower computed from scratch (1,125 s against 905 s) |
| Dominance by exchange at search nodes: one excluded candidate added, one for one, two for one or two, judged by a cover of the node's state by the exchange's (literal, with moves relabelled) and by sampled maximal completions (`examples/dominance.rs nodes`) | [dominance](#dominance-and-symmetry-2026-10) | measured | proved at 0-1% of the probe's nodes and 0-5% of nodes sampled from the solver's proofs; sampled estimates up to 53%, nearly all of it prime exchanges (next rows); an excluded candidate can never be added for free (some maximal completion always blocks it) |
| Twins (candidates with the same live factors and users) and transpositions (one reduced state reached by two promotion sets) | [dominance](#dominance-and-symmetry-2026-10) | measured: none | no twins at any sampled node; no transposition in the 133,006 nodes of six GF(2) branch-and-bound trees (930-1300) |
| Symmetries of the frame model: exchanging two primes whose classes the frame builder treats alike maps promotable sets to promotable sets worth `(q − p)(c_p − c_q)` more (`examples/common/symmetry.rs`, `dominance symmetries`, `symcheck`) | [dominance](#dominance-and-symmetry-2026-10) | kept (the finding; research code) | 3.8-5.9 classes of primes a game at 851-1350, moving 11-14% of the candidates, the group's order 10^4.5 to 10^7.4; checked for every game 2-1300 (images, sums, 22 small boards enumerated, the oracle to 40); IDEAS.md's "no symmetries found" was wrong |
| Breaking the prime symmetries (only sets with `c_p ≤ c_q` searched): hopeless test, the constraint's matching bound in a copy of the search, the constraint's Lagrangian on the GF(2) bound in a copy of `matroid bnb` (pilots, code not kept) | [dominance](#dominance-and-symmetry-2026-10) | not kept: a constant factor | nodes ×0.96 and ×0.88 (solver, 22 games of 880-1090; time +15% and ×1.9; 1184 +27%), ×0.95 and ×0.86 (GF(2) B&B, 43 games of 880-1300; time −3%); the share holds with n and over margin scans (`c` about 3% lower); every game 2-900 still finds its optimum from `P*` − 1 |
| The anatomy of the root's gap: the matching and GF(2) bounds' heaviest sets (ties towards the optimum) against the optimum, frame by frame, with their cores | [anatomy](#the-anatomy-of-the-gap-2026-10) | measured | both promote exactly as many as the optimum at all 361 roots of 880-1300, the wrong ones (swaps: band means 9-15 and 3.5-8); the matching bound's excess is in homes 1-2 over mostly even cores of frames 0-2; the GF(2) bound's over odd cores of frames 1-3 (never 0), whose size and promotions grow with n (conflict 12.9 members, +1.1 per 100 n; 16 to 26 promotions inside) |
| The matching bound as an LP with separated cover cuts (the root's conflicts) | [anatomy](#the-anatomy-of-the-gap-2026-10) | measured | within 27 of the slope study's converged root Lagrangian at every band (114-472 against 112-471) in tens of ms; its solutions are half-integral over overlapping conflicts (odd cycles of pairwise conflicts) |
| Rank cuts `Σ_T z ≤ r` on unions of tight cuts, the rank proved by branching on conflicts, on the matching LP | [anatomy](#the-anatomy-of-the-gap-2026-10) | measured; superseded as a bound by the GF(2) bound | root gap 81 on average (3-15% of the matching gap; better than the GF(2) bound below 1200, equal above), growing 26 ± 3 per 100 n; cuts 28 of 30 probe nodes at 936 and 1215 against 23 and 17 for cover cuts |
| Cover and rank cuts on top of the GF(2) bound (the LP over the frame matroids' common independent sets, by column generation) | [anatomy](#the-anatomy-of-the-gap-2026-10) | measured | root gap 136 to 47 on average (cover cuts alone 84 here, 46% of the gap by exact separation in the nogoods study; the rank cuts the rest), but the residual grows 13.9 ± 1.7 per 100 n (91-92% of the GF(2) gap closed at 851-950, 46% at 1201-1300); unions of 48 members and 20,000 conflict computations a check add almost nothing (1215 by 6, 1300 by 0) |
| The root's cut pool at nodes: at the root's multipliers (one intersection) or the node's (column generation), in the plain GF(2) branch and bound | [anatomy](#the-anatomy-of-the-gap-2026-10) | not kept: a constant factor | 19 games: node's multipliers 4.2 times fewer nodes (1.7-33) in 1.9 times the time, root's 1.26 times in 1.3 times; no change in the slope; the root's gap reduction overpredicts (992: 90 predicted, 2.4 measured) |
| Counting cuts on a frame's movers and factors (the movers whose live factors lie in a set `U`: one fewer than `U` has factors when each has two live factors or more, `d₁ + d₂ − 3` fewer by the first two movers played) | [anatomy](#the-anatomy-of-the-gap-2026-10) | not built | valid, and they cut square cores; a rank cut on the same candidates is at least as strong, so the rank cuts measured bound them |
| The per-frame bound: the frames coupled by Lagrangian multipliers on the candidates, each frame's heaviest winnable configuration found exactly (frames 1-3; GF(2) in the others), the multipliers by subgradient steps from the GF(2) weight split | [per-frame](#the-per-frame-bound-2026-10) | measured: valid and strong where computable, not kept | 19 roots, 200 steps: closes 15-100% of the GF(2) gap (mean 59%, −5 ± 4 points per 100 n; 15-39% at 1254-1300), 13-351 s a root; at the probe's nodes it cuts 25 and 23 of 30 at the root's multipliers (GF(2) 20 and 14) at 0.1-0.5 s a node, 29 and 27 with 20 steps of the node's own at 0.7-2.1 s |
| The per-frame bound with column generation (the master's dual, box steps) instead of subgradient steps | [per-frame](#the-per-frame-bound-2026-10) | not kept | without the box the master's multipliers swing to extremes (`L` of 10^5 to 3·10^6) and nothing improves in 100 steps; with it, 936 reaches 20528 in 400 steps (410 s) against the subgradient's 20520 in 200 (142 s) |
| Frame 0 solved exactly in the per-frame bound | [per-frame](#the-per-frame-bound-2026-10) | not kept | its search hits 20,000 nodes at most multipliers past 1000, and the GF(2) bound's heaviest set has no core there |
| The per-frame bound at the root's multipliers in the plain GF(2) branch and bound | [per-frame](#the-per-frame-bound-2026-10) | not kept | 13 of 19 games finish in 240 s: 2.8 times fewer nodes (geometric; 740 at 900, whose root it cuts, else 1.0-7.3) in 28 times the time, behind the root's cuts at the node's multipliers at 11 of the 13 |

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
| Reuse the previous game's promotions without a search (`reuse_prev`) | [tactics review](#reuse_prev-nothing-left-to-save) | removed 2026-10 (it was unproven, and the cap and same search had left it nothing to save) | with several optimal sets of n − 1 the result depends on which was recorded: the recorded game of 685 plus 686 is unplayable though opt(686) = 686 + opt(685) |
| The sacrifice bound (games on punctured boards) as a proof for `reuse_prev` | [`src/sacrifice.rs`](../src/sacrifice.rs) | not kept | tight on 57 of the 152 `reuse_prev` games, and where it proves it costs 1.1-8.7 times the direct search; the theorem stays and `tests/wide.rs` uses it |
| Bounds on opt(n) from the games above: opt(n) ≤ opt(n + K) − Σ_{even m ∈ (n, n+K]} m/2 − (q − P), proven | [going higher 1](#going-higher-and-coming-back-down-2026-10) | measured; not kept | holds on 2..1299; tight only through a prime n + 1 (whose search problem is n's: 204 of 204) and at 13 even m (2^k, 3·2^k); settles 1023 alone of the 416 exhausted games beyond what the prime formula gives |
| Caps from neighbouring search problems that relax n's (a proven sufficient condition for one problem to relax another) | [going higher 2-3](#going-higher-and-coming-back-down-2026-10) | measured; not kept | 2708 relaxing pairs within 3, no violation; exact for 5 exhausted games (30, 45, 510, 765, 1023), a top-frame change; a relaxation's root bound is never smaller (972 pairs, GF(2) and matching), so an exact cap comes from a problem at least as hard and a cheaper one leaves a cap that misses (1235: 12 against a GF(2) gap of 164) and prunes nothing; from below never tighter than the chain's cap (70 games) |
| Incumbents from the games above: n + 1's optimal set on n's board; the look-ahead start of n + 1..n + 3 seeded from n − 1 | [going higher 4](#going-higher-and-coming-back-down-2026-10) | not kept | the optimal set reaches P\* in 91 of the 110 exhausted games that start below it, but rests on n's optimum; the look-ahead beats n's start in 7 and reaches P\* in 1 (1290) |
| A chain that comes back down: solve n + 1 first and derive n | [going higher 7](#going-higher-and-coming-back-down-2026-10) | not kept | in 1001-1300 only 1023 (from 1024, whose proof costs the same from 6 below): 0.55% of the nodes of the exhausted searches |
| Equivalent search problems (same feasible sets, not equal) as a wider same search: strip the movers that a peel of each frame's hardest state drops, then a checked isomorphism fixing the candidates | [going higher 6](#going-higher-and-coming-back-down-2026-10) | not kept | 8 games beyond the 545 repeats, all below 186 (18 nodes); near repeats with the same P\* and candidates are not isomorphic and cost 643k nodes over 1001-1300 at most |
| Proving a run of neighbours that share a core (equal GF(2) root gaps) in one search | [going higher 5](#going-higher-and-coming-back-down-2026-10) | open | every chain step after an exhausted game keeps the GF(2) gap (37 of 37); runs with equal gaps hold 19.8% of the proof nodes of 880-1300 beyond each run's dearest; trees within a run differ by up to ×3.9; not piloted |
| Lower frame 0's freedoms by solving the two-frame game (`revise_freedoms`) | [tactics review](#revise_freedoms-the-same-trees) | removed 2026-10: its premise is false | assumes the max-sum set also has the max count, which fails on about one perturbed game in 1,300, and for the whole frame game of n = 833 (48 promotions optimal, 49 feasible: [evidence 6](../evidence/README.md#6-does-an-optimal-game-play-the-most-moves)); the two-frame form it applies is undecided |
| What `reuse_prev` still saves in a strict chain that has `same_search`, the cap and the seed (from the setups, no search) | [tactics review](#reuse_prev-nothing-left-to-save) | measured; removed | of 2..1300 it accepts 670 games, every one settled anyway (545 repeats, 115 caps at once, 10 searches of at most 1.6 s): 3.8 s of the chain's 1052 s of search, 0.5 s of 813 s in 1232..1300; no score or move differs |
| What `revise_freedoms` changes in the proofs at the frontier | [tactics review](#revise_freedoms-the-same-trees) | measured; removed | it changes the problem of every searched game of 1232..1300 (frame 0's freedoms down 4 to 6, `max_total` 0 to 5), yet the proofs of 1215, 1225, 1240, 1254, 1272, 1287 and 1300 have the same trees, node for node |
| `preclusions`, `match_bound=off`, `legacy_bound` and the branchings other than `conflict-strong`, again with the Lagrangian bound | [tactics review](#the-alternatives-that-are-off) | measured; removed | the proof of 1215 on one thread: `preclusions` +80% cycles and 4.5 s for its table, `conflict-largest` ×6.3, `largest`, `conflict-smallest` and `match_bound=off` unfinished at 180 s (more than ×17); `legacy_bound` acts only without the matching bound |
| `match_refine=0` with the Lagrangian bound | [tactics review](#the-alternatives-that-are-off) | open | the proof of 1215: cycles −12%, nodes +1%; its default needs the bench set and the small games |
| Support for old file formats: result schemas 1 to 3, the committed schema-1 files, checkpoint compatibility, the golden tests of the Java's tactics | [tactics review](#old-file-formats) | removed | with the tactics: results schemas 1 to 4 (schema 5 only is read), `results/default.jsonl`, checkpoint headers before version 4 and the width widening; the Java's golden data for the frame builder stays |
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
| The model-free search past 989: games 990, 992, 993, 995, 996 and from 1020 | [parity bound](#the-parity-bound-in-the-model-free-proofs-2026-10) | kept: done with the parity bound | with the matching bound each needed tens of millions of nodes (its gap about 1,170 at n = 1000); the parity bound finished the saved searches in 4,000-215,000 more nodes each and proved every game to 1300 |
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
| A model-free bound on opt(n): every playable set of the board is independent in two linear matroids over GF(2) (split by the parity of Ω, with the unit vectors of the members), so their heaviest common independent set bounds opt(n) (`matroid whole`) | [matroid 6](#matroid-intersection-bounds-2026-10) | measured; kept in the evidence crate (next row) | gap 0-3 on average to 300, 15-90 to 900, 108-167 by band to 1300 (exact at 194 games), a tenth of the model-free matching bound's; equals the frame model's GF(2) bound plus `in_the_bag` at all 361 roots of 880-1300; 0.5 s at 1300 |
| The parity bound in the evidence crate's model-free search: reimplemented from its mathematics, proven as THEORY.md's Theorem 10, every value certified by a weight split (`evidence/src/parity.rs`) | [parity bound](#the-parity-bound-in-the-model-free-proofs-2026-10) | kept | every game 2..1300 proven model-free (twice); 990 in 44,389 nodes from scratch where the matching bound had spent 70 million; the root value equals the study's at all 1,088 games it measured; 22 ms a root solve at 990, 43 ms at 1300 |
| Warm starts of the parity bound from the parent node's certificate, improving cycles through the root found by Dijkstra on the inherited potentials | [parity bound](#the-parity-bound-in-the-model-free-proofs-2026-10) | superseded by the next row | 36% fell back to a solve from scratch at 990 (the parent's set minus a barred member is not the heaviest of its size, so an exchange can break the potentials); 108 s against 122 s cold |
| The same with label-correcting searches (rescans allowed, at most 20 a vertex on average) | [parity bound](#the-parity-bound-in-the-model-free-proofs-2026-10) | kept | 2-12% fall back; the same trees twice as fast (990: 61 s against 122 s; 1250: 4.6 s against 13.6 s) |
| The Lagrangian bound over the nogoods beside the parity bound | [parity bound](#the-parity-bound-in-the-model-free-proofs-2026-10) | not kept | cut nothing at 990: the same 44,411 nodes, 7% slower |
| Resuming the matching bound's open searches with the parity bound, against starting afresh | [parity bound](#the-parity-bound-in-the-model-free-proofs-2026-10) | measured | both finish; resuming costs more (990: 89,639 more nodes against 44,389 from scratch; 992: 214,329 against 66,892) |
| Does an optimal game play the most moves, M(n)?  (The wiki's open question; in its whole-game form, the conjecture behind `revise_freedoms`.)  M(n) model-free from the matching number ν(n) and a unit-weight search, against the optimal game's moves, n = 2..1300 | [evidence 6](../evidence/README.md#6-does-an-optimal-game-play-the-most-moves); [THEORY: longest games](../../heuristics/THEORY.md#longest-games) | not kept: false | at n = 833, 834 and 835 every optimal game is one move short of the longest (the heaviest 363-move game of 833 scores 29 less); every other game to 1300 is a longest game, 1,293 by ν(n) alone; pruning branches that cannot reach M(n) moves would be unsound |
| How the parity bound relates to Franklín and Moniot's Theorem 1 and bound | [THEORY: relation to Franklín and Moniot](../../heuristics/THEORY.md#relation-to-franklín-and-moniot) | kept (proven; evaluation/fm_relation.py) | playable / independent / matchable iff exactly one / an odd number / at least one payment scheme onto some payment set (Theorem 12); no odd core below board 45, so the parity bound is exact to 44; checked on every pick set of 2..22 and 18,200 dense sets on 30-120 |

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
| [The port](#the-port-and-its-first-improvements) | does a fresh implementation of the Java's algorithm reproduce its answers? | yes, and the Java's upper bound had a defect (the runner-up), fixed (`legacy_bound` reproduced it until it was removed) | before 2026-10 |
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
| [Which tactics earn their keep](#which-tactics-earn-their-keep-2026-10) | which tactics pay in a strict chain at the frontier, and what support for old file formats can go? | removed `reuse_prev`, `revise_freedoms`, `legacy_bound`, `preclusions`, `match_bound=off` and the branchings other than `conflict-strong`, and with them strict runs and the old file formats; kept the proven switches | the solver of 243af9b |
| [Faster nodes with the Lagrangian bound](#faster-nodes-with-the-lagrangian-bound-same-trees-2026-10) | where do the cycles go with the Lagrangian bound, and what can be cheaper without changing any tree?  Does `match_refine` = 0 now pay? | kept: the probe's windows about 10.5% fewer cycles, proofs 5-10%; `match_refine` stays 1 | dd4f89c to 31b9c95 |
| [Benchmarks](#benchmarks-the-probe-pgo-and-scaling) | what the bench set, the probe, PGO and the thread scaling showed | numbers | e3b8ddc and after |
| [What drives the slope](#what-drives-the-slope-2026-10) | what makes a game's cost grow ×10 per 55-70 in n, and what would change the slope? | the root's gap: ×10 nodes per ~130 of it at every n, nothing from the size at a given gap; the slope is the gap's growth; seeding the search with a better root relaxation is a constant factor | db7faf1 (no solver change) |
| [The committed chain, made again](#the-committed-chain-made-again-2026-10) | does the cleanup's build make the same chain? | yes: the same moves, proofs and scores as the chain of 243af9b; 1022.5 s of search | 271c817 |
| [Matroid intersection bounds](#matroid-intersection-bounds-2026-10) | does the GF(2) independence of playable sets give a bound whose gap grows more slowly? | yes: the intersection of the even and odd frames' GF(2) matroids closes 82% of the matching gap at the root, more as n grows; a plain branch and bound with it needs 12-800 times fewer nodes than the solver, growing 0.14 decades per 100 n instead of 0.48; recommended for the search; model-free, a tenth of the matching bound's gap on opt(n) | 67bb492 to 12a0f60 and the entry (no solver change) |
| [How far can nogoods go](#how-far-can-nogoods-go-2026-10) | what is the best a Lagrangian bound over nogoods can do (all conflicts, ideal multipliers), at the root and at nodes, over the matching and over the GF(2) intersection, and is the gap it leaves still growing? | a constant factor: at the root over the matching it is the Lagrangian bound already (its gap growing as fast); at every node about 10x fewer steps at 1,000x the cost; over GF(2) 46% of the root gap and 6x smaller trees, less as n grows, in a quarter more time; superseded by GF(2) | the entry (no solver change; the study's code was not kept) |
| [Dominance and symmetry](#dominance-and-symmetry-2026-10) | is part of the tree dominated, or explored twice under interchangeable promotions, and does a sound rule's share grow with n? | no transpositions or twins; the frame model has prime-exchange symmetries, and breaking them removes a seventh of the nodes on average (up to 43%), a constant share: not a slope lever | aa56590 to d2de73f (no solver change) |
| [Going higher and coming back down](#going-higher-and-coming-back-down-2026-10) | can the games above n bound, seed or settle it, so that a dear game comes from a cheaper one above? | no: the games above bound n through relaxations, whose root bounds are never smaller, so an exact cap comes from a problem at least as hard; exact only for 1023 in 1001-1300 (0.55%); incumbents from above are n's optimum reflected; neighbours share their GF(2) gap | the entry (no solver change; the study's code was not kept) |
| [The parity bound in the model-free proofs](#the-parity-bound-in-the-model-free-proofs-2026-10) | does the parity matroids' bound, reimplemented in the evidence crate and proven as a theorem, carry the model-free proofs of opt(n) from 989 to 1300? | yes: every game 2..1300 proven model-free (twice), the old open searches finished, a fresh proof of all 557 searched games in 1.2 thread-hours; cost ×10 per 84 points of the root's parity gap, about ×1.7 per 100 in n | ff714b3 to the entry (evidence crate only) |
| [The anatomy of the gap](#the-anatomy-of-the-gap-2026-10) | where does the root's gap live, which rule do the relaxations break, and which valid inequalities cut it? | the relaxations promote as many as the optimum, the wrong ones; above the GF(2) bound only odd cores of frames 1-3 remain, growing with n; cover and rank cuts on top of it are a constant factor (about 4 in nodes with multipliers refitted at each node), not a lower slope | the entry (no solver change; the study's code was not kept) |
| [The per-frame bound](#the-per-frame-bound-2026-10) | does a bound exact frame by frame (frames tied by Lagrangian multipliers) close a share of the GF(2) residual that holds with n? | where it can be computed it closes most of it (90-100% at 900, 936, 1088; 29 and 27 of 30 open probe nodes cut with node-level steps), but each evaluation solves the frames' own odd-core searches: within 200 steps 59% on average, falling with n (15-39% at 1254-1300); in the branch and bound 2.8 times fewer nodes at 28 times the time; not recommended | the entry (no solver change; the study's code was not kept) |
| [The GF(2) bound in the search](#the-gf2-bound-in-the-search-2026-10) | can the search use the GF(2) intersection as its node bound at a cost a node that pays, what machinery earns its keep with it, and what does it do to the chain? | kept, the default: the 146 one-thread proofs 45 times faster (426 times fewer nodes); the chain 1001-1300 16 times faster on half the threads; a chain from 1300 reached 2175 in 92 minutes on 2 threads, with no growth a fit resolves; `lagrange` off | 679635b to the entry |
| [The committed chain to 2175](#the-committed-chain-to-2175-2026-10) | does the build with the GF(2) bound make the committed chain from n = 1 as far as the chain from 1300 reached, in one invocation? | yes: 1..2175 in 1638.6 s on 8 threads, nothing assumed; every score and proof kind of 1..1300 is the previous chain's, every score of 1301..2175 the independent chain's from 1300 | feba6d0 |

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
  833 188, 969 175, 984 119.  The strict chain of commit 243af9b (2026-10-13, nothing unproven)
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

*How it was measured.*  The solver code of commit 243af9b, which made `results/strict.jsonl` as it was then (unchanged
to 72601a2).  Searches are the ladder's proof-only ones on one thread (`ladder --threads 1 --seed-opt --known
results/strict.jsonl --seconds S [--tactic …] N`), whose node counts are exact, with the process's cycles from
`/usr/bin/time -l`, two at a time on the laptop while another agent measured cycles on it, so cycles are good to a few
percent.  What `reuse_prev` and `revise_freedoms` do was worked out from setups alone, with no search, by a scratch
example (not kept):

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

*Decision.*  The owner took the recommendation and went further; the removals are commits f72c332, 8a6df90,
49ec778 and a2bc798 (on 4372b36 by the merge 39ed18b), about 4,800 lines of code removed and 1,650 added (`src/`,
`tests/`, `examples/`, `tools/`, `evidence/` and the study scripts; 3,600 and 990 of them in `src/`):

* *Tactics.*  `reuse_prev`, `revise_freedoms` (premise refuted at n = 833), `legacy_bound`, `preclusions`,
  `match_bound=off` with `blameGame` and the counters only they used, and the tactic `branching` with its three
  losing values are gone, with their tests, tripwire rows and fixtures (`testdata/accel_java.txt`).  The nine proven or
  sound tactics of the table stay as switches.
* *No strict runs.*  With no unproven tactic left there is one kind of run: `--strict`, `UNPROVEN`, `Tactics::strict`,
  `unproven_tactics_on` and the rules of a game's own taint are gone, and so are `STRICT_ARGS` in `tools/cloud`.
* *Assumptions instead of taint.*  A range takes the game of n − 1 from the run or from the `--out` file and otherwise
  solves its first game standalone; `--use-prev` takes it from optimal.json.  A result whose proof rests on such a game
  records `"assumes": ["the game of 1300 in optimal.json is optimal"]` (say), carried down the chain as taint was, and
  `[]` otherwise.  Nothing refuses a game for what it assumes (`--allow-tainted-prev` is gone), and `extend-optimal`
  appends such games: they assume nothing the table does not stand for.
* *optimal.json is built in.*  The binary carries the table it was compiled with (`include_str!`; a rebuild picks up a
  changed file) and confirms every score against it, a difference being a hard error naming the game: `--optimal FILE`,
  `--optimal none`, `TAXMAN_OPTIMAL` and `allow_optimal_mismatch` are gone.  Tests that need another table use the
  library (`solver::solve_range_each_with`, `results::read_results`); `verify-optimal [FILE]` and `extend-optimal
  RESULTS --out FILE [--table FILE]` still take files.
* *Old formats.*  Results lines are schema 5 (`assumes`, `prev.assumes`, `same_search.assumes`, `options.use_prev`; no
  `prev_source`, `setup.revise`, `seconds.revise` or `seconds.preclusions`, `unproven_tactics_on`, `optimal_file` or
  `allow_optimal_mismatch`), and both verifiers read schema 5 only; `results/default.jsonl` and
  `testdata/strict_schema3_1_90.jsonl` are deleted (`git show f72c332:rust/results/default.jsonl`), and
  `bench/curve/chain.jsonl` stays as the cost-curve study's data, which no verifier reads.  The checkpoint header is
  version 4 (no frame 0, the tactics recorded plainly), every other version is refused with one message, and the
  widening to an older build's width is gone; `testdata/n1133_after_1000_nodes.ckpt` (now W = 20) and the probe's
  windows were made again, the probe's fingerprint now 7e8e724717d26545 (5b83bbebf0bb2899 with `lagrange=off`).
* *Left for the owner.*  `results/strict.jsonl` is schema 4, so until it is made again by this build (the chain from
  n = 1, about 18 minutes on four threads) neither verifier reads it: `tests/results.rs`'s
  `the_committed_results_pass_the_verifier` skips it, saying so, and `tools/cloud/verify.sh`, `setup.sh`'s verifier
  smoke check, `tools/profile/profile.sh` and the README's commands that extend a copy of it wait for the rerun.

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
of 331eb09 with `--tactic revise_freedoms=off` (the strict tactics; the tactic has been removed since, and the
commands below, which no longer pass it, run the same searches), one thread a process and at most two processes, on
the laptop while another agent ran the test suite.  `P*` is a game's optimum promotion sum (its score minus
`in_the_bag`).

* *Roots* (`slope roots --known results/strict.jsonl --sums bench/curve/bands.jsonl 880-1300` and the games past 1300
  that `bands.jsonl` solved; about a second a game): the root's matching bound; the matching bound refined one to three
  levels deep (the search's refinement, computed to a value); and a Lagrangian bound over nogoods of the root, made
  without a search: the conflicts the refinement found, then up to 8,000 subgradient steps towards `P* + 1` (Polyak's
  rule damped by halving after 80 steps without a lower bound), each adding up to 12 new conflicts of the weighted
  matching's candidates; it settles on 140-840 nogoods (band means).  The *matching*, *refined* and *Lagrangian gaps*
  are these bounds minus `P*`.  Also the chain's start incumbent and cap, and the candidates each bound leaves undecided
  with the incumbent at the optimum.
* *Proofs* (`slope prove --known results/strict.jsonl --deltas ... N`, with `--sums` as above past 1300): the search of
  the game on one thread, its incumbent a value `P* + δ` with no set behind it, no seed, no cap.  With δ = 0 it is the
  proof of the optimum (the `--seed-opt` ladder's node counts: 29,614 for 1100, 19,473,016 for 1290 and 28,024,916 for
  1300, as in ["A Lagrangian bound over the nogoods"](#a-lagrangian-bound-over-the-nogoods-2026-10)), for all 143 games
  of 880-1300 that `strict.jsonl` searched to exhaustion (the others are repeats, caps and primes) and for 1309, 1350
  and 1352.  With δ > 0 nothing beats the incumbent, and every cut is the cut the search would make with every bound it
  uses (matching, refined, Lagrangian, forced members) lowered by δ: a *margin scan* over δ = 25 to 800 measures how the
  tree shrinks as all the bounds tighten uniformly, for 19 games of 900-1300, and at δ = 100-300 for 1302, 1305, 1309,
  1350 and 1352.  With δ < 0 the search starts below the optimum.  (`--diag` runs, whose lines carry `steps_by_depth`,
  have the same trees and slower times.)
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

## The committed chain, made again (2026-10)

`results/strict.jsonl` was replaced once more, by a run of the clean build of commit 271c817, the cleanup of "Which
tactics earn their keep" (every run strict, schema 5): `taxman solve 1-1300 --threads 4 --out FILE --checkpoint-dir DIR
--checkpoint-interval 60 --progress 0` from n = 1, in one invocation, on the laptop while four other jobs of two
threads each ran: 1048.7 s of wall time and 3965 s of CPU.  Search seconds 46.3, 167.9 and 808.4 for 1..1000,
1001..1231 and 1232..1300 (1022.5 in all), against 51.3, 187.8 and 813.2 (1052.2) of the chain of 243af9b.  Every
score equals optimal.json's, and every move list, proof and method is that chain's (same_search 545, exhausted 416,
prime formula 211, cap 115, trivial 13), with no game assuming anything; the move sets are optimal.json's but at
684..687 ([results](../results/README.md#strictjsonl) has the rest).  The node counts of the other 1299 games total
279.0 M against 280.1 M before; 1232 took 63% more nodes (53.0 M against 32.6 M) at a higher rate (256 k/s against
178 k/s, under another load), so its 206.7 s against 183.6 s are the extra nodes.  This is the rerun that "Which tactics
earn their keep" left for the owner: `tests/results.rs` no longer skips the file.  Entries above that quote
`results/strict.jsonl` from the first replacement on mean the chain of 243af9b.

## Matroid intersection bounds (2026-10)

*Question.*  The slope of the cost curve is the growth of the root's gap ([slope](#what-drives-the-slope-2026-10)),
so a bound whose gap grows more slowly changes it.  The matching bound relaxes winnability to distinct factors.
THEORY.md's Theorem 7 shows that playable sets are linearly independent over GF(2), which made the upper half exact.
Does that structure give a bound on the promotions, how much of the gap does it close at the root and at search
nodes, does the share hold with n, and what does it cost?  Are there other matroids the playable sets live in?

*The bound* (the proof is the module comment of `examples/common/meet.rs`).  Give each frame `h` the space over
GF(2) whose coordinates are its factors `F_h`, each mover `u` paying in it (a move of `h`, or a candidate of home
`h + 1`) the vector `v_h(u)` of its maximal factors in `F_h`, and each candidate `y` of home `h` the unit vector
`e_y`.  If `Q` can be promoted, then in every frame the vectors of the moves, of `Q`'s members promoted into it and
the unit vectors of `Q`'s members promoted out of it are independent: a winnable frame peels completely, each
dropped mover holding a live factor that no mover still there has, so the movers restricted to the live factors
`F_h − Q` are independent (THEORY.md's private-factor argument), and restricting to `F_h − Q` is the same as
adding the unit vectors of what `Q` took out.  So each frame is a linear matroid on the candidates that touch it,
the moves contracted (each candidate's vector reduced modulo the moves' span; its rank is the frame's freedoms).  A
candidate touches its home frame and the one below, one of even index and one of odd, so the even frames' matroids
make one matroid `A` on the candidates and the odd frames' another, `B`.  Every promotable set is independent in
both, and the heaviest common independent set (weighted matroid intersection, polynomial) bounds the best
completion; the cap `max_add` is a size limit at no cost.  As a constraint on the promotion set given the rest of
the game: the game's other moves are fixed, and `Q` must be independent in the contraction by them, frame by frame,
counting what `Q` takes out of each frame as well as what it adds.

It is never above the matching bound: independent vectors have distinct representatives (a nonzero maximal minor
has a nonzero term), which in each frame is the matching bound's assignment, each unit vector taking its own
coordinate (no promoted number pays as a factor).  On movers with at most two live factors (graph edges)
independence over GF(2) is being a forest, which is winnability itself, so the bound misses only the *odd* cores
of movers with three or more factors: self-covering sets no subset of which lists every factor an even number of
times.  It is not one matroid, so THEORY.md's remark (no matroid is exact for the whole game) does not touch it.

The same construction bounds the whole game with no frame model (`examples/matroid.rs`, `whole`, whose doc comment
has the proof): every number `s` of `2..=n` gets `v(s)` and, if `s ≤ n/2`, `e_s`; a playable set's vectors are
independent (each move pays with a maximal factor still in the pot, which no earlier move lists and which is not a
member); `v(s)` lies in the coordinates of the other parity of Ω than `s`, `e_s` in those of its own, so the two
parities make two matroids on `2..=n`, and `opt(n)` is at most their heaviest common independent set.  The
Franklín–Moniot matching bound is this construction with Hall's condition in place of linear independence.

*Method.*  No solver code changed.  The data were made on the build of 4372b36, before the tactics cleanup
(271c817), with `revise_freedoms` off; on the cleaned-up build the examples reproduce `roots.jsonl` exactly, and every command's
values that were checked.  `examples/common/meet.rs` (commits 67bb492, 8ee0199, 12a0f60): direct sums of
linear matroids over GF(2), the weighted intersection by shortest augmenting paths (Lawler; Schrijver, Theorem
41.7: Bellman–Ford lexicographic in length and arcs; the reference `intersect`), a fast form for blocks of rank at
most 64 (`intersect_fast`: rows by pivot with slot masks, the arcs from or to an element independent of the set
relaxed through one minimum, a start from the heaviest uncontended elements), warm starts checked extreme
(`intersect_warm`) and the weight split (`split`).  `examples/matroid.rs` builds the frame matroids of any state
and runs the measurements; `bench/matroid/` holds the data and `matroid.py`, which makes the tables below
(`matroid_output.md`).  The commands (all `cargo run --release --example matroid --`, one thread, at most two
processes, on the laptop shared with other agents, so the times are indicative):

* `roots --opts bench/curve/slope_roots.jsonl 880-1310 1350-1352` (`roots.jsonl`; the matching and Lagrangian bounds
  are those of the slope study's `slope_roots.jsonl`, whose Lagrangian is fitted with up to 8,000 steps over the
  root's own conflicts);
* `bnb --opts bench/curve/slope_roots.jsonl --bounds gf2 --limit 400000` and the 146 games that
  `slope_proofs.jsonl` proves at δ = 0 (`bnb.jsonl`, run before the start from the heaviest elements was added:
  the same trees, about 20% more time than now), and `bnb --bounds matching --limit 200000 880 936`
  (`bnb_matching.jsonl`);
* `nodes --samples 20` on `testdata/perf/n0936.nodes` and `n1215.nodes` (`nodes.jsonl`; a later `nodes --samples
  0` on both checked the contraction of section 4); with `--every 3
  --lagrange 2000` (`nodes_lagrange.jsonl`); with `--exact 20` (936, every node) and `--every 3 --exact 30` (1215)
  (`nodes_exact.jsonl`); `forced --every 3` on both (`forced.jsonl`);
* `lagrange --limit 1500 936 1000 1215 1254 1300` (`lagrange.jsonl`), `cores 936 1000 1100 1215 1254 1300`
  (`cores.jsonl`) and `whole 2-1300` (`whole.jsonl`), each with `--known results/strict.jsonl`.

*Validity*, beyond the proof.  The intersection algorithms agree with brute force on 3,000 random pairs of block
matroids (the heaviest set of every size and under every cap, warm starts, the split's bounds).  At every
non-prime root 2..1300 the bound is at least `P*` and the fast algorithm's value is the reference's; the optimal
game's promotions and 100 random maximal promotable sets a game are independent in both matroids; at the 90 roots
with at most 22 candidates (to n = 136) every promotable set is enumerated, all independent.  At the probe's 600
nodes, 20 random completions each are independent, and the best completion found by the solver's own search from
the node (all 300 nodes of 936, 98 of 100 sampled nodes of 1215, the others over 30 s) is never above the bound.
No heaviest set in the 146 proofs below was promotable and above `P*`.  The model-free bound is at least `opt(n)`,
with the optimal game's moves independent, for every non-prime n ≤ 1300, and equals `in_the_bag` plus the frame
bound at all 361 roots of 880-1300.

**1. At the root it closes 82% of the matching gap and 42% of the Lagrangian gap, more as n grows.**

| n | games | matching gap | Lagrangian gap | GF(2) gap | GF(2) / matching | GF(2) / Lagrangian | ms |
|---|---|---|---|---|---|---|---|
| 851-900 | 18 | 432 | 112 | 88 | 0.20 | 0.79 | 0.42 |
| 901-950 | 43 | 619 | 185 | 147 | 0.24 | 0.80 | 0.43 |
| 951-1000 | 43 | 749 | 261 | 172 | 0.23 | 0.66 | 0.52 |
| 1001-1050 | 42 | 795 | 362 | 156 | 0.20 | 0.43 | 0.64 |
| 1051-1100 | 42 | 546 | 206 | 81 | 0.15 | 0.39 | 0.75 |
| 1101-1150 | 45 | 522 | 162 | 107 | 0.21 | 0.66 | 0.72 |
| 1151-1200 | 43 | 629 | 154 | 108 | 0.17 | 0.70 | 0.73 |
| 1201-1250 | 42 | 1091 | 426 | 148 | 0.14 | 0.35 | 1.02 |
| 1251-1300 | 43 | 1277 | 471 | 185 | 0.14 | 0.39 | 1.03 |
| 1301-1310 | 7 | 1445 | 554 | 154 | 0.11 | 0.28 | 1.15 |
| 1350-1352 | 3 | 1255 | 465 | 272 | 0.22 | 0.59 | 1.26 |

Over the 361 games of 880-1300 the matching gap grows by 142 ± 9 per 100 in n, the Lagrangian gap by 56 ± 5 and
the GF(2) gap by 4.6 ± 2.4 (R² 0.01; the model-free form of section 6, equal to it there, grows 13.8 ± 1.0 over
600-1300 and 14.3 ± 0.3 over 2-1300): it still grows, at a quarter of the Lagrangian gap's rate or less.  The share
of the matching gap it closes rises by 2.3 ± 0.2 points per 100 in n, of the Lagrangian gap by 9.2 ± 0.9.  The GF(2)
bound is below the root's Lagrangian bound at 346 of the 371 roots (at most 24 above it at the others, mostly
897-918), and the cap `max_total` never binds.  It costs 0.4-1.3 ms a root, a few of the Lagrangian fit's steps.

**2. A plain branch and bound with it proves the games with 12 to 800 times fewer nodes than the solver, and its
node count barely grows with n.**  The skeleton (`bnb` in `examples/matroid.rs`) has one bound and no refinement,
forced members, nogoods or strong branching: a node is cut when the bound (capped at `max_add`) falls below what it
needs; otherwise its heaviest set has a conflict `k₁ < … < k_r` and the node branches into "`k₁ … k_{j−1}` promoted,
`k_j` not" for each `j`.  With the incumbent at `P*` (the slope study's δ = 0 proofs), against the solver's
one-thread proofs:

| n | games | log10 nodes, solver | log10 nodes, GF(2) B&B | ratio (geometric) | s, solver | s, GF(2) B&B |
|---|---|---|---|---|---|---|
| 851-900 | 6 | 3.78 | 2.59 | 15 | 0.5 | 1.0 |
| 901-950 | 16 | 4.56 | 3.48 | 12 | 12.8 | 22.8 |
| 951-1000 | 16 | 5.21 | 3.74 | 30 | 107.0 | 120.1 |
| 1001-1050 | 19 | 5.62 | 3.99 | 43 | 163.8 | 160.1 |
| 1051-1100 | 18 | 4.46 | 2.77 | 49 | 12.6 | 8.0 |
| 1101-1150 | 17 | 3.82 | 2.72 | 13 | 1.5 | 6.4 |
| 1151-1200 | 16 | 4.52 | 2.90 | 42 | 13.2 | 32.8 |
| 1201-1250 | 19 | 6.13 | 3.90 | 171 | 872.2 | 168.6 |
| 1251-1300 | 16 | 6.90 | 4.11 | 613 | 1750.5 | 240.5 |
| 1309 | 1 | 7.14 | 4.24 | 794 | 157.5 | 16.3 |
| 1350, 1352 | 2 | 6.88 | 4.79 | 124 | 315.9 | 128.3 |

log10 of the solver's nodes grows 0.476 ± 0.066 per 100 in n over these games, the GF(2) branch and bound's 0.138 ±
0.048 (R² 0.05).  Its node count follows the root's GF(2) gap (0.0111 ± 0.0005 decades per unit, R² 0.78: ×10 per 90
of gap) as the solver's follows the Lagrangian gap, and 0.011 times the gap's growth of 5-14 per 100 in n predicts
0.05-0.15 decades per 100 in n, the measured slope at its upper end.  Its largest trees are 1287 (87,841 nodes; the
solver 20,972,332), 992 (77,567; 2,099,730) and 1350 (77,283; 17,504,520).  With the matching bound in the same
skeleton, 880 and 936 are unfinished after 200,000 nodes (the solver proves them in 25,398 and 41,653).  In time,
computing the bound from scratch at every node (0.68 ms a node): 905 s for the 146 proofs against the solver's 3,408
s, 409 s against 2,622 s over 1201-1300; slower in the five bands whose proofs the solver makes in 10^5.2 nodes or
fewer on average, faster in the others.

**3. At search nodes it cuts more than half of the probe's open nodes, where the matching bound cuts none.**  The
fixtures' nodes are units the solver's proof of 936 or 1215 handed to its searcher; the target is what a completion
must beat, `P* − sum(promoted)`; a *true gap* is a bound less the node's best completion.

| n | nodes | matching cuts | GF(2) cuts | GF(2) ms | every 3rd: node-fitted Lagrangian cuts (GF(2)) | either | true gap, matching | true gap, GF(2) | forced exactly | by the split |
|---|---|---|---|---|---|---|---|---|---|---|
| 936 | 300 | 0 | 163 | 0.33 | 66 of 100 (55) | 69 | 317 | 154 (300 nodes) | 26.1 a node | 66% |
| 1215 | 300 | 0 | 167 | 0.68 | 50 of 100 (59) | 67 | 548 | 272 (98 nodes) | 28.0 a node | 60% |

The GF(2) bound halves the true gaps.  A Lagrangian over the matching with nogoods of the node's own, fitted at the
node (`root_lagrange` of the slope study, moved to `examples/common/root_lagrange.rs`, 2,000 steps: about a second a
node, which no search can afford, and far more than the search's refits from the root), cuts about as many nodes;
the two cut different ones (either cuts 69 and 67 of 100).  At the nodes the GF(2) bound leaves open, 26-28 members
of its heaviest set (of 40-55) are forced: the bound without each falls below the target.

**4. Cost, and the incremental forms.**

* *From scratch*, the fast algorithm: 0.33 and 0.68 ms a node at 936 and 1215.  Building the frame matroids
  (Gaussian elimination of each frame's moves, then every candidate's two vectors projected) is about 0.2 ms of it;
  the rest is about 45 augmentations of about 10 µs (rebuilding the spans, reducing every candidate, Bellman–Ford).
  The solver's step costs about 5 µs.
* *A warm start from the parent's heaviest set* (`bnb --bounds gf2warm`): not kept.  Contracting members of the
  heaviest set keeps the rest optimal (a common independent set of the contraction plus them is one of the parent's),
  but a child also deletes the member branched on, and the rest was extreme (no negative cycle in its exchange graph,
  Schrijver Theorem 41.5, and no heavier smaller set) at 2 of the 2,543 nodes of 1250 and none of the 1,874 of 1100.
* *Starting from the heaviest uncontended elements* (the longest prefix of the candidates by weight that is common
  independent, which is the heaviest set of its size and heavier than every smaller one): 20% fewer augmentations
  (1250: 146,565 to 117,863), the same values.  Kept in the research code.
* *A weight split* (Frank's weight-splitting theorem): `w = w1 + w2` with the heaviest set the heaviest independent
  set of `A` for `w1` and of `B` for `w2`, found from the final exchange graph as difference constraints (Bellman–Ford;
  found for every heaviest set tried).  The two greedy optima then bound the problem with any members left out (weak
  duality), the analogue of the matching bound's `bound_excluding`.  It finds 66% and 60% of the exactly forced
  members (looser by 26 and 43 a member on average) at two greedy runs a member, against an intersection a member.
* The frame matroids of a child are the parent's contracted by the members it promotes (a frame's reduction makes
  the candidates it consumes loops, so nothing else changes; at all 600 probe nodes the root's matroids contracted
  by the node's promotions give the node's bound exactly): one pivot per promoted member in each of its two frames
  instead of the 0.2 ms build.

**5. What it misses.**  The conflicts of the root's heaviest set are large, 7-18 members (most 13-18): the odd cores
of the bound's definition.  The cores themselves (`cores`) span frames 1-3: 18-65 movers each, a third to two thirds
with three or more live factors, over as many factors as movers or a few more.

* *Lorea's hypergraphic matroid* (the evidence crate's rank bound: every subset lists at least one more factor than it
  has movers) is valid per frame too, and each frame could use either matroid.  It sees none of the 14 cores of
  936-1300: every subset of each lists more factors than movers.  Neither known matroid of playable sets reaches
  what is left, and three matroids cannot be intersected in polynomial time.
* *A Lagrangian over nogoods with the intersection as the inner problem* (`lagrange`: the conflicts of each step's
  heaviest set, 1,500 damped Polyak steps at the root): the GF(2) gap of 936 from 132 to 118, 1000 214 to 137, 1215
  213 to 198, 1254 204 to 166, 1300 151 to 129 (7-36%).  The cover inequalities of 15-member conflicts are weak.
* *Other fields*: over GF(3) or the rationals the unsigned vectors of a triangle of movers are independent, so odd
  cycles are lost; signed (oriented) vectors keep every cycle but see only orientable closed surfaces, where GF(2)
  sees all.  GF(2) is the field.
* *Caps*: the frames' freedoms are the ranks of the frame matroids, and `max_total` never binds.  Theorem 8's termwise
  domination is about the upper half, which the frame model fixes; there is nothing like it for an intersection.

**6. The whole game, model-free.**

| n | games | gap mean | median | max | exact | ms |
|---|---|---|---|---|---|---|
| 2-100 | 74 | 0.5 | 0 | 18 | 70 | 0 |
| 101-200 | 79 | 0.9 | 0 | 24 | 64 | 2 |
| 201-300 | 84 | 2.8 | 0 | 19 | 44 | 8 |
| 301-400 | 84 | 15.0 | 12 | 35 | 11 | 18 |
| 401-500 | 83 | 20.7 | 19 | 51 | 0 | 34 |
| 501-600 | 86 | 43.6 | 46 | 106 | 5 | 57 |
| 601-700 | 84 | 66.4 | 59 | 139 | 0 | 90 |
| 701-800 | 86 | 78.1 | 90 | 178 | 0 | 132 |
| 801-900 | 85 | 86.0 | 92 | 178 | 0 | 186 |
| 901-1000 | 86 | 159.4 | 146 | 323 | 0 | 252 |
| 1001-1100 | 84 | 118.4 | 101 | 223 | 0 | 334 |
| 1101-1200 | 88 | 107.7 | 95 | 201 | 0 | 429 |
| 1201-1300 | 85 | 167.0 | 176 | 282 | 0 | 545 |

The model-free matching bound's gap is 140-430 around 500-700 and 650-1,200 from 800 on
([evidence 3](../evidence/README.md#3-model-free-exact-cross-check-of-optn)); this one's is a tenth of that, from the
reference algorithm in half a second at 1300.  That the two constructions agree at every root of 880-1300 checks
each against the other.

*Conclusion.*  The heaviest common independent set of the two GF(2) frame matroids is a valid bound on the
promotions, computable in about a millisecond from scratch, tighter than the matching bound by construction and than
the root's Lagrangian bound over nogoods at 346 of 371 roots, and its gap grows a quarter as fast or less: it closes
a share of the gap that grows with n (82% of the matching gap and 42% of the Lagrangian gap at 880-1300, rising).
Alone, in a plain branch and bound with the bound computed from scratch at every node, it cuts the slope of a proof's
node count from 0.48 to 0.14 decades per 100 in n and the time of the 146 proofs by 3.8 times (6.4 over 1201-1300).
What it misses is large odd cores, which neither Lorea's matroid nor nogood cuts reach well.

*What it implies.*

* *The GF(2) bound belongs in the search* (phase 2): as the node bound, with branching on the conflicts of its
  heaviest set, forced members from the weight split (and exactly where it pays), and the frame matroids carried
  down the path by contraction.  The cost to beat is about 0.3-0.7 ms a node from scratch against a step of about
  5 µs; at the frontier the trees shrink by two to three decades, so even from scratch it wins there, and an
  incremental exact intersection is the engineering that would win everywhere.  Whether the matching bound, the
  refinement and the Lagrangian over nogoods still earn their keep beside it is to be measured; the node-fitted
  Lagrangian cuts different nodes from it, so a Lagrangian over the intersection may pay at nodes even though at the
  root it closes little.
* *For the evidence crate*: the model-free bound is the "relaxation that sees more of the game" its section 8 asks
  for, a tenth of the matching bound's gap, and should take its model-free cross-check of opt(n) well past 989.
* *For THEORY.md*: the lemma that a playable set stays independent with the unit vectors of its members below n/2,
  and the split by the parity of Ω into two matroids, extend Theorem 7's lemma from the upper half to the whole game.

## How far can nogoods go (2026-10)

*Question.*  The Lagrangian bound over the nogoods (tactic `lagrange`) fits its multipliers at the root, to the
nogoods each searcher has learned, and uses them at every node.  The [slope study](#what-drives-the-slope-2026-10)'s
two pilots found that the root's best-fitted bound given to every searcher saved little, and that a refit from zero
multipliers ends 50-540 above the best bound the same nogoods allow.  (1) What is the best bound the nogoods a
searcher holds could give, at the root and at its nodes, and how far is the search's own bound from it?  (2) What does
the bound become with more conflicts: every minimal conflict of a few members, every conflict of the node?  (3) Does
the gap left by the best possible use of all conflicts still grow with n?  If it does, that bounds what this direction
can do.  (4) Is something cheap worth most of it?  And once the
[matroid study](#matroid-intersection-bounds-2026-10) made the GF(2) bound the baseline: (5) what share of its
residual gap do conflicts close, at the root and at nodes, by n, and at what cost?

*What was kept.*  Nothing but this entry; the solver is unchanged.  The study's code lived on a branch that was never
pushed and has been deleted, as the project does with research branches once they are harvested (the pilots of
[IDEAS.md](IDEAS.md#6-the-tools-and-what-became-of-the-pilot-code) went the same way).  It was a library module that
solves the LP below (Dantzig-Wolfe column generation with a dense revised simplex, and exact separation of conflicts),
an example program that ran the measurements, a patch that instrumented the search, and a script that made the tables
from the data; the data went with it.  So the method is written out here to be redone: which LP, with which inner
problems, how it is solved, what was measured where, on which games and fixtures, and how it was checked.  What it
read is on master: the games' optima (`results/strict.jsonl`, `bench/curve/bands.jsonl`), the probe's fixtures
(`testdata/perf/n0936.nodes`, `n1215.nodes`), and the slope and matroid studies' data (`bench/curve/slope_roots.jsonl`
and `slope_proofs.jsonl`, `bench/matroid/roots.jsonl` and `bnb.jsonl`), against which its tables were made.

*The tool: the cover LP.*  A conflict `K` of a node (a set of its candidates that cannot all be promoted) gives every
completion `F` the *cover inequality* `|F ∩ K| ≤ |K| − 1`.  The cover LP of a family of conflicts is `max Σ x·y_x` over
the polytope of an *inner relaxation* (`y` has a number in [0, 1] for each candidate `x`) with those inequalities.  The
inner relaxation's sets are the candidate sets that a matching can promote (the matching bound's relaxation), or the
common independent sets of the two GF(2) frame matroids of the matroid study.  Both polytopes are integral (bipartite
matching; Edmonds' matroid intersection), so the LP is the Lagrangian dual of the Lagrangian bound over the same
conflicts with that inner problem (Geoffrion): the lowest that bound can be, whatever its multipliers, and the LP's
optimal duals are the best multipliers.  The bound itself, for multipliers `λ_K ≥ 0`, is the inner problem's heaviest
set with each candidate `x` weighed `x − Σ_{K ∋ x} λ_K`, plus the constant `Σ λ_K (|K| − 1)`; with the matching inner
it is the bound of the `lagrange` tactic.

*Solving it.*  The *master* is the LP over convex combinations `μ` of the inner problem's sets: a column for each set
`Q` (its sum, and how many members of each conflict it holds), a row `Σ_j μ_j |Q_j ∩ K| ≤ |K| − 1` for each conflict,
and `Σ μ = 1`; it starts from the empty set as its one column, which satisfies every row.  It is solved by a dense
revised simplex with an explicit basis inverse (refactorized every 64 pivots, tolerances 1e-9, the right-hand sides
perturbed by tiny amounts against degeneracy): a new column enters by the primal simplex, a new row with its slack
basic and is repaired by the dual simplex.  A column is priced by the inner problem's heaviest set at multipliers that
are the master's duals smoothed towards the best multipliers found so far (Wentges, weight 0.5; if no column of
positive reduced cost comes out, the plain duals; if none still, the master is optimal).  The matching inner problem
was the solver's own weighted matching solved from scratch, the intersection the matroid study's.  Every pricing is a
Lagrangian bound, an upper bound on the LP whatever its multipliers (weak duality), computed in fixed point with
multipliers rounded to 2⁻²⁰, so exact for the multipliers it used; the master's value is a lower bound on the LP over
the rows it has.  The solve stops when they are within 1e-3 (or after 50,000 pricings).

*Separation, exact.*  The rows to add are the cover inequalities of *all* conflicts of the node that the master's
solution `y = Σ μ_j 1_{Q_j}` violates.  A conflict `K` is violated when its *deficit* `Σ_{x ∈ K} (1 − y_x)` is below 1,
so it lies in `I ∪ F`, where `I` is the set of the candidates with `y_x = 1` and `F` a set of fractional ones whose
deficits sum below 1.  A minimal conflict fails in one frame `i` (a set that cannot be promoted has a frame that is
not winnable with it), and all its members are promoted into or out of frame `i` (one that is not could be left out,
and frame `i` would still fail), so `F` lies among the fractional candidates of homes `i` and `i + 1`.  A subset of a
set that can be promoted can be promoted, so it is enough to check the *maximal* such sets `I ∪ F`, frame by frame
(enumerated by branching on the fractional candidates in ascending order of deficit, pruned when the room left cannot
be filled): if every one can be promoted, no cover is violated, and each that cannot holds a violated conflict, which
the model's conflict analysis finds inside it (its deficit is at most that of `F`; `Model::conflict_using`).  If `I`
alone cannot be promoted, the conflicts are the one found and, for each of its members, the one found without it.
Solving and separating alternate, at most 400 rounds of at most 200,000 feasibility checks, until separation adds
nothing.  The LP over *all* conflicts of the node (`ideal`) is then between the master's value and the lowest
Lagrangian bound found, and is *certified* (exact) when the master was optimal and the last separation complete within
its budget.  Every LP reported below is, except where said.

*Relax-and-cut*, the cheap relative: from some conflicts and multipliers, at most `s` subgradient steps on the
Lagrangian bound (Polyak's rule towards the target, a node's `need`: step `damping × (bound − need + 1) / ‖g‖²` with
`g_K = |Q ∩ K| − (|K| − 1)` for the inner problem's heaviest set `Q`, the damping halved after 3 steps without a lower
bound, multipliers kept at least 0), each step first adding up to `p` conflicts of `Q` with multiplier 0 (the conflict
of `Q`, then of `Q` without that conflict's largest member, and so on), stopping as soon as the bound is below `need`.

*Validity.*  The LP over all conflicts, over the matching, was tested on the matching bound's walk harness (random walks
of the search from the root, 2 walks of 25 loop iterations for each game 40-200 with a golden file in `testdata/golden`,
seeded by n) at 5,495 nodes: separation certified; the LP at least the best completion of the node (exhaustive) and at
most the matching bound; the columns of one node's LP carried to the next node of the walk (a matching of a node is a
matching of the next if every promotion the next has and the first does not is promoted in it, every promotion the first
has and the next does not is a candidate of the next, and it promotes no candidate the next does not have) are matchings
there, and starting from them gives the same LP; and every forced member read off the potentials (13,642 of them) is
promoted by every best completion.  Over the GF(2) intersection, a branch and bound with the incumbent one below the
optimum (which must find a set worth `P*`) finds it in all 709 non-prime games 30-879 (largest tree 1,222 nodes), and
the 146 proofs of section 6 met no promotable set above `P*`.

*Method.*  No solver code changed.  Games are set up with the default (strict) tactics, the optimum promotion sum `P*`
read as the slope study reads it (from `results/strict.jsonl`, and `bench/curve/bands.jsonl` past 1300), and everything
is one thread a process and at most two processes, on the laptop shared with four other agents: times are indicative.  A
node's *excess* under a bound is the bound's integer part less `P* − sum(promoted)`, the most a completion can be worth
without beating the incumbent; the node is cut when that is at most 0 (its `need` is `P* + 1 − sum(promoted)`).  The `±`
of a trend is the standard error of its least-squares slope.  The measurements:

* *Roots*: for each of the 370 non-prime games of 880-1310 and 1350-1352 with a known optimum (371 over GF(2), which
  has 1351 too), the matching bound, the inner problem's own bound, and the LP over all conflicts of the root, with the
  matching or the GF(2) intersection inside; the number and sizes of the covers with a positive dual at the end, the
  columns, pricings and simplex pivots.  The slope study's root Lagrangian bound (8,000 damped steps over conflicts of
  its own Lagrangian matchings) and the matroid study's GF(2) bound are those of `slope_roots.jsonl` and
  `roots.jsonl`.
* *Small conflicts*, at the roots of 880, 1000, 1215 and 1300: every minimal conflict of one, two and three members,
  by the same frame argument as the separation (all of a minimal conflict's members are candidates of home `i` or
  `i + 1` for the frame `i` it fails in): a pair is a conflict if promoting both fails, a triple a minimal one if no
  pair of it is a conflict and promoting all three fails; then the LP, by solving only (no separation), over the
  conflicts of at most one, two and three members, against the LP over all.
* *Nodes*: the 300 nodes of each of the probe's fixtures `n0936.nodes` and `n1215.nodes` (units that the solver's
  proof handed to its searcher; the incumbent is the optimum): the LP over all conflicts of each node, with either
  inner problem, and relax-and-cut from no conflicts and zero multipliers, 3, 10 and 30 steps of 4 conflicts each.
* *Traces of the search*, from an instrumented build of the solver of 4372b36 (before the tactics cleanup, with
  `revise_freedoms` off: the trees the default build makes now) running a one-thread proof seeded with the optimum.
  The instrumentation wrote, to one file: each nogood the table learns (its members in hexadecimal, one digit for four
  numbers, lowest first) and each time the table is cleared; at each refit of the Lagrangian relaxation, its nogoods
  and their integer multipliers (64ths); and, at every K-th Lagrangian check (K = 100 at 936, 2,000 at 1215), the
  node's promotions and candidates, its `need` and the value of the search's bound.  Replaying the file rebuilds the
  table and the relaxation at each sampled check, and measures there: the LP over the relaxation's nogoods that are
  live at the node (the nogood less the promotions, nonempty and among the candidates), the LP over those and the
  table's, relax-and-cut from the search's multipliers (5, 20 and 50 steps, adding 0 or 4 conflicts a step; the table's
  nogoods start at 0), and the LP over all conflicts; and at each refit, the bound its multipliers give at the root
  against the LP over the nogoods it fitted (the relaxation's and the table's).
* *Extra checks in the search*, the same build with a bound added where the search's matching check and Lagrangian bound
  leave a node or child open: if the extra bound is below `need` the node is cut; if it is not, and forced members are
  asked for, the members of the heaviest set at its best multipliers without which its bound (read from the inner
  problem's potentials, as the Lagrangian bound reads its forced members) falls below `need` join the node's forced
  members.  The bound was one of: the LP over all conflicts of the node (`ideal`), stopped at the first Lagrangian bound
  below `need`, starting from the nogoods the searcher holds that are live there, from every conflict found at earlier
  nodes (kept in a pool, as nogoods of the root, and added where live) and from the columns of the last LP carried over;
  the LP over the relaxation's live nogoods (`relax`) or over those and the table's (`known`), solved without
  separation; relax-and-cut from the search's multipliers over the relaxation's and the table's nogoods, 5 or 20 steps,
  with no conflicts added (`rf`) or 4 a step (`rc`); *per-subtree multipliers* (`anchor`): at each node whose promotions
  number a multiple of 4, 8 or 16 and which has no anchor of its own, the LP over the nogoods live there (and those of
  the enclosing anchor), after which the covers with a positive dual are made nogoods of the root and, with their duals
  as multipliers, give the bound of every node below it, by one pricing; and the LP over all conflicts used only at
  nodes of at most 8, 16 or 24 promotions.  (A solve stopped at the first bound below `need` is not certified; any
  Lagrangian bound cuts soundly.)  The searches are the slope study's proofs (one thread; the incumbent a value `P* + δ`
  with no set behind it, so δ = 0 is the proof of the optimum), and the "search's" steps those of `slope_proofs.jsonl`
  (the traced proofs of 936 and 1215 took its 41,653 and 1,111,451 steps).
* *Branch and bound with the LP at every node*: a node is cut when the LP over all conflicts of the node has a
  Lagrangian bound below `need` (checked first with no conflicts); an integral LP solution is a promotable set, the
  node's best completion; the children inherit the conflicts that still hold in them.  Branching, with the matching
  inside, on the most fractional candidate of the LP's solution (promoted first, then left out); with GF(2) inside, as
  the matroid study's branch and bound branches, on a conflict `k₁ < … < k_r` of the heaviest set at the best
  multipliers ("`k₁ … k_{j−1}` promoted, `k_j` not", for each `j`).  The incumbent is `P*`, or `P* − 1` for the
  soundness check.

**1. At the root, all conflicts are worth what the root's own conflicts are.**  The LP over every conflict of the
root, over the matching (every LP exact; the GF(2) gap is the [matroid study's](#matroid-intersection-bounds-2026-10),
the Lagrangian gap the [slope study's](#what-drives-the-slope-2026-10)):

| n | games | matching gap | Lagrangian gap (slope study) | all conflicts | GF(2) gap | ms |
|---|---|---|---|---|---|---|
| 851-900 | 18 | 432 | 112 | 110 | 88 | 30 |
| 901-950 | 43 | 619 | 185 | 180 | 147 | 79 |
| 951-1000 | 43 | 749 | 261 | 256 | 172 | 168 |
| 1001-1050 | 42 | 795 | 362 | 358 | 156 | 203 |
| 1051-1100 | 42 | 546 | 206 | 199 | 81 | 160 |
| 1101-1150 | 45 | 522 | 162 | 162 | 107 | 112 |
| 1151-1200 | 43 | 629 | 154 | 150 | 108 | 182 |
| 1201-1250 | 42 | 1091 | 426 | 419 | 148 | 348 |
| 1251-1300 | 43 | 1277 | 471 | 459 | 185 | 424 |
| 1301-1310 | 7 | 1445 | 554 | 547 | 154 | 685 |
| 1350-1352 | 2 | 1238 | 452 | 430 | 267 | 490 |

Over 880-1300 its gap is 263.9 on average against the slope study's 269.0 (8,000 damped steps over conflicts of its
own Lagrangian matchings), 0.2 to 11.6 below it band by band, and it grows 54.7 ± 5.1 per 100 in n against 56.0 ± 5.2:
those conflicts were already all the root has, and that fit had converged.  It closes 67% of the matching gap, a share
that does not grow (−0.8 ± 0.4 points per 100 in n).  So over the matching the best a Lagrangian over nogoods can do at
the root leaves a gap growing with n as fast as the Lagrangian bound's.  About 30 covers bind at the optimum, a third
of them pairs and 36% of eight members or more; at 880 the root's LP solution is half-integral and its binding covers
span frames 0-2.

**2. Small conflicts are not enough.**  Every minimal conflict of one, two and three members of the root (none of one
member, 245-413 pairs, 892-1,723 triples, 2-11 s to enumerate), against all conflicts (gaps):

| n | matching | ≤ 2 members | ≤ 3 | all | GF(2) | ≤ 3 members | all |
|---|---|---|---|---|---|---|---|
| 880 | 487 | 371 | 371 | 167 | 56 | 56 | 1 |
| 1000 | 911 | 576 | 576 | 392 | 214 | 214 | 130 |
| 1215 | 966 | 739 | 702 | 366 | 213 | 213 | 165 |
| 1300 | 1421 | 1043 | 973 | 520 | 151 | 151 | 130 |

Over the matching they close 24-37% of the gap, against 57-66% for all conflicts, and the triples add almost nothing
to the pairs; over GF(2) they add nothing at all (the intersection already implies them).  The conflicts that bind
are large, and generating them proactively by size cannot reach them.

**3. The search's bound: near the best its nogoods allow at the root, far from it at the nodes.**  Traces of the
one-thread proofs of 936 (41,653 steps, every 100th Lagrangian check) and 1215 (1,111,451 steps, every 2,000th),
replayed.

* *At the root*: at each refit, the bound its multipliers give against the LP over the nogoods it fitted.  At 1215
  (23 refits, 88-329 nogoods each) the 50 warm-started steps end 7.8 above that LP on average, 14.3 at most, and the
  final relaxation's root bound (35,445) is 29 above the LP over all conflicts (35,416; `P*` is 35,050).  The one refit
  of 936 (from zero, after 2,000 steps) ends 39 above.  The slope study's "50-540 above" was a refit from zero
  multipliers; the search refits from its last ones and converges over its refits (the slope
  study did not measure that).
* *At the nodes*, the checks the search leaves open (242 of 251 sampled at 936, 345 of 370 at 1215): the mean excess
  of each bound, and the share of those checks it would cut.

| | the search's | its relaxation, refitted at the node | all the nogoods it holds | all conflicts | 20 refit steps | 50 | 20 with conflicts added | 50 |
|---|---|---|---|---|---|---|---|---|
| 936: excess | 54 | 47 | −22 | −139 | 13 | 9 | 7 | 4 |
| 936: cut | 0 | 5% | 74% | 97% | 38% | 54% | 62% | 79% |
| 1215: excess | 73 | 38 | −18 | −191 | 24 | 18 | 12 | 6 |
| 1215: cut | 0 | 24% | 67% | 95% | 30% | 41% | 46% | 66% |

  (The refits start from the search's multipliers, the table's nogoods at 0; "conflicts added": 4 conflicts of each
  step's Lagrangian matching.)  Refitting the relaxation's own nogoods at the node gains little: a typical node has 1
  of 936's 16 live and 9 of 1215's 52-65, the others' members promoted or dropped on the way down.  The nogoods the
  searcher holds in its table, 37 and 49 of them live at a typical node, would cut two thirds to three quarters of
  the checks it leaves open with multipliers fitted there, and all conflicts of the node 95-97%.  The weakness is not
  the nogoods but the multipliers: fitted at the root, for the root.

**4. All conflicts at every node: a constant factor, not a slope.**  The instrumented build, cutting with the LP over
all conflicts of the node wherever the search's own bounds leave a node or child open (one thread, seeded with the
optimum):

| n | the search's steps | with the extra cut | ratio | ms a check |
|---|---|---|---|---|
| 880 | 25,398 | 1,741 | 14.6 | 9 |
| 900 | 4,194 | 397 | 10.6 | 12 |
| 936 | 41,653 | 1,327 | 31.4 | 9 |
| 950 | 460,813 | 14,658 | 31.4 | 29 |
| 1000 | 366,846 | over 29,080 (stopped at 590 s) | under 12.6 | 22 |
| 1050 | 46,527 | 4,207 | 11.1 | 25 |
| 1088 | 6,205 | 454 | 13.7 | 31 |
| 1100 | 29,614 | 6,157 | 4.8 | 6 |
| 1150 | 4,858 | 550 | 8.8 | 20 |
| 1184 | 52,259 | 6,469 (4,485 with its forced members) | 8.1 (11.7) | 5 |
| 1196 | 216,076 | over 15,675 (stopped at 590 s) | under 13.8 | 39 |

log10 of the steps with the cut against the search's: slope 0.81 ± 0.14 over the nine finished games, and the two
stopped ones, among the largest trees, lie above that line.  Margin scans (incumbent `P* + δ`, the cut and its forced
members): at 1215 the ratio is 1,614 at δ = 400, 209 at 300, 131 at 200, 49 at 150 and 24 at 100; at 936 604, 94, 64
and 30 at δ = 200, 100, 50 and 0.  The advantage shrinks as the remaining gap grows: the LP over all conflicts is no
tighter than the Lagrangian bound at the top of the tree (section 1) and far tighter deep down, so it removes the deep
nodes of a tree whose top it leaves as it was.  A plain branch and bound with the LP at every node (on the most
fractional candidate) needs 10-63 times fewer nodes than the solver's steps at 880, 900, 936, 1000, 1088, 1100 and
1184 (2,439, 127, 1,503, 15,593, 99, 2,531 and 2,135 nodes), with a log-log slope of 1.15 ± 0.17 against them.  The
two log-log slopes, 0.81 and 1.15, lie on either side of 1 and within 1.4 standard errors of it: about ten times fewer
nodes, no visible change of the tree's growth, and a check costs 5-40 ms, 1,000-5,000 of the search's steps.

**5. Nothing cheap gets enough of it.**  The same build with cheaper extra checks, their forced members joining the
node's (steps; the cost of a check):

| extra check at the open checks | 880 | 936 | 1184 | ms a check |
|---|---|---|---|---|
| none (the search) | 25,398 | 41,653 | 52,259 | |
| the LP over the nogoods the searcher holds | 8,856 | 10,004 | 12,471 | 1.3-4.0 |
| 5 refit steps from its multipliers, 4 conflicts added a step | 11,267 | 17,185 | 18,970 | 0.5-1.2 |
| 20 such steps | 4,383 | 5,929 | 9,707 | 1.6-3.6 |
| 20 refit steps, no conflicts added | | 15,741 | | 1.0 |
| the LP over all conflicts at nodes of at most 8, 16 or 24 promotions only | | 39,858; 39,579; 27,507 | | 33-40 |
| per-subtree multipliers: the LP over the nogoods held at every node of 4, 8 or 16k promotions, used below it | | 13,291; 17,841; 26,432 | | 0.2-0.5, and an LP fit every 8-34 steps |

Apart from the per-subtree fits (25-70 steps' cost a check, and a fit every few steps) and the LP at shallow nodes
(4,000-5,000 steps a check, for at most 1.5 times fewer), each costs per check what 80 to 550 of the search's steps
cost, for 2.3-7 times fewer steps, and all lose time: by factors of 11 (the per-subtree multipliers at 936) to 100 (the
table's LP at 1184).  At 936 the search takes 0.34 s and the cheap forms 3.7-13 s; at 880 0.16 s against 4-8 s, at 1184
0.38 s against 15-38 s.  Raising multipliers lowers the weights of their members, which the incremental levels cannot
repair without the Hungarian searches that make a solve, so the refits could not be made much cheaper; keeping the
expensive check to the top of the tree saves nothing, since the gain is at the bottom; and subtrees are short chains
(four nodes in five end after their first child), so per-subtree fits come every few steps.

**6. With the GF(2) intersection inside.**  The GF(2) bound alone beats the best nogood bound over the matching at the
root (section 1's last columns: its gap is 28-82% of the latter's band by band, and grows 4.6 per 100 in n against
54.7), so nogoods over the matching are superseded.  With the heaviest common independent set of the two GF(2)
frame matroids as the cover LP's inner problem:

* *At the root* (all 371 roots of the matroid study, every LP exact, 10-158 ms):

| n | games | GF(2) gap | with all conflicts | share closed |
|---|---|---|---|---|
| 851-900 | 18 | 88 | 26 | 0.73 |
| 901-950 | 43 | 147 | 75 | 0.49 |
| 951-1000 | 43 | 172 | 106 | 0.40 |
| 1001-1050 | 42 | 156 | 107 | 0.32 |
| 1051-1100 | 42 | 81 | 36 | 0.57 |
| 1101-1150 | 45 | 107 | 55 | 0.53 |
| 1151-1200 | 43 | 108 | 44 | 0.65 |
| 1201-1250 | 42 | 148 | 105 | 0.29 |
| 1251-1300 | 43 | 185 | 128 | 0.31 |
| 1301-1310 | 7 | 154 | 138 | 0.10 |
| 1350-1352 | 3 | 272 | 192 | 0.29 |

  Over 880-1300 all conflicts close 46% of the GF(2) gap on average (79 left of 136), a share that falls 3.8 ± 0.8
  points per 100 in n; what is left grows 8.4 ± 2.0 per 100 in n, the GF(2) gap 4.6 ± 2.4.  (The matroid study's
  subgradient over the conflicts of each heaviest set closed 7-36% at five roots; the LP is the best any multipliers
  can do, and its gap at 1300, 129.5, is that study's 129 to within the rounding of its integer bounds.)  About 10
  covers bind at a root, 96% of them of eight members or more.
* *At the probe's nodes*: GF(2) alone cuts 163 and 167 of the 300 nodes of 936 and 1215 (the matroid study's
  figures); with all conflicts 278 and 249, that is 84% and 62% of the nodes it leaves open, at 12 and 46 ms a node.
  Relax-and-cut with the intersection inside, from no conflicts, 4 conflicts of each heaviest set added a step, cuts
  45%, 55% and 63% of them at 936 with 3, 10 and 30 intersections, and 19%, 39% and 42% at 1215.  (Over the
  matching, for comparison: all conflicts cut 84% and 71% of the nodes, relax-and-cut 20-55% and 12-45%.)
* *In a branch and bound* (the matroid study's skeleton and branching, with the LP over all conflicts in place of the
  bare intersection), on the 146 games the slope study proves, against the matroid study's `bnb.jsonl`:

| n | games | log10 nodes, solver | GF(2) B&B | GF(2) and all conflicts | ratio | s, GF(2) B&B | s, with all conflicts |
|---|---|---|---|---|---|---|---|
| 851-900 | 6 | 3.78 | 2.59 | 1.47 | 13.1 | 1.0 | 0.9 |
| 901-950 | 16 | 4.56 | 3.48 | 2.50 | 9.6 | 22.8 | 22.8 |
| 951-1000 | 16 | 5.21 | 3.74 | 2.82 | 8.3 | 120.1 | 136.8 |
| 1001-1050 | 19 | 5.62 | 3.99 | 3.36 | 4.2 | 160.1 | 241.0 |
| 1051-1100 | 18 | 4.46 | 2.77 | 2.09 | 4.8 | 8.0 | 8.9 |
| 1101-1150 | 17 | 3.82 | 2.72 | 2.08 | 4.3 | 6.4 | 9.4 |
| 1151-1200 | 16 | 4.52 | 2.90 | 2.10 | 6.4 | 32.8 | 28.1 |
| 1201-1250 | 19 | 6.13 | 3.90 | 3.04 | 7.3 | 168.6 | 210.2 |
| 1251-1300 | 16 | 6.90 | 4.11 | 3.43 | 4.8 | 240.5 | 334.6 |
| 1309 | 1 | 7.14 | 4.24 | 3.34 | 8.1 | 16.3 | 14.7 |
| 1350, 1352 | 2 | 6.88 | 4.79 | 3.74 | 11.1 | 128.3 | 117.2 |

  All conflicts make the trees 6.2 times smaller (geometric mean; 231,592 nodes against 1,327,966 in all), less as n
  grows (the ratio's logarithm −0.046 ± 0.016 per 100 in n), and the node count grows 0.18 ± 0.05 decades per 100 in n
  against GF(2)'s 0.14 ± 0.05.  It follows the root's GF(2) gap at 0.0112 ± 0.0006 decades per unit, the matroid
  study's 0.0111 for GF(2) alone: the conflicts move the line down, not its slope.  From the root alone, 0.0111 times
  the gap they close there predicts 0.62 decades on average; the trees shrink by 0.79.  In time it is no gain: 1,125 s
  against 905 s for the 146 proofs (545 against 409 s over 1201-1300), a quarter to a third slower, a node costing
  2.4-11 ms against the bare intersection's 0.3-1.

*Conclusion.*  A Lagrangian bound over nogoods can be no better than the cover LP over all conflicts, and that is a
constant factor, not a slope.  Over the matching: at the root it is the slope study's Lagrangian bound (5 below it on
average), whose gap grows 55 per 100 in n; the search's multipliers are near the best its nogoods allow at the root and
far from it at the nodes, where the nogoods it already holds would cut two thirds of the checks it leaves open; but the
best use of every conflict at every node makes the proofs about ten times smaller, with no visible change in how they
grow, at a thousand or more steps' cost a check, and every cheap form tried loses time by a factor of 11 to 100.  Over
the GF(2) intersection, which supersedes it: all conflicts close 46% of the root's gap, a share falling with n, cut
62-84% of the probe's nodes it leaves open, and make its branch and bound about six times smaller (less as n grows)
in a quarter more time.  So the answer to (3) is yes: over the matching the ideal leaves a gap that grows as the
Lagrangian bound's does, and over GF(2) the residual still grows (8.4 ± 2.0 per 100 in n, against GF(2)'s own 4.6 ±
2.4; the two do not differ by more than their errors).  In both, nogoods give a constant factor.

*What it implies.*

* *Nothing of this to implement now; the direction is superseded.*  The slope's lever is the GF(2) bound, going into
  the search.  The existing `lagrange` tactic (the matching, root-fitted multipliers) should be measured again once
  GF(2) bounds every node: GF(2) is tighter at the root than the best a nogood bound over the matching could ever be,
  and what the Lagrangian adds at the nodes (the matroid study found a node-fitted one cutting different nodes from
  GF(2)) is at most this study's constant factor, at its cost.
* *Later, a constant factor over GF(2)*: a Lagrangian over nogoods with the intersection as the inner problem is worth
  at most about six times fewer nodes at the frontier, less as n grows, and only if a node costs little more than one
  intersection (multipliers carried from the parent and a step or two at a node, on an incremental intersection that
  takes falling weights).  Relax-and-cut with three intersections a node already cuts 19-45% of the probe's nodes that
  GF(2) leaves open.
* *What conflicts miss* is what GF(2) misses, the large odd cores; their cover inequalities are weak (`|F ∩ K| ≤ 14`
  for 15 members).  Stronger families over the same sets (rank inequalities over a frame's candidates, frame-local
  hulls) were not tried; the register has no row for them.

## Dominance and symmetry (2026-10)

*Question.*  The slope study found a proof's nodes to be about `a + c·G` decades for a root gap `G`, with `c` the
same at every n, so a lower cost per unit of gap cuts the slope as a tighter bound does.  A search that must rule
out every promotion set inside the gap pays separately for every near-optimal variant; the uniqueness study found
optimal sets that differ by exchanging equal-sum groups of lower-half numbers.  How much of a proof's tree is
*dominated* (another promotion set does at least as well with every completion, so the node can be pruned) or
explored twice under interchangeable promotions?  Which exchanges are provably safe, what share of nodes would a
sound rule remove, and does that share hold or grow with n?  IDEAS.md section 5 had dominance rules and symmetry
breaking as considered, not piloted, and "no symmetries found".

*Definitions.*  A node `(P, C)` is dominated by a set `P'` (a candidate set, feasible, disjoint from `C`) when every
completion `S ⊆ C` of the node that could beat the incumbent is promotable from `P'`'s state and `P' ∪ S` is worth
more.  Pruning such nodes is sound: the pruned sets map to feasible sets worth strictly more (for two-for-two
exchanges of equal sums, larger as binary numbers), so the heaviest feasible set, the largest of them in that
order, is never pruned.  The completions that could beat the incumbent are within the node's *relevant* candidates:
those that can be promoted and with which the matching bound, plus themselves, still reaches what the node needs.
`P'`'s state *covers* the node's when, frame by frame, its *core* (the frame for the edits the relevant candidates
can make, less the moves that can always go last and the factors nobody can use) has no move the node's core lacks
and every factor of the node's core that its moves or the incoming candidates can use: then every completion
promotable from the node's state is promotable from `P'`'s (the proof is the comment of `covers` in
`examples/dominance.rs`).  The cover is also tried with moves relabelled (a move stands in for another with the same
live factors) and up to an exchange of two primes (`covers_mapped`).  A *sampled* estimate replaces the proof by 64
random maximal completions within the relevant candidates (infeasibility only grows with a set, so maximal sets are
the ones to try): an upper estimate of what any cover could prove; 1,024 samples gave the same counts within two
nodes in a hundred.

*Method.*  No solver code changed.  `examples/dominance.rs` (commits aa56590 to d2de73f, on the cleaned-up build)
and `examples/common/symmetry.rs`; `bench/dominance/` holds the node samples, the outputs and `dominance.py`, which
makes the tables below (`dominance_output.md`).  One thread, at most two processes, on the shared laptop.

* *Nodes.*  The probe's fixtures (units of the proofs of 936 and 1215, mostly shallow: 13-16 promotions); nodes of
  the solver's own proofs sampled uniformly over their steps (`dominance sample --count K`: the deterministic
  one-thread proof stopped after `k·T/(K+1)` steps and the node it stopped in read back from its checkpoint; 40-100
  nodes each of 900, 936, 950, 1000, 1100, 1184, 1215, 1250, and of 1000 and 1250 with the incumbent at `P*` + 100
  and + 200); and nodes of the GF(2) branch and bound's trees (`matroid bnb --bounds gf2`; one in 60 to one in 200
  of 930, 990, 1020, 1210, 1230 and 1300).  `dominance nodes` tries at each node every exchange of one excluded
  candidate added, one for one (`a > b`), and, with `--pairs`, two for one and two for two; every exchange of two
  primes applied to `P`; twins among the candidates (same home frame, same live factors below, same users above:
  an automorphism of the node's problem); and the symmetry rule below.
* *Two pilots* (code not kept; how to rebuild them is here).  The *solver pilot* copied `src/search/searcher.rs`,
  `refine.rs` and `lagrange.rs` into an example (paths adapted, the diagnostics probe stubbed) and added one hook in
  `Stepper::check`, after the matching bound: the symmetry rule may cut the node (cut reason `refined`).  With the
  rule off it reproduces the proofs node for node (900 4,194; 936 41,653; 950 460,813; 1000 366,846; 1100 29,614;
  1184 52,259; 1250 709,893).  The *GF(2) pilot* copied `examples/matroid.rs`'s frame matroids and its `bnb` loop
  (bound `gf2`) and added the rule after the bound; with it off it reproduces B's trees (936 3,392; 1100 1,874; 1184
  1,093; 1250 2,543).  Both run the incumbent as a value, `P*` or `P*` + δ, as `slope prove` does.

**1. No transpositions, no twins, and literal dominance is rare.**

* *Transpositions*: in the GF(2) branch and bound's trees of 930, 990, 1020, 1210, 1230 and 1300 (133,006 nodes) no
  reduced state is reached by two different promotion sets; the states that repeat (19-21% of the nodes) are the
  conflict branching's siblings, the same `P` with fewer candidates.  Nothing is explored twice literally.
* *Twins*: none at any node of any sample (the candidates' live neighbourhoods differ, as IDEAS.md expected).
* *Adding an excluded candidate* (`P + a`, the "the node went on without `a`, but `a` fits anyway" case): proved at
  no node; at 57% of the fixtures' nodes some excluded candidate can be added and no single relevant candidate
  blocks it, yet some random maximal completion always does.  Excluded candidates stay blockable.
* *Literal covers* (one for one, two for one or two, with or without relabelled moves) at the solver's nodes: 0-1%
  of the fixtures' nodes, 0-5% of the sampled proof nodes; the sampled estimates are far higher (up to 53%), and the
  exchanges behind the gap are the subject of 2.  For example, at a node of 936 the swap 426 → 438 (6·71 → 6·73)
  passes every sample: the families {284, 426, 568, 852} and {292, 438, 584, 876} are the same graph in the node's
  frames, and a literal cover cannot see it because the numbers differ.

**2. The frame model has symmetries: exchanges of primes.**  Exchanging the exponents of two primes `p < q` in
every number is an automorphism of divisibility.  When it maps the root's frames onto themselves (moves to moves,
factors to factors, the factor graph among them onto itself) and the candidates onto themselves, it maps every
promotable set `S` to a promotable set worth `(q − p)·(c_p(S) − c_q(S))` more, `c_r(S)` being the sum of `x / r`
over the members of `S` divisible by `r` (`examples/common/symmetry.rs`, `root_symmetry` and `root_pairs`; no
candidate is divisible by both or by a square in any pair found).  The pairs are found by checking that directly;
they are not just the primes with the same multiples on the board (at 851, 59, 61 and 67 are one class though
851/59, 851/61 and 851/67 have different floors: the frames see only some multiples).  At 1300 the classes are
{41, 43}, {59, 61}, {67, 71}, {73, 79}, {89, 97, 101, 103, 107}, {109, 113, 127, 131, 137, 139} and {149, 151, 157}.

| n | games | candidates | classes of primes | largest class | candidates moved | log10 of the group's order |
|---|---|---|---|---|---|---|
| 851-900 | 50 | 211 | 3.8 | 4.3 | 23.9 (11%) | 4.5 |
| 951-1000 | 50 | 236 | 4.1 | 6.1 | 28.5 (12%) | 5.1 |
| 1051-1100 | 50 | 266 | 4.6 | 6.3 | 28.3 (11%) | 5.2 |
| 1151-1200 | 50 | 289 | 4.6 | 6.7 | 28.9 (10%) | 5.9 |
| 1251-1300 | 50 | 322 | 5.8 | 6.7 | 41.5 (13%) | 6.7 |
| 1301-1350 | 50 | 336 | 5.9 | 5.9 | 48.4 (14%) | 7.4 |

*The rule.*  A set worth beating the incumbent with `c_p > c_q` for a symmetric pair has an image worth more, so
only sets with `c_p ≤ c_q` for every pair need searching, and a heaviest set always keeps them all (otherwise its
image would be heavier): a node can be cut when, for some pair, no completion that keeps the constraint can reach
what the node needs.  The constraints of a class are the order of its primes by their cofactor sums, which is all
that value can say about the class's whole symmetric group.  Three tests of that, from cheap to dear: *hopeless*,
no completion can lift `c_q` to `c_p` (`c_p(P) > c_q(P) + c_q(C)`); the *matching bound of the constraint*, each
subset `U` of the pair's multiples among the candidates (at most 10 or 12) promoted, the others left out, the rest
relaxed (solver pilot and `dominance nodes`); the *Lagrangian of the constraint* on the GF(2) bound, each multiple
`m·p` weighing `m(p − μ)` and `m·q` weighing `m(q + μ)`, plus `μ(c_q(P) − c_p(P))`, for `μ = (q − p)/2` and `q − p`
(a bound on the completions that keep the constraint for any `μ ≥ 0`; GF(2) pilot).  The two dearer tests run only
for pairs whose constraint the bound's heaviest set breaks.

*Checks* (`dominance symcheck --known results/strict.jsonl 2-1300`, `bench/dominance/symcheck.jsonl`): in all 1,170
games with pairs, the images of the optimal set and of 50 random maximal promotable sets are promotable and worth
what the formula says; on the 22 boards with pairs and at most 22 candidates every promotable set (97,839 in all) is
checked and the best that keeps the constraints is `P*`; the brute-force oracle scores the known optimum to n = 40.
Both pilots with the incumbent at `P*` − 1, so that they must find an optimal set themselves, find one in every
game of 2-900 (880 games each; the rule cut in 573 and 353 of them).

**3. What the rule and the covers remove: a sixth to a third of the nodes, a constant share.**  At the sampled
nodes (`dominance_output.md` has the per-game tables): "proved" is any of the covers, the prime exchanges and the
rule's matching test; "sampled" its upper estimate.

| tree | n | proved | sampled |
|---|---|---|---|
| solver proofs | 900, 936, 950, 1000 | 0%, 25%, 32%, 43% | 0%, 46%, 53%, 50% |
| | 1100, 1184, 1215, 1250 | 36%, 8%, 10%, 25% | 39%, 20%, 12%, 35% |
| incumbent `P*` + 100 | 1000, 1250 | 28%, 8% | 52%, 15% |
| incumbent `P*` + 200 | 1000, 1250 | 0%, 12% | 50%, 27% |
| GF(2) branch and bound | 930, 990, 1020 | 54%, 25%, 16% | 55%, 25%, 21% |
| | 1210, 1230, 1300 | 7%, 19%, 35% | 10%, 20%, 39% |

Two-member exchanges add little (sampled, with everything else: 49%, 34%, 48% of the nodes of 936, 1184, 1250; 25%,
7%, 46% of the GF(2) nodes of 1020, 1210, 1300).  In the GF(2) trees what is proved is nearly all that is sampled,
and it is the rule; in the solver's trees the rest of the estimate is single swaps along a symmetry (426 → 438
above, half of an exchange of 71 and 73) and exchanges of primes that are alike only at the node (29 and 31, 17 and
19, 61 and 67 at nodes of 936, where the multiples that tell them apart are decided).

The pilots, nodes with the rule against without, the incumbent at `P*`:

| search, rule | games | nodes, geometric mean ratio | total | fewest | most | log ratio on log nodes | time |
|---|---|---|---|---|---|---|---|
| solver, hopeless | 880-1090 by 10 (22) | 0.962 | 0.960 | 0.85 (950) | 1.07 (1040) | −0.002 | +15% |
| solver, matching bound of the constraint | the same | 0.878 | 0.829 | 0.67 (1000) | 1.16 (940) | −0.028 | ×1.9 |
| GF(2) B&B, hopeless | 880-1300 by 10 (43) | 0.948 | 0.925 | 0.71 (940) | 1.00 | −0.016 | −3% |
| GF(2) B&B, Lagrangian of the constraint | the same | 0.859 | 0.826 | 0.57 (930) | 0.98 (1100) | −0.030 | −3% |

In the solver the rule's cuts change what the searcher learns, so a tree can grow: 1184 52,259 to 66,250 (+27%) with
the matching test, 940 +16%; 1250 709,893 to 678,415 (−4%).  The margin scans of the GF(2) pilot (δ = 0, 25, 50, 100)
keep the ratio per game: 930 0.57, 0.60, 0.61, 0.92; 990 0.77, 0.77, 0.77, 0.79; 1300 0.69, 0.68, 0.67, 0.75.  The
share does not grow with n (the log ratio changes by ±0.02 decades per 100 in n in both pilots) and barely with the tree
(−0.03 decades per decade of nodes: `c` lowered by about 3%), although the symmetric part of the problem does grow
(the table of 2): most rearrangements within a class differ in value by more than the bound leaves, and the bound
already cuts them.

*Conclusion.*  Literal dominance and transpositions are absent from the trees; what makes interchangeable
alternatives is a real symmetry of the frame model, the exchange of primes within classes that the frame builder
treats alike, which IDEAS.md's "no symmetries found" missed.  Breaking it is sound (proved, and checked to 1300) and
removes about a seventh of the nodes on average (geometric mean 0.86 in the GF(2) branch and bound, 0.88 in the
solver), up to 43% in some games: a constant factor, not a lower cost per unit of gap.  Exchanges that hold only
at a node add a little more in the solver's trees and almost nothing in the GF(2) trees, and cost a state comparison
per exchange, far more than a step.

*What it implies.*

* *Not a slope lever*, and not worth a phase of its own.  If the GF(2) bound goes into the search, the symmetry rule
  is a cheap addition there: the hopeless test from cofactor sums kept along the path (nearly free, about 5% of
  the nodes) and the constraint's Lagrangian at the nodes whose heaviest set breaks a pair (about 14%, at one or two
  intersections there; time-neutral in the pilot, where every bound is computed from scratch).  In the matching
  solver the constraint costs more than it saves.
* The pairs are found at the root in about 20 ms a game (the setup included); `root_pairs` is the definition to reuse.
* The equal-sum exchanges of the uniqueness study are not symmetries (e.g. 425's {110, 132, 168} ↔ {98, 150, 162});
  they are coincidences of value that only the bound can rule out.

## Going higher and coming back down (2026-10)

*Question.*  The chain uses the game below: the ceiling opt(n) ≤ n + opt(n − 1) (the cap), the prime formula,
`same_search` and the seed.  Can the games above n (their optima, optimal games, search problems and proofs) tighten
n's upper bound, raise its incumbent or settle it, so that an expensive game is derived from a cheaper one above it?
Since a proof's cost is set by the root's gap ([slope](#what-drives-the-slope-2026-10); the GF(2) gap for a search
built on the [GF(2) bound](#matroid-intersection-bounds-2026-10)), a relation from above counts if it beats that gap
where the searches are dear.

*What was kept.*  Nothing but this entry; the solver is unchanged.  The study's code lived on a branch that was never
pushed and has been deleted, as the project does with research branches once they are harvested.  It was two example
programs (one that set up the boards n and n + 1..n + 3 and measured how their search problems relate and what each
optimal set is worth on the other board; one that tests search problems for equivalence), a table script and the data
they made, and one-line mentions of the programs in DESIGN.md and README.  The method is written out below to be
redone, with the one result that lived only in the code, the sufficient condition for one problem to relax another,
stated and proved.  What it read is on master: the chain (`results/strict.jsonl`), the GF(2) root gaps
(`bench/matroid/roots.jsonl`) and the one-thread proofs (`bench/curve/slope_proofs.jsonl`).

*Method.*  No solver code changed; one search was run (section 7).  For each board n of 2..1297 (primes and trivial
boards included) and each m = n + 1..n + 3, both boards were set up as `taxman solve` sets them up (default tactics:
the root's frames after reduction, the candidates with their home frames, `max_total`, `in_the_bag`), and the optimal
games read from the chain (a game's P\* is its score less `in_the_bag`).  Recorded for each pair:

* whether the two search problems are *equal* (the test of `same_search`, DESIGN.md
  "[Same search](../DESIGN.md#same-search)") and whether each *relaxes* the other (below);
* how m's root differs from n's, frame by frame: the moves and factors m has and n has not and the other way, the
  change of the frame's freedoms, the candidates gained and lost, the change of `max_total`;
* what each optimal promotion set is worth on the other board: the set's members that are candidates there, whether
  they can all be promoted from the root, and the sum after *repairing* (dropping the smallest member of a conflict
  until the rest can be promoted) and *filling* (adding the largest candidates that keep it promotable);
* for n, the incumbent its chain search starts from (the search stopped before its first node, seeded from the game of
  n − 1), and a *look-ahead* start: the incumbent the search of m starts from when seeded from the game of n − 1, which
  an incumbent for n may use (n's own optimum, and m's, may not be), repaired and filled on n's board.

A game is *exhausted* when the chain proved it by exhausting the search (416 of 1..1300).  The tables come from a
script over those records and the files named above; "nodes" are the chain's, except in section 5, which uses the
one-thread proofs.

*When one problem relaxes another.*  Say problem Π′ *relaxes* Π when every promotion set that Π can promote from its
root, Π′ can promote from its own.  Then P\*(Π) ≤ P\*(Π′), and every optimal set of Π is feasible in Π′.  The study
used this sufficient condition, checked on the two roots as the solver holds them (reduced, with home frames; a problem
without candidates is relaxed by every problem):

* (a) every candidate of Π is a candidate of Π′ with the same home frame, and Π′ has at least as many frames; and
* (b) for every frame i of Π: Π′'s frame i has every factor of Π's frame i, and each move a of Π′'s frame i that Π's
  frame i lacks has a maximal factor y that is a factor of Π′'s frame i but not of Π's, and that no other number which
  could be a move of frame i lists among its maximal factors: no other move of Π′'s frame i, and no candidate of Π of
  home i + 1 (which becomes a move of frame i when promoted).

*Proof.*  A set T of candidates can be promoted from a root exactly when each member is a factor of its home frame
h ≥ 1 and every frame i is winnable with T applied all at once: its moves plus the members of T with home i + 1, its
factors minus the members with home i, with no reduction in between (the module comment of `model::promote`;
reduction never changes winnability).  Let T be promotable in Π.  By (a) its members are candidates of Π′ with the
same homes, so T applies to Π′, and we show that each frame of Π′ is winnable with T applied.  Frames of Π′ above
Π's top are untouched by T (no member has its home there or the one above) and winnable at the root, since the empty
set can be promoted.  Take a frame i of Π; let B be Π's frame i with T applied, winnable by hypothesis, and R Π′'s.
Write T_h for the members of T with home h.

* R's factors are Π′'s factors of frame i less T_i, and so contain B's, which are Π's less T_i, by (b).
* R's moves are Π′'s moves of frame i and T_{i+1}; B's are Π's and T_{i+1}.  Let A be the moves of Π′'s frame i that Π's
  lacks.  No a in A is in T_{i+1} (a is a move of Π′'s frame i, and T_{i+1} are factors of its frame i + 1), so the
  moves of R outside A are moves of B.
* Each a in A has the y of (b).  It is a factor of R (it is not a factor of Π's frame i, and T_i consists of factors of
  Π's frame i), and no other move of R lists it (R's other moves are moves of Π′'s frame i or members of T_{i+1}, which
  are candidates of Π of home i + 1).  So y is private to a, and stays private as other moves are dropped.

A frame is winnable exactly when the *peel* drops all its moves: repeatedly drop a move that has a private live factor
(it can be taken last), in any order (`model::peel`).  Peel R by dropping all of A first.  What is left is R without
A: moves of B, over factors that include all of B's.  Winnability only gets easier with fewer moves and more factors
(a winning order of B, restricted to the moves left, still finds a factor of each move in the pot, which only has
more in it), so the rest of the peel drops every move, and R is winnable.  ∎

*Equivalent problems* (section 6) are tested by a separate method, described there.

**1. What the game above proves: opt(n) ≤ opt(n + K) − Σ m/2 − (q − P).**  The sum is over the even m in
(n, n + K], P and q are the largest primes at most n and n + K, and K < (n + 3)/2.  *Proof.*  Let G be an optimal
game of n; it opens with P (every optimal game opens with the largest prime: Lemma A in the proof of
`accelerate::prime_shortcut`).  For an even m in (n, n + K], f = m/2 is above n/2, so it divides nothing else on 1..n,
and its multiples up to n + K are f and m (3f > n + K, since K < (n + 3)/2).  On 1..n + K, play G with each such f
deleted (if G played it) and, if q > P, with its first move P replaced by q (as in the prime shortcut; q > n, as no
prime lies in (P, n]); then every such m.  Deleting a move leaves more in the pot, so every later move of G keeps a
proper divisor; q is inert (2q > n + K) and removes only 1, like P; each f stays in the pot until its m, which it pays,
because the only multiples of f up to n + K are f and m, and no m removes another's f: m′/f = 2m′/m lies in (4/3, 3),
so it is an integer only for m′ = m.  The game scores at least opt(n) + Σ (m − f) + q − P (at least: when f was not
played, or is P, the deleted score was not there to lose).  ∎  With K = 1 it is opt(n) ≤ opt(n + 1) − (n + 1)/2 for odd
n, and the prime shortcut's equality for a prime n + 1; nothing for an odd composite n + 1 = pf (p ≥ 3 its least
prime), where making room for it can cost the moves f, 2f, …, (p − 1)f, as much as it brings.

On the chain it holds for every n and K checked (2..1299, K up to 39), and it is tight 223 times at K = 1 (the 210 odd
primes and 13 even m), 11 at K = 2 and once at K = 3.  The 13 even m have opt(m) = opt(m − 1) + m/2: 8, 12, 16, 24, 32,
48, 64, 96, 256, 384, 512, 768, 1024.  Of the 416 exhausted games, the bound from n + 1 settles 107: 106 through a prime
n + 1, whose search problem is n's own (each of the 204 primes whose predecessor sets a search up poses that problem,
so nothing is learned), and 1023 through 1024.  The slack is large elsewhere: an even n + 1 above an exhausted odd game
gains about 70, 220 and 450 more than (n + 1)/2 at the quartiles (118 games).

**2. Neighbouring problems differ by a local edit, and the edit runs one way.**  From odd n to n + 1 = 2f, f leaves
the moves of a frame h (a freedom more there) and becomes a candidate factor of frame h + 1 that pays for the new move
2f; at 2^k a new top frame appears.  From even n to an odd composite n + 1, n + 1 joins frame 0 or 1 as a new move,
or with a new hive (2f and 3f over f), a freedom less, at times while another hive leaves the frame.  So after an odd
n, n + 1 *relaxes* n or poses the same problem (all but 6 of 434), and after an even n, n relaxes n + 1 or neither
(pairs of games that are neither prime nor trivial, 2..1297):

| n | equal | n + 1 relaxes n | n relaxes n + 1 | neither |
|---|---|---|---|---|
| odd | 226 | 202 | 0 | 6 |
| even | 244 | 0 | 85 | 106 |

A relaxation gives P\*(n) ≤ P\*(m), a cap from above in the frame model (and P\*(m) ≤ P\*(n) when n relaxes m).
Over the 2708 and 1582 relaxing pairs within three of each other (equal problems included), no P\* breaks it and every
optimal set of the relaxed problem can be promoted in the relaxation.

**3. A cap from above is exact only where the problem above is no cheaper.**  172 of the exhausted games have a
relaxation among n + 1..n + 3 that is not their own problem; its cap (P\* of the relaxation) is exact for 5: 30, 45,
510, 765 and 1023, each at a change of the top frame that is worth nothing.  At 32, 512 and 1024 a new top frame
appears (the move 3·2^(k−2) over the factors 2^(k−2) and 3·2^(k−3): 24 over 8 and 12 at 32; candidates never worth
promoting); at 48 and 768 the move 24, resp. 384, becomes a factor (a freedom more in the top frame).  Elsewhere the cap
misses by the worth of what the edit frees (a freedom of frame h, the candidate f), and a cap that misses prunes
nothing: a node is cut by its own bound, and the cap only stops a search whose incumbent reaches it.  At 23 of the 66
games of 880-1297 that have one, the cap from above is closer than the root's GF(2) bound (1235: 12 against 164, 1254
and 1281: 24 against 204 and 206, 1110: 8 against 134), and at the dearest games it is absent or farther.  The 12
dearest exhausted games of 1001-1300 (the root's GF(2) gap is the matroid study's `roots.jsonl`, less P\*; of the 20
dearest 11 have a cap from above and 4 are closer than GF(2)):

| n | nodes (chain) | GF(2) gap | cap from above, its gap |
|---|---|---|---|
| 1025 | 7,953,672 | 147 | 323 |
| 1230 | 12,706,114 | 154 | 532 |
| 1232 | 53,015,201 | 190 | none |
| 1235 | 7,641,761 | 164 | 12 |
| 1236 | 9,727,370 | 164 | none |
| 1264 | 12,573,516 | 176 | none |
| 1269 | 7,708,479 | 167 | 148 |
| 1274 | 10,149,185 | 206 | none |
| 1287 | 24,699,434 | 282 | 309 |
| 1290 | 23,160,535 | 174 | 646 |
| 1295 | 30,184,598 | 174 | 648 |
| 1300 | 31,165,285 | 151 | not measured (1301 unsolved) |

Why it cannot be otherwise: a relaxation has more factors, fewer moves, more candidates, and new moves only with
private new factors (the condition above), so its matching graph and its GF(2) frame matroids contain those of the
problem it relaxes (a move with a private coordinate contracts nothing the others use), and its root bound is at least
as large: none of the 972 relaxing pairs with both root bounds known has a smaller GF(2) or matching bound.  So gap(m)
≥ gap(n) − (P\*(m) − P\*(n)): a problem above that is cheaper than n's own leaves a cap that misses, and an exact cap
comes from a problem at least as hard.  The cheap neighbours above are the restrictions (after an even n), and they
bound n from below only.  The relaxations from below do no better than the chain's cap: 70 exhausted games have one,
none exact and none tighter than the cap.

**4. Incumbents from above are n's own optimum, reflected.**  The chain's search starts below the optimum in 110 of the
415 exhausted games of 2..1297.  The optimal set of n + 1, on n's board, reaches P\* in 91 of them; but n + 1 comes
after n and is often built on it (98 of the 416 exhausted games are followed by a chain game, opt(n + 1) = opt(n) +
n + 1, whose optimal set is n's plus n + 1), so this is n's optimum coming back.  An incumbent n may use, the
look-ahead start of n + 1..n + 3 seeded from the game of n − 1, beats n's own start in 7 of the 110 (462, 736, 837,
1020, 1107, 1110, 1290) and reaches P\* in 1 (1290, from 1292's and 1293's).  Starting at the optimum would save at most
what a start below costs, 0.9-1.8 times the proof at 10-100 below ([slope](#what-drives-the-slope-2026-10), section 4).

**5. The gap is the same across a chain step.**  The 37 exhausted games of 880-1299 followed by a chain game (opt(n +
1) = opt(n) + n + 1) have the same GF(2) root gap as that game ([matroid](#matroid-intersection-bounds-2026-10), section
1): the edit adds the same to the bound and the optimum, so the hard part of the problem does not change, and solving n
+ 1 first would face n's gap.  More broadly, take the exhausted games of 880-1300 in order and cut the sequence where
two consecutive ones are more than 6 apart or have different GF(2) gaps: 77 of the 143 games fall into 32 runs of two to
five games with equal gaps (1235 and 1236, 1254 and 1256, 1274 with 1280, 1281 and 1284, 1290 and 1295, ...).  The
one-thread proofs of a run beyond its dearest add up to 51,990,258 of the 262,711,582 proof nodes of the 143 games
(19.8%).  That is the ceiling of a search that would prove a run's shared part once; equal root gaps do not make equal
trees (proofs within a run differ by up to ×3.9, median ×1.31), and no mechanism was built.

**6. Equivalent problems, a wider same search: almost nothing.**  Two problems with the same feasible sets have the
same P\*, whether or not they are equal; `same_search` sees only equal problems.  The test (run for each non-prime
non-trivial board of 2..1300 against the 8 earlier boards that set a search up, latest first) reduces a problem to a
*form* and compares forms.

* *The form.*  A set T of candidates is promotable exactly when each member is a factor of its home frame and each frame
  is winnable with T applied (the proof above).  A frame's winnability is a property of the bipartite graph between
  its *movers* (its moves and the candidates of home i + 1) and its factors, a mover listing its maximal factors (the
  model's lists, restricted to the frame's factors); so the feasible sets are fixed by the candidates (with homes) and,
  per frame, that graph with the candidates labelled.  The other numbers, the frame's moves and its factors that are
  not candidates, are *anonymous* and can be renamed freely.
* *Stripping.*  In each frame, peel the movers in the frame's *hardest state*, where every candidate of home i + 1 is a
  mover and every candidate factor is gone: repeatedly drop a mover that lists a live anonymous factor no other
  remaining mover lists.  What a peel drops there never matters: the factor it used is anonymous, so every state has
  it, and is listed by no other mover, which holds in any state with T applied (fewer movers, more factors), so the
  same drop is available at the same point, or the mover is a candidate that T does not promote and is absent.  With T
  applied the frame is winnable exactly when a peel of the movers the hardest state could not drop succeeds, over the
  factors they list.  The form of a frame is that remainder: the surviving candidate movers and anonymous movers, the
  factors they list (candidate and anonymous), and the edges.
* *Comparing.*  Two problems are equivalent when they have the same candidates with the same homes, the same number
  of frames, and in each frame an isomorphism of the forms that fixes every candidate (mover and factor) and carries
  the anonymous movers onto anonymous movers and anonymous factors onto anonymous factors, edges onto edges.  Then T
  is winnable in a frame of one problem exactly when it is in the other's, so the feasible sets, and P\*, are the
  same.  (`max_total` is a count every feasible set obeys, so it is not compared.)  The isomorphism is found by colour
  refinement and then checked: an anonymous mover starts with the colour of the set of candidate factors it lists, an
  anonymous factor with that of the candidate movers that list it; a vertex's next colour hashes its colour with the
  sorted colours of its anonymous neighbours, until the number of colour classes stops growing; the anonymous
  vertices of the two frames are paired by sorting on colour (a mismatch of the colour classes means not equivalent),
  and the pairing is *checked* edge by edge: every mover's list must map onto its image's list exactly.  Only a checked
  isomorphism counts, so a hash collision can only lose an equivalence, never invent one, and so can refinement on
  symmetric forms.

Over 2..1300 it finds the 545 equal problems that `same_search` finds, and 8 equivalent ones more: 21 (to 18), 51 (50),
111 (110), 115 (112), 119 (118), 141 (140), 183 (182), 185 (184), each with the same P\*, all below 186, and exhausted
in 18 nodes in all.  The near repeats it misses are not isomorphic: 1083 after 1080, 1183 after 1180 and 1209 after
1208 have the same P\* and candidates, and frame 0 (which still has 76-83 anonymous movers over 102-111 factors after
stripping) differs in one hive (578 and 867 over 289 at 1080, 722 and 1083 over 361 at 1083; 638 and 957 over 319 at
1208, 806 and 1209 over 403 at 1209) or one move (605 at 1180, 1183 at 1183), with every other frame equal.  Any finer
test of equivalence could save at most what such games cost: among the exhausted games with an earlier game within 8
of the same P\* and as many candidates, 1..1000 14 games (11,262 nodes), 1001..1300 three (643,010 nodes).

**7. A chain that comes back down.**  With every relation above, a chain could solve 1024 before 1023 (seeded from
1022) and conclude 1023 from it: Π_1024 relaxes Π_1023 and 1024's promotions are 1023's optimum.  1024's proof costs
the same from 6 below its optimum (one thread, the slope study's `prove` with the incumbent a value: 1,597,518 nodes
at δ = 0 and 1,568,913 at δ = −6), so the saving is 1023's search: 1,730,169 of the 315,916,127 nodes (0.55%) and 5.3
of the 975 s of the exhausted searches of 1001-1300.  Whether the same happens at 1536 or 2048 is beyond
the chain.  No other game of 1001-1300 changes.

*Conclusion.*  Going higher and coming back down does not make the expensive games cheap.  The games above bound n
through relaxations of its problem (for an odd n, n + 1 relaxes it or poses the same problem but for 6 of 434), and a
relaxation's root bound is at least n's own, so an exact cap from above comes from a problem at least as hard as n's,
and a cheaper problem above leaves a cap that misses and prunes nothing.  The exact relations that exist are the
prime formula (whose problem is n's) and changes of the top frame (1023 in 1001-1300).  Incumbents from above are
n's optimum reflected; the look-ahead that does not use it beats the chain's start in 7 of 110 and reaches P\* once.
What the study does show is that neighbouring problems share their hard part: the edit between them changes the bound
and the optimum alike, so the GF(2) gap is the same across every chain step after an exhausted game, and equal across
runs of nearby searched games that hold a fifth of the proof nodes of 880-1300.

*What it implies.*

* *Nothing to implement in the search.*  A proof kind for the exact relaxation from above (1023 from 1024) would
  save 0.55% of 1001-1300; not worth the reordering of the chain it needs now.
* *Shared cores (open)*: a search that proves a run of neighbours with equal gaps together (for a relaxing pair, one
  tree over the relaxation that keeps both problems' incumbents and cuts a node only when both bounds allow) could
  save up to a fifth of the proof nodes at 880-1300; whether the trees overlap enough is unmeasured, and is worth
  measuring only once the GF(2) bound is in the search, which will change the trees (it is the bound whose gap the
  runs share).
* *The bound is the lever, not the neighbours*: since every exact relation above preserves the gap, the slope is set
  by the shared core's gap, and closing it (the GF(2) bound in the search) is what changes the cost of these games.

## The parity bound in the model-free proofs (2026-10)

*Question.*  The model-free proofs of opt(n) ([evidence 3](../evidence/README.md#3-model-free-exact-cross-check-of-optn))
stopped at 989 (and 999-1019): the matching bound overrates the optimum by about 1,170 at n = 1000, and games 990,
992, 993, 995, 996, 1020 and 1023-1027 had spent millions to tens of millions of nodes each without finishing.  The
[matroid study](#matroid-intersection-bounds-2026-10) found a model-free bound with a tenth of that gap: every playable
set is independent in two linear matroids over GF(2), split by the parity of Ω.  Reimplemented in the evidence crate
from its mathematics (no code shared with the solver or the study) and proven there as a theorem, does it carry the
model-free proofs to the end of the committed chain, 1300?  What does a proof cost, and how fast does that grow?

*The theorem.*  THEORY.md, [the parity matroids](../../heuristics/THEORY.md#the-parity-matroids-a-bound-on-the-whole-game)
(Theorem 10): a playable set's vectors of maximal factors, with the unit vectors of its own members up to n/2, are
linearly independent (from a legal order and Theorem 1's lifting argument; for the upper half it is Theorem 7's
lemma), and each vector lies in the coordinates of one parity of Ω, so every playable set is independent in both
parity matroids; at a node of a search, with the forced picks contracted and the barred numbers deleted.

*Method.*  `src/parity.rs` (commits ff714b3, 7efc38b, 44803b6): the two matroids from the board alone, an echelon basis
over GF(2) kept in reduced form whose rows carry the members they sum (a circuit costs one row per coordinate of a
sparse vector), and the heaviest common independent set by Schrijver's shortest augmenting paths (Theorem 41.7) with
Dijkstra on potentials carried as Frank's weight split.  Every value is *certified*: a split w = w1 + w2 is checked
against the local optimality conditions of each matroid over sets of all sizes, so the bound rests on the theorem and
weak duality, not on the algorithm (none failed).  The same split bounds the value with one member barred.
`Prover::with_parity` (`src/game.rs`) runs it at every node after the matching bound: it cuts, bars the free numbers
dependent on the forced picks, forces the members whose barring alone brings it to the target, and the node branches
on a minimal unplayable subset of its heaviest set.  A child starts from its parent's certificate (warm starts, below).
`exact` (`--parity`, on by default) records the root's parity bound for every game and checks it against the recorded
score, and the recorded game independent in both matroids.

*Validity.*  Theorem 10 on every board to 18: every playable subset of 2..n independent in both matroids (enumerated).
The certified intersection equals enumeration on 3,000 random pairs of small GF(2) matroids with random weights and
roles (15,000 solves), and so do warm starts on 7,571 children of random parents.  The prover's brute-force tests run
with the parity bound too (opt(n) for n <= 20 from nothing, the most moves to 20, every set beating a target to 19,
the optimum found from one below it every 7th game to 420, pause and resume at 600).  At the root of every game to 1300
the bound is at least the recorded optimum, the recorded game is independent in both matroids, and the value equals
the matroid study's whole-game bound at all 1,088 games the study measured (`examples/parityroot.rs`).

**1. At the root it overrates the optimum by a tenth of what the matching bound does.**  `examples/parityroot.rs`
over every game 2..1300 (34 s in all): mean gap 2 to 300 (the matching bound's 23), 26 over 301–600 (172), 77 over
601–900 (510), 160 over 901–1000 (945), 132 over 1001–1300 (1,249); exact at 244 games, the last 527.  A solve from
scratch takes 22 ms at 990 and 43 ms at 1300 (400–560 augmentations from the longest independent prefix), against
0.29 and 0.59 s for the study's reference algorithm.  The certificate's split already forces 284 of the 433 members
at the root of 990 and 464 of 565 at 1300.

**2. Every game of the committed chain is proven.**  `exact` with the parity bound on three threads in nine-minute
checkpointed rounds (`data/exact_parity_990_1300.jsonl`, about 1 hour of wall time): the open searches resumed and
finished in 4,000–215,000 more nodes each (990: 89,639 after 70.5 million; 992: 214,329 after 65.2 million; 1026 and
1027: about 4,500), then every game from 1028 to 1300 (the hardest, 1287, 146,855 nodes in 279 s).  With
`data/exact_2_1231.jsonl`: every n in 2..1300.  A second campaign proves every game again from scratch with the parity
bound alone and the final build (`data/exact_parity_2_1300.jsonl`: 557 searches, 1.17 thread-hours on two threads,
199 games already proven by the root's parity bound), and `examples/paritycheck.rs` runs each of the 665 searched
games against one below its optimum: every one finds a set worth exactly V(n), which replays by the rules
(`data/paritycheck_2_1300.jsonl`, 31 minutes).

**3. A proof's cost follows the root's parity gap.**  Over 900–1300, ×10 nodes per 84 points of gap (R² 0.76); against
n, log10 nodes grows 0.16 ± 0.04 and log10 seconds 0.23 ± 0.04 per 100 (R² 0.06 and 0.11, the gap's swings between
bands dominating), about 1.6 ms a node.  The trees are the size of the study's frame-model branch and bound with the
same bound (992: 66,892 nodes from scratch against its 77,567).  The band table and the extrapolation are in
[evidence 3](../evidence/README.md#3-model-free-exact-cross-check-of-optn): at that slope 1300–2000 would take about
57 thread-hours, the hardest single proofs about 3 hours; the recorded optima, not the search, stop the proofs at 1300.

**4. Warm starts: kept, after one failed form.**  A child starts from its parent's certified set and split.  The split
stays feasible on every circuit arc of the child (forcing members of a heaviest set or barring elements only removes
vertices and shortens circuits), so only the root's arcs (adding an element, dropping a member) can be violated, and
the set is improved by shortest cycles through the root.  With Dijkstra on the inherited potentials 36% of the warm
starts at 990 fell back to a solve from scratch: the parent's set minus a barred member is not the heaviest set of
its size, so after an exchange new circuit arcs can violate the potentials (Schrijver's update needs extremality).
Label-correcting searches (a vertex scanned again when its distance falls, giving up after 20 scans a vertex on
average) cut the fallbacks to 2–12% (n = 1250 and 990), at 0.7 improving cycles a warm start against 15–40
augmentations from scratch: the same trees twice as fast (990: 61 s against 122 s; the 3,215 nodes of 1250: 4.6 s
against 13.6 s).  Forced numbers outside the parent's set, which the warm start cannot use, occurred once in 7,084.

**5. Measured, not kept.**  The Lagrangian bound over the nogoods beside the parity bound cut nothing at 990 (the same
44,411 nodes, 7% slower), so `exact` turns it on only without the parity bound.  Resuming the matching bound's
checkpoints costs more than starting afresh (990: 89,639 more nodes and 232 s, against 44,411 nodes and 122 s from
scratch; 992: 214,329 against 66,892): the old open subtrees follow the matching bound's branching; the campaign
resumed them anyway, so the old searches are part of the proofs (`"matching_nodes"` counts their nodes).

*Conclusion.*  The parity matroids' bound, proven as THEORY.md's Theorem 10 and reimplemented independently with a
certificate checked on every value, carries the model-free proofs of opt(n) from 989 to the end of the committed
chain: the frame model's answer is the true optimum for every game n ≤ 1300, each proven twice (to 989 by each bound,
from 990 by a resumed and a fresh search), at about a thousandth of the matching bound's nodes where both ran.  Its cost grows
×1.7 per 100 in n, so the reach is limited by the recorded optima, not by the laptop.

## The anatomy of the gap (2026-10)

*Question.*  Where does the root's gap live (by frame, by kind of candidate), which rule of the game do the
relaxations' heaviest sets break, which part grows with n, and which valid inequalities cut it?  Begun against the
matching and Lagrangian bounds; when the GF(2) bound of the [matroid study](#matroid-intersection-bounds-2026-10)
arrived, re-aimed at the gap that remains above it.  Run in parallel with that study, on the same laptop.

*What was kept.*  Nothing but this entry and the next ([the per-frame bound](#the-per-frame-bound-2026-10)); the solver
is unchanged.  The code of both lived on a branch that was never pushed and has been deleted, as the project does with
research branches once they are harvested.  It was one example program (the measurements, a small simplex solver of its
own for the LPs, and the per-frame bound's module), a table script and the data, and the move of the matroid study's
frame matroids out of `examples/matroid.rs` into a file the two examples shared.  The methods are written out here to
be redone.  What the studies read is on master: the built-in optimal games, the probe's fixtures
(`testdata/perf/n0936.nodes`, `n1215.nodes`), the matroid study's frame matroids and weighted intersection
(`examples/matroid.rs`, `examples/common/meet.rs`) and its best completions at the fixtures' nodes
(`bench/matroid/nodes_exact.jsonl`), and the slope study's root Lagrangian bounds and 19 margin-scan games
(`bench/curve/slope_roots.jsonl`, `slope_proofs.jsonl`).

*Method.*  No solver code changed.  Games are set up as `taxman solve` sets them up (default tactics); `S*` is the known
game's moves that are candidates and `P* = sum(S*)`, from the built-in optimal games.  One thread a process, at most two
processes; the cut separations stop at time limits, so a loaded machine finds slightly fewer cuts.  The program
computed:

* *Heaviest sets against the optimum.*  The heaviest set of the matching bound (a min-cost flow of its own over the
  matching bound's graph: every move matched to a distinct live factor of its frame, a candidate promoted when matched
  as a mover of the frame below its home, weighing its number) or of the GF(2) bound (the matroid study's
  intersection), with its ties broken towards `S*` (weights `1024·x + [x ∈ S*]`, so the errors shown are the least the
  bound's own ties allow), against `S*`: the members it has that `S*` lacks (*swaps*), by home frame.  Applying the set
  to the root's frames (a frame gains the set's members of the frame above as moves and loses its own members as
  factors), the *core* of each frame it leaves unwinnable (the moves a peel cannot drop), the core's GF(2) rank over its
  movers' live-factor vectors (an *odd* core has rank equal to its size: independent), and the movers' live-factor
  counts.
* *The matching bound as an LP.*  A variable per edge of the matching graph (a mover to a live factor of its frame, a
  candidate weighing its number, the moves weighing a constant larger than every sum so that all are matched), a row
  per vertex (at most one edge each), solved by a dense-tableau simplex in floating point (primal, and dual to
  re-optimize after cuts).  A candidate's promotion variable `z_x` is the sum of its edges as a mover.  *Cover cuts*
  `Σ_K z ≤ |K| − 1`: for the LP's solution, the candidates with `z ≥ t` at `t` = 0.999, 0.9, 0.75, 0.5, 0.3, 0.1, 0.01
  form a set; up to 12 times, a conflict of it (`Model::conflict_using`: a minimal subset that cannot be promoted from
  the node's state) is added if violated, and its largest member removed from the set; rounds until none is violated.
  *Rank cuts* `Σ_T z ≤ r` for unions `T` of the connected groups of fractional candidates linked by tight cuts, their
  supports, and the unions of two or three tight cuts that share a fractional member (at most 26 members; 3 s of
  separation a round, 30 s a root, the 60 most violated added).  With `l = Σ_T z` the cut `Σ_T z ≤ r` for the largest
  integer `r < l` is valid when no promotable set holds more than `r` members of `T`, and that is proved by exhaustion:
  a subset of a promotable set is promotable, so such a set would be `T` with at most `|T| − r − 1` members left out,
  and the check tries every way: if the set at hand cannot be promoted, take a conflict of it (one of its members must
  be out), and branch on which, each branch keeping the members tried before it so that no choice is explored twice,
  down to the allowed number of leave-outs (a budget of 3,000 conflict computations; past it the cut is not made).  A
  greedy promotable subset of `T` as large as `l` rules the cut out at once.
* *The GF(2) bound with cuts.*  The LP over the convex hull of the two frame matroids' common independent sets (of at
  most `max_total` members), with cuts `Σ_T z ≤ r`, by column generation.  The master is the dual: variables `θ` and
  `π_c ≥ 0` for each cut `c`, minimize `θ + Σ_c r_c π_c` subject to `θ + Σ_c π_c |S ∩ T_c| ≥ sum(S)` for each generated
  set `S` (a column; the empty set to start from, costs perturbed by about 10⁻⁶ against degeneracy).  Each step prices
  with one weighted intersection for the weights `64x − Σ_{c ∋ x} π_c`, the multipliers rounded down to 64ths, so that
  it is exact in integers; that set's weight plus `Σ r_c π_c` is a Lagrangian bound, an upper bound on every promotable
  set's sum for any `π ≥ 0` (weak duality), and every step's value is kept, the least reported.  The set becomes a row
  of the master (so a column of the primal) unless the master's multipliers already satisfy it (within 10⁻⁷) or it was
  generated before, which ends the generation, as do 2,000 steps.  The master's solution, `z = Σ μ_S 1_S` for the duals
  `μ` of its rows, is where cuts are separated, in two phases (cover cuts to exhaustion, then cover and rank cuts
  together), and each call starts from the sets generated so far.  Also with the matching LP's cuts added.
* *Nodes.*  At every 10th node of the fixtures (30 each, the units that the solver's proofs of 936 and 1215 handed to
  its searcher), the same bounds next to the target `P* − sum(promoted)`: a node is cut when a bound is at most it, and
  "open" is the mean of `max(0, bound − target)`.  The root's cuts are those of the matching LP and of the GF(2) hull's
  separations together; projected to the node they are `Σ_{T ∩ cands} z ≤ r − |T ∩ promoted|` (kept when they can bind;
  a node with more than `r` of `T` promoted has no completion), valid at every node; the *root's multipliers* are the
  `π` the root's column generation ended with, giving one intersection a node (weights `64x − Σ π`, plus `Σ π (r − |T ∩
  promoted|)`); the *node's multipliers* are those of column generation on the node's own hull with the projected cuts;
  and cuts separated at the node itself, as at a root.
* *A plain branch and bound* with these bounds: the matroid study's skeleton (section 2 there: one bound, no refinement,
  forced members or nogoods; a node is cut when its largest candidates or its bound fall below what it needs; else the
  node branches on the members of a conflict of the GF(2) bound's heaviest set, "`k₁ … k_{j−1}` promoted, `k_j` not"),
  the branching the same in every kind so that the trees differ only by what the bound prunes.  A node is cut when the
  GF(2) bound does; where it does not, the Lagrangian bound over the root's cuts at the root's multipliers (one
  intersection), or the LP over the node's hull and the projected cuts, is tried.  The incumbent is `P*`, proving the
  optimum, on the 19 games of the slope study's margin scans (900, 936, 950, 992, 1000, 1025, 1050, 1088, 1100, 1150,
  1184, 1196, 1215, 1225, 1240, 1250, 1254, 1288, 1300).

The tables below are means by band of 50 over the 361 non-prime games of 880-1300 (the first band is 880-900), the
trends are least-squares slopes per 100 in n with their standard errors.  *Validity*: every bound is an upper bound by
weak duality over valid cuts (a cover cut is a conflict of its node, a rank cut's rank is proved); at all 361 roots of
880-1300 and 309 boards of 2-400 none is below `P*` (the GF(2) bound with all the cuts is within 1 of it at 305 of the
309, the GF(2) bound alone at 177), and at the 39 probe nodes for which the matroid study found the best completion,
none is below it.

**1. Both relaxations promote as many as the optimum, but the wrong ones.**  The heaviest set of the matching bound, and
that of the GF(2) bound, has exactly as many members as `S*` at all 361 roots; they differ by swaps (the matching
bound's 8-18 a game, the GF(2) bound's 2-14; band means 9-15 and 3.5-8).  The freedoms and `max_total` never bind; the
error is entirely which candidates, never how many.

| n | matching gap | its excess by home frame 1, 2, 3 | its cores in frames 0-3 (movers), odd ones | GF(2) gap | its excess by home frame 1, 2, 3 | its cores in frames 0-3, odd ones |
|---|---|---|---|---|---|---|
| 851-900 | 432 | 288, 105, 22 | 41, 53, 6, 15; 21 of 58 | 88 | −228, 278, 22 | 0, 49, 15, 15; all |
| 951-1000 | 749 | 338, 346, 176 | 53, 51, 37, 17; 0 of 172 | 172 | −115, 254, −65 | 0, 54, 32, 6; all |
| 1101-1150 | 522 | 345, −216, 362 | 57, 39, 44, 10; 7 of 242 | 107 | 12, 214, −118 | 0, 34, 43, 0; all |
| 1251-1300 | 1277 | 794, 1169, −769 | 62, 66, 53, 16; 28 of 207 | 185 | 45, 434, −294 | 0, 60, 40, 12; all |

(Band means; "home frame h" are the candidates promoted out of frame h into frame h − 1.)  The matching bound's excess
grows in homes 1 and 2, into frames 0 and 1, whose cores are mostly *even* (GF(2)-dependent; 87 of its 1,602 cores are
odd): exactly what the GF(2) bound removes.  What the GF(2) heaviest set breaks is winnability alone, through cores that
are all odd (all 841 of them), in frames 1-3 and never frame 0: frame 1's holds 34-60 movers, frame 2's 15-43, over as
many factors as movers or a few more; 36-42% of their movers have two live factors, 42-48% three, 5-9% four or more.
Its excess is heavier promotions from frame 2 into frame 1, partly paid back by lighter ones from frame 3.  *What grows
with n* is the size of these odd cores and what they hold: the GF(2) heaviest set's conflict has 12.9 members on
average, growing 1.1 ± 0.1 per 100 in n, and its cores hold 20.6 of its promotions, growing 1.8 ± 0.2 (16 at 851-900,
26 at 1251-1300).

**2. Cover cuts are the Lagrangian bound; their LP is half-integral, and rank cuts close most of the rest of the
matching gap.**  The matching LP with separated cover cuts (24-66 of them, mean 41, tens of milliseconds) gives at every
band what the slope study's root Lagrangian reached in up to 8,000 steps, within 27 of it (band means 114-472 against
112-471; 1 to 27 above, once 2 below), as the exact separation of [How far can nogoods
go](#how-far-can-nogoods-go-2026-10) found too (its LP over every conflict is 0-12 below that Lagrangian): the root's
conflicts are worth that much and no more.  Its solutions are half-integral over clusters of overlapping conflicts: at
936, the pairwise conflicts {370, 410}, {410, 430} and {370, 430} each hold 1 with all three at ½, though at most one of
the three can be promoted.  Rank cuts on the unions of tight cuts (`{370, 410, 430} ≤ 1`, and unions of 20-26 members
with ranks 2-7, mostly 3-4, below their size, in homes 1 and 2) take the gap to 3-15% of the matching gap:

| n | matching gap | cover-cut LP gap | rank-cut LP gap | GF(2) gap |
|---|---|---|---|---|
| 851-900 | 432 | 114 | 17 | 88 |
| 951-1000 | 749 | 288 | 84 | 172 |
| 1101-1150 | 522 | 165 | 25 | 107 |
| 1151-1200 | 629 | 159 | 19 | 108 |
| 1201-1250 | 1091 | 430 | 148 | 148 |
| 1251-1300 | 1277 | 472 | 182 | 185 |

Over the 361 games the rank-cut LP's gap averages 81 and grows 26 ± 3 per 100 in n, the GF(2) bound's 136 and 4.6 ±
2.4: the rank cuts beat the GF(2) bound in every band below 1200 and match it above, and at the probe's nodes they cut
28 of 30 at both 936 and 1215 (the cover-cut LP 23 and 17, the matching bound none).  The GF(2) bound supersedes this as
a bound to build (it is exact on two-factor movers and grows a fifth as fast); the cuts' value is on top of it.

**3. On top of the GF(2) bound, cuts close two thirds of its root gap on average, a falling share.**

| n | GF(2) gap | + cover cuts | + rank cuts | + the matching LP's cuts too | that over the GF(2) gap |
|---|---|---|---|---|---|
| 851-900 | 88 | 28 | 11 | 7 | 0.08 |
| 901-950 | 147 | 78 | 36 | 14 | 0.09 |
| 951-1000 | 172 | 115 | 88 | 59 | 0.34 |
| 1001-1050 | 156 | 111 | 89 | 78 | 0.50 |
| 1051-1100 | 81 | 41 | 32 | 25 | 0.30 |
| 1101-1150 | 107 | 58 | 31 | 21 | 0.20 |
| 1151-1200 | 108 | 51 | 33 | 15 | 0.14 |
| 1201-1250 | 148 | 111 | 101 | 79 | 0.54 |
| 1251-1300 | 185 | 136 | 112 | 101 | 0.54 |

The last column's gap averages 47 and grows 13.9 ± 1.7 per 100 in n (the cover cuts alone: mean 84, +9.1 ± 2.0; with
rank cuts, 62, +13.2 ± 1.9), three times as fast as the GF(2) gap: the cuts remove most of the residual where the odd
cores are small and less than half where they are large.  The cover cuts alone, separated here by rounding, close 38%
of the GF(2) gap; separated exactly (every conflict), 46%, a share falling with n, with a residual growing 8.4 ± 2.0
([How far can nogoods go](#how-far-can-nogoods-go-2026-10)); with the rank cuts, 54%, and with the matching LP's cuts
too, 66%.  Unions of up to 48 members with 20,000 conflict computations a check move 1215 by 6 and 1300 not at all (the
time limit then allows fewer rounds).  The cover cut of a 13-member conflict is weak (the LP can keep 12/13 of each
member), the alternatives to one odd core are many other odd cores of nearly the same candidates, and the unions of two
or three cuts that a rank check can afford do not reach the family.  A counting cut was derived and not built (a set
`U` of factors and the movers whose live factors all lie in it: at most `|U| − 1` of them when each has two live
factors or more, at most `|U| − d₁ − d₂ + 3` with `d₁ ≤ d₂` the two smallest live-factor counts, since the first mover
played takes all its factors and the second shares at most one with it): it cuts a core that is square, and a rank cut
on the same candidates is at least as strong (its right-hand side is the exact rank), so the rank cuts measured bound
what such counting families do on the sets separated.

**4. At search nodes the root's cuts help when the multipliers are the node's own.**  Every 10th node of the fixtures
(30 each; cuts of 30, and "open"):

| n | GF(2): cuts, open | root's cuts, root's multipliers | root's cuts, node's multipliers | cuts separated at the node |
|---|---|---|---|---|
| 936 | 20, 25.7 | 24, 4.2 | 26, 2.0 | 27, 4.1 |
| 1215 | 14, 53.5 | 17, 31.6 | 22, 11.5 | 27, 7.3 |

(The last column is cover and rank cuts; cover cuts alone: 26, 6.9 and 22, 16.0.)  In the plain branch and bound,
against the GF(2) bound alone (nodes; the per-frame column is the [next entry](#the-per-frame-bound-2026-10)'s):

| n | GF(2) gap | with cuts | GF(2) | cuts, root's multipliers | cuts, node's multipliers | per-frame | s: GF(2) | per-frame |
|---|---|---|---|---|---|---|---|---|
| 900 | 119 | 5 | 740 | 254 | 78 | 1 | 0.2 | 13.3 |
| 936 | 132 | 31 | 3,392 | 2,236 | 425 | 467 | 1.0 | 33.0 |
| 950 | 176 | 58 | 8,584 | 7,823 | 1,326 | 5,606 | 2.5 | 175.7 |
| 992 | 318 | 143 | 77,567 | 75,371 | 32,790 | 23,011+ | 29.8 | 240.4 |
| 1000 | 214 | 101 | 10,120 | 9,467 | 3,663 | 9,339 | 4.0 | 158.0 |
| 1025 | 147 | 90 | 28,033 | 27,083 | 15,363 | 5,241+ | 11.3 | 240.4 |
| 1050 | 106 | 26 | 1,171 | 1,108 | 247 | 613 | 0.7 | 27.7 |
| 1088 | 81 | 16 | 289 | 271 | 140 | 253 | 0.1 | 15.5 |
| 1100 | 178 | 79 | 1,874 | 1,759 | 1,038 | 1,847 | 1.0 | 29.9 |
| 1150 | 77 | 2 | 251 | 167 | 29 | 230 | 0.1 | 30.3 |
| 1184 | 139 | 6 | 1,093 | 393 | 33 | 876 | 0.6 | 28.6 |
| 1196 | 174 | 58 | 7,825 | 6,278 | 1,174 | 2,994 | 4.6 | 81.2 |
| 1215 | 213 | 123 | 24,516 | 23,412 | 7,104 | 5,120 | 16.6 | 216.1 |
| 1225 | 105 | 63 | 3,306 | 3,172 | 1,123 | 1,922+ | 2.5 | 240.3 |
| 1240 | 86 | 41 | 1,486 | 1,184 | 394 | 719 | 1.1 | 58.9 |
| 1250 | 126 | 40 | 2,543 | 1,691 | 273 | 2,284 | 1.8 | 87.3 |
| 1254 | 204 | 105 | 18,956 | 17,093 | 7,551 | 5,407+ | 13.6 | 240.3 |
| 1288 | 209 | 153 | 13,831 | 13,377 | 5,609 | 9,345+ | 9.7 | 240.1 |
| 1300 | 151 | 112 | 17,136 | 16,901 | 10,128 | 7,817+ | 11.5 | 240.3 |

("With cuts" is the root's gap with all the cuts, section 3; + marks a run stopped at 240 s.)  The root's cuts at the
node's multipliers need 4.2 times fewer nodes (geometric mean over the 19 games; 33 times at 1184, 9.3 at 1250, 1.7 at
1300, 2.4 at 992) in 1.9 times the time (a column generation a node); at the root's multipliers, one intersection more a
node, 1.26 times fewer nodes (1.0-2.9) in 1.3 times the time.  Neither changes the slope of log10 nodes in n (0.046 ±
0.131 per 100 with the GF(2) bound alone over these 19 games, 0.080 ± 0.169 with the cuts at the node's multipliers,
0.069 ± 0.145 at the root's).  The root's gap reduction overpredicts: at 0.011 decades per unit of GF(2) gap (the
matroid study's rate) the root's 90 units on average predict ten times fewer nodes, and 992's 175 a factor of 90 against
the 2.4 measured: the root's cuts are about the root's cores, and deep nodes meet others.  The same holds for the cover
LP over every conflict solved at every node (nogoods: 6.2 times fewer nodes over the 146 proofs, less as n grows, a
quarter slower), and for the Lagrangian over nogoods beside the parity bound in the model-free proofs, whose multipliers
are fitted at the root and which cut nothing at 990 ([parity](#the-parity-bound-in-the-model-free-proofs-2026-10),
section 5).

*Conclusion.*  Both relaxations get the number of promotions right and the choice wrong.  The matching bound's error
is mostly even cores in frames 0-2 (the GF(2) bound's territory); what is left above the GF(2) bound is odd cores of
two- and three-factor movers in frames 1-3, never frame 0, and what grows with n is their size and the promotions
inside them.  Cover and rank cuts on top of the GF(2) bound close 91-92% of its root gap at 851-950 but 46% at 1201-1300
(both separations), and in a branch and bound give a constant factor of about 4 in nodes with multipliers refitted at
every node (1.26 with the root's), not a lower slope.

*What it implies.*

* *Superseded*: the matching-side anatomy and the rank-cut LP over the matching bound, as a bound to build, by the
  GF(2) bound.  They stand as measurements: cover cuts are worth what the converged Lagrangian is worth (as the
  nogoods study found by exact separation), and rank cuts on their unions are what is beyond them.
* *For the GF(2) search (phase 2 of the matroid study)*: carrying the nogoods and the Lagrangian over onto the
  intersection is worth up to a factor of about 4 in nodes if its multipliers are refitted near the nodes, and
  little with multipliers fitted at the root; whether a cheap refit (a few column-generation steps along the path)
  pays in time is the measurement to make there.  Rank cuts add to cover cuts mostly below 1200.
* *For the slope*: the lever left is a relaxation that sees odd cores of a frame, for frames 1 and 2, which neither
  GF(2), Lorea's matroid, cover cuts nor rank cuts of small unions do.  An exact per-frame bound (each frame's
  winnable sets' convex hull, the frames coupled through their shared candidates by a Lagrangian decomposition, with
  an exact search per frame as the pricing problem) is the next candidate: measured in
  [the next entry](#the-per-frame-bound-2026-10).

## The per-frame bound (2026-10)

*Question.*  [The anatomy](#the-anatomy-of-the-gap-2026-10) found that what the GF(2) bound misses is winnability in
odd cores of frames 1-3.  Does a bound exact frame by frame close a share of that residual that holds or grows with
n, at the roots and at nodes; what does it cost; and what does it do in the plain branch and bound?  (Its code went with
the previous entry's: see "What was kept" there.)

*The bound.*  A candidate `x` of home `h` is a mover of frame `h − 1` and a factor frame `h` loses, and a set can be
promoted exactly when every frame is winnable with its part of the set applied (the frame-by-frame criterion of
`model::promote`).  A frame's *configurations* are the sets of the candidates touching it (those of home `i + 1`, which
become moves of frame `i`, and those of home `i`, which leave its factors) that keep it winnable.  Give `x` a copy in
each of its two frames and a multiplier `μ_x` on their equality, weighing it `μ_x` in frame `h − 1` and `x − μ_x` in
frame `h`.  For every `μ`, `L(μ) = Σ_i max { w_i(C) : C a configuration of frame i }` bounds every promotable set `S`
(weak duality: a promotable `S` gives each frame the winnable configuration `S ∩ (candidates touching it)`, and the
weights add to `sum(S)`).  With every frame's configurations relaxed to the sets independent in its GF(2) matroid (the
matroid study's frame matroids: the even frames' make one matroid `A`, the odd frames' another, `B`, and each candidate
is in one of each), `min_μ L` is the GF(2) bound by Frank's weight splitting for the common independent sets of two
matroids (the maximum weight is the least over splits `w = w₁ + w₂` of the heaviest `A`-independent set for `w₁` plus
the heaviest `B`-independent set for `w₂`); so with winnability where it is exact, and GF(2) elsewhere, the least
`L` is at most the GF(2) bound.

*Method.*  A frame's problem (the heaviest winnable configuration for weights `w`, only members of positive weight) is
solved by its own branch and bound.  A node fixes some members in and some out.  Its bound is the heaviest
configuration independent in the frame's GF(2) matroid with those fixed (the fixed members first, then greedy in order
of weight, a member that is dependent on those before it skipped), which every winnable one is; a node whose fixed
members are dependent, or whose bound is not above the incumbent, is dropped.  If that configuration is winnable it is
the node's best; if not, a minimal conflict of the frame (a subset that leaves it unwinnable, made minimal by dropping
members from the smallest while it stays unwinnable) is branched on, "`k₁ … k_{j−1}` in, `k_j` out" for each `j`,
depth first.  The incumbent starts from the heaviest of the frame's winnable configurations found for earlier weights
(a pool of up to 400 a frame) and from a dive (the lightest member of a conflict dropped until the frame is winnable).
Past a budget of nodes the answer is the largest bound left open, still an upper bound.  Frames 1-3 are exact and the
others use their GF(2) matroids: frame 0's exact problem hits 20,000 nodes at most multipliers past 1000, and the GF(2)
bound's heaviest set has no core there.  The budget is 100,000 nodes a frame at the roots and the probe's nodes, 20,000
in the branch and bound.

The multipliers start at a weight split of the GF(2) bound for the weights `64x` (the matroid study's `split`: with
GF(2) in every frame `L` is then the GF(2) bound, which every run checks) and move by subgradient steps: the subgradient
of `L` at `μ` for candidate `x` is whether the frame below its home takes it, less whether its home frame does; Polyak's
rule towards `P*` (or the node's target), step `θ (L − target)/‖g‖²` with `θ` starting at 1, halved and the multipliers
reset to the best after 20 steps without a lower value, ending when `θ < 10⁻³`.  At the probe's nodes the starting point
is the root's multipliers from such steps at the root (200 for the table's roots, 150 in the branch and bound), and a
node gets 20 steps of its own (ending when `L` is at most the target).  *Column generation* was tried in place of the
steps: the master is the LP over the frames' configurations tied by the copies (variables `π_i` per frame, `μ⁺_x` and
`μ⁻_x ≥ 0` for `μ = μ⁺ − μ⁻`, minimize `Σ π_i` subject to `π_i ≥ w_i(C)` for each configuration `C` generated for frame
`i`), each step evaluating `L` at the master's multipliers and adding every frame's best configuration as a row, the
multipliers kept within a *box* of ±8 around the best point found so far (the box moves to each better one) and the
generation ending when the best value is within 0.25 of the master's optimum.

*Validity.*  Weak duality for any multipliers, given each frame's answer is an upper bound on its heaviest winnable
configuration.  On 141 boards of 2-200, with 20 random multiplier vectors each (between −x/2 and 3x/2), the frame
search agrees with trying every subset on all 9,401 frame problems of at most 20 positive-weight members, and `L` is
never below `P*` (2,820 values); at the 19 roots it is at least `P*`, and at the 39 probe nodes whose best completion
the matroid study found, at least that.

**1. At the root it closes 59% of the GF(2) gap within 200 steps, a share falling with n.**  The 19 games of the slope
study's margin scans, frames 1-3 exact:

| n | GF(2) gap | per-frame gap | closed | GF(2) with cuts | s |
|---|---|---|---|---|---|
| 900 | 119 | 0 | 1.00 | 5 | 13 |
| 936 | 132 | 13 | 0.90 | 31 | 142 |
| 950 | 176 | 98 | 0.44 | 58 | 265 |
| 992 | 318 | 257 | 0.19 | 143 | 351 |
| 1000 | 214 | 128 | 0.40 | 101 | 282 |
| 1025 | 147 | 84 | 0.43 | 90 | 210 |
| 1050 | 106 | 23 | 0.78 | 26 | 20 |
| 1088 | 81 | 8 | 0.90 | 16 | 35 |
| 1100 | 178 | 68 | 0.62 | 79 | 138 |
| 1150 | 77 | 46 | 0.41 | 2 | 164 |
| 1184 | 139 | 46 | 0.67 | 6 | 34 |
| 1196 | 174 | 34 | 0.81 | 58 | 72 |
| 1215 | 213 | 60 | 0.72 | 123 | 123 |
| 1225 | 105 | 45 | 0.58 | 63 | 348 |
| 1240 | 86 | 17 | 0.80 | 41 | 119 |
| 1250 | 126 | 46 | 0.63 | 40 | 168 |
| 1254 | 204 | 125 | 0.39 | 105 | 172 |
| 1288 | 209 | 141 | 0.32 | 153 | 292 |
| 1300 | 151 | 129 | 0.15 | 112 | 235 |

("GF(2) with cuts" is the anatomy's root gap with all its cuts, section 3 there; 900 stopped at 136 steps, having
reached `P*`.)  It closes 15-100% of the GF(2) gap, 59% on average, with a trend of −5 ± 4 points per 100 in n.  At the
GF(2) split's multipliers alone it is the GF(2) bound or up to 11 below it (the frames find winnable configurations as
heavy as the split's ties allow); all the gain is the multipliers'.  The steps are the cost: an evaluation solves frames
1-3 exactly, 0.05-2 s (frame 2 of 1150 takes 100,000 nodes at the split, and a frame's search hits the budget at the
split at 992, 1025 and 1150, the answer still a bound), and the subgradient stalls short of convergence at the frontier
(1254: 37959 after 200 steps and 37958 when the step had been halved ten times, after 272, at a gap of 125).  Column
generation does no better: without the box its master's multipliers swing to extremes (`L` of 10⁵ to 3·10⁶ at 936) and
nothing improves in 100 steps; with it, 936 reaches 20528 in 400 steps (1,036 columns, 410 s), against 20520 in 200
subgradient steps (142 s).  How much the converged bound would close at the frontier is not known; what can be computed
in minutes a root closes less there.

**2. At the probe's nodes it is the strongest bound measured, at hundreds to thousands of times the cost.**  Every 10th
node; cuts of 30, and the mean of `max(0, bound − target)`:

| n | GF(2) | per-frame, root's multipliers | per-frame, 20 steps of the node's own | s a node |
|---|---|---|---|---|
| 936 | 20, 25.7 | 25, 5.9 | 29, 0.8 | 0.10, 0.66 |
| 1215 | 14, 53.5 | 23, 24.9 | 27, 5.9 | 0.46, 2.14 |

The root's cuts of the anatomy at the node's multipliers cut 26 and 22 ([there](#the-anatomy-of-the-gap-2026-10)); the
GF(2) bound costs 0.3-0.7 ms, so the two per-frame columns cost 300 to 3,000 times as much.

**3. In the branch and bound it is a constant factor at many times the time.**  The anatomy's skeleton (the table in
section 4 there) with the per-frame bound at the root's multipliers (150 steps at the root) where the GF(2) bound does
not cut, the frames' pools seeded with the root's winnable configurations: 13 of the 19 games finish within 240 s,
with 2.8 times fewer nodes than the GF(2) bound alone (geometric mean; 740 times at 900, whose root it cuts, otherwise
1.0-7.3) in 28 times the time; the root's cuts at the node's multipliers need fewer nodes than it at 11 of those 13 (not
at 900 and 1215).  The six that do not finish are the hard ones (992, 1025, 1225, 1254, 1288, 1300), so no slope can be
fitted; at 1215 it needs 5,120 nodes against the GF(2) bound's 24,516 and the cuts' 7,104, in 216 s against 17 s and
31 s.

*Conclusion.*  The hypothesis holds where the bound can be computed: exact frames see the odd cores, closing most of
the GF(2) residual at 900, 936, 1088 and at the probe's nodes when the multipliers are the node's own.  But computing
it means solving each frame's own promotion problem exactly, and those problems hold the same odd cores: a frame's
search takes up to 100,000 nodes, the multipliers need hundreds of evaluations, and within budgets the share closed
falls at the frontier (15-39% at 1254-1300).  The residual's difficulty moves into the pricing problem rather than
going away.  In the branch and bound it is a constant factor at 28 times the time, behind the GF(2) bound with
learned cuts.  Not recommended.  A frame-local relaxation that sees odd cores cheaply (a bound for one frame
stronger than its GF(2) matroid and computable without search) would make it practical; none is known (Lorea's
matroid ([matroid](#matroid-intersection-bounds-2026-10), section 5), cover cuts and rank cuts of small unions do not
see the cores).

## The GF(2) bound in the search (2026-10)

*Question.*  The [matroid study](#matroid-intersection-bounds-2026-10) found a bound whose gap at the root is a tenth
of the matching bound's and grows a thirtieth as fast, and a plain branch and bound with it, computed from scratch at
every node, that needs 12 to 613 times fewer nodes than the solver.  Can the search use it as its node bound at a cost
per node that pays, what of the solver's machinery still earns its keep with it, is it sound, and what does it do to
the cost of the chain and the reach of a large machine?

*What was built* (commits 679635b to the entry; the design is DESIGN.md, section 4, "The GF(2) bound", the proof
heuristics/THEORY.md, Theorem 11, and the algorithms the module comments of `src/gf2.rs` and `src/gf2/meet.rs`):

* `gf2::Gf2Graph`, the root's two matroids (the even and the odd frames' quotients by their moves, 1 to 3 words a
  block: the frames' ranks at 1215 are 33, 52, 34, 18, 7, 3, 1), and `gf2::Gf2Bound`, a level per node of the
  searcher's path: the root's vectors contracted by the node's promotions (one pivot per member and matroid), the live
  candidates, and the *history* of dominant sets its augmentations passed through, from which a child, or the node
  without a member, starts again; a child whose parent's heaviest set survives inherits it with no solve.
* `gf2::meet`, the intersection: Lawler's shortest augmenting paths (Bellman–Ford, lexicographic in length and
  arcs), over a tableau that holds, per element, its coordinates on the current set's members and its residue, as
  bitset rows that each exchange updates by one pivot over the block; the start from the heaviest uncontended
  candidates; the weight split (Frank) that bounds the intersection without each member of the optimum in one pass.
* The tactic `gf2` (checkpoint header bit 7; a checkpoint of an earlier build has it clear and resumes on the
  matching path): the node bound, the conflicts of its heaviest set to branch on (from the nogood table or computed),
  forced members from the split, exact strong branching (the bound without each member of the conflict solved, in
  the order of the split's bounds, each solve stopped at the larger of the best so far and `need`, after a greedy lower
  bound; the chosen member's level kept for the node).  Cut reason `gf2`; stat slots 4-7 count its solves and
  augmentations and strong branching's (not in result lines).
* Measurement: `examples/matroid.rs prove` (one-thread proofs with the knobs below, cycles and the bound's counters),
  the probe's F4 (`perf --tactic gf2=on`: the bound from scratch, with its split, and without each conflict member, at
  the 600 fixture nodes), and the knobs `SearchSpace::gf2_prefilter`, `gf2_branching`, `gf2_strong_limit` and
  `gf2_more_forced`, which are not tactics.

*The inner loop.*  Each step measured on the probe's nodes or on proofs, values and trees unchanged:

* Bitset rows per block instead of a span rebuilt per element; members and outside elements kept in lists along
  each path; the sources and the best sink found as distances are set; the pull phase skipping the blocks of `A`
  with no changed member; the split reusing the workspace when it still holds the level's solve: the bound from
  scratch 32% fewer cycles at 1215's nodes, the split 40% (da49d8a).
* Branchless pivots (the row selected by a mask made from the pivot bit): 11% less time at 1254 and 1300; no
  allocation in the split (f1d09e7).
* A greedy lower bound before each strong-branching solve (the level's heaviest set without the member, completed
  greedily): when it reaches the solve's stop, no solve; 10% less time at 1254 (9a99f34).
* Profile-guided optimization (`tools/pgo.sh`, now trained on the GF(2) probe too): 1.5% fewer cycles on the
  probe's GF(2) kernels and windows, 1.8% on the proof of 1287, the same fingerprint and tree.
* Where the time goes now (a sampled proof of 1287, one thread): the shortest paths 57%, the level's own work (the
  solve's setup, history, the split) 22%, the pivots 8%, the peels 3%.  An augmentation takes 2.4-2.5 Bellman–Ford
  rounds, so Dijkstra with potentials would save little; a strong-branching solve makes 22-32 augmentations at the
  probe's nodes, because it restarts from the last dominant set without the member, which for a heavy member is the
  uncontended prefix.  Strong branching is 70% of the augmentations (1287: 549,223 of 799,278).
* The probe: `perf --tactic gf2=on` has the fingerprint 162e4e94a52a95c6 (the default fingerprint, whose windows'
  checkpoints record the matching path's tactics, is unchanged: 7e8e724717d26545).  A step of the window of 936 costs
  192k cycles and its search ends after 1,834 of the 50,000 steps; the window of 1215 (a checkpoint 600,063 steps into
  the matching path's proof) ends after 12,633 steps of 329k cycles, where the matching path's steps cost 22k and 25k
  and its window does not end.  The bound from scratch at a node costs 219k and 455k cycles (936, 1215), with the split
  294k and 570k, and the bound without each conflict member, warm-started, 205k and 378k a member (the node's own
  bound included).

*What the machinery is worth with it.*  One-thread proofs (`matroid prove`, the incumbent `P*` as a value, no seed or
cap); per game the default's nodes and G cycles, and each variant's nodes and cycles relative to the default's:

| variant | 936 | 1100 | 1184 | 1215 | 1250 | 1254 | 1288 | geometric mean |
|---|---|---|---|---|---|---|---|---|
| default: nodes, G cycles | 2,201, 0.43 | 1,480, 0.35 | 409, 0.13 | 10,371, 3.83 | 1,183, 0.46 | 7,964, 3.33 | 11,910, 3.78 | |
| branching=split | 1.80, 1.40 | 1.75, 1.34 | 2.47, 1.54 | 2.90, 1.83 | 3.83, 2.34 | 6.48, 3.96 | 2.89, 2.29 | 2.87, 1.96 |
| branching=largest | 1.75, 1.34 | 2.45, 1.80 | 2.95, 1.81 | 4.22, 2.63 | 4.63, 2.83 | 9.71, 5.84 | 3.26, 2.60 | 3.60, 2.43 |
| branching=forced | 1.26, 1.23 | 1.16, 1.08 | 1.68, 1.34 | 2.20, 1.82 | 2.04, 1.66 | 3.41, 2.66 | 1.53, 1.41 | 1.78, 1.54 |
| prefilter | 1.00, 1.02 | 1.00, 1.04 | 1.00, 1.04 | 1.00, 1.01 | 1.00, 1.02 | 1.00, 1.02 | 1.00, 1.01 | 1.00, 1.02 |
| strong=1 | 1.80, 1.29 | 1.75, 1.12 | 2.47, 1.33 | 2.90, 1.63 | 3.83, 1.96 | 6.48, 3.14 | 2.89, 1.86 | 2.87, 1.67 |
| first-forced | 1.14, 1.03 | 1.22, 1.05 | 1.23, 1.05 | 1.21, 1.03 | 1.28, 1.06 | 1.31, 1.09 | 1.20, 1.06 | 1.23, 1.05 |
| strong=3 | 1.88, 1.70 | 1.29, 1.24 | 1.07, 1.02 | 1.25, 1.12 | 1.14, 1.02 | 1.35, 1.21 | 1.34, 1.28 | 1.31, 1.21 |
| lagrange=on | 1.00, 1.07 | 1.00, 1.00 | 1.00, 1.01 | 1.00, 1.03 | 1.00, 1.00 | 1.00, 1.02 | 1.00, 1.03 | 1.00, 1.02 |
| nogoods=off | 1.06, 1.07 | 0.66, 0.84 | 2.08, 1.75 | 1.46, 1.41 | 1.17, 1.17 | 1.15, 1.09 | 1.45, 1.42 | 1.23, 1.22 |
| fix_forced=off | 0.77, 1.68 | 0.67, 1.86 | 1.60, 3.46 | 1.21, 3.16 | 1.58, 4.29 | 1.64, 3.84 | 0.93, 2.15 | 1.13, 2.76 |
| matching | 18.92, 2.18 | 20.01, 2.06 | 127.77, 9.17 | 107.17, 7.57 | 600.08, 39.59 |  |  | 79.18, 6.58 |

* `lagrange`: no tree changes (992 and 1025 by one to three nodes), 0-7% more cycles: **off**.  Its multipliers
  are first fitted after 2,000 steps, and the GF(2) bound already holds what the nogoods it relaxes would add.
* The matching bound as a first check (`prefilter`): the same trees, 1-4% more cycles: not kept.
* `nogoods`: 22% more cycles without (1100 is 16% cheaper without, 1184 75% dearer): **on**.  `forget_nogoods`
  changes nothing at these sizes (fewer than 50,000 steps a searcher) and stays on.
* `fix_forced`: 2.8 times the cycles without (fewer nodes at three games, each much dearer): **on**.
* `match_refine` has no effect with `gf2` (the matching bound is not computed).
* Branching: exact strong branching beats choosing by the split's bound (`split`: 2.9 times the nodes, 2.0 times the
  cycles), the largest member (3.6 and 2.4), exact solves only to find the forced members (`forced`: 1.8 and 1.5),
  solving at most one or three members (1.7 and 1.2 times the cycles) and stopping at the first forced member (5%):
  **exact, every member**.
* The matching path (`matching`: `gf2=off,lagrange=on`, the former default; not run at 1254 and 1288, whose proofs
  take minutes): 79 times the nodes and 6.6 times the cycles of the five games' GF(2) proofs, from 2.1 times the
  cycles at 936 and 1100 to 40 times at 1250.

*Soundness.*

* Theorem 11 (THEORY.md: the frame model's form of Theorem 10, the parity matroids' bound on the whole game):
  promotable sets are independent in both matroids, at every node in their contractions; the forced members rest on
  weak duality for the split.  DESIGN.md's bounds section links it.
* Tests: the intersection against brute force (every size limit, every reported warm start, deletions, the split's
  bounds); exhaustive promotable sets on the boards up to 140 with at most 22 candidates, and the optimal promotions
  of every board up to 400, independent and under the bound; levels along random paths of games 60-1000 equal to the
  matroids built afresh at the node, and every forced member forced; at the probe's 600 nodes of 936 and 1215,
  random completions independent and under the bound, and the matching path's search from the bound finding nothing
  above it (597 of the 600 searches end within 200 ms); the soundness harness (handicapped incumbents) under 11
  GF(2) configurations (every branching, the prefilter, without forced members or nogoods, forgetting every step,
  two threads, the Lagrangian bound after it) on 2-300, the Lagrangian games of 301-600, the perturbed games and the
  wide units; resumes over and over of 1000 at W = 16, 20, 24 and 64; tripwires for the GF(2) trees (480-1133, with
  1000 and its switches); checkpoints and resumes of 1000 on one and two threads (unit tests) and through the CLI.
* `taxman regress --to 1000` (2 threads): every score matches optimal.json, in 21.6 s and 92.6 G cycles, against 96.9 s
  and 554 G cycles on the matching path.
* The chain 1-1300 (`taxman solve 1-1300 --threads 2`, to a scratch file): every score equals `results/strict.jsonl`'s,
  with the same methods and proofs (trivial 13, prime 211, cap 115, exhausted 416, same search 545), and
  `verify-results` accepts it; 88 s of wall time and 395 G cycles in all.

*The cost.*

1. *The chain.*  1001-1300 took 60.3 s of game time on 2 threads (48.1 s of search, 736,530 nodes), against
   `results/strict.jsonl`'s 988.4 s on 4 threads (976.3 s, 315,916,127 nodes): 16 times less time on half the
   threads, 429 times fewer nodes.  1201-1300: 35.3 s against 901.7 s (26 times).  The dearest game is 1287 at 4.9 s
   (strict: 1232 at 206.7 s; with GF(2) 3.4 s).  1-1000: 21.4 s against 53.4 s.
2. *The 146 proofs of the slope study* (`slope prove`, one thread, two at a time; against
   `bench/curve/slope_proofs.jsonl`, the matching path at its commit):

   | band | games | nodes, matching | nodes, GF(2) | ratio | s, matching | s, GF(2) | ratio | Gcycles, GF(2) |
   |---|---|---|---|---|---|---|---|---|
   | 851-900 | 6 | 48,833 | 1,267 | 39 | 0.5 | 0.1 | 6.5 | 0.9 |
   | 901-950 | 16 | 1,530,782 | 30,295 | 51 | 12.8 | 2.0 | 6.4 | 8.0 |
   | 951-1000 | 16 | 8,915,952 | 123,724 | 72 | 107.0 | 11.8 | 9.1 | 37.7 |
   | 1001-1050 | 19 | 13,060,192 | 154,215 | 85 | 163.8 | 14.4 | 11.4 | 46.3 |
   | 1051-1100 | 18 | 1,137,537 | 5,862 | 194 | 12.6 | 0.6 | 21.0 | 4.3 |
   | 1101-1150 | 17 | 141,744 | 4,798 | 30 | 1.5 | 0.5 | 3.3 | 3.8 |
   | 1151-1200 | 16 | 1,685,147 | 18,014 | 94 | 13.2 | 1.9 | 6.9 | 8.1 |
   | 1201-1250 | 19 | 64,621,157 | 106,869 | 605 | 872.2 | 13.6 | 64.2 | 44.3 |
   | 1251-1300 | 16 | 171,570,238 | 164,180 | 1045 | 1750.5 | 19.4 | 90.3 | 61.8 |
   | 1301-1350 | 2 | 31,354,160 | 53,008 | 591 | 439.1 | 7.3 | 59.8 | 22.8 |
   | 1351-1400 | 1 | 3,343,973 | 35,114 | 95 | 34.3 | 4.3 | 7.9 | 13.4 |
   | all | 146 | 297,409,715 | 697,346 | 426 | 3407.6 | 75.8 | 45.0 | 251.5 |

   The geometric mean ratio is 111 in nodes and 11.5 in seconds.  Fits of log10 against n over the 146 games:
   nodes +0.176 ± 0.052 per 100 n (ten times per 568 n) against +0.476 ± 0.066 (per 210 n); seconds +0.242 ± 0.052
   (per 414 n) against +0.483 ± 0.067 (per 207 n); the processes' cycles +0.195 ± 0.035 (per 514 n).  A node costs
   more as n grows (more candidates, larger blocks), so time grows faster than nodes, but at half the matching path's
   rate.  The dearest proofs: 1350 5.8 s (matching 282 s), 1287 4.6 s (239 s), 1352 4.3 s, 996 3.6 s, 992 3.4 s, 1232
   3.0 s (359 s).
3. *Threads* (`ladder --seed-opt`, the proofs of 1215, 1232, 1287 and 1290): summed, 10.8 s on one thread, 6.1 s on two
   (1.79 times), 3.75 s on four (2.89 times).  The trees grow a little with threads (89,181 nodes, then 96,427 and
   101,737: each thread's nogoods), the effective parallelism is 97-99%, and the node rate per thread falls from 8.2k/s
   to 8.0k and 6.8k (another job used three of the ten cores meanwhile).  These searches last 0.5-5 s, and all four
   threads were busy 30 ms after the start.
4. *Past 1300* (`taxman solve 1301-4095 --use-prev --threads 2`, a chain from optimal.json's game of 1300, in
   three invocations, the last stopped by its 29-minute cap during game 2176) reached 2175: 875 games, all
   completed (prime 115, cap 77, exhausted 314, same search 369), in 5,490 s of game time and 48.9 M nodes;
   `verify-results` accepts the file, and the ten games of 1302-1352 that the matching path had solved standalone
   (`bench/curve/bands.jsonl`) have the same scores.  (The first invocation, 1301-1700, ran 47 minutes, beyond the
   study's half-hour limit for a run, because an interruption of this session left it running.)  Per band of 50 n:

   | band | game time s | dearest game s (n) | nodes | µs a node (thread time) | root GF(2) gap, searched games |
   |---|---|---|---|---|---|
   | 1301-1350 | 79 | 22 (1326) | 1,222,512 | 124 | 204 |
   | 1351-1400 | 304 | 54 (1392) | 3,583,097 | 168 | 376 |
   | 1401-1450 | 345 | 92 (1430) | 3,812,589 | 179 | 419 |
   | 1451-1500 | 896 | 188 (1475) | 9,836,779 | 181 | 481 |
   | 1501-1550 | 675 | 116 (1521) | 7,339,096 | 182 | 463 |
   | 1551-1600 | 321 | 59 (1568) | 3,414,599 | 184 | 459 |
   | 1601-1650 | 80 | 27 (1617) | 715,057 | 209 | 346 |
   | 1651-1700 | 103 | 20 (1696) | 903,769 | 214 | 341 |
   | 1701-1750 | 86 | 11 (1720) | 677,350 | 232 | 296 |
   | 1751-1800 | 83 | 20 (1800) | 656,087 | 228 | 323 |
   | 1801-1850 | 459 | 63 (1848) | 4,229,917 | 213 | 496 |
   | 1851-1900 | 339 | 60 (1860) | 3,144,901 | 210 | 351 |
   | 1901-1950 | 48 | 7 (1938) | 258,232 | 293 | 249 |
   | 1951-2000 | 217 | 42 (1995) | 1,475,919 | 280 | 349 |
   | 2001-2050 | 76 | 11 (2020) | 415,235 | 311 | 324 |
   | 2051-2100 | 351 | 65 (2064) | 2,050,415 | 322 | 458 |
   | 2101-2150 | 182 | 19 (2140) | 829,229 | 385 | 473 |
   | 2151-2175 | 847 | 290 (2170) | 4,286,865 | 389 | 612 |

   The cost comes in waves (up to ×8 between neighbouring bands of 25 n, ×75 between the cheapest and the dearest,
   2151-2175, which holds 2170 at 290 s), and over 1301-2175 there is no growth a fit resolves: log10 of a band's
   time changes by −0.012 ± 0.033 per 100 n (residual sd 0.49).  The waves follow the root's GF(2) gap: an exhausted
   search's nodes grow ×10 per 236 of gap (314 searches, residual sd 0.44), and the gap grows only 6.9 ± 1.7 per
   100 n (sd 115 from game to game; the matroid study's 4.6 ± 2.4 over 880-1300), where over 880-1300 the matching
   bound's grew 142 and the Lagrangian bound's 56.  A node costs more as n grows, +0.045 ± 0.004 decades per 100 n
   (candidates, frames and wider sets: 120-180 µs of thread time at 1300-1500, 320-390 at 2050-2175, past the start
   of W = 64 and of an eighth frame at 2048).  Together: ×10 per about 1,350 n.

   *Projections* (made as `bench/curve/README.md` made them): a 192-core c9g.48xlarge counted as 192 × 0.8 M1 cores,
   linearly or at half that, which is 86 or 43 times this machine's 2 threads (1.79 one-thread equivalents); $3.33/h
   spot, $8.35/h on-demand; a game at n costing the mean of 1301-2175 (6.27 s on 2 threads, at 1738) times
   10^(slope × (n − 1738)), summed from 2176:

   | slope | to 3000, 192 cores | to 4095, this machine's 2 threads | to 4095, 192 cores | a day on 192 cores ($80 spot) reaches | a week ($559) |
   |---|---|---|---|---|---|
   | the fit over 1301-2175 (none) | 1-2 minutes | 2.3 hours | 2-3 minutes | 4095 | 4095 |
   | the fit + 2 s.e. (×10 per 1,878 n) | 3-6 minutes | 23 hours | 16-32 minutes | 4095 | 4095 |
   | the gap model (×10 per 1,350 n) | 5-9 minutes | 55 hours | 38-77 minutes | 4095 | 4095 |
   | the 146 proofs' slope, 880-1352 (×10 per 413 n) | 4-8 hours | 18 years | 77-154 days | 3191-3315 | 3540-3664 |

   In words: with the GF(2) bound the chain is not a question of a large machine's time up to the largest board the
   set widths cover (4095; beyond it is a code change, a wider set).  A day on 192 cores takes it there at every slope
   the chain past 1300 supports, and even at the steepest any GF(2) measurement has shown (the proofs of 880-1352,
   where the gap grows fastest), a day reaches about 3200 and a week about 3600; the matching path's curve gave 1510
   and 1565.  What was not measured: games past 2175, where frames and widths change again; how a search of 10^5-10^6
   nodes spreads over 192 threads (measured to 4 here); single games in the waves, which can cost many times their
   band's mean.
5. *Against the model-free search* (the evidence crate's `exact`, which proves opt(n) without the frame model with the
   parity bound, Theorem 10, the same bound on the whole game; this machine, `--threads 2`, one game at a time, which
   it searches on one thread): 1232 in 70.5 s (34,456 nodes, 221 G cycles), 1300 in 33.9 s (17,771 nodes, 106 G), 990
   in 60.9 s (44,389 nodes, 192 G).  The frame model's search with the GF(2) bound proves them (one thread, the
   incumbent the optimum, as `exact` does) in 3.0 s (22,524 nodes, 9.2 G cycles), 1.35 s (11,128, 4.3 G) and 1.7 s
   (20,975, 5.4 G): 24, 25 and 36 times fewer cycles.  The trees are within a factor of two (the bounds agree at the
   roots: Theorem 10's is in_the_bag plus Theorem 11's), so the difference is the cost of a node, 6.4, 6.0 and 4.3 M
   cycles there against 0.41, 0.39 and 0.26 M here: the frame model's matroids are the whole game's with the upper
   half and the frames' moves contracted away (at 1302, 331 candidates in blocks of rank 1 to 55, against the numbers
   2..n over n/2 coordinates), and each node's are its parent's contracted by one pivot a promotion, with a tableau
   that pivots follow, where `exact` solves each node's matroids warm-started from its parent's certificate.  In the
   chain the frame model also has the seed and the cap: 1232 in 3.4 s on 2 threads.

*The data* are in `bench/gf2/`: `proofs.jsonl` (the 146 proofs, `slope prove` lines with the process's cycles and
instructions), `variants.jsonl` (the `matroid prove` lines of the table above, each with its variant), `chain.jsonl`
(the chain 1301-2175 as `taxman solve --out` wrote it, moves included, and the unfinished line of 2176) and
`roots.jsonl` (`matroid roots` of 1301-2175).  The chain 1-1300 and the scaling run are not kept: the first
reproduces `results/strict.jsonl`, and the numbers of both are above.

*Recommendations.*

* **Defaults**: `gf2` on, `lagrange` off, the rest as they were (`match_refine` 1, which matters only on the matching
  path).  Made the defaults in 7af8b0e; `SearchTactics::matching()` names the former default, which the plumbing
  tests use where they must stop a search of a small game.
* **To remove** (not removed here): the knobs `gf2_prefilter`, `gf2_strong_limit` and `gf2_more_forced` and the
  branchings `Split`, `Largest` and `Forced` (`Gf2Branching`), which lost; and, once a production run has been made
  with the GF(2) search, what only the matching path uses and the GF(2) bound has made idle: the Lagrangian bound in
  the search (`search::lagrange`, `matchbound::lagrange`, the tactic `lagrange`) and refinement (the branches of
  `search::refine::Refiner`, the tactic `match_refine`).  The matching path itself (the matching bound at nodes and
  its strong branching) should stay: it is the tests' independent second search (the probe-node test, the plumbing
  tests' long searches, the tripwires), and the setup's dive rounds the root's matching into the starting incumbent.
  `examples/common/meet.rs` (the study's implementation, matroids built afresh at each node) stays as the
  independent check of the crate's.
* **Next on the inner loop**: strong branching's solves (70% of the augmentations) restart far below their answer.
  The optimum less the member is a common independent set one smaller; canceling the negative cycles of its exchange
  graph (a set is extreme exactly when there are none, Schrijver, Theorem 41.5) would start each solve from an
  extreme set one smaller than the optimum instead of the prefix.  Untried.
* **Seams kept open** for the studies that would build on this bound (the cover LP or a Lagrangian over the conflicts
  with this intersection as the inner problem, cut pools at the root, the symmetry rule): `meet::Problem` takes any
  positive weights, but the start from the heaviest uncontended candidates and the greedy lower bound assume that
  weights do not decrease with the element's index (the candidates by increasing number); a reweighting needs its own
  element order (the root's matroids built in it) or those two starts switched off.  The levels, the contraction and
  the split do not depend on the order.

## The committed chain to 2175 (2026-10)

`results/strict.jsonl` was replaced once more, and now reaches 2175: a run of the clean build of commit feba6d0, the
merge of the GF(2) bound into the search ("The GF(2) bound in the search"), `taxman solve 1-2175 --threads 8 --out FILE
--checkpoint-dir DIR --checkpoint-interval 60 --progress 0` from n = 1, in one invocation, on the 10-core M1 Max with
nothing else running: 1638.6 s of wall time and 10,890 s of CPU.  Search seconds 28.4 for 1..1300 (7.5, 11.8 and 9.1
for 1..1000, 1001..1231 and 1232..1300; 1.08 M nodes, the dearest game 1287 at 1.6 s) and 1422.1 for 1301..2175 (47.4 M
nodes, against 48.9 M of the chain from 1300 on 2 threads), against 1022.5 s and 332.0 M nodes for 1..1300 in the chain
of 271c817 (4 threads, other jobs running).  The slowest games are 2172 (73.1 s), 2170 (68.5), 1475 (50.1), 1472 (37.8)
and 1430 (35.0); the queue never held more than 178 units and none spilled.  No game assumes anything.  Proofs:
trivial 13, prime formula 326, cap 192, exhausted 730, same search 914.

What it agrees with:

* The previous file (271c817) in 1..1300: every score, proof kind and method, game by game.  The moves differ in ten
  games, each a different one of the several optimal sets (539, 684-687, 770-773 and 1215): this file's sets are
  `optimal.json`'s but at 539, 770-773 and 1215, where the previous file's were the table's, and the previous file's
  differed from the table at 684-687.  Of the 192 proofs by the cap, 191 were decided before the first node and 686's
  took 29.
* The chain from `optimal.json`'s game of 1300 of the previous entry (`bench/gf2/chain.jsonl`, build 716ebd9, 2
  threads) in 1301..2175: every score, proof kind and method, and every move list but 1560-1563 (an exchange of five
  numbers of the sum 3008).  Both used the GF(2) bound, so this is a second run of the same search, not a check of the
  bound; the model-free check of 1301..2175 (`rust/evidence`) is pending.
* `optimal.json` in 1..1300 (`optimal_match` true).  It still ends at 1300 and was not extended (the lines of
  1301..2175 say `optimal_match` null); `taxman extend-optimal results/strict.jsonl` would add 875 games.

`taxman verify-results` takes 47.6 s on the file (6.4 s for its first 1300 games) and `tests/results.rs` 49 s; the
Python referee takes 1 s.  Entries above that quote `results/strict.jsonl` mean the chain of 271c817 (git has it at the
commit before the one that replaced it), unless they say otherwise; [results/README.md](../results/README.md)
describes the present file.
