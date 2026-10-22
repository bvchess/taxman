#!/bin/sh
# Nodes of fresh 4875 units written whole (wall local --out), for the scripts: run-local.sh OUT IDS
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
cd $F || exit 1
nice -n 10 target/release/examples/wall local --known $S/known4875.jsonl --suite $S/units-fresh.jsonl --ids $2 \
  --per-unit 12 --checked none --out $S/$1 4875 > $S/$1.lines 2> $S/$1.err
echo "exit $?"
