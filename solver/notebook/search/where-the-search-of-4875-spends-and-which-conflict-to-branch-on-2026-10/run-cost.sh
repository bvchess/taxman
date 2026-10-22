#!/bin/sh
# The cost of a step of conflict branching: whole 4050 from its optimum, and the units 4875-36 and 4875-10 from
# 538080, one thread each, side by side, by the tune binary BIN (built from master, or from master with
# cost-variants.patch, whose variants environment variables set): run-cost.sh BIN LABEL [TUNE ARGS...]
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
R=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169
BIN=$1
LABEL=$2
shift 2
cd $R/solver/frame || exit 1
$BIN solve 4050 --known $R/results/strict.jsonl --ids root "$@" > $S/cost/$LABEL-4050.jsonl 2> $S/cost/$LABEL-4050.err &
$BIN solve 4875 --known $S/known4875.jsonl --units $S/units-fresh.jsonl --ids 4875-36 "$@" \
  > $S/cost/$LABEL-4875-36.jsonl 2> $S/cost/$LABEL-4875-36.err &
$BIN solve 4875 --known $S/known4875.jsonl --units $S/units-fresh.jsonl --ids 4875-10 "$@" \
  > $S/cost/$LABEL-4875-10.jsonl 2> $S/cost/$LABEL-4875-10.err &
wait
for f in $S/cost/$LABEL-4050.jsonl $S/cost/$LABEL-4875-36.jsonl $S/cost/$LABEL-4875-10.jsonl; do
  /Users/brian/dev/taxman_work/taxman/.venv/bin/python -I -c "
import json,sys
d=json.loads(open(sys.argv[1]).readline())
print(sys.argv[2], d['id'], 'steps', d['nodes'], 'cpu_s', round(d['cpu_s'],1), 'us/step', round(1e6*d['cpu_s']/d['nodes'],1), 'best', d['best'], d['completed'])
" $f $LABEL
done
