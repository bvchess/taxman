# Local prime exchanges at the nodes (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  The [anatomy of a hard subtree](../the-anatomy-of-a-hard-subtree-2026-10/README.md) found that a node
often goes on without a member at a small price because the bound takes, instead, the member with one prime
exchanged.  Are those swaps symmetries of the nodes where they happen, so that the search could prune one side
soundly, and how much would that shrink the hard trees?  This is the anatomy's third hypothesis, "local prime exchanges
are near-symmetries".

**Answer.**  No.  An exchange of two primes is a symmetry of a node almost only where it is one of the root's
symmetric pairs, which the search already uses.  Elsewhere the multiples of the smaller prime reach, through the
frames, a number at the top of the board whose image is past n (3350 = 50·67, whose image is 3550 at 3441), or a
member that the path has decided on one side only.  Of the 152,088 swaps of 3300-14 that exchange two primes, 5,691
pass the check, and 119 of those are not a pair of the root's.  Pruning by every exchange that passes changes the
trees by a few percent either way: 3300-14 takes 4.9% fewer steps, 3441-1262 2.5% more, 3441-5459 the same.  The
failing swaps are not even near-symmetries: on samples, 3-18% of them are a dominance.  Not a breakthrough, and nothing
to build.

The ride-along test of the anatomy's first hypothesis finds the prices fixed, but the trees are not their product: the
combinations that a path's own menus allow under the gap vary a hundredfold or more from path to path (section 4).

## Words

