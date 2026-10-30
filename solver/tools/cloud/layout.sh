# shellcheck shell=bash
# The folders of a machine that runs the chain beside other work, all under one root in the home directory
# (solver/docs/runbook/chain.md, "The machine's folders", has the table).  Source it; do not run it.  This file is the
# one place that knows the layout: status.sh sources it.  Every name below is relative to the home directory.
#
#   TAXMAN_HOME    the root                                                  default taxman_work
#
# The other folders follow from TAXMAN_HOME alone.

TAXMAN_HOME="${TAXMAN_HOME:-taxman_work}"
CHAIN_CHECKOUT="$TAXMAN_HOME/chain"
CHAIN_DATA="$TAXMAN_HOME/chain-data"
ARCHIVE_DIR="$TAXMAN_HOME/archive"
RESEARCH_DIR="$TAXMAN_HOME/research"
