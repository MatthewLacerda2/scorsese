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
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Copy media into the pool.
pub(crate) struct Import;

impl Tool for Import {
    fn name(&self) -> &'static str {
        "import"
    }

    fn description(&self) -> &'static str {
        "Copy media into the project's assets/ and add it to the assets \
         table, ready for a clip to reference. `path` is one path or a list of \
         them — media arrives in sets, and naming the set costs one call \
         instead of one per file. A path may also be a directory, \
         which imports the media directly inside it — one asset each, sorted by \
         file name, without recursing; the folder itself never becomes an \
         asset. This is the only way to bring media in from outside the \
         project: writing an asset into project.json points at a file that is \
         already there. Everything is copied, never referenced in place, and \
         everything is probed as it comes in. Files that are not media are \
         skipped and named. A single file whose id is already taken is suffixed \
         out of the way — `intro.mp4` lands as `intro-2` — and the reply says \
         so, so the id to write on a clip is the one the reply names rather than \
         the one the file name suggests. The same collision inside a directory \
         refuses the whole batch with nothing copied at all. Media already in \
         the pool is not a collision — it comes back as the asset that already \
         holds those bytes. When several paths are named, one that fails does \
         not cost the others: everything that worked is saved and reported, \
         and the failures are named at the end."
    }

    fn costs(&self) -> Costs {
        Costs::Probe
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "path": {
                    "type": ["string", "array"],
                    "items": { "type": "string" },
                    "description": "The file or directory to import, anywhere on \
                                    disk, or a list of them. A directory brings in \
                                    the media directly inside it and does not \
                                    recurse. These paths are used to find the media \
                                    and are never written into the project."
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
        let paths = paths(arguments)?;
        let kind = kind(arguments)?;
        let mut project = load(&dir)?;

        // Discovered per call rather than held, for the same reason `render`
        // does it: a server that found ffprobe at startup would keep insisting
        // it was there after someone uninstalled it.
        let probe = Ffprobe::discover().map_err(|error| format!("{error}"))?;
        let mut reports = Vec::new();
        let mut failures = Vec::new();
        let mut copied = false;
        for path in &paths {
            match import_path(&mut project, &dir, std::path::Path::new(path), kind, &probe) {
                Ok(report) => {
                    copied |= report.imported.iter().any(|one| !one.reused);
                    reports.push(report);
                }
                Err(error) => failures.push(format!("{path} — failed: {error}")),
            }
        }
        // Once, after the loop: an id in the reply has to already be an id on
        // disk, and a failure in the middle must not discard what landed
        // before it.
        if copied {
            project
                .save(&dir)
                .map_err(|error| format!("saving the project: {error}"))?;
        }
        // Everything failing is the single-path error this tool has always
        // returned; anything landing is a reply, because the caller needs the
        // ids of what did land and re-importing the rest is free — the same
        // bytes come back as the asset that already holds them.
        if reports.is_empty() {
            return Err(format!("{} — nothing was imported", failures.join("; ")));
        }
        Ok(said(&project, &reports, &failures).into())
    }
}

/// The paths to import: one string, or a list of them.
///
/// Both spellings rather than a list only, because one file is the common call
/// and `"path": "intro.mp4"` is what every caller wrote before the list
/// existed — a schema that broke those would be a worse trade than accepting
/// two shapes.
fn paths(arguments: &Value) -> Result<Vec<String>, String> {
    let named = |value: &Value| {
        value
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .map(ToOwned::to_owned)
    };
    match arguments.get("path") {
        Some(Value::Array(items)) if !items.is_empty() => items
            .iter()
            .map(|item| {
                named(item).ok_or_else(|| "every `path` must be a non-empty string".to_owned())
            })
            .collect(),
        Some(one) => named(one)
            .map(|path| vec![path])
            .ok_or_else(|| "`path` is required".to_owned()),
        None => Err("`path` is required".to_owned()),
    }
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
fn said(project: &Project, reports: &[Report], failures: &[String]) -> String {
    let mut lines = Vec::new();
    for one in reports.iter().flat_map(|report| &report.imported) {
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
        // On the same line as the id, because the id is what the caller is
        // about to write on a clip and "which id" and "not the one you asked
        // for" are one fact rather than two.
        let renamed = one
            .wanted
            .as_ref()
            .map_or_else(String::new, |wanted| format!(", renamed: `{wanted}` taken"));
        lines.push(format!(
            "{} — {:?}, {} ({}){renamed}",
            one.id,
            asset.kind,
            asset
                .media
                .as_ref()
                .map_or_else(|| "no metadata reported".to_owned(), ToString::to_string),
            path.unwrap_or_else(|| one.source.clone())
        ));
    }
    for skipped in reports.iter().flat_map(|report| &report.skipped) {
        lines.push(format!("{} — skipped: {}", skipped.source, skipped.why));
    }
    // Last, with the skips, and for the same reason: what the caller has to
    // act on reads worst buried above a list of successes.
    lines.extend(failures.iter().cloned());
    if lines.is_empty() {
        return "nothing to import: that directory holds no media".to_owned();
    }
    lines.join("\n")
}
