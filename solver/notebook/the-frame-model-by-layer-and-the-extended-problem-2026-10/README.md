# The frame model by layer, and the extended problem (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The second consultation of 2026-10-15, after "The frame model on a proven footing", set the goal (its
steps 2B-2D are the entry above): every game past 2175 that
rests on the frame model certified as a whole-board optimum, at a cost close to the solver's own.  Its first steps
for the solver are (2A) to make the numbering by layer the definition, so that clean numbering (N), the one
hypothesis of Theorem M, holds by construction and also on boards with holes, with no result changing; and (3, E8
stage 1) to build the *extended problem* Π⁺(O), the frame model with a set O of the sources of three or more prime
factors left optional, prove it exact (Theorem M⁺), test it against the whole-board oracle, and search Π⁺(∅) with the
existing engine through a chain.

*2A, numbering by layer.*  `frames::numbered_frames` gives a frame with freedoms of layer ℓ the number ℓ − ℓ₀, ℓ₀
the lowest layer of a frame with freedoms (`frames::layer_of`, `Tables::omega`); the frames of one number are merged
in the order of their smallest source, as before.  The Java's breadth-first numbering is kept for the tests, which
find the two equal frame by frame on every board to 400 and on a sample to 8191 (1.3 s).  The refusal of an unclean
board is gone; in its place a debug assertion of Lemma Ω (every frame in one layer, its factors one lower, the frames in
front one lower), which a test runs on every board to 400 and every board of up to 60 numbers with one hole.  A layer
between ℓ₀ and the highest layer with freedoms that has none leaves its number without frames, and the search then
has an empty frame (no moves, factors or freedoms).  No board 1..n has one; the boards with one hole to 127 that do are
32, 33 and 34 without 4, 10, 14 or 15: on 32 without 4, the frame of 18, 20, 27, 28 and 30 (layer 3) splits into two
closed frames, while layers 2 and 4 have freedoms.  The model and the search take the empty frame as it is (a test searches 32
without 4 with the GF(2) bound and on the matching path against the brute force).  On 56 without 39 the breadth-first
numbering was not clean; by layer it is.

No tree changed: the probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a (`gf2=on`), its cycles 4.732 G and
5.770 G; `cargo test --release` with the tripwires; `taxman regress --to 1000 --threads 1` (9.2 s); the chain 1..1300
on one thread line for line equal to the base commit's (290,503 nodes) but for times and the build; and the census
`examples/structure.rs 2 8191` on one thread, every line equal to `solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`'s but for the seconds (605 s
summed against 624).  `examples/sacrifice validate 52` is exact on all 1,378 punctured boards (40.7 s), the boards
with an empty frame among them.  THEORY.md's "Clean numbering" is now "Numbering by layer"; Theorem M and its
corollary lose the hypothesis; a paragraph covers boards with holes, where the model is exact given the sources setup
keeps there, which is what the sacrifice bound rests on (its module comment says so).

