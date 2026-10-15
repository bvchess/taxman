# The fields study

roots: `lite.jsonl`, 13 records; nodes: `nodes.jsonl`, 1200; census: `census.jsonl`, 8129.

## 1. Per arm, by band of 100 n

Share: the fraction of the GF(2) gap the bound closes; its mean and median are over the converged roots (bracket within 1% of the gap).  Bracket and seconds are means over the roots with gf2 > opt.

### f1cores

| n | roots | gf2 > opt | converged | mean share | median share | mean bracket | mean s |
|---|---|---|---|---|---|---|---|
| 900-999 | 1 | 1 | 1 | 0.975 | 0.975 | 0.30 | 0.6 |
| 1000-1099 | 1 | 1 | 1 | 0.435 | 0.435 | 0.73 | 5.9 |
| 1100-1199 | 1 | 1 | 1 | 0.545 | 0.545 | 0.64 | 0.9 |
| 1200-1299 | 1 | 1 | 1 | 0.502 | 0.502 | 0.93 | 4.6 |
| 1300-1399 | 1 | 1 | 1 | 0.311 | 0.311 | 0.37 | 4.7 |
| 1400-1499 | 1 | 1 | 0 | - | - | 47707.77 | 120.0 |
| 1500-1599 | 1 | 1 | 0 | - | - | 55188.56 | 120.3 |
| 1600-1699 | 1 | 1 | 1 | 0.731 | 0.731 | 1.68 | 5.4 |
| 1700-1799 | 1 | 1 | 0 | - | - | 67649.01 | 120.1 |
| 1800-1899 | 1 | 1 | 0 | - | - | 76686.17 | 120.0 |
| 1900-1999 | 1 | 1 | 1 | 0.519 | 0.519 | 1.13 | 6.2 |
| 2000-2099 | 1 | 1 | 0 | - | - | 95097.27 | 120.0 |
| 2100-2199 | 1 | 1 | 1 | 0.662 | 0.662 | 1.76 | 25.5 |
| all | 13 | 13 | 8 | 0.585 | 0.532 | 26333.56 | 50.3 |

## 2. The kill criteria on f1cores

Fixed in advance by the design.  Only converged roots with a defined share count; K1 and K2 use the roots with n ≥ 1700.

* unconverged: 5 of 13 (38%) of the roots with a defined share (inconclusive above 20%); of those with n ≥ 1700: 3 of 5 (60%)
* K1: mean share over the 2 counted roots with n ≥ 1700: 0.590 (killed below 0.25)
* K2: slope of the share against n over the same roots: no fit (under three roots) (killed at slope ≤ -2 and t ≤ -2)
* passes strongly: mean share ≥ 0.5 and slope above -1 point per 100 n

**Verdict for f1cores: inconclusive**

Every arm, the same numbers:

| arm | roots, gf2 > opt | counted | unconverged | counted, n ≥ 1700 | unconverged, n ≥ 1700 | mean share, n ≥ 1700 | slope ± se, points per 100 n | t |
|---|---|---|---|---|---|---|---|---|
| f1cores | 13 | 8 | 38.5% | 2 | 60.0% | 0.590 | - | - |

