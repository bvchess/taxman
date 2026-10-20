# Proving an upper bound part of the way: searches under a floor (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  Brian's: the search keeps its best promotion set (the incumbent) and prunes a node whose bound is no more
than the incumbent's sum, so a search stopped partway leaves little behind on the upper side: the dual bound of a
stopped search ([the ledger](../the-ledger-bounds-from-both-sides-2026-10/README.md)) closed 23%, 61% and 29% of the root gap of
996, 1020 and 1218 only 65%, 96% and 88% of the way through their trees.  The old Java solver went at the upper bound
one value at a time ("no set reaches x", then x − 1, ...), which banks a proven bound at every step.  A search from a
*floor* F above the optimum, an incumbent worth F with no set behind it, proves "no promotion set weighs more than F" for
the price of a tree whose gap is the distance from the root's bound down to F.  How cheap are the first steps down, and
does the log of the nodes grow about linearly with the distance proven down?

**What was built** (commit 742e88a).  `search::search_above(prepared, cfg, floor, cap, resume)`: the search with a
virtual incumbent worth `floor` and no start (no greedy set, seed, dive or local search); nodes are pruned against it as
against a set of that weight, and a set found heavier becomes the incumbent as usual.  `SearchResult::floor_held` says
that it completed and found none.  With no floor the code path is the old one (the tripwires' node counts, the probe's
trees and every other test are unchanged).  [Lemma F](../../../docs/theory/ledger.md) in
THEORY.md: a completed search with a floor that found no set proves P\* ≤ F, by the argument of Lemma D (the search reads
only the incumbent's value).  The tests (`src/search/tests.rs`): on every game of the Java's that needs a search, to 400,
a floor below the optimum finds it, a floor at it or above completes with nothing found, and a floor at the root's bound
or above ends at the first check (at most one step); the same on four threads and on the matching path and without the
rule of symmetric pairs; the floor at the optimum has the node count of the search seeded with the optimum on games
300-1000; a floor at the ceiling ends the search before it starts; a stopped floor search resumes under its floor and is
refused under another.  The command `wall target --fractions F,F,... [--cap S] --known FILE N|A-B...` (in
`examples/wall/finders.rs`): for each game, with `root` the root's GF(2) bound and `opt` the known optimum (promotion
sums), a one-thread search under the floor `root − round(k · (root − opt))` for each share k, then the full proof (the
sound search from the known optimum as its seed, as the finder hands it to the ledger), each capped at S CPU seconds;
nodes and CPU seconds of each.

