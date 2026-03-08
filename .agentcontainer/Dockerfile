FROM debian:trixie-slim

RUN apt update && \
    apt install -y --no-install-recommends \
        # Enable TLS certificate verification for curl, git, etc.
        ca-certificates \
        # Download installers.
        curl \
        # Accurate times in Claude's `/resume`. Need to pass `TZ` into the
        # container.
        tzdata \
        # The agent will need access to history, commits, etc.
        git \
        # Used to search files in `@`.
        ripgrep \
        # Fuzzy matching of file names in `@`.
        fzf \
        # For reading PRs.
        gh \
        # Status line utility.
        jq \
        # Status line utility.
        bc && \
    rm -rf /var/lib/apt/lists/*

# Add Docker's official GPG key.
RUN install -m 0755 -d /etc/apt/keyrings
RUN curl -fsSL https://download.docker.com/linux/debian/gpg \
    -o /etc/apt/keyrings/docker.asc
RUN chmod a+r /etc/apt/keyrings/docker.asc

# Add the Docker repository to `apt` sources.
RUN tee /etc/apt/sources.list.d/docker.sources <<EOF
Types: deb
URIs: https://download.docker.com/linux/debian
Suites: $(. /etc/os-release && echo "$VERSION_CODENAME")
Components: stable
Signed-By: /etc/apt/keyrings/docker.asc
EOF

RUN apt update && \
    apt -y install \
        docker-ce-cli \
        docker-compose-plugin && \
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
ENV PATH="${HOME}/.local/bin:${PATH}"
RUN mkdir -p "${HOME}/.local/bin" && chown -R "${UID}:${GID}" "${HOME}"
RUN getent group "${GID}" || groupadd -g "${GID}" usergroup && \
    getent passwd "${UID}" || useradd -u "${UID}" -g "${GID}" -d "${HOME}" user
USER "${UID}:${GID}"
WORKDIR "${HOME}"

# Create mountpoints required by Claude Code.
RUN mkdir -p ".claude" && echo '{}' > ".claude.json"

# Bust cache.
ARG BUILD_DATE
RUN test -n "${BUILD_DATE}"
RUN echo "Build date: ${BUILD_DATE}"

# Install Claude Code.
ARG CLAUDE_CODE_CHANNEL="latest"
RUN test -n "${CLAUDE_CODE_CHANNEL}"
RUN curl -fsSL https://claude.ai/install.sh -o claude-install.sh && \
    bash claude-install.sh "${CLAUDE_CODE_CHANNEL}" && \
    rm claude-install.sh

ENTRYPOINT ["claude"]
