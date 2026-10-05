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

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_core::{AssetKind, Import as Report, Project, import_path};
use scorsese_render::Ffprobe;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::tools::args::{self, ProjectDir};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Copy media into the pool.
pub(crate) struct Import;

/// What `import` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The file or directory to import, anywhere on disk, or a list of them. A
    /// directory brings in the media directly inside it and does not recurse.
    /// These paths are used to find the media and are never written into the
    /// project.
    path: Paths,
    /// What the media is, instead of inferring it from the extension — a .mp4
    /// that is in the edit for its sound, say. For a directory it says what the
    /// media in it is; which files count as media at all is still the
    /// extension's answer.
    //
    // Text checked by [`kind`] rather than an enum, so that a kind that cannot
    // be imported is refused in words that say why.
    #[schemars(extend("enum" = ["video", "image", "audio"]))]
    kind: Option<String>,
    /// Bring each path — a folder — in as one image_sequence: its png, jpeg,
    /// bmp, tiff or webp frames become image assets under assets/<folder
    /// name>/, played in the order their numbers say. A gap in the numbering is
    /// reported, never refused; frames of two formats or two sizes refuse the
    /// folder with nothing copied. `kind` does not apply.
    #[serde(default)]
    sequence: bool,
}

impl args::Arguments for Arguments {}

/// The kind override, if one was named.
fn kind(given: Option<&str>) -> Result<Option<AssetKind>, String> {
    match given {
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
         and the failures are named at the end. With `sequence: true` each path \
         is a folder of numbered frames brought in as ONE image_sequence asset — \
         a timelapse, stop motion, a rendered frame directory — whose stills play \
         in number order (frame_9 before frame_10), one frame each and once; the \
         sequence tool changes the hold and the loop. A web page (.html) is \
         copied into pages/ instead, as an html asset: a document played as a \
         moving picture with alpha, neither probed nor hashed. Pages come in \
         one file at a time — a directory passes them over."
    }

    fn costs(&self) -> Costs {
        Costs::Probe
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let paths = arguments.path.checked()?;
        let kind = kind(arguments.kind.as_deref())?;
        let mut project = load(dir)?;

        // Discovered per call rather than held, for the same reason `render`
        // does it: a server that found ffprobe at startup would keep insisting
        // it was there after someone uninstalled it.
        let probe = Ffprobe::discover().map_err(|error| format!("{error}"))?;
        if arguments.sequence {
            return super::sequenced::import(&mut project, dir, &paths, &probe);
        }
        let mut reports = Vec::new();
        let mut failures = Vec::new();
        let mut copied = false;
        for path in &paths {
            match import_path(&mut project, dir, std::path::Path::new(path), kind, &probe) {
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
                .save(dir)
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
enum Paths {
    /// `"path": "intro.mp4"`.
    One(String),
    /// `"path": ["intro.mp4", "outro.mp4"]`.
    Many(Vec<String>),
}

impl Paths {
    /// The paths, with nothing in them refused: an empty list or a blank path
    /// is somebody not naming one, and a blank among several is a list with a
    /// hole in it.
    fn checked(self) -> Result<Vec<String>, String> {
        let blank = |path: &String| path.trim().is_empty();
        match self {
            Self::Many(paths) if paths.is_empty() => Err("`path` is required".to_owned()),
            Self::Many(paths) if paths.iter().any(blank) => {
                Err("every `path` must be a non-empty string".to_owned())
            }
            Self::One(path) if blank(&path) => Err("`path` is required".to_owned()),
            Self::Many(paths) => Ok(paths),
            Self::One(path) => Ok(vec![path]),
        }
    }
}

impl<'de> Deserialize<'de> for Paths {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let shape = || D::Error::custom("a path or a list of paths");
        match Value::deserialize(deserializer)? {
            Value::String(path) => Ok(Self::One(path)),
            Value::Array(items) => items
                .into_iter()
                .map(|item| match item {
                    Value::String(path) => Ok(path),
                    _ => Err(shape()),
                })
                .collect::<Result<_, _>>()
                .map(Self::Many),
            _ => Err(shape()),
        }
    }
}

impl JsonSchema for Paths {
    fn schema_name() -> Cow<'static, str> {
        "Paths".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": ["string", "array"],
            "items": { "type": "string" }
        })
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
