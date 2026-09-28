//! Adding a file that is already where the project will find it — hashed,
//! measured and placed — to the assets table, without copying anything.
//!
//! [`import_asset`](super::import_asset) copies a file from outside into
//! `assets/`. A caller that keeps media elsewhere and lays it out where the
//! document says it is — the hosted server, whose library holds every file
//! once per user and links it into a project at `assets/<sha256>.<ext>`
//! (#534, #539) — has nothing to copy: the file was hashed and measured when it
//! entered the library, by the same [`measure`](super::measure) import uses.
//! What is left is the document half of an import, and it is the same half:
//! the same id rules, and the same answer when the pool already has the file.

use crate::asset::{Asset, AssetId, AssetKind, MediaMetadata};
use crate::path::ProjectPath;
use crate::project::Project;

use super::import::already_in_pool;
use super::naming::{sanitise, unique_asset_id};

/// A file to add to the assets table, as whoever measured it knows it.
#[derive(Debug, Clone)]
pub struct Reference {
    /// What the file is called — where the asset's id comes from, exactly as
    /// an imported file's comes from its file name.
    pub name: String,
    /// What it is added as.
    pub kind: AssetKind,
    /// Where the project finds it, project-relative.
    pub path: ProjectPath,
    /// Its content hash.
    pub sha256: String,
    /// What probing it found.
    pub media: MediaMetadata,
}

/// Add `file` to the assets table, and answer the id a clip references it by.
///
/// **A file the pool already holds is not added twice** — its existing id is
/// the answer, as for an import of the same bytes: assets are entities, and
/// two clips pointing at one entity is the intended shape. A new asset's id is
/// the name's, sanitised and suffixed until it is free.
pub fn reference_asset(project: &mut Project, file: Reference) -> AssetId {
    if let Some(existing) = already_in_pool(project, &file.sha256) {
        return existing;
    }
    let id = unique_asset_id(project, &sanitise(&file.name));
    project.assets.push(Asset {
        sha256: Some(file.sha256),
        media: Some(file.media),
        ..Asset::imported(id.clone(), file.kind, file.path)
    });
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, sha256: &str) -> Reference {
        Reference {
            name: name.to_owned(),
            kind: AssetKind::Video,
            path: ProjectPath::new(format!("assets/{sha256}.mp4")),
            sha256: sha256.to_owned(),
            media: MediaMetadata::default(),
        }
    }

    #[test]
    fn a_file_is_named_from_its_name_and_never_added_twice() {
        let mut project = Project::new("p", Default::default());
        let first = reference_asset(&mut project, file("Rooftop Take 2.mp4", "a"));
        assert_eq!(first.as_str(), "rooftop-take-2");
        let again = reference_asset(&mut project, file("another name.mp4", "a"));
        assert_eq!(again, first);
        let other = reference_asset(&mut project, file("Rooftop Take 2.mp4", "b"));
        assert_eq!(other.as_str(), "rooftop-take-2-2");
        assert_eq!(project.assets.len(), 2);
        assert_eq!(
            project.assets[0].path.as_ref().map(ProjectPath::as_str),
            Some("assets/a.mp4")
        );
    }
}
