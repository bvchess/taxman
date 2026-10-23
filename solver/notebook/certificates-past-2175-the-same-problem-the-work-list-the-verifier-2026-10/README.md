# Certificates past 2175: the same problem, the work list, the verifier (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-15.  Concluded 2026-10-20.

**Question.**  S1 and S2 of the fourth consultation of the theory track (2026-10-15) asked what the campaign of
certificates on 2176-3440 needs before the project's owner approves it: can a game whose extended problem an earlier
certified game posed be certified with no search, and what will certifying every game of 2176-3440 that rests on the
frame model cost, game by game?  The games are the committed chain's (games 1..3440, merged before S2's runs).
**Answer.**  Yes to the first: the rule settles 81 of the 377 games of 600..1300 that need a certificate, and 145 of the
708 games of 2176..3440, each equal to a search of it.  The campaign for 2176..3440 costs 3,301 thread-hours (2,641 to
3,961), 51.6 hours of one `c8g.16xlarge`, $35 at spot; it is prepared, and waits for the owner's approval.

## Method

### S1: `certified (same problem as m)` (c5d585a)

Equal problems have equal heaviest promotion sets.  A certificate of m shows the heaviest promotion set of Π⁺(O) worth
best(m) = V(m) − in_the_bag⁺(m), so it settles a game n whose Π⁺ is the same and whose recorded game is worth as much
in it: n's own barred bounds then give opt(n) = V(n) by Theorem M⁺'s corollary (THEORY.md, a paragraph under it).
`taxman certify` looks, after the checks of n's recorded game and before its search, at the certified games of the 8
board sizes before n (those of `--out` and of the run, latest first), compares the digests and then the problems whole,
and the first equal one decides.  If worth(n) = best(m), n gets a `certified` line with no nodes.  If worth(n) <
best(m), n's game is not optimal, and n is searched (and beaten).  If worth(n) > best(m), the run stops with an error:
with genuine data only equality can occur, since n's game is a promotion set of the very problem m's certificate
bounds, so the strict case is treated as the inconsistency it is.  The rule and the line it writes are in [the
certificates' design](../../docs/design/certificates.md).

### S2: the tools and the work list

* **S2.1, the barred bounds of 3315-3440** (ed526a4): `upper --from 3315 --to 3440` on the committed chain, one thread,
  277 s, into `solver/noframe/data/upper_3315_3440.jsonl`.
* **S2.3-S2.5** (35c614b): `taxman certify --list FILE` takes the boards of a file in its order, so that the campaign
  can split its work by cost.  `taxman verify-certificates CERTS... --results --upper [--exact] [--threads K]` checks
  every line of the certificates without changing the files (the design lists the checks), and reports how far the
  proofs reach: every game to N is proven by the prover (`solver/noframe/data/exact_*.jsonl`, the rule of
  `solver/noframe/tools/summarize.py`), by a certificate that passed, or by a ceiling or the prime formula above a
  proven game.  A search's verdict is taken on trust, as an exhausted proof is.
* **`examples/extended.rs problems`** sets each game's Π⁺(O) up without a search and writes its sizes, worth, width,
  digest and the latest game of the window with the same problem (`solver/tools/certify/problems_600_3440.jsonl`, 1,575
  games, 79 s; three need the widest set width: 3175, 3176 and 3440).
* **S2.2, the work list and the estimate** (`solver/tools/certify_plan.py`, standard library only;
  `solver/tools/certify/plan.md`, the summary for the approval, `plan.jsonl` and the list files).  The chain ran several
  searches at once (a window of 16 to 3310, then 4), so a game's recorded search seconds are the time it shared the
  machine: summed, they are 3.27 times the run's wall time × 64 threads over 2176..3310 and 1.55 times over 3311..3440.
  The plan prices a search by its nodes instead, at each stretch's own cost per node (the run's wall time × 64 / the
  stretch's nodes): 244 µs of a thread a node over 2176..3310 in 6.4 hours, 327 over 3311..3440 in the second instance's
  17.4 hours less the 0.85 that the unfinished 3441 took.  A certificate costs twice that, the design's factor (stage 2
  measured 1.1 to 1.9 at 1400..2156).  A repeat costs its source's search; a repeat of a game proven at its ceiling,
  whose problem no search has finished, costs what the cost model of AGENDA section 2 gives for the source's root gap
  (10^(2.30 + 0.0479 G / n^0.3), +0.06 decades for the window); a game the rule settles costs nothing.

