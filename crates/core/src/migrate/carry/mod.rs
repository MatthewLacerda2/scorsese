//! A local folder's paid files, found under the names a renamed asset's new id
//! gives them.
//!
//! A run trusts the file a brief names, never the document's state: a paid
//! output is `generated/<id>-<brief hash>.<ext>`, and a brief whose file is
//! not there is bought again. So an asset the migration renames (v55 → v56,
//! [`super::ids`]) would be paid for twice unless its files answer to the new
//! name as well. Each is linked there — a hard link, or a copy where the
//! filesystem has none — and the file paid for stays where it was, byte for
//! byte; the asset's `path` names the link (the step moves it).
//!
//! Only the folder has files, so only [`super::folder`] carries them. The web
//! server's stored projects need nothing: a folder is laid out for each tool,
//! and each paid output is linked from the library by its brief's hash at the
//! name the current id gives it.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::MigrateError;
use crate::asset::AssetId;
use crate::path::ProjectPath;
use crate::project::{GENERATED_DIR, Project};

/// How long a brief hash is: a SHA-256 in hex.
const DIGEST_LENGTH: usize = 64;

/// Each asset whose id the walk changed, as `before` named it and as `after`
/// does. The walk keeps the table's order, so position pairs them.
pub(super) fn renamed(before: &Value, after: &Project) -> Vec<(AssetId, AssetId)> {
    let old = before.get("assets").and_then(Value::as_array);
    old.into_iter()
        .flatten()
        .zip(&after.assets)
        .filter_map(|(old, new)| {
            let old = old.get("id")?.as_str()?;
            (old != new.id.as_str()).then(|| (AssetId::new(old), new.id.clone()))
        })
        .collect()
}

/// Links every file in `generated/` named for an old id and a brief hash at
/// the same name under the new id. A name already taken is left as it is.
pub(super) fn paid_outputs(
    root: &Path,
    renames: &[(AssetId, AssetId)],
) -> Result<(), MigrateError> {
    for (old, new) in renames {
        // An old id that climbs names nothing of the folder's.
        let Ok(stem) = ProjectPath::new(format!("{GENERATED_DIR}/{old}-")).resolve(root) else {
            continue;
        };
        let (Some(dir), Some(prefix)) = (stem.parent(), stem.file_name()) else {
            continue;
        };
        let Some(prefix) = prefix.to_str() else {
            continue;
        };
        for (source, rest) in briefs(dir, prefix) {
            let Ok(target) =
                ProjectPath::new(format!("{GENERATED_DIR}/{new}-{rest}")).resolve(root)
            else {
                continue;
            };
            if !target.exists() {
                link(&source, &target)?;
            }
        }
    }
    Ok(())
}

/// Each file in `dir` named `<prefix><brief hash>.<anything>`, and the part of
/// its name after `prefix`.
fn briefs(dir: &Path, prefix: &str) -> Vec<(PathBuf, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let rest = name.strip_prefix(prefix)?;
            is_brief_name(rest).then(|| (entry.path(), rest.to_owned()))
        })
        .collect()
}

/// Whether `rest`, a file name with its asset id taken off the front, is a
/// brief's: a brief hash, then `.` and anything (`.jpg`, `.mp3.words.json`).
pub(super) fn is_brief_name(rest: &str) -> bool {
    rest.split_at_checked(DIGEST_LENGTH)
        .is_some_and(|(digest, extension)| {
            let hashed = digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
            hashed && extension.starts_with('.') && !extension.contains('/')
        })
}

/// `target` made to hold `source`'s bytes: the same file where the
/// filesystem allows, a copy where it does not.
fn link(source: &Path, target: &Path) -> Result<(), MigrateError> {
    let io = |source| MigrateError::Io {
        path: target.to_path_buf(),
        source,
    };
    if std::fs::hard_link(source, target).is_err() {
        std::fs::copy(source, target).map_err(io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
