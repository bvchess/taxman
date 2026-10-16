# Prime exchanges in the search (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  "Dominance and symmetry" found that the frame model has symmetries, the exchanges of two primes that the
frame builder treats alike, and piloted breaking them: in a plain GF(2) branch and bound a hopeless test took 0.948
of the nodes and the constraint's Lagrangian 0.859, a constant share, time −3%.  Its *What it implies* was the plan:
the pairs in the library, the hopeless test at every node, the Lagrangian at the nodes whose heaviest set breaks a
pair, a tactic, on by default only if it pays in time on the real search.  And two questions the pilots left: is
pruning by all the inequalities at once sound when the classes interact (candidates divisible by primes of two
classes), and is it sound with what the search learns (the matching pilot saw its trees grow, 1184 +27%)?

*The theorem* (heuristics/THEORY.md, Theorem 13, "Prime exchanges").  An exchange π of `p < q` that maps the root's
frames (moves to moves, factors to factors) and candidates onto themselves maps promotion sets to promotion sets
(π is multiplicative and sends primes to primes, so it keeps the maximal-factor relation, and a frame with a set
applied is the same game as the frame with its image applied), and changes a sum by `Σ (π(x) − x)`, which is
`(q − p)(c_p − c_q)` when no candidate is divisible by `pq`, `p²` or `q²`.  So a heaviest set `S*` keeps
`c_p(S*) ≤ c_q(S*)`: otherwise `π(S*)` is promotable and heavier.  The argument for one pair uses no other, so every
optimum keeps every pair's inequality at once; no sorting argument (the rearrangement inequality within a class) and
no condition across classes is needed, nor any restriction to classes whose candidates have one prime of them.  A
node none of whose completions keeps them all is pruned soundly, and the search's other prunings hold for the
completions that keep them as for all.

