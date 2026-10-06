//! Shortening the clips of a just-realised asset to what it came out as.

use std::collections::HashMap;

use crate::asset::AssetId;
use crate::project::Project;
use crate::time::Frames;
use crate::timeline::ClipId;

/// One clip [`fit_to_sources`] shortened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortened {
    /// The clip that now ends sooner.
    pub clip: ClipId,
    /// The asset it shows, whose media came out shorter than the clip.
    pub asset: AssetId,
    /// How long the clip ran before.
    pub was: Frames,
    /// How long it runs now — up to the last frame of its source.
    pub now: Frames,
}

impl Shortened {
    /// The line a reply gives it: `c-narr-2: 225f → 206f, …`, so whoever is
    /// driving the edit knows a gap has opened behind it.
    pub fn says(&self) -> String {
        format!(
            "{}: {} → {}, `{}` came out shorter than the clip; the gap after it is yours to close",
            self.clip, self.was, self.now, self.asset
        )
    }
}

/// Shortens every clip of the `realised` assets that now reaches past the end
/// of its source, so it ends on the source's last frame, and says which.
///
/// **A generation's length is only known once it lands.** A sketch is laid out
/// at whatever length its clip was given, and a reading takes as long as its
/// words do, so the clip over it can outlast what came back. Left alone, the
/// next load refuses the document — `ClipOutlastsSource`, measured the moment
/// the new file was probed — and every narrow tool with it, which turns a
/// routine generation into a rewrite of the whole document (#825). Shortening
/// is the one answer that keeps the cut as it was everywhere else: the clip
/// still starts where it did and still opens where it did in its source, and
/// what follows it is not moved, because retiming a cut is an editing choice
/// for whoever drives the edit.
///
/// A clip that comes out **shorter** than its source is not touched: playing
/// less of a reading than there is is an edit, not a mistake. Nor is a clip
/// whose source opens at or past the new end — no window of it is left to
/// keep, and inventing one is a decision this has no basis for; validation
/// still reports it. Only the named assets are considered, so a clip some
/// other edit left overlong is never quietly repaired here.
pub fn fit_to_sources(project: &mut Project, realised: &[AssetId]) -> Vec<Shortened> {
    let fps = project.timeline_fps;
    let lengths: HashMap<AssetId, Frames> = project
        .assets
        .iter()
        .filter(|asset| realised.contains(&asset.id))
        .filter_map(|asset| Some((asset.id.clone(), asset.length(fps)?)))
        .collect();
    if lengths.is_empty() {
        return Vec::new();
    }
    let groups = project
        .assets
        .iter_mut()
        .filter_map(|asset| asset.group.as_mut())
        .flat_map(|group| &mut group.tracks);
    let mut shortened = Vec::new();
    for track in project.tracks.iter_mut().chain(groups) {
        for clip in &mut track.clips {
            let Some(&available) = lengths.get(&clip.asset) else {
                continue;
            };
            if clip.source_end() <= available || clip.source_in >= available {
                continue;
            }
            let now = Frames(available.get() - clip.source_in.get());
            shortened.push(Shortened {
                clip: clip.id.clone(),
                asset: clip.asset.clone(),
                was: clip.duration,
                now,
            });
            clip.duration = now;
        }
    }
    shortened
}

#[cfg(test)]
mod tests {
    use super::super::fixture::placed;
    use super::*;

    /// The 120-frame clip over `shot`, with the shot measured at `seconds`.
    fn measured(seconds: f64) -> Project {
        let mut project = placed();
        let media = project.assets[0]
            .media
            .as_mut()
            .expect("the fixture is measured");
        media.duration_seconds = Some(seconds);
        project
    }

    fn shot() -> Vec<AssetId> {
        vec![AssetId::new("shot")]
    }

    #[test]
    fn a_clip_that_outlasts_its_new_source_ends_on_its_last_frame() {
        let mut project = measured(3.0);
        assert!(project.validate().is_err(), "the overrun is what #825 saw");
        let shortened = fit_to_sources(&mut project, &shot());
        assert_eq!(
            shortened,
            vec![Shortened {
                clip: ClipId::new("shot"),
                asset: AssetId::new("shot"),
                was: Frames(120),
                now: Frames(90),
            }]
        );
        assert_eq!(project.tracks[0].clips[0].duration, Frames(90));
        assert_eq!(project.tracks[0].clips[0].start, Frames::ZERO);
        project.validate().expect("the document loads again");
        assert_eq!(
            shortened[0].says(),
            "shot: 120f → 90f, `shot` came out shorter than the clip; the gap after it is \
             yours to close"
        );
    }

    #[test]
    fn the_window_keeps_its_in_point() {
        let mut project = measured(4.0);
        let clip = &mut project.tracks[0].clips[0];
        (clip.source_in, clip.duration) = (Frames(30), Frames(90));
        project.assets[0]
            .media
            .as_mut()
            .expect("measured")
            .duration_seconds = Some(3.0);
        let shortened = fit_to_sources(&mut project, &shot());
        assert_eq!(shortened[0].now, Frames(60));
        assert_eq!(project.tracks[0].clips[0].source_in, Frames(30));
        project.validate().expect("the document loads again");
    }

    #[test]
    fn a_source_longer_than_its_clip_is_left_alone() {
        let mut project = measured(6.0);
        assert!(fit_to_sources(&mut project, &shot()).is_empty());
        assert_eq!(project.tracks[0].clips[0].duration, Frames(120));
    }

    #[test]
    fn only_the_named_assets_are_repaired() {
        let mut project = measured(3.0);
        assert!(fit_to_sources(&mut project, &[AssetId::new("other")]).is_empty());
        assert_eq!(project.tracks[0].clips[0].duration, Frames(120));
    }

    #[test]
    fn a_window_opening_past_the_new_end_is_not_invented() {
        let mut project = measured(4.0);
        let clip = &mut project.tracks[0].clips[0];
        (clip.source_in, clip.duration) = (Frames(100), Frames(20));
        project.assets[0]
            .media
            .as_mut()
            .expect("measured")
            .duration_seconds = Some(3.0);
        assert!(fit_to_sources(&mut project, &shot()).is_empty());
        assert_eq!(project.tracks[0].clips[0].duration, Frames(20));
    }
}
