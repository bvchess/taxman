# The wiki's open questions (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The wiki's [Questions](https://github.com/bvchess/taxman/wiki/Questions) page has two open questions
besides the one about longest games ([prover 6](../../prover/README.md#6-does-an-optimal-game-play-the-most-moves)).
(1) No optimal game the owner had seen pays more than two numbers of tax for one move: is that always so, or an
artifact of the n he looked at?  Read three ways: (a) does some optimal game of some n have a move that pays three or
more; (b) is there an n every optimal game of which has one; (c) does every n have an optimal game in which every move
pays at most two?  (2) The page [Reusing a previous
solution](https://github.com/bvchess/taxman/wiki/Reusing-a-previous-solution) adapts an optimal game of n − 1 to n
and says that this gives an optimal game in about 70% of 1..1000: is there a pattern that predicts which n?

*What was kept.*  `heuristics/evaluation/wiki_questions.py`, which computes every number below.  Its `tax` and
`adapt` read `optimal.json` and the committed chain and search small boards exhaustively with `certify.py`'s exact
solver: about 7 and 10 seconds with the default `--exhaustive 40`; the figures to 56 and to 50 below are `tax
--exhaustive 56` (5 minutes) and `adapt --exhaustive 50` (40 seconds).  No solver change.  The scores are proven on
the whole board to 1300 ([the prover](../../prover/README.md); its proofs of 1301..2175 came after this was written, [the
run](the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10.md)) and within the frame model above, so what
was said of 1301..2175 then rested on the frame model.

*Words.*  A move's *tax* is what the taxman takes for it, every divisor of the move still in the pot; the move *pays
k* when its tax has k numbers.  δ(n) = n + opt(n − 1) − opt(n) is how far opt(n) falls short of the ceiling.

**1. What the committed games pay.**

| file | games | moves | pay one | pay two | pay three | games with a move paying three |
|---|---|---|---|---|---|---|
| `optimal.json` (1..1300) | 1,300 | 367,449 | 359,599 | 7,846 | 4 | 1250..1253 |
| `strict.jsonl` (1..2175) | 2,175 | 1,027,886 | 1,005,340 | 22,472 | 74 | 74 in 1250..2159 |

No game of the Java's (1..1000) has a move paying three, which is what the owner saw.  The 74 are eight moves, each in
a run of neighbouring games that share it: 748 = 2²·11·17 pays 44, 68 and 374 in 1250..1253; 836 pays 44, 76, 418 in
1332..1339; 988 pays 52, 76, 494 in 1587..1594; 1012 pays 44, 92, 506 in 1596..1609; 1610 pays 70, 230, 322 in
1638..1714 (17 games); 1276 pays 44, 116, 638 in 1984..2000; 1218 pays 174, 406, 609 in 2128..2129; 2030 pays 70, 290,
406 in 2156..2159.  Each pays three of its maximal factors.  The solver's moves come frame by frame from the front
(`accelerate::solution_for_promotions`), each paid with one maximal factor, and nothing in that order spreads the rest
of what a move sweeps: at 1250 the other multiples of 44 (572, 484) and of 68 (476, 1156) come after 748.

**2. (a) Yes, and not only at large n.**  The exhaustive search finds, for each board 2..56, the least and the greatest
largest tax over all of its optimal games.  The greatest is 3 at 40, 41 and 56 and at most 2 elsewhere.  For example
37, 25, 35, 21, 39, 33, 27, 26, 38, 34, 30, 20, 40, 32, 28, 24, 36 scores opt(40) = 525, and 30 pays 6, 10 and 15.  So
optimal games that pay three exist from n = 40 on, and the committed games have none before 1250 because of their
order.

**Theorem (the shape of a tax).**  In an optimal game every move s pays either some of its maximal factors s/p (p
prime), or s/p and s/p² for one prime p with p² | s.  So s pays at most max(2, ω(s)) numbers, ω(s) its distinct prime
factors: paying three needs three distinct primes (s ≥ 30), paying four needs four (s ≥ 210).

*Proof.*  Let A be the tax of s, the proper divisors of s in the pot when s is played.  (i) If some a ∈ A has a proper
divisor in A, every member of A divides a: otherwise the move a just before s is legal (its tax is its divisors in A),
s still has a tax (a member that is neither a nor a divisor of a), and the pot after s is what it was in the game, so
the game with a inserted scores more.  (ii) If d ∈ A and d | e | s with e ≠ s, then e ∈ A: whatever removed e before s
(playing it, or a multiple taking it as tax) would have removed d with it, and e is not a move after s, which takes it
(the lifting argument of THEORY.md, Theorem 1).  If A is an antichain under divisibility, every member is a maximal
factor of s, since one that is not lies strictly below some s/p, which is in A by (ii).  Otherwise some e ∈ A has a
proper divisor in A, and every member divides e by (i); e lies below some s/p, which is in A by (ii) and divides e by
(i), so e = s/p.  For any other member d and any prime r dividing s/d, s/r is a multiple of d, so it is in A by (ii)
and divides s/p by (i), so r = p: s/d = p^k with k ≥ 2.  And k = 2: otherwise dp, in A by (ii), has the proper
divisor d in A, so by (i) it is a multiple of s/p = dp^(k−1).  ∎

The proof uses optimality only in (i), so it holds of every game that inserting one move cannot improve.  Every move of
both files has a tax of one of the two shapes.

**3. (b) No n ≤ 2175 forces it.**  Each of the 74 games becomes a game of the same moves in which no move pays more
than two by moving one move: a later multiple of a number of the tax to just before the move, or the move to just after
such a multiple (`repair`, every step replayed).  At 1250, 572 = 4·11·13 goes just before 748 and takes 44, and 748
pays 68 and 374.  So every n ≤ 2175 has an optimal game in which no move pays more than two, and so does every n ≤ 56
by the exhaustive search (the least largest tax is at most 2 on every board).

**4. (c) A conjecture.**  Every n has an optimal game in which every move pays at most two.  The evidence is section 3,
and that the committed sets leave the order free.  In none of them does a move have two or more numbers outside the set
that only it divides among the moves, so nothing in a set forces a move to pay three in every order.  And in every
order of a set that pays at most two, the number of moves that pay two is the same, the numbers taken as tax less the
moves (each is taken once and each move takes at least one): 3 to 21 at n = 250, 500, ..., 2000.  A proof would have to
rule out an optimal set every order of which crowds two of these extra numbers onto one move.

**5. The page's procedure and what it scores.**  As the page has it: play the game of n − 1 on the board 1..n up to
the move that would leave n no proper divisor in the pot (its tax, or the move itself, takes the last), select n just
before that move, then play every remaining move of the game of n − 1 that is still legal; if no such move comes, n is
selected last.  n takes R, the divisors of n still in the pot, all of which that move would have removed, so after n
(and that move, if it still has a divisor) the pot holds everything it held in the game of n − 1 after that move, and
every later move stays legal.  So the procedure skips at most one move, x, and scores n + opt(n − 1) − x (x = 0 when it
skips none): **it is optimal exactly when x = δ(n)**.  The page's cases: lucky, δ(n) = 0 and no move skipped; prime, n
takes 1 just before the first move, the largest prime P, which it skips (δ(n) = P is the prime formula); general, a
composite n with x = δ(n) > 0.

| games of n − 1 | n | optimal | lucky | prime | general |
|---|---|---|---|---|---|
| `optimal.json` | 2..1000 | 452 (45.2%) | 274 | 167 | 11 |
| `strict.jsonl` | 2..1000 | 456 (45.6%) | 278 | 167 | 11 |
| `strict.jsonl` | 2..2175 | 930 (42.8%) | 592 | 325 | 13 |

**6. Where the page's 70% comes from.**  Not from this procedure, but from the Java's `reusePrev`
(`src/main/java/org/taxman/h6/frame/FrameSolver.java`), which keeps the previous game's promotions (its moves up to
n/2), forms the frames of n around them, and accepts the game when it scores the previous score plus n less the
previous moves that the frames of n find impossible.  The moves it gives up are any the upper half of n gives up,
which the page's procedure cannot reach (section 7).  Emulated on sets (the moves of n − 1 up to n/2 with the upper
half of n's committed game, accepted when the set is playable and scores opt(n)), it accepts 690 of 2..1000 (69.1%),
888 of 2..1300 and 1,456 of 2..2175 (67.0%), the primes included (the Java takes as they are the boards without
promotion candidates, such as 8, 12 and 16, which the emulation does not); the Rust port of it (`reuse_prev`,
removed) accepted 670 of the games of 2..1300 that are neither prime nor trivial ([tactics
review](which-tactics-earn-their-keep-2026-10.md#reuse_prev-nothing-left-to-save)).

**7. What is proven about the procedure.**

* *δ(n) = 0 exactly when some optimal game of n − 1 adapts with no move skipped.*  One way is section 5.  The other:
  take an optimal game of n and delete n.  The rest is a game of n − 1 (deleting a move leaves more in the pot, and n
  is never tax) that scores opt(n − 1).  The procedure selects n no earlier than it stood, since every move before
  that left n a divisor, and the move m before which it selects n keeps a divisor: in the game of n, m came after n and
  took some d that n did not take, so d does not divide n (nor does m), and d is still in the pot.  Which game of n − 1
  adapts depends on its order: of the 102 composite n with δ(n) = 0 whose recorded game of n − 1 fails, 101 succeed
  with the same moves in another order (a legal order of those moves and n, from peel, with n taken out); at 686 the
  recorded moves of 685 and 686 are not playable together, and another optimal set of 685 is needed (the register's row
  on `reuse_prev`).
* *A divisor of n left at the end.*  If a proper divisor of n is neither a move nor taxed in a game of n − 1, the
  procedure selects n last and skips nothing.  For an even n, n/2 has no multiple below n but itself, so this holds
  whenever n/2 is not a move of the game of n − 1.  In particular **opt(2p) = 2p + opt(2p − 1) for every prime p**,
  and every optimal game of 2p − 1 adapts: a prime can be played only as the first move (its one proper divisor is 1),
  every optimal game opens with the largest prime (Lemma A in `accelerate::prime_shortcut`), and by Bertrand's
  postulate that prime is above p.
* *For a composite n the skipped move shares a prime with n.*  The first move P takes only 1 and is never the move n
  goes before (n has a divisor d ≥ 2, and P ∤ n, since n ≥ 2P would put a prime between P and n); after it 1 is gone,
  and the skipped move either divides n or loses its whole tax to n, so its tax is common divisors of it and n other
  than 1.  So **when δ(n) > 0 is coprime to a composite n, no optimal game of n − 1 adapts**: 644 of the 1,155
  composite n of 2..2175 with δ(n) > 0.  Of the other 511, 13 adapt with the recorded game (8, 16, 32, 36, 64, 196, 256,
  350, 400, 416, 512, 1024, 2048); in 280, δ(n) is not a move of the recorded game of n − 1, and in 67 neither δ(n)
  nor any maximal factor of it outside those moves divides n, so no order of the recorded moves adapts (a skipped move
  that does not divide n is paid, when it is played, by a maximal factor of it that n takes); 151 are not settled.

**8. Which n.**  By class, n = 2..2175: the page's procedure on the recorded game of n − 1, the Java's reuse as
emulated in section 6, and δ(n) = 0.

| n | games | page's procedure | Java's reuse | δ(n) = 0 |
|---|---|---|---|---|
| prime | 326 | 326 | 326 | 1 (n = 2) |
| 2p | 181 | 181 | 181 | 181 |
| other even, n/2 not a move of the game of n − 1 | 317 | 317 | 317 | 317 |
| other even, n/2 a move of it | 588 | 52 | 73 | 74 |
| odd, pq with p < q | 427 | 0 | 419 | 2 |
| other odd composite | 335 | 54 | 140 | 119 |

* *Even n.*  The procedure succeeds whenever n/2 is not a move of the game of n − 1 (section 7), and seldom when it is.
  Whether it is, is decided by the upper half of n − 1, which the descending greedy finds in polynomial time (U\*,
  [prover 2](../../prover/README.md#2-conjecture--at-scale)).  By the cofactor c = n/p of n's largest prime factor p: n/2
  is never a move at c = 2, and the procedure succeeds at 98 of the 100 n with c = 4, 67 of 71 with c = 6 and 44 of 45
  with c = 10, but at 1 of 58 with c = 8 and 1 of 41 with c = 12 (where the Java's reuse takes none).
* *Odd n.*  The procedure succeeds only where δ(n) = 0 (54 of the 762 odd composites, each of them).  At the odd
  semiprimes pq it never does: with p′ and q′ the primes just below p and q, **δ(pq) = p′q′ at all 417 odd semiprimes
  of 2..2175 with p′q′ > pq/2**, and at 3 of the 10 others (not at 15, 21, 33, 35, 55, 77, 187), and p′q′ is coprime
  to pq unless q′ = p (143, 221, 323, 437, 667, ...).  p′q′ is then a move of the upper half of pq − 1 that the upper
  half of pq gives up, which the Java's reuse allows: it takes 419 of the 427.
* *Factorization.*  By the number of prime factors counted with multiplicity (the page's procedure, the Java's reuse):
  one, all 326; two, all 181 even n and 1 and 430 of the 440 odd (pq or p²); three, 312 and 313 of the 320 even and 46
  and 105 of the 227 odd; four or more, 57 and 77 of the 585 even and 7 and 24 of the 95 odd.

**9. The committed proofs and the games above.**  δ(n) = 0 is a score at the ceiling: n = 2, the 192 `cap` games and
the repeats and trivial boards at it (493 and 8).  The page's procedure succeeds at the 326 primes, 91 of the 192
`cap` games, the 493 repeats at the ceiling, 9 of the 12 trivial boards and 11 of the 730 `exhausted` games; the
Java's reuse at the primes, all 914 repeats, 191 of the `cap` games, 8 trivial boards and 17 `exhausted` games.  So the
70% are, nearly exactly, the games the solver settles without a search, and the games it has to exhaust are the 30%: a
game whose search problem is that of n − 1 (804) always adapts the Java's way, and the page's way only at the ceiling.
From [going higher](going-higher-and-coming-back-down-2026-10.md): δ(n) ≤ n/2 for even n is that entry's bound with
K = 1, and at its even n with δ(n) = n/2 (8, 12, 16, 24, 32, 48, 64, 96, 256, 384, 512, 768, 1024, and now 2048)
dropping n/2 and selecting n last is optimal; the page's procedure does that at the powers of two, not at 3·2^k.

**10. How much the game of n − 1 matters.**  The two files' games of n − 1 give different answers at 27, 42, 108, 153,
168 and 175.  Over every optimal game of n − 1, n = 3..50: where δ(n) = 0 the share that adapts is 50% at 9 and 18, 7.5%
at 15, 66.7% at 27, 14.6% at 33, 38.5% at 42 and 100% elsewhere; where δ(n) > 0 it is all or none (none at 12, 20, 21,
24, 25, 30, 35, 39, 40, 45, 48 and 49; all at the primes and at 8, 16, 32 and 36).  At 21, 35 and 45, δ(n) is 1, 3 and
2, which no game can skip.  So the Home page's "including all games N < 20" holds of the Java's program, which takes
12, a board without promotion candidates, as it is, but not of the page's procedure: no optimal game of 11 adapts to 12.

*Conclusion.*  (1) An optimal game can have a move that pays three (from n = 40; in the committed games from 1250),
but only a move with three distinct primes paying three of its maximal factors (a theorem), and no n ≤ 2175 needs one:
every committed game that has one reorders into one that does not.  That every n has an optimal game paying at most two
is a conjecture.  (2) The page's procedure is optimal at 45% of 2..1000, not 70%; the 70% is the Java's reuse, which
gives up whatever the upper half of n gives up.  The procedure is optimal exactly when the move it skips is δ(n): always
at the primes and at the even n whose half the game of n − 1 does not play (so opt(2p) = 2p + opt(2p − 1)), never when
δ(n) is coprime to a composite n (nearly every odd semiprime, where δ(pq) = p′q′), and seldom otherwise.  The games
that adapt are, nearly, the games the solver settles without a search.

*Open.*  (c); δ(pq) = p′q′ wherever p′q′ > pq/2; the 151 composite n of section 7 that are not settled.
