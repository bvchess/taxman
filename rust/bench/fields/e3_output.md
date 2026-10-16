# The E3 pilot

runs: `bnb.jsonl`, 15 trees and 4 Knuth lines; audit: `audit.jsonl`, 684 nodes; logs: `/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-ab0aa322a32a3f855/rust/bench/fields/logs`.

## 1. Trees

The exact tree of each variant per game against the same game's v0.  Tree ratio: v0 nodes / nodes.  p: refit pricings per Tier-1 evaluation; c = 2.5 + 3p the model's cost of a node in v0-node units (v0: 1).  Measured cost: (ms - ms_audit) per node over v0's ms per node.  Time ratio: (ms - ms_audit) / v0 ms.  Work ratio: v0 nodes / (nodes · c), the model's prediction of the time ratio's inverse (above 1: the variant wins).  Breakdowns: the master solves that ended optimal at a basis that is not primal feasible, whose duals are not to be trusted.  `≥` and `≤`: a run stopped at a budget, so its counts are only lower bounds and the ratio is bounded on one side.

### 1215

Root GF(2) gap g = 213, optimum 35050, root GF(2) bound 35263; Tier 1 at the fit: 35168, the fit file's bound: 35168.

| variant | policy | k | nodes | completed | tree ratio | cut gf2 | cut tier1 | cut refit | Tier-1 evals | refits | breakdowns | pricings per refit | p | c | measured cost per node | time ratio | work ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| v0 | all | 20 | 24,516 | yes | 1.00 | 19,846 | 0 | 0 | 0 | 0 | 0 | - | - | 1.00 | 1.00 | 1.00 | 1.00 |
| v1 | all | 20 | 18,774 | yes | 1.31 | 14,715 | 621 | 0 | 4,059 | 0 | 0 | - | 0.00 | 2.50 | 1.69 | 1.30 | 0.52 |
| r1 | all | 20 | 12,307 | yes | 1.99 | 9,018 | 734 | 528 | 3,289 | 2,554 | 0 | 17.7 | 13.77 | 43.80 | 4.38 | 2.20 | 0.05 |
| r1 | all | 50 | 10,809 | yes | 2.27 | 7,757 | 721 | 627 | 3,052 | 2,330 | 0 | 40.0 | 30.51 | 94.03 | 8.06 | 3.56 | 0.02 |
| r1 | all | 100 | 10,225 | yes | 2.40 | 7,291 | 680 | 679 | 2,934 | 2,253 | 0 | 64.0 | 49.16 | 149.99 | 11.75 | 4.90 | 0.02 |
| r1 | all | 200 | 9,886 | yes | 2.48 | 7,004 | 657 | 704 | 2,882 | 2,224 | 0 | 77.3 | 59.63 | 181.40 | 13.57 | 5.47 | 0.01 |
| r2 | all | 20 | 4,955 | yes | 4.95 | 3,099 | 585 | 542 | 1,856 | 1,270 | 3 | 14.7 | 10.03 | 32.58 | 6.38 | 1.29 | 0.15 |
| r2 | all | 50 | 3,703 | yes | 6.62 | 2,244 | 409 | 535 | 1,459 | 1,049 | 16 | 24.7 | 17.75 | 55.75 | 11.18 | 1.69 | 0.12 |
| r2 | all | 100 | 3,557 | yes | 6.89 | 2,148 | 389 | 530 | 1,409 | 1,019 | 19 | 27.3 | 19.72 | 61.65 | 13.86 | 2.01 | 0.11 |
| r2 | all | 200 | 3,508 | yes | 6.99 | 2,121 | 382 | 524 | 1,387 | 1,004 | 18 | 27.8 | 20.13 | 62.89 | 13.89 | 1.99 | 0.11 |
| r2 | excess:50 | 20 | 9,582 | yes | 2.56 | 7,128 | 760 | 190 | 2,454 | 723 | 0 | 17.2 | 5.06 | 17.67 | 3.26 | 1.27 | 0.14 |
| r2 | tier1:0.5 | 20 | 15,049 | yes | 1.63 | 11,740 | 406 | 250 | 3,309 | 383 | 0 | 12.9 | 1.50 | 6.99 | 2.21 | 1.36 | 0.23 |
| r2c | all | 20 | 3,818 | yes | 6.42 | 2,278 | 470 | 526 | 1,540 | 1,069 | 0 | 18.4 | 12.76 | 40.78 | 10.16 | 1.58 | 0.16 |

