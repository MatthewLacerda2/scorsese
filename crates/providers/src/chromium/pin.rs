//! Which build to fetch: `tools/chromium/pin`, read at compile time.
//!
//! The pin is the one place the browser's version and checksums are written
//! (#772). The shell script reads it by sourcing it; this reads the same bytes
//! with `include_str!` and splits each line on its first `=`, so there is never
//! a second copy to drift from the first.

use super::ChromiumError;

/// The pin file, as this build was compiled against it.
const PIN: &str = include_str!("../../../../tools/chromium/pin");

/// A platform Chrome for Testing publishes `chrome-headless-shell` for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Linux on x86-64.
    Linux64,
    /// macOS on Apple silicon — the maintainer's daily machine.
    MacArm64,
    /// macOS on Intel.
    MacX64,
    /// Windows on x86-64.
    Win64,
}

impl Platform {
    /// The platform this program was built for, if Chrome for Testing has one.
    pub fn this() -> Option<Self> {
        Self::of(std::env::consts::OS, std::env::consts::ARCH)
    }

    fn of(os: &str, arch: &str) -> Option<Self> {
        match (os, arch) {
            ("linux", "x86_64") => Some(Self::Linux64),
            ("macos", "aarch64") => Some(Self::MacArm64),
            ("macos", "x86_64") => Some(Self::MacX64),
            ("windows", "x86_64") => Some(Self::Win64),
            _ => None,
        }
    }

    /// Chrome for Testing's name for it, as it appears in the URL and the zip.
    pub fn name(self) -> &'static str {
        match self {
            Self::Linux64 => "linux64",
            Self::MacArm64 => "mac-arm64",
            Self::MacX64 => "mac-x64",
            Self::Win64 => "win64",
        }
    }

    /// The executable's file name inside the unpacked folder.
    pub(super) fn binary(self) -> &'static str {
        match self {
            Self::Win64 => "chrome-headless-shell.exe",
            _ => "chrome-headless-shell",
        }
    }

    /// The folder the zip unpacks into.
    pub(super) fn folder(self) -> String {
        format!("chrome-headless-shell-{}", self.name())
    }
}

/// One platform's pinned build: what to download and what it must hash to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    /// The browser version, e.g. `154.0.8037.92`.
    pub version: String,
    /// Where the zip is.
    pub url: String,
    /// The sha256 the zip must have, exactly as downloaded.
    pub sha256: String,
    /// Which platform's build this is.
    pub platform: Platform,
}

impl Build {
    /// The build `tools/chromium/pin` names for `platform`.
    pub fn pinned(platform: Platform) -> Result<Self, ChromiumError> {
        Self::from_pin(PIN, platform)
    }

    /// The build `pin` (a pin file's text) names for `platform`.
    pub(super) fn from_pin(pin: &str, platform: Platform) -> Result<Self, ChromiumError> {
        let value = |key: &str| {
            pin.lines()
                .map(str::trim)
                .filter(|line| !line.starts_with('#'))
                .filter_map(|line| line.split_once('='))
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.to_owned())
                .ok_or_else(|| ChromiumError::Pin(key.to_owned()))
        };
        let version = value("CHROME_VERSION")?;
        let url = value("CHROME_URL")?
            .replace("{version}", &version)
            .replace("{platform}", platform.name());
        let key = format!(
            "CHROME_SHA256_{}",
            platform.name().to_ascii_uppercase().replace('-', "_")
        );
        Ok(Self {
            sha256: value(&key)?,
            version,
            url,
            platform,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pin this build carries names a build for every platform, and the
    /// URL is filled in — so a pin edited into a shape this cannot read fails
    /// here, not on somebody's first page.
    #[test]
    fn the_pin_names_a_build_for_every_platform() {
        for platform in [
            Platform::Linux64,
            Platform::MacArm64,
            Platform::MacX64,
            Platform::Win64,
        ] {
            let build = Build::pinned(platform).expect("the pin names this platform");
            assert_eq!(build.sha256.len(), 64, "{platform:?}");
            assert!(!build.url.contains('{'), "{}", build.url);
            assert!(build.url.contains(platform.name()));
            assert!(build.url.contains(&build.version));
        }
        let mac = Build::pinned(Platform::MacArm64).expect("pinned");
        assert!(
            mac.url
                .ends_with("/mac-arm64/chrome-headless-shell-mac-arm64.zip")
        );
    }

    #[test]
    fn a_line_splits_on_its_first_equals_and_comments_are_skipped() {
        let pin = "# CHROME_VERSION=0\nCHROME_VERSION=1.2\nCHROME_URL=https://x/?a=b/{platform}\n\
                   CHROME_SHA256_MAC_X64=abc\n";
        let build = Build::from_pin(pin, Platform::MacX64).expect("parses");
        assert_eq!(build.version, "1.2");
        assert_eq!(build.url, "https://x/?a=b/mac-x64");
        assert_eq!(build.sha256, "abc");
        assert!(matches!(
            Build::from_pin(pin, Platform::Win64),
            Err(ChromiumError::Pin(key)) if key == "CHROME_SHA256_WIN64"
        ));
    }

    #[test]
    fn the_platforms_chrome_for_testing_publishes_and_no_others() {
        assert_eq!(Platform::of("macos", "aarch64"), Some(Platform::MacArm64));
        assert_eq!(Platform::of("linux", "x86_64"), Some(Platform::Linux64));
        assert_eq!(Platform::of("windows", "x86_64"), Some(Platform::Win64));
        assert_eq!(Platform::of("linux", "aarch64"), None);
        assert_eq!(Platform::Win64.binary(), "chrome-headless-shell.exe");
        assert_eq!(
            Platform::MacArm64.folder(),
            "chrome-headless-shell-mac-arm64"
        );
    }
}
