#!/usr/bin/env bash
# Several runs of run.sh at once, one thread each: from the top of the repository,
#
#   nice -n 10 solver/notebook/search/keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/parallel.sh JOBS [PROCESSES]
#
# JOBS is a file with a line for each run, the arguments of run.sh for one game and one policy (`OUT GAME SECONDS
# POLICY`).  Node counts do not depend on how many runs go at once (a search on one thread is deterministic); CPU
# seconds do (about 8% fewer instructions per CPU second with 8 busy cores than with one), so compare them only
# between runs made under the same load.  Runs already in OUT are skipped, as in run.sh.
set -euo pipefail

jobs=${1:?usage: parallel.sh JOBS [PROCESSES]}
processes=${2:-8}
xargs -P "$processes" -L 1 "$(dirname "$0")/run.sh" <"$jobs"
