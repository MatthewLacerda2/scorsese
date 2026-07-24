//! Why a render stopped.

use std::fmt;
use std::path::PathBuf;

use crate::plan::PlanError;
use crate::tools::ToolsError;

/// Which end of the pipeline a failure came from. Worth naming: "ffmpeg
/// failed" is not actionable, "decoding clip.mp4 failed" is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Decode,
    Encode,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Decode => "decoding",
            Self::Encode => "encoding",
        })
    }
}

/// Why a render could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error(transparent)]
    Tools(#[from] ToolsError),

    #[error(transparent)]
    Plan(#[from] PlanError),

    #[error("asset `{asset}` points at {}, which is not there", path.display())]
    MissingMedia { asset: String, path: PathBuf },

    #[error("could not start ffmpeg for {stage}: {source}")]
    Spawn {
        stage: Stage,
        #[source]
        source: std::io::Error,
    },

    #[error("{stage} broke off part way through: {source}")]
    Pipe {
        stage: Stage,
        #[source]
        source: std::io::Error,
    },

    #[error("ffmpeg failed while {stage} {subject}: {message}")]
    Ffmpeg {
        stage: Stage,
        /// What was being decoded or encoded — a source file, or the output.
        subject: String,
        message: String,
    },
}
