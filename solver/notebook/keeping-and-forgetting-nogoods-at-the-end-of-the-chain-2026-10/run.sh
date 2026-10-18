#!/usr/bin/env bash
# The sweep of the nogood policies: each game proved by the ladder on one thread, seeded with its own optimum, under
# each policy.  From the top of the repository (the release build of the ladder, `cargo build --release --example
# ladder` in solver/frame, must be there):
#
#   nice -n 10 solver/notebook/keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/run.sh OUT "GAME GAME ..." SECONDS POLICY...
#
# A POLICY is `default`, or the `--tactic` specs of the run joined by commas (`nogood_keep=top64`, or
# `forget_nogoods=off,nogood_cap=256`).  One line per run is appended to OUT as `GAME<TAB>POLICY<TAB>the ladder's
# line`, as soon as the run ends, and a run whose line is in OUT already is skipped: an interrupted sweep is resumed
# by starting it again with the same arguments.  The games go one after the other with every policy, so that a sweep
# stopped early has whole games.  SECONDS is the ladder's time limit for a run (a run that reaches it says OPEN).
set -euo pipefail

out=${1:?usage: run.sh OUT "GAME GAME ..." SECONDS POLICY...}
games=${2:?}
seconds=${3:?}
shift 3
ladder=${LADDER:-solver/frame/target/release/examples/ladder}
known=${KNOWN:-results/strict.jsonl}

for game in $games; do
    for policy in "$@"; do
        if [ -f "$out" ] && grep -q -F "$(printf '%s\t%s\t' "$game" "$policy")" "$out"; then
            continue
        fi
        flags=()
        if [ "$policy" != default ]; then
            IFS=, read -ra specs <<<"$policy"
            for spec in "${specs[@]}"; do
                flags+=(--tactic "$spec")
            done
        fi
        line=$("$ladder" --seed-opt --known "$known" --threads 1 --seconds "$seconds" ${flags[@]+"${flags[@]}"} "$game")
        printf '%s\t%s\t%s\n' "$game" "$policy" "$line" >>"$out"
    done
done
