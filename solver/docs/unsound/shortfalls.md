# Short games

Part of the [unsound mode](../unsound.md).

## The repair

A game that falls short hands its shortfall on: the games above it start from its line, a prime and a repeat copy it,
and a searched game stops at the lower ceiling it sets.  The repair takes the shortfall back down when a line above does
better.  It rests on a fact of the game: the moves of a game of N, less those above n, are a game of n that scores the
moves kept, since leaving a move out only leaves more numbers for the others ([the ceiling lemma
(N4)](../../../docs/theory/neighbors.md#the-ceiling-lemma), once for each number above n).  So what survives in game n of
a line above it is a real game of n, and when it scores more than the line of n, that line is short and is replaced.

* **When.**  After each line the run records, it walks down the games below it, from the highest: for each game, what
  survives of the lines within 8 games above it that the walk has written, the line just recorded and those it has
  replaced, the heaviest, the nearest on a tie.  If that scores more than the game's line, it replaces it, and the walk
  goes on from it too.  The walk ends at the first game with none of its lines within 8 above.  What survives is taken
  as `--back` takes it, so the new line is a set of promotions and the moves they make, which a repeat can play.  A
  game below the run's first, which the run has no line of, is weighed on the
  [best-known file's](best-known.md#how-a-run-uses-it) line.
* **Primes, trivial games and the table.**  A prime's line is not weighed: it follows from the line below it, and is
  played again from it once that is replaced.  A trivial line is optimal, and the built-in table's lines are proven, so
  neither is weighed.
* **What it changes above.**  Nothing the walk does not reach: the games above the line just recorded are not played
  yet, and those between it and a game repaired are weighed first.  A repeat of a game whose line is replaced plays its
  new promotions, and the games that follow begin their trajectories from the new lines.
* **The file.**  A replaced line is written again, appended: the line as its game was played, with the new moves,
  promotions and score, `at_ceiling` and `prev.short_by` of the new score, and `repaired`, which says what replaced it.
  The last line of a game is the game's.  A run that opens FILE goes on with the walk where its last lines leave it,
  from the last line that is no repair and the repairs after it, so a run that a stop cuts short ends with the lines of
  one that ran on.
* **The cost.**  The repair runs no rounds, so the work of a level is that of its rounds, as without it.  Its time is
  that of the survivors on the boards the run kept from playing the games, which is small beside a game's, and of the
  boards it sets up again, a few seconds each at n ≈ 5,000: the run keeps the boards of its last 8 games, so a walk
  that goes further needs them, and so does the walk that a run takes up from FILE.

In a range the repair can change the moves that later games start from, as a level does.

## Games played twice

A game played twice is played as any game is and again *unseeded*, and its line is the better of the two plays.  It is
how a run gets past a short line that the repair cannot pull up.  The repair replaces a short line only from a line
above it that does better, and in a run the lines above a short game are made from it: a prime and a repeat copy it,
and a searched game starts from it and stops at the ceiling it sets.  An unseeded play takes nothing from the games
below, so the short line does not hold it down; once it does better, the games above start from it and the repair walks
down from it.

* **Which games.**  The *push* plays a game twice when its [gap grows](#the-bound-and-the-gap) by more than 25 in its
  play, unless the game is trivial.  Which games depends on n and on the moves for n − 1 alone.
* **The unseeded play.**  It is searched on the game's board at the game's level, never played by the prime formula or
  as a repeat: its start has no seed, its rounds follow the start's trajectory alone (and those of `--above`, if
  given), and it has no run's ceiling and no stop on growth.  A prime, which has no board, takes the one set up for its
  bound.
* **The line.**  The better play's, the seeded one on a tie, as that play alone writes it, with `other`, what the other
  play came to.  A line of the unseeded play has no `prev` and no ceiling.  Its `seconds` are those of both plays.
* **The work.**  The unseeded play runs its rounds to the level's budget, as a searched game does that is not stopped
  at a ceiling, so a game played twice does up to twice a level's work: each play counts against a budget of its own.
  Each play of a level runs the rounds of the levels below first.
* **What it gains.**  A streak of short games ends where the unseeded play reaches a better score than the seeded one.
  In the chain of 4861-5060 at level 3 (524,288 work), the unseeded play of every eighth game took the games at opt(n)
  from 182 to 198 of 200, since the unseeded play of 4952 ended the streak that the stuck 4950 starts after 2 games,
  not 18 ([the
  measurement](https://github.com/bvchess/taxman-notebook/blob/main/search/unseeded-games-above-a-short-game-2026-11/README.md)).
  The push plays 4950 itself twice, and with it the unseeded play of every eighth game changed no score of 4861-5060 or
  5061-5260 and took half again the time, so the push is the only rule that plays a game twice ([the bound and the
  gap](#the-bound-and-the-gap)).  An unseeded play alone is weaker than a seeded one, and reaches opt(n) less often;
  it pays only where the seeded play is held down.

## The bound and the gap

The stop and the push compare how far a game falls below a bound that no play exceeds with how far the game below it
fell: they end the rounds of a game that is probably done, and play twice a game that is probably short.

* **The bound.**  A game's bound is `in_the_bag` plus the GF(2) bound of the root's node, the parity bound of n
  ([frame-bounds.md](../../../docs/theory/frame-bounds.md#the-frame-bound), the paragraph "Against the parity bound"); a
  score above it is a hard error.  It costs about 0.02 s a game at n = 5,000 and 0.05 s at 8,000, and a prime, which
  the prime formula plays with no board, 0.2 and 0.5 s more to set one up, on the thread that makes the next game's
  setup.
* **The gap and its growth.**  A line's gap is its bound less its score, and its growth the gap less the gap of the
  moves for n − 1 it starts from.  On the proven games of 3001-5060 the gap does not grow in 81% of the games, and
  grows by more than 50 in 5.6% and by more than 200 in 1.0%.  A short game makes it jump where a streak of short games
  begins (by 376 at the stuck 4950), and the games that it hands its shortfall on to carry the gap on, with a growth of
  about 0.
* **`--stop-growth T`** (default 25; `off` for none) ends a searched game's rounds once its gap is within T of the gap
  below, asked after each round from the one that reaches the budget of level 1 on.  Most games reach their score in
  level 1's rounds; a gap that shrank by more than T says that the game below may be short or that this one has more to
  give, so the rounds go on.  Level 1 plays as without the stop, and a level still plays the rounds of the levels below
  first.
* **The quiet rounds** are built in: the stop waits for 4 *quiet rounds*, 4 rounds in a row that did not raise the
  game's score, since a game that gains late may gain again.  The start counts as round 0.
* **The push** is built in at 25: a game whose gap has grown by more than 25 in its play is played twice, since the
  unseeded play is the one that a short line below cannot hold down.  It counts against a budget of its own, the
  level's.  A pushed game at one level may score more than the same game at a higher level, whose seeded play brought
  the growth to 25 or below and was not pushed.
* **What they gave.**  At level 3, with a push of 50 and the gap alone for the stop, they took the chain of 4861-5060
  from 198 of 200 games at opt(n) to 200 in 44% less time: the push played the stuck 4950 twice, and its streak never
  began.  On 5061-5260 no line changed, in 43% less time ([the
  measurement](https://github.com/bvchess/taxman-notebook/blob/main/search/stopping-and-pushing-by-the-gap-2026-11/README.md)).
* **Where the built-in values come from.**  A push of 25 took level 2 from 1,015 games at opt(n) to 1,039 on 4001-5060,
  for 4% more time ([the
  measurement](https://github.com/bvchess/taxman-notebook/blob/main/search/tuning-the-unsound-mode-on-a-long-chain-2026-11/README.md)).
  On lines made with no stop, 4 quiet rounds would have lost 7 of 2,060 games of level 2 where the gap alone loses 12,
  and saved 28.5% of the work against 28.9% ([the
  simulation](https://github.com/bvchess/taxman-notebook/blob/main/search/simulating-stop-rules-on-lines-already-made-2026-11/README.md)),
  and played by the mode, on 3001-4000 they took level 2 from 932 games at opt(n) of 1,000 with the gap alone to 954,
  for 1.0% more work ([the
  measurement](https://github.com/bvchess/taxman-notebook/blob/main/search/the-stop-that-waits-for-quiet-rounds-2026-11/README.md)).
