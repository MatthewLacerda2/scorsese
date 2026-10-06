# syntax=docker/dockerfile:1
#
# scorsese-server with ffmpeg and the pinned Chromium beside it — one image
# for the `server` and for every page capture the launcher starts — and, as the
# `launcher` target, the capture launcher's own image. Built from the repo root
# by compose.yaml; what the build may see is server.Dockerfile.dockerignore.

# The base image's toolchain is a head start, not the authority:
# rust-toolchain.toml is copied in, so if the two ever disagree rustup fetches
# the pinned one and the binary is built by the compiler CI uses. Bump this tag
# with the pin to skip that download.
FROM rust:1.96.0-slim-trixie AS build
WORKDIR /src
COPY . .
# The cache mounts are this machine's warm `target/` (CLAUDE.md, "A warm
# `target/` is the fast path"): an update recompiles what changed, not the
# whole dependency tree. They live in BuildKit's cache and never in an image,
# so the binary is copied out in the same step.
ARG CARGO_BUILD_JOBS
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release --locked --package scorsese-server \
    && cp target/release/scorsese-server /usr/local/bin/scorsese-server

# The pinned page renderer (#772), baked in: the hosted server never downloads
# one (#776), so nothing is fetched at request time. tools/chromium/fetch checks
# the zip against the pin's sha256 and fails the build on a mismatch. A stage of
# its own so unzip never reaches the image.
FROM debian:trixie-slim AS chromium
RUN apt-get update \
    && apt-get install --yes --no-install-recommends bash curl unzip ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY tools/chromium /src/tools/chromium
RUN mv "$(dirname "$(/src/tools/chromium/fetch /tmp/chromium)")" /opt/chromium

# The capture launcher (#852): the one container holding the Docker socket, so
# it carries as little as it can — the Docker client, and the server's binary,
# whose `capture-launcher` writes out the only `docker run` it ever makes. No
# ffmpeg and no browser: the captures run in the image below, not in this one.
FROM debian:trixie-slim AS launcher
RUN apt-get update \
    && apt-get install --yes --no-install-recommends docker-cli \
    && rm -rf /var/lib/apt/lists/* \
    && docker --version
COPY --from=build /usr/local/bin/scorsese-server /usr/local/bin/scorsese-server
USER nobody
CMD ["scorsese-server", "capture-launcher", "--help"]

# The server's image, and every capture's. Last, so it is what a build without
# a target makes.
FROM debian:trixie-slim
# ffmpeg goes on PATH, which is where scorsese-render's command builder looks
# for it. curl is the health check's client. The rest are the libraries
# chrome-headless-shell links against that ffmpeg does not already bring
# (#773 measured them adding ~9 MB); the `ldd` line fails the build if one is
# ever missing, rather than the first capture.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ffmpeg curl ca-certificates \
        libglib2.0-0t64 libnspr4 libnss3 libatk1.0-0t64 libatk-bridge2.0-0t64 \
        libatspi2.0-0t64 libdbus-1-3 libx11-6 libxcb1 libxcomposite1 libxdamage1 \
        libxext6 libxfixes3 libxrandr2 libxkbcommon0 libgbm1 libexpat1 libudev1 \
        libasound2t64 \
    && rm -rf /var/lib/apt/lists/* \
    && ffmpeg -version | head -n 1
COPY --from=chromium /opt/chromium /opt/chromium
# Where both the server (for the build's version, the captures' cache key) and
# each capture (to run it) find the browser.
ENV SCORSESE_CHROME=/opt/chromium/chrome-headless-shell
RUN ! ldd "$SCORSESE_CHROME" | grep 'not found' \
    && "$SCORSESE_CHROME" --version
COPY --from=build /usr/local/bin/scorsese-server /usr/local/bin/scorsese-server
# compose.yaml runs it as the host user, so the files it writes are the
# maintainer's; this is the fallback for anyone running the image bare.
USER nobody
EXPOSE 8080
CMD ["scorsese-server"]
