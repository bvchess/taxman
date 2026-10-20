# A Linux build-and-test environment for the Taxman solver: the dry run before the cloud run.
#
# It builds the solver and the examples, the prover (solver/noframe, which prover.sh runs) and compiles the tests on
# Linux (Debian), so that a Linux-only problem shows up on your own machine, and it holds the scripts of this directory
# so that they can be tried too.  On an Apple-silicon Mac the container is linux/arm64, the instruction set of Graviton.
# The build context is the repository root, and only the files the build needs are sent (Dockerfile.dockerignore).
#
# How to build and run it: solver/docs/runbook/before-a-big-run.md, "Dry run in Docker (optional)".

ARG RUST_IMAGE=rust:1-bookworm
FROM ${RUST_IMAGE}

# python3: solver/tools/check_results.py and the prover's tools/summarize.py.  procps: the kill command, which tests
# use to send signals.  time: GNU time, which scaling.sh uses to report peak memory.  tmux: the RUNBOOK's way to keep a
# run alive over a dropped connection.
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 procps time tmux \
    && rm -rf /var/lib/apt/lists/*

# The solver finds optimal.json at run time (src/paths.rs): results/optimal.json above the working
# directory, or above the binary.  The copies below put the files where they are in a checkout, so the image needs no
# flag.
WORKDIR /taxman
COPY solver/ solver/
COPY results/ results/
COPY heuristics/results/ heuristics/results/

# Cargo builds generic code, but the scripts opt in to RUSTFLAGS="-C target-cpu=native" by default (env.sh); an image
# is built on one machine and may run on another, so the empty value below keeps it generic and the scripts leave it so.
ENV RUSTFLAGS="" \
    TAXMAN_ROOT=/taxman \
    TAXMAN_WORK=/work

WORKDIR /taxman/solver/frame
RUN cargo build --release --bins --examples \
    && cargo test --no-run

# The prover is a workspace of its own.
WORKDIR /taxman/solver/noframe
RUN cargo build --release --bins --examples \
    && cargo test --release --no-run
WORKDIR /taxman/solver/frame

# Results, checkpoints and logs go to /work: mount a volume there (docker run -v taxman-work:/work) to keep them.
RUN mkdir -p /work

CMD ["cargo", "test"]
