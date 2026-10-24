# Proving an upper bound part of the way: searches under a floor (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-18.  Concluded 2026-10-18.

**Question.**  How cheap are the first steps of proving an upper bound down from the root's bound, by searches under a
floor, and does the log of their nodes grow about linearly with the distance proven down?  **Answer.**  The first steps
are very cheap and the cost is log-linear: on 14 games of 2265-3259, a floor a quarter of the gap down costs a median
1/20,000 of the proof's nodes, a half 1/740 and three quarters 1/26, at 0.68 decades of nodes per 100 points.

## Method

A search stopped partway leaves little behind on the upper side: the dual bound of a stopped search closed only 23%, 61%
and 29% of the root gap of games 996, 1020 and 1218 by 65%, 96% and 88% of the way through their trees ([the
ledger](../the-ledger-bounds-from-both-sides-2026-10/README.md)).  The Java solver went at the upper bound one value at
a time ("no set reaches x", then x − 1, and so on), which banks a proven bound at every step.  A search from a *floor* F
above the optimum starts from an incumbent worth F with no set behind it.  If it completes having found nothing, no
promotion set weighs more than F, and its tree has a gap of the distance from the root's bound down to F.

**What was built** (commit 742e88a).  `search::search_above(prepared, cfg, floor, cap, resume)` is the search with a
virtual incumbent worth `floor` and no start (no greedy set, seed, dive or local search).  `SearchResult::floor_held`
says that it completed and found none.  With no floor the code path is the old one, and every other test is unchanged.
[Lemma F](../../../../docs/theory/search-rules.md#the-floor-lemma) says that a completed search with a floor that found no set proves
P\* ≤ F, by the argument of Lemma D.  The tests (`src/search/tests.rs`) cover every game of the Java's that needs a
search, to 400: a floor below the optimum finds it, a floor at it or above completes with nothing found, and a floor at
the root's bound or above ends at the first check, also on four threads, on the matching path and without the rule of
symmetric pairs.  A floor at the optimum has the node count of the search seeded with the optimum on games 300-1000, and
a stopped floor search resumes under its floor and is refused under another.

The command `wall target --fractions F,F,... [--cap S] --known FILE N|A-B...` (`examples/wall/finders.rs`) ran, for each
game, a one-thread search under the floor `root − round(k · (root − opt))` for each share k of the gap, and then the
full proof (the sound search from the known optimum, as the finder hands it to the ledger).  `root` is the root's GF(2)
bound and `opt` the known optimum, both in promotion sums.

**What was run.**  On the laptop on 2026-10-18, with the test-profile build of `wall`, one process a game and four at
once under `nice -n 10`, in 25 minutes: 14 games of 2265-3259 at the shares 0.1, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7,
0.75, 0.8 and 0.9 of the gap.  The ten *main* games are those of [the
landscape's](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md) standalone one-thread
searches that finished in 286 to 1,450 s (2265, 2299, 2423, 2485, 2528, 2555, 3156, 3204, 3221, 3259); four cheaper
games (2416, 2560, 2696, 3064) add points to the curve.  The root gaps are 627 to 1,113 (median 834); the full proofs
took 28.6 to 1,050 CPU s and 0.22 to 5.1 million nodes (4,135 CPU s in all; all the runs 5,812 CPU s).  The cap of 1.3
times the game's standalone time never bit: every search completed, and every search under a floor ended with nothing
heavier than the floor, as it must.

## Results

**1. The cost of proving down 0.25, 0.5 and 0.75 of the gap**, a search under the floor over the full proof, median
(range) over the games:

| share of the gap | nodes, 14 games | nodes, 10 main games | CPU, 14 games | the 14 games together (nodes) |
|---|---|---|---|---|
| 0.25 | 4.8e-5 (1.2e-5 - 3.8e-4) | 4.0e-5 (1.2e-5 - 2.9e-4) | 6.7e-5 (2.2e-5 - 5.2e-4) | 5.5e-5 |
| 0.5 | 1.35e-3 (4.1e-4 - 4.5e-3) | 1.2e-3 (5.2e-4 - 3.6e-3) | 1.5e-3 (5.3e-4 - 5.5e-3) | 1.35e-3 |
| 0.75 | 0.038 (0.018 - 0.114) | 0.037 (0.020 - 0.114) | 0.041 (0.020 - 0.113) | 0.036 |

The CPU follows the nodes, a little above them where the tree is small.  The other shares cost 4.2e-4 of the proof's
nodes at 0.4, 4.8e-3 at 0.6, 0.020 at 0.7, 0.070 at 0.8 and 0.28 at 0.9; the shares 0.1 and 0.2 are searches of 2 to 130
steps.  The floors at 0.1 to 0.9 cost 1,677 CPU s in all beside the proofs' 4,135.

**2. The log of the nodes is about linear in the distance proven down.**  For each game, the least-squares line of
log10(nodes) on the share, through the floors of at least 100 nodes and the full proof, has an r² of 0.988 to 0.999
(median 0.9956) and a worst residual of 0.09 to 0.26 decades.  Its slope is a median 0.68 decades per 100 points of gap
proven down (0.53 to 0.89): a decade per 147, or a factor of about 27 for each quarter of an average gap of 834.  The
slope shows no trend with the share (a median 0.71, 0.63 and 0.69 between 0.25 and 0.5, 0.5 and 0.75, 0.75 and 1).  The
chain's cost model gives log10 of the nodes as 2.67 + G/247 ([the gap past
2175](../the-gap-past-2175-2026-10/README.md)), and the full proofs fit it with a median residual of −0.01 decades.  A
floor at 0.25, 0.5 and 0.75 of the gap costs 1.75, 1.19 and 0.55 decades less than the model gives a game whose gap were
that share: within a game the cost falls faster with distance (a decade per 147) than across games (a decade per 247).

