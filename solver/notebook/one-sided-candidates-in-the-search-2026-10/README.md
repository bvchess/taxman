# One-sided candidates in the search (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-15.  Concluded 2026-10-15.

**Question.**  Can the frame solver search the extended problem Π⁺(O), with the open sources as one-sided candidates, so
that `taxman certify` proves a recorded score on the whole board game by game ([THEORY.md](../../../docs/THEORY.md),
Theorem M⁺ and its corollary), with the default mode untouched?  **Answer.**  Yes: the search of Π⁺(∅) is the default
engine's node for node, every game of 600..1300 that needs a certificate is certified (377 games in 44.9 s on one
thread), and past 1300 a certificate takes the nodes of the plain proof and 1.1 to 1.9 times its time.

## Method

This was stage 2 of experiment E8 (the third consultation of 2026-10-15), in four parts that its plan numbered 18.1 to
18.4, on the base commit 2e8b851 (stage 1: the extended problem, Theorem M⁺ and the oracle tests).  Each part had to
pass the default's gates: the probe's fingerprints (7e8e724717d26545 and f3deaac0dbdc030a), the suite with the
tripwires, `regress --to 1000 --threads 1`, and a one-thread chain 1..1300 line for line equal to the base commit's.

* **18.1, the model.**  `FrameGraph::build_extended(tables, columns, one_sided)` gives each one-sided candidate its own
  frame.  Promoting it adds it to that frame's moves, takes no factor, and spends one freedom of that frame, so the
  frame-by-frame check of a set, the conflicts and the nogoods take it unchanged.  The most promotions a frame set
  allows (the freedoms) gains the one-sided candidates: sound, not tight.
* **18.2, the GF(2) bound.**  `Gf2Graph::new` gives a one-sided candidate of frame j the vector of its maximal factors
  in frame j's matroid and makes it a coloop of the other, with its own unit vector in a free block (THEORY.md,
  Theorem 11 for Π⁺).
* **18.3, the search.**  `Board::prepare_extended_with(open, tactics)` sets Π⁺(O) up with its one-sided candidates among
  the search's.  The problem's form and the checkpoint header record them (format 5), and plain digests do not change.
  Extended mode needs `gf2` and refuses `lagrange` and `symmetry`.
* **18.4, `taxman certify`.**  `certify A-B --results FILE... --upper FILE... --out FILE` (`src/certify.rs`; [the
  certificates guide](../../docs/design/certificates.md)) takes, for each game whose barred bounds say it needs one, the
  open members O.  It checks that the recorded game replays to its score and is a promotion set of Π⁺(O) worth it, then
  searches Π⁺(O) in extended mode from that game, with the ceiling n + V(n − 1) − in_the_bag⁺, V the recorded score.
  The verdict is `certified` if nothing beats the score, and `beaten` if something does: the beating game, refereed by
  `game::replay_finished`, goes to `<out>.counterexample.jsonl`, and the status is 2.  It resumes like `solve`.

## Results

**Correctness.**  Each part was tested against an oracle or brute force:

* 18.1 on every board 2..300 against the stage-1 reference predicate: 415,000 sets promoted at once and one by one
  agree, and their 660,000 conflicts and 330,000 nogoods are unpromotable.  Eight deliberate bugs in a scratch copy were
  all caught.
* 18.2 on 7,000 random problems with free blocks, and on every board 2..120: the 7.6 million promotion sets of the 230
  problems of at most 22 candidates are independent in both matroids, under the root's bound.
* 18.3: the search of Π⁺(∅) is the default engine's node for node at every tripwire board; 16,128 searches of the
  soundness harness on the boards 2..300 agree; on every board to 80 the search's optimum is the heaviest playable set
  that holds F, as the whole-board oracle finds; and game 992 with 20 sources open, resumed every 1,000 nodes, took the
  16,717 nodes of the uninterrupted search.
* 18.1 to 18.3 changed no tree of the default mode: the fingerprints, the tripwires, `regress --to 1000 --threads 1`
  (9.2, 9.1 and 8.9 s) and the chain 1..1300 (290,503 nodes) are what they were.

Certified children are rarer than in the frame model (0.7% of the nodes against 4.2%).  Opening a source raises the rank
of its frame by one, so with every source of layer 3 or more open, frame 0 has rank 186 at game 1200 and 322 at game
2200, beyond the three words of the narrow set widths (rank 192) and the five of the widest (320).  The natural widths
refuse such a problem from game 1266 (four words), and the widest width (128 words) takes it.  With those sources open
the board has 9 frames on 2048..4095 (layers 3 to 11) and 10 from 4096 ([the claims of unzip in the extended
problem](../the-claims-of-unzip-in-the-extended-problem-2026-10/README.md)).

