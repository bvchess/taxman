### Calibration: standalone (no seed, no cap) against chained, per class of the chained game

| class | games | chain s (sum) | standalone s (sum) | standalone/chain (sums) | median per-game ratio | min-max per-game | nodes standalone/chain (sums) |
|---|---|---|---|---|---|---|---|
| cap reached (0 nodes) | 70 | 6.9 | 146.8 | 21.41 | 3.43 | 0.78-191.6 | n/a (chain 0) |
| small search (chain < 5 s) | 117 | 110.1 | 156.3 | 1.42 | 1.05 | 0.68-4.1 | 1.52 |
| big search (chain >= 5 s) | 10 | 200.5 | 370.1 | 1.85 | 1.28 | 0.96-2.3 | 1.84 |
| big searches, n >= 1150 | 8 | 177.4 | 342.6 | 1.93 | 1.42 | 0.98-2.3 | 1.92 |
| all non-prime games | 197 | 317.4 | 673.2 | 2.12 | 1.23 | 0.68-191.6 | 2.23 |

Chain total 318 s over 231 games; the 34 primes cost 0.43 s together.

Largest games, chained against standalone (s): 1230: 91.5 vs 206.9, 1220: 19.6 vs 43.3, 1225: 17.0 vs 33.1, 1228: 0.1 vs 24.3, 1025: 18.0 vs 22.7, 1227: 19.7 vs 21.6, 1226: 0.1 vs 21.2, 1224: 0.1 vs 16.6

Ratios chain/standalone used below: ratio_big 0.52, ratio_window 0.45, ratio_all 0.47

### Standalone cost per band (8 threads, no seed, no cap; the first non-primes from the band start)

| band start | games (n) | median s | p75 s | max s | mean s | timeouts | median nodes (M) | max nodes (M) | median share of time before the final incumbent |
|---|---|---|---|---|---|---|---|---|---|
| 1001 | 1001-1008 (8) | 1.4 | 1.5 | 1.9 | 1.5 | 0 | 1.0 | 1.3 | 81% |
| 1050 | 1050-1058 (8) | 0.4 | 0.5 | 0.7 | 0.4 | 0 | 0.2 | 0.3 | 57% |
| 1100 | 1100-1108 (8) | 0.1 | 0.2 | 0.2 | 0.1 | 0 | 0.0 | 0.1 | 55% |
| 1150 | 1150-1159 (8) | 0.1 | 0.1 | 0.1 | 0.1 | 0 | 0.0 | 0.0 | 0% |
| 1200 | 1200-1208 (8) | 5.0 | 5.9 | 6.2 | 4.6 | 0 | 4.2 | 5.3 | 43% |
| 1250 | 1250-1257 (8) | 10.0 | 16.8 | 21.8 | 11.4 | 0 | 8.5 | 17.7 | 11% |
| 1300 | 1300-1310 (8) | 184 | 287 | 291 | 196 | 0 | 103.8 | 134.9 | 9% |
| 1350 | 1350-1352 (3) | 530 | 534 | 538 | 526 | 0 | 213.9 | 323.2 | 94% |

### Fits: log10(seconds) = a + b n over band statistics (x = mean n of the band's games)

| statistic | bands used | b (per n) | s.e. of b | x10 per dn | a | residual sd (log10) | seconds at 1231 | at 1300 | at 1400 |
|---|---|---|---|---|---|---|---|---|---|
| median, bands from 1150 | 5 | 0.01832 | 0.00238 | 55 | -21.84 | 0.37 | 5 | 96 | 6521 |
| mean, bands from 1150 | 5 | 0.01840 | 0.00227 | 54 | -21.92 | 0.35 | 5 | 99 | 6865 |
| max, bands from 1150 | 5 | 0.01786 | 0.00240 | 56 | -21.10 | 0.37 | 8 | 133 | 8117 |
| median, bands from 1200 | 4 | 0.01498 | 0.00244 | 67 | -17.48 | 0.27 | 9 | 98 | 3095 |
| mean, bands from 1200 | 4 | 0.01512 | 0.00218 | 66 | -17.65 | 0.24 | 9 | 102 | 3304 |
| max, bands from 1200 | 4 | 0.01420 | 0.00201 | 70 | -16.33 | 0.22 | 14 | 136 | 3585 |
| median, bands from 1250 | 3 | 0.01785 | 0.00410 | 56 | -21.27 | 0.28 | 5 | 86 | 5270 |
| mean, bands from 1250 | 3 | 0.01722 | 0.00411 | 58 | -20.42 | 0.28 | 6 | 93 | 4885 |
| max, bands from 1250 | 3 | 0.01446 | 0.00453 | 69 | -16.67 | 0.31 | 14 | 135 | 3760 |

### Projection of the cumulative chain cost from 1231 to N (chain cost per non-prime game = 0.45 x the standalone fit)

