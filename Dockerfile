FROM debian:trixie-slim AS base

# Install system dependencies.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        ca-certificates \
        git \
        python3 \
        pre-commit && \
    rm -rf /var/lib/apt/lists/*

ARG USERNAME

ENV HOME=/home/${USERNAME}
ENV PATH="${HOME}/.cargo/bin:${PATH}"

COPY rust-toolchain.toml .

# Install Rust toolchain and dependencies.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o install_rust.sh && \
    bash install_rust.sh -y && \
    rustup toolchain install && \
    rm install_rust.sh && \
    rm rust-toolchain.toml && \
    cargo install --locked cargo-binstall && \
    cargo binstall cargo-nextest cargo-deny cargo-machete

ENTRYPOINT ["sleep", "infinity"]
