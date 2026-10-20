# The gap past 2175 (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-14.  Concluded 2026-10-15.

**Question.**  The [research agenda](../../../docs/AGENDA.md#5-what-the-cloud-runs-should-measure) left one measurement
to the chain past 2175: the true root GF(2) gap of every searched game, against the chain's own optima.  Which of two
regimes does the gap's growth follow: +7 per 100 n (the committed games', agenda C5), or +57 (the unseeded starts' upper
bound to 4095, which would put the wall between 3000 and 4000)?  Does the cost model of 1301-2175
(`log10 nodes ≈ 3.24 + G/255`) carry over?  Do the gaps explain the dear games (2343 and 2346 with 175 million and 1.14
billion nodes, 3174-3176 about 210 million each, 3300 635 million), or does something else at the root (agenda A4 and
C6)?  And so what does a game cost at 4000, 5000 and 6000, and how far do 24 hours on one `c8g.16xlarge` reach?

**Answer.**  The root gap of the 509 searched games of 2176..3312 runs from 341 to 1,443 (mean 818) and grows +37 per
100 n over 1301-3312, neither regime.  The cost model carries over in slope (a decade of nodes per 250 of gap) and not
in level: the current build's trees are 0.29 times the committed chain's.  The gaps explain the dear games, nothing else
at the root explains the rest (the best of 18 features explains 13% of the residual's variance, none held out), and a
day on one `c8g.16xlarge` reaches about 4,100 (3,700-4,850), the end of the set widths.

## Method

The EC2 chain is the run past 2175 on a `c8g.16xlarge`: `taxman solve`, 64 threads, the code of 55287ff, a window of 16
to about game 3310 and 4 after it.  It had made 2176..3314 in 6.4 hours when a copy of its results file was taken
(2026-10-15).  The measurements ran on the laptop, one process at a time under `nice -n 10`, one thread, with no solver
change and nothing on AWS; [the wall regime](../the-wall-regime-2026-10/README.md) has the commands and data they share.

* **The gaps:** `examples/wall.rs nodes --nodes 1` over the copy, for its 509 searched games past 2175 (400 `exhausted`,
  109 `cap`: ended by the ceiling; a repeat poses its source's problem, and every source is among them).  The gap is the
  root's GF(2) bound less the optimum.  9 minutes.
* **The conflicts and cores:** `examples/matroid.rs lagrange --limit 0` and `cores` for the same games: the conflicts of
  the root bound's heaviest set and the cores it runs into.  3 minutes.  That example's own intersection and the crate's
  agree on the gaps of all 509 games.
* **The committed chain, solved again:** `taxman solve 1301-2175 --threads 1` on the committed chain cut after game
  1300: its 391 searched games as the EC2 chain's build solves them, in order (24 minutes); every score and proof kind
  is the committed chain's.  Why this was needed is in 4 below.
* [gap.py](gap.py) makes the tables of [gap_output.md](gap_output.md), which holds each table in full; the sections
  below keep the rows that decided.

## Results

Each table is in full in [gap_output.md](gap_output.md); the rows that decided are quoted here.

**1. The gaps** ([by band](gap_output.md#the-root-gaps-by-band)).  The 509 searched games of 2176..3312 have root gaps
from 341 to 1,443 (mean 818, sd 211), against 125 to 887 (mean 391) for the 391 searched games of 1301-2175.  The rise
began inside the committed chain, not at 2176: the mean was about 450 at 2100 (473 at 2101-2150) and 660 at 2151-2200.
Then the gap runs in waves of a hundred games and more around a rising level (a mean of 960 at 2301-2400, 530 at
2601-2700, 1,130 at 3101-3200).  The gap per n is steadier: 1000 G/n is 300 on average (224 at 1301-2175) and drifts
down 3 ± 1 per 100 n within 2176-3312.

**2. How the gap grows** (agenda C5; [the table](gap_output.md#the-gaps-growth-with-n)).  The least-squares slope of the
gap on n, per 100 n, with 95% intervals by a moving-block bootstrap of 100 consecutive games (the waves are longer than
15 games, so blocks of 15 understate):

| games | slope | 95% interval |
|---|---|---|
| 1301-2175 (committed chain) | +7.5 | -44 to 20 |
| 2176-3312 | +25.2 | -43 to 83 |
| 1301-3312 | +37.0 | 22 to 50 |

Over 2176-3312 alone the slope cannot be told, the waves being too long for a window of 1,100 n to average out: flat to
2800 (-15.4) and +119 after are waves, not regimes.  Over the whole 1301-3312 the gap grows +37 per 100 n: not the
committed games' +7, which was a flat stretch below 2150, and not +57, which lies outside the interval and at the edge
of the one by blocks of 200 games (6 to 63; 4% of the draws reach it).  The +57 was an upper bound: the unseeded start
is about 770 below the optimum at 3100 (its gap 1,947, the true one 1,181), against 0 to 403 where it had been measured.
The other form of C5, G/n bounded, holds over 2176-3312 (mean 300, drifting down); a power law over 1301-3312 gives G ∝
n^1.4.

**3. The speculation and the threads in the chain's nodes** ([the
table](gap_output.md#the-speculation-and-the-threads-in-a-games-nodes)).  The chain's nodes come from 64 threads with a
window of 16 (4 after about 3310), whose speculation (younger searches run on a weaker incumbent until the game before
them is concluded) added nodes in the [scaling run](../../scaling-to-192-threads-2026-10/README.md).  Against the
chain's own nodes for the same games (2226..2276 and 2277..2336), that run's nodes at a window of 1 or 4 are -0.04 ±
0.03 decades a game over 39 games (-0.11 over the 18 of 2226-2276 where the dear games of the stretch lay, +0.02 over
2277-2336), while a rerun at the chain's own window reproduces its trees to 0.055 decades a game; so what a smaller
window removes is the speculation and not noise.  The threads: 192 against 64 at a window of 1, -0.02 decades (sd 0.05)
over 18 games; the ladder's 1 against 64 threads on 2170 and 2172, 951 thousand nodes against 1.00 million (+0.02).  The
fits below take the chain's nodes less 0.04 decades (a window of 1 or 4) and less 0.02 (one thread).

**4. The cost model on the EC2 chain's trees.**

* **The build** ([the table](gap_output.md#the-committed-chains-build-against-the-ec2-chains)).  The committed chain's
  fit (`log10 nodes = 3.24 + G/255` on 314 searches) belongs to the build of feba6d0; the EC2 chain runs the build of
  55287ff, which has the inner loop's tree changes and the rule of symmetric pairs.  Fitted to its own gaps the EC2
  chain gave `2.99 + G/266` (R² 0.71, residual sd 0.52) and a mean residual of -0.39 decades against the committed
  model, which reads like a cost per unit of gap that falls with n and is largely the build.  So the committed chain's
  391 searched games were solved again by the EC2 chain's build (every score and proof kind the committed chain's, and
  2170 with the 440,988 nodes of [the prime-exchange entry](../prime-exchanges-in-the-search-2026-10/README.md)).  Over
  the 299 searches of 1000 nodes or more a tree of the current build has 0.29 times the committed chain's nodes (-0.54 ±
  0.19 decades a game; the committed fit is for a build with 3.4 times the trees): 0.27 at 1301-1500, 0.41 at 2101-2175
  and 0.44 at 2176-2276, a smaller saving as the gap grows.  All the fits that follow are on this build, on one thread
  in order.
* **The gap alone** ([the fits](gap_output.md#the-cost-model-refitted)).  The same games on the EC2 chain's build give
  `2.46 + G/221` (314 searches, R² 0.64, sd 0.39); the EC2 chain's, `2.93 + G/266` (400, R² 0.71, sd 0.52); both
  together, 1301-3312, `2.67 + G/247` (714 searches, R² 0.852, sd 0.47).  The slope is the committed chain's, about a
  decade of nodes for 250 of gap, and the intercept 0.6 to 0.8 lower (the build).
* **The scale of the gap.**  Scaling the gap by a power of n, G / n^p, fits better, and the chains disagree on the
  power: each prefers p = 0.5 or more, the pooled fit 0.3-0.4 (R² 0.875 at 0.3 against 0.852).  The step that a pooled
  fit leaves between the chains grows with the power (+0.11 ± 0.05 at p = 0, +0.26 ± 0.04 at 0.3, +0.44 ± 0.04 at 0.5),
  so the smaller powers continue the model across 2175 best.  Out of sample, a model fitted to the games of 1301..origin
  and tested on the later ones (origins 2175 to 3000) has a worst bias of 0.58 decades for the gap alone, 0.27 for p =
  0.3, 0.52 for p = 0.5 and 1.22 for G/n.  The gap alone overpredicts every later stretch by 0.2 to 0.6 decades (the
  intercept's drift); G/n underpredicts by 0.8 to 1.2.
* **The model to project with** is the scaled gap with p = 0.3, since it has the smallest worst error and takes 6.1
  hours over the chain's own games against the chain's 6.4; 0 and 0.5 are its brackets.  On 1301-3312 it is
  `2.30 + 0.0479 G / n^0.3` (R² 0.875, sd 0.44): a decade of nodes per 230 of gap at n = 3000, 251 at 4000 and 284
  at 6000.  The exponent is the weakest number of this entry; the projection carries it as a range.

**5. The dear games** (agenda C6; [the table](gap_output.md#the-structure-of-the-residuals-and-the-dear-games)).  The 47
exhausted searches of gap 1,100 or more (12% of the 400) hold 77% of the chain's nodes, the 11 of gap 1,300 or more 53%,
and the ten dearest searches 54% (2346 alone 18.5%); all of the 18 dearest have a gap of 1,094 or more, in runs of high
gaps (2340-2349, 2376-2388, 3102-3104, 3174-3179, 3297-3312).  Against the gap scaled by n^0.3 their residuals are -0.15
to +0.59 decades, the most in excess 2346 (gap 1,346, 1,143 million nodes: +0.59, 3.9 times the model's nodes),
2387-2388 (+0.57, +0.53), 3300 (gap 1,443, 635 million: +0.52, 3.3 times) and 2385 (+0.44), against a residual sd of
0.48 (the dearest of 400 are expected above the model: the 18 average +0.27).  So the gap explains them: a game is dear
because its root gap is high, to within the scatter that every game has (a factor of 3).

**6. What at the root explains the rest** (agenda A4; [the
table](gap_output.md#what-at-the-root-explains-the-residuals)).  The residuals of the cost model run: their lag-1
autocorrelation is +0.86 against the gap alone and +0.84 against the scaled gap, because consecutive boards pose nearly
one problem.  Nineteen features of the root were tried against them, for the 400 exhausted searches: the heaviest set's
conflicts, the cores it runs into, the forced members at the root, the start incumbent's distance below the optimum, and
the sizes (candidates, `max_total`, the freedoms, n).

* **No feature explains much.**  The share of a residual's variance that a feature explains is at most 0.19 against the
  gap alone and 0.13 against the scaled gap (both for the largest conflict; then n 0.17 and 0.05, the candidates 0.15
  and 0.04), and none adds to the cost per unit of gap.  All 18 features at once (without n) explain 0.47 and 0.38 in
  sample, and nothing held out (R² -0.45 and -0.71, a stretch of 40 consecutive searches predicted from a fit to the
  rest).
* **What the roots do show:** the cores grow with the gap they make (the movers in the cores of frames 1-3, 231 on
  average and +5 per 100 n, correlate 0.68 with G), and 161 of the 509 roots force members, which are games of low gap
  (mean 627 against 904) and small trees (geometric mean 0.18 million nodes against 2.65), and carry no residual.

So A4's kill criterion (nothing explains more than 30%) holds: the victim law's branching on whole cores has nothing
measured under it.

**7. The waves of the gap are partly arithmetic.**  The tiers of the board (the numbers with k multiples on it, n/(k+1)
< m ≤ n/k) are its frames, and the odd cores that make the gap are made of movers with three or more distinct primes.  A
linear model of the gap on the counts, by tier k = 1..7, of the numbers m ≤ n of three or more distinct primes, fitted
to nine tenths of the games (consecutive stretches) and tested on the tenth left out, has R² 0.31 over 2176-3312 (n
alone: -0.12) and 0.68 over 1301-3312 (n alone 0.52); the counts of primes by tier, k = 1..9, 0.30 and 0.58.  So about a
third of the waves' variance is arithmetic that is known for every n; the rest is not in these counts.

**8. The projection** ([the tables](gap_output.md#a-game-at-4000-5000-and-6000-and-what-24-hours-reach)).  A search of N
nodes at board n costs N u(n) / 64 seconds on 64 threads, with the cost of a node u(n) = u0 10^(0.02 (n - 3000) / 100).
The chain's own nodes in its 6.4 hours fix u0 = 265 thread-microseconds, and two independent figures agree with the
line: u(2184) = 182 µs (the ladder's proof of 2184 on 64 threads) and u(4000) = 419 µs (the standalone search of 4000 on
192 threads: 417).  The serial work besides the search is 1.7 s a game at 4000 (from `setup_c8g.jsonl`).  A game is
drawn as the chain's were: 35% are exhausted searches (the others are repeats, primes and ceilings, which cost their
serial work and no search), each with the next wave of the gap and the next residual of a stretch of 100 consecutive
searches taken from the observed ones (resampled in blocks, 300 draws, so that the waves and the residuals' runs are
kept).  The gap's trend is the scenario's: **trend**, the linear fit of 2176-3312 (G = 136 + 0.25 n: 1,142 at 4000);
**ratio**, G proportional to n (0.30 n: 1,210 at 4000); **steep**, the old regime's +57 per 100 n from the fit at 3312
(1,361 at 4000).  The model over the chain's own games, 2176..3312, takes 6.1 hours for p = 0.3 on the trend (6.5 on the
ratio), 8.9 for the gap alone and 5.1 for p = 0.5, against the chain's 6.4.  The hours from 3313 to reach a board, and
the board that 24 hours reach (median [10th, 90th percentile]), of the whole chain from 2176 and past 3313:

| model, gap | to 4000 | to 5000 | 24 hours from 2176 | 24 hours past 3313 |
|---|---|---|---|---|
| G / n^0.3, trend | 13 [4, 33] | 107 [28, 229] | 4,088 [3,726, 4,740] | 4,223 [3,863, 4,963] |
| G / n^0.3, ratio | 20 [5, 56] | 228 [50, 521] | 3,956 [3,553, 4,555] | 4,052 [3,674, 4,679] |
| G / n^0.3, steep | 38 [8, 146] | >720 | 3,827 [3,543, 4,250] | 3,886 [3,607, 4,349] |
| the gap alone, ratio | 70 [11, 442] | >720 [177, >720] | 3,720 [3,420, 4,166] | 3,782 [3,427, 4,264] |
| G / n^0.5, trend | 6 [2, 12] | 29 [12, 50] | 4,613 [4,243, 5,229] | 4,849 [4,467, 5,545] |

* **A game at 4000, 5000 and 6000** (p = 0.3, the trend and the ratio): a median searched game has 3 to 6 million nodes
  at 4000 (20 to 36 s of the 64 threads), 13 to 32 million at 5000 (2 to 6 minutes), 48 to 166 million at 6000 (13 to 46
  minutes); the 90th percentile is 40 to 45 times the median.  A game costs ×10 for each 1,050-1,250 n; at p = 0 for
  each 830 n, at p = 0.5 for each 1,500 n.
* **24 hours on one `c8g.16xlarge`** ($0.675 an hour at the cheapest zone's spot price: $16) reach about 4,100 from 2176
  in 24 hours of the whole chain, and 4,050 to 4,220 in 24 hours past 3313, for p = 0.3; 3,720 to 3,860 for the gap
  alone and 4,270 to 4,850 for p = 0.5.  A single run's luck, the waves, moves it by about -400 to +700.  So a day
  reaches about the end of the set widths, 4095, and no further; 4000 takes 13 to 20 hours past 3313, 5000 takes 4 to 10
  days and 6000 a month or more.  Boards past 4095 also need the widths and `MAX_FRAMES` that [boards past
  4095](../../boards-past-4095-2026-10/README.md) built.
* **The old regime** (+57 per 100 n, the `steep` rows) would have taken 38 hours to 4000, and a day would have reached
  3,830 to 3,890.

## Decision

* **The projection uses the scaled gap** with p = 0.3 as its centre and 0 and 0.5 as its brackets; the gap alone is the
  conservative end (3,700-3,900 in a day).
* **The steeper regime is refuted, and the reason is the gaps, not the 6.4 hours.**  The chain's speed alone is no
  evidence (a crest of the gap costs an hour or two of the 64 threads for a hundred games, a valley seconds).  With the
  measured gaps the cost of a game grows ×10 for each 1,050-1,250 n at the centre of the model (830 to 1,500 across its
  exponents), against ×10 for each 380 n for the old regime; 100 times the compute buys 2,100 to 2,500 n (1,700 to
  3,000).
* **The root explains the dear games and nothing more** (agenda A4, killed): what remains is that consecutive boards are
  nearly one problem.
* **The arithmetic model of the waves is not kept.**  The projection draws the waves from the observed ones; the model
  is a lead for the agenda's C6 and for its density-evolution model.
* **Not settled:** the exponent of n (the chains disagree: 0 to 0.2 across 2175, 0.4 to 0.5 within 2176-3312), the gap's
  trend beyond 3312, and the heights of the waves' crests, which set the cost of the dear games.  The cheapest next
  measurement is `gap.py` on a later copy of the chain's results file, which needs no machine.

The cost of the entry: the laptop only, one thread at a time, 9 minutes for the gaps, 3 for the conflicts and cores, 24
for the committed chain's games solved again.  Nothing was run on AWS.

## Files

The files ran on master ee03872 with no change to the solver, as [the wall
regime's](../the-wall-regime-2026-10/README.md#files-and-the-commands-that-made-them) did, with `K` and `wall` as there.
The example `matroid`, which made `anatomy_chain.jsonl`, was removed from master (2026-10-17); it builds at commit
8753189 (`git checkout 8753189`, then `cargo build --release --example matroid` in `rust/`).

| file | command | what it is |
|---|---|---|
| `gaps_chain.jsonl` | `wall nodes --known SNAPSHOT --nodes 1 $(./gap.py games SNAPSHOT) > RAW`, then `./gap.py join SNAPSHOT RAW > gaps_chain.jsonl`; `SNAPSHOT` is a copy of the EC2 chain's results file taken at game 3314 on 2026-10-15 (16.5 MB, not committed), `wall` built from master ee03872, one thread, 9 minutes in all | one line per searched game past 2175 of the snapshot (509 games, 2176..3312: 400 `exhausted`, 109 `cap`; a repeat has its source's gap, and every source is among them): the root's GF(2) bound, the optimum, the true root gap (the bound less the optimum) and the start incumbent, the chain's own nodes, seconds, proof and threads, and the root's sizes (candidates, `max_total`, the heaviest set's size, forced members, the frames' ranks, moves, factors and freedoms) |
| `anatomy_chain.jsonl` | `matroid lagrange --known SNAPSHOT --limit 0 $(./gap.py games SNAPSHOT) > L`, `matroid cores $(./gap.py games SNAPSHOT) > C` (`cargo build --release -j 4 --example matroid`), then `./gap.py join-anatomy SNAPSHOT L C > anatomy_chain.jsonl`; 3 minutes | for the same games, the sizes of the successive conflicts of the root GF(2) bound's heaviest set (`matroid`'s research intersection, whose bound equals `wall`'s `root_bound` at all 509 games) and the cores that set runs into, frame by frame (movers, promoted, factors, movers with three or more live factors) |
| `resolved_1301_2175.jsonl` | `head -n 1300 results/strict.jsonl > RESOLVE.jsonl`, `taxman solve 1301-2175 --threads 1 --out RESOLVE.jsonl --checkpoint-dir DIR --checkpoint-interval 300 --progress 600` (`nice -n 10`, `taxman` built from master ee03872), then `./gap.py join-resolved RESOLVE.jsonl > resolved_1301_2175.jsonl`; 24 minutes | the committed chain's searched games of 1301-2175 solved again by the EC2 chain's build, in order on one thread (every score and proof is the committed chain's): the game's root gap ([roots.jsonl](../the-gf2-bound-in-the-search-2026-10/roots.jsonl)), its nodes, seconds, method and proof from this run beside the committed chain's nodes, so that the two chains' games are on one build |
| `gap.py`, `gap_output.md` | `./gap.py > gap_output.md` (a table at a time: `./gap.py bands trend arithmetic window build cost runs anatomy projection`) | the tables of this entry from `gaps_chain.jsonl`, `anatomy_chain.jsonl`, `resolved_1301_2175.jsonl`, `results/strict.jsonl`, `roots.jsonl` of the GF(2) entry, `solver/notebook/scaling-to-192-threads-2026-10/c8g48/games.jsonl`, `solver/frame/testdata/c8g.chain-2176.jsonl` and `setup_c8g.jsonl`; the module comment says what each table is |

## Commits

* 7a55ecf: the gap past 2175: the root gaps of the EC2 chain's 509 searched games, the cost model on one build, the dear
  games, and what 4000 to 6000 cost.
* ee03872, 55287ff, feba6d0 and 407645f: the builds and the scaling run the measurements rest on (the scaling run's
  merge, the EC2 chain's code, the GF(2) search's merge, and the Graviton calibration's build).