#### median, bands from 1200

| N | game N standalone (s) | cumulative, this machine (8 thr.) | 192 cores, 19x | spot $ | on-demand $ | 192 cores, 10x | spot $ | on-demand $ |
|---|---|---|---|---|---|---|---|---|
| 1300 | 98.3 | 16.4 min | 51.7 s | 0.0478 | 0.12 | 1.64 min | 0.0909 | 0.228 |
| 1400 | 3.09e+03 | 10.2 h | 32.1 min | 1.78 | 4.46 | 1.02 h | 3.38 | 8.48 |
| 1450 | 1.74e+04 | 2.26 days | 2.86 h | 9.51 | 23.9 | 5.43 h | 18.1 | 45.3 |
| 1500 | 9.75e+04 | 11.9 days | 15 h | 49.9 | 125 | 1.19 days | 94.8 | 238 |
| 1550 | 5.47e+05 | 72 days | 3.79 days | 303 | 759 | 7.2 days | 575 | 1.44e+03 |
| 1600 | 3.07e+06 | 1.12 years | 21.4 days | 1.71e+03 | 4.29e+03 | 40.7 days | 3.25e+03 | 8.16e+03 |
| 1700 | 9.67e+07 | 34.2 years | 1.8 years | 5.26e+04 | 1.32e+05 | 3.42 years | 9.99e+04 | 2.5e+05 |
| 1800 | 3.04e+09 | 1.12e+03 years | 58.9 years | 1.72e+06 | 4.31e+06 | 112 years | 3.27e+06 | 8.19e+06 |
| 2000 | 3.02e+12 | 1.05e+06 years | 5.51e+04 years | 1.61e+09 | 4.03e+09 | 1.05e+05 years | 3.06e+09 | 7.66e+09 |

#### mean, bands from 1200

| N | game N standalone (s) | cumulative, this machine (8 thr.) | 192 cores, 19x | spot $ | on-demand $ | 192 cores, 10x | spot $ | on-demand $ |
|---|---|---|---|---|---|---|---|---|
| 1300 | 102 | 16.8 min | 53.1 s | 0.0491 | 0.123 | 1.68 min | 0.0934 | 0.234 |
| 1400 | 3.3e+03 | 10.7 h | 33.9 min | 1.88 | 4.72 | 1.07 h | 3.58 | 8.97 |
| 1450 | 1.88e+04 | 2.43 days | 3.07 h | 10.2 | 25.6 | 5.83 h | 19.4 | 48.7 |
| 1500 | 1.07e+05 | 12.9 days | 16.4 h | 54.5 | 137 | 1.29 days | 103 | 259 |
| 1550 | 6.12e+05 | 79.9 days | 4.2 days | 336 | 842 | 7.99 days | 638 | 1.6e+03 |
| 1600 | 3.49e+06 | 1.26 years | 24.1 days | 1.93e+03 | 4.84e+03 | 45.9 days | 3.67e+03 | 9.2e+03 |
| 1700 | 1.13e+08 | 39.8 years | 2.09 years | 6.11e+04 | 1.53e+05 | 3.98 years | 1.16e+05 | 2.91e+05 |
| 1800 | 3.68e+09 | 1.34e+03 years | 70.7 years | 2.06e+06 | 5.17e+06 | 134 years | 3.92e+06 | 9.82e+06 |
| 2000 | 3.89e+12 | 1.34e+06 years | 7.04e+04 years | 2.05e+09 | 5.15e+09 | 1.34e+05 years | 3.9e+09 | 9.78e+09 |

#### max, bands from 1200

| N | game N standalone (s) | cumulative, this machine (8 thr.) | 192 cores, 19x | spot $ | on-demand $ | 192 cores, 10x | spot $ | on-demand $ |
|---|---|---|---|---|---|---|---|---|
| 1300 | 136 | 23.7 min | 1.24 min | 0.0691 | 0.173 | 2.37 min | 0.131 | 0.329 |
| 1400 | 3.58e+03 | 12.4 h | 39.1 min | 2.17 | 5.44 | 1.24 h | 4.12 | 10.3 |
| 1450 | 1.84e+04 | 2.53 days | 3.19 h | 10.6 | 26.7 | 6.07 h | 20.2 | 50.6 |
| 1500 | 9.43e+04 | 12.1 days | 15.3 h | 51 | 128 | 1.21 days | 96.9 | 243 |
| 1550 | 4.84e+05 | 67.1 days | 3.53 days | 282 | 708 | 6.71 days | 536 | 1.35e+03 |
| 1600 | 2.48e+06 | 347 days | 18.2 days | 1.46e+03 | 3.66e+03 | 34.7 days | 2.77e+03 | 6.95e+03 |
| 1700 | 6.53e+07 | 24.4 years | 1.28 years | 3.75e+04 | 9.4e+04 | 2.44 years | 7.12e+04 | 1.79e+05 |
| 1800 | 1.72e+09 | 665 years | 35 years | 1.02e+06 | 2.56e+06 | 66.5 years | 1.94e+06 | 4.86e+06 |
| 2000 | 1.19e+12 | 4.36e+05 years | 2.29e+04 years | 6.69e+08 | 1.68e+09 | 4.36e+04 years | 1.27e+09 | 3.19e+09 |

