# Boards past 4095 (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-14.  Concluded 2026-10-15.

**Question.**  The chain on the cloud machine reached 3314 overnight and would stop at 4095, the end of the set widths,
for want of code.  What limits the boards to 4095, how far does one more width take them, and what does a board past
4095 cost?  **Answer.**  Five limits held the solver at 4095, and one more width, 128 words, lifts them all to 8191 (at
most 9 frames and ranks to 314) with no change to the boards up to 4095; a node at 4100 costs 1.1% more at 128 words
than at 72.

This is item 8 of the design of the [wall regime](../search/the-wall-regime-2026-10/README.md).

## Method

**The limits.**  Everything that held the solver at 4095, and what it is after the change:

| limit | was | after | why that is enough |
|---|---|---|---|
| `bits::WIDTHS` and `with_words!` | the widest 64 words (n ≤ 4095) | 128 words (n ≤ 8191) | `solver::MAX_N` is derived from the widest width, and the results reader (so `verify-results` and the start of every `solve` on a results file) checks a line's n against it |
| `model::MAX_FRAMES` | 8 | 9 | every board to 8191 has at most 9 frames (the scan below) |
| the peel's `Worklist` | one summary word: 64 words of set | two: 128 words, the second read only at `W > 64` (a constant) | |
| the GF(2) bound's block words (`Levels<W, R>`, `gf2::MAX_WORDS`) | 3: ranks to 192 | 5 at 128 words (ranks to 320); 3 at the narrower widths | the largest rank of every board: 169 to 4095, 314 to 8191 |
| the tableau's slots (`gf2::meet::Meet`) | `u8`, 255 the mark of no slot | `u16` | a block holds up to its rank in members, numbered by slot: from n = 6390, where ranks reach 256, a slot could have been 255, the mark, or wrapped to 0 |

The GF(2) engines of four and five words are copies of the bound's code for every width that has them.  The release
build is one codegen unit with fat LTO, and the arms of four and five words test the constant `Gf2Bound::WIDE`
(`W > 64`), so the narrower widths compile as before.  An engine of more words than the graph needs runs the same bound
on vectors padded with zero words (`Gf2Bound::with_words`, `Gf2Graph::padded`), which the tests use.  `examples/wall.rs`
takes `--width 128`, its `frames` covers every board the solver does, and `solver/tools/pgo.sh` trains a game of 128
words (4100).

**Not limits.**  The checkpoint header records the width, and n alone picks it, so a checkpoint is resumed at the width
it was made at: the boards to 4095 keep their widths, frames, candidates and tactics, and checkpoints of 1133 (20 words)
and 3600 (64) written by the build before resumed under the build after it.  Also unbound:

* the engine's units, which are `W` words a set (2 KiB a unit at 128), and the search problem's digest, which is
  width-free;
* the factor lists and the peel's cells, which hold 16-bit numbers (to 65535), the matching graph's 16-bit vertices, and
  the frame masks of 16 bits (9 frames);
* the frames of a number to 30029, which has at most five distinct primes, where `BoardFrames::any_frame_of` assumes
  eight slots;
* the Python referee, which has no bound on n, and `chain.sh`, which takes any N (`CHAIN_TO`); the solver refuses a
  board past 8191.

**The target: 8191.**  A width of 128 words holds the boards to 8191 exactly, and nothing else costs more on them: nine
frames and five block words hold every one, which a scan of every board shows.  Past 8191 would take 256 words (or a
width per step), very likely a tenth frame from 8192 (the count steps up at the powers of two: 6 from 512, 7 from 1024,
8 from 2048, 9 from 4096) and six block words.

**The tests** (`cargo test --release`: 376 s with compiling, on two test threads) search 300, 400, 760, 1000, 1050 and
1133 at every width to 128 with the same trees, build a board of 4100 at 128 through a short search, and check the
boards 4096, 4100, 4740, 6000, 6392 and 8191 at 128 against the board layer's invariants.  They run the GF(2) engines of
every word count along random paths of four boards with the same results and the same work, and blocks of 260 to 320
dimensions at five words against the greedy algorithm, with more than 255 members, which fails with the old `u8` slots.
They also read a results line of 4096 and 8191 and refuse one of 0 and 8192.

## Results

**The scan** (`wall frames 2-8191` by the build before the change, in four processes of one thread, 124 s;
[frames_all.jsonl](frames_all.jsonl)).  The frames: 8 on every board 2048..4095, 9 on every board 4096..8191.  The
largest frame rank, which is the GF(2) bound's block dimension, by bands of 512:

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
4096..8191, 665 boards need three words, 1,631 four and 1,800 five.  The members, the numbers in the frames and the
candidates, need 79 words at 8191.

**Nothing changes below 4096.**

* The probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a (`gf2=on`) are unchanged; its cycles are 4.727 G
  against 4.723 G (`gf2=on` 5.781 against 5.776) and its instructions 15.934 against 15.952 (21.644 against 21.653).
* The tripwires are unchanged, and `taxman regress --to 1000 --threads 1` gives every score (9 s).
* The chain 1..1300 on one thread (a scratch file) is identical line for line, apart from the times and the build stamp,
  to the same chain by the build before (290,503 nodes; 26.2 s of search and 37 s of wall time against 26.3 and 38).
  Its scores, methods and proofs are those of `results/strict.jsonl`; the moves are too but at one board, another
  optimal set, as before.
* At 4000 (64 words, one thread, 20,000 steps from the unseeded start, net of the setup;
  [past4095_nodes.jsonl](past4095_nodes.jsonl)) a node takes 4,245 thousand instructions before and after, and 963 and
  954 thousand cycles before, 963 and 947 after.  The fixed costs are the same (the board 0.046 s, the local search 6.16
  and 6.23 s).
