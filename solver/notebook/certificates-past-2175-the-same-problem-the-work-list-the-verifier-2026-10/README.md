# Certificates past 2175: the same problem, the work list, the verifier (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  S1 and S2 of the fourth consultation of the theory track (2026-10-15), what the campaign of certificates
on 2176-3440 needs before the owner approves it: can a game whose extended problem an earlier certified game posed be
certified with no search, and what will certifying every game of 2176-3440 that rests on the frame model cost, game by
game?  The games are the committed chain's (games 1..3440, merged before S2's runs).

*S1, `certified (same problem as m)`* (c5d585a).  Equal problems have equal heaviest promotion sets, so a certificate of
m, which shows the heaviest promotion set of Π⁺(O) worth best(m) = V(m) − in_the_bag⁺(m), settles a game n whose Π⁺ is
the same and whose recorded game is worth as much in it; n's own barred bounds then give opt(n) = V(n) by Theorem M⁺'s
corollary (THEORY.md, a paragraph under it).  `taxman certify` looks, after the checks of n's recorded game and before
its search, at the certified games of the 8 board sizes before n (those of `--out` and of the run, latest first),
compares the digests and then the problems whole (m's from memory, or set up again from its line, which must reproduce
the line's digest, sizes and in_the_bag⁺), and the first equal one decides: worth(n) = best(m) gives a `certified` line
with `"same_problem": {"n", "digest", "best", "worth"}` and no nodes; worth(n) < best(m) means n's game is not optimal,
and n is searched (and beaten); worth(n) > best(m) contradicts m's certificate, and the run stops with an error.  (The
design's condition is worth(n) ≥ best(m); with genuine data only equality can occur, since n's game is a promotion set
of the very problem m's certificate bounds, so the strict case is treated as the inconsistency it is.)  DESIGN.md,
"Certificates of U", has the rule.

* *600..1300*, one thread, in order: 81 of the 377 games that need a certificate are settled by the rule (68,628 nodes
  of search saved; 39 s for the whole range against 45 s), each equal to a search of it (verdict, digest, sizes,
  score), each resting on the latest game of the window whose problem is the same (found independently in the test by
  comparing the problems): 62 on n − 1, 17 on n − 2, 2 on n − 3, and 1260 → 1261 → 1263 a chain of them.  The file is
  committed (`bench/certify/certificates_600_1300.jsonl`).
* *Past 2175*, with the committed chain: the problems file (below) finds 114 such games in 1301..2175, 129 in 2176..3314
  and 16 in 3315..3440, every one of equal worth.  Five pairs past 2175 certified both ways on one thread (2248 → 2249,
  2610 → 2611, 2613 → 2615, 2622 → 2623, 2652 → 2653): the rule's certificate is the direct search's (digest, sizes,
  verdict), and the direct search takes exactly the nodes of the game it rests on (36,398, 15,081, 15,237, 57,026 and
  7,048), as one problem must; `verify-certificates` passes the files.
* Tests (`tests/certify.rs`, the units of `src/certify.rs`): the rule against a direct search on 630..640 and 1312..1317
  (all of 600..1300 ignored, 48 s); a planted wrong score on a repeat (1315, whose problem is 1314's) is beaten by the
  chain's score; lines that are not what they say are not used (a verdict not `certified`, another game's line with
  n's digest, m's line with an open member dropped or with a size or in_the_bag⁺ changed), a lowered score on m's line
  is an error and a raised one makes n searched; the window never looks past 8 sizes.

*S2.1, the barred bounds of 3315-3440* (ed526a4): `upper --from 3315 --to 3440` on the committed chain, one thread,
277 s, into the new file `prover/data/upper_3315_3440.jsonl`: 126 games, 73 needing a certificate, 3424, 3425 and 3440
among them (each an exhausted search below its ceiling, so each rests on the frame model); 19.9% of the members of
U\*_{≥3} open (172 to 271 a game, against 14.9% on 2176-3314), the root's gap 1,103 to 1,555, the root's heaviest set
holding all of U\*_{≥3} in every one.  The same games' root gaps from the solver's GF(2) bound (`wall nodes --nodes 1`,
111 s, `bench/wall/gaps_chain_3313_3440.jsonl`: the 58 searched games of 3315..3440) equal the barred bounds' at all 48
games both have.

*S2.3-S2.5* (35c614b): `taxman certify --list FILE` takes the boards of a file in its order, so that the campaign can
split its work by cost.  `taxman verify-certificates CERTS... --results --upper [--exact] [--threads K]` checks every
line without changing the files: the barred bounds' line, the results' game by the checks `certify` makes (now one
function), the width (now one function), Π⁺(O) set up again reproducing the digest and the sizes, a `certified` line
with no counterexample, a `same_problem` line resting on an earlier certified line of its file with the rule applied
again (the same function `certify` applies) and the two problems equal whole, a `beaten` line's game refereed and in
the counterexample file, no contradictory verdicts; a search's verdict is taken on trust, as an exhausted proof is.
Then how far the proofs reach: every game to N is proven by the prover (`prover/data/exact_*.jsonl`, the rule of
`prover/tools/summarize.py`), by a certificate that passed, or by a ceiling or the prime formula above a proven game.
On the committed files it is 2175, the first game that is not proven 2176 (it needs a certificate); the certificates of
600..1300 change nothing (the prover proved those games already); the five pairs past 2175 above add 13 games past
2176 (the ten certified and three at a ceiling above them).  `examples/extended.rs problems` sets each game's Π⁺(O) up
without a search and writes its sizes, worth, width, digest and the latest game of the window with the same problem
(`bench/certify/problems_600_3440.jsonl`, 1,575 games, 79 s; three need the widest set width: 3175, 3176 and 3440).

*S2.2, the work list and the estimate* (`tools/certify_plan.py`, standard library only; `bench/certify/plan.md`, the
summary for the approval, `plan.jsonl` and the list files).  The chain ran several searches at once (a window of 16 to
3310, then 4), so a game's recorded search seconds are the time it shared the machine: summed, they are 3.27 times the
run's wall time × 64 threads over 2176..3310 and 1.55 times over 3311..3440.  The plan prices a search by its nodes
instead, at each stretch's own cost per node (the run's wall time × 64 / the stretch's nodes: 244 µs of a thread a node
over 2176..3310 in 6.4 hours, 327 over 3311..3440 in the second instance's 17.4 hours less the 0.85 that the unfinished
3441 took), and a certificate at twice that, the design's factor (stage 2 measured 1.1 to 1.9 at 1400..2156).  A
repeat costs its source's search; a repeat of a game proven at its ceiling, whose problem no search has finished, costs
what the cost model of AGENDA section 2 gives for the source's root gap (10^(2.30 + 0.0479 G / n^0.3), +0.06 decades for
the window); a game the rule settles costs nothing.  On 2176..3440:

