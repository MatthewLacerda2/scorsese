//! Bringing media in from outside the project.
//!
//! Without this, media anywhere but inside the project could not be brought in
//! over the protocol at all — the only route was to write an asset into
//! `project.json` and probe it, which needs the file to already be in
//! `assets/`. An assistant handed a folder of footage could not start.
//!
//! The path outside the project is an argument to the call and nothing more:
//! the file is copied, and what gets written down is the relative path it
//! landed at. That is the same bargain `scorsese import` has always kept, and
//! it is what makes a project survive `scp -r`.

use scorsese_core::{AssetKind, Import as Report, Project, import_path};
use scorsese_render::Ffprobe;
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Reply, Tool, project_dir, project_property};

/// Copy media into the pool.
pub(crate) struct Import;

impl Tool for Import {
    fn name(&self) -> &'static str {
        "import"
    }

    fn description(&self) -> &'static str {
        "Copy a media file into the project's assets/ and add it to the assets \
         table, ready for a clip to reference. `path` may also be a directory, \
         which imports the media directly inside it — one asset each, sorted by \
         file name, without recursing; the folder itself never becomes an \
         asset. This is the only way to bring media in from outside the \
         project: writing an asset into project.json points at a file that is \
         already there. Everything is copied, never referenced in place, and \
         everything is probed as it comes in. Files that are not media are \
         skipped and named. A file whose id an asset already answers to is \
         refused with nothing copied at all; media already in the pool is not a \
         collision — it comes back as the asset that already holds those bytes."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "path": {
                    "type": "string",
                    "description": "The file or directory to import, anywhere on \
                                    disk. A directory brings in the media directly \
                                    inside it and does not recurse. This path is \
                                    used to find the media and is never written \
                                    into the project."
                },
                "kind": {
                    "type": "string",
                    "enum": ["video", "image", "audio"],
                    "description": "What the media is, instead of inferring it from \
                                    the extension — a .mp4 that is in the edit for \
                                    its sound, say. For a directory it says what the \
                                    media in it is; which files count as media at \
                                    all is still the extension's answer."
                }
            },
            "required": ["project", "path"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let path = argument(arguments, "path")?;
        let kind = kind(arguments)?;
        let mut project = load(&dir)?;

        // Discovered per call rather than held, for the same reason `render`
        // does it: a server that found ffprobe at startup would keep insisting
        // it was there after someone uninstalled it.
        let probe = Ffprobe::discover().map_err(|error| format!("{error}"))?;
        let report = import_path(
            &mut project,
            &dir,
            std::path::Path::new(&path),
            kind,
            &probe,
        )
        .map_err(|error| format!("importing {path}: {error} — nothing was imported"))?;

        if report.imported.iter().any(|one| !one.reused) {
            project
                .save(&dir)
                .map_err(|error| format!("saving the project: {error}"))?;
        }
        Ok(said(&project, &report).into())
    }
}

/// A required string argument.
fn argument(arguments: &Value, key: &'static str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("`{key}` is required"))
}

/// The kind override, if one was named.
fn kind(arguments: &Value) -> Result<Option<AssetKind>, String> {
    match arguments.get("kind").and_then(Value::as_str) {
        None => Ok(None),
        Some("video") => Ok(Some(AssetKind::Video)),
        Some("image") => Ok(Some(AssetKind::Image)),
        Some("audio") => Ok(Some(AssetKind::Audio)),
        // The authored kinds are absent on purpose: a title and a prompt carry
        // a string rather than a file, so there is nothing to copy in.
        Some(other) => Err(format!(
            "`{other}` is not a kind that can be imported: video, image or audio"
        )),
    }
}

/// What came in, what each was measured to be, and what was passed over.
///
/// The skips go last because they are the part a caller has to act on, and a
/// licence file quietly standing in for a mistyped video is exactly what
/// naming them prevents.
fn said(project: &Project, report: &Report) -> String {
    let mut lines = Vec::new();
    for one in &report.imported {
        if one.reused {
            lines.push(format!(
                "{} — already in the pool, nothing copied ({})",
                one.id, one.source
            ));
            continue;
        }
        let Some(asset) = project.asset(&one.id) else {
            continue;
        };
        let path = asset.path.as_ref().map(ToString::to_string);
        lines.push(format!(
            "{} — {:?}, {} ({})",
            one.id,
            asset.kind,
            asset
                .media
                .as_ref()
                .map_or_else(|| "no metadata reported".to_owned(), ToString::to_string),
            path.unwrap_or_else(|| one.source.clone())
        ));
    }
    for skipped in &report.skipped {
        lines.push(format!("{} — skipped: {}", skipped.source, skipped.why));
    }
    if lines.is_empty() {
        return "nothing to import: that directory holds no media".to_owned();
    }
    lines.join("\n")
}