* The release build of the binary alone from nothing (four jobs, `nice`) takes 82 s before and 106 s after.

**What a board past 4095 costs** (one thread; [past4095_setup.jsonl](past4095_setup.jsonl), `past4095_nodes.jsonl`).
The width, at 4100 (65 words needed):

| words | thousand cycles a node (two runs) | thousand instructions a node | board | the moves | local search |
|---|---|---|---|---|---|
| 72 | 1,204, 1,182 | 5,262 | 0.048 s | 0.018 s | 15.3 s |
| 80 | (not measured) | | 0.048 | 0.018 | 15.0 |
| 96 | (not measured) | | 0.050 | 0.018 | 16.1 |
| 128 | 1,209, 1,203 | 5,274 | 0.050 | 0.018 | 17.5 |

A node costs 1.1% more at 128 words than at 72 (the wall regime's fit said 1.3%), the board layer the same, and the
local search 14% more on one thread (on the chain's 64 threads, where it runs since round A, a share of a second a
searched game).  A width between would save that 1% on the boards it holds (72: 4096..4607), for 12 s more of every
build and of every test binary's.

4100 against 4000, each at its width: a node costs 1,206 against 955 thousand cycles (+26%) and 5,274 against 4,245
thousand instructions, with 8.6 GF(2) augmentations a step against 2.6 (4100's start is 1,498 below its root bound,
4000's 1,316).  That is the game, not the width.  The board takes 0.050 s against 0.046, the moves 0.018 against 0.016,
the prime path 0.031 against 0.029, the root's GF(2) bound 2.8 ms at both; the local search 17.5 s against 6.2 (its
passes depend on the game).  The one-thread process peaks at 55 MB at 4100 and 128 words (47 at 72 words; 4000 at 64,
42), and the 2.3 GB of 4000 on 192 threads (the scaling run) leaves room for twice that on every instance considered.

**Past 4095, solved.**  No results file reaching 4095 exists on the laptop (the cloud chain's is on its machine), so the
games 4096..4110 ran standalone (`taxman solve N --threads 4 --time-limit 10`, one invocation each, into one file): no
seed and no ceiling, so each search stopped at its limit with the best game it had found.  They took 98,000 to 111,000
nodes, 213 to 215 promotions, 5,280,922 at 4096 to 5,315,188 at 4110, and 14 to 15 s a game with the setup
([past4095_standalone.jsonl](past4095_standalone.jsonl); measurements, not results).  Both referees accept the file
(`verify-results`, `solver/tools/check_results.py`).  An incomplete line has no proof to check, so its moves were also
replayed by the Python referee's rules: every game legal, finished, opening with the largest prime, and scoring its
`best_so_far`, which is its `in_the_bag` plus its promotions' sum.

## Decision

Kept: boards 2..8191 at a set width of 128 words, the boards to 4095 unchanged.  Widths of 72, 80 and 96 words, which
would save 1% of a node on 4096..4607, were not kept, since each costs 12 s more of a release build (118 s against 106)
and of every test binary.  A board past 8191 would reopen the question: 256 words, a tenth frame and six block words.

**Taking a chain past 4095 on an older build.**  A build from before commit 4316168 stops at n = 4095, the widest set it
has.  To take a chain past 4095, stop it (or let `chain.sh 4095` finish), build commit 4316168 or a later one in the
same checkout (`git pull`, then `setup.sh`, or `cargo build --release` in `solver/frame`), and run `chain.sh N` again
with N up to 8191 (or `CHAIN_TO=N`).  It skips the completed games, and the checkpoint of a game below 4096 that the old
build wrote resumes (tried with checkpoints of 1133 and 3600).  The solver refuses a game past 8191 ("beyond the largest
supported board"), which ends the script with an error.  [The runbook's page on the chain](../../docs/runbook/chain.md)
gives the current steps.

## Files

From `wall`, as in [the wall regime's
files](../search/the-wall-regime-2026-10/README.md#files-and-the-commands-that-made-them):

| file | command | what it is |
|---|---|---|
| `frames_all.jsonl` | `wall frames 2-8191` (the build of ee03872, in four processes), each line cut to `n`, `frames`, `max_freedoms`, `block_words`, `candidates`, `members` | every board to 8191: what bounds `MAX_FRAMES` (9) and the GF(2) bound's block words (5) |
| `past4095_setup.jsonl` | `wall setup 4000` by the builds before (`"build":"base"`, ee03872) and after (`"new"`) the boards past 4095, and `wall setup --width W 4100` for W = 72, 80, 96, 128 (a build whose `--width` list had 72, 80, 96 and 112 as well; add them to `at_width!` to run it again) | the fixed costs past 4095, by set width |
| `past4095_standalone.jsonl` | `taxman solve N --threads 4 --time-limit 10 --out FILE` for N = 4096..4110, one invocation each (the build of 4316168, before its commit) | the first games past 4095, standalone (no seed, no ceiling) and stopped at the limit: incomplete lines, measurements and not results |
| `past4095_nodes.jsonl` | `wall nodes --no-seed --nodes K 4000` (both builds) and `--width W 4100` (W = 72, 128; the same build), K = 1 and 20,000, twice (`rep`) | the cost of a node net of the setup (the difference of the two K), before and after, and by width |

## Commits

* ee03872: the build before the change, which made `frames_all.jsonl` and the `base` lines of `past4095_setup.jsonl`.
* 4316168: boards past 4095, to 8191: the set width of 128 words, `MAX_FRAMES` 9, the peel's worklist with two summary
  words, the GF(2) bound's engines of four and five words, and the `u16` slots.
