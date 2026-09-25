# syntax=docker/dockerfile:1
#
# scorsese-server with ffmpeg beside it. Built from the repo root by
# compose.yaml; what the build may see is server.Dockerfile.dockerignore.

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

FROM debian:trixie-slim
# ffmpeg goes on PATH, which is where scorsese-render's command builder looks
# for it. curl is the health check's client.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ffmpeg curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && ffmpeg -version | head -n 1
COPY --from=build /usr/local/bin/scorsese-server /usr/local/bin/scorsese-server
# compose.yaml runs it as the host user, so the files it writes are the
# maintainer's; this is the fallback for anyone running the image bare.
USER nobody
EXPOSE 8080
CMD ["scorsese-server"]
