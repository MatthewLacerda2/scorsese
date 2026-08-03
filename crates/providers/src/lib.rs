//! # scorsese-providers — turning briefs into media
//!
//! Responsibility: realising the generated asset kinds. Clients for the
//! generative providers (Veo for video, ElevenLabs for TTS audio), local
//! [`synth`]esis from a recipe, the content-addressed cache under
//! `generated/` — an unchanged brief is never realised twice — and the
//! generation state machine: `sketch → queued → generated → stale`.
//!
//! Two kinds of brief arrive here and they are not alike. A **prompt** is a
//! sentence handed to somebody else's model: it costs money, needs a network,
//! and cannot be reproduced from the project alone. A **recipe** is a document
//! the project carries: synthesis reads it locally, for free, and the same
//! recipe yields the same bytes forever. Only the first kind needs
//! credentials, and only the first kind needs mocking.
//!
//! Credentials come from `.env` (gitignored; `.env.example` documents the
//! keys). Real provider calls are NEVER made in tests — every networked
//! provider is mocked behind a trait. Synthesis needs no such arrangement: it
//! is arithmetic, so its tests are the real thing.
//!
//! Boundary: no rendering, no compositing, no GUI. This crate turns briefs
//! into media files on disk and updates asset state; it depends on
//! `scorsese-core` and `scorsese-zimmer` only.
//!
//! ## What this publishes
//!
//! [`synth`], by that one path: the verbs (`create`, `check`, `bake_asset` and
//! `bake_pending`, `survey`, `set`), the [`Recipe`](synth::Recipe) they read and
//! write, the [`Starter`](synth::Starter) a new one is cut from, and what they
//! answer with. How a recipe becomes samples — reading it, hashing it,
//! resolving the instruments it names — is this crate's own business, so those
//! modules are private.
//!
//! The providers proper have no surface here yet. When Veo and ElevenLabs
//! arrive they get a module of their own beside this one, because a prompt and
//! a recipe are not the same kind of brief and sharing an entry point is how
//! that distinction would go quietly missing.

pub mod synth;
