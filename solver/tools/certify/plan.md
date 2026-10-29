# Certificates campaign plan

- command: `certify_plan.py --results results/optimal.jsonl.gz --upper prover/data/upper_2176_3314.jsonl prover/data/upper_3315_3440.jsonl --problems bench/certify/problems_600_3440.jsonl --gaps bench/wall/gaps_chain.jsonl bench/wall/gaps_chain_3313_3440.jsonl --from 2176 --to 3440 --era 2176-3310:6.396 --era 3311-3440:16.57 --out-dir bench/certify`
- range: 2176..3440 (1,265 boards)
- results: results/optimal.jsonl.gz
- upper: prover/data/upper_2176_3314.jsonl, prover/data/upper_3315_3440.jsonl
- problems: bench/certify/problems_600_3440.jsonl
- gaps: bench/wall/gaps_chain.jsonl, bench/wall/gaps_chain_3313_3440.jsonl
- machine: 64 threads
- overhead: a certificate costs 2 times the plain search of its problem
- prices: $2.5517 an hour on demand, $0.6752 an hour at spot

## The eras: what a node costs

The chain ran several searches at once (a window), so a game's recorded search seconds are the time it
shared the machine, not what it cost.  The cost is taken from nodes: c is the era's wall hours x 3600 x the
machine's threads / the nodes of all its completed games.  Recorded / wall is the window's inflation.

| era | games | wall hours | wall thread-hours | recorded thread-hours | recorded / wall | nodes | c (us per node) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 2176-3310 | 1,135 | 6.396 | 409.3 | 1,337.0 | 3.27 | 6,043,556,521 | 243.84 |
| 3311-3440 | 130 | 16.57 | 1,060.5 | 1,643.1 | 1.55 | 11,684,771,836 | 326.73 |

A repeat whose source was proven at its ceiling has no finished search to price; its nodes are the
cost model's 10^(2.3 + 0.0479 G / n^0.3 + 0.06) for the source's root gap G.

## By band of 100 boards

| boards | games | need a certificate | repeats | shared | searched | prime | ceiling | expected thread-hours |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 2176-2199 | 24 | 18 | 8 | 5 | 10 | 1 | 5 | 1.94 |
| 2200-2299 | 100 | 55 | 18 | 9 | 37 | 15 | 30 | 28.20 |
| 2300-2399 | 100 | 56 | 17 | 11 | 39 | 15 | 29 | 386.96 |
| 2400-2499 | 100 | 60 | 23 | 14 | 37 | 10 | 30 | 27.75 |
| 2500-2599 | 100 | 58 | 22 | 13 | 36 | 11 | 31 | 66.73 |
| 2600-2699 | 100 | 55 | 16 | 14 | 39 | 15 | 30 | 2.00 |
| 2700-2799 | 100 | 49 | 16 | 9 | 33 | 14 | 37 | 1.19 |
| 2800-2899 | 100 | 53 | 21 | 9 | 32 | 12 | 35 | 2.57 |
| 2900-2999 | 100 | 48 | 23 | 9 | 25 | 11 | 41 | 1.64 |
| 3000-3099 | 100 | 59 | 24 | 13 | 35 | 12 | 29 | 26.38 |
| 3100-3199 | 100 | 58 | 22 | 10 | 36 | 10 | 32 | 179.85 |
| 3200-3299 | 100 | 60 | 23 | 12 | 37 | 11 | 29 | 42.79 |
| 3300-3399 | 100 | 56 | 17 | 11 | 39 | 15 | 29 | 658.11 |
| 3400-3440 | 41 | 23 | 10 | 6 | 13 | 3 | 15 | 1,874.73 |
| **2176-3314** | **1,139** | **635** | **235** | **129** | **400** | **140** | **364** | **911.83** |
| **3315-3440** | **126** | **73** | **25** | **16** | **48** | **15** | **38** | **2,389.00** |
| **all 2176-3440** | **1,265** | **708** | **260** | **145** | **448** | **155** | **402** | **3,300.83** |

Repeats, shared and searched count games that need a certificate; a shared game is usually a repeat too.

## The 20 dearest games