### 1472

Root GF(2) gap g = 581, optimum 52565, root GF(2) bound 53146; Tier 1 at the fit: 52871, the fit file's bound: 52871.

| variant | policy | k | nodes | completed | tree ratio | cut gf2 | cut tier1 | cut refit | Tier-1 evals | refits | breakdowns | pricings per refit | p | c | measured cost per node | time ratio | work ratio |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| v0 | all | 20 | 3,401,305 | yes | 1.00 | 2,678,807 | 0 | 0 | 0 | 0 | 0 | - | - | 1.00 | 1.00 | 1.00 | 1.00 |
| r2 | all | 20 | 675,502 | yes | 5.04 | 430,188 | 55,047 | 67,769 | 245,314 | 190,266 | 1,233 | 16.9 | 13.09 | 41.76 | 23.19 | 4.61 | 0.12 |

## 2. Times

Milliseconds per node, wall time of one thread.  Total: (ms - ms_audit) / nodes.  The next four: the GF(2) bound (ms_gf2), the builds of the signed matroids (ms_builds), Tier 1 (ms_tier1) and the refits (ms_refit).  The last three are inside the evaluations of Tier 1 and the refits: matroid intersection (ms_mi), the greedies (ms_greedy) and the simplex (ms_simplex).

### 1215

| variant | policy | k | nodes | total | GF(2) | builds | Tier 1 | refits | MI | greedy | simplex |
|---|---|---|---|---|---|---|---|---|---|---|---|
| v0 | all | 20 | 24,516 | 0.669 | 0.652 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| v1 | all | 20 | 18,774 | 1.132 | 0.665 | 0.351 | 0.083 | 0.000 | 0.045 | 0.013 | 0.000 |
| r1 | all | 20 | 12,307 | 2.934 | 0.686 | 0.432 | 0.128 | 1.583 | 0.706 | 0.560 | 0.000 |
| r1 | all | 50 | 10,809 | 5.398 | 0.714 | 0.462 | 0.142 | 3.966 | 1.576 | 1.529 | 0.000 |
| r1 | all | 100 | 10,225 | 7.865 | 0.717 | 0.477 | 0.141 | 6.416 | 2.491 | 2.465 | 0.000 |
| r1 | all | 200 | 9,886 | 9.083 | 0.700 | 0.469 | 0.145 | 7.656 | 2.958 | 2.936 | 0.000 |
| r2 | all | 20 | 4,955 | 4.271 | 0.711 | 0.607 | 0.132 | 2.678 | 0.785 | 0.198 | 0.207 |
| r2 | all | 50 | 3,703 | 7.481 | 0.743 | 0.645 | 0.154 | 5.782 | 1.468 | 0.335 | 1.067 |
| r2 | all | 100 | 3,557 | 9.277 | 0.745 | 0.657 | 0.142 | 7.570 | 1.648 | 0.382 | 1.854 |
| r2 | all | 200 | 3,508 | 9.298 | 0.720 | 0.633 | 0.143 | 7.646 | 1.626 | 0.373 | 1.980 |
| r2 | excess:50 | 20 | 9,582 | 2.182 | 0.694 | 0.406 | 0.098 | 0.882 | 0.300 | 0.081 | 0.079 |
| r2 | tier1:0.5 | 20 | 15,049 | 1.478 | 0.699 | 0.363 | 0.088 | 0.237 | 0.115 | 0.035 | 0.015 |
| r2c | all | 20 | 3,818 | 6.797 | 0.713 | 1.116 | 0.152 | 4.647 | 1.085 | 0.289 | 0.515 |

### 1472

| variant | policy | k | nodes | total | GF(2) | builds | Tier 1 | refits | MI | greedy | simplex |
|---|---|---|---|---|---|---|---|---|---|---|---|
| v0 | all | 20 | 3,401,305 | 0.938 | 0.918 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| r2 | all | 20 | 675,502 | 21.752 | 0.947 | 0.803 | 0.219 | 18.858 | 1.721 | 0.466 | 2.183 |

