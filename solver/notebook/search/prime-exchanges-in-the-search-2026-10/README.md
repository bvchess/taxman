# Prime exchanges in the search (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-14.  Concluded 2026-10-14.

**Question.**  Does breaking the frame model's prime-exchange symmetries pay in the real search, as the pilots of
[dominance and symmetry](../dominance-and-symmetry-2026-10/README.md) suggested?  **Answer.**  Yes: the rule takes 0.75
of the nodes and 0.81 of the cycles of the one-thread proofs of 15 games of 936-1350, takes the chain 1..1300 from
402,205 to 290,503 nodes, and is on by default as the tactic `symmetry`.

## Method

The pilots had found that exchanging two primes p < q in every number is a symmetry of the frame model's root.  In a
plain GF(2) branch and bound their hopeless test took 0.948 of the nodes and the constraint's Lagrangian 0.859, a
constant share, time −3%.  Two questions remained: whether pruning by all the inequalities at once is sound when the
classes interact, and whether it is sound with what the search learns (the matching pilot's trees grew, 1184 +27%).

**The theorem** ([Theorem 13](../../../../docs/theory/bounds.md#prime-exchanges-symmetries-of-the-promotions)).  An
exchange π of p < q that maps the root's frames and candidates onto themselves maps promotion sets to promotion sets,
and changes a sum by (q − p)(c_p − c_q) when no candidate is divisible by pq, p² or q².  So a heaviest set keeps c_p ≤
c_q, or π of it would be promotable and heavier.  The argument for one pair uses no other, so every optimum keeps every
pair's inequality at once, with no condition across classes.  A node none of whose completions keeps them all is pruned
soundly.

**What the search learns.**  Nothing from the rule: a cut ends a loop and a drop shrinks a candidate set.  The nogoods
and known conflicts are facts about promotability, true whatever the rule cut; the rule changes which conflicts a
searcher meets, as any change of the tree does, never their truth.  The forced members are those every completion
beating the incumbent promotes, a fortiori every one that keeps the inequalities.  The rule is a function of a node's
promotions and candidates, so units, resumes and threads keep it.  A search of a node below the root for its own best
completion (the soundness harness's units) finds the best that keeps the inequalities, which can be lighter, and the
harness holds such searches to an oracle of that.

**What was built** (1e00d42, 706cd8d, bf49ab9):

* `solver/frame/src/search/symmetry.rs`.  `root_pairs` groups the primes that divide a candidate by an invariant (the
  frame, role and exponent of each of their multiples, and the candidates' exponents), checks only the pairs within a
  group, and only on the multiples of the two primes.  It takes 0.17, 0.23, 0.36, 0.59 and 0.55 ms at 1300, 2175, 3000,
  4000 and 4095 (32, 99, 159, 300 and 297 pairs), against about 20 ms at 2175 trying every pair, and is tested against
  the definition on the boards 2..=300 and a sample to 2200.
* The rule (`Symmetries::exclude`), from the cofactor sums of the node's promotions P and candidates C over each pair's
  primes.  A node is *hopeless* if c_p(P) > c_q(P) + c_q(C) for some pair, and a candidate mp with c_p(P) + m > c_q(P) +
  c_q(C) is *dropped*, repeated until nothing drops.  It runs when a child is made, before its bounds (cut reason
  `symmetry`), and after a step.  The sums are computed afresh, a few hundred bit tests, 0.35% of the samples of a
  proof of 1472.
* The tactic `symmetry`, on by default since bf49ab9; the matching path stays without it.

## Results

One thread on the M1 Max shared with other jobs, `matroid prove` (the incumbent P\* as a value, no seed or
ceiling), nodes and G cycles of the process:

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

* The bench set's games 1050, 1100, 1133, 1170 and 1184 (proofs of 33 to 620 nodes) took 0.86, 0.89, 1.00, 0.73 and
  0.87 of the nodes.
* Over the 15 proofs of 936-1350 above 1,000 nodes the geometric means are 0.75 of the nodes and 0.81 of the cycles
  (the quick and bench sets' 16.0 G cycles in all become 12.4); over 1430-2172 they are 0.80 and 0.86.  A node costs 2%
  to 18% more cycles: the rule's arithmetic is under 1% of that, the rest is the bound's work on the candidates the
  rule drops (a child whose parent's heaviest set lost a member to a drop cannot inherit it and solves).
* `taxman bench` (standalone, seeded from n − 1, one thread), with 1472 and 1475 added: 126.2 s to 95.9 s in all; 1472
  541,939 to 352,483 nodes (59.1 to 40.5 s), 1475 743,592 to 553,347 (66.4 to 54.8 s).
* The chain 1..1300 (one thread, checkpoints every 120 s): 402,205 to 290,503 nodes (×0.72), 31.3 to 26.0 s of search,
  41.7 to 35.8 s of wall time, every score, method and proof that of `results/strict.jsonl`.
* The chain 2126..2175 behind `strict.jsonl` (18 exhausted searches, the rest repeats, primes and ceilings): 1,811,690
  to 1,695,594 nodes (×0.94), 234.5 to 225.9 s, the same scores.  2160 took 351,118 to 320,132 nodes, 2170 519,394 to
  440,988, 2172 516,133 to 510,340, and 2166 and 2168 1% and 8% more.  The share is smaller here than at 1000-1500.
* **The hopeless test alone** (drops switched off, measurement only), on 13 of the proofs: 0.89 of the nodes and 0.90
  of the cycles, where the whole rule gives 0.71 and 0.77 on the same games (1287 17,631 nodes against 10,228; 1472
  456,716 against 309,591).  The drops are most of the gain, and they are why the rule does better than the pilot's
  hopeless test (0.948).
* **The Lagrangian of the inequality on the GF(2) bound** (4e80430, knob `SymmetryBound`; `matroid prove
  --symmetry-bound pair|joint`).  At a node whose GF(2) bound does not cut, the pairs its heaviest set breaks were
  bounded again by the heaviest common independent set under doubled weights, each pair alone or all at once.  The
  bound was valid (never below the best completion that keeps the inequalities, 3,120 bounds) and it cut (296 solves
  and 122 cuts in the searches of 880-1010), but on top of the rule it lost: on 11 proofs 1.01 of the nodes and 1.02 of
  the cycles (all at once 1.01 and 1.03), 1430 262,611 and 276,033 nodes against 257,461, 1521 75,817 against 70,829.
  A cut ends a loop whose conflicts the search would have learned from, and the drops had already taken what the
  inequality cuts most often.

**Checks.**  The tests ([the tests](../../../docs/design/tests.md)) hold the pairs to their definition, the images of
optimal and random promotion sets to the formula, every optimum to every inequality, the rule to its definition by
enumeration, and the soundness harness with the rule over both bounds, on two threads and forgetting every step.  Every
board 2..=600 with pairs (470) finds its optimum from the root.  The tripwires moved on purpose (600: 533 to 365; 936:
665 to 385; 1000: 2657 to 2172; the rows `symmetry=off` keep the old ones), and the probe's fingerprints did not move.

## Decision

Kept, on by default: the rule saves about a fifth of the cycles of a proof near 1000-1500, less near 2175 (a twentieth
on the chain there), a constant share as the pilots found, not a lower slope.  Not kept: the hopeless test alone, and
the Lagrangian of the pairs' inequalities (taken out in 92644b3), which cannot do much where the bound barely cuts.
Open: the exchanges that hold only at a node (the dominance study found them in the matching trees, almost none in the
GF(2) trees), and sums kept along the path instead of computed afresh, which would save under 1%.

## Files

The folder holds only this README.md.  The example `matroid`, with the files of `examples/common/` it included, was
removed from master on 2026-10-17 and builds at commit 8753189: `git checkout 8753189`, then `cargo build --release
--example matroid` in `rust/`.

## Commits

* 1e00d42: the root's prime exchanges in the library (`search::symmetry`), and Theorem 13.
* 706cd8d: the tactic `symmetry` (off): the rule in the search.
* bf49ab9: `symmetry` on by default; the matching path stays without it.
* 4e80430: the Lagrangian of the pairs' inequalities on the GF(2) bound, behind a knob.
* 92644b3: the Lagrangian taken out again.
* 8753189: the last commit with the `matroid` example.
