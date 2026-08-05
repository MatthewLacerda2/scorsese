//! Designing the voice, when neither list holds it.
//!
//! The second tool here that spends money, and the shape of it follows from the
//! two ways it is unlike [`generate`](super::generate).
//!
//! **One call, three answers, one charge.** A design is billed once, for the
//! preview text, and answers with three candidates. A client that assumed three
//! charges would quote three times the price and talk somebody out of a feature
//! that costs a cent — so the description says it outright, and `dry_run`
//! answers the same question for free.
//!
//! **It leaves something behind that the project cannot carry.** A kept voice
//! lives in somebody's ElevenLabs account, so the id travels with the project
//! and the voice does not. The description and the seed are written into
//! `designed-voices.json` for exactly that reason, and the reply says so —
//! because a client that does not know this will happily record an id it can
//! never recover.
//!
//! The audition itself is not here. Three samples to play is a window's job;
//! this answers with where they are, so anything that can play a file can.

use scorsese_providers::credentials::{Budget, Provider, Settings, resolve};
use scorsese_providers::spending;
use scorsese_providers::voices::design::{
    Brief, DesignError, Designing, ElevenLabsStudio, Kept, LEDGER_FILE, PASSAGE, PROMPT, design,
    designed, estimate, keep,
};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Designing a voice from a description.
pub(crate) struct VoiceDesign;

impl Tool for VoiceDesign {
    fn name(&self) -> &'static str {
        "voice_design"
    }

    fn description(&self) -> &'static str {
        "Design a new ElevenLabs voice from a description, for when no voice in \
         either list is the one the video needs. Costs money: one call is billed \
         once for the preview text and answers with three candidates to choose \
         between — not three charges — and dry_run quotes it without a key or a \
         request. Pass prompt and text to design; pass keep with a candidate id \
         and name to turn one of them into a real voice_id, which costs nothing \
         more because the candidate was already paid for. Pass list to read back \
         what this project has designed. What keep creates lives in the user's \
         ElevenLabs account rather than in the project, so the description and \
         the seed are recorded in designed-voices.json — a voice that is later \
         deleted, or a project opened under another account, can then be asked \
         for again. Voice cloning from someone's recorded speech is not offered \
         here in any form."
    }

    fn costs(&self) -> Costs {
        Costs::Money
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "prompt": {
                    "type": "string",
                    "description": format!(
                        "What the voice should be like, in a sentence: age, accent, pace, \
                         warmth, what it sounds like it is for. Between {} and {} \
                         characters. Describe a kind of person, never a named one — this \
                         designs a voice, it does not imitate anybody.",
                        PROMPT.start(), PROMPT.end()
                    )
                },
                "text": {
                    "type": "string",
                    "description": format!(
                        "What the three candidates read aloud, between {} and {} \
                         characters. The only thing this call is billed for, so a longer \
                         passage is a better audition and a dearer one. Use a line the \
                         video would actually need — a voice judged on the wrong words is \
                         judged wrongly.",
                        PASSAGE.start(), PASSAGE.end()
                    )
                },
                "seed": {
                    "type": "integer",
                    "description": "Ask for the same candidates again, as far as the vendor \
                                    manages it — best-effort, not a guarantee. Worth passing: \
                                    it is recorded beside the voice and is half of what makes \
                                    a lost one worth attempting again."
                },
                "guidance": {
                    "type": "number",
                    "description": "How literally the candidates follow the description. \
                                    Higher is more literal and less varied. Left out, the \
                                    vendor chooses."
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "Say what the design would cost and stop. Needs no key and \
                                    sends nothing. Worth doing first — this is one of two \
                                    tools here that spends real money."
                },
                "keep": {
                    "type": "string",
                    "description": "A candidate id from a design in this project. Turns it \
                                    into a real voice with a voice_id a narration can name. \
                                    Costs nothing further. Requires name."
                },
                "name": {
                    "type": "string",
                    "description": "What to call the kept voice. The only name it will be \
                                    recognisable by in the user's ElevenLabs account, so it \
                                    should say what the voice is for."
                },
                "list": {
                    "type": "boolean",
                    "description": "Read back every voice this project has designed, with the \
                                    description and seed that made each one. Touches no \
                                    network and spends nothing."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        // Loaded before anything else because everything this writes lands
        // inside the project, and writing into a directory that is not one
        // would scatter samples wherever the client happened to point.
        let project = load(&dir)?;

        if flag(arguments, "list") {
            return Ok(ledger(&dir).into());
        }
        if let Some(chosen) = text(arguments, "keep") {
            let name = text(arguments, "name").ok_or_else(|| {
                String::from(
                    "keep needs a name: it is what the voice will be called in the user's \
                     ElevenLabs account, and the only thing it is recognisable by there.",
                )
            })?;
            return kept(&dir, &chosen, &name);
        }

        let brief = Brief::new(
            &text(arguments, "prompt").unwrap_or_default(),
            &text(arguments, "text").unwrap_or_default(),
            number(arguments, "seed")?,
            arguments.get("guidance").and_then(Value::as_f64),
        )
        .map_err(say)?;

        if flag(arguments, "dry_run") {
            return Ok(estimate(&brief.passage).says().into());
        }

        let key = resolve(Provider::ElevenLabs).map_err(|error| format!("{error}"))?;
        let studio = ElevenLabsStudio::new(&key.secret);
        let budget = Budget::from_settings(
            &Settings::load().unwrap_or_default(),
            spending::so_far(&project, &dir).total(),
        );
        Ok(said(&design(&dir, &studio, &brief, budget).map_err(say)?).into())
    }
}

