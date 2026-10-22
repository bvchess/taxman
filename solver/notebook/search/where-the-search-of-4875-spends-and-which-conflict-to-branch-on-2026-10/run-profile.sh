#!/bin/sh
# The shares of a step's CPU in the two profiles of the default (conflict branching, gate 300), one thread each, 20 s of
# samples (`/usr/bin/sample PID 20`): whole 4050 from its optimum and the unit 4875-10 from 538080.  In the build of
# 4875-10's samples the conflict choice is inlined in the check.
S=/private/tmp/claude-501/-Users-brian-dev-taxman-work-taxman/d4940f63-f84f-4c16-9150-3e813e30d03f/scratchpad/research-4875/cost
P=/Users/brian/dev/taxman_work/taxman/.venv/bin/python
cd $S || exit 1
{
  echo "# whole 4050"
  $P -I profile.py sample-master-4050.txt 'Stepper.*5check' evaluate conflict_to_branch_on certified_excluding winnable
  $P -I profile.py sample-master-4050.txt --under conflict_to_branch_on certified_excluding stuck winnable sort
} > profile-4050.txt
{
  echo "# 4875-10"
  $P -I profile.py sample-master-4875-10.txt --under 'Stepper.*5check' certified_excluding evaluate stuck winnable
} > profile-4875-10.txt
cat profile-4050.txt profile-4875-10.txt
