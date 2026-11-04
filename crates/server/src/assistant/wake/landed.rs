//! The jobs a turn ordered, as the note a woken turn opens with tells of
//! them (#1441): what each was, for which asset, and how it ended.

use serde_json::Value;

use super::is_final;
use crate::jobs::State;

/// A job a turn ordered, as the note tells of it.
#[derive(sqlx::FromRow)]
pub(super) struct Landed {
    pub(super) id: i64,
    kind: String,
    #[sqlx(try_from = "String")]
    state: State,
    payload: Value,
    result: Option<Value>,
    error: Option<String>,
}

impl Landed {
    /// Whether it has stopped for good.
    pub(super) fn ended(&self) -> bool {
        is_final(self.state)
    }

    /// What it was, in a word or two.
    fn noun(&self) -> &str {
        match self.kind.as_str() {
            "veo_shot" => "shot",
            "still_image" | "batch_still" => "still",
            "spoken_line" => "line",
            "music_track" => "music track",
            "sound_effect" => "sound effect",
            "voice_design" => "voice design",
            "voice_keep" => "kept voice",
            other => other,
        }
    }

    /// One line of the note: what it was, for which asset, and how it ended.
    ///
    /// The asset is the one the job's result names, when it has one: a job
    /// queued before a schema bump may have landed on an id the migration
    /// gave it, which its payload does not know.
    fn line(&self) -> String {
        let asset = [self.result.as_ref(), Some(&self.payload)]
            .into_iter()
            .flatten()
            .find_map(|said| said.get("asset").and_then(Value::as_str));
        let what = match asset {
            Some(asset) => format!("{} `{asset}`", self.noun()),
            None => format!("the {}", self.noun()),
        };
        let why = self.error.as_deref().unwrap_or("no reason was given");
        match self.state {
            State::Done => format!("- {what}: made, and in the project or the library."),
            State::Cancelled => format!("- {what}: cancelled, so nothing was charged."),
            State::Stuck => format!(
                "- {what}: stuck, with what was reserved for it held until the operator looks \
                 into it; do not ask for it again. {why}"
            ),
            // The reason says what the failure cost: a refusal says nothing
            // was charged, a billed call that made nothing what it was
            // charged (#1646).
            _ => format!("- {what}: failed. {why}"),
        }
    }
}

/// The server's note on what landed.
pub(super) fn note(landed: &[Landed], woken: bool) -> String {
    let lines: Vec<String> = landed.iter().map(Landed::line).collect();
    let opening = if woken {
        "The person has not written. The server started this turn because the work ordered \
         earlier in this conversation has finished:"
    } else {
        "The work ordered earlier in this conversation has finished:"
    };
    let next = if woken {
        "Carry on with the plan you agreed with the person. If nothing is left to do, say in a \
         line what landed; if something failed, say so plainly and what you suggest."
    } else {
        "Their message follows; take this into account."
    };
    format!("{opening}\n{}\n{next}", lines.join("\n"))
}

/// The line a woken turn shows in place of a prompt: "Continued after the
/// stills landed".
pub(super) fn marker(landed: &[Landed]) -> String {
    let first = landed.first().map(Landed::noun).unwrap_or("generation");
    let what = if landed.iter().all(|job| job.noun() == first) {
        if landed.len() == 1 {
            format!("the {first}")
        } else {
            format!("the {first}s")
        }
    } else {
        "the generations".to_owned()
    };
    format!("Continued after {what} landed")
}
