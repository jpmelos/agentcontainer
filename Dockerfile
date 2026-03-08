FROM debian:trixie-slim

# Install system dependencies.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        # Essential build tools, required by Rust.
        build-essential \
        # Enable TLS certificate verification for curl, git, etc.
        ca-certificates \
        # Download installers.
        curl \
        # Accurate times in Claude's `/resume`. Need to pass `TZ` into the
        # container.
        tzdata \
        # The agent will need access to history, commits, etc.
        git \
        # Required by `pre-commit`.
        python3 \
        # Linter manager.
        pre-commit && \
    rm -rf /var/lib/apt/lists/*

# Define user and what `HOME` will be for the runtime. Build the image as the
# same user that will run it. This avoids permission problems with mounts.
ARG UID
ARG GID
ARG HOME
RUN test -n "${UID}"
RUN test -n "${GID}"
RUN test -n "${HOME}"

ENV HOME="${HOME}"
ENV PATH="${HOME}/.cargo/bin:${HOME}/.local/bin:${PATH}"
RUN mkdir -p "${HOME}/.cargo/bin" && \
    mkdir -p "${HOME}/.local/bin" && \
    chown -R "${UID}:${GID}" "${HOME}"
RUN getent group "${GID}" || groupadd -g "${GID}" usergroup && \
    getent passwd "${UID}" || useradd -u "${UID}" -g "${GID}" -d "${HOME}" user
USER "${UID}:${GID}"
WORKDIR "${HOME}"

COPY --chown="${UID}:${GID}" \
    rust-toolchain.toml scripts/install_rust_dev_dependencies.sh .

# Install Rust toolchain and dependencies.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        -o install_rust.sh && \
    bash install_rust.sh -y && \
    rustup toolchain install && \
    ./install_rust_dev_dependencies.sh && \
    rm install_rust.sh && \
    rm rust-toolchain.toml && \
    rm install_rust_dev_dependencies.sh

ENTRYPOINT ["sleep", "infinity"]
