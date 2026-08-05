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
//! Credentials come from one resolver and one order — see [`credentials`]:
//! the environment first (an exported variable, or the `.env` at the root of a
//! checkout), then the settings file a shipped build keeps per machine. Real provider calls are NEVER made in tests — every networked
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
//! [`prices`], which is what a prompted brief costs before anyone commits to
//! it. It publishes no client and makes no call: it is a table somebody copied
//! off a vendor's page, dated, and the arithmetic over it. The one thing to
//! carry away from reading it is that **nothing here is a bill** — no provider
//! reports what a generation cost, so every figure is our own estimate and the
//! naming says so throughout.
//!
//! The providers proper have no surface here yet. When Veo and ElevenLabs
//! arrive they get a module of their own beside this one, because a prompt and
//! a recipe are not the same kind of brief and sharing an entry point is how
//! that distinction would go quietly missing.
//!
//! [`voices`], which is where a narration's `voice_id` comes from. It spends
//! nothing — reading a list is free — but it is the module that keeps the
//! spending honest, because **every ElevenLabs default voice expires on
//! 2026-12-31** and a voice id written into this repository would be an outage
//! with a date on it. It resolves the list at runtime, caches it under the
//! project's `cache/`, and names the state a withdrawn voice leaves an asset
//! in rather than substituting another one.

pub mod api;
pub mod credentials;
pub mod prices;
pub mod synth;
pub mod video;
pub mod voices;
