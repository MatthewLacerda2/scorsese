//! Taking placed clips off the timeline.

use std::collections::BTreeSet;

use crate::project::Project;
use crate::timeline::{Clip, ClipId, TrackId};
use crate::validate::ValidationErrors;

/// Why nothing was removed. Nothing is ever partly removed either.
#[derive(Debug, thiserror::Error)]
pub enum RemoveError {
    /// No clip was named. A caller that meant to remove something and named
    /// nothing has made a mistake, and a cheerful "done" would hide it.
    #[error("no clip was named — say which clips to remove")]
    Nothing,
    /// One of the ids named no clip. The whole request is refused, not the rest
    /// carried out: a mistyped id in a list of five is a list the caller has
    /// not read back, and removing the other four is a guess at what they meant.
    #[error("no clip in this project is called `{clip}` — nothing was removed")]
    NoSuchClip {
        /// The first id that matched nothing.
        clip: ClipId,
    },
    /// The result was not a document that loads — in practice an arrow still
    /// attached to one of the clips, which would be left pointing at nothing.
    #[error(transparent)]
    Refused(#[from] ValidationErrors),
}

/// A clip taken off the timeline, and the track it was on.
#[derive(Debug, Clone, PartialEq)]
pub struct Removed {
    /// Where it was.
    pub track: TrackId,
    /// The clip as it was, so a reply can say what went.
    pub clip: Clip,
}

/// Takes every named clip off its track, and hands back what was removed.
///
/// **A clip, never an asset.** A clip is a reference, so removing one loses
/// nothing but the placement: the asset stays in the table and its file on
/// disk, ready to be placed again. What happens to an asset nobody places is a
/// different question with a different answer ([`crate::pool`]).
///
/// **The gap stays.** Nothing after a removed clip closes up behind it; a
/// ripple would move clips nobody named, on this track and in sync with every
/// other, and that is a judgement a delete key does not get to make.
pub fn remove(project: &mut Project, ids: &BTreeSet<ClipId>) -> Result<Vec<Removed>, RemoveError> {
    if ids.is_empty() {
        return Err(RemoveError::Nothing);
    }
    if let Some(missing) = ids
        .iter()
        .find(|id| !project.clips().any(|(_, clip)| &clip.id == *id))
    {
        return Err(RemoveError::NoSuchClip {
            clip: missing.clone(),
        });
    }

    let mut proposed = project.clone();
    let mut removed = Vec::new();
    for track in &mut proposed.tracks {
        let (gone, kept) = std::mem::take(&mut track.clips)
            .into_iter()
            .partition(|clip| ids.contains(&clip.id));
        track.clips = kept;
        removed.extend(gone.into_iter().map(|clip| Removed {
            track: track.id.clone(),
            clip,
        }));
    }

    proposed.validate()?;
    *project = proposed;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::super::fixture::placed;
    use super::*;

    fn named(ids: &[&str]) -> BTreeSet<ClipId> {
        ids.iter().map(|id| ClipId::new(*id)).collect()
    }

    #[test]
    fn a_removed_clip_is_gone_and_its_asset_is_not() {
        let mut project = placed();
        let removed = remove(&mut project, &named(&["shot"])).expect("it is there");
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].track.as_str(), "v1");
        assert_eq!(project.clips().count(), 0);
        assert_eq!(project.assets.len(), 1, "the asset stays in the table");
    }

    #[test]
    fn one_unknown_id_removes_nothing_at_all() {
        let mut project = placed();
        let before = project.clone();
        let error = remove(&mut project, &named(&["shot", "nope"])).expect_err("nope is not there");
        assert!(error.to_string().contains("`nope`"), "got {error}");
        assert_eq!(project, before, "nothing was removed");
    }

    #[test]
    fn naming_nothing_says_so() {
        let mut project = placed();
        let error = remove(&mut project, &BTreeSet::new()).expect_err("nothing was named");
        assert!(matches!(error, RemoveError::Nothing), "got {error}");
    }

    /// An arrow attached to a clip is the one thing in the document that names
    /// a clip, and removing the clip under it would leave it pointing at
    /// nothing — so the removal is refused, and says which arrow.
    #[test]
    fn a_clip_an_arrow_is_attached_to_stays() {
        let mut project = Project::from_json(
            r##"{
              "schema_version": 46, "name": "T", "timeline_fps": { "num": 30, "den": 1 },
              "assets": [
                { "id": "box", "kind": "shape", "shape": {
                  "geometry": { "rectangle": { "width": 0.2, "height": 0.1 } },
                  "fill": "#ffffffff" } },
                { "id": "arrow", "kind": "shape", "shape": {
                  "geometry": { "arrow": {
                    "from": { "attach": { "clip": "b", "side": "right" } },
                    "to": { "x": 0.5, "y": 0.5 } } },
                  "stroke": "#ffffffff", "stroke_width": 0.004 } }
              ],
              "tracks": [ { "id": "v1", "kind": "video", "clips": [
                { "id": "b", "asset": "box", "start": 0, "duration": 30 } ] } ]
            }"##,
        )
        .expect("the fixture is a project");
        let before = project.clone();
        let error = remove(&mut project, &named(&["b"])).expect_err("the arrow needs it");
        assert!(matches!(error, RemoveError::Refused(_)), "got {error}");
        assert!(error.to_string().contains("arrow"), "got {error}");
        assert_eq!(project, before, "nothing was removed");
    }
}
