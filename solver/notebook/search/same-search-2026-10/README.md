# Same search (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  Many games pose exactly the search problem of an earlier game of the chain (the same frames, candidates,
`max_total` and factor graph; only the cap differs), so they have its best promotion sum, yet the solver proves it
again: how many are there, what do they cost, and does concluding them from the earlier game hold up?  **Answer.**
Kept: 545 of the 1076 searched games of 1..1300 repeat an earlier game, and the committed chain spent 953 of its 3359 s
of search on them.  The solver concludes each in a fraction of a second without changing a score: 1299 takes 0.08 s,
not 265 s.  The idea is rank 1 of [the pilots' recommended
order](../ideas-from-other-fields-the-pilots-2026-10/README.md#1-the-recommended-order).

## Method

The pilot ([the pilots,
4.1](../ideas-from-other-fields-the-pilots-2026-10/README.md#41-memoizing-searches-that-repeat-rank-1)) hashed the root
frames, the candidates and `max_total` with Rust's `DefaultHasher`.  This round makes the idea a proof kind
([DESIGN.md, "Same search"](../../../docs/design/same-search.md) has the argument).  The problem is written out
canonically (`search::Problem`, which also records the model's factor lists and home frames for every number of the
frames) and compared whole; the digest only finds candidates.

The repeats are found from the setups alone, with no search.  `cargo run --release --example repeats -- 1 1300 --known
results/strict.jsonl` sets up every game of 1..1300 that is neither prime nor trivial as a strict run does, finds the
latest earlier game with an equal problem, and adds up the `seconds.search_all_runs` that the committed strict chain
spent on each repeat.  The setups took 8.4 s for 1076 games, about 8 ms a game and 25 to 30 ms near 1300.

## Results

**How many, and how far back.**  545 of the 1076 games that set a search up repeat an earlier game: 416 in 1..1000, 100
in 1001..1231 (12 of them in 1201..1231) and 29 in 1232..1300.  Each repeats the last board size before it that set a
search up: n − 1 for 471, n − 2 for 74, and for each of those 74, n − 1 is prime.  So a window of 8 board sizes
(`SAME_SEARCH_WINDOW`) misses none, and the setups that a range must make for lines that record no digest (schema 3)
stay at eight.

**What they cost** in the committed `strict.jsonl` (6 threads, commit bd9bb5d), so what a rerun of its chain would not
spend:

| range | repeats | their search | the range's search | share |
|---|---|---|---|---|
| 1..1000 | 416 | 29.0 s | 82.8 s | 35.0% |
| 1001..1231 | 100 | 72.9 s | 409.7 s | 17.8% |
| (1201..1231) | 12 | 45.8 s | 276 s | 16.6% |
| 1232..1300 | 29 | 851.2 s | 2866.3 s | 29.7% |
| 1..1300 | 545 | 953.1 s | 3358.8 s | 28.4% |

The dear repeats of 1232..1300 (game ← the game it repeats, committed search seconds) are 1255←1254 24.2, 1257←1256
23.0, 1261←1260 48.2, 1263←1262 51.3, 1267←1266 63.6, 1271←1270 75.3, 1273←1272 38.5, 1285←1284 36.1, 1293←1292 207.1
and 1299←1298 265.1; the other 19 took 0.1 to 6.7 s.  A repeat saves little where the search reached the cap at once
(0.1 s), and much where the earlier game's proof is a long exhausted search.  The pilot named the first game of each
class (1299 repeats 1296, 1263 and 1261 repeat 1260); the solver takes the latest, which is the same problem.

**At the frontier, without big runs.**  The committed `strict.jsonl` was cut after 1298 (its lines for n ≤ 1298,
incomplete ones included) and extended with `taxman solve 1299 --strict --threads 2 --out FILE`.  1299 is a repeat of
1298: 0.079 s for the game and 0.46 s for the whole process (opening and checking the 1301 lines, and setting the
window's eight schema-3 games up again), score 531389 as committed, against 265 s of search in the committed run.  The
same from the file cut after 1292: 1293 repeats 1292 (0.081 s, 526200; 207 s committed), and 1294 repeats 1293, itself
a repeat (0.082 s).  Cut after 1270: 1271 repeats 1270 (0.064 s, 509880; 75 s committed).  Every file passes `taxman
verify-results` and `solver/tools/check_results.py`.

**Chains with the tactic on and off.**  The strict chain 1..300 concludes 107 games as repeats with the tactic on
(proofs: trivial 13, prime 62, cap 34, exhausted 84, same_search 107), against cap 101 and exhausted 124 with it off,
every score the same; its repeats are exactly the 107 that the setups alone find.  With `revise_freedoms` on as well
(only `reuse_prev` off) there are 107 repeats too, 69 of the 300 games resting on `revise_freedoms`.  With the default
tactics `taxman regress --to 400` has no repeat at all (trivial 13, prime 78, reuse 200, repeat 0, search 109):
`reuse_prev`, which comes first, accepts every game whose problem is its predecessor's.  `--strict` gives repeat 151,
search 158.  The probe's fingerprint is unchanged (6197a47eae6cf8c8; the shortcut is outside the search), and
`cargo test --release` passes, with `tests/same_search.rs` (2.7 s) and the forged repeats of `tests/results.rs`.

## Decision

Kept: tactic `same_search`, on by default and in strict runs; proof `same_search`; results schema 4.  A rerun of the
strict chain to 1300 would skip 28% of its search time, and the frontier games that repeat cost nothing.  The tactic
changes no score, tree or node count of a game that is searched.  Verifying a repeat sets two problems up again (the
verifier caches them by board size), so a schema-4 chain to 1300 takes a few seconds more to open.

## Files

* `README.md`: this record.  The folder holds no data.
* The example `repeats` (`rust/`), which made the numbers above, was removed from master on 2026-10-17.  It builds at
  commit 8753189: `git checkout 8753189`, then `cargo build --release --example repeats` in `rust/`.

## Commits

* f5a2af3 (2026-10-13): the tactic and the proof kind `same_search`.
* 1cc4067 (2026-10-13): a problem set up again is tied to the board size its line records.
* bd9bb5d (2026-10-12): the committed `strict.jsonl` the costs are read from.
* 8753189 (2026-10-17): the last commit that builds the example `repeats`.
