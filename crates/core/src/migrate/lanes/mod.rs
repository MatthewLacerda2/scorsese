//! v54 → v55: one track per property, except where tracks multiply (#1099).

use serde_json::Value;

use crate::keyframe::PropertyPath;

/// v54 → v55: a clip keeps one keyframe track per property that does not
/// stack.
///
/// Up to v54 nothing refused two tracks on one property, and the compositor
/// drew the **last** one wherever both had a value — so the earlier tracks
/// were points nobody saw. This keeps the last and drops the rest, which is
/// the track that renders today: no migrated project changes on screen.
///
/// `volume` is left alone, every track of it: from v55 its tracks multiply
/// ([`PropertyPath::STACKING`]), which is what makes a hand-written fade and a
/// duck both heard. Before, the mixer heard only the first, so a migrated
/// project with both now plays its duck as well — the bug this version fixes,
/// and the one change it makes to how anything sounds.
///
/// Every clip in the document: the timeline's and each group's members.
pub(in crate::migrate) fn one_track_per_property(document: &mut Value) -> Result<(), String> {
    for tracks in lanes(document) {
        for track in tracks.iter_mut() {
            let clips = track.get_mut("clips").and_then(Value::as_array_mut);
            for clip in clips.into_iter().flatten() {
                if let Some(keyframes) = clip.get_mut("keyframes").and_then(Value::as_array_mut) {
                    keep_last(keyframes);
                }
            }
        }
    }
    Ok(())
}

/// The document's timeline tracks, then each group's.
fn lanes(document: &mut Value) -> Vec<&mut Vec<Value>> {
    let Some(fields) = document.as_object_mut() else {
        return Vec::new();
    };
    let mut lanes = Vec::new();
    let (mut timeline, mut assets) = (None, None);
    for (key, value) in fields.iter_mut() {
        match key.as_str() {
            "tracks" => timeline = value.as_array_mut(),
            "assets" => assets = value.as_array_mut(),
            _ => {}
        }
    }
    lanes.extend(timeline);
    let groups = assets
        .into_iter()
        .flatten()
        .filter_map(|asset| asset.get_mut("group")?.get_mut("tracks")?.as_array_mut());
    lanes.extend(groups);
    lanes
}

/// Drops every track a later one on the same property supersedes.
fn keep_last(keyframes: &mut Vec<Value>) {
    let property = |track: &Value| {
        track
            .get("property")
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let mut kept: Vec<Value> = Vec::with_capacity(keyframes.len());
    for track in keyframes.drain(..).rev() {
        let superseded = property(&track).is_some_and(|path| {
            !PropertyPath::new(path.as_str()).stacks()
                && kept
                    .iter()
                    .any(|later| property(later).as_deref() == Some(&path))
        });
        if !superseded {
            kept.push(track);
        }
    }
    kept.reverse();
    *keyframes = kept;
}

#[cfg(test)]
mod tests;
