//! Scaling where clips sit in time, about an instant that stays put.

use std::collections::BTreeSet;

use scorsese_core::{ClipId, Paced, pacing};
use serde_json::Value;

use super::number;
use crate::tools::inspect::load;
use crate::tools::{Reply, Tool, project_dir, project_property};

/// Spread a run of clips out, or close it up, about one instant.
pub(crate) struct ScalePacing;

impl Tool for ScalePacing {
    fn name(&self) -> &'static str {
        "scale_pacing"
    }

    fn description(&self) -> &'static str {
        "Move some clips toward or away from one instant, all by the same \
         factor — the operation for pacing. A montage cut to a song that turned \
         out to be 8% slower, a run of titles that all need a beat more air: \
         those are the same clips spread differently, and doing it by hand means \
         re-deriving every start. Clips with no media behind them (titles, \
         stills, colours, briefs nobody has generated) have their DURATION \
         scaled too, so the rhythm scales rather than the gaps merely closing. \
         Clips backed by a real file move and keep their length — shortening one \
         would mean either showing less of it or playing it faster, and those \
         are two different edits, so this refuses to pick. The reply says which \
         clips those were. Nothing is changed at all unless the whole scale \
         lands in a document that still loads."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "clips": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Ids of the clips to move. Everything else on \
                                    the timeline stays exactly where it is, which \
                                    is what bounds how far this can go: a scaled \
                                    clip may not come to overlap one that was left \
                                    alone."
                },
                "factor": {
                    "type": "number",
                    "description": "How much to stretch time by. Greater than 1 \
                                    spreads the clips out — 1.2 makes the cut 20% \
                                    slower — and less than 1 closes it up. Must be \
                                    positive."
                },
                "about_seconds": {
                    "type": "number",
                    "description": "The instant the scale is pinned to: the one \
                                    moment that does not move, with everything \
                                    before it drawing in and everything after it \
                                    pushing out. Default 0, which scales the whole \
                                    cut from its beginning. Rounded to a whole \
                                    frame on the project's grid."
                }
            },
            "required": ["project", "clips", "factor"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let clips = clip_ids(arguments)?;
        let factor = arguments
            .get("factor")
            .and_then(Value::as_f64)
            .ok_or_else(|| "`factor` is required: how much to stretch time by".to_owned())?;

        let seconds = number(arguments, "about_seconds", 0.0);
        if seconds < 0.0 {
            return Err("`about_seconds` cannot be before the start of the timeline".to_owned());
        }
        let about = project.timeline_fps.frames(seconds);
        let paced = pacing::scale(&mut project, &clips, about, factor)
            .map_err(|error| format!("{error} — nothing was changed"))?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let clips = if paced.moved == 1 { "clip" } else { "clips" };
        Ok(format!(
            "{} {clips} scaled ×{factor} about {seconds}s (frame {}).{}",
            paced.moved,
            about.get(),
            kept(&paced)
        )
        .into())
    }
}

/// The clips to move, as a set — a repeated id is one clip, not two moves.
fn clip_ids(arguments: &Value) -> Result<BTreeSet<ClipId>, String> {
    let named = arguments
        .get("clips")
        .and_then(Value::as_array)
        .ok_or_else(|| "`clips` is required: the ids of the clips to move".to_owned())?;
    let ids: BTreeSet<ClipId> = named
        .iter()
        .filter_map(Value::as_str)
        .map(ClipId::new)
        .collect();
    if ids.is_empty() {
        return Err("`clips` must name at least one clip".to_owned());
    }
    Ok(ids)
}

/// What the scale declined to stretch, and why.
///
/// Always said when there is something to say. This is the one place the
/// operation does other than what was asked, and a caller who is not told is a
/// caller who finds out by rendering.
fn kept(paced: &Paced) -> String {
    if paced.kept.is_empty() {
        return String::new();
    }
    let names: Vec<String> = paced.kept.iter().map(|clip| format!("`{clip}`")).collect();
    let (it, its) = if names.len() == 1 {
        ("it", "its")
    } else {
        ("them", "their")
    };
    format!(
        " {} moved but kept {its} length, because the media behind {it} has one \
         of {its} own — shortening that would mean either showing less of the \
         source or playing it faster, which are two different edits.",
        names.join(", ")
    )
}
