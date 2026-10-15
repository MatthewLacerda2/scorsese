//! A lane to put clips on.

use schemars::JsonSchema;
use scorsese_core::{Lane, Layer, TrackId, TrackKind, authoring};
use serde::Deserialize;
use serde_json::Value;

use super::{maybe, refused, save};
use crate::tools::args::{self, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Add a track.
pub(crate) struct TrackNew;

/// What a lane carries, as the call names it.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Video,
    Audio,
}

/// What `track_new` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// What the lane carries. `video` takes anything visible — footage, a
    /// title, a colour, a shape, an icon, a generated shot; `audio` takes
    /// sound. An asset on the wrong kind of track is refused.
    kind: Kind,
    /// What to call it. Optional: without it the lane is numbered from the
    /// lowest free `v`/`a` number, and the reply says which id it wrote. An id
    /// already in use is refused.
    track: Option<String>,
    /// What a human calls the lane, shown in a lane header. Cosmetic — nothing
    /// reads it and nothing renders it.
    name: Option<String>,
    /// Why this lane is here, for whoever reads the project next: why the
    /// music sits under the narration, which one gets ducked. Never rendered,
    /// under any setting.
    note: Option<String>,
    /// A video track's id to put this one directly UNDER, instead of on top of
    /// everything — a logo beneath the captions. Video only; not with `over`.
    under: Option<String>,
    /// A video track's id to put this one directly OVER, instead of on top of
    /// everything. Video only; not with `under`.
    over: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[("kind", "video or audio")];
}

/// Where `under` or `over` puts a lane, from whichever of the two was given —
/// `None` for neither, and a refusal for both.
pub(super) fn layer(under: Option<&str>, over: Option<&str>) -> Result<Option<Layer>, String> {
    match (maybe(under), maybe(over)) {
        (Some(_), Some(_)) => Err("give `under` or `over`, not both".to_owned()),
        (Some(under), None) => Ok(Some(Layer::Under(TrackId::new(under)))),
        (None, Some(over)) => Ok(Some(Layer::Over(TrackId::new(over)))),
        (None, None) => Ok(None),
    }
}

/// Where a video lane now sits, as a reply says it.
pub(super) fn sits(layer: Option<&Layer>) -> String {
    match layer {
        Some(Layer::Under(beside)) => format!("directly under `{beside}`"),
        Some(Layer::Over(beside)) => format!("directly over `{beside}`"),
        None => "over every video track already there".to_owned(),
    }
}

impl Tool for TrackNew {
    fn name(&self) -> &'static str {
        "track_new"
    }

    fn description(&self) -> &'static str {
        "Add a track — a lane for clips, carrying either picture or sound. This \
         is the answer to the commonest refusal in the whole tool: clips on one \
         track may not overlap, so anything meant to be on screen at the same \
         time as something else needs a lane of its own. A new video track is \
         appended, which means it composites OVER everything already there — \
         which is what a caption above footage wants. Audio tracks are unordered \
         among themselves; everything audible is summed. Left unnamed it is \
         numbered `v1`, `v2`, `a1`, `a2` from the lowest free number. To put a \
         video track lower in the stack — a logo under the captions — name the \
         track it goes `under` or `over`; track_move moves one already there."
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
        let mut project = load(dir)?;
        let kind = match arguments.kind {
            Kind::Video => TrackKind::Video,
            Kind::Audio => TrackKind::Audio,
        };
        let layer = layer(arguments.under.as_deref(), arguments.over.as_deref())?;
        let lane = Lane {
            kind,
            layer: layer.clone(),
            id: maybe(arguments.track.as_deref()).map(scorsese_core::TrackId::new),
            name: maybe(arguments.name.as_deref()),
            note: maybe(arguments.note.as_deref()),
        };
        let id = authoring::add_track(&mut project, &lane).map_err(refused)?;
        save(&project, dir)?;
        let (says, where_it_sits) = match kind {
            TrackKind::Video => ("video", sits(layer.as_ref())),
            TrackKind::Audio => ("audio", "mixed with every other audio track".to_owned()),
        };
        Ok(format!("`{id}` — a {says} track, {where_it_sits}. place_clip fills it.").into())
    }
}
