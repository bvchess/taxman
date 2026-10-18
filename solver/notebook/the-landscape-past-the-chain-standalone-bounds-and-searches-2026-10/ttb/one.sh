#!/bin/bash
# One standalone one-thread search of game $1 (no seed, no ceiling), its ladder line kept in this directory.
# From rust/, after `cargo build --profile test --example ladder`.
D=$(cd "$(dirname "$0")" && pwd)
nice -n 10 target/debug/examples/ladder --threads 1 --no-seed --no-cap --seconds 300 --known results/strict.jsonl "$1" \
    2>&1 | grep -E '^n=' > "$D/$1.txt"
