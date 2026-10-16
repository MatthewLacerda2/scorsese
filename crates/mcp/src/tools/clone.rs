//! Cloning a voice from the project's own recordings (#1263): *narrate in my
//! own voice*.
//!
//! The sibling of [`design`](super::design), and unlike it **free**: creating
//! an Instant Voice Clone is not on ElevenLabs' list of what credits pay for,
//! so there is no quote and no token — what the voice narrates is charged as
//! any line is. What it takes is one of the account's custom voice slots, and
//! `forget` gives it back.
//!
//! **Consent is an argument**, required and asked first: the vendor's API
//! takes no such field, so the tool asks where the vendor's own app does, and
//! its description says the rule and links the terms.

use schemars::JsonSchema;
use scorsese_core::AssetId;
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::voices::clone::{
    Ask, CloneError, Done, FORGOTTEN, LEDGER_FILE, clone, cloned, forget,
};
use scorsese_providers::voices::design::ElevenLabsStudio;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Cloning a voice from recordings.
pub(crate) struct VoiceClone;

/// What `voice_clone` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// The ids of the imported audio assets to clone from: recordings of the
    /// one person whose voice it is, speaking clearly, with no music, reverb
    /// or background noise. About one to two minutes in all is what
    /// ElevenLabs recommends; more than three adds little. Import the
    /// recordings first.
    assets: Option<Vec<String>>,
    /// What to call the voice. The only name it will be recognisable by in
    /// the user's ElevenLabs account, so say whose voice it is.
    name: Option<String>,
    /// What the voice is, in a sentence, kept with it at the vendor.
    /// Optional.
    description: Option<String>,
    /// Ask ElevenLabs to clean background noise out of the recordings first.
    /// Only for recordings that have some: on clean ones, the vendor warns, it
    /// can lower the quality.
    #[serde(default)]
    remove_background_noise: bool,
    /// Set to true only once the person speaking in the recordings has said
    /// the voice is their own, or that they have permission from its owner to
    /// clone it. Ask them in those words; never assume it. Without it nothing
    /// is read or sent.
    #[serde(default)]
    consent: bool,
    /// Read back every voice this project has cloned, with the recordings
    /// each was made from. Touches no network.
    #[serde(default)]
    list: bool,
    /// The voice_id of a voice this project cloned, exactly as list shows it.
    /// Deletes that voice from the user's ElevenLabs account, freeing the
    /// voice slot it takes. Cannot be undone at the vendor, but the
    /// recordings stay in the project, so cloning them again makes a voice
    /// like it.
    forget: Option<String>,
}

impl args::Arguments for Arguments {}

impl Tool for VoiceClone {
    fn name(&self) -> &'static str {
        "voice_clone"
    }

    fn description(&self) -> &'static str {
        "Clone a voice from recordings already in the project, so a video can be narrated in \
         the user's own voice: ElevenLabs' Instant Voice Cloning, on its Starter plan and \
         above. Clone only the user's own voice, or the voice of someone who has given them \
         permission to clone it — ask them in those words before calling, and pass consent \
         only once they have said yes. A cloned voice is bound by ElevenLabs' terms of use \
         (https://elevenlabs.io/terms-of-use) and its prohibited use policy \
         (https://elevenlabs.io/use-policy): never one to impersonate, mislead or harm \
         anyone. Pass assets (imported audio, about one to two minutes of clear speech in \
         all), a name and consent; the answer is a voice_id a narration names like any \
         other. Creating it costs nothing — what it narrates is charged as any line is — \
         but it takes one of the account's custom voice slots. Pass list to read back what \
         this project has cloned, and forget with a voice_id from that list to delete it \
         from the account and free its slot. The voice lives in the user's ElevenLabs \
         account rather than in the project, so the recordings it was made from are \
         recorded in cloned-voices.json."
    }

    fn costs(&self) -> Costs {
        Costs::Request
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let project = load(dir)?;

        if arguments.list {
            return Ok(ledger(dir).into());
        }
        if let Some(voice) = args::given(arguments.forget.as_deref()) {
            let forgotten = forget(dir, &studio()?, voice).map_err(say)?;
            return Ok(format!("{}\n\n{FORGOTTEN}", forgotten.says()).into());
        }

        let ask = Ask {
            assets: arguments
                .assets
                .unwrap_or_default()
                .iter()
                .map(|id| AssetId::new(id.trim()))
                .collect(),
            name: arguments.name.unwrap_or_default(),
            description: args::given(arguments.description.as_deref()).map(ToOwned::to_owned),
            remove_background_noise: arguments.remove_background_noise,
            consent: arguments.consent,
        };
        // Asked before a key is looked for, so a refusal for consent is the
        // first thing said, whatever else is missing.
        scorsese_providers::voices::clone::check(ask.consent, &ask.name).map_err(say)?;
        Ok(said(&clone(dir, &project, &studio()?, &ask).map_err(say)?).into())
    }
}

/// The voice, and what was written down.
fn said(done: &Done) -> String {
    let mut lines = vec![
        done.voice.says(),
        String::new(),
        format!(
            "That voice now exists in the user's ElevenLabs account, which is not inside this \
             project: the id above travels with the project and the voice itself does not. \
             The recordings it was cloned from are recorded in {LEDGER_FILE}, so a voice that \
             is later lost can be cloned again from them. Creating it cost nothing; it takes \
             one of the account's custom voice slots until it is forgotten."
        ),
    ];
    if done.cloned.requires_verification {
        lines.push(String::from(
            "ElevenLabs says this voice needs verifying before it narrates: open it among the \
             voices at elevenlabs.io and follow its steps.",
        ));
    }
    lines.extend(done.advice.clone());
    lines.join("\n")
}

/// The studio, keyed from the one resolver.
fn studio() -> Result<ElevenLabsStudio, String> {
    resolve(Provider::ElevenLabs)
        .map(|key| ElevenLabsStudio::new(&key.secret))
        .map_err(|error| format!("{error}"))
}

/// What this project has cloned.
fn ledger(dir: &std::path::Path) -> String {
    let entries = cloned(dir);
    if entries.is_empty() {
        return String::from(
            "No voices have been cloned in this project. Call this with assets, a name and \
             consent to clone one from the project's recordings.",
        );
    }
    entries
        .iter()
        .map(scorsese_providers::voices::clone::Cloned::says)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A failure as the sentence a client is shown — each already carries its
/// own advice.
fn say(error: CloneError) -> String {
    format!("{error}")
}
