# The wall regime: the data and the commands

What was measured and what it showed: [docs/EXPERIMENTS.md](../../docs/experiments/the-wall-regime-2026-10.md).
This directory holds the data that `examples/wall.rs` wrote (its module comment defines every field) and `wall.py`,
which makes the entry's tables from them (`./wall.py > wall_output.md`).  Everything ran on the M1 Max, one process
at a time under `nice -n 10`, one thread unless a line says otherwise, with other jobs on the machine (load 4 to 5), on
the code of this branch at the merge of master 5252d5d (9c99ca1).  Times are wall seconds of one thread; the cycles
and instructions of the `nodes` lines are the process's, and those of the start mode include the search's own setup.
The files of [the gap past 2175](../../docs/experiments/the-gap-past-2175-2026-10.md) (`gaps_chain.jsonl` and the
three after it, with `gap.py`) ran on master ee03872 with no change to the solver, in the same way.

## Files and the commands that made them

`wall` is `target/release/examples/wall` (`nice -n 10 cargo build --release -j 4 --example wall`), run from `rust/`;
`K` is `--known results/strict.jsonl`.

| file | command | what it is |
|---|---|---|
| `setup.jsonl` | `wall setup K 1000 1300 1600 1900 2100 2175 2200 2500 2800 3100 3400 3700 4000 4095` | the fixed costs of a game piece by piece, the sizes, the root's GF(2) bound and the gap |
| `setup_widths.jsonl` | `wall setup K --repeat 2 --width W N` for N = 2170 (W = 36, 48, 64) and 3000 (48, 64) | the same at other set widths |
| `setup_c8g.jsonl` | `wall setup K $(seq 2200 25 4095)` on one thread of a `c8g.48xlarge` (Graviton4), the code of 55287ff: item 1 of the next section | the fixed costs of a game on Graviton, after round A ([docs/EXPERIMENTS.md](../../docs/experiments/scaling-to-192-threads-2026-10.md)) |
| `start.jsonl` | `wall start K 1301 1306 ... 2171` (every fifth board) | the starting incumbent's local search: singles against pairs |
| `nodes_start_low.jsonl`, `nodes_opt_low.jsonl` | `wall nodes K --no-seed --nodes 5000 1300 1600 1900 2170`, and with `--mode opt` | the cost of a node from the start incumbent and in the proof of the optimum |
| `nodes_start_high.jsonl` | `wall nodes --no-seed --nodes 5000 2200 2500 2800 3100 3400 3700 4000 4095` | the cost of a node past 2175 |
| `gaps.jsonl` | `wall nodes K --no-seed --nodes 1000 1402 1502 ... 2102` and `2252 2352 ... 4052` | root gaps over the unseeded start (and the optimum where known), and node costs, every 100 |
| `gaps_c8g.jsonl` | `wall nodes --known KNOWN --nodes 1000 2184 2185 2190 2200 2220 2240 2250 2254 2256 2260 2268 2275`, `KNOWN` = `results/strict.jsonl` followed by `../cloud/c8g.chain-2176.jsonl` | the root gaps of the dear games past 2175, against their optima (seeded starts) |
| `gaps_c8g_rest.jsonl` | the same with `--nodes 1` for the other 33 searched games of 2176..2276 (2176 2178 2180 2187 2192 2193 2196 2205 2208 2210 2211 2212 2214 2216 2223 2224 2225 2226 2232 2233 2235 2236 2241 2244 2247 2248 2255 2259 2261 2262 2264 2265 2272), a later build of `wall` (2026-10-14) | with `gaps_c8g.jsonl`, the root gap of every searched game of the c8g chain, for the fit of [docs/AGENDA.md](../../docs/AGENDA.md#71-the-root-gaps-of-the-c8g-chain) |
| `gaps_chain.jsonl` | `wall nodes --known SNAPSHOT --nodes 1 $(./gap.py games SNAPSHOT) > RAW`, then `./gap.py join SNAPSHOT RAW > gaps_chain.jsonl`; `SNAPSHOT` is a copy of the EC2 chain's results file taken at game 3314 on 2026-10-15 (64 threads on a `c8g.16xlarge`, the code of 55287ff, a window of 16 to about game 3310 and 4 after it; 16.5 MB, not committed), `wall` built from master ee03872, one thread, 9 minutes in all | one line per searched game past 2175 of the snapshot (509 games, 2176..3312: 400 `exhausted`, 109 `cap`; a repeat has its source's gap, and every source is among them): the root's GF(2) bound, the optimum, the true root gap (the bound less the optimum) and the start incumbent, the chain's own nodes, seconds, proof and threads, and the root's sizes (candidates, `max_total`, the heaviest set's size, forced members, the frames' ranks, moves, factors and freedoms) |
| `gaps_chain_3313_3440.jsonl` | the same with `SNAPSHOT` the committed `results/strict.jsonl` (games 1..3440) and the games past 3312 only, `wall` built from 35c614b, one thread, 111 s | the rest of the committed chain in the same form, for the work list of the certificates (`../certify/`): the 58 searched games of 3315..3440 (48 `exhausted`, 10 `cap`), root gaps 1,103 to 1,555 (the exhausted ones' mean 1,265), equal to the barred bounds' gaps at the roles of Theorem B (`prover/data/upper_3315_3440.jsonl`) where both have the game |
| `anatomy_chain.jsonl` | `matroid lagrange --known SNAPSHOT --limit 0 $(./gap.py games SNAPSHOT) > L`, `matroid cores $(./gap.py games SNAPSHOT) > C` (`cargo build --release -j 4 --example matroid`), then `./gap.py join-anatomy SNAPSHOT L C > anatomy_chain.jsonl`; 3 minutes | for the same games, the sizes of the successive conflicts of the root GF(2) bound's heaviest set (`matroid`'s research intersection, `common/meet.rs`, whose bound equals `wall`'s `root_bound` at all 509 games) and the cores that set runs into, frame by frame (movers, promoted, factors, movers with three or more live factors) |
| `resolved_1301_2175.jsonl` | `head -n 1300 results/strict.jsonl > RESOLVE.jsonl`, `taxman solve 1301-2175 --threads 1 --out RESOLVE.jsonl --checkpoint-dir DIR --checkpoint-interval 300 --progress 600` (`nice -n 10`, `taxman` built from master ee03872), then `./gap.py join-resolved RESOLVE.jsonl > resolved_1301_2175.jsonl`; 24 minutes | the committed chain's searched games of 1301-2175 solved again by the EC2 chain's build, in order on one thread (every score and proof is the committed chain's): the game's root gap (`bench/gf2/roots.jsonl`), its nodes, seconds, method and proof from this run beside the committed chain's nodes, so that the two chains' games are on one build |
| `gap.py`, `gap_output.md` | `./gap.py > gap_output.md` (a table at a time: `./gap.py bands trend arithmetic window build cost runs anatomy projection`) | the tables of [the gap past 2175](../../docs/experiments/the-gap-past-2175-2026-10.md) from `gaps_chain.jsonl`, `anatomy_chain.jsonl`, `resolved_1301_2175.jsonl`, `results/strict.jsonl`, `bench/gf2/roots.jsonl`, `bench/cloud/c8g48/games.jsonl`, `bench/cloud/c8g.chain-2176.jsonl` and `setup_c8g.jsonl`; the module comment says what each table is |
| `widths_1300.jsonl`, `widths_2170.jsonl` | `wall nodes K --mode opt --nodes 5000 --width W N`, each width twice | the same proof at several set widths |
| `widths_3000.jsonl` | `wall nodes --no-seed --nodes 5000 --width W 3000`, W = 48, 56, 64, twice | the same, past 2175 |
| `ckpt_4000.jsonl` | `wall nodes --no-seed --nodes 5000 --ckpt-dir DIR --every 0 4000`, then `--every 1` | what checkpoints cost a thread at 4000 |
| `grow_1430.jsonl`, `.progress` | `wall grow K --delta 0 --nodes 1000000 --progress 2 --ckpt-dir DIR --every 10 1430` | a GF(2) proof of 706,620 nodes: monitor lines and summary (stdout), progress lines (stderr) |
| `grow_1230_matching.jsonl`, `.progress` | `wall grow K --tactic gf2=off --tactic lagrange=on --nodes 15000000 --progress 2 --ckpt-dir DIR --every 10 1230` | a matching-path proof of 10.45 million nodes |
| `grow_2170_t64c1.jsonl`, `.progress` | `wall grow K --threads 64 --cores 1 --seconds 90 --progress 10 --ckpt-dir DIR --every 10 2170` | 64 workers taking turns on one CPU: queue, checkpoint size, memory |
| `frames.jsonl` | `wall frames 1024 2048 3000 4095 4096 4500 5000 5500 6000 6500 7000 7500 8000 8191` | the frames, sizes and ranks beyond the widest set |
| `frames_all.jsonl` | `wall frames 2-8191` (the build of ee03872, in four processes), each line cut to `n`, `frames`, `max_freedoms`, `block_words`, `candidates`, `members` | every board to 8191: what bounds `MAX_FRAMES` (9) and the GF(2) bound's block words (5) ([docs/EXPERIMENTS.md](../../docs/experiments/boards-past-4095-2026-10.md)) |
| `past4095_setup.jsonl` | `wall setup 4000` by the builds before (`"build":"base"`, ee03872) and after (`"new"`) the boards past 4095, and `wall setup --width W 4100` for W = 72, 80, 96, 128 (a build whose `--width` list had 72, 80, 96 and 112 as well; add them to `at_width!` to run it again) | the fixed costs past 4095, by set width |
| `past4095_standalone.jsonl` | `taxman solve N --threads 4 --time-limit 10 --out FILE` for N = 4096..4110, one invocation each (the build of 4316168, before its commit) | the first games past 4095, standalone (no seed, no ceiling) and stopped at the limit: incomplete lines, measurements and not results |
| `past4095_nodes.jsonl` | `wall nodes --no-seed --nodes K 4000` (both builds) and `--width W 4100` (W = 72, 128; the same build), K = 1 and 20,000, twice (`rep`) | the cost of a node net of the setup (the difference of the two K), before and after, and by width |
| `profile_4000_search.txt` (`.jsonl`) | macOS `sample` (1 ms) for 30 s of `wall nodes --no-seed --nodes 30000 4000`, attributed by `tools/profile/attribute.py` | where a node's time goes at 4000 |
| `profile_4000_setup.txt` | the same for 60 s of `wall setup --repeat 3 4000` | where the fixed costs go at 4000 |

In the two profiles `attribute.py` could not symbolicate the inlined frames of this binary, so its kinds and purposes
say `other`; the innermost functions are the useful part, as truncated symbols: `gf2::meet::Meet<3>::shortest_path`
(`...Meet...13s`), `Meet<3>::add` (`...3ad`), `Gf2Bound<64>::evaluate` (`...8eva`), `Meet<3>::insert_both` (`...11i`),
`Meet<3>::apply_path` (`...10a`), `model::peel::Scratch<64>::winnable` (`...peel...Scratch`),
`BoardFrames<64>::count_freedoms` (`...6Apiary...14cou`: the build of the profiles called it `hives::Apiary`),
`model::peel::reduce_from` (`...11reduce_from`).

## For a cloud machine

Items 1, 3, 4, 7 and 8 ran on a `c8g.48xlarge` on 2026-10-15, at 64 and 192 threads on the one machine: what they
showed is the entry [Scaling to 192 threads](../../docs/experiments/scaling-to-192-threads-2026-10.md), their data and
scripts are in [../cloud/](../cloud/README.md#scaling-to-192-threads-c8g48).  Items 2 and 6 have not run, and item 5
only inside `tools/cloud/verify.sh` (its times are in `../cloud/c8g48/checks.txt`).

What could not be run lightly on the laptop, as commands for an instance set up by `tools/cloud/setup.sh` (from
`rust/`, `K` as above; `K2` = `--known KNOWN` with `KNOWN` the committed results followed by the c8g chain's lines,
as for `gaps_c8g.jsonl`).  Each line is one process; the one-thread ones can run side by side, one a vCPU (the
calibration found no contention among 64 copies).

1. The fixed costs on Graviton, every 25 boards past 2175 (one thread, about 15 minutes):
   `wall setup K $(seq 2200 25 4095) > setup_c8g.jsonl`.
2. Large GF(2) trees past 2175 (sets of 36 words since round A), on one thread each, a time series of 10^7 nodes and
   more: the dear games of the c8g chain,
   `wall grow K2 --nodes 40000000 --progress 30 --ckpt-dir ck_N --every 60 N > grow_N.jsonl 2> grow_N.progress` for N =
   2184 (25.6 million nodes on 64 threads), 2260, 2268, 2254 (hours each: about 0.5 ms a node).
3. The pause of a checkpoint with 192 (or 64) threads near the wall, the real one rather than one worker's: on a copy of
   the committed results cut after 2183 and extended with the c8g lines to 2183, `taxman solve 2184 --threads 192
   --out COPY --checkpoint-dir DIR --checkpoint-interval 1 --progress 10 --time-limit 600`; the end of its progress
   line gives `pauses N (x%) max Y park Z`.  Then the same at `--checkpoint-interval 300` (the chain's default since
   round A).
4. Memory, the queue and the checkpoint at 192 threads at 4000, standalone (no game of 3999): `/usr/bin/time -v
   taxman solve 4000 --threads 192 --out scratch.jsonl --checkpoint-dir DIR --checkpoint-interval 300 --progress 10
   --time-limit 600`, with `ls -l DIR` once a minute (maximum resident set size from `time`; the queue and the pauses
   from the progress lines; `max_queue` and `checkpoints` from the incomplete results line).
5. The results check: `time taxman verify-results results/strict.jsonl` on the instance, and on the instance's own
   results file once the chain has gone past 2500 (the model in the entry says 104 s at 2500 and 10 minutes at 4095).
6. What an S3 save costs: with `S3_DEST` set, `/usr/bin/time -v rust/tools/cloud/save.sh` three times while a chain
   runs (user and system seconds per save; the entry estimates 1.5 s).
7. The node rate a thread at 64 and 192 threads (the proof of 2172 scaled 58.5 times on 64 threads, with the node rate
   a thread 93 to 94% of one thread's: this measures where that 6 or 7% of a step's time goes, and what it is at 192).
   The ladder takes one known-games file, `KNOWN` of `K2`: `cat results/strict.jsonl
   bench/cloud/c8g.chain-2176.jsonl > known.jsonl` (for these two games `K` would do; `K2` serves later games too).
   On a 64-vCPU c8g.16xlarge, one process at a time:

   ```
   cargo run --release --example ladder -- --threads 1,16,64 --seed-opt --known known.jsonl --seconds 3600 2170 2172
   ```

   and on a 192-vCPU c8g.48xlarge the same with `--threads 1,64,192`.  What to read, in each game's table and the
   table summed over both: `µs/step` against the 1-thread row's.  Of the difference, `reb %` and `1st %` are the
   units' starts (the rebuild of the searcher's state, and the first node's GF(2) bound solved from scratch), which
   are part of `µs/step`; `lock %`, `ramp %` and `tail %` are time not stepping (the lock after a run, the idle time
   before all workers first ran at once, and after the last moment one finished a unit while all were running); and
   what is left in `µs/tree`, which is `µs/step` without the units' starts, is the step itself (caches, memory,
   nogoods learned by fewer steps a thread).  If the units' starts are the loss, the design to try is a unit that
   carries its node's heaviest common independent set and weight split, so that a rebuilt GF(2) level is checked
   rather than solved (see [round B](../../docs/experiments/round-b-the-chain-on-many-threads-2026-10.md)).
8. The window at 64 and 192 threads (`--window W`: W consecutive games in flight, sharing the threads).  On each
   machine, one process at a time, with the thread count in the shell variable `T` (`T=64` on the c8g.16xlarge,
   `T=192` on the c8g.48xlarge):
   * (a) The short searches, the chain to 1300 with the window at 1, 4, 8, 16 and 32, twice each; the seconds of wall
     time are in the line `solved 1..=1300 ... in X s`:

     ```
     for W in 1 4 8 16 32; do for run in 1 2; do
         echo "W=$W run $run"; taxman regress --to 1300 --threads $T --window $W 2>/dev/null | grep '^solved'
     done; done
     ```

   * (b) The chain past 2175.  A prefix of the committed results and the c8g chain's lines up to 2225 (each line
     starts `{"schema":5,"n":2176,...`, so the first `"n":` of a line is the game's):

     ```
     head -n 2175 results/strict.jsonl > prefix.jsonl
     awk -F'"n":' '{ if ($2 + 0 <= 2225) print }' bench/cloud/c8g.chain-2176.jsonl >> prefix.jsonl
     ```

     Then 51 games, 2226..2276, with the window at 1 and at 16:

     ```
     for W in 1 16; do
         cp prefix.jsonl w$W.jsonl
         time taxman solve 2226-2276 --threads $T --window $W --out w$W.jsonl --progress 0
     done
     ```

     Games an hour = 51 x 3600 over the `real` seconds of each.  Once more with the trace of the window's events, to
     see how the threads moved (`workers n=N a->b`: the threads of the search of N changed from a to b; a search
     that loses threads to an older one has b below a, and one suspended has b = 0), which searches were deferred
     (`defer`), and which were given the game before's best so far (`hint`):

     ```
     cp prefix.jsonl w16t.jsonl
     TAXMAN_WINDOW_TRACE=1 taxman solve 2226-2276 --threads $T --window 16 --out w16t.jsonl --progress 0 2> trace.err
     awk '$3 == "workers" { split($5, w, "->")
                            count[w[2] == 0 ? "suspensions" : w[2] < w[1] ? "reclaims" : "grants"]++ }
          $3 == "defer" || $3 == "hint" { count[$3 "s"]++ }
          END { for (kind in count) print kind, count[kind] }' trace.err
     ```

     The results with the window are to be the same as without it: both referees on the file, `tools/cloud/verify.sh
     w16.jsonl`, and the scores, proofs and `same_search` of its games against the c8g file's lines for the same games
     (`w1.jsonl` too; it prints the games that differ, none expected):

     ```
     python3 -c '
     import json, sys
     load = lambda path: {r["n"]: r for r in map(json.loads, open(path))}
     new, old = load(sys.argv[1]), load(sys.argv[2])
     key = lambda r: (r["score"], r["proof"], r["same_search"])
     games = [n for n in sorted(new) if n > 2225]
     bad = [n for n in games if key(new[n]) != key(old[n])]
     print(len(games), "games after 2225 in", sys.argv[1], "differ from", sys.argv[2], "at", bad)
     ' w16.jsonl bench/cloud/c8g.chain-2176.jsonl
     ```

   * What to set `WINDOW` to in `tools/cloud/chain.sh` (default 4; it was 16 until 2026-10-17): the W with the most
     games an hour in (b) at the thread count of the machine that will run the chain, if (a) points the same way; 1 if
     no W beats 1 by more than the difference between the two runs of one W in (a); and if 16 is still gaining on 8,
     try 32 in (b) first.
