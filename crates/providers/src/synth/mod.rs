//! Synthesis: recipes in, files in `generated/`, asset state updated.
//!
//! The first real occupant of this crate, and the one that costs nothing. A
//! recipe is a brief exactly as a prompt is — the difference is that realising
//! it needs no key, no network and no money, and produces the same bytes every
//! time.
//!
//! That last property is what the cache rests on. A bake is written to
//! `generated/<sha256 of the recipe's bytes>.wav`, so the path an asset holds
//! *is* the record of which recipe produced it:
//!
//! - the recipe hashes to what `path` already names → nothing to do;
//! - it hashes to something else → the recipe was edited, and the asset is
//!   stale whatever the document claims.
//!
//! Nothing has to remember to mark an asset stale, which matters because the
//! thing that edits a recipe is usually not the thing that bakes it.
//!
//! **No mock, and no test that spends money.** Every path here is local
//! arithmetic, so the no-real-provider-calls rule is satisfied by construction
//! rather than by a trait nobody can see through.

pub mod create;
pub mod error;
pub mod recipe;
pub mod starter;

use std::path::{Path, PathBuf};

use scorsese_core::{
    Asset, AssetId, GENERATED_DIR, GenerationState, MediaMetadata, Project, ProjectPath, hash_bytes,
};
use scorsese_soundgen::{Patch, SAMPLE_RATE, bake_note, bake_song, wav};

pub use create::{check, create};
pub use error::SynthesisError;
pub use recipe::{OneShot, Recipe};
pub use starter::Starter;

/// What happened to one asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Baked {
    /// Rendered and written. The only outcome that did any work.
    Rendered {
        /// Where the file landed, project-relative.
        path: ProjectPath,
        /// How big it is, for a report that says something happened.
        bytes: usize,
    },
    /// The file for this exact recipe was already there.
    Cached {
        /// Where it is, project-relative.
        path: ProjectPath,
    },
}

impl Baked {
    /// Where the media is, either way.
    pub fn path(&self) -> &ProjectPath {
        match self {
            Self::Rendered { path, .. } | Self::Cached { path } => path,
        }
    }

    /// True when this bake actually rendered something.
    pub fn is_fresh(&self) -> bool {
        matches!(self, Self::Rendered { .. })
    }
}

/// Realises every `synth_audio` asset that is not already up to date, and
/// records the result on the project.
///
/// Up to date means *the file this recipe hashes to is on disk*, which is a
/// stronger question than the document's `state`: an asset marked `generated`
/// whose recipe has since been edited is rebaked here, without anyone having
/// had to mark it stale first.
///
/// The project is left describing what is on disk. Saving it is the caller's.
pub fn bake_pending(
    project: &mut Project,
    project_root: &Path,
) -> Result<Vec<(AssetId, Baked)>, SynthesisError> {
    let ids: Vec<AssetId> = project
        .assets
        .iter()
        .filter(|asset| asset.kind.is_synthesized())
        .map(|asset| asset.id.clone())
        .collect();

    ids.into_iter()
        .map(|id| bake_asset(project, project_root, &id).map(|baked| (id, baked)))
        .collect()
}

/// Realises one asset by id, whatever state it claims to be in.
pub fn bake_asset(
    project: &mut Project,
    project_root: &Path,
    id: &AssetId,
) -> Result<Baked, SynthesisError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| SynthesisError::NoSuchAsset { id: id.clone() })?;
    if !asset.kind.is_synthesized() {
        return Err(SynthesisError::NotSynthesised { id: id.clone() });
    }

    let (recipe, file, digest) = read_recipe(asset, project_root)?;
    let output = ProjectPath::new(format!("{GENERATED_DIR}/{digest}.wav"));
    let on_disk = output.resolve(project_root);

    let baked = if on_disk.is_file() {
        Baked::Cached { path: output }
    } else {
        let wav = render(&recipe, &file, project_root)?;
        write(&on_disk, &wav)?;
        Baked::Rendered {
            bytes: wav.len(),
            path: output,
        }
    };
    record(project, id, &baked, project_root);
    Ok(baked)
}

