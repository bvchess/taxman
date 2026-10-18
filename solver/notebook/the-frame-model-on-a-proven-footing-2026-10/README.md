# The frame model on a proven footing (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The frame model rested on five assumptions, validated and never proven: the greedy removal of impossible
sources, `unzip`'s mandatory moves, freedoms = factors − sources, closed frames taking no part, and a promoted number
paying only one frame forward.  A consultation on the theory (2026-10-15) proposed that all of them follow from one
statement about the game, *U: some optimal set plays U\*_{≥3}* (the members of U\* with three or more prime
factors), and a per-board fact, *N: the numbering of the frames is clean*, and gave proofs: Lemma Ω (frames are layers
by the number of prime factors), Theorem L (playability splits by layer), Theorem B (every optimal set plays the
largest prime and U\*_2) and Theorem M (given N, U\* ∪ L is playable iff L is a promotion set).  Its first steps,
E1 to E4, are this entry: check the hypotheses on every board, test the theorems against the code and the data, and
write them down.  The statements and proofs are now in [THEORY.md, "The frame model: what is proven and what
remains"](../../../docs/THEORY.md#the-frame-model-what-is-proven-and-what-remains); DESIGN.md (§2, "What optimal
means") and the README say what each proof kind rests on.

*E1, the census* (commits 631e027, a122f43).  `unzip` records its pairs as *claims*, front (a source with one
unclaimed factor takes it) or back (a factor wanted by one source goes to it), in `Frame::claims` and, for the frames
the search takes, `SearchFrames::claims`; `frames::numbering` exposes the frames before any merge with their numbers
and the frames in front of them, from the function `build_frames` itself uses; `BoardFrames::merged_away_reach` counts
the reach of `Home::MergedAway`.  None of it changes a frame or a tree (the probe's fingerprints, the tripwires, the
suite).  `examples/structure.rs` checks, on each board: (A) every frame in one layer, its factors one below, the
frames in front one below; (C) N; (D) no candidate has the factor of a back claim among its maximal factors; (E) no
candidate in frame 0; (F) every candidate's home at least 1; (G) the sources the game with no promotions plays are
an independent descending greedy's.  Every board 2..8191, four threads, 156 s of wall time and 624 s of work (0.27 s
at most, n = 8129); [solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/](../the-claims-of-unzip-in-the-extended-problem-2026-10/README.md):

| what | on the boards 2..8191 |
|---|---|
| checks A, C, D, E, F, G | no violation on any board |
| frame 0's layer | 3 on 8,100 boards; 2 on 80 (8, 12-14, 18-24, 28-34, 42-54, 70-76, 78-84, 98-120, 138-144, 182-186), exactly those where a frame of layer 2 has freedoms; none on the 10 boards with no frame with freedoms |
| the numbering against the layers | on every board frame h is the frames with freedoms of one layer, ℓ₀ + h: the breadth-first numbering is the numbering by layer |
| sources greedy rejects | none of layer 4 or more on any board; of layer 3 on 6,551 boards, the first 741 (10 of 523 at 4000, 30 at 8191); the rest in layers 1 and 2 |
| claims | 108,454 front and 4,602,026 back in the frames as setup forms them, 1,630 and 2,201,516 in the frames the search takes |
| back claims whose factor has a composite among the factors | on 7,423 boards (the boards where check D has something to check) |
| `Home::MergedAway` | reached on one board, 114, twice, never counting as an outside move |
| largest | 22 frames (6000), 9 numbers, layer 12, 2,281 candidates (8190) |

*E2, Theorem M against the code* (commit de63aba).  `tests/model_exact.rs` decides playability on the whole board
with an oracle that shares no code with `frames` or `model` (`tests/common/playable.rs`: the peel of THEORY's
Theorems 1-3, linear with counters, and a play order from the precedence of its pairs), itself checked on every subset
of the boards to 20 against the rules (a dynamic programme over pot states), the brute force and the table.  Then
both directions of Theorem M: on every board to 100 every set L with U\* ∪ L playable is a promotion set whose game
replays to in_the_bag + ΣL, every promotion set is playable with U\*, the two families are equal and the heaviest
is the table's (11,635 sets; an ignored test, 101-120, 167,681 sets in 9.6 s, 74,971 of them at 114); and random sets
on 60-1300 (100 promotion sets made by promoting candidates in random orders, and 100 maximal playable extensions of
U\*, per board) and, ignored, 2000-8191 (10 each, 14 s).  No failure anywhere (and none in two larger throwaway
campaigns: every board 61-420 with 40 sets of each kind, and 600 of each at 500-1500).  Below about 70 few numbers
can be promoted (318 sets on the boards to 50), which is why the exhaustive range goes to 100 rather than to the
40 the consultation suggested.

