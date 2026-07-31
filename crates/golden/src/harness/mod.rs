//! Rendering a fixture and holding it to its references.

pub mod decoder;
mod errors;
mod setup;

use std::fmt;
use std::path::{Path, PathBuf};

use scorsese_render::frames;
use scorsese_render::{Renderer, Tools};

use crate::compare::{self, Difference, Tolerance};
use crate::fixture::Fixture;

pub use decoder::Decoder;
pub use errors::GoldenError;
pub use setup::SetupError;

/// The name of the environment variable that re-blesses references.
pub const UPDATE_VARIABLE: &str = "UPDATE_GOLDENS";

/// Whether a run holds the render to the references or replaces them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Hold the render to what is committed. What CI runs, and the default —
    /// a missing reference fails here rather than being filled in silently.
    Check,
    /// Overwrite every compared frame's reference. Legitimate only when the
    /// render changed on purpose; `docs/golden-renders.md` says when.
    Bless,
}

impl Mode {
    /// `UPDATE_GOLDENS=1` blesses; unset, empty, or `0` checks.
    pub fn from_env() -> Self {
        match std::env::var(UPDATE_VARIABLE) {
            Ok(value) if !value.is_empty() && value != "0" => Self::Bless,
            _ => Self::Check,
        }
    }
}

/// What a passing run found.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The fixture that ran, by directory name.
    pub name: String,
    /// Frames the render wrote in total, not just the compared ones.
    pub frames_rendered: u64,
    /// Every compared frame and how close it was — reported even on success, so
    /// a fixture drifting towards its tolerance is visible before it trips.
    pub compared: Vec<(u64, Difference)>,
    /// Frames whose reference was rewritten. Empty unless blessing.
    pub blessed: Vec<u64>,
}

/// One frame that fell outside tolerance.
#[derive(Debug, Clone, PartialEq)]
pub struct Mismatch {
    /// Index into the rendered file, as the fixture nominated it.
    pub frame: u64,
    /// How far out it was, so the report can say by how much it missed.
    pub difference: Difference,
    /// Where the frame that was actually produced got written.
    pub actual: PathBuf,
    /// The committed reference it was held to.
    pub expected: PathBuf,
}

/// Everything wrong with one fixture's output, and where to look at it.
#[derive(Debug, Clone, PartialEq)]
pub struct Mismatches {
    /// The bar these frames were held to, carried along so the failure can
    /// print what was needed next to what was measured.
    pub tolerance: Tolerance,
    /// The render itself, kept on disk for inspection.
    pub render: PathBuf,
    /// Only the frames that fell outside tolerance, in fixture order.
    pub frames: Vec<Mismatch>,
    /// What decoded these frames, and what decoded the references — so a
    /// disagreement can name a different ffmpeg as its cause instead of
    /// reading as a regression. See [`decoder`].
    pub decoder: Decoder,
}

impl fmt::Display for Mismatches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in &self.frames {
            writeln!(
                f,
                "  frame {}: ssim {:.4} (need >= {:.4}), mean error {:.2} (need <= {:.2})",
                entry.frame,
                entry.difference.ssim,
                self.tolerance.min_ssim,
                entry.difference.mean_error,
                self.tolerance.max_mean_error
            )?;
            writeln!(f, "    expected {}", entry.expected.display())?;
            writeln!(f, "    actual   {}", entry.actual.display())?;
        }
        writeln!(f, "  the render itself is at {}", self.render.display())?;
        writeln!(f, "{}", self.decoder)?;
        write!(
            f,
            "  if this change is intended, re-bless with: \
             {UPDATE_VARIABLE}=1 cargo test -p scorsese-golden"
        )
    }
}

/// Renders one fixture and compares it against its references.
///
/// Works under `workspace`: renders into `<workspace>/renders/<name>` and drops
/// the frames that failed into `<workspace>/golden-failures/<name>`. A passing
/// render is cleaned up; a failing one is left exactly where it is, because the
/// file is the first thing anyone will want to watch.
pub fn run(fixture_dir: &Path, mode: Mode, workspace: &Path) -> Result<Outcome, GoldenError> {
    let fixture = Fixture::load(fixture_dir)?;
    let tools = Tools::discover()?;
    // Asked once per fixture, before anything is rendered: every frame below
    // arrives through this ffmpeg, so it is the one fact a failure needs to be
    // able to name. It never decides anything on its own — see `decoder`.
    let decoder = Decoder::read(&fixture.expected(), tools.ffmpeg_version()?);
    let work = workspace.join("renders").join(&fixture.name);
    let project = setup::materialise(&tools, &fixture, &work)?;

    let output = work.join("out.mp4");
    let report =
        Renderer::new(&tools, fixture.settings).render(&project, &work, fixture.range, &output)?;

    let mut compared = Vec::new();
    let mut blessed = Vec::new();
    let mut failed = Vec::new();
    let failures = workspace.join("golden-failures").join(&fixture.name);

    for &frame in &fixture.manifest.frames {
        if frame >= report.frames {
            return Err(GoldenError::FrameBeyondRender {
                fixture: fixture.name.clone(),
                frame,
                frames: report.frames,
            });
        }
        let actual = frames::extract(&tools, &output, frame, fixture.settings.resolution)?;
        let reference = fixture.reference(frame);

        if mode == Mode::Bless {
            frames::write_png(&tools, &reference, &actual)?;
            blessed.push(frame);
            continue;
        }
        // A missing reference must fail rather than be created: a fixture with
        // nothing to compare against would otherwise pass by asserting nothing.
        if !reference.is_file() {
            return Err(GoldenError::NoReference {
                fixture: fixture.name.clone(),
                frame,
                path: reference,
            });
        }

        let expected = frames::read_png(&tools, &reference, fixture.settings.resolution)?;
        let difference = compare::compare(&actual, &expected)?;
        compared.push((frame, difference));
        if !fixture.manifest.tolerance.accepts(&difference) {
            let dumped = failures.join(format!("frame-{frame:04}-actual.png"));
            frames::write_png(&tools, &dumped, &actual)?;
            failed.push(Mismatch {
                frame,
                difference,
                actual: dumped,
                expected: reference,
            });
        }
    }

    // Written after the loop rather than inside it: one record covers the
    // fixture's references, and it is rewritten in the same act that rewrites
    // them so it can never describe an older set of frames than they are.
    if mode == Mode::Bless {
        decoder
            .write(&fixture.expected())
            .map_err(|source| GoldenError::Record {
                fixture: fixture.name.clone(),
                source,
            })?;
    }

    if !failed.is_empty() {
        return Err(GoldenError::Mismatch {
            fixture: fixture.name,
            mismatches: Box::new(Mismatches {
                tolerance: fixture.manifest.tolerance,
                render: output,
                frames: failed,
                decoder,
            }),
        });
    }

    let _ = std::fs::remove_dir_all(&work);
    Ok(Outcome {
        name: fixture.name,
        frames_rendered: report.frames,
        compared,
        blessed,
    })
}

/// [`run`], panicking with the full diagnosis. For calling from a `#[test]`.
#[track_caller]
pub fn assert_matches(fixture_dir: &Path, mode: Mode, workspace: &Path) -> Outcome {
    match run(fixture_dir, mode, workspace) {
        Ok(outcome) => outcome,
        Err(error) => panic!("{error}"),
    }
}