/// Reads and parses an asset's recipe, returning it with the file it came from
/// and the digest that names its output.
fn read_recipe(
    asset: &Asset,
    project_root: &Path,
) -> Result<(Recipe, PathBuf, String), SynthesisError> {
    let relative = asset
        .recipe
        .as_ref()
        .ok_or_else(|| SynthesisError::NoRecipe {
            id: asset.id.clone(),
        })?;
    // Checked before it is opened, so a recipe can never reach outside the
    // project — the same rule the render path holds media to.
    relative
        .check()
        .map_err(|problem| SynthesisError::BadRecipePath {
            id: asset.id.clone(),
            path: relative.clone(),
            problem,
        })?;

    let file = relative.resolve(project_root);
    let bytes = std::fs::read(&file).map_err(|source| SynthesisError::Read {
        path: file.clone(),
        source,
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let recipe = Recipe::from_json(&text).map_err(|source| SynthesisError::Malformed {
        path: file.clone(),
        source,
    })?;
    // The digest is of the file's own bytes, not of the parsed document: two
    // recipes that mean the same thing but are written differently are still
    // two recipes, and pretending otherwise would make a cache hit depend on
    // serde's formatting rather than on what the author wrote.
    let digest = hash_bytes(&bytes);
    Ok((recipe, file, digest))
}

/// Renders a recipe to a complete WAV.
fn render(recipe: &Recipe, file: &Path, project_root: &Path) -> Result<Vec<u8>, SynthesisError> {
    let unrenderable = |source| SynthesisError::Unrenderable {
        path: file.to_path_buf(),
        source,
    };
    match recipe {
        Recipe::Patch(one_shot) => {
            let midi = one_shot.note.to_midi().map_err(unrenderable)?;
            bake_note(&one_shot.patch, midi, &one_shot.opts()).map_err(unrenderable)
        }
        Recipe::Song(song) => bake_song(song, &instruments(project_root)).map_err(unrenderable),
    }
}

/// The resolver a song's named instruments go through.
///
/// This is the whole of what `scorsese-soundgen` is allowed to reach: a
/// project-relative path, checked by the same rules every other path in the
/// document obeys, read and parsed as a bare patch. The synthesiser never
/// learns that a project exists.
fn instruments(project_root: &Path) -> impl Fn(&str) -> Result<Patch, String> {
    let root = project_root.to_path_buf();
    move |reference: &str| {
        let relative = ProjectPath::new(reference);
        relative
            .check()
            .map_err(|problem| format!("path {problem}"))?;
        let json =
            std::fs::read_to_string(relative.resolve(&root)).map_err(|error| error.to_string())?;
        Patch::from_json(&json).map_err(|error| error.to_string())
    }
}

/// Writes the bake, creating `generated/` if this is the project's first one.
fn write(path: &Path, wav: &[u8]) -> Result<(), SynthesisError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SynthesisError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    // Atomic, and here it is the cache that depends on it: a bake is named for
    // the hash of its brief, so a truncated file is indistinguishable from a
    // finished one and would be served as the bake for ever after.
    //
    // `write` is this module's own function, so the shared one is named in
    // full rather than imported over it.
    scorsese_core::write::atomically(path, wav).map_err(|source| SynthesisError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Points the asset at what is now on disk, and says what is in it.
fn record(project: &mut Project, id: &AssetId, baked: &Baked, project_root: &Path) {
    let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) else {
        return;
    };
    asset.path = Some(baked.path().clone());
    asset.state = Some(GenerationState::Generated);

    let Ok(wav) = std::fs::read(baked.path().resolve(project_root)) else {
        return;
    };
    // Of the finished file, which is what `sha256` means everywhere else — the
    // recipe's own digest is already in the file name.
    asset.sha256 = Some(hash_bytes(&wav));
    // Filled in here rather than left for a probe, because we know it: nothing
    // about a bake needs ffprobe to discover, and an asset that cannot say how
    // long it is makes an agent guess the length of the clip it just made.
    asset.media = Some(MediaMetadata {
        duration_seconds: Some(wav::seconds_in(wav.len())),
        audio_channels: Some(1),
        sample_rate: Some(SAMPLE_RATE),
        ..MediaMetadata::default()
    });
}
