//! What the window has open, how it got there, and how it notices that it
//! changed underneath.

pub(crate) mod probing;
pub(crate) mod watch;

use std::path::{Path, PathBuf};

use scorsese_core::{LoadError, PROJECT_FILE_NAME, Project, ValidationErrors};

/// A project directory, loaded.
pub(crate) struct Open {
    /// Where it lives. Kept because every path in the document is relative to
    /// it, and because the title bar should say what you are editing.
    pub(crate) root: PathBuf,
    /// The document itself. The only model — there is no second copy of the
    /// timeline anywhere in this crate.
    pub(crate) project: Project,
    /// What validation said about it. Empty in the ordinary case.
    ///
    /// A document that parses but does not validate is **still opened**, with
    /// this filled in and the window read-only. An editor that refuses to open
    /// a file it wrote itself is a worse failure than whatever it is refusing
    /// over — and it wrote this one, because a background probe learns how long
    /// a source is and can turn a project that opened a minute ago into one
    /// that will not.
    pub(crate) problems: Vec<String>,
}

impl Open {
    /// The directory's own name, which is what a person calls this project
    /// however the document's `name` field reads.
    pub(crate) fn directory(&self) -> String {
        self.root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string())
    }

    /// Whether the window may change this document.
    ///
    /// Read-only **in full** rather than narrowed to the gestures that could
    /// repair it. Narrowing needs a second opinion about what is legal, kept in
    /// step with validation by hand, and that is the shape #99 already turned
    /// down once. Nothing here may write a project it could not read back.
    pub(crate) fn read_only(&self) -> bool {
        !self.problems.is_empty()
    }

    /// Re-asks validation about the document as it now stands.
    ///
    /// Opening is not the only moment a project can stop validating, and that
    /// is the whole of #170: a clip reaching past its media is not wrong until
    /// somebody measures the media, and the window measures it in the
    /// background of every open. So the answer is refreshed by whatever changed
    /// the document rather than held from the moment it was read.
    pub(crate) fn revalidate(&mut self) {
        self.problems = self.project.validate().err().map_or_else(Vec::new, listed);
    }
}

/// Why a directory could not be opened, in the words to put on the screen.
///
/// Flattened out of [`LoadError`] on purpose: a window shows a heading and a
/// list, and an error chain is neither. **Every** problem is listed rather than
/// the first, which is the same promise validation makes everywhere else — a
/// person fixing a project should see the whole job.
pub(crate) struct Refused {
    /// One line saying what kind of failure this is.
    pub(crate) heading: String,
    /// Every individual problem, in the order validation found them.
    pub(crate) problems: Vec<String>,
}

impl Refused {
    /// A document that parsed but does not validate, worded as a refusal.
    ///
    /// For the one caller that still wants the strict answer: a **re-read**.
    /// Opening an invalid project beats an empty window because there is
    /// nothing else to show; re-reading into one does not, because the last
    /// good version is right there on screen and something else is mid-write.
    pub(crate) fn invalid(problems: Vec<String>) -> Self {
        Self {
            heading: heading(problems.len()),
            problems,
        }
    }
}

/// Loads a project directory, or says why not.
///
/// Deliberately **not** [`Project::load`], which is the strict path and
/// discards the document it parsed the moment validation says no. The window
/// wants that document: it is the only thing that can show a person which clip
/// is the problem, and refusing to draw it is refusing to help with the one
/// thing they opened the file to do.
///
/// So the three failures that leave nothing to draw — no file, unreadable JSON,
/// another build's schema — are still refusals, and the fourth is not. A
/// document that parses opens whatever validation thinks of it, read-only, with
/// what validation thought of it on screen.
pub(crate) fn open(root: &Path) -> Result<Open, Refused> {
    let file = root.join(PROJECT_FILE_NAME);
    let json = std::fs::read_to_string(&file).map_err(|source| {
        refused(root, LoadError::Io {
            path: file.clone(),
            source,
        })
    })?;
    let project = Project::from_json(&json).map_err(|error| refused(root, error))?;
    let mut open = Open {
        root: root.to_path_buf(),
        project,
        problems: Vec::new(),
    };
    open.revalidate();
    Ok(open)
}

/// Turns a load failure into something worth reading.
fn refused(root: &Path, error: LoadError) -> Refused {
    match error {
        // The one that is usually not a fault at all — it is someone picking
        // the wrong folder — so it says what was expected rather than quoting
        // the operating system.
        LoadError::Io { .. } => Refused {
            heading: format!("{} is not a scorsese project", root.display()),
            problems: vec!["no project.json in this directory".to_owned()],
        },
        LoadError::SchemaVersion { found, supported } => Refused {
            heading: "this project was written by another build of scorsese".to_owned(),
            problems: vec![format!(
                "it says schema_version {found}, and this build reads {supported}"
            )],
        },
        LoadError::Parse(problem) => Refused {
            heading: "project.json could not be read".to_owned(),
            problems: vec![problem.to_string()],
        },
        // Unreachable from `open`, and kept because the variant exists rather
        // than because anything produces it here: a document that parses is
        // opened read-only with its problems on screen, which is the whole
        // point of `open` not being `Project::load`.
        LoadError::Invalid(errors) => Refused {
            heading: heading(errors.len()),
            problems: listed(errors),
        },
    }
}

/// How a count of problems reads as a line, in one place, so the panel that
/// says it about an open project and the refusal that says it about one that
/// could not be opened cannot word it two ways.
pub(crate) fn heading(problems: usize) -> String {
    match problems {
        1 => "1 problem in this project".to_owned(),
        many => format!("{many} problems in this project"),
    }
}

/// Every validation problem as its own line.
fn listed(errors: ValidationErrors) -> Vec<String> {
    errors
        .into_vec()
        .into_iter()
        .map(|problem| problem.to_string())
        .collect()
}
