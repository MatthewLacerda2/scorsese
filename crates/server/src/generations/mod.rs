//! The paid generation jobs (#539): a Veo shot, a Gemini still (#461) — drawn
//! now, or ordered in a half-price batch (#947) — and an ElevenLabs line, made
//! by
//! the job queue for one user, paid from their credits, kept in their library.
//!
//! A job is enqueued by web `generate` once the user has confirmed a quote,
//! in the same transaction that reserves its price
//! ([`credits::generations::start`](crate::credits::generations::start), which
//! records the job on the audit row). Its life then is:
//!
//! 1. **Asked for already?** If the library holds a file made from this brief
//!    hash — another job got there first — nothing is sent, and the
//!    reservation is released: nothing was spent.
//! 2. **Sent.** The brief is gathered from the document as it was when the
//!    quote was confirmed, laid out with the user's files, by the same
//!    `scorsese_providers` gathering a local run uses — so what is sent is what
//!    was quoted. A shot's ticket — or a batched still's batch job name — is
//!    committed to the job's row and the audit row the moment Google accepts;
//!    a job that comes back from a crash with a ticket **polls and never
//!    submits** (`jobs`, *Veo: never pay twice*).
//! 3. **Settled.** A generation that worked is kept with
//!    `Library::keep_generated` and charged; one the provider refused is free
//!    ([`credits::generations::finish`](crate::credits::generations::finish)).
//!    A shot that outlasts [`Timing::patience`], or a batch
//!    [`Timing::batch_patience`], goes `stuck`, its reservation held, since
//!    Google may still be billing.
//! 4. **Adopted.** The user's project is opened as it is *now*, laid out with
//!    their generations, and `scorsese_providers`' own `adopt` points each
//!    asset whose current brief has a file at it — then it is measured and
//!    saved. An asset whose brief was edited meanwhile is left alone; the file
//!    is in the library, and is free the day that brief comes back.
//!
//! **The vendors are a trait** ([`Vendors`]), so no test spends a cent: the
//! server's is [`Keys`], which resolves `GEMINI_API_KEY` and
//! `ELEVENLABS_API_KEY` through the one credentials resolver when a job needs
//! one, and a missing key fails that job — free — rather than the server.

mod adopt;
mod batch;
mod land;
mod line;
mod shot;
mod still;

use std::time::Duration;

use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::image::{GeminiProvider, ImageProvider};
use scorsese_providers::speech::{ElevenLabsProvider, SpeechProvider};
use scorsese_providers::video::{POLL_EVERY, VeoProvider, VideoProvider};
use scorsese_providers::voices::design::{ElevenLabsStudio, Studio as DesignStudio};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use adopt::{Adopted, adopt};
pub use batch::handler as batch_handler;
pub use line::handler as line_handler;
pub use shot::handler as shot_handler;
pub use still::handler as still_handler;

use crate::jobs::kinds::{BATCH_PATIENCE, PROVIDER_PATIENCE};

/// What a generation job carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// The project the generation is for.
    pub project: i64,
    /// The asset whose brief it realises.
    pub asset: String,
    /// The brief's hash — what the quote was bound to and the output is found
    /// by.
    pub brief: String,
    /// The document as it was when the quote was confirmed.
    pub document: Value,
}

/// A video provider a job can own and move between threads.
pub type Video = Box<dyn VideoProvider + Send + Sync>;

/// An image provider a job can own and move between threads.
pub type Image = Box<dyn ImageProvider + Send + Sync>;

/// A speech provider a job can own and move between threads.
pub type Speech = Box<dyn SpeechProvider + Send + Sync>;

/// Where voices are designed (#572), owned and moved between threads.
pub type Studio = Box<dyn DesignStudio + Send + Sync>;

/// Where the jobs' providers come from.
pub trait Vendors: Send + Sync + 'static {
    /// Veo, or why it cannot be reached — no key, most likely.
    fn video(&self) -> Result<Video, String>;
    /// ElevenLabs, or why it cannot be reached.
    fn speech(&self) -> Result<Speech, String>;
    /// Gemini's image models, or why they cannot be reached.
    fn image(&self) -> Result<Image, String>;
    /// ElevenLabs' voice design, or why it cannot be reached
    /// ([`crate::designs`]).
    fn studio(&self) -> Result<Studio, String>;
}

/// The real vendors, with keys from the one credentials resolver
/// (`docs/credentials.md`): the server's environment, where compose passes
/// `GEMINI_API_KEY` and `ELEVENLABS_API_KEY`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Keys;

impl Vendors for Keys {
    fn video(&self) -> Result<Video, String> {
        let key = resolve(Provider::Gemini).map_err(|error| error.to_string())?;
        Ok(Box::new(VeoProvider::new(&key.secret)))
    }

    fn speech(&self) -> Result<Speech, String> {
        let key = resolve(Provider::ElevenLabs).map_err(|error| error.to_string())?;
        Ok(Box::new(ElevenLabsProvider::new(&key.secret)))
    }

    fn image(&self) -> Result<Image, String> {
        let key = resolve(Provider::Gemini).map_err(|error| error.to_string())?;
        Ok(Box::new(GeminiProvider::new(&key.secret)))
    }

    fn studio(&self) -> Result<Studio, String> {
        let key = resolve(Provider::ElevenLabs).map_err(|error| error.to_string())?;
        Ok(Box::new(ElevenLabsStudio::new(&key.secret)))
    }
}

/// How a shot job, and a batched still's, waits.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// Between a shot's polls.
    pub poll_every: Duration,
    /// Before a shot's job goes `stuck`.
    pub patience: Duration,
    /// Between asking after a batch.
    pub batch_every: Duration,
    /// Before a batch's job goes `stuck`.
    pub batch_patience: Duration,
}

/// Between asking after a batch: a minute. It answers within a day, so asking
/// every ten seconds as a shot does would be thousands of calls to learn
/// nothing.
pub const BATCH_EVERY: Duration = Duration::from_secs(60);

impl Default for Timing {
    /// The providers' own poll interval, and the queue's patience — a
    /// minute and [`BATCH_PATIENCE`] for a batch.
    fn default() -> Self {
        Self {
            poll_every: POLL_EVERY,
            patience: PROVIDER_PATIENCE,
            batch_every: BATCH_EVERY,
            batch_patience: BATCH_PATIENCE,
        }
    }
}
