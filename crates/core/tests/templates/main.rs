//! Templates (#546): lifting clips out of one edit and copying them into
//! another, asserted on the documents that come out.

mod extract;
mod groups;
mod insert;
mod mattes;

use std::collections::BTreeSet;

use scorsese_core::{ClipId, Project, template};

/// An episode with an intro worth keeping.
///
/// ```text
/// v1          [--- intro (file) ---]   [hero: queued brief]
/// v2               [- title -]           [box]
/// v3                                     [arrow → box]
/// a1          [- sting (file) -]
///             90   120   180   210     300  330
/// ```
pub(crate) const EPISODE: &str = r##"{
  "schema_version": 44, "name": "Episode 1", "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "intro", "kind": "video", "path": "assets/intro.mp4",
      "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "media": { "duration_seconds": 10.0 } },
    { "id": "sting", "kind": "audio", "path": "assets/sting.wav",
      "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "media": { "duration_seconds": 5.0 } },
    { "id": "title", "kind": "text", "text": "EPISODE" },
    { "id": "face", "kind": "image", "path": "assets/face.png",
      "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" },
    { "id": "hero", "kind": "generated_video", "state": "queued", "operation": "op/1",
      "prompt": "she turns to the camera", "video": { "first_image": "face" } },
    { "id": "box", "kind": "shape", "shape": {
        "geometry": { "rectangle": { "width": 0.2, "height": 0.1 } }, "fill": "#1e3a8aff" } },
    { "id": "arrow", "kind": "shape", "shape": {
        "geometry": { "arrow": { "from": { "x": 0.1, "y": 0.1 },
                                 "to": { "attach": { "clip": "c-box", "side": "left" } } } },
        "stroke": "#000000ff" } }
  ],
  "tracks": [
    { "id": "v1", "kind": "video", "name": "Footage", "clips": [
      { "id": "c-intro", "asset": "intro", "start": 90, "duration": 120 },
      { "id": "c-hero", "asset": "hero", "start": 300, "duration": 60 } ] },
    { "id": "v2", "kind": "video", "clips": [
      { "id": "c-title", "asset": "title", "start": 120, "duration": 60,
        "keyframes": [ { "property": "opacity", "keyframes": [
          { "t": 0, "value": 0.0 }, { "t": 15, "value": 1.0 } ] } ] },
      { "id": "c-box", "asset": "box", "start": 300, "duration": 30 } ] },
    { "id": "v3", "kind": "video", "clips": [
      { "id": "c-arrow", "asset": "arrow", "start": 300, "duration": 30 } ] },
    { "id": "a1", "kind": "audio", "clips": [
      { "id": "c-sting", "asset": "sting", "start": 90, "duration": 60 } ] }
  ]
}"##;

pub(crate) fn episode() -> Project {
    Project::from_json(EPISODE).expect("the fixture is a project")
}

pub(crate) fn ids(names: &[&str]) -> BTreeSet<ClipId> {
    names.iter().map(|name| ClipId::new(*name)).collect()
}

/// The intro: its footage, its title over it, and its sting.
pub(crate) fn intro() -> Project {
    template::extract(
        &episode(),
        &ids(&["c-intro", "c-title", "c-sting"]),
        "Intro",
    )
    .expect("the intro stands alone")
}

/// A clip's id, track and start, in the project it is in.
pub(crate) fn placed(project: &Project, clip: &str) -> (String, u64) {
    project
        .clips()
        .find(|(_, c)| c.id.as_str() == clip)
        .map(|(track, c)| (track.id.to_string(), c.start.get()))
        .unwrap_or_else(|| panic!("no clip {clip}"))
}
