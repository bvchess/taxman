# Reproducing

Part of [the prover's README](../README.md).

The prover needs Rust, with the standard library and the repository's gzip decoder only; the tools need Python 3, with
its standard library.  `cargo` and the commands below run in `solver/noframe`.

```
cargo test --release                       # brute-force checks of every bound and search (about a minute)
cargo build --release
./target/release/span --from 2 --to 100000 --threads 6 --seconds 540 --out OUT.jsonl     # resumable; ~10 min
./target/release/span --from 101000 --to 1000000 --step 1000 --threads 6 --seconds 540 --out OUT.jsonl
./target/release/star --from 2 --to 20000 --threads 6 --seconds 540 --out OUT.jsonl
../tools/cloud/prover.sh 2 2530     # the proofs of 2..2530 (all of data/exact_parity_2_2531.jsonl but its last line):
                                    # `exact` over the range in resumable passes on every CPU, then the check.  It takes
                                    # 1,700 thread-hours; the steps are in solver/docs/runbook/prover.md, "On a machine
                                    # of your own".  Game 2531, prime, was added by `exact --list 2531` (a second)
./target/release/exact --from 2 --to 1300 --threads 3 --seconds 540 --checkpoints work/fresh --out OUT.jsonl
                                    # a part of it, rerun until done: the searches start from the roles of the
                                    # bottom-layers theorem (--fix-low 0 starts from the plain root); 6 thread-minutes
./target/release/exact --from 2 --to 1231 --parity 0 --fix-low 0 --threads 6 --seconds 540 --out OUT.jsonl
                                    # the Franklín–Moniot bound alone (data/exact_2_1231.jsonl, which left open
                                    # searches:)
tar -xzf data/checkpoints/exact_open_searches.tar.gz -C work   # to resume them with the parity bound:
./target/release/exact --from 990 --to 1300 --fix-low 0 --threads 3 --seconds 540 --out OUT.jsonl
cargo run --release --example parityroot 2 1300     # the root's parity bound, checked, for every game
cargo run --release --example paritycheck 2 1300    # one below each recorded score, the search finds it (~30 minutes)
bash tools/adversary_campaign.sh                                                          # data/adversary/
./target/release/optsets --from 2 --to 989 --searched-only 1 --threads 3 --seconds 540 --budget 4000000 \
    --out OUT.jsonl                                     # the games that need a search; rerun until done
./target/release/optsets --from 2 --to 989 --out OUT.jsonl     # then the games at their ceiling, the primes and the
                                    # semiprime games (J3), derived
./target/release/optsets --list 798,799,800 --upper fixed --threads 1 --seconds 540 --out FIXED.jsonl
./target/release/moves --from 2 --to 1300 --seconds 540 --out OUT.jsonl         # one thread; ~7 minutes
tools/summarize.py check 2 2531 data/exact_parity_2_2531.jsonl
                                    # every n in 2..2531 proven, scores recorded, and how many proofs rest on F4
./target/release/exact --from 2 --to 2530 --budget 1 --out PRE.jsonl    # no search, only the roles' checks (about a
                                    # second a game); each line's "relation" says whether the game needs a search
                                    # (none) or rests on the game before it (chain, prime, semiprime)
./target/release/exact --from 2532 --to 5060 --budget 1 --threads 2 --out data/rootbounds_2532_5060.jsonl
                                    # the root bounds of the chain's games past 2531 (the file proves nothing): with no
                                    # search, each line has the game's two bounds and the score replayed from the
                                    # chain.  The build of commit d5a5b7ca took 1,245 s on two threads; sort the lines
                                    # by n
./target/release/exact --from 2 --to 989 --parity 1 --threads 2 --out data/compare_parity_2_1000.jsonl      # 14 s
./target/release/exact --from 2 --to 989 --parity 0 --threads 2 --out data/compare_fm_2_1000.jsonl          # 159 s
./target/release/exact --from 990 --to 1000 --parity 1 --threads 2 --out data/compare_parity_2_1000.jsonl   # 6 s
./target/release/exact --from 990 --to 1000 --parity 0 --threads 2 --out data/compare_fm_2_1000.jsonl       # 385 s
                                    # the same searches with only the bound changed, to compare what the bounds cost: the
                                    # parity bound, then the Franklín–Moniot bound alone, both from the roles of the
                                    # bottom-layers theorem, made as two runs each into one file (sort the lines by n).
                                    # The build of commit d5a5b7ca ran them at nice 10 on two threads.  A game counts as
                                    # searched when its "search" is proven and its "relation" is none: 241 games and
                                    # 66,945 nodes with the parity bound, 294 and 9,423,960 with the other
./target/release/upper --from 1301 --to 2175 --out data/upper_1301_2175.jsonl          # one thread, 5 minutes
./target/release/upper --from 2176 --to 3314 --out data/upper_2176_3314.jsonl          # 21 minutes
./target/release/upper --from 3315 --to 3440 --out data/upper_3315_3440.jsonl          # 5 minutes
./target/release/upper --from 3441 --to 5060 --threads 2 --out data/upper_3441_5060.jsonl   # 1,533 s at nice 10, the build of
                                    # commit 0a8fdf7e; sort the lines by n
./target/release/exact --list 1575,1584 --out B.jsonl                   # study E6: the sources of layer >= 3 free
./target/release/exact --list 1575,1584 --fix-upper 1 --out D.jsonl     # ... and fixed to U*
tools/summarize.py sources B.jsonl D.jsonl          # study E6's table and criterion (its entry for the cloud run)
tools/summarize.py span|star|exact|cost|adversary|optsets|moves FILE...
tools/crosscheck_optsets.py data/optsets_2_989.jsonl --max 57 --budget 120     # theory-free, about 8 minutes
tools/crosscheck_greedy.py data/span_2_100000.jsonl.gz --max 1000
cargo run --release --example inspect 9170 4611     # a jam, its factorizations and bounds
cargo run --release --example exactdiag 990 310049 --parity 1   # the search's counters
```

Every campaign appends one JSON line per game and skips the games already in its output, so a stopped run is resumed by
running it again; `exact` also checkpoints the searches in progress (`work/checkpoints`).  The committed data is gzipped
where large (`gunzip -k` to resume into it).  The commands marked study E6 measure whether leaving the sources of layer
3 or more free costs the prover's search anything, the theory track's measurement E6 ([its
entry](https://github.com/bvchess/taxman-notebook/blob/main/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md#the-plan-of-e6)).

## The recorded moves

`exact`, `upper`, `optsets`, `moves` and the examples `parityroot` and `paritycheck` check games against the recorded
moves of the committed chain, the file `results/optimal.jsonl.gz` ([what it holds](../../../results/README.md)).  They
read it through [the gzip decoder](../../gzip/README.md), so the commands run from `solver/noframe` need nothing but
the file.

The four programs take two flags for other files of recorded moves.  `--optimal FILE` names a file in place of the
committed chain, and each `--results FILE` adds one, such as the output of a chain run that reaches past the committed
chain.  The committed chain, or the `--optimal` file, comes first, then the `--results` files in the order given, and
the earliest file that has game n gives it.  A path that ends in `.gz` is gzip, and any other is plain result lines.
