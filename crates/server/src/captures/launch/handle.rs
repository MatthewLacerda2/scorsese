//! Folders opened by handle, never by a path that is looked up again (#1636).
//!
//! Everything the launcher copies out of the spool, and everything it writes
//! back into a page cache, is under a folder the server can change at any
//! moment. A path checked and then opened again can lead somewhere else by
//! the second look, so here a path is never looked at twice: a [`Folder`] is
//! an open directory, each step below it is opened from that handle with
//! `O_NOFOLLOW`, and what an opened entry is gets asked of the handle itself.
//! A link anywhere on the way is refused (`ELOOP`, or `ENOTDIR` where a
//! folder belongs), and a FIFO is never waited on.
//!
//! The standard library opens from a path only, so a step from a handle goes
//! through `/proc/self/fd/<n>/<name>`: the kernel resolves the first part to
//! the very directory that is open, and looks `<name>` up inside it, the last
//! component unfollowed. The workspace forbids the `unsafe` an `openat(2)`
//! would take; this is the same lookup.

use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

/// An open directory, each entry below it reached from the handle.
#[derive(Debug)]
pub(super) struct Folder(File);

/// What an entry of a [`Folder`] is, not following it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// A directory.
    Folder,
    /// A regular file.
    File,
    /// A symbolic link.
    Link,
    /// A FIFO, a socket or a device.
    Other,
}

/// Flags every open here takes: never follow the last component, never hang
/// on a FIFO, never leak into a child.
const NO_FOLLOW: i32 = libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC;

impl Folder {
    /// The folder at `path`, a path the operator gave: its own links are
    /// followed, as the operator meant them.
    pub(super) fn at(path: &Path) -> io::Result<Self> {
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(path)
            .map(Self)
    }

    /// The folder `relative` below this one, each step opened from the last
    /// and none of them a link. Only plain names: no `..`, no root.
    pub(super) fn below(&self, relative: &Path) -> io::Result<Self> {
        let mut folder = self.reopen()?;
        for part in relative.components() {
            let Component::Normal(name) = part else {
                return Err(refused(relative, "is not a plain path"));
            };
            folder = folder.folder(name)?;
        }
        Ok(folder)
    }

    /// The folder `name` in this one, not a link.
    pub(super) fn folder(&self, name: &OsStr) -> io::Result<Self> {
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | NO_FOLLOW)
            .open(self.entry(name)?)
            .map(Self)
    }

    /// The folder `name` in this one, made first if it is not there.
    pub(super) fn folder_made(&self, name: &OsStr) -> io::Result<Self> {
        match std::fs::create_dir(self.entry(name)?) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
            _ => {}
        }
        self.folder(name)
    }

    /// The regular file `name` in this one, open for reading: never a link,
    /// and never anything but a file.
    pub(super) fn file(&self, name: &OsStr) -> io::Result<File> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(NO_FOLLOW)
            .open(self.entry(name)?)?;
        if !file.metadata()?.is_file() {
            return Err(refused(Path::new(name), "is not a file"));
        }
        Ok(file)
    }

    /// Where the link `name` in this one points, as written.
    pub(super) fn link(&self, name: &OsStr) -> io::Result<PathBuf> {
        std::fs::read_link(self.entry(name)?)
    }

    /// Every entry in this folder and what it is.
    pub(super) fn entries(&self) -> io::Result<Vec<(OsString, Kind)>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(self.path())? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let kind = if kind.is_symlink() {
                Kind::Link
            } else if kind.is_dir() {
                Kind::Folder
            } else if kind.is_file() {
                Kind::File
            } else {
                Kind::Other
            };
            entries.push((entry.file_name(), kind));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }

    /// Writes `from` whole as the file `name` in this folder: into a file of
    /// its own made beside it, then renamed over whatever had the name. A
    /// link already there is replaced, never written through.
    pub(super) fn place(&self, name: &OsStr, from: &mut File) -> io::Result<()> {
        let mut scratch = name.to_os_string();
        scratch.push(format!(".{}.placing", std::process::id()));
        let scratch = self.entry(&scratch)?;
        let _ = std::fs::remove_file(&scratch);
        let written = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(NO_FOLLOW)
            .open(&scratch)
            .and_then(|mut to| io::copy(from, &mut to).map(drop))
            .and_then(|()| std::fs::rename(&scratch, self.entry(name)?));
        if written.is_err() {
            let _ = std::fs::remove_file(&scratch);
        }
        written
    }

    /// Whether the file `name` in this folder, not a link, holds exactly what
    /// `from` does. `from` is read from its start, and left at its start.
    pub(super) fn holds_same(&self, name: &OsStr, from: &mut File) -> io::Result<bool> {
        let Ok(mut there) = self.file(name) else {
            return Ok(false);
        };
        if there.metadata()?.len() != from.metadata()?.len() {
            return Ok(false);
        }
        from.rewind()?;
        let same = same_bytes(from, &mut there);
        from.rewind()?;
        same
    }

    /// The same folder, opened again: a handle of its own.
    fn reopen(&self) -> io::Result<Self> {
        Self::at(&self.path())
    }

    /// The path that reaches this very folder, whatever is at its old name.
    fn path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.0.as_raw_fd()))
    }

    /// The path of `name` in this very folder: one plain name only.
    fn entry(&self, name: &OsStr) -> io::Result<PathBuf> {
        let mut parts = Path::new(name).components();
        match (parts.next(), parts.next()) {
            (Some(Component::Normal(_)), None) => Ok(self.path().join(name)),
            _ => Err(refused(Path::new(name), "is not one plain name")),
        }
    }
}

