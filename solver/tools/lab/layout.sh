# shellcheck shell=bash
# The folders of a machine that runs the chain and the lab, all under one root in the home directory
# (solver/docs/runbook/lab.md, "The machine's folders", has the table).  Source it; do not run it.  This file is the
# one place that knows the layout: lab.sh sends it to the host with its commands, on-event.sh and wall.sh source it,
# and the settings of a study take their folders from it.  Every name below is relative to the home directory.
#
#   TAXMAN_HOME    the root                                                  default taxman_work
#   LAB_HOME       the lab                                                   default $TAXMAN_HOME/lab
#   LAB_CKPT_DIR   where the chain keeps its checkpoints; only read          default $TAXMAN_HOME/chain-data/ckpt
#   LAB_TOOLS      the tools of the chain's checkout                         default $TAXMAN_HOME/chain/solver/tools
#
# The other folders follow from TAXMAN_HOME alone.

TAXMAN_HOME="${TAXMAN_HOME:-taxman_work}"
CHAIN_CHECKOUT="$TAXMAN_HOME/chain"
CHAIN_DATA="$TAXMAN_HOME/chain-data"
ARCHIVE_DIR="$TAXMAN_HOME/archive"
RESEARCH_DIR="$TAXMAN_HOME/research"
LAB_HOME="${LAB_HOME:-$TAXMAN_HOME/lab}"
LAB_CKPT_DIR="${LAB_CKPT_DIR:-$CHAIN_DATA/ckpt}"
LAB_TOOLS="${LAB_TOOLS:-$CHAIN_CHECKOUT/solver/tools}"

# The folders beside the lab that a study may read and not write.
CHAIN_DIRS=("$CHAIN_CHECKOUT" "$CHAIN_DATA" "$ARCHIVE_DIR" "$RESEARCH_DIR")
