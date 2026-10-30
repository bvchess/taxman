# The certificates of U: the work list of 2176-3440, and the data it is made from

What a certificate is and what it proves: [solver/docs/design/certificates.md](../../docs/design/certificates.md),
["Certificates of U"](../../docs/design/certificates.md), and [the certificate theorem
(E2)](../../../docs/theory/extended-problem.md#the-certificate-theorem).  What the files below showed: ["Certificates
past 2175"](../../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md).
Everything ran on the M1 Max, one thread, `nice -n 10`, on the code of commit 35c614b, from the top of the repository,
against the committed chain (`results/optimal.jsonl.gz`, its games 1..3440) and the prover's barred bounds
(`solver/noframe/data/upper_*.jsonl`).  `U` stands for the four `--upper` files (2..1300, 1301..2175, 2176..3314,
3315..3440).

| file | command | what it is |
|---|---|---|
| `problems_600_3440.jsonl` | `extended problems 600 3440 U --known results/optimal.jsonl.gz --out problems_600_3440.jsonl` (the example `extended`, removed since: `cargo build --release --features studies --example extended` at commit 3ca11bf; 79 s) | one line per game of 600..3440 that needs a certificate (1,575): its extended problem Π⁺(O) set up without a search (frames, two-sided and one-sided candidates, rejected sources, in_the_bag⁺, the recorded game's worth, the GF(2) block words, the set width `certify` takes, the digest) and `same_as`, the latest of the 8 games before it with the same problem (340, every one of equal worth: the games `certify` settles with no search) |
| `plan.jsonl`, [`plan.md`](plan.md), `lists/*.txt` | `solver/tools/certify_plan.py --results results/optimal.jsonl.gz --upper solver/noframe/data/upper_2176_3314.jsonl solver/noframe/data/upper_3315_3440.jsonl --problems solver/tools/certify/problems_600_3440.jsonl --gaps solver/notebook/search/the-gap-past-2175-2026-10/gaps_chain.jsonl solver/notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/gaps_chain_3313_3440.jsonl --from 2176 --to 3440 --era 2176-3310:6.396 --era 3311-3440:16.57 --out-dir solver/tools/certify` | the work list of the campaign and its estimate: per game of 2176..3440 whether it needs a certificate, why it costs what it is expected to cost, and the list files `taxman certify --list` takes (`plan.md` is the summary that an approval reads) |

The certificates of 600..1300, `certificates_600_1300.jsonl`, are in
[results/certificates/](../../../results/README.md#certificates) (a certificate file is a result, not a tool); the same
games certified claim by claim, `certificates_claims_600_1300.jsonl`, and `problems_claims_600_3440.jsonl` are in the
folder of [the entry on the claims of
`unzip`](../../notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md#files).

The campaign itself (its approval, the phases, the stop rule of the pilot) has its own files, made on the same
data after the plan:

| file | what it is |
|---|---|
| `lists/stage4/*.txt` | the plan's games cut into the phases of the campaign by `solver/tools/certify_campaign.py lists --plan solver/tools/certify/plan.jsonl --out-dir solver/tools/certify/lists/stage4`: `pilot7` (phase A: 2340, 3175, 3300, 3335, 3336, 3408, 3420 and the games that rest on them, 10 games, 288 thread-hours), `dear_rest` (phase B: 3440, 3424, 3425, 2346 and the other dear groups, 14 games, 1,837), `rest_1` to `rest_4` (phase B: 567 games, 204 each), `deferred` (phase C: the 115 repeats the rule cannot settle and the 2 games that share with them, 361).  The old lists of `lists/` stay as `certify_plan.py` made them |
| `certificates_sample_1301_2175.jsonl` | 12 real lines of `taxman certify` (commit ed09ac0, 2 threads) on the laptop for games of 1301..2175 (eight searches, three games settled by the rule, one search stopped by `--node-limit` and resumed), the real input of the tests of `solver/tools/certify_pilot.py`; `verify-certificates` passes it |

`solver/tools/certify_campaign.py check` verifies any directory of lists against `plan.jsonl` (every game that needs a
certificate in one list, none in two, a game that shares after the game it shares with in the same list, `deferred` the
unsettled repeats with their groups), and `solver/tools/certify_pilot.py` rates the certificates of the pilot against
the plan and says GO, STOP or COUNTEREXAMPLE.  The procedure, the commands and the cost table are in
[solver/docs/runbook/certify.md](../../docs/runbook/certify.md).

**The two eras.**  `certify_plan.py` prices a search by its nodes, at the thread-seconds per node that the chain's own
run paid in each stretch of it, since the chain ran several searches at once and a game's recorded seconds are the
time it shared the machine.  The wall times are the run's ([the cloud run to
3440](../../notebook/the-frontier-chain-to-3440-2026-10/README.md)): games
2176..3310 in 23,025 s on the first instance (window 16), and 3311..3440 on the second (window 4) in its 62,701 s less
the 3,054 s that the unfinished 3441's 625.8 million nodes took at its measured 205 thousand nodes a second, 59,647 s.
The recorded seconds over-count the first era 3.27 times and the second 1.55 times; `plan.md` gives the total from
the recorded seconds as well, as an upper end.

**The lists.**  `pilot.txt` holds the five dearest games, `dear.txt` the next fifteen with the games that rest on them,
and `rest_1.txt` to `rest_4.txt`
the others, balanced by expected time and each in increasing n, so that the proven part of the chain grows as a run
goes.  A game that the rule of the same problem settles is in the list of the game it rests on, after it: the rule
looks only at the certificates of its own `--out` file.

To make the list again on other data (more of the chain, other prices or another overhead), run the command of
the table above with `--out-dir` a scratch directory (`plan.md` and the lists record it with the paths of an older
layout).  `solver/tools/test_certify_plan.py` tests the script on
synthetic data, and `tests/certify.rs` checks its prediction against `certificates_600_1300.jsonl`: the games that need
a certificate, their open and rejected sources, and the games the rule settles.
