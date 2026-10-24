# The smallest single loss over all moves (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-23.  Concluded 2026-10-23.

**Question.**  Does one cheap repair turn the recorded optimal game of n − 1 into an optimal game of n?  The repair adds
n to the moves of n − 1.  When that set cannot be played, it gives up the one move of smallest value whose removal makes
the set playable.

**Answer.**  No.  The repair is optimal at 3,829 of the 4,954 games n = 2..4955 (77%) and falls short at the other 1,125
(23%), its violations, from n = 21 on and in every range of a thousand (19% to 25%).  The recorded game of n − 1 is
not the cause: at 178 of the 181 violations that a proof or the prover's lists can settle, no optimal game of n − 1
meets the rule.  An optimal game of n gives up several heavier moves and takes lighter ones in their place, which no
single loss can do.  The chain needs no change.

## The rule

The rule is the strongest form of the procedure of the wiki's page on reusing a previous solution, which [the entry on
the wiki's open questions](../the-wikis-open-questions-2026-10/README.md#4-the-pages-procedure-and-what-it-scores)
tests.  The page's procedure gives up one particular move, the one before which it inserts n, and is optimal exactly
when that move is worth δ(n).  This rule tries every move of n − 1 instead.

Let M be the moves of the recorded game of n − 1, and δ(n) = n + opt(n − 1) − opt(n), how far opt(n) falls short of the
[ceiling](../../../docs/GLOSSARY.md#the-game) n + opt(n − 1).  The rule has two steps:

1. If M ∪ {n} is playable, play it.  It scores the ceiling, which no game of n exceeds, so it is optimal.  We call such
   a game *exact*.
2. Otherwise give up one move x of M, a *single loss* that costs x points, and let x_min be the smallest x for which M ∪
   {n}, less x, is playable.  The rule says that opt(n) = opt(n − 1) + n − x_min.  The rule *holds* when it is right,
   which is exactly when x_min = δ(n), and fails when opt(n) is above its score.  A game where it fails is a
   *violation*.

Every other single loss x scores opt(n − 1) + n − x, at most the rule's score.  So when the rule fails, every way to
adapt the recorded game by giving up one move falls short.  The rule's set is a legal game of n, so a score above opt(n)
would be a bug; there is none.

For example, the optimal game of 20 plays {10, 12, 14, 15, 16, 18, 19, 20} and scores opt(20) = 124.  With 21 added the
set scores 145 and cannot be played.  Giving up 10, the smallest move, makes it playable, so x_min = 10 and the rule
says opt(21) = 135.  But opt(21) = 144.

## Method

* **Games.**  The proven games of `results/strict.jsonl` for n ≤ 4949, which is where the committed file ended on
  2026-10-23, and for 4950..4955 the chain's own file, `~/taxman_work/chain-data/results/strict.jsonl`.  Each n takes
  its first completed line, which carries the proven score.
* **Playability.**  [`peel`](../../../docs/GLOSSARY.md#the-game) of `heuristics/core.py` decides whether a set is
  playable, exactly ([theory/decision-procedure.md](../../../docs/theory/decision-procedure.md), Theorems 1 to 3).  It
  is the call that `playable_order` of [`wiki_questions.py`](../../../heuristics/evaluation/wiki_questions.py) makes.
* **Speed.**  Trying every x costs one peel each.  After a failed test of M ∪ {n}, QuickXplain (an algorithm that
  finds a minimal conflicting subset in few tests) finds one minimal unplayable subset C of it, and only the x in C are
  tried.  That is exact, because playable sets are closed under removal: if removing x makes a set playable, C cannot
  survive the removal, so x lies in C.
* **Checks.**  `validate.py` plays all 4,955 recorded games, replaying both peel's order and the recorded order as legal
  games, and refuses eight sets that are unplayable, among them 20's game with 21.  It compares peel with a search over
  all orders on 24,690 sets of the games 2..24 (1,362 playable) and finds no disagreement.  `run.py --mode both` tries
  every x upward beside the core method and asserts that they agree, on all n ≤ 1000 (131 s) and on 112 sampled n above
  (every 37th n of 1001..4955, and 4950 and 4952..4955; 86 s).  Each answer is rebuilt as an order and replayed with
  the claimed score.
* **Cost.**  The whole range took 576 s on six processes.

## Results

| n | games | exact | rule holds | rule fails | share failing |
|---|---|---|---|---|---|
| 2..1000 | 999 | 331 | 478 | 190 | 19.0% |
| 1001..2000 | 1,000 | 309 | 437 | 254 | 25.4% |
| 2001..3000 | 1,000 | 321 | 430 | 249 | 24.9% |
| 3001..4000 | 1,000 | 310 | 439 | 251 | 25.1% |
| 4001..4955 | 955 | 331 | 443 | 181 | 19.0% |
| 2..4955 | 4,954 | 1,602 | 2,227 | 1,125 | 22.7% |

The rule never overshoots (no opt(n) is below its score), and no n lacks a single loss that makes its set playable.
The first violations are 21, 35, 45, 54, 70, 77, 78, 81, 84 and 105.

**How far short.**  The shortfall opt(n) − (opt(n − 1) + n − x_min) equals x_min − δ(n).  It runs from 1 to 1,806 (n =
4935, with x_min 1,935 and δ = 129); 63 shortfalls are at most 10 and 714 are above 100.  Among the violations, only 686
has δ = 0, and only 21, 78 and 4116 have δ = 1, which no single loss can give because 1 is never a move.

**What an optimal game does instead.**  Over the 1,125 violations, comparing the recorded games of n − 1 and n:

* it drops as many moves as it adds besides n (544 games), or one more (581);
* it trades one move for one other move in 158 games, as at 21, where 10 gives way to 9;
* it drops x_min in 729 games, and in the other 396 it plays the very move the rule gives up;
* it drops 3 moves at the median and 20 at most, and the moves it drops outweigh the moves it adds besides n by exactly
  δ(n).

At 77, for example, opt(76) = 1,869 and δ(77) = 32.  The rule gives up 35 and scores 1,911.  An optimal game
scores 1,914 by dropping 27 and 35 and adding 30, and no single loss can fit, since 32 is not a move of game 76.

**The smallest violation is n = 21.**  opt(20) = 124 and opt(21) = 144, so δ(21) = 1.  The optimal game of 20, listed
above, is its only optimal game.  The optimal game of 21 is {9, 12, 14, 15, 16, 18, 19, 20, 21}: it drops 10 and adds 9
and 21.  The smallest move of game 20 is 10, so the rule loses at least 10 where the optimal game of 21 loses 1.

## Not an artifact of which optimal game was recorded

The recorded game of n − 1 is one of possibly several optimal games, and the rule might hold from another.  The optimal
set is unique for every game up to 424, and from 425 on, 30 games up to 779 have two or three
([uniqueness.md](../../noframe/docs/uniqueness.md)).  The prover's lists of optimal sets
([`optsets_2_989.jsonl`](../../noframe/data/optsets_2_989.jsonl)) are complete for 954 of the 988 games 2..989, and
all 176 complete lists from game 780 on show one set.  For each violation, we asked whether another optimal game of
n − 1 meets the rule:

| outcome | violations |
|---|---|
| another optimal game of n − 1 meets the rule: found, built and replayed (n = 540, 686, 2397) | 3 |
| none can: δ(n) = 1, and 1 is never a move (n = 21, 78, 4116) | 3 |
| none among the complete list of optimal sets of n − 1 (80 of them with n in 426..780) | 175 |
| cannot tell: no list of n − 1 above game 989 (936 violations), or an incomplete list (8) | 944 |

The three found are these.  At 686, the rule from the recorded 685 gives up 196 and predicts 148,977, while opt(686) =
149,173 = opt(685) + 686.  The game of 686 less 686 is an optimal game of 685 that plays with 686 added, so 686 is at
its ceiling and the recorded 685 is the wrong one.  This is the case that led the [tactics
review](../search/which-tactics-earn-their-keep-2026-10/README.md#reuse_prev-nothing-left-to-save) to remove
`reuse_prev`.  At 540 and 2397, the recorded game of n less n, plus δ(n) (120 and 1,066), is an optimal game of n − 1
that meets the rule.

So the choice among optimal games of n − 1 explains 3 violations and cannot explain the 178 that are settled.  For the
944 that cannot be told, the failure from the recorded game stands, and whether another optimal game would meet the
rule is open.

## History: the Java solver's reuse

The Java solver's reuse, `reusePrev`, accepted adapted games like these as optimal until 2021-01-24.  Commit ca3d99a
("remove bogus acceleration") took out one of its steps and raised seven scores of `optimal.json` that the step had
recorded too low: 621, 622 and 623 by 6 each, and 676, 677, 678 and 679 by 44 each (`git show ca3d99a`).  The message
does not name the step ("only mostly true"); the diff removes the lines that took `droppedPromotions`, the previous
game's promotions beyond the cap on their number, off the score `reusePrev` accepted.  The Rust port, `reuse_prev`, was
removed in 2026-10 (the tactics review above).

Two of the seven are violations of the rule here: 621 (x_min 171, so the rule scores 121,211, and opt is 121,217) and
676 (x_min 148, 145,342 against 145,386).  The old scores of both are exactly the rule's scores.  The other five are
not violations: 622 and 678 are exact, and 623, 677 and 679 hold.  Their old scores were low only because their
predecessors were: each is the old score of its predecessor plus the same step as here, which is n at 622 and 678 and
n − 415, n − 673 and n − 445 at 623, 677 and 679.  So the seven errors are two runs that started at 621 and at 676.

## The rule with nothing left to add

At 21 the rule's game can take one more number: with 10 gone, 9 fits too.  So the rule gains a second condition: the
game it makes must have room for nothing more.  Then 21 is no violation, since the condition fails there.  Among the 33
violations up to 300, the rule's game takes one more number at 14 and nothing more at 19, which stay violations
(`nothing_more.py`, `nothing_more_output.txt`).  The first is 77: giving up 35 scores 1,911 and nothing more fits, but
an optimal game scores 1,914 by giving up 27 as well and playing 30, a trade that no single addition finds.  Then 81
(2,095 against 2,105) and 84 (2,245 against 2,263).  The check stopped at 300, by decision, since it settled the
question.

## Decision

The rule is false in about a quarter of the games, in every range, and the choice of the recorded game of n − 1 does
not explain it: an optimal game often trades several heavier moves for lighter ones.  It stays false with the second
condition, that nothing more fits.  Nothing in the code changes.  The chain seeds each game from the one before and
proves its result by search, so it does not rest on this rule.  A rule that names which move an optimal game gives up,
from the factorization of n, would reopen the question, and two patterns point that way: δ(pq) = p′q′ for odd
semiprimes, with p′ and q′ the primes just below p and q ([the wiki's open questions, section
7](../the-wikis-open-questions-2026-10/README.md#7-which-n)), and in its three-prime form at the near repeats where
2·11·p gives way to 3·13·q ([the corner of a near
repeat](../search/the-corner-of-a-near-repeat-2026-10/README.md)).

## Files

All scripts run from the top of the repository with `PYTHONPATH=heuristics`, for the modules of `heuristics/`, and
`nice -n 19`.  `swaplib.py` reads `results/strict.jsonl` and the chain's file named in Method.

* `swaplib.py`: the playability test, the games, and the two ways to find x_min (every x upward, and the minimal
  unplayable subset).
* `run.py --out core_full.jsonl --procs 6`: the rule at each n of 2..4955.  Its output `core_full.jsonl` has one line
  per n: the kind (`exact` or `one_loss`), x_min, the rule's score, opt(n) and opt(n − 1), the count of peel calls and
  the seconds.  `--mode both` is the cross-check: `both_1000.jsonl` came from `run.py --mode both --hi 1000`, and
  `both_sample.jsonl` from `run.py --mode both --lo 1001 --step 37 --also 4950 4952 4953 4954 4955`.
* `validate.py`: the checks of the playability test; `validate_output.txt` is its output.
* `analyze.py core_full.jsonl`: the table, the violations and the classes of the table above; `analyze_output.txt` is
  its output, and it writes `violations.tsv`: a line for each violation with n, x_min, the rule's score, opt(n), the
  shortfall, δ(n), the moves the optimal game drops and adds, and the class of the table above (A and C, found; B,
  none can; F, none among the lists; D, cannot tell; E, a listed set meets the rule, which no violation is).
* `extra_stats.py`: counts over `violations.tsv` (the exchanges of the optimal game); `extra_stats_output.txt` is its
  output.
* `nothing_more.py 300`: the rule with nothing left to add, over the violations up to 300; `nothing_more_output.txt` is
  its output.
