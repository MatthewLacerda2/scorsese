//! `render`: a render of the project asked for, exactly as `POST
//! /api/projects/{id}/renders` asks for one ([`crate::renders::request`]).

use scorsese_mcp::Reply;
use scorsese_render::{Budget, Resolution};
use serde_json::{Value, json};

use super::super::super::surface::project_property;
use super::super::super::{Caller, database, project_id};
use super::state;
use crate::projects::{self, ProjectError};
use crate::renders::request::{AskError, Asked, ask};
use crate::renders::{Ask, Settings};

/// How a client names the render.
pub(in super::super) const RENDER: &str = "render";

/// What the render does.
pub(in super::super) const RENDER_SAYS: &str = "Render the timeline, or a stretch of it, to a video file, \
or to a sound file of its mix alone, made by the server's render queue and kept for you to download. Takes \
real time, on a machine other people share — call project_describe first to check the cut is \
right, since that costs nothing. The same project, unchanged, in the same shape is rendered \
once: asking again answers with the file already made. The reply is either the download \
address, or the job making it; `jobs` says when that job is done and what it made. Download \
with an HTTP GET on this server, with the same token. Sketch and stale generated assets render \
as slug cards rather than failing; narration_bands: false leaves the narration lines' bands \
off the picture. loudness: -14 (any LUFS) brings the soundtrack to that loudness, with a \
limiter holding its peaks; the finished job's notes say how loud the file came out.";

/// `render`'s arguments: the shape of the file, as the stdio tool spells it.
pub(in super::super) fn render_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "project": project_property(),
            "container": {
                "type": "string",
                "description": "Container to deliver in: mp4 (the default), mkv, avi or wmv \
                                for video; mp3, wav or m4a for the soundtrack alone."
            },
            "video_codec": {
                "type": "string",
                "description": "Picture codec: h264, mpeg4 or wmv2. Defaults to what the \
                                container is written with; a pairing scorsese does not write \
                                is refused before anything is queued."
            },
            "audio_codec": {
                "type": "string",
                "description": "Sound codec: aac, pcm_s16le, wmav2 or mp3. Defaults to what \
                                the container is written with."
            },
            "resolution": {
                "type": "string",
                "description": "Output size, e.g. 1080x1920, or a shape like 9:16 at \
                                delivery size. Defaults to the project's own frame at \
                                delivery size (1920x1080 for 16:9, 1080x1920 for 9:16). \
                                Refused for a sound-only container, which has no picture \
                                to size, and beside platform."
            },
            "platform": {
                "type": "string",
                "description": "The placement the video is delivered for, which sizes it: \
                                youtube (1920x1080), or youtube_shorts, instagram_reels, \
                                instagram_reels_ad, instagram_stories_ad, tiktok or \
                                tiktok_ad (1080x1920). For this render only; the project is \
                                not changed. Cannot be given with resolution, and refused \
                                for a sound-only container."
            },
            "fps": {
                "type": "string",
                "description": "Frame rate to deliver at: 30, 60, or a fraction like \
                                30000/1001 (a decimal like 29.97 is refused — write the \
                                fraction). Defaults to the project's own timeline rate; \
                                anything else is conformed from it, nearest frame. Refused \
                                for a sound-only container."
            },
            "range": {
                "type": "string",
                "description": "Render only part of the timeline, in frames: 30:120 covers \
                                frames 30 up to 120, 30: runs to the end, :120 from the \
                                start. Without it the whole timeline is rendered."
            },
            "narration_bands": {
                "type": "boolean",
                "description": "false to leave out the band a narration line not yet \
                                generated draws across the foot of the picture: for a preview \
                                whose captions already carry the words. The line keeps its \
                                place on the timeline, stays silent until generated and still \
                                ducks the music, and the finished job's notes name every line \
                                left out. Default true, every band drawn. Refused for a \
                                sound-only container."
            },
            "loudness": {
                "type": "number",
                "description": "Deliver the soundtrack at this integrated loudness, in LUFS, \
                                e.g. -14: the mix is measured, raised or lowered to it, and \
                                its peaks held under -1 dBTP by a limiter, so a quiet mix \
                                arrives as loud as the feed around it. Between -40 and -5. \
                                Without it the mix is delivered as balanced. The finished \
                                job's notes give the loudness before and after, and the gain \
                                and limiting it took."
            }
        },
        "required": ["project"]
    })
}

/// Ask for a render.
pub(in super::super) async fn render(
    caller: &Caller<'_>,
    arguments: &Value,
) -> Result<Reply, String> {
    let project = project_id(arguments)?;
    let mut shape = arguments.clone();
    if let Some(fields) = shape.as_object_mut() {
        fields.remove("project");
    }
    let mut asked: Ask = serde_json::from_value(shape)
        .map_err(|error| format!("the arguments do not read: {error}"))?;
    let mut settings = Settings::from_ask(&asked)?;
    let toolbox = caller.toolbox;
    // A picture with no size asked for is the project's own frame at delivery
    // size (#1278), as it is over HTTP and on the command line.
    if asked.leaves_size(&settings) {
        let stored = projects::open(&toolbox.pool, caller.user, project)
            .await
            .map_err(|error| error.to_string())?;
        let frame = Resolution::of_project(&stored.document, Budget::Delivery);
        asked.resolution = Some(frame.to_string());
        settings = Settings::from_ask(&asked)?;
    }
    let asked = ask(
        &toolbox.pool,
        &toolbox.queue,
        &toolbox.renders,
        caller.user,
        project,
        settings,
    )
    .await
    .map_err(|error| match error {
        AskError::Invalid(why) => why,
        AskError::Project(ProjectError::NotFound) => "there is no such project of yours".into(),
        other => database(other),
    })?;
    Ok(match asked {
        Asked::Kept(view) => format!(
            "Already rendered, {} bytes: GET {} to download it.",
            view.size,
            view.file()
        ),
        Asked::Queued(job) => format!(
            "Rendering as job {}, {}. Call jobs with job: {} to see when it is done; its \
             result names where to download the file.",
            job.id,
            state(&job),
            job.id
        ),
    }
    .into())
}
