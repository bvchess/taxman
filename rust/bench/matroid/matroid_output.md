## 1. Gaps at the root

| n | games | matching gap | Lagrangian gap | GF(2) gap | GF(2) / matching | GF(2) / Lagrangian | ms |
|---|---|---|---|---|---|---|---|
| 851-900 | 18 | 432 | 112 | 88 | 0.20 | 0.79 | 0.42 |
| 901-950 | 43 | 619 | 185 | 147 | 0.24 | 0.80 | 0.43 |
| 951-1000 | 43 | 749 | 261 | 172 | 0.23 | 0.66 | 0.52 |
| 1001-1050 | 42 | 795 | 362 | 156 | 0.20 | 0.43 | 0.64 |
| 1051-1100 | 42 | 546 | 206 | 81 | 0.15 | 0.39 | 0.75 |
| 1101-1150 | 45 | 522 | 162 | 107 | 0.21 | 0.66 | 0.72 |
| 1151-1200 | 43 | 629 | 154 | 108 | 0.17 | 0.70 | 0.73 |
| 1201-1250 | 42 | 1091 | 426 | 148 | 0.14 | 0.35 | 1.02 |
| 1251-1300 | 43 | 1277 | 471 | 185 | 0.14 | 0.39 | 1.03 |
| 1301-1310 | 7 | 1445 | 554 | 154 | 0.11 | 0.28 | 1.15 |
| 1350-1352 | 3 | 1255 | 465 | 272 | 0.22 | 0.59 | 1.26 |

The cap `max_total` binds at 0 roots.  Growth per 100 in n over the 361 games of 880-1300:

* matching gap: +142.5 ± 9.4 (R² 0.39), mean 760
* Lagrangian gap: +56.0 ± 5.2 (R² 0.24), mean 269
* GF(2) gap: +4.6 ± 2.4 (R² 0.01), mean 136
* share of the matching gap the GF(2) bound closes: mean 0.816, +0.0232 ± 0.0023 per 100 in n
* share of the Lagrangian gap the GF(2) bound closes: mean 0.425, +0.0918 ± 0.0089 per 100 in n

## 2. A plain branch and bound with the GF(2) bound against the solver's proofs

| n | games | log10 nodes, solver | log10 nodes, GF(2) B&B | ratio (geometric) | s, solver | s, GF(2) B&B |
|---|---|---|---|---|---|---|
| 851-900 | 6 | 3.78 | 2.59 | 15 | 0.5 | 1.0 |
| 901-950 | 16 | 4.56 | 3.48 | 12 | 12.8 | 22.8 |
| 951-1000 | 16 | 5.21 | 3.74 | 30 | 107.0 | 120.1 |
| 1001-1050 | 19 | 5.62 | 3.99 | 43 | 163.8 | 160.1 |
| 1051-1100 | 18 | 4.46 | 2.77 | 49 | 12.6 | 8.0 |
| 1101-1150 | 17 | 3.82 | 2.72 | 13 | 1.5 | 6.4 |
| 1151-1200 | 16 | 4.52 | 2.90 | 42 | 13.2 | 32.8 |
| 1201-1250 | 19 | 6.13 | 3.90 | 171 | 872.2 | 168.6 |
| 1251-1300 | 16 | 6.90 | 4.11 | 613 | 1750.5 | 240.5 |
| 1301-1310 | 1 | 7.14 | 4.24 | 794 | 157.5 | 16.3 |
| 1350-1352 | 2 | 6.88 | 4.79 | 124 | 315.9 | 128.3 |

146 games; total seconds: solver 3408, GF(2) B&B 905.