## 3. The audit

At the audited nodes (Tier-1-open, chosen by a hash), each method's excess after 1, 2, 5, 10 and 20 pricings against the reference (E1n's node mode, certified, from the same inherited state).  Host: the variant whose tree the node is in.  Within i: the nodes the method cuts within i pricings (excess at most 0).  Reach: the share of the reference's cuts the method also cuts within 20 pricings.  Missed: the nodes the method cuts within 20 that the reference does not.  Pricings and ms: means over the nodes.

### 1215

| host | method | audited | reference cuts | within 1 | within 2 | within 5 | within 10 | within 20 | reach within 20 | missed | mean pricings | mean ms |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| v1 | r1 | 127 | 127 | 1 | 1 | 2 | 7 | 12 | 0.094 | 0 | 19.0 | 7.8 |
| v1 | r2 | 127 | 127 | 0 | 0 | 9 | 20 | 32 | 0.252 | 0 | 16.5 | 142.8 |
| v1 | r2c | 127 | 127 | 0 | 0 | 9 | 20 | 33 | 0.260 | 0 | 23.7 | 227.8 |
| r1 | r1 | 94 | 94 | 1 | 1 | 1 | 8 | 17 | 0.181 | 0 | 18.4 | 7.8 |
| r1 | r2 | 94 | 94 | 0 | 0 | 3 | 29 | 55 | 0.585 | 0 | 14.2 | 10.4 |
| r1 | r2c | 94 | 94 | 0 | 0 | 3 | 29 | 55 | 0.585 | 0 | 17.9 | 13.8 |
| r2 | r1 | 40 | 40 | 0 | 0 | 0 | 0 | 1 | 0.025 | 0 | 19.4 | 7.6 |
| r2 | r2 | 40 | 40 | 0 | 0 | 4 | 6 | 11 | 0.275 | 0 | 16.4 | 11.5 |
| r2 | r2c | 40 | 40 | 0 | 0 | 4 | 6 | 12 | 0.300 | 0 | 23.2 | 18.7 |
| r2c | r1 | 36 | 36 | 0 | 0 | 0 | 2 | 5 | 0.139 | 0 | 19.0 | 10.8 |
| r2c | r2 | 36 | 36 | 1 | 1 | 3 | 9 | 16 | 0.444 | 0 | 15.5 | 13.7 |
| r2c | r2c | 36 | 36 | 1 | 1 | 3 | 9 | 16 | 0.444 | 0 | 19.1 | 17.6 |

The reference alone: the nodes it cuts, its pricings and time, and why it stopped.

| host | audited | cuts | cut rate | mean pricings | mean ms | stops |
|---|---|---|---|---|---|---|
| v1 | 127 | 127 | 100.0% | 105.6 | 89.1 | Target 127 |
| r1 | 94 | 94 | 100.0% | 100.5 | 97.8 | Target 94 |
| r2 | 40 | 40 | 100.0% | 144.0 | 229.1 | Target 40 |
| r2c | 36 | 36 | 100.0% | 129.0 | 268.7 | Target 36 |

By the node's GF(2) excess: the share of the audited nodes the reference cuts, and the share each method cuts within 20 pricings.