**What was run.**  On the laptop on 2026-10-18, the test-profile build of `wall` (opt-level 2), one process a game and
four at once under `nice -n 10`, 25 minutes (the data, commands and tables are in [solver/notebook/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/](README.md)).
14 games of 2265-3259 with the shares 0.1, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.75, 0.8 and 0.9 of the gap: the ten
**main** games are those of [the landscape's](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md) standalone one-thread searches that
finished in 286 to 1,450 s (2265, 2299, 2423, 2485, 2528, 2555, 3156, 3204, 3221, 3259); four cheaper games (2416, 2560,
2696, 3064) add points to the curve.  The root gaps are 627 to 1,113 (median 834); the full proofs took 28.6 to 1,050 CPU
s and 0.22 to 5.1 million nodes (4,135 CPU s in all; all the runs 5,812 CPU s).  The cap was 1.3 times the game's
standalone time and never bit: every search completed, and every search under a floor ended with nothing heavier than
the floor, as it must (the floors are above the optimum).  A game's setup before the floor search's engine starts (the
matching graph and the search's space) takes a millisecond.  The full proofs' nodes equal those of `wall sound --from opt`
in the finders' stage 1 on the four games they share (2416, 2560, 2696, 3064).

**1. The cost of proving down 0.25, 0.5 and 0.75 of the gap**, a search under the floor over the full proof, median (range)
over the games:

| share of the gap | nodes, 14 games | nodes, 10 main games | CPU, 14 games | the 14 games together (nodes) |
|---|---|---|---|---|
| 0.25 | 4.8e-5 (1.2e-5 - 3.8e-4) | 4.0e-5 (1.2e-5 - 2.9e-4) | 6.7e-5 (2.2e-5 - 5.2e-4) | 5.5e-5 |
| 0.5 | 1.35e-3 (4.1e-4 - 4.5e-3) | 1.2e-3 (5.2e-4 - 3.6e-3) | 1.5e-3 (5.3e-4 - 5.5e-3) | 1.35e-3 |
| 0.75 | 0.038 (0.018 - 0.114) | 0.037 (0.020 - 0.114) | 0.041 (0.020 - 0.113) | 0.036 |

The CPU follows the nodes (a little above them where the tree is small: the engine's start and the setup).  The other
shares: 0.4 costs 4.2e-4 of the proof's nodes, 0.6 4.8e-3, 0.7 0.020, 0.8 0.070 and 0.9 0.28.  So proving down a quarter
of the gap costs 1/20,000 of the proof, half of it 1/740, three quarters 1/26, and the shares 0.1 and 0.2 are searches of
a few steps (2 to 130).  In total the floors at 0.1 to 0.9 cost 1,677 CPU s beside the proofs' 4,135.

**2. The log of the nodes is about linear in the distance proven down.**  For each game the least-squares line of
log10(nodes) on the share, through the floors of at least 100 nodes and the full proof (the shortest trees have a floor
of their own at a couple of steps), has an r² of 0.988 to 0.999 (median 0.9956) and its worst residual is 0.09 to 0.26
decades (a factor 1.2 to 1.8).  Its slope is a median 0.68 decades per 100 of gap proven down (0.53 to 0.89), a decade
per 147, or a factor of about 27 for each quarter of an average gap of 834.  Step by step the slope shows no trend with the
share: a median 0.71 between 0.25 and 0.5, 0.63 between 0.5 and 0.75, 0.69 between 0.75 and 1.  The full proofs are on
the chain's cost model (log10 of the nodes is 2.67 + G/247, [the gap past
2175](../the-gap-past-2175-2026-10/README.md)): a median residual of −0.01 decades; a floor at k of the gap costs 1.75, 1.19 and
0.55 decades less than the model gives a game whose gap were k · G (k = 0.25, 0.5, 0.75): within a game the cost falls
faster with distance (a decade per 147) than across games (a decade per 247).

**3. How far down a budget goes.**  As a share of a game's full proof's nodes: 0.1% of them proves down a median 0.48 of
the gap (0.37 to 0.57), 1% 0.65 (0.58 to 0.70), 10% 0.83 (0.75 to 0.87) and 30% 0.90.  In nodes, from the root's bound
(a laptop thread does about 5 thousand steps a second): 1,000 steps prove down 370 (266 to 537) of a median gap of 834,
10,000 steps (2 s) 514 (414 to 755), 100,000 steps 661 (523 to 955), and a million steps 807 (743 to 1,104; 8 of the 14
games reach it).  Against the dual bound of a stopped search, which needed 65-96% of a tree to close 23-61% of the gap, a
floor search closes half the gap with 0.1% of the proof's nodes.

**Answer.**  Yes, the first steps down are very cheap and the cost is about log-linear in the distance: a quarter of the
gap for a median 1/20,000 of the proof's nodes, a half for 1/740, three quarters for 1/26, a decade of nodes per 147 of
distance, on 14 games of 2265-3259.  A search under a floor banks a proven upper bound a long way down from the root's
bound for a few thousand steps, and the full proof is the last, dearest step: the ledger could bank upper-bound
progress with it on games it cannot finish.

**Caveats.**  The floors here are shares of the known gap (a game with a known optimum); a ledger on an unsolved board
would pick them by distance below the root's bound, for which the table of nodes by distance above is the guide.  The
games are standalone searches that finish in minutes (root gaps up to 1,113); whether the cost at a given distance below
the root holds for the gaps of 1,200 to 8,000 past 3440 is not measured, and the data gives no reason to expect a
different slope but cannot show it.  One thread; the floors' runs on many threads and the checkpoint of a stopped floor
search (which resumes, tested) are not measured.  The ledger does not use floors yet.

**Commits.**  742e88a (`search_above`, Lemma F, `wall target` and their tests); this entry adds the data and the scripts in
`solver/notebook/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/`.

## Later (2026-10): the command removed from master

The command `wall target` was removed from the tree on 2026-10-18.  It builds at commit 9d9dae3 (`git show
9d9dae3:rust/examples/wall/finders.rs`): `git checkout 9d9dae3`, then `cargo build --release --example wall` in `rust/`.
`search_above` and its tests stay in the library.

## Files

The runs were made on the laptop on 2026-10-18, one process a game and four at once under `nice -n 10`, with the
test-profile build of `wall` (`cargo build --profile test --example wall` in `rust/`: opt-level 2, debug assertions on)
of commit 742e88a (copied out of `target/` before the run, so that a rebuild could not change it), from `rust/`, against
the committed chain (`results/strict.jsonl`, which gives each game its optimum).  The command `target` was removed from
master on 2026-10-18; it builds at commit 9d9dae3 (`git checkout 9d9dae3` first), where it is defined in the module
comment of `examples/wall/finders.rs`, which defines every field of a line.  Every time is the CPU seconds of the
process (the laptop was shared with other work, so the wall time says little), and the searches are compared by their
nodes as well.

### Files and the commands that made them

| file | command | what it is |
|---|---|---|
| `target.jsonl` | `./bench/target/run.py target.jsonl --wall PATH` | one line a game of the 14 in `run.py` (`wall target --known results/strict.jsonl --fractions 0.1,0.2,0.25,0.3,0.4,0.5,0.6,0.7,0.75,0.8,0.9 --cap CAP N`, the cap in CPU seconds 1.3 times the game's standalone search in `../landscape/sa/`): the gap, and for each share of it the floor search's completion, nodes and CPU seconds, and the full proof's |
| `tables.py`, `tables.txt` | `./bench/target/tables.py > tables.txt` | the tables the entry quotes |


`GAMES` of `run.py` is the 14 games: the ten **main** games are those of `../landscape/sa/` whose standalone one-thread
search finished in 286 to 1,450 s there (2265, 2299, 2423, 2485, 2528, 2555, 3156, 3204, 3221, 3259: root gaps 628 to
1,113), the four **extra** (2416, 2560, 2696, 3064) cost 28 to 160 s of full proof and add points to the curve.
`run.py` runs the dearest first and skips the games the file has, so an interrupted run goes on where it was.
