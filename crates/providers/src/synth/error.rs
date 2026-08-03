//! What can go wrong between a recipe on disk and a file in `generated/`.

use std::path::PathBuf;

use scorsese_core::{AssetId, PathProblem, ProjectPath};
use scorsese_zimmer::SynthError;

/// Why an asset could not be synthesised.
#[derive(Debug, thiserror::Error)]
pub enum SynthesisError {
    /// No asset by that id — usually a typo, since ids are what an agent
    /// types.
    #[error("no asset `{id}` in this project")]
    NoSuchAsset {
        /// The id that matched nothing.
        id: AssetId,
    },

    /// The asset exists but is not the kind that gets synthesised. Named
    /// rather than skipped, because asking to bake a Veo prompt is a
    /// misunderstanding worth correcting.
    #[error("asset `{id}` is not a synth_audio asset")]
    NotSynthesised {
        /// The asset that was asked for.
        id: AssetId,
    },

    /// A `synth_audio` asset with no `recipe`. Validation catches this on
    /// load, so reaching it here means the project was assembled in memory.
    #[error("asset `{id}` has no `recipe` to synthesise from")]
    NoRecipe {
        /// The asset with nothing to render.
        id: AssetId,
    },

    /// A recipe path that would not survive `scp -r`. Refused before it is
    /// opened, so a recipe can never read outside the project.
    #[error("asset `{id}`: recipe `{path}` {problem}")]
    BadRecipePath {
        /// The asset naming it.
        id: AssetId,
        /// The path as written.
        path: ProjectPath,
        /// Which rule it breaks.
        problem: PathProblem,
    },

    /// The recipe file could not be read — usually deleted, or never copied
    /// along with the project.
    #[error("reading recipe {}: {source}", path.display())]
    Read {
        /// The file that could not be read.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },

    /// The recipe is not a recipe: bad JSON, an unknown `recipe` kind, or a
    /// field the document does not have.
    #[error("recipe {}: {source}", path.display())]
    Malformed {
        /// The file that did not parse.
        path: PathBuf,
        /// Where and why it failed.
        #[source]
        source: serde_json::Error,
    },

    /// The recipe parsed but describes something unrenderable.
    #[error("recipe {}: {source}", path.display())]
    Unrenderable {
        /// The recipe at fault.
        path: PathBuf,
        /// What the synthesiser refused it for.
        #[source]
        source: SynthError,
    },

    /// The bake could not be written into `generated/`.
    #[error("writing {}: {source}", path.display())]
    Write {
        /// The file that could not be written.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
}