The same over all roots with a defined share, converged or not (the bound is valid either way, and the share of an unconverged root is at most the LP's):

| arm | roots | mean share | roots, n ≥ 1700 | mean share, n ≥ 1700 | slope ± se, points per 100 n | t |
|---|---|---|---|---|---|---|
| f1cores | 13 | 0.553 | 5 | 0.480 | +7.45 ± 5.46 | +1.4 |

**E1-lite criterion** (f1cores): mean share over the 3 converged of the 7 roots with n ≥ 1500: 0.637 (early kill below 0.15 with all of them converged): no early kill

## 3. Arms side by side

Share of the GF(2) gap closed, per root; `*` marks a root that did not converge.

| n | gap | f1cores |
|---|---|---|
| 900 | 119 | 0.975 |
| 1000 | 214 | 0.435 |
| 1100 | 178 | 0.545 |
| 1200 | 201 | 0.502 |
| 1300 | 151 | 0.311 |
| 1400 | 395 | 0.534* |
| 1500 | 401 | 0.748* |
| 1600 | 349 | 0.731 |
| 1700 | 379 | 0.493* |
| 1800 | 507 | 0.160* |
| 1900 | 243 | 0.519 |
| 2000 | 491 | 0.568* |
| 2100 | 526 | 0.662 |

The ratio to f1cores needs roots of f1cores and of a fixed arm (f1, f1r1, f1r3): skipped.

## 4. Signings and cores

Over the roots with gf2 > opt.  Signings per frame: core-derived signings (`core_signings`, the fixed ones left out) per frame that carries an extra matroid; cores per signing: pooled over the signings; capped: roots with 8 core signings (the cap) on some frame.

### f1cores

| n | roots | signings per frame | cores per signing | capped | cores found | assigned | skipped (unplaced) | repeated | GF(3) loops |
|---|---|---|---|---|---|---|---|---|---|
| 900-999 | 1 | 5.67 | 2.76 | 1 | 47.0 | 47.0 | 0.0 | 248.0 | 0.0 |
| 1000-1099 | 1 | 5.33 | 3.00 | 1 | 106.0 | 48.0 | 58.0 | 255.0 | 0.0 |
| 1100-1199 | 1 | 5.33 | 1.75 | 1 | 65.0 | 28.0 | 37.0 | 105.0 | 0.0 |
| 1200-1299 | 1 | 4.75 | 2.47 | 1 | 93.0 | 47.0 | 46.0 | 304.0 | 0.0 |
| 1300-1399 | 1 | 3.67 | 1.64 | 1 | 44.0 | 18.0 | 26.0 | 105.0 | 0.0 |
| 1400-1499 | 1 | 6.33 | 2.00 | 1 | 64.0 | 38.0 | 26.0 | 117.0 | 0.0 |
| 1500-1599 | 1 | 8.00 | 4.12 | 1 | 175.0 | 99.0 | 76.0 | 533.0 | 0.0 |
| 1600-1699 | 1 | 8.00 | 3.17 | 1 | 146.0 | 76.0 | 70.0 | 518.0 | 0.0 |
| 1700-1799 | 1 | 6.00 | 3.61 | 1 | 123.0 | 65.0 | 58.0 | 378.0 | 0.0 |
| 1800-1899 | 1 | 2.50 | 1.00 | 0 | 10.0 | 10.0 | 0.0 | 0.0 | 0.0 |
| 1900-1999 | 1 | 8.00 | 2.79 | 1 | 102.0 | 67.0 | 35.0 | 453.0 | 0.0 |
| 2000-2099 | 1 | 6.33 | 3.42 | 1 | 90.0 | 65.0 | 25.0 | 419.0 | 0.0 |
| 2100-2199 | 1 | 6.25 | 6.76 | 1 | 288.0 | 169.0 | 119.0 | 1094.0 | 0.0 |
| all | 13 | 5.76 | 3.21 | 12 | 104.1 | 59.8 | 44.3 | 348.4 | 0.0 |

## 5. The LP's support (f1cores)

Over the roots with gf2 > opt.  Promotable: the support columns whose set can be promoted whole; unpromotable mass: the multiplier mass on the others.

| n | roots | support size | share promotable | mass unpromotable |
|---|---|---|---|---|
| 900-999 | 1 | 6.0 | 0.333 | 0.833 |
| 1000-1099 | 1 | 16.0 | 0.000 | 1.000 |
| 1100-1199 | 1 | 7.0 | 0.000 | 1.000 |
| 1200-1299 | 1 | 10.0 | 0.000 | 1.000 |
| 1300-1399 | 1 | 6.0 | 0.000 | 1.000 |
| 1400-1499 | 1 | 20.0 | 0.000 | 1.000 |
| 1500-1599 | 1 | 34.0 | 0.000 | 1.000 |
| 1600-1699 | 1 | 7.0 | 0.000 | 1.000 |
| 1700-1799 | 1 | 20.0 | 0.000 | 1.000 |
| 1800-1899 | 1 | 9.0 | 0.000 | 1.000 |
| 1900-1999 | 1 | 13.0 | 0.000 | 1.000 |
| 2000-2099 | 1 | 46.0 | 0.000 | 1.000 |
| 2100-2199 | 1 | 17.0 | 0.000 | 1.000 |
| all | 13 | 16.2 | 0.026 | 0.987 |

## 6. Cost

Means over the roots with gf2 > opt.  MI: matroid intersection; the exact checks are the separation's.

### f1cores

| n | roots | pricings, MI | pricings, greedy | cuts, cheap | cuts, exact | exact checks | pivots, master | pivots, exact checks | repairs | s, total | s, intersections | s, simplex |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 900-999 | 1 | 307 | 3697 | 175 | 109 | 304 | 6140 | 33969 | 0.0 | 0.64 | 0.03 | 0.16 |
| 1000-1099 | 1 | 245 | 3231 | 208 | 157 | 494 | 10288 | 474671 | 0.0 | 5.89 | 0.04 | 0.35 |
| 1100-1199 | 1 | 259 | 3702 | 134 | 145 | 361 | 4495 | 58777 | 0.0 | 0.90 | 0.04 | 0.11 |
| 1200-1299 | 1 | 318 | 4983 | 116 | 245 | 570 | 11879 | 352085 | 0.0 | 4.56 | 0.06 | 0.51 |
| 1300-1399 | 1 | 171 | 2008 | 127 | 133 | 348 | 8201 | 332149 | 0.0 | 4.74 | 0.05 | 0.17 |
| 1400-1499 | 1 | 646 | 10163 | 896 | 137 | 378 | 107688 | 5021665 | 4.0 | 120.00 | 0.24 | 13.12 |
| 1500-1599 | 1 | 907 | 21067 | 1869 | 264 | 486 | 183517 | 2932839 | 0.0 | 120.31 | 0.46 | 54.72 |
| 1600-1699 | 1 | 666 | 11146 | 269 | 339 | 535 | 30882 | 130310 | 0.0 | 5.43 | 0.29 | 2.44 |
| 1700-1799 | 1 | 463 | 6819 | 357 | 101 | 147 | 704212 | 87051 | 1.0 | 120.06 | 0.22 | 118.04 |
| 1800-1899 | 1 | 158 | 1064 | 107 | 0 | 0 | 13252639 | 0 | 0.0 | 120.00 | 0.06 | 119.85 |
| 1900-1999 | 1 | 333 | 4809 | 145 | 245 | 561 | 13517 | 344778 | 0.0 | 6.22 | 0.16 | 0.53 |
| 2000-2099 | 1 | 891 | 17619 | 463 | 692 | 1007 | 79817 | 4834282 | 0.0 | 120.00 | 0.67 | 8.91 |
| 2100-2199 | 1 | 798 | 18628 | 270 | 447 | 1092 | 55918 | 963946 | 0.0 | 25.49 | 0.67 | 6.53 |
| all | 13 | 474 | 8380 | 395 | 232 | 483 | 1113015 | 1197425 | 0.4 | 50.33 | 0.23 | 25.03 |

### Time per pricing in the intersections (f1cores)

| n | roots | ms per pricing (total ms_mi / total pricings_mi) |
|---|---|---|
| 900-999 | 1 | 0.094 |
| 1000-1099 | 1 | 0.171 |
| 1100-1199 | 1 | 0.174 |
| 1200-1299 | 1 | 0.198 |
| 1300-1399 | 1 | 0.269 |
| 1400-1499 | 1 | 0.367 |
| 1500-1599 | 1 | 0.503 |
| 1600-1699 | 1 | 0.443 |
| 1700-1799 | 1 | 0.484 |
| 1800-1899 | 1 | 0.392 |
| 1900-1999 | 1 | 0.468 |
| 2000-2099 | 1 | 0.754 |
| 2100-2199 | 1 | 0.843 |
| all | 13 | 0.487 |

## 7. Nodes

A node is cut when the mode's bound (the lower of it and the node's GF(2) bound) is at most the target.  Open: nodes the GF(2) bound alone leaves uncut (gf2 > target).

