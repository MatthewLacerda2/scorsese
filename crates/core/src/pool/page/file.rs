//! The files beside the pages: a script, a stylesheet, data or a drawing that
//! more than one page loads by relative path (`<script src="lib.js">`).
//!
//! **A file here is not an asset and is never placed.** Nothing in the
//! document names it; a page names it, and loading it makes it part of that
//! page's capture, so editing it re-draws every page that loaded it and no
//! other. The Lottie JSON `stock_import` keeps under `pages/` is one already.
//!
//! **Only supporting kinds, never `.html`** (#954): an html file no asset
//! points at is a page nothing plays, and a page is [`super::write_page`]'s.
//! **And at most [`MAX_PAGE_FILE_BYTES`]**, the size the web keeps a file
//! beside the pages to, held on every surface so a project written locally
//! still opens there.

use std::path::{Path, PathBuf};

use crate::path::ProjectPath;
use crate::project::PAGES_DIR;

/// The extensions a file beside the pages may have: script, style, data and
/// drawing, and never a page.
pub const PAGE_FILE_KINDS: [&str; 4] = ["js", "css", "json", "svg"];

/// The most one file beside the pages may hold, in bytes.
pub const MAX_PAGE_FILE_BYTES: usize = 1 << 20;

/// What [`write_page_file`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageFileWritten {
    /// Where the file is, relative to the project root: `pages/<name>`.
    pub path: ProjectPath,
    /// Whether there was no file there before.
    pub created: bool,
}

/// Why a file beside the pages could not be written or read. A refused write
/// changes nothing on disk.
#[derive(Debug, thiserror::Error)]
pub enum PageFileError {
    /// The name is not one file name directly under `pages/`.
    #[error(
        "`{name}` is not a file name under pages/: give one name like lib.js, \
         no folders and no leading dot"
    )]
    NotAName {
        /// The name, as given.
        name: String,
    },
    /// A page's own extension: pages are written by their asset id.
    #[error("`{name}` is a page; a page is written by its asset id, with `page`")]
    IsAPage {
        /// The name, as given.
        name: String,
    },
    /// An extension that is none of [`PAGE_FILE_KINDS`].
    #[error("`{name}` is not a file a page loads beside it: .js, .css, .json or .svg")]
    NotAKind {
        /// The name, as given.
        name: String,
    },
    /// Nothing was given. An empty file is never what was meant.
    #[error("{path} needs contents; these are empty")]
    Empty {
        /// Where it would have gone.
        path: ProjectPath,
    },
    /// Over [`MAX_PAGE_FILE_BYTES`].
    #[error(
        "{path} would be {bytes} bytes; a file beside the pages holds at most {MAX_PAGE_FILE_BYTES}"
    )]
    TooLarge {
        /// Where it would have gone.
        path: ProjectPath,
        /// How many bytes it was given.
        bytes: usize,
    },
    /// Read, and nothing is there.
    #[error("there is no {path}")]
    Missing {
        /// The file asked for.
        path: ProjectPath,
    },
    /// The operating system refused, writing or reading.
    #[error("cannot reach {}: {source}", path.display())]
    Io {
        /// The file or directory.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
}

/// Writes `text` whole as `pages/<name>`, making `pages/` if needed and
/// replacing whatever file was there. The document is not touched.
pub fn write_page_file(
    project_root: &Path,
    name: &str,
    text: &str,
) -> Result<PageFileWritten, PageFileError> {
    let path = page_file_path(name)?;
    if text.trim().is_empty() {
        return Err(PageFileError::Empty { path });
    }
    if text.len() > MAX_PAGE_FILE_BYTES {
        let bytes = text.len();
        return Err(PageFileError::TooLarge { path, bytes });
    }
    let pages = project_root.join(PAGES_DIR);
    std::fs::create_dir_all(&pages).map_err(|source| PageFileError::Io {
        path: pages,
        source,
    })?;
    let file = path.resolve(project_root);
    let created = !file.exists();
    crate::write::atomically(&file, text).map_err(|source| PageFileError::Io {
        path: file.clone(),
        source,
    })?;
    Ok(PageFileWritten { path, created })
}

/// The text of `pages/<name>`, exactly as it is on disk.
pub fn read_page_file(project_root: &Path, name: &str) -> Result<String, PageFileError> {
    let path = page_file_path(name)?;
    let file = path.resolve(project_root);
    match std::fs::read_to_string(&file) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(PageFileError::Missing { path })
        }
        Err(source) => Err(PageFileError::Io { path: file, source }),
    }
}

/// `pages/<name>`, or why `name` cannot be a file beside the pages.
pub fn page_file_path(name: &str) -> Result<ProjectPath, PageFileError> {
    let not_a_name = || PageFileError::NotAName {
        name: name.to_owned(),
    };
    let plain = !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\'])
        && !name.chars().any(char::is_control);
    if !plain {
        return Err(not_a_name());
    }
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if extension == "html" || extension == "htm" {
        return Err(PageFileError::IsAPage {
            name: name.to_owned(),
        });
    }
    if !PAGE_FILE_KINDS.contains(&extension.as_str()) {
        return Err(PageFileError::NotAKind {
            name: name.to_owned(),
        });
    }
    let path = ProjectPath::new(format!("{PAGES_DIR}/{name}"));
    path.check().map_err(|_| not_a_name())?;
    Ok(path)
}
