//! The authored text a stored project keeps beside its document (#560): its
//! synthesis recipes and its script.
//!
//! ## Why a table of files
//!
//! A `.scor` folder holds two kinds of thing besides `project.json` that are
//! **not rebuildable**: `recipes/` (CLAUDE.md: deleting one loses work) and the
//! script the document's `script` names. Media has a home on the server — the
//! user's library, by hash — and these did not, so on the web a recipe could
//! not be written or baked and a script could not be read. They are kept in
//! `project_files`, one row per file keyed by (project, project-relative path),
//! holding its text.
//!
//! Weighed against the other two shapes #560 named: **library items** would
//! make a recipe a file somebody could find and delete from a library page,
//! and would give every edit of a recipe a new hash and a new item — the
//! library is for media, addressed by content, and a recipe is authored text
//! edited in place. **Folding them into the document** would be a
//! `project.json` format change for the server's convenience, and would put
//! a 30 KB script inside the one file an agent opens to learn the edit — the
//! reason `script` is a file in the first place. A table keyed by path is
//! exactly what a folder is, which is why nothing above this module has to
//! know the difference.
//!
//! ## Which files are kept
//!
//! [`kept`]: every file under `recipes/` — recipes, and the instrument
//! patches a song names by path — plus the file the document's `script`
//! names and every file an asset's `recipe` names wherever it lives. Never
//! `project.json`, and nothing under `assets/`, `generated/` or `cache/`:
//! those are the document, the library's and rebuildable, each kept
//! elsewhere or not at all.
//!
//! A file that was kept stays kept until something deletes it: [`gather`]
//! reads back every path that was laid out as well as the ones the rule names
//! now, so removing the asset that pointed at a recipe outside `recipes/` does
//! not quietly delete the recipe — on disk it would still be there.
//!
//! ## Written with the document, under its revision
//!
//! The files are saved in the same transaction as the document and under the
//! same revision check ([`crate::projects::save_with_files`]), so a recipe
//! edit races like any other edit: the second writer is refused and runs again
//! on what is there now. Text only — a recipe is JSON and a script is prose —
//! and at most [`MAX_FILE_BYTES`] each and [`MAX_FILES`] to a project, so a
//! project's files cannot grow without bound on a machine other people share.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use scorsese_core::{
    ASSETS_DIR, CACHE_DIR, GENERATED_DIR, PROJECT_FILE_NAME, Project, ProjectPath, RECIPES_DIR,
};

use super::ProjectError;
use super::media::MaterialiseError;
use crate::db::Tx;

/// The most one kept file may hold, in bytes — the table holds it to the same.
pub const MAX_FILE_BYTES: usize = 1 << 20;

/// The most files one project may keep.
pub const MAX_FILES: usize = 500;

/// A project's kept files: project-relative path to text, in path order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectFiles(BTreeMap<String, String>);

impl ProjectFiles {
    /// Keep `text` at `path` — refused when the path is not one a project
    /// keeps a file at, or the text is over [`MAX_FILE_BYTES`].
    pub fn insert(&mut self, path: &str, text: String) -> Result<(), String> {
        let path = normal(path).ok_or_else(|| format!("{path} is not a file a project keeps"))?;
        if text.len() > MAX_FILE_BYTES {
            return Err(format!(
                "{path} is {} bytes; the server keeps a project's recipes and script up to \
                 {MAX_FILE_BYTES} bytes each",
                text.len()
            ));
        }
        self.0.insert(path, text);
        Ok(())
    }

    /// The text at `path`, if a file is kept there.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.0.get(&normal(path)?).map(String::as_str)
    }

    /// Every file, path and text, in path order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
    }

    /// How many files.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// `path` as one key: `.` and empty segments dropped, so `./script.md` and
/// `script.md` are one file — or `None` for a path no kept file can have.
fn normal(path: &str) -> Option<String> {
    ProjectPath::new(path).check().ok()?;
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect();
    let first = *segments.first()?;
    let elsewhere = [ASSETS_DIR, GENERATED_DIR, CACHE_DIR].contains(&first);
    let document = segments.len() == 1 && first == PROJECT_FILE_NAME;
    (!elsewhere && !document).then(|| segments.join("/"))
}

/// Whether `project` keeps a file at `path`: under `recipes/`, or the script
/// or a recipe the document names — and never the document or media.
pub fn kept(project: &Project, path: &str) -> bool {
    let Some(path) = normal(path) else {
        return false;
    };
    path.starts_with(&format!("{RECIPES_DIR}/")) || named(project).contains(&path)
}

/// The files the document names: its script and its assets' recipes.
fn named(project: &Project) -> BTreeSet<String> {
    naming(project)
        .filter_map(|path| normal(path.as_str()))
        .collect()
}

/// Every path the document names a script or a recipe at, as written.
fn naming(project: &Project) -> impl Iterator<Item = &ProjectPath> {
    let recipes = project
        .assets
        .iter()
        .filter_map(|asset| asset.recipe.as_ref());
    project.script.iter().chain(recipes)
}

/// Refuse a script or recipe the document names at a path no kept file can
/// have, when the folder holds one there: saving would keep the document
/// pointing at it and lose the file with the folder.
fn unkeepable(root: &Path, project: &Project) -> Result<(), String> {
    for path in naming(project) {
        let written = path.check().is_ok()
            && path
                .resolve(root)
                .symlink_metadata()
                .is_ok_and(|meta| meta.is_file());
        if written && normal(path.as_str()).is_none() {
            return Err(format!(
                "{path} cannot hold a script or a recipe on the server: it keeps those \
                 anywhere but project.json, assets/, generated/ and cache/"
            ));
        }
    }
    Ok(())
}

