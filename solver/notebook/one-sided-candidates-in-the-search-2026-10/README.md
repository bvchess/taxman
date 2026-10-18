# One-sided candidates in the search (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  Stage 2 of E8 (the third consultation of 2026-10-15): can the frame solver search the extended problem
Π⁺(O) with the open sources as one-sided candidates, so that `taxman certify` proves a recorded score on the whole
board game by game (Theorem M⁺ and its corollary), with the default mode untouched?  Built in four parts, each with
the default's gates (the probe's fingerprints, the suite with the tripwires, `regress --to 1000 --threads 1`, and a
one-thread chain 1..1300 line for line equal to the base commit's, 2e8b851).

*18.1, the model.*  `FrameGraph::build_extended(tables, columns, one_sided)` gives each one-sided candidate its own
frame; its promotion adds it to that frame's moves (winnable, with the shortcut of an unused factor), takes no factor,
spends one freedom of that frame; `edits` puts it in its frame's gains, so the frame-by-frame check of a set, the
conflicts and the nogoods take it unchanged; `frames_touched` replaces the home frame where the searcher checks the
freedoms of a child that promotes several forced members; `FrameGraph::narrow` and `max_promotions_among` (the
freedoms' count plus the number of one-sided candidates, sound, not tight) replace the frame set's in the searcher, the
greedy set, the local search and `max_total`.  Tested against the reference predicate of stage 1
(`ExtendedFrames::promotable`) on every board 2..300 for O empty, U\*_{≥3} (to 120) and five random subsets: 415,000
sets promoted at once and one by one in random orders (290,000 of them promotion sets, 264,000 with a one-sided member),
their conflicts (660,000) and nogoods (330,000) unpromotable, `narrow` dropping no member of a promotion set that extends
the state (and nothing promotable), the count never below a promotion set's size; eight deliberate bugs in a scratch copy
(narrowing, the count, `edits`, the freedoms, the nogood) were all caught.  `build_extended` refuses more frames than
a frame set holds (9), naming the board and the layers, and the census of the frames records the worst case, every
source of three or more prime factors open, on every board to 8191: 9 frames on 2048..4095 (layers 3 to 11) and 10 on
every board from 4096 (`solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`, every earlier field unchanged).  No tree changed: the fingerprints
7e8e724717d26545 and f3deaac0dbdc030a, the tripwires, `regress --to 1000 --threads 1` in 9.2 s, the chain 1..1300
line for line (290,503 nodes).

*18.2, the GF(2) bound.*  `Gf2Graph::new` gives a one-sided candidate of frame j the vector of its maximal factors in
frame j's matroid and makes it a coloop of the other: its own unit vector in a *free block* after that matroid's frame
blocks, `64·words` to a block, so the vectors are no wider than the frames need (THEORY.md, Theorem 11 for Π⁺).
Nothing reads a frame from a block index outside `Gf2Graph::new`, and contraction needs nothing new.  Tested: the
intersection against brute force on 7,000 random problems with free blocks (coloops in the heaviest set of 6,400 of
them, at the heavy end of the weights or anywhere), its warm starts, weight splits and bounds without a member; on
every board 2..120 with U\*_{≥3} and three random subsets open, the 7.6 million promotion sets of the 230 problems of at
most 22 candidates independent in both matroids and the root's bound above the heaviest of them and above the optimum;
random paths of 13 boards from 60 to 1000 and of 1300 (four words) with U\*_{≥3} and a random half of it open, every
level equal to the matroids built afresh at its node, the forced members forced, the certified and solved bounds
without a member right (12,020 nodes, 3,396 one-sided promotions).  Two things to know: certified children are rarer
than in the frame model (0.7% of the nodes against 4.2%), and opening sources raises the rank of their frame by one
each, so that with every source of layer 3 or more open frame 0 has rank 186 at 1200 and 322 at 2200, beyond the
three words the narrow set widths compile (rank 192) and the five of the widest (320).  No tree changed: the
fingerprints, the tripwires, `regress --to 1000 --threads 1` in 9.1 s, the chain 1..1300 line for line.

*18.3, the search.*  `Board::prepare_extended_with(open, tactics)` (`search::prepare_extended`) sets Π⁺(O) up with its
one-sided candidates among the search's.  The problem's form ends with them after a tag no plain form has
(`Problem::ONE_SIDED`), so a plain problem's form and digest are what they were; the checkpoint header has a digest of
them (format 5, which refuses version 4: the probe's windows and the committed checkpoint of 1133 were made again, with
the matching path's tactics they record, and the fingerprints did not move).  Extended mode needs `gf2` and refuses
`lagrange` and `symmetry` (`SearchTactics::refusal_with_one_sided`; `SearchTactics::extended` is the default without
`symmetry`), and a problem one of whose frames needs more block words of the GF(2) bound than the set width's engines
have: opening a source adds one to its frame's rank, so that with every source of layer 3 or more open the natural
widths refuse it from 1266 (four words), which the widest width (128 words, five) takes.  Tested: the search of Π⁺(∅)
is the default engine's on the same problem node for node at every tripwire board, with the default tactics and with
extended mode's, in a table of its own (`EXTENDED`; extended mode's trees are the `symmetry=off` rows of the frame
model at 600, 936 and 1000, and Π⁺(∅) with the rule of symmetric pairs differs from Π_n with it at 600, 660, 936 and
1000, since the claimed pairs change the root's symmetries); the soundness harness in extended mode, 16,128 searches of
1,258 problems of the boards 2..300 from handicapped incumbents under eight configurations, and the game 992 with 20
sources open resumed every 1,000 nodes (22 pieces, 16,717 nodes uninterrupted); the search against the whole-board
oracle on every board to 80 (to 60 with every source open; 69 ignored): its optimum is the heaviest playable set that
holds F, every time.  No tree of the default mode changed: the fingerprints, the tripwires, `regress --to 1000
--threads 1` in 8.9 s, the chain 1..1300 line for line.

*18.4, `taxman certify`.*  `certify A-B --results FILE... --upper FILE... --out FILE` (`src/certify.rs`; DESIGN.md,
"Certificates of U") takes, for each game whose barred bounds say it needs one, the open members O, checks that the
recorded game replays to its score, plays U\* above n/2 and is a promotion set of Π⁺(O) worth it, and searches Π⁺(O)
in extended mode from it with the ceiling n + V(n − 1) − in_the_bag⁺: `certified` if nothing beats it, `beaten` if
something does (the set F ∪ K ∪ L ordered by the whole-board oracle's peel, moved from the tests into the library as
`playable`, finished with the largest legal moves left and refereed by `game::replay_finished`; written to
`<out>.counterexample.jsonl`; status 2).  Resumable like `solve`; a problem whose GF(2) bound needs more block words
than its set width's engines is searched at the widest width.  The barred bounds of the table's games 2..1300 were
made for its tests (`solver/noframe/data/upper_2_1300.jsonl`, 65 s: 665 games need a certificate, 417 of them with nothing
open, 1.0% of the members open).  Every game of 600..1300 that needs one is certified: 377 games, 382,887 nodes, 44.9 s
on one thread (the dearest 1287, 17,744 nodes); a planted wrong score (1302, 1305 and 1309 each with its smallest move
up to n/2 taken out of the game) is beaten by a game of exactly the chain's score, which the referee accepts; a search
stopped every 1,000 nodes resumes to the same verdict.  The words a certificate needs over the barred bounds of
1301-3314 (`examples/extended.rs words`): one to three on every game but 3175 and 3176 (four, |O| = 255), which run at
the widest width; none needs more than five.

*The first certificates past 1300*, one thread, against the plain proof seeded with the optimum (the ladder's
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

The open sources cost the trees almost nothing: the certificates take the nodes of the plain proof without the rule
of symmetric pairs (0.84 to 1.08 of them).  What they cost is the rule (1.0 to 1.5 times the nodes where it prunes)
and a fifth to a half more time a node past 2000 (more candidates, the one-sided elements in the GF(2) bound, the frames
without claims).  The median over the plain proof is 1.55, inside stage 3's criterion of 2; Theorem 13 for Π⁺ (each
prime exchange mapping the one-sided candidates of each frame onto themselves) would take back the rule.
Consecutive games often pose the same Π⁺(O) (1312 and 1313, 1314 and 1315, 1316 and 1317 had one digest each): the
optional line `certified (same problem as m)` of the design would halve those, and is not built.

*Afterwards, version 4 read again.*  The EC2 chain's build (55287ff) writes headers of version 4, and a game it leaves
unfinished holds hours of 64-thread work, so version 4 is read after all (relaxing spec 18.3.1, which had it refused):
a version-4 header reads as version 5 with the digest of no one-sided candidates, every other check stays, the run
goes on under the header it was written with, and a version-4 checkpoint offered to a search with open sources is
refused, saying why.  The base commit's checkpoint of 1133 after 1,000 nodes (version 4) finishes under this build exactly
as under the build that wrote it (2e8b851): the same line but for times and build, 3,058 nodes in two invocations; and
stopped again after 1,000 more nodes and finished, both builds write a version-4 checkpoint and end at 3,146 nodes.  It
is committed as `testdata/n1133_after_1000_nodes_v4.ckpt`, with a test in `tests/wide.rs`.
