#!/bin/bash
# build.sh: builds tune and widen of research-widen/src into research-widen/target, 8 jobs (JOBS), nice 19, feature
# studies.
R=/home/brian/taxman-lab/work/research-widen
cd $R/src/solver/frame || exit 1
env CARGO_HOME=/home/brian/taxman-lab/work/cargo-home CARGO_TARGET_DIR=$R/target \
  nice -n 19 cargo build --release --offline --features studies --example tune --example widen -j ${JOBS:-8} 2>&1 \
  | grep -E "^(warning|error)|-->|Finished" | head -60
