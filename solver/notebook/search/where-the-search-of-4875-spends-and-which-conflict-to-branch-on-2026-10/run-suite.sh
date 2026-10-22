#!/bin/sh
# Solve units of the hardest-searches suite on one thread from the committed optimum: run-suite.sh OUT GAME IDS [ENV=VALUE...]
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
R=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169
F=$R/solver/frame
OUT=$1
GAME=$2
IDS=$3
shift 3
cd $F || exit 1
env "$@" nice -n 10 target/release/examples/tune solve $GAME --known $R/results/strict.jsonl \
  --units $R/solver/notebook/search/the-hardest-searches-of-the-run-to-5000-2026-10/suite-hard.jsonl \
  --ids $IDS --node-limit 6000000 >> $S/$OUT 2>> $S/$OUT.err
echo "exit $?"