| host | GF(2) excess | audited | reference | r1 | r2 | r2c |
|---|---|---|---|---|---|---|
| v1 | 0-20 | 38 | 100.0% | 13.2% | 42.1% | 42.1% |
| v1 | 21-50 | 55 | 100.0% | 12.7% | 27.3% | 29.1% |
| v1 | 51-100 | 27 | 100.0% | 0.0% | 3.7% | 3.7% |
| v1 | 101-200 | 7 | 100.0% | 0.0% | 0.0% | 0.0% |
| r1 | 0-20 | 23 | 100.0% | 21.7% | 78.3% | 78.3% |
| r1 | 21-50 | 40 | 100.0% | 25.0% | 67.5% | 67.5% |
| r1 | 51-100 | 24 | 100.0% | 8.3% | 37.5% | 37.5% |
| r1 | 101-200 | 7 | 100.0% | 0.0% | 14.3% | 14.3% |
| r2 | 0-20 | 4 | 100.0% | 0.0% | 50.0% | 50.0% |
| r2 | 21-50 | 14 | 100.0% | 0.0% | 35.7% | 42.9% |
| r2 | 51-100 | 15 | 100.0% | 0.0% | 13.3% | 13.3% |
| r2 | 101-200 | 7 | 100.0% | 14.3% | 28.6% | 28.6% |
| r2c | 0-20 | 5 | 100.0% | 40.0% | 80.0% | 80.0% |
| r2c | 21-50 | 10 | 100.0% | 0.0% | 60.0% | 60.0% |
| r2c | 51-100 | 14 | 100.0% | 21.4% | 28.6% | 28.6% |
| r2c | 101-200 | 7 | 100.0% | 0.0% | 28.6% | 28.6% |

### 1472

| host | method | audited | reference cuts | within 1 | within 2 | within 5 | within 10 | within 20 | reach within 20 | missed | mean pricings | mean ms |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r2 | r1 | 387 | 366 | 0 | 3 | 21 | 40 | 76 | 0.208 | 0 | 18.1 | 11.7 |
| r2 | r2 | 387 | 366 | 2 | 5 | 31 | 77 | 151 | 0.413 | 0 | 16.4 | 66.4 |
| r2 | r2c | 387 | 366 | 2 | 5 | 31 | 77 | 151 | 0.413 | 0 | 22.1 | 101.2 |

The reference alone: the nodes it cuts, its pricings and time, and why it stopped.

| host | audited | cuts | cut rate | mean pricings | mean ms | stops |
|---|---|---|---|---|---|---|
| r2 | 387 | 366 | 94.6% | 177.1 | 952.1 | Seconds 21, Target 366 |

By the node's GF(2) excess: the share of the audited nodes the reference cuts, and the share each method cuts within 20 pricings.

| host | GF(2) excess | audited | reference | r1 | r2 | r2c |
|---|---|---|---|---|---|---|
| r2 | 0-20 | 38 | 100.0% | 31.6% | 76.3% | 76.3% |
| r2 | 21-50 | 73 | 100.0% | 31.5% | 67.1% | 67.1% |
| r2 | 51-100 | 122 | 98.4% | 23.0% | 39.3% | 39.3% |
| r2 | 101-200 | 128 | 92.2% | 8.6% | 16.4% | 16.4% |
| r2 | over 200 | 26 | 65.4% | 7.7% | 15.4% | 15.4% |

## 4. The pilot's kill criterion

Fixed in advance by the design: stop E3 if the best variant's tree is less than 3 times smaller than v0's **and** the best method reaches less than 50% of the reference's cuts within 20 pricings.  Either alone is not a kill: a weak tree ratio with a strong audit points at the policy, a weak audit with a strong ratio at the method.  Best variant: the smallest tree among v1, r1, r2, r2c (a completed one before a partial one).  Best method: the highest reach among r1, r2, r2c, over the audited nodes of every host variant of the game.  **Only 1472 decides**; the other games show the trend.

| game | best variant | its nodes | tree ratio | below 3 | best method | reference cuts hit within 20 | reach | below 0.5 | verdict |
|---|---|---|---|---|---|---|---|---|---|
| 1215 | r2 (all, k 200) | 3,508 | 6.99 | no | r2c | 116 of 297 | 0.391 | yes | no kill (does not decide) |
| 1472 | r2 (all, k 20) | 675,502 | 5.04 | no | r2 | 151 of 366 | 0.413 | yes | no kill |

**Verdict at 1472: no kill** (a strong ratio with a weak audit points at the method).

## 5. Refits by excess and depth

From the per-node logs: the Tier-1-open nodes (Tier 1 left them uncut) by the GF(2) excess and by the depth of the node, and all together.  Refit cut rate: the share of the refits that cut their node.  Subtree: the size in the run's tree of the subtree under a refitted node (1 if the refit cut it), of all refits, and of those that did not cut.  Nodes in those subtrees: the share of the run's nodes that lie under a refitted node of the band, each counted once.

