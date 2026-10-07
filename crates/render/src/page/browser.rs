//! Locating `chrome-headless-shell` and starting it with a pipe to talk over.
//!
//! The browser is an external program found much as ffmpeg is ([`crate::Tools`]),
//! and checked up front so a missing one says what is wrong rather than failing
//! inside a capture; the order it is looked for in is `find`'s. Which build it
//! should be is `tools/chromium/pin`'s to say; `tools/chromium/fetch` downloads
//! that build and prints the path to put in [`CHROME_ENV`], and a program that
//! [`super::supply`]s one downloads it on first use.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc;

use serde_json::Value;

use crate::pipe::Process;

/// Overrides where the browser is found — the path `tools/chromium/fetch`
/// prints, or wherever a shipped build keeps its own.
pub const CHROME_ENV: &str = "SCORSESE_CHROME";

/// The flags every capture runs with, and why each one is there.
///
/// - `--remote-debugging-pipe`: the protocol over file descriptors 3 and 4.
/// - `--deterministic-mode`: frames are produced only when we ask for one
///   (`HeadlessExperimental.beginFrame`), never on the browser's own schedule.
/// - `--use-angle=swiftshader --enable-unsafe-swiftshader`: the same software
///   GPU on every machine. #772 measured frames from it byte-identical across
///   two CPU vendors, bar one rare six-pixel race.
/// - `--disable-gpu-rasterization`: the page's tiles are drawn by Skia on the
///   CPU and only composited on SwiftShader. Drawn on SwiftShader, a blur whose
///   radius changes (an animated `box-shadow`) came out differently from one
///   capture to the next: #874 measured 3–21 of 451 frames differing between
///   two captures of the same page, and none with this flag, over four
///   browsers at once. It is also the faster path: 55 against 60 ms a frame at
///   640×360, 182 against 235 at 1080p.
/// - Not here, but added at launch when the sandbox is off: `--no-sandbox`
///   (see [`NO_SANDBOX`]).
/// - `--hide-scrollbars`: a page taller than the frame must not grow a bar.
/// - The last three keep the page offline where request interception cannot
///   see (#839). `Fetch` pauses every HTTP request, but a WebSocket and WebRTC
///   never pass through it, and #773 measured both reaching a listener on the
///   LAN. So the wall is the browser's own networking, not a list of APIs in
///   the page: a third one nobody thought of hits it too.
///   - `--host-resolver-rules=MAP * ~NOTFOUND`: every host, IP literals
///     included, fails to resolve, so no TCP connection is made at all (a
///     WebSocket, TURN over TCP). Requests we answer never reach it.
///   - `--force-webrtc-ip-handling-policy=disable_non_proxied_udp`: WebRTC
///     sends no UDP of its own, so STUN never leaves.
///   - `--no-proxy-server`: the machine's proxy settings (`HTTPS_PROXY` and
///     the like) are not read, so a connection cannot be handed to one.
const FLAGS: &[&str] = &[
    "--remote-debugging-pipe",
    "--deterministic-mode",
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    "--disable-gpu-rasterization",
    "--hide-scrollbars",
    "--no-first-run",
    "--mute-audio",
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-default-apps",
    "--disable-sync",
    "--host-resolver-rules=MAP * ~NOTFOUND",
    "--force-webrtc-ip-handling-policy=disable_non_proxied_udp",
    "--no-proxy-server",
];

/// Turns Chromium's own sandbox off — passed only when it is off.
///
/// On is every capture's default since #853: a page is code, and locally it
/// can come from anywhere — a template, a page somebody shared, an agent's
/// mistake. #773 measured the sandbox costing nothing (129 vs 130 ms a frame,
/// byte-identical frames). It is off only where [`NO_SANDBOX_ENV`] says so, or
/// a caller asks with [`Chrome::unsandboxed`].
pub const NO_SANDBOX: &str = "--no-sandbox";

