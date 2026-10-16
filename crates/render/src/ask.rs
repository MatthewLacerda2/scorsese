//! What a render is asked to be, turned into what it will be — once, for
//! every surface.
//!
//! `scorsese render`, the MCP `render` tool and the web's renders each offer
//! the same choices about the delivered file, and each used to turn them into
//! [`RenderSettings`] with its own copy of the defaults and the refusals. The
//! copies drifted: the tool never held a sample rate to what the codec takes,
//! and the server built its format another way (#1289). Now a surface only
//! reads its own transport — clap flags, tool arguments, a JSON body — into a
//! [`RenderAsk`], and everything after that is said here.
//!
//! **What every default is.** The frame rate is the project's own; the size is
//! the platform's, or [`RenderAsk::resolution`]'s fallback; the sample rate is
//! [`SampleRate::DEFAULT`]. The container is the one the output path's
//! extension asks for ([`RenderAsk::format_for`]), or mp4 for a surface that
//! names its own file ([`RenderAsk::format`]).

use std::path::Path;
use std::str::FromStr;

use scorsese_core::Fps;
use scorsese_core::style::Platform;

use crate::audio::LoudnessTarget;
use crate::format::{AudioCodec, Container, FormatError, OutputFormat, VideoCodec};
use crate::settings::{Bands, Bitrate, RenderSettings, Resolution, SampleRate};

/// Every choice a render offers about the file it delivers, each `None` (or
/// [`Bands::Drawn`]) when the caller left it to its default.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderAsk {
    /// The container; `None` takes the output path's, or mp4.
    pub container: Option<Container>,
    /// The picture codec; `None` takes the container's.
    pub video_codec: Option<VideoCodec>,
    /// The sound codec; `None` takes the container's.
    pub audio_codec: Option<AudioCodec>,
    /// The canvas; never given beside `platform`, which sizes it too.
    pub resolution: Option<Resolution>,
    /// The placement the file is made for, which sizes it.
    pub platform: Option<Platform>,
    /// The delivery rate; `None` keeps the project's timeline rate.
    pub fps: Option<Fps>,
    /// The video bitrate; `None` aims for constant quality.
    pub bitrate: Option<Bitrate>,
    /// The rate the mix is produced at; `None` is [`SampleRate::DEFAULT`].
    pub sample_rate: Option<SampleRate>,
    /// The audio bitrate; `None` leaves it to the encoder.
    pub audio_bitrate: Option<Bitrate>,
    /// Whether a narration line not yet generated draws its band.
    pub bands: Bands,
    /// The loudness to deliver the soundtrack at; `None` delivers it as mixed.
    pub loudness: Option<LoudnessTarget>,
}

/// A [`RenderAsk`] as a surface receives it, spelled out — the names the MCP
/// tool and the web take, and the words they arrive in.
#[derive(Debug, Clone, Copy, Default)]
pub struct Spelled<'a> {
    /// `mp4`, `mkv`, `avi`, `wmv`; `mp3`, `wav` or `m4a`.
    pub container: Option<&'a str>,
    /// `h264`, `mpeg4` or `wmv2`.
    pub video_codec: Option<&'a str>,
    /// `aac`, `pcm_s16le`, `wmav2` or `mp3`.
    pub audio_codec: Option<&'a str>,
    /// `WIDTHxHEIGHT`.
    pub resolution: Option<&'a str>,
    /// A platform's name, e.g. `tiktok`.
    pub platform: Option<&'a str>,
    /// `30`, or a fraction like `30000/1001`.
    pub fps: Option<&'a str>,
    /// `8M`, `800k`.
    pub bitrate: Option<&'a str>,
    /// `48000`, `48k`.
    pub sample_rate: Option<&'a str>,
    /// `192k`.
    pub audio_bitrate: Option<&'a str>,
    /// `false` leaves the narration bands out.
    pub narration_bands: Option<bool>,
    /// LUFS, e.g. `-14`.
    pub loudness: Option<f64>,
}