### 1215, r1 (all, k 20)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 673 | 673 | 41.2% | 15.3 | 5.7 | 8.9 | 24.6% |
| GF(2) excess | 21-50 | 949 | 949 | 20.0% | 17.9 | 10.9 | 13.4 | 53.3% |
| GF(2) excess | 51-100 | 763 | 763 | 8.0% | 19.2 | 26.4 | 28.6 | 83.0% |
| GF(2) excess | 101-200 | 166 | 166 | 0.0% | 20.0 | 174.8 | 174.8 | 99.2% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 20.0 | 484.0 | 484.0 | 10.4% |
| depth | 0-4 | 772 | 771 | 9.2% | 19.0 | 60.7 | 66.7 | 100.0% |
| depth | 5-9 | 1,735 | 1,735 | 25.2% | 17.2 | 10.0 | 13.0 | 63.4% |
| depth | 10-14 | 47 | 47 | 40.4% | 16.5 | 13.4 | 21.8 | 2.4% |
| depth | 15+ | 1 | 1 | 0.0% | 20.0 | 12.0 | 12.0 | 0.1% |
| all | all | 2,555 | 2,554 | 20.7% | 17.7 | 25.4 | 31.7 | 100.0% |

### 1215, r1 (all, k 50)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 552 | 552 | 54.5% | 31.6 | 5.1 | 9.9 | 20.7% |
| GF(2) excess | 21-50 | 876 | 876 | 28.0% | 40.4 | 9.7 | 13.1 | 50.9% |
| GF(2) excess | 51-100 | 733 | 733 | 11.1% | 44.8 | 24.0 | 26.8 | 82.9% |
| GF(2) excess | 101-200 | 166 | 166 | 0.0% | 44.6 | 153.1 | 153.1 | 99.3% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 21.0 | 437.7 | 437.7 | 10.8% |
| depth | 0-4 | 737 | 736 | 11.0% | 43.8 | 55.6 | 62.3 | 100.0% |
| depth | 5-9 | 1,552 | 1,552 | 34.0% | 38.3 | 9.1 | 13.3 | 60.6% |
| depth | 10-14 | 41 | 41 | 43.9% | 34.0 | 12.8 | 22.1 | 2.3% |
| depth | 15+ | 1 | 1 | 0.0% | 50.0 | 12.0 | 12.0 | 0.1% |
| all | all | 2,331 | 2,330 | 26.9% | 40.0 | 23.9 | 32.3 | 100.0% |

### 1215, r1 (all, k 100)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 514 | 514 | 58.0% | 52.0 | 4.8 | 10.0 | 19.7% |
| GF(2) excess | 21-50 | 842 | 842 | 32.4% | 66.8 | 9.3 | 13.3 | 49.7% |
| GF(2) excess | 51-100 | 728 | 728 | 14.7% | 69.2 | 22.8 | 26.5 | 82.7% |
| GF(2) excess | 101-200 | 166 | 166 | 0.6% | 65.4 | 145.3 | 146.2 | 99.3% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 21.0 | 401.0 | 401.0 | 10.5% |
| depth | 0-4 | 732 | 731 | 13.3% | 67.6 | 52.8 | 60.8 | 100.0% |
| depth | 5-9 | 1,483 | 1,483 | 37.9% | 62.6 | 8.8 | 13.6 | 59.7% |
| depth | 10-14 | 38 | 38 | 52.6% | 50.8 | 11.8 | 23.8 | 2.0% |
| depth | 15+ | 1 | 1 | 0.0% | 100.0 | 12.0 | 12.0 | 0.1% |
| all | all | 2,254 | 2,253 | 30.1% | 64.0 | 23.2 | 32.7 | 100.0% |