*What the search learns.*  Nothing from the rule: a cut ends a loop, a drop shrinks a candidate set.  The nogoods
and known conflicts are facts about promotability (a set that cannot be promoted from the root), true whatever the
rule cut; the rule changes which conflicts a searcher meets, as any change of the tree does (the matching pilot's
growing trees were that), never their truth.  The forced members are those every completion beating the incumbent
promotes, a fortiori every one that keeps the inequalities.  The rule is a function of a node's promotions and
candidates, so units, resumes and threads keep it.  A search of a node *below* the root for its own best completion
(the soundness harness's units) finds the best that keeps the inequalities, which can be lighter: the harness holds
such searches to an oracle of that.

*What was built* (commits 1e00d42, 706cd8d, bf49ab9; the Lagrangian in 4e80430, taken out in 92644b3):

* `src/search/symmetry.rs`, from the examples' `common/symmetry.rs` (deleted; `examples/dominance.rs` uses the
  library's).  `root_pairs` groups the primes that divide a candidate by an invariant (the frame, role and exponent of
  each of their multiples, and the candidates' exponents), which an exchange that is a symmetry keeps, and checks only
  the pairs within a group, and only on the multiples of the two primes (π fixes every other number and its maximal
  factors): 0.17, 0.23, 0.36, 0.59 and 0.55 ms at 1300, 2175, 3000, 4000 and 4095 (32, 99, 159, 300 and 297 pairs),
  against about 20 ms at 2175 before.  Tested against the definition (every pair of primes, every number) on the
  boards 2..=300 and a sample to 2200.
* The rule (`Symmetries::exclude`), from the cofactor sums of the node's promotions `P` and candidates `C` over each
  pair's primes: *hopeless* if `c_p(P) > c_q(P) + c_q(C)` for some pair; and a candidate `mp` with `c_p(P) + m >
  c_q(P) + c_q(C)` *dropped*, repeated until nothing drops.  In `make_child` after the nogoods' rule-outs, in
  `check_child` before the bounds (cut reason `symmetry`), and in `check_node` after a step (the node goes on without
  the member it branched on, which lowers `c_q(C)`), the drops leaving the GF(2) and matching levels as a branch member
  does.  The sums are computed afresh over each prime's family of candidates: a few hundred bit tests, 0.35% of the
  samples of a proof of 1472 (`sample`), which needs nothing kept along the path.
* The tactic `symmetry` (bit 8 of the checkpoint's tactics), on by default since bf49ab9; the matching path
  (`SearchTactics::matching`) stays without it, as it was.

*Measurements* (one thread, the M1 Max shared with other agents' jobs; `matroid prove`: the incumbent the optimum as
a value, no seed or ceiling; nodes, and G cycles of the process):

| n | nodes off | nodes on | ratio | G cycles off | on | ratio |
|---|---|---|---|---|---|---|
| 936 | 665 | 385 | 0.58 | 0.11 | 0.07 | 0.62 |
| 990 | 8,474 | 8,178 | 0.97 | 1.31 | 1.29 | 0.98 |
| 992 | 13,011 | 12,172 | 0.94 | 2.23 | 2.17 | 0.98 |
| 1024 | 11,128 | 7,673 | 0.69 | 1.73 | 1.23 | 0.71 |
| 1215 | 4,634 | 3,647 | 0.79 | 0.86 | 0.74 | 0.86 |
| 1232 | 10,125 | 7,346 | 0.73 | 2.55 | 2.17 | 0.85 |
| 1254 | 3,554 | 2,837 | 0.80 | 0.71 | 0.62 | 0.88 |
| 1287 | 18,095 | 10,228 | 0.57 | 3.04 | 1.97 | 0.65 |
| 1300 | 5,675 | 3,325 | 0.59 | 0.79 | 0.54 | 0.69 |
| 1350 | 13,766 | 7,058 | 0.51 | 2.51 | 1.43 | 0.57 |
| 1430 | 308,209 | 257,461 | 0.84 | 81.8 | 73.5 | 0.90 |
| 1472 | 512,103 | 309,591 | 0.60 | 169.5 | 109.6 | 0.65 |
| 1475 | 581,225 | 489,761 | 0.84 | 163.1 | 147.6 | 0.91 |
| 1521 | 93,420 | 70,829 | 0.76 | 25.9 | 21.4 | 0.83 |
| 2170 | 519,394 | 440,988 | 0.85 | 198.6 | 175.1 | 0.88 |
| 2172 | 516,133 | 510,340 | 0.99 | 193.5 | 199.6 | 1.03 |

The bench set's games 1050, 1100, 1133, 1170 and 1184 (proofs of 33 to 620 nodes): 0.86, 0.89, 1.00, 0.73 and 0.87
of the nodes.  Over the 15 proofs of 936-1350 above 1,000 nodes the geometric means are 0.75 of the nodes and 0.81 of
the cycles (the quick and bench sets' 16.0 G cycles in all become 12.4); over 1430-2172 0.80 and 0.86.  A node costs
2% to 18% more cycles: the rule's arithmetic is under 1% of it, the rest is the bound's work on the candidates the
rule drops (a child whose parent's heaviest set lost a member to a drop cannot inherit it and solves), which is what
tightens it.

* *`taxman bench`* (standalone, seeded from n − 1, one thread), with 1472 and 1475 added: 126.2 s to 95.9 s in all;
  1472 541,939 to 352,483 nodes (59.1 to 40.5 s), 1475 743,592 to 553,347 (66.4 to 54.8 s).
* *The chain 1..1300* (one thread, checkpoints every 120 s): 402,205 to 290,503 nodes (×0.72), 31.3 to 26.0 s of
  search, 41.7 to 35.8 s of wall time; every score, method and proof that of `results/strict.jsonl`.
* *The chain 2126..2175* behind `strict.jsonl` (one thread, checkpoints every 120 s; 18 exhausted searches, the rest
  repeats, primes and ceilings): 1,811,690 to 1,695,594 nodes (×0.94), 234.5 to 225.9 s; the same scores; 2160
  351,118 to 320,132 nodes, 2170 519,394 to 440,988, 2172 516,133 to 510,340, 2166 and 2168 1% and 8% more.  The
  share is smaller here than at 1000-1500.
* *The hopeless test alone* (the drops switched off, measurement only), 13 of the proofs: 0.89 of the nodes and 0.90
  of the cycles, where the whole rule gives 0.71 and 0.77 on the same games (1287 17,631 nodes against 10,228; 1472
  456,716 against 309,591): the drops are most of the gain, and they are why the rule does better here than the
  pilot's hopeless test (0.948).
* *The Lagrangian of the inequality on the GF(2) bound* (4e80430, knob `SymmetryBound`; `matroid prove
  --symmetry-bound pair|joint`): at a node whose GF(2) bound does not cut, the pairs its heaviest set breaks with the
  node's promotions, each alone or all at once, bounded again by the heaviest common independent set under the doubled
  weights `2x − kδ(x)` (`δ` the change an exchange makes to a member, `k` = 2 then 1: `μ` = `q − p` and `(q − p)/2`),
  solved from scratch in a workspace of its own after a free screen (the heaviest set itself is a lower bound).  Valid
  (tested never below the best completion that keeps the inequalities, 3,120 bounds), and it cut (296 solves and 122
  cuts in the searches of 880-1010), but on top of the rule it changed little and lost: on 11 proofs 1.01 of the nodes
  and 1.02 of the cycles (each pair alone; all at once 1.01 and 1.03), 1430 262,611 and 276,033 nodes against
  257,461, 1521 75,817 against 70,829.  A cut ends a loop whose conflicts the search would have learned from, and the
  drops had already taken what the inequality cuts most often.  Taken out (92644b3); a better `μ` cannot do much where
  the bound barely cuts.

*Checks.*  The tests of section 9 of DESIGN.md: the pairs against their definition; the images of optimal and random
promotion sets promotable and worth the formula, every optimum keeping every inequality, every promotion set of the
boards with at most 22 candidates checked and the best that keeps the inequalities the optimum; the rule against its
definition by enumeration; the soundness harness with the rule over both bounds, on two threads and forgetting every
step, its searches of units below the root held to an oracle of the best completion that keeps the inequalities;
every board 2..=600 with pairs (470) finding its optimum from the root, with the oracle without the frame model to 40;
units that hold an optimal set less 30 to 70% of it finding the optimum (boards 600..1000); resumes with the rule;
one-thread determinism; the tripwires (the default's trees moved on purpose: 600 533 to 365, 936 665 to 385, 1000
2657 to 2172; the rows `symmetry=off` keep the old ones, the matching path's rows do not move).  The probe's
fingerprints do not move (its windows search with the tactics their checkpoints record, without the rule).

*Conclusion.*  The rule pays and is on by default: about a fifth of the cycles of a proof near 1000-1500, less near
2175 (a twentieth on the chain there), a constant share as the pilots found, not a lower slope.  What is left: the
exchanges that hold only at a node (the dominance study found them in the matching trees, almost none in the GF(2)
trees); and sums kept along the path instead of computed afresh, which would save under 1%.
