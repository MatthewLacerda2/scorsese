//! Taking an asset or a lane out again — and the clips that go with it, only
//! once they are named.
//!
//! Both verbs refuse a call whose `clips` is not exactly the clips that would
//! be lost, and the refusal lists them. That is the confirmation (#396): an
//! assistant reads the list back to whoever it is working for, and only then
//! calls again with it. Nothing destructive happens on one sentence.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use scorsese_core::{AssetId, ClipId, TrackId, authoring};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::{refused, save};
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Remove an asset and the clips showing it.
pub(crate) struct AssetRemove;

/// Remove a track and the clips on it.
pub(crate) struct TrackRemove;

/// What `asset_remove` takes.
#[derive(Deserialize, JsonSchema)]
struct AssetArguments {
    project: ProjectDir,
    /// Id of the asset to remove, as project_assets and project_read show it.
    asset: Name,
    #[serde(default, deserialize_with = "clip_ids")]
    #[schemars(with = "Vec<String>", description = clips_described("every clip that shows the asset"))]
    clips: BTreeSet<ClipId>,
}

impl args::Arguments for AssetArguments {
    const REQUIRED: Required = &[("asset", "the id of the asset to remove")];
}

/// What `track_remove` takes.
#[derive(Deserialize, JsonSchema)]
struct TrackArguments {
    project: ProjectDir,
    /// Id of the track to remove — `v2`, `a1` — as project_describe and
    /// project_read show it.
    track: Name,
    #[serde(default, deserialize_with = "clip_ids")]
    #[schemars(with = "Vec<String>", description = clips_described("every clip on the track"))]
    clips: BTreeSet<ClipId>,
}

impl args::Arguments for TrackArguments {
    const REQUIRED: Required = &[("track", "the id of the track to remove")];
}

impl Tool for AssetRemove {
    fn name(&self) -> &'static str {
        "asset_remove"
    }

    fn description(&self) -> &'static str {
        "Remove an asset from the project, and DESTROY every clip that shows it — \
         on the timeline and inside groups. The clips that go have to be named: \
         `clips` must list exactly the ids of the clips showing the asset, no \
         more and no fewer, and any other list is refused with the right one in \
         the refusal. So call it once without `clips` (or with an empty list) to \
         learn which clips would go, tell the user which those are, and call \
         again with that list only once they have agreed. An asset nothing shows \
         goes with an empty list. All or nothing: a removal that would leave \
         something pointing at nothing — an arrow attached to one of the clips, \
         a generated shot whose first frame is this image — is refused and \
         removes nothing. A still an image sequence plays is refused whatever \
         is named: change the sequence's stills with `sequence`, or remove the \
         sequence, first. The gaps the clips leave stay open. The asset's file \
         stays on disk; only the table entry goes. To take clips off the \
         timeline and keep the asset, use clip_remove."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<AssetArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: AssetArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let asset = AssetId::new(arguments.asset.as_str());
        let removal =
            authoring::remove_asset(&mut project, &asset, &arguments.clips).map_err(refused)?;
        save(&project, dir)?;
        let went: Vec<String> = removal
            .clips
            .iter()
            .map(|gone| format!("`{}` from `{}`", gone.clip.id, gone.track))
            .collect();
        let said = if went.is_empty() {
            format!("Removed `{asset}`; no clip showed it.")
        } else {
            format!(
                "Removed `{asset}` and the clips that showed it: {}. The gaps they \
                 left are still there; nothing else moved.",
                went.join(", ")
            )
        };
        Ok(said.into())
    }
}

impl Tool for TrackRemove {
    fn name(&self) -> &'static str {
        "track_remove"
    }

    fn description(&self) -> &'static str {
        "Remove a track — a lane on the timeline or inside a group — and DESTROY \
         every clip on it. The clips that go have to be named: `clips` must list \
         exactly the ids of the clips on the track, and any other list is \
         refused with the right one in the refusal. So call it once without \
         `clips` to learn what is on the lane, tell the user, and call again with \
         that list only once they have agreed. An empty track goes with an empty \
         list. The assets those clips showed stay in the project. All or \
         nothing: a removal that would leave an arrow attached to nothing, or a \
         group with nothing in it, is refused and removes nothing. Moving a clip \
         to another lane first is clip_move."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<TrackArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: TrackArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let track = TrackId::new(arguments.track.as_str());
        let lane =
            authoring::remove_track(&mut project, &track, &arguments.clips).map_err(refused)?;
        save(&project, dir)?;
        let went: Vec<String> = lane
            .clips
            .iter()
            .map(|clip| format!("`{}`", clip.id))
            .collect();
        let said = if went.is_empty() {
            format!("Removed `{track}`; it was empty.")
        } else {
            format!(
                "Removed `{track}` and the clips on it: {}. Their assets are still in \
                 the project.",
                went.join(", ")
            )
        };
        Ok(said.into())
    }
}

/// `clips`, described for what it confirms.
fn clips_described(which: &str) -> String {
    format!(
        "The ids of {which} — exactly those, which are removed with it. This list \
         is the confirmation: leave it out to be told which clips they are, and \
         send it only once the user has agreed to lose them."
    )
}

/// The clips named, as a set — a repeated id is one clip. Absent or `null` is
/// empty: the question "what would go?" that the refusal answers.
///
/// Read as one value rather than entry by entry, so a list with a number in it
/// is refused as `clips` — the argument the caller wrote — and not as its
/// third entry.
fn clip_ids<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BTreeSet<ClipId>, D::Error> {
    use serde::de::Error as _;
    let wrong = || D::Error::custom("a list of clip ids, each a string");
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(BTreeSet::new()),
        Value::Array(named) => named
            .iter()
            .map(|id| id.as_str().map(ClipId::new).ok_or_else(wrong))
            .collect(),
        _ => Err(wrong()),
    }
}
