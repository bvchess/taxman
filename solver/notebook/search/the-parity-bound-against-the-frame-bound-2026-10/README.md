# The parity bound against the frame bound (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-24.  Concluded 2026-10-24.

**Question.**  Is the prover's parity bound at the root always ΣU\* plus the frame solver's frame bound at the root, as
it was at the 361 roots of 880-1300 that [matroid intersection bounds](../matroid-intersection-bounds-2026-10/README.md)
measured, and why?  **Answer.**  It is, at every board from 2 to 8191.  One side is proved for every board: on the
candidates, the frame matroids are the parity matroids contracted by U\*, so ΣU\* plus the frame bound is never above
the parity bound ([the contraction theorem (B6)](../../../../docs/theory/frame-bounds.md#the-comparison-theorem)).  The
other side is proved, given the span conjecture (♦), wherever every number up to n/2 that the contraction leaves free
is a candidate; that fails on 5,496 boards from 1173 on, where the equality holds with room to spare: forcing such a
number y into the parity bound costs it within a few hundred of n/2 − y, and at least 187 on the boards sampled.

## Method

**The two bounds.**  For each board n, two programs compute the two sides, and share no code, as the two crates do not.

* `paritysplit.rs`, an example of the prover: the parity bound of n with no picks and no bars, the heaviest set
  independent in both parity matroids, as the prover's search computes it at its root (`Meet::solve` on
  `Parity::pair`), with the weight split that certifies the value.  It computes ΣU\* by the prover's own descending
  greedy.
* `framebound.rs`, an example of the frame solver: ΣU\* (`in_the_bag`, the score of the game with no promotions) and
  the GF(2) bound of the root with every candidate and at most `max_total` promotions, which is the bound the search
  computes at its first node (`Gf2Bound::evaluate` on `Gf2Graph::new`).

`compare.py` joins the two by n.  It checks that the two values of ΣU\* agree and reports every board where the parity
bound differs from ΣU\* plus the frame bound.

**What else they record,** for the proof.  The prover's line says whether the heaviest set's sources are U\* ([the
greedy-basis proposition (W3)](../../../../docs/theory/parity-bound.md#the-greedy-basis-proposition)), the parity bound
with U\* picked, whether (♦) holds (`span::span_check`), and the *GF(2) candidates*: the numbers up to n/2 that are
loops of neither M₀/U\* nor M₁/U\*.  The frame solver's line has the candidates, and the frame bound with no limit on
the number of promotions.  Each side gives its set as a count, a sum and a 64-bit digest, and, in the runs with lists,
as a list.  The candidates are GF(2) candidates (B6), so equal counts mean equal sets, and the digests check it.

**The margins.**  Where the GF(2) candidates outnumber the candidates, `paritymargin.rs` takes extra numbers y, on two
samples of the boards, and computes the parity bound with the picks U\* and y.  The *loss* is the parity bound less
that value: a loss above 0 says that no heaviest set holds y.  `which_test.py` recomputes the extra numbers on a third
sample from the theory's definitions, and says which test each fails.

**Cost and where it ran.**  The estimate before the run, from single boards on the laptop: the parity bound takes 0.7 s
at 4000, 2.0 s at 6000 and 3.9 s at 8000, growing as about n^2.9, so 2..8191 would take about 2.5 hours on one thread,
and the frame bound about a tenth of that.  So the prover's side ran on the lab machine, 8 threads at nice 19 beside the
chain, from 2 up, and on the laptop, one thread at nice 19, from 8191 down, until the two met at 7178 and 7179; the
laptop's share paused while its load was above 8.  The frame side ran on the laptop alone.  The lab machine's threads,
behind the chain's at nice 10, ran about five times slower than the laptop's.

## Results

### The two bounds are equal at every board

| n | boards | equal | nothing to promote | limit lowers the frame bound | sources are U\* | (♦) holds | boards with extra GF(2) candidates | extra numbers |
|---|---|---|---|---|---|---|---|---|
| 2-1000 | 999 | 999 | 19 | 0 | 999 | 999 | 0 | 0 |
| 1001-2000 | 1000 | 1000 | 0 | 0 | 1000 | 1000 | 54 | 142 |
| 2001-3000 | 1000 | 1000 | 0 | 0 | 1000 | 1000 | 678 | 2235 |
| 3001-4000 | 1000 | 1000 | 0 | 0 | 1000 | 1000 | 643 | 5919 |
| 4001-5000 | 1000 | 1000 | 0 | 0 | 1000 | 1000 | 930 | 9936 |
| 5001-6000 | 1000 | 1000 | 0 | 0 | 1000 | 1000 | 1000 | 21307 |
| 6001-7000 | 1000 | 1000 | 0 | 0 | 1000 | 1000 | 1000 | 36188 |
| 7001-8000 | 1000 | 1000 | 0 | 0 | 1000 | 1000 | 1000 | 55526 |
| 8001-8191 | 191 | 191 | 0 | 0 | 191 | 191 | 191 | 11772 |
| all | 8190 | 8190 | 19 | 0 | 8190 | 8190 | 5496 | 143025 |

The parity bound equals ΣU\* plus the frame bound at all 8,190 boards, and the two programs' values of ΣU\* agree at
every one.  The 19 boards with nothing to promote (2 to 17, 25, 26 and 27) have the frame bound 0, and there the parity
bound is ΣU\*.  The 23 boards that both machines ran gave the same values.

### What the proof accounts for

* *The sources.*  At every board the heaviest set the prover finds holds U\* and no other source, the parity bound with
  U\* picked is the parity bound, and (♦) holds: W3 at work.
* *The promotion limit.*  It never lowers the frame bound: the bound with no limit is the same at every board, as B6,
  part 2, proves.
* *The candidates.*  Up to 1172 the GF(2) candidates are exactly the candidates, so B6, part 4, proves the equality
  there, given (♦), which holds.  From 1173 on they outnumber the candidates on 5,496 of the 7,019 boards, and on every
  board from 4394 on, by 3 at 1173 (242, 286 and 374) and by up to 92 (7797 to 7799); there B6 proves only one side.

### Numbers free in the contraction that are no candidates

`which_test.py` recomputes the candidates and the GF(2) candidates from the theory's definitions, with the heuristics'
peel, on every fiftieth board from 1150 and on 8191.  It finds the same extra numbers as `extras.jsonl.gz` at all 142
boards, 2,910 of them, and every one fails test (b) and passes test (a): the frame in front cannot take it on, through
an odd core, though over GF(2) its vector is free of that frame's moves.

The heaviest set the prover found holds none of the 143,025 extra numbers.  Forcing one in costs the bound something
at every number tried, so no heaviest set holds any of those:

| sample | boards | numbers | least loss | loss − (n/2 − y) |
|---|---|---|---|---|
| every fiftieth of the 5,496 boards, every extra number | 110 | 2,821 | 187 (n = 1173, y = 374) | −605 to 297 |
| in each hundred of n, the board whose largest extra y has the largest y/(n/2), that y | 61 | 61 | 187 (n = 1173, y = 374) | −253 to 0 |

The loss stays within a few hundred of n/2 − y, as if forcing y in pushed out a number near n/2, so the numbers that
come nearest n/2 come nearest a tie; the least loss in either sample is 187, at n = 1173, y = 374, and it grows with n
(at least 1,058 on the boards of the first sample past 8000).  So the equality does not hang on a near tie on these
boards, though nothing proves that it holds beyond 8191.

### Cost

The frame side took 672 s on the laptop for 2..8191.  The prover's side took 39,067 thread-seconds on the lab machine
for 2..7178 and 4,269 s on the laptop for 7179..8191, from 14:15 to 15:35 Pacific.  The margins took 2,663
thread-seconds on the lab machine (the first sample) and 11 s on the laptop (the second), and `which_test.py` about 15
minutes on the laptop.

## Decision

* **The theory.**  The paragraph "Against the parity bound" of
  [frame-bounds.md](../../../../docs/theory/frame-bounds.md#the-frame-bound) states the measured range, B6 at the end of
  that file proves one side and the other given (♦) where the candidates are the GF(2) candidates, and W3 in
  [parity-bound.md](../../../../docs/theory/parity-bound.md#the-greedy-basis-proposition) settles the sources.  The rows
  of the table of statements and of evidence.md go to whoever edits those files.
* **For the solvers.**  Nothing changes.  At the root the prover's bound on the whole board and the frame solver's
  bound tell the same, so neither can tighten the other there; fixing the sources to U\* costs the parity bound nothing
  wherever (♦) holds.
* **Open.**  The other side on the boards with extra GF(2) candidates: a proof that no heaviest set holds one, perhaps
  through why forcing one in costs about n/2 − y.
* **Reopens** at a board where the bounds differ, which would need an extra GF(2) candidate in a heaviest set, or where
  (♦) fails.

## Files

| file | what it is, and the command that made it |
|---|---|
| `framebound.rs` | the frame solver's side, an example kept out of the build: copy it to `solver/frame/examples/`, then `cargo run --release --example framebound -- 2 8191 frame.jsonl lists` |
| `paritysplit.rs` | the prover's side, likewise for `solver/noframe/examples/`: `cargo run --release --example paritysplit -- --from 2 --to 8191 --threads 8 --lists yes --out parity.jsonl`; the run split the boards between the two machines with `--list`, as Method says |
| `paritymargin.rs` | the losses, likewise for `solver/noframe/examples/`: `cargo run --release --example paritymargin -- --list N,... --numbers margins_in.txt --threads 8 --out margins.jsonl`, the list being the boards of `margins_in.txt`, and the same with `margins_near_in.txt` and `margins_near.jsonl` on one thread |
| `compare.py` | the join and the tables of Results: `./compare.py > compare_output.md`; `./compare.py --lists frame.jsonl parity.jsonl` with the runs' lists writes `extras.jsonl.gz` and the two inputs of `paritymargin.rs` |
| `which_test.py` | the third computation of the extra numbers, with the test each fails: `./which_test.py $(seq 1150 50 8150) 8191 > which_test.jsonl` |
| `frame_2_8191.jsonl.gz` | the frame solver's lines, without the lists |
| `parity_2_8191.jsonl.gz` | the prover's lines, without the lists, the two machines' shares merged by n |
| `extras.jsonl.gz` | the 5,496 boards whose GF(2) candidates outnumber their candidates: the extra numbers, and those of them in the heaviest set the prover found (none) |
| `margins_in.txt`, `margins.jsonl` | the first sample of the losses: every fiftieth of those boards, with every extra number |
| `margins_near_in.txt`, `margins_near.jsonl` | the second: in each hundred of n, the board whose largest extra number y has the largest y/(n/2), with that number |
| `which_test.jsonl` | the output of `which_test.py` |
| `compare_output.md` | the output of `./compare.py` |

## Commits

* The commit that adds this entry (2026-10-24): the entry, its row in the register, [the contraction theorem
  (B6)](../../../../docs/theory/frame-bounds.md#the-comparison-theorem) and [the greedy-basis proposition
  (W3)](../../../../docs/theory/parity-bound.md#the-greedy-basis-proposition).
