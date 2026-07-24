//! The command-line surface.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use scorsese_core::{AssetKind, Fps};

#[derive(Debug, Parser)]
#[command(
    name = "scorsese",
    version,
    about = "A video editor for agentic workflows"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// The project directory to work in. Defaults to the current directory.
    /// Global, so it can follow any subcommand: `scorsese assets gc
    /// --project teaser.scor`.
    #[arg(long, global = true)]
    pub project: Option<PathBuf>,
}

impl Cli {
    pub fn project_dir(&self) -> PathBuf {
        self.project.clone().unwrap_or_else(|| PathBuf::from("."))
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new project directory.
    New {
        /// Where to create it, e.g. `teaser.scor`.
        directory: PathBuf,
        /// Project name. Defaults to the directory's name.
        #[arg(long)]
        name: Option<String>,
        /// The timeline framerate every clip and keyframe time is counted
        /// in: `30`, or a rational like `30000/1001` for 29.97. Chosen once,
        /// here — changing it later is a real operation, not a field edit.
        #[arg(long, default_value = "30")]
        fps: Fps,
    },
    /// Copy a media file into the project and add it to the assets table.
    Import {
        /// The file to import. It is copied, never referenced in place.
        file: PathBuf,
        /// Override the kind instead of inferring it from the extension.
        #[arg(long, value_enum)]
        kind: Option<KindArg>,
    },
    /// List the media pool and the state of everything in it.
    Assets {
        #[command(subcommand)]
        action: Option<AssetsAction>,
        /// Re-hash every file to detect media that changed since import.
        #[arg(long)]
        verify: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum AssetsAction {
    /// Report assets no clip references, and optionally delete them.
    Gc {
        /// Actually remove them. Without this, nothing is deleted.
        #[arg(long)]
        delete: bool,
    },
}

/// The kinds a file can be imported as. The prompt-backed kinds are absent on
/// purpose: they are authored, not imported.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KindArg {
    Video,
    Image,
    Audio,
}

impl From<KindArg> for AssetKind {
    fn from(arg: KindArg) -> Self {
        match arg {
            KindArg::Video => Self::Video,
            KindArg::Image => Self::Image,
            KindArg::Audio => Self::Audio,
        }
    }
}
