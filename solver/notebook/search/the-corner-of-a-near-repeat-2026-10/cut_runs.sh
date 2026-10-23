#!/bin/bash
# The near-repeat cut on the second game of three swapped pairs, on and off, one thread each.
#
#     cut_runs.sh [TAXMAN]
#
# Run from the top of the repository.  For each n it copies the committed chain up to n - 1 to a scratch directory,
# solves n from that copy with the tactic near_repeat on and off, and prints the nodes, the cuts and the score of the
# line that the solve appended.  TAXMAN is the release binary of the build to measure (default: the chain's).
set -eu

TAXMAN=${1:-$HOME/taxman_work/chain/solver/frame/target/release/taxman}
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT

for n in 741 1209 1833; do
  for t in on off; do
    copy=$SCRATCH/strict-$n-$t.jsonl
    head -n $((n - 1)) results/strict.jsonl > "$copy"
    nice -n 19 "$TAXMAN" solve "$n" --use-prev --force-search --threads 1 --progress 0 \
      --tactic near_repeat=$t --out "$copy" > /dev/null
    python3 -I - "$copy" "$t" <<'EOF'
import json
import sys

with open(sys.argv[1]) as lines:
    game = json.loads(lines.readlines()[-1])
print(f"n={game['n']} near_repeat={sys.argv[2]} nodes={game['nodes']} "
      f"cuts={game['stats'].get('near_repeat_cut', 0)} score={game['score']}")
EOF
  done
done
