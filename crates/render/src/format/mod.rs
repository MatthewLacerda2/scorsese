//! What shape the delivered file is: the container, and the codecs inside it.
//!
//! A render used to get its container from whatever extension the output path
//! happened to have, and its codecs from two hardcoded strings in the encoder.
//! `--out cut.wmv` therefore produced an ASF file carrying H.264 — a Windows
//! Media file in name only — and said nothing, after spending the whole encode.
//! For an unattended agent a plausible-looking wrong file is more expensive
//! than a refusal, so the shape of the output is now something the caller
//! decides and something we check.
//!
//! The check is structural rather than a call somebody has to remember to
//! make: an [`OutputFormat`] cannot be constructed around a combination we do
//! not write, [`crate::settings::RenderSettings`] holds one, and the encoder
//! takes the settings. A refusal therefore happens where the settings are
//! built — before a project is loaded, before ffmpeg is even located, and
//! minutes before anything would have been encoded.
//!
//! The accepted set is deliberately short and entirely tested; it is written
//! out for a reader in `docs/output-formats.md`, and
//! `crates/render/tests/formats/` holds the page to what this module says.
//! "Whatever ffmpeg has" would not be an answer we could stand behind — which
//! ffmpeg is a shipping decision (a distro build in dev and CI, a bundled
//! sidecar in a shipped build) and a licensing one.

mod codecs;
mod containers;

use std::fmt;
use std::path::PathBuf;

pub use codecs::{AudioCodec, VideoCodec};
pub use containers::Container;

/// A container and the codecs to write it with, checked to be a combination
/// scorsese delivers.
///
/// Only constructible through [`OutputFormat::new`] and
/// [`OutputFormat::defaults_for`], so holding one is proof the combination was
/// accepted. That is why the fields are private: the invariant is the whole
/// point of the type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputFormat {
    container: Container,
    video: VideoCodec,
    audio: AudioCodec,
}

impl OutputFormat {
    /// The combination `container` is written with when nobody says otherwise.
    ///
    /// Per container rather than globally: an mp4 gets H.264 and AAC, an AVI
    /// gets what an AVI is expected to carry. A caller who names only a file
    /// still gets the right answer for that file.
    pub fn defaults_for(container: Container) -> Self {
        Self {
            container,
            video: container.default_video(),
            audio: container.default_audio(),
        }
    }

    /// `container`, with either codec overridden. `None` takes the container's
    /// own default.
    ///
    /// Refuses a combination we do not write — the point of the type, and the
    /// point of doing it here, where nothing has been spent yet.
    pub fn new(
        container: Container,
        video: Option<VideoCodec>,
        audio: Option<AudioCodec>,
    ) -> Result<Self, FormatError> {
        let video = video.unwrap_or_else(|| container.default_video());
        let audio = audio.unwrap_or_else(|| container.default_audio());
        if !container.video_codecs().contains(&video) {
            return Err(FormatError::VideoNotWritten { container, video });
        }
        if !container.audio_codecs().contains(&audio) {
            return Err(FormatError::AudioNotWritten { container, audio });
        }
        Ok(Self {
            container,
            video,
            audio,
        })
    }

    /// The container, and so the ffmpeg muxer the encode is pinned to.
    pub const fn container(self) -> Container {
        self.container
    }

    /// The video encoder the picture is written with.
    pub const fn video(self) -> VideoCodec {
        self.video
    }

    /// The audio encoder the mix is written with. Silent renders never reach
    /// it, but the format is still a whole answer either way.
    pub const fn audio(self) -> AudioCodec {
        self.audio
    }
}

impl Default for OutputFormat {
    /// mp4/H.264/AAC — what delivering a video means, and what every render
    /// did before any of this was a setting.
    fn default() -> Self {
        Self::defaults_for(Container::DEFAULT)
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({} + {})", self.container, self.video, self.audio)
    }
}

/// Why a render will not be delivered in the shape it was asked for.
///
/// Every one of these is raised before anything is spawned, decoded, or
/// encoded. That is the entire value: an encode that runs to completion and
/// hands back the wrong kind of file costs a re-render nobody knew to ask for.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    /// The output path has no extension, so it cannot supply a default and
    /// nothing named a container either.
    #[error(
        "{} has no extension to take an output format from — \
         name the container, one of: {}",
        path.display(),
        containers::names()
    )]
    NoExtension {
        /// The output path that could not answer.
        path: PathBuf,
    },

    /// The extension is not one we write. Refused rather than handed to
    /// ffmpeg to interpret: guessing is the behaviour this module removed.
    #[error(
        "scorsese does not write `.{extension}` files — it writes: {}",
        containers::names()
    )]
    UnknownExtension {
        /// The extension as it was written, without its dot.
        extension: String,
    },

    /// A container named by hand that is not one we write.
    #[error(
        "`{name}` is not a container scorsese writes — it writes: {}",
        containers::names()
    )]
    UnknownContainer {
        /// What was asked for.
        name: String,
    },

    /// A video codec named by hand that this build does not offer.
    #[error("`{name}` is not a video codec scorsese writes — it writes: {}", named(&VideoCodec::ALL))]
    UnknownVideoCodec {
        /// What was asked for.
        name: String,
    },

    /// An audio codec named by hand that this build does not offer.
    #[error("`{name}` is not an audio codec scorsese writes — it writes: {}", named(&AudioCodec::ALL))]
    UnknownAudioCodec {
        /// What was asked for.
        name: String,
    },

    /// The container will not be written with that picture codec. Sometimes
    /// because it cannot carry it, sometimes — `.wmv` and H.264 — because the
    /// file that came out would not be what its extension promises.
    #[error(
        "scorsese does not write {container} with {video}; \
         it writes {container} with: {}",
        named(container.video_codecs())
    )]
    VideoNotWritten {
        /// The container asked for.
        container: Container,
        /// The video codec it will not be written with.
        video: VideoCodec,
    },

    /// The sound half of [`FormatError::VideoNotWritten`].
    #[error(
        "scorsese does not write {container} with {audio}; \
         it writes {container} with: {}",
        named(container.audio_codecs())
    )]
    AudioNotWritten {
        /// The container asked for.
        container: Container,
        /// The audio codec it will not be written with.
        audio: AudioCodec,
    },
}

/// A comma-separated list of names, so a refusal always says what it would
/// have taken instead.
fn named<T: fmt::Display>(items: &[T]) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
