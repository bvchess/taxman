# Results of the solver

This directory holds the project's results: the chain of optimal moves, and the chain's scores in the OEIS's format.
This page says what each file holds, how far it reaches, how it was made and what supports its scores.  The
[glossary](../docs/GLOSSARY.md) defines the terms; [the frame model](../docs/theory/frame-model.md#the-frame-model) and
statement U, on which the frame solver's scores rest, are defined in the theory, and the prover in [its
README](../solver/noframe/README.md).

A results file has one JSON line per game, as `taxman solve --out` writes it.  The fields are described in
`src/results.rs` and in [the frame solver's reference](../solver/docs/REFERENCE.md#a-results-line---out-schema-5).  The
counts on this page are a snapshot, dated 2026-11-06, which no test pins: the commands in
[the next section](#optimaljsonlgz) print the current ones.

## The files

| file | holds | games | made by |
|---|---|---|---|
| `optimal.jsonl.gz` | the committed chain: the moves of an optimal game for every n, each found from the moves for n − 1; the table of moves built into the frame solver | 1–5060 on 2026-11-06 | one run of `taxman solve` ([below](#optimaljsonlgz)) |
| `b019312.txt` | the scores of `optimal.jsonl.gz`, as a b-file of [A019312](https://oeis.org/A019312) in the OEIS | 1–5060 | the command [below](#b019312txt) |

How far each proof kind reaches:

* **Every line of `optimal.jsonl.gz`** is proven within the frame model.
* **Games 2 to 2530** are also proven on the whole board by the prover, so their scores are opt(n) without the frame
  model.
* **Games 2531 on** are proven within the frame model, which is exact given statement U.  U is not proven past 2530.
  A line of them proven by the ceiling or the prime formula is opt(n)
  wherever the game before it is.

"What supports the scores", below, gives the evidence for each.

**A file that is not a chain from game 1 never goes into `optimal.jsonl.gz`.**  Such a file starts above game 1 or has
a gap, where a run began with a standalone game with no ceiling, or has the line of an unfinished search (a lower bound,
nothing proven).  Each completed line in it is proven on its own terms, given what it assumes, but one that follows a
gap may rest on nothing the chain would have given it.  The verifiers read such files
([verifiers.md](../solver/docs/verifiers.md)); none is committed here.

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
6. **The prover's proofs.**  The prover proves every n up to 2530 optimal without the frame model, with no frames or
   promotions ([the prover's README](../solver/noframe/README.md); [the parity bound
   (W1)](../docs/theory/parity-bound.md#the-parity-bound)).  So
   the file's scores to 2530 do not rest on the frame model either: those 2530 are opt(n) on the whole board.  The
   games 2..2530 were proven in one run of one build, on 2026-11-06 and 07 ([the
   notebook](https://github.com/bvchess/taxman-notebook/blob/main/the-model-free-proofs-of-2-2530-on-one-machine-2026-11/README.md)),
   and its file, `solver/noframe/data/exact_parity_2_2530.jsonl`, is committed
   ([the prover's proofs](../solver/noframe/docs/proofs.md#the-run-of-2-to-2530)).
   The 2530 games of 2531–5060 are proven within the frame model, which is exact given U ([the exactness theorem
   (X7)](../docs/theory/exactness.md#the-exactness-theorem)).

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal, when the moves for n
− 1 are not optimal either.  That is what the proof kinds and the assumptions track, and why the chain from n = 1 and
the standalone solves (3) matter.

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
