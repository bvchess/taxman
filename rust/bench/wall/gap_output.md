### The root gaps by band

| band | searched games | mean gap | median | sd | min | max | mean 1000 G/n |
|---|---|---|---|---|---|---|---|
| 1301-1500 (committed) | 88 | 370 | 388 | 126 | 125 | 590 | 261 |
| 1501-1700 (committed) | 90 | 404 | 401 | 85 | 261 | 561 | 254 |
| 1701-1900 (committed) | 87 | 365 | 336 | 114 | 192 | 543 | 202 |
| 1901-2100 (committed) | 90 | 348 | 345 | 104 | 179 | 598 | 173 |
| 2101-2200 (committed to 2175) | 47 | 560 | 517 | 124 | 398 | 887 | 260 |
| 2201-2300 | 44 | 728 | 710 | 148 | 566 | 1126 | 323 |
| 2301-2400 | 46 | 961 | 904 | 181 | 686 | 1346 | 408 |
| 2401-2500 | 45 | 747 | 748 | 135 | 547 | 1048 | 305 |
| 2501-2600 | 43 | 788 | 852 | 129 | 439 | 950 | 309 |
| 2601-2700 | 46 | 528 | 578 | 106 | 341 | 731 | 199 |
| 2701-2800 | 47 | 820 | 809 | 48 | 717 | 875 | 298 |
| 2801-2900 | 48 | 687 | 680 | 32 | 612 | 774 | 241 |
| 2901-3000 | 42 | 662 | 704 | 111 | 482 | 818 | 224 |
| 3001-3100 | 44 | 912 | 868 | 142 | 766 | 1239 | 299 |
| 3101-3200 | 45 | 1126 | 1115 | 135 | 924 | 1388 | 357 |
| 3201-3300 | 43 | 1014 | 996 | 119 | 795 | 1443 | 312 |
| 3301-3400 | 5 | 1346 | 1343 | 58 | 1300 | 1443 | 407 |
| 2176-3312, all | 509 | 818 | 788 | 211 | 341 | 1443 | 299 |

### The gap's growth with n

| fit of the gap on n | games | slope per 100 n | robust se | bootstrap 95%, blocks of 15 games | of 100 games | R² | sd | fitted at the first | at the last |
|---|---|---|---|---|---|---|---|---|---|
| 1301-2175 searched (committed chain) | 391 | 7.5 | 2.5 | [-15, 16] | [-44, 20] | 0.03 | 115 | 352 | 418 |
| 1301-2150 | 380 | 3.5 | 2.4 | [-18, 14] | [-49, 18] | 0.01 | 109 | 363 | 393 |
| 2176-3312, all searched | 509 | 25.2 | 2.7 | [4, 41] | [-43, 83] | 0.15 | 194 | 675 | 961 |
| 2176-3312, exhausted | 400 | 25.1 | 2.9 | [1, 42] | [-65, 94] | 0.15 | 199 | 683 | 969 |
| 2176-2275 | 45 | -118.3 | 32.9 | [-428, 220] | - | 0.21 | 68 | 733 | 616 |
| 2276-3312 | 464 | 24.4 | 3.5 | [-0, 47] | [-40, 97] | 0.11 | 202 | 706 | 958 |
| 2176-2800 | 282 | -15.4 | 5.2 | [-70, 9] | [-143, 43] | 0.02 | 179 | 809 | 712 |
| 2800-3312 | 227 | 118.9 | 6.2 | [89, 177] | [64, 257] | 0.62 | 137 | 593 | 1195 |
| 1301-3312, both | 900 | 37.0 | 0.9 | [30, 42] | [22, 50] | 0.59 | 177 | 258 | 1001 |

The waves of the gap are 100 games and more long, so the blocks of 15 games understate the uncertainty; 1301-3312 with blocks of 200 games (about 450 n): [6, 63] per 100 n.

