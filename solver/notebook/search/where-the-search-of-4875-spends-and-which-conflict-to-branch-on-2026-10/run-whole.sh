#!/bin/sh
# A whole game proved on one thread from its committed optimum: run-whole.sh OUT GAME [ENV=VALUE...]
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
R=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169
OUT=$1
GAME=$2
shift 2
cd $R/solver/frame || exit 1
env "$@" nice -n 10 target/release/examples/tune solve $GAME --known $R/results/strict.jsonl --ids root \
  --node-limit 40000000 >> $S/$OUT 2>> $S/$OUT.err
echo "exit $?"