| n | proof | basis | cost from | open | rejected | nodes | chain seconds | chain threads | thread-hours | hours on 64 threads |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| 3440 | exhausted | chain | itself | 271 | 3 | 3,721,788,843 | 18,183.9 | 64 | 675.56 | 10.56 |
| 3424 | exhausted | chain | itself | 230 | 3 | 1,296,572,426 | 7,123.7 | 64 | 235.35 | 3.68 |
| 3425 | exhausted | chain | itself | 230 | 3 | 1,231,273,973 | 12,536.9 | 64 | 223.49 | 3.49 |
| 3427 | same_search | source | board 3425 | 230 | 3 | 1,231,273,973 | 12,536.9 | 64 | 223.49 | 3.49 |
| 2346 | exhausted | chain | itself | 317 | 2 | 1,142,783,599 | 4,707.9 | 64 | 154.81 | 2.42 |
| 3430 | exhausted | chain | itself | 231 | 3 | 804,328,190 | 3,998.1 | 64 | 146.00 | 2.28 |
| 3400 | exhausted | chain | itself | 217 | 4 | 522,114,396 | 5,596.5 | 64 | 94.77 | 1.48 |
| 3300 | exhausted | chain | itself | 246 | 3 | 635,072,004 | 3,627.3 | 64 | 86.03 | 1.34 |
| 3399 | exhausted | chain | itself | 229 | 4 | 420,571,254 | 2,467.2 | 64 | 76.34 | 1.19 |
| 3330 | exhausted | chain | itself | 219 | 3 | 312,563,959 | 1,513.7 | 64 | 56.74 | 0.89 |
| 3404 | exhausted | chain | itself | 198 | 4 | 280,911,491 | 2,198.1 | 64 | 50.99 | 0.80 |
| 3333 | exhausted | chain | itself | 245 | 3 | 260,663,155 | 3,515.1 | 64 | 47.31 | 0.74 |
| 3432 | exhausted | chain | itself | 212 | 3 | 252,058,539 | 5,312.2 | 64 | 45.75 | 0.71 |
| 2340 | exhausted | chain | itself | 271 | 2 | 317,935,992 | 997.5 | 64 | 43.07 | 0.67 |
| 3420 | exhausted | chain | itself | 192 | 3 | 206,564,721 | 1,628.3 | 64 | 37.49 | 0.59 |
| 3336 | exhausted | chain | itself | 248 | 3 | 182,952,373 | 2,931.6 | 64 | 33.21 | 0.52 |
| 3335 | exhausted | chain | itself | 245 | 3 | 165,792,226 | 2,792.3 | 64 | 30.09 | 0.47 |
| 3174 | exhausted | chain | itself | 233 | 5 | 217,651,346 | 1,505.3 | 64 | 29.48 | 0.46 |
| 3175 | exhausted | chain | itself | 255 | 5 | 216,248,189 | 2,431.7 | 64 | 29.29 | 0.46 |
| 3408 | exhausted | chain | itself | 198 | 4 | 158,689,184 | 898.2 | 64 | 28.80 | 0.45 |

## Boards named by the plan

- 3424: needs a certificate (its optimality rests on the frame model until it is certified); expected 235.35 thread-hours (chain)
- 3425: needs a certificate (its optimality rests on the frame model until it is certified); expected 223.49 thread-hours (chain)
- 3440: needs a certificate (its optimality rests on the frame model until it is certified); expected 675.56 thread-hours (chain)

## The estimate

- total: 3,300.8 thread-hours over 708 games that need a certificate
- on one machine of 64 threads: 51.6 hours (2.1 days)
- on demand at $2.5517 an hour: $131.61; at spot at $0.6752 an hour: $34.82
- floor on wall time: 10.56 hours on one machine, the dearest game (3440, 675.6 thread-hours) alone
- the lists' expected totals (thread-hours): pilot 1,512.7, dear 835.4, rest_1 240.3, rest_2 244.7, rest_3 231.7, rest_4 236.0 (together 3,300.8)
- the same total from the recorded seconds instead of nodes: 6,701.7 thread-hours, 104.7 hours on one machine; the upper end, which ignores the window

## Games of unknown cost

none

## Games whose share is refused (same_as is set, but the worths differ)

none
