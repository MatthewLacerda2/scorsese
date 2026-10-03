//! Synthesis: recipes in, files in `generated/`, asset state updated.
//!
//! The first real occupant of this crate, and the one that costs nothing. A
//! recipe is a brief exactly as a prompt is — the difference is that realising
//! it needs no key, no network and no money, and produces the same bytes every
//! time from one synthesiser.
//!
//! That last clause is not a hedge, and the cache is built around it. A render
//! has **two** inputs: the recipe, and the version of `scorsese-zimmer` that
//! reads it. So a bake is addressed by both — the `address` module is where
//! that name is made — and the path an asset holds *is* the record of what
//! produced it:
//!
//! - recipe and synthesiser hash to what `path` already names → nothing to do;
//! - either of them differs → the asset is stale whatever the document claims,
//!   and the next bake redoes it.
//!
//! Nothing has to remember to mark an asset stale, which matters because the
//! thing that edits a recipe is usually not the thing that bakes it — and
//! because the thing that changes a filter is nowhere near either of them.
//!
//! **No mock, and no test that spends money.** Every path here is local
//! arithmetic, so the no-real-provider-calls rule is satisfied by construction
//! rather than by a trait nobody can see through.

mod address;
mod create;
mod error;
pub mod kit;
mod midi;
mod partial;
mod recipe;
mod starter;
mod survey;
mod tune;

use std::path::{Path, PathBuf};

use scorsese_core::{
    Asset, AssetId, GenerationState, MediaMetadata, Project, ProjectPath, hash_bytes,
};
use scorsese_zimmer::level::{Cut, Layer, Profile};
use scorsese_zimmer::song::PatchRef;
use scorsese_zimmer::{Bake, Patch, SAMPLE_RATE, bake_excerpt_unless, bake_note, wav};

/// The vocabulary of an excerpt, re-exported.
///
/// A caller asking for less of a recipe says so in the synthesiser's own
/// types, and re-exporting them here is what keeps `cli` and `mcp` from taking
/// a dependency on `scorsese-zimmer` to name one — the same way [`Baked`]
/// hands out a `Profile` without either of them having heard of it.
pub use scorsese_zimmer::{Excerpt, Span, Window};

/// A track to export on the drum channel, re-exported for the same reason:
/// `cli` and `mcp` name it and parse it without depending on the synthesiser.
pub use scorsese_zimmer::midi::Drum;

pub use create::{check, create};
pub use error::SynthesisError;
pub use midi::{FromMidi, ToMidi, export_midi, import_midi};
pub use partial::{Partial, bake_partial, bake_partial_unless};
pub use recipe::{OneShot, Recipe};
pub use starter::Starter;
pub use survey::survey;
pub use tune::{Change, FIELDS, Setting, set};

/// What happened to one asset.
#[derive(Debug, Clone, PartialEq)]
pub enum Baked {
    /// Rendered and written. The only outcome that did any work.
    Rendered {
        /// Where the file landed, project-relative.
        path: ProjectPath,
        /// How big it is, for a report that says something happened.
        bytes: usize,
        /// How it came out: the whole file, and each section of the
        /// arrangement that made it.
        ///
        /// Only on this variant, and that is the honest shape rather than a
        /// gap: a cached bake was not measured on this run, and reporting
        /// numbers nobody just computed would be inventing them. Rebaking to
        /// learn them would undo the whole point of the cache.
        profile: Profile,
        /// How each track came out on its own, post-gain — which layer of the
        /// mix is taking up the room, rather than only how much room there is.
        ///
        /// Empty for a one-shot and for a song of a single track, where the row
        /// would repeat the summary above it.
        tracks: Vec<Layer>,
    },
    /// The file for this recipe, rendered by this synthesiser, was already
    /// there.
    Cached {
        /// Where it is, project-relative.
        path: ProjectPath,
        /// Where the arrangement's sections fall in it, for a song — worked
        /// out from the recipe, not measured, which is why a cache hit can
        /// still say them: they are arithmetic on the document and cost
        /// nothing, and a caller placing a caption on a section wants them
        /// whether or not this run rendered anything. Empty for a one-shot.
        sections: Vec<Cut>,
    },
}

