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
pub fn materialise(
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

    for (asset, recipe) in fixture.media() {
        generate(tools, asset, recipe, directory)?;
    }

    Project::load(directory).map_err(|source| SetupError::Load {
        source: Box::new(source),
    })
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
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("asset `{asset}` has no path to generate media at")]
    NoPath { asset: String },
    #[error("generating `{asset}` at {}: {message}", path.display())]
    Ffmpeg {
        asset: String,
        path: PathBuf,
        message: String,
    },
    #[error("the fixture's project does not load: {source}")]
    Load {
        #[source]
        source: Box<scorsese_core::LoadError>,
    },
}