**3. How far down a budget goes.**  As a share of a game's full proof's nodes, 0.1% proves down a median 0.48 of the gap
(0.37 to 0.57), 1% 0.65, 10% 0.83 and 30% 0.90.  In steps from the root's bound (a laptop thread does about 5 thousand
a second), 1,000 steps prove down 370 of a median gap of 834, 10,000 steps (2 s) 514, 100,000 steps 661, and a million
steps 807 (8 of the 14 games reach it).

## Decision

Kept: `search_above` and Lemma F stay in the library, and the ledger uses floors from f775cc0 ([floor-guided drilling in
the ledger](../the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-18-floor-guided-drilling)).  A search
under a floor banks a proven upper bound a long way down from the root's bound for a few thousand steps, and the full
proof is the last and dearest step.  The command `wall target` was removed from the tree on 2026-10-18, and builds at
commit 9d9dae3 (`git checkout 9d9dae3`, then `cargo build --release --example wall` in `rust/`).

The floors here are shares of the known gap, on games with a known optimum; the ledger picks them by distance below the
root's bound on an unsolved board.  The games finish in minutes (root gaps up to 1,113) on one thread.  The ledger's dry
run at game 4500 found the first points of a line at about half the median slope, so these 14 games' line does not carry
to gaps of 1,200 and more past game 3440; the floors' cost there, on many threads, and the checkpoint of a stopped floor
search (which resumes, tested) are not measured.

## Files

The runs used the test-profile build of `wall` of commit 742e88a (`cargo build --profile test --example wall`), copied
out of `target/` before the run, against the committed chain (`results/strict.jsonl`, which gives each game its
optimum).  The module comment of `examples/wall/finders.rs` at 9d9dae3 defines every field of a line.  Times are the
CPU seconds of the process, since the laptop was shared.

* `target.jsonl`: `./run.py target.jsonl --wall PATH`.  One line a game of the 14: the gap, and for each share of it the
  floor search's completion, nodes and CPU seconds, and the full proof's.  Each game ran `wall target --known
  results/strict.jsonl --fractions 0.1,0.2,0.25,0.3,0.4,0.5,0.6,0.7,0.75,0.8,0.9 --cap CAP N`, CAP being 1.3 times the
  game's standalone search in [the landscape's
  `sa/`](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/sa/).
* `run.py`: the runner, with the 14 games in `GAMES`.  It runs the dearest first and skips the games the file has, so an
  interrupted run goes on where it was.
* `tables.py`, `tables.txt`: `./tables.py > tables.txt`, the tables this entry quotes.

## Commits

* 742e88a (2026-10-18): `search_above`, Lemma F, `wall target` and their tests.
* dfd524b (2026-10-18) removed `wall target` from the tree; its parent, 9d9dae3, is the last commit that has it.
