
### The probe's fixtures (nodes_fixtures.jsonl)

| n | nodes | open | promoted (median) | relevant (median) | literal cover, all of C | literal cover, relevant | prime exchange cover | symmetry rule (hopeless / bound) | any proof | sampled upper estimate | twins |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 936 | 300 | 300 | 13 | 106 | 0% | 1% | 1% | 3% / 9% | 10% | 14% | 0 |
| 1215 | 300 | 300 | 16 | 189 | 0% | 0% | 0% | 1% / 2% | 2% | 3% | 0 |

### The solver's proofs, nodes sampled over their steps (nodes_solver.jsonl)

| n | nodes | open | promoted (median) | relevant (median) | literal cover, all of C | literal cover, relevant | prime exchange cover | symmetry rule (hopeless / bound) | any proof | sampled upper estimate | twins |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 900 | 98 | 98 | 22 | 101 | 0% | 0% | 0% | 0% / 0% | 0% | 0% | 0 |
| 936 | 99 | 99 | 34 | 39 | 0% | 0% | 5% | 12% / 25% | 25% | 46% | 0 |
| 950 | 60 | 60 | 30 | 44 | 0% | 2% | 12% | 12% / 30% | 32% | 53% | 0 |
| 1000 | 60 | 60 | 31 | 46 | 2% | 2% | 7% | 7% / 43% | 43% | 50% | 0 |
| 1100 | 97 | 97 | 42 | 41 | 3% | 3% | 3% | 10% / 36% | 36% | 39% | 0 |
| 1184 | 100 | 100 | 41 | 66 | 0% | 4% | 0% | 2% / 4% | 8% | 20% | 0 |
| 1215 | 40 | 40 | 46 | 74 | 2% | 2% | 2% | 2% / 10% | 10% | 12% | 0 |
| 1250 | 40 | 40 | 44 | 81 | 0% | 5% | 8% | 8% / 20% | 25% | 35% | 0 |

### The same, incumbent P* + 100 (nodes_solver_d100.jsonl)

| n | nodes | open | promoted (median) | relevant (median) | literal cover, all of C | literal cover, relevant | prime exchange cover | symmetry rule (hopeless / bound) | any proof | sampled upper estimate | twins |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1000 | 60 | 60 | 26 | 60 | 5% | 5% | 15% | 15% / 28% | 28% | 52% | 0 |
| 1250 | 60 | 60 | 36 | 87 | 0% | 0% | 2% | 3% / 8% | 8% | 15% | 0 |

### The same, incumbent P* + 200 (nodes_solver_d200.jsonl)

| n | nodes | open | promoted (median) | relevant (median) | literal cover, all of C | literal cover, relevant | prime exchange cover | symmetry rule (hopeless / bound) | any proof | sampled upper estimate | twins |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1000 | 52 | 52 | 20 | 62 | 0% | 0% | 0% | 0% / 0% | 0% | 50% | 0 |
| 1250 | 60 | 60 | 32 | 78 | 0% | 0% | 0% | 0% / 12% | 12% | 27% | 0 |

### The GF(2) branch and bound's trees (nodes_gf2.jsonl)

| n | nodes | open | promoted (median) | relevant (median) | literal cover, all of C | literal cover, relevant | prime exchange cover | symmetry rule (hopeless / bound) | any proof | sampled upper estimate | twins |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 930 | 195 | 195 | 16 | 179 | 34% | 34% | 32% | 46% / 53% | 54% | 55% | 0 |
| 990 | 210 | 210 | 19 | 184 | 20% | 20% | 20% | 20% / 25% | 25% | 25% | 0 |
| 1020 | 164 | 163 | 15 | 202 | 1% | 1% | 8% | 15% / 16% | 16% | 21% | 0 |
| 1210 | 133 | 132 | 14 | 266 | 0% | 0% | 0% | 1% / 7% | 7% | 10% | 0 |
| 1230 | 160 | 160 | 16 | 271 | 18% | 18% | 18% | 18% / 19% | 19% | 20% | 0 |
| 1300 | 171 | 171 | 18 | 288 | 31% | 31% | 31% | 31% / 35% | 35% | 39% | 0 |

### Two-member exchanges, the solver's proofs (nodes_pairs_solver.jsonl)