/// Read back the kept files of the folder at `root`, after something ran on
/// it: everything under `recipes/`, what `project` names, and every path in
/// `laid` that is still there. Only regular files — a link is never followed
/// — and only text; refused when a file or the count is over its cap, or
/// when the document names a script or recipe it wrote where none is kept.
pub fn gather(root: &Path, project: &Project, laid: &ProjectFiles) -> Result<ProjectFiles, String> {
    unkeepable(root, project)?;
    let mut paths: BTreeSet<String> = named(project);
    paths.extend(laid.0.keys().cloned());
    walk(root, RECIPES_DIR, &mut paths);
    let mut files = ProjectFiles::default();
    for path in paths {
        let Some(key) = normal(&path) else {
            continue;
        };
        let file = ProjectPath::new(key.as_str()).resolve(root);
        if !file.symlink_metadata().is_ok_and(|meta| meta.is_file()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue; // Not text: nothing a recipe or a script can be.
        };
        files.insert(&key, text)?;
    }
    if files.len() > MAX_FILES {
        return Err(format!(
            "the project would keep {} recipes and scripts; the server keeps at most \
             {MAX_FILES} to a project",
            files.len()
        ));
    }
    Ok(files)
}

/// Every regular file under `relative` in `root`, recursively, as
/// project-relative paths. A link — to a file or a directory — is skipped.
fn walk(root: &Path, relative: &str, into: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(ProjectPath::new(relative).resolve(root)) else {
        return;
    };
    for entry in entries.flatten() {
        let (Ok(kind), Some(name)) = (
            entry.file_type(),
            entry.file_name().to_str().map(str::to_owned),
        ) else {
            continue;
        };
        let path = format!("{relative}/{name}");
        if kind.is_dir() {
            walk(root, &path, into);
        } else if kind.is_file() {
            into.insert(path);
        }
    }
}

/// Write every file into the folder at `root`, making directories as needed.
/// The paths were checked when they were kept, and are checked again here.
pub(crate) fn lay(files: &ProjectFiles, root: &Path) -> Result<(), MaterialiseError> {
    for (path, text) in files.iter() {
        let key = normal(path).ok_or_else(|| MaterialiseError::BadFile(path.to_owned()))?;
        let file = ProjectPath::new(key.as_str()).resolve(root);
        let made = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, text));
        made.map_err(|source| MaterialiseError::Io { path: file, source })?;
    }
    Ok(())
}

/// Project `id`'s kept files.
pub(super) async fn read(tx: &mut Tx, id: i64) -> Result<ProjectFiles, ProjectError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT path, content FROM project_files WHERE project_id = $1")
            .bind(id)
            .fetch_all(&mut **tx)
            .await?;
    Ok(ProjectFiles(rows.into_iter().collect()))
}

/// Make project `id`'s kept files exactly `files`: the ones not in it
/// deleted, the new and the changed written, the rest left alone.
pub(super) async fn write(tx: &mut Tx, id: i64, files: &ProjectFiles) -> Result<(), ProjectError> {
    let (paths, texts): (Vec<&str>, Vec<&str>) = files.iter().unzip();
    sqlx::query("DELETE FROM project_files WHERE project_id = $1 AND NOT (path = ANY($2::text[]))")
        .bind(id)
        .bind(&paths)
        .execute(&mut **tx)
        .await?;
    // The owner comes from the project row, as `project_assets`' does.
    sqlx::query(
        "INSERT INTO project_files (project_id, user_id, path, content)
         SELECT p.id, p.user_id, f.path, f.content
         FROM projects p, unnest($2::text[], $3::text[]) AS f (path, content) WHERE p.id = $1
         ON CONFLICT (project_id, path) DO UPDATE
             SET content = EXCLUDED.content, updated_at = now()
             WHERE project_files.content IS DISTINCT FROM EXCLUDED.content",
    )
    .bind(id)
    .bind(&paths)
    .bind(&texts)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use scorsese_core::{Asset, AssetId, AssetKind, Fps};

    use super::*;

    fn project() -> Project {
        let mut project = Project::new("p", Fps::default());
        project.script = Some(ProjectPath::new("./notes/brief.md"));
        let mut theme = Asset::imported(
            AssetId::new("theme"),
            AssetKind::SynthAudio,
            ProjectPath::new("generated/x.wav"),
        );
        theme.recipe = Some(ProjectPath::new("music/theme.json"));
        project.assets.push(theme);
        project
    }

    #[test]
    fn recipes_and_what_the_document_names_are_kept_and_nothing_else() {
        let project = project();
        for path in ["recipes/a.json", "recipes/kit/snare.json", "notes/brief.md"] {
            assert!(kept(&project, path), "{path}");
        }
        assert!(kept(&project, "music/theme.json"));
        for path in [
            "project.json",
            "./project.json",
            "assets/x.mp4",
            "generated/x.wav",
            "cache/solo.wav",
            "script.md",
            "../recipes/a.json",
            "/etc/passwd",
            "recipes",
            "",
        ] {
            assert!(!kept(&project, path), "{path}");
        }
    }

    #[test]
    fn a_path_is_one_key_however_it_is_spelled() {
        let mut files = ProjectFiles::default();
        files.insert("./recipes//a.json", "{}".into()).unwrap();
        assert_eq!(files.get("recipes/a.json"), Some("{}"));
        assert_eq!(
            files.iter().next().map(|(path, _)| path),
            Some("recipes/a.json")
        );
        assert!(files.insert("generated/a.json", "{}".into()).is_err());
        let over = "x".repeat(MAX_FILE_BYTES + 1);
        assert!(files.insert("recipes/b.json", over).is_err());
        assert_eq!(files.len(), 1);
    }
}