/// Whether `one` and `other` read the same to their ends.
fn same_bytes(one: &mut File, other: &mut File) -> io::Result<bool> {
    let (mut a, mut b) = (vec![0; 1 << 16], vec![0; 1 << 16]);
    loop {
        let read = one.read(&mut a)?;
        if read == 0 {
            return Ok(other.read(&mut b[..1])? == 0);
        }
        if other.read_exact(&mut b[..read]).is_err() || a[..read] != b[..read] {
            return Ok(false);
        }
    }
}

fn refused(path: &Path, why: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{} {why}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::os::unix::fs::symlink;

    use super::*;

    fn root(label: &str) -> PathBuf {
        let root = crate::scratch::fresh(&format!("scorsese-handle-{label}"), "");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("in/deeper")).unwrap();
        std::fs::write(root.join("in/deeper/file"), "mine").unwrap();
        std::fs::create_dir_all(root.join("elsewhere")).unwrap();
        std::fs::write(root.join("elsewhere/file"), "theirs").unwrap();
        root
    }

    #[test]
    fn a_link_anywhere_on_the_way_is_refused() {
        let root = root("links");
        symlink(root.join("elsewhere"), root.join("in/linked")).unwrap();
        symlink(root.join("elsewhere/file"), root.join("in/deeper/linked")).unwrap();
        let top = Folder::at(&root).unwrap();

        let through = top.below(Path::new("in/linked"));
        let at_the_end = top.below(Path::new("in/deeper")).unwrap();

        // A link where a folder belongs is not a folder (`ENOTDIR`).
        assert_eq!(through.unwrap_err().raw_os_error(), Some(libc::ENOTDIR));
        assert_eq!(
            at_the_end
                .file(OsStr::new("linked"))
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ELOOP)
        );
        assert!(top.below(Path::new("in/../elsewhere")).is_err());
    }

    #[test]
    fn a_file_is_not_a_folder() {
        let root = root("not-a-folder");
        assert!(Folder::at(&root.join("in/deeper/file")).is_err());
    }

    #[test]
    fn an_opened_folder_stays_the_one_opened_when_its_name_is_swapped() {
        let root = root("swapped");
        let deeper = Folder::at(&root)
            .unwrap()
            .below(Path::new("in/deeper"))
            .unwrap();
        std::fs::rename(root.join("in/deeper"), root.join("in/moved")).unwrap();
        symlink(root.join("elsewhere"), root.join("in/deeper")).unwrap();

        let mut text = String::new();
        deeper
            .file(OsStr::new("file"))
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();

        assert_eq!(text, "mine");
    }

    #[test]
    fn a_fifo_is_said_and_never_waited_on() {
        let root = root("fifo");
        let made = scorsese_render::tools::child("mkfifo")
            .arg(root.join("in/pipe"))
            .status()
            .unwrap();
        assert!(made.success());
        let folder = Folder::at(&root.join("in")).unwrap();

        assert!(
            folder
                .entries()
                .unwrap()
                .contains(&("pipe".into(), Kind::Other))
        );
        assert!(folder.file(OsStr::new("pipe")).is_err());
    }

    #[test]
    fn placing_replaces_a_link_and_never_writes_through_it() {
        let root = root("place");
        symlink(root.join("elsewhere/file"), root.join("in/target")).unwrap();
        let folder = Folder::at(&root.join("in")).unwrap();
        let mut from = File::open(root.join("in/deeper/file")).unwrap();

        folder.place(OsStr::new("target"), &mut from).unwrap();

        assert_eq!(
            std::fs::read_to_string(root.join("elsewhere/file")).unwrap(),
            "theirs"
        );
        assert!(!root.join("in/target").is_symlink());
        assert_eq!(
            std::fs::read_to_string(root.join("in/target")).unwrap(),
            "mine"
        );
    }
}
