# The landscape past the chain: the data and the commands

What was measured and what it showed:
[docs/EXPERIMENTS.md](../../docs/experiments/the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10.md).
This directory holds the data of that run (2026-10-17, one `c8g.16xlarge` at spot, 08:53 to 10:33 PDT, about $1.10, the
code of commit 6f57399) and of the quick laptop measurement beside it (`ttb/`), and the scripts that make the entry's
numbers.  `wall` is `target/release/examples/wall` (`cargo build --release --features studies --example wall`) and
`taxman` `target/release/taxman`, run from `rust/`.  The module comment of `examples/wall.rs` ("landscape") defines
every field of a `cal.jsonl` and `map.jsonl.gz` line.  The files
were written on the instance, synced to the project's bucket (`landscape-20261017/out`) every five minutes and at the
end, and copied from there; the run's stdout copies of the two landscape files (`cal.out` and `map.out`, byte for
byte the `.jsonl` files) are not kept.

## Files and the commands that made them

| file | command | what it is |
|---|---|---|
| `userdata.sh` | the instance's user data, as it ran | installs the build (`tools/cloud/setup.sh` at 6f57399), then runs the three stages below one after the other from a tmux session, syncs the output, and powers off; `shutdown -h +240` is its backstop |
| `log` | | the times (UTC) at which each stage began and ended: the run began 15:59:00, the calibration ended 15:59:56, the standalone searches 16:30:05, the map 17:32:14 |
| `cal.jsonl` | `wall landscape 2176-3440 --threads 64 --known results/strict.jsonl --out cal.jsonl` | the calibration, 56 s: one line for each of the 1,265 games of 2176-3440, standalone (no game of n − 1: no seed, no ceiling, no prime formula): the bounds `upper` and `lower`, their `gap`, whether they `meet`, the times, and, from the committed chain, the game's optimum `opt` and with it `root_gap` (`upper` less `opt`) and `shortfall` (`opt` less `lower`) |
| `sample.txt` | | the 64 games of the standalone searches (2196 to 3440), followed by their count |
| `sa/N.jsonl`, `sa/N.out` | for the 64 games of `sample.txt`, all at once: `taxman solve N --threads 1 --time-limit 1800 --progress 0 --out sa/N.jsonl > sa/N.out 2>&1` | the standalone search of game N, one thread, 1,800 s at most: the results file with the one line the search wrote (an unfinished search has `completed` false, `score` null and its best game's score in `best_so_far`) and what `solve` printed |
| `map.jsonl.gz` | `wall landscape 3441-8191 --threads 64 --out map.jsonl`, then gzipped | the map, 62 minutes: the same lines for the 4,751 games of 3441-8191, with `opt`, `opt_proof`, `root_gap` and `shortfall` null |
| `low.jsonl.gz` | `wall landscape 2-2175 --known results/strict.jsonl --out low.jsonl` on the laptop (2026-10-17, about two minutes, 497 CPU s), then gzipped; the command is that of `cal.jsonl` for the stretch below it (the run's own record was not kept) | the quick bounds of the 2,174 games of 2-2175 with their optima: the lines of `cal.jsonl` for the games before it |
| `plot_gap.py`, `gap_landscape.svg` | `./plot_gap.py gap_landscape.svg` | the standalone gap (`upper` less `lower`) of every game 2-8191 from `low.jsonl.gz`, `cal.jsonl` and `map.jsonl.gz`, dots of one game each and rolling medians of 101 games, and the quick upper bound less the proven optimum to 3440: an SVG with a linear y axis (it was at the repository's root; the script reproduces it byte for byte) |
| `land.py` | | what the scripts share: the paths, and loading a file of lines (gzipped or not) |
| `cal.py` | `./cal.py` | the calibration's table from `cal.jsonl` and `../../results/strict.jsonl` (the committed chain): the shortfall, the root gap and the standalone gap, and the fits of the chain's nodes (log10) to the two gaps |
| `map.py` | `./map.py` | the standalone searches against the chain's own search (`sa/`, `cal.jsonl`, `../../results/strict.jsonl`), and the map by bands of 500 games (`map.jsonl.gz`) |

The entry quotes the output of `cal.py`, `map.py` and `ttb/ttb.py`.

## The time to the best game (`ttb/`)

On the laptop, while the instance ran, with the non-release build of the ladder (`cargo build --profile test --example
ladder` in `rust/`: opt-level 2, debug assertions on, in `target/debug/examples/`): a standalone one-thread search of
each of 60 games of 1301-2175 (`games.txt`), no seed and no ceiling, 300 s at most, a few at a time under `nice -n 10`
(the driver that ran them is not kept).  Every search finished, the longest in 116 s; their sum is 452 s.

| file | what it is |
|---|---|
| `one.sh` | `ttb/one.sh N` from `rust/`: the search of game N |
| `N.txt` | its ladder line (`examples/ladder.rs`): the search's time and nodes, the game found against the known optimum, the start incumbent, and when the final game was found |
| `games.txt` | the 60 games |
| `ttb.py` | `./ttb.py`: the time at which the best game was found, as a share of the search's time, for each game and in all |

## Seeding from a neighbour (`seeding/`)

The start incumbents of 2176-3440 seeded from a neighbour's start incumbent (`wall chain`, since removed), against the
standalone start and the start seeded from the proven game of n − 1: [seeding/README.md](seeding/README.md).
