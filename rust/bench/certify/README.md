# The certificates of U: the work list of 2176-3440, and the data it is made from

What a certificate is and what it proves: DESIGN.md, ["Certificates of U"](../../DESIGN.md#certificates-of-u), and
heuristics/THEORY.md, the corollary of Theorem M⁺.  What the files below showed: docs/EXPERIMENTS.md, ["Certificates
past 2175"](../../docs/EXPERIMENTS.md#certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10).
Everything ran on the M1 Max, one thread, `nice -n 10`, on the code of commit 35c614b, from `rust/`, against the
committed chain (`results/strict.jsonl`, games 1..3440) and the prover's barred bounds (`prover/data/upper_*.jsonl`).
`U` stands for the four `--upper` files (2..1300, 1301..2175, 2176..3314, 3315..3440).

| file | command | what it is |
|---|---|---|
| `problems_600_3440.jsonl` | `extended problems 600 3440 U --known results/strict.jsonl --out problems_600_3440.jsonl` (`cargo build --release --example extended`; 79 s) | one line per game of 600..3440 that needs a certificate (1,575): its extended problem Π⁺(O) set up without a search (frames, two-sided and one-sided candidates, rejected sources, in_the_bag⁺, the recorded game's worth, the GF(2) block words, the set width `certify` takes, the digest) and `same_as`, the latest game of the 8 board sizes before it with the same problem (340, every one of equal worth: the games `certify` settles with no search) |
| `certificates_600_1300.jsonl` | `taxman certify 600-1300 --results results/strict.jsonl U --out certificates_600_1300.jsonl --threads 1 --progress 0` (39 s) | a certificate for each of the 377 games of 600..1300 that need one: 296 certified by a search, 81 by the same problem as an earlier game; `taxman verify-certificates` passes it (4.6 s) |
| `plan.jsonl`, `plan.md`, `lists/*.txt` | `tools/certify_plan.py --results results/strict.jsonl --upper prover/data/upper_2176_3314.jsonl prover/data/upper_3315_3440.jsonl --problems bench/certify/problems_600_3440.jsonl --gaps bench/wall/gaps_chain.jsonl bench/wall/gaps_chain_3313_3440.jsonl --from 2176 --to 3440 --era 2176-3310:6.396 --era 3311-3440:16.57 --out-dir bench/certify` | the work list of the campaign and its estimate: per game of 2176..3440 whether it needs a certificate, why it costs what it is expected to cost, and the list files `taxman certify --list` takes (`plan.md` is the summary the owner's approval reads) |

**The two eras.**  `certify_plan.py` prices a search by its nodes, at the thread-seconds per node that the chain's own
run paid in each stretch of it, since the chain ran several searches at once and a game's recorded seconds are the
time it shared the machine.  The wall times are the run's (docs/EXPERIMENTS.md, [the cloud run to
3440](../../docs/EXPERIMENTS.md#the-frontier-chain-to-3440-2026-10)): games
2176..3310 in 23,025 s on the first instance (window 16), and 3311..3440 on the second (window 4) in its 62,701 s less
the 3,054 s that the unfinished 3441's 625.8 million nodes took at its measured 205 thousand nodes a second, 59,647 s.
The recorded seconds over-count the first era 3.27 times and the second 1.55 times; `plan.md` gives the total from
the recorded seconds as well, as an upper end.

**The lists.**  `pilot.txt` holds the five dearest games, `dear.txt` the next fifteen, and `rest_1.txt` to `rest_4.txt`
the others, balanced by expected time and each in increasing n, so that the proven part of the chain grows as a run
goes.  A game that the rule of the same problem settles is in the list of the game it rests on, after it: the rule
looks only at the certificates of its own `--out` file.

To make the list again on other data (more of the chain, other prices or another overhead), run the command of
`plan.md`'s first line with `--out-dir` a scratch directory.  `tools/test_certify_plan.py` tests the script on
synthetic data, and `tests/certify.rs` checks its prediction against `certificates_600_1300.jsonl`: the games that need
a certificate, their open and rejected sources, and the games the rule settles.
