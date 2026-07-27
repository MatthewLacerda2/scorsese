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

/// The manifest's file name, beside the `project.json` it describes.
pub const MANIFEST_FILE: &str = "fixture.json";

/// Where a fixture keeps its reference PNGs — the directory a reviewer looks
/// at when a re-blessing shows up in a diff.
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
    /// The render this fixture is pinned at. Pinned rather than defaulted,
    /// because a reference frame only means anything at one size and fps.
    pub render: RenderSpec,
    /// Which output frames are compared, ascending. Chosen to sit on the
    /// boundaries that matter — the last frame of a clip and the first frame of
    /// the next — because that is where an off-by-one hides.
    pub frames: Vec<u64>,
    /// Loosened or tightened only where a fixture has a reason to; the
    /// [`Tolerance`] defaults are what nearly every fixture should run at.
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
    /// `WIDTHxHEIGHT`. Small — a 64×64 reference is a few hundred bytes, and
    /// nothing a golden asserts gets easier to see at 1080p.
    pub resolution: String,
    /// Frames per second, whole or as a ratio like `30000/1001`.
    pub fps: String,
    /// A partial render, when that is what the fixture is testing.
    #[serde(default)]
    pub range: Option<String>,
}

/// A fixture, loaded and checked over.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// The fixture directory's own name, which is how it is listed in
    /// `tests/goldens.rs` and how it is named when it fails.
    pub name: String,
    /// The committed fixture directory. Read from, never rendered into — the
    /// render happens in a scratch copy so the repository stays clean.
    pub directory: PathBuf,
    /// The `fixture.json` beside the project, already parsed.
    pub manifest: Manifest,
    /// The fixture's `project.json`, parsed here to catch a broken document
    /// before ffmpeg is asked to do anything about it.
    pub project: Project,
    /// [`Manifest::render`] with its resolution and fps parsed.
    pub settings: RenderSettings,
    /// Which frames get rendered at all; [`FrameRange::ALL`] unless the fixture
    /// is testing a partial render. [`Manifest::frames`] index this output, not
    /// the timeline.
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
    /// One of the two files the fixture is made of is missing or unreadable.
    #[error("reading {}: {source}", path.display())]
    Io {
        /// The file that could not be read.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: std::io::Error,
    },
    /// The manifest is not the shape [`Manifest`] describes — usually a typo in
    /// a key, since unknown fields are rejected rather than ignored.
    #[error("fixture.json: {source}")]
    Manifest {
        /// Where in `fixture.json` parsing gave up.
        #[source]
        source: serde_json::Error,
    },
    /// The fixture's `project.json` does not parse or does not validate. The
    /// format is part of what is under test, so this is a real finding.
    #[error("project.json: {source}")]
    Project {
        /// Boxed because a [`LoadError`](scorsese_core::LoadError) is large
        /// next to the rest of these variants.
        #[source]
        source: Box<scorsese_core::LoadError>,
    },
    /// A render setting is written but unparseable.
    #[error("fixture.json render.{what}: {detail}")]
    Setting {
        /// Which setting: `resolution`, `fps`, or `range`.
        what: &'static str,
        /// The parse failure, in the words of the type that refused it.
        detail: String,
    },
    /// A fixture comparing nothing would pass forever while looking like
    /// coverage, which is the failure mode the whole gate exists to prevent.
    #[error("fixture.json lists no frames to compare, so it would assert nothing")]
    NoFrames,
    /// Order is required so the list reads as the boundaries it samples, and
    /// uniqueness so a frame is never quietly compared twice.
    #[error("fixture.json frames must be ascending and unique")]
    FramesOutOfOrder,
    /// A file-backed asset with no recipe would render as a missing file, and
    /// the fixture would be testing that instead of what it claims to.
    #[error("asset `{asset}` needs a media recipe in fixture.json")]
    NoRecipe {
        /// The asset id left without media.
        asset: String,
    },
    /// A recipe for nothing — usually an asset renamed on one side only.
    #[error("fixture.json has a recipe for `{asset}`, which is not in the assets table")]
    NoSuchAsset {
        /// The id the manifest names.
        asset: String,
    },
}