/// The three candidates, where they are, and what they cost.
fn said(designing: &Designing) -> String {
    let mut lines: Vec<String> = designing
        .session
        .candidates
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                "{}. {}\n   {}",
                index + 1,
                sample.generated_voice_id,
                sample.path
            )
        })
        .collect();
    lines.push(String::new());
    lines.push(if designing.already_paid {
        String::from(
            "Already designed — these three were paid for before and are on disk, so this \
             cost nothing.",
        )
    } else {
        designing.estimate.says()
    });
    lines.push(String::from(
        "The samples are files in the project; play one to choose. Then call this again \
         with keep set to that candidate's id and a name for it.",
    ));
    lines.join("\n")
}

/// Turns one candidate into a voice, and says what was written down.
fn kept(dir: &std::path::Path, chosen: &str, name: &str) -> Result<Reply, String> {
    let Kept { voice, designed } =
        keep(dir, &ElevenLabsStudio::new(&studio_key()?), chosen, name).map_err(say)?;
    let mut said = format!("{}\n\n", voice.says());
    said.push_str(&format!(
        "That voice now exists in the user's ElevenLabs account, which is not inside this \
         project: the id above travels with the project and the voice itself does not. The \
         description and the seed that made it are recorded in {LEDGER_FILE}, so a voice \
         that is later lost can be asked for again."
    ));
    if designed.seed.is_none() {
        said.push_str(
            "\nThis one was designed without a seed, so asking again will not get as close. \
             Pass seed when designing a voice worth keeping.",
        );
    }
    Ok(said.into())
}

/// The key, resolved — split out only so the line above stays readable.
fn studio_key() -> Result<scorsese_providers::credentials::Secret, String> {
    resolve(Provider::ElevenLabs)
        .map(|key| key.secret)
        .map_err(|error| format!("{error}"))
}

/// What this project has designed, and what it came to.
fn ledger(dir: &std::path::Path) -> String {
    let entries = designed(dir);
    if entries.is_empty() {
        return String::from(
            "No voices have been designed in this project. Call this with prompt and text \
             to design three candidates to choose between.",
        );
    }
    let mut lines: Vec<String> = entries
        .iter()
        .map(scorsese_providers::voices::design::Designed::says)
        .collect();
    lines.push(format!(
        "\n{} designed here, about {} spent designing them — our arithmetic over the \
         published rate, never a bill, and counted apart from what the shots cost.",
        entries.len(),
        scorsese_providers::prices::dollars(scorsese_providers::voices::design::spent(dir))
    ));
    lines.join("\n")
}

/// A failure as the sentence a client is shown.
///
/// Every one of these already carries its own advice — which length was wrong,
/// which plan the feature needs, that nothing was created — so nothing is added
/// here. Wrapping them would only put a second, vaguer sentence in front of the
/// useful one.
fn say(error: DesignError) -> String {
    format!("{error}")
}

/// A string argument, blank counting as absent.
fn text(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// An integer argument, refused rather than rounded when it is not one.
fn number(arguments: &Value, name: &str) -> Result<Option<u32>, String> {
    match arguments.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("{name}: {value} is not a whole number a seed can be")),
    }
}

/// A boolean argument, absent meaning false.
fn flag(arguments: &Value, name: &str) -> bool {
    arguments
        .get(name)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}
