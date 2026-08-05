//! Voice Design: a voice from a sentence, in two calls.
//!
//! The vendor splits this in half, and the split is the whole shape of the
//! feature:
//!
//! - **`POST /v1/text-to-voice/design`** takes a description of a voice and a
//!   passage for it to read, and answers with **three** candidates. Each is a
//!   `generated_voice_id` and an MP3 of that voice reading the passage, base64
//!   inside the JSON. **This is the call that is billed**, once, for the
//!   passage — not three times, despite three samples coming back.
//! - **`POST /v1/text-to-voice`** takes one of those ids and a name, and turns
//!   it into a real voice with a `voice_id`, usable exactly like any voice from
//!   a listing. It costs nothing: the generation was already paid for above.
//!
//! **The two candidates nobody picks need no cleanup.** They were never
//! persisted as voices, so there is nothing to delete and no tidying pass to
//! forget. What the create call *does* accept is
//! `played_not_selected_voice_ids` — the ones that were listened to and passed
//! over — which the vendor uses to improve the model. Sending it is free and
//! costs a caller nothing, so this file carries it.
//!
//! # The lengths are the vendor's, and they are checked before the call
//!
//! A description under 20 characters or over 1000, or a passage outside
//! 100–1000, is refused by ElevenLabs with a `422`. Refusing locally instead
//! costs no round trip and names the field — see [`PROMPT`] and [`PASSAGE`].
//! The constants live here, with the request they constrain, so a reader with
//! the vendor's reference page open can tick them off; *acting* on them is
//! [`crate::voices::design`]'s business.

use serde::{Deserialize, Serialize};

use super::{BASE, KEY_HEADER};
use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// How long the description of a voice may be, in characters.
pub const PROMPT: std::ops::RangeInclusive<usize> = 20..=1000;

/// How long the passage a preview reads may be, in characters.
///
/// The lower bound is the interesting one: a hundred characters is roughly a
/// sentence and a half, and it is there because a voice cannot be judged on
/// three words. It is also the number the sample is billed by, which makes the
/// floor a floor on the price of one design.
pub const PASSAGE: std::ops::RangeInclusive<usize> = 100..=1000;

/// How many candidates one design call answers with.
///
/// Fixed by the vendor rather than asked for, and written down because it is
/// the fact that makes the billing counter-intuitive: three samples arrive and
/// one passage is charged.
pub const CANDIDATES: usize = 3;

/// The format the samples come back in.
///
/// The API's own default, and the same one the rest of scorsese's ElevenLabs
/// audio uses. Not a picker: the alternatives need somebody's subscription
/// tier, and a setting whose options depend on a plan is tier detection wearing
/// a dropdown.
pub const OUTPUT_FORMAT: &str = "mp3_44100_128";

/// What a design call asks for.
///
/// Declared rather than assembled, like every request in this directory, and
/// every optional field is omitted when it is absent — an explicit `null` is
/// not the same request as a field the vendor gets to default, and sending one
/// pins a value nobody chose.
#[derive(Debug, Clone, Serialize)]
pub struct DesignRequest {
    /// What the voice should be like, in a sentence. 20–1000 characters.
    pub voice_description: String,
    /// What the previews read aloud. 100–1000 characters, and **the only thing
    /// billed by this call**.
    pub text: String,
    /// The container and bitrate of the samples.
    pub output_format: &'static str,
    /// Makes the same description produce the same candidates again, as far as
    /// the vendor manages it. Best-effort and documented as such — it is worth
    /// recording rather than worth relying on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// How closely the candidates should follow the description. Higher is more
    /// literal and less varied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f64>,
}

/// The three candidates, as the vendor answers.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DesignReply {
    /// The candidates. Defaulted so an empty envelope reads as no candidates
    /// rather than as an unreadable reply.
    #[serde(default)]
    pub previews: Vec<PreviewSample>,
}

/// One candidate voice, and it reading the passage.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PreviewSample {
    /// The handle the create call turns into a real voice. **Not a
    /// `voice_id`** — nothing can be generated with it, and it does not outlive
    /// the vendor's own window for it.
    #[serde(default)]
    pub generated_voice_id: String,
    /// The sample, base64 inside the JSON.
    #[serde(default)]
    pub audio_base_64: String,
    /// What the bytes are, e.g. `audio/mpeg`.
    #[serde(default)]
    pub media_type: Option<String>,
    /// How long the sample runs.
    #[serde(default)]
    pub duration_secs: Option<f64>,
}

/// What turning a candidate into a real voice asks for.
#[derive(Debug, Clone, Serialize)]
pub struct CreateRequest {
    /// What the voice will be called in the account. The only name a person
    /// will ever see it under.
    pub voice_name: String,
    /// The description it was designed from, kept with the voice at the vendor
    /// so the account is not full of voices nobody can account for.
    pub voice_description: String,
    /// Which candidate to keep.
    pub generated_voice_id: String,
    /// The candidates that were heard and passed over, for the vendor's own
    /// training. Omitted when nothing was auditioned.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub played_not_selected_voice_ids: Vec<String>,
}

/// The voice that now exists in the account.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CreatedVoice {
    /// The id a narration names. The one field here that is load-bearing.
    #[serde(default)]
    pub voice_id: String,
    /// What it ended up called.
    #[serde(default)]
    pub name: String,
    /// The description it was created with, echoed back.
    #[serde(default)]
    pub description: Option<String>,
}

/// Voice Design, reachable.
///
/// Its own client rather than a method on [`Voices`](super::voices::Voices),
/// and deliberately: that one is the half of ElevenLabs that costs nothing, and
/// this is the half that spends. Keeping them apart makes the difference
/// visible in the type a caller is holding rather than in a comment somebody
/// has to find.
#[derive(Debug, Clone)]
pub struct Design {
    caller: Caller,
}

impl Design {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// Designs three candidates. **This is the call that is billed.**
    pub fn preview(&self, request: &DesignRequest) -> Result<DesignReply, HttpError> {
        self.caller
            .post(&format!("{BASE}/text-to-voice/design"), request)
    }

    /// Turns one candidate into a voice in the account.
    ///
    /// Free — the samples were paid for by [`Design::preview`] — and the one
    /// call in scorsese with an effect **outside** the project directory. What
    /// it produces lives in somebody's ElevenLabs account and does not travel
    /// with an `scp -r`, which is why the caller writes down what made it.
    pub fn create(&self, request: &CreateRequest) -> Result<CreatedVoice, HttpError> {
        self.caller.post(&format!("{BASE}/text-to-voice"), request)
    }
}
