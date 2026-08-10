//! What a document was read from, so that saving it cannot quietly land on
//! somebody else's work.
//!
//! Every tool that edits a project does the same three things inside one call:
//! load the document, change it, save the whole thing back. Nothing in that
//! sequence notices the file moving underneath — and it moves for reasons that
//! need no misbehaviour at all. A `generate` call holds a document for the
//! minutes a provider takes; a second caller realising a different shot loads
//! before any of that landed and saves after it. Both writes are whole and
//! valid, and the later one takes the earlier one's work with it — including
//! the ticket that is the only record money was spent.
//!
//! That is a different failure from a half-written file, which [`crate::write`]
//! already closes. This one is a whole, valid document overwriting another.
//!
//! So a project remembers the fingerprint of the bytes it was read from, and
//! [`Project::save`](crate::Project::save) re-reads the file and compares
//! before publishing. A document that no longer matches is refused, with what
//! to do about it, rather than written.
//!
//! **This is not a transaction, and must not be read as a stronger promise
//! than it is.** A gap remains between the comparison and the rename, so two
//! saves landing in the same instant could both pass. What the guard buys is
//! the window shrinking from minutes to microseconds, and a silent loss
//! becoming a loud, retryable error — not serialisability. A lock would not do
//! better: the second writer's document was built from a stale read whatever
//! the *write* is serialised against, and making a lock correct means holding
//! it across read → think → write, which is a caller holding a mutex for a
//! minute and a permanently wedged project when one dies.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::pool::hash_bytes;
use crate::project::SaveError;

/// The fingerprint of some bytes: what a reader hands back so that a later
/// write can prove it is based on them.
///
/// The project's own content hash, reused rather than reinvented — one hashing
/// scheme in the crate is one thing to be right about, and a fingerprint an
/// agent can also compute from the file itself is one it can check.
#[must_use]
pub fn fingerprint_of(bytes: &[u8]) -> String {
    hash_bytes(bytes)
}

/// What a project was read from, if it was read from anywhere.
///
/// **Shared between clones, and moved on by a save.** A clone is the same
/// reader's copy of the same document — a window handing a background pass
/// something to work on, an editor producing one version after another from a
/// base — and whichever copy saves is speaking for all of them. Independent
/// baselines would refuse the second save of a sequence one writer made
/// deliberately, which is the guard firing on the case it exists to allow.
///
/// It takes no part in equality: it says where a document came from, not what
/// the document says, and two projects that read the same are the same project
/// however each of them got here.
#[derive(Clone, Default)]
pub struct Baseline(Arc<Mutex<Option<String>>>);

impl Baseline {
    /// A document that was not read from a file — [`Project::new`] and
    /// [`Project::from_json`], which parse a string that was never on disk.
    ///
    /// [`Project::new`]: crate::Project::new
    /// [`Project::from_json`]: crate::Project::from_json
    pub fn unread() -> Self {
        Self::default()
    }

    /// The baseline for bytes just read from the project file.
    pub fn of(bytes: &[u8]) -> Self {
        Self::claimed(fingerprint_of(bytes))
    }

    /// The baseline a caller vouches for: the fingerprint it was given when it
    /// read the document it is about to replace.
    ///
    /// Its own constructor because the claim is the caller's rather than this
    /// crate's — a document arriving over a protocol was read somewhere else,
    /// and this is how that read gets carried across the wire with it.
    pub fn claimed(fingerprint: impl Into<String>) -> Self {
        Self(Arc::new(Mutex::new(Some(fingerprint.into()))))
    }

    /// The fingerprint this document is based on, when it is based on one.
    #[must_use]
    pub fn fingerprint(&self) -> Option<String> {
        self.held().clone()
    }

    /// Whether the document at `file` is still the one this is based on.
    ///
    /// A file that is not there passes: there is nothing to lose, which is the
    /// case of a project being created and of one whose document was deleted
    /// out from under a live editor.
    pub(crate) fn guard(&self, file: &Path) -> Result<(), SaveError> {
        let found = match std::fs::read(file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(SaveError::Io {
                    path: file.to_path_buf(),
                    source,
                });
            }
        };
        let path: PathBuf = file.to_path_buf();
        match self.fingerprint() {
            None => Err(SaveError::Unread { path }),
            Some(mine) if mine == fingerprint_of(&found) => Ok(()),
            Some(_) => Err(SaveError::Stale { path }),
        }
    }

    /// Records that these bytes are now what is on disk.
    ///
    /// Called after a successful write, so that the second save of a session
    /// is measured against the first rather than against what was there before
    /// either of them.
    pub(crate) fn moved_to(&self, bytes: &[u8]) {
        *self.held() = Some(fingerprint_of(bytes));
    }

    /// The fingerprint, through a poisoned lock if it comes to it. What is
    /// behind the mutex is one `Option<String>` that is either replaced whole
    /// or not touched, so a panic elsewhere cannot have left it half-anything,
    /// and refusing to save over a poisoned lock would be a refusal with
    /// nothing behind it.
    fn held(&self) -> MutexGuard<'_, Option<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Deliberately always equal — see the type's own documentation for why a
/// document's provenance is not part of what the document says.
impl PartialEq for Baseline {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// The fingerprint rather than the lock around it: a `Project` printed in a
/// test failure should say what it is based on, not how that is stored.
impl fmt::Debug for Baseline {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.fingerprint() {
            Some(fingerprint) => write!(formatter, "Baseline({fingerprint})"),
            None => formatter.write_str("Baseline(unread)"),
        }
    }
}
