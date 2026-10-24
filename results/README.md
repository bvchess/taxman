# Results of the solver

This directory holds the project's results: a chain of optimal games, the table of optimal games, bounds on opt(n) for
the games at the chain's end and past it, and the certificates of [statement
U](../docs/theory/exactness.md#statement-u).  This page says what each file holds, how far it reaches, how it was made
and what supports its scores.  The [glossary](../docs/GLOSSARY.md) defines the terms; [the frame
model](../docs/theory/frame-model.md#the-frame-model) and statement U, on which the frame solver's scores rest, are
defined in the theory, and the prover in [its README](../solver/noframe/README.md).

A results file has one JSON line per game, as `taxman solve --out` writes it.  The fields are described in
`src/results.rs` and in [the frame solver's reference](../solver/docs/REFERENCE.md#a-results-line---out-schema-5).  The
counts on this page are pinned by `tests/results.rs`, so changing a file means updating both together.

## The files

| file | holds | games | made by |
|---|---|---|---|
| `strict.jsonl` | the committed chain: an optimal game for every n, each built from the one before | 1–4949 | six runs of `taxman solve` ([below](#strictjsonl)) |
| `optimal.json` | the table of optimal games, built into the frame solver | 1–1300 | the Java solver for 1–1000, the chain for 1001–1300 ([below](#optimaljson)) |
| `bounds.jsonl` | the bounds store: bounds on opt(n) for the games from 3915 on, one line a bound | 3915–8191 | the ledger's exports ([below](#boundsjsonl)) |
| `certificates/` | certificates of [statement U](../docs/theory/exactness.md#statement-u) | 600–1300 | `taxman certify` ([below](#certificates)) |

How far each kind of proof reaches:

* **Every game of `strict.jsonl`** is proven within the frame model.
* **Games 2 to 2175** are also proven on the whole board by the prover, so their scores are opt(n) without the frame
  model.
* **Games 2176 to 4949** are proven within the frame model, which is exact given statement U.  U is not certified past
  2175, and `taxman certify` can check it game by game.  A game of them proven by the ceiling or the prime formula is
  opt(n) wherever the game before it is.

"What supports the scores", below, gives the evidence for each.

**`explore.jsonl` is not a chain, and never goes into `strict.jsonl`.**  The file of a sweep ([the frame solver's
README, "Keeping going past limits, and the
sweep"](../solver/frame/README.md#keeping-going-past-limits-and-the-sweep); `solver/tools/cloud/sweep.sh`) merges the
games of many blocks of consecutive games, each given a limited search.  So it has:

* games with only an unfinished search, which is a lower bound and no proof;
* games that no block reached;
* games built on the best game of an unfinished search, with no ceiling and no prime formula;
* several lines of one game.

Each completed game in it is proven by its own proof, given what it assumes, in the frame model.  The file is evidence
for those games alone, and a game that follows a hole may rest on nothing the chain would have given it.  It is not
committed here: a sweep's results go to a file of their own, with a notebook entry saying what they are.

## strict.jsonl

The chain of optimal games from n = 1 to n = 4949, and the file to build on.  Every game is built from the one before,
every line has its full provenance, and no game rests on `optimal.json`.  It has 4949 lines in schema 5, the completed
games in order of n, and no line of an unfinished search.  382 of its lines, all in 3984–4783, are games that the bounds
store had proven (proof `known`): the fourth and fifth runs took each from the store with no search, and its line cites
the store's two lines ([the frame solver's reference, "Citing a known
source"](../solver/docs/REFERENCE.md#citing-a-known-source)).  The name is from the builds that had unproven tactics,
which the chain was made without; none is left, so every run from n = 1 is made without them.  [runs.md](runs.md)
records the six runs that made it.

Its games, by proof, as `taxman verify-results` and `solver/tools/check_results.py` report them:

| | 1–1300 | 1301–2175 | 2176–3440 | 3441–3914 | 3915–4547 | 4548–4874 | 4875–4949 | `strict.jsonl` |
|---|---|---|---|---|---|---|---|---|
| games | 1300 | 875 | 1265 | 474 | 633 | 327 | 75 | 4949 |
| proof `trivial` | 13 | 0 | 0 | 0 | 0 | 0 | 0 | 13 |
| proof `exhausted`: the search explored its whole tree | 416 | 314 | 448 | 178 | 150 | 35 | 31 | 1572 |
| of those, searches the [near-repeat cut](../solver/docs/design/search.md#the-near-repeat-cut) shortened (`stats.near_repeat_cut` above 0) | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 2 |
| proof `cap`: the search reached the [ceiling](../docs/GLOSSARY.md#the-game) | 115 | 77 | 119 | 43 | 57 | 14 | 7 | 432 |
| proof `prime_formula` | 211 | 115 | 155 | 60 | 55 | 13 | 9 | 618 |
| proof `same_search`: a repeat of an earlier game's search | 545 | 369 | 543 | 193 | 204 | 50 | 28 | 1932 |
| proof `known`: a game of the bounds store, taken with no search | 0 | 0 | 0 | 0 | 167 | 215 | 0 | 382 |
| score = n + score(n − 1) | 424 | 270 | 402 | 148 | 218 | 115 | 25 | 1602 |
| score = n + score(n − 1) − P (a prime n) | 210 | 115 | 155 | 60 | 75 | 36 | 9 | 660 |

The near-repeat cut shortens a search and leaves its proof `exhausted`, so a line counts the nodes it cut in
`stats.near_repeat_cut`.  The first five runs and the sixth run to game 4920 searched the games without the cut, and no line of them has the key; 14 lines of games 4921–4949 have it, and two of them cut nodes: 4929 (20,458,994) and 4936 (52,079).

Of its 2004 searched games, 1572 are exhausted and 432 at the ceiling.  The proofs at the ceiling were all decided
before the search explored a node (the method `start`) but six: 686's took 29 nodes, and 2695's, 3052's, 3306's, 3796's
and 3933's took 7,175, 2,826, 5,366, 5,530 and 3,093.  A repeat takes the promotions of the latest earlier game that
posed the same search problem: the game of n − 1 for 1719 of the 1932 repeats, and of n − 2 for the other 213, where
n − 1 is prime.

*The proofs by range, in detail:*

* 1–1000: 311 exhausted, 92 at the ceiling, 168 prime, 13 trivial and 416 repeats.
* 1001–1231: 81 exhausted, 16 at the ceiling, 34 prime and 100 repeats.
* 1232–1300: 24 exhausted, 7 at the ceiling, 9 prime and 29 repeats.  Its 69 games are 31 searches, 29 repeats and the
  primes 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291 and 1297.
* 1301–2175: 314 exhausted, 77 at the ceiling, 115 prime and 369 repeats: 391 searches, 369 repeats and 115 primes.
* 2176–3440: 448 exhausted, 119 at the ceiling, 155 prime and 543 repeats: 567 searches, 543 repeats and 155 primes.
* 3441–3914: 178 exhausted, 43 at the ceiling, 60 prime and 193 repeats: 221 searches, 193 repeats and 60 primes.
* 3915–4547: 150 exhausted, 57 at the ceiling, 55 prime, 204 repeats and 167 from the store: 207 searches, 204 repeats,
  55 primes and 167 games of the store.
* 4548–4874: 35 exhausted, 14 at the ceiling, 13 prime, 50 repeats and 215 from the store: 49 searches, 50 repeats,
  13 primes and 215 games of the store.
* 4875–4949: 31 exhausted, 7 at the ceiling, 9 prime and 28 repeats: 38 searches, 28 repeats and 9 primes.

### Games with several optimal sets

Several games have more than one optimal set of moves, and which one a run finds depends on the order of its search.
So scores are reproducible, but move lists need not be.  Each chain that found a set proved its game optimal, so each
set is optimal.  `tests/results.rs` pins the games where this file differs from the table.  The sets of this file
differ from the earlier files' in these games:

* **539, 684–687 and 770–773**, which the prover lists with two or three optimal sets each ([the prover's
  README](../solver/noframe/docs/uniqueness.md) has the exchanges).  This file has `optimal.json`'s
  set of moves in each of them but 539 and 770–773, where the chain found another optimal set.  The earlier chains of
  commits 243af9b and 271c817 had the table's sets at 539 and 770–773 and another at 684–687.
* **1215**, where `optimal.json` and every earlier chain have {312, 450} and this file {342, 420}: an exchange of two
  numbers of the same sum, 762.  The prover enumerates optimal sets only to n = 989.
* **1221–1224**, where the chain of commit bd9bb5d and the cost-curve study's chain found {364, 456, 544}, and this
  file, `default.jsonl` and the chains of 243af9b and 271c817 have {348, 408, 608}.  It is an exchange of three
  numbers of the same sum, 1364, that starts at 1221 and is carried by 1222, 1223 and 1224.  The prover describes
  exchanges like it, which first appear at game 425.
* **1560–1563**, where the GF(2) study's chain has {510, 594, 620, 624, 660} and this file {476, 528, 616, 644, 744}:
  an exchange of five numbers of the same sum, 3008, found by the search of 1560 and carried by the three repeats after
  it.

## What supports the scores

"Optimal" here means optimal within the frame model, given that the solver is right.  That is opt(n) wherever [statement
U](../docs/theory/exactness.md#statement-u) holds
([DESIGN.md](../solver/docs/design/frame-model.md#what-optimal-means)), and ["The files"](#the-files), above, says how
far each kind of proof reaches.

Two verifiers check the files.  From the top of the repository:

    taxman verify-results results/strict.jsonl
    python3 solver/tools/check_results.py results/strict.jsonl

The first is the frame solver's own.  It also runs on any results file that `solve --out` opens, and in
`tests/results.rs`.  The second is an independent verifier in Python, which shares no code with the solver; it also
checks each score against the Franklín–Moniot upper bound and the best-known heuristic chain where `heuristics/results`
has them.  Both replay every game (legal, scoring what is recorded, with no legal move left, and opening with the
largest prime), confirm each score against `optimal.json` and the bounds score(n − 1) ≤ score(n) ≤ n +
score(n − 1), check each line's arithmetic and the predecessor it names and that the files agree, and derive each
line's proof kind and what it assumes by the rules in `src/results.rs`.  [The frame solver's
reference](../solver/docs/REFERENCE.md#4-why-the-answers-can-be-trusted) defines the proof kinds and the assumptions,
and says what each verifier checks.  Both pass `strict.jsonl`, in about 10 s (42 s of CPU, on several threads) and 3 s
on the laptop.

What that evidence amounts to:

1. **Arithmetic certificates.**  A score at the [ceiling](../docs/GLOSSARY.md#the-game), n + score(n − 1), is optimal
   whenever the game of n − 1 is, whatever found it.  So is a score equal to n + score(n − 1) − P for a prime n, with P
   the largest prime below n (the [prime formula](../docs/GLOSSARY.md#the-game), proven in `src/accelerate.rs`).  2262
   of `strict.jsonl`'s games have such a score: 1602 at n + score(n − 1), among them its 432 proofs at the ceiling, and
   660 prime ones.
2. **Exhaustive searches.**  `strict.jsonl`'s 1572 `exhausted` games are proven by a search that explored its whole
   tree, as the solver reports it.  The rest of the file is its 432 proofs at the ceiling, its 618 prime games, its 13
   trivial ones (no promotion candidate, so there is only one game), its 1932
   [repeats](../docs/GLOSSARY.md#results-and-trust), which both verifiers check against the earlier game
   (`src/results.rs`), and its 382 games of the bounds store.  A game of the store is a game the referee replays at the
   score of an upper bound the store holds ([below](#boundsjsonl)).
3. **Standalone solves.**  Some games were solved again on their own, with no seed and no ceiling:
   * `solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/calib.jsonl` holds 197 standalone searches, one for
     every game of 1001–1231 that is not prime (`ladder --no-seed --no-cap --tactic revise_freedoms=off`, 8 threads,
     commit e710d16; see
     [`solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/cost-curve.md`](../solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/cost-curve.md)).
     All of them finished, and each found the promotion sum of the score recorded here.  These proofs rest on no other
     game.  So every game of 1001–1231 that is not prime is proven without the chain, and every prime one by the prime
     formula from a game so proven.
   * There is no such solve for 1232–1300.  Of those 69 games, 24 are proven by an exhausted search of their own, and
     the 7 proofs at the ceiling, the 9 prime ones and the 29 repeats rest on the game before them or on the game they
     repeat, so on the chain from n = 1 (see 5).
   * Past 1300, `solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/bands.jsonl` has ten games of 1302–1352
     solved standalone (the matching path, 8 threads), each finishing with the promotion sum recorded here.  The rest of
     1301–4949 is the chain's: 1156 exhausted searches of their own, 317 proofs at the ceiling, 407 prime ones and 1387
     repeats that rest on the games before them, and 382 games of the store.
4. **Agreement between builds and files.**
   * n ≤ 1000: `strict.jsonl`'s scores are `optimal.json`'s, all 1000 (each line's `optimal_match` is true;
     `tests/results.rs` checks it).  797 of the games are `optimal.json`'s move for move, and 995 are its sets of moves:
     all but 539 and 770–773 (above).
   * n = 1001–1300: `optimal.json` holds the games of the chain of commit 243af9b, copied move for move by `taxman
     extend-optimal`, and `strict.jsonl`'s games are those, move for move, but 1215 (above; `tests/results.rs` checks
     it).  The lines say `optimal_match` true here too.  When the run was made the table held those games, so this is a
     second run reproducing the first's scores, not a comparison with another source.
   * n = 1301–2175: `optimal.json` has no game here (the lines say `optimal_match` null), so the comparison is with
     another run of the same search.  The GF(2) study's chain was made from `optimal.json`'s game of 1300 (`--use-prev`)
     by the build of commit 716ebd9, on 2 threads in three invocations, and reached 2175 from 1301 in 92 minutes of game
     time.  Its 875 games have this file's scores and proofs in every game, and its moves in all but 1560–1563 (above).
     Its methods are this file's too, except that a game whose search took no step was labeled `search` then and is
     `start` here.  Only its game of 1301 assumes anything: that `optimal.json`'s game of 1300 is optimal.
     `taxman verify-results results/strict.jsonl solver/notebook/search/the-gf2-bound-in-the-search-2026-10/chain.jsonl`
     (93 s) accepts them and finds 875 games in both files, with equal scores in all.  `solver/tools/check_results.py`
     finds the same, and refuses the label `search` of the games with no node.
   * n = 2176–3440: `optimal.json` has no game here either, and no other chain reaches far.
     `solver/frame/testdata/c8g.chain-2176.jsonl`, the Graviton calibration's chain (64 threads, one game after
     another, the build of commit 407645f, 15 minutes), has the games 2176–2276, 101 in all.  A rehearsal of the second
     run, made on the same machine type just before it started (the build of commit 55287ff, 64 threads, a window of
     16, stopped and killed and resumed on purpose; not committed), has 2176–2300, 125 in all.  Both have this file's
     scores, proofs, promotions and moves in every game they share with it, and its methods but for that label (checked
     on the final file).  Past 2300 the evidence is the run's own: the verifiers on the whole file.
   * n = 3441–3914: no other chain reaches here either.  The evidence is the third run's own: it solved 1–3440 again
     before these games, with this file's scores in all of them (its lines of those games are kept in [its
     entry](../solver/notebook/the-run-to-the-wall-2026-10/README.md), not here), and the verifiers pass the whole file.
   * n = 3915–4547: no other chain reaches here either.  The evidence is the fourth run's own, which took games 1–3914
     from this file and did not solve them again, and the verifiers pass the whole file.
   * n = 4548–4874: no other chain reaches here either.  The evidence is the fifth run's own ([runs.md](runs.md)),
     which took games 1–4547 from this file and did not solve them again, and the verifiers pass the whole file.
   * n = 4875–4949: no other chain reaches here either.  The evidence is the sixth run's own (runs.md),
     which took games 1–4874 from this file and did not solve them again, and the verifiers pass the whole file.
   * 1001–1231: the scores of `strict.jsonl`, the cost-curve study's chain (commit e710d16, 8 threads, a chain with the
     unproven tactics off) and `default.jsonl` (the earlier solver, with unproven tactics on) are equal in all 231
     games.  `strict.jsonl`'s games are `default.jsonl`'s move for move but 1215, and the cost-curve chain's but 1215
     and 1221–1224: other games, with the same scores.
   * The earlier committed chains, of commits bd9bb5d, 243af9b and 271c817, have the same scores in all 1300 games.  The
     second's and the third's moves are the same as each other's, and differ from this file's at 539, 684–687, 770–773
     and 1215; the first's differ at those and at 1221–1224.
5. **Independence from `optimal.json`.**  `strict.jsonl` starts at n = 1, so no game of it is built from
   `optimal.json`, which is only compared with, up to 1300; a score that differs would be a hard error.
6. **The prover's proofs.**  The prover proves every n up to 2175 optimal without the frame model, with no frames or
   promotions ([the prover's README](../solver/noframe/README.md); [the parity bound
   (W1)](../docs/theory/parity-bound.md#the-parity-bound)).  So
   `strict.jsonl`'s scores to 2175 do not rest on the frame model either: those 2175 are opt(n) on the whole board.  The
   games 1301–2175 were proven in the cloud run of 2026-10-14 and 15 ([the
   notebook](../solver/notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)).  The 2774
   games of 2176–4949 are proven within the frame model, which is exact given U ([the exactness theorem
   (X7)](../docs/theory/exactness.md#the-exactness-theorem)).  `taxman certify` can check U there game by game, with
   the prover's barred bounds ([the prover's README](../solver/noframe/docs/proofs.md#the-upper-part-game-by-game) has
   them to 3440).

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal, when the game of n
− 1 is not optimal either.  That is what the proof kinds and the assumptions track, and why the chain from n = 1 and
the standalone solves (3) matter.

## optimal.json

The table of optimal games, built into the frame solver.  Its games 1–1000 were made by the Java solver, with the
tactics `reuse_prev` and `revise_freedoms` on.  Its games 1001–1300 are the chain's, added by `taxman extend-optimal`:
first from the chain of commit bd9bb5d, then from that of commit 243af9b, which differs from it at 1221–1224 only.
`strict.jsonl` has the second's games but at 1215.

The file records no proofs and no provenance.  So a game taken from it (`--use-prev`) is assumed optimal, and the
results whose proofs rest on it say so: "the game of n in optimal.json is optimal".  A chain that assumes nothing
beyond the frame model starts at n = 1.  `strict.jsonl` is such a chain, and its games 1–1000 reproduce
`optimal.json`'s scores with no unproven tactic on, in 7.5 s of search in all on 8 threads, the slowest n = 992 at
0.7 s.

The table stays at 1300.  `taxman extend-optimal results/strict.jsonl --out FILE` would add the 3649 games of 1301–4949.

## bounds.jsonl

The bounds store: the bounds on opt(n) that [the
ledger](../solver/notebook/search/the-ledger-bounds-from-both-sides-2026-10/README.md) finds, kept for good, for the
games from 3915 on.  A game is *proven* when its best lower bound equals its best upper bound, and the file is the
record of those proofs, which the lines of proof `known` in `strict.jsonl` cite.  The first `ledger export` made it,
from stage 1 of the ledger past 4400 (2026-10-18), and each export since appends to it.

Its lines are bounds that hold.  A lower bound is the score of a finished game that the referee replays.  An upper
bound is the frame model's, carried between games as [the
theory](../docs/theory/search-rules.md#bounds-carried-between-games) says.

A line is a bound, not a game of `taxman solve`.  It is not in the schema of the results lines, and an export adds
no line of a game that `strict.jsonl` has.  It is not `explore.jsonl`'s kind of file either: that one holds the
sweep's games.

* `taxman solve --use-prev` draws on it: a game it has proven is recorded with no search, and its other games seed and
  end the searches ([the frame solver's README, "Known games"](../solver/frame/README.md#known-games)).
* `taxman verify-results` and `solver/tools/check_results.py` read it for the results lines that cite it (`known`).
  They look the cited lines up, and do not check the store, which its own verifiers do.

**Append-only.**  A line is never changed or removed.  A new line is added only when it improves the file's best bound
of its game and side: a lower bound above every earlier lower bound of the game, or an upper bound below every earlier
one.  So a second export of the same ledger appends nothing, and both verifiers refuse a line that improves nothing.

**The commands**, from the top of the repository.  The program is the example `ledger`, built in `solver/frame` on the
laptop with `cargo build --profile test --example ledger`; its module comment, `examples/ledger/main.rs`, has every
flag.

    ledger export RUN_DIR --bounds results/bounds.jsonl --source NAME [--checkpoints S3_PREFIX]
    ledger run A-B --dir DIR --cpu-hours H ... --bounds results/bounds.jsonl
    ledger verify-bounds results/bounds.jsonl
    ledger games results/bounds.jsonl > games.jsonl
    python3 -I solver/tools/check_bounds.py results/bounds.jsonl --games games.jsonl

* **`export`** appends the bounds of the run's `ledger.jsonl` that improve on the file's best, for the games that
  `results/strict.jsonl` does not have (`--strict` names another file).  It creates the file if there is none, and
  prints what it appended: the games, the lower and upper bounds that are the first of their game or improve one, and
  the games newly proven.  `--source` names the run, which every line records with the day.  `--checkpoints` is the
  place of the run's checkpoints, the prefix of `nN.ckpt` (for instance `s3://BUCKET/ledger-DATE/out/run/ckpt`), which
  is recorded on the dual bounds of the games not proven.
* **`run --bounds`** starts the run from the file's best bounds for the games of its stretch.  Each lower bound's game
  is replayed by the referee, each upper bound is taken as the file has it, and a game with both needs no map.  Its
  lower bound's game is its incumbent and the finder's input, as a proven game that is given is.  Run `verify-bounds`
  on the file first, since the run takes its upper bounds on their word.
* **`taxman solve --use-prev`** takes the upper bounds on their word too, and reads the same best bounds
  (`src/store.rs`, the one copy of the format).  A game whose best bounds meet is recorded as the game of its lower
  bound, replayed by the referee, with proof `known`, citing the two lines and assuming U at the games the upper line
  rests on.  A game with one bound, or a gap between two, seeds its search with the lower bound's game and stops it at
  the upper bound.  The verifiers of results files check that the cited lines exist and say what the results line says
  (rule, run, value).  The bounds themselves are checked by `ledger verify-bounds` and `solver/tools/check_bounds.py`,
  not again by them.

**The line.**  One JSON object, with the fields below and no others:

| field | on | meaning |
|---|---|---|
| `schema` | every line | 1 |
| `n`, `side`, `value`, `rule` | every line | the game, `lower` or `upper`, the bound on opt(n), and how it was derived |
| `from` | the relations | the game the bound came from |
| `from_value` | an upper bound by a relation; a lower bound whose game is cited | the value of the upper bound of `from` it was derived from; the lower bound of `from` whose game the line carries across |
| `bag`, `from_bag` | an upper bound by `repeat` | `in_the_bag` of `n` and of `from` |
| `rests_on` | an upper bound | the games at which [statement U](../docs/theory/exactness.md#statement-u) is assumed, ascending |
| `checkpoint` | a `dual` upper bound | where the checkpoint of the search that gave it is, if the export was told |
| `source`, `date` | every line | the run (`--source`) and the day of the export, `YYYY-MM-DD` |
| `promotions` or `moves` | a lower bound with its game | the game: the promotions it was made from, or its moves |

The rules of a lower bound:

* `start`, the standalone start incumbent of the game's map;
* `search`, a search's best game;
* `lns`, the finder's game;
* `given`, a proven game given to the run;
* and the relations: `monotone`, the game of n − 1 followed by n; `ceiling`, the game of n + 1 without the move n + 1,
  completed; `prime`, the game next to a prime with its opening exchanged; and `repeat`, the promotions of the game of
  another n with the same search problem, played on this board.

The rules of an upper bound:

* `root`, the GF(2) bound of the root;
* `dual`, the dual bound of a search that a limit stopped;
* `exhausted`, a search that explored its whole tree;
* `floor`, a search under a floor F (an incumbent worth F and no set) that completed having found nothing.  No promotion
  set weighs more than F ([the floor lemma (R5)](../docs/theory/search-rules.md#the-floor-lemma)), so the bound is
  `in_the_bag` and F;
* `nogoods`, the Lagrangian bound over nogoods that were each checked to fail to promote, with the GF(2) bound inside,
  as the removed `wall nogoods` made it; nothing makes it any more;
* `given`;
* and the relations: `ceiling`, U(n) ≤ U(n − 1) + n; `monotone`, U(n − 1) ≤ U(n); `prime`, U(p) = U(p − 1) + p − q
  with q the largest prime below p, both ways; and `repeat`, U(n) − bag(n) ≤ U(m) − bag(m).

A line of 3916 reads:

    {"schema":1,"n":3916,"side":"upper","value":4827967,"rule":"ceiling","from":3915,"from_value":4824051,"rests_on":[3915],"source":"ledger-survey-stage3","date":"2026-10-18"}

**A lower bound's game** is given in one of three forms, the most compact that is possible:

* A game made from promotions (a search's, the finder's, the start's, or a repeat of such a game) is its
  `promotions`, ascending, which `solution_for_promotions` turns into moves.  That takes about 330 bytes at n = 900
  and about 1.1 kilobytes at n = 3440, where the moves would take 9.
* A game that a relation (`monotone`, `ceiling`, `prime`) made from the game of another n is not written but cited.
  The line has `from` and `from_value` and no game, and its game is the lemma's construction applied to the game of
  the lower bound of `from` that is worth `from_value`, which a line before it gives.  A game of the chain is never
  cited, since a run need not have the chain.
* Any other game is its `moves`.

**What an upper bound rests on.**  Every upper bound is a bound of the frame model at some game: a root bound, a dual
bound, a completed search or a given game.  That is opt(n) where statement U holds at that game.  The lemmas carry it
between games without more, except the second part of [the repeat lemma
(R6)](../docs/theory/search-rules.md#the-repeat-lemma), the bound on opt(n) − in_the_bag(n), which rests on U at the
game it is carried to.  `rests_on` lists the games:

* a bound of the frame model has its own game alone;
* a relation has what the bound it comes from rests on, and a game of the chain that it cites counts as resting on
  itself;
* a repeat adds its own game.

U is certified for every n up to 2175 ([solver/docs/design/frame-model.md, “What "optimal"
means”](../solver/docs/design/frame-model.md#what-optimal-means)), so a bound whose list holds only games up to 2175
rests on no open statement.  A proven game is opt(n) wherever U holds at the games of the `rests_on` of its upper bound.
A results line that rests on a proven game assumes that, one statement a game (`"statement U holds at n=4501"`), in its
`assumes`.

**Two verifiers**, as the results files have:

* **`ledger verify-bounds FILE`** (Rust; `examples/ledger/verify.rs`) checks every line:
  * its form, and that it improves on the lines before it;
  * that every lower bound's game replays under the referee (`replay_finished`: legal, over, opening with the largest
    prime) at the value of the line, a cited game made from the one it cites;
  * that every `root` bound is computed again and equal, and that every repeat's bags and search problems are computed
    again and equal to its source's;
  * that every relation's arithmetic holds against the line it cites, a line before it with a bound of that game at
    most `from_value` or a game of the chain, and rests on what that line rests on;
  * and that where the chain has a game, the file's bounds enclose its score.

  It reports the proven games, and lists the bounds it takes on their record: the `exhausted`, `dual` and `floor`
  bounds of searches and the `nogoods` bounds, which no check short of the search (or the fit) computes again, and the
  `given` games.
* **`solver/tools/check_bounds.py`** (Python, standard library only, run with `python3 -I`) is the second verifier of
  the games and of the arithmetic.  It shares no code with the Rust crate.  It loads the rules of the game from
  `solver/tools/check_results.py`, and writes out again the arithmetic of the lemmas and the constructions of the cited
  games.  It cannot rebuild a game from promotions, which needs the frame model.  So `ledger games FILE` writes the
  moves of every promotion set, the game that `solution_for_promotions` makes of it, and the script replays those by
  its own rules and checks that they contain the promotions and score the line's value.  Whatever the frame model did
  to make them, a game that is not a game of its value is refused; the link between the promotions and the moves is the
  Rust program's.  What the Python verifier takes on the line's record, because only the Rust one can compute it: the
  root bounds, a repeat's bags and the identity of its two search problems, and the bounds of searches and of nogoods.

Both exit with status 1 if any check fails, and name the line (`line K (n=N)`).

**Size.**  The laptop's dry run of 901..960, with the truth hidden, made [the ledger entry's
`store/`](../solver/notebook/search/the-ledger-bounds-from-both-sides-2026-10/store/): 58 games of the stretch not in
the chain, two exports of one stretch in all, 171 lines of 45 kilobytes, 778 bytes a game.  Its lines took 330 bytes for
a game of promotions, 135 for a cited game, 1.7 kilobytes for a game of moves and 155 for an upper bound.  At the end of
the chain (4400..8191) a promotion set is about 2 kilobytes, a game of moves about 15, and a cited game and an upper
bound about 150 and 220 bytes.  On 2026-10-22 the store held 4,277 games in 9.2 megabytes, about 2.1 kilobytes a game,
against `strict.jsonl`'s 7.1 (35.2 megabytes for 4,949 games).

## certificates

`certificates/` holds the committed certificates of U that `taxman certify` made
([solver/docs/design/certificates.md](../solver/docs/design/certificates.md) says what a certificate is and what it
proves).

`certificates_600_1300.jsonl` has a certificate for each of the 377 games of 600..1300 that need one: 296 certified by
a search, and 81 by posing the same problem as an earlier game.  `taxman verify-certificates` passes it, in 4.6 s.  It
was made by `taxman certify 600-1300 --results results/strict.jsonl U --out
results/certificates/certificates_600_1300.jsonl --threads 1 --progress 0`, in 39 s, on the code of commit 35c614b.
`U` there stands for the four `--upper` files, the prover's barred bounds for 2..1300, 1301..2175, 2176..3314 and
3315..3440 (`solver/noframe/data/upper_*.jsonl`).

The certificates are in a directory of their own because `tests/results.rs` and `solver/tools/check_results.py` take
every `results/*.jsonl` for a chain or the bounds store.  `tests/certify.rs` checks the work list's prediction against
the file: the games that need a certificate, their open and rejected sources, and the games that the rule of the same
problem settles ([the work list of 2176-3440](../solver/tools/certify/README.md)).
