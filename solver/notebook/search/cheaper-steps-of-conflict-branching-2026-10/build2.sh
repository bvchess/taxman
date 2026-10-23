#!/bin/bash
# build2.sh SRC TARGET: build tune (feature studies) of SRC into TARGET, nice 19, JOBS jobs (default 4)
cd /home/brian/taxman-lab/work/research-stepcost/$1/solver/frame
env CARGO_HOME=/home/brian/taxman-lab/work/cargo-home CARGO_TARGET_DIR=/home/brian/taxman-lab/work/research-stepcost/$2 \
  nice -n 19 cargo build --release --offline --features studies --example tune -j ${JOBS:-4} 2>&1 | grep -v "^\s*Compiling" | tail -n 40