impl Baked {
    /// Where the media is, either way.
    pub fn path(&self) -> &ProjectPath {
        match self {
            Self::Rendered { path, .. } | Self::Cached { path, .. } => path,
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
/// Up to date means *the file this recipe and this synthesiser hash to is on
/// disk*, which is a stronger question than the document's `state`: an asset
/// marked `generated` whose recipe has since been edited — or whose bake
/// predates a change to the synthesiser — is rebaked here, without anyone
/// having had to mark it stale first.
///
/// The project is left describing what is on disk. Saving it is the caller's.
pub fn bake_pending(
    project: &mut Project,
    project_root: &Path,
) -> Result<Vec<(AssetId, Baked)>, SynthesisError> {
    bake_pending_unless(project, project_root, &never)
}

/// [`bake_pending`] that gives up when `stop` says so — asked before each
/// recipe, and between the notes of a song — with [`SynthesisError::Stopped`].
///
/// What was baked before the stop stays baked: each of those files is
/// complete and named for its own brief, so the next bake finds it as a cache
/// hit. The one being rendered when the stop came leaves nothing behind (#661).
pub fn bake_pending_unless(
    project: &mut Project,
    project_root: &Path,
    stop: &dyn Fn() -> bool,
) -> Result<Vec<(AssetId, Baked)>, SynthesisError> {
    let ids: Vec<AssetId> = project
        .assets
        .iter()
        .filter(|asset| asset.kind.is_synthesized())
        .map(|asset| asset.id.clone())
        .collect();

    ids.into_iter()
        .map(|id| {
            if stop() {
                return Err(SynthesisError::Stopped);
            }
            bake_asset_unless(project, project_root, &id, stop).map(|baked| (id, baked))
        })
        .collect()
}

/// Realises one asset by id, whatever state it claims to be in.
pub fn bake_asset(
    project: &mut Project,
    project_root: &Path,
    id: &AssetId,
) -> Result<Baked, SynthesisError> {
    bake_asset_unless(project, project_root, id, &never)
}

/// [`bake_asset`] that gives up when `stop` says so, between the notes of a
/// song, with [`SynthesisError::Stopped`] — and writes nothing, since the file
/// is only ever written whole.
///
/// A one-shot is a single note and is not interrupted: it is over in the time
/// it would take to notice.
pub fn bake_asset_unless(
    project: &mut Project,
    project_root: &Path,
    id: &AssetId,
    stop: &dyn Fn() -> bool,
) -> Result<Baked, SynthesisError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| SynthesisError::NoSuchAsset { id: id.clone() })?;
    if !asset.kind.is_synthesized() {
        return Err(SynthesisError::NotSynthesised { id: id.clone() });
    }

    let (recipe, file, digest) = read_recipe(asset, project_root)?;
    let output = address::output(&digest, &named_patches(&recipe, project_root));
    let on_disk = output.resolve(project_root);

    let baked = if on_disk.is_file() {
        Baked::Cached {
            path: output,
            sections: sections(&recipe),
        }
    } else {
        let bake = render(&recipe, &file, project_root, stop)?;
        write(&on_disk, &bake.wav)?;
        Baked::Rendered {
            bytes: bake.wav.len(),
            path: output,
            profile: bake.profile,
            tracks: bake.tracks,
        }
    };
    record(project, id, &baked, project_root);
    Ok(baked)
}

/// Reads and parses an asset's recipe, returning it with the file it came from
/// and the digest of its bytes, which is half of what names its output.
pub(super) fn read_recipe(
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
    // Of the file's own bytes, not of the parsed document: two recipes that
    // mean the same thing but are written differently are still two recipes,
    // and pretending otherwise would make a cache hit depend on serde's
    // formatting rather than on what the author wrote. The synthesiser's own
    // version joins it in `address`.
    let digest = hash_bytes(&bytes);
    Ok((recipe, file, digest))
}

/// Renders a recipe to a complete WAV, or to nothing if `stop` says so first.
fn render(
    recipe: &Recipe,
    file: &Path,
    project_root: &Path,
    stop: &dyn Fn() -> bool,
) -> Result<Bake, SynthesisError> {
    let unrenderable = |source| SynthesisError::Unrenderable {
        path: file.to_path_buf(),
        source,
    };
    match recipe {
        Recipe::Patch(one_shot) => {
            let midi = one_shot.note.to_midi().map_err(unrenderable)?;
            bake_note(&one_shot.patch, midi, &one_shot.opts()).map_err(unrenderable)
        }
        Recipe::Song(song) => {
            bake_excerpt_unless(song, &instruments(project_root), &Excerpt::default(), stop)
                .map_err(unrenderable)?
                .ok_or(SynthesisError::Stopped)
        }
    }
}

/// The "should stop" of a bake nobody can stop.
fn never() -> bool {
    false
}

/// Where a recipe's sections fall, without rendering it: a song's
/// arrangement, and nothing for a one-shot, which has none.
fn sections(recipe: &Recipe) -> Vec<Cut> {
    match recipe {
        Recipe::Patch(_) => Vec::new(),
        Recipe::Song(song) => song.sections(),
    }
}

/// The resolver a song's named instruments go through.
///
/// This is the whole of what `scorsese-zimmer` is allowed to reach: a
/// project-relative path, checked by the same rules every other path in the
/// document obeys, read and parsed as a bare patch. The synthesiser never
/// learns that a project exists.
pub(super) fn instruments(project_root: &Path) -> impl Fn(&str) -> Result<Patch, String> {
    let root = project_root.to_path_buf();
    move |reference: &str| {
        let json = read_patch(&root, reference)?;
        Patch::from_json(&json).map_err(|error| error.to_string())
    }
}

/// The text of the patch file `reference` names, by the resolver's own rules:
/// a library instrument is refused, and a path is checked before it is opened.
///
/// Shared by [`instruments`] and [`named_patches`], so the file a song's
/// address hashes is always the file its render reads.
fn read_patch(root: &Path, reference: &str) -> Result<String, String> {
    if let Some(name) = reference.strip_prefix(kit::PREFIX) {
        return Err(format!(
            "`{reference}` is a library instrument, and a song carries its own copy \
             rather than naming one — write the recipe through synth_write, or run \
             `scorsese synth kit --copy-into <recipe>`, and it is copied in{}",
            kit::lookup(name)
                .map(|_| String::new())
                .unwrap_or_else(|| format!(" (the kit has {})", kit::names()))
        ));
    }
    let relative = ProjectPath::new(reference);
    relative
        .check()
        .map_err(|problem| format!("path {problem}"))?;
    std::fs::read_to_string(relative.resolve(root)).map_err(|error| error.to_string())
}

/// Every patch a song names by reference, in track order, with the digest of
/// the file behind it — the other half of a song's address (#672).
///
/// Empty for a one-shot and for a song whose patches are all inline, which is
/// what keeps their addresses where they always were.
fn named_patches(recipe: &Recipe, project_root: &Path) -> Vec<address::Named> {
    let Recipe::Song(song) = recipe else {
        return Vec::new();
    };
    song.tracks
        .iter()
        .filter_map(|track| match &track.patch {
            PatchRef::Named(reference) => Some(reference),
            PatchRef::Inline(_) => None,
        })
        .map(|reference| {
            let digest = read_patch(project_root, reference)
                .ok()
                .map(|json| hash_bytes(json.as_bytes()));
            (reference.clone(), digest)
        })
        .collect()
}

/// Writes the bake, creating `generated/` if this is the project's first one.
pub(super) fn write(path: &Path, wav: &[u8]) -> Result<(), SynthesisError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SynthesisError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    // Atomic, and here it is the cache that depends on it: a bake is named for
    // the hash of its brief, so a truncated file is indistinguishable from a
    // finished one and would be served as the bake for ever after — a stale
    // one at least gets redone the next time the synthesiser moves.
    //
    // `write` is this module's own function, so the shared one is named in
    // full rather than imported over it.
    scorsese_core::write::atomically(path, wav).map_err(|source| SynthesisError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Points the asset at what is now on disk, and says what is in it.
///
/// **What asserts this lives in `crates/cli`**, and that is deliberate rather
/// than an omission. Every line here is a fact written onto an asset from a
/// file that was just produced, so the only way to check one is to bake for
/// real and read the project back — which is what `cli/tests/synth/metadata.rs`
/// does, down to the duration, the channel count and the rate. Restating that
/// here would need a project, a recipe and a bake inside a unit test, to
/// arrive at a weaker version of a test that already exists.
///
/// It shows up in the mutation report as a module where nothing at all is
/// caught, because the signal runs each package's own tests and this one's
/// assertions are next door. That is the shape the report itself names, and
/// this is its written reason.
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
        audio_channels: Some(2),
        sample_rate: Some(SAMPLE_RATE),
        ..MediaMetadata::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What keeps every existing address where it was: nothing named, no
    /// line added.
    #[test]
    fn a_one_shot_or_an_inline_song_names_no_patch() {
        let root = Path::new("/nowhere");
        assert!(named_patches(&Starter::Patch.recipe(), root).is_empty());
        assert!(named_patches(&Starter::Song.recipe(), root).is_empty());
    }

    /// A named patch is hashed from the file the resolver would read.
    #[test]
    fn a_named_patch_is_hashed_from_its_file() {
        let root = std::env::temp_dir().join(format!("scorsese-named-{}", std::process::id()));
        std::fs::create_dir_all(root.join("recipes")).expect("recipes/");
        std::fs::write(root.join("recipes/bass.json"), b"{}").expect("the patch");

        let Recipe::Song(mut song) = Starter::Song.recipe() else {
            panic!("the song starter is a song");
        };
        song.tracks[0].patch = PatchRef::Named("recipes/bass.json".to_owned());
        let named = named_patches(&Recipe::Song(song), &root);
        std::fs::remove_dir_all(&root).ok();

        let expected = ("recipes/bass.json".to_owned(), Some(hash_bytes(b"{}")));
        assert_eq!(named, vec![expected]);
    }
}
