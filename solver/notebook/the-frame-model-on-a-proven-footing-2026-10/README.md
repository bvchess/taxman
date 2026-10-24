# The frame model on a proven footing (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-15.  Concluded 2026-10-15.

**Question.**  Can the five assumptions of the frame model, validated on the data and never proven, be proved from one
statement about the game and one fact about each board?  **Answer.**  Yes: the proofs are in [frame-model.md, "The frame
model"](../../../docs/theory/frame-model.md), and the checks on every board to 8191,
on the code and on the recorded optimal sets found no exception.  The frame solver's answer for a game proven
`exhausted`, `trivial` or `same_search` is opt(n) exactly when statement U holds at n.

## Method

The assumptions were the greedy removal of impossible sources, `unzip`'s mandatory moves, freedoms = factors − sources,
closed frames taking no part, and a promoted number paying only one frame forward.  A consultation on the theory
(2026-10-15) proposed that all of them follow from statement U (some optimal set plays U\*_{≥3}, the members of U\* with
three or more prime factors) and statement N (the numbering of the frames is clean, a fact of each board), and gave the
proofs of Lemma Ω (frames are layers by the number of prime factors), Theorem L (playability splits by layer), Theorem B
(every optimal set plays the largest prime and U\*_2) and Theorem M (given N, U\* ∪ L is playable exactly when L is a
promotion set).  Four steps checked them and wrote them down:

* **E1, the census** (631e027, a122f43).  `unzip` records its pairs as *claims*, front (a source with one unclaimed
  factor takes it) or back (a factor wanted by one source goes to it).  `cargo run --release --example structure -- 2
  8191 --threads 4` checks on each board: (A) every frame lies in one layer, its factors one below, the frames in front
  one below; (C) N; (D) no candidate has the factor of a back claim among its maximal factors; (E) no candidate is in
  frame 0; (F) every candidate's home is at least 1; (G) the sources of the game with no promotions are an independent
  descending greedy's.
* **E2, Theorem M against the code** (de63aba).  `tests/model_exact.rs` checks both directions of the theorem against an
  oracle that shares no code with `frames` or `model` (`tests/common/playable.rs`: the peel, and a play order from the
  precedence of its pairs).  The oracle is itself checked on every subset of the boards to 20 against the rules.
* **E3, Theorem B against the data** (d7a322a).  `tests/bottom_layers.rs` builds U\*_2 by Kruskal's algorithm and checks
  that the one prime is the largest, that the sources of layer 2 are U\*_2, that the numbers of layer 2 up to n/2 keep
  U\*_2 a forest, and that there are none where layer 2 is closed.
* **N as a hard check** (5e44ac6).  `build_frames` refuses a board whose numbering is not clean, naming the two frames.
* **E4, the write-up** (c8ee0a8): THEORY.md.  The tests add about 8 s to `cargo test --release`.

## Results

**E1.**  Every board 2..8191 took 156 s of wall time on four threads and 624 s of work (0.27 s at most, n = 8129).  The
output is in [the claims of unzip](../the-claims-of-unzip-in-the-extended-problem-2026-10/README.md).

* Checks A, C, D, E, F and G: no violation on any board.
* Frame 0 is in layer 3 on 8,100 boards, and in layer 2 on 80 (8, 12-14, 18-24, 28-34, 42-54, 70-76, 78-84, 98-120,
  138-144, 182-186), exactly those where a frame of layer 2 has freedoms.  The 10 boards with no frame with freedoms
  have none.
* On every board frame h is the frames with freedoms of one layer, ℓ₀ + h, so the breadth-first numbering is the
  numbering by layer.
* Greedy rejects no source of layer 4 or more, sources of layer 3 on 6,551 boards (the first 741 boards), and the rest
  in layers 1 and 2.
* Back claims whose factor has a composite among the factors occur on 7,423 boards, where check D has something to
  check.  `Home::MergedAway` is reached on one board, 114, twice, and never counts as an outside move.

**E2.**  No failure.  On every board to 100, every set L with U\* ∪ L playable is a promotion set whose game replays to
in_the_bag + ΣL, every promotion set is playable with U\*, the two families are equal, and the heaviest is the table's
(11,635 sets).  An ignored test covers 101-120 (167,681 sets in 9.6 s, 74,971 of them at 114).  Random sets pass on
60-1300 (per board 100 promotion sets made by promoting candidates in random orders, and 100 maximal playable
extensions of U\*) and, ignored, on 2000-8191 (10 each, 14 s).  Two larger throwaway campaigns found no failure either.
The exhaustive range goes to 100, not the consultation's 40, because below about 70 few numbers can be promoted (318
sets on the boards to 50).

