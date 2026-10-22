#!/bin/sh
# The units of the fresh checkpoint of 4875 (tune units), with the known optimum 538080.
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
cd $F || exit 1
nice -n 10 target/release/examples/tune units 4875 $S/fresh-ckpt/n4875.ckpt --known $S/known4875.jsonl \
  --sample 100000 --seed 1 --out $S/units-fresh.jsonl
