#!/bin/bash
# The payers of the swapped moves of 4952 -> 4953 and 740 -> 741, and the numbers each pays for on the board of n.
#
#     payers.sh [TAXMAN]
#
# `taxman tables N N NUMBERS...` prints, for each number given, its maximal factors and the numbers it is a maximal
# factor of.  This script runs it for the two moves and their six payers on each board, and prints one line a number:
# how many numbers it pays for and which.  TAXMAN is a release binary (default: the chain's).
set -eu

TAXMAN=${1:-$HOME/taxman_work/chain/solver/frame/target/release/taxman}

summarize() {
  awk -F'maximal composites ' '{
    split($1, head, ":")
    gsub(/[^0-9]+/, " ", $2)
    n = split($2, numbers, " ")
    list = ""
    for (i = 1; i <= n; i++) list = list " " numbers[i]
    printf "%5s pays for %2d numbers:%s\n", head[1], n, list
  }'
}

echo "== 4952 -> 4953: the moves 2486 and 4953 and their payers, on the board of 4953"
nice -n 19 "$TAXMAN" tables 4953 4953 2486 39 381 1651 22 226 1243 | summarize
echo
echo "== 740 -> 741: the moves 374 and 741 and their payers, on the board of 741"
nice -n 19 "$TAXMAN" tables 741 741 374 39 57 247 22 34 187 | summarize
