# Measurements made for the research agenda (2026-10)

Part of the [lab notebook](../README.md).

These were section 7 of the [research agenda](../../docs/AGENDA.md), "Measurements made for this agenda", until
2026-10-18, when they moved here unchanged.  The agenda's sections that cite them are its own.

Scratch code, not kept; what follows is enough to redo them.

## 7.1 The root gaps of the c8g chain

`examples/wall.rs nodes --known KNOWN --nodes 1 N...` (one thread, 29 s), `KNOWN` the committed chain followed by
`solver/frame/testdata/c8g.chain-2176.jsonl`, for the 33 searched games of 2176-2276 that `gaps_c8g.jsonl` lacks:
[solver/notebook/the-wall-regime-2026-10/gaps_c8g_rest.jsonl](../the-wall-regime-2026-10/gaps_c8g_rest.jsonl).  With `gaps_c8g.jsonl` that is every exhausted
search of the c8g chain (39 of at least 50 nodes).  Fit: log10 of `results/strict.jsonl`'s nodes for its 314 exhausted
searches of 1301-2175 (at least 50 nodes) against `gf2 − opt` of `solver/notebook/the-gf2-bound-in-the-search-2026-10/roots.jsonl`, least squares; then the
c8g chain's nodes (64 threads, the build before the inner-loop round, as `strict.jsonl`'s) against it.

| fit over 1301-2175 | coefficients | residual sd | R² |
|---|---|---|---|
| gap | 0.00392 ± 0.00022 per unit (×10 per 255) | 0.46 | 0.50 |
| gap, n | 0.00423 ± 0.00019, −0.00093 ± 0.00009 | 0.39 | 0.63 |
| gap / n | 6.95 ± 0.31 per unit of 1000 G/n | 0.40 | 0.61 |

The c8g chain's 39 searches: mean residual +0.08 ± 0.08 decades against the gap fit (sd 0.50), +0.45 against the fit
with n, +0.67 against the fit on G/n; their own fit ×10 per 193 ± 40 of gap.  Largest residuals 2268 (+1.09), 2260
(+0.95), 2272 (+0.77); smallest 2208 (−0.69), 2205 (−0.65).

## 7.2 Other fields at the roots of the whole game

For every n of 78-1300, the parity bound's heaviest common independent set I at the root (the prover's
`parity::Meet::solve` with 0 and 1 barred, the weights the numbers; 17 s for all), the 1,055 games where it exceeds
opt(n).  I is peeled (front moves: a member with one live factor takes it; back moves: a factor with one live lister
goes to it; each keeps the rank over every field), and the residual R's matrix (its members against their maximal
factors outside I) is tested for rank over GF(3), GF(5), GF(7) and a large prime (for the rationals), with 0/1 entries
and with Koszul signs (the entry of s at s/p is (−1)^i, i the index of p among the distinct primes of s, increasing).

| n | games | mean gap | mean residual | dependent over GF(3), 0/1 | in some other field or signing |
|---|---|---|---|---|---|
| 78-299 | 73 | 6 | 8 | 0% | 0% |
| 300-599 | 281 | 28 | 17 | 43% | 49% |
| 600-899 | 300 | 77 | 33 | 47% | 57% |
| 900-1099 | 200 | 139 | 58 | 28% | 55% |
| 1100-1300 | 201 | 138 | 65 | 48% | 73% |

In all, 412 over GF(3) with 0/1 entries, 30 over GF(5), 12 over GF(7); with signs 209, 39 and 27; none over the
rationals; the rank deficiency is 1 but in six residuals (2, over GF(3)).  Of the 658 square residuals, the integer
determinant with 0/1 entries is ±1 in 408, divisible by 3 in 219, by 5 in 21, by 7 in 10; with signs ±1 in 572; ±1
under both in 358 (54%).

## 7.3 Barred bounds on the upper part

For n = 600, 1000 and 1300 (30 s for the three), the parity bound with each member u of the recorded game above n/2
barred, against opt(n): certified (below opt(n)) for 207 of 224, 339 of 377 and 472 of 490 members; the open ones are
above opt(n) by at most 102, 208 and 147, at or below the root's gap (106, 214, 151).  The open members at 1300 include
the largest prime, 1297, which THEORY.md's lemma puts in every optimal set.

## 7.4 The root gaps of the EC2 chain

`examples/wall.rs nodes --known SNAPSHOT --nodes 1 N...` (one thread, 9 minutes) over the 509 searched games of
2176-3312 of a copy of the EC2 chain's results file (games 1..3314, taken on 2026-10-15: 64 threads on a
`c8g.16xlarge`, the code of 55287ff, a window of 16 to about game 3310 and 4 after it, 6.4 hours for 2176..3314), and
`examples/matroid.rs lagrange --limit 0` and `cores` (removed from master since; it builds at commit 8753189) for the
conflicts and cores of the root's heaviest set (3 minutes); then the committed chain's 391 searched games of 1301-2175
solved again by that build (`taxman solve 1301-2175 --threads 1`, in order), to put the two chains' trees on one
build.  The data are in [solver/notebook/the-wall-regime-2026-10/README.md](../the-wall-regime-2026-10/README.md) (`gaps_chain.jsonl`, `anatomy_chain.jsonl`,
`resolved_1301_2175.jsonl`), the tables in `gap_output.md` (`gap.py` makes them), and the study is the entry
[The gap past 2175](../the-gap-past-2175-2026-10/README.md).  What it found, for sections 2, 3 and 4: the gaps (341
to 1,443, +37 per 100 n over 1301-3312, G/n steady at 300), neither regime of section 2; the cost model on one build
(a decade per 250 of gap, trees 0.29 times the committed chain's, the gap counting in units of n^0.3 with 0 and 0.5 as
brackets); the dear games as the games of high gap; A4 killed; and the projection (a day on a `c8g.16xlarge` reaches
about 4,100).  The day itself reached 3440, below the projection's 10th percentile for the centre model
([the cloud run](../the-frontier-chain-to-3440-2026-10/README.md)).
