# The fields study: the data and the commands

What was measured and what it showed: [docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10), with E2
as its last section, and for the bound in a branch and bound (E3's pilot) [its entry](../../docs/EXPERIMENTS.md#the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10).
This directory holds the data that `examples/fields.rs` wrote (its module comment and `common/dw.rs`'s define the
bound and every field), `fields.py`, which makes the first entry's tables from them (`./fields.py >
fields_output.md`; `./fields.py --roots lite.jsonl --lite` for the quick look), `e2.py` (E2's) and `e3.py` (the
pilot's).  Everything ran on the M1 Max, one process at a time under `nice -n 10`, one thread, with other jobs on the
machine; times are wall seconds of one thread.

## Files and the commands that made them

`fields` is `target/release/examples/fields` (`nice -n 10 cargo build --release -j 4 --example fields`), run from
`rust/`; `K` is `--known results/strict.jsonl`.

| file | command | what it is |
|---|---|---|
| `lite.jsonl` | `fields roots K --arms f1cores --seconds 120 --out bench/fields/lite.jsonl 900-2100/100`, the code of b8d8256 | E1-lite: the main arm at 13 roots (before the master's stall restart and the exact checks' column limit) |
| `roots.jsonl`, `census.jsonl` | `fields roots K --arms gf2,f1,f1r1,f1r3,cores,f1cores --seconds 120 --fixed-seconds 30 --out bench/fields/roots.jsonl --census bench/fields/census.jsonl 900-2175/25 936 992 1088 1184 1196 1215 1240 1254 1288 1430 1472 1521 2170 2172`, the code of 1051560 | E1: six arms at 66 roots (the design's 67 count 1475 twice), and every core the main arm found |
| `nodes.jsonl` | `fields nodes K --modes root,node --seconds 10 --root-seconds 120 --exact bench/matroid/nodes_exact.jsonl --out bench/fields/nodes.jsonl testdata/perf/n0936.nodes testdata/perf/n1215.nodes`, the code of 1e12c77 | E1n: the probe's 600 nodes, in modes `root` and `node` |
| `check.jsonl` | `fields check K --samples 200 400-2175/25` | Theorems 14 and 15 and the column-sum signings at scale: per board, the promotion sets and signed matroids checked |
| `fields_output.md`, `lite_output.md` | `./fields.py > fields_output.md`, `./fields.py --roots lite.jsonl --lite > lite_output.md` | the tables |
| `roots_e2.jsonl`, `fits/` | `fields roots K S --arms gf2,f1cores --seconds 600 --save-fit bench/fields/fits --out bench/fields/roots_e2.jsonl 2200-3400/100 3333 3335 3336 3176`, then the same with `--arms f1cores16` at 2200-3200/200, 3300, 3333, 3335, 3336, 3176 and 3400, the code of 32cccb3 (run in pieces of a few roots, each resuming where the last ended) | E2: the main arm certified (600 s a root) at 17 roots past 2175, the limit of 16 core-derived signings a frame (`f1cores16`) at 12 of them, and the fit of each main-arm run (below) |
| `search_e1roots.jsonl` | `fields roots K --arms f1cores --seconds 60 --fit-mode search --out bench/fields/search_e1roots.jsonl 900 925 936 ... 2175` (the 54 games of `roots.jsonl` whose `f1cores` line has `converged` true), the code of 32cccb3 | E2: search mode (no exact checks) against E1's certified bounds |
| `budget60_e1roots.jsonl`, `budget60_e2roots.jsonl` | `fields roots K S --arms f1cores --seconds 60 --out bench/fields/budget60_e1roots.jsonl` at the same 54 roots, and `... budget60_e2roots.jsonl 2200-3400/100 3333 3335 3336 3176`, the code of 32cccb3 | E2: the main arm certified with a budget of 60 s (not run to convergence): the fit a search can afford, against E1's and the 600 s runs |
| `e2_output.md` | `./e2.py > e2_output.md` | E2's tables |
| `bnb.jsonl`, `audit.jsonl`, `fits/`, `logs/` | 1215: `fields bnb K --variants v0,v1,r1,r2,r2c --refit 20 --policy all --fit bench/fields/fits --fit-seconds 300 --fit-mode certified --audit 25 --audit-out bench/fields/audit.jsonl --log bench/fields/logs --checkpoint DIR --out bench/fields/bnb.jsonl 1215`; 1472: the same with `--variants r2 --audit 500 --seconds 36000`, then `--variants v0` with no audit and no log; the paired dives `--variants r2,r2c,v1,r1 --knuth 2000 --seed 1 1472`; the exploration `--variants r1,r2 --refit 50` (100, 200) and `--variants r2 --policy excess:50` (`tier1:0.5`) at 1215; the code of 33fb719 (the root fits `fits/n1215.json` and `fits/n1472.json` made by bc33638's, from the same code but for the saturating sums, in that branch's first format and converted at the merge with E2: below), the logs gzipped afterwards | E3's pilot: a line per run (tree or dives), a line per audited node, the root fits, and per run the per-node log (and per dive run the dives) |
| `e3_output.md` | `./e3.py > e3_output.md` | the pilot's tables |

`S` is `--known` of a snapshot of the EC2 chain's results to board 3423 (`chain_snapshot2.jsonl`, 3,423 games, sha256
`a24709934602002d79c9caf29f4b39e3c3aad17c9650115fcb8d4ccd18e008a9`; the chain's games past 2175 were not yet in
`results/` then; they are now), after `K`; the optimum of each game used is `opt` of its line.  The dear roots are the searched games
(`method` search, proof `exhausted`) past 2175 with the largest root GF(2) gaps in that snapshot: 3300 (1443), 3333,
3335 and 3336 (1427 each; neighbours of one search, with the same root structure) and 3176 (1388, the longest search
of the three games of that gap).  Games with a proof of `cap` (3303: 1443, 3339: 1427) were not searched.

## The fits (`fits/`)

`fits/n{N}.json` is the fit of the main arm's 600 s run at the root of `N`, for the 17 roots of E2 (2200 to 3400 by 100,
3176, 3333, 3335, 3336; 14 MB): the signings, the multipliers of the lowest L the run evaluated, the final master's
columns and its rank inequalities, in the format that `examples/common/fit.rs` describes (its module comment is the
specification).  Each reads back (`SavedFit::load`) and gives its `bound` again at the root (`SavedFit::bound`); the run
asserts that before it writes the file, and `cargo t --release --example fields the_saved_fits_give_their_bounds` checks
the files.  They are the starting point of E3 (a branch and bound with the bound at its nodes).  Five of the 17 did not
converge in 600 s (3100, 3176, 3300, 3336, 3400): their bounds are valid and their multipliers close to the LP's (the
tables in `e2_output.md` give the master's value beside them).  The fits of the arm with a limit of 16 (12 MB) were not
kept: E2 found the limit of 8 the better at the dear roots, and `--arms f1cores16 --save-fit` writes them
(`fits/n{N}.f1cores16.json`) in 5 to 10 minutes a root.

`fits/n1215.json` and `fits/n1472.json` are the E3 pilot's root fits (`fields bnb --fit-seconds 300 --fit-mode
certified`, the main arm): every variant of a game started from its file.  They hold the final family's signings
and the multipliers of its lowest L (which give the same `bound`), and were written in the pilot branch's first
format; at the merge with E2 they were converted to this one, every value kept as it was, with `mode` certified,
`support` null (that format did not record it) and its `local` (0) dropped.  The pilot's trees and logs at 1215
come out the same from the converted files.
