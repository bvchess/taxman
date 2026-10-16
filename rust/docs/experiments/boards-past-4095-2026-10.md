# Boards past 4095 (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The chain on the cloud machine reached 3314 overnight and would stop at 4095, the end of the set widths,
for want of code.  What limits the boards to 4095, how far does one more width take them, and what does a board past
4095 cost?  Item 8 of the [wall regime](the-wall-regime-2026-10.md)'s design, built in commit 4316168, with no change
at all to the boards up to 4095.

*The limits.*  Everything that held the solver at 4095, and what it is now:

| limit | was | now | why that is enough |
|---|---|---|---|
| `bits::WIDTHS` and `with_words!` | the widest 64 words (n ≤ 4095) | 128 words (n ≤ 8191) | `solver::MAX_N` is now derived from the widest width, and the results reader (so `verify-results` and the start of every `solve` on a results file) checks a line's n against it |
| `model::MAX_FRAMES` | 8 | 9 | every board to 8191 has at most 9 frames (the scan below) |
| the peel's `Worklist` | one summary word: 64 words of set | two: 128 words, the second read only at `W > 64` (a constant) | |
| the GF(2) bound's block words (`Levels<W, R>`, `gf2::MAX_WORDS`) | 3: ranks to 192 | 5 at 128 words (ranks to 320); 3 at the narrower widths | the largest rank of every board: 169 to 4095, 314 to 8191 |
| the tableau's slots (`gf2::meet::Meet`) | `u8`, 255 the mark of no slot | `u16` | a block holds up to its rank in members, numbered by slot: from n = 6390, where ranks reach 256, a slot could have been 255, the mark, or wrapped to 0 |

Not limits: the checkpoint header records the width, and n alone picks it, so a checkpoint is resumed at the width it
was made at; the boards to 4095 keep their widths, frames, candidates and tactics, so the checkpoints of a chain below
4096 resume with the new build (checkpoints of 1133, at 20 words, and 3600, at 64, written by the build before, were
resumed by the new one).  The engine's units are `W` words a set (2 KiB a unit at 128); the search problem's digest
is width-free; the factor lists and the peel's cells hold 16-bit numbers (to 65535), the matching graph 16-bit
vertices, the frame masks 16 bits (9 frames); a number to 30029 has at most five distinct primes, which the eight
slots of `BoardFrames::any_frame_of` assume; the Python referee has no bound on n; `chain.sh` takes any N
(`CHAIN_TO`), and the solver refuses a board past 8191.

*The target: 8191.*  A width of 128 words holds the boards to 8191 exactly, and nothing else costs more on them: nine
frames and five block words hold every one, which a scan of every board shows.  Going past 8191 would take 256 words
(or a width per step), very likely a tenth frame from 8192 (the count steps up at the powers of two: 6 from 512, 7
from 1024, 8 from 2048, 9 from 4096) and six block words.

*The scan* (`wall frames 2-8191` by the build before the change, in four processes of one thread, 124 s;
[bench/wall/frames_all.jsonl](../../bench/wall/frames_all.jsonl)).  The frames: 8 on every board 2048..4095, 9 on every
board 4096..8191.  The largest frame rank, which is the GF(2) bound's block dimension, by bands of 512:

| boards | largest rank (at n) | block words | most members | most candidates |
|---|---|---|---|---|
| 2..4095 | 169 (4095) | 3 | 2,461 | 1,107 |
| 4096..4607 | 189 (4598) | 3 | 2,767 | 1,255 |
| 4608..5119 | 203 (5084) | 4 | 3,080 | 1,396 |
| 5120..5631 | 221 (5631) | 4 | 3,382 | 1,544 |
| 5632..6143 | 250 (6143) | 4 | 3,736 | 1,689 |
| 6144..6655 | 267 (6654) | 5 | 4,010 | 1,795 |
| 6656..7167 | 284 (7167) | 5 | 4,285 | 1,909 |
| 7168..7679 | 298 (7640) | 5 | 4,660 | 2,117 |
| 7680..8191 | 314 (8174) | 5 | 5,012 | 2,281 |

Four words first at 4740 (the last board that needs three is 4801), five at 6392 and on every board from it; of
4096..8191, 665 boards need three words, 1,631 four and 1,800 five.  (The members, the numbers in the frames and the
candidates, need 79 words at 8191.)

*What changed* (4316168):

1. `WIDTHS` ends in 128, and `with_words!` has its arm; `MAX_N` = 64 × the widest − 1 = 8191.
2. `MAX_FRAMES` 9: every frame set and per-frame array one frame larger, at every width (a node at 4000 runs the same
   instructions as before: below).
3. The `Worklist` has two summary words; at `W ≤ 64` its pop reads the first only, which the compiler knows.
4. The GF(2) bound has engines of four and five words, and the tableau's slots are `u16`.  Each engine is a copy of the
   bound's code for every width that has it, and the release build is one codegen unit with fat LTO, so the arms of
   four and five words test the constant `Gf2Bound::WIDE` (`W > 64`): the compiler does not generate the code of a
   branch on a constant that is false, and the narrower widths are built as before.  A build of the library with v0
   symbols and no LTO has 127 to 129 symbols of `Levels<W, R>` for R ≤ 3 at every width and for R = 4 and 5 at 128,
   and one (its drop) for R = 4 and 5 at the others.  An engine of more words than the graph needs runs the same bound
   on vectors padded with zero words (`Gf2Bound::with_words`, `Gf2Graph::padded`), which the tests use.
