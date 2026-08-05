//! Hand-written stand-ins for the SDKs that do not exist.
//!
//! Neither Google nor ElevenLabs ships a Rust client, so every call scorsese
//! makes is a JSON body over HTTPS that somebody here wrote out by hand. This
//! directory is where that work lives, and it is kept apart from the rest of
//! the crate on purpose: what a vendor's wire format *is* changes for reasons
//! that have nothing to do with scorsese, and it should be possible to read a
//! vendor's documentation beside one of these files and check them off against
//! each other line by line.
//!
//! Two rules make that possible.
//!
//! **The wire is declared, never assembled.** Each request and each reply is a
//! serde type whose fields are named exactly as the vendor names them, in the
//! shape the vendor nests them. No hand-built JSON, no string formatting, no
//! `json!` macro at a call site. Turning a scorsese asset into one of those
//! types is a separate function in a separate file, so the translation is
//! somewhere you can look at it rather than spread through the code that makes
//! the call.
//!
//! **One place names an HTTP library.** [`http`] is the whole of it — every
//! module here goes through that, so swapping the transport touches one file
//! and reading "how do we talk to the network" is one file to read. It is also
//! what keeps a vendor module honest: with no client to reach for, a vendor
//! module can only describe its endpoints and its payloads.
//!
//! What does **not** belong here: the sketch lifecycle, the brief-hash cache,
//! what a generation costs, where its output lands. Those are scorsese's
//! business and the vendor has no opinion about them, so they live beside this
//! directory rather than inside it. A module here answers exactly one question
//! — *what does this vendor's API look like* — and a reader who wants that
//! answer should not have to read past anything else to get it.

pub mod elevenlabs;
pub mod http;
pub mod veo;
