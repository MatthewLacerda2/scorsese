---
name: install-guide
description: Install scorsese on this machine — Rust, ffmpeg, the CLI, the MCP server and the desktop app with a launcher — and offer the free extras that go well with it (yt-dlp, Kokoro voices, local Whisper). Run by the user as /install-guide.
disable-model-invocation: true
---

# Installing scorsese

You are setting scorsese up on the machine you run on, for a person who may
never have used a terminal. Work through the steps in order. **Each step checks
first and skips what is already there**; say in one line what you found and
what you are doing next. Explain nothing about Rust or ffmpeg unless asked.

- **Find the OS first**: `uname -s` (Linux, Darwin) or, on Windows, PowerShell
  is what you are in. On Linux read `/etc/os-release` for the family: Debian
  and Ubuntu (`apt-get`), Fedora (`dnf`), Arch (`pacman`). Another family: use
  its package manager for the same packages, and say so.
- **You cannot type a password.** Hand every `sudo` line to the user as
  `! sudo …` to paste at the prompt (Claude Code runs a line starting with `!`
  in their shell), wait for them, then check again.
- **Windows: one `winget` line per step**, run from PowerShell. A freshly
  installed tool is on the PATH of *new* terminals only: if a check still fails
  right after an install, look in the install folder winget printed, or ask the
  user to restart the agent in a new terminal and run `/install-guide` again
  (every finished step is skipped).
- **macOS: Homebrew.** If `brew --version` fails, hand the user the install
  line from https://brew.sh (it asks for their password) and wait.
- Nothing here costs money. Never fill in a key on the user's behalf.

## 0. A checkout

Everything below runs from a scorsese checkout (this file is in one). If the
working directory is not one (`git rev-parse --show-toplevel` fails, or it has
no `crates/` and `app/`):

```sh
git clone https://github.com/MatthewLacerda2/scorsese && cd scorsese
```

No git: `! sudo apt-get install git` / `! sudo dnf install git` /
`! sudo pacman -S git`; `xcode-select --install` on macOS (a dialog opens);
`winget install Git.Git` on Windows.

## 1. Rust

Check: `cargo --version`. Skip if it answers.

