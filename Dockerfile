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

COPY rust-toolchain.toml scripts/install_rust_dev_dependencies.sh .

# Install Rust toolchain and dependencies.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o install_rust.sh && \
    bash install_rust.sh -y && \
    rustup toolchain install && \
    ./install_rust_dev_dependencies.sh && \
    rm install_rust.sh && \
    rm rust-toolchain.toml && \
    rm install_rust_dev_dependencies.sh

ENTRYPOINT ["sleep", "infinity"]
