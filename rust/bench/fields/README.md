# The fields study: the data and the commands

What was measured and what it showed: [docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10)
and, for the bound in a branch and bound (E3's pilot), [its entry](../../docs/EXPERIMENTS.md#the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10).
This directory holds the data that `examples/fields.rs` wrote (its module comment and `common/dw.rs`'s define the
bound and every field), `fields.py`, which makes the first entry's tables from them (`./fields.py > fields_output.md`;
`./fields.py --roots lite.jsonl --lite` for the quick look), and `e3.py`, the pilot's (`./e3.py > e3_output.md`).  Everything ran on the M1 Max, one process at a time under
`nice -n 10`, one thread, with other jobs on the machine; times are wall seconds of one thread.

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
| `bnb.jsonl`, `audit.jsonl`, `fits/`, `logs/` | 1215: `fields bnb K --variants v0,v1,r1,r2,r2c --refit 20 --policy all --fit bench/fields/fits --fit-seconds 300 --fit-mode certified --audit 25 --audit-out bench/fields/audit.jsonl --log bench/fields/logs --checkpoint DIR --out bench/fields/bnb.jsonl 1215`; 1472: the same with `--variants r2 --audit 500 --seconds 36000`, then `--variants v0` with no audit and no log; the paired dives `--variants r2,r2c,v1,r1 --knuth 2000 --seed 1 1472`; the exploration `--variants r1,r2 --refit 50` (100, 200) and `--variants r2 --policy excess:50` (`tier1:0.5`) at 1215; the code of 33fb719 (the root fits made by bc33638's, from the same code but for the saturating sums), the logs gzipped afterwards | E3's pilot: a line per run (tree or dives), a line per audited node, the root fits, and per run the per-node log (and per dive run the dives) |
| `e3_output.md` | `./e3.py > e3_output.md` | the pilot's tables |