### 1215, r1 (all, k 200)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 493 | 493 | 61.5% | 67.2 | 4.6 | 10.4 | 18.5% |
| GF(2) excess | 21-50 | 838 | 838 | 34.1% | 82.0 | 9.1 | 13.2 | 49.7% |
| GF(2) excess | 51-100 | 724 | 724 | 15.7% | 80.2 | 22.2 | 26.2 | 82.5% |
| GF(2) excess | 101-200 | 166 | 166 | 0.6% | 71.3 | 140.4 | 141.3 | 99.3% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 21.0 | 388.3 | 388.3 | 10.5% |
| depth | 0-4 | 732 | 731 | 15.2% | 76.2 | 51.0 | 60.0 | 100.0% |
| depth | 5-9 | 1,455 | 1,455 | 39.5% | 78.3 | 8.7 | 13.7 | 59.3% |
| depth | 10-14 | 37 | 37 | 51.4% | 59.8 | 12.1 | 23.8 | 2.1% |
| depth | 15+ | 1 | 1 | 0.0% | 94.0 | 12.0 | 12.0 | 0.1% |
| all | all | 2,225 | 2,224 | 31.7% | 77.3 | 22.7 | 32.7 | 100.0% |

### 1215, r2 (all, k 20)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 145 | 145 | 76.6% | 9.2 | 3.3 | 10.9 | 8.1% |
| GF(2) excess | 21-50 | 412 | 412 | 59.7% | 12.2 | 5.8 | 12.9 | 33.5% |
| GF(2) excess | 51-100 | 552 | 552 | 31.2% | 16.5 | 13.6 | 19.2 | 79.7% |
| GF(2) excess | 101-200 | 158 | 158 | 8.2% | 19.4 | 76.4 | 83.2 | 99.1% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 20.0 | 232.0 | 232.0 | 12.6% |
| depth | 0-4 | 587 | 586 | 34.5% | 16.1 | 30.7 | 46.3 | 99.9% |
| depth | 5-9 | 669 | 669 | 49.8% | 13.4 | 7.3 | 13.6 | 48.0% |
| depth | 10-14 | 14 | 14 | 50.0% | 10.1 | 15.7 | 30.4 | 1.5% |
| depth | 15+ | 1 | 1 | 0.0% | 14.0 | 12.0 | 12.0 | 0.2% |
| all | all | 1,271 | 1,270 | 42.7% | 14.7 | 18.2 | 31.0 | 99.9% |

### 1215, r2 (all, k 50)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 101 | 101 | 87.1% | 10.0 | 2.7 | 14.2 | 5.9% |
| GF(2) excess | 21-50 | 311 | 311 | 68.5% | 15.4 | 5.3 | 14.7 | 29.7% |
| GF(2) excess | 51-100 | 481 | 481 | 43.7% | 28.6 | 11.2 | 19.1 | 77.3% |
| GF(2) excess | 101-200 | 153 | 153 | 15.7% | 40.4 | 60.4 | 71.4 | 99.4% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 50.0 | 177.3 | 177.3 | 13.0% |
| depth | 0-4 | 534 | 533 | 44.8% | 29.8 | 24.9 | 44.4 | 99.9% |
| depth | 5-9 | 503 | 503 | 57.9% | 19.6 | 7.1 | 15.5 | 44.8% |
| depth | 10-14 | 12 | 12 | 41.7% | 12.2 | 18.5 | 31.0 | 2.1% |
| depth | 15+ | 1 | 1 | 0.0% | 0.0 | 12.0 | 12.0 | 0.3% |
| all | all | 1,050 | 1,049 | 51.0% | 24.7 | 16.3 | 32.2 | 99.9% |

### 1215, r2 (all, k 100)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 97 | 97 | 84.5% | 10.2 | 2.8 | 12.9 | 6.1% |
| GF(2) excess | 21-50 | 303 | 303 | 69.3% | 15.6 | 5.3 | 15.1 | 29.7% |
| GF(2) excess | 51-100 | 463 | 463 | 44.7% | 31.1 | 11.2 | 19.5 | 77.2% |
| GF(2) excess | 101-200 | 153 | 153 | 20.3% | 48.9 | 57.7 | 72.1 | 99.3% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 67.0 | 166.0 | 166.0 | 12.7% |
| depth | 0-4 | 529 | 528 | 47.0% | 33.6 | 24.1 | 44.6 | 99.9% |
| depth | 5-9 | 478 | 478 | 57.9% | 20.7 | 7.2 | 15.7 | 45.0% |
| depth | 10-14 | 12 | 12 | 41.7% | 11.9 | 18.5 | 31.0 | 2.2% |
| depth | 15+ | 1 | 1 | 0.0% | 11.0 | 12.0 | 12.0 | 0.3% |
| all | all | 1,020 | 1,019 | 52.0% | 27.3 | 16.1 | 32.5 | 99.9% |