Quadratic in n/1000 over 1301-3312: G = 175 +0 x +80 x², t of the square term 4.0 (robust), R² 0.60.
Power law over 1301-3312: G ∝ n^1.39 ± 0.04, R² 0.58.
1000 G/n over 1301-2175: 377 -8.8 per 100 n (robust se 1.5), mean 224.
1000 G/n over 2176-3312: 378 -2.9 per 100 n (robust se 1.0), mean 299.
1000 G/n over 2276-3312: 397 -3.5 per 100 n (robust se 1.3), mean 299.
1000 G/n over 1301-3312: 168 +4.3 per 100 n (robust se 0.4), mean 267.

| n | searched games | mean G | 80th percentile | 90th percentile | maximum |
|---|---|---|---|---|---|
| 1301-1700 | 178 | 387 | 484 | 514 | 590 |
| 1701-2100 | 177 | 356 | 474 | 519 | 598 |
| 2101-2500 | 182 | 748 | 887 | 1035 | 1346 |
| 2501-2900 | 184 | 705 | 859 | 868 | 950 |
| 2901-3300 | 174 | 932 | 1115 | 1193 | 1443 |
| 3301-3700 | 5 | 1346 | 1363 | 1403 | 1443 |

### Do the waves of the gap follow the arithmetic of n?

The gap of every searched game on counts of numbers by tier (a linear model; held out a tenth of the games at a time, consecutive):

| features of n | 1301-3312: R² in sample, held out | 2176-3312: R² in sample, held out |
|---|---|---|
| n alone | 0.59, 0.52 (sd 177) | 0.15, -0.12 (sd 194) |
| primes in tiers 1-9 | 0.74, 0.58 (sd 143) | 0.53, 0.30 (sd 146) |
| numbers of three or more distinct primes in tiers 1-7 | 0.76, 0.68 (sd 136) | 0.55, 0.31 (sd 142) |

### The speculation and the threads in a game's nodes

| run | window | games | mean log10 of its nodes over the chain's | sd | geometric mean ratio | sum of nodes over the chain's |
|---|---|---|---|---|---|---|
| i8b_T192_W1 | 1 | 18 | -0.123 | 0.169 | 0.75 | 0.65 |
| i8b_T64_W1 | 1 | 18 | -0.103 | 0.161 | 0.79 | 0.69 |
| n2277_T192 | 1 | 21 | +0.018 | 0.188 | 1.04 | 1.08 |
| x8b_T192_W1_r2 | 1 | 18 | -0.100 | 0.156 | 0.79 | 0.68 |
| x8b_T64_W1_r2 | 1 | 18 | -0.100 | 0.167 | 0.79 | 0.69 |
| e8b_T192_W4 | 4 | 18 | -0.064 | 0.137 | 0.86 | 0.77 |
| e8b_T64_W4 | 4 | 18 | -0.092 | 0.132 | 0.81 | 0.69 |
| i8b_T192_W16 | 16 | 18 | -0.002 | 0.088 | 0.99 | 1.01 |
| i8b_T192_W16_trace | 16 | 18 | +0.027 | 0.098 | 1.07 | 0.99 |
| i8b_T64_W16 | 16 | 18 | -0.030 | 0.053 | 0.93 | 0.91 |
| x8b_T192_W16_r2 | 16 | 18 | +0.007 | 0.075 | 1.02 | 1.04 |
| x8b_T64_W16_r2 | 16 | 18 | -0.004 | 0.100 | 0.99 | 1.00 |
| x8b_T192_W32 | 32 | 18 | +0.005 | 0.141 | 1.01 | 0.91 |
| x8b_T64_W32 | 32 | 18 | -0.000 | 0.074 | 1.00 | 0.97 |

By window, each game once (its mean over the runs of that window, so that the games the runs share count once):

| window | games | mean log10 | se | sd | ratio | games of 2226-2276 | of 2277-2336 |
|---|---|---|---|---|---|---|---|
| 1 | 39 | -0.039 | 0.030 | 0.185 | 0.91 | -0.107 (18) | +0.018 (21) |
| 4 | 18 | -0.078 | 0.031 | 0.131 | 0.84 | -0.078 (18) | - |
| 16 | 18 | -0.000 | 0.013 | 0.055 | 1.00 | -0.000 (18) | - |
| 32 | 18 | +0.002 | 0.020 | 0.083 | 1.01 | +0.002 (18) | - |

