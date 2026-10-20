# The hardest searches of the run to 5000 (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  Does the learning of conflicts closed under the cheap exchanges (`wall images --learn --closure`, *the
learning* below; [its entry](../conflicts-closed-under-cheap-exchanges-2026-10/README.md)) hold on subtrees of the
hardest searches of the committed chain's run to 4547, and does a whole game's learning match its units'?

**Answer.**  On the units, yes, a little under the old suite's size.  On whole games it went both ways, and a game's
learning matched its units' for two games of three.

**Later (2026-10-20): closed, not adopted.**  On these whole games the learning was closed: see [the
verdict](../conflicts-closed-under-cheap-exchanges-2026-10/README.md#later-2026-10-20-closed-not-adopted).

* **The units.**  On 30 units of 3940, 4225, 4416 and 4548 (104,080 to 1,048,736 steps) the learning takes 20.0% off the
  steps (9.09 M to 7.27 M): 25.5% at 3940, 29.7% at 4225, 27.6% at 4416 and 7.2% at 4548.  The median unit gains 14.7%,
  and 3 units take more steps.  The old suite's units gained 22.6% of the steps.  Every unit ended with the best set of
  the default search.
* **The whole games.**  The learning takes 24.8% off the steps and 13.4% off the CPU of 4176 and puts 26.6% on the steps
  and 30.9% on the CPU of 4050.  At 4005 it was stopped at about 4,600 CPU s, 2.2 times the default search's, without
  finishing.  The old suite's whole games gained 6.8% of the steps and 4.5% of the CPU.
* **A game against its units.**  Their units (4 at 4176, 3 at each of the others) went the way of the game at 4176
  (steps −26.0%, CPU −17.1%) and at 4050 (+14.5% and +15.1%), and the other way at 4005 (−8.8% and −4.2%).  One by one
  the 10 units vary from −55% to +35% of the steps, more than a game differs from its units together.
* **The dearest problems with the learning on** are the whole games, 4005 (over 4,600 s), 4176 (1,593 s) and 4050 (1,418
  s), then 4416-83 (297 s), 4416-148 (216 s) and 3940-71 (198 s).

The suite is the data for [the study of what the learning
leaves](../what-the-learning-leaves-in-the-hardest-searches-2026-10/README.md).  This entry makes it and measures the
learning on it; it does not explain.

## What was run

* **The ranking** (`ranking.py`).  The run that added games 3915 to 4547 searched 150 games for 29.75 G steps.  The five
  dearest searches carry 22.4% of them, the ten dearest 37.6%, the twenty 57.2% and the forty 76.5% (first table).
* **Checkpoints** (`checkpoint.py`).  For each of 3940, 4225, 4416 and 4548, as the [tuning
  campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md#the-suite) made its checkpoints: `taxman
  solve N --threads 4 --time-limit 120` with `--out` the committed chain cut to N − 1.  The chain has lines that cite
  the bounds store (game 3984 is the first), which a file is opened with only under `--use-prev`; so for the games past
  3984 the solve also took `--use-prev --force-search`, which leaves game N to its search.  The checkpoints of the three
  whole games, 4050, 4176 and 4005, were made the same way.  They are not in the repository (a run on 4 threads is not
  repeatable); the units cut from them are in `suite.jsonl`.  Seven checkpoints took 3,692 CPU seconds.
* **Units** (`tune units`, `tune expand`, `make_suite.py probe`, `assemble`).  The checkpoints hold 144 to 410 units
  (second table).  A unit is solved on one thread from the game's known best set, with fresh tables; the default
  search's steps and CPU seconds are its *probe*.  Three things differ from the tuning suite:
  * *The gap band 740-1000 does not fit.*  At 4416 a unit with a gap of 770 to 1,040 took 760 to 62,000 steps, and units
    of 100,000 steps and more began near a gap of 1,150.  Across the probed units the gap predicts the size weakly (the
    third table: a correlation of +0.07 to +0.70 by game, 0.05 to 0.86 decades of steps for each 100 of gap, where the
    tuning suite's band assumed 0.57).  So the bands were found by probing, game by game (below).
  * *The checkpoints hold few units of the middle sizes.*  A worker of the 120-second search stood at a shallow node,
    whose loop is huge, or at a deep one, whose loop is small.  Of the 22 units of 3940's checkpoint that were probed, 4
    took 100,000 to 3,000,000 steps, 7 fewer and 11 did not finish within the limits tried (up to 1.2 million).  `tune
    expand` (new, in the studies build) solves a unit for 2,048 steps and writes the nodes of the searcher's stack as
    units, which together are what the steps left of it: 47 units of 3940 gave 2,178, 26 of 4225 gave 910 and 53 of 4548
    gave 2,262.  An *expanded unit* has an id such as 3940-0.4, the fourth node of the stack that unit 0 left.  Expanded
    units are 13 of the 30 chosen.
  * *A unit of 1.2 million steps or more cost too much to find.*  Each unit that did not finish within 1.2 million steps
    cost 7 CPU minutes; 9,362 of the 13,684 CPU seconds of the probes went to solves that stopped at their limit. The
    passes of `make_suite.py probe` (ladders of 120,000, 400,000 and 1,200,000 steps; the bands and options are in
    "Files") stopped when a game had enough units in range or a few units did not finish.  So the suite has no unit
    above 1,048,736 steps, and 3940 and 4225 are thin (6 and 5 units).
* **The suite** (`suite.jsonl`).  `make_suite.py assemble --count 32 --lo 100000 --hi 3000000` over the four hard games
  gave 30 units (3940: 6, 4225: 5, 4416: 9, 4548: 10) of 104,080 to 1,048,736 steps (`suite-hard.jsonl`).  For the three
  whole games, 12 units of 50,000 steps or more were assembled (`--count 12 --lo 50000`), 4 a game, of 53,315 to 653,457
  steps; the two largest, 4050-78 and 4005-108 (558,232 and 653,457 steps), were left out of the runs to keep the CPU
  spent under 10 hours, so the suite has 10 (`suite-wholes-units.jsonl`).  The whole games are 4050, 4176 and 4005, the
  three smallest searches of the run with a million steps or more (3.2, 4.4 and 5.8 M steps on 64 threads); none needed
  replacing.
* **The whole games** were not probed: their default search, counted, is the probe.  `wholes-start.jsonl` gives each a
  nominal 10 M steps, which only sets the limits of the runs (20 M for the default search, the rule that a game over 20
  M steps on one thread is left out; 30 M for the learning), with the root gap less one from `wall landscape` (1,012,
  1,010 and 1,148).  `make_suite.py assemble --runs` then took the count runs' steps as the sizes
  (`suite-wholes.jsonl`).
* **The runs** (`suite.py run`, the closure entry's, which now takes `--suite` and `--together`).  `wall images` from
  the build of `--profile test --features studies`, one thread, fresh tables, from the known best set, two processes at
  a time under `nice -n 10`, each started with the load average at most 8, on this laptop on 2026-10-20.  The units of
  the four hard games went with the learning only (`--learn --closure`); their default steps are the probes'.  The three
  whole games and their 10 units went both ways, `--count` (the default search with its tree counted) and the learning,
  side by side.  `wall images` solves the units of a game with no known best set, 4548's, from the incumbent that the
  units file's `game` line holds (as `tune solve` does for the units of an unfinished game); its lines then have `opt`
  null.  Its test is `a_game_with_no_known_optimum_starts_from_its_units_incumbent`.
* **The check.**  Every run completed with the best set of the default search (the probe's), and the known best of its
  game where there is one: 3940, 4225, 4416 and the whole games'.  The ten units of 4548 ended with the incumbent they
  started from, as the default search did: none beat it.

## Tables

Every number below is `python3 -I tables.py` or `ranking.py` run on the data files of this folder (`tables.txt` is the whole output).  A *step* is the engine's node; the learning's columns are its `--learn --closure` run.

### The hardest searches of the run

The searches of 3915..4547: 150 games searched, 29.75 G nodes in all, 62.9 wall hours.

| rank | game | nodes | wall hours | invocations |
|---|---|---|---|---|
| 1 | 3940 | 1,545,343,488 | 2.98 | 2 |
| 2 | 3930 | 1,402,388,157 | 2.11 | 1 |
| 3 | 4225 | 1,365,533,783 | 2.30 | 1 |
| 4 | 3942 | 1,198,491,717 | 4.43 | 2 |
| 5 | 3936 | 1,160,213,320 | 1.36 | 1 |
| 6 | 4416 | 1,137,776,035 | 1.69 | 1 |
| 7 | 3939 | 965,593,907 | 2.51 | 1 |
| 8 | 4420 | 849,876,392 | 1.22 | 1 |
| 9 | 4432 | 791,763,441 | 2.33 | 2 |
| 10 | 4431 | 771,217,687 | 1.16 | 2 |
| 11 | 4424 | 753,571,864 | 1.13 | 1 |
| 12 | 4230 | 752,679,911 | 1.10 | 1 |
| 13 | 4192 | 740,271,824 | 1.28 | 1 |
| 14 | 3952 | 715,441,629 | 0.91 | 1 |
| 15 | 4440 | 600,724,279 | 0.92 | 1 |
| 16 | 3950 | 474,744,493 | 0.82 | 1 |
| 17 | 4433 | 470,687,476 | 3.06 | 2 |
| 18 | 3944 | 444,073,362 | 1.97 | 1 |
| 19 | 4392 | 439,294,332 | 0.86 | 1 |
| 20 | 3955 | 427,486,301 | 1.45 | 1 |

| dearest games | nodes | share of the stretch's nodes |
|---|---|---|
| 5 | 6,671,970,465 | 22.4% |
| 10 | 11,188,197,927 | 37.6% |
| 20 | 17,007,173,398 | 57.2% |
| 40 | 22,755,251,138 | 76.5% |

The searched games with the fewest nodes from 1,000,000 up (the candidates for whole games):

| game | nodes | wall hours | invocations |
|---|---|---|---|
| 4050 | 3,201,217 | 0.005 | 1 |
| 4176 | 4,394,993 | 0.096 | 1 |
| 4005 | 5,793,266 | 0.019 | 1 |
| 4004 | 6,971,943 | 0.095 | 1 |
| 4000 | 6,999,187 | 0.011 | 1 |
| 4015 | 7,327,121 | 0.011 | 1 |
| 4017 | 7,455,875 | 0.024 | 1 |
| 4008 | 7,636,969 | 0.012 | 1 |


### The games, their checkpoints and the units chosen

**The games**: the checkpoint (a 120 s search on 4 threads), the units chosen from it and their probes

| game | checkpoint steps | units in it | its incumbent | P* | checkpoint CPU s | units chosen | of them expanded | default steps | probe solves | stopped at the limit | probe CPU s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 3940 | 1,923,008 | 301 | 341966 | 341976 | 522 | 6 | 2 | 104,080 to 977,030 | 66 | 44 | 5,335 |
| 4005 | 1,682,940 | 236 | 356598 | 356598 | 527 | 3 | 0 | 66,512 to 155,138 | 18 | 4 | 574 |
| 4050 | 1,438,728 | 224 | 368462 | 368462 | 509 | 3 | 0 | 57,800 to 80,321 | 17 | 2 | 522 |
| 4176 | 1,521,421 | 303 | 389035 | 389035 | 516 | 4 | 0 | 53,315 to 290,091 | 27 | 3 | 448 |
| 4225 | 1,545,885 | 144 | 398623 | 398627 | 534 | 5 | 4 | 136,320 to 194,716 | 35 | 11 | 1,547 |
| 4416 | 1,728,363 | 410 | 438776 | 438830 | 541 | 9 | 0 | 119,309 to 1,048,736 | 87 | 13 | 2,634 |
| 4548 | 1,634,004 | 359 | 454465 | unknown | 543 | 10 | 7 | 116,900 to 649,809 | 37 | 18 | 2,624 |


### The size of a unit against its gap

**The size of a unit against its gap**, over the probes that finished (a unit that stopped at its limit is not in it, so the line is low for the large gaps)

| game | units | correlation with the gap | decades of steps per 100 of gap | gap at 100,000 steps | gap at 3,000,000 steps |
|---|---|---|---|---|---|
| 3940 | 22 | +0.22 | 0.16 | 1297 | 2248 |
| 4005 | 14 | +0.57 | 0.27 | 821 | 1359 |
| 4050 | 15 | +0.67 | 0.86 | 780 | 952 |
| 4176 | 24 | +0.51 | 0.35 | 903 | 1327 |
| 4225 | 24 | +0.43 | 0.28 | 1046 | 1573 |
| 4416 | 74 | +0.70 | 0.20 | 1459 | 2205 |
| 4548 | 19 | +0.07 | 0.05 | 1331 | 4339 |


### The learning on the hard games' units

**The hard games' units, by game**: the learning's steps against the default search's (the probes')

|  | units | default steps | learning steps | change: total | median | geometric mean | learning CPU s | ms a step |
|---|---|---|---|---|---|---|---|---|
| all | 30 | 9,088,253 | 7,270,313 | −20.0% | −14.7% | −22.2% | 2361 | 0.325 |
| 3940 | 6 | 1,807,434 | 1,346,483 | −25.5% | −24.0% | −24.1% | 382 | 0.283 |
| 4225 | 5 | 808,070 | 568,112 | −29.7% | −14.6% | −34.9% | 195 | 0.343 |
| 4416 | 9 | 3,189,355 | 2,309,434 | −27.6% | −24.7% | −26.8% | 834 | 0.361 |
| 4548 | 10 | 3,283,394 | 3,046,284 | −7.2% | −2.3% | −9.0% | 950 | 0.312 |

**By the steps of the default search**

|  | units | default steps | learning steps | change: total | median | geometric mean | learning CPU s | ms a step |
|---|---|---|---|---|---|---|---|---|
| steps under 100,000 | 0 | - | - | - | - | - | - | - |
| steps 100,000-299,999 | 21 | 3,543,160 | 2,808,616 | −20.7% | −14.7% | −24.0% | 880 | 0.313 |
| steps 300,000-599,999 | 5 | 2,038,272 | 1,963,920 | −3.6% | −0.3% | −6.0% | 615 | 0.313 |
| steps 600,000-1,199,999 | 4 | 3,506,821 | 2,497,777 | −28.8% | −27.0% | −30.6% | 866 | 0.347 |
| steps 1,200,000 and over | 0 | - | - | - | - | - | - | - |

Correlation (Pearson) of the log of the steps ratio, learning over default, with the gap: -0.12; with the log of the default's steps: +0.01.

**The units of the hard games**, by game and size: the default search's steps (the probe's) and the learning's steps, leaves and CPU

| unit | gap | default steps | learning steps | change | learning leaves | learning CPU s | ms a step |
|---|---|---|---|---|---|---|---|
| 3940-104 | 1060 | 104,080 | 51,790 | −50.2% | 5,079 | 13.7 | 0.264 |
| 3940-105 | 1118 | 105,154 | 103,311 | −1.8% | 10,150 | 28.5 | 0.276 |
| 3940-161 | 794 | 116,472 | 89,946 | −22.8% | 7,221 | 24.8 | 0.276 |
| 3940-0.4 | 855 | 250,458 | 250,358 | −0.0% | 22,089 | 74.2 | 0.297 |
| 3940-81.19 | 872 | 254,240 | 190,008 | −25.3% | 17,253 | 42.4 | 0.223 |
| 3940-71 | 890 | 977,030 | 661,070 | −32.3% | 58,836 | 198.0 | 0.299 |
| 4225-1.15 | 884 | 136,320 | 126,690 | −7.1% | 11,071 | 44.8 | 0.354 |
| 4225-27 | 815 | 147,346 | 145,732 | −1.1% | 13,066 | 51.2 | 0.351 |
| 4225-1.21 | 941 | 149,820 | 97,249 | −35.1% | 8,673 | 31.6 | 0.325 |
| 4225-2.13 | 1042 | 179,868 | 153,615 | −14.6% | 12,620 | 49.9 | 0.325 |
| 4225-4.24 | 1001 | 194,716 | 44,826 | −77.0% | 4,086 | 17.4 | 0.388 |
| 4416-95 | 1210 | 119,309 | 89,884 | −24.7% | 7,810 | 28.8 | 0.321 |
| 4416-98 | 1348 | 128,978 | 109,989 | −14.7% | 9,704 | 36.7 | 0.333 |
| 4416-96 | 1244 | 164,773 | 116,124 | −29.5% | 9,977 | 36.2 | 0.312 |
| 4416-26 | 1335 | 171,176 | 113,270 | −33.8% | 8,611 | 39.3 | 0.347 |
| 4416-230 | 1258 | 194,210 | 147,861 | −23.9% | 12,193 | 46.7 | 0.316 |
| 4416-87 | 1398 | 203,185 | 177,728 | −12.5% | 14,558 | 57.2 | 0.322 |
| 4416-169 | 1307 | 327,742 | 226,756 | −30.8% | 18,053 | 76.2 | 0.336 |
| 4416-148 | 1684 | 831,246 | 420,508 | −49.4% | 35,303 | 216.1 | 0.514 |
| 4416-83 | 1407 | 1,048,736 | 907,314 | −13.5% | 74,927 | 297.1 | 0.327 |
| 4548-107 | 929 | 116,900 | 118,953 | +1.8% | 10,113 | 39.5 | 0.332 |
| 4548-0.6 | 1109 | 138,222 | 128,961 | −6.7% | 10,899 | 41.9 | 0.325 |
| 4548-14.1 | 941 | 166,947 | 156,122 | −6.5% | 13,216 | 48.0 | 0.307 |
| 4548-110 | 963 | 218,153 | 116,873 | −46.4% | 10,263 | 37.5 | 0.321 |
| 4548-13 | 1064 | 282,833 | 279,326 | −1.2% | 25,229 | 90.0 | 0.322 |
| 4548-18.3 | 1054 | 355,630 | 370,830 | +4.3% | 32,690 | 112.0 | 0.302 |
| 4548-18.22 | 963 | 376,474 | 363,821 | −3.4% | 33,214 | 112.2 | 0.308 |
| 4548-1.9 | 1168 | 469,217 | 494,726 | +5.4% | 43,044 | 151.9 | 0.307 |
| 4548-1.6 | 1224 | 509,209 | 507,787 | −0.3% | 43,903 | 162.6 | 0.320 |
| 4548-17.1 | 1040 | 649,809 | 508,885 | −21.7% | 43,901 | 154.6 | 0.304 |


### A whole game against its units

**Each problem of the whole games and their units**: the default search and the learning

| problem | gap | steps | learning | change | leaves | learning | change | CPU s | learning | change |
|---|---|---|---|---|---|---|---|---|---|---|
| 4005-118 | 817 | 66,512 | 74,109 | +11.4% | 5,746 | 6,429 | +11.9% | 23.1 | 24.8 | +7.3% |
| 4005-26 | 696 | 134,958 | 80,947 | −40.0% | 11,689 | 7,014 | −40.0% | 39.4 | 25.0 | −36.4% |
| 4005-109 | 872 | 155,138 | 170,172 | +9.7% | 13,990 | 15,428 | +10.3% | 45.7 | 53.7 | +17.6% |
| 4050-root | 1012 | 3,086,223 | 3,906,598 | +26.6% | 264,459 | 337,854 | +27.8% | 1083.2 | 1417.8 | +30.9% |
| 4050-13 | 671 | 57,800 | 69,366 | +20.0% | 5,023 | 6,023 | +19.9% | 21.8 | 25.7 | +17.7% |
| 4050-121 | 761 | 77,021 | 70,312 | −8.7% | 6,553 | 6,099 | −6.9% | 29.2 | 26.2 | −10.2% |
| 4050-75 | 737 | 80,321 | 106,764 | +32.9% | 6,989 | 9,549 | +36.6% | 31.0 | 42.5 | +37.1% |
| 4176-root | 1010 | 5,962,919 | 4,483,040 | −24.8% | 510,819 | 366,784 | −28.2% | 1840.1 | 1592.8 | −13.4% |
| 4176-66 | 879 | 53,315 | 51,842 | −2.8% | 4,505 | 4,254 | −5.6% | 17.5 | 18.0 | +2.9% |
| 4176-84 | 842 | 121,880 | 164,466 | +34.9% | 10,458 | 13,664 | +30.7% | 38.5 | 55.5 | +44.0% |
| 4176-144 | 752 | 203,988 | 147,989 | −27.5% | 16,976 | 12,152 | −28.4% | 66.2 | 53.3 | −19.5% |
| 4176-83 | 832 | 290,091 | 130,707 | −54.9% | 24,917 | 10,828 | −56.5% | 86.2 | 45.9 | −46.7% |

The learning on 4005-root was stopped by hand at about 4,600 CPU s, 2.2 times the default search's 2,084 s (6,186,795 steps, 538,969 leaves), and has no line.

**Each whole game against its own units**: the learning's change in the steps and in the CPU, for the game and for its units together and one by one

| game | problems | default steps | steps | leaves | CPU | median unit: steps | CPU | ms a step: default | learning |
|---|---|---|---|---|---|---|---|---|---|
| 4005 units | 3 | 356,608 | −8.8% | −8.1% | −4.2% | +9.7% | +7.3% | 0.303 | 0.319 |
| 4050 whole | 1 | 3,086,223 | +26.6% | +27.8% | +30.9% | +26.6% | +30.9% | 0.351 | 0.363 |
| 4050 units | 3 | 215,142 | +14.5% | +16.7% | +15.1% | +20.0% | +17.7% | 0.381 | 0.383 |
| 4176 whole | 1 | 5,962,919 | −24.8% | −28.2% | −13.4% | −24.8% | −13.4% | 0.309 | 0.355 |
| 4176 units | 4 | 669,274 | −26.0% | −28.1% | −17.1% | −15.1% | −8.3% | 0.311 | 0.349 |

**The whole games run both ways, and the units of all three games, together**

|  | problems | default steps | learning steps | change | leaves | default CPU s | learning CPU s | change |
|---|---|---|---|---|---|---|---|---|
| whole games | 2 | 9,049,142 | 8,389,638 | −7.3% | −9.1% | 2923 | 3011 | +3.0% |
| their units | 10 | 1,241,024 | 1,066,674 | −14.0% | −14.4% | 399 | 371 | −7.0% |


### The dearest problems with the learning on

**The 20 dearest problems with the learning on**, by CPU seconds

| rank | problem | gap | default steps | learning steps | learning leaves | learning CPU s | ms a step |
|---|---|---|---|---|---|---|---|
| 1 | 4176-root | 1010 | 5,962,919 | 4,483,040 | 366,784 | 1592.8 | 0.355 |
| 2 | 4050-root | 1012 | 3,086,223 | 3,906,598 | 337,854 | 1417.8 | 0.363 |
| 3 | 4416-83 | 1407 | 1,048,736 | 907,314 | 74,927 | 297.1 | 0.327 |
| 4 | 4416-148 | 1684 | 831,246 | 420,508 | 35,303 | 216.1 | 0.514 |
| 5 | 3940-71 | 890 | 977,030 | 661,070 | 58,836 | 198.0 | 0.299 |
| 6 | 4548-1.6 | 1224 | 509,209 | 507,787 | 43,903 | 162.6 | 0.320 |
| 7 | 4548-17.1 | 1040 | 649,809 | 508,885 | 43,901 | 154.6 | 0.304 |
| 8 | 4548-1.9 | 1168 | 469,217 | 494,726 | 43,044 | 151.9 | 0.307 |
| 9 | 4548-18.22 | 963 | 376,474 | 363,821 | 33,214 | 112.2 | 0.308 |
| 10 | 4548-18.3 | 1054 | 355,630 | 370,830 | 32,690 | 112.0 | 0.302 |
| 11 | 4548-13 | 1064 | 282,833 | 279,326 | 25,229 | 90.0 | 0.322 |
| 12 | 4416-169 | 1307 | 327,742 | 226,756 | 18,053 | 76.2 | 0.336 |
| 13 | 3940-0.4 | 855 | 250,458 | 250,358 | 22,089 | 74.2 | 0.297 |
| 14 | 4416-87 | 1398 | 203,185 | 177,728 | 14,558 | 57.2 | 0.322 |
| 15 | 4176-84 | 842 | 121,880 | 164,466 | 13,664 | 55.5 | 0.337 |
| 16 | 4005-109 | 872 | 155,138 | 170,172 | 15,428 | 53.7 | 0.316 |
| 17 | 4176-144 | 752 | 203,988 | 147,989 | 12,152 | 53.3 | 0.360 |
| 18 | 4225-27 | 815 | 147,346 | 145,732 | 13,066 | 51.2 | 0.351 |
| 19 | 4225-2.13 | 1042 | 179,868 | 153,615 | 12,620 | 49.9 | 0.325 |
| 20 | 4548-14.1 | 941 | 166,947 | 156,122 | 13,216 | 48.0 | 0.307 |


### The check

Checks over 42 problems with a learning run: all agree: the same best and conclusion as the default search's (the probe's), the known optimum's where there is one, and the default runs' steps the suite's.

**Games with no known optimum**: what the learning found against the incumbent the units started from and against the default search's best (the probe's)

| problem | incumbent | default best | learning best | better than the incumbent |
|---|---|---|---|---|
| 4548-107 | 454465 | 454465 | 454465 | no |
| 4548-0.6 | 454465 | 454465 | 454465 | no |
| 4548-14.1 | 454465 | 454465 | 454465 | no |
| 4548-110 | 454465 | 454465 | 454465 | no |
| 4548-13 | 454465 | 454465 | 454465 | no |
| 4548-18.3 | 454465 | 454465 | 454465 | no |
| 4548-18.22 | 454465 | 454465 | 454465 | no |
| 4548-1.9 | 454465 | 454465 | 454465 | no |
| 4548-1.6 | 454465 | 454465 | 454465 | no |
| 4548-17.1 | 454465 | 454465 | 454465 | no |


### The CPU spent

| what | runs | CPU s |
|---|---|---|
| checkpoints (`taxman solve`, 4 threads, 120 s) | 7 | 3,692 |
| build | 1 | 312 |
| clippy-default | 1 | 11 |
| clippy-studies | 1 | 14 |
| expand | 3 | 148 |
| landscape | 1 | 37 |
| quick-check | 1 | 1,106 |
| run stopped by hand | 2 | 4,700 |
| test-tune | 1 | 120 |
| test-wall-images | 1 | 227 |
| tune-build | 1 | 99 |
| units | 3 | 2 |
| wall-build | 2 | 404 |
| probes that finished | 192 | 4,322 |
| probes that stopped at their limit | 95 | 9,362 |
| runs: the default search, counted | 13 | 5,406 |
| runs: the learning | 42 | 5,742 |

In all 35,704 CPU seconds, 9.92 hours.


## What the tables show

* **The hard games' units.**  The gain does not rise with the gap (a correlation of −0.12 with the change in steps) or
  with the size of the tree (+0.01).  The units of 300,000 to 600,000 steps gain least (3.6%), and four of their five
  are 4548's.  A step costs 0.325 ms with the learning (0.283 to 0.361 by game); the default search's CPU was not
  measured on these units, so there is no change in CPU for them.
* **4548.**  It is the one game with no known best set: its units start from the checkpoint's incumbent, 454,465, and
  all ten end with it, as the default search did.  They gain 7.2% of the steps, against 25.5% to 29.7% at the three
  games with a known best set.  This entry does not say why.
* **The whole games.**  A step with the learning costs 3.4% more than the default's at 4050 (0.363 ms against 0.351) and
  14.9% more at 4176 (0.355 against 0.309).  The two whole games run both ways together take 7.3% fewer steps and 3.0%
  more CPU; their ten units together gain 14.0% of the steps and 7.0% of the CPU; four of the ten take less CPU.
* **The sizes.**  The units are 53,315 to 1,048,736 steps and the whole games 3.1 to 6.2 M.  The suite has no unit of
  1.2 million steps or more (see "What was run"), so the trend of the old suite, in which larger trees gained more,
  cannot be seen past 1.05 M: the four units of 600,000 steps and more here gain 28.8%.
* **The gap.**  The line through the probed units' sizes rises by 0.05 to 0.86 decades of steps for each 100 of gap
  (third table), and the gap at which it crosses 100,000 steps runs from 780 to 1,459 by game.


## Files

* `ranking.py`: the ranking of the first table and the smallest searches (`ranking.py top`, `ranking.py smallest --count
  8`); `ranking.txt` is its output.
* `checkpoint.py`: makes a game's checkpoint (`checkpoint.py 3940 --dir DIR --log checkpoints.jsonl`);
  `checkpoints.jsonl` has a line for each of the seven, with its CPU seconds.
* `suite.jsonl`: the problems: the four hard games' 30 units (`suite-hard.jsonl`), the three whole games' 12 units
  (`suite-wholes-units.jsonl`) and the three whole games (`suite-wholes.jsonl`), each unit with its hex sets, its `gap`
  and the default search's steps, CPU seconds and best set (`default`, the probe's).  The files `suite-*.jsonl` are the
  three `make_suite.py assemble` outputs, concatenated into `suite.jsonl`:

  ```text
  make_suite.py assemble --units UNITS_3940 UNITS_4225 UNITS_4416 UNITS_4548 --probes probes.jsonl --count 32 --lo 100000 --hi 3000000 --out suite-hard.jsonl
  make_suite.py assemble --units UNITS_4050 UNITS_4176 UNITS_4005 --probes probes.jsonl --count 12 --lo 50000 --hi 3000000 --out suite-wholes-units.jsonl
  make_suite.py assemble --units UNITS_4050 --probes probes.jsonl --runs runs.jsonl --count 0 --wholes 4050,4176,4005 --whole-lo 0 --whole-hi 20000000 --whole-gaps 4050=1012,4176=1010,4005=1148 --out suite-wholes.jsonl
  ```

  Here `UNITS_N` is the units file that `tune units N CKPT --known results/strict.jsonl --sample 100000 --out UNITS_N`
  wrote (all of the checkpoint's units), with the lines of `tune expand N --units UNITS_N --ids ... --known
  results/strict.jsonl --out EXPANDED` appended for 3940 (the units of gap 850 and up), 4225 (600 and up) and 4548 (850
  and up).  The units files are not kept; `probed-units.jsonl` has the gap and depth of the units the probes tried.
* `probes.jsonl`: every solve of `make_suite.py probe`, one line each, of the default search with a node limit;
  `tables.py fit` and `probed-units.jsonl` read them.  The passes, as `make_suite.py probe --units UNITS_N --probes
  probes.jsonl --bin tune --nice 10 --max-load 8 --workers 1 ...` with: 3940: `--gap-lo 750 --gap-hi 1000 --ladder
  120000,400000,1200000 --per-game 4 --max-failures 1`, the same on 800-1,350 (8, 2), three passes by `--ids` over the
  stacks of units 72-84 and 101-105, 155-157 and 162, and on the expanded units 820-1,150 (12, 2, `--seed 2`) and
  790-1,000 (ladder 120000,400000; 12, 5; `--seed 5`); 4416: 750-1,050 (4, 1), 1,150-1,450 (ladder up to 3000000; 8, 2)
  and 1,450-1,700 (ladder 400000,1200000 from 400000; 12, 2); 4225: 600-1,000 (8, 2) and, on the expanded units,
  760-1,100 (8, 3; stopped by hand); 4548: 800-1,300 (10, 2) and 850-1,250 (10, 2, `--seed 3`); the whole games' units:
  550-900, `--lo 50000`, `--per-game 6`, `--max-failures 2`.  The tuning campaign's tool gained, with defaults that keep
  its behaviour, `--ladder`, `--ids`, `--max-load`, `--whole-limits`, `--whole-gaps`, `--runs` and, for `candidates`,
  `--from` and `--to`; `test_tune.py` tests them and passes.
* `runs.jsonl`: the lines of `wall images`, one a run, with the problem's `gap`, the one-minute `load` at its start and
  its `wall_s`: 30 learning runs of the hard games' units, and the default search and the learning of the three whole
  games and their units.  `suite.py run runs.jsonl --suite suite-hard.jsonl --modes closure --only IDS` made the first;
  `suite.py run runs.jsonl --suite wholes-start.jsonl --modes count,closure --together --workers 1 --only
  4050-root,4176-root,4005-root` and the same with `--suite suite-wholes-units.jsonl` the others.  `suite.py` is the
  closure entry's.
* `wholes_start.py`, `wholes-start.jsonl`: the whole games before their runs, with nominal sizes; `landscape.jsonl` is
  `wall landscape --known results/strict.jsonl 4050 4176 4005`.
* `tables.py`: the tables of this page (`python3 -I tables.py all`); `tables.txt` is its output.  `ledger.py` runs a
  command and adds its CPU seconds to `cpu.jsonl`, for what no other file records: builds, tests, `tune units`, `tune
  expand`, `wall landscape`.

The code: `examples/tune.rs` gained `expand` (and `start_set`, `unit_line`, factored from `solve` and `units`), with the
test `a_stopped_unit_leaves_its_stack_as_units`; `examples/wall/images.rs` and `examples/wall/anatomy.rs` the start from
a units file's incumbent.  Both are studies-build examples; the default search is untouched.

Commits: the commits of this entry on its branch.