### 1215, r2 (all, k 200)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 94 | 94 | 84.0% | 10.5 | 2.9 | 12.9 | 6.1% |
| GF(2) excess | 21-50 | 299 | 299 | 69.6% | 15.9 | 5.4 | 15.3 | 29.8% |
| GF(2) excess | 51-100 | 458 | 458 | 45.2% | 31.7 | 11.2 | 19.6 | 77.1% |
| GF(2) excess | 101-200 | 150 | 150 | 20.0% | 49.8 | 58.0 | 72.2 | 99.3% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 67.0 | 158.0 | 158.0 | 12.1% |
| depth | 0-4 | 522 | 521 | 47.2% | 34.1 | 24.1 | 44.8 | 99.9% |
| depth | 5-9 | 470 | 470 | 58.1% | 21.2 | 7.2 | 15.9 | 45.0% |
| depth | 10-14 | 12 | 12 | 41.7% | 11.9 | 18.5 | 31.0 | 2.3% |
| depth | 15+ | 1 | 1 | 0.0% | 11.0 | 12.0 | 12.0 | 0.3% |
| all | all | 1,005 | 1,004 | 52.2% | 27.8 | 16.1 | 32.6 | 99.9% |

### 1215, r2 (excess:50, k 20)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 411 | 0 | - | - | - | - | 0.0% |
| GF(2) excess | 21-50 | 569 | 10 | 50.0% | 14.9 | 6.9 | 12.8 | 0.7% |
| GF(2) excess | 51-100 | 552 | 552 | 31.2% | 16.5 | 24.5 | 35.1 | 75.6% |
| GF(2) excess | 101-200 | 158 | 158 | 8.2% | 19.4 | 145.6 | 158.6 | 98.8% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 20.0 | 399.7 | 399.7 | 11.2% |
| depth | 0-4 | 645 | 437 | 21.5% | 18.0 | 73.9 | 93.8 | 100.0% |
| depth | 5-9 | 1,004 | 286 | 33.6% | 15.9 | 19.2 | 28.4 | 34.1% |
| depth | 10-14 | 44 | 0 | - | - | - | - | 0.0% |
| depth | 15+ | 1 | 0 | - | - | - | - | 0.0% |
| all | all | 1,694 | 723 | 26.3% | 17.2 | 52.2 | 70.5 | 100.0% |

### 1215, r2 (tier1:0.5, k 20)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 1,023 | 44 | 100.0% | 6.7 | 1.0 | - | 0.3% |
| GF(2) excess | 21-50 | 1,024 | 116 | 94.8% | 9.1 | 1.6 | 13.5 | 1.2% |
| GF(2) excess | 51-100 | 690 | 188 | 47.3% | 15.7 | 6.1 | 10.7 | 5.0% |
| GF(2) excess | 101-200 | 162 | 35 | 20.0% | 18.7 | 20.9 | 25.8 | 4.1% |
| GF(2) excess | over 200 | 4 | 0 | - | - | - | - | 0.0% |
| depth | 0-4 | 765 | 210 | 53.8% | 14.7 | 8.5 | 17.2 | 7.8% |
| depth | 5-9 | 2,033 | 171 | 78.9% | 10.9 | 2.0 | 5.5 | 1.9% |
| depth | 10-14 | 104 | 2 | 100.0% | 4.0 | 1.0 | - | 0.0% |
| depth | 15+ | 1 | 0 | - | - | - | - | 0.0% |
| all | all | 2,903 | 383 | 65.3% | 12.9 | 5.5 | 14.0 | 8.9% |

