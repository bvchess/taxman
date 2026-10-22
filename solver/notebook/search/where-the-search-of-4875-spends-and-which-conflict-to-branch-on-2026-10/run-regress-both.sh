#!/bin/sh
# The full regress to 1300 on all the CPUs, master's build then this branch's (conflict branching on by default), one
# after the other so that neither slows the other.
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875
OLD=$S/master-src/solver/frame/target/release/taxman
NEW=/Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame/target/release/taxman
cd /Users/brian/dev/taxman_work/taxman/.claude/worktrees/agent-af2596551be4b4169/solver/frame || exit 1
/usr/bin/time $OLD regress --to 1300 > $S/regress-master.txt 2>&1
/usr/bin/time $NEW regress --to 1300 > $S/regress-branching.txt 2>&1
/usr/bin/time $OLD regress --to 1300 > $S/regress-master-2.txt 2>&1
/usr/bin/time $NEW regress --to 1300 > $S/regress-branching-2.txt 2>&1
echo done
