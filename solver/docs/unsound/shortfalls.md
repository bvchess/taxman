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
  as a trajectory from the games back takes it, so the new line is a set of promotions and the moves they make, which a
  repeat can play.
* **Which lines.**  The repair writes the lines of the run's own games, in FILE, and in the [best-known
  file](best-known.md#how-a-run-uses-it) alone those of the games below the run's first, which it weighs on that file's
  lines, and never a line of a game at or above the run's first that the run has no line of yet.  Such a game is the
  run's to play, or another run's, and its [known score](#the-descent) stays the best-known file's from before it is
  played.
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

In a range the repair can change the moves that later games start from, as a level does.  It takes back only what the
better line, cut down, scores: [the descent](#the-descent) plays the games below a line that beats its best-known one
again, with trajectories from it, which finds much more.

## The descent

A pass over a range from the [best-known lines](best-known.md) plays the games in order, so the 8 games below a game
whose line beats its best-known one were played before that line existed, and never followed a trajectory from it.
Most games stop at [the run's ceiling](play.md#the-runs-ceiling), so a shortfall carries up through them, and a gain
high in a streak means that the games below it may be short too.  The *descent* plays those games again, from the
better line.

* **Gains and roots.**  A game's *known score* is the score of the best-known file's line of the game when the run
  first began it, or none if the file had no line of it or the run has no best-known file.  A game played again keeps
  the known score it began with.  A *gain* is a game of the run whose line, as played or as the repair or the descent
  wrote it again, scores more than its known score, and a *root* is a gain whose game below is no gain of the run.  A
  game below the run's first is no game of the run.  So a run with no best-known file, or a first pass over games the
  file has no lines of, never descends.
* **When.**  After each game's line and the repair's walk after it, the run descends from each root it has not
  descended from, the lowest first.  It plays again, lowest first, the games of the root's *window*: from root − 8, or
  the run's first game if that is higher, to root − 1.  These are the games whose trajectories from the 8 games above
  take the root's line, and the run's first game has none.  A game of a window that gains while its game below does not
  is a new root, and the run descends from it in its turn.  The run goes on to its next game once no root is left that
  it has not descended from, and as it descends from each game once at most, the descent ends.
* **A game played again** is played as any game of the run is: its level, seed and ceiling, the push, and the
  trajectories from the 8 games before and the 8 above, all from the lines as they now stand.  One more trajectory comes
  first after the start's: the game's own line as it stands, the better of the run's and the best-known file's, in both
  plays of a game played twice.  So a searched game never scores less than it did.  A prime or a repeat searches
  nothing and plays the line it follows, which can score less than its own, as a repeat of a game whose line is below
  the repeat's can; such a play is not recorded, and the game keeps its line.
* **Its lines** are recorded as a game's are: appended to FILE, offered to the best-known file, and walked down from by
  the repair.  Each says the root and the score of the line it played again from (`descent` in [the line](line.md)).
* **The range below.**  A descent never plays a game below the run's first.  A run over the range above another's
  leaves those games to the run below, whose trajectories from the games above take the new lines.
* **A run cut short** goes on with the descent where FILE leaves it, before its next game.  FILE holds each game's
  known score (`known_before`), the roots descended from, and in its last line that is no repair, where the last window
  was.  A line is determined by the game, its start, the moves it starts from, the level and the seed ([how a game is
  played](play.md#how-a-game-is-played)), so the run ends with the lines of one that was not stopped.
* **The cost.**  A game played again is a game's play at the run's level.  It takes the board it was played on if the
  run kept it, as it keeps those of its last 8 games, and the setup of the next game of the window is made while it
  plays; the repair's walk after it may set boards up again, a few seconds each at n ≈ 5,000.
* **What it gains.**  On the 8 games below each of the 22 gains of a pass at level 3, 184 games, playing them again with
  trajectories from the 8 games above raised 49 games in 10 streaks by 4,432 points.  The same work on 184 games in
  random windows raised 6 games in 1 streak by 18 points.  In 8 of the 10 streaks the gain came from a trajectory that
  began from a line above.  The pass was scripted, with seed 3, on 2026-11-08.

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
  as a repeat: its start has no seed, its rounds follow the start's trajectory and those of the 8 games above (and of
  the game's own line, where [the descent](#the-descent) plays it again), and it has no run's ceiling and no stop on
  growth.  A prime, which has no board, takes the one set up for its bound.
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
