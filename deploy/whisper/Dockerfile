# syntax=docker/dockerfile:1
#
# The speech-to-text server `transcribe` reaches (#1549): speaches, an
# OpenAI-style server over faster-whisper (what was measured on the
# maintainer's machine, #1549), with Whisper large-v3-turbo baked in. One image
# for the web app's stack (deploy/compose.yaml) and for a person's own machine
# (deploy/whisper/compose.yaml), CPU only.
#
# Nothing is downloaded when it runs: the model is fetched here, at build
# time, at a pinned revision, and the server is told the hub is offline. On
# the web it sits on a network with no way out (compose.yaml), since what it
# reads is a user's recording.

# Pinned by digest: an update is a deliberate change like any other.
FROM ghcr.io/speaches-ai/speaches:0.8.3-cpu@sha256:21e3df06d842fb7802ab470dd77c25f0e8c0d22950e8d8c6ae886e851af53ef8

# The model, at the revision this was built and checked against: the one
# `scorsese_providers::transcribe::MODEL` asks for, which a test holds to this
# line. A download by
# revision leaves the hub's `main` ref unwritten, and faster-whisper asks for
# `main` when it loads offline, so the ref is written to the same commit.
ARG MODEL=deepdml/faster-whisper-large-v3-turbo-ct2
ARG REVISION=4df90f75321148c3a29a9e2351b7ddf8f5b115a8
RUN huggingface-cli download "$MODEL" --revision "$REVISION" \
    && refs="$HOME/.cache/huggingface/hub/models--$(echo "$MODEL" | sed 's|/|--|g')/refs" \
    && mkdir -p "$refs" \
    && printf '%s' "$REVISION" > "$refs/main"

# The hub is never asked again; int8 on the CPU is what was measured (#1549);
# the model stays loaded between requests rather than reloading for each one;
# no web UI, since nobody browses to it.
ENV HF_HUB_OFFLINE=1 \
    WHISPER__INFERENCE_DEVICE=cpu \
    WHISPER__COMPUTE_TYPE=int8 \
    WHISPER__TTL=-1 \
    ENABLE_UI=false
EXPOSE 8000
