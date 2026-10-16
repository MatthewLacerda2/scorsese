//! A voice from somebody's own recordings — Instant Voice Cloning (#1263).
//!
//! The third way to a voice, beside **finding** one ([`super`]) and
//! **designing** one ([`super::design`]): *narrate in my own voice*. About a
//! minute of a person's recorded speech, already an audio asset of the
//! project, becomes a voice in the ElevenLabs account, narrating like any
//! other.
//!
//! # Consent is part of the feature
//!
//! A person clones **only their own voice, or one they have permission
//! for**. The vendor's own app asks for that confirmation before it saves a
//! clone; its API takes no such field, so every surface here asks for it
//! ([`Ask::consent`]), refuses without it before anything is read or sent,
//! and says [`CONSENT`] — with the vendor's terms — wherever cloning is
//! described.
//!
//! # It costs nothing, and takes a slot
//!
//! Creating a clone is not on the vendor's list of what credits pay for
//! (`api::elevenlabs::cloning` has the reading, with its date), so there is
//! no quote and no token; narrating with the voice is charged as any line is.
//! What it does take is one of the account's custom voice slots, as a kept
//! design does, and [`forget`] gives it back — through the same
//! [`Studio::forget`](super::design::Studio::forget) call.
//!
//! # What it creates is outside the project
//!
//! As with a designed voice, the id travels and the voice does not, so
//! `cloned-voices.json` beside `project.json` records which recordings made
//! it ([`Cloned`]). Those are assets of the project, so a lost voice is
//! cloned again from them.

mod error;
mod ledger;
mod recordings;
mod run;

pub use error::CloneError;
pub use ledger::{Cloned, LEDGER_FILE, Source, read as cloned};
pub use recordings::{advice, media_type};
pub use run::{Ask, Done, FORGOTTEN, Made, check, clone, forget};

pub use crate::api::elevenlabs::cloning::{
    AddRequest as Request, ENOUGH_SECONDS, RECOMMENDED_SECONDS, Recording, TERMS, USE_POLICY,
};

/// What every surface says about consent, word for word.
pub const CONSENT: &str = "Clone only your own voice, or the voice of someone who has given you \
     permission to clone it. A cloned voice is bound by ElevenLabs' terms of use \
     (https://elevenlabs.io/terms-of-use) and its prohibited use policy \
     (https://elevenlabs.io/use-policy): never use one to impersonate, mislead or harm anyone.";
