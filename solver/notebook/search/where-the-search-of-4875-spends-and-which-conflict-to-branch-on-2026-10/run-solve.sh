#!/bin/sh
# Solve units of the fresh checkpoint on one thread from 538080: run-solve.sh OUT IDS [ENV=VALUE...] [-- TUNE ARGS]
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
OUT=$1
IDS=$2
shift 2
cd $F || exit 1
env "$@" nice -n 10 target/release/examples/tune solve 4875 --known $S/known4875.jsonl --units $S/units-fresh.jsonl \
  --ids $IDS --node-limit 6000000 >> $S/$OUT 2>> $S/$OUT.err
echo "exit $?"