* An *exclusion* is a node going on without the member it branched on; its *price* is the node's bound before it less
  the bound at the node's next step.  The other words of the anatomy (unit, home, core, twin) are defined
  [there](../the-anatomy-of-a-hard-subtree-2026-10/README.md#words).
* For primes p < q, the *exchange* π swaps their exponents in every number: π(24·71) = 24·67.  It keeps the
  maximal-factor relation: y is a maximal factor of x exactly when π(y) is one of π(x).
* An *exchange swap* is an exclusion of a member x after which the node's heaviest set holds π(x) for an exchange π
  that maps x onto another candidate: leaving 1704 = 24·71 out brings 1608 = 24·67 in.
* The root's *symmetric pairs* are the exchanges that map the root's frames and candidates onto themselves; the search
  keeps c_p ≤ c_q for each, c_r being the sum of the cofactors of r ([glossary](../../../docs/GLOSSARY.md#the-search),
  "prime exchange"; [THEORY.md, Theorem 13](../../../docs/theory/bounds.md#prime-exchanges-symmetries-of-the-promotions)).

## What changed in the solver

**Step 0: the hook costs a default build nothing.**  `search::StepHook` and every site of it in the searcher are built
only with `--features studies` (commit c80bb3c).  A default build compiles them out, and its search loop is the one
before the hook (46c4e76).  The quick check passes, and so does `cargo test --features studies`, whose test of `wall
anatomy` runs a search with the hook and without it and finds the same tree.

**For this study**, in the studies build only, the hook is told of each exclusion before it is made
(`StepHook::excluding`).  It may also narrow a node's candidates by a rule of its own (`StepHook::narrow`), which the
searcher applies as it applies the rule of symmetric pairs: the candidates dropped leave the node's bounds, and a node
with no completion left is cut.  The study is the command `wall exchanges`
([examples/wall/exchanges.rs](../../frame/examples/wall/exchanges.rs)).  The production search is unchanged: the one
edit outside the studies build moves the tail of the rule of symmetric pairs, which takes its drops out of the bounds,
into a function both rules call, with the same steps.

## 1. When an exchange is a symmetry of a node

**The setting.**  A node has frames, each frame f with its moves M_f and factors Y_f, and candidates C, each a factor
of its home frame; C_f is the set of candidates of home f.  A *completion* is a set F ⊆ C that the node can promote:
every frame f is winnable with F applied, that is with the movers M_f ∪ (F ∩ C_(f+1)) and the factors Y_f less
F ∩ C_f (solver/frame/src/model/promote.rs).  The *movers* of f are M_f ∪ C_(f+1); a mover and a factor of one frame
are *adjacent* when the factor is a maximal factor of the mover.  Whether a frame is winnable depends on this
bipartite graph alone (the peel), and within a frame the factor graph lists every maximal-factor pair.

For a set A of numbers that π moves, with π(A) = A, π_A is π on A and the identity elsewhere.  A number's *place* at
the node is one of: a move of frame f, a factor of f in C, a factor of f not in C.

**Lemma 1 (an exchange at a node).**  Suppose that every member of A has its image in its own place, and that every
number π moves that is adjacent to a member of A is in A.  Then F ⊆ C is a completion exactly when π_A(F) is, and

    Σ π_A(F) − Σ F = Σ over a in F ∩ A of (π(a) − a),

which is (q − p)(c_p(F ∩ A) − c_q(F ∩ A)) when no candidate in A is divisible by pq, p² or q².

*Proof.*  π_A is a bijection of the node's numbers that keeps every place, so it maps frame f with F applied to frame f
with π_A(F) applied, number for number.  It keeps adjacency: between two members of A because π does; between a member
of A and a number b that π fixes because π(b) = b; between a member a of A and a number b outside A that π moves
because neither a nor π(a) is adjacent to b, both being in A; and outside A it is the identity.  So the two frames are
the same game of Taxman Mini, winnable together.  The freedoms and the count of promotions are counts it keeps.  The
sum is the definition, and under the condition each candidate in A is m·p or m·q with m prime to pq. ∎

**Lemma 2 (inert pairs).**  Call a factor of frame f that is no candidate *private* if it is a maximal factor of
exactly one mover of f, and that mover *inert* in f.  Lemma 1 holds with adjacency taken without the edges of the inert
movers and the private factors, if every member of A also agrees with its image on being inert and on being private.

*Proof.*  In every completion an inert mover that is present can be played last, paying with its private factor,
which no other mover lists and no promotion removes; and a winning order without it exists, since no other mover pays
with that factor.  So frame f with F applied is winnable exactly when it is without its inert movers and their private
factors, and Lemma 1 applies to those reduced frames. ∎

**The checks.**  From the member x that a node is about to leave out, and each exchange with π(x) in C, three checks,
each implied by the one before:

* *whole node*: every number of the node's frames that π moves has its image in its place (Theorem 13's conditions,
  at the node instead of the root);
* *component*: the same for the smallest A that holds x and is closed under π and under adjacency among the numbers π
  moves (Lemma 1);
* *reduced*: the same with the inert pairs left out (Lemma 2).

`wall exchanges` runs all three at every exclusion, for every exchange that maps the member onto another candidate.

**The rule.**  Let the reduced check pass for A at a node N₀ with promotions P₀.  Then every heaviest completion F of
N₀ keeps w(F) = Σ over a in F ∩ A of (π(a) − a) ≤ 0: otherwise π_A(F) is a heavier one.  So the subtree of N₀, its
later steps and all their children, needs only the completions that keep it.  At a node below N₀ with promotions P
and candidates C, the members of P − P₀ in A add b to w, and the candidates of negative weight can take at most r off:
the node is cut if b > r, and a candidate of positive weight w(x) is dropped if b + w(x) > r.  This is the rule of
symmetric pairs, with the node's own exchange and the offset of its promotions.  With A = {x, π(x)} and x = m·q it says
what the hypothesis proposed: the branch that leaves x out leaves out m·p too.

*Soundness.*  If the incumbent is not optimal, take a heaviest promotion set S.  At every node N₀ whose subtree holds
S, S − P₀ is a heaviest completion of N₀ (a heavier one would make a heavier promotion set), so it keeps each
constraint derived at N₀.  Each constraint is argued alone, as Theorem 13's are, so S keeps them all at once, and the
root's pairs as well.  The bounds, the forced members and the nogoods hold for every completion, so for these.  So
the search never prunes S, and finds a set of its weight.  With the incumbent at the optimum, as in a unit's proof,
nothing beats it, and the rule changes only the tree.

**How the whole-board argument extends, and where it fails.**  Theorem 13 is Lemma 1 at the root with A all the
numbers π moves.  At a node the frames have changed: members are promoted (moves of the frame in front of their home),
excluded (factors that are no candidates) or decided by reductions (in no frame).  The check fails at the first number
of the component whose image is not in its place.  The failures at the exchange swaps (section 2) fall in three
groups:

* *Numbers pushed past n* (`M/>`, two fifths to a half of the failures): a move m·p at the top of the board whose
  image m·q is past n.  The multiples of p form a connected ladder through the frames, since m·p is adjacent to its
  maximal factors (m/r)·p where it moves and to its composites (m·r)·p where it is a factor, so the component of a
  candidate reaches the top.  At 3441, 1704 = 24·71 for 1608 = 24·67 fails in all its 15,893 swaps, 15,187 times at
  3350 = 50·67, whose image is 3550.  The root's pairs are the exchanges for which the top is symmetric.
* *Decided members* (`C/M`, `M/C`, `C/F`, `F/C`; a quarter to two fifths): the path promoted or excluded one side of
  a pair and not the other, as 1206 = 18·67 promoted while 1098 = 18·61 is still a candidate.  Numbers in no frame
  (`M/-`, `F/-`, `C/-`) are the same at the root: the frame builder or a reduction left a number's image out of the
  frames, as 1802 = 2·17·53 against the move 2014 = 2·19·53 at 3441.
* *Numbers in other frames* (marked `*`): the image is in another frame than the number.  These are under 1%.

The inert pairs of Lemma 2 never turned a failure into a pass.  The commonest failure of their own kind is 1636 =
4·409 for 1076 = 4·269 at 3300-14, where only one of the two is inert where it moves.

## 2. How often a swap is a symmetry of its node

The three traced units of the anatomy were solved again, on one thread from the known optimum, with the checks at
every exclusion (`wall exchanges`, `runs.jsonl`; `tables.py` makes the tables, and `analysis.txt` has all of them).
The trees are the anatomy's: 512,966 nodes and 104,400 leaves at 3300-14, 437,940 and 88,755 at 3441-1262, 22,441 and
4,706 at 3441-5459.

| | 3300-14 | 3441-1262 | 3441-5459 |
|---|---|---|---|
| exclusions | 345,237 | 342,969 | 18,075 |
| an exchange maps the member onto another candidate | 97.6% | 99.0% | 98.8% |
| one passes the whole-node check | 8.3% | 7.0% | 1.2% |
| one passes the component check, and the reduced one | 8.3% | 7.0% | 1.2% |
| one passes and is no pair of the root's | 593 (0.2%) | 2 | 0 |
| exchange swaps | 152,088 (44.1%) | 137,617 (40.1%) | 7,093 (39.2%) |
| the swap's exchange passes the reduced check | 5,691 (3.7%) | 22,455 (16.3%) | 200 (2.8%) |
| it passes and is no pair of the root's | 119 | 1 | 0 |
| cheap exclusions (priced under 50) | 68,836 (19.9%) | 70,665 (20.6%) | 669 (3.7%) |
| cheap exchange swaps | 39,861 (57.9%) | 61,962 (87.7%) | 539 (80.6%) |
| the swap's exchange passes | 5,576 (14.0%) | 20,915 (33.8%) | 166 (30.8%) |
| it passes and is no pair of the root's | 4 | 1 | 0 |

Nearly every exclusion has an exchange that maps the member onto another candidate, and two fifths of them are
exchange swaps, as the anatomy found.  An exchange passes at 1-8% of the exclusions, and all but a few hundred of those
are the root's symmetric pairs.  The component check passes 71 times more than the whole-node check at 3300-14, and
the reduced check never more than the component check.

**The cheap exclusions** are exchange swaps 58-88% of the time.  At 3441-1262 a third of those pass, all on the root's
pairs, among them the anatomy's 197 ↔ 199.  Its 67 ↔ 71 fails every time.  The cheap exclusions met most:

| unit | member left out | came in | times | median price | exchange swap | passes |
|---|---|---|---|---|---|---|
| 3441-1262 | 1704 = 24·71 | 1420, 1608 | 15,888 | 16 | 67 ↔ 71 | 0 |
| 3441-1262 | 1688 = 8·211 | 1266, 1592 | 8,980 | 24 | 199 ↔ 211, a root's pair | all |
| 3441-1262 | 1548 = 36·43 | 1269, 1290, 1476 | 7,494 | 30 | 41 ↔ 43 | 0 |
| 3441-1262 | 1712 = 16·107 | 1284, 1648 | 5,639 | 16 | 103 ↔ 107 | 0 |
| 3441-1262 | 1592 = 8·199 | 796, 1576 | 5,403 | 8 | 197 ↔ 199, a root's pair | all |
| 3300-14 | 1220 = 20·61 | 1340, 1464 | 5,765 | 24 | 61 ↔ 67 | 0 |
| 3300-14 | 1608 = 24·67 | 1340, 1464 | 5,256 | 24 | 61 ↔ 67 | 0 |
| 3300-14 | 1644 = 12·137 | 1370, 1572 | 2,590 | 12 | 131 ↔ 137 | 0 |
| 3300-14 | 1148 = 28·41 | 1107 | 1,983 | 41 | no | - |
| 3300-14 | 1572 = 12·131 | 1524 | 1,653 | 48 | 127 ↔ 131, a root's pair | all |

**Where the check fails**, at the exchange swaps, by the places of the number that failed and of its image: `M`, `F`
and `C` a move, a factor and a candidate; `>` past n; `-` in no frame; `*` in another frame; `~` the same place, only
being inert or private differs.

| kind | 3300-14 | 3441-1262 | 3441-5459 |
|---|---|---|---|
| M/>: a move whose image is past n | 36.7% | 49.3% | 52.0% |
| C/M, M/C: one side promoted | 24.2% | 37.5% | 23.6% |
| C/F, F/C: one side excluded | 4.2% | 1.7% | 1.8% |
| M/-, F/-, C/-: the image in no frame | 16.9% | 7.0% | 21.3% |
| C/C~, M/M~: inert or private on one side | 17.4% | 3.7% | 1.2% |
| in another frame (`*`) | 0.6% | 0.8% | 0.2% |

## 3. What pruning would save

Each unit was solved again with the rule (`--prune`): wherever the reduced check passes, the constraint is kept for the
node's subtree, and candidates are dropped and nodes cut by it.

| unit | steps | with the rule | nodes | with the rule | leaves | with the rule | dropped | cut |
|---|---|---|---|---|---|---|---|---|
| 3300-14 | 1,248,609 | 1,186,793 (−4.9%) | 512,966 | 487,772 | 104,400 | 99,108 (−5.1%) | 173 | 3 |
| 3441-1262 | 1,099,410 | 1,127,111 (+2.5%) | 437,940 | 445,690 | 88,755 | 90,102 (+1.5%) | 264 | 0 |
| 3441-5459 | 52,855 | 52,855 | 22,441 | 22,441 | 4,706 | 4,706 | 0 | 0 |

The rule rarely applies, and changes each tree by a few percent, either way.  Where an exchange of a root's pair
passes, its constraint is mostly the rule of symmetric pairs again, which has already dropped what it can.  A
constraint at a node can still be the stronger: its component can be a part of the pair's multiples, and it leaves
out the node's own promotions.  At 3441-1262 all 264 drops are by the root's pairs; at 3300-14, 84 of 173 are by the
others.  A drop changes which
conflicts a searcher meets and learns, as the rule of symmetric pairs does, so a tree can grow.  The best and the
conclusion are the same in all three, as they must be with the incumbent at the optimum.

**The rule keeps the optimum.**  With nothing to find in a unit's proof, the check of the best is weak.  So the 501
whole games 800-1300 were searched from a floor 60 under their optimum (`soundness.jsonl`), where the search must find
the optimum itself:

| rule of symmetric pairs | this rule | steps | optimum missed | candidates dropped | nodes cut |
|---|---|---|---|---|---|
| on | off | 770,240 | 0 | - | - |
| on | on | 770,268 | 0 | 238 | 476 |
| off | off | 1,035,257 | 0 | - | - |
| off | on | 781,097 | 0 | 88,832 | 472 |

With the rule of symmetric pairs, this rule adds nothing.  Without it, this rule finds the same exchanges at the
nodes and takes 96% of its gain: further evidence that the exchanges that pass are the root's.

**How near the failing swaps are.**  Where the check fails, the swap may still be a dominance that some other sound
check could prove.  With `--sample 16`, at each exchange swap whose check failed and whose image is the lighter, 16
random maximal completions of the node that hold the image were tried with the image exchanged back for the member.
This is an estimate, not a proof; the maximal sets are the ones to try, since a set can be promoted with the exchange
when a maximal set holding it can.

| unit | steps | swaps sampled | dominance held | cheap ones sampled | held |
|---|---|---|---|---|---|
| 3300-14 | the first 200,703 | 22,307 | 4,007 (18.0%) | 3,835 | 538 (14.0%) |
| 3441-1262 | the first 200,703 | 19,463 | 1,507 (7.7%) | 7,473 | 735 (9.8%) |
| 3441-5459 | all 52,855 | 6,277 | 205 (3.3%) | 269 | 25 (9.3%) |

Most failing swaps are no dominance either: some random maximal completion that holds the image cannot be promoted
with the member in its place.  The 18% at 3300-14 are mostly one pair, 1636 for 1076 (all 2,436 of its samples held), an
exclusion that costs 560, not a cheap one.  The anatomy's 1704 for 1608 held in 239 of 2,873.

## 4. Ride-along: are the trees the count of fixed prices?

The anatomy's first hypothesis is that a tree enumerates the combinations of exclusions, at fixed prices, that fit
under its gap.  `prices.py` tests it on traces of the three units, made again with `wall anatomy --trace`.  A node's
*menu* is what reaching each of its children costs: the prices of the exclusions its loop made before that child.  If
the prices were fixed and every path met the same menus, the leaves would be the combinations, one child chosen at
each node along any one path, whose costs add up to no more than the unit's gap.  For 400 random leaves of each unit,
the script counts those combinations along the leaf's own path, at fixed prices (each pair of conflict and member at
its median over the tree) and at the prices met:

| unit | gap | leaves | prices | combinations a path, p10 | median | p90 | median over the leaves |
|---|---|---|---|---|---|---|---|
| 3300-14 | 998 | 104,400 | fixed | 64,178 | 1,957,024 | 76,257,113 | 18.7 |
| | | | as met | 63,338 | 1,962,970 | 74,989,884 | 18.8 |
| 3441-1262 | 970 | 88,755 | fixed | 82,801 | 1,617,769 | 23,477,194 | 18.2 |
| | | | as met | 74,976 | 1,597,870 | 24,705,290 | 18.0 |
| 3441-5459 | 960 | 4,706 | fixed | 2,922 | 35,384 | 362,084 | 7.5 |
| | | | as met | 2,837 | 34,596 | 346,525 | 7.4 |

The prices are fixed: the counts at fixed prices and at the prices met agree within 3% at the median and 10% at the
deciles, and a pair's price spreads by a tenth of itself or less (0.06-0.12 of the median, from p10 to p90).  But a
tree is not their product.  The count a path predicts spreads over two to three decades from path to path, and its
median is 7 to 19 times the leaves.  The exclusions on offer below a node depend on the choices above it, so the
combinations are not independent.  Counting the prices does not replace searching them; they explain the depth of a
path, as the anatomy measured, but not the number of paths.

## Decision

Not a breakthrough, and not worth proving into the solver.  Local exchanges are symmetries of the nodes only where the
root's pairs already are.  Elsewhere the ladder of multiples reaches the top of the board, and the path's decisions
break the rest.  The rule that uses them changes the trees by a few percent either way.  The cheap exclusions of the
hard trees are mostly exchanges of primes that are not symmetric at the node, and not even a dominance on samples, so
no symmetry argument removes them.  The instrument (`wall exchanges`, with `StepHook::excluding` and
`StepHook::narrow`) stays for studies; the production search is unchanged.

## Files

* `runs.jsonl`: the lines of `wall exchanges` for the three units, in check mode, with `--prune`, and with `--sample
  16`.  The fields are in the module comment of
  [examples/wall/exchanges.rs](../../frame/examples/wall/exchanges.rs).  From `solver/frame`, after `cargo build
  --profile test --example wall --features studies`: `nice -n 10 target/debug/examples/wall exchanges --suite
  ../notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl --known ../../results/strict.jsonl --ids
  3300-14 [--prune | --sample 16 --nodes 200000] 3300`, and the same with `--ids 3441-1262` or `--ids 3441-5459` and
  `3441` (`--sample 16` without `--nodes` for 3441-5459).
* `soundness.jsonl`: the whole games 800-1300 from a floor 60 under their optimum, with and without `--prune`, with
  and without the rule of symmetric pairs: `wall exchanges --known ../../results/strict.jsonl --root --below 60
  --tactic symmetry=on|off [--prune] 800-1300`, each line without its lists (`swaps`, `cheap`, `fails`, `swap_fails`,
  `fail_examples`), which made the file 4.5 MB.
* `tables.py` makes the tables of sections 2 and 3 (`python3 -I tables.py runs.jsonl soundness.jsonl`), and
  `prices.py` the table of section 4 from the traces (`python3 -I prices.py TRACE_DIR ID...`, reading them with the
  anatomy's `analyze.py`).  `analysis.txt` is their output.
* The traces, 350 MB, are not kept: `wall anatomy --trace DIR`, as the anatomy's entry gives it, makes them again.

The runs took about 65 CPU minutes on this laptop, on one or two threads, on 2026-10-18, and the builds and tests about
20 more; nothing ran on EC2.  The numbers above are those of the final build: a first build gave the same trees and
counts, with coarser labels for the failures.  Commits: c80bb3c (step 0) and the commit of this entry.
