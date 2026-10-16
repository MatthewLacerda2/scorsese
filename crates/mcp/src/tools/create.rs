//! Starting a project, which is the one step that had to happen elsewhere.
//!
//! Every other tool here takes a `project` that must already exist, so an
//! assistant handed a machine with nothing on it had no first move: it had to
//! ask for a command line, which is the one thing this server exists to avoid.
//! The operation itself is `scorsese new` — a directory, its four
//! sub-directories, and a `project.json` for an empty timeline on a chosen
//! grid.

use schemars::{JsonSchema, Schema, SchemaGenerator};
use scorsese_core::style::{Platform, Start};
use scorsese_core::{Fps, PROJECT_DIRS, PROJECT_FILE_NAME, Project, Ratio};
use serde::Deserialize;
use serde_json::Value;

use super::args::{self, ProjectDir};
use super::{Costs, Reply, Tool};

/// Lay out a new project directory.
pub(crate) struct New;

/// What `project_new` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    /// Path of the *.scor directory to create, e.g. teaser.scor. It is made if
    /// it is not there, and must be empty if it is.
    project: ProjectDir,
    /// What a human calls this edit. Cosmetic, and independent of the
    /// directory. Defaults to the directory's own name without its extension.
    name: Option<String>,
    /// The timeline framerate every clip and keyframe time is counted in: 30,
    /// or a rational like 30000/1001 for 29.97. Chosen once, here — changing it
    /// later is a real operation, not a field edit. A decimal is refused: 29.97
    /// is not a framerate. Defaults to 30.
    // Read as a bare value so [`fps`] can take either spelling; `default`
    // because `schema_with` hides that it is optional.
    #[serde(default)]
    #[schemars(schema_with = "fps_schema")]
    fps: Option<Value>,
    /// The placement the video is made for: youtube, youtube_shorts,
    /// instagram_reels, instagram_reels_ad, instagram_stories_ad, tiktok or
    /// tiktok_ad. Written into the brief with the render preset it means
    /// (render's platform), and frames the project in its shape; the project
    /// stores the shape, never the platform.
    platform: Option<String>,
    /// The shape of the frame, e.g. "9:16", "16:9", "1:1" or "4:5": what every
    /// render, still and layout of the project defaults to. Defaults to
    /// platform's shape, or 16:9 without one. project_reframe changes it later.
    aspect: Option<String>,
    /// The kind of video, by id: kinetic_type, whiteboard, narrated_captions…
    /// An unknown id, or one not made for platform, is refused with the ids
    /// that fit. Its prompt is written into the brief.
    style: Option<String>,
}

impl args::Arguments for Arguments {}

/// The two spellings `fps` takes on the wire.
fn fps_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({ "type": ["string", "number"] })
}

impl Tool for New {
    fn name(&self) -> &'static str {
        "project_new"
    }

    fn description(&self) -> &'static str {
        "Create a *.scor project directory: project.json, and the assets/, \
         generated/, recipes/, pages/ and cache/ folders beside it. This is the first \
         call on a machine that has no project yet — every other tool here works \
         on one that already exists. The name defaults to the directory's own \
         and the grid to 30 fps, so the usual call names nothing but the path. \
         The directory must not already hold anything: a project is never laid \
         over what was there before, and nothing is written when it refuses. \
         Give platform and/or style when the person has said where the video \
         is going or what kind it is: the project then starts with a brief in \
         script.md (script_read). Either way, before editing anything, propose \
         the script to the person and agree it: scene by scene, each with its \
         narration, what is on screen, and the music or sound, plus a \
         storyboard in words of every shot to be generated (guide prompts)."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let fps = fps(arguments.fps.as_ref())?;
        let start = start(arguments.platform.as_deref(), arguments.style.as_deref())?;
        let aspect = match arguments.aspect.as_deref() {
            Some(text) => text
                .parse::<Ratio>()
                .map_err(|problem| format!("aspect: {problem}"))?,
            None => start.aspect().unwrap_or_default(),
        };

        let mut project = Project::create(dir, arguments.name.as_deref(), fps, aspect)
            .map_err(|error| error.to_string())?;
        let script = start
            .write(dir, &mut project)
            .map_err(|error| format!("the project was made, but its brief: {error}"))?;

        let mut reply = format!(
            "Created project \"{}\" at {} fps, framed {}, in {}\n  {PROJECT_FILE_NAME}, {}/",
            project.name,
            project.timeline_fps,
            project.aspect,
            dir.display(),
            PROJECT_DIRS.join("/, ")
        );
        if let Some(script) = script {
            reply.push_str(&format!(
                "\n  {script}: the brief. Read it, then propose the script, scene by scene."
            ));
        }
        Ok(reply.into())
    }
}

/// The platform and style asked for, checked before anything is written, in
/// `scorsese new`'s words.
fn start(platform: Option<&str>, style: Option<&str>) -> Result<Start, String> {
    let platform = platform
        .map(str::parse::<Platform>)
        .transpose()
        .map_err(|problem| format!("platform: {problem}"))?;
    Start::new(platform, style).map_err(|problem| format!("style: {problem}"))
}

/// The grid asked for, as either the text `scorsese new --fps` takes or the
/// plain number a client is just as likely to send.
///
/// Both land in the same parser, so `30000/1001` and `29.97` mean here exactly
/// what they mean on the command line — the second of them nothing, and loudly.
fn fps(asked: Option<&Value>) -> Result<Fps, String> {
    let Some(asked) = asked else {
        return Ok(Fps::DEFAULT);
    };
    let text = match asked {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        other => return Err(format!("`fps` should be `30` or `30000/1001`, not {other}")),
    };
    text.parse()
        .map_err(|error| format!("`fps` is not a framerate: {error}"))
}
