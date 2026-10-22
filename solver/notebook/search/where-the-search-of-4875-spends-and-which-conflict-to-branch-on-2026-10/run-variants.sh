#!/bin/sh
# The rule's cheaper variants on a few units (the second build): run-variants.sh OUT ENV=VALUE...
# The settings were the study patch's environment variables (the patch is not kept), given as the arguments.
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
OUT=$1
shift
sh $S/run-suite-b.sh $OUT 3940 3940-104 "$@"
sh $S/run-suite-b.sh $OUT 4416 4416-95 "$@"
sh $S/run-suite-b.sh $OUT 4548 4548-13 "$@"
sh $S/run-solve-b.sh $OUT 4875-36,4875-10 "$@"
