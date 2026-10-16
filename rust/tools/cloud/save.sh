#!/usr/bin/env bash
# Get the results (and the checkpoints) off the machine, or back onto a fresh one.
#
#   S3_DEST=s3://bucket/prefix rust/tools/cloud/save.sh             copy up: results, logs, checkpoints
#   S3_DEST=s3://bucket/prefix rust/tools/cloud/save.sh --restore   copy down, onto a machine that has none of them
#   rust/tools/cloud/save.sh                                        with no S3_DEST: says how to pull from your laptop
#
# Needs the AWS CLI (preinstalled on Amazon Linux; on Ubuntu see https://aws.amazon.com/cli/) and an instance role or
# credentials that may list, read and write that prefix.  chain.sh runs this every SAVE_EVERY seconds when S3_DEST is
# set.  The results file only grows and a checkpoint is replaced atomically, so a copy taken while a run is going is a
# valid copy of an earlier moment; the solver and the verifiers check what they read.
#
# The checkpoints are copied so that a replacement instance can resume a game that has taken hours, not start it
# again.  `--delete` keeps the copy to the checkpoints that exist (a finished game's is removed).  The spill and lock
# files are left out at any depth: certify.sh keeps a checkpoint directory of its own for each list ($CKPT/NAME).

set -eu
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

if [ -z "${S3_DEST:-}" ]; then
    cat >&2 <<EOF
S3_DEST is not set, so nothing was copied.  From your own machine, pull the results with
    rsync -av ec2-user@HOST:$TAXMAN_WORK/results/ ./results/          (ubuntu@HOST on Ubuntu)
and, to keep a copy of a game in progress, also  $CKPT/ .  Pulling needs no credentials on the instance.
EOF
    exit 0
fi
AWS_CLI="${AWS_CLI:-aws}"
command -v "$AWS_CLI" >/dev/null 2>&1 || die "the AWS CLI ($AWS_CLI) is not installed"
S3_DEST="${S3_DEST%/}"

if [ "${1:-}" = "--restore" ]; then
    if [ -e "$RESULTS" ] && [ "${FORCE:-0}" != 1 ]; then
        die "$RESULTS exists; --restore is for a fresh machine (FORCE=1 overwrites files that differ)"
    fi
    mkdir -p "$(dirname "$RESULTS")" "$CKPT"
    "$AWS_CLI" s3 sync "$S3_DEST/results" "$(dirname "$RESULTS")" --only-show-errors
    "$AWS_CLI" s3 sync "$S3_DEST/ckpt" "$CKPT" --only-show-errors
    log "restored from $S3_DEST"
    exit 0
fi

"$AWS_CLI" s3 sync "$(dirname "$RESULTS")" "$S3_DEST/results" --only-show-errors
[ -d "$LOGS" ] && "$AWS_CLI" s3 sync "$LOGS" "$S3_DEST/logs" --only-show-errors
if [ -d "$CKPT" ]; then
    "$AWS_CLI" s3 sync "$CKPT" "$S3_DEST/ckpt" --delete --only-show-errors \
        --exclude 'spill/*' --exclude '*.tmp' --exclude 'taxman.lock' \
        --exclude '*/spill/*' --exclude '*/taxman.lock'
fi
log "saved to $S3_DEST"