*The near miss.*  A scan of 2..2000 for the back claims (s, f) whose factor has a composite c up to n/2 in a frame
of the root found 67, on 27 boards, all in 98-120 and 182-186 (the boards where layer 2 has freedoms): none of them
a candidate (check D), none playable with U\* — and every one accepted by the model alone, so on those boards the
candidate filter is the only thing between the model and an unsound promotion.  The first is n = 98: 77 = 7·11
claims 11, and 22 = 2·11 would be taken into frame 0, a path of the six sources 57, 62, 65, 91, 93, 95 on the
factors 2, 31, 3, 19, 5, 13, 7, with the factor 2; on the board 22 and 77 close the path into a cycle of eight
numbers on eight factors.  The unit test pins it.  (The consultation's example, n = 19 with {15, 10, 6}, is not a
claim on the board: there 3 is not claimed, as it said, and the model alone refuses 6, the triangle being
unwinnable.)  The distinct near misses are (77, 11, 22), (77, 11, 33), (91, 13, 26), (91, 13, 39), (119, 17, 34),
(119, 17, 51), (133, 19, 38) and (133, 19, 57).

*E3, Theorem B against the data* (commit d7a322a).  `tests/bottom_layers.rs` builds U\*_2 by Kruskal's algorithm
(the same as setup's on 500 boards to 8191) and checks the theorem's statements (the one prime is the largest; the
sources of layer 2 are U\*_2; the numbers of layer 2 up to n/2 keep U\*_2 a forest; none where layer 2 is closed) on
every game of optimal.json (1,299), `results/strict.jsonl` (2,174) and the chain 2176-2276 of
`solver/frame/testdata/c8g.chain-2176.jsonl` (101); on every optimal set the prover enumerated (987 sets of 2..989, 57 of
798..1019 with the upper part fixed), each rebuilt from its line, checked playable and worth the optimum; and on every
optimal set of the boards 2..48 (and, ignored, 49..54) found again by a search with the oracle alone, whose counts are
the prover's.  No exception.  The games play numbers of layer 2 up to n/2 on 76 boards, all at most 186 (9, 10, 14,
15, 21, 25, 35, 39, 49, 57, 69, 91; at 114 both 49 and 57), always where layer 2 has freedoms; the EC2 chain plays
none.

*N as a hard check* (commit 5e44ac6).  `build_frames` refuses a board whose numbering is not clean, naming the two
frames and their numbers, from what the numbering has already found.  Every supported board passes, so nothing
changes: the probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a (`gf2=on`), its cycles 4.728 G and 5.769 G
(4.727 and 5.781 before the entry), the tripwires, `taxman regress --to 1000 --threads 1` (8.6 s), and the chain
1..1300 on one thread line for line equal to the base commit's (290,503 nodes) apart from times and the build.  A
punctured board (the sacrifice study's) is not checked: 56 without 39 has a frame numbered 1 with a frame with
freedoms numbered 1 in front of it, so Theorem M does not cover the frame model there (`sacrifice`'s module comment
says so).

*E4, the write-up.*  THEORY.md has Lemma Ω, Theorems L, B and M (with M1-M6, each tied to a step of setup), N, U
with what is known of it and where a proof stops, the corollary (with N, the answer is opt(n) iff U holds at n), and
the evidence rows.  Theorem L's proof is shorter than the consultation's: a whole set's reduction instance is the
disjoint union of its layer instances, and peel decides each.  The tests add about 8 s to `cargo test --release`
(the two new files 3 and 4 s, the frames' unit tests 0.2 s, the sample in `tests/wide.rs` 2 s).

*What it changes.*  The frame solver's claim is now: on every board it supports, its score for a game proven
`exhausted`, `trivial` or `same_search` is opt(n) iff U holds at n, given that the code is right; `cap` and
`prime_formula` games need nothing of the model.  U holds for every n ≤ 2175 (the prover).  The next steps of the
consultation certify U game by game past 2175 (barred parity bounds with layers 1 and 2 fixed by Theorem B, then a
search of the members they leave open; the register's last theory row).