/// Set (to anything but empty) to capture without Chromium's sandbox — the one
/// opt-out, for a machine that cannot start it: one running as root, where
/// Chromium refuses the sandbox, or one whose kernel will not let it make a
/// user namespace (GitHub's ubuntu-24.04 runners, under AppArmor). Never a
/// default, and never a fallback: a sandbox that cannot start fails the
/// capture with [`ChromeError::Sandbox`], which names this variable.
pub const NO_SANDBOX_ENV: &str = "SCORSESE_CHROME_NO_SANDBOX";

/// Whether [`NO_SANDBOX_ENV`] turns the sandbox off.
fn sandbox_wanted(opt_out: Option<std::ffi::OsString>) -> bool {
    opt_out.is_none_or(|value| value.is_empty())
}

/// The browser a page is captured with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chrome {
    binary: PathBuf,
    version: String,
    sandbox: bool,
}

impl Chrome {
    /// Finds the browser — [`CHROME_ENV`] first, then the build this program
    /// downloaded ([`super::supply`]), then `chrome-headless-shell` on `PATH`,
    /// and only then downloading that build — and checks it runs.
    pub fn discover() -> Result<Self, ChromeError> {
        super::find::find().map(|found| found.chrome)
    }

    /// Uses this binary, after asking it its version.
    pub fn at(binary: impl Into<PathBuf>) -> Result<Self, ChromeError> {
        let binary = binary.into();
        let output = Command::new(&binary)
            .arg("--version")
            .output()
            .map_err(|source| ChromeError::NotFound {
                binary: binary.clone(),
                source,
            })?;
        Ok(Self {
            version: version_from(&String::from_utf8_lossy(&output.stdout)),
            binary,
            sandbox: sandbox_wanted(std::env::var_os(NO_SANDBOX_ENV)),
        })
    }

    /// Runs every capture with Chromium's own sandbox on, whatever
    /// [`NO_SANDBOX_ENV`] says — for the web app's capture container, where
    /// there is no opt-out (#594). The sandbox needs a non-root user and a
    /// kernel that lets it make a user namespace; a browser that cannot start
    /// it fails the capture ([`ChromeError::Sandbox`]) and never falls back to
    /// running without one. The version, and so the cache key, is unchanged:
    /// the sandbox decides what a page may reach, never what it draws (#773).
    pub fn sandboxed(self) -> Self {
        Self {
            sandbox: true,
            ..self
        }
    }

    /// Runs every capture without the sandbox, whatever [`NO_SANDBOX_ENV`]
    /// says — for a caller that has its own explicit opt-out
    /// (`capture-worker --no-sandbox`).
    pub fn unsandboxed(self) -> Self {
        Self {
            sandbox: false,
            ..self
        }
    }

    /// Whether captures run inside Chromium's sandbox.
    pub fn is_sandboxed(&self) -> bool {
        self.sandbox
    }

    /// Every flag a launch passes, bar the scale and the first URL.
    fn flags(&self) -> Vec<&'static str> {
        let mut flags = FLAGS.to_vec();
        if !self.sandbox {
            flags.push(NO_SANDBOX);
        }
        flags
    }

    /// What the browser calls its version, e.g. `154.0.8037.92`. Part of every
    /// capture's cache key: a different build is different pixels.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Starts the browser with its protocol on a pipe, its fonts restricted to
    /// the ones `fonts` (a fontconfig file) lists.
    ///
    /// The browser reads commands from descriptor 3 and writes to 4. The
    /// standard library hands a child only 0, 1 and 2, and moving a descriptor
    /// by hand would take `unsafe`, which this workspace forbids — so a shell
    /// does the moving: it is started with our pipes as its stdin and stdout,
    /// renumbers them `3<&0 4>&1`, and `exec`s the browser in its own place.
    #[cfg(unix)]
    pub(crate) fn launch(&self, fonts: &Path, scale: f64) -> Result<Launched, ChromeError> {
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(r#"exec "$0" "$@" 3<&0 4>&1 </dev/null >/dev/null"#)
            .arg(&self.binary)
            .args(self.flags())
            .arg(format!("--force-device-scale-factor={scale}"))
            .arg("about:blank")
            .env("FONTCONFIG_FILE", fonts)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| ChromeError::NotFound {
                binary: self.binary.clone(),
                source,
            })?;
        let to = child.stdin.take().expect("stdin was piped");
        let from = child.stdout.take().expect("stdout was piped");
        Ok(Launched::start(Process::new(child), to, from))
    }

    /// Not yet: Windows passes the protocol pipe as handles, which the standard
    /// library cannot hand a child without `unsafe` (#797).
    #[cfg(not(unix))]
    pub(crate) fn launch(&self, _fonts: &Path, _scale: f64) -> Result<Launched, ChromeError> {
        Err(ChromeError::Unsupported)
    }
}

