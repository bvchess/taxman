#!/bin/bash
# tests.sh: the carry patch (carry3.patch) in src-carry; the GF(2) tests in a debug build, then the GF(2) tests, the
# tripwires and the soundness tests in a release build; 4 jobs and 4 test threads, nice 19
W=/home/brian/taxman-lab/work/research-stepcost
cd $W/src-carry && git checkout -q . && git apply ../carry3.patch
cd solver/frame
export CARGO_HOME=/home/brian/taxman-lab/work/cargo-home
( time CARGO_TARGET_DIR=$W/target-dbg nice -n 19 cargo test --offline -j 4 --lib gf2::tests -- --test-threads 4 --nocapture ) > $W/test-dbg.log 2>&1
( time CARGO_TARGET_DIR=$W/target-carry nice -n 19 cargo test --offline --release -j 4 --lib gf2::tests -- --test-threads 4 --nocapture ) > $W/test-rel-gf2.log 2>&1
( time CARGO_TARGET_DIR=$W/target-carry nice -n 19 cargo test --offline --release -j 4 --test tripwires --test soundness -- --test-threads 4 ) > $W/test-rel.log 2>&1
echo finished > $W/tests.done