#### Sensitivity: cumulative chain cost on this machine (8 threads) for each fit

| fit | slope b | s.e. of b | to 1400 | to 1500 | to 1600 |
|---|---|---|---|---|---|
| median, bands from 1150 | 0.01832 | 0.00238 | 17.6 h | 43.9 days | 9.01 years |
| mean, bands from 1150 | 0.01840 | 0.00227 | 18.5 h | 46.8 days | 9.78 years |
| max, bands from 1150 | 0.01786 | 0.00240 | 22.5 h | 50.4 days | 9.3 years |
| median, bands from 1200 | 0.01498 | 0.00244 | 10.2 h | 11.9 days | 1.12 years |
| mean, bands from 1200 | 0.01512 | 0.00218 | 10.7 h | 12.9 days | 1.26 years |
| mean, bands from 1200, slope low (+-1 s.e.) | 0.01294 |  | 6.77 h | 5 days | 106 days |
| mean, bands from 1200, slope high (+-1 s.e.) | 0.01730 |  | 17.4 h | 34.3 days | 5.55 years |
| max, bands from 1200 | 0.01420 | 0.00201 | 12.4 h | 12.1 days | 347 days |
| median, bands from 1250 | 0.01785 | 0.00410 | 14.6 h | 32.7 days | 6.01 years |
| mean, bands from 1250 | 0.01722 | 0.00411 | 14 h | 27.2 days | 4.32 years |
| max, bands from 1250 | 0.01446 | 0.00453 | 12.8 h | 13.3 days | 1.1 years |

#### How far the chain gets: the largest N reached within the budget (mean fit from 1200; central slope, and in brackets the range for the slope +1 s.e. to -1 s.e.)

| budget | 192 cores, 19x | 192 cores, 10x | this machine (8 thr.) |
|---|---|---|---|
| 1 day | 1509 (1484-1541) | 1493 (1467-1520) | 1423 (1407-1443) |
| 1 week | 1564 (1532-1609) | 1545 (1515-1586) | 1479 (1459-1509) |
| 30 days | 1607 (1568-1655) | 1588 (1553-1635) | 1521 (1496-1557) |
| 1 year | 1678 (1632-1739) | 1659 (1616-1717) | 1593 (1557-1642) |
| $100 spot | 1515 (1490-1549) | 1499 (1473-1528) | 1430 (1413-1453) |
| $1,000 spot | 1581 (1545-1629) | 1563 (1529-1605) | 1499 (1473-1528) |
| $10,000 spot | 1647 (1604-1704) | 1630 (1588-1682) | 1563 (1529-1605) |
| $100,000 spot | 1713 (1661-1780) | 1694 (1645-1760) | 1630 (1588-1682) |
| $1,000 on-demand | 1555 (1523-1595) | 1536 (1507-1575) | 1471 (1449-1499) |
| $10,000 on-demand | 1623 (1581-1674) | 1603 (1564-1651) | 1536 (1507-1575) |

### Thread scaling, proof only (seeded with the game's own optimum)

| n | threads | search s | speed-up over 1 thread | nodes (M) | k nodes/s per thread | speed-up of nodes/s | effective parallelism % |
|---|---|---|---|---|---|---|---|
| 1225 | 1 | 150.9 | 1.00 | 12.07 | 80.0 | 1.00 | 100.0 |
| 1225 | 2 | 62.7 | 2.41 | 9.79 | 78.0 | 1.95 | 100.0 |
| 1225 | 4 | 31.5 | 4.79 | 9.45 | 75.1 | 3.75 | 100.0 |
| 1225 | 8 | 18.0 | 8.39 | 9.30 | 64.6 | 6.46 | 100.0 |

### Repeat runs (same game, later in the day with Zoom running)

| n | first run s | nodes (M) | k nodes/s | repeat s | nodes (M) | k nodes/s |
|---|---|---|---|---|---|---|
| 1215 | 9.8 | 7.8 | 796 | 9.0 | 6.3 | 698 |
| 1225 | 33.1 | 19.3 | 583 | 25.5 | 14.0 | 548 |
| 1300 | 127.0 | 82.9 | 653 | 134.6 | 84.7 | 629 |
| 1230 | 206.9 | 111.7 | 540 | 219.9 | 116.3 | 529 |
