//! What a golden fixture declares about itself.
//!
//! A fixture is a directory holding a real `project.json` — the format is part
//! of what is under test, so it is not a reduced stand-in — plus a
//! `fixture.json` saying how to conjure its media and how to render it, plus an
//! `expected/` directory of reference frames.
//!
//! Media is generated at test time from ffmpeg's synthetic sources rather than
//! committed, so the repository never carries sample footage.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use scorsese_core::{Asset, Fps, Project};
use scorsese_render::{FrameRange, RenderSettings, Resolution};

use crate::compare::Tolerance;

pub const MANIFEST_FILE: &str = "fixture.json";
pub const EXPECTED_DIR: &str = "expected";

/// The manifest beside a fixture's `project.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// What this fixture is for, in prose. Read by whoever has to decide
    /// whether a change to its references is legitimate, so it should say what
    /// would break if the fixture were wrong.
    pub description: String,
    /// How to make each asset's media, keyed by asset id. The file name comes
    /// from the asset's `path` in `project.json`, so there is one place a
    /// fixture's file names are written down.
    pub media: BTreeMap<String, Recipe>,
    pub render: RenderSpec,
    /// Which output frames are compared, ascending. Chosen to sit on the
    /// boundaries that matter — the last frame of a clip and the first frame of
    /// the next — because that is where an off-by-one hides.
    pub frames: Vec<u64>,
    #[serde(default)]
    pub tolerance: Tolerance,
}

/// How to generate one asset's media.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Recipe {
    /// A single lavfi source, e.g. `color=c=red:s=64x64:d=2:r=30`. The harness
    /// adds what the asset's kind needs — one frame for a still, 4:2:0 for
    /// video — so the common case stays one line.
    Lavfi(String),
    /// Raw ffmpeg arguments, for what one source cannot express. Nothing is
    /// added to these; they are complete as written.
    Arguments(Vec<String>),
}

/// The render settings a fixture is pinned at. Strings, then parsed, because
/// `64x64` and `30000/1001` are how a human writes them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderSpec {
    pub resolution: String,
    pub fps: String,
    /// A partial render, when that is what the fixture is testing.
    #[serde(default)]
    pub range: Option<String>,
}

/// A fixture, loaded and checked over.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub name: String,
    pub directory: PathBuf,
    pub manifest: Manifest,
    pub project: Project,
    pub settings: RenderSettings,
    pub range: FrameRange,
}

impl Fixture {
    /// Reads a fixture directory and checks it hangs together, before anything
    /// is rendered. A fixture that disagrees with itself is a broken test, not
    /// a failing one, and it should say so in those terms.
    pub fn load(directory: &Path) -> Result<Self, FixtureError> {
        let name = directory
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unnamed")
            .to_owned();
        let read = |file: &str| -> Result<String, FixtureError> {
            let path = directory.join(file);
            std::fs::read_to_string(&path).map_err(|source| FixtureError::Io { path, source })
        };

        let manifest: Manifest = serde_json::from_str(&read(MANIFEST_FILE)?)
            .map_err(|source| FixtureError::Manifest { source })?;
        let project =
            Project::from_json(&read(scorsese_core::PROJECT_FILE_NAME)?).map_err(|source| {
                FixtureError::Project {
                    source: Box::new(source),
                }
            })?;

        let settings = RenderSettings::new(
            manifest
                .render
                .resolution
                .parse::<Resolution>()
                .map_err(|source| FixtureError::Setting {
                    what: "resolution",
                    detail: source.to_string(),
                })?,
            manifest
                .render
                .fps
                .parse::<Fps>()
                .map_err(|source| FixtureError::Setting {
                    what: "fps",
                    detail: source.to_string(),
                })?,
        );
        let range = match &manifest.render.range {
            Some(text) => text
                .parse::<FrameRange>()
                .map_err(|source| FixtureError::Setting {
                    what: "range",
                    detail: source.to_string(),
                })?,
            None => FrameRange::ALL,
        };

        let fixture = Self {
            name,
            directory: directory.to_path_buf(),
            manifest,
            project,
            settings,
            range,
        };
        fixture.check_frames()?;
        fixture.check_recipes()?;
        Ok(fixture)
    }

    /// Where a frame's reference image lives.
    pub fn reference(&self, frame: u64) -> PathBuf {
        self.directory
            .join(EXPECTED_DIR)
            .join(format!("frame-{frame:04}.png"))
    }

    /// The assets needing a file generated for them, paired with their recipe.
    pub fn media(&self) -> Vec<(&Asset, &Recipe)> {
        self.project
            .assets
            .iter()
            .filter_map(|asset| {
                self.manifest
                    .media
                    .get(asset.id.as_str())
                    .map(|recipe| (asset, recipe))
            })
            .collect()
    }

    fn check_frames(&self) -> Result<(), FixtureError> {
        let frames = &self.manifest.frames;
        if frames.is_empty() {
            return Err(FixtureError::NoFrames);
        }
        if frames.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(FixtureError::FramesOutOfOrder);
        }
        Ok(())
    }

    /// Every file-backed asset needs a recipe, and every recipe needs an asset.
    /// Both directions, because either mismatch means the fixture is testing
    /// something other than what it looks like it is testing.
    fn check_recipes(&self) -> Result<(), FixtureError> {
        for asset in &self.project.assets {
            let needs_file = asset.kind.is_file_backed() && !asset.kind.is_generated();
            if needs_file && !self.manifest.media.contains_key(asset.id.as_str()) {
                return Err(FixtureError::NoRecipe {
                    asset: asset.id.to_string(),
                });
            }
        }
        for id in self.manifest.media.keys() {
            if !self.project.assets.iter().any(|a| a.id.as_str() == id) {
                return Err(FixtureError::NoSuchAsset { asset: id.clone() });
            }
        }
        Ok(())
    }
}

/// Why a fixture is not usable. Every one of these is a fault in the fixture
/// rather than in what it tests.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("reading {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("fixture.json: {source}")]
    Manifest {
        #[source]
        source: serde_json::Error,
    },
    #[error("project.json: {source}")]
    Project {
        #[source]
        source: Box<scorsese_core::LoadError>,
    },
    #[error("fixture.json render.{what}: {detail}")]
    Setting { what: &'static str, detail: String },
    #[error("fixture.json lists no frames to compare, so it would assert nothing")]
    NoFrames,
    #[error("fixture.json frames must be ascending and unique")]
    FramesOutOfOrder,
    #[error("asset `{asset}` needs a media recipe in fixture.json")]
    NoRecipe { asset: String },
    #[error("fixture.json has a recipe for `{asset}`, which is not in the assets table")]
    NoSuchAsset { asset: String },
}
