#!/usr/bin/env bash
# Readies a fresh cloud container for `make gates` (#722).
#
# Claude Code runs this at the start of every session (`.claude/settings.json`).
# Locally it does nothing: the machine is the operator's, already set up, and a
# hook that installed things there would be a surprise. In a cloud container
# (`CLAUDE_CODE_REMOTE=true`) it does, the same way every time, what each cloud
# coder used to learn by hitting it — `.claude/skills/issue-batch/cloud-brief.md`
# keeps only what a coder still has to *decide*.
#
# Three properties, all load-bearing:
#
# - Idempotent. Every step checks before it acts, so a resumed session, or a
#   second run by hand, costs seconds and changes nothing.
# - Best-effort. A step that fails prints what to do instead and the next one
#   runs; the hook always exits 0. A broken setup step must never be what stops
#   a session from starting — the gate that needs the tool says so itself.
# - Timed. Each line carries how long its step took, so a slow step is visible
#   the day it becomes slow rather than as a vaguely longer session start.
#
# Run it by hand as a fresh container would:
#   CLAUDE_PROJECT_DIR=$PWD .claude/hooks/session-start.sh

set -u

[ "${CLAUDE_CODE_REMOTE:-}" = "true" ] || exit 0
cd "${CLAUDE_PROJECT_DIR:-$(dirname "$0")/../..}" || exit 0

# The versions CI and the brief pin. cargo-deny must be 0.19 or newer: older
# releases cannot parse the advisory database's CVSS 4.0 entries. cargo-mutants
# is the version `.github/workflows/mutation-sweep.yml` runs.
NEXTEST=0.9.146
DENY=0.20.2
MUTANTS=27.1.0

SUDO=
[ "$(id -u)" -eq 0 ] || SUDO=sudo
BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
failed=0

# `step NAME FUNCTION`: run it, time it, and on failure say so without stopping.
step() {
    local start=$SECONDS out
    if out=$("$2" 2>&1); then
        printf 'session-start: %-10s ok   %3ss  %s\n' "$1" $((SECONDS - start)) "${out##*$'\n'}"
    else
        failed=1
        printf 'session-start: %-10s FAIL %3ss\n%s\n' "$1" $((SECONDS - start)) "$out"
    fi
}

# Exported for every later Bash call, through the file Claude Code sources after
# this hook. Run by hand there is no such file, so the lines are printed at the
# end instead, for the caller to run.
ENV_FILE=${CLAUDE_ENV_FILE:-$(mktemp)}
export_var() {
    grep -qxF "export $1=$2" "$ENV_FILE" 2>/dev/null || echo "export $1=$2" >> "$ENV_FILE"
    echo "$1=$2"
}

toolchain() {
    # rust-toolchain.toml's channel, with rustfmt and clippy.
    rustup toolchain install >/dev/null && rustc --version
}

hooks() {
    git config core.hooksPath .githooks && echo "core.hooksPath -> .githooks"
}

ffmpeg_() {
    # Ubuntu 24.04's is 6.1.1, the one CI's golden renders gate under.
    command -v ffmpeg >/dev/null \
        || { $SUDO apt-get update -qq && $SUDO apt-get install -y -qq --no-install-recommends ffmpeg; } >/dev/null \
        || { echo "install it: apt-get install ffmpeg"; return 1; }
    ffmpeg -version | head -1 | cut -d' ' -f1-3
}

# `prebuilt NAME VERSION URL`: the release tarball into ~/.cargo/bin, unless the
# right version is already there. Seconds, where `cargo install` is minutes; the
# GitHub release downloads pass the container's proxy (`get.nexte.st` does not).
prebuilt() {
    if "$BIN/$1" --version 2>/dev/null | grep -qF "$2"; then echo "$1 $2"; return; fi
    local tmp
    tmp=$(mktemp -d)
    curl -sSfL "$3" | tar xz -C "$tmp" \
        && install -m 755 "$(find "$tmp" -type f -name "$1" | head -1)" "$BIN/$1" \
        || { rm -rf "$tmp"; echo "install it: cargo install --locked $1@$2"; return 1; }
    rm -rf "$tmp"
    echo "$1 $2"
}
nextest() {
    prebuilt cargo-nextest "$NEXTEST" "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-$NEXTEST/cargo-nextest-$NEXTEST-x86_64-unknown-linux-gnu.tar.gz"
}
deny() {
    prebuilt cargo-deny "$DENY" "https://github.com/EmbarkStudios/cargo-deny/releases/download/$DENY/cargo-deny-$DENY-x86_64-unknown-linux-musl.tar.gz"
}
mutants() {
    prebuilt cargo-mutants "$MUTANTS" "https://github.com/sourcefrog/cargo-mutants/releases/download/v$MUTANTS/cargo-mutants-x86_64-unknown-linux-gnu.tar.gz"
}