### 1215, r2c (all, k 20)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 105 | 105 | 83.8% | 9.0 | 2.8 | 12.1 | 6.5% |
| GF(2) excess | 21-50 | 303 | 303 | 71.3% | 13.0 | 4.9 | 14.5 | 26.6% |
| GF(2) excess | 51-100 | 504 | 504 | 40.3% | 20.7 | 11.6 | 18.8 | 78.1% |
| GF(2) excess | 101-200 | 154 | 154 | 12.3% | 27.5 | 61.9 | 70.5 | 99.1% |
| GF(2) excess | over 200 | 4 | 3 | 0.0% | 30.0 | 191.7 | 191.7 | 13.3% |
| depth | 0-4 | 536 | 535 | 43.4% | 22.2 | 25.6 | 44.4 | 99.9% |
| depth | 5-9 | 520 | 520 | 55.4% | 14.6 | 7.3 | 15.2 | 46.1% |
| depth | 10-14 | 13 | 13 | 46.2% | 12.1 | 17.5 | 31.7 | 2.2% |
| depth | 15+ | 1 | 1 | 0.0% | 3.0 | 12.0 | 12.0 | 0.3% |
| all | all | 1,070 | 1,069 | 49.2% | 18.4 | 16.6 | 31.7 | 99.9% |

### 1472, r2 (all, k 20)

| by | band | Tier-1-open nodes | refits | refit cut rate | mean pricings | mean subtree | mean subtree, uncut refits | nodes in those subtrees |
|---|---|---|---|---|---|---|---|---|
| GF(2) excess | 0-20 | 16,675 | 16,675 | 71.7% | 12.4 | 2.8 | 7.4 | 6.2% |
| GF(2) excess | 21-50 | 34,801 | 34,801 | 57.8% | 14.6 | 4.6 | 9.5 | 18.4% |
| GF(2) excess | 51-100 | 64,427 | 64,427 | 37.8% | 16.9 | 8.3 | 12.7 | 48.8% |
| GF(2) excess | 101-200 | 64,281 | 64,281 | 17.4% | 18.7 | 22.5 | 27.0 | 86.7% |
| GF(2) excess | over 200 | 10,083 | 10,082 | 1.2% | 19.9 | 387.2 | 391.9 | 100.0% |
| depth | 0-4 | 2,697 | 2,696 | 20.0% | 18.6 | 1001.5 | 1251.5 | 100.0% |
| depth | 5-9 | 99,297 | 99,297 | 33.3% | 17.2 | 28.2 | 41.8 | 98.5% |
| depth | 10-14 | 85,708 | 85,708 | 38.3% | 16.5 | 6.8 | 10.4 | 40.2% |
| depth | 15+ | 2,565 | 2,565 | 49.6% | 15.2 | 3.4 | 5.8 | 0.9% |
| all | all | 190,267 | 190,266 | 35.6% | 16.9 | 32.0 | 49.2 | 100.0% |

## 6. Knuth

Paired estimates of the tree sizes from the same dives: the GF(2) tree's (that of v0) and the variant's (A1), and the ratio of their means (the estimated tree ratio), each with the bootstrap interval of the mean at the level shown.  Beside them the exact counts, where a completed tree line of the same game exists: v0's nodes for the GF(2) estimate, the variant's for the A1 estimate, their ratio for the ratio.  In: whether the exact figure lies in the interval.

| game | variant | policy | k | dives | level | GF(2) estimate [interval] | v0 nodes | in | A1 estimate [interval] | variant nodes | in | ratio [interval] | exact ratio | in |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1472 | v1 | all | 20 | 2,000 | 90.0% | 741,585 [389,979, 1,166,791] | 3,401,305 | no | 734,262 [382,779, 1,159,432] | - | ? | 1.010 [1.005, 1.021] | - | ? |
| 1472 | r1 | all | 20 | 2,000 | 90.0% | 741,585 [389,979, 1,166,791] | 3,401,305 | no | 602,456 [264,880, 1,014,840] | - | ? | 1.231 [1.100, 1.577] | - | ? |
| 1472 | r2 | all | 20 | 2,000 | 90.0% | 741,585 [389,979, 1,166,791] | 3,401,305 | no | 341,789 [137,227, 637,208] | 675,502 | no | 2.170 [1.484, 4.120] | 5.04 | no |
| 1472 | r2c | all | 20 | 2,000 | 90.0% | 741,585 [389,979, 1,166,791] | 3,401,305 | no | 482,741 [154,189, 885,189] | - | ? | 1.536 [1.249, 2.757] | - | ? |

