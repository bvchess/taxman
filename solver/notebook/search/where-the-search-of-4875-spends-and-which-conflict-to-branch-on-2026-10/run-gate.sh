#!/bin/sh
# The rule with a gap gate on 4548-13, 4875-36 and the whole game 4050 (the first build): run-gate.sh OUT ENV=VALUE...
# The settings were the study patch's environment variables (the patch is not kept), given as the arguments.
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
OUT=$1
shift
sh $S/run-suite.sh $OUT 4548 4548-13 "$@"
sh $S/run-solve.sh $OUT 4875-36 "$@"
sh $S/run-whole.sh $OUT 4050 "$@"
