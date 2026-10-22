#!/bin/sh
# The release build's proof of the whole game 4050 from the committed optimum on 1 and 8 threads, conflict branching on
# (the default) and off, one run at a time: run-threads.sh BIN
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
BIN=$1
cd $F || exit 1
for t in 8 1; do
  for rule in on off; do
    OUT=$S/cost/threads-$rule-$t.jsonl
    rm -f $OUT
    /usr/bin/time $BIN solve 4050 --use-prev --force-search --threads $t --tactic conflict_branching=$rule \
      --finder-after 0 --out $OUT --progress 0 > $OUT.out 2> $OUT.err
    echo "$rule $t exit $?"
  done
done