*3, the extended problem (E8 stage 1).*  `frames::build_extended(t, open)` builds Π⁺(O) for a set O of members of
U\*_{≥3}: the frames as setup forms them (the components), each with its fixed sources F = U\* ∖ O and all its factors;
freedoms⁺ = factors − fixed sources; the frames with freedoms⁺ numbered by layer and united by number (no claims, no
merge order to keep); the two-sided candidates by setup's two tests with F for the remaining sources; the one-sided
candidates O ∪ R (R: the sources greedy rejected in the frames that hold a member of O), each with the frame of its
layer; in_the_bag⁺ = ΣF.  `ExtendedFrames::promotable` is the reference predicate for a pair (K, L): plain peels of the
unreduced frames with K and L applied (setup's own peel, `BoardFrames::count_freedoms`).  `Board::prepare_extended`
hands Π⁺(∅) to `search::prepare` as it is; the one-sided candidates are not in the search (stage 2).  THEORY.md has
Theorem M⁺ (S ⊇ F is playable iff (K, L) is a promotion set, and weighs in_the_bag⁺ + ΣK + ΣL), its proof (Theorem M's,
without M5) and the certificate of U it gives, with the evidence row.

`tests/model_extended.rs` checks Theorem M⁺ against the whole-board oracle of `tests/common/playable.rs`, both ways, on
every set: every board 2..80 for O = ∅ and for three random O of 1-8 members each (2.2 million sets), every board 2..60
for O = U\*_{≥3} (4.2 million), and, ignored, 81..100 (3.4 million) and 61..69 with every source open (20.9 million;
the boards 70 and 71 have 62 million each, the count doubling every two boards); the heaviest set is optimal.json's on
every board.  Random sets, 100 of each kind (promotion sets built in a random order, maximal playable sets built in a
random order) for O a random 20 members of U\*_{≥3}, for O = U\*_{≥3} and for O = ∅ (where the frame model's random
promotion sets are Π⁺(∅)'s and the reverse) at 300, 600, 1000 and 1300, and at 741, 1100 and 1200, the boards where
greedy rejects a source of layer 3 (374, 578, 605; none on the boards to 740, nor at 1000 or 1300), so that sets with a
rejected source are met (92-97 of 200 with every source open); ignored, 2000, 3000 and 4095 (R of 5 and 10 members at
3000 and 4095).  No mismatch anywhere: 29 s for the default tests, 132 s for the ignored ones.  With O = ∅, Π⁺(∅)'s
frames are the frame model's with the claimed pairs put back, and its candidates, freedoms and `in_the_bag` the same,
on every board to 400; its promotion sets are those of Π_n and of the problem `prepare_extended` poses on every board
to 100 (unit tests of `frames::extended` and `board`).

`examples/extended.rs chain 2 1300 --compare` searches every game (primes included, no repeats) on one thread, seeded
from the game before and capped at the ceiling, on Π⁺(∅) and on the default problem.  Every one of the 1,299 games
scores optimal.json's, through a legal game of in_the_bag + its promotions.  What dropping the claims costs:

| band | nodes, Π⁺(∅) | nodes, default | ratio | seconds, Π⁺(∅) | seconds, default |
|---|---|---|---|---|---|
| 2..900 | 36,387 | 34,812 | 1.045 | 5.7 | 5.0 |
| 901..1000 | 118,770 | 117,783 | 1.008 | 9.5 | 8.9 |
| 1001..1100 | 123,871 | 116,945 | 1.059 | 10.6 | 9.5 |
| 1101..1200 | 16,899 | 16,529 | 1.022 | 5.6 | 4.6 |
| 1201..1300 | 178,846 | 177,286 | 1.009 | 19.2 | 17.6 |
| all | 474,773 | 463,355 | 1.025 | 50.7 | 45.6 |

1,064 games take the same nodes; 31 take more than 10% more (the most 1.24, at 616; 19 of them in 601-700, the rest at
432-433, 576-595, 832-865 and 1023-1029), 5 more than 10% fewer (940-943, 984-985; at least 0.88).  The probe's games:
936 376 nodes against 394, 1215 10,012 against 9,979; the bench set 1050, 1100, 1133 and 1170 the same nodes, 1184 174
against 170.  A node costs about 8% more (the claimed pairs stay in the frames the peels see).  So the claims are worth
a few percent to the search, not more: the extended problem's search, with nothing open, costs what the default's does.

The default mode is unchanged by all of it (the same gates as 2A: the fingerprints, the suite with the tripwires,
`regress --to 1000 --threads 1` in 9.1 s, and the chain 1..1300 on one thread line for line).

## Later (2026-10): the program removed from master

The example `sacrifice` was removed from master on 2026-10-17.  It builds at commit 8753189: `git checkout 8753189`,
then `cargo build --release --example sacrifice` in `rust/`.
