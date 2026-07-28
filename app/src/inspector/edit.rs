//! Changing the document — and refusing to, when the change would break it.
//!
//! One shape for every edit in this panel: apply it to a *copy* of the project,
//! ask `scorsese-core` whether the copy is still a project, write it to disk,
//! and only then let the window see it. The copy is what makes refusal free —
//! a rejected edit never existed anywhere, so there is nothing to undo, nothing
//! half-applied on screen, and nothing on disk to repair.
//!
//! It is also the reason a value in this panel can be trusted: it is a value
//! that validated and reached the file, which is the same document the CLI and
//! the MCP server read.

use scorsese_core::{Clip, ClipId, Project};

use crate::project::Open;

/// Why a change did not happen.
#[derive(Debug)]
pub(super) struct Refusal {
    /// The clip it was about. Carried so a refusal cannot outlive the selection
    /// that earned it and go on complaining about a clip nobody is looking at.
    pub(super) clip: ClipId,
    /// What was being changed, in the words of the control that tried.
    pub(super) what: String,
    /// Every problem the change would have caused — all of them, which is the
    /// promise validation makes everywhere else in scorsese.
    pub(super) problems: Vec<String>,
}

/// Applies `change` to a clip, saves the project, and keeps the result — or
/// returns every problem, having touched neither the window nor the disk.
pub(super) fn apply(
    open: &mut Open,
    clip: &ClipId,
    change: impl FnOnce(&mut Clip),
) -> Result<(), Vec<String>> {
    let candidate = changed(&open.project, clip, change)?;
    // Saved before it is shown, rather than after. A window holding a change
    // the file has not got is the one divergence this app is not allowed to
    // have — the document on disk is the model.
    candidate
        .save(&open.root)
        .map_err(|problem| vec![problem.to_string()])?;
    open.project = candidate;
    Ok(())
}

/// The project as it would be with `change` applied, or every problem that
/// would stop it being a project at all.
///
/// Cloning the document to try a change on it is not the expense it looks like:
/// a `project.json` is a few hundred clips at the very outside, and an edit here
/// happens at the speed of a hand. What it buys is that the only copy anybody
/// ever sees is one that validated.
fn changed(
    project: &Project,
    clip: &ClipId,
    change: impl FnOnce(&mut Clip),
) -> Result<Project, Vec<String>> {
    let mut candidate = project.clone();
    let target = candidate
        .tracks
        .iter_mut()
        .flat_map(|track| track.clips.iter_mut())
        .find(|found| &found.id == clip)
        .ok_or_else(|| vec![format!("there is no clip `{clip}` in this project")])?;
    change(target);
    candidate.validate().map_err(|errors| {
        errors
            .into_vec()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
    })?;
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector::fixture::project;
    use scorsese_core::{Fit, Frames};

    /// The ordinary case: a legal change comes back as a whole new document,
    /// with the old one left exactly as it was.
    #[test]
    fn a_change_the_project_survives_is_kept() {
        let before = project();
        let after = changed(&before, &ClipId::new("c1"), |clip| clip.fit = Fit::Fill)
            .expect("filling the raster breaks nothing");
        assert_eq!(after.tracks[0].clips[0].fit, Fit::Fill);
        assert_eq!(
            before.tracks[0].clips[0].fit,
            Fit::Fit,
            "the document it was tried against must not have moved"
        );
    }

    /// The rule this module exists for. A clip of no frames is not a clip, so
    /// the edit is refused *before* anything is written rather than saved and
    /// reported later.
    #[test]
    fn a_clip_dragged_to_no_duration_is_refused() {
        let before = project();
        let problems = changed(&before, &ClipId::new("c1"), |clip| {
            clip.duration = Frames::ZERO;
        })
        .expect_err("a clip covering no frame is not a clip");
        assert!(
            problems.iter().any(|problem| problem.contains("c1")),
            "the refusal has to name the clip: {problems:?}"
        );
    }

    /// Two clips fighting over the same instant of a track is the refusal a
    /// person will actually meet, by dragging one into the next.
    #[test]
    fn a_start_that_runs_into_the_next_clip_is_refused() {
        let before = project();
        let problems = changed(&before, &ClipId::new("c1"), |clip| clip.start = Frames(100))
            .expect_err("frames 120–190 are already spoken for");
        assert_eq!(problems.len(), 1, "one overlap, one problem: {problems:?}");

        // And the same move that stops short of it is allowed, so the refusal
        // is about the overlap and not about moving at all.
        assert!(changed(&before, &ClipId::new("c1"), |clip| clip.start = Frames(30)).is_ok());
    }

    /// Every problem, never just the first — a person repairing something
    /// should see the whole job.
    #[test]
    fn a_refusal_lists_every_problem_at_once() {
        let before = project();
        let problems = changed(&before, &ClipId::new("c1"), |clip| {
            clip.start = Frames(100);
            clip.asset = scorsese_core::AssetId::new("nothing");
        })
        .expect_err("an overlap and a dangling reference");
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    /// A selection can outlive the clip it points at. Asked to change one that
    /// is gone, the answer is a refusal rather than a panic or a silent no-op.
    #[test]
    fn a_clip_that_is_not_there_cannot_be_changed() {
        let attempt = changed(&project(), &ClipId::new("gone"), |clip| {
            clip.start = Frames(1);
        });
        assert!(attempt.is_err());
    }
}