postgres() {
    # The image's native Postgres 16, not `tools/with-postgres`' docker
    # container: no daemon to start, no image to pull, and so no Docker Hub
    # rate limit (a second pull minutes after the first got 429). The server's
    # tests only create and drop databases, which 16 does the same as CI's 17.
    command -v pg_ctlcluster >/dev/null || { echo "no native Postgres: start dockerd and let tools/with-postgres pull one"; return 1; }
    pg_lsclusters -h | grep -q '^16 main .* online' || $SUDO pg_ctlcluster 16 main start || return 1
    local sql="ALTER USER postgres PASSWORD 'postgres'"
    if [ -z "$SUDO" ]; then su postgres -c "psql -qc \"$sql\"" >/dev/null || return 1
    else sudo -u postgres psql -qc "$sql" >/dev/null || return 1; fi
    export_var SCORSESE_TEST_DATABASE_URL postgres://postgres:postgres@localhost:5432/postgres
}

disk() {
    # The container's disk allowance is about 30 GB and a debug build with debug
    # info filled it (#588). Without debug info the whole test build is ~2 GB.
    export_var CARGO_PROFILE_DEV_DEBUG 0
    export_var CARGO_INCREMENTAL 0
}

app_libs() {
    # The libraries egui, winit and the sound stack link against: the list in
    # ci.yml's *Install the graphics, windowing and sound libraries* step, which
    # this follows. About 13 s in a fresh container, for the gate of a branch
    # touching `app/` -- and a schema bump touches its fixtures, so that is not
    # only app work. Nothing when they are already there.
    local pkgs="libgtk-3-dev libxkbcommon-dev libwayland-dev mesa-vulkan-drivers libvulkan1 libasound2-dev"
    # shellcheck disable=SC2086
    dpkg -s $pkgs >/dev/null 2>&1 \
        || { $SUDO apt-get update -qq && $SUDO apt-get install -y -qq --no-install-recommends $pkgs; } >/dev/null \
        || { echo "install them: apt-get install $pkgs"; return 1; }
    echo "graphics, windowing and sound libraries"
}

bun_() {
    # Only the web gate needs it, but it is one small binary: cheaper to have
    # than to have a coder discover it. The version is web/package.json's pin.
    local want
    want=$(grep -o '"bun@[0-9.]*"' web/package.json | tr -d '"' | cut -d@ -f2)
    if [ "$("$HOME/.bun/bin/bun" --version 2>/dev/null)" != "$want" ]; then
        curl -fsSL https://bun.sh/install | bash -s "bun-v$want" >/dev/null \
            || { echo "install it: https://bun.com/docs/installation (bun $want)"; return 1; }
    fi
    command -v bun >/dev/null || export_var PATH "$HOME/.bun/bin:\$PATH" >/dev/null
    echo "bun $want"
}

chromium() {
    # The pinned chrome-headless-shell, which `html` clips are captured with and
    # their golden renders gate on (#775). The fetch script verifies it against
    # tools/chromium/pin and downloads nothing on a second run.
    local chrome
    chrome=$(tools/chromium/fetch 2>&1 | tail -1)
    [ -x "$chrome" ] || { echo "$chrome"; return 1; }
    # Captures run inside Chromium's sandbox (#853), which refuses to start as
    # root — and a cloud container is root. The opt-out is the documented one.
    if [ "$(id -u)" = 0 ]; then
        export_var SCORSESE_CHROME_NO_SANDBOX 1 >/dev/null
    fi
    export_var SCORSESE_CHROME "$chrome"
}

step toolchain toolchain
step hooks hooks
step ffmpeg ffmpeg_
step nextest nextest
step deny deny
step mutants mutants
step postgres postgres
step disk disk
step bun bun_
step app-libs app_libs
step chromium chromium

if [ -z "${CLAUDE_ENV_FILE:-}" ]; then
    echo "session-start: not run by Claude Code, so export these yourself:"
    cat "$ENV_FILE" && rm -f "$ENV_FILE"
fi
[ "$failed" -eq 0 ] || echo "session-start: a step failed -- the lines above say what to do; nothing else was skipped."
exit 0
