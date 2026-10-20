# The upper part game by game, and two cloud runs prepared (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  Steps 2B to 2D of the theory track's second consultation (2026-10-15).  Since the entry above, the
prover's proofs reach 2175, so U holds there; every game past 2175 that rests on the frame model rests on U.  (E5)
For every game of 1301 to the end of the chain that rests on the model, which members of U\*_{≥3} do barred parity
bounds certify, and how many stay open?  (E6) Does the prover's search cost more with the sources of layer 3 and up
free than with the whole upper part fixed to U\*: is the integrated search, which leaves the open sources optional,
worth building?  (E9(a)) Does the frame solver find a game above the recorded score on the board without an open
source, at the games of largest gap past 2175?  E5 ran on the laptop; E6 and E9(a) are built, tested and calibrated on
the laptop and wait for approval to run on EC2 ([RUNBOOK section
10](#later-2026-10-the-procedures-of-the-two-runs-from-the-runbook)).

*E5* (commit abcfb18 and the data).  The prover's `upper` (`src/upper.rs`, `src/bin/upper.rs`): per game that rests on
the model (neither at its ceiling nor given by the prime formula), the roles of Theorem B (the largest prime forced
and every other prime barred, U\*_2 forced and every other source of layer 2 barred, and from 187 on, where it checks
that U\*_2 spans every prime up to n/2, every number of layer 2 up to n/2 barred), then each member u of U\*_{≥3}
barred in turn; u is certified when the parity bound is at most the recorded score V (every set beating V that obeys
Theorem B plays u), open otherwise, with its margin.  Its tests: on boards to 60 every member certified against
V − 1, V − 2 and V − 5 is in every playable set beating that claim (found by enumeration with the matching bound
alone; 1,403 checks), Theorem B's roles hold for every optimal set there and every recorded game to 1300, and the
figures at 600 are the consultation's.  It reproduces the consultation's scratch figures where they overlap (600, 1000,
1300, 2000).  The committed chain's 1301-2175 (490 games need it, 300 s) and the copy of the EC2 chain's results of
2026-10-15, 2176-3314 (635, 1,272 s), one thread on the laptop; the table by band is in [the prover's
README](../../noframe/docs/proofs.md#the-upper-part-game-by-game):

| | 1301-2175 | 2176-3314 |
|---|---|---|
| games resting on the model | 490 | 635 |
| members of U\*_{≥3} per game | 371 to 640 | 641 to 993 |
| open, in all | 18,596 of 247,482 (7.5%) | 77,195 of 518,992 (14.9%) |
| open per game | 1 (1305) to 102 (2172) | 15 (2610) to 317 (2346) |
| certified per game | 84% to 99.7% (median 93%) | 54% to 98% (median 85%) |
| the open share against the root's gap | +3.1 points per 100 (R² 0.73) | +2.0 points per 100 (R² 0.73) |
| the open members' layers | 3: 52%, 5: 21%, 4: 19%, 6-10: 8% | 3: 58%, 5: 27%, 4: 9%, 6-11: 7% |

No game is certified whole, and the open share follows the root's gap, not n (it falls with the gap at 2600-2699).
The root's gap with layers 1 and 2 fixed is the solver's GF(2) gap at the root on all 400 games past 2175 that
[the gap past 2175](../search/the-gap-past-2175-2026-10/README.md) measured, and the root's heaviest parity set plays all of U\*_{≥3} in
every one of the 1,125 games, so no game is flagged.  The fixed set of the integrated search (U\* less a game's open
members) is in these files.  THEORY.md's "What is known of U" now cites them; its scratch figures and the sentence on
searches closing U to 1500, which the prover's proofs to 2175 supersede, are gone.

*E6, prepared* (commit 4076730).  `exact --fix-low 1` searches from Theorem B's roles (still a proof: every optimal set
obeys them) and `--fix-low 1 --fix-upper 1` from the roles that also fix the upper part to U\* (the frame model's
problem on the whole board, written `"search":"upper_fixed"` so that no tool counts it as a proof); every searched line
counts the nodes that branched on a source of layer 3 or up (`branched_on_sources`) and those whose roles bar a member
of U\*_{≥3} (`barred_upper`), with `game::Prover::watch`, which changes no tree; checkpoints are kept apart by kind.
`summarize.py sources B D` prints the table and the criterion fixed in advance (B/D ≤ 1.2 on at least 90% of the games
and never above 2, and branching on those sources at most 5% of B's nodes: then build the integrated search's stages 2
and 3; otherwise stage 1 only, and stage 2 on two games first); `prover.sh --list` runs `exact` on given games in its
resumable passes, with no check at the end.  Tests: the counters change nothing (verdicts and nodes), Theorem B's roles
keep every proof and every optimum of 187-700 that needs a search, and with the upper part fixed both counters are 0.
A plain run of 1575 has the committed line's 16,801 nodes.  On the laptop, one thread:

| n | gap | plain proof (section 3) | B: nodes, s | D: nodes, s | B's counters |
|---|---|---|---|---|---|
| 1575 | 364 | 16,801 nodes, 36.1 s (laptop) | 16,651, 19.8 | 16,651, 19.4 | 0, 0 |
| 1584 | 400 | 25,981 nodes, 60 s (Graviton4) | 26,516, 30.8 | 26,510, 30.8 | 0, 0 |

Two games say nothing about the criterion, but one thing is plain already: **Theorem B's roles halve the time of a
node** at the same nodes within 2.1%, since the parity solves have the primes and the numbers of layer 2 settled.  The
prover's next campaign can run with `--fix-low 1` and stay a proof.  Over 56 searched games of 187-700 (the test) B
took 9,405 nodes and D 9,398, with 3 nodes branching on a source of layer 3 or up and 6 barring a member of
U\*_{≥3}.  The cloud run: the 25 games of the design (gaps 364 to 603, proofs of 0.6 to 30 minutes, 4.8 thread-hours in
the run of 1301-2175), B and D, on one `c8g.4xlarge`: one to one and a half hours, about $0.40 at spot, $1 on demand.

*E9(a), prepared* (commit 8afefaa).  `sacrifice::hunt(n, hole, claim, ...)` searches the board without `hole` with
the frame model from a handicapped incumbent worth claim − in_the_bag with an empty payload (the soundness tests'
method), so it looks only for promotion sets beating the claim; a set found becomes a game, refereed on the punctured
board, and `examples/hunt.rs` replays it on the whole board too (a source is nobody's factor, so a game without it is a
game of n).  The search's payload helpers are public for it (`search::payload_of`, `set_of_payload`).  Tests: on 624
pairs (every source of the boards 30, 37, ..., 149) the hunt finds a game at one below the punctured board's exact
optimum and nothing at it, the game replaying on the whole board; on every board to 30 and every hole it agrees with
the brute force.  `examples/hunt.rs` takes the 20 games of largest gap that need a certificate and each one's open
members (`--per-game`, `--list` to choose otherwise), runs `--jobs` pairs at once on `--threads` each with a time
limit, appends a line per pair, resumes, summarizes, and exits 2 on a counterexample; `solver/tools/cloud/hunt.sh` runs it
under `run.sh`'s deadline, saves and power-off, and `spot-watch.sh` now stops it too.  Calibration, one laptop thread
(`calibration.jsonl`): at 1302 the three open members end in milliseconds (U is proven there); at 3311 (gap 1303) and 2346
(gap 1346) the cost of a pair follows its margin, not the gap: under 6 seconds to margin 781, 51 to 182 seconds at 835
to 899, and every pair from 959 to 1215 out of its 3 to 5 minutes; no counterexample.  The 20 games (3300,
3174-3176, 3104, 2346, 3304, 3305, 3309, 2340, 3311, 3102, 3103, 3312, 3172, 3173, 3105, 3107, 3168, 3171; gaps 1257 to
1443) have 4,499 open members, 578 of margin above 900; at 4 threads and 10 minutes a pair, about 400 vCPU-hours: a
`c8g.16xlarge` for about 7 hours, $5 at spot, $18 on demand, with a few hundred timeouts among the largest margins.
The punctured boards are numbered by the present (breadth-first) numbering, which the census did not check on them;
that can only hide games from the hunt, never invent one, since every game found is refereed.  (Since the next entry
every board, a punctured one included, is numbered by layer, which is clean by construction, so the hunt's model on
a punctured board is exact given the sources setup keeps there: THEORY.md, "Boards with holes".)

*What it changes.*  The figures of U's certification are now committed data rather than scratch measurements, and they
say that the barred bounds alone close no game: the certificate needs a search of the open sources, at 15 to 317 a game
past 2175, which is what the integrated search (stage 1 next, after numbering by layer) is for.  E6 decides how far to
build it and E9(a) is the cheap early warning if U fails; both wait for approval.  Theorem B's roles halve the
prover's node at no cost to its proofs.

## Later (2026-10): the program removed from master

The example `hunt` and its script `solver/tools/cloud/hunt.sh` were removed from master on 2026-10-17.  They build at commit
8753189: `git checkout 8753189`, then `cargo build --release --example hunt` in `rust/`.

## Later (2026-10): the procedures of the two runs, from the RUNBOOK

Until 2026-10-18 the [RUNBOOK](../../docs/RUNBOOK.md) held the procedures of E6 and E9(a) as its section 10, "The
theory track's measurements: E6 and E9(a)".  Neither ran, and the RUNBOOK no longer holds them, so they moved
here unchanged.  The section numbers they cite (9, 9.4, 9.5, 9.6, 11) are the RUNBOOK's at commit 46f360d.

*Removed from master (2026-10-17).*  The hunt of 10.2 (E9(a)) has no program or script on master any more: the example
`hunt` and `solver/tools/cloud/hunt.sh` went with the examples of finished experiments (the certificates of section 11
superseded the hunt, which was never run; its calibrations are in `bench/hunt/`).  Both build and run at commit
8753189: launch from a checkout of it (`git checkout 8753189`), where the rest of 10.2 holds as written.  E6 (10.1)
runs the prover and is unaffected.

Two measurements of the theory track (docs/THEORY.md, "The upper part"; the account is the entry
["The upper part game by game"](README.md)),
built, tested and calibrated on the laptop and **not run**: each waits for Brian's approval with the estimate below
([rules for every launch](../../docs/runbook/launch.md#rules-for-every-launch)).  Both launch as section 9.4 does (an instance profile, steps
through SSM, `S3_DEST`, a timer, `spot-watch.sh` beside the run, a tmux session), from a checkout of a commit that has
this section (pushed first: the instance clones it); only the instance, the setup and the command differ, and the
results come home as in 9.6.  E6 may run first; E9(a) is independent of it.

### 10.1 E6: does leaving the sources free cost the prover's search anything?

*What.*  `exact` on 25 games of 1301-2175 with a root parity gap of 364 to 603 and a proof of 0.6 to 30 minutes in
the run of section 9, twice: **B**, the default `--fix-low 1`, from the roles of Theorem B (the largest prime and
U\*_2 forced, every other prime and number of layer 2 barred), the sources of layer 3 and up free, which is still a
proof of opt(n); and **D**, `--fix-upper 1`, the whole upper part fixed to U\* as well, which is the frame model's
problem on the whole board and proves nothing (its lines say `"search":"upper_fixed"`).  Each line also counts the
nodes that branched on a source of layer 3 or up and those that barred a member of U\*_{≥3}.  The games, dearest first:

```
L=2159,2168,2166,2167,1800,2020,1537,1820,1504,1400,1551,2100,2160,2165,2163,1512,1482,1495,1547,1445,1573,1500,1480,1584,1575
```

*Criterion, fixed in advance* (`summarize.py sources` prints it part by part): B/D at most 1.2 on at least 90% of the
games and never above 2, and B's nodes branching on a source of layer 3 or up at most 5% of all B's nodes → build the
integrated search's stages 2 and 3; otherwise build its stage 1 only and measure stage 2 on two games first.

*Cost.*  The 25 proofs took 4.8 thread-hours in the run of section 9, the dearest (2159) 30 minutes.  On the laptop,
B at 1575 and 1584 had the plain proof's nodes within 2.1% and took half its time a node (Theorem B's roles leave the
parity solves fewer free elements), and D had B's nodes; so B and D are about 2.5 thread-hours each, and each set
of 25 is bounded by its dearest game, about 16 to 30 minutes on a `c8g.4xlarge` (16 threads).  With the setup (about
10 minutes, `PROVER_ONLY=1`) and margin: **one to one and a half instance-hours, about $0.40 at the spot price of
2026-10-14 ($0.24 an hour), $1 on demand.**  `DEADLINE_HOURS=3` and a timer of 200 minutes bound it.

*Run* (the launch of 9.4 with `Purpose=e6` and the user data `shutdown -h +200`; then `PROVER_ONLY=1 setup.sh`):

```
export D=s3://taxman-013764204840-us-east-1/e6 W=/home/ec2-user/taxman-work/results
cd ~/taxman && tmux new-session -d -s e6 "export DEADLINE_HOURS=3 S3_DEST=$D; \
    OUT=$W/e6_low.jsonl solver/tools/cloud/prover.sh --list $L; \
    POWEROFF=always OUT=$W/e6_upper.jsonl EXTRA_ARGS='--fix-upper 1' solver/tools/cloud/prover.sh --list $L"
tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh
```

The two runs share the deadline (it is fixed at the first start), the second powers the machine off, and each is
resumable as in 9.5 (`prover.sh --list` skips the games its `OUT` has).  A stop with "a playable set worth X beats the
recorded Y" in `prover.err` is a counterexample (9.5).  At home:

```
aws s3 cp $D/results/e6_low.jsonl solver/noframe/data/e6_low_25.jsonl
aws s3 cp $D/results/e6_upper.jsonl solver/noframe/data/e6_upper_25.jsonl
python3 solver/noframe/tools/summarize.py sources solver/noframe/data/e6_low_25.jsonl solver/noframe/data/e6_upper_25.jsonl
```

### 10.2 E9(a): a counterexample hunt at the games of largest gap

*What.*  For the 20 games of largest root gap past 2175 that rest on the frame model (`solver/noframe/data/
upper_2176_3314.jsonl`, from the copy of the EC2 chain's results of 2026-10-15: 3300, 3174-3176, 3104, 2346,
3304, 3305, 3309, 2340, 3311, 3102, 3103, 3312, 3172, 3173, 3105, 3107, 3168, 3171, gaps 1257 to 1443) and each of
their open sources u (those the barred parity bounds did not certify; 4,499 pairs), the frame solver on the board
without u, searched from an incumbent worth the recorded score V, so that it looks only for a game above V
(`examples/hunt.rs`; `sacrifice::hunt`), 4 threads a pair, 16 pairs at once.  A game of the board without u is a game
of n, so a hit is a game better than the chain's.

*Policy* (the theory track's, program part III, section 15(a)).  A pair's margin is the barred parity bound's excess
over V, and the pairs of largest margin matter most (a counterexample needs the punctured problem's own slack to be
below the margin) but will not finish.  So the pairs run in **decreasing margin**, so that a run cut short has done the
informative ones; those of **margin up to 900 run to completion** under a limit of 10 minutes (`MINUTES`), and those
**above 900 are probes**: the solver's own start on the board without u (greedy set, dive, local search; the seed too
if `EXTRA_ARGS='--results FILE'` gives the recorded games, which the repository has only as far as the committed chain
goes) and then 3 minutes of search (`PROBE_MINUTES`), recorded as `probed`, never as `none`.

*Criterion.*  Any `found` line is a counterexample to U and to the chain's score at n: stop extending the chain,
report it, and solve n again on the whole board (the theory track's program, part I, section 6).  A `none` is
evidence, recorded with the margins searched, and a `probed` only says that the start and 3 minutes found no game
(the line has `start_score`, how near V the start came, and `finished` if the search happened to end); a `timeout`,
of a pair of margin up to 900 that outlasted 10 minutes, says nothing.

*Cost.*  Calibrated on one laptop thread (`solver/notebook/theorem-b-by-default-in-the-prover-and-the-hunts-probes-2026-10/recalibration.jsonl`, 2026-10-15, with the code of this policy on
master): a pair costs what its margin (the barred bound's excess over V) says, not what its game's gap does.  At 3311
(gap 1303) and 2346 (gap 1346) every pair of margin up to 781 ended in under 6 seconds, 835 in 53 and 896-899 in 121 to
185, with the nodes of the first calibration to the node (numbering by layer changed no tree); every one from 959 up
(to 1215) is a probe, which takes its 3 minutes and a start of 2 to 7 seconds, and ends `probed` (they were
`timeout`), the start reaching 598 to 854 below V.  Of the 4,499 pairs 3,533 have a margin up to 800, 388 between 800
and 900 and 578 above 900.  The probes cost their time whatever happens: 578 × 3.1 minutes × 4 threads is about **120
vCPU-hours**; the 388 pairs of 800 to 900 about a minute and a half of one thread each, 10 to 15 vCPU-hours; the 3,533
below 800 a few seconds each, 3 to 5.  So **about 140 vCPU-hours (the first estimate, which let every pair use its
whole 10 minutes, was 400): a `c8g.16xlarge` (64 vCPUs) for about 2.2 hours, 2.6 with the setup, about $2 at the spot
price of 2026-10-14 ($0.68 an hour), $7 on demand.**  The probes come first, so about the first 2 hours are the
informative ones.  The four-thread speed-up of a pair was not measured (the calibration is one thread); were it nil,
the two cheaper bands would cost four times as much, 190 vCPU-hours in all.  `PROBE_MINUTES=2` takes a third off the
probes; `PER_GAME=K` keeps the K open sources of largest margin of each game.  `DEADLINE_HOURS=5` and a timer of 320
minutes bound it at about $3.40 at spot.

*Run* (the launch of 9.4 on a `c8g.16xlarge` with `Purpose=hunt` and `shutdown -h +320`; then the full `setup.sh`,
which builds the solver's examples, `hunt` among them):

```
export D=s3://taxman-013764204840-us-east-1/hunt
cd ~/taxman && tmux new-session -d -s hunt "DEADLINE_HOURS=5 POWEROFF=always S3_DEST=$D \
    rust/tools/cloud/hunt.sh solver/noframe/data/upper_2176_3314.jsonl"
tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh
```

`hunt.sh` reruns the hunt after a crash and resumes it after a stop (finished pairs are in its output); exit status 2
and "COUNTEREXAMPLE" lines in `hunt.log` are the finding to look for.  At home: `aws s3 cp $D/results/hunt.jsonl
rust/bench/hunt/hunt_2176_3314.jsonl`, the summary from `hunt.out` beside it, and the README there brought up to date.

## Files

What the hunt does and what each verdict is worth: the module comment of `examples/hunt.rs` and `sacrifice::hunt`
(the board without one open source of U\*, searched by the frame solver from an incumbent worth the recorded score, so
that it looks only for a better game).  Why: docs/THEORY.md, "The upper part" (U), and the entry
["The upper part game by game"](README.md).
The open sources and their margins come from the prover's `upper` (`../../prover/data/upper_*.jsonl`).  The example
`hunt` and its script `solver/tools/cloud/hunt.sh` were removed from master (2026-10-17); they build and run at commit 8753189
(`git checkout 8753189`, then `cargo build --release --example hunt` in `rust/`).

| file | command | what it is |
|---|---|---|
| `calibration.jsonl` | `hunt --upper ../../prover/data/upper_1301_2175.jsonl --list 1302:682,1302:715,1302:754 --minutes 2`, then on `upper_2176_3314.jsonl` four pairs of 3311 (`--minutes 3`), five (`--minutes 6`), three of 3311 and three of 2346 (`--minutes 5`); one thread, one job, `nice -n 10`, on the M1 Max, the code of the commit that added the hunt (and one smoke pair, 1304:682) | 19 pairs, no counterexample: at 1302 (U proven there) `none` in milliseconds; at 3311 (gap 1303) and 2346 (gap 1346) `none` for every margin up to 899 (under 6 s to margin 781, 51 to 182 s at 835 to 899), `timeout` for every margin from 959 to 1215 |

The cost of a pair follows its margin, the barred parity bound's excess over the recorded score, not its game's gap:

| margin | pairs | verdict | seconds (one thread) | nodes |
|---|---|---|---|---|
| 10 to 582 | 7 | none | 0.01 to 2.7 | 1 to 9,634 |
| 674 to 781 | 3 | none | 1.0 to 5.5 | 2,485 to 15,108 |
| 835 to 899 | 3 | none | 51 to 182 | 152,993 to 1,146,404 |
| 959 to 1215 | 6 | timeout | 180 to 300 (the limit) | 710,842 to 2,046,333 |


`solver/docs/RUNBOOK.md`, section 10.2, turns this into the estimate of the cloud run (20 games, 4,499 pairs); its
output is to go here as `hunt_2176_3314.jsonl` with its summary.
