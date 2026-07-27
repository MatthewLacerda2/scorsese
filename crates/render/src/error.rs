//! Why a render stopped.

use std::fmt;
use std::path::PathBuf;

use crate::plan::PlanError;
use crate::tools::ToolsError;

/// Which end of the pipeline a failure came from. Worth naming: "ffmpeg
/// failed" is not actionable, "decoding clip.mp4 failed" is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Pulling raw frames out of one of the sources on the timeline.
    Decode,
    /// Decoding and summing audio, which is its own stage because it happens
    /// before a single picture frame is encoded.
    Mix,
    /// Writing composited frames into the deliverable — the one stage whose
    /// subject is the output file rather than something the author imported.
    Encode,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Decode => "decoding",
            Self::Mix => "mixing",
            Self::Encode => "encoding",
        })
    }
}

/// Why a render could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// ffmpeg or ffprobe is unusable, so nothing was attempted at all.
    #[error(transparent)]
    Tools(#[from] ToolsError),

    /// The timeline could not be sequenced. Nothing was decoded or written.
    #[error(transparent)]
    Plan(#[from] PlanError),

    /// The assets table and the project directory disagree — a file deleted or
    /// renamed under a project that still expects it.
    #[error("asset `{asset}` points at {}, which is not there", path.display())]
    MissingMedia {
        /// The asset whose recorded path came up empty.
        asset: String,
        /// Where the assets table said the file would be.
        path: PathBuf,
    },

    /// A clip fitted at its native size needs the source's own dimensions, and
    /// probing them failed — so there is no raster to decode into.
    #[error(
        "clip `{clip}` asks for asset `{asset}` at its native size, \
         but how big that is could not be established: {reason}"
    )]
    UnknownSourceSize {
        /// The clip asking for the native size.
        clip: String,
        /// The asset whose dimensions are unknown.
        asset: String,
        /// What ffprobe said, or why it was never asked.
        reason: String,
    },

    /// A text asset names a font file the render cannot use — missing, or not
    /// a font at all. Refused rather than quietly falling back to the shipped
    /// sans: a title in the wrong face is a re-render nobody would know to ask
    /// for until they watched the result.
    #[error("asset `{asset}` asks for the font at {}: {detail}", path.display())]
    UnusableFont {
        /// The text asset naming the font.
        asset: String,
        /// Where the project said the file would be.
        path: PathBuf,
        /// What the filesystem or the font parser said.
        detail: String,
    },

    /// The mix is written to scratch before the encoder reads it back, so a
    /// full or read-only working directory surfaces here first.
    #[error("could not write the mix to {}: {source}", path.display())]
    Scratch {
        /// The scratch file that could not be written.
        path: PathBuf,
        /// The filesystem's own complaint.
        #[source]
        source: std::io::Error,
    },

    /// The binary was there when [`crate::tools::Tools`] checked it, but would
    /// not start now.
    #[error("could not start ffmpeg for {stage}: {source}")]
    Spawn {
        /// Which process failed to launch.
        stage: Stage,
        /// Why the operating system refused.
        #[source]
        source: std::io::Error,
    },

    /// A pipe closed mid-stream, which usually means the ffmpeg on the far end
    /// died — its own account of why arrives as [`RenderError::Ffmpeg`].
    #[error("{stage} broke off part way through: {source}")]
    Pipe {
        /// Which end of the pipeline went quiet.
        stage: Stage,
        /// The read or write that hit the closed pipe.
        #[source]
        source: std::io::Error,
    },

    /// Drawing a frame failed; the compositor says which part of it.
    #[error("compositing: {0}")]
    Composite(#[from] scorsese_compositor::CompositeError),

    /// ffmpeg ran, understood the request, and refused it — an unsupported
    /// codec, an unwritable output, a source it cannot open.
    #[error("ffmpeg failed while {stage} {subject}: {message}")]
    Ffmpeg {
        /// Which process gave up.
        stage: Stage,
        /// What was being decoded or encoded — a source file, or the output.
        subject: String,
        /// ffmpeg's own stderr, trimmed.
        message: String,
    },
}
