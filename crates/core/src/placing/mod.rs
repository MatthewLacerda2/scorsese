//! Putting a clip on a track, and moving or trimming one already there.
//!
//! The commonest edit there is, and the one whose arithmetic is easiest to get
//! quietly wrong. A shot is described in seconds — *two seconds in, run it to
//! sixteen, start it at forty-eight* — and the document takes whole frames on
//! the project's grid, so every placement is a small conversion done by hand.
//! An off-by-half-a-second window and an off-by-one-frame J-cut both validate
//! perfectly and are only ever caught by watching the render.
//!
//! So the conversion happens once, above this line, and the two operations here
//! are the only way a caller has to reach the document:
//!
//! - [`place`] writes a new clip onto an existing track.
//! - [`trim`] changes where a placed clip starts, how long it runs, or where in
//!   its source it opens.
//!
//! **Both are all-or-nothing.** The change is worked out on a copy and only a
//! copy [`Project::validate`](crate::Project::validate) accepts becomes the
//! document, exactly as [`crate::pacing`] does it. A clip that would overlap its
//! neighbour, or reach past the end of the media it shows, leaves the project
//! byte-for-byte as it was — which is what lets a caller keep asking until it
//! asks for something possible.
//!
//! **This is placement in *time*.** Where a clip's picture lands in the frame —
//! [`Fit`](crate::Fit), [`Crop`](crate::Crop), [`Anchor`](crate::Anchor) — is a
//! different question with a different set of fields, and nothing here touches
//! it.

mod place;
mod trim;

pub use place::{PlaceError, Placement, place};
pub use trim::{Trim, TrimError, trim};

/// The document both halves are tested against, so a placement and a trim are
/// checked against the same four seconds of media rather than two fixtures that
/// drift apart.
#[cfg(test)]
mod fixture {
    use crate::asset::AssetId;
    use crate::project::Project;
    use crate::time::Frames;
    use crate::timeline::TrackId;

    const DOCUMENT: &str = r#"{
      "schema_version": 31,
      "name": "T",
      "timeline_fps": { "num": 30, "den": 1 },
      "assets": [
        { "id": "shot", "kind": "video", "path": "assets/shot.mp4",
          "media": { "duration_seconds": 4.0 } }
      ],
      "tracks": [ { "id": "v1", "kind": "video", "clips": [] } ]
    }"#;

    /// A 4-second shot on a 30fps grid — 120 frames of media — and an empty
    /// video track to put it on.
    pub(super) fn project() -> Project {
        Project::from_json(DOCUMENT).expect("the fixture is a project")
    }

    /// The same, with the whole shot placed at the head of `v1` as clip `shot`.
    pub(super) fn placed() -> Project {
        let mut project = project();
        super::place(
            &mut project,
            &super::Placement {
                asset: AssetId::new("shot"),
                track: TrackId::new("v1"),
                start: Frames::ZERO,
                duration: None,
                source_in: Frames::ZERO,
                id: None,
            },
        )
        .expect("the shot fits");
        project
    }
}
