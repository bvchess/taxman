# Does an optimal game play the most moves?

Part of [the prover's README](../README.md).

No: at n = 833, 834 and 835 every optimal game is one move short of the longest game.  The wiki's "Questions" page asks
it as its third question, left open, with the belief that the answer is yes: does an optimal game take as many moves as
any game of the board can, or does some longer game score less?  If optimal games were always longest, a solver could
stop searching every branch that cannot reach the most moves.  Neither the Java solver nor the frame solver does that
(below), but the Java's revision of the freedoms rested on a local form of the claim.

## What is proven

The theory, in [the length lemma (Q3)](../../../docs/theory/uniqueness-and-longest-games.md#the-length-lemma) and [the
length proposition (Q4)](../../../docs/theory/uniqueness-and-longest-games.md#the-length-proposition).  Let M(n) be the
most moves of a game of n, the size of the largest playable set of picks (ℓ(n) in the theory).

* **M(n) <= ν(n)**, the matching number of the board's maximal-factor graph, which joins each number to its maximal
  factors: a playable set pays each pick with a distinct maximal factor outside it ([the playability theorem
  (P2)](../../../docs/theory/playability.md#the-playability-theorem)).  So a legal game of ν(n) moves is a longest game,
  a certificate that costs one matching.
* **M(n − 1) <= M(n) <= M(n − 1) + 1**, and **M(n) = M(n − 1) for a prime n >= 3**: the count forms of the ceiling and
  the prime formula.
* Given (★), a playable set has at most |U\*| members above n/2 ([the uniqueness theorem
  (Q1)](../../../docs/theory/uniqueness-and-longest-games.md#the-uniqueness-theorem)'s count inequality): a longest game
  cannot gain on an optimal game whose upper part is U\* except below n/2.

## Method

`src/moves.rs` and `src/bin/moves.rs` work the games in increasing order.  The number M(n) lies between two bounds:

* below, the longer of the recorded optimal game and the longest game of n − 1;
* above, the smaller of ν(n), computed for every board at once by augmenting paths, and what M(n − 1) allows.

Where the bounds meet, M(n) is settled.  Elsewhere the branch and bound of [proofs.md](proofs.md) decides it, with every
pick weighing 1 ([`Prover::weighted`](../src/game.rs)), in a mode that raises its target to each longer set it finds
([`Prover::optimize`](../src/game.rs)), with segment cuts from the smaller games' bounds.  Where a game longer than the
recorded one exists, the heaviest longest game comes from the same search with the pick of c weighing K + c, where K is
above every sum, so that a set's weight orders by moves, then by sum.  That search starts from the heaviest longest game
with the recorded game's upper part, found with the sources fixed, which is cheap.  Every longest game written is
replayed by the rules, in the order P2 gives from peel's payments (`play_order`).

The checks: M(n) and the heaviest longest game against brute force for n <= 22, ν(n) against the matching solver for
every board to 400, and `play_order` against the recorded games to 400.

## Results

`data/moves_2_1300.jsonl` holds every game 2..1300 (`tools/summarize.py moves`).  It took seven minutes on one thread,
nearly all of it the three searches for the heaviest longest game, 2.5 to 2.6 million nodes each.

| | games |
|---|---|
| the recorded optimal game has ν(n) moves: a longest game by the certificate | 1,293 |
| ν(n) = M(n) + 1, the recorded game a longest one by a search (51, 182, 183; at most 173 nodes) | 3 |
| **the recorded game one move short of M(n) = ν(n)** (833, 834, 835) | **3** |

| n | optimal game: moves (above n/2) | M(n) | heaviest longest game | short of the optimal game by | its exchange of the optimal game's moves below n/2 |
|---|---|---|---|---|---|
| 833 = 7²·17 | 362 (314) | 363 | 218,197 | 29 | {250, 252, 294, 400} out, {116, 180, 200, 320, 351} in |
| 834 | 363 (315) | 364 | 219,031 | 29 | the same (834 is a game at its ceiling: both games of 833 plus 834) |
| 835 = 5·167 | 363 (315) | 364 | 219,377 | 29 | the same |

The two games of 833 in full (the optimal one is optimal.json's): above n/2 both play the same 314 moves, U\*; below
it,

```
optimal game (48):   245 250 252 258 261 273 280 288 294 297 300 306 310 315 322 330 333 336 338 342 343 345 350 357
                     363 364 368 369 372 375 376 378 380 385 387 390 392 396 399 400 402 405 406 408 410 412 414 416
longest game (49):   the same with 250 252 294 400 out and 116 180 200 320 351 in
```

Both replay by the rules (in `src/moves.rs`, the test `game_833_has_a_longer_game_than_its_optimal_one`), and the frame
model plays both (`solver/frame/tests/soundness.rs`).  The optimal games of 834 and 835 hold the same four numbers
below n/2, and the same exchange makes their heaviest longest games.

What the data shows:

* **An optimal game need not be a longest one.**  The first counterexample is n = 833.  Its optimal set is unique
  ([uniqueness.md](uniqueness.md)) and has 362 members, and a legal game has 363.  The heaviest 363-move game keeps the
  optimal game's upper part, U\*, and exchanges four of its picks below n/2 for five lighter ones, 29 points for one
  move.  That needs nothing of the frame model: the longer game replays by the rules, ν(833) = 363 bounds every game,
  and the search proves no 363-move game worth more than 218,197, less than the recorded game's 218,226.
* **The difference is at most one move to 1300, and one only at 833-835.**  Every other recorded optimal game is a
  longest game, 1,293 of them by the matching certificate alone.  At 836 the optimal game reaches ν again.  Since a
  game's optimal sets all have the same size wherever they were enumerated ([uniqueness.md](uniqueness.md)), "the
  optimal game's moves" is well defined there; at 833-835 the optimal set is unique.
* **The mechanism is a trade of weight for moves below n/2.**  A pick above n/2 uses one number up to n/2, its payment;
  a pick below uses two, itself and its payment.  At 833 the optimal game's 48 picks below n/2 and 362 payments use 410
  of the 416 numbers up to n/2, and the longest game's 49 and 363 use 412.  Four heavy picks below n/2 (1,196)
  outweigh five light ones (1,167).
* **ν(n) is a tight bound on moves.**  It exceeds M(n) only at 51, 182 and 183, by one, where every maximum matching is
  precedence-cyclic.  The Franklín–Moniot bound, weighted by value, is off by hundreds of points.

## The solvers and the wiki

* **Neither the Java solver nor the frame solver prunes a branch for having fewer moves than possible.**  Counts enter
  the search only as upper bounds.
  * A frame's *freedoms* are its factors minus its moves.
  * `max_total` (`search/setup.rs`) is the most promotions the freedoms allow, and `max_additional` (`bounds.rs`) the
    most a node can still add.
  * The upper bound then sums at most that many of the largest candidates, and a branch is cut on that *sum*.
  * *In the bag* (`in_the_bag`, `board.rs`) is a score, what the game with no promotions makes, not a count.
* **The belief appears once, in the Java's revision of the freedoms.**  It stated as a theorem, without proof, that the
  promotion set with the largest sum also has the most promotions.  It solves the
  two-frame game of frames 0 and 1 for its heaviest promotion set, and lowers frame 0's freedoms to that set's size,
  which can cut sets with *more* promotions into frame 0, never fewer.  The frame solver ported it as the tactic
  `revise_freedoms`, measured it and removed it ([which tactics earn their
  keep](../../notebook/search/which-tactics-earn-their-keep-2026-10/README.md)).
* **833 refutes that "theorem" for the whole frame game.**  The exchange above is a promotable set: the
  optimal game promotes 48 numbers and the longest 49, which is the frame model's `max_total`
  (`solver/frame/tests/soundness.rs`, `the_heaviest_promotion_set_of_833_is_not_the_largest`).  So at 833 `max_total`
  was not an over-estimate.  It does not decide the two-frame form that the tactic used: at 833 the tactic searched the
  two-frame game (`Searched { size: 21 }`) and left `max_total` at 49, and the scores of 833-835 are proven on the
  whole board.

## What it would buy a solver

A sound statement about move counts can only bound them from above, as `max_total` and ν(n) do.  Such a bound on
counts limits a sum only through the largest candidates, which the frame solver's matching bound already outdoes.  The
tempting unsound one, pruning every branch that cannot reach M(n) moves, would return 218,197 at n = 833 instead of
218,226, and 29 too little at 834 and 835.  In the frame model, pruning the branches that cannot reach `max_total`
promotions would do the same.  The certificate is still useful as a check: a game with ν(n) moves is a longest game,
in milliseconds.
