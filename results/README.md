# Results of the solver

This directory holds the project's results: the chain of optimal moves, the proven games past its end, the best-known
games past that and what has been tried on them, and the chain's scores in the OEIS's format.  This page says what each
file holds, how far it reaches, how it was made and what supports its scores.  The [glossary](../docs/GLOSSARY.md)
defines the terms; [the frame model](../docs/theory/frame-model.md#the-frame-model) and statement U, on which the frame
solver's scores rest, are defined in the theory, and the prover in [its README](../solver/noframe/README.md).

A file of proven games has one JSON line per game, as `taxman solve --out` writes it.  The fields are described in
`src/results.rs` and in [the frame solver's reference](../solver/docs/REFERENCE.md#a-results-line---out-schema-5).  The
two files about the best-known games have lines of their own kind ([below](#best-knownjsonlgz)).  The counts on this
page are a snapshot, dated 2026-11-06 (2026-11-13 for the best-known games), which no test pins: the commands in the
section of each file print the current ones.

## The files

| file | holds | games | made by |
|---|---|---|---|
| `optimal.jsonl.gz` | the committed chain: the moves of an optimal game for every n, each found from the moves for n − 1; the table of moves built into the frame solver | 1–5060 on 2026-11-06 | one run of `taxman solve` ([below](#optimaljsonlgz)) |
| `beyond-the-chain.jsonl` | the proven games past the end of the chain, in blocks that each start with a standalone game | past the end of the chain | `taxman solve`, a standalone game and the games after it ([below](#beyond-the-chainjsonl)) |
| `best-known.jsonl.gz` | the best-known game of each game past the chain, none of them proven, with the parity bound, the step and the rule that settles it from the game before | 5061–10000 but the proven games, on 2026-11-13 | `solver/tools/best_known_table.py` ([below](#best-knownjsonlgz)) |
| `search-effort.jsonl` | what has been tried on those games: the dedicated plays and the passes of the finder | the same games | the same tool ([below](#search-effortjsonl)) |
| `b019312.txt` | the scores of `optimal.jsonl.gz`, as a b-file of [A019312](https://oeis.org/A019312) in the OEIS | 1–5060 | the command [below](#b019312txt) |

How far each proof kind reaches:

* **Every line of `optimal.jsonl.gz`** is proven within the frame model.
* **Games 2 to 2531** are also proven on the whole board by the prover, so their scores are opt(n) without the frame
  model.
* **Games 2532 on** are proven within the frame model, which is exact given statement U.  U is not proven past 2531.
  A line of them proven by the ceiling or the prime formula is opt(n)
  wherever the game before it is.
* **Every line of `beyond-the-chain.jsonl`** is proven within the frame model too, so it holds given U as the games 2532
  on do, and a line that is not standalone rests on a line of that file ([below](#beyond-the-chainjsonl)).
* **Every line of `best-known.jsonl.gz`** is a game that was played, so its score is a lower bound on opt(n) and no
  more.  Where the line has a `rule`, its game is optimal if the game that the rule rests on, the one before it or the
  one it repeats, is ([below](#best-knownjsonlgz)).

"What supports the scores", below, gives the evidence for each.

**A file that is not a chain from game 1 never goes into `optimal.jsonl.gz`.**  The frame solver builds that file in and
reads its line n as game n, so it can hold no gap, and a game past a gap rests on no game of it.  The proven games past
the end of the chain go into [`beyond-the-chain.jsonl`](#beyond-the-chainjsonl).  The line of an unfinished search (a
lower bound, nothing proven) goes into no file of proven lines: the best-known games, which are lower bounds, are in
[`best-known.jsonl.gz`](#best-knownjsonlgz), as games with their scores and moves and not as lines of a search.  The
verifiers read files of every kind ([verifiers.md](../solver/docs/verifiers.md)).

## optimal.jsonl.gz

The chain of moves from n = 1, the file to build on, and the table of moves that the frame solver has built in.  Every
line is found from the moves for n − 1, every line has its full provenance, and no line rests on a table.
It has one line a game in schema 5, the completed lines in order of n (so line n is game n), and no line of an
unfinished search.  [runs.md](runs.md) records the run that made it.  On 2026-11-06 it replaced a file that six runs had
made, and runs.md records those too.

**The file is gzip, one member for each batch of lines.**  The first 500 lines are one member, the next 500 the next,
and so on; a batch of lines brought home later is appended as one member of its own (`solver/docs/runbook/chain.md`,
["Verifying, and bringing the results home"](../solver/docs/runbook/chain.md#verifying-and-bringing-the-results-home)),
so the bytes of the lines before it never change.  Both Rust programs read it through the crate
[solver/gzip](../solver/gzip/README.md), which checks each member's CRC-32 and so fails loudly on a damaged file;
Python and the shell read it with `gzip`.  The frame solver builds the file into its binary.  Its table is the lines of
the file as of the build, each decoded and parsed when it is asked for, member by member: a run that needs none decodes
nothing, and one that needs the first lines decodes the first members ([the design](../solver/docs/design/engine.md)).
The table holds each line's score and moves and not how its game was proven.  A chain that assumes nothing beyond the
frame model starts at n = 1, as this one does.

To read it, from the top of the repository:

    gzip -dc results/optimal.jsonl.gz | sed -n 5000p      # the line of game 5000
    taxman verify-results results/optimal.jsonl.gz
    python3 solver/tools/check_results.py results/optimal.jsonl.gz

The last two are the verifiers, which print the counts below as they stand.

**The snapshot of 2026-11-06: games 1–5060** in 11 members (ten of 500 games and one of 60), 1,306,219 bytes, which
decompress to 38,188,918 bytes.  It is the file of the run of 2026-11-01 ([runs.md](runs.md)), one build, commit
aaba59b.  Its games, by proof kind, as the verifiers report them:

| | 1–1300 | 1301–2175 | 2176–3440 | 3441–4000 | 4001–4500 | 4501–5060 | all |
|---|---|---|---|---|---|---|---|
| games | 1300 | 875 | 1265 | 560 | 500 | 560 | 5060 |
| proof `trivial` | 13 | 0 | 0 | 0 | 0 | 0 | 13 |
| proof `exhausted`: the search explored its whole tree | 416 | 314 | 448 | 208 | 165 | 194 | 1745 |
| of those, searches the [near-repeat cut](../solver/docs/design/search.md#the-near-repeat-cut) shortened (`stats.near_repeat_cut` above 0) | 13 | 30 | 44 | 25 | 19 | 30 | 161 |
| proof `cap`: the search reached the [ceiling](../docs/GLOSSARY.md#the-game) | 115 | 77 | 119 | 51 | 65 | 67 | 494 |
| proof `prime_formula` | 211 | 115 | 155 | 69 | 60 | 67 | 677 |
| proof `same_search`: a repeat of an earlier game's search | 545 | 369 | 543 | 232 | 210 | 232 | 2131 |
| score = n + score(n − 1) | 424 | 270 | 402 | 176 | 175 | 192 | 1639 |
| score = n + score(n − 1) − P (a prime n) | 210 | 115 | 155 | 69 | 60 | 67 | 676 |

The near-repeat cut shortens a search and leaves its proof kind `exhausted`, so a line counts the nodes it cut in
`stats.near_repeat_cut`.  2,151 lines have the key, the searches that were armed with the cut, and 161 of them cut
nodes.

Of its 2239 searched games, 1745 are exhausted and 494 at the ceiling.  The games proven at the ceiling were all decided
before the search explored a node (the method `start`) but four: 686's took 16 nodes, 812's 30, 1564's 63,308 and
3933's 1,211.  A repeat takes the promotions of the latest earlier game that posed the same search problem: the moves
for n − 1 for 1898 of the 2131 repeats, and of n − 2 for the other 233, where n − 1 is prime.

**Most games that need no search can be told from n alone.**  Write n = c · P with P the largest prime factor of n, and
call c the cofactor.  A game whose cofactor is 1, a prime, twice a prime or 9 was settled with no search: a prime by the
prime formula, nearly all the others as repeats, and the rest at the start.  Over games 1301–4979, the chain's games past
4949 included, this rule names about 85% of the games settled with no search, and none of the games it names has the
method `search`.  [The lab notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/search/which-games-need-no-search-the-cofactor-of-n-2026-10/README.md)
has the tables and the cofactors that are always searched.

*The proof kinds by range, in detail:*

* 1–1000: 311 exhausted, 92 at the ceiling, 168 prime, 13 trivial and 416 repeats.
* 1001–1231: 81 exhausted, 16 at the ceiling, 34 prime and 100 repeats.
* 1232–1300: 24 exhausted, 7 at the ceiling, 9 prime and 29 repeats.  Its 69 games are 31 searches, 29 repeats and the
  primes 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291 and 1297.
* 1301–2175: 314 exhausted, 77 at the ceiling, 115 prime and 369 repeats: 391 searches, 369 repeats and 115 primes.
* 2176–3440: 448 exhausted, 119 at the ceiling, 155 prime and 543 repeats: 567 searches, 543 repeats and 155 primes.
* 3441–4000: 208 exhausted, 51 at the ceiling, 69 prime and 232 repeats: 259 searches, 232 repeats and 69 primes.
* 4001–4500: 165 exhausted, 65 at the ceiling, 60 prime and 210 repeats: 230 searches, 210 repeats and 60 primes.
* 4501–5060: 194 exhausted, 67 at the ceiling, 67 prime and 232 repeats: 261 searches, 232 repeats and 67 primes.

### Games with several optimal sets

Several games have more than one optimal set of moves, and which one a run finds depends on the order of its search.
So scores are reproducible, but move lists need not be.  Each chain that found a set proved its game optimal, so each
set is optimal.  The file has the sets of the run that made it, from n = 1.  The earlier file, which this one replaced
on 2026-11-06 (`git show d05191a:results/optimal.jsonl.gz`), has the same scores and, in 54 games, other sets.  Other
chains differ from the file in other games.  These are all of them:

* **539, 684–687 and 770–773**, where the earlier chains of commits 243af9b and 271c817 have other sets than the
  file's, and so had the earlier file's.  The prover lists 539 and 770–773 with two or three optimal sets each ([the
  prover's README](../solver/noframe/docs/uniqueness.md) has the exchanges).
* **1056–1063**, where the file has {256, 336, 480} and the earlier file and chains {240, 384, 448}: an exchange of
  three numbers of the same sum, 1072.
* **1215**, where the file and the earlier chains have {312, 450} and the earlier file {342, 420}: an exchange of two
  numbers of the same sum, 762.  The prover enumerates optimal sets only to n = 989.
* **1221–1224**, where the chain of commit bd9bb5d and the cost-curve study's chain found {364, 456, 544}, and the file,
  the earlier file, `default.jsonl` and the chains of 243af9b and 271c817 have {348, 408, 608}.  It is an exchange of
  three numbers of the same sum, 1364, that starts at 1221 and is carried by 1222, 1223 and 1224.  The prover
  describes exchanges like it, which first appear at game 425.
* **1560–1563, 1566 and 1567**, where the file has {510, 594, 620, 624, 660} and the earlier file {476, 528, 616, 644,
  744}: an exchange of five numbers of the same sum, 3008, found by the search of 1560 and carried by the three repeats
  after it, and again by 1566 and its prime 1567.  The GF(2) study's chain has the file's set in 1560–1563 and the
  earlier file's in 1566 and 1567.
* **3988–4756, 39 games** that the earlier file took from [the ledger's bounds
  store](https://github.com/bvchess/taxman-notebook/blob/main/search/the-ledger-bounds-from-both-sides-2026-10/committed-store/bounds.jsonl),
  with no search: the file's searches found other optimal sets than the store's moves: 3988, 3994, 4012, 4054, 4082,
  4090, 4323, 4330, 4348, 4362, 4366, 4466, 4472, 4474, 4478, 4486, 4492, 4506, 4510, 4516, 4526, 4558, 4566, 4582,
  4586, 4588, 4612, 4618, 4636, 4654, 4666, 4670, 4674, 4685, 4694, 4708, 4726, 4754 and 4756.

The lines for 1–1000 reproduce the Java solver's table score for score, as every run of the chain has.  The Java solver
is at the git tag `java-final`.

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
   whenever the moves for n − 1 are, whatever found it.  So is a score equal to n + score(n − 1) − P for a prime n, with
   P the largest prime below n (the [prime formula](../docs/GLOSSARY.md#the-game), proven in `src/accelerate.rs`).  2315
   of the file's lines have such a score: 1639 at n + score(n − 1), among them its 494 games proven at the ceiling, and
   676 prime ones.
2. **Exhaustive searches.**  The file's 1745 `exhausted` games are proven by a search that explored its whole tree, as
   the solver reports it.  The rest of the file is its 494 games proven at the ceiling, its 677 prime games, its 13
   trivial ones (no promotion candidate, so there is only one game) and its 2131
   [repeats](../docs/GLOSSARY.md#results-and-trust), which both verifiers check against the earlier game
   (`src/results.rs`).
3. **Standalone solves.**  Some games were solved again on their own, with no seed and no ceiling:
   * [taxman-notebook/search/benchmarks-the-probe-pgo-and-scaling/calib.jsonl](https://github.com/bvchess/taxman-notebook/blob/main/search/benchmarks-the-probe-pgo-and-scaling/calib.jsonl)
     holds 197 standalone searches, one for every game of 1001–1231 that is not prime (the example `ladder`, which
     builds at commit 3ca11bf: `ladder --no-seed --no-cap --tactic revise_freedoms=off`, 8 threads, commit e710d16; see
     [`taxman-notebook/search/benchmarks-the-probe-pgo-and-scaling/cost-curve.md`](https://github.com/bvchess/taxman-notebook/blob/main/search/benchmarks-the-probe-pgo-and-scaling/cost-curve.md)).
     All of them finished, and each found the promotion sum of the score recorded here.  These solves rest on no other
     game.  So every game of 1001–1231 that is not prime is proven without the chain, and every prime one by the prime
     formula from a game so proven.
   * There is no such solve for 1232–1300.  Of those 69 games, 24 are proven by an exhausted search of their own, and
     the 7 games proven at the ceiling, the 9 prime ones and the 29 repeats rest on the game before them or on the game
     they repeat, so on the chain from n = 1 (see 5).
   * Past 1300,
     [taxman-notebook/search/benchmarks-the-probe-pgo-and-scaling/bands.jsonl](https://github.com/bvchess/taxman-notebook/blob/main/search/benchmarks-the-probe-pgo-and-scaling/bands.jsonl)
     has ten games of 1302–1352 solved standalone (the matching path, 8 threads), each finishing with the promotion sum
     recorded here.  The rest of 1301–5060 is the chain's: 1329 exhausted searches of their own, 379 games proven at the
     ceiling, 466 prime ones and 1586 repeats that rest on the games before them.
4. **Agreement between runs and files.**
   * n = 1–5060: the run compared each score with the table built into its binary, the earlier committed file (each
     line's `optimal_match` is true), and a score that differed would have been a hard error.  So all 5060 scores are
     the earlier file's, which six runs on twenty builds had made ([runs.md](runs.md)); its moves are the file's in all
     but 54 games, with the same scores ("Games with several optimal sets", above).
   * n = 1301–2175: the GF(2) study's chain was made from the moves for 1300 by the build of commit 716ebd9, on 2
     threads in three invocations, and reached 2175 from 1301 in 92 minutes of game time.  Its 875 games, compared line
     by line, have this file's scores and proof kinds in every game, and its moves in all but 1566 and 1567 (above).
     Its methods are this file's too, except that a game whose search took no step was labeled `search` then and is
     `start` here.
   * n = 2176–3440: no other chain reaches far.  `solver/frame/testdata/c8g.chain-2176.jsonl`, the Graviton
     calibration's chain (64 threads, one game after another, the build of commit 407645f, 15 minutes), has the
     lines for 2176–2276, 101 in all, with this file's scores, proof kinds, promotions and moves in every game, and its
     methods but for that label.  A rehearsal of the earlier run to 3440, made on the same machine type just before it
     started (the build of commit 55287ff, 64 threads, a window of 16, stopped and killed and resumed on purpose; not
     committed), has 2176–2300, 125 in all, and agreed with the earlier file, whose lines for them are this file's.
     Past 2300 the evidence is the run's own: the verifiers on the whole file.
   * n = 3441–5060: no other chain reaches here.  The evidence is the run's own, one build from game 1 to game 5060,
     the scores equal to the earlier file's, and the verifiers on the whole file.
   * 1001–1231: the scores of this file, the cost-curve study's chain (commit e710d16, 8 threads, a chain with the
     unproven tactics off) and `default.jsonl` (the earlier solver, with unproven tactics on) are equal in all 231
     games.  This file's moves are `default.jsonl`'s move for move but 1056–1063, and the cost-curve chain's but
     1056–1063 and 1221–1224: other moves, with the same scores.
   * The earlier committed chains, of commits bd9bb5d, 243af9b and 271c817, have the same scores in all 1300 games.  The
     second's and the third's moves are the same as each other's, and differ from this file's at 539, 684–687, 770–773
     and 1056–1063; the first's differ at those and at 1221–1224.
5. **Independence from the table.**  The chain starts at n = 1, so no line of it is built from the table, which was
   only compared with; a score that differed would have been a hard error.
6. **The prover's proofs.**  The prover proves every n up to 2531 optimal without the frame model, with no frames or
   promotions ([the prover's README](../solver/noframe/README.md); [the parity bound
   (W1)](../docs/theory/parity-bound.md#the-parity-bound)).  So
   the file's scores to 2531 do not rest on the frame model either: those 2531 are opt(n) on the whole board.  The
   games 2..2530 were proven in one run of one build, on 2026-11-06 and 07, and game 2531 afterward, by the prime
   step ([the
   notebook](https://github.com/bvchess/taxman-notebook/blob/main/the-model-free-proofs-of-2-2530-on-one-machine-2026-11/README.md)),
   and its file, `solver/noframe/data/exact_parity_2_2531.jsonl`, is committed
   ([the prover's proofs](../solver/noframe/docs/proofs.md#the-run-of-2-to-2530)).
   The 2529 games of 2532–5060 are proven within the frame model, which is exact given U ([the exactness theorem
   (X7)](../docs/theory/exactness.md#the-exactness-theorem)).

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal, when the moves for n
− 1 are not optimal either.  That is what the proof kinds and the assumptions track, and why the chain from n = 1 and
the standalone solves (3) matter.

## beyond-the-chain.jsonl

**The proven games past the end of the chain**, which `optimal.jsonl.gz` cannot hold because a gap of games parts them
from it.  The file has one line a game in schema 5, no line of an unfinished search, and blocks: a block starts with a
game solved [standalone](../docs/GLOSSARY.md#results-and-trust) and goes on with the games after it, each settled from
the game before it.

**Each line is proven, and says what it rests on.**  A standalone line, the first of a block, is proven by a search
that started from no earlier game, so it rests on none.  A line settled by the [prime
formula](../docs/GLOSSARY.md#the-game), a [repeat](../docs/GLOSSARY.md#results-and-trust) or the
[ceiling](../docs/GLOSSARY.md#the-game) rests on the line in this file of the game before it, or for a repeat of the
game it repeats.  All are proven within the frame model, and so rest on [statement
U](../docs/theory/exactness.md#statement-u) as the chain's games past 2531 do.

**How it was made.**  On 2026-11-10, on one clean build (commit aeffc29), the game 5260 was solved standalone, with no
seed, no ceiling and no near-repeat cut (`--force-search --tactic prev_cap=off --tactic near_repeat=off`).  The search
was stopped at its budget and resumed from its checkpoint on 88 threads, and exhausted its tree after 78.5 CPU-hours.
The games after it, 5261 (the prime formula), 5262 and 5263 (repeats) and 5264 (the ceiling), were solved from it in
chain mode with every tactic at its default.  [The lab notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/the-first-proofs-past-5060-2026-11/README.md) has the full
record.

To check it, from the top of the repository:

    taxman verify-results results/beyond-the-chain.jsonl
    python3 solver/tools/check_results.py results/beyond-the-chain.jsonl

Both accept the file and print its games and proof kinds as they stand, and `tests/results.rs` runs both on it.

**The rules for adding lines:**

* Only completed lines that both verifiers accept go in.
* A game that `optimal.jsonl.gz` holds never goes in.
* Each game goes in once.
* A new block starts with a standalone line.

## best-known.jsonl.gz

**The best-known game of each game past the end of the chain, with what the proved results say of it.**  A best-known
game is the game with the highest score that a run of the unsound mode has found for n ([the best-known
file](../solver/docs/unsound/best-known.md#what-the-best-known-file-is)).  Its score is a lower bound on opt(n) that
nothing proves, so the file says "best-known" and never "optimal".  It has a line for each game from the one after the
chain's end to the last that the runs reach, in order of n, but for the games that
[`beyond-the-chain.jsonl`](#beyond-the-chainjsonl) proves: those are left out, so that each fact has one home, and the
file has a gap at each.

**A line has six fields**, `moves` last, so that a reader can skip it:

| field | holds |
|---|---|
| `n` | the game |
| `score` | the best-known score: the sum of the moves |
| `parity_bound` | the game's [parity bound](../docs/theory/parity-bound.md#the-parity-bound), an upper bound on opt(n) |
| `step` | how the parity bound moves from the game before to this one: `exchange`, `added`, `half-in` or `half-out` ([below](#the-step)) |
| `rule` | the proved rule that gives this score from the game before's, or `null` ([below](#the-rule)) |
| `moves` | the best-known game: a legal game that is over and opens with the largest prime |

    gzip -dc results/best-known.jsonl.gz | sed -n 1p | cut -c1-100
    {"n":5061,"score":8057936,"parity_bound":8060985,"step":"added","rule":null,"moves":[5059,5041,4757,

**What arithmetic gives is not stored.**  The parity gap is `parity_bound` less `score`.  The ceiling is the score of the
game before plus n ([the ceiling lemma](../docs/theory/neighbors.md#the-ceiling-lemma)), that score being the proven one
where the game before is proven (the chain's last game, and those of `beyond-the-chain.jsonl`); the shortfall is the
ceiling less `score`, and a game at its ceiling has 0.  Three fields are stored because they cannot be had from the rest:
`parity_bound` is the prover's and lies in the notebook, not in this repository, and `step` and `rule` cost real
computation: a GF(2) basis of each board's sources, and for `rule` the forest of layer 2, a basis of layer 4 and the root
problem of each game.

### The step

The step says how the parity bound moves from the game before ([the parity steps
(W4)](../docs/theory/parity-bound.md#the-parity-steps)).  At an odd n it is `exchange` if n takes the place of a
[source](../docs/GLOSSARY.md#the-frame-model) that the game before keeps, as every prime does (it replaces the prime
before it), and `added` if n joins them.  At an even n it is `half-out` if n/2 is not among those sources, and
`half-in` if it is, and leaves when n joins.  The tool reads it from the change in the sum of the sources that descending
greedy keeps, which the prover recorded with each board's parity bound.

### The rule

The rule names a proved relation between this game and the game before that the best-known score meets exactly.  Where it
is set, the best-known game is optimal if the best-known game it rests on is, the game before or, for `same_search`, the
game it repeats; so a run of games with rules is as optimal as the game that starts it.  Where it is `null`, no proved
relation connects the score to the game before's, and a short game can start there with the game before it not short.

| `rule` | the relation, and where it applies | the score is | rests on U |
|---|---|---|---|
| `prime_formula` | [the prime formula](../docs/theory/neighbors.md#the-prime-formula), n prime | the score before + n − q, q the prime before n | no |
| `cap` | [the ceiling lemma](../docs/theory/neighbors.md#the-ceiling-lemma), n composite | the score before + n | no |
| `theorem_8p` | [the 8p theorem](../docs/theory/below-the-ceiling.md#the-8p-theorem), n = 8p ≥ 187 | the score before + n − 4r | no |
| `semiprime_formula` | [the semiprime formula](../docs/theory/arrival.md#the-semiprime-formula), n odd with two prime factors, n ≥ 189 | the score before + n − x | no |
| `theorem_12p` | [the 12p theorem](../docs/theory/below-the-ceiling.md#the-12p-theorem), n = 12p ≥ 187 | the score before + n − 6r | yes, at n |
| `same_search` | [the repeat lemma](../docs/theory/search-rules.md#the-repeat-lemma), n poses the root problem of the game before it (of the one before that if the game before is prime) | that game's promotion sum | yes, at n |

The theorems define p, r, x and the hypotheses that the tool checks.  [Statement U](../docs/theory/exactness.md#statement-u)
is the 12p theorem's hypothesis and the repeat lemma's third part.  Where several rules hold, the file names the first
row, which puts the rules that need no U first.  At a half-out step the [half lemma](../docs/theory/neighbors.md#the-half-lemma)
puts opt(n) at the ceiling given U at the game before, so a half-out game at its ceiling meets it and `cap`, and the file
names `cap`.

### What the lines rest on, and how they were made

`score` and `moves` rest on the runs that found them, and a test replays every game by the rules.  `parity_bound` is
proven.  `step` follows from the prover's sums, and `rule` from a theorem and the hypotheses that the tool checks; a test
checks the rules' arithmetic against the scores.

The tool `solver/tools/best_known_table.py` reads a copy of the best-known file, taken once so that a second run reads
the same bytes, and keeps each game's line of the highest score (the later of two).  The parity bounds and sums of the
sources are those of the prover's example `paritysplit` for the boards 2 to 10000 ([the notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/search/the-parity-bound-against-the-frame-bound-2026-10/README.md#the-provers-side-to-10000-2026-11-09)).
It stops and writes nothing when a source does not fit: moves that do not make a game of their board with its score, a
game that does not open with the largest prime, a score above its bound, a bound that is not the prover's, a step that is
none of the four, or a game or board with no line.

**The snapshot of 2026-11-13**, from a copy taken at 18:34 PST of 7,190 lines of 4,940 games: 4,935 games, 5061 to 10000
but 5260 to 5264, in ten gzip members of at most 500 lines, 1,249,156 bytes, decompressing to 80,806,017.  By step, 1,708
are `exchange` (551 of them primes), 760 `added`, 1,350 `half-in` and 1,117 `half-out`.  By rule, 551 are `prime_formula`,
1,701 `cap`, 89 `theorem_8p`, 947 `semiprime_formula`, 63 `theorem_12p`, 26 `same_search` and 1,558 have none.

To check the file, `tests/results.rs` replays the games (the quick check the first 100) and checks their order, parity
bounds, steps and rules.  To refresh it, copy the best-known file and the lab's ledgers (`rootfirst/ledger.tsv`,
`rootfirst/ledger2.tsv`, `hammer/ledger.tsv`, `transplant/driver/ledger.tsv`, `transplant/driver/ledger-b23.tsv`) into
`COPY`, run the tool with the notebook entry's parity files as `PARITY1` and `PARITY2`, and name the time of the
copies in the commit message:

    python3 -I solver/tools/best_known_table.py --best-known COPY/best-known.jsonl --parity PARITY1 PARITY2 \
        --plays COPY/ledger.tsv:3 --plays COPY/ledger2.tsv --plays COPY/hammer-ledger.tsv \
        --finder COPY/finder-ledger.tsv:60 --finder COPY/ledger-b23.tsv:600

## search-effort.jsonl

**What has been tried on each best-known game**: its dedicated plays and the passes of the finder.  It backs the claim of
how many best-known games are short ([the notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/how-many-best-known-games-past-5060-are-short-2026-11/README.md)),
since the chance that a fresh play raises a short game depends on the plays the game has had.  It has a line for each game
of `best-known.jsonl.gz` that has a play or a pass, in order of n; a game that nothing was tried on has none, and a play
or a pass on a proven game is left out with the game.

**A line has three fields:** `n`; `plays`, a `[level, seed, raised]` for each play, sorted by level and seed; and
`finder`, a `[B, seconds, outcome]` for each pass of the finder, sorted by B.

    {"n":6615,"plays":[[3,41,0],[3,42,0],[3,43,0],[3,44,0],[4,41,0],[4,42,0],[5,41,0],[5,42,0],[5,43,0]],"finder":[[13,60,"none"],[19,60,"stopped"]]}

A *play* is one run of the unsound mode on game n from its best-known line, at a [level of
effort](../solver/docs/unsound/play.md#the-level-of-effort) and with a seed that sets its random choices; `raised` is 1
if it reached a score above the game's best-known score when it began.  A play made twice at one level and seed appears
twice.  A *pass* is an exact search of game n for a better game, with the numbers up to n/2 whose prime factors are all
at most B free and every other number up to n/2 held as the best-known game has it.  `seconds` is the time limit of each
search of the pass, and `outcome` is `none` if every search finished, so that no better game has the rest so held,
`stopped` if a limit ended a search and none found a better game, and `gain` if one was found.  The file leaves out the
ledgers' times of day, CPU times and scores.

**The snapshot of 2026-11-13**, from copies of the ledgers taken at 18:34 PST: 4,935 games, 5,280 plays and 10,110
passes.  The plays are 53 at level 3, 4,628 at level 4 and 599 at level 5, on 1,620 games, and 58 raised their game.  The
passes are at B = 13 (4,806 `none`, 129 `stopped`) and B = 19 (569 `none`, 4,366 `stopped`), one of each for every game,
all with a limit of 60 seconds, and at B = 23 (240 `stopped`), on 240 games between 9243 and 10000, with a limit of 600
seconds.  The pass at B = 23 was stopped by hand; the searches it had under way are rows of its ledger with the status
`error`, which are no pass, and the tool leaves them out.  None found a better game.  The finders' ledgers do not record
the limit, nor the pilot's ledger (level 3) its level, so the tool takes each after the path.  To check the file,
`tests/results.rs` checks that it names only games of the table, each once and in order, with these fields.  The command
above refreshes it too.

## b019312.txt

The scores of `optimal.jsonl.gz` as a b-file of [A019312](https://oeis.org/A019312), the Taxman sequence in the OEIS: the
format in which the OEIS takes the terms of a sequence beyond the few on its page.  Three comment lines say what range
the file covers, who made it, when and from what code, and where to read more; then each line is `n opt(n)`, from n = 1
on.  Its first 1000 terms match, one for one, the b-file the OEIS has had since 2022.

The file is made from `optimal.jsonl.gz`, so it is made again whenever that file changes.  From the top of the
repository, with `who` the name of whoever submits it:

    { printf '# Optimal Taxman scores for N=1 through N=%s.\n' "$(gzip -dc results/optimal.jsonl.gz | wc -l | tr -d ' ')"
      printf '# Generated by %s on %s using the code posted at https://github.com/bvchess/taxman/\n' "$who" \
          "$(date '+%B %-d, %Y')"
      printf '# See https://vantuyl.com/taxman for more information\n'
      gzip -dc results/optimal.jsonl.gz | jq -r '"\(.n) \(.score)"'; } > results/b019312.txt