| n | mode | nodes | cut by GF(2) | cut | cut among the open | mean excess, uncut | mean ms |
|---|---|---|---|---|---|---|---|
| 936 | root | 300 | 163 | 212 | 49 of 137 (36%) | 24.5 | 2.8 |
| 936 | node | 300 | 163 | 297 | 134 of 137 (98%) | 14.3 | 105.7 |
| 1215 | root | 300 | 167 | 209 | 42 of 133 (32%) | 58.6 | 5.2 |
| 1215 | node | 300 | 167 | 281 | 114 of 133 (86%) | 31.4 | 577.3 |

The recorded figures (rust/docs/EXPERIMENTS.md): GF(2) alone, "Matroid intersection bounds (2026-10)"; the LP over all conflicts, "How far can nogoods go (2026-10)"; the per-frame bound at every 10th node (the root's multipliers to 20 steps of the node's own), "The per-frame bound (2026-10)".  Here: cuts of the nodes in the data.

| n | GF(2), recorded | GF(2), here | all conflicts, recorded | per-frame, recorded | root mode, here | node mode, here |
|---|---|---|---|---|---|---|
| 936 | 163 of 300 | 163 of 300 (54%) | 278 of 300 | 25-29 of 30 | 212 of 300 (71%) | 297 of 300 (99%) |
| 1215 | 167 of 300 | 167 of 300 (56%) | 249 of 300 | 23-27 of 30 | 209 of 300 (70%) | 281 of 300 (94%) |