Threads at a window of 1, 192 against 64 (2226..2276, 18 games): -0.020 decades, sd 0.053.

### The committed chain's build against the EC2 chain's

The committed chain's 391 searched games of 1301-2175 solved again by the EC2 chain's build, in order on one thread: 0 scores differ from the committed chain's, 0 proofs.

| games | searches of 1000 nodes or more | mean log10 of the new nodes over the committed | sd | ratio of the sums | correlation with the gap | with n |
|---|---|---|---|---|---|---|
| 1301-1500 | 79 | -0.614 | 0.173 | 0.27 | +0.58 | +0.63 |
| 1501-1700 | 73 | -0.414 | 0.164 | 0.34 | -0.18 | -0.02 |
| 1701-1900 | 64 | -0.627 | 0.099 | 0.23 | +0.17 | -0.03 |
| 1901-2100 | 56 | -0.525 | 0.240 | 0.29 | +0.38 | +0.66 |
| 2101-2175 | 27 | -0.468 | 0.135 | 0.41 | +0.47 | +0.30 |
| 1301-2175 | 299 | -0.538 | 0.190 | 0.29 | +0.34 | +0.16 |

The EC2 chain (64 threads, a window of 16) against the Graviton calibration's chain (64 threads, in order, the build of 407645f) on its 39 searches of 2176-2276: -0.355 decades (se 0.024, sd 0.153); less the window's 0.04, -0.395 for the build.

### The cost model refitted

The gap alone, from the games' own nodes (the EC2 chain's as it ran: 64 threads, a window of 16):

| fit of log10 nodes = a + G/c | searches | a | c (gap per decade) | R² | residual sd |
|---|---|---|---|---|---|
| 1301-2175, committed chain (8 threads, the build of feba6d0) | 314 | 3.24 ± 0.09 | 255 ± 15 | 0.50 | 0.46 |
| 1301-2175, the same games on the EC2 build (1 thread, in order) | 314 | 2.46 ± 0.08 | 221 ± 10 | 0.64 | 0.39 |
| 2176-3312, the EC2 chain | 400 | 2.99 ± 0.10 | 266 ± 7 | 0.71 | 0.52 |
| 2176-3312 on one thread in order (a less 0.06) | 400 | 2.93 ± 0.10 | 266 ± 7 | 0.71 | 0.52 |
| 2176-2400 | 87 | 2.62 ± 0.19 | 219 ± 10 | 0.83 | 0.43 |
| 2401-2600 | 73 | 1.50 ± 0.25 | 161 ± 9 | 0.83 | 0.37 |
| 2601-3000 | 129 | 4.25 ± 0.17 | 637 ± 117 | 0.24 | 0.39 |
| 3001-3312 | 111 | 1.81 ± 0.18 | 216 ± 8 | 0.87 | 0.30 |

