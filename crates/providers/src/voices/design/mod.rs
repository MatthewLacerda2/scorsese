//! A voice from a sentence, when no voice in either list is the one the video
//! needs.
//!
//! The sibling of [`super`], and the difference is one word: that module
//! **finds** a voice, this one **makes** one. Everything below follows from
//! three ways in which making is not finding.
//!
//! # It spends money, and it spends it oddly
//!
//! One design call answers with **three** candidates and is billed **once**,
//! for the passage they read. The obvious guess is three times the price, and
//! the obvious guess is wrong — which is exactly why the arithmetic is written
//! down in one place — `price::estimate` — with the sentence that explains it attached to
//! the number.
//!
//! Keeping a candidate costs nothing at all: it was paid for when it was
//! designed. That is why [`design`] and [`keep`] are two verbs rather than one
//! call with a flag — a person auditions three samples in between, and a single
//! call would either spend money on a choice nobody had made or hide the
//! audition from the only surface that can hold it.
//!
//! And the spending is **its own line item**. A total that folded a designed
//! voice into what narration cost would move while nobody was generating
//! narration, and a counter that does that is a counter nobody trusts. It is
//! still real money, so a ceiling counts it — see [`spent`].
//!
//! # What it creates is outside the project, and that breaks a promise
//!
//! Every other thing scorsese makes lands inside the `*.scor` directory, which
//! is what makes "a project survives `scp -r`" true. **A designed voice does
//! not.** It is created in somebody's ElevenLabs account; the project can only
//! hold its id, so the id travels and the voice does not.
//!
//! The answer is the **ledger**: a file beside `project.json` recording, for each
//! designed voice, the **description and the seed** that made it. A voice that
//! has gone — deleted, or the project opened under another account — is then a
//! setback rather than a loss, because the same sentence and the same seed
//! designed again is the nearest thing there is to asking for it twice. The
//! seed is best-effort by the vendor's own account and is written down as
//! exactly that; a best-effort attempt still beats a dead id and no idea what
//! it sounded like.
//!
//! # An unchanged brief is never paid for twice
//!
//! The three samples land in `generated/`, named for the hash of the brief that
//! produced them, and that file existing is the answer to *has this been paid
//! for* — the same guarantee a generated shot has, arrived at the same way. So
//! a repeated command, a dropped connection, or listening again tomorrow all
//! cost nothing. [`Brief::digest`] is what makes it true, and its module doc
//! carries the warning that goes with it: adding a line to the fingerprint
//! later re-bills every design already paid for.
//!
//! # The trait is the test seam
//!
//! [`Studio`] is what the verbs talk to, so the whole of this — the cache hit,
//! the ceiling, the plan refusal, a candidate that has gone, the ledger — is
//! exercised without a network, a key, or a cent. The repo rule against real
//! provider calls in tests is satisfied by construction rather than by anyone
//! remembering.

mod brief;
mod elevenlabs;
mod error;
mod ledger;
mod price;
mod run;
mod session;
mod studio;

pub use brief::Brief;
pub use elevenlabs::ElevenLabsStudio;
pub use error::DesignError;
pub use ledger::{Designed, LEDGER_FILE, read as designed, spent};
pub use price::{Estimate, estimate};
pub use run::{Designing, Kept, design, keep};
pub use session::{Sample, Session};
pub use studio::{Candidate, Studio};

pub use crate::api::elevenlabs::design::{CANDIDATES, PASSAGE, PROMPT};