/// Why a render is not made as asked — said before anything is opened.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AskError {
    /// A combination of container, codecs and settings scorsese does not write.
    #[error(transparent)]
    Format(#[from] FormatError),
    /// Two settings that each size the render.
    #[error("resolution and platform both size the render; give one")]
    BothSized,
    /// A value that does not read as its setting, already worded.
    #[error("{0}")]
    Unread(String),
}

impl RenderAsk {
    /// What `spelled` asks for, each value read by the type its setting is,
    /// so a value and its refusal read the same from every surface. A refusal
    /// names the argument it came in, but for the container and the codecs and
    /// the loudness, whose own words already do.
    pub fn read(spelled: &Spelled<'_>) -> Result<Self, AskError> {
        let loudness = spelled.loudness.map(LoudnessTarget::new).transpose();
        Ok(Self {
            container: spelled.container.map(own).transpose()?,
            video_codec: spelled.video_codec.map(own).transpose()?,
            audio_codec: spelled.audio_codec.map(own).transpose()?,
            resolution: named(spelled.resolution, "resolution")?,
            platform: named(spelled.platform, "platform")?,
            fps: named(spelled.fps, "fps")?,
            bitrate: named(spelled.bitrate, "bitrate")?,
            sample_rate: named(spelled.sample_rate, "sample_rate")?,
            audio_bitrate: named(spelled.audio_bitrate, "audio_bitrate")?,
            bands: match spelled.narration_bands {
                Some(false) => Bands::Omitted,
                Some(true) | None => Bands::Drawn,
            },
            loudness: loudness.map_err(|problem| AskError::Unread(problem.to_string()))?,
        })
    }

    /// The format a render to `out` is delivered in: the container asked
    /// for, else the one `out`'s extension asks for ([`OutputFormat::for_path`]).
    pub fn format_for(&self, out: &Path) -> Result<OutputFormat, FormatError> {
        OutputFormat::for_path(out, self.container, self.video_codec, self.audio_codec)
    }

    /// The format, for a surface that names its own output file and so has
    /// no extension to ask: the container asked for, else mp4.
    pub fn format(&self) -> Result<OutputFormat, FormatError> {
        let container = self.container.unwrap_or(Container::DEFAULT);
        OutputFormat::new(container, self.video_codec, self.audio_codec)
    }

    /// The canvas every layer is composited onto: the one asked for, the
    /// platform's, or 1920x1080.
    ///
    /// The one place the fallback lives, so the frame a project is made for
    /// (#1278) replaces it here for every surface at once.
    pub fn resolution(&self) -> Resolution {
        self.resolution
            .or(self.platform.map(Resolution::of))
            .unwrap_or(Resolution::HD)
    }

    /// Refuses what `format` cannot carry: every setting that only means
    /// something to a picture, for a format with none — rather than ignored,
    /// since whoever asked thought it would matter — and a sample rate the
    /// sound codec will not take, which ffmpeg would only say after the whole
    /// mix. Needs no project, so a surface asks it before opening one.
    pub fn check(&self, format: OutputFormat) -> Result<(), AskError> {
        if self.resolution.is_some() && self.platform.is_some() {
            return Err(AskError::BothSized);
        }
        for (given, setting) in [
            (self.resolution.is_some(), "a resolution"),
            (self.platform.is_some(), "a platform"),
            (self.fps.is_some(), "a frame rate"),
            (self.bitrate.is_some(), "a video bitrate"),
            (self.bands == Bands::Omitted, "leaving narration bands out"),
        ] {
            if given {
                format.picture_setting(setting)?;
            }
        }
        format.sample_rate(self.sample_rate.unwrap_or(SampleRate::DEFAULT))?;
        Ok(())
    }

    /// The settings this asks for, delivered in `format`, at the project's
    /// `timeline` rate unless another was named — once [`RenderAsk::check`]
    /// allows them.
    pub fn settings(
        &self,
        format: OutputFormat,
        timeline: Fps,
    ) -> Result<RenderSettings, AskError> {
        self.check(format)?;
        let sample_rate = self.sample_rate.unwrap_or(SampleRate::DEFAULT);
        Ok(
            RenderSettings::new(self.resolution(), self.fps.unwrap_or(timeline))
                .with_bitrate(self.bitrate)
                .with_audio(sample_rate, self.audio_bitrate)
                .with_format(format)
                .with_bands(self.bands)
                .with_loudness(self.loudness),
        )
    }
}