* log10 nodes of the solver per 100 in n: 0.476 ± 0.066 (R² 0.27)
* log10 nodes of the GF(2) B&B per 100 in n: 0.138 ± 0.048 (R² 0.05)
* log10 nodes of the GF(2) B&B against the root's GF(2) gap: 0.01106 ± 0.00049 per unit (R² 0.78)
* log10 nodes of the GF(2) B&B against the root's Lagrangian gap: 0.00428 ± 0.00025 per unit (R² 0.67)
* log10 nodes of the GF(2) B&B against the root's matching gap: 0.00194 ± 0.00014 per unit (R² 0.56)
* largest GF(2) B&B trees (n, nodes, solver's nodes, GF(2) gap): 1287 87841 20972332 282, 992 77567 2099730 318, 1350 77283 17504520 283, 996 68216 3353839 318, 1352 49144 3343973 251, 1024 46847 1597518 199
* the same branch and bound with the matching bound, n = 880: unfinished after 200000 nodes, 12.7 s
* the same branch and bound with the matching bound, n = 936: unfinished after 200000 nodes, 16.8 s

## 3. At the probe's search nodes

| n | nodes | matching cuts | GF(2) cuts | GF(2) ms | of every 3rd: node-fitted Lagrangian cuts | either | true gap, matching | true gap, GF(2) | forced exactly | by the split |
|---|---|---|---|---|---|---|---|---|---|---|
| 936 | 300 | 0 | 163 | 0.33 | 66 of 100 (GF(2): 55) | 69 | 317 | 154 (300 nodes) | 26.1 a node | 66% |
| 1215 | 300 | 0 | 167 | 0.68 | 50 of 100 (GF(2): 59) | 67 | 548 | 272 (98 nodes) | 28.0 a node | 60% |

## 4. What the GF(2) bound misses

* n = 936: GF(2) gap 132, with the Lagrangian over 48 nogoods 118; the GF(2) optimum's conflicts have 14-17 members
* n = 1000: GF(2) gap 214, with the Lagrangian over 80 nogoods 137; the GF(2) optimum's conflicts have 7-18 members
* n = 1215: GF(2) gap 213, with the Lagrangian over 57 nogoods 198; the GF(2) optimum's conflicts have 13-18 members
* n = 1254: GF(2) gap 204, with the Lagrangian over 56 nogoods 166; the GF(2) optimum's conflicts have 11-18 members
* n = 1300: GF(2) gap 151, with the Lagrangian over 56 nogoods 129; the GF(2) optimum's conflicts have 15-17 members
* n = 936: frame 1: 50 movers (14 promoted, 31 with 3+ factors) over 50 factors, Lorea misses it; frame 2: 27 movers (6 promoted, 9 with 3+ factors) over 29 factors, Lorea misses it
* n = 1000: frame 1: 54 movers (15 promoted, 37 with 3+ factors) over 54 factors, Lorea misses it; frame 2: 34 movers (7 promoted, 12 with 3+ factors) over 34 factors, Lorea misses it
* n = 1100: frame 1: 46 movers (12 promoted, 26 with 3+ factors) over 49 factors, Lorea misses it; frame 2: 44 movers (8 promoted, 23 with 3+ factors) over 44 factors, Lorea misses it
* n = 1215: frame 1: 57 movers (15 promoted, 31 with 3+ factors) over 58 factors, Lorea misses it; frame 2: 43 movers (9 promoted, 22 with 3+ factors) over 43 factors, Lorea misses it; frame 3: 18 movers (4 promoted, 6 with 3+ factors) over 19 factors, Lorea misses it
* n = 1254: frame 1: 61 movers (15 promoted, 35 with 3+ factors) over 64 factors, Lorea misses it; frame 2: 45 movers (10 promoted, 25 with 3+ factors) over 45 factors, Lorea misses it; frame 3: 18 movers (5 promoted, 6 with 3+ factors) over 19 factors, Lorea misses it
* n = 1300: frame 1: 65 movers (16 promoted, 36 with 3+ factors) over 65 factors, Lorea misses it; frame 2: 30 movers (5 promoted, 12 with 3+ factors) over 30 factors, Lorea misses it

## 5. The model-free bound on opt(n)

| n | games | gap mean | median | max | exact | ms |
|---|---|---|---|---|---|---|
| 2-100 | 74 | 0.5 | 0 | 18 | 70 | 0 |
| 101-200 | 79 | 0.9 | 0 | 24 | 64 | 2 |
| 201-300 | 84 | 2.8 | 0 | 19 | 44 | 8 |
| 301-400 | 84 | 15.0 | 12 | 35 | 11 | 18 |
| 401-500 | 83 | 20.7 | 19 | 51 | 0 | 34 |
| 501-600 | 86 | 43.6 | 46 | 106 | 5 | 57 |
| 601-700 | 84 | 66.4 | 59 | 139 | 0 | 90 |
| 701-800 | 86 | 78.1 | 90 | 178 | 0 | 132 |
| 801-900 | 85 | 86.0 | 92 | 178 | 0 | 186 |
| 901-1000 | 86 | 159.4 | 146 | 323 | 0 | 252 |
| 1001-1100 | 84 | 118.4 | 101 | 223 | 0 | 334 |
| 1101-1200 | 88 | 107.7 | 95 | 201 | 0 | 429 |
| 1201-1300 | 85 | 167.0 | 176 | 282 | 0 | 545 |

The model-free gap equals the frame model's GF(2) gap at 361 of the 361 games both have.  Its growth per 100 in n:

* 2-1300: +14.3 ± 0.3 (R² 0.65)
* 300-1000: +21.7 ± 0.8 (R² 0.57)
* 600-1300: +13.8 ± 1.0 (R² 0.24)
* 880-1300: +4.6 ± 2.4 (R² 0.01)