## Results

**S1 on 600..1300**, one thread, in order: 81 of the 377 games that need a certificate are settled by the rule (68,628
nodes of search saved; 39 s for the whole range against 45 s), each equal to a search of it (verdict, digest, sizes,
score), each resting on the latest game of the window whose problem is the same (found independently in the test by
comparing the problems): 62 on n − 1, 17 on n − 2, 2 on n − 3, and 1260 → 1261 → 1263 a chain of them.  The file is
committed (`results/certificates/certificates_600_1300.jsonl`).

**S1 past 2175**, with the committed chain: the problems file finds 114 such games in 1301..2175, 129 in 2176..3314 and
16 in 3315..3440, every one of equal worth.  Five pairs past 2175 certified both ways on one thread (2248 → 2249, 2610
→ 2611, 2613 → 2615, 2622 → 2623, 2652 → 2653): the rule's certificate is the direct search's (digest, sizes, verdict),
and the direct search takes exactly the nodes of the game it rests on (36,398, 15,081, 15,237, 57,026 and 7,048), as one
problem must.  `verify-certificates` passes the files.

**The tests** (`tests/certify.rs`, the units of `src/certify.rs`) run the rule against a direct search on 630..640 and
1312..1317 (all of 600..1300 ignored, 48 s).  A planted wrong score on a repeat (1315, whose problem is 1314's) is
beaten by the chain's score.  Lines that are not what they say are not used: a verdict not `certified`, another game's
line with n's digest, m's line with an open member dropped or with a size or in_the_bag⁺ changed.  A lowered score on
m's line is an error and a raised one makes n searched; the window never looks past 8 sizes.

**The barred bounds of 3315-3440** cover 126 games, 73 needing a certificate, 3424, 3425 and 3440 among them (each an
exhausted search below its ceiling, so each rests on the frame model).  19.9% of the members of U\*_{≥3} are open (172
to 271 a game, against 14.9% on 2176-3314), the root's gap is 1,103 to 1,555, and the root's heaviest set holds all of
U\*_{≥3} in every one.  The same games' root gaps from the solver's GF(2) bound (`wall nodes --nodes 1`, 111 s;
`gaps_chain_3313_3440.jsonl` in this folder: the 58 searched games of 3315..3440) equal the barred bounds' at all 48
games both have.

**How far the proofs reach.**  On the committed files every game to 2175 is proven, and 2176 is the first that is not
(it needs a certificate).  The certificates of 600..1300 change nothing, since the prover proved those games already.
The five pairs past 2175 above add 13 games past 2176 (the ten certified and three at a ceiling above them).

**The work list** on 2176..3440:

| | games | need a certificate | settled by S1 | expected thread-hours |
|---|---|---|---|---|
| 2176-3314 | 1,139 | 635 (400 searches, 235 repeats) | 129 | 912 |
| 3315-3440 | 126 | 73 (48 searches, 25 repeats) | 16 | 2,389 |
| all | 1,265 | 708 | 145 | 3,301 |

* **The estimate:** 3,301 thread-hours (2,641 to 3,961 for a factor of 1.6 to 2.4), 51.6 hours of one `c8g.16xlarge`
  (41 to 62), $35 at the spot price paid on 2026-10-15 or $132 on demand; from the recorded seconds instead of nodes it
  would be 6,702, the upper end that ignores the window.  It is above the consultation's 1,800-3,060 because of the end
  of the chain: 3315-3440 is 72% of it.
* **The dearest:** 3440 (3.72 billion nodes in the chain; 676 thread-hours, 10.6 hours on 64 threads, the floor on the
  campaign's wall time), 3424 (235), 3425 (223), 3427 (223: a repeat of 3425 that the rule cannot settle), 2346 (155),
  3430 (146), 3400 (95), 3300 (86).  2349, which the consultation's pilot named for its recorded 4,879 s, took 68
  million nodes, 9 thread-hours: its seconds are mostly the window's (2346 and its neighbors ran beside it).
* **The lists:** `pilot.txt` the five dearest (2346, 3424, 3425, 3427, 3440: 1,513 thread-hours, 24 hours of one
  machine), `dear.txt` the next fifteen with the games that rest on them (835), and `rest_1..4.txt` the others in
  increasing n, balanced (231 to 245 each).  A game the rule settles is in its source's list, after it.
* **The repeats the rule cannot settle:** 115 of the 260, 361 thread-hours, 3427 alone 223.  3427 has 3425's search
  problem, the same open members, sizes and worth, but another Π⁺: the two forms differ in exactly one pair.  On the
  board of 3427 the new source 3426 = 2·3·571 and its factor 1713 = 3·571, whose only maximal composite it is, enter
  frame 0.  The frame model's `unzip` claims the pair (3426 takes 1713), so the plain problems are equal word for word,
  but Π⁺ claims nothing and keeps both in frame 0 (with 3426 in the graph's lists of 6 and 1142), so it is another form,
  though 3426 can always take 1713 and nothing else changes: the freedoms, the candidates and `max_total` are the same,
  and in_the_bag⁺ is 4,212 more (3426 and 3427 fixed and 2641 no longer), which is the difference in score.  The claims
  in Π⁺ with the check of Lemma M5⁺ (the third consultation's 15(f), postponed then as an optimization worth about 10%)
  would make the two problems equal, and the rule would settle 3427 and its like.

**Checks.**  No tree of the default mode changed at any step: the probe's fingerprints 7e8e724717d26545 and
f3deaac0dbdc030a, the tripwires, `regress --to 1000 --threads 1` (8.9 s), the one-thread chain 1..1300 line for line
equal to the base commit's binary (4f1c35b), both suites, clippy and fmt.  The plan's prediction matches `certify`'s own
lines on 600..1300 (the games that need one, their open and rejected sources, the 81 the rule settles), and
`verify-certificates` passes the committed certificates.  The laptop only ran setups and searches of seconds, one thread
at a time; nothing ran on AWS.

## Decision

S1 is kept: the rule is proven, and the campaign is prepared on it.  The claim-by-claim rule (Lemma M5⁺ claim by claim)
would settle 3427 and 59 more of the deferred repeats from the certificates of the first two phases, but it was measured
and not kept ([the claims of unzip](../the-claims-of-unzip-in-the-extended-problem-2026-10/README.md)); the build of
Lemma M5⁺ that is merged settles none of them.  Without it the 117 games are searched, for 361 thread-hours.

The campaign (stage 4, as the notebook's register calls it, after E8's stages 1 and 2) waits for the owner's approval of
the estimate below, a launch under the [rules for every launch](../../docs/runbook/launch.md#rules-for-every-launch).
[Certificates](../../docs/design/certificates.md) says what a certificate is, [the work list's
README](../../tools/certify/README.md) describes its data and the lists, and [STATUS](../../docs/STATUS.md) says where
it stands.  The plan was a runbook page until the runbook became general advice; it is here as it stood on 2026-10-20.

### The stage 4 campaign, as the runbook planned it

`certify.sh` runs `taxman certify` on every game of 2176..3440 that rests on the frame model, so that every game to 3440
is proven on the whole board, as the games to 2175 are ([results/README.md](../../../results/README.md)).  The
[runbook's page](../../docs/runbook/certify.md) has the procedure that holds for any such campaign; this section has
this one's values.

**What to approve:** up to $50 at spot (or $160 on demand), in two steps: phase A first ($3 at spot, $11 on demand),
then phases B and C once the stop rule says GO.  The table prices the campaign at $31 to $35 at spot.  Of the 708 games
that need a certificate, the rule settles 145 with no search (each in the list of the game it rests on, after it), and
115 more repeat a search that the rule cannot settle: they are searched too, and go last.  The prices are in
thread-hours of the plan, in hours of one `c8g.16xlarge` (64 threads), and in dollars at the spot price paid on
2026-10-15 ($0.6752 an hour) and on demand ($2.5517):

| phase | lists | games | thread-hours | hours on 64 threads | spot | on demand |
|---|---|---:|---:|---:|---:|---:|
| A: the pilot, with the stop rule | `pilot7` | 10 (7 searched) | 288 | 4.5 | $3.04 | $11.48 |
| B: the dear games, the dearest first (3440, 3424, 3425, 2346, ...) | `dear_rest` | 14 (12 searched) | 1,837 | 28.7 | $19.38 | $73.23 |
| B: the others, four processes of 16 threads | `rest_1` to `rest_4` | 567 | 815 | 12.7 | $8.60 | $32.50 |
| C: the repeats the rule cannot settle | `deferred` | 117 (115 searched) | 0-361 | 0-5.6 | $0-3.81 | $0-14.40 |
| **all** | | **708** | **2,940-3,301** | **46-52** | **$31-35** | **$117-132** |

* **If the overhead is not 2.**  The plan prices a certificate at 2 times the plain search; the certificates of eight
  games of 1400..2156 took a median of 1.55 times it ([one-sided candidates in the
  search](../one-sided-candidates-in-the-search-2026-10/README.md), the second stage).  At 1.6 the campaign costs
  2,350-2,640 thread-hours ($25-28 at spot, $94-105 on demand), at 2.4 3,530-3,960 ($37-42, $141-158).  The pilot
  measures it where the money goes (|O| near 200-300, root gaps 1,100-1,550).
* **Spending past 1.5 times the estimate pauses the campaign.**  Each phase runs under a `DEADLINE_HOURS` of 1.5 times
  its expected hours, after which the script stops cleanly and the campaign waits for a report; it resumes only with the
  owner's yes.
* **Wall time:** about two days on one machine; the dearest game, 3440, takes 10.6 hours by itself.  Two machines (the
  dear list on one, the four rest lists on the other, each with its own S3 prefix) bring the rest down to about 29 hours
  at the same cost.
* **Risks:** the overhead at the dear end could be worse than the plan's, which the pilot finds out for $3.  A spot
  interruption costs at most a checkpoint interval (5 minutes), and nothing with `spot-watch.sh`.  A counterexample (a
  game that beats a recorded score) stops everything and is the result itself: the chain's score would be too low at
  that game.

**The stop rule of the pilot.**  Go on if the median R is at most 3 and no R is above 5, where R is a certificate's cost
over the plan's price of its plain search, so that the campaign then costs at most 1.5 times the plan.  Stop otherwise:
report, and build the speedups (the claim-by-claim rule and a faster node) before spending more.  Stop at once on any
`beaten` verdict.  The seven games of `pilot7` (2340, 3175, 3300, 3335, 3336, 3408, 3420, with the games that rest on
them) run one after the other on all 64 threads.  The cost of a node c is 243.84 microseconds of a thread up to 3310 and
326.73 after (the table of eras in `plan.md`).  The second stage predicts R_nodes (the certificate's nodes over the
chain's) about 1.1-1.5, since the rule of symmetric pairs is missing, and R_node (its time per node over c) about
1.2-1.6; [the runbook](../../docs/runbook/certify.md#a-pilot-first) defines R, R_nodes and R_node.

**The lists** are made by `solver/tools/certify_campaign.py lists` from `plan.jsonl` and checked by `check` (7 lists,
708 games) in `solver/tools/certify/lists/stage4/`:

| list | phase | games | thread-hours | threads | expected hours | `DEADLINE_HOURS` (1.5 times) |
|---|---|---:|---:|---:|---:|---:|
| `pilot7` | A | 10 | 288.0 | 64 | 4.5 | 7 |
| `dear_rest` | B | 14 | 1,836.6 | 64 | 28.7 | 43 |
| `rest_1`, `rest_2`, `rest_3`, `rest_4` | B | 151, 139, 134, 143 | 203.8 each | 16 each | 12.7 | 19 |
| `deferred` | C | 117 | 361.2 | 64 | 5.6 | 9 |

`D=s3://taxman-013764204840-us-east-1/certify-2176-3440`, with one prefix a phase.  The timer is the deadline plus an
hour or more for the setup, the final save and the verification.

| phase | command (in tmux, from `~/taxman`) | S3 prefix | timer at launch |
|---|---|---|---|
| A | `DEADLINE_HOURS=7 POWEROFF=done S3_DEST=$D/A solver/tools/cloud/certify.sh pilot7` | `$D/A` | `shutdown -h +480` |
| B | `DEADLINE_HOURS=43 S3_DEST=$D/B solver/tools/cloud/certify.sh dear_rest && DEADLINE_HOURS=19 POWEROFF=always S3_DEST=$D/B solver/tools/cloud/certify.sh rest_1 rest_2 rest_3 rest_4` | `$D/B` | `shutdown -h +3900` |
| C | `DEADLINE_HOURS=9 POWEROFF=done S3_DEST=$D/C solver/tools/cloud/certify.sh deferred` | `$D/C` | `shutdown -h +600` |

* **The machine and the build:** one `c8g.16xlarge` at spot in the zone of the lowest price that day (the price of the
  estimate), a 40 GB gp3 volume, `Purpose=certify`, and a price ceiling of $1.2 an hour.  The whole campaign runs on one
  commit of master, pushed first; phases A and B never change build.  If a build with the claim-by-claim rule is merged
  by phase C, phase C runs on it.  The inputs are committed: `results/strict.jsonl`, `solver/noframe/data/upper_*.jsonl`
  (2..1300, 1301..2175, 2176..3314, 3315..3440), `plan.jsonl`, `plan.md` and the lists.
* **Phase A** starts with a rehearsal of the six small games 1312 to 1317 in a work directory of its own (3 searched, 3
  by the rule), which must end `ended: complete` with `verify-certificates` passing; the pilot then takes about 4.5
  hours, and `certify_pilot.py` on its certificates (`--list` the pilot's list, `--bands`) applies the stop rule.
* **Phase B** runs on a new machine, with no rehearsal.  `dear_rest` goes first, the dearest games first, one game after
  the other on all threads; if it stops for any reason, the second command does not run.  On two machines instead, the
  two commands run one on each, with the prefixes `D/B1` and `D/B2`.
* **Phase C** starts from the earlier phases' output files, joined in the order pilot, `dear_rest`, `rest_1` to `rest_4`
  (a game follows the game it rests on), as its own output file `certificates_deferred.jsonl`, which then holds all 708
  games.  If it is left for later, that file is the `cat` of the earlier phases, and the proven end of the chain is the
  first of the 117 games.
* **Results home:** the campaign's file is committed as `results/certificates_2176_3440.jsonl` (under 1 MB), never
  overwritten by a rerun, and `taxman verify-certificates` over it with `--results results/strict.jsonl` and the four
  `--upper` files must report every game to 3440 proven.  Then the new end of the proofs goes into
  [results/README.md](../../../results/README.md), the docs that name the old end are updated, and the notebook gets an
  entry with the times against the estimate and the overhead by |O| and gap, and the spending.

## Files

| file | command | what it is |
|---|---|---|
| `gaps_chain_3313_3440.jsonl` | the command of `gaps_chain.jsonl` in [the gap past 2175](../search/the-gap-past-2175-2026-10/README.md), with `SNAPSHOT` the committed `results/strict.jsonl` (games 1..3440) and the games past 3312 only, `wall` built from 35c614b, one thread, 111 s | the rest of the committed chain in the same form, for the work list of the certificates (`solver/tools/certify/`): the 58 searched games of 3315..3440 (48 `exhausted`, 10 `cap`), root gaps 1,103 to 1,555 (the exhausted ones' mean 1,265), equal to the barred bounds' gaps at the roles of Theorem B (`solver/noframe/data/upper_3315_3440.jsonl`) where both have the game |

The entry's other products are outside the folder: `results/certificates/certificates_600_1300.jsonl`,
`solver/noframe/data/upper_3315_3440.jsonl`, `solver/tools/certify/problems_600_3440.jsonl`, `plan.md`, `plan.jsonl` and
the lists in `solver/tools/certify/`, and the scripts `solver/tools/certify_plan.py`, `solver/tools/certify_campaign.py`
and `solver/tools/certify_pilot.py`.

## Commits

* c5d585a: S1, `certified (same problem as m)`.
* ed526a4: S2.1, the barred bounds of 3315-3440.
* 35c614b: S2.3-S2.5, `certify --list`, `verify-certificates` and `examples/extended.rs problems`.
* 4f1c35b: the merge of E8 stage 2, the base the checks compare with.
* 9b6855c: the stage 4 plan moved here from the runbook.
