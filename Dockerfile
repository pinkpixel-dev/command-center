# Self-hosted Command Center.
#
# Three stages that do not wait on each other: the web bundle, the server
# binary, and the Codex CLI download. Only their outputs reach the runtime
# image, so neither Node nor the Rust toolchain ships in it.

# The Codex build this image was tested against. It has to stay at or above
# MINIMUM_CODEX_VERSION in crates/core/src/ai/providers/codex/version.rs.
# Raising it is a user-visible change: test that the provider still connects,
# because --strict-config makes a retired config key a startup failure, which
# is exactly how 0.149.1 broke the previous pin.
ARG CODEX_VERSION=0.153.4
ARG RUST_VERSION=1.98
ARG NODE_VERSION=22


# --- the web bundle ---------------------------------------------------------
FROM node:${NODE_VERSION}-bookworm-slim AS web

WORKDIR /build

# Dependencies first, so editing source does not reinstall them.
COPY package.json package-lock.json ./
RUN npm ci

COPY tsconfig.json tsconfig.node.json vite.config.ts index.html ./
COPY public ./public
COPY src ./src

# `--mode web` is what resolves @platform to the HTTP client rather than Tauri.
RUN npm run build:web


# --- the server binary ------------------------------------------------------
FROM rust:${RUST_VERSION}-bookworm AS server

# rusqlite builds SQLite from source, so this needs a C compiler.
RUN apt-get update \
 && apt-get install --no-install-recommends -y build-essential \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The desktop crate is in the workspace and is not built here, but Cargo still
# has to be able to read its manifest to resolve the workspace.
COPY src-tauri/Cargo.toml ./src-tauri/Cargo.toml
RUN mkdir -p src-tauri/src \
 && echo 'fn main() {}' > src-tauri/src/main.rs \
 && echo '' > src-tauri/src/lib.rs

RUN cargo build --release -p command-center-server


# --- the Codex CLI ----------------------------------------------------------
# Downloaded rather than installed through npm: the published binary is
# static-musl, so it needs no runtime of its own. Installing it in the image is
# also less fragile than bind-mounting the NAS host's copy, which would not be
# built for this container in the first place.
FROM debian:bookworm-slim AS codex
ARG CODEX_VERSION

RUN apt-get update \
 && apt-get install --no-install-recommends -y ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# The release names the architecture the way Rust does, which is not what dpkg
# calls it. Running the binary afterwards proves the download is the right
# architecture, rather than finding that out at runtime on the NAS.
RUN set -eu; \
    case "$(dpkg --print-architecture)" in \
      amd64) target=x86_64-unknown-linux-musl ;; \
      arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "Codex publishes no build for $(dpkg --print-architecture)" >&2; exit 1 ;; \
    esac; \
    curl --fail --silent --show-error --location \
      "https://github.com/openai/codex/releases/download/rust-v${CODEX_VERSION}/codex-${target}.tar.gz" \
      | tar -xz -C /tmp; \
    install -m 0755 "/tmp/codex-${target}" /usr/local/bin/codex; \
    codex --version


# --- the image that runs ----------------------------------------------------
FROM debian:bookworm-slim

# ca-certificates for the OpenAI and Codex connections. tini because Codex is a
# child process, and PID 1 without a reaper leaves zombies behind it. curl for
# the health check below and nothing else. bubblewrap because Codex is launched
# with sandbox_mode="read-only", and without it Codex reports that it is
# falling back to something weaker.
RUN apt-get update \
 && apt-get install --no-install-recommends -y bubblewrap ca-certificates curl tini \
 && rm -rf /var/lib/apt/lists/*

COPY --from=server /build/target/release/command-center-server /usr/local/bin/command-center-server
COPY --from=web /build/dist-web /usr/share/command-center/web
COPY --from=codex /usr/local/bin/codex /usr/local/bin/codex

# Not root. The mounted volume has to be writable by this user, which the
# entrypoint checks and says plainly rather than failing on the first write.
RUN useradd --system --create-home --uid 10001 command-center

# The Codex home goes under the data directory rather than under HOME, which is
# set here only so that anything which does look for a home directory has one.
ENV COMMAND_CENTER_DATA_DIR=/var/lib/command-center \
    COMMAND_CENTER_WEB_DIR=/usr/share/command-center/web \
    COMMAND_CENTER_ADDR=0.0.0.0:8787 \
    HOME=/home/command-center

RUN mkdir -p "$COMMAND_CENTER_DATA_DIR" \
 && chown command-center:command-center "$COMMAND_CENTER_DATA_DIR"

COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod 0755 /usr/local/bin/docker-entrypoint.sh

USER command-center
WORKDIR /var/lib/command-center

# One mount covers the library and the Codex home, because the server puts
# both under the data directory.
VOLUME ["/var/lib/command-center"]
EXPOSE 8787

# /api/health reports the schema version as well as liveness, so a server
# answering while its migrations failed does not read as healthy. The port is
# taken from the configured address rather than assumed, so overriding it does
# not quietly break the check.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl --fail --silent --show-error \
      "http://127.0.0.1:${COMMAND_CENTER_ADDR##*:}/api/health" || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/docker-entrypoint.sh"]
CMD ["command-center-server"]