| n | nodes | sampled: one for one | two for one or two | either, or a prime exchange or the symmetry rule |
|---|---|---|---|---|
| 936 | 99 | 36% | 36% | 49% |
| 1184 | 100 | 16% | 34% | 34% |
| 1250 | 40 | 22% | 35% | 48% |

### Two-member exchanges, the GF(2) trees (nodes_pairs_gf2.jsonl)

| n | nodes | sampled: one for one | two for one or two | either, or a prime exchange or the symmetry rule |
|---|---|---|---|---|
| 1020 | 55 | 13% | 9% | 25% |
| 1210 | 45 | 7% | 0% | 7% |
| 1300 | 57 | 42% | 21% | 46% |

### The root's prime symmetries by band (symmetries.jsonl)

| n | games | candidates | classes of primes | largest class | candidates moved | log10 of the group order |
|---|---|---|---|---|---|---|
| 851-900 | 50 | 211 | 3.8 | 4.3 | 23.9 (11%) | 4.5 |
| 901-950 | 50 | 223 | 4.7 | 5.4 | 28.3 (13%) | 4.9 |
| 951-1000 | 50 | 236 | 4.1 | 6.1 | 28.5 (12%) | 5.1 |
| 1001-1050 | 50 | 252 | 3.7 | 6.7 | 27.2 (11%) | 5.4 |
| 1051-1100 | 50 | 266 | 4.6 | 6.3 | 28.3 (11%) | 5.2 |
| 1101-1150 | 50 | 277 | 5.7 | 6.2 | 38.8 (14%) | 5.7 |
| 1151-1200 | 50 | 289 | 4.6 | 6.7 | 28.9 (10%) | 5.9 |
| 1201-1250 | 50 | 304 | 5.5 | 6.3 | 40.2 (13%) | 6.4 |
| 1251-1300 | 50 | 322 | 5.8 | 6.7 | 41.5 (13%) | 6.7 |
| 1301-1350 | 50 | 336 | 5.9 | 5.9 | 48.4 (14%) | 7.4 |
| 1351-1400 | 2 | 344 | 6.0 | 6.0 | 44.0 (13%) | 7.2 |

### The pilots: nodes with the rule against without (pilots/)

| search, rule | games | nodes, geometric ratio | total ratio | fewest | most | slope of log ratio on log nodes | time |
|---|---|---|---|---|---|---|---|
| solver, hopeless | 22 | 0.962 | 0.960 | 0.85 (950) | 1.07 (1040) | -0.002 | 52 -> 59 s |
| solver, matching bound of the constraint | 22 | 0.878 | 0.829 | 0.67 (1000) | 1.16 (940) | -0.028 | 52 -> 99 s |
| GF(2) B&B, hopeless | 43 | 0.948 | 0.925 | 0.71 (940) | 1.00 (1130) | -0.016 | 139 -> 135 s |
| GF(2) B&B, Lagrangian of the constraint | 43 | 0.859 | 0.826 | 0.57 (930) | 0.98 (1100) | -0.030 | 139 -> 134 s |

### Margin scans of the GF(2) pilot (pilots/gf2_delta.jsonl: δ = 0, 25, 50, 100 in that order)

| n | δ | nodes without | with | ratio |
|---|---|---|---|---|
| 930 | 0 | 11677 | 6621 | 0.567 |
| 930 | 25 | 4384 | 2614 | 0.596 |
| 930 | 50 | 1341 | 824 | 0.614 |
| 930 | 100 | 89 | 82 | 0.921 |
| 990 | 0 | 42017 | 32392 | 0.771 |
| 990 | 25 | 24361 | 18819 | 0.773 |
| 990 | 50 | 13735 | 10587 | 0.771 |
| 990 | 100 | 4308 | 3409 | 0.791 |
| 1300 | 0 | 17136 | 11791 | 0.688 |
| 1300 | 25 | 7950 | 5387 | 0.678 |
| 1300 | 50 | 3664 | 2454 | 0.670 |
| 1300 | 100 | 452 | 340 | 0.752 |

### Soundness runs

* GF(2) pilot, incumbent P* - 1: 880 games, found the optimum in 880, the rule cut in 353
* solver pilot, incumbent P* - 1: 880 games, found the optimum in 880, the rule cut in 573
* symcheck: 1170 games with pairs; 22 boards enumerated (97839 promotable sets), the constrained best equal to P* in all
