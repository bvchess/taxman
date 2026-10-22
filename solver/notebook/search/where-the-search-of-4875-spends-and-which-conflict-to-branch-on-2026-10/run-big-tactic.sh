#!/bin/sh
# The largest fresh units of 4875 with conflict branching, each on one thread of its own, in parallel: the projection's
# line checked where it extrapolates (gaps 1,647 to 1,701).
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
cd $F || exit 1
for id in 4875-2 4875-3 4875-12 4875-18; do
  target-b/release/examples/tune solve 4875 --known $S/known4875.jsonl --units $S/units-fresh.jsonl --ids $id \
    --tactic conflict_branching=on --node-limit 12000000 > $S/big-tactic-$id.jsonl 2> $S/big-tactic-$id.err &
done
wait
cat $S/big-tactic-4875-*.jsonl > $S/big-tactic.jsonl
echo done