**The near miss.**  A scan of 2..2000 for back claims (s, f) whose factor has a composite c up to n/2 in a frame of the
root found 67, on 27 boards, all in 98-120 and 182-186.  None is a candidate (check D) and none is playable with U\*,
yet the model alone accepts every one, so on those boards the candidate filter is the only thing between the model and
an unsound promotion.  The first is n = 98: 77 = 7·11 claims 11, and 22 = 2·11 would be taken into frame 0, a path of
the six sources 57, 62, 65, 91, 93, 95 on seven factors.  On the board, 22 and 77 close the path into a cycle of eight
numbers on eight factors, and a unit test pins this.  The eight distinct triples (s, f, c) run from (77, 11, 22) to
(133, 19, 57).

**E3.**  No exception, in 3,574 recorded games: every game of optimal.json (1,299), of `results/strict.jsonl` (2,174)
and of the chain 2176-2276 of `solver/frame/testdata/c8g.chain-2176.jsonl` (101).  None either in the 1,044 optimal sets
the prover enumerated (987 of 2..989, 57 of 798..1019 with the upper part fixed), each rebuilt from its line, checked
playable and worth opt(n), nor in every optimal set of the boards 2..48 (and, ignored, 49..54), found again by a
search with the oracle alone, whose counts are the prover's.  The games play numbers of layer 2 up to n/2 on 76 boards,
all at most 186 and always where layer 2 has freedoms; the chain of the EC2 run plays none.

**N as a hard check.**  Every supported board passed, so nothing changed: the probe's fingerprints and tripwires,
`taxman regress --to 1000 --threads 1` (8.6 s), and the chain 1..1300 on one thread (290,503 nodes) came out line for
line equal to the base commit's apart from times and the build.  A punctured board (the sacrifice study's) is not
checked: 56 without 39 has a frame numbered 1 with a frame with freedoms numbered 1 in front of it, so Theorem M does
not cover the frame model there (the module comment of `sacrifice` says so).

## Decision

Kept: the proofs, the tests and the census are the footing of the frame solver's answers.  On every board it supports,
its score for a game proven `exhausted`, `trivial` or `same_search` is opt(n) exactly when U holds at n, given that the
code is right; games proven by `cap` or `prime_formula` need nothing of the model.  The prover had proven U for every n
up to 2175, and the certificates of U go game by game past it (barred parity bounds with layers 1 and 2 fixed by
Theorem B, then a search of the members they leave open).

The hard check of N is superseded by the numbering by layer, which is clean by construction ([frame-model.md, "Clean
numbering"](../../../docs/theory/frame-model.md#clean-numbering)).  A board or a game that fails a check above would
reopen the proofs.

## Files

The folder holds only this README.md.  The code is in `solver/frame/`: `examples/structure.rs` (the census, with the
command above), `tests/model_exact.rs` with `tests/common/playable.rs`, and `tests/bottom_layers.rs`.

## Commits

* 631e027, a122f43: E1, the census of the frames' structure and of every board 2..8191.
* de63aba: E2, Theorem M against the code.
* d7a322a: E3, Theorem B against the data.
* 5e44ac6: setup refuses a board whose numbering is not clean (N).
* c8ee0a8: E4, THEORY.md's proofs of Lemma Ω and Theorems L, B and M.
