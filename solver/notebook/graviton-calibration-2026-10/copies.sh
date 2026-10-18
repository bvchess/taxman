#!/usr/bin/env bash
# K copies of one run at once: copies.sh OUTDIR LABEL K COMMAND [ARG...]
#
# Each copy is a timed.sh run labelled LABEL.kK.cI (I = 1..K), and every "@" in the command's arguments is replaced by
# the copy's number I, so that copies can write files of their own (--out FILE.@.jsonl).  Returns when all are done.

set -u
out="$1"
label="$2"
k="$3"
shift 3
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for i in $(seq 1 "$k"); do
    args=()
    for arg in "$@"; do args+=("${arg//@/$i}"); done
    "$here/timed.sh" "$out" "$label.k$k.c$i" "${args[@]}" &
done
wait
