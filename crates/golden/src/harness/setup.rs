//! Building a real project directory for a fixture to render from.

use std::path::{Path, PathBuf};

use scorsese_core::{ASSETS_DIR, Asset, AssetKind, CACHE_DIR, GENERATED_DIR, Project};
use scorsese_render::Tools;

use crate::fixture::{Fixture, Recipe};

/// Materialises a fixture into `directory`: its `project.json` verbatim, its
/// media generated beside it, and the project loaded back off disk.
///
/// Loaded back rather than reused from memory on purpose — that path validates,
/// and it is the same one a person running `scorsese render` takes. A fixture
/// whose document does not survive a real load should fail here.
pub(super) fn materialise(
    tools: &Tools,
    fixture: &Fixture,
    directory: &Path,
) -> Result<Project, SetupError> {
    let _ = std::fs::remove_dir_all(directory);
    for sub in [ASSETS_DIR, GENERATED_DIR, CACHE_DIR] {
        let path = directory.join(sub);
        std::fs::create_dir_all(&path).map_err(|source| SetupError::Io { path, source })?;
    }

    let document = fixture.directory.join(scorsese_core::PROJECT_FILE_NAME);
    let target = directory.join(scorsese_core::PROJECT_FILE_NAME);
    std::fs::copy(&document, &target).map_err(|source| SetupError::Io {
        path: target,
        source,
    })?;

    carry(
        &fixture.directory.join(ASSETS_DIR),
        &directory.join(ASSETS_DIR),
    )?;

    for (asset, recipe) in fixture.media() {
        generate(tools, asset, recipe, directory)?;
    }

    Project::load(directory).map_err(|source| SetupError::Load {
        source: Box::new(source),
    })
}

/// Copies a fixture's own `assets/` into the scratch project, if it has one.
///
/// **Media is still generated, not committed** — that rule is why the
/// repository carries no sample footage and it has not moved. This is for the
/// files ffmpeg cannot conjure, and there is exactly one kind of them today: a
/// **font**. A face is not footage. It is a few glyph outlines, it is the same
/// bytes on every machine, and no `lavfi` source will ever produce one — so a
/// fixture that needs a project to bring its own font has no other way to say
/// so. Most fixtures have no `assets/` at all and this does nothing for them.
///
/// Recursive, so a fixture may organise what it carries into subdirectories the
/// same way a real project would.
fn carry(from: &Path, into: &Path) -> Result<(), SetupError> {
    if !from.is_dir() {
        return Ok(());
    }
    let io = |path: &Path, source: std::io::Error| SetupError::Io {
        path: path.to_path_buf(),
        source,
    };
    std::fs::create_dir_all(into).map_err(|source| io(into, source))?;
    for entry in std::fs::read_dir(from).map_err(|source| io(from, source))? {
        let entry = entry.map_err(|source| io(from, source))?;
        let target = into.join(entry.file_name());
        if entry.path().is_dir() {
            carry(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target).map_err(|source| io(&target, source))?;
        }
    }
    Ok(())
}

/// Runs ffmpeg to make one asset's media at the path `project.json` claims.
fn generate(
    tools: &Tools,
    asset: &Asset,
    recipe: &Recipe,
    directory: &Path,
) -> Result<(), SetupError> {
    let relative = asset.path.as_ref().ok_or_else(|| SetupError::NoPath {
        asset: asset.id.to_string(),
    })?;
    let file = relative.resolve(directory);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SetupError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let mut command = tools.ffmpeg();
    command.args(["-nostdin", "-v", "error", "-y"]);
    match recipe {
        Recipe::Lavfi(source) => {
            command.args(["-f", "lavfi", "-i", source]);
            // What the kind needs, so the shorthand stays one line: a still is
            // one frame, and video is 4:2:0 because that is what plays
            // everywhere and what the encoder would pick anyway.
            match asset.kind {
                AssetKind::Image => command.args(["-frames:v", "1"]),
                AssetKind::Video => command.args(["-pix_fmt", "yuv420p"]),
                _ => &mut command,
            };
        }
        Recipe::Arguments(arguments) => {
            command.args(arguments);
        }
    }

    let output = command
        .arg(&file)
        .output()
        .map_err(|source| SetupError::Io {
            path: file.clone(),
            source,
        })?;
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr);
    Err(SetupError::Ffmpeg {
        asset: asset.id.to_string(),
        path: file,
        message: message.trim().to_owned(),
    })
}

/// Why a fixture could not be set up. All faults in the fixture.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    /// The scratch project directory could not be built or copied into.
    #[error("{}: {source}", path.display())]
    Io {
        /// The directory or file being created or written.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },
    /// A recipe names an asset that has no `path`, so there is nowhere to put
    /// the media it describes.
    #[error("asset `{asset}` has no path to generate media at")]
    NoPath {
        /// The asset id with a recipe but no destination.
        asset: String,
    },
    /// ffmpeg refused the recipe — almost always a malformed lavfi source.
    #[error("generating `{asset}` at {}: {message}", path.display())]
    Ffmpeg {
        /// The asset whose media was being conjured.
        asset: String,
        /// Where the file was to be written.
        path: PathBuf,
        /// ffmpeg's own stderr, trimmed.
        message: String,
    },
    /// The document copied in does not survive a real [`Project::load`] — the
    /// same path `scorsese render` takes, so this would bite a user too.
    #[error("the fixture's project does not load: {source}")]
    Load {
        /// Boxed to keep this variant from dominating the enum's size.
        #[source]
        source: Box<scorsese_core::LoadError>,
    },
}