/// A running browser: the pipe to it, and its messages arriving on a channel.
pub(crate) struct Launched {
    pub(crate) process: Process,
    pub(crate) to: ChildStdin,
    pub(crate) from: mpsc::Receiver<Value>,
}

impl Launched {
    fn start(process: Process, to: ChildStdin, from: impl Read + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || super::cdp::read_messages(from, &sender));
        Self {
            process,
            to,
            from: receiver,
        }
    }
}

/// The version out of `--version`'s one line, `Google Chrome for Testing
/// 154.0.8037.92`: its last word. A line of any other shape is kept whole, since
/// this is a record of what ran and a verbose one beats none.
fn version_from(output: &str) -> String {
    let line = output.lines().next().unwrap_or_default().trim();
    line.rsplit(' ')
        .next()
        .filter(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .unwrap_or(line)
        .to_owned()
}

/// Why the browser is unusable.
#[derive(Debug, thiserror::Error)]
pub enum ChromeError {
    /// Nothing runnable at the path resolved.
    #[error(
        "cannot run {}: {source}\n\
         download it with tools/chromium/fetch and set {CHROME_ENV} to the path it prints, \
         or put chrome-headless-shell on PATH",
        binary.display()
    )]
    NotFound {
        /// The path that was tried.
        binary: PathBuf,
        /// Why it would not start.
        #[source]
        source: std::io::Error,
    },
    /// Nothing was found, and downloading the pinned build failed.
    #[error("the page renderer could not be downloaded: {0}")]
    Fetch(String),
    /// The browser died before answering anything with its sandbox on — on
    /// Linux almost always because the sandbox could not start.
    #[error(
        "the page renderer stopped before it started, which usually means its sandbox could not \
         start: it cannot run as root, and needs the kernel to allow unprivileged user \
         namespaces. On a machine that cannot give it either, set {NO_SANDBOX_ENV}=1 to capture \
         without it"
    )]
    Sandbox,
    /// This platform cannot run a capture yet.
    #[error("capturing web pages is not supported on this platform yet")]
    Unsupported,
}

#[cfg(test)]
mod tests {
    use super::{Chrome, NO_SANDBOX, sandbox_wanted, version_from};

    #[test]
    fn the_sandbox_is_on_unless_opted_out() {
        assert!(sandbox_wanted(None));
        assert!(sandbox_wanted(Some("".into())));
        assert!(!sandbox_wanted(Some("1".into())));
    }

    #[test]
    fn no_sandbox_is_passed_only_when_the_sandbox_is_off() {
        let chrome = Chrome {
            binary: "chrome".into(),
            version: "154".into(),
            sandbox: false,
        };
        assert!(chrome.flags().contains(&NO_SANDBOX));
        assert!(!chrome.is_sandboxed());
        let sandboxed = chrome.clone().sandboxed();
        assert!(!sandboxed.flags().contains(&NO_SANDBOX));
        assert!(sandboxed.is_sandboxed());
        assert_eq!(sandboxed.version(), chrome.version());
        let unsandboxed = sandboxed.unsandboxed();
        assert_eq!(unsandboxed, chrome);
    }

    #[test]
    fn the_version_is_the_last_word_of_the_banner() {
        assert_eq!(
            version_from("Google Chrome for Testing 154.0.8037.92 \n"),
            "154.0.8037.92"
        );
        assert_eq!(version_from("HeadlessChrome\n"), "HeadlessChrome");
        assert_eq!(version_from(""), "");
    }
}
