#!/bin/sh
# A fresh search of 4875 on one thread: the finder starts after the first step and offers 538080; a node limit stops
# it, keeping its checkpoint, from which `tune units` takes the units of the proof of 538080.
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
F=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame
L=${1:-3000000}
mkdir -p $S/fresh-ckpt
cd $F || exit 1
/usr/bin/time -l nice -n 10 target/release/taxman solve 4875 --use-prev --force-search --threads 1 --finder-after 1 \
  --checkpoint-dir $S/fresh-ckpt --checkpoint-interval 60 --node-limit $L --progress 5 --out $S/fresh-out.jsonl \
  > $S/fresh.out 2> $S/fresh.err
echo "exit $?"