5. `examples/wall.rs` takes `--width 128`, and `frames` covers every board the solver does; `tools/pgo.sh` trains a game
   of 128 words (4100).

*Tests* (`cargo test --release`: 376 s with compiling, on two test threads):
the widths against 128 (300, 400, 760, 1000, 1050 and 1133 searched at every width to 128 with the same trees; 4095's
start at 64 and 128; 2050's node-limited search at its width, at 64 and at 128); a board of 4100 built at 128 through
frames, matching bound, conflict, greedy set and a short search; the boards 4096, 4100, 4740, 6000, 6392 and 8191 at 128
against the board layer's invariants (nine frames, numbers on the board, the plain game legal and finished and scoring
`in_the_bag`, the freedoms, the greedy set and random subsets of it promotable and played into games that score
`in_the_bag` plus their sum and open with the largest prime, the root's GF(2) bound of three, four and five words
between the greedy sum and the matching bound and equal to the five-word engine's); game 1000's tree at 16 words
and at 128 with the GF(2) graph of one, four and five words, the same; along random paths of four boards at 128, the
engines of every word count (natural, `with_words` and padded) with the same results and the same work; blocks of
260 to 320 dimensions at five words against the greedy algorithm (one matroid free), with more than 255 members, which
fails with the old `u8` slots; the worklist across both summary words; `solve` of 4100 through setup, the game
replayed; a results line of 4096 and 8191 read, of 0 and 8192 refused.

*Nothing changes below 4096.*  The probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a (`gf2=on`) unchanged,
its cycles 4.727 G against 4.723 G (`gf2=on` 5.781 against 5.776) and instructions 15.934 against 15.952 (21.644
against 21.653); the tripwires unchanged; `taxman regress --to 1000 --threads 1` every score (9 s); the chain 1..1300 on
one thread (a scratch file) identical line for line, apart from the times and the build stamp, to the same chain by
the build before (290,503 nodes; 26.2 s of search and 37 s of wall time against 26.3 and 38), its scores, methods and
proofs those of `results/strict.jsonl` (the moves too but at one board, another optimal set, as before).  At 4000 (64
words, one thread, 20,000 steps from the unseeded start, net of the setup;
[bench/wall/past4095_nodes.jsonl](../../bench/wall/past4095_nodes.jsonl)): 4,245 thousand instructions a node before and
after, 963 and 954 thousand cycles before, 963 and 947 after; the fixed costs the same (the board 0.046 s, the local
search 6.16 and 6.23 s).  The release build of the binary alone from nothing (four jobs, `nice`): 82 s before, 106 s after.

*What a board past 4095 costs* (one thread; [bench/wall/past4095_setup.jsonl](../../bench/wall/past4095_setup.jsonl),
`past4095_nodes.jsonl`).  The width, at 4100 (65 words needed):

| words | thousand cycles a node (two runs) | thousand instructions a node | board | the moves | local search |
|---|---|---|---|---|---|
| 72 | 1,204, 1,182 | 5,262 | 0.048 s | 0.018 s | 15.3 s |
| 80 | (not measured) | | 0.048 | 0.018 | 15.0 |
| 96 | (not measured) | | 0.050 | 0.018 | 16.1 |
| 128 | 1,209, 1,203 | 5,274 | 0.050 | 0.018 | 17.5 |

A node costs 1.1% more at 128 words than at 72 (the wall regime's fit said 1.3%), the board layer the same, the local
search 14% more on one thread (on the chain's 64 threads, where it runs since round A, a share of a second a searched
game).  A width between would save that 1% on the boards it holds (72: 4096..4607), for 12 s more of every
build and of every test binary's; not kept.  4100 against 4000, each at its width: a node 1,206 against 955 thousand
cycles (+26%) and 5,274 against 4,245 thousand instructions, 8.6 GF(2) augmentations a step against 2.6 (4100's
start is 1,498 below its root bound, 4000's 1,316): the game, not the width; the board 0.050 s against 0.046, the
search problem set up again 0.050 against 0.046, the moves 0.018 against 0.016, the prime path 0.031 against 0.029, the
root's GF(2) bound 2.8 ms at both; the local search 17.5 s against 6.2 (its passes depend on the game).  Memory: the
one-thread process peaks at 55 MB at 4100 and 128 words (47 at 72 words; 4000 at 64, 42): a unit and a searcher's sets
are twice their size at 64 words, which the 2.3 GB of 4000 on 192 threads (the scaling run) leaves room for on every
instance considered.

*Past 4095, solved.*  No results file reaching 4095 exists on the laptop (the cloud chain's is on its machine), so the
games 4096..4110 ran standalone (`taxman solve N --threads 4 --time-limit 10`, one invocation each, into one file): no
seed, no ceiling, so each search stopped at its limit with the best game it had found: 98,000 to 111,000 nodes, 213 to
215 promotions, 5,280,922 at 4096 to 5,315,188 at 4110, 14 to 15 s a game with the setup
([bench/wall/past4095_standalone.jsonl](../../bench/wall/past4095_standalone.jsonl); measurements, not results).  Both
referees accept the file (`verify-results`, `tools/check_results.py`); an incomplete line has no proof to check, so
its moves were also replayed by the Python referee's rules: every game legal, finished, opening with the largest
prime, and scoring its `best_so_far`, which is its `in_the_bag` plus its promotions' sum.  The chain behind a file
that reaches 4095 is the cloud machine's to run: the RUNBOOK says how (section 5, "Past 4095").
