# How far an optimal game falls short of a longest one (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-24.  Concluded 2026-10-24.

**Question.**  Is the recorded optimal game a longest game in every game of the committed chain past 1300, as it is to
1300 but for 833, 834 and 835, and how far short does it fall where it is not?

**Answer.**  It is a longest game in all 3,649 games 1301 to 4949, each by the matching certificate: the recorded game
has ν(n) moves, so no search was needed.  Over the games 2 to 4949 the largest shortfall is still one move, at 833, 834
and 835 alone, and from 836 on there is none.  Whether the shortfall is bounded in general stays open.

## Method

* **Games.**  The recorded game of each n is the first line of `results/strict.jsonl` for n, which ends at 4949 in the
  committed file ([results/README.md](../../../results/README.md#strictjsonl)), and `results/optimal.json` up to 1300,
  as in [the earlier run](../../noframe/docs/longest-games.md#results).  The chain's own file holds games past 4949,
  which this machine does not have, so they are not covered.
* **Program.**  The prover's tool `moves` ([`bin/moves.rs`](../../noframe/src/bin/moves.rs), made in commit 5700578),
  whose method is in [longest-games.md](../../noframe/docs/longest-games.md#method).  For each game it replays the
  recorded game by the rules, computes ν(n), and settles ℓ(n) by the matching certificate when the recorded game has
  ν(n) moves.  Where it falls short, a search decides ℓ(n) and then finds the heaviest longest game.  Each game reads
  its predecessor's line, so the run continues the output of the earlier run, from a copy of it.
* **The cost of the searches.**  A probe with `--budget 1` came first, which lets no search run and reports which games
  would need one.  None did: the recorded game has ν(n) moves in every game of 1301 to 4949, so no estimate was
  needed, and nothing went to the lab machine.  The full run, with no budget, took the same path.
* **A check without the prover's code.**  [`crosscheck.py`](crosscheck.py) reads the chain's lines, replays each
  recorded game by the rules in Python, and computes ν(n) from scratch for every n by Hopcroft-Karp, with the sides of
  the bipartite graph taken from the parity of Ω.  It also replays every 97th game with the slow reference checker
  `check_sequence` of `heuristics/core.py`.  Then it compares the move count and ν(n) of every game with the prover's.
* **The machine.**  This laptop, one thread at `nice -n 19`, with the 1-minute load between 2 and 3 throughout.

## Results

| | games 2..1300 | games 1301..4949 | games 2..4949 |
|---|---|---|---|
| games | 1,299 | 3,649 | 4,948 |
| the recorded game has ν(n) moves: a longest game by the certificate | 1,293 | 3,649 | 4,942 |
| ν(n) = ℓ(n) + 1, and the recorded game is a longest one by a search (51, 182, 183) | 3 | 0 | 3 |
| the recorded game is one move short (833, 834, 835) | 3 | 0 | 3 |

* **No search.**  Every game of 1301 to 4949 is settled by the matching, with 0 nodes.  The run took 27 s, of which 10
  CPU seconds, and no game took more than 0.01 s of its own.  The earlier run of 2 to 1300 took seven minutes, nearly
  all in the three searches at 833 to 835.
* **The largest shortfall is one move, 29 points.**  It is the shortfall of 833, 834 and 835, whose heaviest longest
  game scores 29 less than the optimal game ([the length
  proposition (Q4)](../../../docs/theory/uniqueness-and-longest-games.md#the-length-proposition)).  The games 836 to
  4949 have none.
* **ν(n) is a tight bound.**  ℓ(n) = ν(n) in every game of 2 to 4949 but 51, 182 and 183, where ν(n) = ℓ(n) + 1.  At
  4949, ℓ = 2,144.
* **The check agreed.**  The Python replay accepted all 4,948 recorded games at their recorded scores, 52 of them by
  the reference checker too, and its ν(n) and move counts matched the prover's for every game.  It ran in 26 s
  ([`crosscheck_output.txt`](crosscheck_output.txt)).

## What the result rests on

* **That the recorded game is a longest game** rests on two facts.  It replays by the rules, which the prover and the
  Python check both do.  And ν(n) bounds the length of every game ([the length lemma
  (Q3)](../../../docs/theory/uniqueness-and-longest-games.md#the-length-lemma)), which is proved.  Neither depends on
  [statement U](../../../docs/theory/exactness.md#statement-u).
* **That the recorded game is optimal** rests on the chain's proof: on the whole board by the prover to 2175, and in the
  frame model for 2176 to 4949, which is exact given statement U (not certified past 2175).
* **Only the recorded game is measured.**  A game may have other optimal games, and the run does not look at them.
  Wherever the prover listed every optimal set, to 989, they all have the same size.  Past 989 whether they do is
  open, so the measurement says that some optimal game is a longest game, not that every one is.

## Decision

Nothing in the code changes.  The theory's paragraph on the shortfall in [the length
proposition](../../../docs/theory/uniqueness-and-longest-games.md#the-length-proposition) and the prover's
[longest-games.md](../../noframe/docs/longest-games.md) record the new range.  The measurement is closed at the end of
the committed chain, and a longer chain extends it with the same command, in seconds, with `--results` naming the
chain's file.  A line of the output with `gap` above 0 or `how` equal to `search` would reopen it: a recorded game with
fewer than ν(n) moves, which needs a search and, if it is short of ℓ(n), is a new example beside 833.

## Files

All commands run from `solver/noframe` unless they say otherwise.

* `moves_1301_4949.jsonl`: the prover's line of each game 1301 to 4949, in the format of
  [`data/moves_2_1300.jsonl`](../../noframe/data/moves_2_1300.jsonl)
  ([longest-games.md](../../noframe/docs/longest-games.md#results)).  The two files together are the run of 2 to 4949.
  Made by

      cargo build --release -j 2 --bin moves
      cp data/moves_2_1300.jsonl OUT.jsonl
      nice -n 19 target/release/moves --from 1301 --to 4949 --out OUT.jsonl
      tail -n +1300 OUT.jsonl > ENTRY/moves_1301_4949.jsonl

  where ENTRY is this folder.
* `summary_output.txt`: the summary of the two files together, from `python3 tools/summarize.py moves
  data/moves_2_1300.jsonl ENTRY/moves_1301_4949.jsonl`.
* `crosscheck.py` and `crosscheck_output.txt`: the check without the prover's code, run from the top of the repository
  as `PYTHONPATH=heuristics nice -n 19 python3 ENTRY/crosscheck.py --lo 2 --hi 4949 --run OUT.jsonl`.

## Commits

The tool is from commit 5700578.  The commit that adds this folder holds the data and the check.
