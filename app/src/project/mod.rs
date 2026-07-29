//! What the window has open, how it got there, and how it notices that it
//! changed underneath.

pub(crate) mod watch;

use std::path::{Path, PathBuf};

use scorsese_core::{LoadError, Project, ValidationErrors};

/// A project directory, loaded.
pub(crate) struct Open {
    /// Where it lives. Kept because every path in the document is relative to
    /// it, and because the title bar should say what you are editing.
    pub(crate) root: PathBuf,
    /// The document itself. The only model — there is no second copy of the
    /// timeline anywhere in this crate.
    pub(crate) project: Project,
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

/// Loads a project directory, or says why not.
pub(crate) fn open(root: &Path) -> Result<Open, Refused> {
    match Project::load(root) {
        Ok(project) => Ok(Open {
            root: root.to_path_buf(),
            project,
        }),
        Err(error) => Err(refused(root, error)),
    }
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
        LoadError::Invalid(errors) => Refused {
            heading: match errors.len() {
                1 => "1 problem in this project".to_owned(),
                many => format!("{many} problems in this project"),
            },
            problems: listed(errors),
        },
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
