#!/bin/bash
# lk-build.sh NAME: release build of tune (feature studies) in the look builder's worktree, copied to bin/tune-NAME
. ~/.cargo/env
cd /home/brian/taxman_work/research/.claude/worktrees/agent-a0b8d02e93ffd8afe/solver/frame || exit 1
export CARGO_BUILD_JOBS=4 RUSTFLAGS="-C target-cpu=native"
unset CARGO_TARGET_DIR
nice -n 19 cargo build --release --features studies --example tune 2>&1 | grep -v "^\s*Compiling" | tail -n 30
cp target/release/examples/tune /tmp/claude-1001/-home-brian-taxman-work-research/ee866c42-a3bc-4d6b-b3f5-6d2038b6659f/scratchpad/bin/tune-$1
