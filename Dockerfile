
ARG BASE_IMAGE=ubuntu:20.04

FROM $BASE_IMAGE as builder
SHELL ["/bin/bash", "-c"]

# This is being set so that no interactive components are allowed when updating.
ARG DEBIAN_FRONTEND=noninteractive

LABEL ai.basedprelude.image.authors="operations@getbased.ai" \
        ai.basedprelude.image.vendor="Based Labs" \
        ai.basedprelude.image.title="basedprelude/basednode" \
        ai.basedprelude.image.description="BasedAI Node" \
        ai.basedprelude.image.revision="${VCS_REF}" \
        ai.basedprelude.image.created="${BUILD_DATE}" \
        ai.basedprelude.image.documentation="https://docs.basedai.ai"

# show backtraces
ENV RUST_BACKTRACE 1

# Necessary libraries for Rust execution
RUN apt-get update && \
    apt-get install -y curl build-essential protobuf-compiler clang git && \
    rm -rf /var/lib/apt/lists/*

# Install cargo and Rust
RUN set -o pipefail && curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN mkdir -p /basednode && \
    mkdir /basednode/scripts

# Scripts
COPY ./scripts/init.sh /basednode/scripts/

# Capture dependencies
COPY Cargo.lock Cargo.toml /basednode/

# Specs
COPY ./snapshot.json /basednode/snapshot.json
COPY ./raw_spec.json /basednode/raw_spec.json
COPY ./raw_testspec.json /basednode/raw_testspec.json

# Copy our sources
COPY ./integration-tests /basednode/integration-tests
COPY ./node /basednode/node
COPY ./pallets /basednode/pallets
COPY ./runtime /basednode/runtime

# Update to nightly toolchain
COPY rust-toolchain.toml /basednode/
RUN /basednode/scripts/init.sh

# Cargo build
WORKDIR /basednode
RUN cargo build --release --features runtime-benchmarks --locked
EXPOSE 30333 9933 9944


FROM $BASE_IMAGE AS basednode

COPY --from=builder /basednode/snapshot.json /
COPY --from=builder /basednode/raw_spec.json /
COPY --from=builder /basednode/raw_testspec.json /
COPY --from=builder /basednode/target/release/basednode /usr/local/bin
