# Results of the solver

This directory holds the project's results: a chain of optimal games, bounds on opt(n) for the games at the chain's end
and past it, and the certificates of [statement U](../docs/theory/exactness.md#statement-u).  This page says what each
file holds, how far it reaches, how it was made and what supports its scores.  The [glossary](../docs/GLOSSARY.md)
defines the terms; [the frame model](../docs/theory/frame-model.md#the-frame-model) and statement U, on which the frame
solver's scores rest, are defined in the theory, and the prover in [its README](../solver/noframe/README.md).

A results file has one JSON line per game, as `taxman solve --out` writes it.  The fields are described in
`src/results.rs` and in [the frame solver's reference](../solver/docs/REFERENCE.md#a-results-line---out-schema-5).  The
counts on this page are a snapshot, dated 2026-10-29, which no test pins: the commands in
[the next section](#optimaljsonlgz) print the current ones.

## The files

| file | holds | games | made by |
|---|---|---|---|
| `optimal.jsonl.gz` | the committed chain: an optimal game for every n, each built from the one before; the table of optimal games built into the frame solver | 1–5060 on 2026-10-29 | six runs of `taxman solve` ([below](#optimaljsonlgz)) |
| `bounds.jsonl` | the bounds store: bounds on opt(n) for the games from 3915 on, one line a bound | 3915–8191 | the ledger's exports ([below](#boundsjsonl)) |
| `certificates/` | certificates of [statement U](../docs/theory/exactness.md#statement-u) | 600–1300 | `taxman certify` ([below](#certificates)) |

How far each kind of proof reaches:

* **Every game of `optimal.jsonl.gz`** is proven within the frame model.
* **Games 2 to 2175** are also proven on the whole board by the prover, so their scores are opt(n) without the frame
  model.
* **Games 2176 on** are proven within the frame model, which is exact given statement U.  U is not certified past 2175,
  and `taxman certify` can check it game by game.  A game of them proven by the ceiling or the prime formula is opt(n)
  wherever the game before it is.

"What supports the scores", below, gives the evidence for each.

**`explore.jsonl` is not a chain, and never goes into `optimal.jsonl.gz`.**  The file of a sweep ([the frame solver's
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

## optimal.jsonl.gz

The chain of optimal games from n = 1, the file to build on, and the table of optimal games that the frame solver has
built in.  Every game is built from the one before, every line has its full provenance, and no game rests on a table.  It
has one line a game in schema 5, the completed games in order of n (so line n is game n), and no line of an unfinished
search.  Some of its lines are games that the bounds store had proven (proof `known`, in the table below): the fourth and
fifth runs took each from the store with no search, and its line cites the store's two lines ([the frame solver's
reference, "Citing a known source"](../solver/docs/REFERENCE.md#citing-a-known-source)).  [runs.md](runs.md) records the
six runs that made it.

**The file is gzip, one member for each batch of games.**  The first 500 games are one member, the next 500 the next, and
so on; a batch of games brought home later is appended as one member of its own (`solver/docs/runbook/chain.md`,
["Verifying, and bringing the results home"](../solver/docs/runbook/chain.md#verifying-and-bringing-the-results-home)),
so the bytes of the games before it never change.  Both Rust programs read it through the crate
[solver/gzip](../solver/gzip/README.md), which checks each member's CRC-32 and so fails loudly on a damaged file;
Python and the shell read it with `gzip`.  The frame solver builds the file into its binary.  Its table is the games of
the file as of the build, each decoded and parsed when it is asked for, member by member: a run that needs none decodes
nothing, and one that needs the first games decodes the first members ([the design](../solver/docs/design/engine.md)).
The table holds each line's score and moves and not its proof, so a game taken from it (`--use-prev`) is assumed
optimal, and the results whose proofs rest on it say so: "the game of n in results/optimal.jsonl.gz is optimal".  A
chain that assumes nothing beyond the frame model starts at n = 1, as this one does.

To read it, from the top of the repository:

    gzip -dc results/optimal.jsonl.gz | sed -n 5000p      # the line of game 5000
    taxman verify-results results/optimal.jsonl.gz
    python3 solver/tools/check_results.py results/optimal.jsonl.gz

The last two are the verifiers, which print the counts below as they stand.

**The snapshot of 2026-10-29: games 1–5060** in 12 members (ten of 500 games, one of 55 and one of 5), 1,097,929
bytes, which decompress to 36,779,313 bytes.  Its games, by proof, as the verifiers report them:

| | 1–1300 | 1301–2175 | 2176–3440 | 3441–3914 | 3915–4547 | 4548–4874 | 4875–5000 | 5001–5060 | all |
|---|---|---|---|---|---|---|---|---|---|
| games | 1300 | 875 | 1265 | 474 | 633 | 327 | 126 | 60 | 5060 |
| proof `trivial` | 13 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 13 |
| proof `exhausted`: the search explored its whole tree | 416 | 314 | 448 | 178 | 150 | 35 | 49 | 22 | 1612 |
| of those, searches the [near-repeat cut](../solver/docs/design/search.md#the-near-repeat-cut) shortened (`stats.near_repeat_cut` above 0) | 0 | 0 | 0 | 0 | 0 | 0 | 5 | 4 | 9 |
| proof `cap`: the search reached the [ceiling](../docs/GLOSSARY.md#the-game) | 115 | 77 | 119 | 43 | 57 | 14 | 11 | 8 | 444 |
| proof `prime_formula` | 211 | 115 | 155 | 60 | 55 | 13 | 17 | 8 | 634 |
| proof `same_search`: a repeat of an earlier game's search | 545 | 369 | 543 | 193 | 204 | 50 | 49 | 22 | 1975 |
| proof `known`: a game of the bounds store, taken with no search | 0 | 0 | 0 | 0 | 167 | 215 | 0 | 0 | 382 |
| score = n + score(n − 1) | 424 | 270 | 402 | 148 | 218 | 115 | 42 | 20 | 1639 |
| score = n + score(n − 1) − P (a prime n) | 210 | 115 | 155 | 60 | 75 | 36 | 17 | 8 | 676 |

The near-repeat cut shortens a search and leaves its proof `exhausted`, so a line counts the nodes it cut in
`stats.near_repeat_cut`.  The first five runs and the sixth run to game 4920 searched the games without the cut, and
no line of them has the key; 66 lines of games 4921–5060 have it, and nine of them cut nodes: 4929 (20,458,994), 4936
(52,079), 4952 (2,670), 4953 (165), 4991 (58,300,645), 5037 (254,558), 5043 (12,716,671), 5047 (1,925,714) and 5048
(585,636).

Of its 2056 searched games, 1612 are exhausted and 444 at the ceiling.  The proofs at the ceiling were all decided
before the search explored a node (the method `start`) but six: 686's took 29 nodes, and 2695's, 3052's, 3306's, 3796's
and 3933's took 7,175, 2,826, 5,366, 5,530 and 3,093.  A repeat takes the promotions of the latest earlier game that
posed the same search problem: the game of n − 1 for 1758 of the 1975 repeats, and of n − 2 for the other 217, where
n − 1 is prime.

**Most games that need no search can be told from n alone.**  Write n = c · P with P the largest prime factor of n, and
call c the cofactor.  A game whose cofactor is 1, a prime, twice a prime or 9 was settled with no search: a prime by the
prime formula, nearly all the others as repeats, and the rest at the start.  Over games 1301–4979, the chain's games past
4949 included, this rule names about 85% of the games settled with no search, and none of the games it names has the
method `search`.  [The lab notebook's entry](../solver/notebook/search/which-games-need-no-search-the-cofactor-of-n-2026-10/README.md)
has the tables and the cofactors that are always searched.

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
* 4875–5000: 49 exhausted, 11 at the ceiling, 17 prime and 49 repeats: 60 searches, 49 repeats and 17 primes.
* 5001–5060: 22 exhausted, 8 at the ceiling, 8 prime and 22 repeats: 30 searches, 22 repeats and 8 primes.

### Games with several optimal sets

Several games have more than one optimal set of moves, and which one a run finds depends on the order of its search.
So scores are reproducible, but move lists need not be.  Each chain that found a set proved its game optimal, so each
set is optimal.  The file has the sets of the chain that made it, from n = 1.  Until the committed chain and the table
of optimal games became this one file, the table (`optimal.json`: the Java solver's games 1–1000, the earlier chains'
games to 1300, and then this chain's games) and the chain (`strict.jsonl`) differed in some games, and other chains
differ from the file in others.  These are all of them:

* **539 and 770–773**, which the prover lists with two or three optimal sets each ([the prover's
  README](../solver/noframe/docs/uniqueness.md) has the exchanges).  The file has the chain's set.  The table had the
  Java's set in each, as did the earlier chains of commits 243af9b and 271c817.
* **684–687**, where those two earlier chains had another optimal set than the table's, which the file has.
* **1215**, where the table and every earlier chain have {312, 450} and the file {342, 420}: an exchange of two numbers
  of the same sum, 762.  The prover enumerates optimal sets only to n = 989.
* **1221–1224**, where the chain of commit bd9bb5d and the cost-curve study's chain found {364, 456, 544}, and the file,
  `default.jsonl`, the table and the chains of 243af9b and 271c817 have {348, 408, 608}.  It is an exchange of three
  numbers of the same sum, 1364, that starts at 1221 and is carried by 1222, 1223 and 1224.  The prover describes
  exchanges like it, which first appear at game 425.
* **1560–1563**, where the GF(2) study's chain has {510, 594, 620, 624, 660} and the file {476, 528, 616, 644, 744}: an
  exchange of five numbers of the same sum, 3008, found by the search of 1560 and carried by the three repeats after it.

**The games 1–1300 in history.**  The games 1–1000 of the file reproduce the Java solver's table score for score, with no
unproven tactic on, in 7.5 s of search in all on 8 threads, the slowest n = 992 at 0.7 s.  The Java solver is at the git
tag `java-final`, and its table, whose games it made with the tactics `reuse_prev` and `revise_freedoms` on, was the
file `results/optimal.json` until the table and the chain became one file.  That table's games 1001–1300 came from the
chain of commit bd9bb5d and then from that of commit 243af9b, which differs from the first at 1221–1224 only, added by
`taxman extend-optimal` (a command since removed).  Its games 1301 on were the chain's own, move for move, copied in the
same way on 2026-10-28 (1301–5000) and 2026-10-29 (5001–5053).

## What supports the scores

"Optimal" here means optimal within the frame model, given that the solver is right.  That is opt(n) wherever [statement
U](../docs/theory/exactness.md#statement-u) holds
([DESIGN.md](../solver/docs/design/frame-model.md#what-optimal-means)), and ["The files"](#the-files), above, says how
far each kind of proof reaches.

Two verifiers check the file, as [above](#optimaljsonlgz) shows.  The first is the frame solver's own.  It also runs on
any results file that `solve --out` opens, and in `tests/results.rs`.  The second is an independent verifier in Python,
which shares no code with the solver; it also checks each score against the Franklín–Moniot upper bound and the
best-known heuristic chain where `heuristics/results` has them.  Both replay every game (legal, scoring what is
recorded, with no legal move left, and opening with the largest prime), confirm each score against the table and the
bounds score(n − 1) ≤ score(n) ≤ n + score(n − 1), check each line's arithmetic and the predecessor it names and that
the files agree, and derive each line's proof kind and what it assumes by the rules in `src/results.rs`.  [The frame
solver's reference](../solver/docs/REFERENCE.md#4-why-the-answers-can-be-trusted) defines the proof kinds and the
assumptions, and says what each verifier checks.  Both pass `optimal.jsonl.gz`.  Checked against itself, the file's
comparison with the table is each game with itself, so it has nothing to do there.

What that evidence amounts to:

1. **Arithmetic certificates.**  A score at the [ceiling](../docs/GLOSSARY.md#the-game), n + score(n − 1), is optimal
   whenever the game of n − 1 is, whatever found it.  So is a score equal to n + score(n − 1) − P for a prime n, with P
   the largest prime below n (the [prime formula](../docs/GLOSSARY.md#the-game), proven in `src/accelerate.rs`).  2315
   of the file's games have such a score: 1639 at n + score(n − 1), among them its 444 proofs at the ceiling, and
   676 prime ones.
2. **Exhaustive searches.**  The file's 1612 `exhausted` games are proven by a search that explored its whole
   tree, as the solver reports it.  The rest of the file is its 444 proofs at the ceiling, its 634 prime games, its 13
   trivial ones (no promotion candidate, so there is only one game), its 1975
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
     1301–5060 is the chain's: 1196 exhausted searches of their own, 329 proofs at the ceiling, 423 prime ones and 1430
     repeats that rest on the games before them, and 382 games of the store.
4. **Agreement between builds and files.**
   * n ≤ 1000: the chain's scores are the table's, all 1000, where the table was the Java solver's `optimal.json` when
     the run was made (each line's `optimal_match` is true).  797 of the chain's games are that table's move for move,
     and 995 are its sets of moves: all but 539 and 770–773 (above).
   * n = 1001–1300: that table held the games of the chain of commit 243af9b, copied move for move by `taxman
     extend-optimal`, and the chain's games are those, move for move, but 1215 (above).  The lines say `optimal_match`
     true here too.  When the run was made the table held those games, so this is a second run reproducing the first's
     scores, not a comparison with another source.
   * n = 1301–2175: the table had no game here when the run was made (the lines say `optimal_match` null; the file holds
     these games from 2026-10-28), so the comparison is with another run of the same search.  The GF(2) study's chain
     was made from the table's game of 1300 (`--use-prev`) by the build of commit 716ebd9, on 2 threads in three
     invocations, and reached 2175 from 1301 in 92 minutes of game time.  Its 875 games have this file's scores and
     proofs in every game, and its moves in all but 1560–1563 (above).  Its methods are this file's too, except that a
     game whose search took no step was labeled `search` then and is `start` here.  Only its game of 1301 assumes
     anything: that the table's game of 1300 is optimal.
     `taxman verify-results results/optimal.jsonl.gz solver/notebook/search/the-gf2-bound-in-the-search-2026-10/chain.jsonl`
     (93 s) accepts them and finds 875 games in both files, with equal scores in all.  `solver/tools/check_results.py`
     finds the same, and refuses the label `search` of the games with no node.
   * n = 2176–3440: the table had no game here either, and no other chain reaches far.
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
   * n = 4875–5060: no other chain reaches here either.  The evidence is the sixth run's own (runs.md),
     which took games 1–4874 from this file and did not solve them again, and the verifiers pass the whole file.
   * 1001–1231: the scores of this file, the cost-curve study's chain (commit e710d16, 8 threads, a chain with the
     unproven tactics off) and `default.jsonl` (the earlier solver, with unproven tactics on) are equal in all 231
     games.  This file's games are `default.jsonl`'s move for move but 1215, and the cost-curve chain's but 1215
     and 1221–1224: other games, with the same scores.
   * The earlier committed chains, of commits bd9bb5d, 243af9b and 271c817, have the same scores in all 1300 games.  The
     second's and the third's moves are the same as each other's, and differ from this file's at 539, 684–687, 770–773
     and 1215; the first's differ at those and at 1221–1224.
5. **Independence from the table.**  The chain starts at n = 1, so no game of it is built from the table (the
   old `optimal.json`), which was only compared with, up to 1300, where the table ended then; a score that differed would
   have been a hard error.
6. **The prover's proofs.**  The prover proves every n up to 2175 optimal without the frame model, with no frames or
   promotions ([the prover's README](../solver/noframe/README.md); [the parity bound
   (W1)](../docs/theory/parity-bound.md#the-parity-bound)).  So
   the file's scores to 2175 do not rest on the frame model either: those 2175 are opt(n) on the whole board.  The
   games 1301–2175 were proven in the cloud run of 2026-10-14 and 15 ([the
   notebook](../solver/notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)).  The 2885
   games of 2176–5060 are proven within the frame model, which is exact given U ([the exactness theorem
   (X7)](../docs/theory/exactness.md#the-exactness-theorem)).  `taxman certify` can check U there game by game, with
   the prover's barred bounds ([the prover's README](../solver/noframe/docs/proofs.md#the-upper-part-game-by-game) has
   them to 3440).

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal, when the game of n
− 1 is not optimal either.  That is what the proof kinds and the assumptions track, and why the chain from n = 1 and
the standalone solves (3) matter.

## bounds.jsonl

The bounds store: the bounds on opt(n) that [the
ledger](../solver/notebook/search/the-ledger-bounds-from-both-sides-2026-10/README.md) finds, kept for good, for the
games from 3915 on.  A game is *proven* when its best lower bound equals its best upper bound, and the file is the
record of those proofs, which the lines of proof `known` in `optimal.jsonl.gz` cite.  The first `ledger export` made it,
from stage 1 of the ledger past 4400 (2026-10-18), and each export since appends to it.

Its lines are bounds that hold.  A lower bound is the score of a finished game that the referee replays.  An upper
bound is the frame model's, carried between games as [the
theory](../docs/theory/search-rules.md#bounds-carried-between-games) says.

A line is a bound, not a game of `taxman solve`.  It is not in the schema of the results lines, and an export adds
no line of a game that `optimal.jsonl.gz` has.  It is not `explore.jsonl`'s kind of file either: that one holds the
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
  `results/optimal.jsonl.gz` does not have (`--strict` names another file).  It creates the file if there is none, and
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
  as the removed example `wall` made it; nothing makes it any more;
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
against the chain's 7.3 (36.8 megabytes for 5,060 games uncompressed; the file of 5,060 games is 1.10 megabytes).

## certificates

`certificates/` holds the committed certificates of U that `taxman certify` made
([solver/docs/design/certificates.md](../solver/docs/design/certificates.md) says what a certificate is and what it
proves).

`certificates_600_1300.jsonl` has a certificate for each of the 377 games of 600..1300 that need one: 296 certified by
a search, and 81 by posing the same problem as an earlier game.  `taxman verify-certificates` passes it, in 4.6 s.  It
was made by `taxman certify 600-1300 --results results/optimal.jsonl.gz U --out
results/certificates/certificates_600_1300.jsonl --threads 1 --progress 0`, in 39 s, on the code of commit 35c614b.
`U` there stands for the four `--upper` files, the prover's barred bounds for 2..1300, 1301..2175, 2176..3314 and
3315..3440 (`solver/noframe/data/upper_*.jsonl`).

The certificates are in a directory of their own because `tests/results.rs` and `solver/tools/check_results.py` take
every `results/*.jsonl` for a chain or the bounds store.  `tests/certify.rs` checks the work list's prediction against
the file: the games that need a certificate, their open and rejected sources, and the games that the rule of the same
problem settles ([the work list of 2176-3440](../solver/tools/certify/README.md)).