Which scale for the gap: R² of log10 nodes (all on the EC2 build, in order) on G / n^p, and the step between the two chains that a model leaves (the coefficient of a 0/1 variable for the EC2 chain's games added to the pooled fit; a model that holds across 2175 leaves none):

| p | R² 1301-2175 (314) | R² 2176-3312 (400) | R² 1301-3312 (714) | step at 2176 (se) |
|---|---|---|---|---|
| 0.0 | 0.643 | 0.706 | 0.852 | +0.11 (0.05) |
| 0.1 | 0.668 | 0.726 | 0.862 | +0.14 (0.05) |
| 0.2 | 0.691 | 0.742 | 0.870 | +0.19 (0.04) |
| 0.3 | 0.710 | 0.755 | 0.875 | +0.26 (0.04) |
| 0.4 | 0.725 | 0.764 | 0.875 | +0.34 (0.04) |
| 0.5 | 0.735 | 0.768 | 0.870 | +0.44 (0.04) |
| 0.6 | 0.742 | 0.768 | 0.858 | +0.56 (0.04) |
| 0.8 | 0.744 | 0.753 | 0.806 | +0.81 (0.04) |
| 1.0 | 0.733 | 0.721 | 0.709 | +1.09 (0.04) |
| 1.2 | 0.711 | 0.675 | 0.570 | +1.35 (0.04) |

| model, 1301-3312 | R² | residual sd |
|---|---|---|
| G | 0.852 | 0.47 |
| G and n | 0.869 | 0.45 |
| G and G/n | 0.874 | 0.44 |
| G / n^0.3 | 0.875 | 0.44 |
| G / n | 0.709 | 0.66 |

G on 1301-2175: a = 2.461 ± 0.077, b = 0.004527 ± 0.00021 (robust), residual sd 0.39; one decade per 220.9 units of G, which is 221 of gap at n = 3000, 221 at 4000 and 221 at 6000.
G on 2176-3312: a = 2.926 ± 0.103, b = 0.003762 ± 9.3e-05 (robust), residual sd 0.52; one decade per 265.8 units of G, which is 266 of gap at n = 3000, 266 at 4000 and 266 at 6000.
G on 1301-3312: a = 2.670 ± 0.044, b = 0.00405 ± 6e-05 (robust), residual sd 0.47; one decade per 246.9 units of G, which is 247 of gap at n = 3000, 247 at 4000 and 247 at 6000.
G / n^0.3 on 1301-2175: a = 2.356 ± 0.070, b = 0.04483 ± 0.0017 (robust), residual sd 0.35; one decade per 22.3 units of G / n^0.3, which is 246 of gap at n = 3000, 269 at 4000 and 303 at 6000.
G / n^0.3 on 2176-3312: a = 2.668 ± 0.099, b = 0.04377 ± 0.00094 (robust), residual sd 0.48; one decade per 22.8 units of G / n^0.3, which is 252 of gap at n = 3000, 275 at 4000 and 311 at 6000.
G / n^0.3 on 1301-3312: a = 2.296 ± 0.045, b = 0.04793 ± 0.00062 (robust), residual sd 0.44; one decade per 20.9 units of G / n^0.3, which is 230 of gap at n = 3000, 251 at 4000 and 284 at 6000.
Beyond G / n^0.3 on 2176-3312: its square adds 1.46e-04 ± 3.8e-05 (residual sd 0.473 against 0.478); n/1000 adds -0.349 ± 0.064 decades.  The scaled gap of the games ranges from 32.2 to 131.2.
Beyond G / n^0.3 on 1301-3312: its square adds 1.69e-06 ± 2.0e-05 (residual sd 0.437 against 0.436); n/1000 adds -0.058 ± 0.036 decades.  The scaled gap of the games ranges from 14.5 to 131.2.

Out of sample: a model fitted to the games of 1301..origin, the bias (mean error in decades, the actual less the predicted) / rmse over the later games, and the worst bias of the five origins:

| model | fitted to 1301..2175 | fitted to 1301..2400 | fitted to 1301..2600 | fitted to 1301..2800 | fitted to 1301..3000 | worst bias |
|---|---|---|---|---|---|---|
| G | -0.16 / 0.57 | -0.34 / 0.66 | -0.58 / 0.79 | -0.45 / 0.62 | -0.57 / 0.65 | 0.58 |
| G / n^0.1 | -0.04 / 0.52 | -0.23 / 0.59 | -0.47 / 0.70 | -0.35 / 0.54 | -0.48 / 0.57 | 0.48 |
| G / n^0.2 | +0.09 / 0.50 | -0.12 / 0.54 | -0.35 / 0.62 | -0.25 / 0.48 | -0.38 / 0.49 | 0.38 |
| G / n^0.3 | +0.23 / 0.53 | -0.01 / 0.51 | -0.23 / 0.56 | -0.13 / 0.42 | -0.27 / 0.41 | 0.27 |
| G / n^0.4 | +0.38 / 0.60 | +0.11 / 0.50 | -0.11 / 0.51 | -0.02 / 0.40 | -0.14 / 0.35 | 0.38 |
| G / n^0.5 | +0.52 / 0.70 | +0.24 / 0.53 | +0.03 / 0.49 | +0.11 / 0.41 | -0.01 / 0.33 | 0.52 |
| G / n^1.0 | +1.22 / 1.35 | +0.91 / 1.03 | +0.75 / 0.89 | +0.83 / 0.92 | +0.80 / 0.89 | 1.22 |
| G and n | +0.58 / 0.76 | +0.11 / 0.56 | -0.31 / 0.65 | -0.20 / 0.50 | -0.44 / 0.54 | 0.58 |

The model of 1301-2175 on the EC2 build (a = 2.46, c = 221) against the EC2 chain's searches (less 0.06 for the window and the threads), the mean residual in decades:

| searches | n | mean residual | se | sd | median |
|---|---|---|---|---|---|
| 2176-2276 | 39 | +0.03 | 0.09 | 0.53 | -0.12 |
| 2277-2600 | 121 | +0.24 | 0.03 | 0.38 | +0.25 |
| 2601-3312 | 240 | -0.40 | 0.03 | 0.49 | -0.45 |
| 2176-3312 | 400 | -0.16 | 0.03 | 0.55 | -0.19 |

### The structure of the residuals, and the dear games

| model | searches | residual sd | autocorrelation at lag 1 | 2 | 5 | 10 | 20 | runs of signs, expected | z | variance of 5-block means over independence's | 20-block means |
|---|---|---|---|---|---|---|---|---|---|---|---|
| G alone | 400 | 0.52 | +0.86 | +0.81 | +0.63 | +0.46 | +0.30 | 53 of 201 | -14.8 | 4.2 | 13.1 |
| G / n^0.3 | 400 | 0.48 | +0.84 | +0.78 | +0.57 | +0.36 | +0.18 | 60 of 201 | -14.1 | 4.0 | 11.8 |

The residuals of the model G / n^0.3 (a = 2.668, b = 0.0438) by block of n:

| games | searches | mean G | mean residual | sd of the residual | largest residual (n) |
|---|---|---|---|---|---|
| 2176-2325 | 57 | 735 | +0.06 | 0.51 | +0.98 (2268) |
| 2326-2475 | 59 | 872 | +0.10 | 0.31 | +0.59 (2366) |
| 2476-2625 | 56 | 724 | +0.47 | 0.39 | +1.23 (2538) |
| 2626-2775 | 51 | 678 | -0.24 | 0.54 | +0.71 (2639) |
| 2776-2925 | 48 | 700 | -0.04 | 0.52 | +0.86 (2925) |
| 2926-3075 | 44 | 780 | -0.33 | 0.42 | +0.80 (2928) |
| 3076-3225 | 54 | 1106 | -0.15 | 0.28 | +0.59 (3198) |
| 3226-3375 | 31 | 1056 | +0.05 | 0.24 | +0.52 (3300) |

The 18 dearest searches, and what the models predict for them (the models' nodes are for one thread in order, the game's own are the chain's, and the residuals are against the models):

| n | nodes (millions) | gap | start below opt | model G / n^p: nodes (millions) | residual | model G: nodes (millions) | residual |
|---|---|---|---|---|---|---|---|
| 2340 | 317.9 | 1333 | 285 | 228.80 | +0.08 | 87.09 | +0.50 |
| 2343 | 174.6 | 1248 | 63 | 98.74 | +0.19 | 41.71 | +0.56 |
| 2344 | 117.7 | 1248 | 0 | 98.58 | +0.02 | 41.71 | +0.39 |
| 2346 | 1142.8 | 1346 | 290 | 257.36 | +0.59 | 97.47 | +1.01 |
| 2376 | 114.2 | 1213 | 106 | 66.59 | +0.17 | 30.80 | +0.51 |
| 2379 | 95.9 | 1213 | 0 | 66.29 | +0.10 | 30.80 | +0.43 |
| 2384 | 99.9 | 1165 | 0 | 41.16 | +0.33 | 20.32 | +0.63 |
| 2385 | 139.9 | 1173 | 60 | 44.44 | +0.44 | 21.78 | +0.75 |
| 2387 | 86.6 | 1094 | 40 | 20.48 | +0.57 | 10.99 | +0.84 |
| 2388 | 80.2 | 1094 | 0 | 20.45 | +0.53 | 10.99 | +0.80 |
| 3102 | 172.3 | 1301 | 0 | 59.21 | +0.40 | 66.01 | +0.36 |
| 3104 | 92.0 | 1373 | 162 | 113.19 | -0.15 | 123.15 | -0.19 |
| 3174 | 217.7 | 1388 | 81 | 119.22 | +0.20 | 140.23 | +0.13 |
| 3175 | 216.2 | 1388 | 0 | 119.08 | +0.20 | 140.23 | +0.13 |
| 3176 | 208.9 | 1388 | 0 | 118.94 | +0.18 | 140.23 | +0.11 |
| 3300 | 635.1 | 1443 | 15 | 168.02 | +0.52 | 225.80 | +0.39 |
| 3304 | 122.9 | 1343 | 0 | 68.92 | +0.19 | 94.97 | +0.05 |
| 3311 | 93.8 | 1303 | 40 | 47.99 | +0.23 | 67.16 | +0.09 |

### What at the root explains the residuals

400 exhausted searches with the anatomy.  'Share' is the R² of a residual on the feature: the share of the residual's variance that it explains, for the residual of the model on the gap alone (R² 0.71, sd 0.52) and of the model G / n^0.3 (R² 0.75, sd 0.48); 'with G' is the R² of log10 nodes on G and the feature, 'x G' the same with the feature times G (the cost per unit of gap) as well.

| feature | mean | sd | share of the residual of G | share of the residual of G / n^p | R² with G | R² with G and x G | t of the feature with G |
|---|---|---|---|---|---|---|---|
| largest conflict | 28.7 | 3.3 | 0.19 | 0.13 | 0.76 | 0.76 | -9.2 |
| n | 2728.6 | 335.2 | 0.17 | 0.05 | 0.77 | 0.77 | -11.2 |
| freedoms, all frames | 330.1 | 40.6 | 0.16 | 0.04 | 0.76 | 0.76 | -11.1 |
| candidates | 724.4 | 94.4 | 0.15 | 0.04 | 0.76 | 0.76 | -10.5 |
| freedoms of frames 0-2 | 248.8 | 28.0 | 0.15 | 0.04 | 0.76 | 0.76 | -10.8 |
| mean conflict size | 20.4 | 2.3 | 0.14 | 0.09 | 0.75 | 0.75 | -7.5 |
| max_total | 160.2 | 20.5 | 0.12 | 0.03 | 0.75 | 0.75 | -10.7 |
| root_chosen | 149.1 | 17.4 | 0.11 | 0.03 | 0.75 | 0.75 | -10.1 |
| frames holding a core | 3.6 | 0.7 | 0.07 | 0.05 | 0.73 | 0.73 | -7.1 |
| members in the first conflict | 13.3 | 4.7 | 0.05 | 0.02 | 0.72 | 0.72 | +6.1 |
| start below the optimum | 33.8 | 82.9 | 0.04 | 0.02 | 0.72 | 0.72 | +3.6 |
| factors in cores | 254.0 | 44.5 | 0.03 | 0.01 | 0.72 | 0.72 | -5.8 |
| movers in cores | 246.3 | 42.6 | 0.02 | 0.01 | 0.72 | 0.72 | -5.0 |
| wide movers in cores | 154.0 | 30.7 | 0.02 | 0.00 | 0.71 | 0.72 | -3.8 |
| conflicts in the heaviest set | 13.5 | 3.0 | 0.01 | 0.00 | 0.71 | 0.71 | +2.2 |
| members in all conflicts | 275.5 | 68.7 | 0.01 | 0.01 | 0.71 | 0.71 | -2.3 |
| forced at the root | 1.0 | 2.9 | 0.00 | 0.00 | 0.71 | 0.71 | -1.0 |
| promoted in cores | 60.5 | 11.8 | 0.00 | 0.00 | 0.71 | 0.71 | -1.4 |
| movers in the cores of frames 1-3 | 230.5 | 37.6 | 0.00 | 0.00 | 0.71 | 0.71 | +0.0 |

All 18 features at once (without n) explain, in sample, 0.38 of the residual of G / n^0.3 and 0.47 of the residual of G alone; held out, a stretch of 40 consecutive searches at a time (the features fitted to the others), -0.71 and -0.45.

### A game at 4000, 5000 and 6000, and what 24 hours reach

The chain's searches: 400 exhausted of 1137 games (0.35), 6.19 billion nodes in all, in 6.4 hours on 64 threads.  With a node costing u(n) = u0 x 10^(0.02 (n - 3000) / 100), those hours fix u0 = 265 thread-microseconds, so u(2184) = 182 (the ladder's proof of 2184 on 64 threads: 166 µs a step, plus the window's idle time) and u(4000) = 419 (the standalone search of 4000 on 192 threads: 417).  The serial work besides the search (the board, moves and replays of every game, and half an unseeded local search for a searched one) is 0.11 + 0.35 x 4.66 s a game at 4000, whatever the threads.

The chain's own hundreds of games (the nodes it made, at that cost of a node):

| games | exhausted searches | nodes (millions) | hours of the 64 threads in the search |
|---|---|---|---|
| 2176-2275 | 39 | 42 | 0.03 |
| 2276-2375 | 37 | 2207 | 1.87 |
| 2376-2475 | 40 | 703 | 0.61 |
| 2476-2575 | 36 | 618 | 0.57 |
| 2576-2675 | 39 | 12 | 0.01 |
| 2676-2775 | 32 | 7 | 0.01 |
| 2776-2875 | 31 | 11 | 0.01 |
| 2876-2975 | 27 | 12 | 0.01 |
| 2976-3075 | 34 | 23 | 0.03 |
| 3076-3175 | 36 | 1037 | 1.27 |
| 3176-3275 | 35 | 503 | 0.63 |
| 3276-3375 | 14 | 1018 | 1.34 |

A searched game's nodes at n, on 64 threads with a window of 4 (millions; the median over the draws, then the median and the 10th and 90th percentile over all the draws' waves and residuals):

| model, gap scenario | gap at 4000 | 5000 | 6000 | nodes at 4000 | 5000 | 6000 |
|---|---|---|---|---|---|---|
| G / n^0.3, trend | 1142 | 1393 | 1645 | 3.1 [0.5, 136.1] | 12.7 [1.9, 503.7] | 47.7 [6.1, 1733.4] |
| G / n^0.3, ratio | 1210 | 1513 | 1815 | 5.5 [0.9, 245.9] | 32.1 [4.2, 1298.7] | 166.4 [18.8, 6268.3] |
| G / n^0.3, steep | 1361 | 1931 | 2501 | 19.2 [2.6, 895.2] | 834.2 [76.1, 37058.1] | 27533.0 [1673.6, 1165341.4] |
| the gap alone, trend | 1142 | 1393 | 1645 | 7.7 [1.1, 648.2] | 77.6 [7.4, 7198.8] | 785.7 [52.2, 77654.5] |
| the gap alone, ratio | 1210 | 1513 | 1815 | 14.4 [1.8, 1252.9] | 233.1 [18.7, 22259.5] | 3777.4 [193.8, 403737.8] |
| the gap alone, steep | 1361 | 1931 | 2501 | 57.8 [5.8, 5303.1] | 10941.2 [466.0, 1268893.3] | 1948743.0 [37304.9, 361995660.3] |
| G / n^0.5, trend | 1142 | 1393 | 1645 | 1.7 [0.3, 52.1] | 4.4 [0.7, 103.1] | 10.2 [1.6, 194.9] |
| G / n^0.5, ratio | 1210 | 1513 | 1815 | 2.9 [0.5, 89.4] | 10.0 [1.6, 239.2] | 29.8 [4.2, 582.2] |

Hours of the 64 threads for 100 consecutive games (the mean over the draws and starting points; the median [10th, 90th percentile]):

| model, gap scenario | at 4000 | 5000 | 6000 |
|---|---|---|---|
| G / n^0.3, trend | 4.57; 2.34 [0.13, 10.40] | 22.22; 8.30 [0.68, 55.47] | past the horizon |
| G / n^0.3, ratio | 8.33; 4.11 [0.19, 19.47] | 44.29; 9.69 [1.38, 119.61] | past the horizon |
| G / n^0.3, steep | 34.21; 15.01 [0.55, 82.25] | past the horizon | past the horizon |
| the gap alone, trend | 34.50; 6.48 [0.23, 75.67] | past the horizon | past the horizon |
| the gap alone, ratio | 42.90; 8.94 [0.36, 105.53] | past the horizon | past the horizon |
| the gap alone, steep | past the horizon | past the horizon | past the horizon |
| G / n^0.5, trend | 1.51; 1.04 [0.10, 3.38] | 3.93; 2.02 [0.35, 10.42] | 11.67; 7.48 [1.14, 31.38] |
| G / n^0.5, ratio | 2.58; 1.69 [0.13, 5.83] | 9.03; 4.27 [0.57, 23.98] | 34.28; 20.44 [2.33, 94.26] |

Hours from 3313 to the board (median [10th, 90th percentile]; more than 720 hours is shown as >720), and the board reached in 24 hours of the whole chain from 2176 (the 17.6 hours past 3313 that remain) and in 24 hours past 3313:

| model, gap scenario | hours to 4000 | 5000 | 6000 | reached, 24 hours from 2176 | reached, 24 hours past 3313 |
|---|---|---|---|---|---|
| G / n^0.3, trend | 13 [4, 33] | 107 [28, 229] | 607 [134, >720] | 4088 [3726, 4740] | 4223 [3863, 4963] |
| G / n^0.3, ratio | 20 [5, 56] | 228 [50, 521] | >720 [325, >720] | 3956 [3553, 4555] | 4052 [3674, 4679] |
| G / n^0.3, steep | 38 [8, 146] | >720 [299, >720] | >720 [>720, >720] | 3827 [3543, 4250] | 3886 [3607, 4349] |
| the gap alone, trend | 42 [7, 243] | >720 [92, >720] | >720 [>720, >720] | 3800 [3427, 4343] | 3858 [3436, 4492] |
| the gap alone, ratio | 70 [11, 442] | >720 [177, >720] | >720 [>720, >720] | 3720 [3420, 4166] | 3782 [3427, 4264] |
| the gap alone, steep | 140 [18, >720] | >720 [>720, >720] | >720 [>720, >720] | 3674 [3424, 3996] | 3723 [3430, 4087] |
| G / n^0.5, trend | 6 [2, 12] | 29 [12, 50] | 98 [39, 161] | 4613 [4243, 5229] | 4849 [4467, 5545] |
| G / n^0.5, ratio | 9 [3, 18] | 56 [19, 101] | 234 [78, 406] | 4274 [3982, 4947] | 4454 [4145, 5099] |

The same models over the chain's own games, 2176..3312 (hours; the chain took 6.4, which fixes the cost of a node):

| model, gap scenario | hours, median [10th, 90th percentile] |
|---|---|
| G / n^0.3, trend | 6.1 [1.8, 13.0] |
| G / n^0.3, ratio | 6.5 [1.9, 15.2] |
| the gap alone, trend | 8.9 [1.8, 29.3] |
| the gap alone, ratio | 9.5 [1.8, 36.6] |
| G / n^0.5, trend | 5.1 [1.8, 8.6] |
| G / n^0.5, ratio | 5.5 [1.9, 9.6] |

