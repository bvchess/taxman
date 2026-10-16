# The upper part game by game, and two cloud runs prepared (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  Steps 2B to 2D of the theory track's second consultation (2026-10-15).  Since the entry above, the
prover's proofs reach 2175, so U holds there; every game past 2175 that rests on the frame model rests on U.  (E5)
For every game of 1301 to the end of the chain that rests on the model, which members of U\*_{≥3} do barred parity
bounds certify, and how many stay open?  (E6) Does the prover's search cost more with the sources of layer 3 and up
free than with the whole upper part fixed to U\*: is the integrated search, which leaves the open sources optional,
worth building?  (E9(a)) Does the frame solver find a game above the recorded score on the board without an open
source, at the games of largest gap past 2175?  E5 ran on the laptop; E6 and E9(a) are built, tested and calibrated on
the laptop and wait for approval to run on EC2 ([RUNBOOK section
10](../../tools/cloud/RUNBOOK.md#10-the-theory-tracks-measurements-e6-and-e9a)).

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
README](../../prover/README.md#the-upper-part-game-by-game):

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
[the gap past 2175](the-gap-past-2175-2026-10.md) measured, and the root's heaviest parity set plays all of U\*_{≥3} in
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
limit, appends a line per pair, resumes, summarizes, and exits 2 on a counterexample; `tools/cloud/hunt.sh` runs it
under `run.sh`'s deadline, saves and power-off, and `spot-watch.sh` now stops it too.  Calibration, one laptop thread
(`bench/hunt/`): at 1302 the three open members end in milliseconds (U is proven there); at 3311 (gap 1303) and 2346
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
