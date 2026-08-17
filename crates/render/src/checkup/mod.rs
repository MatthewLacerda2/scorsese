//! Everything wrong or questionable about a project, worked out once.
//!
//! This is the whole of the answer `scorsese check` prints and the whole of
//! the answer `project_check` returns, and it lives below both of them for the
//! reason that made it worth moving: two implementations of *what is wrong
//! with this project* drift, and the one that drifts is the one nobody runs by
//! hand. A caller here decides how to print the answer and what to do about
//! it; it never decides what the answer is.
//!
//! Six sources meet, and no other command can see all six at once — the pool's
//! health ([`scorsese_core::asset_status`]), the fonts a text asset names
//! ([`crate::unknown_fonts`], [`crate::uncovered_glyphs`]), the symbols an icon
//! asset names ([`crate::unknown_icons`]), the properties a keyframe track
//! animates ([`crate::unknown_in`]), what the picture draws on top of what
//! ([`crate::Layout`], filtered down to the stacks nobody meant) and the
//! document's own validation. That is why the assembly sits in this crate rather
//! than in `core`: a face is a file this crate opens, a symbol is a catalogue it
//! re-exports, and where a title lands is this crate's own matrix.
//!
//! **No ffmpeg.** Existence and hashing are questions about files, and a
//! checkup answers them without probing anything — so it stays cheap enough to
//! run after every edit, which is when it is worth running. That holds for the
//! overlap pass too: it sequences the timeline and reads rectangles off the
//! plan, and a clip whose source size nobody recorded is one it stays quiet
//! about rather than spawning a probe for.

mod media;
mod overlap;

use std::fmt;
use std::path::Path;

use scorsese_core::{AssetId, HashCheck, Project, ProjectPath, ValidationErrors, asset_status};

use crate::properties::unknown_in;
use crate::report::Note;
use crate::symbol::unknown_icons;
use crate::text::{uncovered_glyphs, unknown_fonts};

pub use media::{Finding, Severity, findings};

/// One thing a checkup has to say, already worded and naming its subject.
///
/// Worded here rather than by whoever prints it, so the CLI and the MCP server
/// cannot describe the same fault in two different sentences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Whether this stops a render or merely colours it.
    pub severity: Severity,
    /// What is wrong, subject first — `asset \`boat\`: its file is not there`.
    pub says: String,
}

impl Line {
    /// A finding that renders perfectly well and is probably not what anyone
    /// meant.
    fn warning(says: String) -> Self {
        Self {
            severity: Severity::Warning,
            says,
        }
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity, self.says)
    }
}

/// What the checkup comes to, once every finding has been read.
///
/// Two answers rather than a string and a boolean, because the caller has to
/// branch on it: the CLI's exit code is this distinction, and a client asking
/// whether it may render now is asking exactly this question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing blocks a render. There may still be warnings, and the wording
    /// says how many.
    Clear(String),
    /// Why a render is impossible — every reason at once, never just the
    /// first, since repairing one of them is not the job.
    Problems(String),
}

impl Verdict {
    /// The verdict as a line to print, whichever it is.
    pub fn says(&self) -> &str {
        match self {
            Self::Clear(words) | Self::Problems(words) => words,
        }
    }
}

/// Everything one pass over a project and its media found.
///
/// Two kinds of finding, and the difference is the point:
///
/// - **Problems** make a render impossible — a dangling asset reference, two
///   clips fighting over the same instant, a file a clip references that is
///   not on disk, a `style.font` naming a face this build does not ship.
/// - **Warnings** are things that render perfectly well and are probably not
///   what anyone meant: a keyframe track naming a property nobody animates, a
///   file whose content changed since it was imported, a script the document
///   names and the disk does not have, two layers of comparable size drawn
///   across each other for long enough that it was probably not a transition.
///
/// Both are collected even when there are problems: an agent repairing a
/// project unattended should see the whole list, not discover it one
/// round-trip at a time.
#[derive(Debug, Clone)]
pub struct Checkup {
    summary: String,
    lines: Vec<Line>,
    invalid: Option<ValidationErrors>,
}

