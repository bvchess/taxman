#!/bin/sh
# The first K steps of 4875's proof from 538080, traced by `wall anatomy --root`.
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
K=${1:-2000000}
mkdir -p $S/trace-root
cd $F || exit 1
/usr/bin/time -l nice -n 10 target/release/examples/wall anatomy --known $S/known4875.jsonl --root --nodes $K \
  --trace $S/trace-root 4875 > $S/anatomy-root.jsonl 2> $S/anatomy-root.err
echo "exit $?"
