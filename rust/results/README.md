# Results of the Rust solver

One JSON line per game, as written by `taxman solve --out` (fields: n, score,
moves, promotions, method, nodes, timings, tactics and whether any unproven
tactic was on; see `solver::GameResult`).  Every file here is produced by a
single reproducible command from a committed tree and checked independently
(legal replay; score ≤ n + score(n−1); score ≤ the Franklín–Moniot bound;
score ≥ the best-known heuristic chain in `heuristics/results`).

| file | games | tactics | command |
|---|---|---|---|
| `default.jsonl` | 1001–1200 | default (includes the unproven `reuse_prev` and `revise_freedoms`, as the Java's optimal.json does) | `taxman solve 1001-1200 --threads 10 --out results/default.jsonl` |
| `strict.jsonl` | 1001–1200 | `reuse_prev=off`, `revise_freedoms=off`: no unproven tactic | `taxman solve 1001-1200 --threads 10 --tactic reuse_prev=off --tactic revise_freedoms=off --out results/strict.jsonl` |

"Optimal" here means optimal within the frame model, whose own assumptions
(greedy impossible sources in hives, mandatory moves, closed hives, one-frame
promotions) are listed in DESIGN.md; the same holds for optimal.json beyond
the brute-force-certified n ≤ 62.  Each game starts from the previous game's
result (the first from optimal.json's n=1000), which feeds `prev_cap` and the
derivation shortcuts.
