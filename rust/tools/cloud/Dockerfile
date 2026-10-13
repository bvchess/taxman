# A Linux build-and-test environment for the Taxman solver: the dry run before the cloud run.
#
# It builds the solver and the examples and compiles the tests on Linux (Debian), so that a Linux-only problem shows up
# on your own machine, and it holds the scripts of this directory so that they can be tried too.  On an Apple-silicon
# Mac the container is linux/arm64, the instruction set of Graviton.  The build context is the repository root, and
# only the files the build needs are sent (Dockerfile.dockerignore).
#
# How to build and run it: RUNBOOK.md, "Dry run in Docker".

ARG RUST_IMAGE=rust:1-bookworm
FROM ${RUST_IMAGE}

# python3: tools/check_results.py.  procps: the kill command, which tests use to send signals.  time: GNU time, which
# scaling.sh uses to report peak memory.  tmux: the RUNBOOK's way to keep a run alive over a dropped connection.
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 procps time tmux \
    && rm -rf /var/lib/apt/lists/*

# The solver finds optimal.json at run time (src/paths.rs): src/main/resources/optimal.json above the working
# directory, or above the binary.  The copy below puts it where it is in a checkout, so the image needs no flag.
WORKDIR /taxman
COPY rust/ rust/
COPY src/main/resources/optimal.json src/main/resources/optimal.json
COPY heuristics/results/ heuristics/results/

# Cargo builds generic code, but the scripts opt in to RUSTFLAGS="-C target-cpu=native" by default (env.sh); an image
# is built on one machine and may run on another, so the empty value below keeps it generic and the scripts leave it so.
ENV RUSTFLAGS="" \
    TAXMAN_ROOT=/taxman \
    TAXMAN_WORK=/work

WORKDIR /taxman/rust
RUN cargo build --release --bins --examples \
    && cargo test --no-run

# Results, checkpoints and logs go to /work: mount a volume there (docker run -v taxman-work:/work) to keep them.
RUN mkdir -p /work

CMD ["cargo", "test"]
