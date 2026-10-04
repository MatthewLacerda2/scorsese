//! A job in words: the line `jobs` answers with, and what a render is doing.
//!
//! The line has the web's shape — `job 3 (render): running — …` — because an
//! assistant that has used one surface reads the other by habit. The number
//! comes with the phase beside it, always: [`Reading::percent`] counts frames
//! only, so it sits at `0` while the sound is mixed and at `99` while the file
//! is finished, and a bare number there reads as a render that is stuck.

use scorsese_render::{Phase, Reading};

use super::{Job, State};

/// One job as a line.
pub(crate) fn line(job: &Job) -> String {
    let head = format!("job {} (render)", job.id);
    match job.state() {
        State::Running(reading) => format!(
            "{head}: running, {}% — {}, writing {}",
            reading.percent(),
            doing(reading),
            job.out
        ),
        State::Done(said) => format!("{head}: done — {said}"),
        State::Cancelled(why) => format!("{head}: cancelled — {why}"),
        State::Failed(why) => format!("{head}: failed — {why}"),
    }
}

/// What a render is doing at `reading`, in a few words.
pub(crate) fn doing(reading: Reading) -> String {
    match reading.phase {
        Phase::Waiting => "starting".to_owned(),
        Phase::Preparing => "preparing: probing the media and planning the timeline".to_owned(),
        Phase::Mixing => "mixing the sound".to_owned(),
        Phase::Drawing => format!("drawing frame {} of {}", reading.done, reading.of),
        Phase::Finishing if reading.of == 0 => "encoding the sound".to_owned(),
        Phase::Finishing => "finishing the file: every frame is in".to_owned(),
        Phase::Done => "done".to_owned(),
    }
}
