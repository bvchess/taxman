#!/bin/sh
# The release build's solve of game 4050 on one thread with the tactic on: run-release-4050.sh OUT [TACTIC...]
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
OUT=$1
shift
rm -f $S/$OUT
cd $F || exit 1
/usr/bin/time nice -n 10 target/release/taxman solve 4050 --use-prev --force-search --threads 1 "$@" \
  --out $S/$OUT --progress 0 > $S/$OUT.out 2> $S/$OUT.err
echo "exit $?"