- **Linux:** a C toolchain and pkg-config first —
  `! sudo apt-get install build-essential pkg-config`,
  `! sudo dnf install gcc pkgconf-pkg-config`, or
  `! sudo pacman -S --needed base-devel`. Then
  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`
  and `. "$HOME/.cargo/env"`.
- **macOS:** `xcode-select --install` if `xcode-select -p` fails (a dialog
  opens; wait for it), then the same rustup line.
- **Windows:** the MSVC C++ build tools, then rustup:

  ```powershell
  winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
  winget install Rustlang.Rustup
  ```

## 2. ffmpeg and ffprobe

Check: `ffmpeg -version` and `ffprobe -version`, then that it can encode H.264:
`ffmpeg -hide_banner -encoders | grep libx264` (PowerShell:
`ffmpeg -hide_banner -encoders | Select-String libx264`). Skip if all three
answer. A stock build is right: scorsese composites frames itself and encodes
H.264 on the CPU, so no GPU build is needed.

- Debian/Ubuntu `! sudo apt-get install ffmpeg`; Arch `! sudo pacman -S ffmpeg`;
  macOS `brew install ffmpeg`; Windows `winget install Gyan.FFmpeg`.
- **Fedora:** its own `ffmpeg-free` lacks libx264. Hand the user these two
  lines, which swap it for RPM Fusion's:

  ```sh
  sudo dnf install https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
  sudo dnf swap ffmpeg-free ffmpeg --allowerasing
  ```

An ffmpeg the user keeps off PATH on purpose: set `SCORSESE_FFMPEG` and
`SCORSESE_FFPROBE` to the two binaries instead of installing another.

## 3. Sound headers for the desktop app (Linux only)

Check: `pkg-config --exists alsa && echo ok`. Otherwise
`! sudo apt-get install libasound2-dev`, `! sudo dnf install alsa-lib-devel`, or
`! sudo pacman -S alsa-lib`. No sound card is needed.

## 4. Build

```sh
cargo build --release
cargo build --release --manifest-path app/Cargo.toml
```

The first build takes several minutes; run it in the background and say so.
It makes `target/release/scorsese` (the CLI), `target/release/scorsese-mcp`
(the MCP server) and `app/target/release/scorsese-app` (the desktop app;
`.exe` on Windows). Re-running after a `git pull` updates them in place.

## 5. Connect the agent

Check: `claude mcp list` shows `scorsese`. Otherwise, from the checkout:

```sh
claude mcp add scorsese -- "$PWD/target/release/scorsese-mcp"
```

On Windows: `claude mcp add scorsese -- "$PWD\target\release\scorsese-mcp.exe"`.
Another MCP client takes the same binary as a stdio server: `docs/mcp.md`,
*Pointing a client at it*. The tools appear once the agent restarts; say so,
and that the remaining steps do not need them.

## 6. Keys (optional)

Editing, titling, scoring and rendering need no key. Ask whether the user
wants any of these, and skip the step if not:

- free stock footage and photos: a free Pixabay key;
- generated footage and stills: a Gemini key;
- narration: an ElevenLabs key.

If they do: `cp .env.example .env` (keep an existing `.env`) and tell them
which line to fill in; `docs/credentials.md` says where each key comes from
and how the spending ceiling works. `scorsese settings` shows which keys
resolved, never the keys.

## 7. A launcher for the desktop app

So the app shows up in the app menu, Spotlight or the Start menu, pointing at
the built binary, so a rebuild updates it. Check first that the launcher is
not already there; the commands are in `launchers.md` beside this file.

- Linux: `~/.local/share/applications/scorsese.desktop`.
- macOS: a small `scorsese.app` in `~/Applications`.
- Windows: a Start-menu shortcut. Tell the user that on Windows the animated
  titles and graphics made as pages show as placeholder cards for now; Linux
  and macOS draw them.

## 8. Extras, offered one by one

Offer each with its one-line purpose; install only what the user says yes to.

- **yt-dlp**, to bring in a video from a link (YouTube and many others).
  Check `yt-dlp --version`. Install: `! sudo apt-get install yt-dlp` /
  `! sudo dnf install yt-dlp` / `! sudo pacman -S yt-dlp`, `brew install yt-dlp`,
  `winget install yt-dlp.yt-dlp`; or `uvx yt-dlp` to run it without
  installing. Recommend **deno** beside it (check `deno --version`): without a
  JavaScript runtime yt-dlp warns that some YouTube formats may be missing.
  `brew install deno`, `winget install DenoLand.Deno`, on Linux
  `curl -fsSL https://deno.land/install.sh | sh`. yt-dlp merges picture and
  sound with the ffmpeg from step 2. What the user downloads is their own
  business; scorsese does not download anything itself.
- **Kokoro**, free voices that run on this machine. scorsese does not call it:
  you make a line with it and import the file like any other audio. It runs
  in Docker (next item); the commands are in `extras.md`.
- **Whisper**, local transcription, for captioning real footage. `scorsese
  transcribe` reaches it. Docker too; the commands are in `extras.md`.
- **Docker**, needed by the two above. Check `docker version`. Linux: Docker
  Engine from the distribution (`! sudo apt-get install docker.io`,
  `! sudo dnf install moby-engine`, `! sudo pacman -S docker`), then
  `! sudo systemctl enable --now docker` and `! sudo usermod -aG docker $USER`
  (takes effect at the next login). macOS `brew install --cask docker`,
  Windows `winget install Docker.DockerDesktop`; then the user opens Docker
  Desktop once and accepts its terms.

## 9. Check it worked

- `target/release/scorsese --help` answers.
- The libx264 check from step 2 answers.
- The user opens scorsese from the launcher and the window comes up.
- The first time a video uses an animated page, scorsese fetches its own
  headless browser (about 100–150 MB) on Linux and macOS; nothing to install.

Finish with what is installed, what was skipped and why, and the next move:
open the agent in the folder where the footage is and say what to make.