| | games | need a certificate | settled by S1 | expected thread-hours |
|---|---|---|---|---|
| 2176-3314 | 1,139 | 635 (400 searches, 235 repeats) | 129 | 912 |
| 3315-3440 | 126 | 73 (48 searches, 25 repeats) | 16 | 2,389 |
| all | 1,265 | 708 | 145 | 3,301 |

* *The estimate*: 3,301 thread-hours (2,641 to 3,961 for a factor of 1.6 to 2.4), 51.6 hours of one `c8g.16xlarge`
  (41 to 62), $35 at the spot price paid on 2026-10-15 or $132 on demand; from the recorded seconds instead of nodes it
  would be 6,702, the upper end that ignores the window.  It is above the consultation's 1,800-3,060 because of the end
  of the chain: 3315-3440 is 72% of it.
* *The dearest*: 3440 (3.72 billion nodes in the chain; 676 thread-hours, 10.6 hours on 64 threads, the floor on the
  campaign's wall time), 3424 (235), 3425 (223), 3427 (223: a repeat of 3425 that the rule cannot settle), 2346 (155),
  3430 (146), 3400 (95), 3300 (86).  2349, which the consultation's pilot named for its recorded 4,879 s, took 68
  million nodes, 9 thread-hours: its seconds are mostly the window's (2346 and its neighbours ran beside it).
* *The lists*: `pilot.txt` the five dearest (2346, 3424, 3425, 3427, 3440: 1,513 thread-hours, 24 hours of one
  machine), `dear.txt` the next fifteen with the games that rest on them (835), and `rest_1..4.txt` the others in
  increasing n, balanced (231 to 245 each).  A game the rule settles is in its source's list, after it.
* *The repeats the rule cannot settle*: 115 of the 260, 361 thread-hours, 3427 alone 223.  3427 has 3425's search
  problem, the same open members, sizes and worth, but another Π⁺: the two forms differ in exactly one pair.  On the
  board of 3427 the new source 3426 = 2·3·571 and its factor 1713 = 3·571, whose only maximal composite it is, enter
  frame 0; the frame model's `unzip` claims the pair (3426 takes 1713), so the plain problems are equal word for word,
  but Π⁺ claims nothing, keeps both in frame 0 (with 3426 in the graph's lists of 6 and 1142), and so is another form,
  though 3426 can always take 1713 and nothing else changes (the freedoms, the candidates and `max_total` are the same,
  and in_the_bag⁺ is 4,212 more, 3426 and 3427 fixed and 2641 no longer, which is the difference in score).  The
  claims in Π⁺ with the check of Lemma M5⁺ (the third consultation's 15(f), postponed then as an optimization worth
  about 10%) would make the two problems equal, and the rule would settle 3427 and its like; a question for the next
  consultation.

*Checks.*  No tree of the default mode changed at any step: the probe's fingerprints 7e8e724717d26545 and
f3deaac0dbdc030a, the tripwires, `regress --to 1000 --threads 1` (8.9 s), the one-thread chain 1..1300 line for line
equal to the base commit's binary (4f1c35b), both suites, clippy and fmt.  The plan's prediction is checked against
`certify`'s own lines on 600..1300 (the games that need one, their open and rejected sources, the 81 the rule settles),
and `verify-certificates` passes the committed certificates.  The laptop only ran setups and searches of seconds, one
thread at a time; nothing ran on AWS.