/// `text` read as a `T` whose refusal already says what it is about.
fn own<T>(text: &str) -> Result<T, AskError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|problem: T::Err| AskError::Unread(problem.to_string()))
}

/// `text`, if given, read as a `T`, a refusal naming the `argument`.
fn named<T>(text: Option<&str>, argument: &str) -> Result<Option<T>, AskError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    text.map(|text| {
        text.parse()
            .map_err(|problem| AskError::Unread(format!("{argument}: {problem}")))
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp4() -> OutputFormat {
        OutputFormat::defaults_for(Container::Mp4)
    }

    #[test]
    fn nothing_asked_is_the_project_rate_at_1080p_in_mp4() {
        let ask = RenderAsk::default();
        assert_eq!(ask.format(), Ok(mp4()));
        let settings = ask.settings(mp4(), Fps::SIXTY).expect("the defaults");
        assert_eq!(
            settings,
            RenderSettings::new(Resolution::HD, Fps::SIXTY).with_format(mp4())
        );
    }

    #[test]
    fn a_path_names_the_container_and_a_surface_without_one_gets_mp4() {
        let ask = RenderAsk::default();
        let mp3 = ask.format_for(Path::new("score.mp3")).expect("an mp3");
        assert!(!mp3.has_picture());
        assert!(ask.format_for(Path::new("cut")).is_err(), "no extension");
        let named = RenderAsk::read(&Spelled {
            container: Some("mkv"),
            ..Spelled::default()
        })
        .expect("mkv");
        assert_eq!(
            named.format().map(OutputFormat::container),
            Ok(Container::Mkv)
        );
    }

    #[test]
    fn a_picture_setting_is_refused_for_sound_only_and_a_platform_sizes_it() {
        let mp3 = OutputFormat::defaults_for(Container::Mp3);
        for spelled in [
            Spelled {
                fps: Some("60"),
                ..Spelled::default()
            },
            Spelled {
                platform: Some("tiktok"),
                ..Spelled::default()
            },
            Spelled {
                narration_bands: Some(false),
                ..Spelled::default()
            },
        ] {
            let ask = RenderAsk::read(&spelled).expect("read");
            let refused = ask.settings(mp3, Fps::THIRTY).expect_err("sound only");
            assert!(refused.to_string().contains("mp3"), "{refused}");
        }
        let tiktok = RenderAsk::read(&Spelled {
            platform: Some("tiktok"),
            ..Spelled::default()
        });
        let settings = tiktok.expect("read").settings(mp4(), Fps::THIRTY);
        assert_eq!(settings.expect("sized").resolution.to_string(), "1080x1920");
    }

    #[test]
    fn both_sizes_and_a_rate_the_codec_will_not_take_are_refused() {
        let both = RenderAsk::read(&Spelled {
            resolution: Some("640x360"),
            platform: Some("youtube"),
            ..Spelled::default()
        });
        let refused = both.expect("read").settings(mp4(), Fps::THIRTY);
        assert_eq!(refused, Err(AskError::BothSized));
        let wmv = OutputFormat::defaults_for(Container::Wmv);
        let fast = RenderAsk::read(&Spelled {
            sample_rate: Some("96000"),
            ..Spelled::default()
        });
        assert!(fast.expect("read").settings(wmv, Fps::THIRTY).is_err());
    }

    #[test]
    fn a_value_that_does_not_read_is_refused_by_its_argument() {
        let refused = |spelled| RenderAsk::read(&spelled).expect_err("refused").to_string();
        assert!(
            refused(Spelled {
                fps: Some("29.97"),
                ..Spelled::default()
            })
            .starts_with("fps: ")
        );
        assert!(
            refused(Spelled {
                bitrate: Some("0"),
                ..Spelled::default()
            })
            .starts_with("bitrate: ")
        );
        let container = refused(Spelled {
            container: Some("mov"),
            ..Spelled::default()
        });
        assert!(!container.starts_with("container: "), "{container}");
    }
}
