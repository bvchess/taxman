#!/bin/sh
# A whole game proved on one thread from its committed optimum, with tune's --tactic flags, by the second build:
# run-whole-tactic.sh OUT GAME TACTIC...
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
R=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169
OUT=$1
GAME=$2
shift 2
cd $R/solver/frame || exit 1
nice -n 10 target-b/release/examples/tune solve $GAME --known $R/results/strict.jsonl --ids root \
  --node-limit 40000000 "$@" >> $S/$OUT 2>> $S/$OUT.err
echo "exit $?"