impl Checkup {
    /// Looks at the document and at every file it points at.
    ///
    /// The project is parsed rather than loaded, because
    /// [`Project::load`](scorsese_core::Project::load) validates and returns
    /// nothing to warn about when it refuses — here the two are reported
    /// together.
    ///
    /// `hashes` decides whether the media pass re-reads every file.
    /// [`HashCheck::Skip`] is the answer to reach for by default: existence is
    /// cheap and always checked, hashing a whole pool is not, and everything
    /// hashing can find is a warning that never blocks a render.
    pub fn of(project: &Project, project_dir: &Path, hashes: HashCheck) -> Self {
        let mut lines: Vec<Line> = unknown_in(project)
            .into_iter()
            .map(|unknown| Line::warning(Note::from(unknown).to_string()))
            .collect();
        // A warning and never a problem, for the same reason a missing
        // generated file is one: a project that has lost its script still
        // renders, and the frames it produces are not one pixel different for
        // it. Worth saying loudly all the same — the script is the only part
        // of the edit that cannot be reconstructed by looking at the film.
        if let Some(path) = missing_script(project, project_dir) {
            lines.push(Line::warning(format!(
                "the project's script `{path}` is not there"
            )));
        }
        lines.extend(
            media(project, project_dir, hashes)
                .iter()
                .map(|finding| Line {
                    severity: finding.severity,
                    says: finding.to_string(),
                }),
        );
        // Last, and never a problem. Two layers over each other renders
        // perfectly — a title over a shot is the arrangement most films are
        // made of — so this can only ever be a note that something might not
        // have been meant, in the report the author is already reading.
        lines.extend(
            overlap::collisions(project, project_dir)
                .into_iter()
                .map(Line::warning),
        );

        Self {
            summary: format!(
                "{} — {} asset(s), {} track(s), {} clip(s)",
                project.name,
                project.assets.len(),
                project.tracks.len(),
                project.clips().count()
            ),
            lines,
            invalid: project.validate().err(),
        }
    }

    /// What the project is, in one line: its name and what it holds.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Every finding, in the order to read them: the document's own oddities
    /// first, then the media, asset by asset, then what the picture draws over
    /// what, in the order the timeline runs.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// How many findings render fine and are worth a second look.
    pub fn warnings(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.severity == Severity::Warning)
            .count()
    }

    /// Whether this project can be rendered, and why not when it cannot.
    ///
    /// Document problems and media problems land in one list under one
    /// heading: they are the same answer to the same question, and whoever
    /// reads it should not have to know which half of the checkup produced
    /// which line.
    pub fn verdict(&self) -> Verdict {
        let mut listed: Vec<String> = self
            .invalid
            .as_ref()
            .map(|errors| errors.as_slice().iter().map(ToString::to_string).collect())
            .unwrap_or_default();
        listed.extend(
            self.lines
                .iter()
                .filter(|line| line.severity == Severity::Problem)
                .map(|line| line.says.clone()),
        );
        if listed.is_empty() {
            return Verdict::Clear(match self.warnings() {
                0 => "no problems, nothing to warn about".to_owned(),
                warnings => format!("no problems, {warnings} warning(s)"),
            });
        }

        let plural = if listed.len() == 1 {
            "problem"
        } else {
            "problems"
        };
        let mut report = format!("{} {plural} in this project:", listed.len());
        for line in &listed {
            report.push_str("\n  - ");
            report.push_str(line);
        }
        Verdict::Problems(report)
    }
}

/// Everything the media has to say: the pool's health, the faces the document
/// names, and the characters those faces cannot draw.
///
/// Folded into one list because it is one kind of answer — a face this build
/// cannot find stops a render exactly as a missing file does.
fn media(project: &Project, project_dir: &Path, hashes: HashCheck) -> Vec<Finding> {
    let mut found = findings(&asset_status(project, project_dir, hashes));
    found.extend(unknown_fonts(project).into_iter().map(|unknown| Finding {
        asset: AssetId::new(unknown.asset.clone()),
        severity: Severity::Problem,
        detail: unknown.to_string(),
    }));
    // A symbol this build does not ship, for the same reason and with the same
    // severity: the render carries on, and the layer it carries on with is
    // empty. The catalogue is too large to list, so the finding names the near
    // matches instead — which is the part that makes it actionable.
    found.extend(unknown_icons(project).into_iter().map(|unknown| Finding {
        asset: AssetId::new(unknown.asset.clone()),
        severity: Severity::Problem,
        detail: unknown.to_string(),
    }));
    // A warning rather than a problem, and the split is the same one the rest
    // of a checkup makes: the render succeeds and every other frame is right.
    // What earns it a line at all is that the failure is otherwise invisible —
    // the character is dropped with its advance, so the text closes up and
    // reads as something nobody wrote.
    found.extend(
        uncovered_glyphs(project, project_dir)
            .into_iter()
            .map(|uncovered| Finding {
                asset: AssetId::new(uncovered.asset.clone()),
                severity: Severity::Warning,
                detail: uncovered.to_string(),
            }),
    );
    found
}

/// The script the document names, when there is no file where it says.
///
/// A path that breaks the project-path rules is validation's to report and not
/// this function's, so a badly-shaped one is left alone here rather than
/// producing a second complaint about the same field.
fn missing_script(project: &Project, project_dir: &Path) -> Option<ProjectPath> {
    let script = project.script.clone()?;
    script.check().ok()?;
    (!script.resolve(project_dir).is_file()).then_some(script)
}
