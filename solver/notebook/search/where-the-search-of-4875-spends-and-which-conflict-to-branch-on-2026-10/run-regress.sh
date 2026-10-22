#!/bin/sh
# Games 1..TO solved in memory on one thread with the rule on, every score checked against optimal.json, with the
# shortcuts off so that more games are searched: run-regress.sh OUT TO [ENV=VALUE...]
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
OUT=$1
TO=$2
shift 2
cd $F || exit 1
env "$@" nice -n 10 target-b/release/taxman regress --to $TO --threads 1 --tactic same_search=off \
  --tactic prime_shortcut=off > $S/$OUT 2>&1
echo "exit $?"