**Certificates of 600..1300.**  The barred bounds of the table's games 2..1300
(`solver/noframe/data/upper_2_1300.jsonl`) took 65 s: 665 games need a certificate, 417 of them with nothing open, and
1.0% of the members are open.  Every game of 600..1300 that needs one is certified: 377 games, 382,887 nodes, 44.9 s on
one thread (the dearest, game 1287, 17,744 nodes).  A planted wrong score (games 1302, 1305 and 1309, each with its
smallest move up to n/2 taken out) is beaten by a game of exactly the chain's score, which the referee accepts.  A
search stopped every 1,000 nodes resumes to the same verdict.  Over the barred bounds of 1301-3314, a certificate needs
one to three block words on every game but 3175 and 3176 (four, |O| = 255, run at the widest width), and none needs more
than five (`examples/extended.rs words`).

**The first certificates past 1300**, on one thread, against the plain proof seeded with the optimum (the ladder's
`--seed-opt`), with the default tactics and without the rule of symmetric pairs, which extended mode refuses:

| n | gap | \|O\| | certify: nodes, s | plain: nodes, s | plain, `symmetry=off`: nodes, s | time over plain |
|---|---|---|---|---|---|---|
| 1400 | 395 | 35 | 12,257, 1.65 | 12,743, 1.51 | 14,572, 2.07 | 1.10 |
| 1445 | 399 | 31 | 5,195, 0.84 | 5,373, 0.73 | 5,700, 0.79 | 1.15 |
| 1592 | 484 | 57 | 26,459, 2.85 | 17,422, 1.53 | 26,580, 2.25 | 1.86 |
| 1680 | 363 | 40 | 16,979, 2.30 | 11,925, 1.32 | 17,043, 1.85 | 1.74 |
| 1880 | 203 | 11 | 5,591, 0.86 | 4,201, 0.59 | 5,563, 0.69 | 1.45 |
| 2000 | 491 | 69 | 55,915, 8.00 | 38,118, 4.89 | 58,290, 6.90 | 1.64 |
| 2112 | 517 | 69 | 15,119, 3.42 | 14,029, 1.99 | 14,057, 2.02 | 1.72 |
| 2156 | 557 | 70 | 43,599, 7.77 | 43,899, 5.76 | 42,188, 5.73 | 1.35 |

The open sources cost the trees almost nothing: a certificate takes 0.84 to 1.08 times the nodes of the plain proof
without the rule of symmetric pairs.  What it costs is the rule (1.0 to 1.5 times the nodes where the rule prunes) and a
fifth to a half more time a node past game 2000 (more candidates, the one-sided elements in the GF(2) bound, the frames
without claims).  The median time over the plain proof is 1.55, inside stage 3's criterion of 2.  Consecutive games
often pose the same Π⁺(O): games 1312 and 1313, 1314 and 1315, and 1316 and 1317 had one digest each.

## Decision

Kept as research code, proven by Theorem 11 for Π⁺.  Two improvements are open.  Theorem 13 for Π⁺ (each prime exchange
mapping the one-sided candidates of each frame onto themselves) would give the rule of symmetric pairs back.  A line
`certified (same problem as m)` would halve the cost of the games that pose the same Π⁺(O), and is not built.

Version-4 checkpoint headers are read after all, which relaxes the plan's refusal of them.  The EC2 chain's build
(55287ff) writes them, and a game it leaves unfinished holds hours of 64-thread work.  A version-4 header reads as
version 5 with the digest of no one-sided candidates, the run goes on under the header it was written with, and a
version-4 checkpoint offered to a search with open sources is refused, with the reason.  The checkpoint of game 1133
after 1,000 nodes (committed as `testdata/n1133_after_1000_nodes_v4.ckpt`, tested in `tests/wide.rs`) finishes under
this build as under the build that wrote it (2e8b851): the same line but for times and build, 3,058 nodes in two
invocations, and 3,146 nodes when stopped once more.

## Commits

* f747855 (18.1), 0c32d11 (18.2), 100eea1 (18.3) and e43b4f2 (18.4), all of 2026-10-15, on the base 2e8b851.
* 3de05b7 (2026-10-15): version-4 checkpoint headers are read again.
