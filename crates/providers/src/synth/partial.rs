//! A bake of *less* than a recipe, and the one rule it exists to keep.
//!
//! **It never lands in `generated/`, and it never satisfies the cache.** That
//! is not a tidiness preference, it is the same rule `SYNTH_VERSION` exists to
//! enforce from the other side: a bake in `generated/` is addressed by a hash
//! of the recipe and the synthesiser, and a *fragment* stored under that name
//! is a project holding audio its own recipe does not describe — silently, and
//! for ever, because the address still looks fresh. So nothing here writes to
//! that directory, nothing here touches the asset's `path`, `sha256` or
//! `state`, and nothing here saves the project.
//!
//! What it writes instead is a file under `cache/` — rebuildable, gitignored,
//! and named for the asset rather than for a digest, so the next partial bake
//! of the same recipe **overwrites** it. That is deliberate: the loop this
//! serves is bake, read, move one number, bake again, and the alternative is
//! the pile of superseded ten-megabyte files that #453 was filed about. A
//! caller that wants the file kept names one.

use std::path::{Path, PathBuf};

use scorsese_core::{AssetId, CACHE_DIR, Project, ProjectPath};
use scorsese_zimmer::level::{Layer, Profile};
use scorsese_zimmer::{Excerpt, bake_excerpt};

use super::error::SynthesisError;
use super::recipe::Recipe;
use super::{instruments, read_recipe, write};

/// Where a partial bake lands when the caller does not say — under `cache/`,
/// which is the directory a project already treats as rebuildable.
const PARTIAL_DIR: &str = "synth";

/// What a partial bake produced. No `Cached` twin, because there is no cache:
/// a partial is rendered every time it is asked for, by construction.
#[derive(Debug, Clone, PartialEq)]
pub struct Partial {
    /// The file that was written.
    pub file: PathBuf,
    /// The same path as a report should say it: project-relative when it
    /// landed in the project's own `cache/`, and as the caller wrote it when
    /// the caller chose.
    pub shown: String,
    /// How big it is.
    pub bytes: usize,
    /// How the excerpt came out — over its whole length, and by section of the
    /// arrangement, whose rows are measured from where the window opens.
    pub profile: Profile,
    /// How each *heard* track came out. A track a solo left out has no row: it
    /// is not in this mix, and a silent row reads as an instrument that failed
    /// to play rather than one nobody asked for.
    pub tracks: Vec<Layer>,
}

/// Renders `excerpt` of one asset's recipe and writes it where `out` says, or
/// under `cache/` when it says nothing.
///
/// The project is read and never written. See the module doc for why that is
/// the whole point rather than an omission.
pub fn bake_partial(
    project: &Project,
    project_root: &Path,
    id: &AssetId,
    excerpt: &Excerpt,
    out: Option<&Path>,
) -> Result<Partial, SynthesisError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| SynthesisError::NoSuchAsset { id: id.clone() })?;
    if !asset.kind.is_synthesized() {
        return Err(SynthesisError::NotSynthesised { id: id.clone() });
    }

    let (recipe, file, _) = read_recipe(asset, project_root)?;
    // A one-shot is one gesture played by one voice: it has no arrangement to
    // window and no tracks to solo, so an excerpt of one is a question about a
    // document that cannot answer it. Refused rather than ignored, for the
    // reason every other unhonourable field here is refused.
    let Recipe::Song(song) = &recipe else {
        return Err(SynthesisError::NotASong { id: id.clone() });
    };
    let bake = bake_excerpt(song, &instruments(project_root), excerpt).map_err(|source| {
        SynthesisError::Unrenderable {
            path: file.clone(),
            source,
        }
    })?;

    let (destination, shown) = match out {
        Some(path) => (path.to_path_buf(), path.display().to_string()),
        None => {
            let relative = ProjectPath::new(format!("{CACHE_DIR}/{PARTIAL_DIR}/{id}.wav"));
            (relative.resolve(project_root), relative.to_string())
        }
    };
    write(&destination, &bake.wav)?;
    Ok(Partial {
        file: destination,
        shown,
        bytes: bake.wav.len(),
        profile: bake.profile,
        tracks: bake.tracks,
    })
}