**K3**: node mode at 1215 cuts 114 of 133 (86%) of the nodes GF(2) leaves open (killed below 50%): passes K3

Root-mode cuts against node-mode cuts, among the open nodes both modes have:

| n | open nodes | root mode cuts | node mode cuts | ratio |
|---|---|---|---|---|
| 936 | 137 | 49 | 134 | 0.37 |
| 1215 | 133 | 42 | 114 | 0.37 |

## 8. Census of the cores

The cores the main arm found.  Square: as many columns as members; three primes: `primes` = 3; det0 = 0: the plain 0/1 matrix is singular over GF(3), among the square cores; F1, R3: the plain signing, the three random signings of arm f1r3 see the core; assigned: it went to a signing.

| n | cores | mean size | square | three primes | det0 = 0 among square | seen by F1 | seen by R3 | assigned |
|---|---|---|---|---|---|---|---|---|
| 900-999 | 453 | 25.5 | 0.002 | 0.000 | 0.000 | 0.000 | 0.033 | 0.629 |
| 1000-1099 | 367 | 26.4 | 0.011 | 0.000 | 0.000 | 0.011 | 0.022 | 0.540 |
| 1100-1199 | 404 | 28.6 | 0.002 | 0.000 | 0.000 | 0.000 | 0.020 | 0.540 |
| 1200-1299 | 739 | 29.7 | 0.000 | 0.000 | - | 0.001 | 0.019 | 0.548 |
| 1300-1399 | 360 | 33.9 | 0.003 | 0.000 | 0.000 | 0.003 | 0.019 | 0.425 |
| 1400-1499 | 736 | 33.2 | 0.003 | 0.000 | 1.000 | 0.010 | 0.005 | 0.567 |
| 1500-1599 | 784 | 32.0 | 0.009 | 0.000 | 0.714 | 0.032 | 0.015 | 0.483 |
| 1600-1699 | 527 | 34.0 | 0.017 | 0.000 | 0.778 | 0.032 | 0.021 | 0.488 |
| 1700-1799 | 345 | 37.2 | 0.000 | 0.000 | - | 0.009 | 0.014 | 0.461 |
| 1800-1899 | 593 | 36.3 | 0.000 | 0.000 | - | 0.000 | 0.012 | 0.479 |
| 1900-1999 | 762 | 37.2 | 0.003 | 0.000 | 0.000 | 0.000 | 0.010 | 0.480 |
| 2000-2099 | 686 | 38.1 | 0.000 | 0.000 | - | 0.000 | 0.004 | 0.536 |
| 2100-2199 | 1373 | 40.1 | 0.000 | 0.000 | - | 0.001 | 0.002 | 0.562 |
| all | 8129 | 34.2 | 0.003 | 0.000 | 0.519 | 0.007 | 0.013 | 0.524 |

